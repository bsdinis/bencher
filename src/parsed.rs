use crate::*;
use std::fs::File;
use std::io::BufReader;

/// A linear experiment represents a histogram
///
/// The group labels are the labels to be used of the groups in the histogram.
/// Example: if the histogram is latency per operation,
/// and there are two labels (A and B) and two operations (get and put),
/// the groups are put/get
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LinearExperiment {
    pub(crate) exp_type: String,
    pub(crate) horizontal_label: String,
    pub(crate) v_label: String,
    pub(crate) v_units: String,
}

/// A bidimensional (xy) experiment represents a line graph
///
/// The tag is an id for a point in a line
/// Additionally, a line has a label
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct XYExperiment {
    pub(crate) exp_type: String,
    pub(crate) x_label: String,
    pub(crate) x_units: String,
    pub(crate) y_label: String,
    pub(crate) y_units: String,
}

/// A virtual linear experiment
///
/// This takes an existing linear experiment and performs an operation on the each value
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct VirtualLinearExperiment {
    pub(crate) exp_type: String,
    pub(crate) source_exp_type: String, // TODO: make this a vector
    pub(crate) horizontal_label: String,
    pub(crate) v_label: String,
    pub(crate) v_units: String,
    pub(crate) v_operation: Option<String>,
    pub(crate) tag_operation: Option<String>,
}

/// A virtual bidimensional (xy) experiment
///
/// This takes an existing xy experiment and performs an operation
/// on the xy values
#[derive(serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct VirtualXYExperiment {
    pub(crate) exp_type: String,
    pub(crate) source_exp_type: String, // TODO: make this a vector
    pub(crate) x_label: String,
    pub(crate) x_units: String,
    pub(crate) y_label: String,
    pub(crate) y_units: String,
    pub(crate) x_operation: Option<String>,
    pub(crate) y_operation: Option<String>,
    pub(crate) tag_operation: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ParsedConfig {
    /// database filepath relative to the config filepath
    pub default_database_filepath: String,

    /// bidimensional experiment descriptions
    pub xy_experiments: Option<Vec<XYExperiment>>,

    /// linear experiment descriptions
    pub linear_experiments: Option<Vec<LinearExperiment>>,

    /// virtual bidimensional experiment descriptions
    pub virtual_xy_experiments: Option<Vec<VirtualXYExperiment>>,

    /// virtual linear experiment descriptions
    pub virtual_linear_experiments: Option<Vec<VirtualLinearExperiment>>,
}

impl ParsedConfig {
    pub(crate) fn from_path(path: &std::path::Path) -> BencherResult<Self> {
        let config_file = File::open(&path)
            .map_err(|e| BencherError::io_err(e, format!("opening {:?}", &path)))?;
        let reader = BufReader::new(config_file);
        let config = serde_json::from_reader(reader)?;

        Ok(config)
    }
}
