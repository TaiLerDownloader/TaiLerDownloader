#![cfg(feature = "ed2k")]

use std::sync::Arc;
use tokio::sync::RwLock;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;
use super::file_utils::create_download_file;

/// ED2K downloader implementation
/// Parses ed2k://|file|<name>|<size>|<hash>|/ format URLs
/// Converts ED2K to HTTP via public gateway: https://ed2k.lyoko.io/hash/<hash>
pub struct ED2KDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

/// Parsed ED2K link info
struct Ed2kInfo {
    name: String,
    size: u64,
    hash: String,
}

impl ED2KDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;
        ED2KDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }

    /// Parse ed2k:// URL
    /// Format: ed2k://|file|<filename>|<filesize>|<md4hash>|/
    fn parse_ed2k_url(url: &str) -> Result<Ed2kInfo, String> {
        // Remove ed2k:// prefix
        let stripped = url.strip_prefix("ed2k://")
            .ok_or("Invalid ed2k:// URL")?;

        // Split by |: ["", "file", name, size, hash, "", ""]
        let parts: Vec<&str> = stripped.split('|').collect();

        if parts.len() < 5 {
            return Err(format!("Invalid ED2K URL format, insufficient parts: {}", url));
        }

        // Check type (currently only supports 'file')
        if parts[1] != "file" {
            return Err(format!("Unsupported ED2K type: '{}' (only 'file' supported)", parts[1]));
        }

        let _name = url::form_urlencoded::parse(parts[2].as_bytes())
            .next()
            .map(|(k, _)| k.into_owned())
            .unwrap_or_else(|| parts[2].to_string());
        // Simple URL decode (filename may have % encoding)
        let name = percent_decode(parts[2]);

        let size: u64 = parts[3].parse()
            .map_err(|_| format!("Failed to parse ED2K file size: '{}'", parts[3]))?;

        let hash = parts[4].to_string();
        if hash.len() != 32 {
            return Err(format!("Invalid ED2K hash length: {} (should be 32)", hash.len()));
        }

        Ok(Ed2kInfo { name, size, hash })
    }
}

/// Simple URL percent-decode (handles %XX sequences only)
fn percent_decode(s: &str) -> String {
    let mut result = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i+1..i+3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte as char);
                    i += 3;
                    continue;
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

#[async_trait::async_trait]
impl Downloader for ED2KDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ed2k_info = Self::parse_ed2k_url(&task.url)
            .map_err(|e| format!("Failed to parse ED2K URL: {}", e))?;

        eprintln!("ED2K Download: {} ({} bytes, hash={})",
            ed2k_info.name, ed2k_info.size, ed2k_info.hash);

        if let Some(ref monitor) = self.monitor {
            monitor.set_total_bytes(ed2k_info.size as i64);
        }

        // Build HTTP gateway URL (lyoko.io ED2K gateway)
        let gateway_url = format!("https://ed2k.lyoko.io/hash/{}", ed2k_info.hash);
        eprintln!("Downloading via HTTP gateway: {}", gateway_url);

        // Use reqwest for streaming download
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (compatible; TTHSDNext)")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // Try gateway request, return friendly error on failure
        let response = client.get(&gateway_url)
            .send().await
            .map_err(|e| format!("ED2K gateway request failed ({}): {}", gateway_url, e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "ED2K gateway returned HTTP {}: {}\n  hash={}\n  gateway={}",
                status.as_u16(), status.canonical_reason().unwrap_or("Unknown"),
                ed2k_info.hash, gateway_url
            ).into());
        }

        let total = response.content_length().unwrap_or(ed2k_info.size) as i64;
        if let Some(ref monitor) = self.monitor {
            monitor.set_total_bytes(total);
        }

        let mut file = create_download_file(&task.save_path, Some(total)).await?;

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

        eprintln!("ED2K download complete: {:.2} MB ({})",
            downloaded as f64 / 1024.0 / 1024.0, ed2k_info.name);
        Ok(())
    }

    fn get_type(&self) -> String {
        "ED2K".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for ED2KDownloader {
    fn default() -> Self {
        ED2KDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}