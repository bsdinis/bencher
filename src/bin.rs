use bencher::{
    Bars, BencherError, ExperimentView, ReadConfig, Selector, SelectorBuilder, WriteConfig,
};

use clap::{Parser, Subcommand};
use cli_table::{format::Justify, Cell, Style, Table};
use eyre::Result;
use std::fs::File;

#[derive(Parser)]
#[command(author, version, about, long_about)]
struct Cli {
    /// Whether to use the default db or not
    #[arg(short, long)]
    default: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    List {
        #[arg(short, long)]
        exclude_code_regex: Vec<String>,

        #[arg(short, long)]
        include_code_regex: Vec<String>,

        #[arg(long)]
        exclude_type_regex: Vec<String>,

        #[arg(long)]
        include_type_regex: Vec<String>,

        /// Paths to DBs
        dbs: Vec<std::path::PathBuf>,
    },
    Status {
        #[arg(short, long)]
        exclude_code_regex: Vec<String>,

        #[arg(short, long)]
        include_code_regex: Vec<String>,

        #[arg(long)]
        exclude_type_regex: Vec<String>,

        #[arg(long)]
        include_type_regex: Vec<String>,

        /// Paths to DBs
        dbs: Vec<std::path::PathBuf>,
    },
    Table {
        exp_type: String,

        #[arg(short, long)]
        exclude_code_regex: Vec<String>,

        #[arg(short, long)]
        include_code_regex: Vec<String>,

        #[arg(long)]
        exclude_type_regex: Vec<String>,

        #[arg(long)]
        include_type_regex: Vec<String>,

        /// Paths to DBs
        dbs: Vec<std::path::PathBuf>,
    },
    Latex {
        exp_type: String,

        #[arg(short, long)]
        file: Option<std::path::PathBuf>,

        #[arg(short, long)]
        exclude_code_regex: Vec<String>,

        #[arg(short, long)]
        include_code_regex: Vec<String>,

        #[arg(long)]
        exclude_type_regex: Vec<String>,

        #[arg(long)]
        include_type_regex: Vec<String>,

        /// Paths to DBs
        dbs: Vec<std::path::PathBuf>,
    },
    Dat {
        exp_type: String,

        prefix: std::path::PathBuf,

        #[arg(short, long)]
        bar: Option<usize>,

        #[arg(short, long)]
        xbar: Option<usize>,

        #[arg(short, long)]
        ybar: Option<usize>,

        #[arg(short, long)]
        exclude_code_regex: Vec<String>,

        #[arg(short, long)]
        include_code_regex: Vec<String>,

        #[arg(long)]
        exclude_type_regex: Vec<String>,

        #[arg(long)]
        include_type_regex: Vec<String>,

        /// Paths to DBs
        dbs: Vec<std::path::PathBuf>,
    },
    Gnuplot {
        exp_type: String,

        prefix: std::path::PathBuf,

        #[arg(short, long)]
        bar: bool,

        #[arg(short, long)]
        xbar: bool,

        #[arg(short, long)]
        ybar: bool,

        #[arg(short, long)]
        exclude_code_regex: Vec<String>,

        #[arg(long)]
        include_code_regex: Vec<String>,

        #[arg(long)]
        exclude_type_regex: Vec<String>,

        #[arg(short, long)]
        include_type_regex: Vec<String>,

        /// Paths to DBs
        dbs: Vec<std::path::PathBuf>,
    },
    Plot {
        exp_type: String,

        prefix: std::path::PathBuf,

        #[arg(short, long)]
        bar: Option<usize>,

        #[arg(short, long)]
        xbar: Option<usize>,

        #[arg(short, long)]
        ybar: Option<usize>,

        #[arg(short, long)]
        exclude_code_regex: Vec<String>,

        #[arg(short, long)]
        include_code_regex: Vec<String>,

        #[arg(long)]
        exclude_type_regex: Vec<String>,

        #[arg(long)]
        include_type_regex: Vec<String>,

        /// Paths to DBs
        dbs: Vec<std::path::PathBuf>,
    },
    Revert {
        code: String,

        version: Option<usize>,

        /// Paths to DB
        #[arg(short, long)]
        db: Option<std::path::PathBuf>,

        #[arg(short, long)]
        tag: Option<isize>,

        #[arg(short, long)]
        group: Option<String>,
    },
}

