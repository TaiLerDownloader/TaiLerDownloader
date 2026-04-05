# endOne 事件 - 单个任务完成

## 事件概述

`endOne`事件在单个下载任务完成时触发，无论是成功下载还是失败终止，都会发送此事件通知。

## 事件特征

- **事件类型**：`endOne`
- **触发时机**：单个下载任务结束时（成功、失败或取消）
- **触发频率**：与任务数量相等，每个任务结束时触发一次

## Event字段信息

```json
{
  "Type": "endOne",
  "Name": "下载器实例标识符",
  "ShowName": "任务显示名称",
  "ID": "会话/实例ID"
}
```

### 字段说明

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| Type | String | 是 | 事件类型，固定值"endOne" |
| Name | String | 是 | 下载器实例的唯一标识符 |
| ShowName | String | 是 | 任务的显示名称，通常为文件名或用户指定的名称 |
| ID | String | 是 | 会话/实例ID |

## Msg字段信息

`endOne` 事件在源码中主要携带单任务的基本信息，示例字段如下：

```json
{
  "URL": "下载文件的完整URL",
  "SavePath": "文件保存路径",
  "ShowName": "任务显示名称",
  "Index": 1,   // 源码使用 1-based（i+1）
  "Total": 5
}
```

### 说明

- 如果实现方在外层逻辑中扩展了 `msg`，可以额外提供更详细的统计或状态字段，但接收方在使用时需做兼容处理。

## 使用示例（对源码发送字段的处理）

```python
def callback_func(event_dict, msg_dict):
    if event_dict['Type'] == 'endOne':
        url = msg_dict.get('URL', '')
        save_path = msg_dict.get('SavePath', '')
        index = msg_dict.get('Index', 1)
        total = msg_dict.get('Total', 0)
        print(f"任务完成 [{index}/{total}]: {url} -> {save_path}")
```
