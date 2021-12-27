use cli_table::{format::Justify, Cell, Style, Table, TableStruct};
use either::Either;
use eyre::Result;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

mod error;
mod model;

pub use error::*;
pub use model::*;

const BENCHER_CONFIG_FILENAME: &str = ".bencher-config";

pub struct Config {
    db: rusqlite::Connection,
    inner_config: BencherConfig,
}

pub struct ExperimentHandle<'a> {
    db: &'a rusqlite::Connection,
    experiment: Experiment,
}

impl<'a> Config {
    fn find_config_dir() -> Result<PathBuf> {
        let mut dir: PathBuf = Path::new(".").canonicalize()?;
        while !dir.parent().is_none() {
            let file = dir.join(BENCHER_CONFIG_FILENAME);
            if file.exists() {
                return Ok(dir);
            }

            dir.pop();
        }

        Err(BencherError::NotFound.into())
    }

    fn open_db(db_path: &Path) -> Result<rusqlite::Connection> {
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_CREATE;

        match rusqlite::Connection::open_with_flags(db_path, flags) {
            Ok(conn) => Ok(conn),
            Err(e) => Err(BencherError::Database(e).into()),
        }
    }

    fn setup_db(db: &rusqlite::Connection) -> Result<()> {
        db.execute(
            "create table results (
                experiment_label varchar,

                x_int int,
                x_int_1 int,
                x_int_5 int,
                x_int_10 int,
                x_int_25 int,
                x_int_99 int,
                x_int_95 int,
                x_int_90 int,
                x_int_75 int,

                y_int int,
                y_int_1 int,
                y_int_5 int,
                y_int_10 int,
                y_int_25 int,
                y_int_99 int,
                y_int_95 int,
                y_int_90 int,
                y_int_75 int,

                x_float float,
                x_float_1 float,
                x_float_5 float,
                x_float_10 float,
                x_float_25 float,
                x_float_99 float,
                x_float_95 float,
                x_float_90 float,
                x_float_75 float,

                y_float float,
                y_float_1 float,
                y_float_5 float,
                y_float_10 float,
                y_float_25 float,
                y_float_99 float,
                y_float_95 float,
                y_float_90 float,
                y_float_75 float,

                primary key (experiment_label, x_int, x_float, y_int, y_float)
            )",
            [],
        )?;
        Ok(())
    }

    pub fn new() -> Result<Self> {
        let config_dir = Self::find_config_dir()?;
        let config_file_path = config_dir.join(BENCHER_CONFIG_FILENAME);

        let config_file = File::open(config_file_path)?;
        let reader = BufReader::new(config_file);

        let inner_config: BencherConfig = serde_json::from_reader(reader)?;

        let db_path = config_dir.join(&inner_config.database_filepath);
        let is_created = db_path.exists();

        let db = Self::open_db(&db_path)?;

        if !is_created {
            Self::setup_db(&db)?;
        }

        Ok(Self { db, inner_config })
    }

    pub fn experiments(&self) -> Vec<Experiment> {
        self.inner_config.experiments.clone()
    }

    pub fn status(&self) -> Result<Vec<ExperimentStatus>> {
        let mut stmt = self
            .db
            .prepare("select experiment_label, count(*) from results group by experiment_label")?;
        let mut vec = vec![];
        for status in stmt.query_map([], |row| {
            Ok(ExperimentStatus {
                label: row.get(0).unwrap_or("".to_string()),
                n_datapoints: row.get(1).unwrap_or(0),
            })
        })? {
            vec.push(status.unwrap())
        }

        Ok(vec)
    }

    pub fn get_experiment_handle(&'a self, label: &str) -> Option<ExperimentHandle<'a>> {
        self.inner_config
            .experiments
            .iter()
            .find(|e| e.label == label)
            .map(|e| ExperimentHandle::new(&self.db, e.clone()))
    }
}

