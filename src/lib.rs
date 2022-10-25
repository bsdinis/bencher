use cli_table::{format::Justify, Cell, Style, Table};
use either::Either;
use rusqlite::OptionalExtension;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

mod error;
mod model;
pub mod stat;

pub use error::*;
pub use model::*;

const BENCHER_CONFIG_FILENAME: &str = ".bencher-config";
const COLORS: [&str; 5] = ["f6511d", "ffb400", "00a6ed", "7fb800", "0d2c54"];

type BencherResult<T> = std::result::Result<T, BencherError>;

pub enum Axis {
    X,
    Y,
}

#[derive(Debug)]
pub struct Config {
    db: rusqlite::Connection,
    inner_config: BencherConfig,
}

impl<'a> Config {
    pub fn new() -> BencherResult<Self> {
        let config_dir = find_config_dir()?;

        let config_file_path = config_dir.join(BENCHER_CONFIG_FILENAME);
        let config_file = File::open(config_file_path)?;
        let reader = BufReader::new(config_file);
        let inner_config: BencherConfig = serde_json::from_reader(reader)?;

        let db_path = config_dir.join(&inner_config.database_filepath);
        let db = open_db(&db_path)?;

        setup_db(&db)?;

        Ok(Self { db, inner_config })
    }

    pub fn from_conn_and_config(
        db: rusqlite::Connection,
        inner_config: BencherConfig,
    ) -> BencherResult<Self> {
        setup_db(&db)?;
        Ok(Self { db, inner_config })
    }

    fn check_if_code_exists(&self, exp_type: &str, label: &str, code: &str) -> BencherResult<bool> {
        if let Some((existing_type, existing_label)) = self.db.query_row(
            "select experiment_type, experiment_label from experiments where experiment_code = :code",
            rusqlite::named_params! { ":code": code },
            |row| Ok((row.get(0).unwrap_or("".into()), row.get(1).unwrap_or("".into())))
            ).optional()? {

            if &existing_type != exp_type {
                Err(BencherError::MismatchedType(code.into(), existing_type, exp_type.into()))
            } else if &existing_label != label {
                Err(BencherError::MismatchedLabel(code.into(), existing_label, label.into()))
            } else {
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    pub fn add_linear_experiment(
        &self,
        exp_type: &str,
        label: &str,
        code: &str,
    ) -> BencherResult<()> {
        if self.check_if_code_exists(exp_type, label, code)? {
            return Ok(());
        }

        let mut stmt = self.db.prepare(
            "insert into experiments (
                    experiment_type,
                    experiment_label,
                    experiment_code
                    ) values (
                    :exp_type,
                    :label,
                    :code)",
        )?;

        stmt.execute(rusqlite::named_params! {
            ":exp_type": exp_type,
            ":label": label,
            ":code": code,
        })?;

        Ok(())
    }

    pub fn add_xy_experiment(&self, exp_type: &str, label: &str, code: &str) -> BencherResult<()> {
        if self.check_if_code_exists(exp_type, label, code)? {
            return Ok(());
        }

        let mut stmt = self.db.prepare(
            "insert into experiments (
                    experiment_type,
                    experiment_label,
                    experiment_code
                    ) values (
                    :exp_type,
                    :label,
                    :code)",
        )?;

        stmt.execute(rusqlite::named_params! {
            ":exp_type": exp_type,
            ":label": label,
            ":code": code,
        })?;

        Ok(())
    }

    fn get_linear_experiment(&self, exp_type: &str) -> BencherResult<LinearExperiment> {
        Ok(self
            .inner_config
            .linear_experiments
            .iter()
            .find(|e| e.exp_type == exp_type)
            .ok_or_else(|| {
                BencherError::ExperimentNotFound(
                    exp_type.to_string(),
                    self.inner_config
                        .linear_experiments
                        .iter()
                        .map(|e| e.exp_type.clone())
                        .collect::<Vec<_>>()
                        .join(","),
                )
            })?
            .clone())
    }

    fn get_linear_experiment_set(&self, code: &str) -> BencherResult<LinearExperimentSet> {
        let (label, exp_type) = self
            .db
            .query_row("select experiment_label, experiment_type from experiments where experiment_code = :code",
                       rusqlite::named_params! {":code": code}, |row| Ok((row.get(0).unwrap_or("".to_string()).clone(), row.get(1).unwrap_or("".to_string()).clone())))?;

        self.get_linear_experiment(&exp_type)
            .map(|e| LinearExperimentSet {
                experiment: e,
                label,
                code: code.to_string(),
            })
    }

    fn get_linear_experiment_sets(
        &self,
        exp_type: &str,
    ) -> BencherResult<Vec<LinearExperimentSet>> {
        let experiment = self.get_linear_experiment(exp_type)?;

        let mut stmt = self
            .db
            .prepare("select experiment_code, experiment_label from experiments where experiment_type = :type")?;

        let mut vec = vec![];
        for line in stmt.query_map(rusqlite::named_params! {":type": exp_type}, |row| {
            Ok(LinearExperimentSet {
                experiment: experiment.clone(),
                code: row.get(0).unwrap_or("".to_string()),
                label: row.get(1).unwrap_or("".to_string()),
            })
        })? {
            vec.push(line.unwrap());
        }

        Ok(vec)
    }

    pub fn linear_experiments(&self) -> Vec<LinearExperiment> {
        self.inner_config.linear_experiments.clone()
    }

    pub fn linear_experiment_sets(&self) -> BencherResult<Vec<LinearExperimentSet>> {
        let mut stmt = self.db.prepare(
            "select experiment_type, experiment_label, experiment_code from experiments",
        )?;

        let mut vec = vec![];
        for exp_info in stmt.query_map([], |row| {
            Ok((
                row.get(0).unwrap_or("".to_string()),
                row.get(1).unwrap_or("".to_string()),
                row.get(2).unwrap_or("".to_string()),
            ))
        })? {
            let (exp_type, label, code) = exp_info.unwrap();
            if let Ok(experiment) = self.get_linear_experiment(&exp_type) {
                vec.push(LinearExperimentSet {
                    experiment,
                    label,
                    code,
                });
            }
        }

        Ok(vec)
    }

    pub fn get_linear_experiment_handle(
        &'a self,
        exp_type: &str,
    ) -> BencherResult<LinearExperimentHandle<'a>> {
        Ok(LinearExperimentHandle::new(
            &self.db,
            self.get_linear_experiment(exp_type)?,
            self.get_linear_experiment_sets(exp_type)?,
        )?)
    }

    pub fn get_linear_set_handle(&'a self, code: &str) -> Option<LinearSetHandle<'a>> {
        self.get_linear_experiment_set(code)
            .map(|exp_set| LinearSetHandle::new(&self.db, exp_set))
            .ok()
    }

    fn get_xy_experiment(&self, exp_type: &str) -> BencherResult<XYExperiment> {
        Ok(self
            .inner_config
            .xy_experiments
            .iter()
            .find(|e| e.exp_type == exp_type)
            .ok_or_else(|| {
                BencherError::ExperimentNotFound(
                    exp_type.to_string(),
                    self.inner_config
                        .xy_experiments
                        .iter()
                        .map(|e| e.exp_type.clone())
                        .collect::<Vec<_>>()
                        .join(","),
                )
            })?
            .clone())
    }

    fn get_xy_experiment_line(&self, code: &str) -> BencherResult<XYExperimentLine> {
        let (label, exp_type) = self
            .db
            .query_row("select experiment_label, experiment_type from experiments where experiment_code = :code",
                       rusqlite::named_params! {":code": code}, |row| Ok((row.get(0).unwrap_or("".to_string()).clone(), row.get(1).unwrap_or("".to_string()).clone())))?;

        self.get_xy_experiment(&exp_type).map(|e| XYExperimentLine {
            experiment: e,
            label,
            code: code.to_string(),
        })
    }

    fn get_xy_experiment_lines(&self, exp_type: &str) -> BencherResult<Vec<XYExperimentLine>> {
        let experiment = self.get_xy_experiment(exp_type)?;

        let mut stmt = self
            .db
            .prepare("select experiment_code, experiment_label from experiments where experiment_type = :type")?;

        let mut vec = vec![];
        for line in stmt.query_map(rusqlite::named_params! {":type": exp_type}, |row| {
            Ok(XYExperimentLine {
                experiment: experiment.clone(),
                code: row.get(0).unwrap_or("".to_string()),
                label: row.get(1).unwrap_or("".to_string()),
            })
        })? {
            vec.push(line.unwrap());
        }

        Ok(vec)
    }

    pub fn xy_experiments(&self) -> Vec<XYExperiment> {
        self.inner_config.xy_experiments.clone()
    }

    pub fn xy_experiment_lines(&self) -> BencherResult<Vec<XYExperimentLine>> {
        let mut stmt = self.db.prepare(
            "select experiment_type, experiment_label, experiment_code from experiments",
        )?;

        let mut vec = vec![];
        for exp_info in stmt.query_map([], |row| {
            Ok((
                row.get(0).unwrap_or("".to_string()),
                row.get(1).unwrap_or("".to_string()),
                row.get(2).unwrap_or("".to_string()),
            ))
        })? {
            let (exp_type, label, code) = exp_info.unwrap();
            if let Ok(experiment) = self.get_xy_experiment(&exp_type) {
                vec.push(XYExperimentLine {
                    experiment,
                    label,
                    code,
                });
            }
        }

        Ok(vec)
    }

    pub fn get_xy_experiment_handle(
        &'a self,
        exp_type: &str,
    ) -> BencherResult<XYExperimentHandle<'a>> {
        Ok(XYExperimentHandle::new(
            &self.db,
            self.get_xy_experiment(exp_type)?,
            self.get_xy_experiment_lines(exp_type)?,
        )?)
    }

    pub fn get_xy_line_handle(&'a self, code: &str) -> Option<XYLineHandle<'a>> {
        self.get_xy_experiment_line(code)
            .map(|exp_line| XYLineHandle::new(&self.db, exp_line))
            .ok()
    }

    pub fn status(&self) -> BencherResult<Vec<ExperimentStatus>> {
        let mut map = BTreeMap::new();

        let mut stmt = self.db.prepare(
            "select experiment_code, experiment_label, experiment_type from experiments",
        )?;
        for status in stmt.query_map([], |row| {
            Ok(ExperimentStatus {
                code: row.get(0).unwrap_or("".to_string()),
                label: row.get(1).unwrap_or("".to_string()),
                exp_type: row.get(2).unwrap_or("".to_string()),
                n_datapoints: 0,
                n_active_datapoints: 0,
            })
        })? {
            let status = status.unwrap();
            map.insert(status.code.clone(), status);
        }

        let mut stmt = self
            .db
            .prepare("select experiment_code, count(*) from xy_results union select experiment_code, count(*) from linear_results group by experiment_code")?;
        for status in stmt.query_map([], |row| {
            Ok((
                row.get(0).unwrap_or("".to_string()),
                row.get(1).unwrap_or(0),
            ))
        })? {
            let (code, n_datapoints) = status.unwrap();
            map.get_mut(&code).map(|s| s.n_datapoints = n_datapoints);
        }

        let mut stmt = self
            .db
            .prepare("select experiment_code, tag, max(version) from xy_results")?;
        for code in stmt.query_map([], |row| Ok(row.get(0).unwrap_or("".to_string())))? {
            map.get_mut(&code.unwrap())
                .map(|s| s.n_active_datapoints += 1);
        }

        let mut stmt = self
            .db
            .prepare("select experiment_code, v_group, max(version) from linear_results group by experiment_code, v_group")?;
        for code in stmt.query_map([], |row| Ok(row.get(0).unwrap_or("".to_string())))? {
            map.get_mut(&code.unwrap())
                .map(|s| s.n_active_datapoints += 1);
        }

        Ok(map.into_iter().map(|(_, v)| v).collect())
    }

    pub fn has_xy_experiment(&self, exp_type: &str) -> bool {
        self.inner_config
            .xy_experiments
            .iter()
            .find(|e| e.exp_type == exp_type)
            .is_some()
    }

    pub fn has_linear_experiment(&self, exp_type: &str) -> bool {
        self.inner_config
            .linear_experiments
            .iter()
            .find(|e| e.exp_type == exp_type)
            .is_some()
    }

    pub fn has_experiment(&self, exp_type: &str) -> bool {
        self.has_xy_experiment(exp_type) || self.has_linear_experiment(exp_type)
    }
}

