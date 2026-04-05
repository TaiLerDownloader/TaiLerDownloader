use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicI64, Ordering};

pub struct PerformanceMonitor {
    start_time: Instant,
    total_bytes: Arc<AtomicI64>,
    last_bytes: Arc<AtomicI64>,
    last_update_time: Arc<RwLock<Instant>>,
    current_speed: Arc<RwLock<f64>>,
    average_speed: Arc<RwLock<f64>>,
    peak_speed: Arc<RwLock<f64>>,
    total_expected_bytes: Arc<AtomicI64>,
    chunk_downloads: Arc<AtomicI64>,
    failed_chunks: Arc<AtomicI64>,
    retried_chunks: Arc<AtomicI64>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        PerformanceMonitor {
            start_time: Instant::now(),
            total_bytes: Arc::new(AtomicI64::new(0)),
            last_bytes: Arc::new(AtomicI64::new(0)),
            last_update_time: Arc::new(RwLock::new(Instant::now())),
            current_speed: Arc::new(RwLock::new(0.0)),
            average_speed: Arc::new(RwLock::new(0.0)),
            peak_speed: Arc::new(RwLock::new(0.0)),
            total_expected_bytes: Arc::new(AtomicI64::new(0)),
            chunk_downloads: Arc::new(AtomicI64::new(0)),
            failed_chunks: Arc::new(AtomicI64::new(0)),
            retried_chunks: Arc::new(AtomicI64::new(0)),
        }
    }

    pub async fn add_bytes(&self, bytes: i64) {
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.update_speed().await;
    }

    pub fn set_total_bytes(&self, bytes: i64) {
        self.total_expected_bytes.store(bytes, Ordering::Relaxed);
    }

    pub fn add_chunk_download(&self) {
        self.chunk_downloads.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_failed_chunk(&self) {
        self.failed_chunks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_retried_chunk(&self) {
        self.retried_chunks.fetch_add(1, Ordering::Relaxed);
    }

    async fn update_speed(&self) {
        let now = Instant::now();
        let last_update = {
            let lu = self.last_update_time.write().await;
            *lu
        };
        let duration = now.duration_since(last_update).as_secs_f64();

        if duration > 0.5 {
            let current_bytes = self.total_bytes.load(Ordering::Relaxed);
            let last_bytes = self.last_bytes.load(Ordering::Relaxed);
            let bytes_diff = current_bytes - last_bytes;

            let current_speed = bytes_diff as f64 / duration;

            {
                let mut peak = self.peak_speed.write().await;
                if current_speed > *peak {
                    *peak = current_speed;
                }
            }

            let total_duration = now.duration_since(self.start_time).as_secs_f64();
            let average_speed = if total_duration > 0.0 {
                current_bytes as f64 / total_duration
            } else {
                0.0
            };

            {
                let mut cs = self.current_speed.write().await;
                *cs = current_speed;
            }

            {
                let mut as_speed = self.average_speed.write().await;
                *as_speed = average_speed;
            }

            self.last_bytes.store(current_bytes, Ordering::Relaxed);
            {
                let mut lu = self.last_update_time.write().await;
                *lu = now;
            }
        }
    }

    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);
        let current_speed = *self.current_speed.read().await;
        let average_speed = *self.average_speed.read().await;
        let peak_speed = *self.peak_speed.read().await;
        let chunk_downloads = self.chunk_downloads.load(Ordering::Relaxed);
        let failed_chunks = self.failed_chunks.load(Ordering::Relaxed);
        let retried_chunks = self.retried_chunks.load(Ordering::Relaxed);
        let elapsed_time = self.start_time.elapsed().as_secs_f64();
        let total_expected = self.total_expected_bytes.load(Ordering::Relaxed);

        let mut stats = HashMap::new();
        stats.insert("total_bytes".to_string(), serde_json::Value::Number(serde_json::Number::from(total_bytes)));
        stats.insert("Total".to_string(), serde_json::Value::Number(serde_json::Number::from(total_expected)));
        stats.insert("current_speed_bps".to_string(), serde_json::Value::Number(serde_json::Number::from(current_speed as i64)));
        stats.insert("current_speed_mbps".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(current_speed / (1024.0 * 1024.0)).unwrap_or(serde_json::Number::from(0))));
        stats.insert("average_speed_bps".to_string(), serde_json::Value::Number(serde_json::Number::from(average_speed as i64)));
        stats.insert("average_speed_mbps".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(average_speed / (1024.0 * 1024.0)).unwrap_or(serde_json::Number::from(0))));
        stats.insert("peak_speed_bps".to_string(), serde_json::Value::Number(serde_json::Number::from(peak_speed as i64)));
        stats.insert("peak_speed_mbps".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(peak_speed / (1024.0 * 1024.0)).unwrap_or(serde_json::Number::from(0))));
        stats.insert("chunk_downloads".to_string(), serde_json::Value::Number(serde_json::Number::from(chunk_downloads)));
        stats.insert("failed_chunks".to_string(), serde_json::Value::Number(serde_json::Number::from(failed_chunks)));
        stats.insert("retried_chunks".to_string(), serde_json::Value::Number(serde_json::Number::from(retried_chunks)));
        stats.insert("elapsed_time".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(elapsed_time).unwrap_or(serde_json::Number::from(0))));

        stats
    }

    pub async fn print_stats(&self) {
        let stats = self.get_stats().await;

        println!("=== 下载性能统计 ===");
        if let Some(total_bytes) = stats.get("total_bytes").and_then(|v| v.as_i64()) {
            println!("总下载量: {:.2} MB", total_bytes as f64 / (1024.0 * 1024.0));
        }
        if let Some(current_speed_mbps) = stats.get("current_speed_mbps").and_then(|v| v.as_f64()) {
            println!("当前速度: {:.2} MB/s", current_speed_mbps);
        }
        if let Some(average_speed_mbps) = stats.get("average_speed_mbps").and_then(|v| v.as_f64()) {
            println!("平均速度: {:.2} MB/s", average_speed_mbps);
        }
        if let Some(peak_speed_mbps) = stats.get("peak_speed_mbps").and_then(|v| v.as_f64()) {
            println!("峰值速度: {:.2} MB/s", peak_speed_mbps);
        }
        if let Some(chunk_downloads) = stats.get("chunk_downloads").and_then(|v| v.as_i64()) {
            println!("分块下载�? {}", chunk_downloads);
        }
        if let Some(failed_chunks) = stats.get("failed_chunks").and_then(|v| v.as_i64()) {
            println!("失败分块: {}", failed_chunks);
        }
        if let Some(retried_chunks) = stats.get("retried_chunks").and_then(|v| v.as_i64()) {
            println!("重试分块: {}", retried_chunks);
        }
        if let Some(elapsed_time) = stats.get("elapsed_time").and_then(|v| v.as_f64()) {
            println!("运行时间: {:.1} 秒", elapsed_time);
        }
    }
}

static GLOBAL_MONITOR: tokio::sync::OnceCell<Arc<PerformanceMonitor>> = tokio::sync::OnceCell::const_new();

pub async fn get_global_monitor() -> Option<Arc<PerformanceMonitor>> {
    GLOBAL_MONITOR.get_or_init(|| async {
        Arc::new(PerformanceMonitor::new())
    }).await.clone().into()
}

pub fn get_global_monitor_blocking() -> Option<Arc<PerformanceMonitor>> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(get_global_monitor())
}
