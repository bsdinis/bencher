use std::fs::File;
use std::io::Write;

use crate::*;

use cli_table::{format::Justify, Cell, Style, Table};

pub(crate) struct XYExperimentLine {
    /// XY values that form the line
    pub(crate) values: Vec<XYDatapoint>,

    pub(crate) line_label: String,
}

pub struct XYExperimentView {
    lines: Vec<XYExperimentLine>,
    x_label: String,
    x_units: String,
    y_label: String,
    y_units: String,
}

/// Choose a magnitude based on a given iterator of LinearExperimentSets
fn choose_magnitude<'a>(
    lines: impl Iterator<Item = &'a XYExperimentLine>,
) -> (Magnitude, Magnitude) {
    let mut x_magnitude_counts = [0; 7];
    let mut y_magnitude_counts = [0; 7];

    lines.for_each(|lines| {
        lines.values.iter().for_each(|d| {
            let (x_mag, y_mag) = d.magnitudes();
            match x_mag {
                Magnitude::Nano => x_magnitude_counts[0] += 1,
                Magnitude::Micro => x_magnitude_counts[1] += 1,
                Magnitude::Mili => x_magnitude_counts[2] += 1,
                Magnitude::Normal => x_magnitude_counts[3] += 1,
                Magnitude::Kilo => x_magnitude_counts[4] += 1,
                Magnitude::Mega => x_magnitude_counts[5] += 1,
                Magnitude::Giga => x_magnitude_counts[6] += 1,
            };
            match y_mag {
                Magnitude::Nano => y_magnitude_counts[0] += 1,
                Magnitude::Micro => y_magnitude_counts[1] += 1,
                Magnitude::Mili => y_magnitude_counts[2] += 1,
                Magnitude::Normal => y_magnitude_counts[3] += 1,
                Magnitude::Kilo => y_magnitude_counts[4] += 1,
                Magnitude::Mega => y_magnitude_counts[5] += 1,
                Magnitude::Giga => y_magnitude_counts[6] += 1,
            };
        })
    });

    let x_idx = x_magnitude_counts
        .iter()
        .enumerate()
        .max_by_key(|v| v.1)
        .map(|(idx, c)| if *c > 0 { idx } else { 3 })
        .unwrap();

    let y_idx = y_magnitude_counts
        .iter()
        .enumerate()
        .max_by_key(|v| v.1)
        .map(|(idx, c)| if *c > 0 { idx } else { 3 })
        .unwrap();

    let x_mag = match x_idx {
        0 => Magnitude::Nano,
        1 => Magnitude::Micro,
        2 => Magnitude::Mili,
        3 => Magnitude::Normal,
        4 => Magnitude::Kilo,
        5 => Magnitude::Mega,
        _ => Magnitude::Giga,
    };
    let y_mag = match y_idx {
        0 => Magnitude::Nano,
        1 => Magnitude::Micro,
        2 => Magnitude::Mili,
        3 => Magnitude::Normal,
        4 => Magnitude::Kilo,
        5 => Magnitude::Mega,
        _ => Magnitude::Giga,
    };

    (x_mag, y_mag)
}

impl XYExperimentView {
    pub(crate) fn new(
        experiment: &XYExperiment,
        lines: Vec<XYExperimentLine>,
    ) -> BencherResult<Self> {
        if lines.len() == 0 {
            Err(BencherError::NoLines(experiment.exp_type.clone()))
        } else {
            Ok(Self {
                lines,
                x_label: experiment.x_label.clone(),
                x_units: experiment.x_units.clone(),
                y_label: experiment.y_label.clone(),
                y_units: experiment.y_units.clone(),
            })
        }
    }
}

