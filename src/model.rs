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

/// Struct to filter out results
#[derive(Debug, Default)]
pub struct Selector {
    exp_code_exclude: Option<regex::Regex>,
    exp_code_include: Option<regex::Regex>,
    exp_type_exclude: Option<regex::Regex>,
    exp_type_include: Option<regex::Regex>,
}

impl Selector {
    pub(crate) fn filter_code(&self, exp_code: &str) -> bool {
        match (&self.exp_code_exclude, &self.exp_code_include) {
            (None, None) => true,
            (Some(ex), None) => !ex.is_match(exp_code),
            (None, Some(in_)) => in_.is_match(exp_code),
            (Some(ex), Some(in_)) => !ex.is_match(exp_code) && in_.is_match(exp_code),
        }
    }

    pub(crate) fn filter_type(&self, exp_type: &str) -> bool {
        match (&self.exp_type_exclude, &self.exp_type_include) {
            (None, None) => true,
            (Some(ex), None) => !ex.is_match(exp_type),
            (None, Some(in_)) => in_.is_match(exp_type),
            (Some(ex), Some(in_)) => !ex.is_match(exp_type) && in_.is_match(exp_type),
        }
    }
}

pub struct SelectorBuilder {
    selector: Selector,
}

impl SelectorBuilder {
    pub fn new() -> Self {
        SelectorBuilder {
            selector: Selector::default(),
        }
    }

    pub fn code_exclude(mut self, re: regex::Regex) -> Self {
        self.selector.exp_code_exclude = Some(re);
        self
    }

    pub fn code_include(mut self, re: regex::Regex) -> Self {
        self.selector.exp_code_include = Some(re);
        self
    }

    pub fn type_exclude(mut self, re: regex::Regex) -> Self {
        self.selector.exp_type_exclude = Some(re);
        self
    }

    pub fn type_include(mut self, re: regex::Regex) -> Self {
        self.selector.exp_type_include = Some(re);
        self
    }

    pub fn build(self) -> Selector {
        self.selector
    }
}
