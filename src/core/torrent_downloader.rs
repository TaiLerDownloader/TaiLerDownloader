#![cfg(feature = "torrent")]

use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;

use librqbit::{AddTorrent, AddTorrentOptions, Session};

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;

/// BitTorrent Downloader
/// Supports magnet: links, torrent file URLs, DHT network, PEX (Peer Exchange)
/// Based on librqbit - Rust BitTorrent client library
pub struct TorrentDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl TorrentDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;

        TorrentDownloader {
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
impl Downloader for TorrentDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Determine output directory (get parent from save_path)
        let save_path = PathBuf::from(&task.save_path);
        let output_dir = save_path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        eprintln!("BT download: {} -> {:?}", task.url, output_dir);

        // Create librqbit Session
        let session = Session::new(output_dir).await
            .map_err(|e| format!("Failed to create BT Session: {}", e))?;

        // Build AddTorrent parameters
        let add_torrent = if task.url.starts_with("magnet:") {
            AddTorrent::from_url(&task.url)
        } else if task.url.ends_with(".torrent") {
            // .torrent file URL - first download file content
            AddTorrent::from_url(&task.url)
        } else {
            return Err(format!("Unsupported BT URL format: {}", task.url).into());
        };

        // Add torrent and start download
        let opts = AddTorrentOptions {
            ..Default::default()
        };

        let response = session.add_torrent(add_torrent, Some(opts)).await
            .map_err(|e| format!("Failed to add torrent: {}", e))?;

        let handle = match response {
            librqbit::AddTorrentResponse::Added(_, handle) => handle,
            librqbit::AddTorrentResponse::AlreadyManaged(_, handle) => {
                eprintln!("BT torrent already exists, continuing download");
                handle
            }
            librqbit::AddTorrentResponse::ListOnly(_info) => {
                eprintln!("BT torrent added in list-only mode");
                return Err("Torrent added in list-only mode, download not started".into());
            }
        };

        eprintln!("BT download started, waiting for completion...");

        // Poll for download completion
        let mut last_reported_bytes: u64 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let stats = handle.stats();
            let downloaded = stats.progress_bytes;
            let total = stats.total_bytes;

            // Update progress
            if let Some(ref monitor) = self.monitor {
                let new_bytes = downloaded.saturating_sub(last_reported_bytes);
                if new_bytes > 0 {
                    if last_reported_bytes == 0 {
                        monitor.set_total_bytes(total as i64);
                    }
                    monitor.add_bytes(new_bytes as i64).await;
                    last_reported_bytes = downloaded;
                }
            }

            // Check if completed
            if downloaded >= total && total > 0 {
                eprintln!("BT download complete: {:.2} MB", total as f64 / 1024.0 / 1024.0);
                break;
            }

            // Check if cancelled
            if !self.base.running {
                eprintln!("BT download cancelled by user");
                return Err("BT download cancelled by user".into());
            }
        }

        Ok(())
    }

    fn get_type(&self) -> String {
        "BitTorrent".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for TorrentDownloader {
    fn default() -> Self {
        TorrentDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}