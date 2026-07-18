# 设计：分组体验补强

## 边界

| 层 | 职责 |
|----|------|
| `src/api/group.ts` | `updateGroup` + 类型 |
| `GroupsPage` | 列表明细、编辑面板、删除确认 |
| 侧车 | create/list/update/delete |

## 上游契约（dev 对照）

- `POST /api/v1/group/update` body `GroupUpdateRequest`：
  - `id` 必填
  - `name?` `mode?` `match_regex?` …
  - `items_to_add[]`：`channel_id` + `model_name` + priority/weight
  - `items_to_update[]`：改 priority/weight（**不能改 channel/model**）
  - `items_to_delete[]`：item id

换绑渠道/改 model_name：对首条 item 执行 **delete 旧 + add 新**（同一次 update 或分两次，以真机为准）。

## 探测（测试端口）

1. create group with item  
2. update `{id, name}`  
3. update 换绑：`items_to_delete` + `items_to_add`  
4. 仅改 model：同样 delete+add 或探测是否有其他字段  

## UI

```text
GroupsPage
  CreateForm (现有)
  List
    item: name, mode, bindings text
    actions: 编辑 | 删除(确认)
    EditPanel: name, channel select, model_name
```

渠道下拉复用 listChannels；编辑打开时预填首条 item。

## 回滚

- 仅前端；去掉编辑与 update 即可。
