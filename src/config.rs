use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use std::collections::HashMap;

use crate::model::*;
use crate::*;

pub(crate) const BENCHER_CONFIG_FILENAME: &str = ".bencher-config";
pub(crate) const COLORS: [&str; 5] = ["f6511d", "ffb400", "00a6ed", "7fb800", "0d2c54"];

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
        config_path.push(BENCHER_CONFIG_FILENAME);

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
    linear_experiments: Vec<LinearExperiment>,
    xy_experiments: Vec<XYExperiment>,
    virtual_linear_experiments: Vec<VirtualLinearExperiment>,
    virtual_xy_experiments: Vec<VirtualXYExperiment>,
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
            linear_experiments: inner_config.linear_experiments.unwrap_or(vec![]),
            xy_experiments: inner_config.xy_experiments.unwrap_or(vec![]),
            virtual_linear_experiments: inner_config.virtual_linear_experiments.unwrap_or(vec![]),
            virtual_xy_experiments: inner_config.virtual_xy_experiments.unwrap_or(vec![]),
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
            linear_experiments: inner_config.linear_experiments.unwrap_or(vec![]),
            xy_experiments: inner_config.xy_experiments.unwrap_or(vec![]),
            virtual_linear_experiments: inner_config.virtual_linear_experiments.unwrap_or(vec![]),
            virtual_xy_experiments: inner_config.virtual_xy_experiments.unwrap_or(vec![]),
        })
    }

    pub fn status(
        &self,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<Vec<ExperimentStatus>> {
        self.db.status(selector, sorter)
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

    pub fn virtual_linear_experiments(&self) -> &Vec<VirtualLinearExperiment> {
        &self.virtual_linear_experiments
    }

    pub fn virtual_xy_experiments(&self) -> &Vec<VirtualXYExperiment> {
        &self.virtual_xy_experiments
    }

    pub fn list_linear_experiments(
        &self,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<Vec<LinearExperimentInfo>> {
        self.db.list_linear_experiments(
            self.linear_experiments(),
            self.virtual_linear_experiments(),
            selector,
            sorter,
        )
    }

    pub fn list_xy_experiments(
        &self,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<Vec<XYExperimentInfo>> {
        self.db.list_xy_experiments(
            self.xy_experiments(),
            self.virtual_xy_experiments(),
            selector,
            sorter,
        )
    }

    /// Linear experiments
    ///

    fn find_linear_experiment<'a>(&'a self, exp_type: &str) -> Option<&'a LinearExperiment> {
        self.linear_experiments
            .iter()
            .find(|e| e.exp_type == exp_type)
    }

    fn find_virtual_linear_experiment<'a>(
        &'a self,
        exp_type: &str,
    ) -> Option<&'a VirtualLinearExperiment> {
        self.virtual_linear_experiments
            .iter()
            .find(|e| e.exp_type == exp_type)
    }

    fn linear_experiments_as_string(&self) -> String {
        self.linear_experiments
            .iter()
            .map(|e| e.exp_type.clone())
            .chain(
                self.virtual_linear_experiments
                    .iter()
                    .map(|e| e.exp_type.clone()),
            )
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Get the linear experiment sets for a given experiment type
    fn get_linear_experiment_sets(
        &self,
        experiment: &LinearExperiment,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<Vec<LinearExperimentSet>> {
        let codes_labels =
            self.db
                .list_codes_labels_by_exp_type(&experiment.exp_type, selector, sorter)?;

        codes_labels
            .into_iter()
            .map(|(code, set_label)| {
                let mut values = self.db.get_linear_datapoints(&code)?;
                values.sort_by_key(|x| x.tag.unwrap());
                Ok(LinearExperimentSet { values, set_label })
            })
            .collect::<BencherResult<_>>()
    }

    /// Get the linear experiment sets for a given virtual experiment type
    /// This is done by getting the sets for the source and then transforming them
    fn get_virtual_linear_experiment_sets(
        &self,
        experiment: &VirtualLinearExperiment,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<Vec<LinearExperimentSet>> {
        fn map_linear_datapoints(
            vec: Vec<LinearDatapoint>,
            virtual_experiment: &VirtualLinearExperiment,
        ) -> BencherResult<Vec<LinearDatapoint>> {
            if vec.is_empty() {
                return Ok(vec);
            }

            let min = vec.iter().map(|e| e.v).min().unwrap();
            let max = vec.iter().map(|e| e.v).max().unwrap();
            let avg = if vec.iter().all(|e| e.v.is_int()) {
                Value::Int(
                    vec.iter()
                        .map(|e| e.v)
                        .map(|x| x.to_int().unwrap())
                        .sum::<i64>()
                        / vec.len() as i64,
                )
            } else {
                Value::Float(
                    vec.iter()
                        .map(|e| e.v.to_float().or(e.v.to_int().map(|x| x as f64)).unwrap())
                        .sum::<f64>()
                        / vec.len() as f64,
                )
            };

            vec.into_iter()
                .map(|dp| {
                    dp.map_expression(
                        virtual_experiment.v_operation.as_ref().map(|x| x.as_str()),
                        virtual_experiment
                            .tag_operation
                            .as_ref()
                            .map(|x| x.as_str()),
                        min,
                        max,
                        avg,
                    )
                })
                .collect::<BencherResult<Vec<_>>>()
        }

        if let Some(e) = self.find_virtual_linear_experiment(&experiment.source_exp_type) {
            let source_sets = self.get_virtual_linear_experiment_sets(e, selector, sorter)?;
            source_sets
                .into_iter()
                .map(|set| {
                    let mut values = map_linear_datapoints(set.values, experiment)?;
                    values.sort_by_key(|v| v.tag.unwrap());
                    Ok(LinearExperimentSet {
                        values,
                        set_label: set.set_label,
                    })
                })
                .collect::<BencherResult<Vec<_>>>()
        } else if let Some(e) = self.find_linear_experiment(&experiment.source_exp_type) {
            let source_sets = self.get_linear_experiment_sets(e, selector, sorter)?;
            source_sets
                .into_iter()
                .map(|set| {
                    let mut values = map_linear_datapoints(set.values, experiment)?;
                    values.sort_by_key(|v| v.tag.unwrap());
                    Ok(LinearExperimentSet {
                        values,
                        set_label: set.set_label,
                    })
                })
                .collect::<BencherResult<Vec<_>>>()
        } else {
            Err(BencherError::ExperimentNotFound(
                experiment.source_exp_type.clone(),
                self.linear_experiments_as_string(),
            ))
        }
    }

    pub fn linear_experiment_view(
        &self,
        exp_type: &str,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<LinearExperimentView> {
        let linear_experiment = self.find_linear_experiment(exp_type);
        let virtual_linear_experiment = self.find_virtual_linear_experiment(exp_type);

        match (linear_experiment, virtual_linear_experiment) {
            (Some(linear_experiment), _) => {
                let sets = self.get_linear_experiment_sets(linear_experiment, selector, sorter)?;
                LinearExperimentView::from_linear(linear_experiment, sets)
            }
            (None, Some(virtual_linear_experiment)) => {
                let sets = self.get_virtual_linear_experiment_sets(
                    virtual_linear_experiment,
                    selector,
                    sorter,
                )?;
                LinearExperimentView::from_virtual(virtual_linear_experiment, sets)
            }
            (None, None) => Err(BencherError::ExperimentNotFound(
                exp_type.to_string(),
                self.linear_experiments_as_string(),
            )),
        }
    }

    /// Bidimentional experiments
    ///

    fn find_xy_experiment<'a>(&'a self, exp_type: &str) -> Option<&'a XYExperiment> {
        self.xy_experiments.iter().find(|e| e.exp_type == exp_type)
    }

    fn find_virtual_xy_experiment<'a>(&'a self, exp_type: &str) -> Option<&'a VirtualXYExperiment> {
        self.virtual_xy_experiments
            .iter()
            .find(|e| e.exp_type == exp_type)
    }

    fn xy_experiments_as_string(&self) -> String {
        self.xy_experiments
            .iter()
            .map(|e| e.exp_type.clone())
            .chain(
                self.virtual_xy_experiments
                    .iter()
                    .map(|e| e.exp_type.clone()),
            )
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Get the xy experiment sets for a given experiment type
    fn get_xy_experiment_lines(
        &self,
        experiment: &XYExperiment,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<Vec<XYExperimentLine>> {
        let codes_labels =
            self.db
                .list_codes_labels_by_exp_type(&experiment.exp_type, selector, sorter)?;

        codes_labels
            .into_iter()
            .map(|(code, line_label)| {
                let mut values = self.db.get_xy_datapoints(&code)?;
                values.sort_by_key(|v| v.tag.unwrap());
                Ok(XYExperimentLine { values, line_label })
            })
            .collect::<BencherResult<_>>()
    }

    /// Get the xy experiment sets for a given virtual experiment type
    /// This is done by getting the source sets and then applying the transformation
    fn get_virtual_xy_experiment_lines(
        &self,
        experiment: &VirtualXYExperiment,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<Vec<XYExperimentLine>> {
        fn map_xy_datapoints(
            vec: Vec<XYDatapoint>,
            virtual_experiment: &VirtualXYExperiment,
        ) -> BencherResult<Vec<XYDatapoint>> {
            if vec.is_empty() {
                return Ok(vec);
            }

            let x_min = vec.iter().map(|e| e.x).min().unwrap();
            let x_max = vec.iter().map(|e| e.x).max().unwrap();
            let x_avg = if vec.iter().all(|e| e.x.is_int()) {
                Value::Int(
                    vec.iter()
                        .map(|e| e.x)
                        .map(|x| x.to_int().unwrap())
                        .sum::<i64>()
                        / vec.len() as i64,
                )
            } else {
                Value::Float(
                    vec.iter()
                        .map(|e| e.x.to_float().or(e.x.to_int().map(|x| x as f64)).unwrap())
                        .sum::<f64>()
                        / vec.len() as f64,
                )
            };

            let y_min = vec.iter().map(|e| e.y).min().unwrap();
            let y_max = vec.iter().map(|e| e.y).max().unwrap();
            let y_avg = if vec.iter().all(|e| e.y.is_int()) {
                Value::Int(
                    vec.iter()
                        .map(|e| e.y)
                        .map(|x| x.to_int().unwrap())
                        .sum::<i64>()
                        / vec.len() as i64,
                )
            } else {
                Value::Float(
                    vec.iter()
                        .map(|e| e.y.to_float().or(e.y.to_int().map(|x| x as f64)).unwrap())
                        .sum::<f64>()
                        / vec.len() as f64,
                )
            };

            vec.into_iter()
                .map(|dp| {
                    dp.map_expression(
                        virtual_experiment.x_operation.as_ref().map(|x| x.as_str()),
                        virtual_experiment.y_operation.as_ref().map(|x| x.as_str()),
                        virtual_experiment
                            .tag_operation
                            .as_ref()
                            .map(|x| x.as_str()),
                        x_min,
                        x_max,
                        x_avg,
                        y_min,
                        y_max,
                        y_avg,
                    )
                })
                .collect::<BencherResult<Vec<_>>>()
        }

        fn map_linear_sets_into_xy_lines(
            sets: Vec<LinearExperimentSet>,
            virtual_experiment: &VirtualXYExperiment,
        ) -> BencherResult<Vec<XYExperimentLine>> {
            let mut group_map: HashMap<String, Vec<_>> = HashMap::new();
            sets.into_iter()
                .map(|set| set.values)
                .flatten()
                .for_each(|linear_dp| {
                    group_map
                        .entry(linear_dp.group.clone())
                        .and_modify(|v| v.push(linear_dp.clone()))
                        .or_insert(vec![linear_dp.clone()]);
                });

            group_map
                .into_iter()
                .map(|(group, linear_dps)| {
                    let values = map_linear_datapoints(linear_dps, virtual_experiment)?;
                    Ok(XYExperimentLine {
                        values,
                        line_label: group.to_string(),
                    })
                })
                .collect::<BencherResult<Vec<_>>>()
        }

        fn map_linear_datapoints(
            vec: Vec<LinearDatapoint>,
            virtual_experiment: &VirtualXYExperiment,
        ) -> BencherResult<Vec<XYDatapoint>> {
            if vec.is_empty() {
                return Ok(vec![]);
            }

            let min = vec.iter().map(|e| e.v).min().unwrap();
            let max = vec.iter().map(|e| e.v).max().unwrap();
            let avg = if vec.iter().all(|e| e.v.is_int()) {
                Value::Int(
                    vec.iter()
                        .map(|e| e.v)
                        .map(|x| x.to_int().unwrap())
                        .sum::<i64>()
                        / vec.len() as i64,
                )
            } else {
                Value::Float(
                    vec.iter()
                        .map(|e| e.v.to_float().or(e.v.to_int().map(|x| x as f64)).unwrap())
                        .sum::<f64>()
                        / vec.len() as f64,
                )
            };

            vec.into_iter()
                .map(|dp| {
                    dp.map_expression_to_xy(
                        virtual_experiment.x_operation.as_ref().map(|x| x.as_str()),
                        virtual_experiment.y_operation.as_ref().map(|x| x.as_str()),
                        virtual_experiment
                            .tag_operation
                            .as_ref()
                            .map(|x| x.as_str()),
                        min,
                        max,
                        avg,
                    )
                })
                .collect::<BencherResult<Vec<_>>>()
        }

        if let Some(e) = self.find_virtual_xy_experiment(&experiment.source_exp_type) {
            let source_lines = self.get_virtual_xy_experiment_lines(e, selector, sorter)?;
            source_lines
                .into_iter()
                .map(|line| {
                    let mut values = map_xy_datapoints(line.values, experiment)?;
                    values.sort_by_key(|v| v.tag.unwrap());

                    Ok(XYExperimentLine {
                        values,
                        line_label: line.line_label,
                    })
                })
                .collect::<BencherResult<Vec<_>>>()
        } else if let Some(e) = self.find_virtual_linear_experiment(&experiment.source_exp_type) {
            let source_sets = self.get_virtual_linear_experiment_sets(e, selector, sorter)?;
            map_linear_sets_into_xy_lines(source_sets, experiment)
        } else if let Some(e) = self.find_linear_experiment(&experiment.source_exp_type) {
            let source_sets = self.get_linear_experiment_sets(e, selector, sorter)?;
            map_linear_sets_into_xy_lines(source_sets, experiment)
        } else if let Some(e) = self.find_xy_experiment(&experiment.source_exp_type) {
            let source_lines = self.get_xy_experiment_lines(e, selector, sorter)?;
            source_lines
                .into_iter()
                .map(|line| {
                    let mut values = map_xy_datapoints(line.values, experiment)?;
                    values.sort_by_key(|v| v.tag.unwrap());

                    Ok(XYExperimentLine {
                        values,
                        line_label: line.line_label,
                    })
                })
                .collect::<BencherResult<Vec<_>>>()
        } else {
            Err(BencherError::ExperimentNotFound(
                experiment.source_exp_type.clone(),
                self.xy_experiments_as_string(),
            ))
        }
    }

    /// Get the xy experiment view for a given experiment type
    pub fn xy_experiment_view(
        &self,
        exp_type: &str,
        selector: &Selector,
        sorter: &Sorter,
    ) -> BencherResult<XYExperimentView> {
        let xy_experiment = self.find_xy_experiment(exp_type);
        let virtual_xy_experiment = self.find_virtual_xy_experiment(exp_type);

        match (xy_experiment, virtual_xy_experiment) {
            (Some(xy_experiment), _) => {
                let sets = self.get_xy_experiment_lines(xy_experiment, selector, sorter)?;
                XYExperimentView::from_xy(xy_experiment, sets)
            }
            (None, Some(virtual_xy_experiment)) => {
                let sets =
                    self.get_virtual_xy_experiment_lines(virtual_xy_experiment, selector, sorter)?;
                XYExperimentView::from_virtual(virtual_xy_experiment, sets)
            }
            (None, None) => Err(BencherError::ExperimentNotFound(
                exp_type.to_string(),
                self.xy_experiments_as_string(),
            )),
        }
    }
}
