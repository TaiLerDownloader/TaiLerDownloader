"""
TLD 综合功能测试脚本
============================
覆盖四大验证类别：功能验证、性能验证、兼容性验证、稳定性验证

用法:
    1. 先启动本地测试服务器: python3 test_server.py
    2. 运行测试: python3 test_comprehensive.py

需要的环境:
    - 本地 HTTP 测试服务器运行在 127.0.0.1:18080
    - libTLD.so 动态库在 /home/amd/TTSD/ 下
    - TLD_interface.py 在同一目录
"""

import hashlib
import json
import os # pyright: ignore[reportUnusedImport]
import sys
import time
import threading
import traceback
import resource
from pathlib import Path
from datetime import datetime
from typing import Any

# 添加同目录到 path
sys.path.insert(0, str(Path(__file__).parent))
from tld_interface import TLDownloader, EventLogger # pyright: ignore[reportUnusedImport]

# ──────────────────────────────────────────────────────────────────
# 配置
# ──────────────────────────────────────────────────────────────────

LOCAL_BASE_URL = "http://127.0.0.1:18080"
PUBLIC_TEST_URLS = {
    "5mb": "http://ipv4.download.thinkbroadband.com/5MB.zip",
    "10mb": "http://ipv4.download.thinkbroadband.com/10MB.zip",
}
DLL_PATH = Path("/home/amd/TTSD/libTLD.so")
DOWNLOAD_DIR = Path("/home/amd/TTSD/test_downloads")
MANIFEST_PATH = Path("/home/amd/TTSD/test_files/manifest.json")

# ──────────────────────────────────────────────────────────────────
# 工具
# ──────────────────────────────────────────────────────────────────

class Colors:
    GREEN = "\033[92m"
    RED = "\033[91m"
    YELLOW = "\033[93m"
    CYAN = "\033[96m"
    BOLD = "\033[1m"
    RESET = "\033[0m"

def md5_file(filepath: str) -> str:
    """计算文件 MD5"""
    h = hashlib.md5()
    with open(filepath, "rb") as f:
        while chunk := f.read(8192):
            h.update(chunk)
    return h.hexdigest()

def clean_download_dir():
    """清空下载目录"""
    DOWNLOAD_DIR.mkdir(parents=True, exist_ok=True)
    for f in DOWNLOAD_DIR.iterdir():
        if f.is_file():
            f.unlink()

def load_manifest() -> dict[str, dict[str, str | int]]:
    """加载测试文件 manifest"""
    with open(MANIFEST_PATH) as f:
        return json.load(f)

def print_header(title: str):
    print(f"\n{'='*70}")
    print(f"  {Colors.BOLD}{Colors.CYAN}{title}{Colors.RESET}")
    print(f"{'='*70}")

def print_result(name: str, passed: bool, detail: str = ""):
    icon = f"{Colors.GREEN}✅ PASS{Colors.RESET}" if passed else f"{Colors.RED}❌ FAIL{Colors.RESET}"
    det = f" — {detail}" if detail else ""
    print(f"  {icon}  {name}{det}")

# ──────────────────────────────────────────────────────────────────
# 事件收集器（用于测试回调的正确性）
# ──────────────────────────────────────────────────────────────────

class EventCollector:
    """收集所有回调事件，供测试断言使用"""
    def __init__(self):
        self.events: list[dict[str, Any]] = []
        self.done = threading.Event()
        self.errors: list[dict[str, Any]] = []
        self.start_time: float = time.time()
        self.first_update_time: float | None = None

    def __call__(self, event: dict[str, Any], msg: dict[str, Any]) -> None:
        event_type: str = event.get("event_type", event.get("Type", "?"))
        self.events.append({"event": event, "msg": msg, "time": time.time()})

        if event_type == "update" and self.first_update_time is None:
            self.first_update_time = time.time()

        if event_type == "end":
            self.done.set()
        elif event_type == "err":
            self.errors.append(msg)

    def wait(self, timeout: int=30):
        return self.done.wait(timeout=timeout)

    def get_event_types(self) -> list[str]:
        return [e["event"].get("event_type", e["event"].get("Type", "?")) for e in self.events]

    @property
    def elapsed(self) -> float:
        return time.time() - self.start_time

