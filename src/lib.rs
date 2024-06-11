use metrics::{
    Counter, Gauge, Histogram, Key, KeyName, Metadata, Recorder, SetRecorderError, SharedString,
    Unit,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot::{self, Receiver, Sender};
use tracing::error;

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

struct State {
    counters: HashMap<Key, Arc<AtomicU64>>,
}

impl State {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
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

    fn register_gauge(&self, _key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        unimplemented!()
    }

    fn register_histogram(&self, _key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        unimplemented!()
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
            println!("{data:?}");
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
    let state = state.lock().unwrap();
    for (key, counter) in state.counters.iter() {
        let val = counter.load(Ordering::Relaxed);
        data.push_counter(key, val);
    }
}

#[derive(Debug)]
struct DataCollector {
    counters: HashMap<Key, Vec<u64>>,
}

impl DataCollector {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    fn push_counter(&mut self, key: &Key, value: u64) {
        if let Some(vec) = self.counters.get_mut(key) {
            vec.push(value);
        } else {
            self.counters.insert(key.to_owned(), vec![value]);
        }
    }
}
