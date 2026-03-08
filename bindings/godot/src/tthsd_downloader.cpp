#include "tthsd_downloader.h"

#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/utility_functions.hpp>

/*
 * TTHSD 动态库默认文件名（编译期决定）
 * 命名规则: 桌面 x86_64→tthsd.*, arm64→tthsd_arm64.*
 *          Android: tthsd_android_{arm64,armv7,x86_64}.so
 *          HarmonyOS: tthsd_harmony_{arm64,x86_64}.so
 */
#ifdef _WIN32
  #include <windows.h>
  #define TTHSD_OPEN_LIB(path)  LoadLibraryA(path)
  #define TTHSD_GET_SYM(h, sym) GetProcAddress((HMODULE)(h), sym)
  #define TTHSD_CLOSE_LIB(h)    FreeLibrary((HMODULE)(h))
  #if defined(_M_ARM64) || defined(__aarch64__)
    #define TTHSD_DEFAULT_LIB   "tthsd_arm64.dll"
  #else
    #define TTHSD_DEFAULT_LIB   "tthsd.dll"
  #endif
#elif defined(__ANDROID__)
  #include <dlfcn.h>
  #define TTHSD_OPEN_LIB(path)  dlopen(path, RTLD_LAZY)
  #define TTHSD_GET_SYM(h, sym) dlsym(h, sym)
  #define TTHSD_CLOSE_LIB(h)    dlclose(h)
  #if defined(__aarch64__)
    #define TTHSD_DEFAULT_LIB   "tthsd_android_arm64.so"
  #elif defined(__arm__)
    #define TTHSD_DEFAULT_LIB   "tthsd_android_armv7.so"
  #else
    #define TTHSD_DEFAULT_LIB   "tthsd_android_x86_64.so"
  #endif
#elif defined(__OHOS__)
  #include <dlfcn.h>
  #define TTHSD_OPEN_LIB(path)  dlopen(path, RTLD_LAZY)
  #define TTHSD_GET_SYM(h, sym) dlsym(h, sym)
  #define TTHSD_CLOSE_LIB(h)    dlclose(h)
  #if defined(__aarch64__)
    #define TTHSD_DEFAULT_LIB   "tthsd_harmony_arm64.so"
  #else
    #define TTHSD_DEFAULT_LIB   "tthsd_harmony_x86_64.so"
  #endif
#else
  #include <dlfcn.h>
  #define TTHSD_OPEN_LIB(path)  dlopen(path, RTLD_LAZY)
  #define TTHSD_GET_SYM(h, sym) dlsym(h, sym)
  #define TTHSD_CLOSE_LIB(h)    dlclose(h)
  #ifdef __APPLE__
    #if defined(__aarch64__)
      #define TTHSD_DEFAULT_LIB "tthsd_arm64.dylib"
    #else
      #define TTHSD_DEFAULT_LIB "tthsd.dylib"
    #endif
  #else
    #if defined(__aarch64__)
      #define TTHSD_DEFAULT_LIB "tthsd_arm64.so"
    #else
      #define TTHSD_DEFAULT_LIB "tthsd.so"
    #endif
  #endif
#endif

#include <nlohmann/json.hpp>
#include <string>
#include <uuid/uuid.h>

using json = nlohmann::json;

