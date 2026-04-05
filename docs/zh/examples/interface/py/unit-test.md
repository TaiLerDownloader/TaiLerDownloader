# TTHSD Python 单元测试框架

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的 Python 单元测试框架。
> 
> 该测试框架位于 `scripts/test_tthsd.py`，用于测试 TTHSD 动态库的各项功能。

## 运行测试

### 基本命令

```bash
# 运行所有测试
python scripts/test_tthsd.py

# 详细输出
python scripts/test_tthsd.py -v

# 运行特定测试类
python scripts/test_tthsd.py TestDownload

# 运行特定测试
python scripts/test_tthsd.py TestDownload.test_http_download
```

## 测试类说明

### TestTTHSDInterface
测试 TTHSD 接口的基本功能。

- `test_interface_load`: 测试接口能否正常加载
- `test_get_downloader_invalid_params`: 测试无效参数处理
- `test_get_downloader_single_task`: 测试创建单个任务下载器

### TestDownloadFunctionality
测试下载功能。

- `test_http_download`: 测试 HTTP 下载
- `test_multiple_urls`: 测试多 URL 下载
- `test_callback_function`: 测试回调函数

### TestSpeedLimit
测试速度限制功能。

- `test_set_speed_limit`: 测试设置速度限制

### TestProxySupport
测试代理支持。

- `test_set_proxy`: 测试设置代理
- `test_disable_proxy`: 测试禁用代理

### TestRetryConfig
测试重试配置。

- `test_set_retry_config`: 测试设置重试配置

### TestPerformanceStats
测试性能统计。

- `test_get_performance_stats`: 测试获取性能统计
- `test_performance_stats_no_downloader`: 测试获取不存在的下载器统计

### TestErrorHandling
测试错误处理。

- `test_invalid_url`: 测试无效 URL 处理

### TestPauseResume
测试暂停和恢复功能。

- `test_pause_resume`: 测试暂停和恢复

### TestHeaders
测试 Headers 功能。

- `test_global_headers`: 测试全局 Headers
- `test_task_headers`: 测试单个任务 Headers
- `test_combined_headers`: 测试全局 + 任务 Headers 合并

## 注意事项

1. 测试需要 TTHSD 动态库文件（tthsd.dll/.so/.dylib）
2. 部分测试会创建临时文件和目录，测试结束后会自动清理
3. 测试使用 httpbin.org 作为测试服务器，确保网络连接正常
4. 某些测试可能需要较长时间运行（如下载测试）