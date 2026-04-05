<script setup>
import { VPTeamMembers } from 'vitepress/theme'

const members_CoreStudio = [
  {
    avatar: 'https://images-sxxyrry.pages.dev/sxxyrryAvatar_old.jpg',
    name: 'TT23XR Studio',
    title: '项目创始工作室',
    links: [
      { icon: 'github', link: 'https://github.com/sxxyrry' },
    ]
  },
]

const members_Core = [
  {
    avatar: 'https://images-sxxyrry.pages.dev/sxxyrryAvatar.jpg',
    name: 'Sxxyrry（来自 TT23XR Studio ）',
    title: '项目负责人，作者',
    links: [
      { icon: 'github', link: 'https://github.com/sxxyrry' },
    ]
  },
  {
    avatar: 'https://images-sxxyrry.pages.dev/Wangziqi0Avatar.png',
    name: 'Wangziqi0',
    title: '文档审查， Rust 版本主要修复者',
    links: [
      { icon: 'github', link: 'https://github.com/Wangziqi0' }
    ]
  },
]

const members_Contributor = [
  {
    avatar: 'https://images-sxxyrry.pages.dev/XiaoHuiHuiAvatar.jpg',
    name: 'XiaoHuiHuiB',
    title: '文档审查和测试',
    links: [
      { icon: 'github', link: 'https://github.com/xiaohuihuib' }
    ]
  },
  {
    avatar: 'https://images-sxxyrry.pages.dev/ChengHaoLee-2012Avatar.jpg',
    name: 'ChengHaoLee',
    title: '测试',
    links: [
      { icon: 'github', link: 'https://github.com/chenghaolee-2012' }
    ]
  },
  {
    avatar: 'https://images-sxxyrry.pages.dev/CGrakeskiAvatar.jpg',
    name: 'CGrakeski（来自 Xnors Studio ）',
    title: '文档审查',
    links: [
      { icon: 'github', link: 'https://github.com/CGrakeski' }
    ]
  },
  {
    avatar: 'https://images-sxxyrry.pages.dev/w1wenjieAvatar.png',
    name: 'w1wenjie（来自 TT23XR Studio ）',
    title: '文档审查',
    links: [
      { icon: 'github', link: 'https://github.com/w1wenjie' }
    ]
  },
]
</script>

# 鸣谢

TT 高速下载器 (TTHSD) 项目的成功离不开开源社区的支持和众多贡献者的努力。在此，我们向所有帮助过项目的人和组织表示最诚挚的感谢。

