# TTHSD Python 接口封装 的回调函数示例

> [!TIP]
> 本示例适用于 **TTHSD Next**（Rust 版本）。
> 
> API 接口与 [TTHSD Golang](https://github.com/sxxyrry/TTHighSpeedDownloader) 版本完全兼容，代码无需修改即可使用。

## 提示：类型注解为可选，不影响代码运行

```python
from typing import Literal, TypedDict

class Event(TypedDict):
    Type: Literal['start', 'startOne', 'update', 'end', 'endOne', 'msg', 'err']
    Name: str
    ShowName: str
    ID: str

def callback_func(event_dict: Event, msg_dict: dict[str, str | int | float]):
    # 从事件字典中提取公共字段
    event_type: Literal['start', 'startOne', 'update', 'end', 'endOne', 'msg', 'err'] = event_dict.get('Type', '')
    event_name: str = event_dict.get('Name', '')
    event_showname: str = event_dict.get('ShowName', '')
    event_id: str = event_dict.get('ID', '')

    # 根据事件类型分支处理
    match event_type:
        case 'update':
            total: int = msg_dict.get('Total', 0)
            downloaded: int = msg_dict.get('Downloaded', 0)
            if total > 0:
                percent = (downloaded / total) * 100
                print(f"{event_showname}（{event_id}）：{downloaded}/{total} 字节 ({percent:.2f}%)", end='\r', flush=True)

        case 'startOne':
            url: str = msg_dict.get('URL', '')
            task_id: str = event_dict.get('ID', '')  # 与 event_id 相同，可能用于上下文
            index: int = msg_dict.get('Index', 0)
            total_tasks: int = msg_dict.get('Total', 0)
            print(f"\n{event_showname}（{event_id}）：开始下载：{url}，这是第 {index} 个下载任务，总共 {total_tasks} 个任务。")

        case 'start':
            print(f"\n{event_showname}（{event_id}）：开始下载")

        case 'endOne':
            url: str = msg_dict.get('URL', '')
            task_id: str = event_dict.get('ID', '')
            index: int = msg_dict.get('Index', 0)
            total_tasks: int = msg_dict.get('Total', 0)
            print(f"\n{event_showname}（{event_id}）：下载完成：{url}，这是第 {index} 个下载任务，总共 {total_tasks} 个任务。")

        case 'end':
            print(f"\n{event_showname}（{event_id}）：下载完成或已被取消")

        case 'msg':
            text: str = msg_dict.get('Text', '')
            print(f"\n{event_showname}（{event_id}）：{text}")

        case 'err':
            error: str = msg_dict.get('Error', '')
            print(f"\n{event_showname}（{event_id}）：错误: {error}")

        case _:
            # 忽略未知的事件类型
            pass

```

## TTHSD Next 的性能优势

使用 TTHSD Next 的 Python 接口，您可以获得以下性能提升：

- **更高的下载速度**：优化的异步 I/O 模型
- **更低的内存占用**：精细的内存管理，无 GC 开销
- **更高的并发数**：可支持数十万并发连接
- **更稳定的性能**：无 GC 暂停，性能可预测

## 兼容性说明

此回调函数示例完全兼容 TTHSD Golang 版本。如果您正在使用 TTHSD Golang 版本，只需将动态库文件替换为 tthsd.dll/so/dylib 即可，代码无需修改。