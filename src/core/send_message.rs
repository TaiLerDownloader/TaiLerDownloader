use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use super::downloader::{DownloadConfig, Event};
#[cfg(feature = "websocket")]
use super::websocket_client::WebSocketClient;
#[cfg(feature = "socket")]
use super::socket_client::SocketClient;

pub async fn send_message(
    event: Event,
    data: HashMap<String, serde_json::Value>,
    config: &Arc<RwLock<DownloadConfig>>,
    #[cfg(feature = "websocket")] ws_client: &Option<Arc<tokio::sync::Mutex<WebSocketClient>>>,
    #[cfg(not(feature = "websocket"))] ws_client: &Option<Arc<tokio::sync::Mutex<()>>>,
    #[cfg(feature = "socket")] socket_client: &Option<Arc<tokio::sync::Mutex<SocketClient>>>,
    #[cfg(not(feature = "socket"))] socket_client: &Option<Arc<tokio::sync::Mutex<()>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut is_called = false;

    let config_clone = config.clone();
    let ws_client_clone = ws_client.clone();
    let socket_client_clone = socket_client.clone();
    let event_clone = event.clone();

    tokio::spawn(async move {
        let config = config_clone.read().await;

        // 调用回调函数
        if let Some(callback_func) = config.callback_func {
            if let Ok(event_json) = serde_json::to_string(&event_clone) {
                if let Ok(data_json) = serde_json::to_string(&data) {
                    if let (Ok(c_event), Ok(c_data)) = (
                        std::ffi::CString::new(event_json),
                        std::ffi::CString::new(data_json),
                    ) {
                        callback_func(c_event.as_ptr(), c_data.as_ptr());
                        is_called = true;
                    }
                }
            }
        }

        // 发送WebSocket消息
        #[cfg(feature = "websocket")]
        if let Some(ref ws_client) = ws_client_clone {
            if config.callback_url.is_some() {
                let client = ws_client.lock().await;
                client.send_message(event_clone.clone(), data.clone()).await;
                is_called = true;
            }
        }

        // 发送Socket消息
        #[cfg(feature = "socket")]
        if let Some(ref socket_client) = socket_client_clone {
            if config.callback_url.is_some() {
                let client = socket_client.lock().await;
                client.send_message(event_clone.clone(), data.clone()).await;
                is_called = true;
            }
        }

        if !is_called && event_clone.event_type != super::downloader::EventType::Update {
            eprintln!("警告: 没有回调函数 (event {:?}, data {:?})", event_clone.name, data);
        }
    });

    Ok(())
}
