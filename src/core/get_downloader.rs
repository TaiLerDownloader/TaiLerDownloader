use std::sync::Arc;
use tokio::sync::RwLock;
use super::downloader::DownloadConfig;
use super::downloader_interface::Downloader;
use super::http_downloader::HTTPDownloader;

#[cfg(feature = "ftp")]
use super::ftp_downloader::FTPDownloader;
#[cfg(feature = "torrent")]
use super::torrent_downloader::TorrentDownloader;
#[cfg(feature = "metalink")]
use super::metalink_downloader::MetalinkDownloader;
#[cfg(feature = "ed2k")]
use super::ed2k_downloader::ED2KDownloader;
#[cfg(feature = "http3")]
use super::http3_downloader::HTTP3Downloader;
#[cfg(feature = "sftp")]
use super::sftp_downloader::SFTPDownloader;

/// Downloader factory function
/// Automatically routes to the appropriate downloader implementation based on URL scheme
/// All downloaders implement the `Downloader` trait, callers don't need to know the concrete type
/// Currently supported protocols:
/// - `http://`, `https://` -> HTTPDownloader
///
/// Planned support:
/// - `ftp://`, `ftps://`   -> FTPDownloader
/// - `sftp://`             -> SFTPDownloader
/// - `magnet:?`            -> TorrentDownloader (BT/DHT/Magnet)
/// - `ed2k://`             -> ED2KDownloader
pub async fn get_downloader(
    config: Arc<RwLock<DownloadConfig>>,
) -> Box<dyn Downloader> {
    let url = {
        let cfg = config.read().await;
        cfg.tasks.first()
           .map(|t| t.url.clone())
           .unwrap_or_default()
    };

    let scheme = detect_scheme(&url);

    match scheme {
        Protocol::Http => {
            #[cfg(feature = "http3")]
            {
                if probe_h3_support(&url).await {
                    eprintln!("Server supports HTTP/3, using QUIC download");
                    return Box::new(HTTP3Downloader::new(config).await) as Box<dyn Downloader>;
                }
            }
            Box::new(HTTPDownloader::new(config).await) as Box<dyn Downloader>
        }
        #[cfg(feature = "ftp")]
        Protocol::Ftp => Box::new(FTPDownloader::new(config).await),
        #[cfg(feature = "torrent")]
        Protocol::BitTorrent => Box::new(TorrentDownloader::new(config).await),
        #[cfg(feature = "ed2k")]
        Protocol::Ed2k => Box::new(ED2KDownloader::new(config).await),
        #[cfg(feature = "metalink")]
        Protocol::Metalink => Box::new(MetalinkDownloader::new(config).await),
        #[cfg(feature = "sftp")]
        Protocol::Sftp => Box::new(SFTPDownloader::new(config).await),
        _ => {
            eprintln!("Warning: Unknown protocol '{}', falling back to HTTP download", url.split("://").next().unwrap_or("unknown"));
            Box::new(HTTPDownloader::new(config).await)
        }
    }
}

/// Send HEAD request, check if Alt-Svc header contains h3
/// Timeout 800ms, return false on failure (non-blocking)
#[cfg(feature = "http3")]
async fn probe_h3_support(url: &str) -> bool {
    use std::time::Duration;

    // Reuse global HTTP client (if available), otherwise create temporary
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.head(url).send().await {
        Ok(resp) => {
            // Check Alt-Svc header: h3="..." or h3-29="..."
            resp.headers()
                .get("alt-svc")
                .and_then(|v| v.to_str().ok())
                .map(|s| {
                    let lower = s.to_lowercase();
                    lower.contains("h3=") || lower.contains("h3-")
                })
                .unwrap_or(false)
        }
        Err(_) => false,
    }
}

/// Supported download protocol enum
#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Http,
    Ftp,
    Sftp,
    BitTorrent,
    Ed2k,
    Metalink,
    Http3,
    Unknown,
}

/// Detect protocol type from URL string
fn detect_scheme(url: &str) -> Protocol {
    let lower = url.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Protocol::Http
    } else if lower.starts_with("ftp://") || lower.starts_with("ftps://") {
        Protocol::Ftp
    } else if lower.starts_with("sftp://") {
        Protocol::Sftp
    } else if lower.starts_with("magnet:") || lower.ends_with(".torrent") {
        Protocol::BitTorrent
    } else if lower.starts_with("ed2k://") {
        Protocol::Ed2k
    } else if lower.ends_with(".metalink") || lower.ends_with(".meta4") {
        Protocol::Metalink
    } else {
        Protocol::Unknown
    }
}