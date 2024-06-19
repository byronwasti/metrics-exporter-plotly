# metrics-exporter-plotly 

An embedded metrics exporter which plots metrics in the browser.

## Usage

Normal usage is setting up the `PlotlyRecorder` using the `PlotlyRecorderBuilder`. This will return a `PlotlyRecorderHandle` which can be used for a one-shot plotting of the metrics.

`PlotlyRecorderHandle` takes a slice of `PatternGroup`s, which are just regex patterns you wish to group together into rows of plots. For instance, in the following:

```rust
use metrics_exporter_plotly::{PatternGroup, PlotKind, PlotlyRecorderBuilder};

#[tokio::main]
async fn main() {
    let handle = PlotlyRecorderBuilder::new().install().unwrap();

    /* Your code */

    // Have to call `.plot()` on handle when you want to plot metrics
    handle
        .plot(&[PatternGroup::new()
            .pattern(r"(?<transaction>.*)_success", PlotKind::Rate)
            .pattern(r"(?<transaction>.*)_error", PlotKind::Rate)])
        .await;
}
```

`foo_success` and `foo_error` would be plotted together on a row, and `bar_success` and `bar_error` would be on another row.
