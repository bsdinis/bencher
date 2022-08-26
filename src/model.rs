use either::Either;
use std::collections::BTreeMap;

use crate::error::*;
use crate::stat::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Magnitude {
    Nano,
    Micro,
    Mili,
    Normal,
    Kilo,
    Mega,
    Giga,
}

impl Magnitude {
    pub fn prefix(&self) -> &'static str {
        match self {
            Magnitude::Nano => "n",
            Magnitude::Micro => "μ",
            Magnitude::Mili => "m",
            Magnitude::Normal => "",
            Magnitude::Kilo => "K",
            Magnitude::Mega => "M",
            Magnitude::Giga => "G",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    One,
    Five,
    Ten,
    TwentyFive,
}

impl Confidence {
    pub fn new(c: usize) -> Result<Confidence, BencherError> {
        match c {
            1 | 99 => Ok(Confidence::One),
            5 | 95 => Ok(Confidence::Five),
            10 | 90 => Ok(Confidence::Ten),
            25 | 75 => Ok(Confidence::TwentyFive),
            _ => Err(BencherError::InvalidConfidence(c))?,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
}

impl Value {
    pub fn new(i: Option<i64>, f: Option<f64>) -> Result<Self, BencherError> {
        match (i, f) {
            (Some(i), _) => Ok(Value::Int(i)),
            (_, Some(f)) => Ok(Value::Float(f)),
            _ => Err(BencherError::EmptyValue),
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            Value::Int(_) => true,
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            Value::Float(_) => true,
            _ => false,
        }
    }

    pub fn to_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn to_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn magnitude(&self) -> Magnitude {
        match self {
            Value::Int(i) => match i.abs() {
                0..=999 => Magnitude::Normal,
                1_000..=999_999 => Magnitude::Kilo,
                1_000_000..=999_999_999 => Magnitude::Mega,
                _ => Magnitude::Giga,
            },
            Value::Float(f) => match f.abs() {
                x if x == 0.0_f64 => Magnitude::Normal,
                x if x < 1e-6_f64 => Magnitude::Nano,
                x if x >= 1e-6_f64 && x < 1e-3_f64 => Magnitude::Micro,
                x if x >= 1e-3_f64 && x < 1e+0_f64 => Magnitude::Mili,
                x if x >= 1e+0_f64 && x < 1e+3_f64 => Magnitude::Normal,
                x if x >= 1e+3_f64 && x < 1e+6_f64 => Magnitude::Kilo,
                x if x >= 1e+6_f64 && x < 1e+9_f64 => Magnitude::Mega,
                _ => Magnitude::Giga,
            },
        }
    }

    pub fn display_with_magnitude(&self, mag: Magnitude) -> String {
        match self {
            Value::Int(i) => match mag {
                Magnitude::Nano => format!("{}", i * 1_000_000_000),
                Magnitude::Micro => format!("{}", i * 1_000_000),
                Magnitude::Mili => format!("{}", i * 1_000),
                Magnitude::Normal => format!("{}", i),
                Magnitude::Kilo => format!("{:.1}", *i as f64 * 1e-3_f64),
                Magnitude::Mega => format!("{:.1}", *i as f64 * 1e-6_f64),
                Magnitude::Giga => format!("{:.1}", *i as f64 * 1e-9_f64),
            },
            Value::Float(f) => match mag {
                Magnitude::Nano => format!("{:.2}", f * 1e+9_f64),
                Magnitude::Micro => format!("{:.2}", f * 1e+6_f64),
                Magnitude::Mili => format!("{:.2}", f * 1e+3_f64),
                Magnitude::Normal => format!("{:.3}", f),
                Magnitude::Kilo => format!("{:.1}", f * 1e-3),
                Magnitude::Mega => format!("{:.1}", f * 1e-6),
                Magnitude::Giga => format!("{:.1}", f * 1e-9),
            },
        }
    }
}

/// A Linear datapoint represents a single column in a histogram
/// The group is the label of the histogram group
///
/// Example: if the histogram is latency per operation,
/// and there are two labels (A and B) and two operations (get and put),
/// there are 4 datapoints
///
/// A/get
/// A/put
/// B/get
/// B/put
///
/// the groups are put/get
#[derive(Debug, PartialEq, Clone)]
pub struct LinearDatapoint {
    pub group: String,

    pub v: Value,

    pub v_confidence: BTreeMap<Confidence, (Value, Value)>,
}

impl LinearDatapoint {
    pub fn new(group: impl Into<String>, v: Value) -> Self {
        LinearDatapoint {
            group: group.into(),
            v,
            v_confidence: BTreeMap::new(),
        }
    }

    pub fn from_sample_i64(
        group: impl Into<String>,
        sample: &mut Vec<i64>,
    ) -> Result<Option<Self>, BencherError> {
        if sample.len() == 0 {
            return Ok(None);
        }
        sample.sort_unstable();
        let mut datapoint = LinearDatapoint::new(group, Value::Int(integer_median(&sample)));

        for confidence in [1, 5, 10, 25].iter() {
            let (lower, upper) = (
                integer_percentile(&sample, *confidence),
                integer_percentile(&sample, 100 - *confidence),
            );
            datapoint
                .add_confidence(*confidence, Either::Left((lower, upper)))
                .expect("Unexpected type mismatch");
        }

        Ok(Some(datapoint))
    }

    pub fn from_sample_f64(
        group: impl Into<String>,
        sample: &mut Vec<f64>,
    ) -> Result<Option<Self>, BencherError> {
        if sample.len() == 0 {
            return Ok(None);
        }
        sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = LinearDatapoint::new(group, Value::Float(float_median(&sample)));

        for confidence in [1, 5, 10, 25].iter() {
            let (lower, upper) = (
                float_percentile(&sample, *confidence),
                float_percentile(&sample, 100 - *confidence),
            );
            datapoint
                .add_confidence(*confidence, Either::Right((lower, upper)))
                .expect("Unexpected type mismatch");
        }

        Ok(Some(datapoint))
    }

    pub fn magnitude(&self) -> Magnitude {
        self.v.magnitude()
    }

    pub fn add_confidence(
        &mut self,
        confidence: usize,
        values: Either<(i64, i64), (f64, f64)>,
    ) -> Result<(), BencherError> {
        let confidence = Confidence::new(confidence)?;
        if self.v.is_int() {
            if let Either::Left((lower, upper)) = values {
                self.v_confidence
                    .insert(confidence, (Value::Int(lower), Value::Int(upper)));
            } else {
                Err(BencherError::MismatchedTypes)?;
            }
        } else {
            if let Either::Right((lower, upper)) = values {
                self.v_confidence
                    .insert(confidence, (Value::Float(lower), Value::Float(upper)));
            } else {
                Err(BencherError::MismatchedTypes)?;
            }
        }
        Ok(())
    }

    pub fn get_confidence(&self, confidence: usize) -> Option<(Value, Value)> {
        let confidence = Confidence::new(confidence).ok()?;
        self.v_confidence.get(&confidence).cloned()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct XYDatapoint {
    pub x: Value,

    pub x_confidence: BTreeMap<Confidence, (Value, Value)>,

    pub y: Value,

    pub y_confidence: BTreeMap<Confidence, (Value, Value)>,

    pub tag: Option<isize>,
}

impl XYDatapoint {
    pub fn new(x: Value, y: Value) -> Self {
        XYDatapoint {
            x,
            y,
            x_confidence: BTreeMap::new(),
            y_confidence: BTreeMap::new(),
            tag: None,
        }
    }

    fn from_samples_i64_i64(x_sample: &mut Vec<i64>, y_sample: &mut Vec<i64>) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable();
        y_sample.sort_unstable();
        let mut datapoint = XYDatapoint::new(
            Value::Int(integer_median(&x_sample)),
            Value::Int(integer_median(&y_sample)),
        );

        for confidence in [1, 5, 10, 25].iter() {
            let (x_lower, x_upper) = (
                integer_percentile(&x_sample, *confidence),
                integer_percentile(&x_sample, 100 - *confidence),
            );
            let (y_lower, y_upper) = (
                integer_percentile(&y_sample, *confidence),
                integer_percentile(&y_sample, 100 - *confidence),
            );
            datapoint
                .add_x_confidence(*confidence, Either::Left((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(*confidence, Either::Left((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_i64_f64(x_sample: &mut Vec<i64>, y_sample: &mut Vec<f64>) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable();
        y_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = XYDatapoint::new(
            Value::Int(integer_median(&x_sample)),
            Value::Float(float_median(&y_sample)),
        );

        for confidence in [1, 5, 10, 25].iter() {
            let (x_lower, x_upper) = (
                integer_percentile(&x_sample, *confidence),
                integer_percentile(&x_sample, 100 - *confidence),
            );
            let (y_lower, y_upper) = (
                float_percentile(&y_sample, *confidence),
                float_percentile(&y_sample, 100 - *confidence),
            );
            datapoint
                .add_x_confidence(*confidence, Either::Left((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(*confidence, Either::Right((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_f64_i64(x_sample: &mut Vec<f64>, y_sample: &mut Vec<i64>) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        y_sample.sort_unstable();
        let mut datapoint = XYDatapoint::new(
            Value::Float(float_median(&x_sample)),
            Value::Int(integer_median(&y_sample)),
        );

        for confidence in [1, 5, 10, 25].iter() {
            let (x_lower, x_upper) = (
                float_percentile(&x_sample, *confidence),
                float_percentile(&x_sample, 100 - *confidence),
            );
            let (y_lower, y_upper) = (
                integer_percentile(&y_sample, *confidence),
                integer_percentile(&y_sample, 100 - *confidence),
            );
            datapoint
                .add_x_confidence(*confidence, Either::Right((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(*confidence, Either::Left((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_f64_f64(x_sample: &mut Vec<f64>, y_sample: &mut Vec<f64>) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        y_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = XYDatapoint::new(
            Value::Float(float_median(&x_sample)),
            Value::Float(float_median(&y_sample)),
        );

        for confidence in [1, 5, 10, 25].iter() {
            let (x_lower, x_upper) = (
                float_percentile(&x_sample, *confidence),
                float_percentile(&x_sample, 100 - *confidence),
            );
            let (y_lower, y_upper) = (
                float_percentile(&y_sample, *confidence),
                float_percentile(&y_sample, 100 - *confidence),
            );
            datapoint
                .add_x_confidence(*confidence, Either::Right((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(*confidence, Either::Right((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    pub fn from_samples(
        x_sample: Either<&mut Vec<i64>, &mut Vec<f64>>,
        y_sample: Either<&mut Vec<i64>, &mut Vec<f64>>,
    ) -> Option<Self> {
        match (x_sample, y_sample) {
            (Either::Left(x), Either::Left(y)) => Self::from_samples_i64_i64(x, y),
            (Either::Left(x), Either::Right(y)) => Self::from_samples_i64_f64(x, y),
            (Either::Right(x), Either::Left(y)) => Self::from_samples_f64_i64(x, y),
            (Either::Right(x), Either::Right(y)) => Self::from_samples_f64_f64(x, y),
        }
    }

    pub fn tag(mut self, tag: isize) -> Self {
        self.tag = Some(tag);
        self
    }

    pub fn magnitudes(&self) -> (Magnitude, Magnitude) {
        (self.x.magnitude(), self.y.magnitude())
    }

    pub fn add_x_confidence(
        &mut self,
        confidence: usize,
        values: Either<(i64, i64), (f64, f64)>,
    ) -> Result<(), BencherError> {
        let confidence = Confidence::new(confidence)?;
        if self.x.is_int() {
            if let Either::Left((lower, upper)) = values {
                self.x_confidence
                    .insert(confidence, (Value::Int(lower), Value::Int(upper)));
            } else {
                Err(BencherError::MismatchedTypes)?;
            }
        } else {
            if let Either::Right((lower, upper)) = values {
                self.x_confidence
                    .insert(confidence, (Value::Float(lower), Value::Float(upper)));
            } else {
                Err(BencherError::MismatchedTypes)?;
            }
        }
        Ok(())
    }

    pub fn add_y_confidence(
        &mut self,
        confidence: usize,
        values: Either<(i64, i64), (f64, f64)>,
    ) -> Result<(), BencherError> {
        let confidence = Confidence::new(confidence)?;
        if self.y.is_int() {
            if let Either::Left((lower, upper)) = values {
                self.y_confidence
                    .insert(confidence, (Value::Int(lower), Value::Int(upper)));
            } else {
                Err(BencherError::MismatchedTypes)?;
            }
        } else {
            if let Either::Right((lower, upper)) = values {
                self.y_confidence
                    .insert(confidence, (Value::Float(lower), Value::Float(upper)));
            } else {
                Err(BencherError::MismatchedTypes)?;
            }
        }
        Ok(())
    }

    pub fn get_x_confidence(&self, confidence: usize) -> Option<(Value, Value)> {
        let confidence = Confidence::new(confidence).ok()?;
        self.x_confidence.get(&confidence).cloned()
    }

    pub fn get_y_confidence(&self, confidence: usize) -> Option<(Value, Value)> {
        let confidence = Confidence::new(confidence).ok()?;
        self.y_confidence.get(&confidence).cloned()
    }
}

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

/// A linear experiment set represents all the groups under a label
///
/// Example: if the histogram is latency per operation,
/// and there are two labels (A and B) and two operations (get and put),
/// the sets are A (with A/get and A/put) and B (with B/get and B/put)
pub struct LinearExperimentSet {
    pub experiment: LinearExperiment,
    pub label: String,
    pub code: String,
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct XYExperiment {
    pub exp_type: String,
    pub x_label: String,
    pub x_units: String,
    pub y_label: String,
    pub y_units: String,
}

pub struct XYExperimentLine {
    pub experiment: XYExperiment,
    pub label: String,
    pub code: String,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ExperimentStatus {
    pub code: String,
    pub label: String,
    pub exp_type: String,
    pub n_datapoints: usize,
    pub n_active_datapoints: usize,
}

#[derive(Debug, serde::Deserialize)]
pub struct BencherConfig {
    /// database filepath relative to the config filepath
    pub database_filepath: String,

    /// experiment descriptions
    pub xy_experiments: Vec<XYExperiment>,

    /// experiment descriptions
    pub linear_experiments: Vec<LinearExperiment>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn magnitude_prefix() {
        assert_eq!(Magnitude::Nano.prefix(), "n");
        assert_eq!(Magnitude::Micro.prefix(), "μ");
        assert_eq!(Magnitude::Mili.prefix(), "m");
        assert_eq!(Magnitude::Normal.prefix(), "");
        assert_eq!(Magnitude::Kilo.prefix(), "K");
        assert_eq!(Magnitude::Mega.prefix(), "M");
        assert_eq!(Magnitude::Giga.prefix(), "G");
    }

    #[test]
    fn confidence() {
        assert!(Confidence::new(0).is_err());
        assert!(Confidence::new(14).is_err());
        assert!(Confidence::new(50).is_err());

        assert_eq!(Confidence::new(1).unwrap(), Confidence::One);
        assert_eq!(Confidence::new(5).unwrap(), Confidence::Five);
        assert_eq!(Confidence::new(10).unwrap(), Confidence::Ten);
        assert_eq!(Confidence::new(25).unwrap(), Confidence::TwentyFive);
        assert_eq!(Confidence::new(99).unwrap(), Confidence::One);
        assert_eq!(Confidence::new(95).unwrap(), Confidence::Five);
        assert_eq!(Confidence::new(90).unwrap(), Confidence::Ten);
        assert_eq!(Confidence::new(75).unwrap(), Confidence::TwentyFive);
    }

    #[test]
    fn value_new() {
        assert_eq!(Value::new(Some(1234), None).unwrap(), Value::Int(1234));
        assert_eq!(Value::new(None, Some(5.5)).unwrap(), Value::Float(5.5));
        assert!(Value::new(None, None).is_err());
    }

    #[test]
    fn value_is() {
        assert_eq!(Value::Int(0).is_int(), true);
        assert_eq!(Value::Int(0).is_float(), false);
        assert_eq!(Value::Float(0.0).is_int(), false);
        assert_eq!(Value::Float(0.0).is_float(), true);
    }

    #[test]
    fn value_to() {
        assert_eq!(Value::Int(0).to_int(), Some(0));
        assert_eq!(Value::Int(0).to_float(), None);
        assert_eq!(Value::Float(0.0).to_int(), None);
        assert_eq!(Value::Float(0.0).to_float(), Some(0.0));
    }

    #[test]
    fn value_magnitude() {
        assert_eq!(Value::Int(0).magnitude(), Magnitude::Normal);
        assert_eq!(Value::Int(999).magnitude(), Magnitude::Normal);
        assert_eq!(Value::Int(1_000).magnitude(), Magnitude::Kilo);
        assert_eq!(Value::Int(999_999).magnitude(), Magnitude::Kilo);
        assert_eq!(Value::Int(1_000_000).magnitude(), Magnitude::Mega);
        assert_eq!(Value::Int(999_999_999).magnitude(), Magnitude::Mega);
        assert_eq!(Value::Int(1_000_000_000).magnitude(), Magnitude::Giga);
        assert_eq!(Value::Int(1_000_000_000_000).magnitude(), Magnitude::Giga);

        assert_eq!(Value::Float(0.0_f64).magnitude(), Magnitude::Normal);
        assert_eq!(Value::Float(9e+2_f64).magnitude(), Magnitude::Normal);
        assert_eq!(Value::Float(1e+3_f64).magnitude(), Magnitude::Kilo);
        assert_eq!(Value::Float(9e+5_f64).magnitude(), Magnitude::Kilo);
        assert_eq!(Value::Float(1e+6_f64).magnitude(), Magnitude::Mega);
        assert_eq!(Value::Float(9e+7_f64).magnitude(), Magnitude::Mega);
        assert_eq!(Value::Float(1e+9_f64).magnitude(), Magnitude::Giga);
        assert_eq!(Value::Float(9e+10_f64).magnitude(), Magnitude::Giga);

        assert_eq!(Value::Float(0.1_f64).magnitude(), Magnitude::Mili);
        assert_eq!(Value::Float(0.001_f64).magnitude(), Magnitude::Mili);
        assert_eq!(Value::Float(0.0001_f64).magnitude(), Magnitude::Micro);
        assert_eq!(Value::Float(0.000001_f64).magnitude(), Magnitude::Micro);
        assert_eq!(Value::Float(0.0000001_f64).magnitude(), Magnitude::Nano);
        assert_eq!(Value::Float(0.000000001_f64).magnitude(), Magnitude::Nano);

        assert_eq!(Value::Float(-1e+6_f64).magnitude(), Magnitude::Mega);
        assert_eq!(Value::Float(-9e+10_f64).magnitude(), Magnitude::Giga);
        assert_eq!(Value::Float(-0.1_f64).magnitude(), Magnitude::Mili);
        assert_eq!(Value::Float(-0.000000001_f64).magnitude(), Magnitude::Nano);

        assert_eq!(Value::Int(-1_000).magnitude(), Magnitude::Kilo);
        assert_eq!(Value::Int(-1_000_000).magnitude(), Magnitude::Mega);
        assert_eq!(Value::Int(-1_000_000_000).magnitude(), Magnitude::Giga);
    }

    #[test]
    fn linear_datapoint_magnitudes() {
        assert_eq!(
            LinearDatapoint::new("", Value::Int(50_000_000)).magnitude(),
            Magnitude::Mega
        );
        assert_eq!(
            LinearDatapoint::new("", Value::Float(0.00005)).magnitude(),
            Magnitude::Micro
        );
    }

    #[test]
    fn linear_datapoint_confidence() {
        assert!(LinearDatapoint::new("", Value::Int(0))
            .add_confidence(0, Either::Left((0, 0)))
            .is_err());
        assert!(LinearDatapoint::new("", Value::Int(0))
            .add_confidence(1, Either::Left((0, 0)))
            .is_ok());
        assert!(LinearDatapoint::new("", Value::Int(0))
            .add_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_err());
        assert!(LinearDatapoint::new("", Value::Float(0.0))
            .add_confidence(1, Either::Left((0, 0)))
            .is_err());
        assert!(LinearDatapoint::new("", Value::Float(0.0))
            .add_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_ok());
    }

    #[test]
    fn linear_datapoint_from_sample_i64() {
        assert!(LinearDatapoint::from_sample_i64("", &mut vec![])
            .unwrap()
            .is_none());
        let mut sample: Vec<i64> = (0..100).into_iter().collect();
        let datapoint = LinearDatapoint::from_sample_i64("", &mut sample)
            .unwrap()
            .unwrap();
        assert_eq!(datapoint.v, Value::Int(49));
        assert_eq!(
            datapoint.get_confidence(1),
            Some((Value::Int(1), Value::Int(99)))
        );
        assert_eq!(
            datapoint.get_confidence(5),
            Some((Value::Int(5), Value::Int(95)))
        );
        assert_eq!(
            datapoint.get_confidence(10),
            Some((Value::Int(10), Value::Int(90)))
        );
        assert_eq!(
            datapoint.get_confidence(25),
            Some((Value::Int(25), Value::Int(75)))
        );
    }

    #[test]
    fn xy_datapoint_tag() {
        assert_eq!(XYDatapoint::new(Value::Int(0), Value::Int(0)).tag, None);
        assert_eq!(
            XYDatapoint::new(Value::Int(0), Value::Int(0)).tag(42).tag,
            Some(42)
        );
    }

    #[test]
    fn xy_datapoint_magnitudes() {
        assert_eq!(
            XYDatapoint::new(Value::Int(50_000_000), Value::Float(0.00005)).magnitudes(),
            (Magnitude::Mega, Magnitude::Micro)
        );
    }

    #[test]
    fn xy_datapoint_confidence() {
        assert!(XYDatapoint::new(Value::Int(0), Value::Int(0))
            .add_x_confidence(0, Either::Left((0, 0)))
            .is_err());
        assert!(XYDatapoint::new(Value::Int(0), Value::Int(0))
            .add_x_confidence(1, Either::Left((0, 0)))
            .is_ok());
        assert!(XYDatapoint::new(Value::Int(0), Value::Int(0))
            .add_x_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_err());
        assert!(XYDatapoint::new(Value::Float(0.0), Value::Int(0))
            .add_x_confidence(1, Either::Left((0, 0)))
            .is_err());
        assert!(XYDatapoint::new(Value::Float(0.0), Value::Int(0))
            .add_x_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_ok());

        assert!(XYDatapoint::new(Value::Int(0), Value::Int(0))
            .add_y_confidence(1, Either::Left((0, 0)))
            .is_ok());
        assert!(XYDatapoint::new(Value::Int(0), Value::Int(0))
            .add_y_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_err());
        assert!(XYDatapoint::new(Value::Int(0), Value::Float(0.0))
            .add_y_confidence(1, Either::Left((0, 0)))
            .is_err());
        assert!(XYDatapoint::new(Value::Int(0), Value::Float(0.0))
            .add_y_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_ok());
    }

    #[test]
    fn xy_datapoint_from_sample_i64() {
        assert!(
            XYDatapoint::from_samples(Either::Left(&mut vec![]), Either::Left(&mut vec![]))
                .is_none()
        );
        assert!(
            XYDatapoint::from_samples(Either::Left(&mut vec![1]), Either::Left(&mut vec![]))
                .is_none()
        );
        assert!(
            XYDatapoint::from_samples(Either::Left(&mut vec![]), Either::Left(&mut vec![1]))
                .is_none()
        );
        let mut x_sample: Vec<i64> = (0..100).into_iter().collect();
        let mut y_sample: Vec<i64> = (1000..1100).rev().into_iter().collect();
        let datapoint =
            XYDatapoint::from_samples(Either::Left(&mut x_sample), Either::Left(&mut y_sample))
                .unwrap();
        assert_eq!(datapoint.x, Value::Int(49));
        assert_eq!(datapoint.y, Value::Int(1049));
        assert_eq!(
            datapoint.get_x_confidence(1),
            Some((Value::Int(1), Value::Int(99)))
        );
        assert_eq!(
            datapoint.get_x_confidence(5),
            Some((Value::Int(5), Value::Int(95)))
        );
        assert_eq!(
            datapoint.get_x_confidence(10),
            Some((Value::Int(10), Value::Int(90)))
        );
        assert_eq!(
            datapoint.get_x_confidence(25),
            Some((Value::Int(25), Value::Int(75)))
        );
        assert_eq!(
            datapoint.get_y_confidence(1),
            Some((Value::Int(1001), Value::Int(1099)))
        );
        assert_eq!(
            datapoint.get_y_confidence(5),
            Some((Value::Int(1005), Value::Int(1095)))
        );
        assert_eq!(
            datapoint.get_y_confidence(10),
            Some((Value::Int(1010), Value::Int(1090)))
        );
        assert_eq!(
            datapoint.get_y_confidence(25),
            Some((Value::Int(1025), Value::Int(1075)))
        );
    }
}
