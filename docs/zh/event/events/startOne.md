# startOne 事件 - 单个任务开始

## 事件概述

`startOne`事件在下载器开始执行单个下载任务时触发，用于通知调用方某个具体文件下载的开始。

### 事件特征

- **事件类型**：`startOne`
- **触发时机**：每个下载任务实际开始执行时触发
- **触发频率**：与任务数量相等，每个任务触发一次

## Event 字段信息

```json
{
  "Type": "startOne",
  "Name": "事件描述",
  "ShowName": "友好显示名称",
  "ID": "任务ID"
}
```

## Msg 字段信息（源码实际发送）

源码中 `startOne` 事件的 `msg` 包含下列字段：

```json
{
  "URL": "下载文件的完整URL",
  "SavePath": "文件保存路径",
  "ShowName": "任务显示名称",
  "Index": 1,          // 源码中使用 1-based（i+1）
  "Total": 5           // 总任务数量
}
```

### 字段说明

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| URL | String | 是 | 要下载文件的完整URL |
| SavePath | String | 是 | 下载文件的本地保存路径 |
| ShowName | String | 是 | 友好的显示名称（任务名） |
| Index | Integer | 是 | 当前任务在队列中的位置（源码为 1 开始） |
| Total | Integer | 是 | 总共需要处理的任务数量 |

## 使用示例

```python
def callback_func(event_dict, msg_dict):
    if event_dict['Type'] == 'startOne':
        url = msg_dict.get('URL', '')
        save_path = msg_dict.get('SavePath', '')
        index = msg_dict.get('Index', 1)
        total = msg_dict.get('Total', 0)
        print(f"开始下载第 {index}/{total} 个文件: {url} -> {save_path}")
```

## 注意事项

1. 源码中 `Index` 使用的是 1-based（即 `i+1`），文档应按此解释
2. `ShowName` 与 `ID` 用于定位任务显示名与唯一标识
3. 不要假定额外的字段（如 FileSize）一定存在，需做兼容性判断