fn get_read_config(default: bool, dbs: Vec<std::path::PathBuf>) -> Result<ReadConfig> {
    if default {
        ReadConfig::with_dbs_and_default(dbs.iter().map(|p| p.as_path())).map_err(|e| e.into())
    } else {
        ReadConfig::with_dbs(dbs.iter().map(|p| p.as_path())).map_err(|e| e.into())
    }
}

fn get_write_config(db: Option<std::path::PathBuf>) -> Result<WriteConfig> {
    if let Some(db) = db {
        WriteConfig::from_file(&db).map_err(|e| e.into())
    } else {
        WriteConfig::new().map_err(|e| e.into())
    }
}

fn build_selector(
    exclude_code_regex: &Vec<String>,
    include_code_regex: &Vec<String>,
    exclude_type_regex: &Vec<String>,
    include_type_regex: &Vec<String>,
) -> Result<Selector> {
    let exclude_code_regex = exclude_code_regex.iter().map(|re| regex::Regex::new(re));
    let include_code_regex = include_code_regex.iter().map(|re| regex::Regex::new(re));
    let exclude_type_regex = exclude_type_regex.iter().map(|re| regex::Regex::new(re));
    let include_type_regex = include_type_regex.iter().map(|re| regex::Regex::new(re));

    let mut builder = SelectorBuilder::new();
    for re in exclude_code_regex {
        let re = re.map_err(|e| eyre::eyre!("regex error: {:?}", e))?;
        builder = builder.code_exclude(re);
    }
    for re in include_code_regex {
        let re = re.map_err(|e| eyre::eyre!("regex error: {:?}", e))?;
        builder = builder.code_exclude(re);
    }
    for re in exclude_type_regex {
        let re = re.map_err(|e| eyre::eyre!("regex error: {:?}", e))?;
        builder = builder.type_exclude(re);
    }
    for re in include_type_regex {
        let re = re.map_err(|e| eyre::eyre!("regex error: {:?}", e))?;
        builder = builder.type_exclude(re);
    }
    Ok(builder.build())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Command::List {
            dbs,
            exclude_code_regex,
            include_code_regex,
            exclude_type_regex,
            include_type_regex,
        } => {
            let selector = build_selector(
                &exclude_code_regex,
                &include_code_regex,
                &exclude_type_regex,
                &include_type_regex,
            )?;
            let config = get_read_config(cli.default, dbs)?;
            list(&config, &selector)?;
        }
        Command::Status {
            dbs,
            exclude_code_regex,
            include_code_regex,
            exclude_type_regex,
            include_type_regex,
        } => {
            let selector = build_selector(
                &exclude_code_regex,
                &include_code_regex,
                &exclude_type_regex,
                &include_type_regex,
            )?;
            let config = get_read_config(cli.default, dbs)?;
            status(&config, &selector)?;
        }
        Command::Table {
            dbs,
            exclude_code_regex,
            include_code_regex,
            exclude_type_regex,
            include_type_regex,
            exp_type,
        } => {
            let selector = build_selector(
                &exclude_code_regex,
                &include_code_regex,
                &exclude_type_regex,
                &include_type_regex,
            )?;
            let config = get_read_config(cli.default, dbs)?;
            table(&config, &exp_type, &selector)?;
        }
        Command::Latex {
            dbs,
            exclude_code_regex,
            include_code_regex,
            exclude_type_regex,
            include_type_regex,
            exp_type,
            file,
        } => {
            let selector = build_selector(
                &exclude_code_regex,
                &include_code_regex,
                &exclude_type_regex,
                &include_type_regex,
            )?;
            let config = get_read_config(cli.default, dbs)?;
            latex(
                &config,
                &exp_type,
                file.as_ref().map(|x| x.as_path()),
                &selector,
            )?;
        }
        Command::Dat {
            dbs,
            exclude_code_regex,
            include_code_regex,
            exclude_type_regex,
            include_type_regex,
            exp_type,
            prefix,
            bar,
            xbar,
            ybar,
        } => {
            let selector = build_selector(
                &exclude_code_regex,
                &include_code_regex,
                &exclude_type_regex,
                &include_type_regex,
            )?;
            let config = get_read_config(cli.default, dbs)?;
            dat(
                &config,
                &exp_type,
                prefix.as_path(),
                bar,
                xbar,
                ybar,
                &selector,
            )?;
        }
        Command::Gnuplot {
            dbs,
            exclude_code_regex,
            include_code_regex,
            exclude_type_regex,
            include_type_regex,
            exp_type,
            prefix,
            bar,
            xbar,
            ybar,
        } => {
            let selector = build_selector(
                &exclude_code_regex,
                &include_code_regex,
                &exclude_type_regex,
                &include_type_regex,
            )?;
            let config = get_read_config(cli.default, dbs)?;
            gnuplot(&config, &exp_type, &prefix, bar, xbar, ybar, &selector)?;
        }
        Command::Plot {
            dbs,
            exclude_code_regex,
            include_code_regex,
            exclude_type_regex,
            include_type_regex,
            exp_type,
            prefix,
            bar,
            xbar,
            ybar,
        } => {
            let selector = build_selector(
                &exclude_code_regex,
                &include_code_regex,
                &exclude_type_regex,
                &include_type_regex,
            )?;
            let config = get_read_config(cli.default, dbs)?;
            plot(&config, &exp_type, &prefix, bar, xbar, ybar, &selector)?;
        }
        Command::Revert {
            db,
            code,
            version,
            tag,
            group,
        } => {
            let config = get_write_config(db)?;
            revert(&config, &code, tag, group.as_ref(), version)?;
        }
    }

    Ok(())
}

