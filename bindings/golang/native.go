package tthsd

/*
#cgo linux LDFLAGS: -ldl
#cgo darwin LDFLAGS: -ldl

#include <stdlib.h>
#include <stdbool.h>
#include <dlfcn.h>

#ifdef _WIN32
  #include <windows.h>
#endif

// ---- C ABI 函数指针类型 ----

typedef void (*tthsd_callback_t)(const char* event_json, const char* data_json);

typedef int (*fn_start_download_t)(
    const char* tasks_data, int task_count,
    int thread_count, int chunk_size_mb,
    tthsd_callback_t callback,
    _Bool use_callback_url,
    const char* user_agent,
    const char* remote_callback_url,
    const _Bool* use_socket,
    const _Bool* is_multiple
);

typedef int (*fn_get_downloader_t)(
    const char* tasks_data, int task_count,
    int thread_count, int chunk_size_mb,
    tthsd_callback_t callback,
    _Bool use_callback_url,
    const char* user_agent,
    const char* remote_callback_url,
    const _Bool* use_socket
);

typedef int (*fn_int_int_t)(int id);

// ---- 库加载 ----

static void* tthsd_open(const char* path) {
#ifdef _WIN32
    return (void*)LoadLibraryA(path);
#else
    return dlopen(path, RTLD_LAZY);
#endif
}

static void* tthsd_sym(void* handle, const char* name) {
#ifdef _WIN32
    return (void*)GetProcAddress((HMODULE)handle, name);
#else
    return dlsym(handle, name);
#endif
}

static void tthsd_close(void* handle) {
#ifdef _WIN32
    FreeLibrary((HMODULE)handle);
#else
    dlclose(handle);
#endif
}

// ---- 函数调用包装器 (CGo 不能直接调用函数指针) ----

static int call_start_download(void* fp,
    const char* tasks, int count, int threads, int chunk,
    tthsd_callback_t cb, _Bool use_cb_url,
    const char* ua, const char* cb_url,
    const _Bool* use_socket, const _Bool* is_multiple)
{
    return ((fn_start_download_t)fp)(
        tasks, count, threads, chunk,
        cb, use_cb_url, ua, cb_url,
        use_socket, is_multiple
    );
}

static int call_get_downloader(void* fp,
    const char* tasks, int count, int threads, int chunk,
    tthsd_callback_t cb, _Bool use_cb_url,
    const char* ua, const char* cb_url,
    const _Bool* use_socket)
{
    return ((fn_get_downloader_t)fp)(
        tasks, count, threads, chunk,
        cb, use_cb_url, ua, cb_url,
        use_socket
    );
}

static int call_int_int(void* fp, int id) {
    return ((fn_int_int_t)fp)(id);
}

// ---- Go 回调桥接 (声明在 Go 中实现) ----
extern void goCallbackBridge(const char* event_json, const char* data_json);
*/
import "C"

import (
	"fmt"
	"runtime"
	"unsafe"
)

// nativeLib 持有动态库句柄和所有函数指针
type nativeLib struct {
	handle                   unsafe.Pointer
	fnStartDownload          unsafe.Pointer
	fnGetDownloader          unsafe.Pointer
	fnStartDownloadID        unsafe.Pointer
	fnStartMultipleDownloads unsafe.Pointer
	fnPauseDownload          unsafe.Pointer
	fnResumeDownload         unsafe.Pointer
	fnStopDownload           unsafe.Pointer
}

// defaultLibName 根据操作系统和 CPU 架构返回默认动态库文件名。
//
// TTHSD 命名规则:
//   桌面: x86_64 → tthsd.*, arm64 → tthsd_arm64.*
//   Android: tthsd_android_{arm64,armv7,x86_64}.so
//   HarmonyOS: tthsd_harmony_{arm64,x86_64}.so
func defaultLibName() string {
	arch := runtime.GOARCH // amd64, arm64, arm, etc.

	switch runtime.GOOS {
	case "android":
		switch arch {
		case "arm64":
			return "tthsd_android_arm64.so"
		case "arm":
			return "tthsd_android_armv7.so"
		default: // amd64
			return "tthsd_android_x86_64.so"
		}

	case "windows":
		if arch == "arm64" {
			return "tthsd_arm64.dll"
		}
		return "tthsd.dll"

	case "darwin":
		if arch == "arm64" {
			return "tthsd_arm64.dylib"
		}
		return "tthsd.dylib"

	default: // linux 和其它
		// HarmonyOS 也是 linux，但 runtime.GOOS 无法区分，
		// 鸿蒙环境请通过参数显式指定库路径。
		if arch == "arm64" {
			return "tthsd_arm64.so"
		}
		return "tthsd.so"
	}
}

