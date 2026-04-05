# get_performance_stats 函数

**功能描述**：获取下载性能统计信息。  
**返回值**：返回 JSON 格式的性能统计数据字符串（需使用 `free_string` 释放内存），失败返回 `"{}"`。

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | `int` | 下载器实例 ID |

### 返回数据格式

```json
{
  "total_bytes": 104857600,
  "Total": 104857600,
  "current_speed_bps": 5242880,
  "current_speed_mbps": 5.0,
  "average_speed_bps": 4194304,
  "average_speed_mbps": 4.0,
  "peak_speed_bps": 10485760,
  "peak_speed_mbps": 10.0,
  "chunk_downloads": 100,
  "failed_chunks": 0,
  "retried_chunks": 5,
  "elapsed_time": 25.5
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `total_bytes` | int64 | 已下载总字节数 |
| `Total` | int64 | 文件总大小（预估） |
| `current_speed_bps` | float64 | 当前下载速度（bytes/s） |
| `current_speed_mbps` | float64 | 当前下载速度（MB/s） |
| `average_speed_bps` | float64 | 平均下载速度（bytes/s） |
| `average_speed_mbps` | float64 | 平均下载速度（MB/s） |
| `peak_speed_bps` | float64 | 峰值下载速度（bytes/s） |
| `peak_speed_mbps` | float64 | 峰值下载速度（MB/s） |
| `chunk_downloads` | int64 | 成功下载的分块数 |
| `failed_chunks` | int64 | 失败的分块数 |
| `retried_chunks` | int64 | 重试的分块数 |
| `elapsed_time` | float64 | 已运行时间（秒） |

### 使用示例

**C 语言**:
```c
char* stats_json = get_performance_stats(downloader_id);
// 使用 stats_json ...
free_string(stats_json);  // 释放内存
```

**Python**:
```python
stats = dl.get_performance_stats(downloader_id)
print(f"当前速度: {stats.get('current_speed_mbps', 0)} MB/s")
print(f"平均速度: {stats.get('average_speed_mbps', 0)} MB/s")
print(f"峰值速度: {stats.get('peak_speed_mbps', 0)} MB/s")
```

### 说明

- 该函数返回的是实时统计数据，在下载过程中会不断变化
- 如果下载器不存在，返回空 JSON 对象 `{}`
- C/C++ 调用者必须使用 `free_string` 释放返回的字符串内存
- Python 接口会自动管理内存，无需手动释放