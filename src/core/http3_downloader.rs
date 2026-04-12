#![cfg(feature = "http3")]

use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use bytes::Buf;

use super::downloader_interface::{Downloader, BaseDownloader};
use super::downloader::{DownloadTask, DownloadConfig};
use super::performance_monitor::PerformanceMonitor;
use super::file_utils::create_download_file;

// HTTP/3 downloader
// Uses QUIC (quinn) + HTTP/3 (h3) for downloads
// Can fallback to HTTPDownloader on failure
pub struct HTTP3Downloader {
    base: BaseDownloader,
    monitor: Option<Arc<PerformanceMonitor>>,
}

impl HTTP3Downloader {
    pub async fn new(config: Arc<RwLock<DownloadConfig>>) -> Self {
        let monitor = super::performance_monitor::get_global_monitor().await;
        HTTP3Downloader {
            base: BaseDownloader {
                config: Some(config),
                running: true,
                ..Default::default()
            },
            monitor,
        }
    }

    /// Build rustls TLS config for QUIC
    fn build_tls_config() -> Result<Arc<rustls::ClientConfig>, Box<dyn std::error::Error + Send + Sync>> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Arc::new(tls_config))
    }

    /// Build QUIC endpoint
    fn build_quic_endpoint() -> Result<quinn::Endpoint, Box<dyn std::error::Error + Send + Sync>> {
        let tls_config = Self::build_tls_config()?;

        let quic_client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(tls_config.as_ref().clone())
                .map_err(|e| format!("QUIC TLS config failed: {}", e))?
        ));

        let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let mut endpoint = quinn::Endpoint::client(bind_addr)
            .map_err(|e| format!("Failed to create QUIC endpoint: {}", e))?;

        endpoint.set_default_client_config(quic_client_config);
        Ok(endpoint)
    }
}

#[async_trait::async_trait]
impl Downloader for HTTP3Downloader {
    async fn download(&mut self, task: &DownloadTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url_str = &task.url;

        // Parse URL
        let url = url::Url::parse(url_str)
            .map_err(|e| format!("Failed to parse URL: {}", e))?;

        let host = url.host_str()
            .ok_or("URL missing host")?
            .to_string();
        let port = url.port().unwrap_or(443);
        let path = url.path().to_string();
        let query = url.query().map(|q| format!("?{}", q)).unwrap_or_default();
        let full_path = format!("{}{}", path, query);

        eprintln!("HTTP/3 Download: {}:{}{}", host, port, full_path);

        // Build QUIC endpoint
        let endpoint = Self::build_quic_endpoint()
            .map_err(|e| format!("Failed to build QUIC endpoint: {}", e))?;

        // DNS resolution
        let addr_str = format!("{}:{}", host, port);
        let addrs: Vec<SocketAddr> = tokio::net::lookup_host(&addr_str).await
            .map_err(|e| format!("DNS resolution failed ({}): {}", addr_str, e))?
            .collect();

        let server_addr = addrs.into_iter().next()
            .ok_or_else(|| format!("Failed to resolve host: {}", host))?;

        // Establish QUIC connection
        let connecting = endpoint.connect(server_addr, &host)
            .map_err(|e| format!("Failed to initiate QUIC connection: {}", e))?;

        let quic_conn = connecting.await
            .map_err(|e| format!("QUIC handshake failed: {}", e))?;

        eprintln!("HTTP/3 QUIC handshake successful ({})", server_addr);

        // Establish h3 connection
        let (mut driver, mut send_request) = h3::client::new(h3_quinn::Connection::new(quic_conn))
            .await
            .map_err(|e| format!("Failed to establish HTTP/3 connection: {}", e))?;

        // Drive connection in background
        let _driver_task = tokio::spawn(async move {
            let _ = futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
        });

        // Build HTTP/3 GET request
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri(url_str.as_str())
            .header("host", &host)
            .header("user-agent", "TTHSDNext/1.0 (HTTP/3)")
            .header("accept", "*/*")
            .body(())
            .map_err(|e| format!("Failed to build HTTP/3 request: {}", e))?;

        let mut stream = send_request.send_request(request).await
            .map_err(|e| format!("Failed to send HTTP/3 request: {}", e))?;

        stream.finish().await
            .map_err(|e| format!("Failed to end HTTP/3 stream: {}", e))?;

        // Read response
        let response = stream.recv_response().await
            .map_err(|e| format!("Failed to receive HTTP/3 response: {}", e))?;

        let status = response.status();
        eprintln!("HTTP/3 response status: {}", status);

        if !status.is_success() {
            return Err(format!("HTTP/3 server returned error: {}", status).into());
        }

        // Get Content-Length from response headers
        let total = response.headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        if total > 0 {
            if let Some(ref monitor) = self.monitor {
                monitor.set_total_bytes(total);
            }
        }

        // Create output file
        let mut file = create_download_file(&task.save_path, Some(total)).await?;

        // Stream response body
        let mut downloaded: i64 = 0;
        // use tokio::io::AsyncWriteExt;

        loop {
            match stream.recv_data().await {
                Ok(Some(mut data)) => {
                    // data implements bytes::Buf
                    use tokio::io::AsyncWriteExt;
                    while data.has_remaining() {
                        let chunk_len = data.remaining().min(65536);
                        let chunk = data.chunk()[..chunk_len].to_vec();
                        file.write_all(&chunk).await
                            .map_err(|e| format!("Failed to write file: {}", e))?;
                        data.advance(chunk_len);
                        downloaded += chunk_len as i64;
                        if let Some(ref monitor) = self.monitor {
                            monitor.add_bytes(chunk_len as i64).await;
                        }
                    }
                }
                Ok(None) => break, // Response body ended
                Err(e) => return Err(format!("HTTP/3 data read error: {}", e).into()),
            }
        }

        eprintln!("HTTP/3 download complete: {:.2} MB", downloaded as f64 / 1024.0 / 1024.0);
        Ok(())
    }

    fn get_type(&self) -> String {
        "HTTP/3".to_string()
    }

    async fn cancel(&mut self, _downloader: Box<dyn Downloader>) {
        self.base.running = false;
    }

    async fn get_snapshot(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for HTTP3Downloader {
    fn default() -> Self {
        HTTP3Downloader {
            base: BaseDownloader::new(),
            monitor: None,
        }
    }
}