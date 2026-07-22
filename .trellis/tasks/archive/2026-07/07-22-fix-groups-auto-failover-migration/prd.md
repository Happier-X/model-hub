# 修复 groups.auto_failover 数据库迁移

## 目标

修复已有 SQLite 数据库启动后查询 `groups.auto_failover` 失败的问题，确保旧版当前架构数据库在升级迁移后可以正常读取分组，同时保留已有用户数据。

## 背景

新代码的建表 SQL 包含 `auto_failover`，但 `CREATE TABLE IF NOT EXISTS` 不会修改已存在的 `groups` 表。旧表缺少该列时，`list_groups` / `get_group_by_name` 查询会报：

`no such column: auto_failover`

## 要求

- 启动迁移时检测 `groups.auto_failover` 是否存在。
- 缺失时添加 `INTEGER NOT NULL DEFAULT 1` 列。
- 已存在时迁移必须幂等，不重复添加、不改变已有值。
- 不删除、不重建、不覆盖现有分组和分组条目数据。
- 增加回归测试：旧表缺列经过迁移后可查询，默认值为 1；重复迁移成功。

## 验收标准

- [x] 缺少 `auto_failover` 的旧 `groups` 表经 `open_db` 后可正常查询分组。
- [x] 已有分组数据保留，`auto_failover` 默认值为启用。
- [x] 已有 `auto_failover=0` 的值不会被覆盖。
- [x] 迁移重复执行不报错。
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` 通过。

## 范围外

- 不迁移旧 `gateway-rust` 的其他历史 schema。
- 不修改前端分组逻辑。
- 不改变数据库路径与数据目录策略。
