# octopus 到 Rust 数据迁移

## Goal

提供 **尽力而为** 的 octopus v0.9.28 SQLite → `gateway-rust` schema 导入工具，迁移渠道、分组、客户端 API Key（及可选请求日志），并明确不可迁移/需重建项。

## Background

- 实测 octopus 表：`api_keys`（明文 api_key）、`channels`（base_urls JSON）、`channel_keys`、`groups`、`group_items`、`relay_logs`、`users`、`settings`、stats 等。
- rust schema：规范化 `channel_base_urls`、api_keys 只存哈希、`request_logs`、自有 `schema_migrations`。
- 混用同一文件危险；迁移应生成/写入 **目标库**，并建议备份源库。

## Requirements

### R1. CLI

- `model-hub-gateway migrate-octopus --source <octopus.db> --dest <rust.db>`
- dest 不存在则创建；已存在且非空时需 `--force` 清空业务表或拒绝（实现选一种并文档化；推荐：`--force` 才覆盖写入，否则若 dest 已有 channels/api_keys 则失败）。
- 退出码：成功 0，失败非 0 + 可读错误。

### R2. 迁移内容（尽力）

| 源 | 目标 | 说明 |
|----|------|------|
| channels + channel_keys + base_urls JSON | channels / channel_base_urls / channel_keys | 保留 id 映射若可能 |
| groups + group_items | groups / group_items | 保留 channel_id 引用 |
| api_keys | api_keys | 明文 → hash+mask；**完整 Key 不写目标库** |
| relay_logs（可选） | request_logs | 映射字段子集；默认开启或 `--with-logs` |
| users / settings / stats | 不迁移 | admin 仍用 config 默认；文档说明 |

### R3. 限制与文档

- 管理密码不迁移（rust 用 config.auth）。
- 统计表不迁移。
- API Key 迁移后客户端 Key **可继续使用**（因可对明文 rehash）。
- 文档：备份、命令示例、失败时回退 octopus。

### R4. 测试

- 用内存/临时文件构造迷你 octopus 源库（或复制 smoke 结构）→ migrate → 在 rust 侧校验 channel/group/key 可用（hash 校验 raw key）。

## Acceptance Criteria

- [x] AC1：CLI migrate-octopus 可将 smoke 级源库导入目标库。
- [x] AC2：渠道/分组/上游 key 可读；客户端 key 可 find_by_raw_key。
- [x] AC3：目标库不存客户端 key 明文。
- [x] AC4：users/settings/stats 明确不迁移并有文档。
- [x] AC5：单测 + fmt/check/test/clippy 通过。
- [x] AC6：默认服务启动路径不自动破坏现有 db。

## Out of Scope

- 在线热迁移 / 双写
- 设置页一键迁移 UI
- 完美兼容所有 octopus 版本差异
