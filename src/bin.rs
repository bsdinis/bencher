use bencher::{Axis, Config, LinearDatapoint, Value, XYDatapoint};
use clap::{App, Arg};
use cli_table::{format::Justify, Cell, Style, Table};
use either::Either;
use eyre::Result;
use std::collections::HashMap;

fn main() -> Result<()> {
    color_eyre::install()?;

    let config = Config::new()?;
    let xy_experiments = config.xy_experiments();
    let xy_lines = config.xy_experiment_lines()?;
    let linear_experiments = config.linear_experiments();
    let linear_sets = config.linear_experiment_sets()?;

    let available_xy_experiments = xy_experiments
        .iter()
        .map(|e| e.exp_type.as_ref())
        .collect::<Vec<&str>>();

    let available_xy_codes = xy_lines
        .iter()
        .map(|e| e.code.as_ref())
        .collect::<Vec<&str>>();

    let available_linear_experiments = linear_experiments
        .iter()
        .map(|e| e.exp_type.as_ref())
        .collect::<Vec<&str>>();

    let available_linear_codes = linear_sets
        .iter()
        .map(|e| e.code.as_ref())
        .collect::<Vec<&str>>();

    let app = App::new("bencher")
        .version("0.2")
        .author("Baltasar D. <baltasar.dinis@tecnico.ulisboa.pt>")
        .about("Manage benchmark results")
        .subcommand(App::new("list").about("list the available experiments"))
        .subcommand(
            App::new("status")
                .about("queries the database to see how many datapoints we have per test label"),
        )
        .subcommand(
            App::new("table")
                .about("get the results for a specific experiment in tabular form")
                .arg(
                    Arg::new("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(available_xy_experiments.iter().chain(available_linear_experiments.iter())),
                ),
        )
        .subcommand(
            App::new("latex")
                .about("get the results for a specific experiment in a latex table form")
                .arg(
                    Arg::new("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(available_xy_experiments.iter().chain(available_linear_experiments.iter())),
                ),
        )
        .subcommand(
            App::new("dat")
                .about("get the results for a specific experiment in a .dat form")
                .arg(
                    Arg::new("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(available_xy_experiments.iter().chain(available_linear_experiments.iter())),
                )
                .arg(
                    Arg::new("prefix")
                        .help("the prefix for the files")
                        .required(true),
                )
                .arg(
                    Arg::new("bar")
                        .short('b')
                        .long("bar")
                        .takes_value(true)
                        .value_name("percentile")
                        .possible_values(&["1", "5", "10", "25", "75", "90", "95", "99"])
                        .help("If toggled, output dat with error bars, with the given percentile")
                        .required(false),
                )
                .arg(
                    Arg::new("xbar")
                        .short('x')
                        .long("xbar")
                        .takes_value(true)
                        .value_name("percentile")
                        .possible_values(&["1", "5", "10", "25", "75", "90", "95", "99"])
                        .help("If toggled, output dat with x error bars, with the given percentile")
                        .required(false),
                )
                .arg(
                    Arg::new("ybar")
                        .short('y')
                        .long("ybar")
                        .takes_value(true)
                        .value_name("percentile")
                        .possible_values(&["1", "5", "10", "25", "75", "90", "95", "99"])
                        .help("If toggled, output dat with y error bars, with the given percentile")
                        .required(false),
                ),
        )
        .subcommand(
            App::new("gnuplot")
                .about("get the gnuplot representation for an experiment")
                .arg(
                    Arg::new("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(available_xy_experiments.iter().chain(available_linear_experiments.iter())),
                )
                .arg(
                    Arg::new("prefix")
                        .help("the prefix for the files")
                        .required(true),
                )
                .arg(
                    Arg::new("bar")
                        .short('b')
                        .long("bar")
                        .help("If toggled, dump gnuplot with capacity to display error bars")
                        .required(false),
                )
                .arg(
                    Arg::new("xbar")
                        .short('x')
                        .long("xbar")
                        .help("If toggled, dump gnuplot with capacity to display x error bars")
                        .required(false),
                )
                .arg(
                    Arg::new("ybar")
                        .short('y')
                        .long("ybar")
                        .help("If toggled, dump gnuplot with capacity to display y error bars")
                        .required(false),
                ),
        )
        .subcommand(
            App::new("add")
                .about("add a new experiment")
                .arg(
                    Arg::new("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(available_xy_experiments.iter().chain(available_linear_experiments.iter())),
                )
                .arg(
                    Arg::new("experiment_label")
                        .help("the label for this experiment")
                        .required(true),
                )
                .arg(
                    Arg::new("experiment_code")
                        .help("the code for this experiment")
                        .required(true),
                ),
        )
        .subcommand(
            App::new("raw")
                .about("outputs a point (possible along one of the axis) from the experiment in the raw format `<percentile> <unnormalized value>`. Useful for manipulating points")
                .arg(
                    Arg::new("experiment_code")
                        .help("the code for this experiment")
                        .possible_values(available_xy_codes.iter().chain(available_linear_codes.iter())),
                )
                .arg(
                    Arg::new("tag")
                        .help("the tag to get")
                        .conflicts_with("group")
                        .required_unless_present("group")
                )
                .arg(
                    Arg::new("group")
                        .help("the group to get")
                        .conflicts_with("tag")
                        .required_unless_present("tag")
                )
                .arg( Arg::new("x").short('x').conflicts_with("group").conflicts_with("y"))
                .arg( Arg::new("y").short('y').conflicts_with("group").conflicts_with("x"))
                ,
        )
        .subcommand(
            App::new("raw_add")
                .about("add a new datapoint to an experiment from std in. Note that this is designed for manipulation of existing points, to be used in conjunction with the raw get command")
                .arg(
                    Arg::new("experiment_code")
                        .help("the code for this experiment")
                        .required(true)
                        .possible_values(available_xy_codes.iter().chain(available_linear_codes.iter())),
                )
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .help("the tag to get")
                        .conflicts_with("group")
                        .required_unless_present("group")
                        .takes_value(true)
                )
                .arg(
                    Arg::new("group")
                        .long("group")
                        .help("the group to get")
                        .conflicts_with("tag")
                        .required_unless_present("tag")
                        .takes_value(true)
                )
                .arg( Arg::new("x")
                      .short('x')
                      .help("value of the x coordinate")
                      .conflicts_with("group")
                      .conflicts_with("y")
                      .required_unless_present("group")
                      .takes_value(true))
                .arg( Arg::new("y")
                      .short('y')
                      .help("value of the y coordinate")
                      .conflicts_with("group")
                      .conflicts_with("x")
                      .required_unless_present("group")
                      .takes_value(true))
                ,
        )
        .subcommand(
            App::new("versions")
                .about("get the list of versions for an specific point from an experiment")
                .arg(
                    Arg::new("experiment_code")
                        .help("the code for this experiment")
                        .required(true)
                        .possible_values(available_xy_codes.iter().chain(available_linear_codes.iter())),
                )
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .help("the tag to get")
                        .conflicts_with("group")
                        .required_unless_present("group")
                        .takes_value(true)
                )
                .arg(
                    Arg::new("group")
                        .long("group")
                        .help("the group to get")
                        .conflicts_with("tag")
                        .required_unless_present("tag")
                        .takes_value(true)
                )
        )
        .subcommand(
            App::new("revert")
                .about("revert a value in an experiment (possible to a specific previous value)")
                .arg(
                    Arg::new("experiment_code")
                        .help("the code for this experiment")
                        .required(true)
                        .possible_values(available_xy_codes.iter().chain(available_linear_codes.iter())),
                )
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .help("the tag to get")
                        .conflicts_with("group")
                        .required_unless_present("group")
                        .takes_value(true)
                )
                .arg(
                    Arg::new("group")
                        .long("group")
                        .help("the group to get")
                        .conflicts_with("tag")
                        .required_unless_present("tag")
                        .takes_value(true)
                )
                .arg(Arg::new("version").help("the (optional) version to revert to")),
        )
        .get_matches();

    match app.subcommand() {
        Some(("list", _)) => list(&config)?,
        Some(("status", _)) => status(&config)?,
        Some(("table", matches)) => {
            let exp_type = matches.value_of("experiment_type").unwrap();
            if available_xy_experiments.contains(&exp_type) {
                config.get_xy_experiment_handle(exp_type)?.dump_table()?;
            } else {
                config
                    .get_linear_experiment_handle(exp_type)?
                    .dump_table()?;
            }
        }
        Some(("latex", matches)) => {
            let exp_type = matches.value_of("experiment_type").unwrap();
            if available_xy_experiments.contains(&exp_type) {
                config
                    .get_xy_experiment_handle(exp_type)?
                    .dump_latex_table()?;
            } else {
                config
                    .get_linear_experiment_handle(exp_type)?
                    .dump_latex_table()?;
            }
        }
        Some(("dat", matches)) => {
            let exp_type = matches.value_of("experiment_type").unwrap();
            let prefix = matches.value_of("prefix").unwrap();
            if available_xy_experiments.contains(&exp_type) {
                config.get_xy_experiment_handle(exp_type)?.dump_dat(
                    prefix,
                    matches
                        .value_of("xbar")
                        .map(|c| c.parse::<usize>().unwrap()),
                    matches
                        .value_of("ybar")
                        .map(|c| c.parse::<usize>().unwrap()),
                )?;
            } else {
                config
                    .get_linear_experiment_handle(matches.value_of("experiment_type").unwrap())?
                    .dump_dat(
                        prefix,
                        matches.value_of("bar").map(|c| c.parse::<usize>().unwrap()),
                    )?;
            }
        }
        Some(("gnuplot", matches)) => {
            let exp_type = matches.value_of("experiment_type").unwrap();
            let prefix = matches.value_of("prefix").unwrap();
            if available_xy_experiments.contains(&exp_type) {
                config.get_xy_experiment_handle(exp_type)?.dump_gnuplot(
                    prefix,
                    matches.is_present("xbar"),
                    matches.is_present("ybar"),
                )?;
            } else {
                config
                    .get_linear_experiment_handle(matches.value_of("experiment_type").unwrap())?
                    .dump_gnuplot(prefix, matches.is_present("bar"))?;
            }
        }
        Some(("add", matches)) => {
            let exp_type = matches.value_of("experiment_type").unwrap();
            if config.has_xy_experiment(&exp_type) {
                config.add_xy_experiment(
                    exp_type,
                    matches.value_of("experiment_label").unwrap(),
                    matches.value_of("experiment_code").unwrap(),
                )?;
            } else if config.has_linear_experiment(&exp_type) {
                config.add_linear_experiment(
                    exp_type,
                    matches.value_of("experiment_label").unwrap(),
                    matches.value_of("experiment_code").unwrap(),
                )?;
            } else {
                return Err(eyre::eyre!("unknown experiment type: {}", exp_type));
            }
        }
        Some(("raw", matches)) => {
            let exp_code = matches.value_of("experiment_code").unwrap();
            if available_xy_codes.contains(&exp_code) {
                let handle = config.get_xy_line_handle(&exp_code).ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
                let tag = matches.value_of("tag").unwrap().parse::<isize>()?;
                handle.dump_raw(
                    tag,
                    if matches.is_present("x") {
                        Axis::X
                    } else {
                        Axis::Y
                    },
                )?;
            } else {
                let handle = config.get_linear_set_handle(&exp_code).ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
                let group = matches.value_of("tag").unwrap();
                handle.dump_raw(group)?;
            }
        }
        Some(("raw_add", matches)) => {
            let exp_code = matches.value_of("experiment_code").unwrap();
            if available_xy_codes.contains(&exp_code) {
                let handle = config.get_xy_line_handle(&exp_code).ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
                let tag = matches.value_of("tag").unwrap().parse::<isize>()?;
                let datapoint =
                    raw_xy_datapoint_from_stdin(matches.value_of("x"), matches.value_of("y"))?
                        .tag(tag);
                handle.add_datapoint(datapoint)?;
            } else {
                let handle = config.get_linear_set_handle(&exp_code).ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
                let group = matches.value_of("group").unwrap();
                let datapoint = raw_linear_datapoint_from_stdin(group.to_string())?;
                handle.add_datapoint(datapoint)?;
            }
        }
        Some(("versions", matches)) => {
            let exp_code = matches.value_of("experiment_code").unwrap();
            let (id, version, versions) = if available_xy_codes.contains(&exp_code) {
                let handle = config.get_xy_line_handle(&exp_code).ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
                let tag = matches.value_of("tag").unwrap().parse::<isize>()?;
                let version = handle.version(tag)?;
                let versions = handle.versions(tag)?;
                (format!("{}", tag), version, versions)
            } else {
                let handle = config.get_linear_set_handle(&exp_code).ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
                let group = matches.value_of("group").unwrap();
                let version = handle.version(group)?;
                let versions = handle.versions(group)?;
                (group.to_string(), version, versions)
            };
            println!(
                "[{}:{}]: {}",
                matches.value_of("experiment_code").unwrap(),
                id,
                versions
                    .into_iter()
                    .map(|v| if v == version {
                        format!("[{}]", v)
                    } else {
                        format!("{}", v)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Some(("revert", matches)) => {
            let exp_code = matches.value_of("experiment_code").unwrap();
            let version = matches
                .value_of("version")
                .map(|v| v.parse::<usize>())
                .transpose()?;
            if available_xy_codes.contains(&exp_code) {
                let handle = config.get_xy_line_handle(&exp_code).ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
                handle.revert(matches.value_of("tag").unwrap().parse::<isize>()?, version)?;
            } else {
                let handle = config.get_linear_set_handle(&exp_code).ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
                handle.revert(matches.value_of("group").unwrap(), version)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn list(config: &Config) -> Result<()> {
    let xy_table = config
        .xy_experiment_lines()?
        .into_iter()
        .map(|e| {
            vec![
                e.code.cell().justify(Justify::Center).bold(true),
                e.experiment
                    .exp_type
                    .cell()
                    .justify(Justify::Center)
                    .bold(true),
                e.label.cell().justify(Justify::Center).bold(true),
                e.experiment.x_label.cell().justify(Justify::Center),
                e.experiment.x_units.cell().justify(Justify::Center),
                e.experiment.y_label.cell().justify(Justify::Center),
                e.experiment.y_units.cell().justify(Justify::Center),
            ]
        })
        .collect::<Vec<_>>()
        .table()
        .title(vec![
            "Code".cell().justify(Justify::Center).bold(true),
            "Type".cell().justify(Justify::Center).bold(true),
            "Label".cell().justify(Justify::Center).bold(true),
            "x label".cell().justify(Justify::Center).bold(true),
            "x units".cell().justify(Justify::Center).bold(true),
            "y label".cell().justify(Justify::Center).bold(true),
            "y units".cell().justify(Justify::Center).bold(true),
        ])
        .bold(true);

    cli_table::print_stdout(xy_table)?;

    let linear_table = config
        .linear_experiment_sets()?
        .into_iter()
        .map(|e| {
            vec![
                e.code.cell().justify(Justify::Center).bold(true),
                e.experiment
                    .exp_type
                    .cell()
                    .justify(Justify::Center)
                    .bold(true),
                e.label.cell().justify(Justify::Center).bold(true),
                e.experiment
                    .horizontal_label
                    .cell()
                    .justify(Justify::Center),
                e.experiment.v_label.cell().justify(Justify::Center),
                e.experiment.v_units.cell().justify(Justify::Center),
            ]
        })
        .collect::<Vec<_>>()
        .table()
        .title(vec![
            "Code".cell().justify(Justify::Center).bold(true),
            "Type".cell().justify(Justify::Center).bold(true),
            "Label".cell().justify(Justify::Center).bold(true),
            "horizontal label "
                .cell()
                .justify(Justify::Center)
                .bold(true),
            "v label".cell().justify(Justify::Center).bold(true),
            "v units".cell().justify(Justify::Center).bold(true),
        ])
        .bold(true);

    cli_table::print_stdout(linear_table)?;
    Ok(())
}

fn status(config: &Config) -> Result<()> {
    let table = config
        .status()?
        .into_iter()
        .map(|s| {
            vec![
                s.code.cell().justify(Justify::Center).bold(true),
                s.exp_type.cell().justify(Justify::Center).bold(true),
                s.label.cell().justify(Justify::Center).bold(true),
                s.n_active_datapoints.cell().justify(Justify::Right),
                s.n_datapoints.cell().justify(Justify::Right),
            ]
        })
        .collect::<Vec<_>>()
        .table()
        .title(vec![
            "Code".cell().justify(Justify::Center).bold(true),
            "Type".cell().justify(Justify::Center).bold(true),
            "Label".cell().justify(Justify::Center).bold(true),
            "#Active Datapoints"
                .cell()
                .justify(Justify::Center)
                .bold(true),
            "#Datapoints".cell().justify(Justify::Center).bold(true),
        ])
        .bold(true);

    cli_table::print_stdout(table)?;
    Ok(())
}

fn raw_linear_datapoint_from_stdin(group: String) -> Result<LinearDatapoint> {
    let mut raw_percentiles = HashMap::with_capacity(10);
    let mut buffer = String::new();
    let mut line = 0;
    loop {
        match std::io::stdin().read_line(&mut buffer) {
            Ok(0) => break, // EOF
            Err(e) => Err(e)?,
            Ok(_) => {
                let mut iter = buffer.split_whitespace();
                let percentile = if let Some(p) = iter.next() {
                    p.parse::<usize>()?
                } else {
                    return Err(eyre::eyre!("no data in line {}", line));
                };

                let value = if let Some(v) = iter.next() {
                    if let Ok(i) = v.parse::<i64>() {
                        Either::Left(i)
                    } else {
                        Either::Right(v.parse::<f64>()?)
                    }
                } else {
                    return Err(eyre::eyre!("no second field in line {}", line));
                };

                raw_percentiles.insert(percentile, value);
                buffer.clear();
            }
        }

        line += 0;
    }
    if !raw_percentiles.contains_key(&50) {
        return Err(eyre::eyre!("missing median (key = 50)"));
    }

    let mut percentiles = if raw_percentiles.values().all(|x| x.is_left()) {
        raw_percentiles
            .into_iter()
            .map(|(k, v)| (k, Value::Int(v.left().unwrap())))
            .collect::<HashMap<usize, _>>()
    } else {
        raw_percentiles
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    Value::Float(v.right().or(v.left().map(|x| x as f64)).unwrap()),
                )
            })
            .collect::<HashMap<usize, _>>()
    };

    let mut datapoint = LinearDatapoint::new(group, percentiles.remove(&50).unwrap());
    for c in &[1, 5, 10, 25] {
        let simmetric = 100 - c;
        if percentiles.contains_key(c) && percentiles.contains_key(&simmetric) {
            match (
                percentiles.remove(c).unwrap(),
                percentiles.remove(&simmetric).unwrap(),
            ) {
                (Value::Int(min), Value::Int(max)) => {
                    datapoint.add_confidence(*c, Either::Left((min, max)))?
                }
                (Value::Float(min), Value::Float(max)) => {
                    datapoint.add_confidence(*c, Either::Right((min, max)))?
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(datapoint)
}

fn raw_xy_datapoint_from_stdin(x: Option<&str>, y: Option<&str>) -> Result<XYDatapoint> {
    let mut raw_percentiles = HashMap::with_capacity(10);
    let mut buffer = String::new();
    let mut line = 0;
    loop {
        match std::io::stdin().read_line(&mut buffer) {
            Ok(0) => break, // EOF
            Err(e) => Err(e)?,
            Ok(_) => {
                let mut iter = buffer.split_whitespace();
                let percentile = if let Some(p) = iter.next() {
                    p.parse::<usize>()?
                } else {
                    return Err(eyre::eyre!("no data in line {}", line));
                };

                let value = if let Some(v) = iter.next() {
                    if let Ok(i) = v.parse::<i64>() {
                        Either::Left(i)
                    } else {
                        Either::Right(v.parse::<f64>()?)
                    }
                } else {
                    return Err(eyre::eyre!("no second field in line {}", line));
                };

                raw_percentiles.insert(percentile, value);
                buffer.clear();
            }
        }

        line += 0;
    }
    if !raw_percentiles.contains_key(&50) {
        return Err(eyre::eyre!("missing median (key = 50)"));
    }

    let mut percentiles = if raw_percentiles.values().all(|x| x.is_left()) {
        raw_percentiles
            .into_iter()
            .map(|(k, v)| (k, Value::Int(v.left().unwrap())))
            .collect::<HashMap<usize, _>>()
    } else {
        raw_percentiles
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    Value::Float(v.right().or(v.left().map(|x| x as f64)).unwrap()),
                )
            })
            .collect::<HashMap<usize, _>>()
    };

    let x = x
        .map(|v| {
            if let Ok(a) = v.parse::<i64>() {
                Ok(Value::Int(a))
            } else {
                v.parse::<f64>().map(|b| Value::Float(b))
            }
        })
        .transpose()?;
    let y = y
        .map(|v| {
            if let Ok(a) = v.parse::<i64>() {
                Ok(Value::Int(a))
            } else {
                v.parse::<f64>().map(|b| Value::Float(b))
            }
        })
        .transpose()?;

    match (x, y) {
        (Some(x), _) => {
            let mut datapoint = XYDatapoint::new(x, percentiles.remove(&50).unwrap());
            for c in &[1, 5, 10, 25] {
                let simmetric = 100 - c;
                if percentiles.contains_key(c) && percentiles.contains_key(&simmetric) {
                    match (
                        percentiles.remove(c).unwrap(),
                        percentiles.remove(&simmetric).unwrap(),
                    ) {
                        (Value::Int(min), Value::Int(max)) => {
                            datapoint.add_y_confidence(*c, Either::Left((min, max)))?
                        }
                        (Value::Float(min), Value::Float(max)) => {
                            datapoint.add_y_confidence(*c, Either::Right((min, max)))?
                        }
                        _ => unreachable!(),
                    }
                }
            }

            Ok(datapoint)
        }
        (_, Some(y)) => {
            let mut datapoint = XYDatapoint::new(percentiles.remove(&50).unwrap(), y);
            for c in &[1, 5, 10, 25] {
                let simmetric = 100 - c;
                if percentiles.contains_key(c) && percentiles.contains_key(&simmetric) {
                    match (
                        percentiles.remove(c).unwrap(),
                        percentiles.remove(&(100 - c)).unwrap(),
                    ) {
                        (Value::Int(min), Value::Int(max)) => {
                            datapoint.add_x_confidence(*c, Either::Left((min, max)))?
                        }
                        (Value::Float(min), Value::Float(max)) => {
                            datapoint.add_x_confidence(*c, Either::Right((min, max)))?
                        }
                        _ => unreachable!(),
                    }
                }
            }

            Ok(datapoint)
        }
        _ => return Err(eyre::eyre!("neither x nor y is set")),
    }
}
