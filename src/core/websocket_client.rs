#![cfg(feature = "websocket")]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures::sink::SinkExt;
use serde::{Deserialize, Serialize};
use super::downloader::{Event, EventType};

const WS_SEND_QUEUE_SIZE: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessageWs {
    #[serde(rename = "Type")]
    pub msg_type: String,
    #[serde(rename = "Msg")]
    pub msg: String,
}

pub struct WebSocketClient {
    url: String,
    connection: Arc<tokio::sync::Mutex<Option<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>>,
    connected: Arc<tokio::sync::Mutex<bool>>,
    send_queue: tokio::sync::broadcast::Sender<Vec<u8>>,
    done: Arc<tokio::sync::Mutex<bool>>,
    close_once: Arc<tokio::sync::Mutex<bool>>,
}

impl WebSocketClient {
    pub fn new(url: String) -> Self {
        if url.is_empty() {
            return WebSocketClient {
                url,
                connection: Arc::new(Mutex::new(None)),
                connected: Arc::new(Mutex::new(false)),
                send_queue: tokio::sync::broadcast::channel(WS_SEND_QUEUE_SIZE).0,
                done: Arc::new(Mutex::new(false)),
                close_once: Arc::new(Mutex::new(false)),
            };
        }

        let client = WebSocketClient {
            url: url.clone(),
            connection: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
            send_queue: tokio::sync::broadcast::channel(WS_SEND_QUEUE_SIZE).0,
            done: Arc::new(Mutex::new(false)),
            close_once: Arc::new(Mutex::new(false)),
        };

        client.connect();
        client.start_write_loop();

        client
    }

    fn connect(&self) {
        if self.url.is_empty() {
            return;
        }

        let ws_url = Self::normalize_websocket_url(&self.url);
        if ws_url.is_empty() {
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            connect_async(&ws_url).await
        });

        match result {
            Ok((ws_stream, _)) => {
                let mut connection = self.connection.blocking_lock();
                *connection = Some(ws_stream);

                let mut connected = self.connected.blocking_lock();
                *connected = true;
            }
            Err(e) => {
                eprintln!("WebSocket连接失败: {:?}", e);
            }
        }
    }

    fn normalize_websocket_url(raw: &str) -> String {
        let mut ws_url = raw.trim().to_string();
        if ws_url.is_empty() {
            return String::new();
        }

        if ws_url.starts_with("http://") {
            ws_url = format!("ws://{}", &ws_url[7..]);
        } else if ws_url.starts_with("https://") {
            ws_url = format!("wss://{}", &ws_url[8..]);
        }

        if !ws_url.ends_with('/') {
            ws_url.push('/');
        }

        format!("{}websocket", ws_url)
    }

    fn start_write_loop(&self) {
        let send_queue = self.send_queue.clone();
        let connection = self.connection.clone();
        let connected = self.connected.clone();
        let done = self.done.clone();

        tokio::spawn(async move {
            let mut receiver = send_queue.subscribe();

            loop {
                tokio::select! {
                    _ = async {
                        let d = done.lock().await;
                        *d
                    } => {
                        break;
                    }
                    result = receiver.recv() => {
                        match result {
                            Ok(payload) => {
                                if let Err(e) = Self::write_raw(&connection, &connected, payload).await {
                                    eprintln!("Write failed: {:?}", e);
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });
    }

    async fn write_raw(
        connection: &Arc<Mutex<Option<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>>,
        connected: &Arc<Mutex<bool>>,
        payload: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn_guard = connection.lock().await;
        let conn = conn_guard.as_mut().ok_or("websocket not connected")?;

        let conn_connected = connected.lock().await;
        if !*conn_connected {
            return Err("websocket not connected".into());
        }
        drop(conn_connected);

        let message = Message::Text(String::from_utf8(payload)?.into());
        conn.send(message).await?;

        Ok(())
    }

    pub async fn send_message(&self, event: Event, data: HashMap<String, serde_json::Value>) {
        let done = self.done.lock().await;
        if *done {
            return;
        }
        drop(done);

        let connected = self.connected.lock().await;
        if !*connected {
            return;
        }
        drop(connected);

        let data_bytes = match serde_json::to_string(&data) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("序列化额外数据失�? {:?}", e);
                return;
            }
        };

        let message = ProgressMessageWs {
            msg_type: format!("{:?}", event.event_type),
            msg: data_bytes,
        };

        let json_data = match serde_json::to_string(&message) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("序列化消息失�? {:?}", e);
                return;
            }
        };

        let json_data = json_data.into_bytes();

        if event.event_type == EventType::Update {
            let _ = self.send_queue.send(json_data);
            return;
        }

        match self.send_queue.send(json_data) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("WebSocket发送队列阻塞，丢弃非进度消息");
            }
        }
    }

    pub fn close(&self) {
        let mut close_once = self.close_once.blocking_lock();
        if *close_once {
            return;
        }
        *close_once = true;
        drop(close_once);

        let mut done = self.done.blocking_lock();
        *done = true;
        drop(done);

        let mut connection = self.connection.blocking_lock();
        if let Some(mut conn) = connection.take() {
            let _ = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(conn.close(None))
            });
        }
        drop(connection);

        let mut connected = self.connected.blocking_lock();
        *connected = false;
    }
}

impl Clone for WebSocketClient {
    fn clone(&self) -> Self {
        WebSocketClient {
            url: self.url.clone(),
            connection: self.connection.clone(),
            connected: self.connected.clone(),
            send_queue: self.send_queue.clone(),
            done: self.done.clone(),
            close_once: self.close_once.clone(),
        }
    }
}
