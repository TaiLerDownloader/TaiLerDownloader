use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;
use super::file_utils::create_download_file;
use russh::keys::ssh_key;

/// SFTP downloader
/// Uses russh (pure Rust SSH) + russh-sftp for async SFTP file downloads
pub struct SFTPDownloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl SFTPDownloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;

        SFTPDownloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }

    /// Parse SFTP URL -> (host, port, path, username, password)
    /// Format: sftp://[user[:password]@]host[:port]/path/to/file
    fn parse_sftp_url(url: &str) -> Result<(String, u16, String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let parsed = url::Url::parse(url)
            .map_err(|e| format!("Invalid SFTP URL: {}", e))?;

        let host = parsed.host_str()
            .ok_or("SFTP URL missing host")?
            .to_string();
        let port = parsed.port().unwrap_or(22);
        let path = parsed.path().to_string();
        let username = if parsed.username().is_empty() {
            "root".to_string()
        } else {
            parsed.username().to_string()
        };
        let password = parsed.password().unwrap_or("").to_string();

        if path.is_empty() || path == "/" {
            return Err("SFTP URL missing file path".into());
        }

        Ok((host, port, path, username, password))
    }
}

/// russh requires a Handler to process SSH session events
/// Minimal implementation: accept all host keys, no extra processing
struct SshHandler;

#[async_trait::async_trait]
impl russh::client::Handler for SshHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        async { Ok(true) }
    }
}

#[async_trait::async_trait]
impl Downloader for SFTPDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (host, port, remote_path, username, password) = Self::parse_sftp_url(&task.url)?;
        let save_path = task.save_path.clone();
        let monitor = self.monitor.clone();

        eprintln!("SFTP connecting: {}@{}:{} path: {}", username, host, port, remote_path);

        // 1. Configure SSH client
        let config = russh::client::Config::default();
        let config = Arc::new(config);

        // 2. Establish SSH connection
        let mut session = russh::client::connect(config, (host.as_str(), port), SshHandler)
            .await
            .map_err(|e| format!("SSH connection failed: {}", e))?;

        // 3. Password authentication
        let auth_result = session.authenticate_password(&username, &password)
            .await
            .map_err(|e| format!("SSH authentication failed: {}", e))?;

        if !auth_result.success() {
            return Err("SSH password authentication rejected".into());
        }

        eprintln!("SSH authentication successful");

        // 4. Open SFTP channel
        let channel = session.channel_open_session()
            .await
            .map_err(|e| format!("Failed to open SSH channel: {}", e))?;

        channel.request_subsystem(true, "sftp")
            .await
            .map_err(|e| format!("Failed to request SFTP subsystem: {}", e))?;

        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| format!("Failed to initialize SFTP session: {}", e))?;

        eprintln!("SFTP session established");

        // 5. Get remote file info
        let metadata = sftp.metadata(&remote_path)
            .await
            .map_err(|e| format!("Failed to get remote file info: {}", e))?;

        let file_size = metadata.size.unwrap_or(0) as i64;
        eprintln!("SFTP file size: {} bytes ({:.2} MB)",
            file_size, file_size as f64 / 1024.0 / 1024.0);

        // 6. Open remote file
        let mut remote_file = sftp.open(&remote_path)
            .await
            .map_err(|e| format!("Failed to open remote file: {}", e))?;

        // 7. Create local file and write
        let mut local_file = create_download_file(&save_path, Some(file_size)).await?;

        let start_time = Instant::now();
        let mut downloaded: i64 = 0;

        // Streaming copy
        use tokio::io::AsyncReadExt;
        use tokio::io::AsyncWriteExt;

        let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer
        loop {
            let n = remote_file.read(&mut buf)
                .await
                .map_err(|e| format!("Failed to read remote file: {}", e))?;
            if n == 0 {
                break;
            }

            local_file.write_all(&buf[..n])
                .await
                .map_err(|e| format!("Failed to write local file: {}", e))?;

            downloaded += n as i64;
        }

        local_file.flush()
            .await
            .map_err(|e| format!("Failed to flush file buffer: {}", e))?;

        let elapsed = start_time.elapsed().as_secs_f64();

        // 8. Verify size
        if file_size > 0 && downloaded != file_size {
            return Err(format!("SFTP download incomplete: {}/{} bytes", downloaded, file_size).into());
        }

        // 9. Update performance monitor
        if let Some(ref monitor) = monitor {
            monitor.set_total_bytes(downloaded);
            monitor.add_bytes(downloaded).await;
        }

        let speed_mbps = if elapsed > 0.0 {
            (downloaded as f64 / 1024.0 / 1024.0) / elapsed
        } else { 0.0 };

        eprintln!("SFTP download complete: {:.2} MB, elapsed {:.1}s, speed {:.2} MB/s",
            downloaded as f64 / 1024.0 / 1024.0, elapsed, speed_mbps);

        // 10. Close SSH session
        let _ = session.disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;

        Ok(())
    }

    fn get_type(&self) -> String {
        "SFTP".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for SFTPDownloader {
    fn default() -> Self {
        SFTPDownloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}