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
| `groups` | 对外模型名（**无** `auto_failover`；故障转移始终按队列顺序） |
| `group_items` | 有序队列；`sort_order` 越小越优先 |
| `request_logs` | 请求/故障转移摘要；不存 messages/完整密钥；**默认保留最近 7 天内的最新 1000 条**（`LOG_RETENTION_DAYS` + `LOG_MAX_ROWS`），启动/写日志/列表时 best-effort 清理过期或超量 |

**已移除**：

- 客户端 `api_keys` 表。新库不创建；启动路径**不**读、**不**迁移、**不** DROP 旧库残留 `api_keys`。
- `groups.auto_failover`。新库不创建；旧库迁移删除该列并保留分组数据。

---

## 密钥边界

| 种类 | 存储 |
|------|------|
| 客户端 API Key | **不存储**；`/v1/*` 不校验客户端 Key |
| 上游 Provider Key | 可明文存 SQLite（`providers.api_key`） |
| 日志 | 禁完整 Key / messages |

---

## Anti-Patterns

- 壳与代理各维护一份业务库。
- 客户端 Key 明文写进前端 localStorage。
- MVP 引入第二数据库引擎。
- 为已删除的 `api_keys` 写 DROP/兼容迁移路径。
- 新库或领域模型重新引入 `auto_failover`。

---

## Verification

- migrate 幂等；`cargo test` 含 domain CRUD 与临时库。

## 场景：请求日志保留策略

### 1. Scope / Trigger

当修改请求日志存储、列表、手动清理或 UI 展示时，必须保持同一套保留策略：只保留最近 7 天内的最新 1000 条。该策略控制本地 SQLite 体积，且不能影响代理请求主路径。

### 2. Signatures

- Rust：`pub const LOG_RETENTION_DAYS: i64 = 7`
- Rust：`pub const LOG_MAX_ROWS: i64 = 1000`
- Rust：`Stores::purge_expired_logs() -> Result<LogPurgeResult, AppError>`
- Rust：`Stores::purge_logs(retention_days: i64, max_rows: i64) -> Result<LogPurgeResult, AppError>`
- IPC response：`LogPage { retention_days, max_rows, stored_total, ... }`
- IPC response：`LogPurgeResult { deleted, retained, retention_days, max_rows, cutoff_unix }`

### 3. Contracts

- 默认清理必须同时执行时间窗口和数量上限：`time < now - LOG_RETENTION_DAYS * 86400` 的行删除；剩余行再按 `id DESC` 仅保留最新 `LOG_MAX_ROWS` 条。
- 自动清理入口包括启动打开库、写日志成功后、日志列表查询前；这些入口必须 best-effort，失败只写 tracing warn，不阻断请求或 UI 列表。
- 手动清理 `purge_expired_logs` 与自动清理使用同一默认策略，不得只按天数清理。
- `list_logs` 返回的 `retention_days` 与 `max_rows` 是 UI 展示的唯一来源；前端可有兼容旧后端的本地回退值，但文案不得写旧的 30 天策略。
- `purge_logs_older_than_days` 仅用于按天数测试或特殊调用；默认路径不得绕过 `LOG_MAX_ROWS`。

### 4. Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| 日志超过 7 天 | 删除 |
| 日志在 7 天内但不属于最新 1000 条 | 删除 |
| 日志在 7 天内且属于最新 1000 条 | 保留 |
| `retention_days <= 0` | 按 1 天处理 |
| `max_rows <= 0` | 按 1 条处理 |
| SQLite `DELETE` / `COUNT` 失败 | 返回 `AppError::Database`；best-effort 入口只 warn |

### 5. Good / Base / Bad Cases

- Good：1 条 8 天前记录 + 1002 条当天记录，默认清理后删除 3 条、保留 1000 条。
- Base：空日志表清理成功，`deleted = 0`、`retained = 0`。
- Bad：只改 `LOG_RETENTION_DAYS` 为 7，未加 `LOG_MAX_ROWS`，导致 7 天内高频请求无限增长。

### 6. Tests Required