fn list(config: &ReadConfig, selector: &Selector) -> Result<()> {
    let linear_list = config.list_linear_experiments(selector)?;
    let linear_table = linear_list
        .into_iter()
        .map(|e| {
            vec![
                e.database.cell().justify(Justify::Center).bold(true),
                e.exp_type.cell().justify(Justify::Center).bold(true),
                e.exp_label.cell().justify(Justify::Center).bold(true),
                e.exp_code.cell().justify(Justify::Center).bold(true),
                e.horizontal_label.cell().justify(Justify::Center),
                e.v_label.cell().justify(Justify::Center),
                e.v_units.cell().justify(Justify::Center),
            ]
        })
        .collect::<Vec<_>>()
        .table()
        .title(vec![
            "Database".cell().justify(Justify::Center).bold(true),
            "Type".cell().justify(Justify::Center).bold(true),
            "Label".cell().justify(Justify::Center).bold(true),
            "Code".cell().justify(Justify::Center).bold(true),
            "Horiz Label ".cell().justify(Justify::Center).bold(true),
            "Label".cell().justify(Justify::Center).bold(true),
            "Units".cell().justify(Justify::Center).bold(true),
        ])
        .bold(true);

    cli_table::print_stdout(linear_table)?;

    let xy_list = config.list_xy_experiments(selector)?;
    let xy_table = xy_list
        .into_iter()
        .map(|e| {
            vec![
                e.database.cell().justify(Justify::Center).bold(true),
                e.exp_type.cell().justify(Justify::Center).bold(true),
                e.exp_label.cell().justify(Justify::Center).bold(true),
                e.exp_code.cell().justify(Justify::Center).bold(true),
                e.x_label.cell().justify(Justify::Center),
                e.x_units.cell().justify(Justify::Center),
                e.y_label.cell().justify(Justify::Center),
                e.y_units.cell().justify(Justify::Center),
            ]
        })
        .collect::<Vec<_>>()
        .table()
        .title(vec![
            "Database".cell().justify(Justify::Center).bold(true),
            "Type".cell().justify(Justify::Center).bold(true),
            "Label".cell().justify(Justify::Center).bold(true),
            "Code".cell().justify(Justify::Center).bold(true),
            "X label".cell().justify(Justify::Center).bold(true),
            "X units".cell().justify(Justify::Center).bold(true),
            "Y label".cell().justify(Justify::Center).bold(true),
            "Y units".cell().justify(Justify::Center).bold(true),
        ])
        .bold(true);

    cli_table::print_stdout(xy_table)?;

    Ok(())
}