# ──────────────────────────────────────────────────────────────────
# 一、功能验证
# ──────────────────────────────────────────────────────────────────

def test_single_file_download_md5():
    """测试 1: 单文件下载 + MD5 校验"""
    clean_download_dir()
    manifest = load_manifest()

    filename = "medium_1mb.bin"
    url = f"{LOCAL_BASE_URL}/{filename}"
    save_path = str(DOWNLOAD_DIR / filename)
    expected_md5: str = manifest[filename]["md5"] # pyright: ignore[reportAssignmentType]

    collector = EventCollector()
    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download(
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
            chunk_size_mb=1,
            callback=collector,
        )
        assert dl_id > 0, f"start_download 返回 {dl_id}"
        collector.wait(timeout=30)

    actual_md5 = md5_file(save_path)
    passed = (actual_md5 == expected_md5)
    print_result("单文件下载 + MD5 校验", passed,
                 f"期望={expected_md5[:8]}..., 实际={actual_md5[:8]}...")
    return passed


def test_multi_file_sequential():
    """测试 2: 多文件顺序下载"""
    clean_download_dir()
    manifest = load_manifest()

    files = ["tiny_1kb.bin", "small_100kb.bin", "medium_1mb.bin"]
    urls = [f"{LOCAL_BASE_URL}/{f}" for f in files]
    save_paths = [str(DOWNLOAD_DIR / f) for f in files]

    collector = EventCollector()
    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download(
            urls=urls,
            save_paths=save_paths,
            thread_count=4,
            chunk_size_mb=1,
            callback=collector,
        )
        assert dl_id > 0
        collector.wait(timeout=60)

    all_correct = True
    for filename in files:
        filepath = DOWNLOAD_DIR / filename
        if not filepath.exists():
            all_correct = False
            continue
        actual_md5 = md5_file(str(filepath))
        if actual_md5 != manifest[filename]["md5"]:
            all_correct = False

    print_result("多文件顺序下载 (3 files)", all_correct,
                 f"下载耗时 {collector.elapsed:.2f}s")
    return all_correct


def test_callback_events():
    """测试 3: 回调事件完整性"""
    clean_download_dir()

    url = f"{LOCAL_BASE_URL}/small_100kb.bin"
    save_path = str(DOWNLOAD_DIR / "callback_test.bin")

    collector = EventCollector()
    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
            callback=collector,
        )
        collector.wait(timeout=15)

    event_types = collector.get_event_types()
    has_start = "start" in event_types
    has_start_one = "startOne" in event_types
    has_end_one = "endOne" in event_types
    has_end = "end" in event_types

    passed = has_start and has_end
    detail_parts: list[str] = []
    for name, val in [("start", has_start), ("startOne", has_start_one),
                       ("endOne", has_end_one), ("end", has_end)]:
        icon = "✓" if val else "✗"
        detail_parts.append(f"{name}={icon}")

    print_result("回调事件完整性", passed, ", ".join(detail_parts))
    return passed


def test_error_handling_404():
    """测试 4: 错误处理 - 404 URL"""
    clean_download_dir()

    url = f"{LOCAL_BASE_URL}/nonexistent_file.bin"
    save_path = str(DOWNLOAD_DIR / "should_not_exist.bin")

    collector = EventCollector()
    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
            callback=collector,
        )
        collector.wait(timeout=15)

    has_error = len(collector.errors) > 0

    print_result("错误处理 (404 URL)", has_error,
                 f"收到 {len(collector.errors)} 个错误事件")
    return has_error


