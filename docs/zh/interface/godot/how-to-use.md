**# Godot GDExtension 接口使用文档

> [!TIP]
> 本文档介绍的是 **TTHSD Next** 的 Godot 4 GDExtension 封装。
> 源码位于 [`bindings/godot/`](https://github.com/TTHSDownloader/TTHSDNext/tree/main/bindings/godot)

---

## 特性

- **Godot Signal**：4 种信号，直接在 GDScript 中 `connect`
- **RefCounted**：继承自 `RefCounted`，Godot 自动管理内存
- **跨平台**：支持 Windows / Linux / macOS

---

## 快速开始 (GDScript)

```gdscript
extends Node

@onready var downloader: TTHSDownloader = TTHSDownloader.new()

func _ready() -> void:
    # 1. 加载动态库
    if not downloader.load_library(""):
        push_error("动态库加载失败")
        return

    # 2. 连接信号
    downloader.on_progress.connect(_on_progress)
    downloader.on_finished.connect(_on_finished)
    downloader.on_error.connect(_on_error)
    downloader.on_event.connect(_on_event)

    # 3. 启动下载
    var id: int = downloader.start_download(
        ["https://example.com/a.zip"],
        ["/tmp/a.zip"],
        64,   # thread_count
        10    # chunk_size_mb
    )
    print("下载 ID: %d" % id)


func _on_progress(event: Dictionary, data: Dictionary) -> void:
    var pct: float = float(data.get("Downloaded", 0)) / \
                     float(data.get("Total", 1)) * 100.0
    print("[%s] 进度: %.2f%%" % [event.get("ShowName", ""), pct])


func _on_finished(event: Dictionary, data: Dictionary) -> void:
    if event.get("Type") == "endOne":
        print("✅ 完成: %s" % data.get("URL", ""))
    elif event.get("Type") == "end":
        print("🏁 全部下载完成")


func _on_error(event: Dictionary, data: Dictionary) -> void:
    push_error("❌ 错误: %s" % data.get("Error", "未知"))


func _on_event(event: Dictionary, _data: Dictionary) -> void:
    match event.get("Type", ""):
        "start": print("🚀 下载会话开始")
        "startOne": print("▶ 开始: %s" % event.get("ShowName", ""))
        "msg": print("📢 %s" % _data.get("Text", ""))
```

---

## API 参考

### 方法

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `load_library(path)` | `String` | `bool` | 加载动态库（空字符串自动搜索） |
| `start_download(urls, paths, threads, chunk)` | `Array[String]` × 2 + `int` × 2 | `int` | 创建并启动 |
| `get_downloader(urls, paths, threads, chunk)` | 同上 | `int` | 创建不启动 |
| `start_download_by_id(id)` | `int` | `bool` | 顺序启动 |
| `start_multiple_downloads_by_id(id)` | `int` | `bool` | 并行启动 |
| `pause_download(id)` | `int` | `bool` | 暂停 |
| `resume_download(id)` | `int` | `bool` | 恢复 |
| `stop_download(id)` | `int` | `bool` | 停止销毁 |

### Signal 信号

| Signal | 触发条件 | 参数 |
|--------|----------|------|
| `on_progress(event, data)` | 进度更新（`update`） | `Dictionary` × 2 |
| `on_error(event, data)` | 下载错误（`err`） | `Dictionary` × 2 |
| `on_finished(event, data)` | 任务完成（`end` / `endOne`） | `Dictionary` × 2 |
| `on_event(event, data)` | 其他事件（`start` / `startOne` / `msg`） | `Dictionary` × 2 |

---

## 构建

### 依赖

- Godot 4.x C++ GDExtension SDK (`godot-cpp`)
- [nlohmann/json](https://github.com/nlohmann/json) (header-only)
- `uuid` 库 (`libuuid-dev` on Linux)

### 编译

```bash
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build .
```

### 安装到 Godot 项目

1. 将编译产物复制到 `project/addons/tthsd/bin/`
2. 将 `tthsd.dll` / `libtthsd.so` 也复制到同目录
3. 在 Godot 编辑器中启用插件
```