// loadNativeLib 加载 TTHSD 动态库并解析所有符号
func loadNativeLib(path string) (*nativeLib, error) {
	if path == "" {
		path = defaultLibName()
	}

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	handle := C.tthsd_open(cPath)
	if handle == nil {
		return nil, fmt.Errorf("[TTHSD] 无法加载动态库: %s", path)
	}

	lib := &nativeLib{handle: handle}

	// 加载所有符号
	syms := map[string]*unsafe.Pointer{
		"start_download":             &lib.fnStartDownload,
		"get_downloader":             &lib.fnGetDownloader,
		"start_download_id":          &lib.fnStartDownloadID,
		"start_multiple_downloads_id": &lib.fnStartMultipleDownloads,
		"pause_download":             &lib.fnPauseDownload,
		"resume_download":            &lib.fnResumeDownload,
		"stop_download":              &lib.fnStopDownload,
	}

	for name, ptr := range syms {
		cName := C.CString(name)
		sym := C.tthsd_sym(handle, cName)
		C.free(unsafe.Pointer(cName))

		if sym == nil {
			C.tthsd_close(handle)
			return nil, fmt.Errorf("[TTHSD] 符号未找到: %s", name)
		}
		*ptr = sym
	}

	return lib, nil
}

func (lib *nativeLib) close() {
	if lib.handle != nil {
		C.tthsd_close(lib.handle)
		lib.handle = nil
	}
}

func (lib *nativeLib) callStartDownload(
	tasksJSON string, count, threads, chunk int,
	useCallbackURL bool, userAgent, callbackURL *string,
	useSocket, isMultiple *bool,
) int {
	cTasks := C.CString(tasksJSON)
	defer C.free(unsafe.Pointer(cTasks))

	var cUA, cCBURL *C.char
	if userAgent != nil {
		cUA = C.CString(*userAgent)
		defer C.free(unsafe.Pointer(cUA))
	}
	if callbackURL != nil {
		cCBURL = C.CString(*callbackURL)
		defer C.free(unsafe.Pointer(cCBURL))
	}

	var cUseSocket, cIsMultiple *C.bool
	if useSocket != nil {
		v := C.bool(*useSocket)
		cUseSocket = &v
	}
	if isMultiple != nil {
		v := C.bool(*isMultiple)
		cIsMultiple = &v
	}

	return int(C.call_start_download(
		lib.fnStartDownload,
		cTasks, C.int(count), C.int(threads), C.int(chunk),
		C.tthsd_callback_t(C.goCallbackBridge),
		C.bool(useCallbackURL),
		cUA, cCBURL,
		cUseSocket, cIsMultiple,
	))
}

func (lib *nativeLib) callGetDownloader(
	tasksJSON string, count, threads, chunk int,
	useCallbackURL bool, userAgent, callbackURL *string,
	useSocket *bool,
) int {
	cTasks := C.CString(tasksJSON)
	defer C.free(unsafe.Pointer(cTasks))

	var cUA, cCBURL *C.char
	if userAgent != nil {
		cUA = C.CString(*userAgent)
		defer C.free(unsafe.Pointer(cUA))
	}
	if callbackURL != nil {
		cCBURL = C.CString(*callbackURL)
		defer C.free(unsafe.Pointer(cCBURL))
	}

	var cUseSocket *C.bool
	if useSocket != nil {
		v := C.bool(*useSocket)
		cUseSocket = &v
	}

	return int(C.call_get_downloader(
		lib.fnGetDownloader,
		cTasks, C.int(count), C.int(threads), C.int(chunk),
		C.tthsd_callback_t(C.goCallbackBridge),
		C.bool(useCallbackURL),
		cUA, cCBURL,
		cUseSocket,
	))
}

func (lib *nativeLib) callIntInt(fp unsafe.Pointer, id int) int {
	return int(C.call_int_int(fp, C.int(id)))
}