def test_get_downloader_then_start():
    """测试 5: 先创建后启动"""
    clean_download_dir()
    manifest = load_manifest() # pyright: ignore[reportUnusedVariable]

    url = f"{LOCAL_BASE_URL}/small_100kb.bin"
    save_path = str(DOWNLOAD_DIR / "deferred_start.bin")

    collector = EventCollector()
    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.get_downloader(
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
            callback=collector,
        )
        assert dl_id > 0, f"get_downloader 返回 {dl_id}"

        # 稍等再启动
        time.sleep(0.5)
        ok = dl.start_download_by_id(dl_id)
        assert ok, "start_download_by_id 返回 False"
        collector.wait(timeout=15)

    filepath = DOWNLOAD_DIR / "deferred_start.bin"
    exists = filepath.exists() and filepath.stat().st_size > 0
    print_result("创建后启动 (get_downloader + start_by_id)", exists,
                 f"文件大小: {filepath.stat().st_size if exists else 0} bytes")
    return exists


# ──────────────────────────────────────────────────────────────────
# 二、性能验证
# ──────────────────────────────────────────────────────────────────

def test_throughput_local():
    """测试 6: 本地服务器吞吐量测试 (10MB)"""
    clean_download_dir()

    url = f"{LOCAL_BASE_URL}/large_10mb.bin"
    save_path = str(DOWNLOAD_DIR / "throughput_test.bin")

    collector = EventCollector()
    start_time = time.time()

    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
            urls=[url],
            save_paths=[save_path],
            thread_count=8,
            chunk_size_mb=2,
            callback=collector,
        )
        collector.wait(timeout=60)

    elapsed = time.time() - start_time
    file_size = Path(save_path).stat().st_size if Path(save_path).exists() else 0
    speed_mbps = (file_size / 1024 / 1024) / elapsed if elapsed > 0 else 0

    passed = file_size == 10 * 1024 * 1024 and elapsed < 30
    print_result("本地吞吐量 (10MB)", passed,
                 f"{speed_mbps:.1f} MB/s, 耗时 {elapsed:.2f}s")
    return passed


def test_memory_usage() -> bool:
    """测试 7: 内存占用监控"""
    clean_download_dir()

    # 测量 baseline
    baseline_rss: int = int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss) # pyright: ignore[reportUnknownArgumentType, reportAttributeAccessIssue, reportUnknownMemberType]

    files = ["medium_1mb.bin", "large_10mb.bin"]
    urls = [f"{LOCAL_BASE_URL}/{f}" for f in files]
    save_paths = [str(DOWNLOAD_DIR / f"mem_{f}") for f in files]

    collector = EventCollector()
    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
            urls=urls,
            save_paths=save_paths,
            thread_count=8,
            chunk_size_mb=2,
            callback=collector,
        )
        collector.wait(timeout=60)

    peak_rss: int = int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss) # pyright: ignore[reportUnknownArgumentType, reportAttributeAccessIssue, reportUnknownMemberType]
    delta_mb = (peak_rss - baseline_rss) / 1024  # KB -> MB

    passed = delta_mb < 200  # 内存增量应 < 200MB
    print_result("内存占用", passed,
                 f"基线={baseline_rss/1024:.1f}MB, 峰值={peak_rss/1024:.1f}MB, 增量={delta_mb:.1f}MB")
    return passed


def test_startup_latency():
    """测试 8: 启动延迟（调用到首次回调的时间）"""
    clean_download_dir()

    url = f"{LOCAL_BASE_URL}/small_100kb.bin"
    save_path = str(DOWNLOAD_DIR / "latency_test.bin")

    collector = EventCollector()
    call_time = time.time()

    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
            callback=collector,
        )
        collector.wait(timeout=15)

    latency_ms = (collector.first_update_time - call_time) * 1000 if collector.first_update_time else -1
    if latency_ms < 0:
        # 可能没有 update 事件，用第一个事件替代
        if collector.events:
            latency_ms = (collector.events[0]["time"] - call_time) * 1000

    passed = 0 < latency_ms < 5000  # 应在 5 秒内收到首个回调
    print_result("启动延迟", passed,
                 f"首次回调于 {latency_ms:.0f}ms")
    return passed


# ──────────────────────────────────────────────────────────────────
# 三、兼容性验证
# ──────────────────────────────────────────────────────────────────

