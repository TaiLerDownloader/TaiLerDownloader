# API 参考

本节详细介绍了 TT High Speed Downloader (TTHSD) 的 API 接口、数据结构和回调机制。

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的 API。API 接口与 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 完全兼容，可以直接替换动态库使用。
> 
> TTHSD Golang 已停止开发，建议使用 TTHSD Next。

## 核心概念

### 任务数据结构 (Task Data)

传递给 `start_download` 等函数的任务列表必须是 JSON 格式字符串：

```json
[
  {
    "url": "https://example.com/file.zip",
    "save_path": "/path/to/save/file.zip",
    "show_name": "文件显示名称",
    "id": "unique-task-id-123",
    "headers": {
      "X-Custom-Header": "custom-value"
    }
  }
]
```

#### 任务字段说明

| 字段 | 类型 | 必填 | 说明 |
| :--- | :--- | :--- | :--- |
| `url` | string | 是 | 下载链接 |
| `save_path` | string | 是 | 保存路径 |
| `show_name` | string | 否 | 显示名称（默认为 URL 文件名） |
| `id` | string | 否 | 任务唯一标识（自动生成 UUID） |
| `headers` | object | 否 | 该任务额外的 HTTP 请求头 |

> [!NOTE]
> 任务中的 `headers` 会与下载器的全局 `headers` 合并，任务级别会覆盖全局同名 header。

### 回调函数定义

回调函数用于接收下载过程中的所有事件和状态更新。

**C 语言定义**:
```c
typedef void (*progress_callback_t)(void* event, void* msg);
```

**Python 定义**:
```python
def callback(event_json_str, msg_json_str):
    # event_json_str 和 msg_json_str 是 JSON 字符串
    pass
```

## 事件与消息结构 (Event & Message)

回调函数的两个参数 `event` 和 `msg` 均为 JSON 格式字符串。

### 1. 事件结构 (Event)

`Event` 对象描述了当前发生的事件类型和元数据。

| 字段 | 类型 | 说明 | 注意事项 |
| :--- | :--- | :--- | :--- |
| `Type` | string | 事件类型 | `start`, `startOne`, `update`, `endOne`, `end`, `msg`, `err` |
| `Name` | string | 事件名称 | 仅在部分事件中包含（如 `msg` 类型可能包含 "错误"、"停止" 等） |
| `ShowName` | string | 显示名称 | `update` 和 `start`/`end` 事件中为空字符串 |
| `ID` | string | 任务 ID | `update` 事件中包含对应任务 ID |

**事件类型详解**:

- `start`: 批量任务开始（ID为空）
- `startOne`: 单个文件开始（包含完整 ID 和 ShowName）
- `update`: 进度更新（**包含任务 ID**）
- `endOne`: 单个文件结束（包含完整 ID 和 ShowName）
- `end`: 批量任务结束（ID为空）
- `msg`: 消息通知（包括错误、暂停、恢复、停止等状态）
- `err`: 错误通知

### 2. 消息结构 (Msg)

`Msg` 对象的内容取决于 `Event.Type`，且在 `msg` 类型下对应的 Key 可能不同。

#### `start` / `end`
- **Msg 内容**: 空对象 `{}`

#### `startOne` / `endOne`
- **Msg 内容**:
```json
{
  "URL": "任务URL",
  "SavePath": "保存路径",
  "ShowName": "显示名称",
  "Index": 1,       // 当前任务索引 (从1开始)
  "Total": 5        // 总任务数
}
```

#### `update`
- **Msg 内容**:
```json
{
  "Total": 1048576,      // 文件总字节数
  "Downloaded": 524288   // 已下载字节数
}
```

#### `msg`
`msg` 类型的事件用于传递各种状态信息，其 JSON Key 固定为 "Text"

- **Msg 内容**:
```json
{ "Text": "下载已停止" }
```
```json
{ "Text": "下载已暂停" }
```
```json
{ "Text": "下载已恢复" }
```

#### `err`
`err` 类型的事件用于传递错误信息，其 JSON Key 固定为 "Error"

- **Msg 内容**:
```json
{ "Error": "下载文件失败: ..." }
```

## 通用规则

### 内存管理 (C/C++ 调用者)
- `tasksData` 指向的内存需在调用期间保持有效。
- 回调函数中的 `event` 和 `msg` 指针仅在回调执行期间有效，不要在回调外部保存这些指针。
- `stop_download` 会清理下载器内部所有资源，包括异步任务和网络连接，当前下载器也会被销毁。
- 在 Python 接口封装 端调用 close() 后，应确保不再使用对应的下载器 ID。
- 暂停/恢复功能在并行下载（is_multiple=true）时可能存在限制，建议顺序下载时使用暂停/恢复功能。

### 线程安全
- API 函数是线程安全的。
- 可以同时管理多个不同的 `download` 实例。
- 对同一个 `download ID` 的操作（如 暂停 -> 恢复）应顺序执行。

### 错误处理
- API 函数返回 `-1` 表示失败（如参数错误）。
- 异步过程中的错误（如网络超时）将通过 `err` 事件类型的 `Error` 字段回调通知。

## API 函数列表

以下是所有可用的 API 函数：

| 函数名 | 功能描述 |
|--------|----------|
| [get_downloader](/zh/api/functions/get_downloader) | 创建下载器实例但不启动下载 |
| [start_download](/zh/api/functions/start_download) | 创建下载器实例并立即启动下载 |
| [start_download_ID](/zh/api/functions/start_download_ID) | 启动已创建的下载器（顺序下载） |
| [start_multiple_downloads_id](/zh/api/functions/start_multiple_downloads_id) | 启动已创建的下载器（并行下载） |
| [pause_download](/zh/api/functions/pause_download) | 暂停下载 |
| [resume_download](/zh/api/functions/resume_download) | 恢复下载 |
| [stop_download](/zh/api/functions/stop_download) | 停止下载并清理资源 |
| [set_speed_limit](/zh/api/functions/set_speed_limit) | 设置下载速度限制 |
| [set_proxy](/zh/api/functions/set_proxy) | 设置代理服务器 |
| [set_retry_config](/zh/api/functions/set_retry_config) | 配置重试参数 |
| [get_performance_stats](/zh/api/functions/get_performance_stats) | 获取性能统计信息 |

## TTHSD Next 的性能优势

相比 TTHSD Golang 版本，TTHSD Next 具有以下性能优势：

| 特性 | TTHSD Next (Rust) | TTHSD Golang |
|------|-------------------|--------------|
| 下载速度 | 更快 | 较快 |
| 内存占用 | 更低 | 较高 |
| 并发数 | 可达数十万 | 有限制 |
| 内存安全 | Rust 所有权机制 | 依赖 GC |
| GC 暂停 | 无 | 有 |
| Android 支持 | 支持 | 不支持 |

## 版本说明

### TTHSD Next (当前版本)
- 基于 Rust 语言开发
- 高性能、低内存占用
- 支持 Windows、Linux、macOS、Android 平台
- 持续维护和更新

### TTHSD Golang (已停止开发)
- 基于 Go 语言开发
- 已停止维护
- API 接口与 TTHSD Next 完全兼容
- 建议迁移到 TTHSD Next

---

如需了解各个函数的详细信息，请点击上表中的函数名称查看对应文档。