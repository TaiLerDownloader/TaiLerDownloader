# Golang 接口使用文档

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的 Go 语言绑定。
> 源码位于 [`bindings/golang/`](https://github.com/TTHSDownloader/TTHSDNext/tree/main/bindings/golang)

---

## 特性

- **CGo + dlopen**：运行时动态加载，无需链接时依赖
- **Go channel**：通过 `chan DownloadEventMsg` 接收事件，与 `goroutine` / `select` 完美配合
- **全局回调路由**：C 回调通过 `sync.RWMutex` 保护的全局 map 路由到对应 channel
- **跨平台**：自动选择 `dlopen` (Linux/macOS) 或 `LoadLibrary` (Windows)

---

## 架构

```
用户代码 (Go)
    │
    ▼
TTHSDownloader          ← tthsd.go (高层 API + channel 事件流)
    │
    ▼
nativeLib               ← native.go (CGo + dlopen/dlsym)
    │
    ▼
tthsd.dll / libtthsd.so ← Rust 编译的动态库
```

回调路由：C 回调 → `goCallbackBridge` (CGo 导出) → 全局 `callbackChans` map → 对应 channel。

---

## 安装

```bash
go get github.com/TTHSDownloader/TTHSDNext/bindings/golang
```

将 `tthsd.dll` / `libtthsd.so` / `libtthsd.dylib` 放到可执行文件同级目录或系统库路径中。

---

## 快速开始

```go
package main

import (
    "fmt"
    tthsd "github.com/TTHSDownloader/TTHSDNext/bindings/golang"
)

func main() {
    dl, err := tthsd.Load("")  // 自动搜索
    if err != nil {
        panic(err)
    }
    defer dl.Close()

    id, events, err := dl.StartDownload(
        []string{"https://example.com/file.zip"},
        []string{"./file.zip"},
        tthsd.DownloadOptions{ThreadCount: 32},
    )
    if err != nil {
        panic(err)
    }

    fmt.Printf("下载 ID: %d\n", id)

    for evt := range events {
        switch evt.Event.Type {
        case tthsd.EventUpdate:
            downloaded, _ := evt.Data["Downloaded"].(float64)
            total, _ := evt.Data["Total"].(float64)
            fmt.Printf("\r进度: %.1f%%", downloaded/total*100)
        case tthsd.EventEnd:
            fmt.Println("\n下载完成")
            dl.StopDownload(id)
            return
        case tthsd.EventErr:
            fmt.Printf("\n错误: %v\n", evt.Data["Error"])
            dl.StopDownload(id)
            return
        }
    }
}
```

---

## API 参考

### `TTHSDownloader`

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `Load(path)` | `(*TTHSDownloader, error)` | 加载动态库（空字符串自动搜索） |
| `Close()` | — | 释放资源 |
| `StartDownload(urls, paths, opts)` | `(int, <-chan DownloadEventMsg, error)` | 创建并启动 |
| `GetDownloader(urls, paths, opts)` | `(int, <-chan DownloadEventMsg, error)` | 创建不启动 |
| `StartDownloadByID(id)` | `bool` | 顺序启动 |
| `StartMultipleDownloadsByID(id)` | `bool` | 并行启动 |
| `PauseDownload(id)` | `bool` | 暂停 |
| `ResumeDownload(id)` | `bool` | 恢复 |
| `StopDownload(id)` | `bool` | 停止（同时关闭 channel） |

### `DownloadOptions`

```go
type DownloadOptions struct {
    ThreadCount       int     // 默认 64
    ChunkSizeMB       int     // 默认 10
    UserAgent         *string
    UseCallbackURL    bool
    RemoteCallbackURL *string
    UseSocket         *bool
    IsMultiple        *bool   // true=并行, false=顺序
}
```

### 事件常量

| 常量 | 值 | 说明 |
|------|----|------|
| `EventStart` | `"start"` | 下载会话开始 |
| `EventStartOne` | `"startOne"` | 单个任务开始 |
| `EventUpdate` | `"update"` | 进度更新 |
| `EventEnd` | `"end"` | 全部完成 |
| `EventEndOne` | `"endOne"` | 单个完成 |
| `EventMsg` | `"msg"` | 消息通知 |
| `EventErr` | `"err"` | 错误 |

---

## 注意事项

- **channel 缓冲区**：内部 channel 缓冲 1024 条消息，如果消费太慢可能丢弃事件
- **StopDownload**：会关闭对应的 channel，之后 `range events` 自动退出
- **CGo 开销**：每次回调有 CGo 调用开销（~100ns），对于高频 update 事件可忽略
