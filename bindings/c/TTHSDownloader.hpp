/**
 * TTHSDownloader.hpp - TTHSD 高速下载器 C++ 封装类
 *
 * RAII 风格，通过 dlopen/LoadLibrary 动态加载 TTHSD 动态库。
 * 使用 std::function 接收回调，内部使用 nlohmann/json 解析事件 JSON。
 *
 * 依赖:
 *   - nlohmann/json (header-only): https://github.com/nlohmann/json
 *   - C++17 或更高
 *
 * 使用示例:
 * ```cpp
 * TTHSDownloader dl;
 * dl.load();  // 自动搜索 tthsd.dll/libtthsd.so/libtthsd.dylib
 *
 * int id = dl.startDownload(
 *   {"https://example.com/a.zip"},
 *   {"/tmp/a.zip"},
 *   {.threadCount=32},
 *   [](const json& event, const json& data) {
 *       if (event["Type"] == "update")
 *           std::cout << "进度: " << data["Downloaded"] << "/" << data["Total"] << "\n";
 *   }
 * );
 * ```
 */

#pragma once

#include <string>
#include <vector>
#include <functional>
#include <stdexcept>
#include <cstring>
#include <nlohmann/json.hpp>

/*
 * TTHSD 动态库默认文件名（编译期决定）
 *
 * 命名规则:
 *   桌面: x86_64 → tthsd.*, arm64 → tthsd_arm64.*
 *   Android: tthsd_android_{arm64,armv7,x86_64}.so
 *   HarmonyOS: tthsd_harmony_{arm64,x86_64}.so
 */
#ifdef _WIN32
  #include <windows.h>
  #define TTHSD_LIB_OPEN(p)    LoadLibraryA(p)
  #define TTHSD_LIB_SYM(h, s)  GetProcAddress((HMODULE)(h), s)
  #define TTHSD_LIB_CLOSE(h)   FreeLibrary((HMODULE)(h))
  #if defined(_M_ARM64) || defined(__aarch64__)
    #define TTHSD_DEFAULT_LIB  "tthsd_arm64.dll"
  #else
    #define TTHSD_DEFAULT_LIB  "tthsd.dll"
  #endif
#elif defined(__ANDROID__)
  #include <dlfcn.h>
  #define TTHSD_LIB_OPEN(p)    dlopen(p, RTLD_LAZY)
  #define TTHSD_LIB_SYM(h, s)  dlsym(h, s)
  #define TTHSD_LIB_CLOSE(h)   dlclose(h)
  #if defined(__aarch64__)
    #define TTHSD_DEFAULT_LIB  "tthsd_android_arm64.so"
  #elif defined(__arm__)
    #define TTHSD_DEFAULT_LIB  "tthsd_android_armv7.so"
  #else
    #define TTHSD_DEFAULT_LIB  "tthsd_android_x86_64.so"
  #endif
#elif defined(__OHOS__)
  #include <dlfcn.h>
  #define TTHSD_LIB_OPEN(p)    dlopen(p, RTLD_LAZY)
  #define TTHSD_LIB_SYM(h, s)  dlsym(h, s)
  #define TTHSD_LIB_CLOSE(h)   dlclose(h)
  #if defined(__aarch64__)
    #define TTHSD_DEFAULT_LIB  "tthsd_harmony_arm64.so"
  #else
    #define TTHSD_DEFAULT_LIB  "tthsd_harmony_x86_64.so"
  #endif
#else
  #include <dlfcn.h>
  #define TTHSD_LIB_OPEN(p)    dlopen(p, RTLD_LAZY)
  #define TTHSD_LIB_SYM(h, s)  dlsym(h, s)
  #define TTHSD_LIB_CLOSE(h)   dlclose(h)
  #ifdef __APPLE__
    #if defined(__aarch64__)
      #define TTHSD_DEFAULT_LIB  "tthsd_arm64.dylib"
    #else
      #define TTHSD_DEFAULT_LIB  "tthsd.dylib"
    #endif
  #else
    #if defined(__aarch64__)
      #define TTHSD_DEFAULT_LIB  "tthsd_arm64.so"
    #else
      #define TTHSD_DEFAULT_LIB  "tthsd.so"
    #endif
  #endif
#endif

using json = nlohmann::json;

/// 回调函数类型
using DownloadCallback = std::function<void(const json& event, const json& data)>;

struct DownloadParams {
    int threadCount     = 64;
    int chunkSizeMB     = 10;
    bool useCallbackUrl = false;
    std::string userAgent;
    std::string remoteCallbackUrl;
    bool* useSocket   = nullptr;
    bool* isMultiple  = nullptr;
};

class TTHSDownloader {
public:
    TTHSDownloader() = default;

    ~TTHSDownloader() {
        if (_handle) TTHSD_LIB_CLOSE(_handle);
    }

    // 禁止拷贝
    TTHSDownloader(const TTHSDownloader&) = delete;
    TTHSDownloader& operator=(const TTHSDownloader&) = delete;

