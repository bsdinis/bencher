pub fn integer_avg(sample: &Vec<impl Into<i64> + Clone>) -> i64 {
    let sum: i64 = sample
        .iter()
        .map(|x| {
            let y: i64 = x.clone().into();
            y
        })
        .sum();
    sum / sample.len() as i64
}

pub fn integer_median(sorted_sample: &Vec<impl Into<i64> + Clone>) -> i64 {
    integer_percentile(sorted_sample, 50)
}

pub fn integer_percentile(sorted_sample: &Vec<impl Into<i64> + Clone>, percentile: usize) -> i64 {
    let n = (sorted_sample.len() as f64 * (percentile as f64 / 100.0)).ceil() as usize;
    if n < sorted_sample.len() {
        sorted_sample[n].clone().into()
    } else {
        sorted_sample[sorted_sample.len() - 1].clone().into()
    }
}

pub fn float_avg(sample: &Vec<impl Into<f64> + Clone>) -> f64 {
    let sum: f64 = sample
        .iter()
        .map(|x| {
            let y: f64 = x.clone().into();
            y
        })
        .sum();
    sum / sample.len() as f64
}

pub fn float_median(sorted_sample: &Vec<impl Into<f64> + Clone>) -> f64 {
    float_percentile(sorted_sample, 50)
}

pub fn float_percentile(sorted_sample: &Vec<impl Into<f64> + Clone>, percentile: usize) -> f64 {
    let n = (sorted_sample.len() as f64 * (percentile as f64 / 100.0)).ceil() as usize;
    if n < sorted_sample.len() {
        sorted_sample[n].clone().into()
    } else {
        sorted_sample[sorted_sample.len() - 1].clone().into()
    }
}
