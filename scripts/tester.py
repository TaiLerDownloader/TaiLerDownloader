#!/usr/bin/env python3
"""
TTHSD Next 命令行测试器

功能：
- 交互式命令行界面，支持所有下载器操作
- 支持多种下载协议 (HTTP/HTTPS/FTP/SFTP/BitTorrent/ED2K/Metalink)
- 实时进度显示
- 多下载器实例管理
- 支持暂停/恢复/停止操作
- 支持顺序/并行下载模式

依赖: Python 3.11+, tthsd_interface.py
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path

# 添加脚本目录到路径
SCRIPT_DIR = Path(__file__).parent.resolve()
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from tthsd_interface import TTHSDownloader


# ------------------------------------------------------------------
# 颜色输出
# ------------------------------------------------------------------

class Color:
    """终端颜色常量"""
    reset = "\033[0m"
    bold = "\033[1m"
    red = "\033[91m"
    green = "\033[92m"
    yellow = "\033[93m"
    blue = "\033[94m"
    magenta = "\033[95m"
    cyan = "\033[96m"
    white = "\033[97m"
    dim = "\033[2m"

    @classmethod
    def disable(cls):
        """禁用颜色输出"""
        cls.reset = ""
        cls.bold = ""
        cls.red = ""
        cls.green = ""
        cls.yellow = ""
        cls.blue = ""
        cls.magenta = ""
        cls.cyan = ""
        cls.white = ""
        cls.dim = ""


def color_print(color: str, text: str, end: str = "\n"):
    """带颜色的打印"""
    print(f"{color}{text}{Color.reset}", end=end)


# ------------------------------------------------------------------
# 下轾示例 URL
# ------------------------------------------------------------------

EXAMPLE_URLS: dict[str, list[tuple[str, str, str]]] = {
    "http": [
        ("HTTP 小文件", "https://www.baidu.com/img/flexible/logo/pc/result.png", "baidu_logo.png"),
        ("HTTP 中文件", "https://github.com/TTHSDownloader/TTHSDNext/releases/latest", "release_info.html"),
    ],
    "http3": [
        ("HTTP/3 (Cloudflare)", "https://www.speedtest.net/api/js/config", "speedtest_config.json"),
    ],
    "ftp": [
        ("FTP 匿名", "ftp://ftp.gnu.org/gnu/README", "gnu_readme.txt"),
    ],
    "sftp": [
        # SFTP 需要密码，仅作示例格式
        ("SFTP 示例格式", "sftp://user:password@host:22/path/to/file", "example.bin"),
    ],
    "torrent": [
        ("Ubuntu Torrent", "https://releases.ubuntu.com/24.04/ubuntu-24.04-desktop-amd64.iso.torrent", "ubuntu.torrent"),
    ],
    "magnet": [
        ("Ubuntu Magnet", "magnet:?xt=urn:btih:3b245504cf5f11bbdbe1201cea6a6bf45aade1f3&dn=ubuntu-24.04-desktop-amd64.iso", "ubuntu.iso"),
    ],
    "ed2k": [
        ("ED2K 示例", "ed2k://|file|test.iso|1073741824|A1B2C3D4E5F6G7H8|/", "test.iso"),
    ],
    "metalink": [
        ("Arch Linux Metalink", "https://archlinux.org/metalink?protocol=https", "archlinux.metalink"),
    ],
}


# ------------------------------------------------------------------
# 下载状态
# ------------------------------------------------------------------

class DownloadStatus(Enum):
    """下载状态枚举"""
    IDLE = "idle"
    DOWNLOADING = "downloading"
    PAUSED = "paused"
    COMPLETED = "completed"
    STOPPED = "stopped"
    ERROR = "error"


@dataclass
class TaskInfo:
    """任务信息"""
    url: str
    save_path: str
    show_name: str
    task_id: str = ""
    status: DownloadStatus = DownloadStatus.IDLE
    total_bytes: int = 0
    downloaded_bytes: int = 0
    start_time: float = 0.0
    end_time: float = 0.0
    _speed: float = 0.0

    @property
    def progress(self) -> float:
        """进度百分比"""
        if self.total_bytes <= 0:
            return 0.0
        return (self.downloaded_bytes / self.total_bytes) * 100

    @property
    def speed(self) -> float:
        """下载速度 (bytes/s)"""
        return self._speed

    @speed.setter
    def speed(self, value: float):
        """设置下载速度"""
        self._speed = value

    @property
    def elapsed_time(self) -> float:
        """已用时间"""
        if self.start_time <= 0:
            return 0.0
        return (self.end_time or time.time()) - self.start_time


@dataclass
class DownloaderInstance:
    """下载器实例信息"""
    id: int
    tasks: dict[str, TaskInfo] = field(default_factory=dict[str, TaskInfo])
    status: DownloadStatus = DownloadStatus.IDLE
    is_multiple: bool = False
    thread_count: int = 64
    chunk_size_mb: int = 10
    start_time: float = 0.0


# ------------------------------------------------------------------
# 工具函数
# ------------------------------------------------------------------

def format_size(size: float) -> str:
    """格式化文件大小"""
    for unit in ["B", "KB", "MB", "GB", "TB"]:
        if size < 1024:
            return f"{size:.2f} {unit}"
        size /= 1024
    return f"{size:.2f} PB"


def format_time(seconds: float) -> str:
    """格式化时间"""
    if seconds < 60:
        return f"{seconds:.1f}s"
    elif seconds < 3600:
        minutes = int(seconds // 60)
        secs = int(seconds % 60)
        return f"{minutes}m {secs}s"
    else:
        hours = int(seconds // 3600)
        minutes = int((seconds % 3600) // 60)
        return f"{hours}h {minutes}m"


# ------------------------------------------------------------------
# 进度回调处理器
# ------------------------------------------------------------------

class ProgressHandler:
    """进度回调处理器"""

    def __init__(self, instance: DownloaderInstance, verbose: bool = True):
        self.instance = instance
        self.verbose = verbose
        self._lock = threading.Lock()
        self._completed_count = 0
        self._error_count = 0
        self._last_update_time = 0.0
        self._last_downloaded: int | float = 0

    def __call__(self, event: dict[str, object], msg: dict[str, object]) -> None:
        """回调入口"""
        event_type = event.get("Type", "")

        if event_type == "start":
            self._handle_start(event, msg)
        elif event_type == "startOne":
            self._handle_start_one(event, msg)
        elif event_type == "update":
            self._handle_update(event, msg)
        elif event_type == "endOne":
            self._handle_end_one(event, msg)
        elif event_type == "end":
            self._handle_end(event, msg)
        elif event_type == "msg":
            self._handle_msg(event, msg)
        elif event_type == "err":
            self._handle_err(event, msg)

    def _handle_start(self, event: dict[str, object], msg: dict[str, object]):
        """处理开始事件"""
        self.instance.status = DownloadStatus.DOWNLOADING
        self.instance.start_time = time.time()
        if self.verbose:
            color_print(Color.cyan, "\n🚀 下载会话开始")

    def _handle_start_one(self, event: dict[str, object], msg: dict[str, object]):
        """处理单个任务开始"""
        task_id = str(event.get("ID", ""))
        url = str(msg.get("URL", ""))
        save_path = str(msg.get("SavePath", ""))
        raw_show_name = msg.get("ShowName", url.split("/")[-1] if url else "")
        show_name = str(raw_show_name) if raw_show_name else ""
        raw_index = msg.get("Index", 0)
        raw_total = msg.get("Total", 0)
        index = int(raw_index) if isinstance(raw_index, (int, float, str)) else 0
        total = int(raw_total) if isinstance(raw_total, (int, float, str)) else 0

        with self._lock:
            if task_id not in self.instance.tasks:
                task = TaskInfo(
                    url=url,
                    save_path=save_path,
                    show_name=show_name,
                    task_id=task_id,
                    status=DownloadStatus.DOWNLOADING,
                    start_time=time.time()
                )
                self.instance.tasks[task_id] = task
            else:
                task = self.instance.tasks[task_id]
                task.status = DownloadStatus.DOWNLOADING
                task.start_time = time.time()

        if self.verbose:
            color_print(
                Color.blue,
                f"\n▶ 开始下载 [{index}/{total}]: {show_name or url}"
            )

    def _handle_update(self, event: dict[str, object], msg: dict[str, object]):
        """处理进度更新"""
        task_id = str(event.get("ID", ""))
        raw_total = msg.get("Total", 0)
        raw_downloaded = msg.get("Downloaded", 0)
        total = int(raw_total) if isinstance(raw_total, (int, float, str)) else 0
        downloaded = int(raw_downloaded) if isinstance(raw_downloaded, (int, float, str)) else 0

        with self._lock:
            if task_id in self.instance.tasks:
                task = self.instance.tasks[task_id]
                task.total_bytes = total
                task.downloaded_bytes = downloaded

                # 计算速度
                now = time.time()
                if self._last_update_time > 0:
                    elapsed = now - self._last_update_time
                    if elapsed > 0:
                        speed = (downloaded - self._last_downloaded) / elapsed
                        task.speed = speed

                self._last_update_time = now
                self._last_downloaded = downloaded

        if self.verbose:
            self._print_progress(task_id, total, downloaded)

    def _handle_end_one(self, event: dict[str, object], msg: dict[str, object]):
        """处理单个任务结束"""
        task_id = str(event.get("ID", ""))
        url = str(msg.get("URL", ""))
        raw_show_name = msg.get("ShowName", "")
        show_name = str(raw_show_name) if raw_show_name else ""
        raw_index = msg.get("Index", 0)
        raw_total = msg.get("Total", 0)
        index = int(raw_index) if isinstance(raw_index, (int, float, str)) else 0
        total = int(raw_total) if isinstance(raw_total, (int, float, str)) else 0

        with self._lock:
            if task_id in self.instance.tasks:
                task = self.instance.tasks[task_id]
                task.status = DownloadStatus.COMPLETED
                task.end_time = time.time()
                self._completed_count += 1

        if self.verbose:
            color_print(
                Color.green,
                f"\n✅ 下载完成 [{index}/{total}]: {show_name or url}"
            )

    def _handle_end(self, event: dict[str, object], msg: dict[str, object]):
        """处理全部结束"""
        self.instance.status = DownloadStatus.COMPLETED
        if self.verbose:
            elapsed = time.time() - self.instance.start_time
            total_downloaded = sum(
                t.downloaded_bytes for t in self.instance.tasks.values()
            )
            avg_speed = total_downloaded / elapsed if elapsed > 0 else 0
            color_print(
                Color.green,
                f"\n🏁 全部下载完成 | 总计: {format_size(total_downloaded)} | "
                f"用时: {format_time(elapsed)} | 平均速度: {format_size(avg_speed)}/s"
            )

    def _handle_msg(self, event: dict[str, object], msg: dict[str, object]):
        """处理消息"""
        text = str(msg.get("Text", ""))
        raw_show_name = event.get("ShowName", "")
        show_name = str(raw_show_name) if raw_show_name else ""
        
        # 检测特定状态
        if "暂停" in text:
            self.instance.status = DownloadStatus.PAUSED
            for task in self.instance.tasks.values():
                if task.status == DownloadStatus.DOWNLOADING:
                    task.status = DownloadStatus.PAUSED
        elif "恢复" in text:
            self.instance.status = DownloadStatus.DOWNLOADING
            for task in self.instance.tasks.values():
                if task.status == DownloadStatus.PAUSED:
                    task.status = DownloadStatus.DOWNLOADING
        elif "停止" in text:
            self.instance.status = DownloadStatus.STOPPED
            for task in self.instance.tasks.values():
                task.status = DownloadStatus.STOPPED

        if self.verbose:
            prefix = f"[{show_name}] " if show_name else ""
            color_print(Color.yellow, f"\n📢 {prefix}{text}")

    def _handle_err(self, event: dict[str, object], msg: dict[str, object]):
        """处理错误"""
        error = str(msg.get("Error", ""))
        raw_show_name = event.get("ShowName", "")
        show_name = str(raw_show_name) if raw_show_name else ""
        raw_task_id = event.get("ID", "")
        task_id = str(raw_task_id) if raw_task_id else ""

        with self._lock:
            if task_id and task_id in self.instance.tasks:
                self.instance.tasks[task_id].status = DownloadStatus.ERROR
            self._error_count += 1

        if self.verbose:
            prefix = f"[{show_name}] " if show_name else ""
            color_print(Color.red, f"\n❌ {prefix}错误: {error}")

    def _print_progress(self, task_id: str, total: int, downloaded: int):
        """打印进度条"""
        task = self.instance.tasks.get(task_id)
        if not task:
            return

        progress = task.progress
        bar_width = 30
        filled = int(bar_width * progress / 100)
        bar = "█" * filled + "░" * (bar_width - filled)

        total_str = format_size(total)
        downloaded_str = format_size(downloaded)
        speed_str = format_size(task.speed)

        # 使用回车符覆盖当前行
        print(
            f"\r  [{Color.cyan}{bar}{Color.reset}] "
            f"{progress:5.1f}% | {downloaded_str}/{total_str} | "
            f"{speed_str}/s",
            end="",
            flush=True
        )


# ------------------------------------------------------------------
# 命令行测试器
# ------------------------------------------------------------------

class TTHSDTester:
    """TTHSD 命令行测试器"""

    def __init__(self, dll_path: Path | None = None, dir_path: Path | None = None):
        """
        初始化测试器。

        参数:
            dll_path: 动态库路径。若为 None，根据操作系统在当前目录下寻找默认文件名。
            dir_path: 下载目录路径。若为 None，默认根据 dll_path 的方式。
        """
        self.dll_path = dll_path
        self.dir_path = dir_path
        self.downloaders: dict[int, DownloaderInstance] = {}
        self._downloader: TTHSDownloader | None = None
        self.verbose = True
        self._running = True

    def _init_downloader(self) -> TTHSDownloader:
        """初始化下载器实例"""
        if self._downloader is None:
            self._downloader = TTHSDownloader(
                dll_path=self.dll_path,
                dir_path=self.dir_path
            )
        return self._downloader

    def _close_downloader(self):
        """关闭下载器"""
        if self._downloader:
            self._downloader.close()
            self._downloader = None

    def create_download(
        self,
        urls: list[str],
        save_paths: list[str],
        thread_count: int = 64,
        chunk_size_mb: int = 10,
        is_multiple: bool = False,
        show_names: list[str] | None = None,
        auto_start: bool = True,
    ) -> int:
        """
        创建下载任务。

        参数:
            urls: 下载 URL 列表
            save_paths: 保存路径列表
            thread_count: 线程数
            chunk_size_mb: 分块大小 (MB)
            is_multiple: 是否并行下载
            show_names: 显示名称列表
            auto_start: 是否自动启动

        返回:
            下载器实例 ID
        """
        dl = self._init_downloader()

        # 创建实例信息
        instance = DownloaderInstance(
            id=-1,
            thread_count=thread_count,
            chunk_size_mb=chunk_size_mb,
            is_multiple=is_multiple
        )

        # 预填充任务信息
        for i, (url, save_path) in enumerate(zip(urls, save_paths)):
            show_name = show_names[i] if show_names and i < len(show_names) else Path(url.split("?")[0]).name
            task_id = f"task_{i}"
            instance.tasks[task_id] = TaskInfo(
                url=url,
                save_path=save_path,
                show_name=show_name,
                task_id=task_id
            )

        # 创建进度处理器
        handler = ProgressHandler(instance, self.verbose)

        if auto_start:
            dl_id = dl.start_download(
                urls=urls,
                save_paths=save_paths,
                thread_count=thread_count,
                chunk_size_mb=chunk_size_mb,
                callback=handler,
                is_multiple=is_multiple,
                show_names=show_names,
            )
        else:
            dl_id = dl.get_downloader(
                urls=urls,
                save_paths=save_paths,
                thread_count=thread_count,
                chunk_size_mb=chunk_size_mb,
                callback=handler,
                show_names=show_names,
            )

        if dl_id == -1:
            color_print(Color.red, "❌ 创建下载器失败")
            return -1

        instance.id = dl_id
        self.downloaders[dl_id] = instance

        color_print(
            Color.green,
            f"✅ 下载器已创建 (ID={dl_id}), 模式={'并行' if is_multiple else '顺序'}"
        )
        return dl_id

    def pause(self, dl_id: int) -> bool:
        """暂停下载"""
        if dl_id not in self.downloaders:
            color_print(Color.red, f"❌ 下载器 ID {dl_id} 不存在")
            return False

        dl = self._init_downloader()
        result = dl.pause_download(dl_id)

        if result:
            self.downloaders[dl_id].status = DownloadStatus.PAUSED
            color_print(Color.yellow, f"⏸️  下载器 {dl_id} 已暂停")
        else:
            color_print(Color.red, f"❌ 暂停下载器 {dl_id} 失败")

        return result

    def resume(self, dl_id: int) -> bool:
        """恢复下载"""
        if dl_id not in self.downloaders:
            color_print(Color.red, f"❌ 下载器 ID {dl_id} 不存在")
            return False

        dl = self._init_downloader()
        result = dl.resume_download(dl_id)

        if result:
            self.downloaders[dl_id].status = DownloadStatus.DOWNLOADING
            color_print(Color.green, f"▶️  下载器 {dl_id} 已恢复")
        else:
            color_print(Color.red, f"❌ 恢复下载器 {dl_id} 失败")

        return result

    def stop(self, dl_id: int) -> bool:
        """停止下载"""
        if dl_id not in self.downloaders:
            color_print(Color.red, f"❌ 下载器 ID {dl_id} 不存在")
            return False

        dl = self._init_downloader()
        result = dl.stop_download(dl_id)

        if result:
            self.downloaders[dl_id].status = DownloadStatus.STOPPED
            color_print(Color.red, f"⏹️  下载器 {dl_id} 已停止")
        else:
            color_print(Color.red, f"❌ 停止下载器 {dl_id} 失败")

        return result

    def list_instances(self):
        """列出所有下载器实例"""
        if not self.downloaders:
            color_print(Color.dim, "暂无下载器实例")
            return

        print(f"\n{Color.bold}下载器实例列表:{Color.reset}")
        print("-" * 80)

        for dl_id, instance in self.downloaders.items():
            status_color = {
                DownloadStatus.IDLE: Color.dim,
                DownloadStatus.DOWNLOADING: Color.cyan,
                DownloadStatus.PAUSED: Color.yellow,
                DownloadStatus.COMPLETED: Color.green,
                DownloadStatus.STOPPED: Color.red,
                DownloadStatus.ERROR: Color.red,
            }.get(instance.status, Color.white)

            total_size = sum(t.total_bytes for t in instance.tasks.values())
            downloaded = sum(t.downloaded_bytes for t in instance.tasks.values())
            progress = (downloaded / total_size * 100) if total_size > 0 else 0

            print(
                f"  ID: {Color.bold}{dl_id}{Color.reset} | "
                f"状态: {status_color}{instance.status.value}{Color.reset} | "
                f"模式: {'并行' if instance.is_multiple else '顺序'} | "
                f"任务: {len(instance.tasks)} | "
                f"进度: {progress:.1f}%"
            )

            for _, task in instance.tasks.items():
                task_status_color = {
                    DownloadStatus.IDLE: Color.dim,
                    DownloadStatus.DOWNLOADING: Color.cyan,
                    DownloadStatus.PAUSED: Color.yellow,
                    DownloadStatus.COMPLETED: Color.green,
                    DownloadStatus.STOPPED: Color.red,
                    DownloadStatus.ERROR: Color.red,
                }.get(task.status, Color.white)

                print(
                    f"    └─ {task.show_name[:40]:<40} | "
                    f"{task_status_color}{task.status.value}{Color.reset}"
                )

        print("-" * 80)

    def show_status(self, dl_id: int):
        """显示下载器状态"""
        if dl_id not in self.downloaders:
            color_print(Color.red, f"❌ 下载器 ID {dl_id} 不存在")
            return

        instance = self.downloaders[dl_id]

        print(f"\n{Color.bold}下载器 {dl_id} 状态:{Color.reset}")
        print("-" * 60)

        status_color = {
            DownloadStatus.IDLE: Color.dim,
            DownloadStatus.DOWNLOADING: Color.cyan,
            DownloadStatus.PAUSED: Color.yellow,
            DownloadStatus.COMPLETED: Color.green,
            DownloadStatus.STOPPED: Color.red,
            DownloadStatus.ERROR: Color.red,
        }.get(instance.status, Color.white)

        print(f"  状态: {status_color}{instance.status.value}{Color.reset}")
        print(f"  模式: {'并行' if instance.is_multiple else '顺序'}")
        print(f"  线程数: {instance.thread_count}")
        print(f"  分块大小: {instance.chunk_size_mb} MB")
        print(f"  任务数: {len(instance.tasks)}")

        if instance.start_time > 0:
            elapsed = time.time() - instance.start_time
            print(f"  运行时间: {format_time(elapsed)}")

        print("\n  任务详情:")
        for tid, task in instance.tasks.items():
            print(f"    [{tid}] {task.show_name}")
            print(f"      URL: {task.url}")
            print(f"      保存路径: {task.save_path}")
            print(f"      状态: {task.status.value}")
            if task.total_bytes > 0:
                print(f"      大小: {format_size(task.total_bytes)}")
                print(f"      已下载: {format_size(task.downloaded_bytes)}")
                print(f"      进度: {task.progress:.1f}%")

        print("-" * 60)

    def show_examples(self):
        """显示示例 URL"""
        print(f"\n{Color.bold}可用示例 URL:{Color.reset}")
        print("-" * 80)

        for protocol, examples in EXAMPLE_URLS.items():
            color_print(Color.cyan, f"\n  [{protocol.upper()}]")
            for name, url, _ in examples:
                print(f"    {name}:")
                print(f"      {Color.dim}{url}{Color.reset}")

        print("\n" + "-" * 80)

    def interactive_mode(self):
        """交互模式"""
        color_print(Color.bold, "\n═══════════════════════════════════════════════════════════")
        color_print(Color.bold, "         TTHSD Next 命令行测试器 - 交互模式")
        color_print(Color.bold, "═══════════════════════════════════════════════════════════")

        print(f"\n{Color.dim}输入 'help' 查看可用命令{Color.reset}\n")

        while self._running:
            try:
                cmd_input = input(f"{Color.green}TTHSD>{Color.reset} ").strip()
                if not cmd_input:
                    continue

                parts = cmd_input.split()
                cmd = parts[0].lower()
                args = parts[1:]

                if cmd in ("help", "h", "?"):
                    self._print_help()
                elif cmd in ("download", "dl"):
                    self._cmd_download(args)
                elif cmd in ("quick", "q"):
                    self.cmd_quick_download(args)
                elif cmd in ("pause", "p"):
                    self._cmd_pause(args)
                elif cmd in ("resume", "r"):
                    self._cmd_resume(args)
                elif cmd in ("stop", "s"):
                    self._cmd_stop(args)
                elif cmd in ("list", "ls"):
                    self.list_instances()
                elif cmd in ("status", "st"):
                    self._cmd_status(args)
                elif cmd in ("examples", "ex"):
                    self.show_examples()
                elif cmd in ("clear", "cls"):
                    if os.name == "nt":
                        subprocess.run(["cmd", "/c", "cls"], check=False)
                    else:
                        subprocess.run(["clear"], check=False)
                elif cmd in ("quit", "exit", "q!"):
                    self._cmd_quit()
                else:
                    color_print(Color.yellow, f"未知命令: {cmd}。输入 'help' 查看帮助。")

            except KeyboardInterrupt:
                print()
                self._cmd_quit()
            except Exception as e:
                color_print(Color.red, f"错误: {e}")

    def _print_help(self):
        """打印帮助信息"""
        print(f"""
{Color.bold}可用命令:{Color.reset}

  {Color.cyan}下载操作:{Color.reset}
    download <url> [save_path] [options]  创建下载任务
    quick <protocol_type>                  使用示例 URL 快速测试
    pause <id>                             暂停下载
    resume <id>                            恢复下载
    stop <id>                              停止下载

  {Color.cyan}信息查询:{Color.reset}
    list, ls                               列出所有下载器实例
    status <id>                            查看下载器详情
    examples, ex                           显示示例 URL

  {Color.cyan}其他:{Color.reset}
    help, h, ?                             显示帮助
    clear, cls                             清屏
    quit, exit, q!                         退出

