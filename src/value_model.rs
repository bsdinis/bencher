use either::Either;
use evalexpr::ContextWithMutableVariables;
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

pub const SUPPORTED_CONFIDENCES: [Confidence; 4] = [
    Confidence::One,
    Confidence::Five,
    Confidence::Ten,
    Confidence::TwentyFive,
];

impl TryFrom<usize> for Confidence {
    type Error = BencherError;
    fn try_from(c: usize) -> BencherResult<Confidence> {
        match c {
            1 | 99 => Ok(Confidence::One),
            5 | 95 => Ok(Confidence::Five),
            10 | 90 => Ok(Confidence::Ten),
            25 | 75 => Ok(Confidence::TwentyFive),
            _ => Err(BencherError::InvalidConfidence(c))?,
        }
    }
}

impl From<Confidence> for usize {
    fn from(c: Confidence) -> usize {
        match c {
            Confidence::One => 1,
            Confidence::Five => 5,
            Confidence::Ten => 10,
            Confidence::TwentyFive => 25,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{}", v),
        }
    }
}

impl std::cmp::PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Float(b)) => (&(*a as f64)).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
        }
    }
}

impl std::cmp::Eq for Value {}

impl std::cmp::Ord for Value {
    fn cmp(&self, other: &Value) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl From<Value> for evalexpr::Value {
    fn from(value: Value) -> evalexpr::Value {
        match value {
            Value::Int(i) => evalexpr::Value::Int(i),
            Value::Float(f) => evalexpr::Value::Float(f),
        }
    }
}

impl TryFrom<evalexpr::Value> for Value {
    type Error = BencherError;
    fn try_from(value: evalexpr::Value) -> BencherResult<Value> {
        match value {
            evalexpr::Value::Int(i) => Ok(Value::Int(i)),
            evalexpr::Value::Float(f) => Ok(Value::Float(f)),
            _ => Err(BencherError::ExpressionConversionError(value)),
        }
    }
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

    pub tag: Option<isize>,
}

impl LinearDatapoint {
    pub fn new(group: impl Into<String>, v: Value) -> Self {
        LinearDatapoint {
            group: group.into(),
            v,
            v_confidence: BTreeMap::new(),
            tag: None,
        }
    }

