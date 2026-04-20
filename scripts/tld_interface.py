"""
TLD_interface.py - TT 高速下载器 Python 接口封装

兼容 TLD (Rust 版本) 与 TLD Golang 版本的动态库。
自动根据操作系统选择动态库文件名：
  - Windows: TLD.dll
  - macOS:   TLD.dylib
  - Linux:   TLD.so

依赖: Python 3.11+, 标准库 (ctypes, json, queue)

作者: TT23XR Studio
文档: https://docss.sxxyrry.qzz.io/TLD/
"""

from __future__ import annotations

import ctypes
import json
import logging
import platform
import queue
import sys
import uuid
from pathlib import Path
from types import TracebackType
from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable
    from _ctypes import CFuncPtr

# ------------------------------------------------------------------
# 类型别名
# ------------------------------------------------------------------

# 回调函数类型
CallbackFunc = "Callable[[dict[str, Any], dict[str, Any]], None]"

# ------------------------------------------------------------------
# 内部日志器
# ------------------------------------------------------------------

_log_queue: queue.Queue[Any] = queue.Queue()
_logger = logging.getLogger("TLD_interface")
if not _logger.handlers:
    _handler = logging.StreamHandler(sys.stdout)
    _handler.setFormatter(logging.Formatter("[%(asctime)s][%(name)s][%(levelname)s] %(message)s"))
    _logger.addHandler(_handler)
    _logger.setLevel(logging.INFO)

# 尝试写入日志文件
try:
    _log_file_path = Path(sys.executable).parent / "TLDPyInter.log"
    _file_handler = logging.FileHandler(str(_log_file_path), mode="a", encoding="utf-8")
    _file_handler.setFormatter(logging.Formatter("[%(asctime)s][%(levelname)s] %(message)s"))
    _logger.addHandler(_file_handler)
except Exception:
    pass  # 忽略日志文件写入失败


# ------------------------------------------------------------------
# 回调类型定义 (C 接口)
# event_ptr / msg_ptr 均为 C 字符串 (char*) 指针
# ------------------------------------------------------------------

# 创建回调函数类型
_C_CALLBACK_TYPE = ctypes.CFUNCTYPE(None, ctypes.c_char_p, ctypes.c_char_p)


