# stop_download 函数

| 项目 | 说明 |
|------|------|
| **描述** | 停止下载并清理所有资源，会等待任务退出后释放连接，且销毁当前下载器。 |
| **参数** | `id` - 下载器实例ID |
| **返回值** | 0（成功）或 -1（下载器不存在） |

## 版本差异 ( Golang )

### 核心版本 0.5.0
- 调用 `stopDownload` 后**立即从下载器映射表中删除**该下载器
- 删除后才调用 `StopDownload()` 方法清理资源
- `StopDownload()` 方法内部调用 `PauseDownload()` 后直接清理 WebSocket/Socket 连接

### 核心版本 0.5.1（未发布）
- 调用 `stopDownload` 后**立即从下载器映射表中删除**该下载器
- 然后调用 `StopDownload()` 方法
- `StopDownload()` 方法内部：
  1. 调用 `PauseDownload()` 取消 context
  2. 等待所有任务退出（`activeTasks.Wait()`）
  3. 关闭网络连接（WebSocket/Socket）
  4. 发送停止消息
- **重要**：停止后**无法恢复**下载，因为下载器已被销毁

> **注意**：如果您使用的是 TTHSD Core 0.5.0 版本，`stopDownload` 不会等待任务退出，而是立即清理资源。这可能导致部分任务未完全结束。
