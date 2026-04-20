"""
TLD Next 本地测试 HTTP 服务器
- 支持 Range 请求（分块下载）
- 支持 HEAD 请求（获取文件大小）
- 支持 Content-Length 头
- 自动生成测试文件
"""

import os # pyright: ignore[reportUnusedImport]
import sys # pyright: ignore[reportUnusedImport]
import hashlib
import json
import threading # pyright: ignore[reportUnusedImport]
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from typing import Any

TEST_DIR = Path(__file__).parent / "test_files"
SERVER_PORT = 18080

# ─── 测试文件生成 ───────────────────────────────────────────────

def generate_test_files() -> dict[str, dict[str, str | int]]:
    """生成不同大小的测试文件，内容为可验证的重复模式"""
    TEST_DIR.mkdir(parents=True, exist_ok=True)

    files = {
        "tiny_1kb.bin":    1 * 1024,
        "small_100kb.bin": 100 * 1024,
        "medium_1mb.bin":  1 * 1024 * 1024,
        "large_10mb.bin":  10 * 1024 * 1024,
        "huge_100mb.bin":  100 * 1024 * 1024,
    }

    manifest: dict[str, dict[str, str | int]] = {}

    for name, size in files.items():
        filepath = TEST_DIR / name
        if filepath.exists() and filepath.stat().st_size == size:
            # 已存在且大小正确，只计算 MD5
            md5 = hashlib.md5()
            with open(filepath, "rb") as f:
                while chunk := f.read(8192):
                    md5.update(chunk)
            manifest[name] = {"size": size, "md5": md5.hexdigest()}
            print(f"  [已存在] {name} ({size:,} bytes) MD5={manifest[name]['md5']}")
            continue

        print(f"  [生成中] {name} ({size:,} bytes)...", end="", flush=True)
        # 用文件名 + 偏移量生成可预测的内容
        md5 = hashlib.md5()
        pattern = (name * 256)[:256].encode("utf-8")  # 256 字节的重复模式
        with open(filepath, "wb") as f:
            written = 0
            while written < size:
                chunk_size = min(len(pattern), size - written)
                chunk = pattern[:chunk_size]
                f.write(chunk)
                md5.update(chunk)
                written += chunk_size

        manifest[name] = {"size": size, "md5": md5.hexdigest()}
        print(f" OK  MD5={manifest[name]['md5']}")

    # 保存 manifest
    manifest_path = TEST_DIR / "manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    print(f"\n  Manifest 已保存到 {manifest_path}")
    return manifest


# ─── HTTP Handler（支持 Range） ─────────────────────────────────

class RangeRequestHandler(BaseHTTPRequestHandler):
    """支持 Range 请求的 HTTP 文件服务器"""

    def log_message(self, format: str, *args: tuple[Any]):
        """简化日志格式"""
        print(f"  [{self.client_address[0]}] {format % args}")

    def _resolve_path(self):
        """从 URL 解析文件路径"""
        path = self.path.lstrip("/")
        if not path:
            return None
        filepath = TEST_DIR / path
        if not filepath.exists() or not filepath.is_file():
            return None
        # 安全检查：不允许路径遍历
        try:
            filepath.resolve().relative_to(TEST_DIR.resolve())
        except ValueError:
            return None
        return filepath

    def do_HEAD(self):
        """处理 HEAD 请求（返回文件大小）"""
        filepath = self._resolve_path()
        if filepath is None:
            self.send_error(404, "File not found")
            return

        file_size = filepath.stat().st_size
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(file_size))
        self.send_header("Accept-Ranges", "bytes")
        self.end_headers()

    def do_GET(self):
        """处理 GET 请求（支持 Range）"""
        # 特殊路由：获取文件列表
        if self.path == "/manifest.json":
            manifest_path = TEST_DIR / "manifest.json"
            if manifest_path.exists():
                data = manifest_path.read_bytes()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
                return

        filepath = self._resolve_path()
        if filepath is None:
            self.send_error(404, "File not found")
            return

        file_size = filepath.stat().st_size
        range_header = self.headers.get("Range")

        if range_header:
            # 解析 Range 头
            try:
                range_spec = range_header.replace("bytes=", "")
                start_str, end_str = range_spec.split("-")
                start = int(start_str)
                end = int(end_str) if end_str else file_size - 1
                end = min(end, file_size - 1)

                if start >= file_size or start > end:
                    self.send_error(416, "Range Not Satisfiable")
                    return

                content_length = end - start + 1
                self.send_response(206)
                self.send_header("Content-Type", "application/octet-stream")
                self.send_header("Content-Length", str(content_length))
                self.send_header("Content-Range", f"bytes {start}-{end}/{file_size}")
                self.send_header("Accept-Ranges", "bytes")
                self.end_headers()

                with open(filepath, "rb") as f:
                    f.seek(start)
                    remaining = content_length
                    while remaining > 0:
                        chunk_size = min(65536, remaining)
                        chunk = f.read(chunk_size)
                        if not chunk:
                            break
                        self.wfile.write(chunk)
                        remaining -= len(chunk)

            except (ValueError, IndexError):
                self.send_error(400, "Bad Range header")
                return
        else:
            # 完整文件返回
            self.send_response(200)
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Content-Length", str(file_size))
            self.send_header("Accept-Ranges", "bytes")
            self.end_headers()

            with open(filepath, "rb") as f:
                while True:
                    chunk = f.read(65536)
                    if not chunk:
                        break
                    self.wfile.write(chunk)


# ─── 主入口 ─────────────────────────────────────────────────────

def main():
    print("=" * 60)
    print("  TLD Next 本地测试 HTTP 服务器")
    print("=" * 60)

    print("\n📦 生成测试文件...")
    manifest = generate_test_files()

    print(f"\n🚀 启动 HTTP 服务器，端口 {SERVER_PORT}...")
    print(f"   地址: http://127.0.0.1:{SERVER_PORT}/")
    print(f"   文件目录: {TEST_DIR.resolve()}")
    print(f"   可下载文件:")
    for name, info in manifest.items():
        print(f"     - http://127.0.0.1:{SERVER_PORT}/{name}  ({info['size']:,} bytes)")
    print(f"\n   按 Ctrl+C 停止服务器\n")

    server = HTTPServer(("0.0.0.0", SERVER_PORT), RangeRequestHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n\n⛔ 服务器已停止")
        server.shutdown()


if __name__ == "__main__":
    main()
