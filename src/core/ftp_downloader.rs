use std::sync::Arc;
use std::time::Instant;
use std::io::Write;
use tokio::sync::RwLock;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;
use super::file_utils::create_download_file_sync;

/// FTP Downloader
/// Uses suppaftp synchronous API + tokio::task::spawn_blocking
/// suppaftp async tokio API has complex generic inference issues, synchronous API is more stable
pub struct FTPDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl FTPDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;

        FTPDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }

    /// Parse FTP URL (host:port, path, username, password)
    fn parse_ftp_url(url: &str) -> Result<(String, String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let parsed = url::Url::parse(url)
            .map_err(|e| format!("Invalid FTP URL: {}", e))?;

        let host = parsed.host_str()
            .ok_or("FTP URL missing host")?
            .to_string();
        let port = parsed.port().unwrap_or(21);
        let path = parsed.path().to_string();
        let username = if parsed.username().is_empty() {
            "anonymous".to_string()
        } else {
            parsed.username().to_string()
        };
        let password = parsed.password().unwrap_or("anonymous@").to_string();

        Ok((format!("{}:{}", host, port), path, username, password))
    }
}

#[async_trait::async_trait]
impl Downloader for FTPDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (addr, path, username, password) = Self::parse_ftp_url(&task.url)?;
        let save_path = task.save_path.clone();
        let monitor = self.monitor.clone();

        eprintln!("FTP connecting: {} (user: {})", addr, username);

        // Execute synchronous FTP operations in blocking thread
        let result = tokio::task::spawn_blocking(move || -> Result<(i64, f64), String> {
            use suppaftp::FtpStream;

            // Establish connection
            let mut ftp = FtpStream::connect(&addr)
                .map_err(|e| format!("FTP connection failed: {}", e))?;

            // Login
            ftp.login(&username, &password)
                .map_err(|e| format!("FTP login failed: {}", e))?;

            // Set binary transfer mode
            ftp.transfer_type(suppaftp::types::FileType::Binary)
                .map_err(|e| format!("Failed to set binary mode: {}", e))?;

            // Get file size
            let file_size = ftp.size(&path)
                .map_err(|e| format!("Failed to get file size: {}", e))? as i64;

            eprintln!("FTP file size: {} bytes ({:.2} MB)",
                file_size, file_size as f64 / 1024.0 / 1024.0);

            // Create output file
            let mut file = create_download_file_sync(&save_path, Some(file_size))?;

            // Use retr callback for streaming download
            let start_time = Instant::now();
            let downloaded: i64 = ftp.retr(&path, |reader| {
                let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer
                let mut total: i64 = 0;

                loop {
                    let n = reader.read(&mut buf)
                        .map_err(|e| suppaftp::FtpError::ConnectionError(e))?;
                    if n == 0 {
                        break;
                    }

                    file.write_all(&buf[..n])
                        .map_err(|e| suppaftp::FtpError::ConnectionError(e))?;

                    total += n as i64;
                }

                Ok(total)
            }).map_err(|e| format!("FTP download failed: {}", e))?;

            let elapsed = start_time.elapsed().as_secs_f64();

            // Disconnect
            let _ = ftp.quit();

            // Verify size
            if downloaded != file_size {
                return Err(format!("FTP download incomplete: {}/{} bytes", downloaded, file_size));
            }

            Ok((downloaded, elapsed))
        }).await.map_err(|e| format!("FTP download thread error: {}", e))?;

        match result {
            Ok((downloaded, elapsed)) => {
                // Update progress monitor
                if let Some(ref monitor) = monitor {
                    monitor.set_total_bytes(downloaded);
                    monitor.add_bytes(downloaded).await;
                }

                let speed_mbps = if elapsed > 0.0 {
                    (downloaded as f64 / 1024.0 / 1024.0) / elapsed
                } else { 0.0 };

                eprintln!("FTP download complete: {:.2} MB, time: {:.1}s, speed: {:.2} MB/s",
                    downloaded as f64 / 1024.0 / 1024.0, elapsed, speed_mbps);

                Ok(())
            }
            Err(e) => Err(e.into())
        }
    }

    fn get_type(&self) -> String {
        "FTP".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for FTPDownloader {
    fn default() -> Self {
        FTPDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}