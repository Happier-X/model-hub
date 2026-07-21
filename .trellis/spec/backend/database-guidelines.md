# Database Guidelines

> 持久化约定（MVP）。

---

## Overview

| 项 | MVP 约定 |
|----|----------|
| 引擎 | **SQLite only** |
| 位置 | 应用数据目录下（如 `data/data.db` 或侧车默认相对数据目录） |
| MySQL / PostgreSQL | **不做**（后续阶段） |
| 所有者 | 侧车/网关进程读写；壳只负责目录可写与路径注入 |

---

## Patterns

1. **单一数据目录**：配置、`*.db`、缓存/临时文件放在同一 app data 根下，便于备份与卸载说明。
2. **启动注入**：壳通过 CLI 参数或环境变量把 data dir 传给侧车，避免侧车写到 cwd。
3. **迁移**：侧车/网关负责 schema；应用升级时允许自动 migrate；禁止手改生产库当常规流程。
4. **优雅退出**：退出前信号通知侧车 flush（统计内存批量写库）；禁止默认 `kill -9` 式强杀作为正常退出路径。

---

## gateway-rust SQLite 契约

### Scope / Trigger

修改 `gateway-rust` 持久化、API Key / 渠道 / 分组表结构或打开路径时遵守本节。

### 打开与类型

- 仅 `database.type = sqlite`；其它类型配置校验失败并非零退出。
- `database.path` 相对路径相对进程 **cwd**；绝对路径原样使用；启动时创建父目录。
- 打开后 `PRAGMA foreign_keys = ON`，执行 `db/migrate.rs` version=1+2（幂等）。
- 连接策略 MVP：`Arc<Mutex<Connection>>`；handler 内同步短事务，避免跨 await 持锁。
- 测试优先 tempfile 文件库；单连接场景可用 `:memory:`。

### Schema v1+v2（gateway-rust 自有）

| 表 | 用途 |
|----|------|
| `schema_migrations` | version + applied_at |
| `api_keys` | 客户端 Key：哈希 + 脱敏，**无**完整明文 |
| `channels` | 渠道主表；`type` 为 INTEGER |
| `channel_base_urls` | 渠道 URL；`ON DELETE CASCADE` |
| `channel_keys` | 上游 Key 明文（本机数据）；日志禁打完整值 |
| `groups` | 分组；`mode` INTEGER，默认 1（轮询） |
| `group_items` | 分组绑定；`ON DELETE CASCADE` |
| `request_logs`（v2） | chat 请求日志；对齐 UI `RelayLog`；**不**存 messages / 密钥 |

### 密钥边界

| 种类 | 存储 |
|------|------|
| 客户端 `sk-modelhub-...` | 仅 SHA-256 哈希 + 脱敏展示 |
| 上游 `channel_key` | 可明文存 SQLite（与侧车同类本机数据） |
| 管理 JWT secret | 配置或启动随机生成，不进业务表 |

### Verification

- migrate v1+v2 幂等；重启后 `find_by_raw_key` / list 一致。
- 渠道 `type`、分组 `mode` 以数字进出 JSON。
- 请求日志 list `page_size` clamp ≤100；clear 后为空。
- `cargo test --manifest-path gateway-rust/Cargo.toml` 含临时库 CRUD、跨 store 实例持久化与 log 矩阵。

---

## Naming

- 表/列名：`gateway-rust` 使用自有 snake_case schema（见上表）。
- 配置文件：JSON（如 `config.json`），键名稳定、可环境变量覆盖。

---

## Anti-Patterns

- 壳与侧车各维护一份 SQLite 业务库。
- 把客户端 API Key 明文写进前端 localStorage 或 SQLite。
- MVP 引入第二数据库引擎。
- 在日志/错误消息中打印完整客户端 Key 或上游 channel_key。

---

## Verification

- 首次启动后，设置页或文档能指出 DB 路径。
- 杀进程前正常退出，再次启动数据仍在。
