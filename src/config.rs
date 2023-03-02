use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::model::*;
use crate::*;

pub(crate) const BENCHER_CONFIG_FILENAME: &str = ".bencher-config";
pub(crate) const COLORS: [&str; 5] = ["f6511d", "ffb400", "00a6ed", "7fb800", "0d2c54"];

/// A linear experiment represents a histogram
///
/// The group labels are the labels to be used of the groups in the histogram.
/// Example: if the histogram is latency per operation,
/// and there are two labels (A and B) and two operations (get and put),
/// the groups are put/get
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LinearExperiment {
    pub exp_type: String,
    pub horizontal_label: String,
    pub v_label: String,
    pub v_units: String,
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct XYExperiment {
    pub exp_type: String,
    pub x_label: String,
    pub x_units: String,
    pub y_label: String,
    pub y_units: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ParsedConfig {
    /// database filepath relative to the config filepath
    pub default_database_filepath: String,

    /// experiment descriptions
    pub xy_experiments: Vec<XYExperiment>,

    /// experiment descriptions
    pub linear_experiments: Vec<LinearExperiment>,
}

impl ParsedConfig {
    fn from_path(path: &std::path::Path) -> BencherResult<Self> {
        let config_file = File::open(&path)
            .map_err(|e| BencherError::io_err(e, format!("opening {:?}", &path)))?;
        let reader = BufReader::new(config_file);
        let config = serde_json::from_reader(reader)?;

        Ok(config)
    }
}

fn find_config_dir() -> BencherResult<PathBuf> {
    let mut dir: PathBuf = Path::new(".")
        .canonicalize()
        .map_err(|e| BencherError::io_err(e, "failed to canonicalize current dir name"))?;
    loop {
        let file = dir.join(BENCHER_CONFIG_FILENAME);
        if file.exists() {
            return Ok(dir);
        }

        if dir.parent().is_none() {
            break;
        }

        dir.pop();
    }

    Err(BencherError::NotFound.into())
}

#[derive(Debug)]
pub struct WriteConfig {
    db: DbWriteBackend,
}

impl WriteConfig {
    /// Constructors
    ///

    /// Create a new config, given the filename
    pub fn from_file(path: &std::path::Path) -> BencherResult<Self> {
        let db = DbWriteBackend::new(path)?;

        Ok(Self { db })
    }

    /// Create a new config, looking at the default path for the filename
    pub fn new() -> BencherResult<Self> {
        let mut config_path = find_config_dir()?;
        config_path.set_file_name(BENCHER_CONFIG_FILENAME);

        let inner_config = ParsedConfig::from_path(&config_path)?;
        config_path.set_file_name(inner_config.default_database_filepath);

        Self::from_file(&config_path)
    }

    pub fn to_read_config(self, inner_config: ParsedConfig) -> BencherResult<ReadConfig> {
        ReadConfig::from_conn_and_config(vec![self.db.into()], inner_config)
    }

    /// Create a new config from a pre-established connection and parsed config
    pub fn from_conn_and_config(conn: rusqlite::Connection) -> BencherResult<Self> {
        Ok(Self {
            db: DbWriteBackend::from_conn(conn)?,
        })
    }

    /// Linear Experiments
    ///

    /// Add a new linear experiment
    pub fn add_linear_set(
        &self,
        exp_type: &str,
        exp_label: &str,
        exp_code: &str,
    ) -> BencherResult<LinearSetHandle> {
        if self.db.experiment_exists(exp_type, exp_label, exp_code)? {
            return Err(BencherError::DuplicateExperiment(exp_code.into()));
        }

        self.db.insert_linear_set(exp_type, exp_label, exp_code)?;

        self.get_linear_set(exp_code)
            .map(|x| x.expect("just inserted this linear set, it *should* exist"))
    }

    /// Get the linear set handle
    pub fn get_linear_set(&self, exp_code: &str) -> BencherResult<Option<LinearSetHandle<'_>>> {
        self.db.get_linear_set(exp_code)
    }

    /// XY Experiments
    ///

    /// Add a new bidimensional experiment
    pub fn add_xy_line(
        &self,
        exp_type: &str,
        exp_label: &str,
        exp_code: &str,
    ) -> BencherResult<XYLineHandle> {
        if self.db.experiment_exists(exp_type, exp_label, exp_code)? {
            return Err(BencherError::DuplicateExperiment(exp_code.into()));
        }

        self.db.insert_xy_line(exp_type, exp_label, exp_code)?;

        self.get_xy_line(exp_code)
            .map(|x| x.expect("just inserted this xy line, it *should* exist"))
    }

    pub fn get_xy_line(&self, exp_code: &str) -> BencherResult<Option<XYLineHandle<'_>>> {
        self.db.get_xy_line(exp_code)
    }

    pub fn list_codes(&self) -> BencherResult<Vec<String>> {
        self.db.list_codes()
    }
}

