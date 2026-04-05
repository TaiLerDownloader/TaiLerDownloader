# TTHSDownloader TypeScript 库 Electron 集成指南

本文档指导您将 **TTHSDownloader** TypeScript 库（基于 [`koffi`](https://koffi.dev) 调用 TT 高速下载器动态库）集成到 Electron 应用中。该库可通过 npm 包 `tthsd` 安装，并用于在 Electron 主进程中创建、控制多任务下载，通过回调接收进度事件。

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的集成方式。
> 
> API 接口与 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 版本完全兼容，只需替换动态库即可迁移。
> 
> TTHSD Golang 已停止开发，建议使用 TTHSD Next 以获得更好的性能。

---

## 1. 概述

`tthsd` 库通过 Node.js 的原生模块 `koffi` 直接调用 C 动态库（`tthsd.dll`/`tthsd.dylib`/`tthsd.so`），实现高性能下载功能。在 Electron 中，建议将该库运行于 **主进程**，因为它依赖于原生模块，且需要与文件系统交互。通过 IPC（进程间通信），您可以安全地将下载进度和控制能力暴露给渲染进程。

---

## 2. 前提条件

- 已初始化的 Electron 项目（可使用 `electron-forge`、`electron-builder` 或手动搭建）
- Node.js 18+ 和 npm/yarn/pnpm
- TT 高速下载器动态库文件（**注意文件名均为小写**）：
  - Windows: `tthsd.dll`
  - macOS: `tthsd.dylib`
  - Linux: `tthsd.so`
- 动态库架构（32/64 位）必须与 Electron 应用的架构一致

> [!TIP]
> 您可以在 [TTHSD Next GitHub Releases](https://github.com/TTHSDownloader/TTHSDNext/releases/latest) 页面下载最新版本的动态库文件。

---

## 3. 安装依赖

在 Electron 项目根目录执行：

```bash
npm install tthsd
npm install --save-dev electron-rebuild
```

`tthsd` 已内置所需的 `koffi` 依赖，并提供了完整的 TypeScript 类型定义，因此无需单独安装其他 FFI 包。

> **注意**：由于 `koffi` 是原生模块，在 Electron 中运行时需要针对 Electron 的 Node 版本重新编译。我们将在后续步骤中通过 `electron-rebuild` 完成。

如果使用 yarn 或 pnpm，请相应替换命令。

---

## 4. 配置 TypeScript

创建或修改 `tsconfig.json`：

```json
{
  "compilerOptions": {
    "target": "es6",
    "module": "commonjs",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "types": ["node"]
  },
  "include": ["src/**/*"]
}
```

`tthsd` 已提供类型定义，`skipLibCheck: true` 可避免潜在的库类型冲突。

---

## 5. 处理动态库路径

`tthsd` 内部已经实现了智能的路径搜索（参见 `native.ts` 中的 `resolveDllPath` 函数），它会按以下顺序自动查找动态库：

1. 构造函数中显式传入的 `dllPath` 参数
2. Electron 的 `resources/app.asar.unpacked/` 目录（适用于 Electron 打包后）
3. 可执行文件（`process.execPath`）所在目录
4. 当前工作目录（`process.cwd()`）
5. `__dirname` 及上级目录

因此，在大多数情况下您**无需手动指定路径**，只需确保动态库文件在以上任一位置即可。如果您需要自定义路径（例如放在项目根目录的 `resources` 文件夹），可以按如下方式传入：

```typescript
import { TTHSDownloader } from 'tthsd';
import path from 'path';

const libPath = path.join(__dirname, '../resources/tthsd.dll'); // 根据平台调整
const downloader = new TTHSDownloader({ dllPath: libPath });
```

### 5.1 开发环境建议

为了方便开发，建议将动态库文件放在项目根目录的 `resources` 文件夹中，然后在代码中使用自动搜索（不传 `dllPath`），因为 `tthsd` 默认会在 `process.cwd()` 中查找，而 `process.cwd()` 通常就是项目根目录。

### 5.2 打包后路径处理

`tthsd` 会自动识别 Electron 的 `app.asar.unpacked` 目录。您只需在打包配置中将动态库文件复制到该目录即可（详见 [第 10 节](#10-打包应用)）。

---

## 6. 在主进程中使用 TTHSDownloader

### 6.1 引入库并创建实例

```typescript
// src/main.ts
import { app, BrowserWindow, ipcMain } from 'electron';
import { TTHSDownloader } from 'tthsd';
import path from 'path';

let mainWindow: BrowserWindow;
let downloader: TTHSDownloader;

app.whenReady().then(() => {
  // 初始化下载器（不传路径则自动搜索）
  downloader = new TTHSDownloader();

  createWindow();
});

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'), // 预加载脚本
      contextIsolation: true, // 启用上下文隔离，增强安全性
    },
  });
  mainWindow.loadFile('index.html');
}
```

### 6.2 通过 IPC 处理下载请求

在主进程中监听渲染进程发起的下载请求，并调用库方法。同时将下载进度通过 IPC 回传。

```typescript
ipcMain.handle('start-download', async (event, urls, savePaths, options) => {
  // 合并用户传入的 options，例如 threadCount、callback 等
  const id = downloader.startDownload(urls, savePaths, {
    threadCount: 32,
    chunkSizeMB: 10,
    callback: (evt, data) => {
      // 通过 IPC 向渲染进程发送进度
      mainWindow.webContents.send('download-progress', { event: evt, data });
    },
    ...options, // 允许渲染进程覆盖默认选项
  });
  return id;
});

ipcMain.handle('pause-download', (event, id) => {
  return downloader.pauseDownload(id);
});

ipcMain.handle('resume-download', (event, id) => {
  return downloader.resumeDownload(id);
});

ipcMain.handle('stop-download', (event, id) => {
  return downloader.stopDownload(id);
});
```

> **说明**：`startDownload` 返回下载器 ID（正整数），后续暂停/恢复/停止都需要此 ID。`pauseDownload`、`resumeDownload`、`stopDownload` 均返回 `boolean` 表示操作是否成功。

### 6.3 使用 `getDownloader` 创建后手动启动

如果您需要先创建下载器实例，稍后再启动下载，可以使用 `getDownloader` 方法：

```typescript
ipcMain.handle('create-downloader', (event, urls, savePaths, options) => {
  const id = downloader.getDownloader(urls, savePaths, options);
  return id;
});

ipcMain.handle('start-download-by-id', (event, id, isMultiple) => {
  if (isMultiple) {
    return downloader.startMultipleDownloadsById(id);
  } else {
    return downloader.startDownloadById(id);
  }
});
```

---

## 7. 预加载脚本与 IPC 通信

为了安全地将功能暴露给渲染进程，需编写预加载脚本（`preload.ts`）：

```typescript
// src/preload.ts
import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('electronAPI', {
  // 启动下载（立即启动）
  startDownload: (urls: string[], savePaths: string[], options?: any) =>
    ipcRenderer.invoke('start-download', urls, savePaths, options),

  // 创建下载器（不启动）
  createDownloader: (urls: string[], savePaths: string[], options?: any) =>
    ipcRenderer.invoke('create-downloader', urls, savePaths, options),

  // 手动启动（顺序/并行）
  startDownloadById: (id: number, isMultiple?: boolean) =>
    ipcRenderer.invoke('start-download-by-id', id, isMultiple),

  pauseDownload: (id: number) => ipcRenderer.invoke('pause-download', id),
  resumeDownload: (id: number) => ipcRenderer.invoke('resume-download', id),
  stopDownload: (id: number) => ipcRenderer.invoke('stop-download', id),

  // 监听下载进度
  onProgress: (callback: (event: any, data: any) => void) => {
    ipcRenderer.on('download-progress', (_, payload) => callback(payload.event, payload.data));
  },
});
```

编译 TypeScript 预加载脚本（需在 `tsconfig.json` 中包含 `preload.ts`，并单独输出到 `dist`）。

---

## 8. 渲染进程调用示例

在渲染进程的 HTML 或 JavaScript 中：

```html
<!-- index.html -->
<button id="downloadBtn">开始下载</button>
<div id="progress"></div>

<script>
window.electronAPI.onProgress((event, data) => {
  console.log('事件:', event);
  console.log('数据:', data);
  if (event.Type === 'update') {
    const percent = ((data.Downloaded / data.Total) * 100).toFixed(1);
    document.getElementById('progress').innerText = `进度: ${percent}%`;
  }
});

document.getElementById('downloadBtn').addEventListener('click', async () => {
  const id = await window.electronAPI.startDownload(
    ['https://example.com/file.zip'],
    ['C:/Downloads/file.zip'], // 请根据平台调整路径格式
    { threadCount: 64 }        // 可选参数
  );
  console.log('下载器ID:', id);
});
</script>
```

---

## 9. 重新编译原生模块

由于 Electron 使用自己的 Node 运行时，必须重新编译 `tthsd` 所依赖的 `koffi` 原生模块。在 `package.json` 中添加脚本：

```json
"scripts": {
  "postinstall": "electron-rebuild"
}
```

然后运行 `npm install` 或 `yarn` 会自动触发重建。如果手动重建，执行：

```bash
npx electron-rebuild
```

---

## 10. 打包应用

### 10.1 配置 electron-builder

在 `package.json` 中添加或修改 `build` 字段，确保动态库被复制到 `resources` 目录（Electron 打包后会成为 `app.asar.unpacked` 的一部分）：

```json
{
  "build": {
    "files": [
      "dist/**/*",
      "node_modules/**/*"
    ],
    "extraResources": [
      {
        "from": "resources/",
        "to": "resources/",
        "filter": ["**/*"]
      }
    ]
  }
}
```

将动态库文件（`tthsd.dll` / `tthsd.dylib` / `tthsd.so`）放入项目根目录的 `resources` 文件夹。

### 10.2 打包命令

```bash
npm run build    # 编译 TypeScript
npm run dist     # 打包应用（根据您的配置，例如 electron-builder）
```

打包后，`tthsd` 会自动在 `resources/app.asar.unpacked` 目录中找到动态库，无需额外代码。

---

## 11. TTHSD Next 的优势

相比 TTHSD Golang 版本，在 Electron 中使用 TTHSD Next 可以获得以下优势：

### 性能提升
- **更高的下载速度**：优化的异步 I/O 模型
- **更低的内存占用**：精细的内存管理，无 GC 开销
- **更稳定的性能**：无 GC 暂停，性能可预测，对 Electron 主进程影响更小

### 兼容性
- **完全兼容**：API 接口与 TTHSD Golang 版本完全兼容
- **直接替换**：只需替换动态库文件即可迁移
- **代码无需修改**：现有代码可以直接使用（注意包名改为 `tthsd`）

---

## 12. 常见问题

### Q1: 编译时提示找不到模块 `tthsd` 的类型声明

**解决方法**：`tthsd` 已内置类型定义，通常无需额外操作。确保您的 `tsconfig.json` 中 `moduleResolution` 设置为 `node`，并且 `node_modules` 被正确解析。

### Q2: 运行时错误 "Dynamic Linking Error" 或 "无法加载动态库"

- 检查动态库路径是否正确：`tthsd` 自动搜索路径包括 `resources/app.asar.unpacked`、`process.cwd()` 等。可以尝试在构造函数中显式传入绝对路径调试。
- 确认动态库架构与 Electron 匹配（例如均为 64 位）。
- 在 Windows 上，确保系统已安装必要的 VC++ 运行库。

### Q3: 回调函数未触发

- 确认 `callback` 参数已传入，且回调函数没有被意外释放。`tthsd` 内部已使用 Map 保存回调引用，只要下载器实例存活，回调就会一直有效。
- 检查动态库是否正确调用回调，可以通过 `EventLogger` 内置类快速测试：
  ```typescript
  import { EventLogger } from 'tthsd';
  const logger = new EventLogger();
  downloader.startDownload(urls, savePaths, { callback: logger.callback });
  ```

### Q4: 打包后找不到动态库

- 验证 `extraResources` 配置是否正确，动态库是否被复制到 `resources` 目录。
- 在代码中打印 `process.resourcesPath` 和最终路径，确认文件存在。`tthsd` 会自动搜索 `resources/app.asar.unpacked`，无需手动拼接路径。

### Q5: 渲染进程无法调用 IPC 方法

- 确保预加载脚本正确编译并加载（在 `BrowserWindow` 中指定了正确的 `preload` 路径）。
- 检查 `contextIsolation` 是否启用，并正确使用 `contextBridge`。

### Q6: 如何自定义任务 ID 和显示名称？

在 `startDownload` 或 `getDownloader` 的 `options` 中传入 `ids` 和 `showNames` 数组，长度需与 `urls` 一致：

```typescript
const id = downloader.startDownload(urls, savePaths, {
  ids: ['task1', 'task2'],
  showNames: ['File 1', 'File 2'],
  // ...其他选项
});
```

### Q7: 使用 `quickDownload` 简化单次下载

`tthsd` 还提供了 `quickDownload` 函数，返回 `Promise&lt;void&gt;`，适合简单场景。但在 Electron 中，由于它内部会创建新的 `TTHSDownloader` 实例，您可能需要自行管理生命周期。建议仅在主进程的简单任务中使用。

---

## 13. 完整示例项目

您可以参考以下目录结构的最小示例：

```
my-electron-app/
├── resources/
│   ├── tthsd.dll   (Windows)
│   ├── tthsd.dylib (macOS)
│   └── tthsd.so    (Linux)
├── src/
│   ├── main.ts                      (主进程)
│   ├── preload.ts                   (预加载脚本)
│   └── renderer/
│       └── index.html               (渲染页面)
├── package.json
├── tsconfig.json
└── ...
```

按照本文档配置后，您即可在 Electron 应用中享受 TT 高速下载器的强大功能。

---

如有问题，请参考 [TTHSD Next GitHub](https://github.com/TTHSDownloader/TTHSDNext) 、 [tthsd-interface (TypeScript) GitHub](https://github.com/sxxyrry/tthsd-interface-ts) 或 [API 文档](/zh/api/API-overview.md)。祝集成顺利！
