//! FFI 层：通过 libloading 在运行时加载 TTHSD 动态库并绑定 C ABI 符号

use libloading::{Library, Symbol};
use std::ffi::{c_bool, c_char, c_int, c_void};
use std::path::Path;

/// C 回调函数类型：`void callback(const char* event_json, const char* data_json)`
pub type RawCallback = unsafe extern "C" fn(*const c_char, *const c_char);

/// 与 export.rs 对应的全套原始函数指针持有者
pub struct TthsdRaw {
    /// 保持库句柄存活（Drop 时卸载库）
    _lib: Library,

    pub fn_get_downloader:              unsafe fn(*const c_char, c_int, c_int, c_int, *mut c_void, bool, *const c_char, *const c_char, *const c_bool) -> c_int,
    pub fn_start_download:             unsafe fn(*const c_char, c_int, c_int, c_int, *mut c_void, bool, *const c_char, *const c_char, *const c_bool, *const c_bool) -> c_int,
    pub fn_start_download_id:          unsafe fn(c_int) -> c_int,
    pub fn_start_multiple_downloads_id: unsafe fn(c_int) -> c_int,
    pub fn_pause_download:             unsafe fn(c_int) -> c_int,
    pub fn_resume_download:            unsafe fn(c_int) -> c_int,
    pub fn_stop_download:              unsafe fn(c_int) -> c_int,
}

impl TthsdRaw {
    /// 从指定路径加载 TTHSD 动态库
    pub fn load(lib_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        // SAFETY: 加载外部 C 动态库，所有后续调用都在 unsafe 块中进行
        let lib = unsafe { Library::new(lib_path)? };

        macro_rules! sym {
            ($lib:expr, $name:literal, $ty:ty) => {{
                let sym: Symbol<$ty> = unsafe { $lib.get($name)? };
                *sym
            }};
        }

        let raw = TthsdRaw {
            fn_get_downloader:              sym!(lib, b"get_downloader\0",              unsafe fn(*const c_char, c_int, c_int, c_int, *mut c_void, bool, *const c_char, *const c_char, *const c_bool) -> c_int),
            fn_start_download:             sym!(lib, b"start_download\0",             unsafe fn(*const c_char, c_int, c_int, c_int, *mut c_void, bool, *const c_char, *const c_char, *const c_bool, *const c_bool) -> c_int),
            fn_start_download_id:          sym!(lib, b"start_download_id\0",          unsafe fn(c_int) -> c_int),
            fn_start_multiple_downloads_id: sym!(lib, b"start_multiple_downloads_id\0", unsafe fn(c_int) -> c_int),
            fn_pause_download:             sym!(lib, b"pause_download\0",             unsafe fn(c_int) -> c_int),
            fn_resume_download:            sym!(lib, b"resume_download\0",            unsafe fn(c_int) -> c_int),
            fn_stop_download:              sym!(lib, b"stop_download\0",              unsafe fn(c_int) -> c_int),
            _lib: lib,
        };

        Ok(raw)
    }

    /// 根据操作系统和 CPU 架构返回默认动态库文件名
    ///
    /// TTHSD 命名规则:
    ///   桌面: x86_64 → tthsd.*, arm64 → tthsd_arm64.*
    ///   Android: tthsd_android_{arm64,armv7,x86_64}.so
    ///   HarmonyOS: tthsd_harmony_{arm64,x86_64}.so
    pub fn default_lib_name() -> &'static str {
        #[cfg(all(target_os = "android", target_arch = "aarch64"))]   { "tthsd_android_arm64.so"  }
        #[cfg(all(target_os = "android", target_arch = "arm"))]       { "tthsd_android_armv7.so"  }
        #[cfg(all(target_os = "android", not(any(target_arch = "aarch64", target_arch = "arm"))))]
                                                                      { "tthsd_android_x86_64.so" }

        #[cfg(all(target_os = "windows", target_arch = "aarch64"))]   { "tthsd_arm64.dll"    }
        #[cfg(all(target_os = "windows", not(target_arch = "aarch64")))] { "tthsd.dll"        }

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]     { "tthsd_arm64.dylib"  }
        #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))] { "tthsd.dylib"       }

        #[cfg(all(not(any(target_os = "windows", target_os = "macos", target_os = "android")),
                  target_arch = "aarch64"))]                          { "tthsd_arm64.so"     }
        #[cfg(all(not(any(target_os = "windows", target_os = "macos", target_os = "android")),
                  not(target_arch = "aarch64")))]                     { "tthsd.so"           }
    }
}
