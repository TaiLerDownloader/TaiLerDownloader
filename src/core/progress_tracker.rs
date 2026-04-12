use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[allow(dead_code)]
pub struct ProgressTracker {
    task_id: String,
    url: String,
    save_path: String,
    show_name: String,
    state: Arc<RwLock<TaskState>>,
    total_bytes: Arc<AtomicI64>,
    downloaded_bytes: Arc<AtomicI64>,
    speed_bps: Arc<RwLock<f64>>,
    start_time: Option<Instant>,
    last_update: Arc<RwLock<Instant>>,
    chunks_total: Arc<AtomicUsize>,
    chunks_completed: Arc<AtomicUsize>,
    error_message: Arc<RwLock<Option<String>>>,
    extra: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl ProgressTracker {
    pub fn new(
        task_id: impl Into<String>,
        url: impl Into<String>,
        save_path: impl Into<String>,
        show_name: impl Into<String>,
    ) -> Self {
        ProgressTracker {
            task_id: task_id.into(),
            url: url.into(),
            save_path: save_path.into(),
            show_name: show_name.into(),
            state: Arc::new(RwLock::new(TaskState::Pending)),
            total_bytes: Arc::new(AtomicI64::new(0)),
            downloaded_bytes: Arc::new(AtomicI64::new(0)),
            speed_bps: Arc::new(RwLock::new(0.0)),
            start_time: None,
            last_update: Arc::new(RwLock::new(Instant::now())),
            chunks_total: Arc::new(AtomicUsize::new(0)),
            chunks_completed: Arc::new(AtomicUsize::new(0)),
            error_message: Arc::new(RwLock::new(None)),
            extra: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_total_bytes(&self, bytes: i64) {
        self.total_bytes.store(bytes, Ordering::Relaxed);
    }

    pub fn add_downloaded(&self, bytes: i64) {
        self.downloaded_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn set_chunks_total(&self, count: usize) {
        self.chunks_total.store(count, Ordering::Relaxed);
    }

    pub fn inc_chunks_completed(&self) {
        self.chunks_completed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_error(&self, msg: impl Into<String>) {
        let mut error = self.error_message.write().unwrap();
        *error = Some(msg.into());
    }

    pub fn set_state(&self, state: TaskState) {
        let mut s = self.state.write().unwrap();
        *s = state;
    }

    pub fn start(&mut self) {
        let mut s = self.state.write().unwrap();
        *s = TaskState::Running;
    }

    pub fn get_progress(&self) -> ProgressSnapshot {
        let state = *self.state.read().unwrap();
        let total = self.total_bytes.load(Ordering::Relaxed);
        let downloaded = self.downloaded_bytes.load(Ordering::Relaxed);
        let current_speed = *self.speed_bps.read().unwrap();
        let chunks_total = self.chunks_total.load(Ordering::Relaxed);
        let chunks_completed = self.chunks_completed.load(Ordering::Relaxed);
        let error = self.error_message.read().unwrap().clone();

        let progress_percentage = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let elapsed = if let Some(start) = &self.start_time {
            start.elapsed().as_secs_f64()
        } else {
            0.0
        };

        ProgressSnapshot {
            task_id: self.task_id.clone(),
            url: self.url.clone(),
            save_path: self.save_path.clone(),
            show_name: self.show_name.clone(),
            state,
            total_bytes: total,
            downloaded_bytes: downloaded,
            progress_percentage,
            current_speed_bps: current_speed,
            chunks_total,
            chunks_completed,
            error_message: error,
            elapsed_seconds: elapsed,
        }
    }

    pub fn to_json(&self) -> String {
        let snapshot = self.get_progress();
        serde_json::to_string(&snapshot).unwrap_or_default()
    }

    pub fn set_extra(&self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        let mut extra = self.extra.write().unwrap();
        extra.insert(key.into(), value.into());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressSnapshot {
    pub task_id: String,
    pub url: String,
    pub save_path: String,
    pub show_name: String,
    pub state: TaskState,
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    pub progress_percentage: f64,
    pub current_speed_bps: f64,
    pub chunks_total: usize,
    pub chunks_completed: usize,
    pub error_message: Option<String>,
    pub elapsed_seconds: f64,
}

#[derive(Clone)]
pub struct ProgressReporter {
    trackers: Arc<RwLock<HashMap<String, Arc<ProgressTracker>>>>,
}

impl ProgressReporter {
    pub fn new() -> Self {
        ProgressReporter {
            trackers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, tracker: Arc<ProgressTracker>) {
        let mut trackers = self.trackers.write().unwrap();
        trackers.insert(tracker.task_id.clone(), tracker);
    }

    pub fn unregister(&self, task_id: &str) {
        let mut trackers = self.trackers.write().unwrap();
        trackers.remove(task_id);
    }

    pub fn get(&self, task_id: &str) -> Option<Arc<ProgressTracker>> {
        let trackers = self.trackers.read().unwrap();
        trackers.get(task_id).cloned()
    }

    pub fn get_all(&self) -> Vec<Arc<ProgressTracker>> {
        let trackers = self.trackers.read().unwrap();
        trackers.values().cloned().collect()
    }

    pub fn get_snapshot(&self, task_id: &str) -> Option<ProgressSnapshot> {
        self.get(task_id).map(|t| t.get_progress())
    }

    pub fn get_all_snapshots(&self) -> Vec<ProgressSnapshot> {
        let trackers = self.get_all();
        trackers.into_iter().map(|t| t.get_progress()).collect()
    }

    pub fn to_json(&self) -> String {
        let snapshots = self.get_all_snapshots();
        serde_json::to_string(&snapshots).unwrap_or_default()
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_REPORTER: once_cell::sync::Lazy<ProgressReporter> =
    once_cell::sync::Lazy::new(ProgressReporter::new);

pub fn get_global_reporter() -> &'static ProgressReporter {
    &GLOBAL_REPORTER
}