def test_public_server_download():
    """测试 9: 公网下载服务器兼容性 (thinkbroadband 5MB)"""
    clean_download_dir()

    url = PUBLIC_TEST_URLS["5mb"]
    save_path = str(DOWNLOAD_DIR / "public_5mb.zip")

    collector = EventCollector()
    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
            urls=[url],
            save_paths=[save_path],
            thread_count=4,
            chunk_size_mb=2,
            callback=collector,
        )
        collector.wait(timeout=120)

    file_exists = Path(save_path).exists()
    file_size = Path(save_path).stat().st_size if file_exists else 0
    # thinkbroadband 的 5MB 文件大约 5242880 字节
    correct_size = 4_000_000 < file_size < 6_000_000

    passed = file_exists and correct_size
    print_result("公网下载 (thinkbroadband 5MB)", passed,
                 f"文件大小: {file_size:,} bytes")
    return passed


def test_callback_json_format():
    """测试 10: 回调 JSON 格式正确性"""
    clean_download_dir()

    url = f"{LOCAL_BASE_URL}/tiny_1kb.bin"
    save_path = str(DOWNLOAD_DIR / "json_test.bin")

    raw_events: list[dict[str, Any]] = []

    def raw_callback(event: dict[str, Any], msg: dict[str, Any]):
        raw_events.append({"event": event, "msg": msg})

    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
            callback=raw_callback,
        )
        time.sleep(5)

    # 检查 event 字段
    valid = True
    issues: list[str] = []
    for entry in raw_events:
        ev: dict[str, Any] | Any = entry["event"]
        if not isinstance(ev, dict):
            valid = False
            issues.append("event 不是 dict")
        if "event_type" not in ev and "Type" not in ev:
            valid = False
            issues.append("缺少 event_type/Type")

    print_result("回调 JSON 格式", valid,
                 f"收到 {len(raw_events)} 条事件" + (f", 问题: {'; '.join(issues)}" if issues else ""))
    return valid


# ──────────────────────────────────────────────────────────────────
# 四、稳定性验证
# ──────────────────────────────────────────────────────────────────

def test_repeated_create_destroy():
    """测试 11: 反复创建/销毁下载器"""
    clean_download_dir()

    url = f"{LOCAL_BASE_URL}/tiny_1kb.bin"
    iterations = 10
    success_count = 0

    for i in range(iterations):
        save_path = str(DOWNLOAD_DIR / f"repeat_{i}.bin")
        collector = EventCollector()
        try:
            with TLDownloader(DLL_PATH) as dl:
                dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
                    urls=[url],
                    save_paths=[save_path],
                    thread_count=2,
                    chunk_size_mb=1,
                    callback=collector,
                )
                collector.wait(timeout=10)
                if Path(save_path).exists():
                    success_count += 1
        except Exception as e: # pyright: ignore[reportUnusedVariable]
            pass

    passed = success_count == iterations
    print_result(f"反复创建/销毁 ({iterations} 次)", passed,
                 f"成功 {success_count}/{iterations}")
    return passed


def test_unicode_filename():
    """测试 12: Unicode 文件名"""
    clean_download_dir()

    url = f"{LOCAL_BASE_URL}/tiny_1kb.bin"
    save_path = str(DOWNLOAD_DIR / "下载测试_文件名.bin")

    collector = EventCollector()
    with TLDownloader(DLL_PATH) as dl:
        dl_id = dl.start_download( # pyright: ignore[reportUnusedVariable]
            urls=[url],
            save_paths=[save_path],
            thread_count=2,
            chunk_size_mb=1,
            callback=collector,
        )
        collector.wait(timeout=15)

    exists = Path(save_path).exists() and Path(save_path).stat().st_size > 0
    print_result("Unicode 文件名", exists,
                 f"路径: {save_path}")
    return exists


