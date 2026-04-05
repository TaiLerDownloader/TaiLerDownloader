# 贡献指南

感谢您对 TT 高速下载器 (TTHSD) 项目的关注和贡献意愿！本文档将指导您如何参与 TTHSD 的开发。

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的贡献指南。
> 
> [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 已停止开发，不再接收贡献。

## 贡献方式

我们欢迎以下形式的贡献：

### 1. 代码贡献
- 修复 bug
- 添加新功能
- 优化性能
- 改进代码质量

### 2. 文档改进
- 完善现有文档
- 纠正文档错误
- 添加示例代码
- 翻译文档到其他语言

### 3. 问题反馈
- 报告 bug
- 提出功能建议
- 提供使用反馈

### 4. 测试验证
- 在不同平台测试
- 测试特定功能
- 验证修复效果

### 5. 推广分享
- 向他人介绍 TTHSD
- 分享使用经验
- 撰写教程或博客

## 开发环境设置

### 前置要求

- **Rust 1.70 或更高版本** - 用于编译核心库
- **Python 3.11 或更高版本** - 用于开发和测试 Python 接口
- **Git** - 用于版本控制
- **Cargo** - Rust 包管理器（随 Rust 安装）

### 克隆仓库

```bash
git clone https://github.com/TTHSDownloader/TTHSDNext.git
cd TTHSDNext
```

### 安装依赖

#### Rust 依赖

```bash
cargo build
```

这会自动下载并编译所有依赖项。

#### Python 依赖 （可选，仅用于测试）

```bash
pip install -r requirements.txt
```

## 编译项目

### 编译核心库

#### Windows

```bash
# 编译共享库
cargo build --release
```

输出文件: `target\release\tthsd.dll`

#### Linux

```bash
# 编译共享库
cargo build --release
```

输出文件: `target/release/libtthsd.so`

#### macOS

```bash
# 编译共享库
cargo build --release
```

输出文件: `target/release/libtthsd.dylib`

### 编译 Android 库（可选）

#### Windows

```bash
build-all-android.bat
```

#### Linux/macOS

```bash
chmod +x build-all-android.sh
./build-all-android.sh
```

### 跨平台编译

可以使用 `cargo-ndk` 工具简化 Android 编译：

```bash
cargo install cargo-ndk
cargo ndk -t arm64-v8a -o ./jniLibs build --release --features android
```

## 使用的 Rust 库

TTHSD Next 使用以下主要的 Rust 库：

- **tokio** - 异步运行时
- **rquest** - HTTP 客户端（支持 TLS 指纹模拟）
- **quinn / h3** - HTTP/3 (QUIC) 协议支持
- **suppaftp** - FTP 协议支持
- **russh / russh-sftp** - SFTP 协议支持
- **librqbit** - BitTorrent 协议支持
- **metalink** - Metalink 4.0 解析
- **tokio-tungstenite** - WebSocket 客户端
- **serde / serde_json** - 序列化/反序列化
- **async-trait** - 异步 trait 支持
- **jni** - Android JNI 支持（可选）

## 提交代码

### 分支策略

- **main** - 主分支，稳定版本
- **develop** - 开发分支，功能开发
- **feature/xxx** - 功能分支
- **fix/xxx** - 修复分支

### 提交规范

使用清晰的提交信息格式：

```
<type>(<scope>): <subject>

<body>

<footer>
```

**类型 (type):**
- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档更新
- `style`: 代码格式（不影响功能）
- `refactor`: 重构
- `test`: 测试相关
- `chore`: 构建或辅助工具
- `perf`: 性能优化

**示例:**

```
feat(api): 添加断点续传功能

实现了基于本地文件的断点续传功能，
支持 HTTP Range 请求。

使用 Rust 的异步特性优化了性能。

Closes #123
```

### Pull Request 流程

1. Fork 本项目到您的 GitHub 账户
2. 从 `develop` 分支创建新的功能分支
3. 进行开发和测试
4. 运行测试确保没有回归
5. 提交代码到您的 Fork 仓库
6. 创建 Pull Request 到项目的 `develop` 分支
7. 等待代码审查和反馈

### Pull Request 要求

- **描述清晰**：详细说明更改的目的和内容
- **测试通过**：确保所有测试通过
- **代码规范**：遵循 Rust 的代码风格（`cargo fmt`）
- **文档更新**：如有必要，更新相关文档
- **无冲突**：确保与目标分支无冲突
- **性能考虑**：对于性能敏感的代码，提供性能对比数据

## 报告问题

### Issue 模板

提交 Issue 时，请提供以下信息：

#### Bug 报告

```markdown
**问题描述**
简要描述遇到的问题

**复现步骤**
1. 步骤一
2. 步骤二
3. ...

**预期行为**
描述预期的正确行为

**实际行为**
描述实际发生的行为

**环境信息**
- 操作系统：
- TTHSD 版本：
- Python 版本（如适用）：
- Rust 版本：
- CPU 架构：

**附加信息**
- 错误日志
- 截图（如适用）
- 其他相关信息
```

#### 功能建议

```markdown
**功能描述**
清晰描述您希望添加的功能

**使用场景**
说明该功能的使用场景和价值

**可能的实现方案**
如果您有实现想法，请分享

**附加信息**
- 相关 Issue
- 参考资料
```

## 代码规范

### Rust 代码

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 进行代码检查
- 遵循 Rust 官方编码规范
- 添加必要的注释和文档
- 使用有意义的变量和函数名
- 编写单元测试和集成测试

### Python 代码

- 遵循 PEP 8 编码规范
- 使用类型注解
- 添加 docstring 文档字符串
- 编写单元测试

### 文档

- 使用 Markdown 格式
- 保持简洁明了
- 提供示例代码
- 保持与代码同步更新

## 性能考虑

由于 TTHSD Next 是一个高性能下载器，在提交代码时请注意：

1. **避免不必要的分配**：尽量重用内存
2. **使用异步 I/O**：充分利用 Rust 的异步特性
3. **测试性能影响**：对于关键路径，使用基准测试验证性能
4. **文档化性能决策**：在代码注释中说明重要的性能权衡

## 行为准则

### 尊重与包容

- 尊重所有贡献者
- 包容不同的观点和背景
- 建设性的反馈和讨论
- 避免人身攻击和歧视

### 协作精神

- 积极参与讨论
- 乐于帮助他人
- 分享知识和经验
- 共同维护项目健康

## 测试

在提交 PR 之前，请确保：

1. **单元测试**：运行 `cargo test`
2. **集成测试**：在不同平台上测试
3. **性能测试**：确保没有性能回归
4. **格式检查**：运行 `cargo fmt --check`
5. **代码检查**：运行 `cargo clippy`

## 许可证

通过向 TTHSD Next 项目贡献代码，您同意您的贡献将遵循项目的 [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.en.html) 许可证。

注意：文档采用 [GNU Affero General Public License v3.0](https://www.gnu.org/licenses/agpl-3.0.en.html) 许可证

## 联系方式

- **GitHub Issues**: [提交 Issue](https://github.com/TTHSDownloader/TTHSDNext/issues)
- **Pull Requests**: [发起 PR](https://github.com/TTHSDownloader/TTHSDNext/pulls)
- **Discussions**: [参与讨论](https://github.com/TTHSDownloader/TTHSDNext/discussions)

## TTHSD Next vs TTHSD Golang

请注意，本文档仅适用于 TTHSD Next（Rust 版本）。TTHSD Golang 已停止开发，不再接收贡献。如果您想为 TTHSD Golang 版本贡献，请注意：

- TTHSD Golang 已停止维护
- 不再接收新的功能和修复
- 建议将精力投入到 TTHSD Next 的开发中

---

**再次感谢您的贡献！** 🙏

每一份贡献都让 TTHSD 变得更好！