namespace godot {

TTHSDownloader* TTHSDownloader::_instance = nullptr;

TTHSDownloader::TTHSDownloader() {
    _instance = this;
}

TTHSDownloader::~TTHSDownloader() {
    if (_lib_handle) {
        TTHSD_CLOSE_LIB(_lib_handle);
        _lib_handle = nullptr;
    }
    if (_instance == this) _instance = nullptr;
}

bool TTHSDownloader::load_library(const String& lib_path) {
    std::string path = lib_path.is_empty()
        ? TTHSD_DEFAULT_LIB
        : lib_path.utf8().get_data();

    _lib_handle = TTHSD_OPEN_LIB(path.c_str());
    if (!_lib_handle) {
        UtilityFunctions::printerr("[TTHSD] 无法加载动态库: ", lib_path);
        return false;
    }

#define LOAD_SYM(var, type, name)                                          \
    var = reinterpret_cast<type>(TTHSD_GET_SYM(_lib_handle, name));       \
    if (!var) {                                                             \
        UtilityFunctions::printerr("[TTHSD] 找不到符号: " #name);           \
        return false;                                                       \
    }

    LOAD_SYM(_fn_get_downloader,  FnGetDownloader,           "get_downloader")
    LOAD_SYM(_fn_start_download,  FnStartDownload,           "start_download")
    LOAD_SYM(_fn_start_download_id, FnStartDownloadId,       "start_download_id")
    LOAD_SYM(_fn_start_multiple,  FnStartMultipleDownloadsId,"start_multiple_downloads_id")
    LOAD_SYM(_fn_pause_download,  FnPauseDownload,           "pause_download")
    LOAD_SYM(_fn_resume_download, FnResumeDownload,          "resume_download")
    LOAD_SYM(_fn_stop_download,   FnStopDownload,            "stop_download")

#undef LOAD_SYM

    _loaded = true;
    return true;
}

String TTHSDownloader::_build_tasks_json(
    const TypedArray<String>& urls,
    const TypedArray<String>& save_paths
) {
    json tasks = json::array();
    for (int i = 0; i < urls.size(); ++i) {
        std::string url = static_cast<String>(urls[i]).utf8().get_data();
        std::string save_path = static_cast<String>(save_paths[i]).utf8().get_data();
        std::string show_name = url.substr(url.find_last_of('/') + 1);

        // Generate a simple UUID-like ID
        char uuid_str[37];
        uuid_t uuid;
        uuid_generate_random(uuid);
        uuid_unparse(uuid, uuid_str);

        tasks.push_back({
            {"url",       url},
            {"save_path", save_path},
            {"show_name", show_name.empty() ? ("task_" + std::to_string(i)) : show_name},
            {"id",        std::string(uuid_str)}
        });
    }
    return String(tasks.dump().c_str());
}

// 静态 C 回调，将 JSON 转为 Godot Signal 并分发
void TTHSDownloader::_c_callback(const char* event_json, const char* data_json) {
    if (!_instance) return;

    try {
        auto event = json::parse(event_json ? event_json : "{}");
        auto data  = json::parse(data_json  ? data_json  : "{}");

        String event_type = String(event.value("Type", "").c_str());
        Dictionary event_dict, data_dict;

        // 填充 event_dict
        event_dict["Type"]     = String(event.value("Type", "").c_str());
        event_dict["Name"]     = String(event.value("Name", "").c_str());
        event_dict["ShowName"] = String(event.value("ShowName", "").c_str());
        event_dict["ID"]       = String(event.value("ID", "").c_str());

        // 填充 data_dict
        for (auto& [k, v] : data.items()) {
            String key = String(k.c_str());
            if (v.is_number_integer())   data_dict[key] = (int64_t)v;
            else if (v.is_number_float()) data_dict[key] = (double)v;
            else if (v.is_string())      data_dict[key] = String(v.get<std::string>().c_str());
            else if (v.is_boolean())     data_dict[key] = (bool)v;
        }

        // 根据事件类型分发不同 Signal
        if (event_type == "update") {
            _instance->emit_signal("on_progress", event_dict, data_dict);
        } else if (event_type == "err") {
            _instance->emit_signal("on_error", event_dict, data_dict);
        } else if (event_type == "end" || event_type == "endOne") {
            _instance->emit_signal("on_finished", event_dict, data_dict);
        } else {
            _instance->emit_signal("on_event", event_dict, data_dict);
        }
    } catch (...) {
        UtilityFunctions::printerr("[TTHSD] 回调 JSON 解析失败");
    }
}

int TTHSDownloader::start_download(
    TypedArray<String> urls,
    TypedArray<String> save_paths,
    int thread_count,
    int chunk_size_mb
) {
    if (!_loaded) { UtilityFunctions::printerr("[TTHSD] 库未加载"); return -1; }
    String tasks_json = _build_tasks_json(urls, save_paths);
    return _fn_start_download(
        tasks_json.utf8().get_data(),
        (int)urls.size(), thread_count, chunk_size_mb,
        reinterpret_cast<void*>(&TTHSDownloader::_c_callback),
        false, nullptr, nullptr, nullptr, nullptr
    );
}

int TTHSDownloader::get_downloader(
    TypedArray<String> urls,
    TypedArray<String> save_paths,
    int thread_count,
    int chunk_size_mb
) {
    if (!_loaded) { UtilityFunctions::printerr("[TTHSD] 库未加载"); return -1; }
    String tasks_json = _build_tasks_json(urls, save_paths);
    return _fn_get_downloader(
        tasks_json.utf8().get_data(),
        (int)urls.size(), thread_count, chunk_size_mb,
        reinterpret_cast<void*>(&TTHSDownloader::_c_callback),
        false, nullptr, nullptr, nullptr
    );
}

bool TTHSDownloader::start_download_by_id(int id)           { return _fn_start_download_id(id)  == 0; }
bool TTHSDownloader::start_multiple_downloads_by_id(int id) { return _fn_start_multiple(id)     == 0; }
bool TTHSDownloader::pause_download(int id)                  { return _fn_pause_download(id)     == 0; }
bool TTHSDownloader::resume_download(int id)                 { return _fn_resume_download(id)    == 0; }
bool TTHSDownloader::stop_download(int id)                   { return _fn_stop_download(id)      == 0; }

void TTHSDownloader::_bind_methods() {
    ClassDB::bind_method(D_METHOD("load_library", "lib_path"),    &TTHSDownloader::load_library, DEFVAL(""));
    ClassDB::bind_method(D_METHOD("start_download", "urls", "save_paths", "thread_count", "chunk_size_mb"),
                         &TTHSDownloader::start_download, DEFVAL(64), DEFVAL(10));
    ClassDB::bind_method(D_METHOD("get_downloader", "urls", "save_paths", "thread_count", "chunk_size_mb"),
                         &TTHSDownloader::get_downloader, DEFVAL(64), DEFVAL(10));
    ClassDB::bind_method(D_METHOD("start_download_by_id", "id"),          &TTHSDownloader::start_download_by_id);
    ClassDB::bind_method(D_METHOD("start_multiple_downloads_by_id", "id"), &TTHSDownloader::start_multiple_downloads_by_id);
    ClassDB::bind_method(D_METHOD("pause_download", "id"),                 &TTHSDownloader::pause_download);
    ClassDB::bind_method(D_METHOD("resume_download", "id"),                &TTHSDownloader::resume_download);
    ClassDB::bind_method(D_METHOD("stop_download", "id"),                  &TTHSDownloader::stop_download);

    // Signals
    ADD_SIGNAL(MethodInfo("on_progress",
        PropertyInfo(Variant::DICTIONARY, "event"),
        PropertyInfo(Variant::DICTIONARY, "data")));
    ADD_SIGNAL(MethodInfo("on_error",
        PropertyInfo(Variant::DICTIONARY, "event"),
        PropertyInfo(Variant::DICTIONARY, "data")));
    ADD_SIGNAL(MethodInfo("on_finished",
        PropertyInfo(Variant::DICTIONARY, "event"),
        PropertyInfo(Variant::DICTIONARY, "data")));
    ADD_SIGNAL(MethodInfo("on_event",
        PropertyInfo(Variant::DICTIONARY, "event"),
        PropertyInfo(Variant::DICTIONARY, "data")));
}

} // namespace godot
