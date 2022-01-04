use bencher::Config;
use clap::{App, Arg, SubCommand};
use cli_table::{format::Justify, Cell, Style, Table};
use eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let config = Config::new()?;
    let experiments = config.experiments();
    let lines = config.experiment_lines()?;

    let available_experiments = experiments
        .iter()
        .map(|e| e.exp_type.as_ref())
        .collect::<Vec<&str>>();

    let available_codes = lines.iter().map(|e| e.code.as_ref()).collect::<Vec<&str>>();

    let app = App::new("bencher")
        .version("0.1")
        .author("Baltasar D. <baltasar.dinis@tecnico.ulisboa.pt>")
        .about("Manage benchmark results")
        .subcommand(SubCommand::with_name("list").about("list the available experiments"))
        .subcommand(
            SubCommand::with_name("status")
                .about("queries the database to see how many datapoints we have per test label"),
        )
        .subcommand(
            SubCommand::with_name("table")
                .about("get the results for a specific experiment in tabular form")
                .arg(
                    Arg::with_name("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(&available_experiments),
                ),
        )
        .subcommand(
            SubCommand::with_name("latex")
                .about("get the results for a specific experiment in a latex table form")
                .arg(
                    Arg::with_name("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(&available_experiments),
                ),
        )
        .subcommand(
            SubCommand::with_name("dat")
                .about("get the results for a specific experiment in a .dat form")
                .arg(
                    Arg::with_name("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(&available_experiments),
                )
                .arg(
                    Arg::with_name("prefix")
                        .help("the prefix for the files")
                        .required(true),
                )
                .arg(
                    Arg::with_name("xbar")
                        .short("x")
                        .long("xbar")
                        .takes_value(true)
                        .value_name("percentile")
                        .possible_values(&["1", "5", "10", "25", "75", "90", "95", "99"])
                        .help("If toggled, output dat with x error bars, with the given percentile")
                        .required(false),
                )
                .arg(
                    Arg::with_name("ybar")
                        .short("y")
                        .long("ybar")
                        .takes_value(true)
                        .value_name("percentile")
                        .possible_values(&["1", "5", "10", "25", "75", "90", "95", "99"])
                        .help("If toggled, output dat with y error bars, with the given percentile")
                        .required(false),
                ),
        )
        .subcommand(
            SubCommand::with_name("gnuplot")
                .about("get the gnuplot representation for an experiment")
                .arg(
                    Arg::with_name("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(&available_experiments),
                )
                .arg(
                    Arg::with_name("prefix")
                        .help("the prefix for the files")
                        .required(true),
                )
                .arg(
                    Arg::with_name("xbar")
                        .short("x")
                        .short("xbar")
                        .help("If toggled, dump gnuplot with capacity to display x error bars")
                        .required(false),
                )
                .arg(
                    Arg::with_name("ybar")
                        .short("y")
                        .long("ybar")
                        .help("If toggled, dump gnuplot with capacity to display y error bars")
                        .required(false),
                ),
        )
        .subcommand(
            SubCommand::with_name("add")
                .about("add a new experiment")
                .arg(
                    Arg::with_name("experiment_type")
                        .help("the experiment to use")
                        .required(true)
                        .possible_values(&available_experiments),
                )
                .arg(
                    Arg::with_name("experiment_label")
                        .help("the label for this experiment")
                        .required(true),
                )
                .arg(
                    Arg::with_name("experiment_code")
                        .help("the code for this experiment")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("versions")
                .about("get the list of versions for an specific point from an experiment")
                .arg(
                    Arg::with_name("experiment_code")
                        .help("the code for this experiment")
                        .required(true)
                        .possible_values(&available_codes),
                )
                .arg(
                    Arg::with_name("tag")
                        .help("the tag to get versions from")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("revert")
                .about("revert a tag in an experiment (possible to a previous value)")
                .arg(
                    Arg::with_name("experiment_code")
                        .help("the code for this experiment")
                        .required(true)
                        .possible_values(&available_codes),
                )
                .arg(
                    Arg::with_name("tag")
                        .help("the tag to get versions from")
                        .required(true),
                )
                .arg(Arg::with_name("version").help("the (optional) version to revert to")),
        )
        .get_matches();

    match app.subcommand() {
        ("list", Some(_)) => list(&config)?,
        ("status", Some(_)) => status(&config)?,
        ("table", Some(matches)) => {
            config
                .get_xy_experiment_handle(matches.value_of("experiment_type").unwrap())?
                .dump_table()?;
        }
        ("latex", Some(matches)) => {
            config
                .get_xy_experiment_handle(matches.value_of("experiment_type").unwrap())?
                .dump_latex_table()?;
        }
        ("dat", Some(matches)) => {
            config
                .get_xy_experiment_handle(matches.value_of("experiment_type").unwrap())?
                .dump_dat(
                    matches.value_of("prefix").unwrap(),
                    matches
                        .value_of("xbar")
                        .map(|c| c.parse::<usize>().unwrap()),
                    matches
                        .value_of("ybar")
                        .map(|c| c.parse::<usize>().unwrap()),
                )?;
        }
        ("gnuplot", Some(matches)) => {
            config
                .get_xy_experiment_handle(matches.value_of("experiment_type").unwrap())?
                .dump_gnuplot(
                    matches.value_of("prefix").unwrap(),
                    matches.is_present("xbar"),
                    matches.is_present("ybar"),
                )?;
        }
        ("add", Some(matches)) => {
            config.add_xy_experiment(
                matches.value_of("experiment_type").unwrap(),
                matches.value_of("experiment_label").unwrap(),
                matches.value_of("experiment_code").unwrap(),
            )?;
        }
        ("versions", Some(matches)) => {
            let handle = config
                .get_xy_line_handle(matches.value_of("experiment_code").unwrap())
                .ok_or_else(|| {
                    eyre::eyre!(
                        "could not find experiment {}",
                        matches.value_of("experiment_code").unwrap()
                    )
                })?;
            let tag = matches.value_of("tag").unwrap().parse::<isize>()?;
            let version = handle.version(tag)?;
            let versions = handle.versions(tag)?;
            println!(
                "[{}:{}]: {}",
                matches.value_of("experiment_code").unwrap(),
                tag,
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
        ("revert", Some(matches)) => config
            .get_xy_line_handle(matches.value_of("experiment_code").unwrap())
            .ok_or_else(|| {
                eyre::eyre!(
                    "could not find experiment {}",
                    matches.value_of("experiment_code").unwrap()
                )
            })?
            .revert(
                matches.value_of("tag").unwrap().parse::<isize>()?,
                matches
                    .value_of("version")
                    .map(|v| v.parse::<usize>())
                    .transpose()?,
            )?,
        _ => {}
    }

    Ok(())
}

fn list(config: &Config) -> Result<()> {
    let table = config
        .experiment_lines()?
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

    cli_table::print_stdout(table)?;
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
