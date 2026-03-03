# pylint: disable=invalid-name
"""
tthsd_interface.py - TT 高速下载器 Python 接口封装

兼容 TTHSD Next (Rust 版本) 与 TTHSD Golang 版本的动态库。
自动根据操作系统选择动态库文件名：
  - Windows: tthsd.dll
  - macOS:   tthsd.dylib
  - Linux:   tthsd.so

依赖: Python 3.11+, 标准库 (ctypes, json, threading, queue, weakref)

作者: 23XR Studio
文档: https://p.ceroxe.fun:58000/TTHSD/
"""

import ctypes
import json
import logging
import platform
import queue
import sys
import sysconfig
import types
import uuid
from pathlib import Path
from collections.abc import Callable
from typing import Any

# ------------------------------------------------------------------
# 内部日志器
# ------------------------------------------------------------------

_log_queue: queue.Queue[str] = queue.Queue()
_logger = logging.getLogger("TTHSD_interface")
if not _logger.handlers:
    _handler = logging.StreamHandler(sys.stdout)
    _formatter = logging.Formatter("[%(asctime)s][%(name)s][%(levelname)s] %(message)s")
    _handler.setFormatter(_formatter)
    _logger.addHandler(_handler)
    _logger.setLevel(logging.INFO)

# 尝试写入日志文件
try:
    _log_file_path = Path(sys.executable).parent / "TTHSDPyInter.log"
    _file_handler = logging.FileHandler(str(_log_file_path), mode="a", encoding="utf-8")
    _file_handler.setFormatter(logging.Formatter("[%(asctime)s][%(levelname)s] %(message)s"))
    _logger.addHandler(_file_handler)
except OSError:
    pass  # 忽略日志文件写入失败


# ------------------------------------------------------------------
# 回调类型定义 (C 接口)
# event_ptr / msg_ptr 均为 C 字符串 (char*) 指针
# ------------------------------------------------------------------

_CALLBACK_TYPE = ctypes.CFUNCTYPE(None, ctypes.c_char_p, ctypes.c_char_p)

# Pylance/MyPy 兼容：不能暴漏 _CFuncPtr 给它们，因为它是类型检查器未知的 ctypes 内部类
_CCallbackType = Any
def _default_dll_name() -> str:
    """根据当前操作系统返回默认动态库文件名。
    
    TTHSD 默认只支持 64 位系统，且命名规则为：
    - 桌面系统（ Windows, Linux, MacOS）是
      - x86_64 架构使用默认名称（tthsd.*）
      - ARM64 架构使用带后缀的名称（tthsd_arm64.*）
    - Android 版本是
      - tthsd_android_x86_64.so
      - tthsd_android_arm64.so
      - tthsd_android_armv7.so
    - HarmonyOS 版本是
      - tthsd_harmony_x86_64.so
      - tthsd_harmony_arm64.so
    """
    system = platform.system()
    machine = platform.machine().lower()
    
    # Android 特殊处理
    if hasattr(sys, 'getandroidapilevel'):
        android_map = {
            ('x86_64', 'amd64'): "tthsd_android_x86_64.so",
            ('arm64', 'aarch64'): "tthsd_android_arm64.so",
            ('armv7', 'armv7l'): "tthsd_android_armv7.so",
        }
        for patterns, filename in android_map.items():
            if machine in patterns:
                return filename
        raise OSError(f"不支持的 Android 架构: {machine}")
    
    # HarmonyOS 检测
    is_harmony = (
        system == "HarmonyOS" or 
        (system == "Linux" and any(x in platform.version().lower() for x in ('harmony', 'ohos')))
    )
    if is_harmony:
        if machine in ('arm64', 'aarch64'):
            return "tthsd_harmony_arm64.so"
        return "tthsd_harmony_x86_64.so"
    
    # 桌面系统
    if system == "Windows":
        if machine in ('arm64', 'aarch64'):
            return "tthsd_arm64.dll"
        return "tthsd.dll"
    
    if system == "Darwin":
        if machine in ('arm64', 'aarch64'):
            return "tthsd_arm64.dylib"
        return "tthsd.dylib"
    
    if system == "Linux":
        if machine in ('arm64', 'aarch64'):
            return "tthsd_arm64.so"
        return "tthsd.so"
    
    raise OSError(f"不支持的操作系统: {system}")