#[derive(Debug)]
pub struct ReadConfig {
    db: DbReadBackend,
    xy_experiments: Vec<XYExperiment>,
    linear_experiments: Vec<LinearExperiment>,
}

impl ReadConfig {
    /// Constructors
    ///

    /// Create a new config, given the config filename and the list of db paths
    fn from_files<'a>(
        config_path: &std::path::Path,
        db_paths: impl Iterator<Item = &'a std::path::Path>,
        with_default: bool,
    ) -> BencherResult<Self> {
        let mut config_path: PathBuf = config_path.into();
        let config_file = File::open(&config_path)
            .map_err(|e| BencherError::io_err(e, format!("opening {:?}", &config_path)))?;
        let reader = BufReader::new(config_file);
        let inner_config: ParsedConfig = serde_json::from_reader(reader)?;

        let db = if with_default {
            config_path.set_file_name(inner_config.default_database_filepath);
            let db = DbReadBackend::new(&config_path, db_paths)?;
            db
        } else {
            DbReadBackend::from_paths(db_paths)?
        };

        Ok(Self {
            db,
            linear_experiments: inner_config.linear_experiments,
            xy_experiments: inner_config.xy_experiments,
        })
    }

    /// Create a new config,
    ///     looking at the default path for the config
    ///     and given a set of paths to DBs
    pub fn with_dbs<'a>(paths: impl Iterator<Item = &'a std::path::Path>) -> BencherResult<Self> {
        let mut config_path = find_config_dir()?;
        config_path.push(BENCHER_CONFIG_FILENAME);
        Self::from_files(&config_path, paths, false)
    }

    /// Create a new config,
    ///     looking at the default path for the config
    ///     and given a set of paths to DBs
    ///     including the default db in the config
    pub fn with_dbs_and_default<'a>(
        paths: impl Iterator<Item = &'a std::path::Path>,
    ) -> BencherResult<Self> {
        let mut config_path = find_config_dir()?;
        config_path.push(BENCHER_CONFIG_FILENAME);
        Self::from_files(&config_path, paths, true)
    }

    /// Create a new config,
    ///     looking at the default path for the config,
    ///     using the default db
    pub fn new() -> BencherResult<Self> {
        let mut config_path = find_config_dir()?;
        config_path.push(BENCHER_CONFIG_FILENAME);
        Self::from_files(&config_path, std::iter::empty(), true)
    }

    /// Create a new config from a pre-established connection and parsed config
    pub fn from_conn_and_config(
        dbs: Vec<rusqlite::Connection>,
        inner_config: ParsedConfig,
    ) -> BencherResult<Self> {
        Ok(Self {
            db: DbReadBackend::from_conns(dbs)?,
            linear_experiments: inner_config.linear_experiments,
            xy_experiments: inner_config.xy_experiments,
        })
    }

    pub fn status(&self, selector: &Selector) -> BencherResult<Vec<ExperimentStatus>> {
        self.db.status(selector)
    }

    pub fn list_codes(&self) -> BencherResult<Vec<String>> {
        self.db.list_codes()
    }

    pub fn linear_experiments(&self) -> &Vec<LinearExperiment> {
        &self.linear_experiments
    }

    pub fn xy_experiments(&self) -> &Vec<XYExperiment> {
        &self.xy_experiments
    }

    pub fn list_linear_experiments(
        &self,
        selector: &Selector,
    ) -> BencherResult<Vec<LinearExperimentInfo>> {
        self.db
            .list_linear_experiments(self.linear_experiments(), selector)
    }

    pub fn list_xy_experiments(&self, selector: &Selector) -> BencherResult<Vec<XYExperimentInfo>> {
        self.db.list_xy_experiments(self.xy_experiments(), selector)
    }

    /// Linear experiments
    ///

    fn find_linear_experiment<'a>(&'a self, exp_type: &str) -> Option<&'a LinearExperiment> {
        self.linear_experiments
            .iter()
            .find(|e| e.exp_type == exp_type)
    }

    fn linear_experiments_as_string(&self) -> String {
        self.linear_experiments
            .iter()
            .map(|e| e.exp_type.clone())
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Get the linear experiment view for a given experiment type
    ///
    fn get_linear_experiment_sets(
        &self,
        experiment: &LinearExperiment,
        selector: &Selector,
    ) -> BencherResult<Vec<LinearExperimentSet>> {
        let codes_labels = self
            .db
            .list_codes_labels_by_exp_type(&experiment.exp_type, selector)?;

        codes_labels
            .into_iter()
            .map(|(code, label)| {
                Ok(LinearExperimentSet {
                    values: self.db.get_linear_datapoints(&code)?,
                    set_label: label,
                })
            })
            .collect::<BencherResult<_>>()
    }

    pub fn linear_experiment_view(
        &self,
        exp_type: &str,
        selector: &Selector,
    ) -> BencherResult<LinearExperimentView> {
        let linear_experiment = self.find_linear_experiment(exp_type).ok_or_else(|| {
            BencherError::ExperimentNotFound(
                exp_type.to_string(),
                self.linear_experiments_as_string(),
            )
        })?;

        let sets = self.get_linear_experiment_sets(linear_experiment, selector)?;

        LinearExperimentView::new(linear_experiment, sets)
    }

    /// Bidimentional experiments
    ///

    fn find_xy_experiment<'a>(&'a self, exp_type: &str) -> Option<&'a XYExperiment> {
        self.xy_experiments.iter().find(|e| e.exp_type == exp_type)
    }

    fn xy_experiments_as_string(&self) -> String {
        self.xy_experiments
            .iter()
            .map(|e| e.exp_type.clone())
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Get the xy experiment view for a given experiment type
    fn get_xy_experiment_lines(
        &self,
        experiment: &XYExperiment,
        selector: &Selector,
    ) -> BencherResult<Vec<XYExperimentLine>> {
        let codes_labels = self
            .db
            .list_codes_labels_by_exp_type(&experiment.exp_type, selector)?;

        codes_labels
            .into_iter()
            .map(|(code, label)| {
                Ok(XYExperimentLine {
                    values: self.db.get_xy_datapoints(&code)?,
                    line_label: label,
                })
            })
            .collect::<BencherResult<_>>()
    }

    /// Get the xy experiment view for a given experiment type
    pub fn xy_experiment_view(
        &self,
        exp_type: &str,
        selector: &Selector,
    ) -> BencherResult<XYExperimentView> {
        let xy_experiment = self.find_xy_experiment(exp_type).ok_or_else(|| {
            BencherError::ExperimentNotFound(exp_type.to_string(), self.xy_experiments_as_string())
        })?;

        let lines = self.get_xy_experiment_lines(xy_experiment, selector)?;

        XYExperimentView::new(xy_experiment, lines)
    }
}
