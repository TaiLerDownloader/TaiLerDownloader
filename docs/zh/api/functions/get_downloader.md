# get_downloader 函数

**功能描述**：创建下载器实例但不启动下载。  
**返回值**：成功返回下载器实例ID（正整数），失败返回 -1。

| 参数 | 类型 | 说明 |
|------|------|------|
| `tasksData` | `char*` | JSON格式的任务数据（必需） |
| `taskCount` | `int` | 任务数量（必需） |
| `threadCount` | `int` | 下载线程数（必需） |
| `chunkSizeMB` | `int` | 分块大小（MB）（必需） |
| `callback` | `progress_callback_t` | 进度回调函数（可选） |
| `useCallbackURL` | `bool` | 是否启用远程回调URL（必需） |
| `userAgent` | `char*` | User-Agent（可选，NULL 使用默认值①） |
| `remoteCallbackUrl` | `char*` | 远程回调URL（可选） |
| `useSocket` | `bool*` | 是否启用Socket通信（可选） |
| `headersJson` | `char*` | 全局 Headers（可选，NULL 不启用） |

① 默认 User-Agent：`Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36`

### headersJson 参数说明

`headersJson` 参数用于设置全局 HTTP 请求头，会应用到所有下载任务。格式为 JSON 对象：

```json
{
  "Authorization": "Bearer token123",
  "X-Custom-Header": "custom-value"
}
```

任务级别的 `headers` 字段会与全局 `headers` 合并，任务级别会覆盖全局同名 header。