pub struct LinearExperimentHandle<'a> {
    db: &'a rusqlite::Connection,
    sets: Vec<LinearExperimentSet>,
    horizontal_label: String,
    v_label: String,
    v_units: String,
}

pub struct LinearSetHandle<'a> {
    db: &'a rusqlite::Connection,
    experiment_set: LinearExperimentSet,
}

fn linear_datapoint_from_row(row: &rusqlite::Row) -> Result<LinearDatapoint, rusqlite::Error> {
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

    let mut datapoint = LinearDatapoint::new(
        row.get::<usize, String>(0).unwrap(),
        Value::new(row.get(1).unwrap(), row.get(2).unwrap())?,
    );

    // x 1 - 99
    if let Some(e) = create_confidence_arg(
        row.get(3).unwrap(),
        row.get(4).unwrap(),
        row.get(5).unwrap(),
        row.get(6).unwrap(),
    ) {
        let _ = datapoint.add_confidence(1, e);
    }

    // x 5 - 95
    if let Some(e) = create_confidence_arg(
        row.get(7).unwrap(),
        row.get(8).unwrap(),
        row.get(9).unwrap(),
        row.get(10).unwrap(),
    ) {
        let _ = datapoint.add_confidence(5, e);
    }

    // x 10 - 90
    if let Some(e) = create_confidence_arg(
        row.get(11).unwrap(),
        row.get(12).unwrap(),
        row.get(13).unwrap(),
        row.get(14).unwrap(),
    ) {
        let _ = datapoint.add_confidence(10, e);
    }

    // x 25 - 75
    if let Some(e) = create_confidence_arg(
        row.get(15).unwrap(),
        row.get(16).unwrap(),
        row.get(17).unwrap(),
        row.get(18).unwrap(),
    ) {
        let _ = datapoint.add_confidence(10, e);
    }

    Ok(datapoint)
}

fn get_linear_datapoints(
    db: &rusqlite::Connection,
    code: &str,
) -> BencherResult<Vec<LinearDatapoint>> {
    let mut stmt = db.prepare(
        "select v_group, v_int, v_float,
                v_int_1,    v_int_99,
                v_float_1,  v_float_99,

                v_int_5,    v_int_95,
                v_float_5,  v_float_95,

                v_int_10,   v_int_90,
                v_float_10, v_float_90,

                v_int_25,   v_int_75,
                v_float_25, v_float_75,

                max(version)
         from linear_results
         where experiment_code = :code
         group by v_group
         ",
    )?;

    let mut vec = vec![];
    for datapoint in stmt.query_map(
        rusqlite::named_params! { ":code": code },
        linear_datapoint_from_row,
    )? {
        vec.push(datapoint?);
    }

    vec.sort_by_key(|d| d.group.clone());
    Ok(vec)
}

impl<'a> LinearExperimentHandle<'a> {
    fn new(
        db: &'a rusqlite::Connection,
        experiment: LinearExperiment,
        sets: Vec<LinearExperimentSet>,
    ) -> BencherResult<Self> {
        if sets.len() == 0 {
            Err(BencherError::NoLines(experiment.exp_type))
        } else {
            let v_label = experiment.v_label.clone();
            let v_units = experiment.v_units.clone();
            let horizontal_label = experiment.horizontal_label.clone();
            Ok(Self {
                db,
                sets,
                horizontal_label,
                v_label,
                v_units,
            })
        }
    }

