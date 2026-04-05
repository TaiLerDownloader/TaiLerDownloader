# err 事件 - 错误通知

## 事件概述

`err`事件用于传递错误信息和异常情况，提醒调用程序或用户注意问题。

## 事件特征

- **事件类型**：`err`
- **触发时机**：系统需要发送通知或警告时
- **触发频率**：不定期触发，根据实际需要发送

## Event字段信息

```json
{
  "Type": "err",
  "Name": "消息来源标识符",
  "ShowName": "显示名称",
  "ID": "会话/实例ID"
}
```

### 字段说明

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| Type | String | 是 | 事件类型，固定值"err" |
| Name | String | 是 | 消息来源的标识符（通常是下载器实例ID） |
| ShowName | String | 是 | 显示名称，通常为消息来源的友好名称 |
| ID | String | 是 | 会话/实例ID |

## Msg字段信息


`err` 事件用于传递错误信息，源码中的 `err` 字段如下：

```json
{
    "Error": "错误文本",
}
```

### 字段说明

| 字段名 | 类型 | 说明 |
|--------|------|------|
| Error | String | 主要的人类可读的错误文本 |

```python
def callback_func(event_dict, msg_dict):
    if event_dict['Type'] == 'err':
        error = msg_dict.get('Error', '')
        
        print(f"{event_dict['ShowName']}：{error}")
```

---

## 版本差异

### 核心版本 0.5.0
在 TTHSD Core 0.5.0 版本中，某些错误情况（如 `startDownload_ID` 或 `startMultipleDownloads_ID` 失败）会以 `EventTypeMsg` 形式发送错误事件，而不是 `EventTypeErr`。此时错误信息存储在 `msg["Text"]` 字段中。

### 核心版本 0.5.1
所有错误统一以 `EventTypeErr` 事件类型发送，错误信息统一存储在 `msg["Error"]` 字段中。

---

> **重要提示**：如果您使用的是 TTHSD Core 0.5.0 版本，需要同时监听 `EventTypeErr` 和 `EventTypeMsg` 类型的事件来获取完整的错误信息：
> ```python
> def callback_func(event_dict, msg_dict):
>     event_type = event_dict.get('Type', '')
>     
>     # 处理 0.5.1+ 版本的错误事件
>     if event_type == 'err':
>         error = msg_dict.get('Error', '')
>         print(f"错误: {error}")
>     
>     # 处理 0.5.0 版本的错误事件（以 msg 形式发送）
>     elif event_type == 'msg':
>         text = msg_dict.get('Text', '')
>         # 检查是否包含错误信息
>         if text and ('错误' in text or 'Error' in text or '失败' in text):
>             print(f"错误: {text}")
> ```
