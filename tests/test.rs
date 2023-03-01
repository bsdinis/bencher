use bencher::*;
use std::collections::HashSet;

fn gen_xy_experiments() -> Vec<XYExperiment> {
    vec![
        XYExperiment {
            exp_type: "Throughput Latency".to_string(),
            x_label: "Throughput".to_string(),
            x_units: "ops/s".to_string(),
            y_label: "Latency".to_string(),
            y_units: "s".to_string(),
        },
        XYExperiment {
            exp_type: "Throughput".to_string(),
            x_label: "Offered Load".to_string(),
            x_units: "ops/s".to_string(),
            y_label: "Throughput".to_string(),
            y_units: "ops/s".to_string(),
        },
    ]
}

fn gen_linear_experiments() -> Vec<LinearExperiment> {
    vec![
        LinearExperiment {
            exp_type: "Operational Latency".to_string(),
            horizontal_label: "Operation".to_string(),
            v_label: "Latency".to_string(),
            v_units: "s".to_string(),
        },
        LinearExperiment {
            exp_type: "Operational Throughput".to_string(),
            horizontal_label: "Operation".to_string(),
            v_label: "Throughput".to_string(),
            v_units: "ops/s".to_string(),
        },
    ]
}

fn gen_inner_config() -> ParsedConfig {
    ParsedConfig {
        default_database_filepath: "".to_string(),
        xy_experiments: gen_xy_experiments(),
        linear_experiments: gen_linear_experiments(),
    }
}

fn gen_in_memory_write_config() -> WriteConfig {
    let conf =
        WriteConfig::from_conn_and_config(rusqlite::Connection::open_in_memory().unwrap()).unwrap();

    conf.add_xy_line("Throughput Latency", "Write", "tput_lat")
        .unwrap();
    conf.add_xy_line("Throughput", "Read", "tput_1").unwrap();
    conf.add_xy_line("Throughput", "Write", "tput_2").unwrap();
    conf.add_xy_line("Throughput", "Version", "tput_version")
        .unwrap();

    conf.add_linear_set("Operational Latency", "System A", "linear_lat_a")
        .unwrap();
    conf.add_linear_set("Operational Latency", "System B", "linear_lat_b")
        .unwrap();
    conf.add_linear_set("Operational Throughput", "System A", "linear_tput_a")
        .unwrap();
    conf.add_linear_set("Operational Throughput", "System B", "linear_tput_b")
        .unwrap();

    conf
}

fn gen_in_memory_read_config() -> ReadConfig {
    gen_in_memory_write_config()
        .to_read_config(gen_inner_config())
        .unwrap()
}

fn linear_populate<'a>(handle: &LinearSetHandle<'a>, points: &[(&str, i64)]) {
    for (label, value) in points {
        handle
            .add_datapoint(LinearDatapoint::new(label.to_string(), Value::Int(*value)))
            .unwrap();
    }
}

#[test]
fn config_experiments() {
    let config = gen_in_memory_read_config();
    assert_eq!(config.xy_experiments().clone(), gen_xy_experiments());
}

#[test]
fn config_empty_status() {
    let config = gen_in_memory_read_config();
    assert_eq!(
        config.status().unwrap().into_iter().collect::<HashSet<_>>(),
        vec![
            ExperimentStatus {
                database: ":memory:".to_string(),
                exp_code: "tput_1".to_string(),
                exp_type: "Throughput".to_string(),
                exp_label: "Read".to_string(),
                n_datapoints: 0,
                n_active_datapoints: 0,
            },
            ExperimentStatus {
                database: ":memory:".to_string(),
                exp_code: "tput_lat".to_string(),
                exp_type: "Throughput Latency".to_string(),
                exp_label: "Write".to_string(),
                n_datapoints: 0,
                n_active_datapoints: 0,
            },
            ExperimentStatus {
                database: ":memory:".to_string(),
                exp_code: "tput_version".to_string(),
                exp_type: "Throughput".to_string(),
                exp_label: "Version".to_string(),
                n_datapoints: 0,
                n_active_datapoints: 0,
            },
            ExperimentStatus {
                database: ":memory:".to_string(),
                exp_code: "tput_2".to_string(),
                exp_type: "Throughput".to_string(),
                exp_label: "Write".to_string(),
                n_datapoints: 0,
                n_active_datapoints: 0,
            },
            ExperimentStatus {
                database: ":memory:".to_string(),
                exp_code: "linear_lat_a".to_string(),
                exp_type: "Operational Latency".to_string(),
                exp_label: "System A".to_string(),
                n_datapoints: 0,
                n_active_datapoints: 0,
            },
            ExperimentStatus {
                database: ":memory:".to_string(),
                exp_code: "linear_lat_b".to_string(),
                exp_type: "Operational Latency".to_string(),
                exp_label: "System B".to_string(),
                n_datapoints: 0,
                n_active_datapoints: 0,
            },
            ExperimentStatus {
                database: ":memory:".to_string(),
                exp_code: "linear_tput_a".to_string(),
                exp_type: "Operational Throughput".to_string(),
                exp_label: "System A".to_string(),
                n_datapoints: 0,
                n_active_datapoints: 0,
            },
            ExperimentStatus {
                database: ":memory:".to_string(),
                exp_code: "linear_tput_b".to_string(),
                exp_type: "Operational Throughput".to_string(),
                exp_label: "System B".to_string(),
                n_datapoints: 0,
                n_active_datapoints: 0,
            },
        ]
        .into_iter()
        .collect::<HashSet<_>>()
    );
}

#[test]
fn linear_config_get_handle() {
    let config = gen_in_memory_write_config();
    assert!(config.get_linear_set("non_linear_lat_a").unwrap().is_none());
    assert!(config.get_linear_set("linear_lat_a").unwrap().is_some());
}

#[test]
fn linear_can_populate() {
    let config = gen_in_memory_write_config();
    let handle_a = config.get_linear_set("linear_lat_a").unwrap().unwrap();
    let handle_b = config.get_linear_set("linear_lat_b").unwrap().unwrap();
    linear_populate(&handle_a, &[("get", 12), ("put", 42)]);
    linear_populate(&handle_b, &[("get", 6), ("put", 20)]);
}
