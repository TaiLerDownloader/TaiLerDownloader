use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use super::downloader::{HSDownloader, DownloadTask, DownloadConfig, Event, EventType, UA};
use super::send_message::send_message;

lazy_static::lazy_static! {
    static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
}

fn get_downloaders() -> &'static Mutex<HashMap<i32, Arc<RwLock<HSDownloader>>>> {
    static DOWNLOADERS: once_cell::sync::Lazy<Mutex<HashMap<i32, Arc<RwLock<HSDownloader>>>>> =
        once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
    &DOWNLOADERS
}

fn get_downloader_id() -> &'static Mutex<i32> {
    static DOWNLOADER_ID: once_cell::sync::Lazy<Mutex<i32>> =
        once_cell::sync::Lazy::new(|| Mutex::new(0));
    &DOWNLOADER_ID
}

#[unsafe(no_mangle)]
pub extern "C" fn start_download(
    tasks_data: *const i8,
    task_count: i32,
    thread_count: i32,
    chunk_size_mb: i32,
    callback: usize,
    use_callback_url: bool,
    _user_agent: *const i8,
    remote_callback_url: *const i8,
    use_socket: *const bool,
    is_multiple: *const bool,
    headers_json: *const i8,
) -> i32 {
    if tasks_data.is_null() || task_count <= 0 {
        eprintln!("无效参数: tasks_data={:?}, task_count={}", tasks_data, task_count);
        return -1;
    }

    let tasks_str = unsafe { std::ffi::CStr::from_ptr(tasks_data as *const u8 as *const std::ffi::c_char) };
    let tasks_json = match tasks_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("转换任务数据失败: {:?}", e);
            return -1;
        }
    };

    let tasks: Vec<DownloadTask> = match serde_json::from_str(tasks_json) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("解析任务数据失败: {:?}", e);
            return -1;
        }
    };

    let callback_url = if !remote_callback_url.is_null() {
        let url_str = unsafe { std::ffi::CStr::from_ptr(remote_callback_url as *const u8 as *const std::ffi::c_char) };
        match url_str.to_str() {
            Ok(s) if !s.is_empty() => Some(s.to_string()),
            _ => None,
        }
    } else {
        None
    };

    let use_socket_val = if !use_socket.is_null() {
        Some(unsafe { *use_socket })
    } else {
        None
    };

    let is_multiple_val = if !is_multiple.is_null() {
        unsafe { *is_multiple }
    } else {
        false
    };

    let callback_func = if callback != 0 {
        unsafe {
            Some(std::mem::transmute::<usize, super::downloader::ProgressCallback>(callback))
        }
    } else {
        None
    };

    let global_headers = if !headers_json.is_null() {
        let headers_str = unsafe { std::ffi::CStr::from_ptr(headers_json as *const u8 as *const std::ffi::c_char) };
        match headers_str.to_str() {
            Ok(s) if !s.is_empty() => {
                serde_json::from_str::<std::collections::HashMap<String, String>>(s).unwrap_or_default()
            }
            _ => std::collections::HashMap::new(),
        }
    } else {
        std::collections::HashMap::new()
    };

    let config = DownloadConfig {
        tasks,
        thread_count: thread_count as usize,
        chunk_size_mb: chunk_size_mb as usize,
        callback_func,
        use_callback_url,
        callback_url,
        use_socket: use_socket_val,
        show_name: String::new(),
        user_agent: UA.to_string(),
        max_retries: 3,
        retry_delay_ms: 1000,
        max_retry_delay_ms: 30000,
        speed_limit_bps: 0,
        proxy_url: None,
        headers: global_headers,
    };

    let downloader = Arc::new(RwLock::new(HSDownloader::new(config)));

    let downloader_id = {
        let mut id = get_downloader_id().lock().unwrap();
        *id += 1;
        *id
    };

    {
        let mut downloaders = get_downloaders().lock().unwrap();
        downloaders.insert(downloader_id, downloader.clone());
    }

    let downloader_clone = downloader.clone();
    RUNTIME.spawn(async move {
        let result = if is_multiple_val {
            downloader_clone.read().await.start_multiple_downloads().await
        } else {
            downloader_clone.read().await.start_download().await
        };

        if let Err(e) = result {
            let event = Event {
                event_type: EventType::Err,
                name: "错误".to_string(),
                show_name: String::new(),
                id: String::new(),
            };

            let mut data = HashMap::new();
            data.insert("Error".to_string(), serde_json::Value::String(e.to_string()));

            let config = downloader_clone.read().await.config.clone();
            let ws_client = downloader_clone.read().await.ws_client.clone();
            let socket_client = downloader_clone.read().await.socket_client.clone();

            let _ = send_message(event, data, &config, &ws_client, &socket_client).await;
        }

        let mut downloaders = get_downloaders().lock().unwrap();
        downloaders.remove(&downloader_id);
    });

    downloader_id
}

