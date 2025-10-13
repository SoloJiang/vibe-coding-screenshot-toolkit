use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

#[derive(Default)]
pub struct Counter(AtomicU64);
impl Counter {
    pub fn incr(&self, v: u64) {
        self.0.fetch_add(v, Ordering::Relaxed);
    }
    pub fn inc(&self) {
        self.incr(1);
    }
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

pub struct Histogram {
    buckets: Vec<u64>,
    counts: Vec<AtomicU64>,
    sum: AtomicU64,
    count: AtomicU64,
}
impl Histogram {
    pub fn new(mut buckets: Vec<u64>) -> Self {
        buckets.sort_unstable();
        buckets.dedup();
        let counts = buckets.iter().map(|_| AtomicU64::new(0)).collect();
        Self {
            buckets,
            counts,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }
    pub fn observe(&self, v: u64) {
        for (i, b) in self.buckets.iter().enumerate() {
            if v <= *b {
                self.counts[i].fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
        self.sum.fetch_add(v, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }
    pub fn snapshot(&self) -> (Vec<(u64, u64)>, u64, u64) {
        // (bucket,value), sum, count
        let pairs = self
            .buckets
            .iter()
            .enumerate()
            .map(|(i, b)| (*b, self.counts[i].load(Ordering::Relaxed)))
            .collect();
        (
            pairs,
            self.sum.load(Ordering::Relaxed),
            self.count.load(Ordering::Relaxed),
        )
    }
}

pub struct MetricsRegistry {
    counters: Mutex<HashMap<&'static str, &'static Counter>>, // &'static 保证生命周期简单
    hists: Mutex<HashMap<&'static str, &'static Histogram>>,
}

impl MetricsRegistry {
    fn global() -> &'static MetricsRegistry {
        static REG: OnceLock<MetricsRegistry> = OnceLock::new();
        REG.get_or_init(|| MetricsRegistry {
            counters: Mutex::new(HashMap::new()),
            hists: Mutex::new(HashMap::new()),
        })
    }

    pub fn counter(name: &'static str) -> &'static Counter {
        static COUNTERS: OnceLock<Mutex<HashMap<&'static str, &'static Counter>>> = OnceLock::new();
        let map_lock = COUNTERS.get_or_init(|| Mutex::new(HashMap::new()));
        {
            let map = map_lock.lock();
            if let Some(c) = map.get(name) {
                return c;
            }
        }
        // Intentionally leak to obtain a 'static reference for global metrics in this process.
        let boxed: &'static Counter = Box::leak(Box::new(Counter::default()));
        let mut map = map_lock.lock();
        map.insert(name, boxed);
        MetricsRegistry::global()
            .counters
            .lock()
            .insert(name, boxed);
        boxed
    }

    pub fn histogram(name: &'static str, buckets: &[u64]) -> &'static Histogram {
        static HISTS: OnceLock<Mutex<HashMap<&'static str, &'static Histogram>>> = OnceLock::new();
        let lock = HISTS.get_or_init(|| Mutex::new(HashMap::new()));
        {
            let map = lock.lock();
            if let Some(h) = map.get(name) {
                return h;
            }
        }
        let boxed: &'static Histogram = Box::leak(Box::new(Histogram::new(buckets.to_vec())));
        let mut map = lock.lock();
        map.insert(name, boxed);
        MetricsRegistry::global().hists.lock().insert(name, boxed);
        boxed
    }

    pub fn export_text() -> String {
        let reg = MetricsRegistry::global();
        let mut out = String::new();
        for (name, c) in reg.counters.lock().iter() {
            out.push_str(&format!("counter{{name=\"{}\"}} {}\n", name, c.get()));
        }
        for (name, h) in reg.hists.lock().iter() {
            let (pairs, sum, count) = h.snapshot();
            for (b, v) in pairs {
                out.push_str(&format!(
                    "histogram_bucket{{name=\"{}\",le=\"{}\"}} {}\n",
                    name, b, v
                ));
            }
            out.push_str(&format!("histogram_sum{{name=\"{}\"}} {}\n", name, sum));
            out.push_str(&format!("histogram_count{{name=\"{}\"}} {}\n", name, count));
        }
        out
    }
}

pub struct Timer {
    start: std::time::Instant,
    hist: &'static Histogram,
}
impl Timer {
    pub fn start(hist: &'static Histogram) -> Self {
        Self {
            start: Instant::now(),
            hist,
        }
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        self.hist.observe(elapsed.as_micros() as u64);
    }
}

pub fn start_timer(name: &'static str, buckets: &[u64]) -> Timer {
    let h = MetricsRegistry::histogram(name, buckets);
    Timer::start(h)
}
pub fn export() -> String {
    MetricsRegistry::export_text()
}
/// 获取/创建一个计数器（便捷函数）
pub fn counter(name: &'static str) -> &'static Counter {
    MetricsRegistry::counter(name)
}