{Color.bold}download 命令选项:{Color.reset}
    -t, --threads <n>     线程数 (默认: 64)
    -c, --chunk <mb>      分块大小 MB (默认: 10)
    -m, --multiple        并行下载模式
    --no-start            仅创建不启动

{Color.bold}示例:{Color.reset}
    download https://example.com/file.zip
    download https://example.com/file.zip ./downloads/file.zip -t 32
    download https://example.com/file.zip ./file.zip -m
    quick http
    pause 1
    resume 1
    stop 1
""")

    def _cmd_download(self, args: list[str]):
        """处理 download 命令"""
        if not args:
            color_print(Color.red, "用法: download <url> [save_path] [options]")
            return

        # 解析参数
        url = args[0]
        save_path = args[1] if len(args) > 1 and not args[1].startswith("-") else f"./downloads/{Path(url.split('?')[0]).name}"
        thread_count = 64
        chunk_size_mb = 10
        is_multiple = False
        auto_start = True

        i = 1
        while i < len(args):
            arg = args[i]
            if arg in ("-t", "--threads") and i + 1 < len(args):
                thread_count = int(args[i + 1])
                i += 2
            elif arg in ("-c", "--chunk") and i + 1 < len(args):
                chunk_size_mb = int(args[i + 1])
                i += 2
            elif arg in ("-m", "--multiple"):
                is_multiple = True
                i += 1
            elif arg == "--no-start":
                auto_start = False
                i += 1
            else:
                i += 1

        # 确保下载目录存在
        save_dir = Path(save_path).parent
        save_dir.mkdir(parents=True, exist_ok=True)

        self.create_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=thread_count,
            chunk_size_mb=chunk_size_mb,
            is_multiple=is_multiple,
            auto_start=auto_start
        )

    def cmd_quick_download(self, args: list[str]):
        """处理 quick 命令 - 快速测试"""
        if not args:
            print(f"\n{Color.bold}可用的协议类型:{Color.reset}")
            for protocol in EXAMPLE_URLS.keys():
                print(f"  - {protocol}")
            return

        protocol = args[0].lower()
        if protocol not in EXAMPLE_URLS:
            color_print(Color.red, f"未知协议类型: {protocol}")
            print(f"可用类型: {', '.join(EXAMPLE_URLS.keys())}")
            return

        examples = EXAMPLE_URLS[protocol]
        if not examples:
            color_print(Color.yellow, f"协议 {protocol} 暂无示例")
            return

        # 显示选项
        print(f"\n{Color.bold}{protocol.upper()} 示例:{Color.reset}")
        for i, (name, url, _) in enumerate(examples, 1):
            print(f"  {i}. {name}")
            print(f"     {Color.dim}{url}{Color.reset}")

        # 选择示例
        if len(examples) == 1:
            choice = 1
        else:
            try:
                choice_input = input(f"\n选择示例 [1-{len(examples)}] 或按回车取消: ").strip()
                if not choice_input:
                    return
                choice = int(choice_input)
                if choice < 1 or choice > len(examples):
                    color_print(Color.red, "无效选择")
                    return
            except ValueError:
                color_print(Color.red, "请输入数字")
                return

        name, url, filename = examples[choice - 1]
        save_path = f"./downloads/{filename}"

        # 确保下载目录存在
        Path(save_path).parent.mkdir(parents=True, exist_ok=True)

        color_print(Color.cyan, f"\n开始测试: {name}")
        color_print(Color.dim, f"URL: {url}")
        color_print(Color.dim, f"保存到: {save_path}")

        self.create_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=32,
            is_multiple=False
        )

    def _cmd_pause(self, args: list[str]):
        """处理 pause 命令"""
        if not args:
            color_print(Color.red, "用法: pause <id>")
            return
        try:
            dl_id = int(args[0])
            self.pause(dl_id)
        except ValueError:
            color_print(Color.red, "ID 必须是数字")

    def _cmd_resume(self, args: list[str]):
        """处理 resume 命令"""
        if not args:
            color_print(Color.red, "用法: resume <id>")
            return
        try:
            dl_id = int(args[0])
            self.resume(dl_id)
        except ValueError:
            color_print(Color.red, "ID 必须是数字")

    def _cmd_stop(self, args: list[str]):
        """处理 stop 命令"""
        if not args:
            color_print(Color.red, "用法: stop <id>")
            return
        try:
            dl_id = int(args[0])
            self.stop(dl_id)
        except ValueError:
            color_print(Color.red, "ID 必须是数字")

    def _cmd_status(self, args: list[str]):
        """处理 status 命令"""
        if not args:
            color_print(Color.red, "用法: status <id>")
            return
        try:
            dl_id = int(args[0])
            self.show_status(dl_id)
        except ValueError:
            color_print(Color.red, "ID 必须是数字")

    def _cmd_quit(self):
        """处理 quit 命令"""
        color_print(Color.dim, "\n正在清理资源...")

        # 停止所有下载器
        for dl_id in list(self.downloaders.keys()):
            if self.downloaders[dl_id].status == DownloadStatus.DOWNLOADING:
                self.stop(dl_id)

        self._close_downloader()
        self._running = False

        color_print(Color.green, "再见! 👋")


# ------------------------------------------------------------------
# 命令行参数解析
# ------------------------------------------------------------------

def parse_args():
    """解析命令行参数"""
    parser = argparse.ArgumentParser(
        description="TTHSD Next 命令行测试器",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 交互模式
  python tester.py

  # 指定动态库路径
  python tester.py --dll ./tthsd.dll

  # 直接下载
  python tester.py download https://example.com/file.zip

  # 快速测试 HTTP 协议
  python tester.py quick http
        """
    )

    parser.add_argument(
        "--dll", "-d",
        type=Path,
        help="动态库路径 (默认自动检测)"
    )

    parser.add_argument(
        "--dir",
        type=Path,
        help="下载目录路径 (默认 ./downloads)"
    )

    parser.add_argument(
        "--no-color",
        action="store_true",
        help="禁用颜色输出"
    )

    parser.add_argument(
        "--quiet", "-q",
        action="store_true",
        help="安静模式，减少输出"
    )

    # 子命令
    subparsers = parser.add_subparsers(dest="command", help="命令")

    # download 子命令
    dl_parser = subparsers.add_parser("download", help="下载文件")
    dl_parser.add_argument("url", help="下载 URL")
    dl_parser.add_argument("save_path", nargs="?", help="保存路径")
    dl_parser.add_argument("-t", "--threads", type=int, default=64, help="线程数")
    dl_parser.add_argument("-c", "--chunk", type=int, default=10, help="分块大小 (MB)")
    dl_parser.add_argument("-m", "--multiple", action="store_true", help="并行下载")

    # quick 子命令
    quick_parser = subparsers.add_parser("quick", help="快速测试")
    quick_parser.add_argument("protocol", nargs="?", help="协议类型")

    return parser.parse_args()