    fn get_set_datapoints(&self) -> BencherResult<BTreeMap<String, Vec<LinearDatapoint>>> {
        let mut map = BTreeMap::new();
        for exp in &self.sets {
            let datapoints = get_linear_datapoints(self.db, &exp.code)?;
            map.insert(exp.label.clone(), datapoints);
        }

        Ok(map)
    }

    /// This function returns a mapping from set_label -> datapoints and the magnitude to normalize
    /// them with
    fn get_set_datapoints_magnitude(
        &self,
    ) -> BencherResult<(BTreeMap<String, Vec<LinearDatapoint>>, Magnitude)> {
        let set_datapoints = self.get_set_datapoints()?;
        let mut magnitude_counts = [0; 7];

        set_datapoints.values().for_each(|v| {
            v.iter().for_each(|d| match d.magnitude() {
                Magnitude::Nano => magnitude_counts[0] += 1,
                Magnitude::Micro => magnitude_counts[1] += 1,
                Magnitude::Mili => magnitude_counts[2] += 1,
                Magnitude::Normal => magnitude_counts[3] += 1,
                Magnitude::Kilo => magnitude_counts[4] += 1,
                Magnitude::Mega => magnitude_counts[5] += 1,
                Magnitude::Giga => magnitude_counts[6] += 1,
            })
        });

        let idx = magnitude_counts
            .iter()
            .enumerate()
            .max_by_key(|v| v.1)
            .map(|(idx, c)| if *c > 0 { idx } else { 3 })
            .unwrap();

        let mag = match idx {
            0 => Magnitude::Nano,
            1 => Magnitude::Micro,
            2 => Magnitude::Mili,
            3 => Magnitude::Normal,
            4 => Magnitude::Kilo,
            5 => Magnitude::Mega,
            _ => Magnitude::Giga,
        };

        Ok((set_datapoints, mag))
    }

    pub fn dump_table(&self) -> BencherResult<()> {
        let (set_datapoints, mag) = self.get_set_datapoints_magnitude()?;

        for (label, datapoints) in set_datapoints {
            println!(">> {} <<", label);
            let table = datapoints
                .into_iter()
                .map(|d| {
                    vec![
                        d.group.cell().justify(Justify::Right),
                        d.v.display_with_magnitude(mag)
                            .cell()
                            .justify(Justify::Right),
                    ]
                })
                .collect::<Vec<_>>()
                .table()
                .title(vec![
                    label.cell().justify(Justify::Center).bold(true),
                    format!("{} ({}{})", self.v_label, mag.prefix(), self.v_units)
                        .cell()
                        .justify(Justify::Center)
                        .bold(true),
                ])
                .bold(true);

            cli_table::print_stdout(table)?;
        }
        Ok(())
    }

    pub fn dump_latex_table(&self) -> BencherResult<()> {
        let (set_datapoints, mag) = self.get_set_datapoints_magnitude()?;
        for (label, datapoints) in set_datapoints {
            println!("\\begin{{table}}[t]\n    \\centering\n    \\begin{{tabular}}{{|r|r|}}\n        \\hline");
            println!(
                "        \\textbf{{ {} }} & \\textbf{{ {} ({}{}) }} \\\\ \\hline",
                label,
                self.v_label,
                mag.prefix(),
                self.v_units,
            );
            for d in datapoints {
                println!(
                    "        ${:>8}$ & ${:>8}$ \\\\ \\hline",
                    d.group,
                    d.v.display_with_magnitude(mag)
                )
            }
            println!(
                "    \\end{{tabular}}\n    \\caption{{Caption: {0}}}\\label{{table:{0}}}\n\\end{{table}}", label
            );
        }
        Ok(())
    }

    pub fn dump_gnuplot(&self, prefix: &str, bar: bool) -> BencherResult<()> {
        let (set_datapoints, mag) = self.get_set_datapoints_magnitude()?;
        println!(
            "reset

set terminal postscript eps colour size 12cm,8cm enhanced font 'Helvetica,20'
set output '{}.eps'

set border linewidth 0.75
set key outside above
set style data histogram
",
            prefix
        );

        if bar {
            println!("set style histogram cluster gap 1 errorbars lw 2");
        } else {
            println!("set style histogram cluster gap 1");
        }

        println!(
            "
# set axis
set style fill pattern 4 border rgb \"black\"
set auto x
set yrange [0:*]
set ylabel '{} ({}{})'
",
            self.v_label,
            mag.prefix(),
            self.v_units,
        );

        let n_sets = set_datapoints.len();

        if bar {
            println!(
                "plot for [i=2:{}:3] '{}.dat' using i:i+1:i+2:xtic(1) title col(i)",
                2 + 3 * (n_sets - 1),
                prefix
            );
        } else {
            println!(
                "plot for [i=2:{}:1] '{}.dat' using i:xtic(1) title col(i)",
                2 + n_sets - 1,
                prefix
            );
        }

        Ok(())
    }

    pub fn dump_dat(&self, prefix: &str, bar: Option<usize>) -> BencherResult<()> {
        let (set_datapoints, mag) = self.get_set_datapoints_magnitude()?;

        let mut file = File::create(format!("{}.dat", prefix))?;
        let confidence_str = bar
            .map(|c| {
                format!(
                    "confidence interval: {}% - {}%",
                    std::cmp::min(c, 100 - c),
                    std::cmp::max(c, 100 - c)
                )
            })
            .unwrap_or("".to_owned());
        writeln!(&mut file, "#begin {} {}", prefix, confidence_str)?;

        // table: mapping from group to values, ordered by what it matters
        let mut group_values = BTreeMap::new();

        // header
        write!(
            &mut file,
            "{:>34} ",
            format!("\"{}\"", self.horizontal_label)
        )?;

        for (label, datapoints) in set_datapoints {
            write!(&mut file, "{:>34} ", format!("\"{}\"", label))?;
            if bar.is_some() {
                write!(&mut file, "{:>34} ", "\"min\"")?;
                write!(&mut file, "{:>34} ", "\"max\"")?;
            }

            for datapoint in datapoints {
                let guard = group_values
                    .entry(datapoint.group.clone())
                    .or_insert(vec![]);
                guard.push(datapoint.v.display_with_magnitude(mag));
                if let Some(confidence) = bar {
                    let (min, max) = datapoint
                        .get_confidence(confidence)
                        .unwrap_or((datapoint.v.clone(), datapoint.v.clone()));
                    guard.push(min.display_with_magnitude(mag));
                    guard.push(max.display_with_magnitude(mag));
                }
            }
        }

        for (group, values) in group_values {
            write!(&mut file, "\n{:>34} ", format!("\"{}\"", group))?;
            for v in values {
                write!(&mut file, "{:>34} ", v)?;
            }
        }

        writeln!(&mut file, "\n#end")?;
        Ok(())
    }
}

impl<'a> LinearSetHandle<'a> {
    fn new(db: &'a rusqlite::Connection, experiment_set: LinearExperimentSet) -> Self {
        Self { db, experiment_set }
    }

