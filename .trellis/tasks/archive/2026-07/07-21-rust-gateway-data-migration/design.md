# 设计：octopus 数据迁移

## CLI

clap 改为 subcommand：

```text
model-hub-gateway serve --config data/config.json   # 默认也可无 subcommand 保持兼容
model-hub-gateway migrate-octopus --source X --dest Y [--force] [--with-logs]
```

**兼容**：保留当前 `model-hub-gateway --config` 无 subcommand 即 serve（clap 可用默认 command 或扁平 arg）。  
推荐：`#[command(subcommand)]` optional；若无 subcommand 则 serve。

## 检测源库

存在 `migration_records` 或 `users` 表且存在 `channels.base_urls` 列 → 视为 octopus。  
若源已有 rust `schema_migrations` 且无 octopus 特征 → 报错「已是 rust 库」。

## 导入步骤

1. 打开 source 只读  
2. 打开 dest，`PRAGMA foreign_keys=ON`，`migrate()`  
3. 若 dest 业务表非空且无 force → Err  
4. force：DELETE 业务表（保留 schema）  
5. 事务导入：

**channels**  
- 读 channels 行  
- INSERT channels（映射 type/enabled/model/...）  
- parse base_urls JSON array → channel_base_urls  
- channel_keys by channel_id  

**groups / group_items**  
- 按 id 插入  

**api_keys**  
- raw = api_key 列  
- hash_key(raw), mask_key(raw)  
- supported_models: 空字符串→None；JSON 数组则解析  

**relay_logs → request_logs**（--with-logs）  
- time, request_model_name, channel_name, actual_model_name, tokens, use_time, cost, error  

6. commit；打印计数摘要  

## 模块

```text
migrate_octopus/
  mod.rs
  detect.rs
  import.rs
```

## 测试

构造 octopus 风格表结构 + 样例行 → dest → assert counts + hash_key matches。
