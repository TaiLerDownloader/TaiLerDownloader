# start 事件 - 下载任务组开始

## 事件概述

`start`事件在下载器开始执行整个下载任务组时触发，标志着下载流程的正式启动。

## 事件特征

- **事件类型**：`start`
- **触发时机**：下载器开始处理任务组时触发（源码中通常发送一个空的 msg）
- **触发频率**：每个下载会话仅触发一次

## Event 字段信息

```json
{
  "Type": "start",
  "Name": "事件描述",
  "ShowName": "友好显示名称"
}
```

### Msg 字段说明

源码中对 `start` 事件通常发送一个空的 `msg`（即 `{}`）。如果实现方扩展，可包含例如 `TaskCount` 等会话级别信息，但默认不依赖这些字段。

## 使用示例

```python
def callback_func(event_dict, msg_dict):
    if event_dict['Type'] == 'start':
        # msg_dict 默认通常为空，检查并兼容处理
        task_count = msg_dict.get('TaskCount', None)
        print("下载会话已开始", event_dict.get('ShowName'))
```

## 注意事项

1. 此事件标志着下载流程的正式开始
2. 不要假设 `msg` 中一定包含统计信息，接收方应对空 `msg` 做兼容处理
3. 在此事件之后才会触发每个任务的 `startOne` 事件
