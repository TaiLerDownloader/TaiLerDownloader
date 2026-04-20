"""
TLD Unit Test Framework

用于测试 TLD 动态库的各种功能。
使用 Python 调用 TLD_interface.py 来测试 DLL。

用法:
    python test_TLD.py                    # 运行所有测试
    python test_TLD.py -v                 # 详细输出
    python test_TLD.py TestDownload       # 运行特定测试类
    python test_TLD.py TestDownload.test_http_download  # 运行特定测试
"""

from __future__ import annotations

import json # pyright: ignore[reportUnusedImport]
import os
import sys
import tempfile
import time
import unittest
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).parent))

from scripts.tld_interface import TLDownloader, EventLogger # pyright: ignore[reportUnusedImport]


class TestTLDInterface(unittest.TestCase):
    """测试 TLD 接口的基本功能"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path: Path = Path(__file__).parent.parent
        cls.temp_dir: str = tempfile.mkdtemp()
        cls.test_files: list[str] = []

    @classmethod
    def tearDownClass(cls):
        for f in cls.test_files:
            try:
                if os.path.exists(f):
                    os.remove(f)
            except Exception:
                pass
        try:
            os.rmdir(cls.temp_dir)
        except Exception:
            pass

    def test_interface_load(self):
        """测试接口能否正常加载"""
        with TLDownloader(dll_path=self.dll_path) as dl:
            self.assertIsNotNone(dl._dll) # pyright: ignore[reportPrivateUsage]

    def test_get_downloader_invalid_params(self):
        """测试无效参数处理"""
        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.get_downloader(
                urls=[],
                save_paths=[],
            )
            self.assertEqual(dl_id, -1)

    def test_get_downloader_single_task(self):
        """测试创建单个任务下载器"""
        test_url = "https://httpbin.org/bytes/1024"
        save_path = os.path.join(self.temp_dir, "test_single.bin")
        self.test_files.append(save_path)

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.get_downloader(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=4,
                chunk_size_mb=1,
            )
            self.assertGreater(dl_id, 0)

            result = dl.start_download_by_id(dl_id)
            self.assertTrue(result)

            time.sleep(2)
            dl.stop_download(dl_id)


class TestDownloadFunctionality(unittest.TestCase):
    """测试下载功能"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path: Path = Path(__file__).parent.parent
        cls.temp_dir: str = tempfile.mkdtemp()
        cls.test_files: list[str] = []

    @classmethod
    def tearDownClass(cls):
        for f in cls.test_files:
            try:
                if os.path.exists(f):
                    os.remove(f)
            except Exception:
                pass
        try:
            os.rmdir(cls.temp_dir)
        except Exception:
            pass

    def test_http_download(self):
        """测试 HTTP 下载"""
        test_url = "https://httpbin.org/bytes/10240"
        save_path = os.path.join(self.temp_dir, "test_http.bin")
        self.test_files.append(save_path)

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.start_download(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=8,
                chunk_size_mb=1,
            )
            self.assertGreater(dl_id, 0)
            time.sleep(5)

    def test_multiple_urls(self):
        """测试多 URL 下载"""
        test_urls = [
            "https://httpbin.org/bytes/512",
            "https://httpbin.org/bytes/512",
        ]
        save_paths = [
            os.path.join(self.temp_dir, "test_multi1.bin"),
            os.path.join(self.temp_dir, "test_multi2.bin"),
        ]
        self.test_files.extend(save_paths)

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.start_download(
                urls=test_urls,
                save_paths=save_paths,
                thread_count=4,
                is_multiple=False,
            )
            self.assertGreater(dl_id, 0)
            time.sleep(5)

    def test_callback_function(self):
        """测试回调函数"""
        test_url = "https://httpbin.org/bytes/1024"
        save_path = os.path.join(self.temp_dir, "test_callback.bin")
        self.test_files.append(save_path)

        callback_events: list[tuple[dict[str, Any], dict[str, Any]]] = []

        def my_callback(event: dict[str, Any], msg: dict[str, Any]) -> None:
            callback_events.append((event, msg))

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.start_download(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=4,
                callback=my_callback,
            )
            self.assertGreater(dl_id, 0)
            time.sleep(3)

            self.assertGreater(len(callback_events), 0)