#[unsafe(no_mangle)]
pub extern "C" fn get_downloader(
    tasks_data: *const i8,
    task_count: i32,
    thread_count: i32,
    chunk_size_mb: i32,
    callback: usize,
    use_callback_url: bool,
    _user_agent: *const i8,
    remote_callback_url: *const i8,
    use_socket: *const bool,
    headers_json: *const i8,
) -> i32 {
    if tasks_data.is_null() || task_count <= 0 {
        return -1;
    }

    let tasks_str = unsafe { std::ffi::CStr::from_ptr(tasks_data as *const u8 as *const std::ffi::c_char) };
    let tasks_json = match tasks_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let tasks: Vec<DownloadTask> = match serde_json::from_str(tasks_json) {
        Ok(t) => t,
        Err(_) => return -1,
    };

    let callback_url = if !remote_callback_url.is_null() {
        let url_str = unsafe { std::ffi::CStr::from_ptr(remote_callback_url as *const u8 as *const std::ffi::c_char) };
        match url_str.to_str() {
            Ok(s) if !s.is_empty() => Some(s.to_string()),
            _ => None,
        }
    } else {
        None
    };

    let use_socket_val = if !use_socket.is_null() {
        Some(unsafe { *use_socket })
    } else {
        None
    };

    let callback_func = if callback != 0 {
        unsafe {
            Some(std::mem::transmute::<usize, super::downloader::ProgressCallback>(callback))
        }
    } else {
        None
    };

    let global_headers = if !headers_json.is_null() {
        let headers_str = unsafe { std::ffi::CStr::from_ptr(headers_json as *const u8 as *const std::ffi::c_char) };
        match headers_str.to_str() {
            Ok(s) if !s.is_empty() => {
                serde_json::from_str::<std::collections::HashMap<String, String>>(s).unwrap_or_default()
            }
            _ => std::collections::HashMap::new(),
        }
    } else {
        std::collections::HashMap::new()
    };

    let config = DownloadConfig {
        tasks,
        thread_count: thread_count as usize,
        chunk_size_mb: chunk_size_mb as usize,
        callback_func,
        use_callback_url,
        callback_url,
        use_socket: use_socket_val,
        show_name: String::new(),
        user_agent: UA.to_string(),
        max_retries: 3,
        retry_delay_ms: 1000,
        max_retry_delay_ms: 30000,
        speed_limit_bps: 0,
        proxy_url: None,
        headers: global_headers,
    };

    let downloader = Arc::new(RwLock::new(HSDownloader::new(config)));

    let downloader_id = {
        let mut id = get_downloader_id().lock().unwrap();
        *id += 1;
        *id
    };

    {
        let mut downloaders = get_downloaders().lock().unwrap();
        downloaders.insert(downloader_id, downloader);
    }

    downloader_id
}

