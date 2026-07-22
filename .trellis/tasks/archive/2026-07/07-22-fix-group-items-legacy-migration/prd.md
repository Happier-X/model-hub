# 修复 group_items 旧表结构迁移

## Goal

兼容数据库中旧 `gateway-rust` 风格 `group_items(group_id, channel_id, model_name, priority, weight)`，避免当前代码插入/读取 `provider_id`、`upstream_model`、`sort_order` 时报错，同时保留旧行。

## 已确认旧结构（Git 历史）

```sql
group_items(
  id, group_id,
  channel_id,
  model_name,
  priority,
  weight
)
```

当前结构需要：`provider_id`、`upstream_model`、`sort_order`。

## Requirements

- R1：检测 `group_items` 当前列。
- R2：缺失时逐列增加：
  - `provider_id INTEGER NOT NULL DEFAULT 0`
  - `upstream_model TEXT NOT NULL DEFAULT ''`
  - `sort_order INTEGER NOT NULL DEFAULT 0`
- R3：存在旧列时回填：`provider_id=channel_id`、`upstream_model=model_name`、`sort_order=priority`，但仅更新新列仍为默认值的旧行。
- R4：不删除旧列、不重建表、不删除旧行；重复迁移幂等。
- R5：旧列 `channel_id` / `model_name` 的 NOT NULL 约束仍存在，因此当前应用写入条目时须同步填充旧列，避免后续 `NOT NULL constraint failed`。
- R6：说明兼容边界：旧 `channel_id` 仅按数值映射为 `provider_id`；旧 channels 不自动迁移为 providers。若没有同 ID provider，UI 会显示数字且用户需编辑分组重新选择供应商。
- R7：单测覆盖旧结构读取、应用写入当前条目、重复迁移。

## Acceptance Criteria

- [x] 旧表迁移后不再报 `no column named provider_id`。
- [x] 旧行模型名/优先级保留到新字段。
- [x] 当前应用对兼容表写入时同步新旧列并成功。
- [x] 重复迁移成功。
- [x] `cargo test --lib` 通过。

## Out of Scope

- 自动完整迁移旧 `channels` / `channel_keys` / `channel_base_urls` 到 providers。