class TestSpeedLimit(unittest.TestCase):
    """测试速度限制功能"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path: Path = Path(__file__).parent.parent
        cls.temp_dir: str = tempfile.mkdtemp()
        cls.test_files: list[str] = []

    @classmethod
    def tearDownClass(cls):
        for f in cls.test_files:
            try:
                if os.path.exists(f):
                    os.remove(f)
            except Exception:
                pass
        try:
            os.rmdir(cls.temp_dir)
        except Exception:
            pass

    def test_set_speed_limit(self):
        """测试设置速度限制"""
        test_url = "https://httpbin.org/bytes/10240"
        save_path = os.path.join(self.temp_dir, "test_speed_limit.bin")
        self.test_files.append(save_path)

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.get_downloader(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=4,
            )
            self.assertGreater(dl_id, 0)

            result = dl.set_speed_limit(dl_id, 1024 * 1024)
            self.assertTrue(result)

            dl.start_download_by_id(dl_id)
            time.sleep(3)
            dl.stop_download(dl_id)


class TestProxySupport(unittest.TestCase):
    """测试代理支持"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path = Path(__file__).parent.parent
        cls.temp_dir = tempfile.mkdtemp()

    @classmethod
    def tearDownClass(cls):
        try:
            os.rmdir(cls.temp_dir)
        except Exception:
            pass

    def test_set_proxy(self):
        """测试设置代理"""
        test_url = "https://httpbin.org/bytes/1024"
        save_path = os.path.join(self.temp_dir, "test_proxy.bin")

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.get_downloader(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=4,
            )
            self.assertGreater(dl_id, 0)

            result = dl.set_proxy(dl_id, "http://proxy.example.com:8080")
            self.assertTrue(result)

            dl.stop_download(dl_id)

    def test_disable_proxy(self):
        """测试禁用代理"""
        test_url = "https://httpbin.org/bytes/1024"
        save_path = os.path.join(self.temp_dir, "test_no_proxy.bin")

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.get_downloader(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=4,
            )
            self.assertGreater(dl_id, 0)

            result = dl.set_proxy(dl_id, None)
            self.assertTrue(result)

            dl.stop_download(dl_id)


class TestRetryConfig(unittest.TestCase):
    """测试重试配置"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path = Path(__file__).parent.parent
        cls.temp_dir = tempfile.mkdtemp()

    @classmethod
    def tearDownClass(cls):
        try:
            os.rmdir(cls.temp_dir)
        except Exception:
            pass

    def test_set_retry_config(self):
        """测试设置重试配置"""
        test_url = "https://httpbin.org/bytes/1024"
        save_path = os.path.join(self.temp_dir, "test_retry.bin")

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.get_downloader(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=4,
            )
            self.assertGreater(dl_id, 0)

            result = dl.set_retry_config(
                dl_id,
                max_retries=5,
                retry_delay_ms=2000,
                max_retry_delay_ms=60000,
            )
            self.assertTrue(result)

            dl.stop_download(dl_id)


class TestPerformanceStats(unittest.TestCase):
    """测试性能统计"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path: Path = Path(__file__).parent.parent
        cls.temp_dir: str = tempfile.mkdtemp()
        cls.test_files: list[str] = []

    @classmethod
    def tearDownClass(cls):
        for f in cls.test_files:
            try:
                if os.path.exists(f):
                    os.remove(f)
            except Exception:
                pass
        try:
            os.rmdir(cls.temp_dir)
        except Exception:
            pass

    def test_get_performance_stats(self):
        """测试获取性能统计"""
        test_url = "https://httpbin.org/bytes/10240"
        save_path = os.path.join(self.temp_dir, "test_stats.bin")
        self.test_files.append(save_path)

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.start_download(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=8,
            )
            self.assertGreater(dl_id, 0)

            time.sleep(2)

            stats = dl.get_performance_stats(dl_id)
            self.assertIsInstance(stats, dict)

            if "total_bytes" in stats:
                print(f"总下载: {stats['total_bytes']} bytes")
            if "current_speed_mbps" in stats:
                print(f"当前速度: {stats['current_speed_mbps']} MB/s")
            if "average_speed_mbps" in stats:
                print(f"平均速度: {stats['average_speed_mbps']} MB/s")

            dl.stop_download(dl_id)

    def test_performance_stats_no_downloader(self):
        """测试获取不存在的下载器统计"""
        with TLDownloader(dll_path=self.dll_path) as dl:
            stats = dl.get_performance_stats(99999)
            self.assertIsInstance(stats, dict)


