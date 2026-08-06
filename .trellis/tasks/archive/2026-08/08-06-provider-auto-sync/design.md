# 设计：供应商级自动同步模型

## 数据模型

### providers 表（ALTER）
```
id          INTEGER PRIMARY KEY AUTOINCREMENT   (已有)
name        TEXT NOT NULL                       (已有)
base_url    TEXT NOT NULL                       (已有)
api_key     TEXT NOT NULL DEFAULT ''            (已有)
enabled     INTEGER NOT NULL DEFAULT 1          (已有)
created_at  TEXT NOT NULL                       (已有)
auto_sync   INTEGER NOT NULL DEFAULT 1          (新增)
last_sync_at INTEGER                            (新增, NULL=未同步)
```

### provider_models 表（CREATE）
```
id          INTEGER PRIMARY KEY AUTOINCREMENT
provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE
model_name  TEXT NOT NULL
sort_order  INTEGER NOT NULL DEFAULT 0
UNIQUE(provider_id, model_name)
```

### groups 表
`source_provider_id` / `last_sync_at` 字段**保留不删**，但迁移后不再写入、不再读取用于同步。分组队列回到纯手动维护。

## 后端同步流程

```
runtime.rs 后台循环 (每1h检查)
  └─ perform_due_provider_syncs(stores)
       for p in stores.list_providers():
         if !p.enabled || !p.auto_sync: continue
         due = p.last_sync_at is None || now - last_sync_at >= 24h
         if !due: continue
         sleep(stagger 5s) // 错峰
         perform_sync_provider(stores, p.id)
              └─ fetch_upstream_model_ids(base_url, api_key)
              └─ stores.replace_provider_models(provider_id, ids)
              └─ stores.touch_provider_synced_at(provider_id, now)
```

关键：`enabled=false` 或 `auto_sync=false` 直接跳过；单供应商同步失败只记录 warning 不影响其他。

## 前端数据流

### 供应商页（ProvidersPage）
- 表格列：名称 / Base URL / 启用 / 自动同步 / 上次同步 / 操作
- 「自动同步」列：`HSwitch` 就地切换 → `setProviderAutoSync(id, v)` → 刷新列表
- 「上次同步」列：`last_sync_at` 格式化，null → 「未同步」
- 「操作」列：新增「立即同步」按钮 → `syncProviderNow(id)` → loading → 刷新列表

### 分组页左侧（GroupFormPage + useProviderModelCache）
`useProviderModelCache` 改造：
- `ensure(providerId)`：先读 `getProviderModels(providerId)`（本地持久化）。非空 → ready + 直接展示；空 → 实时 `fetchProviderModels` 兜底
- `refresh(providerId)`：实时拉取（保留，供「重试/刷新」按钮）

供应商条目 HCell #suffix：显示同步状态小字（已同步时间 / 未同步），来源 `providers.last_sync_at`。

### GroupFormPage 清理
- 移除：`source_provider_id` 绑定下拉（`form.Field source_provider_id`）、绑定态提示块（v-if isBound 紫块 + 立即同步按钮）、`boundLastSyncText`、`handleSyncNow`、`loadedGroupLastSyncAt`、`isBound` computed
- 移除各处 `isBound` 分支的禁用逻辑（拖拽、加模型、删除、清空、排序均恢复可交互）
- `bindProviderOptions` 移除
- 保留：手动队列增删排序、能力排序、提交 payload 中不再带 `source_provider_id`
- `Group` 前端类型仍可含 `source_provider_id`（后端返回），但表单不再使用

## API 层（src/api/tauri.ts）

```ts
// 新增
setProviderAutoSync(id: number, enabled: boolean): Promise<Provider>
syncProviderNow(id: number): Promise<Provider>
getProviderModels(id: number): Promise<string[]>

// 移除
syncGroupNow
```

`Provider` 类型：
```ts
type Provider = {
  id: number; name: string; base_url: string; api_key: string; enabled: boolean;
  auto_sync: boolean; last_sync_at: number | null; created_at: string;
}
```

## 命令调整汇总（commands.rs）

| 命令 | 动作 |
|------|------|
| `sync_provider_now` | 新增，立即同步单个供应商 |
| `get_provider_models` | 新增，读本地持久化模型 |
| `set_provider_auto_sync` | 新增，就地切换开关 |
| `sync_group_now` | 移除 |
| `fetch_provider_models` | 保留（草稿预览 + 兜底刷新） |
| `list_providers` | 返回含 auto_sync / last_sync_at |

## 迁移实现要点

沿用 migrate.rs 模式：
```rust
// 1. providers 加列（幂等）
if !columns.contains("auto_sync") { ALTER TABLE providers ADD COLUMN auto_sync INTEGER NOT NULL DEFAULT 1 }
if !columns.contains("last_sync_at") { ALTER TABLE providers ADD COLUMN last_sync_at INTEGER }
// 2. provider_models 建表（IF NOT EXISTS）
```
注意 `ALTER TABLE ADD COLUMN` 带 `NOT NULL DEFAULT` 在 SQLite 是合法的（常量默认值）。

## 风险与回退

- 旧库有绑定 source_provider_id 的分组：升级后绑定静默失效，队列保持上次同步的模型不变（不丢数据），用户可手动编辑。回退：保留字段不删，可随时恢复旧逻辑。
- provider_models 全量替换采用事务：失败回滚，旧模型列表保留。