    pub fn label(&'a self) -> &'a str {
        self.experiment_set.label.as_ref()
    }

    fn get_new_version(&self, datapoint: &LinearDatapoint) -> BencherResult<isize> {
        let new_version = self.db.query_row(
                "select max(abs(version)) + 1 from linear_results where experiment_code = :code and v_group = :v_group",
            rusqlite::named_params! { ":code": self.experiment_set.code, ":v_group": datapoint.group },
            |row| Ok(row.get(0).unwrap_or(1)),
        )?;

        Ok(new_version)
    }

    pub fn add_datapoint(&self, datapoint: LinearDatapoint) -> BencherResult<()> {
        let version = self.get_new_version(&datapoint)?;
        let mut stmt = self.db.prepare(
            "insert into linear_results (
                    experiment_code,
                    version,
                    v_group,

                    v_int,
                    v_int_1,
                    v_int_5,
                    v_int_10,
                    v_int_25,
                    v_int_99,
                    v_int_95,
                    v_int_90,
                    v_int_75,

                    v_float,
                    v_float_1,
                    v_float_5,
                    v_float_10,
                    v_float_25,
                    v_float_99,
                    v_float_95,
                    v_float_90,
                    v_float_75
                ) values (
                    :experiment_code,
                    :version,
                    :v_group,

                    :v_int,
                    :v_int_1,
                    :v_int_5,
                    :v_int_10,
                    :v_int_25,
                    :v_int_99,
                    :v_int_95,
                    :v_int_90,
                    :v_int_75,

                    :v_float,
                    :v_float_1,
                    :v_float_5,
                    :v_float_10,
                    :v_float_25,
                    :v_float_99,
                    :v_float_95,
                    :v_float_90,
                    :v_float_75
                )",
        )?;

        stmt.execute(rusqlite::named_params! {
            ":experiment_code": self.experiment_set.code,
            ":v_group": datapoint.group,
            ":version": version,

            ":v_int": datapoint.v.to_int(),
            ":v_float": datapoint.v.to_float(),

            ":v_int_1": datapoint.get_confidence(1).clone().map(|val| val.0.to_int()).flatten(),
            ":v_int_99": datapoint.get_confidence(1).clone().map(|val| val.1.to_int()).flatten(),

            ":v_int_5": datapoint.get_confidence(5).clone().map(|val| val.0.to_int()).flatten(),
            ":v_int_95": datapoint.get_confidence(5).clone().map(|val| val.1.to_int()).flatten(),

            ":v_int_10": datapoint.get_confidence(10).clone().map(|val| val.0.to_int()).flatten(),
            ":v_int_90": datapoint.get_confidence(10).clone().map(|val| val.1.to_int()).flatten(),

            ":v_int_25": datapoint.get_confidence(25).clone().map(|val| val.0.to_int()).flatten(),
            ":v_int_75": datapoint.get_confidence(25).clone().map(|val| val.1.to_int()).flatten(),

            ":v_float_1": datapoint.get_confidence(1).clone().map(|val| val.0.to_float()).flatten(),
            ":v_float_99": datapoint.get_confidence(1).clone().map(|val| val.1.to_float()).flatten(),

            ":v_float_5": datapoint.get_confidence(5).clone().map(|val| val.0.to_float()).flatten(),
            ":v_float_95": datapoint.get_confidence(5).clone().map(|val| val.1.to_float()).flatten(),

            ":v_float_10": datapoint.get_confidence(10).clone().map(|val| val.0.to_float()).flatten(),
            ":v_float_90": datapoint.get_confidence(10).clone().map(|val| val.1.to_float()).flatten(),

            ":v_float_25": datapoint.get_confidence(25).clone().map(|val| val.0.to_float()).flatten(),
            ":v_float_75": datapoint.get_confidence(25).clone().map(|val| val.1.to_float()).flatten(),
        })?;
        Ok(())
    }

    pub fn dump_raw(&self, group: &str) -> BencherResult<()> {
        let datapoint = self.db.query_row(
            "select v_group, v_int, v_float,
                v_int_1,    v_int_99,
                v_float_1,  v_float_99,

                v_int_5,    v_int_95,
                v_float_5,  v_float_95,

                v_int_10,   v_int_90,
                v_float_10, v_float_90,

                v_int_25,   v_int_75,
                v_float_25, v_float_75,

                max(version)
         from linear_results
         where experiment_code = :code and v_group = :v_group",
            rusqlite::named_params! { ":code": self.experiment_set.code, ":v_group": group },
            linear_datapoint_from_row,
        )?;

        println!(
            "{:>3}\t{}",
            50,
            datapoint.v.display_with_magnitude(Magnitude::Normal)
        );
        for confidence in &[1, 5, 10, 25] {
            if let Some((min, max)) = datapoint.get_confidence(*confidence) {
                println!(
                    "{:>3}\t{}",
                    confidence,
                    min.display_with_magnitude(Magnitude::Normal)
                );
                println!(
                    "{:>3}\t{}",
                    100 - confidence,
                    max.display_with_magnitude(Magnitude::Normal)
                );
            }
        }
        Ok(())
    }

    pub fn version(&self, group: &str) -> BencherResult<usize> {
        Ok(self.db.query_row(
            "select abs(max(version)) from linear_results where experiment_code = :code and v_group = :v_group",
            rusqlite::named_params! { ":code": self.experiment_set.code, ":v_group": group },
            |row| row.get(0),
        )?)
    }

    pub fn versions(&self, group: &str) -> BencherResult<Vec<usize>> {
        let mut stmt = self.db.prepare(
            "select abs(version) from linear_results where experiment_code = :code and v_group = :v_group",
        )?;

        let result = stmt.query_map(
            rusqlite::named_params! { ":code": self.experiment_set.code, ":v_group": group },
            |row| Ok(row.get(0).unwrap_or(0)),
        )?;

        Ok(result.into_iter().map(|x| x.unwrap()).collect())
    }

    pub fn revert(&self, group: &str, version: Option<usize>) -> BencherResult<()> {
        if let Some(v) = version {
            self.db.execute("update linear_results set version = abs(version) where experiment_code = :code and v_group = :v_group and abs(version) = :version",
                            rusqlite::named_params! { ":code": self.experiment_set.code, ":v_group": group, ":version": v})?;
            self.db.execute("update linear_results set version = -version where experiment_code = :code and v_group = :v_group and version > :version",
                            rusqlite::named_params! { ":code": self.experiment_set.code, ":v_group": group, ":version": v})?;
        } else {
            self.db.execute("update linear_results set version = -version where experiment_code = :code and v_group = :v_group and version in
                            (select max(version) from linear_results where experiment_code = :code and v_group = :v_group)",
                            rusqlite::named_params! { ":code": self.experiment_set.code, ":v_group": group })?;
        }
        Ok(())
    }
}

pub struct XYExperimentHandle<'a> {
    db: &'a rusqlite::Connection,
    lines: Vec<XYExperimentLine>,
    x_label: String,
    x_units: String,
    y_label: String,
    y_units: String,
}

pub struct XYLineHandle<'a> {
    db: &'a rusqlite::Connection,
    experiment_line: XYExperimentLine,
}

fn xy_datapoint_from_row(row: &rusqlite::Row) -> Result<XYDatapoint, rusqlite::Error> {
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

    let mut datapoint = XYDatapoint::new(
        Value::new(row.get(0).unwrap(), row.get(1).unwrap())?,
        Value::new(row.get(2).unwrap(), row.get(3).unwrap())?,
    );

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

    Ok(if let Some(tag) = row.get(36).unwrap() {
        datapoint.tag(tag)
    } else {
        datapoint
    })
}

fn get_xy_datapoints(db: &rusqlite::Connection, code: &str) -> BencherResult<Vec<XYDatapoint>> {
    let mut stmt = db.prepare(
        "select x_int, x_float,
                y_int, y_float,
                x_int_1,    x_int_99,
                x_float_1,  x_float_99,

                x_int_5,    x_int_95,
                x_float_5,  x_float_95,

                x_int_10,   x_int_90,
                x_float_10, x_float_90,

                x_int_25,   x_int_75,
                x_float_25, x_float_75,

                y_int_1,    y_int_99,
                y_float_1,  y_float_99,

                y_int_5,    y_int_95,
                y_float_5,  y_float_95,

                y_int_10,   y_int_90,
                y_float_10, y_float_90,

                y_int_25,   y_int_75,
                y_float_25, y_float_75,

                tag, max(version)
         from xy_results
         where experiment_code = :code
         group by tag
         ",
    )?;

    let mut vec = vec![];
    for datapoint in stmt.query_map(
        rusqlite::named_params! { ":code": code },
        xy_datapoint_from_row,
    )? {
        vec.push(datapoint?);
    }

    vec.sort_by_key(|d| d.tag);
    Ok(vec)
}