- 领域测试：`purge_logs(7, 1000)` 同时删除过期行和超出最新 1000 条的行，断言 `deleted`、`retained`、`retention_days`、`max_rows`。
- 领域测试：`purge_expired_logs()` 使用 `LOG_RETENTION_DAYS` 与 `LOG_MAX_ROWS` 默认值。
- 领域测试：`list_logs` 返回 `max_rows == LOG_MAX_ROWS` 与 `retention_days == LOG_RETENTION_DAYS`。
- 前端类型检查：`LogPage` 与 `LogPurgeResult` 包含 `max_rows`，日志页文案展示 7 天和 1000 条。

### 7. Wrong vs Correct

#### 错误

```rust
pub const LOG_RETENTION_DAYS: i64 = 7;

conn.execute("DELETE FROM request_logs WHERE time < ?1", params![cutoff])?;
```

只按天数删除，无法限制 7 天内的日志数量。

#### 正确

```rust
pub const LOG_RETENTION_DAYS: i64 = 7;
pub const LOG_MAX_ROWS: i64 = 1000;

stores.purge_logs(LOG_RETENTION_DAYS, LOG_MAX_ROWS)?;
```

默认路径同时应用时间窗口和最新条数上限，UI 从 IPC 响应展示当前策略。

---

## 场景：为既有 SQLite 表补充字段

### 1. 范围 / 触发条件

当新版本查询依赖既有表中的新字段，而 `CREATE TABLE IF NOT EXISTS` 不会修改已存在的表时，必须在启动迁移中补充字段。已覆盖示例：`groups.created_at`、`group_items` 的旧 `channel_id/model_name/priority` 到新 `provider_id/upstream_model/sort_order` 兼容，以及 `request_logs` 的 `status_code`、`use_time_ms`、failover 等列。

**不再**覆盖：`api_keys` 缺列迁移；**不再**向 `groups` **添加** `auto_failover`。

### 2. 签名

- Rust：`fn migrate(conn: &rusqlite::Connection) -> Result<(), AppError>`
- Rust：`fn ensure_group_columns(conn: &rusqlite::Connection) -> Result<(), AppError>`
- SQLite：`ALTER TABLE groups ADD COLUMN created_at TEXT NOT NULL DEFAULT '...'`（仅缺列时）

### 3. 契约

- 迁移先用一次 `PRAGMA table_info(groups)` 收集列名，再分别决定是否执行 `ALTER TABLE`。
- `created_at` 缺失时新增 `TEXT NOT NULL DEFAULT '<迁移时 UTC RFC3339>'`；历史行使用迁移时间，不能虚构原始创建时间。
- SQLite 的 `ALTER TABLE ... ADD COLUMN ... DEFAULT` 不能参数化动态默认值；只能拼接经过 SQL 单引号转义的应用生成值。
- 已有列时不得执行重复添加，也不得更新既有值。
- 除「删除已废弃列」场景外，迁移不得重建或删除 `groups`、`group_items`，不得覆盖既有业务数据。
- 迁移重复执行必须成功。

### 4. 校验与错误矩阵

| 条件 | 行为 |
|------|------|
| `groups` 缺少 `created_at` | 添加列，既有行为非空有效 RFC3339 迁移时间 |
| 旧 `group_items` 只有 `channel_id/model_name/priority` | 补 `provider_id/upstream_model/sort_order` 并条件回填；旧列保留。应用写新条目时检测旧列并同步双写，满足旧 NOT NULL 约束 |
| `request_logs` 缺少 `status_code` 等业务列 | `ensure_request_logs_columns` 逐列 `ALTER TABLE ... ADD COLUMN`，默认值与 schema v1 一致；既有行保留 |
| 旧 `request_logs` 仍有 `request_model_name` / `channel_name` / `actual_model_name` / `use_time` | 条件回填到 `group_name` / `provider_name` / `upstream_model` / `use_time_ms`；`insert_log` 检测旧列并双写，避免 NOT NULL 失败 |
| 字段已经存在 | 跳过添加，保留所有值 |
| `PRAGMA table_info` 或读取字段失败 | 返回 `AppError::Database` |
| `ALTER TABLE` 失败 | 返回 `AppError::Database` |
| 重复执行迁移 | 成功且不产生重复列 |

### 5. 正常 / 基线 / 异常案例

