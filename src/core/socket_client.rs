#![cfg(feature = "socket")]

use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use super::downloader::{Event, EventType};

const SOCKET_SEND_QUEUE_SIZE: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessageS {
    #[serde(rename = "Type")]
    pub msg_type: String,
    #[serde(rename = "Msg")]
    pub msg: String,
}

pub struct SocketClient {
    address: String,
    connection: Arc<tokio::sync::Mutex<Option<TcpStream>>>,
    connected: Arc<tokio::sync::Mutex<bool>>,
    send_queue: tokio::sync::broadcast::Sender<Vec<u8>>,
    done: Arc<tokio::sync::Mutex<bool>>,
    close_once: Arc<tokio::sync::Mutex<bool>>,
}

impl SocketClient {
    pub fn new(address: String) -> Self {
        if address.is_empty() {
            return SocketClient {
                address,
                connection: Arc::new(Mutex::new(None)),
                connected: Arc::new(Mutex::new(false)),
                send_queue: tokio::sync::broadcast::channel(SOCKET_SEND_QUEUE_SIZE).0,
                done: Arc::new(Mutex::new(false)),
                close_once: Arc::new(Mutex::new(false)),
            };
        }

        let client = SocketClient {
            address: address.clone(),
            connection: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
            send_queue: tokio::sync::broadcast::channel(SOCKET_SEND_QUEUE_SIZE).0,
            done: Arc::new(Mutex::new(false)),
            close_once: Arc::new(Mutex::new(false)),
        };

        client.connect();
        client.start_write_loop();

        client
    }

    fn connect(&self) {
        if self.address.is_empty() {
            return;
        }

        match TcpStream::connect_timeout(&self.address.parse().unwrap(), Duration::from_secs(10)) {
            Ok(conn) => {
                let mut connection = self.connection.blocking_lock();
                *connection = Some(conn);

                let mut connected = self.connected.blocking_lock();
                *connected = true;
            }
            Err(e) => {
                eprintln!("Socket connection failed: {:?}", e);
            }
        }
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
        connection: &Arc<Mutex<Option<TcpStream>>>,
        connected: &Arc<Mutex<bool>>,
        payload: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conn_guard = connection.lock().await;
        let mut conn = conn_guard.as_ref().ok_or("socket not connected")?.try_clone()?;
        drop(conn_guard);

        let conn_connected = connected.lock().await;
        if !*conn_connected {
            return Err("socket not connected".into());
        }
        drop(conn_connected);

        conn.set_write_timeout(Some(Duration::from_secs(3)))?;
        conn.write_all(&payload)?;

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
                eprintln!("Failed to serialize extra data: {:?}", e);
                return;
            }
        };

        let message = ProgressMessageS {
            msg_type: format!("{:?}", event.event_type),
            msg: data_bytes,
        };

        let json_data = match serde_json::to_string(&message) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to serialize message: {:?}", e);
                return;
            }
        };

        let json_data = format!("{}\n", json_data).into_bytes();

        if event.event_type == EventType::Update {
            let _ = self.send_queue.send(json_data);
            return;
        }

        match self.send_queue.send(json_data) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("Socket send queue blocked, dropping non-progress message");
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
        if let Some(conn) = connection.take() {
            let _ = conn.shutdown(std::net::Shutdown::Both);
        }
        drop(connection);

        let mut connected = self.connected.blocking_lock();
        *connected = false;
    }
}

impl Clone for SocketClient {
    fn clone(&self) -> Self {
        SocketClient {
            address: self.address.clone(),
            connection: self.connection.clone(),
            connected: self.connected.clone(),
            send_queue: self.send_queue.clone(),
            done: self.done.clone(),
            close_once: self.close_once.clone(),
        }
    }
}