def test_concurrent_multiple_downloaders():
    """测试 13: 并发多个独立下载器"""
    clean_download_dir()

    files: list[str] = ["tiny_1kb.bin", "small_100kb.bin", "medium_1mb.bin"]
    collectors: list[EventCollector] = []
    threads: list[threading.Thread] = []

    def download_one(filename: str, idx: int):
        url = f"{LOCAL_BASE_URL}/{filename}"
        save_path = str(DOWNLOAD_DIR / f"concurrent_{idx}_{filename}")
        collector = EventCollector()
        collectors.append(collector)
        with TLDownloader(DLL_PATH) as dl:
            dl.start_download( # pyright: ignore[reportUnusedVariable]
                urls=[url],
                save_paths=[save_path],
                thread_count=2,
                chunk_size_mb=1,
                callback=collector,
            )
            collector.wait(timeout=30)

    for i, f in enumerate(files):
        t = threading.Thread(target=download_one, args=(f, i))
        threads.append(t)
        t.start()

    for t in threads:
        t.join(timeout=30)

    all_done = all(c.done.is_set() for c in collectors)
    downloaded_count = sum(1 for c in collectors if c.done.is_set())
    print_result("并发多下载器 (3 同时)", all_done,
                 f"完成: {downloaded_count}/{len(files)}")
    return all_done


# ──────────────────────────────────────────────────────────────────
# 主入口
# ──────────────────────────────────────────────────────────────────

def main():
    print(f"\n{'#'*70}")
    print(f"#{' '*17}TLD 综合测试报告{' '*17}#")
    print(f"#{' '*15}{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}{' '*22}#")
    print(f"{'#'*70}")

    results: dict[str, bool] = {}

    # ── 一、功能验证 ──
    print_header("一、功能验证")
    tests_functional = [
        ("单文件下载+MD5", test_single_file_download_md5),
        ("多文件顺序下载", test_multi_file_sequential),
        ("回调事件完整性", test_callback_events),
        ("错误处理(404)", test_error_handling_404),
        ("创建后启动", test_get_downloader_then_start),
    ]

    for name, func in tests_functional:
        try:
            results[name] = func()
        except Exception as e:
            results[name] = False
            print_result(name, False, f"异常: {e}")
            traceback.print_exc()

    # ── 二、性能验证 ──
    print_header("二、性能验证")
    tests_performance = [
        ("本地吞吐量", test_throughput_local),
        ("内存占用", test_memory_usage),
        ("启动延迟", test_startup_latency),
    ]

    for name, func in tests_performance:
        try:
            results[name] = func()
        except Exception as e:
            results[name] = False
            print_result(name, False, f"异常: {e}")
            traceback.print_exc()

    # ── 三、兼容性验证 ──
    print_header("三、兼容性验证")
    tests_compat = [
        ("公网下载", test_public_server_download),
        ("回调JSON格式", test_callback_json_format),
    ]

    for name, func in tests_compat:
        try:
            results[name] = func()
        except Exception as e:
            results[name] = False
            print_result(name, False, f"异常: {e}")
            traceback.print_exc()

    # ── 四、稳定性验证 ──
    print_header("四、稳定性验证")
    tests_stability = [
        ("反复创建销毁", test_repeated_create_destroy),
        ("Unicode文件名", test_unicode_filename),
        ("并发多下载器", test_concurrent_multiple_downloaders),
    ]

    for name, func in tests_stability:
        try:
            results[name] = func()
        except Exception as e:
            results[name] = False
            print_result(name, False, f"异常: {e}")
            traceback.print_exc()

    # ── 汇总 ──
    print_header("测试汇总")
    total = len(results)
    passed = sum(1 for v in results.values() if v)
    failed = total - passed

    for name, ok in results.items():
        status = f"{Colors.GREEN}PASS{Colors.RESET}" if ok else f"{Colors.RED}FAIL{Colors.RESET}"
        print(f"  [{status}] {name}")

    print(f"\n  {'='*50}")
    color = Colors.GREEN if failed == 0 else Colors.RED
    print(f"  {Colors.BOLD}{color}通过: {passed}/{total}, 失败: {failed}/{total}{Colors.RESET}")
    print(f"  {'='*50}\n")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