class TestErrorHandling(unittest.TestCase):
    """测试错误处理"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path = Path(__file__).parent.parent

    def test_invalid_url(self):
        """测试无效 URL"""
        test_url = "https://this-domain-does-not-exist-12345.com/file.bin"
        with tempfile.TemporaryDirectory() as temp_dir:
            save_path = os.path.join(temp_dir, "test_error.bin")

            with TLDownloader(dll_path=self.dll_path) as dl:
                dl_id = dl.start_download(
                    urls=[test_url],
                    save_paths=[save_path],
                    thread_count=4,
                )
                self.assertGreater(dl_id, 0)
                time.sleep(3)


class TestPauseResume(unittest.TestCase):
    """测试暂停和恢复功能"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path: Path = Path(__file__).parent.parent
        cls.temp_dir: str = tempfile.mkdtemp()
        cls.test_files: list[str] = []

    @classmethod
    def tearDownClass(cls):
        for f in cls.test_files:
            try:
                if os.path.exists(f):
                    os.remove(f)
            except Exception:
                pass
        try:
            os.rmdir(cls.temp_dir)
        except Exception:
            pass

    def test_pause_resume(self):
        """测试暂停和恢复"""
        test_url = "https://httpbin.org/bytes/20480"
        save_path = os.path.join(self.temp_dir, "test_pause.bin")
        self.test_files.append(save_path)

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.start_download(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=8,
            )
            self.assertGreater(dl_id, 0)

            time.sleep(2)

            result = dl.pause_download(dl_id)
            self.assertTrue(result)

            time.sleep(1)

            result = dl.resume_download(dl_id)
            self.assertTrue(result)

            time.sleep(2)

            dl.stop_download(dl_id)


class TestHeaders(unittest.TestCase):
    """测试 Headers 功能"""

    @classmethod
    def setUpClass(cls):
        cls.dll_path: Path = Path(__file__).parent.parent
        cls.temp_dir: str = tempfile.mkdtemp()
        cls.test_files: list[str] = []

    @classmethod
    def tearDownClass(cls):
        for f in cls.test_files:
            try:
                if os.path.exists(f):
                    os.remove(f)
            except Exception:
                pass
        try:
            os.rmdir(cls.temp_dir)
        except Exception:
            pass

    def test_global_headers(self):
        """测试全局 Headers"""
        test_url = "https://httpbin.org/headers"
        save_path = os.path.join(self.temp_dir, "test_headers.bin")
        self.test_files.append(save_path)

        headers = {
            "X-Custom-Header": "custom-value",
            "X-Test-Header": "test-value",
        }

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.start_download(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=4,
                headers=headers,
            )
            self.assertGreater(dl_id, 0)
            time.sleep(3)
            dl.stop_download(dl_id)

    def test_task_headers(self):
        """测试单个任务 Headers"""
        test_url = "https://httpbin.org/headers"
        save_path = os.path.join(self.temp_dir, "test_task_headers.bin")
        self.test_files.append(save_path)

        task_headers = [
            {
                "X-Task-Header": "task-value-1",
            }
        ]

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.start_download(
                urls=[test_url],
                save_paths=[save_path],
                thread_count=4,
                task_headers=task_headers,
            )
            self.assertGreater(dl_id, 0)
            time.sleep(3)
            dl.stop_download(dl_id)

    def test_combined_headers(self):
        """测试全局 + 单任务 Headers 合并"""
        test_urls = [
            "https://httpbin.org/headers",
            "https://httpbin.org/headers",
        ]
        save_paths = [
            os.path.join(self.temp_dir, "test_combined1.bin"),
            os.path.join(self.temp_dir, "test_combined2.bin"),
        ]
        self.test_files.extend(save_paths)

        global_headers = {
            "X-Global-Header": "global-value",
        }
        task_headers = [
            {"X-Task-Header": "task-value-1"},
            {"X-Task-Header": "task-value-2"},
        ]

        with TLDownloader(dll_path=self.dll_path) as dl:
            dl_id = dl.start_download(
                urls=test_urls,
                save_paths=save_paths,
                thread_count=4,
                headers=global_headers,
                task_headers=task_headers,
            )
            self.assertGreater(dl_id, 0)
            time.sleep(5)
            dl.stop_download(dl_id)


if __name__ == "__main__":
    print("=" * 60)
    print("TLD Unit Test Framework")
    print("=" * 60)
    print(f"测试目录: {Path(__file__).parent}")
    print(f"临时目录: {tempfile.gettempdir()}")
    print("=" * 60)

    unittest.main(verbosity=2)