impl<'a> XYExperimentHandle<'a> {
    fn new(
        db: &'a rusqlite::Connection,
        experiment: XYExperiment,
        lines: Vec<XYExperimentLine>,
    ) -> BencherResult<Self> {
        if lines.len() == 0 {
            Err(BencherError::NoLines(experiment.exp_type))
        } else {
            let x_label = experiment.x_label.clone();
            let x_units = experiment.x_units.clone();
            let y_label = experiment.y_label.clone();
            let y_units = experiment.y_units.clone();
            Ok(Self {
                db,
                lines,
                x_label,
                x_units,
                y_label,
                y_units,
            })
        }
    }

    fn get_datapoints(&self) -> BencherResult<BTreeMap<String, Vec<XYDatapoint>>> {
        let mut map = BTreeMap::new();
        for exp in &self.lines {
            let datapoints = get_xy_datapoints(self.db, &exp.code)?;
            map.insert(exp.label.clone(), datapoints);
        }

        Ok(map)
    }

    fn get_datapoints_magnitudes(
        &self,
    ) -> BencherResult<(BTreeMap<String, Vec<XYDatapoint>>, Magnitude, Magnitude)> {
        let datapoints = self.get_datapoints()?;
        let mut x_magnitude_counts = [0; 7];
        let mut y_magnitude_counts = [0; 7];

        datapoints.values().for_each(|v| {
            v.iter().for_each(|d| {
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
            })
        });

        let x_idx = x_magnitude_counts
            .iter()
            .enumerate()
            .max_by_key(|v| v.1)
            .map(|(idx, c)| if *c > 0 { idx } else { 3 })
            .unwrap();

        let y_idx = y_magnitude_counts
            .iter()
            .enumerate()
            .max_by_key(|v| v.1)
            .map(|(idx, c)| if *c > 0 { idx } else { 3 })
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

    pub fn dump_table(&self) -> BencherResult<()> {
        let (datapoints, x_mag, y_mag) = self.get_datapoints_magnitudes()?;

        for (label, datapoints) in datapoints {
            let table = datapoints
                .into_iter()
                .map(|d| {
                    vec![
                        d.tag.unwrap().cell().justify(Justify::Right),
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
                    "Tag".cell().justify(Justify::Center).bold(true),
                    format!("{} ({}{})", self.x_label, x_mag.prefix(), self.x_units)
                        .cell()
                        .justify(Justify::Center)
                        .bold(true),
                    format!("{} ({}{})", self.y_label, y_mag.prefix(), self.y_units)
                        .cell()
                        .justify(Justify::Center)
                        .bold(true),
                ])
                .bold(true);

            println!(">> {} <<", label);
            cli_table::print_stdout(table)?;
        }
        Ok(())
    }

    pub fn dump_latex_table(&self) -> BencherResult<()> {
        let (datapoints, x_mag, y_mag) = self.get_datapoints_magnitudes()?;
        for (label, datapoints) in datapoints {
            println!("\\begin{{table}}[t]\n    \\centering\n    \\begin{{tabular}}{{|r|r|}}\n        \\hline");
            println!(
                "        \\textbf{{ {} ({}{}) }} & \\textbf{{ {} ({}{}) }} \\\\ \\hline",
                self.x_label,
                x_mag.prefix(),
                self.x_units,
                self.y_label,
                y_mag.prefix(),
                self.y_units
            );
            for d in datapoints {
                println!(
                    "        ${:>8}$ & ${:>8}$ \\\\ \\hline",
                    d.x.display_with_magnitude(x_mag),
                    d.y.display_with_magnitude(y_mag)
                )
            }
            println!(
                "    \\end{{tabular}}\n    \\caption{{Caption: {0}}}\\label{{table:{0}}}\n\\end{{table}}", label
            );
        }
        Ok(())
    }

    pub fn dump_gnuplot(&self, prefix: &str, xbar: bool, ybar: bool) -> BencherResult<()> {
        let (datapoints, x_mag, y_mag) = self.get_datapoints_magnitudes()?;
        println!(
            "reset

set terminal postscript eps colour size 12cm,8cm enhanced font 'Helvetica,20'
set output '{}.eps'

set border linewidth 0.75
set key outside above",
            prefix
        );

        for (idx, _) in datapoints.iter().enumerate() {
            println!(
"# Set color of linestyle {0} to #{3}
set style line {0} linecolor rgb '#{3}' linetype 2 linewidth 2.5 pointtype {2} pointsize 2 dashtype 2
# Set yerror color of linestyle {1} to #{2}
set style line {1} linecolor rgb '#{3}' linetype 2 linewidth 2.5 pointtype {2} pointsize 2",
                2 * idx + 1,
                2 * idx + 2,
                2 * idx + 4,
                COLORS[idx % COLORS.len()]
                )
        }

        println!(
            "

# set axis
set tics scale 0.75
set xlabel '{} ({}{})'
set ylabel '{} ({}{})'
set xrange [*:*]
set yrange [*:*]
",
            self.x_label,
            x_mag.prefix(),
            self.x_units,
            self.y_label,
            y_mag.prefix(),
            self.y_units,
        );

        println!(
            "plot {}",
            datapoints
                .iter()
                .enumerate()
                .map(|(idx, (label, _))| match (xbar, ybar) {
                    (true, true) => format!(
                        "'{}_{}.dat' title '{}' with xyerrorbars linestyle {}, '' title '' with lines linestyle {}",
                        prefix,
                        label.to_lowercase(),
                        label,
                        2 * idx + 2,
                        2 * idx + 1,
                    ),
                    (true, false) => format!(
                        "'{}_{}.dat' title '{}' with xerrorbars linestyle {}, '' title '' with lines linestyle {}",
                        prefix,
                        label.to_lowercase(),
                        label,
                        2 * idx + 2,
                        2 * idx + 1,
                    ),
                    (false, true) => format!(
                        "'{}_{}.dat' title '{}' with yerrorbars linestyle {}, '' title '' with lines linestyle {}",
                        prefix,
                        label.to_lowercase(),
                        label,
                        2 * idx + 2,
                        2 * idx + 1,
                    ),
                    (false, false) => format!(
                        "'{}_{}.dat' title '{}' with linespoint linestyle {}",
                        prefix,
                        label.to_lowercase(),
                        label,
                        2 * idx + 1
                    ),
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(())
    }

    pub fn dump_dat(
        &self,
        prefix: &str,
        xbar: Option<usize>,
        ybar: Option<usize>,
    ) -> BencherResult<()> {
        let (datapoints, x_mag, y_mag) = self.get_datapoints_magnitudes()?;

        for (label, datapoints) in datapoints {
            let mut file = File::create(format!("{}_{}.dat", prefix, label.to_lowercase()))?;
            writeln!(
                &mut file,
                "# {}\n# x axis: {} ({}{})\n# y axis: {} ({}{})\n",
                label,
                self.x_label,
                x_mag.prefix(),
                self.x_units,
                self.y_label,
                y_mag.prefix(),
                self.y_units
            )?;
            for d in datapoints {
                write!(
                    &mut file,
                    "{:>8} {:>8}",
                    d.x.display_with_magnitude(x_mag),
                    d.y.display_with_magnitude(y_mag)
                )?;

                if let Some(c) = xbar {
                    let (xmin, xmax) = d.get_x_confidence(c).unwrap_or((d.x.clone(), d.x.clone()));
                    write!(
                        &mut file,
                        " {:>8} {:>8}",
                        xmin.display_with_magnitude(x_mag),
                        xmax.display_with_magnitude(x_mag)
                    )?;
                }

                if let Some(c) = ybar {
                    let (ymin, ymax) = d.get_y_confidence(c).unwrap_or((d.y.clone(), d.y.clone()));
                    write!(
                        &mut file,
                        " {:>8} {:>8}",
                        ymin.display_with_magnitude(y_mag),
                        ymax.display_with_magnitude(y_mag)
                    )?;
                }

                writeln!(&mut file, "")?;
            }
            writeln!(&mut file, "\n# end")?;
        }
        Ok(())
    }
}

impl<'a> XYLineHandle<'a> {
    fn new(db: &'a rusqlite::Connection, experiment_line: XYExperimentLine) -> Self {
        Self {
            db,
            experiment_line,
        }
    }

    fn tag_datapoint(&self, datapoint: XYDatapoint) -> BencherResult<(XYDatapoint, isize)> {
        if datapoint.tag.is_some() {
            let new_version = self.db.query_row(
                    "select max(abs(version)) + 1 from xy_results where experiment_code = :code and tag = :tag",
                rusqlite::named_params! { ":code": self.experiment_line.code, ":tag": datapoint.tag.unwrap() },
                |row| Ok(row.get(0).unwrap_or(1)),
            )?;

            return Ok((datapoint, new_version));
        }

        let new_tag = self.db.query_row(
            "select max(tag) + 1 from xy_results where experiment_code = :code",
            rusqlite::named_params! { ":code": self.experiment_line.code },
            |row| Ok(row.get(0).unwrap_or(0)),
        )?;

        Ok((datapoint.tag(new_tag), 0))
    }

    pub fn label(&'a self) -> &'a str {
        self.experiment_line.label.as_ref()
    }

    pub fn add_datapoint(&self, datapoint: XYDatapoint) -> BencherResult<()> {
        let (datapoint, version) = self.tag_datapoint(datapoint)?;
        let mut stmt = self.db.prepare(
            "insert into xy_results (
                    experiment_code,
                    tag,
                    version,

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
                    y_int_75,

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
                    :experiment_code,
                    :tag,
                    :version,

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
                    :y_int_75,

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
            ":experiment_code": self.experiment_line.code,
            ":tag": datapoint.tag.unwrap(),
            ":version": version,

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

    pub fn dump_raw(&self, tag: isize, axis: Axis) -> BencherResult<()> {
        let datapoint = self.db.query_row(
            "select x_int, x_float,
                y_int, y_float,
                x_int_1,    x_int_99,
                x_float_1,  x_float_99,

                x_int_5,    x_int_95,
                x_float_5,  x_float_95,

                x_int_10,   x_int_90,
                x_float_10, x_float_90,

                x_int_25,   x_int_75,
                x_float_25, x_float_75,

                y_int_1,    y_int_99,
                y_float_1,  y_float_99,

                y_int_5,    y_int_95,
                y_float_5,  y_float_95,

                y_int_10,   y_int_90,
                y_float_10, y_float_90,

                y_int_25,   y_int_75,
                y_float_25, y_float_75,

                tag, max(version)
         from xy_results
         where experiment_code = :code and tag = :tag",
            rusqlite::named_params! { ":code": self.experiment_line.code, ":tag": tag },
            xy_datapoint_from_row,
        )?;

        match axis {
            Axis::X => {
                println!(
                    "{:>3}\t{}",
                    50,
                    datapoint.x.display_with_magnitude(Magnitude::Normal)
                );
                for confidence in &[1, 5, 10, 25] {
                    if let Some((min, max)) = datapoint.get_x_confidence(*confidence) {
                        println!(
                            "{:>3}\t{}",
                            confidence,
                            min.display_with_magnitude(Magnitude::Normal)
                        );
                        println!(
                            "{:>3}\t{}",
                            100 - confidence,
                            max.display_with_magnitude(Magnitude::Normal)
                        );
                    }
                }
            }
            Axis::Y => {
                println!(
                    "{:>3}\t{}",
                    50,
                    datapoint.y.display_with_magnitude(Magnitude::Normal)
                );
                for confidence in &[1, 5, 10, 25] {
                    if let Some((min, max)) = datapoint.get_y_confidence(*confidence) {
                        println!(
                            "{:>3}\t{}",
                            confidence,
                            min.display_with_magnitude(Magnitude::Normal)
                        );
                        println!(
                            "{:>3}\t{}",
                            100 - confidence,
                            max.display_with_magnitude(Magnitude::Normal)
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub fn version(&self, tag: isize) -> BencherResult<usize> {
        Ok(self.db.query_row(
            "select abs(max(version)) from xy_results where experiment_code = :code and tag = :tag",
            rusqlite::named_params! { ":code": self.experiment_line.code, ":tag": tag },
            |row| row.get(0),
        )?)
    }

    pub fn versions(&self, tag: isize) -> BencherResult<Vec<usize>> {
        let mut stmt = self.db.prepare(
            "select abs(version) from xy_results where experiment_code = :code and tag = :tag",
        )?;

        let result = stmt.query_map(
            rusqlite::named_params! { ":code": self.experiment_line.code, ":tag": tag },
            |row| Ok(row.get(0).unwrap_or(0)),
        )?;

        Ok(result.into_iter().map(|x| x.unwrap()).collect())
    }

    pub fn revert(&self, tag: isize, version: Option<usize>) -> BencherResult<()> {
        if let Some(v) = version {
            self.db.execute("update xy_results set version = abs(version) where experiment_code = :code and tag = :tag and abs(version) = :version",
                            rusqlite::named_params! { ":code": self.experiment_line.code, ":tag": tag, ":version": v})?;
            self.db.execute("update xy_results set version = -version where experiment_code = :code and tag = :tag and version > :version",
                            rusqlite::named_params! { ":code": self.experiment_line.code, ":tag": tag, ":version": v})?;
        } else {
            self.db.execute("update xy_results set version = -version where experiment_code = :code and tag = :tag and version in
                            (select max(version) from xy_results where experiment_code = :code and tag = :tag)",
                            rusqlite::named_params! { ":code": self.experiment_line.code, ":tag": tag })?;
        }
        Ok(())
    }
}

fn find_config_dir() -> BencherResult<PathBuf> {
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

fn open_db(db_path: &Path) -> BencherResult<rusqlite::Connection> {
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX
        | rusqlite::OpenFlags::SQLITE_OPEN_CREATE;

    match rusqlite::Connection::open_with_flags(db_path, flags) {
        Ok(conn) => Ok(conn),
        Err(e) => Err(BencherError::Database(e).into()),
    }
}

fn setup_db(db: &rusqlite::Connection) -> BencherResult<()> {
    db.execute(
        "create table if not exists experiments (
            experiment_code text not null primary key,
            experiment_type text not null,
            experiment_label text not null
        )",
        [],
    )?;

    db.execute(
        "create table if not exists xy_results (
            experiment_code text not null,
            tag int not null,
            version int not null,

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

            foreing key experiment_code references experiments,
            primary key (experiment_code, tag, version)
        )",
        [],
    )?;

    db.execute(
        "create table if not exists linear_results (
            experiment_code text not null,
            v_group text not null,
            version int not null,

            v_int int,
            v_int_1 int,
            v_int_5 int,
            v_int_10 int,
            v_int_25 int,
            v_int_99 int,
            v_int_95 int,
            v_int_90 int,
            v_int_75 int,

            v_float float,
            v_float_1 float,
            v_float_5 float,
            v_float_10 float,
            v_float_25 float,
            v_float_99 float,
            v_float_95 float,
            v_float_90 float,
            v_float_75 float,

            foreing key experiment_code references experiments,
            primary key (experiment_code, v_group, version)
        )",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    fn gen_xy_experiments() -> Vec<XYExperiment> {
        vec![
            XYExperiment {
                exp_type: "Throughput Latency".to_string(),
                x_label: "Throughput".to_string(),
                x_units: "ops/s".to_string(),
                y_label: "Latency".to_string(),
                y_units: "s".to_string(),
            },
            XYExperiment {
                exp_type: "Throughput".to_string(),
                x_label: "Offered Load".to_string(),
                x_units: "ops/s".to_string(),
                y_label: "Throughput".to_string(),
                y_units: "ops/s".to_string(),
            },
        ]
    }

    fn gen_linear_experiments() -> Vec<LinearExperiment> {
        vec![
            LinearExperiment {
                exp_type: "Operational Latency".to_string(),
                horizontal_label: "Operation".to_string(),
                v_label: "Latency".to_string(),
                v_units: "s".to_string(),
            },
            LinearExperiment {
                exp_type: "Operational Throughput".to_string(),
                horizontal_label: "Operation".to_string(),
                v_label: "Throughput".to_string(),
                v_units: "ops/s".to_string(),
            },
        ]
    }

    fn gen_inner_config() -> BencherConfig {
        BencherConfig {
            database_filepath: "".to_string(),
            xy_experiments: gen_xy_experiments(),
            linear_experiments: gen_linear_experiments(),
        }
    }

    fn gen_in_memory_config() -> Config {
        let conf = Config::from_conn_and_config(
            rusqlite::Connection::open_in_memory().unwrap(),
            gen_inner_config(),
        )
        .unwrap();

        conf.add_xy_experiment("Throughput Latency", "Write", "tput_lat")
            .unwrap();
        conf.add_xy_experiment("Throughput", "Read", "tput_1")
            .unwrap();
        conf.add_xy_experiment("Throughput", "Write", "tput_2")
            .unwrap();
        conf.add_xy_experiment("Throughput", "Version", "tput_version")
            .unwrap();

        conf.add_linear_experiment("Operational Latency", "System A", "linear_lat_a")
            .unwrap();
        conf.add_linear_experiment("Operational Latency", "System B", "linear_lat_b")
            .unwrap();
        conf.add_linear_experiment("Operational Throughput", "System A", "linear_tput_a")
            .unwrap();
        conf.add_linear_experiment("Operational Throughput", "System B", "linear_tput_b")
            .unwrap();

        conf
    }

    fn xy_populate(handle1: &XYLineHandle, handle2: &XYLineHandle) {
        for x in (0..=100).step_by(10) {
            let y = 100 - x;
            handle1
                .add_datapoint(XYDatapoint::new(Value::Int(x), Value::Int(x * x)))
                .unwrap();
            handle2
                .add_datapoint(XYDatapoint::new(Value::Int(y), Value::Int(10_000 - y * y)))
                .unwrap();
        }
    }

    fn linear_populate(handle: &LinearSetHandle, points: &[(&str, i64)]) {
        for (label, value) in points {
            handle
                .add_datapoint(LinearDatapoint::new(label.to_string(), Value::Int(*value)))
                .unwrap();
        }
    }

    #[test]
    fn config_experiments() {
        let config = gen_in_memory_config();
        assert_eq!(config.xy_experiments(), gen_xy_experiments());
    }

    #[test]
    fn config_empty_status() {
        let config = gen_in_memory_config();
        assert_eq!(
            config.status().unwrap().into_iter().collect::<HashSet<_>>(),
            vec![
                ExperimentStatus {
                    code: "tput_1".to_string(),
                    exp_type: "Throughput".to_string(),
                    label: "Read".to_string(),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                },
                ExperimentStatus {
                    code: "tput_lat".to_string(),
                    exp_type: "Throughput Latency".to_string(),
                    label: "Write".to_string(),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                },
                ExperimentStatus {
                    code: "tput_version".to_string(),
                    exp_type: "Throughput".to_string(),
                    label: "Version".to_string(),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                },
                ExperimentStatus {
                    code: "tput_2".to_string(),
                    exp_type: "Throughput".to_string(),
                    label: "Write".to_string(),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                },
                ExperimentStatus {
                    code: "linear_lat_a".to_string(),
                    exp_type: "Operational Latency".to_string(),
                    label: "System A".to_string(),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                },
                ExperimentStatus {
                    code: "linear_lat_b".to_string(),
                    exp_type: "Operational Latency".to_string(),
                    label: "System B".to_string(),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                },
                ExperimentStatus {
                    code: "linear_tput_a".to_string(),
                    exp_type: "Operational Throughput".to_string(),
                    label: "System A".to_string(),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                },
                ExperimentStatus {
                    code: "linear_tput_b".to_string(),
                    exp_type: "Operational Throughput".to_string(),
                    label: "System B".to_string(),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                },
            ]
            .into_iter()
            .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn linear_config_get_handle() {
        let config = gen_in_memory_config();
        assert_eq!(config.has_experiment("Non-operational Latency"), false);
        assert_eq!(
            config.has_linear_experiment("Non-operational Latency"),
            false
        );
        assert!(config
            .get_linear_experiment_handle("Non-operational Latency")
            .is_err());
        assert_eq!(config.has_experiment("Operational Latency"), true);
        assert_eq!(config.has_linear_experiment("Operational Latency"), true);
        assert_eq!(config.has_xy_experiment("Operational Latency"), false);
        assert!(config
            .get_linear_experiment_handle("Operational Latency")
            .is_ok());
        assert_eq!(config.has_experiment("Operational Throughput"), true);
        assert_eq!(config.has_xy_experiment("Operational Throughput"), false);
        assert!(config
            .get_linear_experiment_handle("Operational Throughput")
            .is_ok());
    }

    #[test]
    fn linear_can_populate() {
        let config = gen_in_memory_config();
        let handle_a = config.get_linear_set_handle("linear_lat_a").unwrap();
        let handle_b = config.get_linear_set_handle("linear_lat_b").unwrap();
        linear_populate(&handle_a, &[("get", 12), ("put", 42)]);
        linear_populate(&handle_b, &[("get", 6), ("put", 20)]);
    }

    #[test]
    fn linear_can_get() {
        let config = gen_in_memory_config();
        let handle_a = config.get_linear_set_handle("linear_lat_a").unwrap();
        let handle_b = config.get_linear_set_handle("linear_lat_b").unwrap();
        linear_populate(&handle_a, &[("get", 12), ("put", 42)]);
        linear_populate(&handle_b, &[("get", 6), ("put", 20)]);

        let handle = config
            .get_linear_experiment_handle("Operational Latency")
            .unwrap();

        let datapoints = handle.get_set_datapoints().unwrap();

        assert_eq!(datapoints.len(), 2);
        eprintln!("{:#?}", datapoints);
        assert_eq!(
            datapoints["System A"],
            vec![
                LinearDatapoint::new("get".to_string(), Value::Int(12)),
                LinearDatapoint::new("put".to_string(), Value::Int(42))
            ]
        );
        assert_eq!(
            datapoints["System B"],
            vec![
                LinearDatapoint::new("get".to_string(), Value::Int(6)),
                LinearDatapoint::new("put".to_string(), Value::Int(20))
            ]
        );

        let (datapoints, mag) = handle.get_set_datapoints_magnitude().unwrap();
        assert_eq!(datapoints.len(), 2);
        assert_eq!(
            datapoints["System A"],
            vec![
                LinearDatapoint::new("get".to_string(), Value::Int(12)),
                LinearDatapoint::new("put".to_string(), Value::Int(42))
            ]
        );
        assert_eq!(
            datapoints["System B"],
            vec![
                LinearDatapoint::new("get".to_string(), Value::Int(6)),
                LinearDatapoint::new("put".to_string(), Value::Int(20))
            ]
        );
        assert_eq!(mag, Magnitude::Normal);
    }

    #[test]
    fn linear_versions() {
        fn gen_datapoint(group: &str, v: i64) -> LinearDatapoint {
            LinearDatapoint::new(group.to_string(), Value::Int(v))
        }

        let config = gen_in_memory_config();

        let handle_a = config.get_linear_set_handle("linear_tput_a").unwrap();
        let handle = config
            .get_linear_experiment_handle("Operational Throughput")
            .unwrap();

        assert!(handle_a.version("get").is_err());

        handle_a.add_datapoint(gen_datapoint("get", 1)).unwrap();
        assert_eq!(
            handle.get_set_datapoints().unwrap()["System A"][0],
            gen_datapoint("get", 1)
        );
        assert_eq!(handle_a.version("get").unwrap(), 1);
        let mut versions = handle_a.versions("get").unwrap();
        versions.sort();
        assert_eq!(versions, vec![1]);

        handle_a.add_datapoint(gen_datapoint("get", 2)).unwrap();
        assert_eq!(
            handle.get_set_datapoints().unwrap()["System A"][0],
            gen_datapoint("get", 2)
        );
        assert_eq!(handle_a.version("get").unwrap(), 2);
        let mut versions = handle_a.versions("get").unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2]);

        handle_a.revert("get", None).unwrap();
        assert_eq!(
            handle.get_set_datapoints().unwrap()["System A"][0],
            gen_datapoint("get", 1)
        );
        assert_eq!(handle_a.version("get").unwrap(), 1);
        let mut versions = handle_a.versions("get").unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2]);

        handle_a.add_datapoint(gen_datapoint("get", 3)).unwrap();
        assert_eq!(
            handle.get_set_datapoints().unwrap()["System A"][0],
            gen_datapoint("get", 3)
        );
        assert_eq!(handle_a.version("get").unwrap(), 3);
        let mut versions = handle_a.versions("get").unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2, 3]);

        handle_a.revert("get", Some(1)).unwrap();
        assert_eq!(
            handle.get_set_datapoints().unwrap()["System A"][0],
            gen_datapoint("get", 1)
        );
        assert_eq!(handle_a.version("get").unwrap(), 1);
        let mut versions = handle_a.versions("get").unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2, 3]);

        handle_a.revert("get", Some(2)).unwrap();
        assert_eq!(
            handle.get_set_datapoints().unwrap()["System A"][0],
            gen_datapoint("get", 2)
        );
        assert_eq!(handle_a.version("get").unwrap(), 2);
        let mut versions = handle_a.versions("get").unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2, 3]);

        handle_a.revert("get", Some(3)).unwrap();
        assert_eq!(
            handle.get_set_datapoints().unwrap()["System A"][0],
            gen_datapoint("get", 3)
        );
        assert_eq!(handle_a.version("get").unwrap(), 3);
        assert_eq!(handle_a.versions("get").unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn xy_config_get_handle() {
        let config = gen_in_memory_config();
        assert_eq!(config.has_experiment("Latency"), false);
        assert_eq!(config.has_xy_experiment("Latency"), false);
        assert!(config.get_xy_experiment_handle("Latency").is_err());
        assert_eq!(config.has_experiment("Throughput"), true);
        assert_eq!(config.has_xy_experiment("Throughput"), true);
        assert_eq!(config.has_linear_experiment("Throughput"), false);
        assert!(config.get_xy_experiment_handle("Throughput").is_ok());
    }

    #[test]
    fn xy_can_populate() {
        let config = gen_in_memory_config();
        let handle1 = config.get_xy_line_handle("tput_1").unwrap();
        let handle2 = config.get_xy_line_handle("tput_2").unwrap();
        xy_populate(&handle1, &handle2);
    }

    #[test]
    fn xy_can_get() {
        let config = gen_in_memory_config();
        let handle1 = config.get_xy_line_handle("tput_1").unwrap();
        let handle2 = config.get_xy_line_handle("tput_2").unwrap();
        xy_populate(&handle1, &handle2);

        let handle = config.get_xy_experiment_handle("Throughput").unwrap();

        let mut v1 = (0..=100)
            .step_by(10)
            .into_iter()
            .map(|x| XYDatapoint::new(Value::Int(x), Value::Int(x * x)).tag(x as isize / 10))
            .collect::<Vec<XYDatapoint>>();
        let mut v2 = (0..=100)
            .step_by(10)
            .into_iter()
            .map(|x| 100 - x)
            .map(|x| {
                XYDatapoint::new(Value::Int(x), Value::Int(10_000 - x * x))
                    .tag((100 - x) as isize / 10)
            })
            .collect::<Vec<XYDatapoint>>();

        let v3 = vec![];

        v1.sort_by_key(|x| x.tag);
        v2.sort_by_key(|x| x.tag);

        let datapoints = handle
            .get_datapoints()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(datapoints.len(), 3);
        assert!(datapoints[0] == v1 || datapoints[0] == v2 || datapoints[0] == v3);
        assert!(datapoints[1] == v1 || datapoints[1] == v2 || datapoints[1] == v3);
        assert!(datapoints[2] == v1 || datapoints[2] == v2 || datapoints[2] == v3);

        let (datapoints, x_mag, y_mag) = handle.get_datapoints_magnitudes().unwrap();
        let datapoints = datapoints.values().cloned().collect::<Vec<_>>();
        assert_eq!(datapoints.len(), 3);
        assert!(datapoints[0] == v1 || datapoints[0] == v2 || datapoints[0] == v3);
        assert!(datapoints[1] == v1 || datapoints[1] == v2 || datapoints[1] == v3);
        assert!(datapoints[2] == v1 || datapoints[2] == v2 || datapoints[2] == v3);
        assert_eq!(x_mag, Magnitude::Normal);
        assert_eq!(y_mag, Magnitude::Kilo);
    }

    #[test]
    fn xy_versions() {
        fn gen_datapoint(v: i64) -> XYDatapoint {
            XYDatapoint::new(Value::Int(v), Value::Int(v)).tag(42)
        }

        let config = gen_in_memory_config();

        let handle = config.get_xy_line_handle("tput_version").unwrap();
        let get_handle = config.get_xy_experiment_handle("Throughput").unwrap();

        assert!(handle.version(42).is_err());

        handle.add_datapoint(gen_datapoint(1)).unwrap();
        assert_eq!(
            get_handle.get_datapoints().unwrap()["Version"][0],
            gen_datapoint(1)
        );
        assert_eq!(handle.version(42).unwrap(), 1);
        let mut versions = handle.versions(42).unwrap();
        versions.sort();
        assert_eq!(versions, vec![1]);

        handle.add_datapoint(gen_datapoint(2)).unwrap();
        assert_eq!(
            get_handle.get_datapoints().unwrap()["Version"][0],
            gen_datapoint(2)
        );
        assert_eq!(handle.version(42).unwrap(), 2);
        let mut versions = handle.versions(42).unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2]);

        handle.revert(42, None).unwrap();
        assert_eq!(
            get_handle.get_datapoints().unwrap()["Version"][0],
            gen_datapoint(1)
        );
        assert_eq!(handle.version(42).unwrap(), 1);
        let mut versions = handle.versions(42).unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2]);

        handle.add_datapoint(gen_datapoint(3)).unwrap();
        assert_eq!(
            get_handle.get_datapoints().unwrap()["Version"][0],
            gen_datapoint(3)
        );
        assert_eq!(handle.version(42).unwrap(), 3);
        let mut versions = handle.versions(42).unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2, 3]);

        handle.revert(42, Some(1)).unwrap();
        assert_eq!(
            get_handle.get_datapoints().unwrap()["Version"][0],
            gen_datapoint(1)
        );
        assert_eq!(handle.version(42).unwrap(), 1);
        let mut versions = handle.versions(42).unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2, 3]);

        handle.revert(42, Some(2)).unwrap();
        assert_eq!(
            get_handle.get_datapoints().unwrap()["Version"][0],
            gen_datapoint(2)
        );
        assert_eq!(handle.version(42).unwrap(), 2);
        let mut versions = handle.versions(42).unwrap();
        versions.sort();
        assert_eq!(versions, vec![1, 2, 3]);

        handle.revert(42, Some(3)).unwrap();
        assert_eq!(
            get_handle.get_datapoints().unwrap()["Version"][0],
            gen_datapoint(3)
        );
        assert_eq!(handle.version(42).unwrap(), 3);
        assert_eq!(handle.versions(42).unwrap(), vec![1, 2, 3]);
    }
}
