use super::DataCollector;
use crate::{MetricKind, PatternGroup, PlotKind};
use plotly::common::Mode;
use plotly::layout::{GridPattern, Layout, LayoutGrid};
use plotly::{Plot, Scatter};

pub(crate) fn plot_data(data: DataCollector, groups: &[PatternGroup]) {
    let metrics = data.metrics();

    for group in groups {
        let mut plot = Plot::new();
        let rows = group.apply(&metrics);

        let columns = if !rows.is_empty() {
            rows[0].len()
        } else {
            continue;
        };

        let mut idx = 0;
        for row in &rows {
            for (name, kind) in row.iter() {
                let vals = data.get_metric(name);
                idx += 1;

                match vals {
                    Some(MetricKind::Single(vals)) => {
                        let vals: Vec<u64> = match kind {
                            PlotKind::Line => vals,
                            PlotKind::Rate => std::iter::once(vals[0])
                                .chain(vals.windows(2).map(|window| window[1] - window[0]))
                                .collect(),
                        };

                        let mut x = data.timestamps.clone();
                        let x = x.split_off(x.len() - vals.len());
                        let trace = Scatter::new(x, vals)
                            .name(name.clone())
                            .x_axis(format!("x{idx}"))
                            .y_axis(format!("y{idx}"))
                            .mode(Mode::Lines);
                        plot.add_trace(trace);
                    }
                    Some(MetricKind::Quantile(vals)) => {
                        let mut x = data.timestamps.clone();
                        let x = x.split_off(x.len() - vals.len());
                        let mut unzipped = vec![vec![]; 4];
                        for (a, b, c, d) in vals.iter() {
                            unzipped[0].push(*a);
                            unzipped[1].push(*b);
                            unzipped[2].push(*c);
                            unzipped[3].push(*d);
                        }

                        let trace = Scatter::new(x.clone(), unzipped.pop().unwrap())
                            .name(name.clone())
                            .x_axis(format!("x{idx}"))
                            .y_axis(format!("y{idx}"))
                            .mode(Mode::Lines);
                        plot.add_trace(trace);

                        let trace = Scatter::new(x.clone(), unzipped.pop().unwrap())
                            .name(name.clone())
                            .x_axis(format!("x{idx}"))
                            .y_axis(format!("y{idx}"))
                            .mode(Mode::Lines);
                        plot.add_trace(trace);

                        let trace = Scatter::new(x.clone(), unzipped.pop().unwrap())
                            .name(name.clone())
                            .x_axis(format!("x{idx}"))
                            .y_axis(format!("y{idx}"))
                            .mode(Mode::Lines);
                        plot.add_trace(trace);

                        let trace = Scatter::new(x, unzipped.pop().unwrap())
                            .name(name.clone())
                            .x_axis(format!("x{idx}"))
                            .y_axis(format!("y{idx}"))
                            .mode(Mode::Lines);
                        plot.add_trace(trace);
                    }
                    None => {}
                }
            }
        }

        let layout = Layout::new().grid(
            LayoutGrid::new()
                .rows(rows.len())
                .columns(columns)
                .pattern(GridPattern::Independent),
        );
        plot.set_layout(layout);

        plot.show();
    }
}
