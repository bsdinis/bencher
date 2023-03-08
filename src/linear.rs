use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;

use crate::*;

use cli_table::{format::Justify, Cell, Style, Table};

/// A linear experiment set represents all the groups under a label
///
/// Example: if the histogram is latency per operation,
/// and there are two labels (A and B) and two operations (get and put),
/// the sets are A (with A/get and A/put) and B (with B/get and B/put)
pub(crate) struct LinearExperimentSet {
    /// Following the example, this could have two datapoints, one get and one put
    pub(crate) values: Vec<LinearDatapoint>,

    /// Following the example, this could be "A"
    pub(crate) set_label: String,
}

pub struct LinearExperimentView {
    sets: Vec<LinearExperimentSet>,
    magnitude: Magnitude,
    horizontal_label: String,
    v_label: String,
    v_units: String,
}

impl LinearExperimentView {
    pub(crate) fn from_linear(
        experiment: &LinearExperiment,
        sets: Vec<LinearExperimentSet>,
    ) -> BencherResult<Self> {
        if sets.len() == 0 {
            Err(BencherError::NoSets(experiment.exp_type.clone()))
        } else {
            let magnitude = choose_magnitude(sets.iter());
            Ok(Self {
                sets,
                magnitude,
                horizontal_label: experiment.horizontal_label.clone(),
                v_label: experiment.v_label.clone(),
                v_units: experiment.v_units.clone(),
            })
        }
    }

    pub(crate) fn from_virtual(
        experiment: &VirtualLinearExperiment,
        sets: Vec<LinearExperimentSet>,
    ) -> BencherResult<Self> {
        if sets.len() == 0 {
            Err(BencherError::NoSets(experiment.exp_type.clone()))
        } else {
            let magnitude = choose_magnitude(sets.iter());
            Ok(Self {
                sets,
                magnitude,
                horizontal_label: experiment.horizontal_label.clone(),
                v_label: experiment.v_label.clone(),
                v_units: experiment.v_units.clone(),
            })
        }
    }
}

impl ExperimentView for LinearExperimentView {
    fn gnuplot(&self, prefix: &std::path::Path, bar: Bars) -> BencherResult<()> {
        let mut gnu_path: std::path::PathBuf = prefix.into();
        if !gnu_path.set_extension("gnu") {
            return Err(BencherError::PathCreateError(gnu_path, "gnu".to_string()));
        }
        let mut dat_path: std::path::PathBuf = prefix.into();
        if !dat_path.set_extension("dat") {
            return Err(BencherError::PathCreateError(dat_path, "dat".to_string()));
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
set style data histogram
",
            eps_path.to_string_lossy()
        )
        .map_err(|e| BencherError::io_err(e, "writing gnu to file"))?;

        match bar {
            Bars::Linear(_) => writeln!(file, "set style histogram cluster gap 1 errorbars lw 2")
                .map_err(|e| BencherError::io_err(e, "writing gnu to file"))?,
            _ => writeln!(file, "set style histogram cluster gap 1")
                .map_err(|e| BencherError::io_err(e, "writing gnu to file"))?,
        }

        write!(
            &mut file,
            "
# set axis
set style fill pattern 4 border rgb \"black\"
set auto x
set yrange [0:*]
set ylabel '{} ({}{})'
",
            self.v_label,
            self.magnitude.prefix(),
            self.v_units,
        )
        .map_err(|e| BencherError::io_err(e, "writing gnu to file"))?;

        match bar {
            Bars::Linear(_) => writeln!(
                &mut file,
                "plot for [i=2:{}:3] '{}' using i:i+1:i+2:xtic(1) title col(i)",
                2 + 3 * (self.sets.len() - 1),
                dat_path.to_string_lossy()
            )
            .map_err(|e| BencherError::io_err(e, "writing gnu to file"))?,
            _ => writeln!(
                &mut file,
                "plot for [i=2:{}:1] '{}' using i:xtic(1) title col(i)",
                2 + self.sets.len() - 1,
                dat_path.to_string_lossy()
            )
            .map_err(|e| BencherError::io_err(e, "writing gnu to file"))?,
        }

        Ok(())
    }

