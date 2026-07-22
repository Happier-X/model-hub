# 审计并修复数据库架构兼容性

## 目标

系统核对当前 Rust domain 查询/写入依赖的 SQLite 字段与可能存在的旧版当前架构数据库，避免继续出现 `no such column` 启动或管理页面错误；对确认需要兼容的缺列增加安全、幂等、保留数据的迁移与回归测试。

## 背景

连续发现：

- 旧 `groups` 缺少 `auto_failover`，查询失败。
- 旧 `groups` 缺少 `created_at`，查询失败。
- 根因是 `CREATE TABLE IF NOT EXISTS` 不会给已有表增加新字段，而当前 `MIGRATION_V1` 仅对新库完整建表。

当前查询依赖表包括：`providers`、`groups`、`group_items`、`api_keys`、`request_logs`。

## 要求

- 枚举当前 domain 层对上述业务表读取、插入和更新所依赖的字段。
- 对比历史当前架构数据库形态与提交历史，确认哪些表可能缺列；不得凭空兼容旧 `gateway-rust` 等范围外 schema。
- 对确认缺失的字段，采用 `PRAGMA table_info` + `ALTER TABLE ADD COLUMN` 补齐；默认值必须满足当前非空查询契约。
- 迁移必须幂等，不重建表、不删除或覆盖已有数据。
- 增加一个覆盖真实旧库形态的综合回归测试：经 `open_db` 后，各 domain list/get 路径不因缺列失败，已有数据保持不变。
- 若未发现其他需迁移字段，也必须记录审计矩阵和验证证据，不进行无依据代码改动。

## 验收标准

- [x] 当前 domain SQL 依赖字段有完整审计矩阵。
- [x] 已确认的旧 schema 来源与字段差异有仓库证据。
- [x] 所有需要补齐的字段均有幂等迁移（仅 `groups.auto_failover` / `groups.created_at`）。
- [x] `providers` / `groups` / `group_items` / `api_keys` / `request_logs` 的相关读取不会因确认的旧 schema 缺列失败。
- [x] 现有数据与已有非默认值不被覆盖。
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` 与 `cargo check` 通过。

## 范围外

- 兼容已移除的 `gateway-rust` 全量历史 schema。
- 猜测并迁移没有仓库证据的第三方数据库。
- 前端功能修改。
