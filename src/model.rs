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
    exp_code_exclude: Vec<regex::Regex>,
    exp_code_include: Vec<regex::Regex>,
    exp_type_exclude: Vec<regex::Regex>,
    exp_type_include: Vec<regex::Regex>,
}

impl Selector {
    pub(crate) fn filter_code(&self, exp_code: &str) -> bool {
        let res = (self.exp_code_exclude.len() == 0
            || !self.exp_code_exclude.iter().any(|re| re.is_match(exp_code)))
            && (self.exp_code_include.len() == 0
                || self.exp_code_include.iter().any(|re| re.is_match(exp_code)));

        res
    }

    pub(crate) fn filter_type(&self, exp_type: &str) -> bool {
        (self.exp_type_exclude.len() == 0
            || !self.exp_type_exclude.iter().any(|re| re.is_match(exp_type)))
            && (self.exp_type_include.len() == 0
                || self.exp_type_include.iter().any(|re| re.is_match(exp_type)))
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
        self.selector.exp_code_exclude.push(re);
        self
    }

    pub fn code_include(mut self, re: regex::Regex) -> Self {
        self.selector.exp_code_include.push(re);
        self
    }

    pub fn type_exclude(mut self, re: regex::Regex) -> Self {
        self.selector.exp_type_exclude.push(re);
        self
    }

    pub fn type_include(mut self, re: regex::Regex) -> Self {
        self.selector.exp_type_include.push(re);
        self
    }

    pub fn build(self) -> Selector {
        self.selector
    }
}
