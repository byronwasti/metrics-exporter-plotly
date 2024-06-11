use metrics::{
    Counter, Gauge, Histogram, Key, KeyName, Metadata, Recorder, SetRecorderError, SharedString,
    Unit,
};
use metrics_util::AtomicBucket;
use pdatastructs::tdigest::{TDigest, K1};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot::{self, Receiver, Sender};
use tracing::error;

const TDIGEST_COMPRESSION_FACTOR: f64 = 100.0;
const TDIGEST_MAX_BACKLOG_SIZE: usize = 10;

pub struct PlotlyRecorderBuilder {}

impl PlotlyRecorderBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn install(self) -> Result<PlotlyRecorderHandle, SetRecorderError<PlotlyRecorder>> {
        std::thread::spawn(|| {});

        let mut recorder = self.build();
        let handle = recorder.get_handle();
        metrics::set_global_recorder(recorder)?;

        Ok(handle)
    }

    fn build(&self) -> PlotlyRecorder {
        PlotlyRecorder::new()
    }
}

pub struct PlotlyRecorder {
    state: Arc<Mutex<State>>,
    handle: Option<PlotlyRecorderHandle>,
}

impl PlotlyRecorder {
    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::new()));
        let (tx0, rx0) = oneshot::channel();
        let (tx1, rx1) = oneshot::channel();

        let state2 = state.clone();
        tokio::spawn(scraper(state2, (rx0, tx1)));

        let handle = PlotlyRecorderHandle {
            channel: (tx0, rx1),
        };
        Self {
            state,
            handle: Some(handle),
        }
    }

    fn get_handle(&mut self) -> PlotlyRecorderHandle {
        self.handle.take().unwrap()
    }
}

impl Recorder for PlotlyRecorder {
    fn describe_counter(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // TODO
    }

    fn describe_gauge(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // TODO
    }

    fn describe_histogram(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // TODO
    }

    fn register_counter(&self, key: &Key, _metadata: &Metadata<'_>) -> Counter {
        let atomic = self.state.lock().unwrap().get_counter(&key);
        Counter::from_arc(atomic)
    }

    fn register_gauge(&self, key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        let atomic = self.state.lock().unwrap().get_gauge(&key);
        Gauge::from_arc(atomic)
    }

    fn register_histogram(&self, key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        let atomic = self.state.lock().unwrap().get_histogram(&key);
        Histogram::from_arc(atomic)
    }
}

struct State {
    counters: HashMap<Key, Arc<AtomicU64>>,
    gauges: HashMap<Key, Arc<AtomicU64>>,
    histograms: HashMap<Key, Arc<AtomicBucket<f64>>>,
}

impl State {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    fn get_counter(&mut self, key: &Key) -> Arc<AtomicU64> {
        if let Some(val) = self.counters.get(&key) {
            val.clone()
        } else {
            let val = Arc::new(AtomicU64::new(0));
            self.counters.insert(key.clone(), val.clone());
            val
        }
    }

    fn get_gauge(&mut self, key: &Key) -> Arc<AtomicU64> {
        if let Some(val) = self.gauges.get(&key) {
            val.clone()
        } else {
            let val = Arc::new(AtomicU64::new(0));
            self.gauges.insert(key.clone(), val.clone());
            val
        }
    }

    fn get_histogram(&mut self, key: &Key) -> Arc<AtomicBucket<f64>> {
        if let Some(val) = self.histograms.get(&key) {
            val.clone()
        } else {
            let val = Arc::new(AtomicBucket::new());
            self.histograms.insert(key.clone(), val.clone());
            val
        }
    }
}

pub struct PlotlyRecorderHandle {
    channel: (Sender<()>, Receiver<DataCollector>),
}

impl PlotlyRecorderHandle {
    pub async fn plot(self) {
        let _ = self.channel.0.send(());
        let res = self.channel.1.await;

        if let Ok(data) = res {
            plot_data(data);
        } else {
            error!("Channel broke in Drop impl.");
        }
    }
}

async fn scraper(state: Arc<Mutex<State>>, (rx, tx): (Receiver<()>, Sender<DataCollector>)) {
    let mut data = DataCollector::new();
    // TODO: Configurable scrape time
    let scrape_interval = Duration::from_millis(1000);

    tokio::select! {
        _ = async {
            loop {
                tokio::time::sleep(scrape_interval).await;
                scrape_data(&mut data, &state);
            }
        } => {
        }
        _ = rx => {
            scrape_data(&mut data, &state);
        }
    }

    let _ = tx.send(data);
}

fn scrape_data(data: &mut DataCollector, state: &Mutex<State>) {
    data.log_time();
    let state = state.lock().unwrap();
    for (key, counter) in state.counters.iter() {
        let val = counter.load(Ordering::Relaxed);
        data.push_counter(key, val);
    }

    for (key, gauge) in state.gauges.iter() {
        let val = gauge.load(Ordering::Relaxed);
        data.push_gauge(key, val);
    }

    for (key, histogram) in state.histograms.iter() {
        let scale_function = K1::new(TDIGEST_COMPRESSION_FACTOR);
        let mut tdigest = TDigest::new(scale_function, TDIGEST_MAX_BACKLOG_SIZE);
        histogram.clear_with(|data| {
            for d in data {
                tdigest.insert(*d);
            }
        });

        let vals = (
            tdigest.quantile(0.5),
            tdigest.quantile(0.9),
            tdigest.quantile(0.95),
            tdigest.quantile(0.99),
        );
        data.push_histogram(key, vals);
    }
}

#[derive(Debug)]
struct DataCollector {
    counters: HashMap<Key, Vec<u64>>,
    gauges: HashMap<Key, Vec<u64>>,
    histograms: HashMap<Key, Vec<(f64, f64, f64, f64)>>,
    start: Instant,
    timestamps: Vec<f64>,
}

impl DataCollector {
    fn new() -> Self {
        let start = Instant::now();
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            start,
            timestamps: vec![],
        }
    }

    fn log_time(&mut self) {
        self.timestamps.push(self.start.elapsed().as_secs_f64());
    }

    fn push_counter(&mut self, key: &Key, value: u64) {
        if let Some(vec) = self.counters.get_mut(key) {
            vec.push(value);
        } else {
            self.counters.insert(key.to_owned(), vec![value]);
        }
    }

    fn push_gauge(&mut self, key: &Key, value: u64) {
        if let Some(vec) = self.gauges.get_mut(key) {
            vec.push(value);
        } else {
            self.gauges.insert(key.to_owned(), vec![value]);
        }
    }

    fn push_histogram(&mut self, key: &Key, value: (f64, f64, f64, f64)) {
        if let Some(vec) = self.histograms.get_mut(key) {
            vec.push(value);
        } else {
            self.histograms.insert(key.to_owned(), vec![value]);
        }
    }
}

fn plot_data(mut data: DataCollector) {
    use plotly::common::Mode;
    use plotly::{Plot, Scatter};
    println!("{data:?}");

    let mut plot = Plot::new();

    for (key, y) in data.counters.drain() {
        let mut x = data.timestamps.clone();
        let x = x.split_off(x.len() - y.len());

        println!("xs: {x:?}, ys: {y:?}");

        let trace = Scatter::new(x, y).name(key.name()).mode(Mode::Lines);
        plot.add_trace(trace);
    }

    plot.show();
}
