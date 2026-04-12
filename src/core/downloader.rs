use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
#[cfg(feature = "websocket")]
use super::websocket_client::WebSocketClient;
#[cfg(feature = "socket")]
use super::socket_client::SocketClient;
use super::send_message::send_message;
use super::performance_monitor::get_global_monitor;

pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub url: String,
    pub save_path: String,
    pub show_name: String,
    pub id: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
}

pub type ProgressCallback = extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char);

#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub tasks: Vec<DownloadTask>,
    pub thread_count: usize,
    pub chunk_size_mb: usize,
    pub callback_func: Option<ProgressCallback>,
    pub use_callback_url: bool,
    pub callback_url: Option<String>,
    pub use_socket: Option<bool>,
    pub show_name: String,
    pub user_agent: String,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
    pub speed_limit_bps: u64,
    pub proxy_url: Option<String>,
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct DownloadChunk {
    pub start_offset: i64,
    pub end_offset: i64,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "startOne")]
    StartOne,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "end")]
    End,
    #[serde(rename = "endOne")]
    EndOne,
    #[serde(rename = "msg")]
    Msg,
    #[serde(rename = "err")]
    Err,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "Type")]
    pub event_type: EventType,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "ShowName")]
    pub show_name: String,
    #[serde(rename = "ID")]
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub total: i64,
    pub downloaded: i64,
}

pub struct HSDownloader {
    pub config: Arc<RwLock<DownloadConfig>>,
    #[cfg(feature = "websocket")]
    pub ws_client: Option<Arc<tokio::sync::Mutex<WebSocketClient>>>,
    #[cfg(not(feature = "websocket"))]
    pub ws_client: Option<Arc<tokio::sync::Mutex<()>>>,
    #[cfg(feature = "socket")]
    pub socket_client: Option<Arc<tokio::sync::Mutex<SocketClient>>>,
    #[cfg(not(feature = "socket"))]
    pub socket_client: Option<Arc<tokio::sync::Mutex<()>>>,
    pub cancel_token: Arc<tokio::sync::Mutex<Option<tokio_util::sync::CancellationToken>>>,
    pub current_task_index: Arc<tokio::sync::Mutex<usize>>,
}

impl HSDownloader {
    pub fn merge_headers(global_headers: &std::collections::HashMap<String, String>, task_headers: &std::collections::HashMap<String, String>) -> std::collections::HashMap<String, String> {
        let mut merged = global_headers.clone();
        for (key, value) in task_headers {
            merged.insert(key.clone(), value.clone());
        }
        merged
    }

