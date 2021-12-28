use either::Either;
use std::collections::HashMap;

use crate::error::*;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
            Value::Int(i) => match i {
                0..=999 => Magnitude::Normal,
                1_000..=999_999 => Magnitude::Kilo,
                1_000_000..=999_999_999 => Magnitude::Mega,
                _ => Magnitude::Giga,
            },
            Value::Float(f) => match *f {
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

#[derive(Debug, PartialEq)]
pub struct Datapoint {
    pub x: Value,

    pub x_confidence: HashMap<Confidence, (Value, Value)>,

    pub y: Value,

    pub y_confidence: HashMap<Confidence, (Value, Value)>,
}

impl Datapoint {
    pub fn new(
        x_int: Option<i64>,
        x_float: Option<f64>,

        y_int: Option<i64>,
        y_float: Option<f64>,
    ) -> Result<Datapoint, BencherError> {
        let x = x_int
            .map(|x| Value::Int(x))
            .or(x_float.map(|x| Value::Float(x)))
            .ok_or_else(|| BencherError::MissingXValue)?;
        let y = y_int
            .map(|y| Value::Int(y))
            .or(y_float.map(|y| Value::Float(y)))
            .ok_or_else(|| BencherError::MissingYValue)?;

        Ok(Datapoint {
            x,
            y,
            x_confidence: HashMap::new(),
            y_confidence: HashMap::new(),
        })
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

#[derive(serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Experiment {
    pub code: String,
    pub label: String,
    pub exp_type: String,
    pub x_label: String,
    pub x_units: String,
    pub y_label: String,
    pub y_units: String,
}

impl Experiment {
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.exp_type == other.exp_type
            && self.x_label == other.x_label
            && self.x_units == other.x_units
            && self.y_label == other.y_label
            && self.y_units == other.y_units
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ExperimentStatus {
    pub code: String,
    pub label: String,
    pub exp_type: String,
    pub n_datapoints: usize,
}

#[derive(serde::Deserialize)]
pub struct BencherConfig {
    /// database filepath relative to the config filepath
    pub database_filepath: String,

    /// experiment descriptions
    pub experiments: Vec<Experiment>,
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
    }

    #[test]
    fn datapoint_new() {
        assert!(Datapoint::new(None, None, None, None).is_err());

        assert!(Datapoint::new(Some(0), None, None, None).is_err());
        assert!(Datapoint::new(Some(0), Some(0.0), None, None).is_err());

        assert!(Datapoint::new(None, None, Some(0), None).is_err());
        assert!(Datapoint::new(None, None, Some(0), Some(0.0)).is_err());

        assert!(Datapoint::new(Some(0), None, Some(0), None).is_ok());
        assert!(Datapoint::new(Some(0), Some(0.0), Some(0), None).is_ok());
        assert!(Datapoint::new(Some(0), None, Some(0), Some(0.0)).is_ok());
        assert!(Datapoint::new(Some(0), Some(0.0), Some(0), Some(0.0)).is_ok());
    }

    #[test]
    fn datapoint_magnitudes() {
        assert_eq!(
            Datapoint::new(Some(50_000_000), None, None, Some(0.00005))
                .unwrap()
                .magnitudes(),
            (Magnitude::Mega, Magnitude::Micro)
        );
    }

    #[test]
    fn datapoint_confidence() {
        assert!(Datapoint::new(Some(0), None, Some(0), None)
            .unwrap()
            .add_x_confidence(0, Either::Left((0, 0)))
            .is_err());
        assert!(Datapoint::new(Some(0), None, Some(0), None)
            .unwrap()
            .add_x_confidence(1, Either::Left((0, 0)))
            .is_ok());
        assert!(Datapoint::new(Some(0), None, Some(0), None)
            .unwrap()
            .add_x_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_err());
        assert!(Datapoint::new(None, Some(0.0), Some(0), None)
            .unwrap()
            .add_x_confidence(1, Either::Left((0, 0)))
            .is_err());
        assert!(Datapoint::new(None, Some(0.0), Some(0), None)
            .unwrap()
            .add_x_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_ok());

        assert!(Datapoint::new(Some(0), None, Some(0), None)
            .unwrap()
            .add_y_confidence(1, Either::Left((0, 0)))
            .is_ok());
        assert!(Datapoint::new(Some(0), None, Some(0), None)
            .unwrap()
            .add_y_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_err());
        assert!(Datapoint::new(Some(0), None, None, Some(0.0))
            .unwrap()
            .add_y_confidence(1, Either::Left((0, 0)))
            .is_err());
        assert!(Datapoint::new(Some(0), None, None, Some(0.0))
            .unwrap()
            .add_y_confidence(1, Either::Right((0.0_f64, 0.0_f64)))
            .is_ok());
    }

    #[test]
    fn experiment_compatibility() {
        let exp1 = Experiment {
            code: "tput_latency_xyz".to_string(),
            exp_type: "Throughput Latency".to_string(),
            label: "Read".to_string(),
            x_label: "Throughput".to_string(),
            x_units: "ops/s".to_string(),
            y_label: "Latency".to_string(),
            y_units: "s".to_string(),
        };

        let exp2 = Experiment {
            code: "tput_latency_abc".to_string(),
            exp_type: "Throughput Latency".to_string(),
            label: "Write".to_string(),
            x_label: "Throughput".to_string(),
            x_units: "ops/s".to_string(),
            y_label: "Latency".to_string(),
            y_units: "s".to_string(),
        };

        let exp3 = Experiment {
            code: "tput_latency_abc".to_string(),
            exp_type: "Throughput Latency".to_string(),
            label: "Write".to_string(),
            x_label: "Throughput".to_string(),
            x_units: "ops".to_string(),
            y_label: "Latency".to_string(),
            y_units: "s".to_string(),
        };

        let exp4 = Experiment {
            code: "tput_latency_abc".to_string(),
            exp_type: "Throughput Latency".to_string(),
            label: "Write".to_string(),
            x_label: "Latency".to_string(),
            x_units: "ops/s".to_string(),
            y_label: "Latency".to_string(),
            y_units: "s".to_string(),
        };

        let exp5 = Experiment {
            code: "tput_latency_abc".to_string(),
            exp_type: "Throughput Latency".to_string(),
            label: "Write".to_string(),
            x_label: "Latency".to_string(),
            x_units: "ops/s".to_string(),
            y_label: "Latency".to_string(),
            y_units: "s".to_string(),
        };

        let exp6 = Experiment {
            code: "tput_latency_zzz".to_string(),
            exp_type: "Throughput".to_string(),
            label: "Write".to_string(),
            x_label: "Throughput".to_string(),
            x_units: "ops/s".to_string(),
            y_label: "Latency".to_string(),
            y_units: "s".to_string(),
        };

        assert!(exp1.is_compatible(&exp1));
        assert!(exp1.is_compatible(&exp2));
        assert!(!exp1.is_compatible(&exp3));
        assert!(!exp1.is_compatible(&exp4));
        assert!(!exp1.is_compatible(&exp5));
        assert!(!exp1.is_compatible(&exp6));
    }
}
