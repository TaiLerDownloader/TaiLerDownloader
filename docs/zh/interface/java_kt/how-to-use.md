# Java / Kotlin 接口封装使用文档

> [!TIP]
> 本文档介绍的是 **TTHSD Next**（Rust 版本）的 Java / Kotlin 接口。
> 源码位于 [`bindings/java/`](https://github.com/TTHSDownloader/TTHSDNext/tree/main/bindings/java)

---

## 架构概览

```
┌──────────────────┐
│ TTHSDownloader        │  ← 用户使用的高层 API
├──────────────────┤
│ JNA (桌面端 )             │  TTHSDLibraryJNA.kt
│ JNI (Android)            │  TTHSDLibraryJNI.kt
├──────────────────┤
│ tthsd.dll/so             │  ← Rust 编译的动态库
└──────────────────┘
```

- **桌面端 (Windows/Linux/macOS)**：通过 **JNA** 加载 `tthsd.dll` / `tthsd.so` / `tthsd.dylib`
- **Android/HarmonyOS**：通过 **JNI** 调用 `tthsd_android.so` / `tthsd_harmonyos.so`

---

## 快速开始 (Kotlin)

```kotlin
val dl = TTHSDownloader()  // 自动从 JAR 提取或搜索动态库

val id = dl.startDownload(
    urls = listOf("https://example.com/a.zip"),
    savePaths = listOf("/tmp/a.zip"),
    threadCount = 32,
    callback = { event, data ->
        when (event.Type) {
            "update" -> {
                val pct = (data["Downloaded"] as Double) /
                          (data["Total"] as Double) * 100
                print("\r进度: ${"%.1f".format(pct)}%")
            }
            "endOne" -> println("\n完成: ${event.ShowName}")
            "err"    -> println("\n错误: ${data["Error"]}")
        }
    }
)

println("下载 ID: $id")
```

---

## API 参考

### `TTHSDownloader`

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `startDownload(urls, savePaths, ...)` | `Int` | 创建并启动下载 |
| `getDownloader(urls, savePaths, ...)` | `Int` | 创建不启动 |
| `startDownloadById(id)` | `Boolean` | 顺序启动 |
| `startMultipleDownloadsById(id)` | `Boolean` | 并行启动 |
| `pauseDownload(id)` | `Boolean` | 暂停 |
| `resumeDownload(id)` | `Boolean` | 恢复 |
| `stopDownload(id)` | `Boolean` | 停止并销毁 |
| `close()` | — | 释放资源 (`AutoCloseable`) |

### `startDownload` 完整参数

```kotlin
fun startDownload(
    urls: List<String>,
    savePaths: List<String>,
    threadCount: Int = 64,
    chunkSizeMB: Int = 10,
    callback: DownloadCallback? = null,
    useCallbackUrl: Boolean = false,
    userAgent: String? = null,
    remoteCallbackUrl: String? = null,
    useSocket: Boolean? = null,
    isMultiple: Boolean? = null,
    showNames: List<String>? = null,
    ids: List<String>? = null
): Int
```

### 回调类型

```kotlin
data class DownloadEvent(
    val Type: String,
    val Name: String?,
    val ShowName: String?,
    val ID: String?
)

typealias DownloadCallback =
    (event: DownloadEvent, data: Map<String, Any?>) -> Unit
```

---

## Gradle 依赖

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.google.code.gson:gson:2.10+")
    implementation("net.java.dev.jna:jna:5.13+")
}
```

---

## GC 安全

封装类内部维护 `callbackRefs: MutableMap<Int, ProgressCallback>`，持有所有 JNA 回调引用。**务必在下载完成后调用 `stopDownload()` 或 `close()`**，否则回调引用不会被释放。

**下一步**

- 查看 [Java/Kotlin 接口怎么在 Minecraft Mod / Plugin 中使用](/zh/interface/java_kt/how-to-use-in-minecraft)
- 查看 [Java/Kotlin 接口怎么在 Android / HarmonyOS 中使用](/zh/interface/java_kt/how-to-use-in-AndroidAndHarmonyOS)
