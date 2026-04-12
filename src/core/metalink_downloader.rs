#![cfg(feature = "metalink")]

use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::RwLock;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;
use super::file_utils::create_download_file;

/// Metalink Downloader
/// Supports Metalink 4.0 (.metalink / .meta4) format
/// Parses XML file to extract mirror URL list, selects best mirror for HTTP download
pub struct MetalinkDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl MetalinkDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;
        MetalinkDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }
}

#[async_trait::async_trait]
impl Downloader for MetalinkDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = &task.url;
        let save_path = task.save_path.clone();

        eprintln!("Metalink download: {}", url);

        // 1. Get .metalink / .meta4 file content
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let xml_text = client.get(url)
            .send().await
            .map_err(|e| format!("Failed to fetch Metalink file: {}", e))?
            .text().await
            .map_err(|e| format!("Failed to read Metalink content: {}", e))?;

        // 2. Parse Metalink XML
        let metalink = metalink::Metalink4::from_str(&xml_text)
            .map_err(|e| format!("Failed to parse Metalink: {}", e))?;

        if metalink.files.is_empty() {
            return Err("No file entries found in Metalink file".into());
        }

        // 3. Get first file entry (usually only one)
        let file_entry = &metalink.files[0];
        let file_name = &file_entry.name;

        eprintln!("Metalink file name: {}", file_name);
        if let Some(size) = file_entry.size {
            eprintln!("Metalink file size: {} bytes ({:.2} MB)", size, size as f64 / 1024.0 / 1024.0);
            if let Some(ref monitor) = self.monitor {
                monitor.set_total_bytes(size as i64);
            }
        }

        // 4. Extract all HTTP(S) mirror URLs, sorted by priority
        let mut mirror_urls: Vec<(u32, String)> = file_entry.urls.iter()
            .filter_map(|u| {
                let url_str = u.value.clone();
                if url_str.starts_with("http://") || url_str.starts_with("https://") {
                    // Lower priority is better (Metalink spec)
                    Some((u.priority.unwrap_or(999999), url_str))
                } else {
                    None
                }
            })
            .collect();

        mirror_urls.sort_by_key(|(priority, _)| *priority);

        if mirror_urls.is_empty() {
            return Err("No available HTTP(S) mirror URLs in Metalink".into());
        }

        eprintln!("Found {} mirror URLs, using highest priority mirror", mirror_urls.len());
        for (p, u) in &mirror_urls {
            eprintln!("  [priority {}] {}", p, u);
        }

        // 5. Build download task, use first (highest priority) URL
        //    Actually download via HTTP downloader
        let best_url = mirror_urls[0].1.clone();
        eprintln!("Selected mirror: {}", best_url);

        // Directly use reqwest streaming download (to avoid circular dependency with HTTPDownloader)
        let response = client.get(&best_url)
            .send().await
            .map_err(|e| format!("Metalink HTTP request failed: {}", e))?;

        let total = response.content_length().unwrap_or(0) as i64;
        if total > 0 {
            if let Some(ref monitor) = self.monitor {
                monitor.set_total_bytes(total);
            }
        }

        let mut file = create_download_file(&save_path, Some(total)).await?;

        let mut stream = response.bytes_stream();
        let mut downloaded: i64 = 0;

        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| format!("Stream read error: {}", e))?;
            file.write_all(&bytes).await
                .map_err(|e| format!("Write failed: {}", e))?;
            downloaded += bytes.len() as i64;
            if let Some(ref monitor) = self.monitor {
                monitor.add_bytes(bytes.len() as i64).await;
            }
        }

        eprintln!("Metalink download complete: {:.2} MB", downloaded as f64 / 1024.0 / 1024.0);
        Ok(())
    }

    fn get_type(&self) -> String {
        "Metalink".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for MetalinkDownloader {
    fn default() -> Self {
        MetalinkDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}