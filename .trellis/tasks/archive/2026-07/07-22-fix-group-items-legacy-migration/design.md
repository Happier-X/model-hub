# 设计

采用**加列 + 条件回填**而非重建表：SQLite 在开启外键时为旧表直接补 REFERENCES 列存在限制，因此兼容列使用普通 `INTEGER NOT NULL DEFAULT 0`；新建数据库仍由建表 SQL获得正式外键约束。

迁移顺序：

1. `MIGRATION_V1` 确保表存在。
2. `PRAGMA table_info(group_items)`。
3. 补当前三列。
4. 若旧列存在，执行条件回填。
5. group / request_logs 其他兼容迁移。

回填语义：

```sql
UPDATE group_items SET provider_id = channel_id WHERE provider_id = 0;
UPDATE group_items SET upstream_model = model_name WHERE upstream_model = '';
UPDATE group_items SET sort_order = COALESCE(priority, 0) WHERE sort_order = 0;
```

注意：priority=0 与默认 0 无法区分，但重复赋相同值幂等。

旧表的 `channel_id` / `model_name` 是 NOT NULL 且无默认值，SQLite 加列无法移除其约束。为避免重建和外键风险，`domain::group::replace_items` 在检测到旧列时同时写入新旧两套列；新建 schema 仍只写当前列。