- 正常：旧表有分组和条目但缺少 `created_at`，`open_db` 后可通过 list/get 查询；`created_at` 是有效 RFC3339，条目仍存在。
- 基线：新表首次创建并重复 `migrate`，两次均成功。
- 异常：旧表已有原始 `created_at`，迁移后保持不变。

### 6. 必要测试

- 迁移单测：旧 `groups` 缺少 `created_at` 时添加列，保留分组；时间可按 RFC3339 解析，重复迁移不改变迁移时间。
- 迁移单测：旧 `group_items` 补齐并从 `channel_id/model_name/priority` 回填；已有新字段值不覆盖；重复迁移成功。
- 领域测试：保留旧 NOT NULL 列的兼容表，应用新增/更新分组条目时同步双写新旧列。
- 迁移单测：残缺 `request_logs` 缺 `status_code` 等列时补齐，保留既有行；重复迁移成功。
- 迁移单测：旧 `request_model_name/channel_name/actual_model_name/use_time` 回填到当前列；重复迁移不覆盖。
- 领域测试：兼容表 `insert_log` 双写旧列，list/stats 可读。
- 数据库集成单测：通过 `open_db` 升级旧库后，`list_groups` 与 `get_group_by_name` 均可读，且 `group_items` 数量、供应商关联与上游模型保持不变。

### 7. 错误与正确做法

#### 错误

```sql
CREATE TABLE IF NOT EXISTS groups (...);
-- 假定旧表已有新列
SELECT created_at FROM groups;
```

它不会为已经存在的旧表添加新列；直接查询新列可能产生 `no such column`。

#### 正确

```text
PRAGMA table_info(groups) → 不存在时 ALTER TABLE ADD COLUMN
```

只做一次结构性补充，依靠列默认值填充旧行，并用回归测试验证数据保留与幂等性。

---

## 场景：删除已废弃的 `groups.auto_failover`

### 1. Scope / Trigger

- Trigger：旧库 `groups` 仍含 `auto_failover`，而领域查询与新 schema 已不再使用该列。
- 目标：删除列且完整保留 `id` / `name` / `created_at` 与 `group_items` 关联。

### 2. Signatures

- Rust：`fn drop_groups_auto_failover_if_present(conn: &rusqlite::Connection) -> Result<(), AppError>`
- 由 `migrate` 调用；幂等。

### 3. Contracts

- 若列不存在：立即成功返回。
- 若列存在：在事务内重建 `groups`（仅 `id/name/created_at`），拷贝数据，替换旧表，校验列已消失且 `group_items` 无孤儿行。
- 操作期间正确处理 `foreign_keys`（关闭→重建→恢复），失败返回 `AppError::Database`。
- 不得丢失分组行与条目；`group_items.group_id` 外键与 CASCADE 行为保持可用。
- 领域 `Group` / 创建更新 payload **不得**再读写 `auto_failover`。

### 4. Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| 无 `auto_failover` 列 | 跳过 |
| 有 `auto_failover` 列 | 重建表删除列并拷贝数据 |
| 重建后列仍存在 / 出现孤儿 `group_items` | 失败 `AppError::Database` |
| 重复 migrate | 成功且数据不变 |

### 5. Good / Base / Bad Cases

- **Good**：含 `auto_failover=0` 的旧分组迁移后仍可 list，条目数量与 `provider_id`/`upstream_model` 不变。
- **Base**：新库自始无该列，migrate 幂等。
- **Bad**：`CREATE TABLE IF NOT EXISTS` 继续带 `auto_failover`；或删列后丢 `group_items`。

### 6. Tests Required

- 迁移单测：旧表含 `auto_failover` 时删列并保留分组/条目；重复 migrate 成功。
- 迁移单测：删列后外键 CASCADE 仍生效（删分组清条目）。
- 领域测试：`list_groups` / create / update 不依赖 `auto_failover`。

### 7. Wrong vs Correct

#### Wrong

```sql
-- 仅停止读写但列永久残留，且新 schema 仍声明 auto_failover
CREATE TABLE IF NOT EXISTS groups (..., auto_failover INTEGER NOT NULL DEFAULT 1, ...);
```

#### Correct

```text
PRAGMA table_info(groups) → 存在 auto_failover 时事务内重建表拷贝 id/name/created_at → 校验无孤儿 group_items
```
