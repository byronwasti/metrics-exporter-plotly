use metrics::counter;
use metrics_exporter_plotly::PlotlyRecorderBuilder;

fn main() {
    let _handle = PlotlyRecorderBuilder::new().install();

    for _ in 0..100 {
        counter!("test").increment(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
