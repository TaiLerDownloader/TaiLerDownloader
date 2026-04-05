# set_retry_config 函数

**功能描述**：配置下载重试参数。  
**返回值**：成功返回 0，失败返回 -1。

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | `int` | 下载器实例 ID |
| `maxRetries` | `uint32` | 最大重试次数（默认 3） |
| `retryDelayMs` | `uint64` | 初始重试延迟（毫秒，默认 1000） |
| `maxRetryDelayMs` | `uint64` | 最大重试延迟（毫秒，默认 30000） |

### 重试机制说明

1. **指数退避**：每次重试失败后，延迟时间会翻倍（最多翻倍到 `maxRetryDelayMs`）
2. **自动重试**：下载失败时会自动重试，直到成功或达到最大重试次数
3. **任务级别**：每个下载任务独立进行重试

### 使用示例

**C 语言**:
```c
// 最大重试 5 次，初始延迟 2 秒，最大延迟 60 秒
int result = set_retry_config(downloader_id, 5, 2000, 60000);
```

**Python**:
```python
dl.set_retry_config(
    downloader_id,
    max_retries=5,
    retry_delay_ms=2000,
    max_retry_delay_ms=60000
)
```

### 默认配置

| 参数 | 默认值 |
|------|--------|
| maxRetries | 3 |
| retryDelayMs | 1000 (1秒) |
| maxRetryDelayMs | 30000 (30秒) |

### 重试延迟计算示例

假设配置：`retryDelayMs=1000, maxRetryDelayMs=30000`

| 重试次数 | 延迟时间 |
|----------|----------|
| 第 1 次 | 1000ms |
| 第 2 次 | 2000ms |
| 第 3 次 | 4000ms |
| 第 4 次 | 8000ms |
| 第 5 次 | 16000ms |
| 第 6 次+ | 30000ms (最大) |