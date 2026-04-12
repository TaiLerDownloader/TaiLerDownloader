#![cfg(feature = "sftp")]

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;
use super::file_utils::create_download_file;

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

/// KnownHosts - stores verified host key fingerprints
struct KnownHosts {
    keys: tokio::sync::RwLock<HashMap<String, String>>,
}

impl KnownHosts {
    fn new() -> Self {
        KnownHosts { keys: tokio::sync::RwLock::new(HashMap::new()) }
    }

    async fn add(&self, host: String, fingerprint: String) {
        self.keys.write().await.insert(host, fingerprint);
    }

    async fn get(&self, host: &str) -> Option<String> {
        self.keys.read().await.get(host).cloned()
    }
}

static KNOWN_HOSTS: once_cell::sync::Lazy<KnownHosts> = once_cell::sync::Lazy::new(KnownHosts::new);

/// SSH Handler with host key verification
struct SshHandler {
    host: String,
    accept_new: std::sync::atomic::AtomicBool,
}

impl SshHandler {
    fn new(host: String) -> Self {
        SshHandler {
            host,
            accept_new: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn allow_new(&self) {
        self.accept_new.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl russh::client::Handler for SshHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let host = self.host.clone();
        let accept = self.accept_new.load(std::sync::atomic::Ordering::SeqCst);

        async move {
            let algo = format!("{:?}", server_public_key.algorithm());
            let fp = format!("{:?}", server_public_key.fingerprint(russh::keys::HashAlg::Sha256));

            if let Some(saved) = KNOWN_HOSTS.get(&host).await {
                let accepted = saved == fp;
                if !accepted {
                    eprintln!("[SECURITY] Host key changed for {}", host);
                }
                return Ok(accepted);
            }

            eprintln!("[SECURITY] New SSH host key: {} ({})", host, algo);
            eprintln!("  Fingerprint: {}", fp);

            KNOWN_HOSTS.add(host, fp).await;
            Ok(accept)
        }
    }
}

#[async_trait::async_trait]
impl Downloader for SFTPDownloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (host, port, remote_path, username, password) = Self::parse_sftp_url(&task.url)?;
        let save_path = task.save_path.clone();
        let monitor = self.monitor.clone();

        eprintln!("SFTP connecting: {}@{}:{}", username, host, port);

        let config = russh::client::Config::default();
        let config = Arc::new(config);

        let handler = SshHandler::new(host.clone());
        handler.allow_new();
        let mut session = russh::client::connect(config, (host.as_str(), port), handler)
            .await
            .map_err(|e| format!("SSH failed: {}", e))?;

        let auth_result = session.authenticate_password(&username, &password)
            .await
            .map_err(|e| format!("Auth failed: {}", e))?;

        if !auth_result.success() {
            return Err("Auth rejected".into());
        }

        let channel = session.channel_open_session()
            .await
            .map_err(|e| format!("Channel failed: {}", e))?;

        channel.request_subsystem(true, "sftp")
            .await
            .map_err(|e| format!("SFTP failed: {}", e))?;

        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| format!("SFTP init failed: {}", e))?;

        let metadata = sftp.metadata(&remote_path)
            .await
            .map_err(|e| format!("Metadata failed: {}", e))?;

        let file_size = metadata.size.unwrap_or(0) as i64;

        let mut remote_file = sftp.open(&remote_path)
            .await
            .map_err(|e| format!("Open failed: {}", e))?;

        let mut local_file = create_download_file(&save_path, Some(file_size)).await?;

        let start_time = Instant::now();
        let mut downloaded: i64 = 0;

        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = remote_file.read(&mut buf).await.map_err(|e| e.to_string())?;
            if n == 0 { break; }
            local_file.write_all(&buf[..n]).await.map_err(|e| e.to_string())?;
            downloaded += n as i64;
        }

        local_file.flush().await.map_err(|e| e.to_string())?;
        let elapsed = start_time.elapsed().as_secs_f64();

        if file_size > 0 && downloaded != file_size {
            return Err(format!("Incomplete: {}/{}", downloaded, file_size).into());
        }

        if let Some(ref m) = monitor {
            m.set_total_bytes(downloaded);
            m.add_bytes(downloaded).await;
        }

        eprintln!("Done: {:.2}MB {:.1}s", downloaded as f64 / 1048576.0, elapsed);

        let _ = session.disconnect(russh::Disconnect::ByApplication, "", "en").await;
        Ok(())
    }

    fn get_type(&self) -> String { "SFTP".to_string() }

    async fn cancel(&mut self, _: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> { None }
}

impl Default for SFTPDownloader {
    fn default() -> Self {
        SFTPDownloader { base: BaseDownloader::new(), monitor: None }
    }
}