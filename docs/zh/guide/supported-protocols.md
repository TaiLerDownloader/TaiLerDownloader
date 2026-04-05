# 支持的下载协议

TTHSD Next 内置了灵活的协议路由工厂 (***get_downloader***)，根据 URL 自动选择最优的下载器实现。目前原生支持以下 **7 种下载协议**：

## HTTP / HTTPS

**源码模块**: ***http_downloader.rs***

标准的 HTTP/HTTPS 协议下载，是最常用的下载方式。

**核心特性：**
- **动态工作量窃取分片**：当一个分片完成后，自动从剩余工作量最大的分片中窃取后半部分继续下载，最大化带宽利用率
- **TLS 指纹伪装**：模拟 Chrome 133 的 TLS/JA3/HTTP2 指纹，绕过 Cloudflare 等反爬机制
- **全局连接池**：所有下载器实例共享统一的 HTTP 客户端，减少 TLS 握手开销
- **停滞检测**：30 秒无数据自动重连
- **最大并发**：单文件最多 64 个并发连接

**URL 格式：**
```
https://example.com/file.zip
http://example.com/file.zip
```

---

## HTTP/3 (QUIC)

**源码模块**: ***http3_downloader.rs***

基于 QUIC 协议的 HTTP/3 下载，通过 UDP 传输提速。

**核心特性：**
- **自动探测**：下载前发送 HEAD 请求（800ms 超时），检查服务端 ***Alt-Svc: h3*** 响应头
- **无缝降级**：如果服务器不支持 HTTP/3，自动回退到 HTTP/HTTPS
- **QUIC 协议栈**：使用 ***quinn*** + ***h3*** 构建完整的 QUIC/HTTP3 协议栈

**触发条件：**
- URL 以 ***https://*** 开头且服务器返回 ***Alt-Svc*** 包含 ***h3***

> [!TIP]
> 如 Cloudflare、Google 等 CDN 通常支持 HTTP/3，下载这类资源时会自动启用。

---

## FTP

**源码模块**: ***ftp_downloader.rs***

标准 FTP 文件传输协议。

**核心特性：**
- 支持匿名登录和密码认证
- 二进制传输模式
- 64KB 读取缓冲

**URL 格式：**
```
ftp://ftp.example.com/path/to/file.zip
ftp://user:password@ftp.example.com/path/to/file.zip
```

---

## SFTP (SSH)

**源码模块**: ***sftp_downloader.rs***

基于 SSH 隧道的安全文件传输协议，通过 ***russh*** 和 ***russh-sftp*** 实现纯 Rust 异步连接。

**核心特性：**
- 密码认证和密钥认证
- 自动流式传输（无需暂存完整文件到内存）
- 进度回调集成（实时字节数统计）
- 性能监控集成

**URL 格式：**
```
sftp://user:password@host/path/to/file
sftp://user:password@host:2222/path/to/file
```

> [!TIP]
> SFTP 与 FTP/FTPS 是完全不同的协议。SFTP 基于 SSH 隧道加密传输，安全性更高。

---

## BitTorrent / Magnet

**源码模块**: ***torrent_downloader.rs***

基于 ***librqbit***（纯 Rust BitTorrent 客户端）的种子下载。

**核心特性：**
- 支持磁力链接 (***magnet:***) 和 ***.torrent*** 文件 URL
- 完整的 DHT 网络支持
- PEX (Peer Exchange) 节点发现
- 1 秒轮询进度更新

**URL 格式：**
```
magnet:?xt=urn:btih:\<hash\>&dn=\<name\>
https://example.com/file.torrent
```

> [!NOTE]
> BT 下载需要 DHT 网络预热，通常 30-60 秒后才开始接收到 Peer 数据。

---

## ED2K (eMule)

**源码模块**: ***ed2k_downloader.rs***

eDonkey2000 协议链接的下载，无需安装传统电驴客户端。

**核心特性：**
- 解析标准 ***ed2k://*** URL 格式
- 通过 HTTP 网关 (`https://ed2k.lyoko.io/hash/<hash>`) 转换为 HTTP 下载
- 自动文件名 percent-decode

**URL 格式：**
```
ed2k://|file|\<文件名\>|\<大小\>|\<Hash\>|/
```

> [!WARNING]
> ED2K 下载依赖第三方 HTTP 网关服务 ***ed2k.lyoko.io***，其可用性取决于该服务的运行状态。

---

## Metalink 4.0

**源码模块**: ***metalink_downloader.rs***

Metalink (***.meta4*** / ***.metalink***) 是一种 XML 元数据格式，包含多个镜像 URL 及其优先级。

**核心特性：**
- 解析 Metalink 4.0 XML 文件
- 按 ***priority*** 字段排序，自动选择最优 HTTP/HTTPS 镜像
- 流式下载所选镜像

**URL 格式：**
```
https://example.com/file.meta4
https://example.com/file.metalink
```

---

## 协议路由逻辑

下载器的协议选择由 ***get_downloader()*** 工厂函数自动完成：

| URL 特征 | 路由到 |
|-----------|--------|
| ***http://*** 或 ***https://*** | HTTPDownloader 或 HTTP3Downloader（自动探测） |
| ***ftp://*** 或 ***ftps://*** | FTPDownloader |
| ***sftp://*** | SFTPDownloader |
| ***magnet:*** 或 URL 以 ***.torrent*** 结尾 | TorrentDownloader |
| ***ed2k://*** | ED2KDownloader |
| URL 以 ***.metalink*** 或 ***.meta4*** 结尾 | MetalinkDownloader |
| 其他 | 回退到 HTTPDownloader |

---

**下一步**:
- [快速开始](/zh/guide/getting-started) - 各协议的实际测试用例
- [API 详解](/zh/api/API-overview) - 了解如何调用下载器
