use metrics::counter;
use metrics_exporter_plotly::{PatternGroup, PlotKind, PlotlyRecorderBuilder};

#[tokio::main]
async fn main() {
    let handle = PlotlyRecorderBuilder::new().install().unwrap();

    let h = tokio::spawn(async {
        for _ in 0..200 {
            counter!("something_success").increment(1);
            counter!("something_error").increment(1);

            counter!("foo_asdf_success").increment(2);
            counter!("foo_asdf_error").increment(2);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });

    h.await;

    handle
        .plot(&[PatternGroup::new()
            .pattern(r"(?<transaction>.*)_success", PlotKind::Rate)
            .pattern(r"(?<transaction>.*)_error", PlotKind::Rate)])
        .await;
}
