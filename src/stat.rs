pub fn integer_median(sorted_sample: &Vec<i64>) -> i64 {
    integer_percentile(sorted_sample, 50)
}

pub fn integer_percentile(sorted_sample: &Vec<i64>, percentile: usize) -> i64 {
    let n = (sorted_sample.len() as f64 * (percentile as f64 / 100.0)).ceil() as usize;
    if n < sorted_sample.len() {
        sorted_sample[n]
    } else {
        sorted_sample[sorted_sample.len() - 1]
    }
}

pub fn float_median(sorted_sample: &Vec<f64>) -> f64 {
    float_percentile(sorted_sample, 50)
}

pub fn float_percentile(sorted_sample: &Vec<f64>, percentile: usize) -> f64 {
    let n = (sorted_sample.len() as f64 * (percentile as f64 / 100.0)).ceil() as usize;
    if n < sorted_sample.len() {
        sorted_sample[n]
    } else {
        sorted_sample[sorted_sample.len() - 1]
    }
}
