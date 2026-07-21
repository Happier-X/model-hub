# 设计：SQLite 与渠道分组

## 依赖

- `rusqlite`（bundled）精确版本
- 可选 `tempfile` 作 dev-dependency

## Schema（自有，非 1:1 复制 octopus）

```sql
CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE api_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  api_key_masked TEXT NOT NULL,
  key_hash TEXT NOT NULL UNIQUE,
  enabled INTEGER NOT NULL DEFAULT 1,
  expire_at TEXT,
  max_cost REAL,
  supported_models_json TEXT
);

CREATE TABLE channels (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  type INTEGER NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  model TEXT NOT NULL DEFAULT '',
  custom_model TEXT NOT NULL DEFAULT '',
  proxy INTEGER NOT NULL DEFAULT 0,
  auto_sync INTEGER NOT NULL DEFAULT 0,
  auto_group INTEGER NOT NULL DEFAULT 0,
  custom_header_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE channel_base_urls (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  channel_id INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  delay INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE channel_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  channel_id INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
  enabled INTEGER NOT NULL DEFAULT 1,
  channel_key TEXT NOT NULL,
  remark TEXT NOT NULL DEFAULT ''
);

CREATE TABLE groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  mode INTEGER NOT NULL DEFAULT 1,
  match_regex TEXT NOT NULL DEFAULT ''
);

CREATE TABLE group_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  channel_id INTEGER NOT NULL,
  model_name TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 1,
  weight INTEGER NOT NULL DEFAULT 1
);
```

迁移：`db/migrate.rs` 执行 version=1。启用 `PRAGMA foreign_keys = ON`。

## 模块

```text
db/
  mod.rs          # open, migrate, path resolve
  migrate.rs
apikey/sqlite_store.rs
channel/
  mod.rs model.rs store.rs service.rs
group/
  mod.rs model.rs store.rs service.rs
routes/channel.rs routes/group.rs
```

`AppState`：

```rust
pub struct AppState {
  config, auth,
  api_keys: Arc<dyn ApiKeyStore>,
  channels: Arc<ChannelService>,
  groups: Arc<GroupService>,
  // 可选: db: Arc<Mutex<Connection>> 由 service 持有
}
```

连接策略：单连接 `Arc<Mutex<Connection>>` 足够 MVP；避免 rusqlite Connection 跨 await 持锁过久——handler 内同步短事务。

## API 形状

成功一律 `ok_data(value)` → `{ "data": ... }`。

### Channel list 元素

```json
{
  "id": 1,
  "name": "...",
  "type": 0,
  "enabled": true,
  "base_urls": [{"url":"...", "delay":0}],
  "keys": [{"id":1,"channel_id":1,"enabled":true,"channel_key":"...","remark":""}],
  "model": "gpt-4o-mini",
  "custom_model": ""
}
```

create 可返回 `{data: null}` 或创建后对象（UI 用 list 刷新，smoke 只看 200）。

update body 可选字段 +：

```json
"keys_to_update": [{"id":1,"channel_key":"..."}],
"keys_to_add": [{"enabled":true,"channel_key":"...","remark":""}]
```

### Group

```json
{
  "id": 1,
  "name": "smoke-group",
  "mode": 1,
  "match_regex": "",
  "items": [{"id":1,"group_id":1,"channel_id":1,"model_name":"...","priority":1,"weight":1}]
}
```

update：

```json
{
  "id": 1,
  "name": "...",
  "items_to_delete": [1],
  "items_to_add": [{"channel_id":1,"model_name":"...","priority":1,"weight":1}]
}
```

## 错误

- 404 资源：HTTP 404 或 200+业务——对齐 UI 简单路径：**HTTP 404** + message，或 400；推荐 `404`/`400` + 顶层 message。
- 鉴权仍 401。

## 测试

- migrate 幂等
- apikey 持久化：写库 → 新 store 实例读回校验
- channel/group CRUD + keys_to_* / items_to_*
- 集成：login → channel → group → apikey → models 鉴权（扩 auth_matrix 或新文件）
- 临时目录 `data/data.db`，不用 8080

## 回滚

删除 db/channel/group 模块，apikey 回退 Memory；不影响壳。
