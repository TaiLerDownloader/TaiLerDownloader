// Android 平台的 JNI 导出函数
// 只有在编译 Android 版本时才会包含此模块

#[cfg(feature = "android")]
use jni::EnvUnowned;
#[cfg(feature = "android")]
use jni::objects::JClass;
#[cfg(feature = "android")]
use jni::sys::{jint, jboolean};
#[cfg(feature = "android")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "android")]
use std::collections::HashMap;
#[cfg(feature = "android")]
use tokio::sync::RwLock;

#[cfg(feature = "android")]
use super::downloader::{HSDownloader, DownloadTask, DownloadConfig, Event, EventType, UA};
#[cfg(feature = "android")]
use super::send_message::send_message;

#[cfg(feature = "android")]
lazy_static::lazy_static! {
    static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
}

#[cfg(feature = "android")]
fn get_downloaders() -> &'static Mutex<HashMap<i32, Arc<RwLock<HSDownloader>>>> {
    static DOWNLOADERS: once_cell::sync::Lazy<Mutex<HashMap<i32, Arc<RwLock<HSDownloader>>>>> =
        once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
    &DOWNLOADERS
}

#[cfg(feature = "android")]
fn get_downloader_id() -> &'static Mutex<i32> {
    static DOWNLOADER_ID: once_cell::sync::Lazy<Mutex<i32>> =
        once_cell::sync::Lazy::new(|| Mutex::new(0));
    &DOWNLOADER_ID
}

/// 从 JString 获取 String (jni 0.22 新 API)
#[cfg(feature = "android")]
fn jstring_to_string(env: EnvUnowned<'_>, jstr: &jni::objects::JString<'_>) -> Option<String> {
    env.with_env(|e| jstr.to_string(e)).ok().flatten()
}

/// JNI 函数: 启动下载任务
/// 
/// 参数说明:
/// - env: JNI 环境
/// - _class: Java 类对象
/// - tasks_json: 下载任务的 JSON 字符串
/// - thread_count: 下载线程数
/// - chunk_size_mb: 分块大小 (MB)
/// - use_callback_url: 是否使用回调 URL
/// - callback_url: 回调 URL 地址
/// - use_socket: 是否使用 Socket
/// - is_multiple: 是否为多任务下载
///
/// 返回值: 下载器 ID，失败返回 -1
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_tthsd_TTHSDLibrary_startDownload<'local>(
    env: EnvUnowned<'local>,
    _class: JClass,
    tasks_json: jni::objects::JString<'local>,
    thread_count: jint,
    chunk_size_mb: jint,
    use_callback_url: jboolean,
    callback_url: jni::objects::JString<'local>,
    use_socket: jboolean,
    is_multiple: jboolean,
) -> jint {
    // 转换 JSON 字符串
    let tasks_str: String = match jstring_to_string(env, &tasks_json) {
        Some(s) => s,
        None => return -1,
    };

    let tasks_json_str = tasks_str;

    // 解析任务
    let tasks: Vec<DownloadTask> = match serde_json::from_str(&tasks_json_str) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("解析任务数据失败: {:?}", e);
            return -1;
        }
    };

    // 获取回调 URL
    let cb_url = if use_callback_url != jni::sys::JNI_FALSE {
        match jstring_to_string(env, &callback_url) {
            Some(url) if !url.is_empty() => Some(url),
            _ => return -1,
        }
    } else {
        None
    };

    let config = DownloadConfig {
        tasks,
        thread_count: thread_count as usize,
        chunk_size_mb: chunk_size_mb as usize,
        callback_func: None,
        use_callback_url: use_callback_url != jni::sys::JNI_FALSE,
        callback_url: cb_url,
        use_socket: if use_socket != jni::sys::JNI_FALSE { Some(true) } else { None },
        show_name: String::new(),
        user_agent: UA.to_string(),
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
        let result = if is_multiple != jni::sys::JNI_FALSE {
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

/// JNI 函数: 创建下载器但不立即启动
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_tthsd_TTHSDLibrary_getDownloader<'local>(
    env: EnvUnowned<'local>,
    _class: JClass,
    tasks_json: jni::objects::JString<'local>,
    thread_count: jint,
    chunk_size_mb: jint,
    use_callback_url: jboolean,
    callback_url: jni::objects::JString<'local>,
    use_socket: jboolean,
) -> jint {
    let tasks_str: String = match jstring_to_string(env, &tasks_json) {
        Some(s) => s,
        None => return -1,
    };

    let tasks_json_str = tasks_str;

    let tasks: Vec<DownloadTask> = match serde_json::from_str(&tasks_json_str) {
        Ok(t) => t,
        Err(_) => return -1,
    };

    let cb_url = if use_callback_url != jni::sys::JNI_FALSE {
        match jstring_to_string(env, &callback_url) {
            Some(url) if !url.is_empty() => Some(url),
            _ => return -1,
        }
    } else {
        None
    };

    let config = DownloadConfig {
        tasks,
        thread_count: thread_count as usize,
        chunk_size_mb: chunk_size_mb as usize,
        callback_func: None,
        use_callback_url: use_callback_url != jni::sys::JNI_FALSE,
        callback_url: cb_url,
        use_socket: if use_socket != jni::sys::JNI_FALSE { Some(true) } else { None },
        show_name: String::new(),
        user_agent: UA.to_string(),
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

/// JNI 函数: 启动指定 ID 的下载
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_tthsd_TTHSDLibrary_startDownloadById<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass,
    id: jint,
) -> jint {
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

/// JNI 函数: 启动指定 ID 的多任务下载
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_tthsd_TTHSDLibrary_startMultipleDownloadsById<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass,
    id: jint,
) -> jint {
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

/// JNI 函数: 暂停下载
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_tthsd_TTHSDLibrary_pauseDownload<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass,
    id: jint,
) -> jint {
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

/// JNI 函数: 恢复下载
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_tthsd_TTHSDLibrary_resumeDownload<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass,
    id: jint,
) -> jint {
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

/// JNI 函数: 停止下载
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_tthsd_TTHSDLibrary_stopDownload<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass,
    id: jint,
) -> jint {
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