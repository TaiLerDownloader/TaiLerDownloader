# set_proxy 函数

**功能描述**：设置代理服务器。  
**返回值**：成功返回 0，失败返回 -1。

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | `int` | 下载器实例 ID |
| `proxyUrl` | `char*` | 代理服务器 URL（可选，NULL 表示不使用代理） |

### 支持的代理协议

- `http://proxy:port` - HTTP 代理
- `https://proxy:port` - HTTPS 代理
- `socks5://proxy:port` - SOCKS5 代理

### 使用示例

**C 语言**:
```c
int result = set_proxy(downloader_id, "http://proxy.example.com:8080");
```

**Python**:
```python
dl.set_proxy(downloader_id, "http://proxy.example.com:8080")  # 设置代理
dl.set_proxy(downloader_id, None)  # 禁用代理
```

### 说明

- 设置代理后，所有下载请求将通过指定代理服务器
- 设置为 NULL/None 表示不使用代理
- 该设置仅影响当前下载任务，不会影响后续创建的下载器