    pub fn from_sample_i64_median(
        group: impl Into<String>,
        sample: &mut Vec<i64>,
    ) -> Result<Option<Self>, BencherError> {
        if sample.len() == 0 {
            return Ok(None);
        }
        sample.sort_unstable();
        let mut datapoint = LinearDatapoint::new(group, Value::Int(integer_median(&sample)));

        for confidence in SUPPORTED_CONFIDENCES {
            let (lower, upper) = (
                integer_percentile(&sample, usize::from(confidence)),
                integer_percentile(&sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_confidence(confidence.into(), Either::Left((lower, upper)))
                .expect("Unexpected type mismatch");
        }

        Ok(Some(datapoint))
    }

    pub fn from_sample_f64_median(
        group: impl Into<String>,
        sample: &mut Vec<f64>,
    ) -> Result<Option<Self>, BencherError> {
        if sample.len() == 0 {
            return Ok(None);
        }
        sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = LinearDatapoint::new(group, Value::Float(float_median(&sample)));

        for confidence in SUPPORTED_CONFIDENCES {
            let (lower, upper) = (
                float_percentile(&sample, usize::from(confidence)),
                float_percentile(&sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_confidence(confidence, Either::Right((lower, upper)))
                .expect("Unexpected type mismatch");
        }

        Ok(Some(datapoint))
    }

    pub fn from_sample_i64_avg(
        group: impl Into<String>,
        sample: &mut Vec<i64>,
    ) -> Result<Option<Self>, BencherError> {
        if sample.len() == 0 {
            return Ok(None);
        }
        sample.sort_unstable();
        let mut datapoint = LinearDatapoint::new(group, Value::Int(integer_avg(&sample)));

        for confidence in SUPPORTED_CONFIDENCES {
            let (lower, upper) = (
                integer_percentile(&sample, usize::from(confidence)),
                integer_percentile(&sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_confidence(confidence, Either::Left((lower, upper)))
                .expect("Unexpected type mismatch");
        }

        Ok(Some(datapoint))
    }

    pub fn from_sample_f64_avg(
        group: impl Into<String>,
        sample: &mut Vec<f64>,
    ) -> Result<Option<Self>, BencherError> {
        if sample.len() == 0 {
            return Ok(None);
        }
        sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = LinearDatapoint::new(group, Value::Float(float_avg(&sample)));

        for confidence in SUPPORTED_CONFIDENCES {
            let (lower, upper) = (
                float_percentile(&sample, usize::from(confidence)),
                float_percentile(&sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_confidence(confidence, Either::Right((lower, upper)))
                .expect("Unexpected type mismatch");
        }

        Ok(Some(datapoint))
    }

    pub fn tag(mut self, tag: isize) -> Self {
        self.tag = Some(tag);
        self
    }

    pub fn magnitude(&self) -> Magnitude {
        self.v.magnitude()
    }

    pub fn add_confidence(
        &mut self,
        confidence: Confidence,
        values: Either<(i64, i64), (f64, f64)>,
    ) -> BencherResult<()> {
        if self.v.is_int() {
            if let Either::Left((lower, upper)) = values {
                self.v_confidence
                    .insert(confidence, (Value::Int(lower), Value::Int(upper)));
            } else {
                Err(BencherError::MismatchedBarTypes)?;
            }
        } else {
            if let Either::Right((lower, upper)) = values {
                self.v_confidence
                    .insert(confidence, (Value::Float(lower), Value::Float(upper)));
            } else {
                Err(BencherError::MismatchedBarTypes)?;
            }
        }
        Ok(())
    }

    fn add_value_confidence(&mut self, confidence: Confidence, values: (Value, Value)) {
        self.v_confidence.insert(confidence, values);
    }

    pub fn get_confidence(&self, confidence: Confidence) -> Option<(Value, Value)> {
        self.v_confidence.get(&confidence).cloned()
    }

    fn get_evalexpr_context(
        value: Value,
        tag: isize,
        min: Value,
        max: Value,
        avg: Value,
    ) -> BencherResult<evalexpr::HashMapContext> {
        let value: evalexpr::Value = value.into();
        let mut ctx = evalexpr::HashMapContext::new();
        ctx.set_value("v".to_string(), value.clone())?;
        ctx.set_value("V".to_string(), value)?;
        ctx.set_value("tag".to_string(), evalexpr::Value::Int(tag as i64))?;
        ctx.set_value("min".to_string(), min.into())?;
        ctx.set_value("max".to_string(), max.into())?;
        ctx.set_value("avg".to_string(), avg.into())?;

        Ok(ctx)
    }

    pub(crate) fn map_expression(
        &self,
        v_expr: Option<&str>,
        tag_expr: Option<&str>,
        global_min: Value,
        global_max: Value,
        global_avg: Value,
    ) -> BencherResult<LinearDatapoint> {
        let v_expr = v_expr.unwrap_or("v");
        let tag_expr = tag_expr.unwrap_or("tag");

        // build basic datapoint
        let ctx = Self::get_evalexpr_context(
            self.v,
            self.tag.unwrap(),
            global_min,
            global_max,
            global_avg,
        )?;
        let new_v: Value = evalexpr::eval_with_context(v_expr, &ctx)?.try_into()?;
        let new_tag = evalexpr::eval_with_context(tag_expr, &ctx)?;
        let new_tag = match new_tag {
            evalexpr::Value::Int(t) => Ok(t as isize),
            _ => Err(BencherError::ExpressionConversionError(new_tag.into())),
        }?;
        let mut new_datapoint = LinearDatapoint::new(self.group.clone(), new_v).tag(new_tag);

        for c in SUPPORTED_CONFIDENCES {
            if let Some((min, max)) = self.v_confidence.get(&c) {
                let new_min: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        min.clone(),
                        self.tag.unwrap(),
                        global_min,
                        global_max,
                        global_avg,
                    )?;
                    evalexpr::eval_with_context(v_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_min = new_min??;

                let new_max: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        max.clone(),
                        self.tag.unwrap(),
                        global_min,
                        global_max,
                        global_avg,
                    )?;
                    evalexpr::eval_with_context(v_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_max = new_max??;

                new_datapoint.add_value_confidence(c, (new_min, new_max));
            }
        }

        Ok(new_datapoint)
    }

    pub(crate) fn map_expression_to_xy(
        &self,
        x_expr: Option<&str>,
        y_expr: Option<&str>,
        tag_expr: Option<&str>,
        global_min: Value,
        global_max: Value,
        global_avg: Value,
    ) -> BencherResult<XYDatapoint> {
        let x_expr = x_expr.unwrap_or("tag");
        let y_expr = y_expr.unwrap_or("v");
        let tag_expr = tag_expr.unwrap_or("tag");

        // build basic datapoint
        let ctx = Self::get_evalexpr_context(
            self.v,
            self.tag.unwrap(),
            global_min,
            global_max,
            global_avg,
        )?;

        let new_x: Value = evalexpr::eval_with_context(x_expr, &ctx)?.try_into()?;
        let new_y: Value = evalexpr::eval_with_context(y_expr, &ctx)?.try_into()?;
        let new_tag = evalexpr::eval_with_context(tag_expr, &ctx)?;
        let new_tag = match new_tag {
            evalexpr::Value::Int(t) => Ok(t as isize),
            _ => Err(BencherError::ExpressionConversionError(new_tag.into())),
        }?;
        let mut new_datapoint = XYDatapoint::new(new_x, new_y).tag(new_tag);

        for c in SUPPORTED_CONFIDENCES {
            if let Some((min, max)) = self.v_confidence.get(&c) {
                let new_x_min: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        min.clone(),
                        self.tag.unwrap(),
                        global_min,
                        global_max,
                        global_avg,
                    )?;
                    evalexpr::eval_with_context(x_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_x_min = new_x_min??;

                let new_x_max: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        max.clone(),
                        self.tag.unwrap(),
                        global_min,
                        global_max,
                        global_avg,
                    )?;
                    evalexpr::eval_with_context(x_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_x_max = new_x_max??;

                new_datapoint.add_x_value_confidence(c, (new_x_min, new_x_max));

                let new_y_min: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        min.clone(),
                        self.tag.unwrap(),
                        global_min,
                        global_max,
                        global_avg,
                    )?;
                    evalexpr::eval_with_context(y_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_y_min = new_y_min??;

                let new_y_max: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        max.clone(),
                        self.tag.unwrap(),
                        global_min,
                        global_max,
                        global_avg,
                    )?;
                    evalexpr::eval_with_context(y_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_y_max = new_y_max??;

                new_datapoint.add_y_value_confidence(c, (new_y_min, new_y_max));
            }
        }

        Ok(new_datapoint)
    }
}

impl std::fmt::Display for LinearDatapoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for c in SUPPORTED_CONFIDENCES {
            if let Some((min, max)) = self.v_confidence.get(&c) {
                return write!(f, "{}: {} ([{};{}])", self.group, self.v, min, max);
            }
        }

        write!(f, "{}: {}", self.group, self.v)
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

    pub fn x_linear(&self, group: impl Into<String>) -> LinearDatapoint {
        LinearDatapoint {
            group: group.into(),
            v: self.x.clone(),
            v_confidence: self.x_confidence.clone(),
            tag: self.tag,
        }
    }

    pub fn y_linear(&self, group: impl Into<String>) -> LinearDatapoint {
        LinearDatapoint {
            group: group.into(),
            v: self.y.clone(),
            v_confidence: self.y_confidence.clone(),
            tag: self.tag,
        }
    }

    fn from_samples_i64_i64_median(
        x_sample: &mut Vec<i64>,
        y_sample: &mut Vec<i64>,
    ) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable();
        y_sample.sort_unstable();
        let mut datapoint = XYDatapoint::new(
            Value::Int(integer_median(&x_sample)),
            Value::Int(integer_median(&y_sample)),
        );

        for confidence in SUPPORTED_CONFIDENCES {
            let (x_lower, x_upper) = (
                integer_percentile(&x_sample, usize::from(confidence)),
                integer_percentile(&x_sample, 100 - usize::from(confidence)),
            );
            let (y_lower, y_upper) = (
                integer_percentile(&y_sample, usize::from(confidence)),
                integer_percentile(&y_sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_x_confidence(confidence, Either::Left((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(confidence, Either::Left((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_i64_f64_median(
        x_sample: &mut Vec<i64>,
        y_sample: &mut Vec<f64>,
    ) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable();
        y_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = XYDatapoint::new(
            Value::Int(integer_median(&x_sample)),
            Value::Float(float_median(&y_sample)),
        );

        for confidence in SUPPORTED_CONFIDENCES {
            let (x_lower, x_upper) = (
                integer_percentile(&x_sample, usize::from(confidence)),
                integer_percentile(&x_sample, 100 - usize::from(confidence)),
            );
            let (y_lower, y_upper) = (
                float_percentile(&y_sample, usize::from(confidence)),
                float_percentile(&y_sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_x_confidence(confidence, Either::Left((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(confidence, Either::Right((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_f64_i64_median(
        x_sample: &mut Vec<f64>,
        y_sample: &mut Vec<i64>,
    ) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        y_sample.sort_unstable();
        let mut datapoint = XYDatapoint::new(
            Value::Float(float_median(&x_sample)),
            Value::Int(integer_median(&y_sample)),
        );

        for confidence in SUPPORTED_CONFIDENCES {
            let (x_lower, x_upper) = (
                float_percentile(&x_sample, usize::from(confidence)),
                float_percentile(&x_sample, 100 - usize::from(confidence)),
            );
            let (y_lower, y_upper) = (
                integer_percentile(&y_sample, usize::from(confidence)),
                integer_percentile(&y_sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_x_confidence(confidence, Either::Right((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(confidence, Either::Left((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_f64_f64_median(
        x_sample: &mut Vec<f64>,
        y_sample: &mut Vec<f64>,
    ) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        y_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = XYDatapoint::new(
            Value::Float(float_median(&x_sample)),
            Value::Float(float_median(&y_sample)),
        );

        for confidence in SUPPORTED_CONFIDENCES {
            let (x_lower, x_upper) = (
                float_percentile(&x_sample, usize::from(confidence)),
                float_percentile(&x_sample, 100 - usize::from(confidence)),
            );
            let (y_lower, y_upper) = (
                float_percentile(&y_sample, usize::from(confidence)),
                float_percentile(&y_sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_x_confidence(confidence, Either::Right((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(confidence, Either::Right((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    pub fn from_samples_median(
        x_sample: Either<&mut Vec<i64>, &mut Vec<f64>>,
        y_sample: Either<&mut Vec<i64>, &mut Vec<f64>>,
    ) -> Option<Self> {
        match (x_sample, y_sample) {
            (Either::Left(x), Either::Left(y)) => Self::from_samples_i64_i64_median(x, y),
            (Either::Left(x), Either::Right(y)) => Self::from_samples_i64_f64_median(x, y),
            (Either::Right(x), Either::Left(y)) => Self::from_samples_f64_i64_median(x, y),
            (Either::Right(x), Either::Right(y)) => Self::from_samples_f64_f64_median(x, y),
        }
    }

    fn from_samples_i64_i64_avg(x_sample: &mut Vec<i64>, y_sample: &mut Vec<i64>) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable();
        y_sample.sort_unstable();
        let mut datapoint = XYDatapoint::new(
            Value::Int(integer_avg(&x_sample)),
            Value::Int(integer_avg(&y_sample)),
        );

        for confidence in SUPPORTED_CONFIDENCES {
            let (x_lower, x_upper) = (
                integer_percentile(&x_sample, usize::from(confidence)),
                integer_percentile(&x_sample, 100 - usize::from(confidence)),
            );
            let (y_lower, y_upper) = (
                integer_percentile(&y_sample, usize::from(confidence)),
                integer_percentile(&y_sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_x_confidence(confidence, Either::Left((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(confidence, Either::Left((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_i64_f64_avg(x_sample: &mut Vec<i64>, y_sample: &mut Vec<f64>) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable();
        y_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = XYDatapoint::new(
            Value::Int(integer_avg(&x_sample)),
            Value::Float(float_avg(&y_sample)),
        );

        for confidence in SUPPORTED_CONFIDENCES {
            let (x_lower, x_upper) = (
                integer_percentile(&x_sample, usize::from(confidence)),
                integer_percentile(&x_sample, 100 - usize::from(confidence)),
            );
            let (y_lower, y_upper) = (
                float_percentile(&y_sample, usize::from(confidence)),
                float_percentile(&y_sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_x_confidence(confidence, Either::Left((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(confidence, Either::Right((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_f64_i64_avg(x_sample: &mut Vec<f64>, y_sample: &mut Vec<i64>) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        y_sample.sort_unstable();
        let mut datapoint = XYDatapoint::new(
            Value::Float(float_avg(&x_sample)),
            Value::Int(integer_avg(&y_sample)),
        );

        for confidence in SUPPORTED_CONFIDENCES {
            let (x_lower, x_upper) = (
                float_percentile(&x_sample, usize::from(confidence)),
                float_percentile(&x_sample, 100 - usize::from(confidence)),
            );
            let (y_lower, y_upper) = (
                integer_percentile(&y_sample, usize::from(confidence)),
                integer_percentile(&y_sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_x_confidence(confidence, Either::Right((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(confidence, Either::Left((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    fn from_samples_f64_f64_avg(x_sample: &mut Vec<f64>, y_sample: &mut Vec<f64>) -> Option<Self> {
        if x_sample.len() == 0 || y_sample.len() == 0 {
            return None;
        }
        x_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        y_sample.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let mut datapoint = XYDatapoint::new(
            Value::Float(float_avg(&x_sample)),
            Value::Float(float_avg(&y_sample)),
        );

        for confidence in SUPPORTED_CONFIDENCES {
            let (x_lower, x_upper) = (
                float_percentile(&x_sample, usize::from(confidence)),
                float_percentile(&x_sample, 100 - usize::from(confidence)),
            );
            let (y_lower, y_upper) = (
                float_percentile(&y_sample, usize::from(confidence)),
                float_percentile(&y_sample, 100 - usize::from(confidence)),
            );
            datapoint
                .add_x_confidence(confidence, Either::Right((x_lower, x_upper)))
                .expect("Unexpected type mismatch");
            datapoint
                .add_y_confidence(confidence, Either::Right((y_lower, y_upper)))
                .expect("Unexpected type mismatch");
        }
        Some(datapoint)
    }

    pub fn from_samples_avg(
        x_sample: Either<&mut Vec<i64>, &mut Vec<f64>>,
        y_sample: Either<&mut Vec<i64>, &mut Vec<f64>>,
    ) -> Option<Self> {
        match (x_sample, y_sample) {
            (Either::Left(x), Either::Left(y)) => Self::from_samples_i64_i64_avg(x, y),
            (Either::Left(x), Either::Right(y)) => Self::from_samples_i64_f64_avg(x, y),
            (Either::Right(x), Either::Left(y)) => Self::from_samples_f64_i64_avg(x, y),
            (Either::Right(x), Either::Right(y)) => Self::from_samples_f64_f64_avg(x, y),
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
        confidence: Confidence,
        values: Either<(i64, i64), (f64, f64)>,
    ) -> Result<(), BencherError> {
        if self.x.is_int() {
            if let Either::Left((lower, upper)) = values {
                self.x_confidence
                    .insert(confidence, (Value::Int(lower), Value::Int(upper)));
            } else {
                Err(BencherError::MismatchedBarTypes)?;
            }
        } else {
            if let Either::Right((lower, upper)) = values {
                self.x_confidence
                    .insert(confidence, (Value::Float(lower), Value::Float(upper)));
            } else {
                Err(BencherError::MismatchedBarTypes)?;
            }
        }
        Ok(())
    }

    fn add_x_value_confidence(&mut self, confidence: Confidence, values: (Value, Value)) {
        self.x_confidence.insert(confidence, values);
    }

    pub fn add_y_confidence(
        &mut self,
        confidence: Confidence,
        values: Either<(i64, i64), (f64, f64)>,
    ) -> Result<(), BencherError> {
        if self.y.is_int() {
            if let Either::Left((lower, upper)) = values {
                self.y_confidence
                    .insert(confidence, (Value::Int(lower), Value::Int(upper)));
            } else {
                Err(BencherError::MismatchedBarTypes)?;
            }
        } else {
            if let Either::Right((lower, upper)) = values {
                self.y_confidence
                    .insert(confidence, (Value::Float(lower), Value::Float(upper)));
            } else {
                Err(BencherError::MismatchedBarTypes)?;
            }
        }
        Ok(())
    }

    fn add_y_value_confidence(&mut self, confidence: Confidence, values: (Value, Value)) {
        self.y_confidence.insert(confidence, values);
    }

    pub fn get_x_confidence(&self, confidence: Confidence) -> Option<(Value, Value)> {
        self.x_confidence.get(&confidence).cloned()
    }

    pub fn get_y_confidence(&self, confidence: Confidence) -> Option<(Value, Value)> {
        self.y_confidence.get(&confidence).cloned()
    }

    fn get_evalexpr_context(
        xvalue: Value,
        yvalue: Value,
        tag: isize,
        xmin: Value,
        xmax: Value,
        xavg: Value,
        ymin: Value,
        ymax: Value,
        yavg: Value,
    ) -> BencherResult<evalexpr::HashMapContext> {
        let mut ctx = evalexpr::HashMapContext::new();
        let xvalue: evalexpr::Value = xvalue.into();
        let yvalue: evalexpr::Value = yvalue.into();
        ctx.set_value("x".to_string(), xvalue.clone())?;
        ctx.set_value("X".to_string(), xvalue)?;
        ctx.set_value("y".to_string(), yvalue.clone())?;
        ctx.set_value("Y".to_string(), yvalue)?;
        ctx.set_value("tag".to_string(), evalexpr::Value::Int(tag as i64))?;
        ctx.set_value("xmin".to_string(), xmin.into())?;
        ctx.set_value("xmax".to_string(), xmax.into())?;
        ctx.set_value("xavg".to_string(), xavg.into())?;
        ctx.set_value("ymin".to_string(), ymin.into())?;
        ctx.set_value("ymax".to_string(), ymax.into())?;
        ctx.set_value("yavg".to_string(), yavg.into())?;
        Ok(ctx)
    }

    pub(crate) fn map_expression(
        &self,
        x_expr: Option<&str>,
        y_expr: Option<&str>,
        tag_expr: Option<&str>,
        global_x_min: Value,
        global_x_max: Value,
        global_x_avg: Value,
        global_y_min: Value,
        global_y_max: Value,
        global_y_avg: Value,
    ) -> BencherResult<XYDatapoint> {
        // build basic datapoint
        let x_expr = x_expr.unwrap_or("x");
        let y_expr = y_expr.unwrap_or("y");
        let tag_expr = tag_expr.unwrap_or("tag");

        let ctx = Self::get_evalexpr_context(
            self.x,
            self.y,
            self.tag.unwrap(),
            global_x_min,
            global_x_max,
            global_x_avg,
            global_y_min,
            global_y_max,
            global_y_avg,
        )?;
        let new_x: Value = evalexpr::eval_with_context(x_expr, &ctx)?.try_into()?;
        let new_y: Value = evalexpr::eval_with_context(y_expr, &ctx)?.try_into()?;
        let new_tag = evalexpr::eval_with_context(tag_expr, &ctx)?;
        let new_tag = match new_tag {
            evalexpr::Value::Int(t) => Ok(t as isize),
            _ => Err(BencherError::ExpressionConversionError(new_tag.into())),
        }?;
        let mut new_datapoint = XYDatapoint::new(new_x, new_y).tag(new_tag);

        for c in SUPPORTED_CONFIDENCES {
            if let Some((x_min, x_max)) = self.x_confidence.get(&c.try_into().unwrap()) {
                let new_x_min: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        x_min.clone(),
                        self.y,
                        self.tag.unwrap(),
                        global_x_min,
                        global_x_max,
                        global_x_avg,
                        global_y_min,
                        global_y_max,
                        global_y_avg,
                    )?;
                    evalexpr::eval_with_context(x_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_x_min = new_x_min??;

                let new_x_max: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        x_max.clone(),
                        self.y,
                        self.tag.unwrap(),
                        global_x_min,
                        global_x_max,
                        global_x_avg,
                        global_y_min,
                        global_y_max,
                        global_y_avg,
                    )?;
                    evalexpr::eval_with_context(x_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_x_max = new_x_max??;

                new_datapoint.add_x_value_confidence(c, (new_x_min, new_x_max));
            }

            if let Some((y_min, y_max)) = self.y_confidence.get(&c.try_into().unwrap()) {
                let new_y_min: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        self.x,
                        y_min.clone(),
                        self.tag.unwrap(),
                        global_x_min,
                        global_x_max,
                        global_x_avg,
                        global_y_min,
                        global_y_max,
                        global_y_avg,
                    )?;
                    evalexpr::eval_with_context(y_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_y_min = new_y_min??;

                let new_y_max: BencherResult<BencherResult<Value>> = {
                    let ctx = Self::get_evalexpr_context(
                        self.x,
                        y_max.clone(),
                        self.tag.unwrap(),
                        global_x_min,
                        global_x_max,
                        global_x_avg,
                        global_y_min,
                        global_y_max,
                        global_y_avg,
                    )?;
                    evalexpr::eval_with_context(y_expr, &ctx)
                        .map_err(|e| e.into())
                        .map(|v| v.try_into())
                };
                let new_y_max = new_y_max??;

                new_datapoint.add_y_value_confidence(c, (new_y_min, new_y_max));
            }
        }

        Ok(new_datapoint)
    }
}

impl std::fmt::Display for XYDatapoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let x_interval = {
            let mut interval = None;
            for c in SUPPORTED_CONFIDENCES {
                if let Some((min, max)) = self.x_confidence.get(&c) {
                    interval = Some((min, max));
                    break;
                }
            }
            interval
        };

        let y_interval = {
            let mut interval = None;
            for c in SUPPORTED_CONFIDENCES {
                if let Some((min, max)) = self.y_confidence.get(&c) {
                    interval = Some((min, max));
                    break;
                }
            }
            interval
        };

        match (self.tag, x_interval, y_interval) {
            (None, None, None) => write!(f, "[]({}, {})", self.x, self.y),
            (None, Some((x_min, x_max)), None) => {
                write!(f, "[]({} [{};{}], {})", self.x, x_min, x_max, self.y)
            }
            (None, None, Some((y_min, y_max))) => {
                write!(f, "[]({}, {} [{};{}])", self.x, self.y, y_min, y_max)
            }
            (None, Some((x_min, x_max)), Some((y_min, y_max))) => write!(
                f,
                "[]({} [{};{}], {} [{};{}])",
                self.x, x_min, x_max, self.y, y_min, y_max
            ),
            (Some(tag), None, None) => write!(f, "[{}]({}, {})", tag, self.x, self.y),
            (Some(tag), Some((x_min, x_max)), None) => {
                write!(f, "[{}]({} [{};{}], {})", tag, self.x, x_min, x_max, self.y)
            }
            (Some(tag), None, Some((y_min, y_max))) => {
                write!(f, "[{}]({}, {} [{};{}])", tag, self.x, self.y, y_min, y_max)
            }
            (Some(tag), Some((x_min, x_max)), Some((y_min, y_max))) => write!(
                f,
                "[{}]({} [{};{}], {} [{};{}])",
                tag, self.x, x_min, x_max, self.y, y_min, y_max
            ),
        }
    }
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
        {
            let c: BencherResult<Confidence> = (0 as usize).try_into();
            assert!(c.is_err())
        };
        {
            let c: BencherResult<Confidence> = (14 as usize).try_into();
            assert!(c.is_err())
        };
        {
            let c: BencherResult<Confidence> = (50 as usize).try_into();
            assert!(c.is_err())
        };

        {
            let c: Confidence = (1 as usize).try_into().unwrap();
            assert_eq!(c, Confidence::One)
        };
        {
            let c: Confidence = (5 as usize).try_into().unwrap();
            assert_eq!(c, Confidence::Five)
        };
        {
            let c: Confidence = (10 as usize).try_into().unwrap();
            assert_eq!(c, Confidence::Ten)
        };
        {
            let c: Confidence = (25 as usize).try_into().unwrap();
            assert_eq!(c, Confidence::TwentyFive)
        };
        {
            let c: Confidence = (99 as usize).try_into().unwrap();
            assert_eq!(c, Confidence::One)
        };
        {
            let c: Confidence = (95 as usize).try_into().unwrap();
            assert_eq!(c, Confidence::Five)
        };
        {
            let c: Confidence = (90 as usize).try_into().unwrap();
            assert_eq!(c, Confidence::Ten)
        };
        {
            let c: Confidence = (75 as usize).try_into().unwrap();
            assert_eq!(c, Confidence::TwentyFive)
        };
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
    fn linear_datapoint_from_sample_i64() {
        assert!(LinearDatapoint::from_sample_i64_median("", &mut vec![])
            .unwrap()
            .is_none());
        let mut sample: Vec<i64> = (0..100).into_iter().collect();
        let datapoint = LinearDatapoint::from_sample_i64_median("", &mut sample)
            .unwrap()
            .unwrap();
        assert_eq!(datapoint.v, Value::Int(50));
        assert_eq!(
            datapoint.get_confidence(1.try_into().unwrap()),
            Some((Value::Int(1), Value::Int(99)))
        );
        assert_eq!(
            datapoint.get_confidence(5.try_into().unwrap()),
            Some((Value::Int(5), Value::Int(95)))
        );
        assert_eq!(
            datapoint.get_confidence(10.try_into().unwrap()),
            Some((Value::Int(10), Value::Int(90)))
        );
        assert_eq!(
            datapoint.get_confidence(25.try_into().unwrap()),
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
    fn xy_datapoint_from_sample_i64() {
        assert!(XYDatapoint::from_samples_median(
            Either::Left(&mut vec![]),
            Either::Left(&mut vec![])
        )
        .is_none());
        assert!(XYDatapoint::from_samples_median(
            Either::Left(&mut vec![1]),
            Either::Left(&mut vec![])
        )
        .is_none());
        assert!(XYDatapoint::from_samples_median(
            Either::Left(&mut vec![]),
            Either::Left(&mut vec![1])
        )
        .is_none());
        let mut x_sample: Vec<i64> = (0..100).into_iter().collect();
        let mut y_sample: Vec<i64> = (1000..1100).rev().into_iter().collect();
        let datapoint = XYDatapoint::from_samples_median(
            Either::Left(&mut x_sample),
            Either::Left(&mut y_sample),
        )
        .unwrap();
        assert_eq!(datapoint.x, Value::Int(50));
        assert_eq!(datapoint.y, Value::Int(1050));
        assert_eq!(
            datapoint.get_x_confidence(1.try_into().unwrap()),
            Some((Value::Int(1), Value::Int(99)))
        );
        assert_eq!(
            datapoint.get_x_confidence(5.try_into().unwrap()),
            Some((Value::Int(5), Value::Int(95)))
        );
        assert_eq!(
            datapoint.get_x_confidence(10.try_into().unwrap()),
            Some((Value::Int(10), Value::Int(90)))
        );
        assert_eq!(
            datapoint.get_x_confidence(25.try_into().unwrap()),
            Some((Value::Int(25), Value::Int(75)))
        );
        assert_eq!(
            datapoint.get_y_confidence(1.try_into().unwrap()),
            Some((Value::Int(1001), Value::Int(1099)))
        );
        assert_eq!(
            datapoint.get_y_confidence(5.try_into().unwrap()),
            Some((Value::Int(1005), Value::Int(1095)))
        );
        assert_eq!(
            datapoint.get_y_confidence(10.try_into().unwrap()),
            Some((Value::Int(1010), Value::Int(1090)))
        );
        assert_eq!(
            datapoint.get_y_confidence(25.try_into().unwrap()),
            Some((Value::Int(1025), Value::Int(1075)))
        );
    }

    #[test]
    fn linear_datapoint_from_xy_datapoint() {
        let mut x_sample: Vec<i64> = (0..100).into_iter().collect();
        let mut y_sample: Vec<i64> = (1000..1100).rev().into_iter().collect();
        let datapoint = XYDatapoint::from_samples_median(
            Either::Left(&mut x_sample),
            Either::Left(&mut y_sample),
        )
        .unwrap();

        let x_datapoint = LinearDatapoint::from_sample_i64_median("", &mut x_sample)
            .unwrap()
            .unwrap();
        let y_datapoint = LinearDatapoint::from_sample_i64_median("", &mut y_sample)
            .unwrap()
            .unwrap();

        assert_eq!(x_datapoint, datapoint.x_linear(""));
        assert_eq!(y_datapoint, datapoint.y_linear(""));
    }
}
