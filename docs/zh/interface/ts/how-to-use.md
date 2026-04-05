# TT 高速下载器 Node.js/TypeScript 接口封装使用文档

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的 TypeScript 接口。
> 
> API 接口与 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 版本完全兼容，只需替换动态库即可迁移。
> 
> TTHSD Golang 已停止开发，建议使用 TTHSD Next 以获得更好的性能。

本文档介绍 `TTHSDownloader` TypeScript 类，该类为 **TT高速下载器**（`tthsd.dll`/`tthsd.so`/`tthsd.dylib`）提供 Node.js 封装，支持多任务下载的创建、控制与进度回调。该封装以 npm 包 `tthsd` 形式发布。

---

## 0. 安装说明

### 0.1 环境要求
- Node.js 18 及以上版本
- TypeScript 4.0 及以上版本（如使用纯 JavaScript 也可）

### 0.2 安装依赖

```bash
npm install tthsd
# 或
yarn add tthsd
```

`tthsd` 已内置所需的 [Koffi](https://koffi.dev) 依赖（用于调用动态库），并提供了完整的 TypeScript 类型定义，因此无需单独安装其他 FFI 包。

### 0.3 动态库文件
将 TT 高速下载器动态库放置在项目可访问的路径：
- Windows 平台：`tthsd.dll`
- macOS 平台：`tthsd.dylib`
- Linux 平台：`tthsd.so`

> [!TIP]
> 您可以在 [TTHSD Next Releases](https://github.com/TTHSDownloader/TTHSDNext/releases/latest) 页面下载最新版本的动态库文件。

---

## 1. 模块概述

- **核心类**：`TTHSDownloader`（默认导出）
- **便捷函数**：`quickDownload`（快速开始下载，适合简单场景）
- **依赖**：内部使用 `Koffi` 调用 C 动态库，您无需直接操作它们
- **功能**：
  - 创建下载器实例（仅创建或立即启动）
  - 控制下载（开始、暂停、恢复、停止）
  - 通过回调函数接收下载进度、事件通知（`update`、`endOne`、`end`、`msg`、`err` 等）

---

## 2. `TTHSDownloader` 类

### 2.1 构造函数
```typescript
constructor(options?: { dllPath?: string })
```
- **参数** `options.dllPath`：动态库路径（可选）。若不提供，自动在以下目录搜索（优先级从高到低）：
  1. Electron 的 `resources/app.asar.unpacked/` 目录（适用于 Electron 打包场景）
  2. 可执行文件（`process.execPath`）所在目录
  3. 当前工作目录（`process.cwd()`）
  4. `__dirname` 及上级目录

### 2.2 创建下载器
#### `getDownloader(...) => number`
仅创建下载器（不启动），返回实例 ID。
```typescript
getDownloader(
    urls: string[],
    savePaths: string[],
    options?: {
        threadCount?: number;       // 默认 64
        chunkSizeMB?: number;       // 默认 10
        callback?: DownloadCallback; // 进度回调
        userAgent?: string;         // 自定义 UA（不填使用 DLL 内置值）
        useCallbackUrl?: boolean;   // 是否启用远程回调 URL
        remoteCallbackUrl?: string; // 远程回调地址
        useSocket?: boolean;        // 是否使用 Socket（与 WebSocket 二选一）
        showNames?: string[];       // 各任务显示名称覆盖
        ids?: string[];             // 各任务 ID 覆盖（不填自动生成 UUID）
    }
): number
```
- **返回**：下载器实例 ID（正整数），若 DLL 返回 -1 则抛出异常。

#### `startDownload(...) => number`
创建并**立即启动**下载，参数与 `getDownloader` 基本相同，额外支持：
```typescript
startDownload(
    urls: string[],
    savePaths: string[],
    options?: {
        // ... 同 getDownloader 的 options
        isMultiple?: boolean;       // 是否并行下载（默认 false 顺序下载）
    }
): number
```
- **返回**：下载器实例 ID（正整数），若 DLL 返回 -1 则抛出异常。

### 2.3 控制下载
所有控制方法均返回 `boolean`，`true` 表示操作成功（DLL 返回 0），`false` 表示失败（如 ID 不存在）。
- `startDownloadById(downloaderId: number): boolean`：开始顺序下载（配合 `getDownloader` 使用）
- `startMultipleDownloadsById(downloaderId: number): boolean`：开始并行下载（实验性）
- `pauseDownload(downloaderId: number): boolean`：暂停下载
- `resumeDownload(downloaderId: number): boolean`：恢复下载
- `stopDownload(downloaderId: number): boolean`：停止下载并销毁下载器实例（无法恢复）
- `dispose(): void`：释放实例持有的所有资源（回调引用等），通常在程序退出前调用。

---

## 3. 便捷函数 `quickDownload`

对于简单的单次下载任务，可以直接使用 `quickDownload` 函数，它会自动创建下载器、启动下载并返回 Promise（等待下载完成）。

```typescript
import { quickDownload } from "tthsd";

quickDownload({
  urls: ["https://example.com/file1.zip"],
  savePaths: ["./downloads/file1.zip"],
  callback: (event, data) => {
    if (event.Type === "update") {
      console.log(`进度: ${data.Downloaded}/${data.Total}`);
    }
  },
  threadCount: 32,
  chunkSizeMB: 10,
});
```

函数签名：
```typescript
function quickDownload(options: {
    urls: string[];
    savePaths: string[];
    threadCount?: number;
    chunkSizeMB?: number;
    callback?: DownloadCallback;
    userAgent?: string;
    useCallbackUrl?: boolean;
    remoteCallbackUrl?: string;
    useSocket?: boolean;
    isMultiple?: boolean;  // 是否并行下载
    dllPath?: string;      // 动态库路径（可选）
}): Promise<void>;
```

> **注意**：`quickDownload` 返回一个 `Promise`，当所有下载任务完成或发生错误时 resolve/reject。因此它适合在 async 函数中配合 `await` 使用。

---

## 4. 回调函数

### 4.1 定义格式
用户提供的回调函数需接受两个参数：
```typescript
type DownloadCallback = (event: DownloadEvent, data: CallbackData) => void;
```

- **event**：包含事件类型、名称等元数据
  ```typescript
  interface DownloadEvent {
    Type: EventType;      // "start" | "startOne" | "update" | "endOne" | "end" | "msg" | "err"
    Name: string;         // 事件名称（如“开始下载”）
    ShowName: string;     // 任务显示名称
    ID: string;           // 任务 ID
  }
  ```
- **data**：携带具体数据（类型根据 event.Type 变化）

### 4.2 事件类型与数据格式

本部分与 [Event 文档](/zh/event/event-overview) 相同，在此不过多赘述。

### 4.3 线程安全要求
- 回调函数在 **DLL 内部的工作线程** 中执行，但在 Node.js 中会被包装后进入事件循环，因此不会阻塞其他操作。
- **禁止在回调中执行耗时操作**（如磁盘写入、网络请求），也**禁止直接进行可能阻塞事件循环的操作**。
- 如果多个下载器共享同一个回调，需自行加锁保护共享数据（Node.js 是单线程，但回调可能交错，可使用原子操作或队列）。

---

## 5. 使用示例

### 5.1 基本用法（创建后手动启动）
```typescript
import { TTHSDownloader } from 'tthsd';

const onProgress = (event: any, msg: any) => {
    if (event.Type === 'update') {
        const total = msg.Total;
        const downloaded = msg.Downloaded;
        const taskId = event.ID;
        if (total) {
            console.log(`进度 [${taskId}]: ${(downloaded / total * 100).toFixed(1)}%`);
        }
    } else if (event.Type === 'startOne') {
        const url = msg.URL;
        const taskId = event.ID;
        console.log(`开始下载: ${url} (ID: ${taskId})`);
    } else if (event.Type === 'endOne') {
        const url = msg.URL;
        const taskId = event.ID;
        console.log(`下载完成: ${url} (ID: ${taskId})`);
    } else if (event.Type === 'err') {
        console.error('错误:', msg.Error);
    }
};

const downloader = new TTHSDownloader(); // 自动搜索动态库
const id = downloader.getDownloader(
    ['https://example.com/file1.zip', 'https://example.com/file2.zip'],
    ['./downloads/file1.zip', './downloads/file2.zip'],
    {
        threadCount: 32,
        chunkSizeMB: 5,
        callback: onProgress
    }
);

if (id > 0) {
    downloader.startDownloadById(id);
    // 稍后可以暂停、恢复等
    // downloader.pauseDownload(id);
}
```

### 5.2 创建后手动启动（并行下载）
```typescript
import { TTHSDownloader } from 'tthsd';

const downloader = new TTHSDownloader({ dllPath: './custom/path/libtthsd.so' });
const id = downloader.getDownloader(
    ['https://example.com/bigfile.iso'],
    ['./downloads/bigfile.iso'],
    {
        threadCount: 64,
        chunkSizeMB: 10,
        callback: onProgress
    }
);
if (id > 0) {
    downloader.startMultipleDownloadsById(id);
}
```

### 5.3 使用内置事件日志器
```typescript
import { TTHSDownloader, EventLogger } from 'tthsd';

const logger = new EventLogger(); // 内置的控制台打印器
const downloader = new TTHSDownloader();
const id = downloader.startDownload(
    ['https://example.com/file.zip'],
    ['./downloads/file.zip'],
    { callback: logger.callback }
);
```

---

## 6. 内存管理

### 6.1 回调函数引用管理
每次调用 `getDownloader` 或 `startDownload` 时，若提供了回调函数，会通过 `Koffi` 创建一个 C 可调用对象。该对象必须保持存活，否则 C 代码调用时会崩溃。

TypeScript 接口通过以下方式管理这些引用：
- 回调对象被保存在内部 Map 中（键为下载器 ID）
- 调用 `stopDownload` 时会自动取消注册对应的回调引用
- 调用 `dispose()` 会释放所有引用
- 垃圾回收器不会自动回收这些引用（因为 C 代码持有它们），因此必须显式调用 `stopDownload` 或 `dispose` 来避免内存泄漏。

**最佳实践**：在下载完成后调用 `stopDownload` 释放资源，或在程序退出前调用 `dispose()`。

---

## 7. 注意事项

1. **DLL 路径**：若自动搜索失败，需显式传入正确路径。动态库必须与 Node.js 进程架构（32/64 位）匹配。
2. **任务数据一致性**：`urls` 与 `savePaths` 长度必须相等。
3. **ID 有效性**：所有控制方法（`pauseDownload` 等）在 ID 不存在时返回 `false`，不会抛出异常。
4. **回调异常**：回调中若抛出异常，会被模块捕获并输出到控制台（`console.error`），但不会中断下载流程。
5. **多线程环境**：Node.js 是单线程事件循环，但回调可能来自不同线程，不过经过 Koffi 处理后它们会被调度到事件循环中，因此无需额外加锁。但要注意不要在回调中执行同步耗时操作。
6. **Node.js 版本**：Koffi 需要 Node.js 18 或更高版本，建议使用 LTS 版本。

---

## 8. TTHSD Next 的优势

相比 TTHSD Golang 版本，使用 TTHSD Next 的 TypeScript 接口可以获得以下优势：

### 性能提升
- **更高的下载速度**：优化的异步 I/O 模型
- **更低的内存占用**：精细的内存管理，无 GC 开销
- **更高的并发数**：可支持数十万并发连接
- **更稳定的性能**：无 GC 暂停，性能可预测

### 兼容性
- **完全兼容**：API 接口与 TTHSD Golang 版本完全兼容
- **直接替换**：只需替换动态库文件即可迁移
- **代码无需修改**：现有代码可以直接使用

---

## 9. 常见问题

**Q: 为什么我的回调没有收到 `update` 事件？**
A: 检查 `callback` 参数是否传入，且您的回调处理正常。也可通过 `EventLogger` 观察是否有回调异常。

**Q: 需要手动释放资源或调用 close() 吗？**
A: 建议在下载完成后调用 `stopDownload` 释放对应 ID 的资源。如果不再使用整个实例，可调用 `dispose()`。但实例被垃圾回收时也会尝试释放，不过不保证及时。

**Q: 需要并发下载多个任务组怎么办？**
A: 创建多个 `TTHSDownloader` 实例，每个管理一组任务。它们独立运行。或配合 `worker_threads` 模块执行多次开始下载。

**Q: TTHSD Next 与 TTHSD Golang 有什么区别？**  
A: TTHSD Next 是 TTHSD Golang 的 Rust 重写版本，具有更高的性能、更低的内存占用和更稳定的运行表现。API 接口完全兼容，只需替换动态库即可迁移。

**Q: 我可以从 TTHSD Golang 迁移到 TTHSD Next 吗？**  
A: 是的，API 接口完全兼容，只需将动态库文件从 TTHighSpeedDownloader.dll/so/dylib 替换为 tthsd.dll/so/dylib 即可，代码无需修改。

**Q: 安装 `tthsd` 后还需要单独安装 `koffi` 吗？**
A: 不需要。`tthsd` 已将其作为依赖包含，您只需安装这一个包即可。

**Q: 如何自定义任务 ID 和显示名称？**
A: 在 `getDownloader` 或 `startDownload` 的 options 中传入 `ids` 和 `showNames` 数组，长度需与 urls 一致。

**Q: 为什么 `getDownloader` 返回的 ID 是负数？**  
A: 正常情况不会返回负数；若返回 -1 会抛出异常。如果看到负数，可能是 DLL 加载失败或参数错误。

---

如有问题，请参考 [TTHSD Next GitHub](https://github.com/TTHSDownloader/TTHSDNext) 、 [tthsd-interface (TypeScript) GitHub ](https://github.com/sxxyrry/tthsd-interface-ts) 或 [API 文档](/zh/api/API-overview.md)。