fn status(config: &ReadConfig, selector: &Selector) -> Result<()> {
    let table = config
        .status(selector)?
        .into_iter()
        .map(|s| {
            vec![
                s.database.cell().justify(Justify::Center).bold(true),
                s.exp_type.cell().justify(Justify::Center).bold(true),
                s.exp_label.cell().justify(Justify::Center).bold(true),
                s.exp_code.cell().justify(Justify::Center).bold(true),
                s.n_active_datapoints.cell().justify(Justify::Right),
                s.n_datapoints.cell().justify(Justify::Right),
            ]
        })
        .collect::<Vec<_>>()
        .table()
        .title(vec![
            "DB".cell().justify(Justify::Center).bold(true),
            "Type".cell().justify(Justify::Center).bold(true),
            "Label".cell().justify(Justify::Center).bold(true),
            "Code".cell().justify(Justify::Center).bold(true),
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

fn table(config: &ReadConfig, exp_type: &str, selector: &Selector) -> Result<()> {
    let linear_view = config.linear_experiment_view(exp_type, selector);
    let xy_view = config.xy_experiment_view(exp_type, selector);

    match (linear_view, xy_view) {
        (Ok(_), Ok(_)) => {
            // impossible, exp_type is known to be unique
        }
        (Ok(linear_view), Err(_)) => {
            let mut stdout = std::io::stdout().lock();
            linear_view.table(&mut stdout)?;
        }
        (Err(_), Ok(xy_view)) => {
            let mut stdout = std::io::stdout().lock();
            xy_view.table(&mut stdout)?;
        }
        (Err(linear_err), Err(xy_err)) => match (linear_err, xy_err) {
            (
                BencherError::ExperimentNotFound(_, available_linear),
                BencherError::ExperimentNotFound(_, available_xy),
            ) => {
                return Err(BencherError::ExperimentNotFound(
                    exp_type.to_string(),
                    format!("{}, {}", available_linear, available_xy),
                )
                .into());
            }
            (e, BencherError::ExperimentNotFound(_, _)) => {
                return Err(e.into());
            }
            (BencherError::ExperimentNotFound(_, _), e) => {
                return Err(e.into());
            }
            (e, _) => {
                return Err(e.into());
            }
        },
    }

    Ok(())
}

fn latex(
    config: &ReadConfig,
    exp_type: &str,
    file: Option<&std::path::Path>,
    selector: &Selector,
) -> Result<()> {
    let linear_view = config.linear_experiment_view(exp_type, selector);
    let xy_view = config.xy_experiment_view(exp_type, selector);

    match (linear_view, xy_view) {
        (Ok(_), Ok(_)) => {
            // impossible, exp_type is known to be unique
        }
        (Ok(linear_view), Err(_)) => {
            if let Some(path) = file {
                let mut file = File::create(path)?;
                linear_view.latex_table(&mut file)?;
            } else {
                let mut stdout = std::io::stdout().lock();
                linear_view.latex_table(&mut stdout)?;
            }
        }
        (Err(_), Ok(xy_view)) => {
            if let Some(path) = file {
                let mut file = File::create(path)?;
                xy_view.latex_table(&mut file)?;
            } else {
                let mut stdout = std::io::stdout().lock();
                xy_view.latex_table(&mut stdout)?;
            }
        }
        (Err(linear_err), Err(xy_err)) => match (linear_err, xy_err) {
            (
                BencherError::ExperimentNotFound(_, available_linear),
                BencherError::ExperimentNotFound(_, available_xy),
            ) => {
                return Err(BencherError::ExperimentNotFound(
                    exp_type.to_string(),
                    format!("{}, {}", available_linear, available_xy),
                )
                .into());
            }
            (e, BencherError::ExperimentNotFound(_, _)) => {
                return Err(e.into());
            }
            (BencherError::ExperimentNotFound(_, _), e) => {
                return Err(e.into());
            }
            (e, _) => {
                return Err(e.into());
            }
        },
    }

    Ok(())
}

fn dat(
    config: &ReadConfig,
    exp_type: &str,
    prefix: &std::path::Path,
    bar: Option<usize>,
    xbar: Option<usize>,
    ybar: Option<usize>,
    selector: &Selector,
) -> Result<()> {
    let bars = Bars::from_optionals(bar, xbar, ybar)?;
    let linear_view = config.linear_experiment_view(exp_type, selector);
    let xy_view = config.xy_experiment_view(exp_type, selector);

    match (linear_view, xy_view) {
        (Ok(_), Ok(_)) => {
            // impossible, exp_type is known to be unique
        }
        (Ok(linear_view), Err(_)) => {
            linear_view.dat(prefix, bars)?;
        }
        (Err(_), Ok(xy_view)) => {
            xy_view.dat(prefix, bars)?;
        }
        (Err(linear_err), Err(xy_err)) => match (linear_err, xy_err) {
            (
                BencherError::ExperimentNotFound(_, available_linear),
                BencherError::ExperimentNotFound(_, available_xy),
            ) => {
                return Err(BencherError::ExperimentNotFound(
                    exp_type.to_string(),
                    format!("{}, {}", available_linear, available_xy),
                )
                .into());
            }
            (e, BencherError::ExperimentNotFound(_, _)) => {
                return Err(e.into());
            }
            (BencherError::ExperimentNotFound(_, _), e) => {
                return Err(e.into());
            }
            (e, _) => {
                return Err(e.into());
            }
        },
    }

    Ok(())
}

fn gnuplot(
    config: &ReadConfig,
    exp_type: &str,
    prefix: &std::path::Path,
    bar: bool,
    xbar: bool,
    ybar: bool,
    selector: &Selector,
) -> Result<()> {
    let bars = Bars::from_bools(bar, xbar, ybar)?;

    let linear_view = config.linear_experiment_view(exp_type, selector);
    let xy_view = config.xy_experiment_view(exp_type, selector);

    match (linear_view, xy_view) {
        (Ok(_), Ok(_)) => {
            // impossible, exp_type is known to be unique
        }
        (Ok(linear_view), Err(_)) => {
            linear_view.gnuplot(prefix, bars)?;
        }
        (Err(_), Ok(xy_view)) => {
            xy_view.gnuplot(prefix, bars)?;
        }
        (Err(linear_err), Err(xy_err)) => match (linear_err, xy_err) {
            (
                BencherError::ExperimentNotFound(_, available_linear),
                BencherError::ExperimentNotFound(_, available_xy),
            ) => {
                return Err(BencherError::ExperimentNotFound(
                    exp_type.to_string(),
                    format!("{}, {}", available_linear, available_xy),
                )
                .into());
            }
            (e, BencherError::ExperimentNotFound(_, _)) => {
                return Err(e.into());
            }
            (BencherError::ExperimentNotFound(_, _), e) => {
                return Err(e.into());
            }
            (e, _) => {
                return Err(e.into());
            }
        },
    }

    Ok(())
}

fn plot(
    config: &ReadConfig,
    exp_type: &str,
    prefix: &std::path::Path,
    bar: Option<usize>,
    xbar: Option<usize>,
    ybar: Option<usize>,
    selector: &Selector,
) -> Result<()> {
    let bars = Bars::from_optionals(bar, xbar, ybar)?;
    let linear_view = config.linear_experiment_view(exp_type, selector);
    let xy_view = config.xy_experiment_view(exp_type, selector);

    match (linear_view, xy_view) {
        (Ok(_), Ok(_)) => {
            // impossible, exp_type is known to be unique
        }
        (Ok(linear_view), Err(_)) => {
            linear_view.plot(prefix, bars)?;
        }
        (Err(_), Ok(xy_view)) => {
            xy_view.plot(prefix, bars)?;
        }
        (Err(linear_err), Err(xy_err)) => match (linear_err, xy_err) {
            (
                BencherError::ExperimentNotFound(_, available_linear),
                BencherError::ExperimentNotFound(_, available_xy),
            ) => {
                return Err(BencherError::ExperimentNotFound(
                    exp_type.to_string(),
                    format!("{}, {}", available_linear, available_xy),
                )
                .into());
            }
            (e, BencherError::ExperimentNotFound(_, _)) => {
                return Err(e.into());
            }
            (BencherError::ExperimentNotFound(_, _), e) => {
                return Err(e.into());
            }
            (e, _) => {
                return Err(e.into());
            }
        },
    }

    Ok(())
}

fn revert(
    config: &WriteConfig,
    exp_code: &str,
    tag: Option<isize>,
    group: Option<&String>,
    version: Option<usize>,
) -> Result<()> {
    match (group, tag) {
        (None, None) => {
            return Err(eyre::eyre!("to revert a tag or group is required"));
        }
        (Some(_), Some(_)) => {
            return Err(eyre::eyre!(
                "cannot revert something with a tag and a group at the same time"
            ));
        }
        (Some(group), None) => {
            let linear_set = config
                .get_linear_set(exp_code)?
                .ok_or_else(|| eyre::eyre!("Could not find linear set with code {}", exp_code))?;
            linear_set.revert(&group, version)?;
        }
        (None, Some(tag)) => {
            let xy_line = config.get_xy_line(exp_code)?.ok_or_else(|| {
                eyre::eyre!("Could not find bidimensional line with code {}", exp_code)
            })?;
            xy_line.revert(tag, version)?;
        }
    }

    Ok(())
}
