# resume_download 函数

| 项目 | 说明 |
|------|------|
| **描述** | 恢复已暂停的下载，无法恢复已停止的下载。 |
| **参数** | `id` - 下载器实例ID |
| **返回值** | 0（成功）或 -1（下载器不存在） |

## 版本差异 ( Golang )

### 核心版本 0.5.0
- **无法恢复**下载
- 因为 `pauseDownload` 会立即从 map 中删除下载器
- `resumeDownload` 会找不到下载器而返回 -1

### 核心版本 0.5.1（未发布）
- **可以恢复**已暂停的下载
- 因为 `pauseDownload` 保留了下载器在映射表中
- `resumeDownload` 内部调用 `ResumeDownload()` 方法：
  1. 等待所有旧任务退出（`activeTasks.Wait()`）
  2. 重新创建 context 并调用 `StartDownload()` 重新开始下载
- **注意**：无法恢复已停止（`stopDownload`）的下载，因为下载器已被销毁
