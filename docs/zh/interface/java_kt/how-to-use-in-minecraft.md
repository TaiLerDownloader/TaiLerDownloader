# Java/Kotlin 接口封装 在 Minecraft Mod / Plugin 中使用

> [!TIP]
> TTHSD 可作为 Minecraft Mod/Plugin 的下载加速引擎，
> 或集成到第三方启动器中用于资源/模组下载。

---

## 使用场景

- **Mod/Plugin**: 在服务端或客户端 Mod 中加速资源包、模组下载
- **第三方启动器**: 集成到 HMCL、PCL 等启动器中加速游戏文件下载
- **资源管理器**: CurseForge / Modrinth 资源批量下载加速

---

## Gradle 集成

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.google.code.gson:gson:2.10+")
    implementation("net.java.dev.jna:jna:5.13+")
}
```

将 TTHSD 动态库打包到 JAR 的 `resources/native/` 目录：

```
src/main/resources/native/
├── windows-x86_64/tthsd.dll
├── linux-x86_64/libtthsd.so
└── macos-aarch64/libtthsd.dylib
```

`NativeLibraryLoader.kt` 会自动从 JAR 中提取动态库到临时目录。

---

## 在 Mod 中使用

```kotlin
import com.tthsd.TTHSDownloader

class DownloadManager {
    private val downloader = TTHSDownloader()

    fun downloadMod(url: String, savePath: String) {
        val id = downloader.startDownload(
            urls = listOf(url),
            savePaths = listOf(savePath),
            threadCount = 32,
            chunkSizeMB = 10,
            callback = { event, data ->
                when (event.Type) {
                    "update" -> {
                        val pct = (data["Downloaded"] as Double) /
                                  (data["Total"] as Double) * 100
                        logger.info("下载进度: ${"%.1f".format(pct)}%")
                    }
                    "endOne" -> logger.info("下载完成: ${event.ShowName}")
                    "err"    -> logger.error("下载错误: ${data["Error"]}")
                }
            }
        )

        if (id == -1) {
            logger.error("启动下载失败")
        }
    }

    fun close() {
        downloader.close()
    }
}
```

---

## 在第三方启动器中使用

```kotlin
// 批量下载 Minecraft 资源
val dl = TTHSDownloader()

val urls = listOf(
    "https://launcher.mojang.com/v1/objects/abc123/client.jar",
    "https://libraries.minecraft.net/com/example/lib-1.0.jar",
    // ...更多文件
)
val paths = urls.map { url ->
    ".minecraft/libraries/${url.substringAfterLast('/')}"
}

val id = dl.startDownload(
    urls = urls,
    savePaths = paths,
    threadCount = 64,
    isMultiple = true,  // 并行下载
    callback = { event, data ->
        if (event.Type == "update") {
            updateProgressBar(data)
        }
    }
)
```

---

## 注意事项

- **线程安全**：TTHSD 内部使用 Tokio 异步运行时，回调会在 Rust 的异步线程中触发。如果需要更新 GUI，请在回调中使用 `SwingUtilities.invokeLater()` 或 `Platform.runLater()` 切换到 UI 线程
- **JVM 退出**：确保在 JVM 退出前调用 `close()` 释放资源，否则动态库可能不会正常卸载