    pub fn new(config: DownloadConfig) -> Self {
        let config = Arc::new(RwLock::new(config));

        let (ws_client, socket_client) = {
            let cfg = config.blocking_read();
            let mut ws_client = None;
            let mut socket_client = None;

            if cfg.use_callback_url {
                if let Some(ref callback_url) = cfg.callback_url {
                    if let Some(use_socket) = cfg.use_socket {
                        #[cfg(feature = "socket")]
                        if use_socket {
                            socket_client = Some(Arc::new(tokio::sync::Mutex::new(SocketClient::new(callback_url.clone()))));
                        }
                        #[cfg(feature = "websocket")]
                        if !use_socket {
                            ws_client = Some(Arc::new(tokio::sync::Mutex::new(WebSocketClient::new(callback_url.clone()))));
                        }
                    }
                }
            }
            (ws_client, socket_client)
        };

        HSDownloader {
            config,
            ws_client,
            socket_client,
            cancel_token: Arc::new(tokio::sync::Mutex::new(None)),
            current_task_index: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }

    pub fn get_downloader(tasks: Vec<DownloadTask>, thread_count: usize, chunk_size_mb: usize) -> Self {
        let num_cpus = num_cpus::get();
        let thread_count = if thread_count == 0 { num_cpus * 2 } else { thread_count };
        let chunk_size_mb = if chunk_size_mb == 0 { 10 } else { chunk_size_mb };

        let config = DownloadConfig {
            tasks,
            thread_count,
            chunk_size_mb,
            callback_func: None,
            use_callback_url: false,
            callback_url: None,
            use_socket: None,
            show_name: String::new(),
            user_agent: UA.to_string(),
            max_retries: 3,
            retry_delay_ms: 1000,
            max_retry_delay_ms: 30000,
            speed_limit_bps: 0,
            proxy_url: None,
            headers: std::collections::HashMap::new(),
        };

        Self::new(config)
    }

    pub async fn start_download(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cancel_guard = self.cancel_token.lock().await;
        if cancel_guard.is_some() {
            drop(cancel_guard);
            return Err("downloader already running".into());
        }

        let token = tokio_util::sync::CancellationToken::new();
        *cancel_guard = Some(token.clone());
        drop(cancel_guard);

        let event = Event {
            event_type: EventType::Start,
            name: "Start Download".to_string(),
            show_name: "Global".to_string(),
            id: String::new(),
        };

        send_message(event, HashMap::new(), &self.config, &self.ws_client, &self.socket_client).await?;

        let tasks = {
            let config = self.config.read().await;
            config.tasks.clone()
        };

        let mut join_set = tokio::task::JoinSet::new();

        for (index, task) in tasks.into_iter().enumerate() {
            let token_clone = token.clone();
            let config = self.config.clone();
            let ws_client = self.ws_client.clone();
            let socket_client = self.socket_client.clone();

            join_set.spawn(async move {
                Self::download_task(
                    task,
                    index,
                    token_clone,
                    config,
                    ws_client,
                    socket_client,
                ).await
            });
        }

        // 启动进度监控上报任务
        let (progress_done_tx, mut progress_done_rx) = tokio::sync::mpsc::channel::<()>(1);
        let monitor_config = self.config.clone();
        let monitor_ws = self.ws_client.clone();
        let monitor_socket = self.socket_client.clone();
        let monitor_token = token.clone();
        let monitor_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Some(monitor) = get_global_monitor().await {
                            let mut stats = monitor.get_stats().await;
                            
                            // 兼容旧版 Golang 接口的字段命名 (各语言 Bindings 依赖这两个字段计算进度)
                            if let Some(total_bytes) = stats.get("total_bytes").cloned() {
                                stats.insert("Downloaded".to_string(), total_bytes);
                            }
                            let event = Event {
                                event_type: EventType::Update,
                                name: "Progress Update".to_string(),
                                show_name: "Global".to_string(),
                                id: String::new(),
                            };
                            let _ = send_message(event, stats, &monitor_config, &monitor_ws, &monitor_socket).await;
                        }
                    }
                    _ = progress_done_rx.recv() => {
                        break;
                    }
                    _ = monitor_token.cancelled() => {
                        break;
                    }
                }
            }
        });

        // 等待所有下载任务完成，或者被取消
        while let Some(result) = join_set.join_next().await {
            if let Err(e) = result {
                eprintln!("Task failed: {:?}", e);
            }
            // 如果 token 被取消（暂停/停止），中止剩余任务
            if token.is_cancelled() {
                join_set.abort_all();
                break;
            }
        }

        // 停止进度监控
        let _ = progress_done_tx.send(()).await;
        let _ = monitor_handle.await;

        let end_event = Event {
            event_type: EventType::End,
            name: "End All Downloads".to_string(),
            show_name: "Global".to_string(),
            id: String::new(),
        };

        send_message(end_event, HashMap::new(), &self.config, &self.ws_client, &self.socket_client).await?;

        if let Some(monitor) = get_global_monitor().await {
            monitor.print_stats().await;
        }

        let mut cancel_guard = self.cancel_token.lock().await;
        *cancel_guard = None;
        drop(cancel_guard);

        Ok(())
    }

    pub async fn start_multiple_downloads(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cancel_guard = self.cancel_token.lock().await;
        if cancel_guard.is_some() {
            drop(cancel_guard);
            return Err("downloader already running".into());
        }

        let token = tokio_util::sync::CancellationToken::new();
        *cancel_guard = Some(token.clone());
        drop(cancel_guard);

        let event = Event {
            event_type: EventType::Start,
            name: "Start Batch Download".to_string(),
            show_name: "Global".to_string(),
            id: String::new(),
        };

        send_message(event, HashMap::new(), &self.config, &self.ws_client, &self.socket_client).await?;

        let tasks = {
            let config = self.config.read().await;
            config.tasks.clone()
        };

        let mut join_set = tokio::task::JoinSet::new();

        for (index, task) in tasks.into_iter().enumerate() {
            let token_clone = token.clone();
            let config = self.config.clone();
            let ws_client = self.ws_client.clone();
            let socket_client = self.socket_client.clone();

            join_set.spawn(async move {
                Self::download_task(
                    task,
                    index,
                    token_clone,
                    config,
                    ws_client,
                    socket_client,
                ).await
            });
        }

        // 启动进度监控上报任务
        let (progress_done_tx, mut progress_done_rx) = tokio::sync::mpsc::channel::<()>(1);
        let monitor_config = self.config.clone();
        let monitor_ws = self.ws_client.clone();
        let monitor_socket = self.socket_client.clone();
        let monitor_token = token.clone();
        let monitor_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Some(monitor) = get_global_monitor().await {
                            let mut stats = monitor.get_stats().await;
                            
                            // 兼容旧版 Golang 接口的字段命名
                            if let Some(total_bytes) = stats.get("total_bytes").cloned() {
                                stats.insert("Downloaded".to_string(), total_bytes);
                            }
                            
                            let event = Event {
                                event_type: EventType::Update,
                                name: "Progress Update".to_string(),
                                show_name: "Global".to_string(),
                                id: String::new(),
                            };
                            let _ = send_message(event, stats, &monitor_config, &monitor_ws, &monitor_socket).await;
                        }
                    }
                    _ = progress_done_rx.recv() => {
                        break;
                    }
                    _ = monitor_token.cancelled() => {
                        break;
                    }
                }
            }
        });

        // 等待所有下载任务完成，或者被取消
        while let Some(result) = join_set.join_next().await {
            if let Err(e) = result {
                eprintln!("Task failed: {:?}", e);
            }
            if token.is_cancelled() {
                join_set.abort_all();
                break;
            }
        }

        // 停止进度监控
        let _ = progress_done_tx.send(()).await;
        let _ = monitor_handle.await;

        let end_event = Event {
            event_type: EventType::End,
            name: "End Batch Download".to_string(),
            show_name: "Global".to_string(),
            id: String::new(),
        };

        send_message(end_event, HashMap::new(), &self.config, &self.ws_client, &self.socket_client).await?;

        let mut cancel_guard = self.cancel_token.lock().await;
        *cancel_guard = None;
        drop(cancel_guard);

        Ok(())
    }

    async fn download_task(
        task: DownloadTask,
        index: usize,
        token: tokio_util::sync::CancellationToken,
        config: Arc<RwLock<DownloadConfig>>,
        #[cfg(feature = "websocket")] ws_client: Option<Arc<Mutex<WebSocketClient>>>,
        #[cfg(not(feature = "websocket"))] ws_client: Option<Arc<Mutex<()>>>,
        #[cfg(feature = "socket")] socket_client: Option<Arc<Mutex<SocketClient>>>,
        #[cfg(not(feature = "socket"))] socket_client: Option<Arc<Mutex<()>>>,
    ) {
        let total = {
            let cfg = config.read().await;
            cfg.tasks.len()
        };

        let start_event = Event {
            event_type: EventType::StartOne,
            name: "Start One Download".to_string(),
            show_name: task.show_name.clone(),
            id: task.id.clone(),
        };

        let mut data = HashMap::new();
        data.insert("URL".to_string(), serde_json::Value::String(task.url.clone()));
        data.insert("SavePath".to_string(), serde_json::Value::String(task.save_path.clone()));
        data.insert("ShowName".to_string(), serde_json::Value::String(task.show_name.clone()));
        data.insert("Index".to_string(), serde_json::Value::Number(serde_json::Number::from(index + 1)));
        data.insert("Total".to_string(), serde_json::Value::Number(serde_json::Number::from(total)));

        if let Err(e) = send_message(start_event, data, &config, &ws_client, &socket_client).await {
            eprintln!("Failed to send start event: {:?}", e);
        }

        let err: Option<Box<dyn std::error::Error + Send + Sync>> = {
            let cfg = config.read().await;
            let max_retries = cfg.max_retries;
            let retry_delay_ms = cfg.retry_delay_ms;
            let max_retry_delay_ms = cfg.max_retry_delay_ms;
            drop(cfg);

            let mut current_retry = 0;
            let mut delay = retry_delay_ms;
            let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>>;

            loop {
                let mut downloader = super::get_downloader::get_downloader(config.clone()).await;
                let result = downloader.download(&task).await;

                match result {
                    Ok(()) => {
                        if current_retry > 0 {
                            let retry_event = Event {
                                event_type: EventType::Msg,
                                name: "Retry Success".to_string(),
                                show_name: task.show_name.clone(),
                                id: task.id.clone(),
                            };
                            let mut retry_data = HashMap::new();
                            retry_data.insert("Text".to_string(), serde_json::Value::String(format!("Retry {} succeeded", current_retry)));
                            let _ = send_message(retry_event, retry_data, &config, &ws_client, &socket_client).await;
                        }
                        return;
                    }
                    Err(e) => {
                        last_error = Some(e);
                        current_retry += 1;

                        if current_retry > max_retries {
                            break;
                        }

                        let retry_event = Event {
                            event_type: EventType::Msg,
                            name: "Retry Download".to_string(),
                            show_name: task.show_name.clone(),
                            id: task.id.clone(),
                        };
                        let mut retry_data = HashMap::new();
                        retry_data.insert("Text".to_string(), serde_json::Value::String(format!("Retry {} failed, retrying in {}ms (max {})", current_retry, delay, max_retries)));
                        let _ = send_message(retry_event, retry_data, &config, &ws_client, &socket_client).await;

                        if !token.is_cancelled() {
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                        }

                        delay = (delay * 2).min(max_retry_delay_ms);
                    }
                }

                if token.is_cancelled() {
                    break;
                }
            }

            if let Some(ref e) = last_error {
                eprintln!("Download failed [{}] (retried {} times): {:?}", task.show_name, max_retries, e);
            }
            last_error
        };

        let mut end_data = HashMap::new();
        end_data.insert("URL".to_string(), serde_json::Value::String(task.url));
        end_data.insert("SavePath".to_string(), serde_json::Value::String(task.save_path));
        end_data.insert("ShowName".to_string(), serde_json::Value::String(task.show_name.clone()));
        end_data.insert("Index".to_string(), serde_json::Value::Number(serde_json::Number::from(index + 1)));
        end_data.insert("Total".to_string(), serde_json::Value::Number(serde_json::Number::from(total)));

        if let Some(e) = err {
            if !token.is_cancelled() {
                let error_event = Event {
                    event_type: EventType::Err,
                    name: "Error".to_string(),
                    show_name: task.show_name.clone(),
                    id: task.id.clone(),
                };
                let mut error_data = HashMap::new();
                let error_msg = format!("Download failed: {}", Self::format_error(&e));
                error_data.insert("Error".to_string(), serde_json::Value::String(error_msg));

                let _ = send_message(error_event, error_data, &config, &ws_client, &socket_client).await;
            }
        }

        let end_event = Event {
            event_type: EventType::EndOne,
            name: "End One Download".to_string(),
            show_name: task.show_name,
            id: task.id,
        };

        let _ = send_message(end_event, end_data, &config, &ws_client, &socket_client).await;
    }

    fn format_error(e: &Box<dyn std::error::Error + Send + Sync>) -> String {
        let error_str = e.to_string();
        if error_str.contains("status code: 502") {
            return "Server returned 502 Bad Gateway - server may be overloaded or under maintenance, please retry later".to_string();
        } else if error_str.contains("status code: 503") {
            return "Server returned 503 Service Unavailable - service temporarily unavailable, please retry later".to_string();
        } else if error_str.contains("status code: 504") {
            return "Server returned 504 Gateway Timeout - server response timeout, please check network or retry later".to_string();
        } else if error_str.contains("status code: 404") {
            return "Server returned 404 Not Found - file does not exist or link has expired".to_string();
        } else if error_str.contains("status code: 403") {
            return "Server returned 403 Forbidden - no access permission, may require authentication or proxy".to_string();
        } else if error_str.contains("Connection refused") {
            return "Connection refused - server rejected connection, possibly wrong port or service not started".to_string();
        } else if error_str.contains("Connection reset") {
            return "Connection reset - server unexpectedly closed connection, possibly unstable network or overloaded server".to_string();
        } else if error_str.contains("Timeout") || error_str.contains("timed out") {
            return "Request timeout - network connection timeout, please check network status or retry later".to_string();
        } else if error_str.contains("No route to host") {
            return "No route to host - please check network connection or target address".to_string();
        } else if error_str.contains("StorageFull") || error_str.contains("No space left") {
            return "Insufficient disk space - please free up disk space and retry".to_string();
        } else if error_str.contains("Permission denied") {
            return "Permission denied - cannot write to target path, please check file permissions".to_string();
        } else if error_str.contains("Not Found") || error_str.contains("does not exist") {
            return "File not found - please check if URL is correct".to_string();
        }
        error_str
    }

    pub async fn pause_download(&self) {
        let mut cancel_guard = self.cancel_token.lock().await;
        if let Some(token) = cancel_guard.take() {
            token.cancel();
        }
        drop(cancel_guard);

        let event = Event {
            event_type: EventType::Msg,
            name: "Pause".to_string(),
            show_name: "Global".to_string(),
            id: String::new(),
        };

        let mut data = HashMap::new();
        data.insert("Text".to_string(), serde_json::Value::String("Download paused".to_string()));

        let _ = send_message(event, data, &self.config, &self.ws_client, &self.socket_client).await;
    }

    pub async fn resume_download(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.start_download().await
    }

    pub async fn stop_download(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.pause_download().await;

        // 关闭网络连接
        #[cfg(feature = "websocket")]
        if let Some(ref ws_client) = self.ws_client {
            let client = ws_client.lock().await;
            client.close();
        }

        #[cfg(feature = "socket")]
        if let Some(ref socket_client) = self.socket_client {
            let client = socket_client.lock().await;
            client.close();
        }

        let event = Event {
            event_type: EventType::Msg,
            name: "Stop".to_string(),
            show_name: "Global".to_string(),
            id: String::new(),
        };

        let mut data = HashMap::new();
        data.insert("Text".to_string(), serde_json::Value::String("Download stopped".to_string()));

        send_message(event, data, &self.config, &self.ws_client, &self.socket_client).await?;

        Ok(())
    }

    pub async fn get_snapshot(&self, _task_id: &str) -> Option<HashMap<String, serde_json::Value>> {
        if let Some(monitor) = get_global_monitor().await {
            return Some(monitor.get_stats().await);
        }
        None
    }
}