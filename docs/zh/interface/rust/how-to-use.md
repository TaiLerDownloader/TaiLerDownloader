# Rust 接口使用文档

> [!TIP]
> 本文档介绍的是 **TTHSD Next** 的 Rust 封装 Crate。
> 源码位于 [`bindings/rust/`](https://github.com/TTHSDownloader/TTHSDNext/tree/main/bindings/rust)

---

## 特性

- **安全封装**：所有 `unsafe` FFI 调用封装在内部，对外暴露 safe API
- **异步事件流**：通过 `tokio::sync::mpsc::UnboundedReceiver` 接收下载事件
- **libloading**：运行时动态加载，无需链接时依赖

---

## 架构

```
用户代码 (safe Rust)
    │
    ▼
TTHSDownloader          ← downloader.rs (safe API + mpsc channel)
    │
    ▼
TthsdRaw                ← ffi.rs (unsafe FFI + libloading)
    │
    ▼
tthsd.dll / libtthsd.so ← Rust 编译的动态库
```

**回调路由**：C 回调 `global_c_callback()` 收到事件后，通过全局 `SENDER_MAP` 广播到所有已注册的 `mpsc::UnboundedSender`。

---

## 快速开始

```rust
use tthsd::{TTHSDownloader, DownloadOptions};

#[tokio::main]
async fn main() {
    let dl = TTHSDownloader::load(None)
        .expect("加载动态库失败");

    let (id, mut rx) = dl.start_download(
        vec!["https://example.com/a.zip".into()],
        vec!["/tmp/a.zip".into()],
        DownloadOptions {
            thread_count: Some(32),
            ..Default::default()
        },
    ).expect("启动下载失败");

    println!("下载 ID: {}", id);

    while let Some(evt) = rx.recv().await {
        match evt.event.event_type.as_str() {
            "update" => println!("进度: {:?}", evt.data),
            "end"    => { println!("下载完成"); break; }
            "err"    => { eprintln!("错误: {:?}", evt.data); break; }
            _        => {}
        }
    }

    dl.stop_download(id);
}
```

---

## API 参考

### `TTHSDownloader`

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `load(path)` | `Result<Self>` | 加载动态库（`None` 自动搜索） |
| `start_download(urls, paths, opts)` | `Result<(i32, Receiver)>` | 创建并启动 |
| `get_downloader(urls, paths, opts)` | `Result<(i32, Receiver)>` | 创建不启动 |
| `start_download_by_id(id)` | `bool` | 顺序启动 |
| `start_multiple_downloads_by_id(id)` | `bool` | 并行启动 |
| `pause_download(id)` | `bool` | 暂停 |
| `resume_download(id)` | `bool` | 恢复 |
| `stop_download(id)` | `bool` | 停止并销毁 |

### `DownloadOptions`

```rust
pub struct DownloadOptions {
    pub thread_count: Option<usize>,        // 默认 64
    pub chunk_size_mb: Option<usize>,       // 默认 10
    pub user_agent: Option<String>,
    pub use_callback_url: bool,
    pub remote_callback_url: Option<String>,
    pub use_socket: Option<bool>,
    pub is_multiple: Option<bool>,
}
```

---

## 依赖

```toml
[dependencies]
tthsd = { path = "../bindings/rust" }
tokio = { version = "1", features = ["full"] }
```
