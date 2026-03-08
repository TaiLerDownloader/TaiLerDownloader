/**
 * TTHSD Native Bindings via Koffi
 *
 * 使用 Koffi 在运行时动态加载 TTHSD 动态库（.dll/.so/.dylib），
 * 并将 C ABI 导出函数映射为 TypeScript 可调用的函数。
 *
 * 支持 Node.js、Electron（包括 asar.unpacked 资产路径解析）。
 */

import koffi from "koffi";
import * as path from "path";
import * as fs from "fs";
import { DownloadCallback, DownloadEvent, CallbackData } from "./types";

// ------------------------------------------------------------------
// 动态库路径自动解析
// ------------------------------------------------------------------

/**
 * 根据当前操作系统和 CPU 架构决定动态库文件名。
 *
 * TTHSD 命名规则:
 *   桌面: x86_64 → tthsd.*, arm64 → tthsd_arm64.*
 *   Android: tthsd_android_{arm64,armv7,x86_64}.so
 *   HarmonyOS: tthsd_harmony_{arm64,x86_64}.so
 */
function getDefaultLibName(): string {
  const arch = process.arch; // x64, arm64, arm, ia32, etc.

  // Android 检测（如 Termux 等）
  if (process.platform === "android") {
    if (arch === "arm64") return "tthsd_android_arm64.so";
    if (arch === "arm")   return "tthsd_android_armv7.so";
    return "tthsd_android_x86_64.so";
  }

  switch (process.platform) {
    case "win32":
      return arch === "arm64" ? "tthsd_arm64.dll" : "tthsd.dll";
    case "darwin":
      return arch === "arm64" ? "tthsd_arm64.dylib" : "tthsd.dylib";
    default: // linux
      return arch === "arm64" ? "tthsd_arm64.so" : "tthsd.so";
  }
}

/**
 * 自动解析动态库路径。
 * 查找顺序:
 *   1. 用户指定路径
 *   2. 可执行文件目录（Electron 打包 asar.unpacked 场景）
 *   3. 当前工作目录
 *   4. __dirname 同级目录
 */
export function resolveDllPath(userPath?: string): string {
  const libName = getDefaultLibName();

  if (userPath) {
    const abs = path.resolve(userPath);
    if (fs.existsSync(abs)) return abs;
    throw new Error(`[TTHSD] 指定的动态库路径不存在: ${abs}`);
  }

  // Electron: app.asar.unpacked 目录
  const appDir = process.execPath ? path.dirname(process.execPath) : "";
  const candidates: string[] = [
    path.join(appDir, "resources", "app.asar.unpacked", libName),
    path.join(appDir, libName),
    path.join(process.cwd(), libName),
    path.join(__dirname, "..", libName),
    path.join(__dirname, libName),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }

  throw new Error(
    `[TTHSD] 未找到动态库文件 "${libName}"，已搜索以下路径:\n` +
    candidates.map(c => `  - ${c}`).join("\n") + "\n" +
    "请通过 dllPath 参数显式指定动态库路径。"
  );
}

// ------------------------------------------------------------------
// Koffi 绑定层
// ------------------------------------------------------------------

/** C 回调类型：void callback(const char* event_json, const char* data_json) */
const CALLBACK_PROTO = koffi.proto("void CallbackFn(const char *event, const char *data)");

/** 已加载的库和函数引用 */
let _lib: koffi.IKoffiLib | null = null;

interface NativeFunctions {
  get_downloader:               (...args: unknown[]) => number;
  start_download:               (...args: unknown[]) => number;
  start_download_id:            (id: number) => number;
  start_multiple_downloads_id:  (id: number) => number;
  pause_download:               (id: number) => number;
  resume_download:              (id: number) => number;
  stop_download:                (id: number) => number;
}

let _fn: NativeFunctions | null = null;

/**
 * 加载动态库，返回绑定好的原生函数对象。
 * 同一进程只加载一次（单例）。
 */
export function loadLibrary(dllPath: string): NativeFunctions {
  if (_fn) return _fn;

  _lib = koffi.load(dllPath);

  const charPtr  = "const char *";
  const intType  = "int";
  const voidPtr  = "void *";
  const boolPtr  = "void *";  // bool* 用 void* 传递

  _fn = {
    get_downloader: _lib.func("int get_downloader(" +
      `${charPtr} tasksData, ` +   // JSON 字符串
      `${intType} taskCount, ` +
      `${intType} threadCount, ` +
      `${intType} chunkSizeMB, ` +
      `${voidPtr} callback, ` +    // 函数指针（可为 null）
      `bool useCallbackUrl, ` +
      `${charPtr} userAgent, ` +   // 可为 null
      `${charPtr} remoteCallbackUrl, ` + // 可为 null
      `${boolPtr} useSocket` +     // bool*（可为 null）
      ")"
    ),

    start_download: _lib.func("int start_download(" +
      `${charPtr} tasksData, ` +
      `${intType} taskCount, ` +
      `${intType} threadCount, ` +
      `${intType} chunkSizeMB, ` +
      `${voidPtr} callback, ` +
      `bool useCallbackUrl, ` +
      `${charPtr} userAgent, ` +
      `${charPtr} remoteCallbackUrl, ` +
      `${boolPtr} useSocket, ` +
      `${boolPtr} isMultiple` +
      ")"
    ),

    start_download_id:           _lib.func("int start_download_id(int id)"),
    start_multiple_downloads_id: _lib.func("int start_multiple_downloads_id(int id)"),
    pause_download:              _lib.func("int pause_download(int id)"),
    resume_download:             _lib.func("int resume_download(int id)"),
    stop_download:               _lib.func("int stop_download(int id)"),
  };

  return _fn;
}

// ------------------------------------------------------------------
// 回调包装工具
// ------------------------------------------------------------------

/**
 * 将 JS 回调函数包装为 Koffi 可传递的 C 函数指针。
 * 注意：必须保持返回值引用，防止被 GC 回收。
 */
export function makeNativeCallback(
  userCallback: DownloadCallback
): koffi.IKoffiRegisteredCallback {
  return koffi.register(
    (eventPtr: string | null, dataPtr: string | null): void => {
      try {
        const event: DownloadEvent = JSON.parse(eventPtr ?? "{}");
        const data: CallbackData   = JSON.parse(dataPtr  ?? "{}");
        userCallback(event, data);
      } catch (e) {
        // 回调内异常不应崩溃宿主
        console.error("[TTHSD] 回调处理异常:", e);
      }
    },
    koffi.pointer(CALLBACK_PROTO)
  );
}
