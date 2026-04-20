<div align="center">
  <h1>TLD Core TaiLerDownloader Core</h1>
  <p>一个高性能、跨平台、多语言可调用的下载引擎内核</p>
  <img src="https://img.shields.io/badge/Rust-1.75+-orange.svg" alt="Rust Version">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS%20%7C%20Android%20%7C%20iOS%20%7C%20HarmonyOS-blue.svg" alt="Platform">
  <img src="https://img.shields.io/badge/License-GPL--3.0-green.svg" alt="License">
</div>

> Copyright © 2026 - present TT23XRStudio 保留所有权利

> **⚠️ 法律状态提示**  
> 本项目的作者发布时为未成年人，但本项目的开发与发布已获得法定代理人的知情与同意。  
> 本项目采用 GNU AGPL v3.0 + 附加权限进行许可，该许可具有法律约束力。  
> 如您对商业使用的合规性有严格内部要求，建议等待作者成年后获取书面商业许可，或联系作者协商。

## 概述

**TLD 核心**（TaiLerDownloader Core）是一个高性能、跨平台、多语言可调用的下载引擎内核，可为外部项目提供强大的下载能力支持，使开发者能够在自己的应用中轻松集成专业级的文件下载功能。该项目使用 **Rust** 语言开发，编译为 DLL/SO/DYLIB 动态库，供全语言原生调用。