def _build_tasks_json(
    urls: list[str],
    save_paths: list[str],
    show_names: list[str] | None = None,
    ids: list[str] | None = None,
) -> str:
    """
    将 URL / 保存路径列表打包为 DLL 所接受的 JSON 字符串。

    参数:
        urls:       下载 URL 列表
        save_paths: 对应保存路径列表（长度必须与 urls 相等）
        show_names: 显示名称（可选，省略时使用 URL 最后一段）
        ids:        任务 ID（可选，省略时自动生成）

    返回:
        JSON 格式字符串
    """
    if len(urls) != len(save_paths):
        raise ValueError(
            f"urls 与 save_paths 长度不一致: {len(urls)} vs {len(save_paths)}"
        )
    tasks: list[dict[str, str]] = []
    for i, (url, save_path) in enumerate(zip(urls, save_paths)):
        show_name = (show_names[i] if show_names and i < len(show_names)
                     else Path(url.split("?")[0]).name or f"task_{i}")
        task_id = (ids[i] if ids and i < len(ids)
                   else str(uuid.uuid4()))
        tasks.append({
            "url": url,
            "save_path": str(save_path),
            "show_name": show_name,
            "id": task_id,
        })
    return json.dumps(tasks, ensure_ascii=False)


# ------------------------------------------------------------------
# 主封装类
# ------------------------------------------------------------------

