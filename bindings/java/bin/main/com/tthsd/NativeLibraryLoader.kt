package com.tthsd

import java.io.File
import java.io.FileOutputStream

/**
 * NativeLibraryLoader
 *
 * 负责自动解析并加载 TTHSD 动态库。
 *
 * 工作流程：
 *   1. 优先检测 TTHSD_LIB_PATH 环境变量
 *   2. 从 JAR 内部 /native/<os>/<arch>/ 提取动态库到临时目录
 *   3. 从当前工作目录、程序目录搜索
 *
 * 这使得将 TTHSD 发布为单一 fat-jar （内嵌动态库）成为可能。
 * 在 Minecraft Mod、Plugin 或第三方启动器中均可直接引用该 jar。
 */
object NativeLibraryLoader {

    private val osName: String = System.getProperty("os.name").lowercase()
    private val archName: String = System.getProperty("os.arch").lowercase()

    /**
     * 动态库文件名（根据 OS + CPU 架构自动选择）
     *
     * TTHSD 命名规则:
     *   桌面: x86_64 → tthsd.*, arm64 → tthsd_arm64.*
     *   Android: tthsd_android_{arm64,armv7,x86_64}.so
     *   HarmonyOS: tthsd_harmony_{arm64,x86_64}.so
     */
    val libFileName: String
        get() {
            val isArm64 = archName.contains("aarch64") || archName.contains("arm64")
            val isArm32 = !isArm64 && archName.contains("arm")

            // Android
            if (osName.contains("android") || System.getProperty("java.vm.vendor")?.lowercase()?.contains("android") == true) {
                return when {
                    isArm64 -> "tthsd_android_arm64.so"
                    isArm32 -> "tthsd_android_armv7.so"
                    else    -> "tthsd_android_x86_64.so"
                }
            }

            // HarmonyOS
            val osVersion = System.getProperty("os.version")?.lowercase() ?: ""
            if (osName.contains("harmony") || osVersion.contains("ohos") || osVersion.contains("harmony")) {
                return if (isArm64) "tthsd_harmony_arm64.so" else "tthsd_harmony_x86_64.so"
            }

            // 桌面系统
            return when {
                osName.contains("win") -> if (isArm64) "tthsd_arm64.dll" else "tthsd.dll"
                osName.contains("mac") -> if (isArm64) "tthsd_arm64.dylib" else "tthsd.dylib"
                else                   -> if (isArm64) "tthsd_arm64.so" else "tthsd.so"
            }
        }

    /** OS 分类标识（对应 JAR 内路径 /native/<osKey>/） */
    private val osKey: String
        get() = when {
            osName.contains("win")   -> "windows"
            osName.contains("mac")   -> "macos"
            osName.contains("android")   ->  "android"
            else                     -> "linux"
        }

    /** CPU 架构（对应 JAR 内路径 /native/<osKey>/<archKey>/） */
    private val archKey: String
        get() = when {
            archName.contains("aarch64") || archName.contains("arm64") -> "arm64"
            archName.contains("arm")                                    -> "arm"
            archName.contains("x86_64") || archName.contains("amd64")  -> "x86_64"
            archName.contains("x86")                                    -> "x86"
            else                                                        -> archName
        }

    /**
     * 解析动态库绝对路径，如有需要会将其从 JAR 内部提取到临时目录。
     *
     * @return 动态库绝对路径（供 JNA.Native.load 使用）
     */
    fun resolve(): String {
        // 1. 环境变量
        val envPath = System.getenv("TTHSD_LIB_PATH")
        if (envPath != null && File(envPath).exists()) return envPath

        // 2. 从 JAR 内部提取
        val extracted = extractFromJar()
        if (extracted != null) return extracted.absolutePath

        // 3. 当前工作目录、主类所在目录
        val candidates = listOf(
            File(System.getProperty("user.dir"), libFileName),
            File(
                NativeLibraryLoader::class.java.protectionDomain
                    ?.codeSource?.location?.toURI()?.let { File(it).parentFile }
                    ?: File("."),
                libFileName
            )
        )

        for (f in candidates) {
            if (f.exists()) return f.absolutePath
        }

        throw UnsatisfiedLinkError(
            "[TTHSD] 未能找到动态库 $libFileName，请设置 TTHSD_LIB_PATH 环境变量，" +
            "或将动态库放置到工作目录中。"
        )
    }

    /**
     * 尝试从 JAR 内部 /native/\<os>/\<arch>/\<libFileName> 提取到 Java 临时目录。
     */
    private fun extractFromJar(): File? {
        val resourcePath = "/native/$osKey/$archKey/$libFileName"
        val inputStream = NativeLibraryLoader::class.java.getResourceAsStream(resourcePath)
            ?: return null

        val tempDir = File(System.getProperty("java.io.tmpdir"), "tthsd_native")
        tempDir.mkdirs()

        val outFile = File(tempDir, libFileName)
        // 若已提取且文件完整则直接复用
        if (outFile.exists() && outFile.length() > 0) return outFile

        FileOutputStream(outFile).use { out ->
            inputStream.copyTo(out)
        }
        return outFile
    }
}
