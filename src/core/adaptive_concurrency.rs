use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};

pub struct AdaptiveConcurrency {
    min_threads: Arc<AtomicUsize>,
    max_threads: Arc<AtomicUsize>,
    current_threads: Arc<AtomicUsize>,
    target_speed_bps: Arc<AtomicU64>,
    speed_history: Arc<RwLock<Vec<(Instant, f64)>>>,
    stagnation_count: Arc<AtomicUsize>,
}

impl AdaptiveConcurrency {
    pub fn new(min_threads: usize, max_threads: usize) -> Self {
        AdaptiveConcurrency {
            min_threads: Arc::new(AtomicUsize::new(min_threads)),
            max_threads: Arc::new(AtomicUsize::new(max_threads)),
            current_threads: Arc::new(AtomicUsize::new(min_threads)),
            target_speed_bps: Arc::new(AtomicU64::new(0)),
            speed_history: Arc::new(RwLock::new(Vec::new())),
            stagnation_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn set_target_speed(&self, bps: u64) {
        self.target_speed_bps.store(bps, Ordering::Relaxed);
    }

    pub fn record_speed(&self, current_speed: f64) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut history = self.speed_history.write().await;
            history.push((Instant::now(), current_speed));

            let now = Instant::now();
            history.retain(|(time, _)| now.duration_since(*time).as_secs_f64() < 10.0);
        });
    }

    pub fn should_increase(&self) -> bool {
        let current = self.current_threads.load(Ordering::Relaxed);
        let max = self.max_threads.load(Ordering::Relaxed);
        current < max
    }

    pub fn should_decrease(&self) -> bool {
        let current = self.current_threads.load(Ordering::Relaxed);
        let min = self.min_threads.load(Ordering::Relaxed);
        current > min
    }

    pub fn adjust(&self, current_speed: f64) -> usize {
        self.record_speed(current_speed);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let history = self.speed_history.read().await;
            if history.len() < 2 {
                return self.current_threads.load(Ordering::Relaxed);
            }

            let target = self.target_speed_bps.load(Ordering::Relaxed) as f64;
            let avg_speed = history.iter()
                .map(|(_, s)| s)
                .sum::<f64>() / history.len() as f64;

            let current = self.current_threads.load(Ordering::Relaxed);

            if target > 0.0 && avg_speed < target * 0.8 && self.should_increase() {
                self.current_threads.fetch_add(1, Ordering::Relaxed);
                self.stagnation_count.store(0, Ordering::Relaxed);
            } else if avg_speed > target * 1.2 && self.should_decrease() && current > self.min_threads.load(Ordering::Relaxed) {
                self.current_threads.fetch_sub(1, Ordering::Relaxed);
                self.stagnation_count.store(0, Ordering::Relaxed);
            } else if (target == 0.0 || avg_speed >= target) && self.stagnation_count.load(Ordering::Relaxed) > 3 && self.should_decrease() {
                self.current_threads.fetch_sub(1, Ordering::Relaxed);
            } else {
                self.stagnation_count.fetch_add(1, Ordering::Relaxed);
            }

            self.current_threads.load(Ordering::Relaxed)
        })
    }

    pub fn get_current_threads(&self) -> usize {
        self.current_threads.load(Ordering::Relaxed)
    }
}

impl Default for AdaptiveConcurrency {
    fn default() -> Self {
        Self::new(1, 64)
    }
}