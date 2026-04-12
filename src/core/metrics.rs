use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct Metric {
    value: AtomicU64,
}

impl Metric {
    pub const fn new() -> Self {
        Metric {
            value: AtomicU64::new(0),
        }
    }

    pub fn set(&self, value: u64) {
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn add(&self, value: u64) {
        self.value.fetch_add(value, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct Metrics {
    pub download_total_bytes: Arc<Metric>,
    pub download_speed_bps: Arc<Metric>,
    pub download_peak_speed_bps: Arc<Metric>,
    pub active_tasks: Arc<Metric>,
    pub completed_tasks: Arc<Metric>,
    pub failed_tasks: Arc<Metric>,
    pub retry_count: Arc<Metric>,
    pub connection_count: Arc<Metric>,
    pub memory_usage_bytes: Arc<Metric>,
}

impl Metrics {
    pub fn new() -> Self {
        Metrics {
            download_total_bytes: Arc::new(Metric::new()),
            download_speed_bps: Arc::new(Metric::new()),
            download_peak_speed_bps: Arc::new(Metric::new()),
            active_tasks: Arc::new(Metric::new()),
            completed_tasks: Arc::new(Metric::new()),
            failed_tasks: Arc::new(Metric::new()),
            retry_count: Arc::new(Metric::new()),
            connection_count: Arc::new(Metric::new()),
            memory_usage_bytes: Arc::new(Metric::new()),
        }
    }

    pub fn record_total_bytes(&self, bytes: u64) {
        self.download_total_bytes.set(bytes);
    }

    pub fn record_speed(&self, bps: u64) {
        self.download_speed_bps.set(bps);

        if bps > self.download_peak_speed_bps.get() {
            self.download_peak_speed_bps.set(bps);
        }
    }

    pub fn inc_active_tasks(&self) {
        self.active_tasks.inc();
    }

    pub fn dec_active_tasks(&self) {
        let current = self.active_tasks.get();
        if current > 0 {
            self.active_tasks.set(current - 1);
        }
    }

    pub fn inc_completed_tasks(&self) {
        self.completed_tasks.inc();
        self.dec_active_tasks();
    }

    pub fn inc_failed_tasks(&self) {
        self.failed_tasks.inc();
        self.dec_active_tasks();
    }

    pub fn inc_retry(&self) {
        self.retry_count.inc();
    }

    pub fn set_connections(&self, count: u64) {
        self.connection_count.set(count);
    }

    pub fn set_memory_usage(&self, bytes: u64) {
        self.memory_usage_bytes.set(bytes);
    }

    pub fn to_prometheus(&self) -> String {
        let metrics = [
            (
                "tthsd_download_total_bytes",
                self.download_total_bytes.get(),
            ),
            ("tthsd_download_speed_bps", self.download_speed_bps.get()),
            (
                "tthsd_download_peak_speed_bps",
                self.download_peak_speed_bps.get(),
            ),
            ("tthsd_active_tasks", self.active_tasks.get()),
            ("tthsd_completed_tasks", self.completed_tasks.get()),
            ("tthsd_failed_tasks", self.failed_tasks.get()),
            ("tthsd_retry_count", self.retry_count.get()),
            ("tthsd_connection_count", self.connection_count.get()),
            ("tthsd_memory_usage_bytes", self.memory_usage_bytes.get()),
        ];

        metrics
            .iter()
            .map(|(name, value)| format!("{} {}", name, value))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn to_json(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert(
            "download_total_bytes".into(),
            serde_json::Value::from(self.download_total_bytes.get()),
        );
        map.insert(
            "download_speed_bps".into(),
            serde_json::Value::from(self.download_speed_bps.get()),
        );
        map.insert(
            "download_peak_speed_bps".into(),
            serde_json::Value::from(self.download_peak_speed_bps.get()),
        );
        map.insert(
            "active_tasks".into(),
            serde_json::Value::from(self.active_tasks.get()),
        );
        map.insert(
            "completed_tasks".into(),
            serde_json::Value::from(self.completed_tasks.get()),
        );
        map.insert(
            "failed_tasks".into(),
            serde_json::Value::from(self.failed_tasks.get()),
        );
        map.insert(
            "retry_count".into(),
            serde_json::Value::from(self.retry_count.get()),
        );
        map.insert(
            "connection_count".into(),
            serde_json::Value::from(self.connection_count.get()),
        );
        map.insert(
            "memory_usage_bytes".into(),
            serde_json::Value::from(self.memory_usage_bytes.get()),
        );
        map
    }
}

static GLOBAL_METRICS: once_cell::sync::Lazy<Metrics> = once_cell::sync::Lazy::new(Metrics::new);

pub fn get_global_metrics() -> &'static Metrics {
    &GLOBAL_METRICS
}
