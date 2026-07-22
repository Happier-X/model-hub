# 修复 api_keys 旧表结构迁移

## Goal

兼容旧 gateway-rust `api_keys` 表（`api_key_masked`、无 `masked/created_at`），修复列表查询 `no such column: masked`，并保证当前创建/更新/鉴权可用。

## 已确认旧结构

```sql
api_keys(id, name, api_key_masked, key_hash, enabled,
         expire_at, max_cost, supported_models_json)
```

当前结构需要：`name`、`key_hash`、`masked`、`enabled`、`created_at`。

## Requirements

- R1：检测并补齐当前所需列；`masked` 从旧 `api_key_masked` 条件回填。
- R2：`created_at` 缺失时添加迁移时 UTC RFC3339 默认值。
- R3：不覆盖已有 `masked/created_at/key_hash/enabled` 值；不删除旧行；重复迁移幂等。
- R4：旧 `api_key_masked` 为 NOT NULL，当前创建 Key 时检测旧列并同步写入 `masked` 与 `api_key_masked`。
- R5：旧 Key 的 `key_hash` 保留，因此原始 Key 鉴权语义不变。
- R6：单测覆盖列表读取、旧字段回填、创建当前 Key、重复迁移。

## Acceptance Criteria

- [x] 旧表迁移后 `list_api_keys` 成功。
- [x] `masked` 等于旧 `api_key_masked`；`created_at` 有效。
- [x] 新建 Key 成功且明文只在创建响应出现。
- [x] 旧 Key 的 `key_hash` 不变。
- [x] `cargo test --lib` 通过。

## Out of Scope

- 自动导入更早的明文 `api_key`（octopus）表。
