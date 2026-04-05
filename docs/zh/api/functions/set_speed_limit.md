# set_speed_limit 函数

**功能描述**：设置下载速度限制。  
**返回值**：成功返回 0，失败返回 -1。

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | `int` | 下载器实例 ID |
| `speedLimitBps` | `uint64` | 速度限制（bytes/s），0 表示不限速 |

### 使用示例

**C 语言**:
```c
int result = set_speed_limit(downloader_id, 1024 * 1024); // 限制为 1MB/s
```

**Python**:
```python
dl.set_speed_limit(downloader_id, 1024 * 1024)  # 限制为 1MB/s
```

### 说明

- 设置速度限制后，所有下载任务的下载速度将被限制在指定值
- 设置为 0 表示不限制速度
- 该设置仅影响当前下载任务，不会影响后续创建的下载器