    /// 加载动态库（空路径则自动搜索）
    void load(const std::string& libPath = "") {
        std::string path = libPath.empty() ? TTHSD_DEFAULT_LIB : libPath;
        _handle = TTHSD_LIB_OPEN(path.c_str());
        if (!_handle)
            throw std::runtime_error("[TTHSD] 无法加载动态库: " + path);

        #define LOAD(name, type) \
            _fn_##name = reinterpret_cast<type>(TTHSD_LIB_SYM(_handle, #name)); \
            if (!_fn_##name) throw std::runtime_error("[TTHSD] 符号未找到: " #name);

        LOAD(start_download,            StartDownloadFn)
        LOAD(get_downloader,            GetDownloaderFn)
        LOAD(start_download_id,         IntIntFn)
        LOAD(start_multiple_downloads_id, IntIntFn)
        LOAD(pause_download,            IntIntFn)
        LOAD(resume_download,           IntIntFn)
        LOAD(stop_download,             IntIntFn)
        #undef LOAD
        _loaded = true;
    }

    /// 创建并立即启动下载
    int startDownload(
        const std::vector<std::string>& urls,
        const std::vector<std::string>& savePaths,
        DownloadParams params = {},
        DownloadCallback callback = nullptr
    ) {
        assertLoaded();
        auto tasksJson = buildTasksJson(urls, savePaths);
        _callback = std::move(callback);

        return _fn_start_download(
            tasksJson.c_str(), (int)urls.size(),
            params.threadCount, params.chunkSizeMB,
            _callback ? reinterpret_cast<void*>(&TTHSDownloader::cCallback) : nullptr,
            params.useCallbackUrl,
            params.userAgent.empty() ? nullptr : params.userAgent.c_str(),
            params.remoteCallbackUrl.empty() ? nullptr : params.remoteCallbackUrl.c_str(),
            params.useSocket,
            params.isMultiple
        );
    }

    /// 创建下载器（不立即启动）
    int getDownloader(
        const std::vector<std::string>& urls,
        const std::vector<std::string>& savePaths,
        DownloadParams params = {},
        DownloadCallback callback = nullptr
    ) {
        assertLoaded();
        auto tasksJson = buildTasksJson(urls, savePaths);
        _callback = std::move(callback);

        return _fn_get_downloader(
            tasksJson.c_str(), (int)urls.size(),
            params.threadCount, params.chunkSizeMB,
            _callback ? reinterpret_cast<void*>(&TTHSDownloader::cCallback) : nullptr,
            params.useCallbackUrl,
            params.userAgent.empty() ? nullptr : params.userAgent.c_str(),
            params.remoteCallbackUrl.empty() ? nullptr : params.remoteCallbackUrl.c_str(),
            params.useSocket
        );
    }

    bool startDownloadById(int id)          { assertLoaded(); return _fn_start_download_id(id)           == 0; }
    bool startMultipleDownloadsById(int id) { assertLoaded(); return _fn_start_multiple_downloads_id(id) == 0; }
    bool pauseDownload(int id)              { assertLoaded(); return _fn_pause_download(id)               == 0; }
    bool resumeDownload(int id)             { assertLoaded(); return _fn_resume_download(id)              == 0; }
    bool stopDownload(int id)               { assertLoaded(); return _fn_stop_download(id)                == 0; }

private:
    void*  _handle = nullptr;
    bool   _loaded = false;
    DownloadCallback _callback;

    // 函数指针类型别名
    using StartDownloadFn  = int(*)(const char*, int, int, int, void*, bool, const char*, const char*, const bool*, const bool*);
    using GetDownloaderFn  = int(*)(const char*, int, int, int, void*, bool, const char*, const char*, const bool*);
    using IntIntFn         = int(*)(int);

    StartDownloadFn  _fn_start_download              = nullptr;
    GetDownloaderFn  _fn_get_downloader              = nullptr;
    IntIntFn         _fn_start_download_id           = nullptr;
    IntIntFn         _fn_start_multiple_downloads_id = nullptr;
    IntIntFn         _fn_pause_download              = nullptr;
    IntIntFn         _fn_resume_download             = nullptr;
    IntIntFn         _fn_stop_download               = nullptr;

    void assertLoaded() const {
        if (!_loaded) throw std::runtime_error("[TTHSD] 未调用 load()");
    }

    std::string buildTasksJson(
        const std::vector<std::string>& urls,
        const std::vector<std::string>& savePaths
    ) {
        json tasks = json::array();
        for (size_t i = 0; i < urls.size(); ++i) {
            std::string showName = urls[i].substr(urls[i].find_last_of('/') + 1);
            tasks.push_back({
                {"url",       urls[i]},
                {"save_path", savePaths[i]},
                {"show_name", showName.empty() ? ("task_" + std::to_string(i)) : showName},
                {"id",        std::to_string(i)}  // 简化版 ID
            });
        }
        return tasks.dump();
    }

    // 静态 C 回调（转发给实例的 _callback）
    // 简化版：单实例场景
    static TTHSDownloader* _instance;

    static void cCallback(const char* eventJson, const char* dataJson) {
        if (!_instance || !_instance->_callback) return;
        try {
            auto event = json::parse(eventJson ? eventJson : "{}");
            auto data  = json::parse(dataJson  ? dataJson  : "{}");
            _instance->_callback(event, data);
        } catch (...) {}
    }
};

inline TTHSDownloader* TTHSDownloader::_instance = nullptr;
