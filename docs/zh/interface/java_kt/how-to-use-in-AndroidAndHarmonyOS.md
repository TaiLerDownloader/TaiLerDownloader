# Java/Kotlin 接口封装 在 Android / HarmonyOS 中使用

> [!TIP]
> Android 和 HarmonyOS 使用 **JNI** 接口（而非桌面端的 JNA），对应 Rust 源码 `src/core/android_export.rs`。

---

## 初始化

在 `Application.onCreate()` 中加载动态库：

```kotlin
class MyApp : Application() {
    override fun onCreate() {
        super.onCreate()
        TTHSDLibraryJNI.load()  // System.loadLibrary("TTHSD")
    }
}
```

---

## 使用

Android 端**不支持函数指针回调**，需通过远程回调 URL（WebSocket 或 Socket）接收事件。

```kotlin
val id = TTHSDLibraryJNI.startDownload(
    tasksJson = """[{
        "url": "https://example.com/file.apk",
        "save_path": "/data/data/com.example/files/file.apk",
        "show_name": "file.apk",
        "id": "uuid-1"
    }]""",
    threadCount = 16,
    chunkSizeMB = 10,
    useCallbackUrl = true,
    callbackUrl = "ws://localhost:8080/tthsd",
    useSocket = false,
    isMultiple = false
)

if (id == -1) {
    Log.e("TTHSD", "启动下载失败")
}
```

---

## JNI API

| 方法 | 参数 | 返回值 |
|------|------|--------|
| `startDownload(tasksJson, threadCount, chunkSizeMB, useCallbackUrl, callbackUrl, useSocket, isMultiple)` | 见上 | `Int` |
| `getDownloader(tasksJson, threadCount, chunkSizeMB, useCallbackUrl, callbackUrl, useSocket)` | 类似 | `Int` |
| `startDownloadById(id)` | `Int` | `Int` (0=成功) |
| `startMultipleDownloadsById(id)` | `Int` | `Int` |
| `pauseDownload(id)` | `Int` | `Int` |
| `resumeDownload(id)` | `Int` | `Int` |
| `stopDownload(id)` | `Int` | `Int` |

---

## 动态库放置

- **Android**: 将 `libtthsd.so` 放到 `src/main/jniLibs/<abi>/` 目录
  - `arm64-v8a/libtthsd.so`
  - `armeabi-v7a/libtthsd.so`
- **HarmonyOS**: 按鸿蒙 Native 模块规范放置

---

## 与桌面端的区别

| 特性 | 桌面 (JNA) | Android/HarmonyOS (JNI) |
|------|-----------|------------------------|
| 加载方式 | `Native.load()` | `System.loadLibrary()` |
| 回调 | 函数指针 | 远程回调 URL |
| 接口类 | `TTHSDLibraryJNA` | `TTHSDLibraryJNI` |
| 封装类 | `TTHSDownloader` | `TTHSDownloaderAndroid`（可选） |
