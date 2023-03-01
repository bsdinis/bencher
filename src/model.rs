pub enum Axis {
    X,
    Y,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ExperimentStatus {
    pub database: String,
    pub exp_type: String,
    pub exp_label: String,
    pub exp_code: String,
    pub n_datapoints: usize,
    pub n_active_datapoints: usize,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct LinearExperimentInfo {
    pub database: String,
    pub exp_type: String,
    pub exp_label: String,
    pub exp_code: String,
    pub horizontal_label: String,
    pub v_label: String,
    pub v_units: String,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct XYExperimentInfo {
    pub database: String,
    pub exp_type: String,
    pub exp_label: String,
    pub exp_code: String,
    pub x_label: String,
    pub x_units: String,
    pub y_label: String,
    pub y_units: String,
}