class TTHSDownloader:
    """
    TTHSD 下载器 Python 封装类。

    支持功能:
    - 创建下载器实例（立即启动 / 仅创建）
    - 顺序或并行下载
    - 暂停 / 恢复 / 停止下载
    - 通过回调函数接收 update / end / endOne / msg / err 等事件

    基本用法:
        with TTHSDownloader() as dl:
            dl_id = dl.start_download(
                urls=["https://example.com/a.zip"],
                save_paths=["./a.zip"],
                callback=my_callback,
            )

    回调函数签名:
        def my_callback(event: dict, msg: dict) -> None: ...
    """

    def __init__(self, dll_path: str | Path | None = None):
        """
        初始化下载器封装。

        参数:
            dll_path: 动态库路径。若为 None，根据操作系统在当前目录下寻找默认文件名。
        """
        if dll_path is None:
            dll_path = Path.cwd() / _default_dll_name()

        dll_path = Path(dll_path).resolve()
        if not dll_path.exists():
            raise FileNotFoundError(
                f"动态库文件不存在 {dll_path}\n"
                "请确保 tthsd.so (Linux) / tthsd.dll (Windows) / tthsd.dylib (macOS) "
                "位于执行目录，或通过 dll_path 参数显式指定路径。"
            )

        _logger.info("加载动态库: %s", dll_path)
        self._dll = ctypes.CDLL(str(dll_path))
        self._setup_dll_signatures()

        # 保存回调函数的 C 可调用对象，防止被 GC 回收导致崩溃
        self._callback_refs: dict[int, Any] = {}

    # ------------------------------------------------------------------
    # DLL 函数签名配置
    # ------------------------------------------------------------------

    def _setup_dll_signatures(self):
        """配置 DLL 导出函数的参数类型和返回值类型。"""
        dll = self._dll

        # --- get_downloader ---
        dll.get_downloader.argtypes = [
            ctypes.c_char_p,   # tasksData (JSON)
            ctypes.c_int,      # taskCount
            ctypes.c_int,      # threadCount
            ctypes.c_int,      # chunkSizeMB
            ctypes.c_void_p,   # callback (nullable)
            ctypes.c_bool,     # useCallbackURL
            ctypes.c_char_p,   # userAgent (nullable)
            ctypes.c_char_p,   # remoteCallbackUrl (nullable)
            ctypes.c_void_p,   # useSocket (bool*, nullable)
        ]
        dll.get_downloader.restype = ctypes.c_int

        # --- start_download ---
        dll.start_download.argtypes = [
            ctypes.c_char_p,   # tasksData
            ctypes.c_int,      # taskCount
            ctypes.c_int,      # threadCount
            ctypes.c_int,      # chunkSizeMB
            ctypes.c_void_p,   # callback (nullable)
            ctypes.c_bool,     # useCallbackURL
            ctypes.c_char_p,   # userAgent (nullable)
            ctypes.c_char_p,   # remoteCallbackUrl (nullable)
            ctypes.c_void_p,   # useSocket (bool*, nullable)
            ctypes.c_void_p,   # isMultiple (bool*, nullable)
        ]
        dll.start_download.restype = ctypes.c_int

        # --- start_download_id ---
        dll.start_download_id.argtypes = [ctypes.c_int]
        dll.start_download_id.restype = ctypes.c_int

        # --- start_multiple_downloads_id ---
        dll.start_multiple_downloads_id.argtypes = [ctypes.c_int]
        dll.start_multiple_downloads_id.restype = ctypes.c_int

        # --- pause_download ---
        dll.pause_download.argtypes = [ctypes.c_int]
        dll.pause_download.restype = ctypes.c_int

        # --- resume_download ---
        dll.resume_download.argtypes = [ctypes.c_int]
        dll.resume_download.restype = ctypes.c_int

        # --- stop_download ---
        dll.stop_download.argtypes = [ctypes.c_int]
        dll.stop_download.restype = ctypes.c_int

    # ------------------------------------------------------------------
    # 内部工具：构建 C 回调
    # ------------------------------------------------------------------

    def _make_c_callback(
        self,
        user_callback: Callable[[dict[str, Any], dict[str, Any]], None],
    ) -> _CCallbackType:
        """
        将 Python 回调函数包装为 C 可调用对象。

        DLL 调用时传入两个 char* 参数（均为 JSON 字符串）；
        本包装器负责解析 JSON 并以 dict 形式转发给用户回调。
        """
        def _inner(event_ptr: ctypes.c_char_p, msg_ptr: ctypes.c_char_p):
            try:
                evt_s = event_ptr.value.decode("utf-8") if event_ptr else "{}" # pyright: ignore[reportOptionalMemberAccess]
                msg_s = msg_ptr.value.decode("utf-8") if msg_ptr else "{}" # pyright: ignore[reportOptionalMemberAccess]
                user_callback(json.loads(evt_s), json.loads(msg_s))
            except (json.JSONDecodeError, TypeError, ValueError) as exc:
                _logger.error("回调: %s", exc, exc_info=True)

        c_cb: _CCallbackType = _CALLBACK_TYPE(_inner)
        # 用 id(c_cb) 作为键保存引用，防止 GC 回收
        self._callback_refs[id(c_cb)] = c_cb
        return c_cb

    def _release_c_callback(self, c_cb: _CCallbackType) -> None:
        """释放已不再需要的 C 回调引用。"""
        key = id(c_cb)
        self._callback_refs.pop(key, None)

    # ------------------------------------------------------------------
    # 公开 API
    # ------------------------------------------------------------------

    def get_downloader(  # pylint: disable=too-many-arguments,too-many-locals
        self,
        urls: list[str],
        save_paths: list[str],
        thread_count: int = 64,
        chunk_size_mb: int = 10,
        callback: Callable[[dict[str, Any], dict[str, Any]], None] | None = None,
        use_callback_url: bool = False,
        user_agent: str | None = None,
        remote_callback_url: str | None = None,
        use_socket: bool | None = None,
        show_names: list[str] | None = None,
        ids: list[str] | None = None,
    ) -> int:
        """
        创建下载器实例，但**不启动下载**。

        参数:
            urls:               下载 URL 列表
            save_paths:         保存路径列表（与 urls 等长）
            thread_count:       下载线程数（默认 64）
            chunk_size_mb:      分块大小（MB，默认 10）
            callback:           进度回调函数 (event: dict, msg: dict) -> None
            use_callback_url:   是否启用远程回调 URL（默认 False）
            user_agent:         自定义 User-Agent（None 使用 DLL 默认值）
            remote_callback_url:远程回调 URL（None 不启用）
            use_socket:         是否启用 Socket 通信（None 不启用）
            show_names:         各任务显示名称列表（可选）
            ids:                各任务 ID 列表（可选）

        返回:
            下载器实例 ID（正整数），失败时返回 -1
        """
        tasks_json = _build_tasks_json(urls, save_paths, show_names, ids)
        task_count = len(urls)

        c_cb = None
        cb_ptr = None
        if callback is not None:
            c_cb = self._make_c_callback(callback)
            cb_ptr = ctypes.cast(c_cb, ctypes.c_void_p)

        ua_bytes = user_agent.encode("utf-8") if user_agent else None
        rc_url_bytes = remote_callback_url.encode("utf-8") if remote_callback_url else None

        use_socket_ptr: ctypes.c_void_p | None = None
        if use_socket is not None:
            _use_socket_c = ctypes.c_bool(use_socket)
            use_socket_ptr = ctypes.cast(ctypes.byref(_use_socket_c), ctypes.c_void_p)
        else:
            use_socket_ptr = None

        dl_id = self._dll.get_downloader(
            tasks_json.encode("utf-8"),
            task_count,
            thread_count,
            chunk_size_mb,
            cb_ptr,
            use_callback_url,
            ua_bytes,
            rc_url_bytes,
            use_socket_ptr,
        )

        if dl_id == -1:
            _logger.error("getDownloader 返回 -1，创建下载器实例失败")
        else:
            _logger.info("下载器已创建 (ID=%s)，共 %s 个任务", dl_id, task_count)

        return int(dl_id)

    def start_download(  # pylint: disable=too-many-arguments,too-many-locals
        self,
        urls: list[str],
        save_paths: list[str],
        thread_count: int = 64,
        chunk_size_mb: int = 10,
        callback: Callable[[dict[str, Any], dict[str, Any]], None] | None = None,
        use_callback_url: bool = False,
        user_agent: str | None = None,
        remote_callback_url: str | None = None,
        use_socket: bool | None = None,
        is_multiple: bool | None = None,
        show_names: list[str] | None = None,
        ids: list[str] | None = None,
    ) -> int:
        """
        创建下载器实例并**立即启动下载**。

        参数:
            urls:               下载 URL 列表
            save_paths:         保存路径列表（与 urls 等长）
            thread_count:       下载线程数（默认 64）
            chunk_size_mb:      分块大小（MB，默认 10）
            callback:           进度回调函数 (event: dict, msg: dict) -> None
            use_callback_url:   是否启用远程回调 URL（默认 False）
            user_agent:         自定义 User-Agent（None 使用 DLL 默认值）
            remote_callback_url:远程回调 URL（None 不启用）
            use_socket:         是否启用 Socket 通信（None 不启用）
            is_multiple:        True=并行下载(实验性), False/None=顺序下载
            show_names:         各任务显示名称列表（可选）
            ids:                各任务 ID 列表（可选）

        返回:
            下载器实例 ID（正整数），失败时返回 -1
        """
        tasks_json = _build_tasks_json(urls, save_paths, show_names, ids)
        task_count = len(urls)

        c_cb = None
        cb_ptr = None
        if callback is not None:
            c_cb = self._make_c_callback(callback)
            cb_ptr = ctypes.cast(c_cb, ctypes.c_void_p)

        ua_bytes = user_agent.encode("utf-8") if user_agent else None
        rc_url_bytes = remote_callback_url.encode("utf-8") if remote_callback_url else None

        if use_socket is not None:
            _use_socket_c = ctypes.c_bool(use_socket)
            use_socket_ptr = ctypes.cast(ctypes.byref(_use_socket_c), ctypes.c_void_p)
        else:
            use_socket_ptr = None

        if is_multiple is not None:
            _is_multiple_c = ctypes.c_bool(is_multiple)
            is_multiple_ptr = ctypes.cast(ctypes.byref(_is_multiple_c), ctypes.c_void_p)
        else:
            is_multiple_ptr = None

        dl_id = self._dll.start_download(
            tasks_json.encode("utf-8"),
            task_count,
            thread_count,
            chunk_size_mb,
            cb_ptr,
            use_callback_url,
            ua_bytes,
            rc_url_bytes,
            use_socket_ptr,
            is_multiple_ptr,
        )

        if dl_id == -1:
            _logger.error("startDownload 返回 -1，创建/启动下载器失败")
        else:
            mode_str = '并行' if is_multiple else '顺序'
            _logger.info(
                "下载器已创建并启动 (ID=%s)，共 %s 个任务，模式=%s",
                dl_id, task_count, mode_str
            )

        return int(dl_id)

    def start_download_by_id(self, downloader_id: int) -> bool:
        """
        启动已创建的下载器（**顺序**下载）。

        参数:
            downloader_id: get_downloader() 返回的实例 ID

        返回:
            True 表示成功（DLL 返回 0），False 表示失败（如 ID 不存在）
        """
        ret = self._dll.start_download_id(ctypes.c_int(downloader_id))
        if ret != 0:
            _logger.warning("start_download_id(id=%s) 返回 %s（失败）", downloader_id, ret)
        return ret == 0

    def start_multiple_downloads_by_id(self, downloader_id: int) -> bool:
        """
        启动已创建的下载器（**并行**下载，实验性）。

        参数:
            downloader_id: get_downloader() 返回的实例 ID

        返回:
            True 表示成功，False 表示失败
        """
        ret = self._dll.start_multiple_downloads_id(ctypes.c_int(downloader_id))
        if ret != 0:
            _logger.warning(
                "start_multiple_downloads_id(id=%s) 返回 %s（失败）", downloader_id, ret
            )
        return ret == 0

    def pause_download(self, downloader_id: int) -> bool:
        """
        暂停下载。

        核心版本 ≥0.5.1：立即取消所有进行中的连接，保留资源，可通过 resume_download() 恢复。
        核心版本 0.5.0：暂停后无法恢复（下载器已从映射表删除）。

        参数:
            downloader_id: 下载器实例 ID

        返回:
            True 表示成功，False 表示下载器不存在
        """
        ret = self._dll.pause_download(ctypes.c_int(downloader_id))
        if ret != 0:
            _logger.warning("pause_download(id=%s) 返回 %s（失败，ID 不存在）", downloader_id, ret)
        return ret == 0

    def resume_download(self, downloader_id: int) -> bool:
        """
        恢复已暂停的下载（需核心版本 ≥0.5.1）。

        注意：无法恢复已通过 stop_download() 停止的下载器。

        参数:
            downloader_id: 下载器实例 ID

        返回:
            True 表示成功，False 表示下载器不存在或版本不支持
        """
        ret = self._dll.resume_download(ctypes.c_int(downloader_id))
        if ret != 0:
            _logger.warning("resume_download(id=%s) 返回 %s（失败）", downloader_id, ret)
        return ret == 0

    def stop_download(self, downloader_id: int) -> bool:
        """
        停止下载并清理所有资源（下载器实例将被销毁，无法恢复）。

        参数:
            downloader_id: 下载器实例 ID

        返回:
            True 表示成功，False 表示下载器不存在
        """
        ret = self._dll.stop_download(ctypes.c_int(downloader_id))
        if ret != 0:
            _logger.warning("stop_download(id=%s) 返回 %s（失败）", downloader_id, ret)
        return ret == 0

    def close(self):
        """
        清理所有内部回调引用（可选调用）。
        通常无需手动调用，Python GC 会自动释放。
        """
        self._callback_refs.clear()
        _logger.info("TTHSDownloader.close() 已调用，回调引用已清理")

    # ------------------------------------------------------------------
    # 上下文管理器支持
    # ------------------------------------------------------------------

    def __enter__(self):
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: types.TracebackType | None,
    ) -> bool:
        self.close()
        return False

    def __del__(self):
        try:
            self._callback_refs.clear()
        except AttributeError:
            pass


