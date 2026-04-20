"""
TLD Progress Event 功能验证脚本
=================================
直接加载编译好的 .so 动态库，下载一个公网小文件，
验证是否能正确收到 Update 类型的进度事件。
"""

import ctypes
import json
import os
import sys
import time
import threading
from pathlib import Path

# ── 配置 ──
LIB_PATH = Path("/home/amd/TTSD_GitHub_Repo/target/release/libTLD.so")
TEST_URL = "http://127.0.0.1:19877/libTLD.so"
SAVE_PATH = "/tmp/TLD_progress_test.zip"

# ── 回调类型定义 ──
CALLBACK_TYPE = ctypes.CFUNCTYPE(None, ctypes.c_char_p, ctypes.c_char_p)

# ── 事件收集器 ──
events_collected: list[dict[str, object]] = []
event_types_seen: set[str] = set()
update_count = 0
done_event = threading.Event()

def callback_handler(event_ptr: ctypes.c_char_p | bytes, data_ptr: ctypes.c_char_p | bytes):
    global update_count
    try:
        event_str: str = str(event_ptr.decode("utf-8") if event_ptr else "{}") # pyright: ignore[reportUnknownArgumentType, reportAttributeAccessIssue, reportUnknownMemberType]
        data_str: str = str(data_ptr.decode("utf-8") if data_ptr else "{}") # pyright: ignore[reportUnknownArgumentType, reportAttributeAccessIssue, reportUnknownMemberType]
        event: dict[str, object] = json.loads(event_str)
        data: dict[str, object] = json.loads(data_str)

        event_type: str = event.get("Type", "unknown") # pyright: ignore[reportAssignmentType]
        event_types_seen.add(event_type)
        events_collected.append({"type": event_type, "data": data})

        if event_type == "update":
            update_count += 1
            total_bytes: int = data.get("total_bytes", 0) # pyright: ignore[reportAssignmentType]
            speed_mbps: float = data.get("current_speed_mbps", 0) # pyright: ignore[reportAssignmentType]
            if update_count % 4 == 0:  # 每 2 秒打印一次
                print(f"  [UPDATE #{update_count}] 已下载: {total_bytes / 1024 / 1024:.2f} MB, 速度: {speed_mbps:.2f} MB/s")
        elif event_type == "start":
            print(f"  [START] {event.get('Name', '')}")
        elif event_type == "startOne":
            show_name = event.get("ShowName", "")
            print(f"  [START_ONE] {show_name}")
        elif event_type == "endOne":
            print(f"  [END_ONE] {event.get('ShowName', '')}")
        elif event_type == "end":
            print(f"  [END] 下载完成")
            done_event.set()
        elif event_type == "err":
            print(f"  [ERROR] {data}")
            done_event.set()
    except Exception as e:
        print(f"  [回调异常] {e}")

def main():
    print("=" * 60)
    print("  TLD Progress Event 功能验证")
    print("=" * 60)

    # 1. 加载动态库
    print(f"\n[1] 加载动态库: {LIB_PATH}")
    if not LIB_PATH.exists():
        print(f"❌ 动态库不存在: {LIB_PATH}")
        return 1
    lib = ctypes.CDLL(str(LIB_PATH))
    print("✅ 动态库加载成功")

    # 2. 设置函数签名
    print("\n[2] 配置 FFI 函数签名")
    lib.start_download.argtypes = [
        ctypes.c_char_p,  # tasks_data
        ctypes.c_int,     # task_count
        ctypes.c_int,     # thread_count
        ctypes.c_int,     # chunk_size_mb
        ctypes.c_void_p,  # callback (usize)
        ctypes.c_bool,    # use_callback_url
        ctypes.c_char_p,  # user_agent
        ctypes.c_char_p,  # remote_callback_url
        ctypes.POINTER(ctypes.c_bool),  # use_socket
        ctypes.POINTER(ctypes.c_bool),  # is_multiple
    ]
    lib.start_download.restype = ctypes.c_int
    print("✅ FFI 签名配置完成")

    # 3. 准备下载任务
    print(f"\n[3] 准备下载任务")
    print(f"    URL: {TEST_URL}")
    print(f"    保存路径: {SAVE_PATH}")

    # 清理旧文件
    if os.path.exists(SAVE_PATH):
        os.remove(SAVE_PATH)

    tasks = [{"url": TEST_URL, "save_path": SAVE_PATH, "show_name": "5MB测试文件", "id": "test-001"}]
    tasks_json = json.dumps(tasks).encode("utf-8")

    # 4. 创建 C 回调
    c_callback = CALLBACK_TYPE(callback_handler)
    callback_ptr = ctypes.cast(c_callback, ctypes.c_void_p).value

    # 5. 启动下载
    print(f"\n[4] 启动下载 (线程数=8, 分块=2MB)")
    start_time = time.time()

    dl_id = lib.start_download(
        tasks_json,
        1,       # task_count
        8,       # thread_count
        2,       # chunk_size_mb
        callback_ptr,
        False,   # use_callback_url
        None,    # user_agent
        None,    # remote_callback_url
        None,    # use_socket
        None,    # is_multiple
    )

    print(f"    下载器 ID: {dl_id}")

    # 6. 等待完成
    print(f"\n[5] 等待下载完成 (超时 120s)...")
    done_event.wait(timeout=120)
    elapsed = time.time() - start_time

    # 7. 结果验证
    print(f"\n{'=' * 60}")
    print(f"  测试结果")
    print(f"{'=' * 60}")

    file_exists = os.path.exists(SAVE_PATH)
    file_size = os.path.getsize(SAVE_PATH) if file_exists else 0
    speed_mbps = (file_size / 1024 / 1024) / elapsed if elapsed > 0 else 0

    print(f"\n  事件类型收到: {event_types_seen}")
    print(f"  总事件数: {len(events_collected)}")
    print(f"  Update 事件数: {update_count}")
    print(f"  文件大小: {file_size / 1024 / 1024:.2f} MB")
    print(f"  下载耗时: {elapsed:.2f}s")
    print(f"  平均速度: {speed_mbps:.2f} MB/s")

    # 核心断言
    has_start = "start" in event_types_seen
    has_update = "update" in event_types_seen
    has_end = "end" in event_types_seen
    file_ok = file_exists and file_size > 1_000_000

    print(f"\n  ── 断言结果 ──")
    print(f"  {'✅' if has_start else '❌'} 收到 start 事件")
    print(f"  {'✅' if has_update else '❌'} 收到 update 事件 (进度更新)")
    print(f"  {'✅' if has_end else '❌'} 收到 end 事件")
    print(f"  {'✅' if file_ok else '❌'} 文件下载完整 ({file_size:,} bytes)")
    print(f"  {'✅' if update_count > 0 else '❌'} Update 事件次数 > 0 (实际: {update_count})")

    all_passed = has_start and has_update and has_end and file_ok and update_count > 0
    print(f"\n  {'✅ 全部通过！进度上报功能正常！' if all_passed else '❌ 存在失败项'}")
    print(f"{'=' * 60}\n")

    # 清理
    if file_exists:
        os.remove(SAVE_PATH)

    return 0 if all_passed else 1

if __name__ == "__main__":
    sys.exit(main())