#[unsafe(no_mangle)]
pub extern "C" fn start_download_id(id: i32) -> i32 {
    let downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.get(&id).cloned();
    drop(downloaders);

    match downloader {
        Some(d) => {
            let d_clone = d.clone();
            RUNTIME.spawn(async move {
                let result = d_clone.read().await.start_download().await;

                if let Err(e) = result {
                    let event = Event {
                        event_type: EventType::Err,
                        name: "错误".to_string(),
                        show_name: String::new(),
                        id: String::new(),
                    };

                    let mut data = HashMap::new();
                    data.insert("Error".to_string(), serde_json::Value::String(e.to_string()));

                    let config = d_clone.read().await.config.clone();
                    let ws_client = d_clone.read().await.ws_client.clone();
                    let socket_client = d_clone.read().await.socket_client.clone();

                    let _ = send_message(event, data, &config, &ws_client, &socket_client).await;
                }

                let mut downloaders = get_downloaders().lock().unwrap();
                downloaders.remove(&id);
            });
            0
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start_multiple_downloads_id(id: i32) -> i32 {
    let downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.get(&id).cloned();
    drop(downloaders);

    match downloader {
        Some(d) => {
            let d_clone = d.clone();
            RUNTIME.spawn(async move {
                let result = d_clone.read().await.start_multiple_downloads().await;

                if let Err(e) = result {
                    let event = Event {
                        event_type: EventType::Err,
                        name: "错误".to_string(),
                        show_name: String::new(),
                        id: String::new(),
                    };

                    let mut data = HashMap::new();
                    data.insert("Error".to_string(), serde_json::Value::String(e.to_string()));

                    let config = d_clone.read().await.config.clone();
                    let ws_client = d_clone.read().await.ws_client.clone();
                    let socket_client = d_clone.read().await.socket_client.clone();

                    let _ = send_message(event, data, &config, &ws_client, &socket_client).await;
                }

                let mut downloaders = get_downloaders().lock().unwrap();
                downloaders.remove(&id);
            });
            0
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pause_download(id: i32) -> i32 {
    let downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.get(&id).cloned();
    drop(downloaders);

    match downloader {
        Some(d) => {
            RUNTIME.block_on(async {
                d.read().await.pause_download().await;
            });
            0
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn resume_download(id: i32) -> i32 {
    let downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.get(&id).cloned();
    drop(downloaders);

    match downloader {
        Some(d) => {
            let result = RUNTIME.block_on(async {
                d.read().await.resume_download().await
            });
            match result {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stop_download(id: i32) -> i32 {
    let mut downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.remove(&id);
    drop(downloaders);

    match downloader {
        Some(d) => {
            let result = RUNTIME.block_on(async {
                d.read().await.stop_download().await
            });
            match result {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_speed_limit(id: i32, speed_limit_bps: u64) -> i32 {
    let downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.get(&id).cloned();
    drop(downloaders);

    match downloader {
        Some(d) => {
            RUNTIME.block_on(async {
                let cfg = d.write().await;
                cfg.config.write().await.speed_limit_bps = speed_limit_bps;
            });
            0
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_proxy(id: i32, proxy_url: *const i8) -> i32 {
    let downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.get(&id).cloned();
    drop(downloaders);

    match downloader {
        Some(d) => {
            let proxy = if !proxy_url.is_null() {
                let url_str = unsafe { std::ffi::CStr::from_ptr(proxy_url as *const u8 as *const std::ffi::c_char) };
                match url_str.to_str() {
                    Ok(s) if !s.is_empty() => Some(s.to_string()),
                    _ => None,
                }
            } else {
                None
            };

            RUNTIME.block_on(async {
                let cfg = d.write().await;
                cfg.config.write().await.proxy_url = proxy;
            });
            0
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_retry_config(id: i32, max_retries: u32, retry_delay_ms: u64, max_retry_delay_ms: u64) -> i32 {
    let downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.get(&id).cloned();
    drop(downloaders);

    match downloader {
        Some(d) => {
            RUNTIME.block_on(async {
                let cfg = d.write().await;
                let mut config = cfg.config.write().await;
                config.max_retries = max_retries as usize;
                config.retry_delay_ms = retry_delay_ms;
                config.max_retry_delay_ms = max_retry_delay_ms;
            });
            0
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_performance_stats(id: i32) -> *mut std::ffi::c_char {
    let downloaders = get_downloaders().lock().unwrap();
    let downloader = downloaders.get(&id).cloned();
    drop(downloaders);

    match downloader {
        Some(d) => {
            let stats = RUNTIME.block_on(async {
                d.read().await.get_snapshot("").await
            });

            match stats {
                Some(s) => {
                    let json = serde_json::to_string(&s).unwrap_or_default();
                    let c_string = std::ffi::CString::new(json).unwrap_or_default();
                    c_string.into_raw() as *mut std::ffi::c_char
                }
                None => std::ffi::CString::new("{}").unwrap().into_raw() as *mut std::ffi::c_char,
            }
        }
        None => std::ffi::CString::new("{}").unwrap().into_raw() as *mut std::ffi::c_char,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_string(s: *mut std::ffi::c_char) {
    if !s.is_null() {
        unsafe { drop(std::ffi::CString::from_raw(s)) };
    }
}
