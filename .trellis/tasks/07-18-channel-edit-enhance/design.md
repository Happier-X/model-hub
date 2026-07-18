# 设计：渠道能力补强

## 边界

| 层 | 职责 |
|----|------|
| `src/api/channel.ts` | `updateChannel` / 类型；复用 create 载荷形状 |
| `ChannelsPage` | 编辑 UI、显示 Key、删除确认 |
| 侧车 | update/enable/delete（不改二进制） |

## 上游契约

### 已用

- `GET /api/v1/channel/list`
- `POST /api/v1/channel/create`（type 数字 0）
- `POST /api/v1/channel/enable` `{id, enabled}`
- `DELETE /api/v1/channel/delete/:id`

### 新增

- `POST /api/v1/channel/update`

dev 源码 `ChannelUpdateRequest`（部分更新 + keys_to_*）。  
**风险**：v0.9.28 可能仍要「整对象」或 keys 字段名不同 → 实现第 1 步真机探测。

### 探测策略（测试端口）

1. create 最小渠道  
2. 尝试 update：
   - A：`{id, name:"renamed"}`  
   - B：整对象（list 回写 + 改 name）  
   - C：`keys_to_update` / 全量 `keys`  
3. 以成功的 body 固化到 `updateOpenAiChatChannel`

## UI

```text
ChannelsPage
  CreateForm (现有 + type 说明)
  ChannelList
    item
      summary (name, url, model, mask/show key)
      actions: 编辑 | 启用 | 删除(确认)
      EditPanel (editingId === id)
        name, baseUrl, model, newApiKey(optional)
        保存 / 取消
```

## 数据流

```text
编辑保存 → updateChannel(payload)
       → listChannels()
显示 Key → React state Set<id> revealed
```

## 回滚

- 仅前端；去掉编辑面板与 update API 即可。