def main():
    """主函数"""
    args = parse_args()

    # 禁用颜色
    if args.no_color or not sys.stdout.isatty():
        Color.disable()

    # 创建测试器
    tester = TTHSDTester(
        dll_path=args.dll,
        dir_path=args.dir
    )

    if args.quiet:
        tester.verbose = False

    # 处理子命令
    if args.command == "download":
        save_path = args.save_path or f"./downloads/{Path(args.url.split('?')[0]).name}"
        Path(save_path).parent.mkdir(parents=True, exist_ok=True)

        dl_id = tester.create_download(
            urls=[args.url],
            save_paths=[save_path],
            thread_count=args.threads,
            chunk_size_mb=args.chunk,
            is_multiple=args.multiple
        )

        if dl_id != -1:
            # 等待下载完成
            try:
                while True:
                    time.sleep(1)
                    instance = tester.downloaders.get(dl_id)
                    if instance and instance.status in (
                        DownloadStatus.COMPLETED,
                        DownloadStatus.STOPPED,
                        DownloadStatus.ERROR
                    ):
                        break
            except KeyboardInterrupt:
                tester.stop(dl_id)

    elif args.command == "quick":
        if args.protocol:
            tester.cmd_quick_download([args.protocol])
            # 等待下载
            try:
                while tester.downloaders:
                    time.sleep(1)
                    all_done = all(
                        inst.status in (DownloadStatus.COMPLETED, DownloadStatus.STOPPED, DownloadStatus.ERROR)
                        for inst in tester.downloaders.values()
                    )
                    if all_done:
                        break
            except KeyboardInterrupt:
                for dl_id in list(tester.downloaders.keys()):
                    tester.stop(dl_id)
        else:
            tester.show_examples()

    else:
        # 交互模式
        tester.interactive_mode()


if __name__ == "__main__":
    main()