impl<'a> ExperimentHandle<'a> {
    fn new(db: &'a rusqlite::Connection, experiment: Experiment) -> Self {
        Self { db, experiment }
    }

    fn get_datapoints(&self) -> Result<Vec<Datapoint>> {
        fn create_confidence_arg(
            min_int: Option<i64>,
            max_int: Option<i64>,
            min_float: Option<f64>,
            max_float: Option<f64>,
        ) -> Option<Either<(i64, i64), (f64, f64)>> {
            if let (Some(lower), Some(upper)) = (min_int, max_int) {
                Some(Either::Left((lower, upper)))
            } else if let (Some(lower), Some(upper)) = (min_float, max_float) {
                Some(Either::Right((lower, upper)))
            } else {
                None
            }
        }

        let mut stmt = self.db.prepare(
            "select (
                                x_int,
                                x_float,

                                y_int,
                                y_float,

                                x_int_1,
                                x_int_99,
                                x_float_1,
                                x_float_99,

                                x_int_5,
                                x_int_95,
                                x_float_5,
                                x_float_95,

                                x_int_10,
                                x_int_90,
                                x_float_10,
                                x_float_90,

                                x_int_25,
                                x_int_75,
                                x_float_25,
                                x_float_75,

                                y_int_1,
                                y_int_99,
                                y_float_1,
                                y_float_99,

                                y_int_5,
                                y_int_95,
                                y_float_5,
                                y_float_95,

                                y_int_10,
                                y_int_90,
                                y_float_10,
                                y_float_90,

                                y_int_25,
                                y_int_75,
                                y_float_25,
                                y_float_75,
                             )
                      from results
                      where experiment_label = :label",
        )?;

        let mut vec = vec![];
        for datapoint in stmt.query_map(
            rusqlite::named_params! { ":label": &self.experiment.label },
            |row| {
                let mut datapoint = Datapoint::new(
                    row.get(0).unwrap(),
                    row.get(1).unwrap(),
                    row.get(2).unwrap(),
                    row.get(3).unwrap(),
                )?;

                // x 1 - 99
                if let Some(e) = create_confidence_arg(
                    row.get(4).unwrap(),
                    row.get(5).unwrap(),
                    row.get(6).unwrap(),
                    row.get(7).unwrap(),
                ) {
                    let _ = datapoint.add_x_confidence(1, e);
                }

                // x 5 - 95
                if let Some(e) = create_confidence_arg(
                    row.get(8).unwrap(),
                    row.get(9).unwrap(),
                    row.get(10).unwrap(),
                    row.get(11).unwrap(),
                ) {
                    let _ = datapoint.add_x_confidence(5, e);
                }

                // x 10 - 90
                if let Some(e) = create_confidence_arg(
                    row.get(12).unwrap(),
                    row.get(13).unwrap(),
                    row.get(14).unwrap(),
                    row.get(15).unwrap(),
                ) {
                    let _ = datapoint.add_x_confidence(10, e);
                }

                // x 25 - 75
                if let Some(e) = create_confidence_arg(
                    row.get(16).unwrap(),
                    row.get(17).unwrap(),
                    row.get(18).unwrap(),
                    row.get(19).unwrap(),
                ) {
                    let _ = datapoint.add_x_confidence(10, e);
                }

                // y 1 - 99
                if let Some(e) = create_confidence_arg(
                    row.get(20).unwrap(),
                    row.get(21).unwrap(),
                    row.get(22).unwrap(),
                    row.get(23).unwrap(),
                ) {
                    let _ = datapoint.add_y_confidence(1, e);
                }

                // y 5 - 95
                if let Some(e) = create_confidence_arg(
                    row.get(24).unwrap(),
                    row.get(25).unwrap(),
                    row.get(26).unwrap(),
                    row.get(27).unwrap(),
                ) {
                    let _ = datapoint.add_y_confidence(5, e);
                }

                // y 10 - 90
                if let Some(e) = create_confidence_arg(
                    row.get(28).unwrap(),
                    row.get(29).unwrap(),
                    row.get(30).unwrap(),
                    row.get(31).unwrap(),
                ) {
                    let _ = datapoint.add_y_confidence(10, e);
                }

                // y 25 - 75
                if let Some(e) = create_confidence_arg(
                    row.get(32).unwrap(),
                    row.get(33).unwrap(),
                    row.get(34).unwrap(),
                    row.get(35).unwrap(),
                ) {
                    let _ = datapoint.add_y_confidence(10, e);
                }
                Ok(datapoint)
            },
        )? {
            vec.push(datapoint?);
        }

