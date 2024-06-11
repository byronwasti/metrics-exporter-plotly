use metrics::{
    Counter, Gauge, Histogram, Key, KeyName, Metadata, Recorder, SetRecorderError, SharedString,
    Unit,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
}

impl PlotlyRecorder {
    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::new()));

        let state2 = state.clone();
        tokio::spawn(scraper(state2));

        Self { state }
    }

    fn get_handle(&mut self) -> PlotlyRecorderHandle {
        PlotlyRecorderHandle {}
    }
}

impl Recorder for PlotlyRecorder {
    fn describe_counter(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        unimplemented!()
    }

    fn describe_gauge(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        unimplemented!()
    }

    fn describe_histogram(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        unimplemented!()
    }

    fn register_counter(&self, key: &Key, metadata: &Metadata<'_>) -> Counter {
        let atomic = self.state.lock().unwrap().get_counter(&key);
        Counter::from_arc(atomic)
    }

    fn register_gauge(&self, key: &Key, metadata: &Metadata<'_>) -> Gauge {
        unimplemented!()
    }

    fn register_histogram(&self, key: &Key, metadata: &Metadata<'_>) -> Histogram {
        unimplemented!()
    }
}

pub struct PlotlyRecorderHandle {}

impl Drop for PlotlyRecorderHandle {
    fn drop(&mut self) {
        println!("Bye bye");
    }
}

async fn scraper(state: Arc<Mutex<State>>) {
    let mut data = DataCollector::new();

    loop {
        // TODO: Configurable scrape time
        tokio::time::sleep(Duration::from_millis(1000));

        let state = state.lock().unwrap();

        for (key, counter) in state.counters.iter() {
            let val = counter.load(Ordering::Relaxed);
            data.push_counter(key, val);
        }
    }
}

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
