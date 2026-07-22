# 设计

采用加列 + 回填，不重建表。

1. `PRAGMA table_info(api_keys)`。
2. 若缺 `masked`，添加 `TEXT NOT NULL DEFAULT ''`。
3. 若有 `api_key_masked`，仅对 `masked=''` 行回填。
4. 若缺 `created_at`，添加迁移时 RFC3339 默认值。
5. 其他当前基础列若缺失，按安全默认补齐；但不从明文旧 Key 自动生成哈希。
6. `create_api_key` 检测 `api_key_masked`，旧兼容表同步双写；新表走当前 INSERT。

安全：错误/日志不输出 Key；旧 hash 不覆盖。
