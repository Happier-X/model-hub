# Database Guidelines

> 持久化约定（Vue3 重写后）。

---

## Overview

| 项 | 约定 |
|----|------|
| 引擎 | **SQLite only** |
| 位置 | 应用数据目录（代理 `gateway_dir` 下 `data.db` 或等价） |
| 所有者 | 进程内代理 / domain 层读写 |
| 迁移 | 当前 schema v1；不执行旧版数据库自动迁移 |

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

## 场景：为既有 SQLite 表补充字段

### 1. 范围 / 触发条件

当新版本查询依赖既有表中的新字段，而 `CREATE TABLE IF NOT EXISTS` 不会修改已存在的表时，必须在启动迁移中补充字段。已覆盖示例：`groups.auto_failover` / `groups.created_at`、`group_items` 的旧 `channel_id/model_name/priority` 到新 `provider_id/upstream_model/sort_order` 兼容、`api_keys` 的旧 `api_key_masked` 到新 `masked/created_at` 兼容，以及 `request_logs` 的 `status_code`、`use_time_ms`、failover 等列。

### 2. 签名

- Rust：`fn migrate(conn: &rusqlite::Connection) -> Result<(), AppError>`
- Rust：`fn ensure_group_auto_failover(conn: &rusqlite::Connection) -> Result<(), AppError>`
- SQLite：`ALTER TABLE groups ADD COLUMN auto_failover INTEGER NOT NULL DEFAULT 1`

### 3. 契约

- 迁移先用一次 `PRAGMA table_info(groups)` 收集列名，再分别决定是否执行 `ALTER TABLE`。
- `auto_failover` 缺失时新增 `INTEGER NOT NULL DEFAULT 1`；SQLite 会为既有行提供默认值 `1`。
- `created_at` 缺失时新增 `TEXT NOT NULL DEFAULT '<迁移时 UTC RFC3339>'`；历史行使用迁移时间，不能虚构原始创建时间。
- SQLite 的 `ALTER TABLE ... ADD COLUMN ... DEFAULT` 不能参数化动态默认值；只能拼接经过 SQL 单引号转义的应用生成值。UTC RFC3339 时间仍应执行单引号转义。
- 已有列时不得执行重复添加，也不得更新既有值。
- 迁移不得重建或删除 `groups`、`group_items`，不得覆盖既有业务数据。
- 迁移重复执行必须成功。

### 4. 校验与错误矩阵

| 条件 | 行为 |
|------|------|
| `groups` 缺少 `auto_failover` | 添加列，既有行读取为 `1` |
| `groups` 缺少 `created_at` | 添加列，既有行为非空有效 RFC3339 迁移时间 |
| 旧 `group_items` 只有 `channel_id/model_name/priority` | 补 `provider_id/upstream_model/sort_order` 并条件回填；旧列保留。应用写新条目时检测旧列并同步双写，满足旧 NOT NULL 约束 |
| 旧 `api_keys` 只有 `api_key_masked` 无 `masked/created_at` | 补 `masked/created_at`；`masked` 从 `api_key_masked` 条件回填；`key_hash` 不覆盖。创建 Key 时若仍有 `api_key_masked` 则同步双写 |
| `request_logs` 缺少 `status_code` 等业务列 | `ensure_request_logs_columns` 逐列 `ALTER TABLE ... ADD COLUMN`，默认值与 schema v1 一致；既有行保留 |
| 字段已经存在 | 跳过添加，保留所有值 |
| `PRAGMA table_info` 或读取字段失败 | 返回 `AppError::Database` |
| `ALTER TABLE` 失败 | 返回 `AppError::Database` |
| 重复执行迁移 | 成功且不产生重复列 |

### 5. 正常 / 基线 / 异常案例

- 正常：旧表有分组和条目但同时缺少 `auto_failover`、`created_at`，`open_db` 后可通过 list/get 查询；默认 `auto_failover=1`，`created_at` 是有效 RFC3339，条目仍存在。
- 基线：新表首次创建并重复 `migrate`，两次均成功。
- 异常：旧表已有 `auto_failover=0` 和原始 `created_at`，迁移后两者都保持不变；禁止用默认值更新覆盖它们。

### 6. 必要测试

- 迁移单测：旧 `groups` 同时缺少 `auto_failover`、`created_at` 时添加列，保留分组；默认值为 `1`，时间可按 RFC3339 解析，重复迁移不改变迁移时间。
- 迁移单测：已有 `auto_failover=0`、原始 `created_at` 时重复迁移不改变值。
- 迁移单测：旧 `group_items` 补齐并从 `channel_id/model_name/priority` 回填；已有新字段值不覆盖；重复迁移成功。
- 领域测试：保留旧 NOT NULL 列的兼容表，应用新增/更新分组条目时同步双写新旧列。
- 迁移单测：旧 `api_keys` 补齐 `masked/created_at`，从 `api_key_masked` 回填；已有值与 `key_hash` 不覆盖。
- 领域测试：迁移后的兼容表可 `list_api_keys` / 创建 Key，且双写 `masked` 与 `api_key_masked`。
- 迁移单测：残缺 `request_logs` 缺 `status_code` 等列时补齐，保留既有行；重复迁移成功。
- 数据库集成单测：通过 `open_db` 升级旧库后，`list_groups` 与 `get_group_by_name` 均可读，且 `group_items` 数量、供应商关联与上游模型保持不变。

### 7. 错误与正确做法

#### 错误

```sql
CREATE TABLE IF NOT EXISTS groups (... auto_failover ...);
```

它不会为已经存在的旧表添加新列；直接查询新列可能产生 `no such column`。

#### 正确

```text
PRAGMA table_info(groups) → 不存在时 ALTER TABLE ADD COLUMN
```

只做一次结构性补充，依靠列默认值填充旧行，并用回归测试验证数据保留与幂等性。