        Ok(vec)
    }

    fn get_datapoints_magnitudes(&self) -> Result<(Vec<Datapoint>, Magnitude, Magnitude)> {
        let datapoints = self.get_datapoints()?;
        let mut x_magnitude_counts = [0; 7];
        let mut y_magnitude_counts = [0; 7];

        datapoints.iter().for_each(|d| {
            let (x_mag, y_mag) = d.magnitudes();
            match x_mag {
                Magnitude::Nano => x_magnitude_counts[0] += 1,
                Magnitude::Micro => x_magnitude_counts[1] += 1,
                Magnitude::Mili => x_magnitude_counts[2] += 1,
                Magnitude::Normal => x_magnitude_counts[3] += 1,
                Magnitude::Kilo => x_magnitude_counts[4] += 1,
                Magnitude::Mega => x_magnitude_counts[5] += 1,
                Magnitude::Giga => x_magnitude_counts[6] += 1,
            }
            match y_mag {
                Magnitude::Nano => y_magnitude_counts[0] += 1,
                Magnitude::Micro => y_magnitude_counts[1] += 1,
                Magnitude::Mili => y_magnitude_counts[2] += 1,
                Magnitude::Normal => y_magnitude_counts[3] += 1,
                Magnitude::Kilo => y_magnitude_counts[4] += 1,
                Magnitude::Mega => y_magnitude_counts[5] += 1,
                Magnitude::Giga => y_magnitude_counts[6] += 1,
            }
        });

        let (x_idx, _) = x_magnitude_counts
            .iter()
            .enumerate()
            .max_by_key(|v| v.1)
            .unwrap();
        let (y_idx, _) = y_magnitude_counts
            .iter()
            .enumerate()
            .max_by_key(|v| v.1)
            .unwrap();

        let x_mag = match x_idx {
            0 => Magnitude::Nano,
            1 => Magnitude::Micro,
            2 => Magnitude::Mili,
            3 => Magnitude::Normal,
            4 => Magnitude::Kilo,
            5 => Magnitude::Mega,
            _ => Magnitude::Giga,
        };

        let y_mag = match y_idx {
            0 => Magnitude::Nano,
            1 => Magnitude::Micro,
            2 => Magnitude::Mili,
            3 => Magnitude::Normal,
            4 => Magnitude::Kilo,
            5 => Magnitude::Mega,
            _ => Magnitude::Giga,
        };

        Ok((datapoints, x_mag, y_mag))
    }

    pub fn get_table(&self) -> Result<TableStruct> {
        let (datapoints, x_mag, y_mag) = self.get_datapoints_magnitudes()?;

        Ok(datapoints
            .into_iter()
            .map(|d| {
                vec![
                    d.x.display_with_magnitude(x_mag)
                        .cell()
                        .justify(Justify::Right),
                    d.y.display_with_magnitude(y_mag)
                        .cell()
                        .justify(Justify::Right),
                ]
            })
            .collect::<Vec<_>>()
            .table()
            .title(vec![
                format!(
                    "{} ({}{})",
                    self.experiment.x_label,
                    x_mag.prefix(),
                    self.experiment.x_units
                )
                .cell()
                .justify(Justify::Center)
                .bold(true),
                format!(
                    "{} ({}{})",
                    self.experiment.y_label,
                    y_mag.prefix(),
                    self.experiment.y_units
                )
                .cell()
                .justify(Justify::Center)
                .bold(true),
            ])
            .bold(true))
    }

    pub fn get_latex_table(&self) -> Result<String> {
        let (datapoints, x_mag, y_mag) = self.get_datapoints_magnitudes()?;
        let mut table =
            "\\begin{table}[t]\n\t\\centering\n\t\\begin{tabular}{|r|r|}\n\t\t\\hline\n"
                .to_string();
        table += &format!(
            "\t\t\\textbf{{ {} ({}{}) }} & \\textbf{{ {} ({}{}) }} \\\\ \\hline\n",
            self.experiment.x_label,
            x_mag.prefix(),
            self.experiment.x_units,
            self.experiment.y_label,
            y_mag.prefix(),
            self.experiment.y_units
        );
        for d in datapoints {
            table += &format!(
                "\t\t${:>8}$ & ${:>8}$ \\\\ \\hline\n",
                d.x.display_with_magnitude(x_mag),
                d.y.display_with_magnitude(y_mag)
            )
        }
        table += "\t\\end{tabular}\n\t\\caption{Caption}\\label{table:label}\\end{table}";
        Ok(table)
    }

    pub fn get_gnuplot(&self) -> Result<String> {
        todo!()
    }

    pub fn get_dat(&self) -> Result<String> {
        let (datapoints, x_mag, y_mag) = self.get_datapoints_magnitudes()?;

        let mut dat = format!(
            "# x axis: {} ({}{})\n# y axis: {} ({}{})\n\n",
            self.experiment.x_label,
            x_mag.prefix(),
            self.experiment.x_units,
            self.experiment.y_label,
            y_mag.prefix(),
            self.experiment.y_units
        );
        for d in datapoints {
            dat += &format!(
                "{:>8} {:>8}\n",
                d.x.display_with_magnitude(x_mag),
                d.y.display_with_magnitude(y_mag)
            );
        }
        dat += "# end";
        Ok(dat)
    }

    pub fn add_datapoint(&self, datapoint: Datapoint) -> Result<()> {
        let mut stmt = self.db.prepare(
            "insert into results (
                    x_int,
                    x_int_1,
                    x_int_5,
                    x_int_10,
                    x_int_25,
                    x_int_99,
                    x_int_95,
                    x_int_90,
                    x_int_75,

                    y_int,
                    y_int_1,
                    y_int_5,
                    y_int_10,
                    y_int_25,
                    y_int_99,
                    y_int_95,
                    y_int_90,
                    y_int_75

                    x_float,
                    x_float_1,
                    x_float_5,
                    x_float_10,
                    x_float_25,
                    x_float_99,
                    x_float_95,
                    x_float_90,
                    x_float_75,

                    y_float,
                    y_float_1,
                    y_float_5,
                    y_float_10,
                    y_float_25,
                    y_float_99,
                    y_float_95,
                    y_float_90,
                    y_float_75
                ) values (
                    :x_int,
                    :x_int_1,
                    :x_int_5,
                    :x_int_10,
                    :x_int_25,
                    :x_int_99,
                    :x_int_95,
                    :x_int_90,
                    :x_int_75,

                    :y_int,
                    :y_int_1,
                    :y_int_5,
                    :y_int_10,
                    :y_int_25,
                    :y_int_99,
                    :y_int_95,
                    :y_int_90,
                    :y_int_75

                    :x_float,
                    :x_float_1,
                    :x_float_5,
                    :x_float_10,
                    :x_float_25,
                    :x_float_99,
                    :x_float_95,
                    :x_float_90,
                    :x_float_75,

                    :y_float,
                    :y_float_1,
                    :y_float_5,
                    :y_float_10,
                    :y_float_25,
                    :y_float_99,
                    :y_float_95,
                    :y_float_90,
                    :y_float_75
                )",
        )?;

        stmt.execute(rusqlite::named_params! {
            ":x_int": datapoint.x.to_int(),
            ":x_float": datapoint.x.to_float(),
            ":y_int": datapoint.y.to_int(),
            ":y_float": datapoint.y.to_float(),

            ":x_int_1": datapoint.get_x_confidence(1).clone().map(|val| val.0.to_int()).flatten(),
            ":x_int_99": datapoint.get_x_confidence(1).clone().map(|val| val.1.to_int()).flatten(),

            ":x_int_5": datapoint.get_x_confidence(5).clone().map(|val| val.0.to_int()).flatten(),
            ":x_int_95": datapoint.get_x_confidence(5).clone().map(|val| val.1.to_int()).flatten(),

            ":x_int_10": datapoint.get_x_confidence(10).clone().map(|val| val.0.to_int()).flatten(),
            ":x_int_90": datapoint.get_x_confidence(10).clone().map(|val| val.1.to_int()).flatten(),

            ":x_int_25": datapoint.get_x_confidence(25).clone().map(|val| val.0.to_int()).flatten(),
            ":x_int_75": datapoint.get_x_confidence(25).clone().map(|val| val.1.to_int()).flatten(),

            ":x_float_1": datapoint.get_x_confidence(1).clone().map(|val| val.0.to_float()).flatten(),
            ":x_float_99": datapoint.get_x_confidence(1).clone().map(|val| val.1.to_float()).flatten(),

            ":x_float_5": datapoint.get_x_confidence(5).clone().map(|val| val.0.to_float()).flatten(),
            ":x_float_95": datapoint.get_x_confidence(5).clone().map(|val| val.1.to_float()).flatten(),

            ":x_float_10": datapoint.get_x_confidence(10).clone().map(|val| val.0.to_float()).flatten(),
            ":x_float_90": datapoint.get_x_confidence(10).clone().map(|val| val.1.to_float()).flatten(),

            ":x_float_25": datapoint.get_x_confidence(25).clone().map(|val| val.0.to_float()).flatten(),
            ":x_float_75": datapoint.get_x_confidence(25).clone().map(|val| val.1.to_float()).flatten(),

            ":y_int_1": datapoint.get_y_confidence(1).clone().map(|val| val.0.to_int()).flatten(),
            ":y_int_99": datapoint.get_y_confidence(1).clone().map(|val| val.1.to_int()).flatten(),

            ":y_int_5": datapoint.get_y_confidence(5).clone().map(|val| val.0.to_int()).flatten(),
            ":y_int_95": datapoint.get_y_confidence(5).clone().map(|val| val.1.to_int()).flatten(),

            ":y_int_10": datapoint.get_y_confidence(10).clone().map(|val| val.0.to_int()).flatten(),
            ":y_int_90": datapoint.get_y_confidence(10).clone().map(|val| val.1.to_int()).flatten(),

            ":y_int_25": datapoint.get_y_confidence(25).clone().map(|val| val.0.to_int()).flatten(),
            ":y_int_75": datapoint.get_y_confidence(25).clone().map(|val| val.1.to_int()).flatten(),

            ":y_float_1": datapoint.get_y_confidence(1).clone().map(|val| val.0.to_float()).flatten(),
            ":y_float_99": datapoint.get_y_confidence(1).clone().map(|val| val.1.to_float()).flatten(),

            ":y_float_5": datapoint.get_y_confidence(5).clone().map(|val| val.0.to_float()).flatten(),
            ":y_float_95": datapoint.get_y_confidence(5).clone().map(|val| val.1.to_float()).flatten(),

            ":y_float_10": datapoint.get_y_confidence(10).clone().map(|val| val.0.to_float()).flatten(),
            ":y_float_90": datapoint.get_y_confidence(10).clone().map(|val| val.1.to_float()).flatten(),

            ":y_float_25": datapoint.get_y_confidence(25).clone().map(|val| val.0.to_float()).flatten(),
            ":y_float_75": datapoint.get_y_confidence(25).clone().map(|val| val.1.to_float()).flatten(),
        })?;
        Ok(())
    }
}