def _default_dll_name() -> str:
    """根据当前操作系统返回默认动态库文件名。
    
    TLD 默认只支持 64 位系统，且命名规则为：
    - 桌面系统（ Windows, Linux, MacOS）是
      - x86_64 架构使用默认名称（TaiLerDownloader.*）
      - ARM64 架构使用带后缀的名称（TaiLerDownloader_arm64.*）
    - Android 版本是
      - TaiLerDownloader_android_x86_64.so
      - TaiLerDownloader_android_arm64.so
      - TaiLerDownloader_android_armv7.so
    - HarmonyOS 版本是
      - TaiLerDownloader_harmony_x86_64.so
      - TaiLerDownloader_harmony_arm64.so
    - IOS 版本是
      - TaiLerDownloader_ios_arm64_device.dylib
      - TaiLerDownloader_ios_arm64_simulator.dylib
    """
    system = platform.system()
    machine = platform.machine().lower()
    
    # Android 特殊处理
    if hasattr(sys, 'getandroidapilevel'):
        android_map = {
            ('x86_64', 'amd64'): "TaiLerDownloader_android_x86_64.so",
            ('arm64', 'aarch64'): "TaiLerDownloader_android_arm64.so",
            ('armv7', 'armv7l'): "TaiLerDownloader_android_armv7.so",
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
            return "TaiLerDownloader_harmony_arm64.so"
        return "TaiLerDownloader_harmony_x86_64.so"
    
    # 桌面系统
    if system == "Windows":
        if machine in ('arm64', 'aarch64'):
            return "TaiLerDownloader_arm64.dll"
        return "TaiLerDownloader.dll"
    
    if system == "Darwin":
        if machine in ('arm64', 'aarch64'):
            return "TaiLerDownloader_arm64.dylib"
        return "TaiLerDownloader.dylib"
    
    if system == "iOS":
        # iOS 需要区分设备和模拟器，暂时统一使用 arm64 后缀的版本，用户可根据实际情况替换为对应的 dylib 文件
        if machine in ('arm64', 'aarch64'):
            return "TaiLerDownloader_ios_arm64_device.dylib"  # 或 TaiLerDownloader_ios_arm64_simulator.dylib
        raise OSError(f"不支持的 iOS 架构: {machine}")
    
    if system == "Linux":
        if machine in ('arm64', 'aarch64'):
            return "TaiLerDownloader_arm64.so"
        return "TaiLerDownloader.so"
    
    raise OSError(f"不支持的操作系统: {system}")


def _build_tasks_json(
    urls: list[str],
    save_paths: list[str],
    show_names: list[str] | None = None,
    ids: list[str] | None = None,
    task_headers: list[dict[str, str]] | None = None,
) -> str:
    """
    将 URL / 保存路径列表打包为 DLL 所接受的 JSON 字符串。

    参数:
        urls:        下载 URL 列表
        save_paths:  对应保存路径列表（长度必须与 urls 相等）
        show_names:  显示名称（可选，省略时使用 URL 最后一段）
        ids:         任务 ID（可选，省略时自动生成）
        task_headers: 每个任务的额外 headers（可选，与 urls 等长）

    返回:
        JSON 格式字符串
    """
    if len(urls) != len(save_paths):
        raise ValueError(
            f"urls 与 save_paths 长度不一致: {len(urls)} vs {len(save_paths)}"
        )

    tasks: list[dict[str, Any]] = []
    for i, (url, save_path) in enumerate(zip(urls, save_paths)):
        show_name = (show_names[i] if show_names and i < len(show_names)
                     else Path(url.split("?")[0]).name or f"task_{i}")
        task_id = (ids[i] if ids and i < len(ids)
                   else str(uuid.uuid4()))
        task_data: dict[str, Any] = {
            "url": url,
            "save_path": str(save_path),
            "show_name": show_name,
            "id": task_id,
        }
        if task_headers and i < len(task_headers) and task_headers[i]:
            task_data["headers"] = task_headers[i]
        tasks.append(task_data)
    return json.dumps(tasks, ensure_ascii=False)


# ------------------------------------------------------------------
# 主封装类
# ------------------------------------------------------------------

class TLDownloader:
    """
    TLD 下载器 Python 封装类。

    支持功能:
    - 创建下载器实例（立即启动 / 仅创建）
    - 顺序或并行下载
    - 暂停 / 恢复 / 停止下载
    - 通过回调函数接收 update / end / endOne / msg / err 等事件

    基本用法:
        with TLDownloader() as dl:
            dl_id = dl.start_download(
                urls=["https://example.com/a.zip"],
                save_paths=["./a.zip"],
                callback=my_callback,
            )

    回调函数签名:
        def my_callback(event: dict, msg: dict) -> None: ...
    """

    def __init__(self, dll_path: str | Path | None = None, dir_path: str | Path | None = None):
        """
        初始化下载器封装。

        参数:
            dll_path: 动态库路径。若为 None，根据操作系统在当前目录下寻找默认文件名。
            dir_path: 下载目录路径。若为 None，默认根据 dll_path 的方式。
        """
        if dll_path is None:
            dll_path = Path.cwd() / _default_dll_name()
        if dir_path is not None:
            dll_path = Path(dir_path).resolve() / _default_dll_name()
        else:
            if isinstance(dll_path, str):
                dll_path = Path(dll_path)
            dll_path = dll_path.parent / _default_dll_name()

        dll_path = Path(dll_path).resolve()
        if not dll_path.exists():
            raise FileNotFoundError(
                f"动态库文件不存在: {dll_path}\n"
                "请确保 TLD.so (Linux) / TLD.dll (Windows) / TLD.dylib (macOS) "
                "位于执行目录，或通过 dll_path 参数显式指定路径。"
            )

        _logger.info(f"加载动态库: {dll_path}")
        self._dll = ctypes.CDLL(str(dll_path))
        self._setup_dll_signatures()

        # 保存回调函数的 C 可调用对象，防止被 GC 回收导致崩溃
        self._callback_refs: dict[int, CFuncPtr] = {}

    # ------------------------------------------------------------------
    # DLL 函数签名配置
    # ------------------------------------------------------------------

    def _setup_dll_signatures(self) -> None:
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
            ctypes.c_char_p,   # headersJson (nullable)
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
            ctypes.c_char_p,   # headersJson (nullable)
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

        # --- set_speed_limit ---
        dll.set_speed_limit.argtypes = [ctypes.c_int, ctypes.c_uint64]
        dll.set_speed_limit.restype = ctypes.c_int

        # --- set_proxy ---
        dll.set_proxy.argtypes = [ctypes.c_int, ctypes.c_char_p]
        dll.set_proxy.restype = ctypes.c_int

        # --- set_retry_config ---
        dll.set_retry_config.argtypes = [ctypes.c_int, ctypes.c_uint32, ctypes.c_uint64, ctypes.c_uint64]
        dll.set_retry_config.restype = ctypes.c_int

        # --- get_performance_stats ---
        dll.get_performance_stats.argtypes = [ctypes.c_int]
        dll.get_performance_stats.restype = ctypes.c_char_p

        # --- free_string ---
        dll.free_string.argtypes = [ctypes.c_char_p]
        dll.free_string.restype = None

    # ------------------------------------------------------------------
    # 内部工具：构建 C 回调
    # ------------------------------------------------------------------

    def _make_c_callback(
        self,
        user_callback: Callable[[dict[str, Any], dict[str, Any]], None],
    ) -> CFuncPtr:
        """
        将 Python 回调函数包装为 C 可调用对象。

        DLL 调用时传入两个 char* 参数（均为 JSON 字符串）；
        本包装器负责解析 JSON 并以 dict 形式转发给用户回调。
        """
        def _inner(event_ptr: ctypes.c_char_p | None, msg_ptr: ctypes.c_char_p | None) -> None:
            try:
                # 获取事件字符串
                if event_ptr is not None:
                    event_bytes: bytes = event_ptr.value  # type: ignore
                    event_str: str = event_bytes.decode("utf-8")
                else:
                    event_str = "{}"
                
                # 获取消息字符串
                if msg_ptr is not None:
                    msg_bytes: bytes = msg_ptr.value  # type: ignore
                    msg_str: str = msg_bytes.decode("utf-8")
                else:
                    msg_str = "{}"
                
                event_dict: dict[str, Any] = json.loads(event_str)
                msg_dict: dict[str, Any] = json.loads(msg_str)

                print(event_dict, msg_dict)
                user_callback(event_dict, msg_dict)
            except Exception as exc:
                _logger.error(f"回调函数异常 (已捕获，不影响下载): {exc}", exc_info=True)

        c_cb = _C_CALLBACK_TYPE(_inner)
        # 用 id(c_cb) 作为键，避免同一 callback 重复保存多份引用
        self._callback_refs[id(c_cb)] = c_cb
        return c_cb

    def _release_c_callback(self, c_cb: CFuncPtr) -> None:
        """释放已不再需要的 C 回调引用。"""
        key = id(c_cb)
        self._callback_refs.pop(key, None)

    # ------------------------------------------------------------------
    # 公开 API
    # ------------------------------------------------------------------

    def get_downloader(
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
        headers: dict[str, str] | None = None,
        task_headers: list[dict[str, str]] | None = None,
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
            headers:            全局 headers（对该下载器所有任务生效）
            task_headers:       每个任务的额外 headers（与 urls 等长）

        返回:
            下载器实例 ID（正整数），失败时返回 -1
        """
        tasks_json = _build_tasks_json(urls, save_paths, show_names, ids, task_headers)
        task_count = len(urls)

        headers_json = json.dumps(headers) if headers else None
        headers_bytes = headers_json.encode("utf-8") if headers_json else None

        c_cb: CFuncPtr | None = None
        cb_ptr: ctypes.c_void_p | None = None
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
            headers_bytes,
        )

        if dl_id == -1:
            _logger.error("getDownloader 返回 -1，创建下载器实例失败")
        else:
            _logger.info(f"下载器已创建 (ID={dl_id})，共 {task_count} 个任务")

        return int(dl_id)

    def start_download(
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
        headers: dict[str, str] | None = None,
        task_headers: list[dict[str, str]] | None = None,
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
            headers:            全局 headers（对该下载器所有任务生效）
            task_headers:       每个任务的额外 headers（与 urls 等长）

        返回:
            下载器实例 ID（正整数），失败时返回 -1
        """
        tasks_json = _build_tasks_json(urls, save_paths, show_names, ids, task_headers)
        task_count = len(urls)

        c_cb: CFuncPtr | None = None
        cb_ptr: ctypes.c_void_p | None = None
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

        headers_json = json.dumps(headers) if headers else None
        headers_bytes = headers_json.encode("utf-8") if headers_json else None

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
            headers_bytes,
        )

        if dl_id == -1:
            _logger.error("startDownload 返回 -1，创建/启动下载器失败")
        else:
            _logger.info(
                f"下载器已创建并启动 (ID={dl_id})，共 {task_count} 个任务，"
                f"模式={'并行' if is_multiple else '顺序'}"
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
            _logger.warning(f"start_download_id(id={downloader_id}) 返回 {ret}（失败）")
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
            _logger.warning(f"start_multiple_downloads_id(id={downloader_id}) 返回 {ret}（失败）")
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
            _logger.warning(f"pause_download(id={downloader_id}) 返回 {ret}（失败，ID 可能不存在）")
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
            _logger.warning(f"resume_download(id={downloader_id}) 返回 {ret}（失败）")
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
            _logger.warning(f"stop_download(id={downloader_id}) 返回 {ret}（失败）")
        return ret == 0

    def set_speed_limit(self, downloader_id: int, speed_limit_bps: int) -> bool:
        """
        设置下载速度限制。

        参数:
            downloader_id: 下载器实例 ID
            speed_limit_bps: 速度限制 (bytes/s)，0 表示不限制

        返回:
            True 表示成功，False 表示下载器不存在
        """
        ret = self._dll.set_speed_limit(ctypes.c_int(downloader_id), ctypes.c_uint64(speed_limit_bps))
        if ret != 0:
            _logger.warning(f"set_speed_limit(id={downloader_id}) 返回 {ret}（失败）")
        return ret == 0

    def set_proxy(self, downloader_id: int, proxy_url: str | None) -> bool:
        """
        设置代理服务器。

        参数:
            downloader_id: 下载器实例 ID
            proxy_url: 代理 URL (如 "http://proxy:8080")，None 表示不使用代理

        返回:
            True 表示成功，False 表示下载器不存在
        """
        proxy_bytes = proxy_url.encode("utf-8") if proxy_url else None
        ret = self._dll.set_proxy(ctypes.c_int(downloader_id), proxy_bytes)
        if ret != 0:
            _logger.warning(f"set_proxy(id={downloader_id}) 返回 {ret}（失败）")
        return ret == 0

    def set_retry_config(self, downloader_id: int, max_retries: int = 3, retry_delay_ms: int = 1000, max_retry_delay_ms: int = 30000) -> bool:
        """
        配置重试参数。

        参数:
            downloader_id: 下载器实例 ID
            max_retries: 最大重试次数 (默认 3)
            retry_delay_ms: 初始重试延迟 (ms，默认 1000)
            max_retry_delay_ms: 最大重试延迟 (ms，默认 30000)

        返回:
            True 表示成功，False 表示下载器不存在
        """
        ret = self._dll.set_retry_config(
            ctypes.c_int(downloader_id),
            ctypes.c_uint32(max_retries),
            ctypes.c_uint64(retry_delay_ms),
            ctypes.c_uint64(max_retry_delay_ms),
        )
        if ret != 0:
            _logger.warning(f"set_retry_config(id={downloader_id}) 返回 {ret}（失败）")
        return ret == 0

    def get_performance_stats(self, downloader_id: int) -> dict[str, Any]:
        """
        获取性能统计信息。

        参数:
            downloader_id: 下载器实例 ID

        返回:
            性能统计字典，包含:
            - total_bytes: 总下载字节数
            - current_speed_bps: 当前速度 (bytes/s)
            - current_speed_mbps: 当前速度 (MB/s)
            - average_speed_bps: 平均速度 (bytes/s)
            - average_speed_mbps: 平均速度 (MB/s)
            - peak_speed_bps: 峰值速度 (bytes/s)
            - peak_speed_mbps: 峰值速度 (MB/s)
            - chunk_downloads: 分块下载数
            - failed_chunks: 失败分块数
            - retried_chunks: 重试分块数
            - elapsed_time: 运行时间 (秒)
        """
        result_ptr = self._dll.get_performance_stats(ctypes.c_int(downloader_id))
        try:
            result_str = ctypes.string_at(result_ptr).decode("utf-8")
            return json.loads(result_str) if result_str else {}
        finally:
            self._dll.free_string(result_ptr)

    def close(self) -> None:
        """
        清理所有内部回调引用（可选调用）。
        通常无需手动调用，Python GC 会自动释放。
        """
        self._callback_refs.clear()
        _logger.info("TLDownloader.close() 已调用，回调引用已清理")

    # ------------------------------------------------------------------
    # 上下文管理器支持
    # ------------------------------------------------------------------

    def __enter__(self) -> TLDownloader:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> bool:
        self.close()
        return False

    def __del__(self) -> None:
        try:
            self._callback_refs.clear()
        except Exception:
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

def quick_download(
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
    dl_id = -1
    with TLDownloader(dll_path) as dl:
        dl_id = dl.start_download(
            urls=urls,
            save_paths=save_paths,
            thread_count=thread_count,
            chunk_size_mb=chunk_size_mb,
            callback=callback,
            is_multiple=is_multiple,
        )
    return dl_id
