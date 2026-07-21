# Database Guidelines

> 持久化约定（Vue3 重写后）。

---

## Overview

| 项 | 约定 |
|----|------|
| 引擎 | **SQLite only** |
| 位置 | 应用数据目录（代理 `gateway_dir` 下 `data.db` 或等价） |
| 所有者 | 进程内代理 / domain 层读写 |
| 迁移 | 新栈 schema v1；**不**兼容旧 gateway-rust 库自动迁移 |

---

## Schema v1（`src-tauri/src/db/migrate.rs`）

| 表 | 用途 |
|----|------|
| `schema_migrations` | version + applied_at |
| `providers` | 上游供应商；`api_key` 本机可明文 |
| `groups` | 对外模型名；`auto_failover` |
| `group_items` | 有序队列；`sort_order` 越小越优先 |
| `api_keys` | 客户端 Key：`key_hash` + `masked`，无明文 |
| `request_logs` | 请求/故障转移摘要；不存 messages/完整密钥 |

---

## 密钥边界

| 种类 | 存储 |
|------|------|
| 客户端 `sk-modelhub-...` | 仅哈希 + 脱敏 |
| 上游 Provider Key | 可明文存 SQLite |
| 日志 | 禁完整 Key / messages |

---

## Anti-Patterns

- 壳与代理各维护一份业务库。
- 客户端 Key 明文写进前端 localStorage。
- MVP 引入第二数据库引擎。

---

## Verification

- migrate 幂等；`cargo test` 含 domain CRUD 与临时库。
