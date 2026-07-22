# 修复旧 groups 表 created_at 迁移

## 目标

修复旧版 SQLite 数据库的 `groups` 表缺少 `created_at` 字段时，分组查询报 `no such column: created_at` 的问题，使 `open_db` 启动迁移后 `list_groups` / `get_group_by_name` 可以正常工作，同时保留已有分组与分组条目数据。

## 已确认事实

- 当前查询位于 `src-tauri/src/domain/group.rs`，读取：`id, name, auto_failover, created_at`。
- 当前建表 SQL 已声明 `groups.created_at TEXT NOT NULL`，但 `CREATE TABLE IF NOT EXISTS` 不会修改旧表。
- 现有迁移已通过 `PRAGMA table_info(groups)` 为旧表补充 `auto_failover`，但没有补充 `created_at`。
- 旧库可能同时缺少 `auto_failover` 和 `created_at`，迁移必须同时兼容。

## 要求

- 启动迁移时检测 `groups.created_at` 是否存在。
- 缺失时添加非空文本列，并为历史行填充可用的 UTC RFC3339 时间值（推荐使用迁移执行时间）。
- 已存在时迁移幂等，不重复添加、不改变已有时间值。
- 不删除、不重建、不覆盖现有 `groups` 和 `group_items` 数据。
- 保留并扩展回归测试：旧表缺少 `auto_failover` 与 `created_at` 时，经 `open_db` 后可以查询分组和条目；重复迁移成功；已有字段值保持不变。

## 验收标准

- [x] 旧 `groups` 表缺少 `created_at` 时，`open_db` 成功。
- [x] `list_groups` 和 `get_group_by_name` 不再报 `no such column: created_at`。
- [x] 历史分组的 `created_at` 为非空有效 RFC3339 文本。
- [x] 已有 `created_at` 与 `auto_failover=0` 的值不会被覆盖。
- [x] 既有分组条目数量、供应商关联和上游模型名称保持不变。
- [x] 迁移重复执行不报错且不新增重复列。
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` 通过。

## 范围外

- 不迁移其他历史表字段。
- 不恢复旧数据库无法提供的真实创建时间；缺失时间统一使用迁移时生成的 UTC 时间。
- 不修改前端分组逻辑、不改变数据库路径。