# ------------------------------------------------------------------
# 快捷辅助工具: 构建事件回调
# ------------------------------------------------------------------

class EventLogger:
    """
    一个开箱即用的日志回调实现，将所有事件打印到控制台。
    可作为 callback 参数直接传给 start_download / get_downloader。

    用法:
        cb = EventLogger()
        dl.start_download(urls=[...], save_paths=[...], callback=cb)
    """

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}()"

    def __call__(self, event: dict[str, Any], msg: dict[str, Any]) -> None:
        event_type = event.get("Type", "?")
        show_name = event.get("ShowName", "")
        eid = event.get("ID", "")
        prefix = f"[{show_name}({eid})]" if show_name or eid else ""

        if event_type == "update":
            total = msg.get("Total", 0)
            downloaded = msg.get("Downloaded", 0)
            if total > 0:
                pct = downloaded / total * 100
                print(f"\r{prefix} 进度: {downloaded}/{total} ({pct:.2f}%)", end="", flush=True)

        elif event_type == "startOne":
            url = msg.get("URL", "")
            idx = msg.get("Index", 0)
            total = msg.get("Total", 0)
            print(f"\n{prefix} ▶ 开始下载 [{idx}/{total}]: {url}")

        elif event_type == "start":
            print(f"\n{prefix} 🚀 下载会话开始")

        elif event_type == "endOne":
            url = msg.get("URL", "")
            idx = msg.get("Index", 0)
            total = msg.get("Total", 0)
            print(f"\n{prefix} ✅ 下载完成 [{idx}/{total}]: {url}")

        elif event_type == "end":
            print(f"\n{prefix} 🏁 全部下载完成")

        elif event_type == "msg":
            text = msg.get("Text", "")
            print(f"\n{prefix} 📢 消息: {text}")

        elif event_type == "err":
            error = msg.get("Error", "")
            print(f"\n{prefix} ❌ 错误: {error}")

        else:
            print(f"\n{prefix} [未知事件 {event_type}] event={event} msg={msg}")


# ------------------------------------------------------------------
# 快捷函数: 一行启动下载
# ------------------------------------------------------------------

def quick_download(  # pylint: disable=too-many-arguments
    urls: list[str],
    save_paths: list[str],
    dll_path: str | Path | None = None,
    thread_count: int = 64,
    chunk_size_mb: int = 10,
    callback: Callable[[dict[str, Any], dict[str, Any]], None] | None = None,
    is_multiple: bool = False,
) -> int:
    """
    快捷函数：一行代码发起下载，返回下载器 ID。

    注意：此函数内部不会等待下载完成，使用方需自行等待（如通过 callback 中的 end 事件判断）。

    用法:
        dl_id = quick_download(
            urls=["https://example.com/a.zip"],
            save_paths=["./a.zip"],
            callback=EventLogger(),
        )
    """
    with TTHSDownloader(dll_path) as dl:
        return dl.start_download(
            urls=urls,
            save_paths=save_paths,
            thread_count=thread_count,
            chunk_size_mb=chunk_size_mb,
            callback=callback,
            is_multiple=is_multiple,
        )
    return -1
