# 数据库 schema 兼容性审计

## 当前 domain SQL 依赖字段

| 表 | 读取/写入依赖字段 | 位置 |
|----|------------------|------|
| providers | id, name, base_url, api_key, enabled, created_at | src-tauri/src/domain/provider.rs |
| groups | id, name, auto_failover, created_at | src-tauri/src/domain/group.rs |
| group_items | id, group_id, provider_id, upstream_model, sort_order | src-tauri/src/domain/group.rs |
| api_keys | id, name, key_hash, masked, enabled, created_at | src-tauri/src/domain/apikey.rs |
| request_logs | id, time, group_name, provider_name, upstream_model, status_code, use_time_ms, error, failover_from, failover_to, failover_reason | src-tauri/src/domain/log.rs |

## 仓库证据

- `1095a2b feat: Vue3 重写内嵌代理与 CC Switch 式故障转移` 引入当前新栈 schema v1，完整包含上述字段。
- `ffa2441` 与 `aa82c27` 已修复真实旧库中 `groups.auto_failover` 与 `groups.created_at` 缺列问题。
- 提交历史中未发现当前新栈 v1 曾以缺少 providers/api_keys/request_logs 字段的形态提交；缺列证据目前仅集中在用户本地旧 `groups` 表。
- 已移除的 `gateway-rust` 使用 `channels/channel_keys/channel_base_urls` 等旧 schema，按 PRD 与 spec 不纳入当前自动迁移范围。

## 结论

- 需要保留并测试 `groups.auto_failover`、`groups.created_at` 兼容迁移。
- 暂无仓库证据支持对 providers/api_keys/request_logs 追加猜测性缺列迁移。
- 应增加综合回归测试，构造真实问题形态（groups 仅 id/name，其他当前表为完整当前 schema），验证 domain list/get 路径均不因缺列失败。

## 实现与验证

- 迁移实现：`src-tauri/src/db/migrate.rs` 中 `ensure_group_columns` 继续用 `PRAGMA table_info(groups)` + 幂等 `ALTER TABLE` 补齐 `auto_failover`、`created_at`。
- 综合回归：`src-tauri/src/db/mod.rs` 测试 `open_db_upgrades_confirmed_legacy_schema_and_all_domain_reads_succeed` 构造真实旧库形态，经 `open_db` 后验证：
  - `list_groups` / `get_group_by_name`
  - `list_providers` / `get_provider`
  - `list_api_keys`
  - `list_logs`
  - 已有非默认业务值与密钥哈希不被覆盖；重复 `migrate` 不改写 `groups.created_at`
- 命令：`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml` 均通过。
