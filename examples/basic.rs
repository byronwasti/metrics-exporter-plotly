use metrics::counter;
use metrics_exporter_plotly::PlotlyRecorderBuilder;

#[tokio::main]
async fn main() {
    let handle = PlotlyRecorderBuilder::new().install().unwrap();

    for _ in 0..1000 {
        counter!("test").increment(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    handle.plot().await;
}