> [!TIP]
> 本项目是 [TaiLerDownloader Golang](https://github.com/TaiLerDownloader/TTHighSpeedDownloader) 的 Rust 完全重写版本。
>
> Tip: TLD Core 原名 TaiLerDownloader Next ，现在已改名为 TLD Core
> **注**：[TaiLerDownloader Golang](https://github.com/TaiLerDownloader/TTHighSpeedDownloader) 已经停止开发，强烈建议所有新项目迁移至性能更优的 TLD。

## ✨ 功能特性

- **极致性能**: 多线程并发下载，极高的下载速度，全面压榨带宽限制。
- **批量与并发**: 支持多个文件同时并行下载。
- **实时监控**: 提供实时进度监控和瞬时下载速度计算。
- **断点续传**: 原生支持随时暂停、中断和恢复下载功能。
- **高度自定义**: 支持自定义工作线程数和单次网络请求的分块(Chunk)大小。
- **智能重试**: 自动重试机制，支持指数退避策略，有效应对网络波动。
- **速度限制**: 支持全局和单任务下载速度限制，避免占用全部带宽。
- **代理支持**: 支持 HTTP/HTTPS/SOCKS5 代理服务器配置。
- **性能统计**: 提供实时下载性能数据统计，包括速度和进度。
- **自定义请求头**: 支持全局和单任务级别的自定义 HTTP 请求头。
- **多语言生态体系**: 
  - 提供标准的 C ABI 接口，支持 C/C++, Python, C# 等语言快速调用。
  - 原生暴露 JNI 接口，可直接在 Android / Java / Kotlin 应用中被集成。
- **丰富的任务信息**: 支持追踪任务 URL、保存路径、自定义显示名称和唯一任务 ID。
- **非阻塞式架构**: 回调事件全是异步的，保证绝对不会阻塞调用者的主线程 UI。
- **进度推送协议**: 内置对 WebSocket 和 Socket 通信的支持，并且支持将下载进度通过指定的 URL 自动回调推送。

## 🌐 支持的下载协议

由于内置了灵活的协议路由工厂，TaiLerDownloader Next 原生支持多达 **7 种**下载协议环境：
- **HTTP / HTTPS**: 支持动态工作量窃取的自适应并发分片，并附带针对反爬机制的 TLS 证书指纹伪装能力。
- **HTTP/3 (QUIC)**: 针对支持 `Alt-Svc: h3` 头的服务器，进行无缝探查并使用 QUIC UDP 提速。
- **FTP / FTPS**: 提供全功能的带密码登录或匿名服务器文件访问下载能力。
- **SFTP (SSH)**: 基于 SSH 隧道的安全文件传输，支持密码和密钥认证，通过 `russh` 实现纯 Rust 异步连接。
- **BitTorrent / Magnet**: 利用纯 Rust 的 DHT 网络节点构建实现了完整的种子文件解析和磁力链接解析。
- **ED2K (eMule)**: 无需臃肿的电驴客户端，通过底层 HTTP 代理转换技术直接拉取资源。
- **Metalink 4.0 (.meta4)**: XML 镜像元数据深度解析，智能筛选出当前网络最佳镜像优先级后极速下载。

## 📊 和 Golang 版本 (前代) 的性能对比

作为一个采用系统级语言重写的下一代内核，TLD Core 带来了根本性的提升：

- 🚀 **更快的绝对速度**: 网卡吞吐极限更高。
- 📉 **极低的内存占用**: 峰值内存占用大幅度下降（通常运行在十几 MB 测试级别内，不会随并发任务直线上升）。
- 🔥 **海量并发能力**: 最大网络并发数可达数十万个句柄。
- 🛡️ **无垃圾回收停顿**: 拥有更安全的内存管理体系，实现真·零 GC (Garbage Collection) 暂停，避免了 Golang 遇到的长时卡顿。
- 📱 **新增 Android 支持**: 补齐了在移动端 ARM 系列架构下稳定运行的能力。
- 🌌 **新增 HarmonyOS 鸿蒙支持**: 第一时间适配基于 OpenHarmony SDK 的全原生 aarch64 跨平台核心。

---

## 📦 发行版下载与目录结构

您可以直接在 GitHub 的 `Releases` 页面中下载预先编译好的开箱即用压缩包 `TaiLerDownloader_Release.7z`，其结构如下：

```text
📁 TaiLerDownloader_Release/
 ├── 📁 desktop/
 │    ├── TaiLerDownloader.dll           # Windows x86_64 动态库
 │    ├── TaiLerDownloader_arm64.dll     # Windows ARM64 动态库
 │    ├── TaiLerDownloader.so            # Linux x86_64 动态库
 │    ├── TaiLerDownloader_arm64.so      # Linux ARM64 动态库
 │    ├── TaiLerDownloader.dylib         # macOS Intel 动态库
 │    └── TaiLerDownloader_arm64.dylib   # macOS Silicon 动态库
 ├── 📁 android/
 │    ├── TaiLerDownloader_android_arm64.so # Android ARM64 库
 │    ├── TaiLerDownloader_android_armv7.so # Android 32位 (armeabi-v7a)
 │    └── TaiLerDownloader_android_x86_64.so# Android 模拟器库
 ├── 📁 harmony/
 │    └── TaiLerDownloader_harmony_arm64.so # HarmonyOS ARM64 库
 ├── 📁 ios/
 │    └── TaiLerDownloader_ios_arm64_device.dylib # iOS ARM64 设备库
 │    └── TaiLerDownloader_ios_arm64_simulator.dylib # iOS ARM64 模拟器库
 └── 📁 scripts/
      ├── TaiLerDownloader_interface.py     # Python FFI 接口封装示范
      └── test_comprehensive.py  # 本地全量功能、稳定性与性能压测套件
```

---

## 🚀 快速上手 (Python 示例)

你可以通过内置的 `ctype` 直接调用 TaiLerDownloader。我们提供了 `tld_interface.py` （在 scripts 目录下）方便你将其作为参考直接集成进 Python 等其他高级语言项目中。

```python
import time
from scripts.TaiLerDownloader_interface import TaiLerDownloader, EventLogger

# 1. 实例化下载器引擎，跨平台自动加载对应的动态链接库
downloader = TaiLerDownloader('./desktop/TaiLerDownloader.so') # 以Linux路径为例

# 2. 定义回调日志监听器以接收下载器的异步事件
logger_callback = EventLogger()

# 3. 发起多线程并发下载任务
task_id = downloader.start_download(
    urls=["https://example.com/large_file.zip"],
    save_paths=["/tmp/large_file.zip"],
    thread_count=8,           # 启用 8 个内部异步分片并发
    chunk_size_mb=2,          # HTTP分片包大小 2MB
    callback=logger_callback
)

print(f"📦 任务已提交到底层引擎，任务内部 ID 为: {task_id}")

# ...你的主线程可以继续做任何事情，完全无阻塞...
time.sleep(10) 

# 下载结束后安全关闭引擎清理内存
downloader.close()
```

---

## 🛠️ 本地编译指南

如果项目 Release 包中不包含你需要的目标架构（例如特殊的嵌入式架构），你可以使用 Cargo 非常容易地从零编译此项目：

### 1. 基础环境搭建

你需要安装最新的夜叉 (Nightly) 或者稳定版 Rust 构建工具链：
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
随后拉取本项目源代码：
```bash
git clone https://github.com/TaiLerDownloader/TaiLerDownloader.git
cd TaiLerDownloader
```

### 2. 编译为本地平台库 (Windows / Linux / macOS)

无需额外配置，自动根据你的开发平台生成动态链接库：
```bash
cargo build --release
```
编译产物位于 `target/release/` 目录下。

### 3. 交叉编译为 Android (JNI) 库

使用 `cargo-ndk` 可以极其简便地构建 Android `jniLibs`：
```bash
cargo install cargo-ndk
rustup target add aarch64-linux-android
cargo ndk --target arm64-v8a --platform 21 build --release --features android
```

### 4. 交叉编译为 HarmonyOS 库

鸿蒙系统使用独立的 OHOS NDK 和专有编译规则链。我们推荐您：
1. **直接参考项目目录中的：`.github/workflows/build_and_test.yml` GitHub Actions CI 文件**，它带有最完整的华为 OpenHarmony SDK 的下载、环境路径配置以及强制软链接修复流程。
2. 安装对应目标的 Rust 支持库并替换 C 编译器环境。

## 📄 协议

本项目基于 **[GNU Affero General Public License v3.0 (AGPL-3.0)](LICENSE)** 协议开源。这保证了核心底层下载软件始终维持开源与自由复制分发的权利，对代码的任意修改也请务必同等以 GPL 协议开源并向社区开放。

## 其他

### 文档：[文档](https://docss.sxxyrry.qzz.io/TaiLerDownloader/)
### 文档（备用）：[文档](https://docss-23xr.pages.dev/TaiLerDownloader/)

### **本项目隶属于 TT23XR Studio**

**注意：本 README.MD 不会进行维护，只有文档站点中的文档会维护**