    fn dat(&self, prefix: &std::path::Path, bar: Bars) -> BencherResult<()> {
        let mut dat_path: std::path::PathBuf = prefix.into();
        if !dat_path.set_extension("dat") {
            return Err(BencherError::PathCreateError(dat_path, "dat".to_string()));
        }
        let mut file = File::create(&dat_path).map_err(|e| {
            BencherError::io_err(e, format!("creating {}", dat_path.to_string_lossy()))
        })?;

        let confidence_str = match bar {
            Bars::Linear(c) => format!(
                "confidence interval: {}% - {}%",
                std::cmp::min(c, 100 - c),
                std::cmp::max(c, 100 - c)
            ),
            _ => "".to_string(),
        };

        writeln!(
            &mut file,
            "#begin {} {}",
            dat_path.to_string_lossy(),
            confidence_str
        )
        .map_err(|e| BencherError::io_err(e, "writing dat to file"))?;

        // table: mapping from group to values, ordered by what it matters
        let mut group_values = BTreeMap::new();

        // header
        write!(
            &mut file,
            "{:>34} ",
            format!("\"{}\"", self.horizontal_label)
        )
        .map_err(|e| BencherError::io_err(e, "writing dat to file"))?;

        for set in &self.sets {
            write!(&mut file, "{:>34} ", format!("\"{}\"", set.set_label))
                .map_err(|e| BencherError::io_err(e, "writing dat to file"))?;

            match bar {
                Bars::Linear(_) => {
                    write!(&mut file, "{:>34} ", "\"min\"")
                        .map_err(|e| BencherError::io_err(e, "writing dat to file"))?;
                    write!(&mut file, "{:>34} ", "\"max\"")
                        .map_err(|e| BencherError::io_err(e, "writing dat to file"))?;
                }
                _ => {}
            }

            for datapoint in &set.values {
                let guard = group_values
                    .entry(datapoint.group.clone())
                    .or_insert(vec![]);
                guard.push(datapoint.v.display_with_magnitude(self.magnitude));
                match bar {
                    Bars::Linear(confidence) => {
                        let (min, max) = datapoint
                            .get_confidence(confidence.try_into()?)
                            .unwrap_or((datapoint.v.clone(), datapoint.v.clone()));
                        guard.push(min.display_with_magnitude(self.magnitude));
                        guard.push(max.display_with_magnitude(self.magnitude));
                    }
                    _ => {}
                }
            }
        }

        for (group, values) in group_values {
            write!(&mut file, "\n{:>34} ", format!("\"{}\"", group))
                .map_err(|e| BencherError::io_err(e, "writing dat to file"))?;
            for v in values {
                write!(&mut file, "{:>34} ", v)
                    .map_err(|e| BencherError::io_err(e, "writing dat to file"))?;
            }
        }

        writeln!(&mut file, "\n#end")
            .map_err(|e| BencherError::io_err(e, "writing dat to file"))?;
        Ok(())
    }

    fn table<W: Write>(&self, writer: &mut W) -> BencherResult<()> {
        let mut rows = Vec::new();
        for set in &self.sets {
            rows.extend(set.values.iter().map(|datapoint| {
                vec![
                    set.set_label.clone().cell().justify(Justify::Right),
                    datapoint.group.clone().cell().justify(Justify::Right),
                    datapoint
                        .v
                        .display_with_magnitude(self.magnitude)
                        .cell()
                        .justify(Justify::Right),
                ]
            }));
        }
        let table = rows
            .into_iter()
            .table()
            .title(vec![
                "Set".cell().justify(Justify::Center).bold(true),
                "Group".cell().justify(Justify::Center).bold(true),
                format!(
                    "{} ({}{})",
                    self.v_label,
                    self.magnitude.prefix(),
                    self.v_units
                )
                .cell()
                .justify(Justify::Center)
                .bold(true),
            ])
            .bold(true);

        let table_display = table
            .display()
            .map_err(|e| BencherError::io_err(e, "creating table display"))?;
        writeln!(writer, "{}", table_display)
            .map_err(|e| BencherError::io_err(e, "writing table display"))?;
        Ok(())
    }

    fn latex_table<W: Write>(&self, writer: &mut W) -> BencherResult<()> {
        for set in &self.sets {
            writeln!(writer, "\\begin{{table}}[t]\n    \\centering\n    \\begin{{tabular}}{{|r|r|}}\n        \\hline").map_err(|e| BencherError::io_err(e, "writing latex table"))?;
            writeln!(
                writer,
                "        \\textbf{{ {} }} & \\textbf{{ {} ({}{}) }} \\\\ \\hline",
                set.set_label,
                self.v_label,
                self.magnitude.prefix(),
                self.v_units,
            )
            .map_err(|e| BencherError::io_err(e, "writing latex table"))?;
            for datapoint in &set.values {
                writeln!(
                    writer,
                    "        ${:>8}$ & ${:>8}$ \\\\ \\hline",
                    datapoint.group,
                    datapoint.v.display_with_magnitude(self.magnitude)
                )
                .map_err(|e| BencherError::io_err(e, "writing latex table"))?
            }
            writeln!(writer,
                "    \\end{{tabular}}\n    \\caption{{Caption: {0}}}\\label{{table:{0}}}\n\\end{{table}}", set.set_label
            ).map_err(|e| BencherError::io_err(e, "writing latex table"))?;
        }
        Ok(())
    }
}

/// Choose a magnitude based on a given iterator of LinearExperimentSets
fn choose_magnitude<'a>(sets: impl Iterator<Item = &'a LinearExperimentSet>) -> Magnitude {
    let mut magnitude_counts = [0; 7];

    sets.for_each(|set| {
        set.values.iter().for_each(|d| match d.magnitude() {
            Magnitude::Nano => magnitude_counts[0] += 1,
            Magnitude::Micro => magnitude_counts[1] += 1,
            Magnitude::Mili => magnitude_counts[2] += 1,
            Magnitude::Normal => magnitude_counts[3] += 1,
            Magnitude::Kilo => magnitude_counts[4] += 1,
            Magnitude::Mega => magnitude_counts[5] += 1,
            Magnitude::Giga => magnitude_counts[6] += 1,
        })
    });

    let idx = magnitude_counts
        .iter()
        .enumerate()
        .max_by_key(|v| v.1)
        .map(|(idx, c)| if *c > 0 { idx } else { 3 })
        .unwrap();

    match idx {
        0 => Magnitude::Nano,
        1 => Magnitude::Micro,
        2 => Magnitude::Mili,
        3 => Magnitude::Normal,
        4 => Magnitude::Kilo,
        5 => Magnitude::Mega,
        _ => Magnitude::Giga,
    }
}