> [!TIP]
> 本文档主要介绍的是 **TTHSD Next**（Rust 版本）的鸣谢信息。
> 
> [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 已停止开发，但我们仍然感谢所有为 Golang 版本做出贡献的人。

## 核心依赖项目

TTHSD Next 的开发离不开以下优秀的开源项目：

### Rust 语言生态

#### 异步运行时与并发
- **[tokio](https://github.com/tokio-rs/tokio)** - 异步运行时
  - 提供了高效的异步 I/O 和任务调度
  - 版本: 1.40
  - 许可证: MIT

- **[tokio-util](https://github.com/tokio-rs/tokio)** - Tokio 实用工具
  - 提供了额外的异步实用功能
  - 版本: 0.7
  - 许可证: MIT

- **[futures](https://github.com/rust-lang/futures-rs)** - Future 抽象
  - 提供了异步编程的核心抽象
  - 版本: 0.3.32
  - 许可证: MIT OR Apache-2.0

- **[async-trait](https://github.com/dtolnay/async-trait)** - 异步 trait 支持
  - 提供了在 trait 中使用 async 的能力
  - 版本: 0.1
  - 许可证: MIT OR Apache-2.0

#### HTTP 客户端
- **[rquest](https://github.com/penumbra-x/rquest)** - HTTP 客户端
  - 提供了支持 TLS 指纹模拟的异步 HTTP 客户端
  - 版本: 5.1
  - 许可证: Apache-2.0

- **[rquest-util](https://github.com/penumbra-x/rquest)** - rquest 工具库
  - 提供了浏览器指纹模拟功能
  - 版本: 2.2
  - 许可证: Apache-2.0

#### HTTP/3 (QUIC)
- **[quinn](https://github.com/quinn-rs/quinn)** - QUIC 协议实现
  - 提供了纯 Rust 的 QUIC 协议栈
  - 版本: 0.11
  - 许可证: MIT OR Apache-2.0

- **[h3](https://github.com/hyperium/h3)** - HTTP/3 实现
  - 提供了 HTTP/3 协议支持
  - 版本: 0.0.8
  - 许可证: MIT

- **[h3-quinn](https://github.com/hyperium/h3)** - h3 与 quinn 集成
  - 提供了 h3 与 quinn 的集成层
  - 版本: 0.0.10
  - 许可证: MIT

#### TLS 加密
- **[rustls](https://github.com/rustls/rustls)** - TLS 实现
  - 提供了纯 Rust 的 TLS 支持
  - 版本: 0.23
  - 许可证: Apache-2.0 OR MIT OR ISC

- **[webpki-roots](https://github.com/rustls/webpki-roots)** - CA 证书
  - 提供了 Mozilla 的根证书集合
  - 版本: 0.26
  - 许可证: MPL-2.0

#### 文件传输协议
- **[suppaftp](https://github.com/veeso/suppaftp)** - FTP 客户端
  - 提供了异步 FTP 协议支持
  - 版本: 8.0
  - 许可证: MIT

- **[russh](https://github.com/warp-tech/russh)** - SSH 客户端
  - 提供了纯 Rust 的 SSH 协议支持
  - 版本: 0.46
  - 许可证: Apache-2.0

- **[russh-sftp](https://github.com/warp-tech/russh)** - SFTP 客户端
  - 提供了基于 russh 的 SFTP 支持
  - 版本: 2.0
  - 许可证: Apache-2.0

#### BitTorrent
- **[librqbit](https://github.com/ikatson/rqbit)** - BitTorrent 客户端
  - 提供了纯 Rust 的 BitTorrent 协议支持
  - 版本: 8.1
  - 许可证: MIT

#### Metalink
- **[metalink](https://github.com/mpolden/metalink-rs)** - Metalink 解析
  - 提供了 Metalink 4.0 XML 解析支持
  - 版本: 0.1
  - 许可证: MIT

#### 序列化与数据
- **[serde](https://github.com/serde-rs/serde)** - 序列化框架
  - 提供了高效的序列化和反序列化功能
  - 版本: 1.0
  - 许可证: MIT OR Apache-2.0

- **[serde_json](https://github.com/serde-rs/json)** - JSON 支持
  - 提供了 JSON 序列化支持
  - 版本: 1.0
  - 许可证: MIT OR Apache-2.0

- **[bytes](https://github.com/tokio-rs/bytes)** - 字节处理
  - 提供了高效的字节缓冲区管理
  - 版本: 1
  - 许可证: MIT

#### WebSocket
- **[tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)** - WebSocket 客户端
  - 提供了基于 tokio 的 WebSocket 支持
  - 版本: 0.24
  - 许可证: MIT

#### 工具库
- **[url](https://github.com/servo/rust-url)** - URL 解析
  - 提供了 URL 解析和处理功能
  - 版本: 2
  - 许可证: MIT OR Apache-2.0

- **[once_cell](https://github.com/matklad/once_cell)** - 延迟初始化
  - 提供了线程安全的延迟初始化
  - 版本: 1.21.3
  - 许可证: MIT OR Apache-2.0

- **[lazy_static](https://github.com/rust-lang-nursery/lazy-static.rs)** - 静态延迟初始化
  - 提供了静态变量的延迟初始化
  - 版本: 1.5.0
  - 许可证: MIT OR Apache-2.0

- **[num_cpus](https://github.com/seanmonstar/num_cpus)** - CPU 核心数
  - 提供了获取系统 CPU 核心数的功能
  - 版本: 1.16
  - 许可证: MIT OR Apache-2.0

- **[http](https://github.com/hyperium/http)** - HTTP 类型
  - 提供了 HTTP 相关的类型定义
  - 版本: 1
  - 许可证: MIT OR Apache-2.0

#### 跨语言绑定
- **[jni](https://github.com/jni-rs/jni-rs)** - Rust JNI 绑定
  - 提供了 Rust 与 Java/Kotlin 的互操作性
  - 版本: 0.21
  - 许可证: MIT OR Apache-2.0

### 构建工具
- **[Rust](https://www.rust-lang.org/)** - Rust 编程语言
  - 提供了强大的内存安全保证和零成本抽象
  - 使 TTHSD Next 能够轻松支持 Windows、Linux、macOS、Android 和 HarmonyOS 平台

### Python 接口
- **[ctypes](https://docs.python.org/3/library/ctypes.html)** - Python 外部函数库
  - 提供了 Python 与 C 动态库的互操作性
  - Python 标准库的一部分

### TypeScript 接口
- **[ffi-napi](https://github.com/node-ffi-napi/node-ffi-napi)** - Node.js FFI 库
  - 提供了 Node.js 与 C 动态库的互操作性
  - 许可证: MIT
- **[koffi](https://github.com/Koromix/koffi)** - Node.js FFI 库
  - 提供了更简洁的 Node.js 与 C 动态库的互操作性
  - 许可证: MIT

### Golang 依赖（由于其已停止开发所以这里只花一小点篇幅提到）

- **[gorilla/websocket](https://github.com/gorilla/websocket)** - WebSocket 库
  - 提供了稳定、高效的WebSocket通信支持

  - 版本:v1.5.3

  - 许可证：BSD-2-Clause

-  **[Go](https://golang.org/)** - Go编程语言
    - 提供了强大的并发特性和跨平台编译能力

    - 使TTHSD能够轻松支持Windows、Linux 和macOS平台

## 项目团队

### 核心开发工作室

<VPTeamMembers size="small" :members="members_CoreStudio" />

### 核心开发

<VPTeamMembers size="small" :members="members_Core" />

### 贡献者

<VPTeamMembers size="small" :members="members_Contributor" />

## 社区贡献者

感谢以下开发者对项目的贡献：

- 感谢所有提交 Issue 和 Pull Request 的开发者
- 感谢提供宝贵建议和反馈的用户
- 感谢帮助完善文档的贡献者

## 特别感谢

- 感谢所有使用 TTHSD Next 的用户和开发者，您的反馈和建议是我们持续改进的动力
- 感谢开源社区提供的宝贵资源和支持
- 感谢 Rust 社区提供的优秀工具和库
- 感谢所有为 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 版本做出贡献的人，为项目奠定了基础

## 资源支持

- **[VitePress](https://vitepress.dev/)** - 文档站点构建工具
- **[GitHub](https://github.com/)** - 代码托管和协作平台

## TTHSD Next vs TTHSD Golang

感谢所有参与 TTHSD Golang 版本开发和测试的贡献者。虽然 TTHSD Golang 已停止开发，但您的贡献为 TTHSD Next 的开发奠定了坚实的基础。

TTHSD Next 在 TTHSD Golang 的基础上进行了重写，使用 Rust 语言实现了更高的性能、更低的内存占用和更稳定的运行表现。

---

## 如何贡献

如果您想为 TTHSD Next 做出贡献，我们欢迎以下形式的帮助：

- **代码贡献**：提交 Pull Request 修复 bug 或添加新功能
- **文档改进**：完善文档，纠正错误，或翻译文档
- **问题反馈**：提交 Issue 报告 bug 或提出功能建议
- **测试验证**：在不同平台和场景下测试 TTHSD Next 的功能
- **推广分享**：向他人介绍和推荐 TTHSD Next

[查看贡献指南](/zh/guide/contributing) | [提交 Issue](https://github.com/TTHSDownloader/TTHSDNext/issues) | [发起 Pull Request](https://github.com/TTHSDownloader/TTHSDNext/pulls)

---

**再次感谢所有为 TTHSD 项目提供帮助的人和组织！** 🎉

无论是 TTHSD Golang 还是 TTHSD Next，每一份贡献都让项目变得更好！
