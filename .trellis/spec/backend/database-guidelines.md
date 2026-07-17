# Database Guidelines

> 持久化约定（MVP）。

---

## Overview

| 项 | MVP 约定 |
|----|----------|
| 引擎 | **SQLite only** |
| 位置 | 应用数据目录下（如 `data/data.db` 或侧车默认相对数据目录） |
| MySQL / PostgreSQL | **不做**（后续阶段） |
| 所有者 | 侧车进程读写；壳只负责目录可写与路径注入 |

---

## Patterns

1. **单一数据目录**：配置、`*.db`、缓存/临时文件放在同一 app data 根下，便于备份与卸载说明。
2. **启动注入**：壳通过 CLI 参数或环境变量把 data dir 传给侧车，避免侧车写到 cwd。
3. **迁移**：侧车负责 schema；应用升级时允许侧车自动 migrate；禁止手改生产库当常规流程。
4. **优雅退出**：退出前信号通知侧车 flush（统计内存批量写库）；禁止默认 `kill -9` 式强杀作为正常退出路径。

---

## Naming

- 表/列名跟随侧车实现（阶段 1 对齐 octopus 或兼容层）；Rust 重写阶段再统一 snake_case 文档。
- 配置文件：JSON（如 `config.json`），键名稳定、可环境变量覆盖。

---

## Anti-Patterns

- 壳与侧车各维护一份 SQLite 业务库。
- 把 API Key 明文写进前端 localStorage 当唯一存储。
- MVP 引入第二数据库引擎。

---

## Verification

- 首次启动后，设置页或文档能指出 DB 路径。
- 杀进程前正常退出，再次启动数据仍在。