impl ExperimentView for XYExperimentView {
    fn gnuplot(&self, prefix: &std::path::Path, bar: Bars) -> BencherResult<()> {
        let mut gnu_path: std::path::PathBuf = prefix.into();
        if !gnu_path.set_extension("gnu") {
            return Err(BencherError::PathCreateError(gnu_path, "gnu".to_string()));
        }
        let mut eps_path: std::path::PathBuf = prefix.into();
        if !eps_path.set_extension("eps") {
            return Err(BencherError::PathCreateError(eps_path, "eps".to_string()));
        }

        let mut file = File::create(&gnu_path).map_err(|e| {
            BencherError::io_err(e, format!("creating {}", gnu_path.to_string_lossy()))
        })?;
        write!(
            &mut file,
            "reset

set terminal postscript eps colour size 12cm,8cm enhanced font 'Helvetica,20'
set output '{}'

set border linewidth 0.75
set key outside above
",
            eps_path.to_string_lossy()
        )
        .map_err(|e| BencherError::io_err(e, "writing gnu to file"))?;

        let dat_paths = self
            .lines
            .iter()
            .map(|line| {
                let mut dat_path: std::path::PathBuf = prefix.into();
                dat_path.set_file_name(format!(
                    "{}_{}",
                    prefix
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or("".to_string()),
                    line.line_label.to_lowercase().replace("/", "_"),
                ));
                if !dat_path.set_extension("dat") {
                    return Err(BencherError::PathCreateError(dat_path, "dat".to_string()));
                }
                Ok(dat_path)
            })
            .collect::<BencherResult<Vec<_>>>()?;

        for (idx, _) in self.lines.iter().enumerate() {
            writeln!(
                &mut file,
"# Set color of linestyle {0} to #{3}
set style line {0} linecolor rgb '#{3}' linetype 2 linewidth 2.5 pointtype {2} pointsize 2 dashtype 2
# Set yerror color of linestyle {1} to #{2}
set style line {1} linecolor rgb '#{3}' linetype 2 linewidth 2.5 pointtype {2} pointsize 2",
                2 * idx + 1,
                2 * idx + 2,
                2 * idx + 4,
                COLORS[idx % COLORS.len()]
                ).map_err(|e| BencherError::io_err(e, "writing gnu to file"))?;
        }

        let (x_mag, y_mag) = choose_magnitude(self.lines.iter());
        write!(
            file,
            "

# set axis
set tics scale 0.75
set xlabel '{} ({}{})'
set ylabel '{} ({}{})'
set xrange [*:*]
set yrange [*:*]
",
            self.x_label,
            x_mag.prefix(),
            self.x_units,
            self.y_label,
            y_mag.prefix(),
            self.y_units,
        )
        .map_err(|e| BencherError::io_err(e, "writing gnu to file"))?;

        println!(
            "plot {}",
            self.lines
                .iter()
                .enumerate()
                .zip(dat_paths.iter())
                .map(|((idx, line), dat_path)| match bar {
                    Bars::XY(_, _) => format!(
                        "'{}' title '{}' with xyerrorbars linestyle {}, '' title '' with lines linestyle {}",
                        dat_path.to_string_lossy(),
                        line.line_label,
                        2 * idx + 2,
                        2 * idx + 1,
                    ),
                    Bars::X(_) => format!(
                        "'{}' title '{}' with xerrorbars linestyle {}, '' title '' with lines linestyle {}",
                        dat_path.to_string_lossy(),
                        line.line_label,
                        2 * idx + 2,
                        2 * idx + 1,
                    ),
                    Bars::Y(_) => format!(
                        "'{}' title '{}' with yerrorbars linestyle {}, '' title '' with lines linestyle {}",
                        dat_path.to_string_lossy(),
                        line.line_label,
                        2 * idx + 2,
                        2 * idx + 1,
                    ),
                    _ => format!(
                        "'{}' title '{}' with linespoint linestyle {}",
                        dat_path.to_string_lossy(),
                        line.line_label,
                        2 * idx + 1
                    ),
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(())
    }

    fn dat(&self, prefix: &std::path::Path, bar: Bars) -> BencherResult<()> {
        let (x_mag, y_mag) = choose_magnitude(self.lines.iter());
        for line in &self.lines {
            let mut dat_path: std::path::PathBuf = prefix.into();
            dat_path.set_file_name(format!(
                "{}_{}",
                prefix
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or("".to_string()),
                line.line_label.to_lowercase().replace("/", "_"),
            ));
            if !dat_path.set_extension("dat") {
                return Err(BencherError::PathCreateError(dat_path, "dat".to_string()));
            }

            let mut file = File::create(&dat_path).map_err(|e| {
                BencherError::io_err(e, format!("creating file {}", dat_path.to_string_lossy()))
            })?;

            writeln!(
                &mut file,
                "# {}\n# x axis: {} ({}{})\n# y axis: {} ({}{})\n",
                line.line_label,
                self.x_label,
                x_mag.prefix(),
                self.x_units,
                self.y_label,
                y_mag.prefix(),
                self.y_units
            )
            .map_err(|e| BencherError::io_err(e, "writing dat file"))?;

            for d in &line.values {
                write!(
                    &mut file,
                    "{:>8} {:>8}",
                    d.x.display_with_magnitude(x_mag),
                    d.y.display_with_magnitude(y_mag)
                )
                .map_err(|e| BencherError::io_err(e, "writing dat file"))?;

                match bar {
                    Bars::X(c) | Bars::XY(c, _) => {
                        let (xmin, xmax) =
                            d.get_x_confidence(c).unwrap_or((d.x.clone(), d.x.clone()));
                        write!(
                            &mut file,
                            " {:>8} {:>8}",
                            xmin.display_with_magnitude(x_mag),
                            xmax.display_with_magnitude(x_mag)
                        )
                        .map_err(|e| BencherError::io_err(e, "writing dat file"))?;
                    }
                    _ => {}
                }

                match bar {
                    Bars::Y(c) | Bars::XY(_, c) => {
                        let (ymin, ymax) =
                            d.get_y_confidence(c).unwrap_or((d.y.clone(), d.y.clone()));
                        write!(
                            &mut file,
                            " {:>8} {:>8}",
                            ymin.display_with_magnitude(y_mag),
                            ymax.display_with_magnitude(y_mag)
                        )
                        .map_err(|e| BencherError::io_err(e, "writing dat file"))?;
                    }
                    _ => {}
                }

                writeln!(&mut file, "").map_err(|e| BencherError::io_err(e, "writing dat file"))?;
            }

            writeln!(&mut file, "\n# end")
                .map_err(|e| BencherError::io_err(e, "writing dat file"))?;
        }
        Ok(())
    }

    fn table<W: Write>(&self, writer: &mut W) -> BencherResult<()> {
        let (x_mag, y_mag) = choose_magnitude(self.lines.iter());

        for line in &self.lines {
            let table = line
                .values
                .iter()
                .map(|d| {
                    vec![
                        d.tag.unwrap().cell().justify(Justify::Right),
                        d.x.display_with_magnitude(x_mag)
                            .cell()
                            .justify(Justify::Right),
                        d.y.display_with_magnitude(y_mag)
                            .cell()
                            .justify(Justify::Right),
                    ]
                })
                .collect::<Vec<_>>()
                .table()
                .title(vec![
                    "Tag".cell().justify(Justify::Center).bold(true),
                    format!("{} ({}{})", self.x_label, x_mag.prefix(), self.x_units)
                        .cell()
                        .justify(Justify::Center)
                        .bold(true),
                    format!("{} ({}{})", self.y_label, y_mag.prefix(), self.y_units)
                        .cell()
                        .justify(Justify::Center)
                        .bold(true),
                ])
                .bold(true);

            let table_display = table
                .display()
                .map_err(|e| BencherError::io_err(e, "creating table display"))?;
            writeln!(writer, "{}:", line.line_label)
                .map_err(|e| BencherError::io_err(e, "writing table display"))?;
            writeln!(writer, "{}", table_display)
                .map_err(|e| BencherError::io_err(e, "writing table display"))?;
        }
        Ok(())
    }

    fn latex_table<W: Write>(&self, writer: &mut W) -> BencherResult<()> {
        let (x_mag, y_mag) = choose_magnitude(self.lines.iter());
        for line in &self.lines {
            writeln!(writer, "\\begin{{table}}[t]\n    \\centering\n    \\begin{{tabular}}{{|r|r|}}\n        \\hline").map_err(|e| BencherError::io_err(e, "writing latex table"))?;
            writeln!(
                writer,
                "        \\textbf{{ {} ({}{}) }} & \\textbf{{ {} ({}{}) }} \\\\ \\hline",
                self.x_label,
                x_mag.prefix(),
                self.x_units,
                self.y_label,
                y_mag.prefix(),
                self.y_units
            )
            .map_err(|e| BencherError::io_err(e, "writing latex table"))?;
            for d in &line.values {
                writeln!(
                    writer,
                    "        ${:>8}$ & ${:>8}$ \\\\ \\hline",
                    d.x.display_with_magnitude(x_mag),
                    d.y.display_with_magnitude(y_mag)
                )
                .map_err(|e| BencherError::io_err(e, "writing latex table"))?
            }
            writeln!(writer,
                "    \\end{{tabular}}\n    \\caption{{Caption: {0}}}\\label{{table:{0}}}\n\\end{{table}}", line.line_label
            ).map_err(|e| BencherError::io_err(e, "writing latex table"))?;
        }
        Ok(())
    }
}
