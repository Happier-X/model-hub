# 技术设计：自动同步供应商模型到分组

## 架构边界

- **数据模型**：`groups` 表增加 `source_provider_id` 列。通过 `ensure_group_columns` 添加。
- **定时调度**：依托于 `ProxyHandle.tokio_rt`。因为代理随时可能启动停止（修改端口时会先停后启），把定时器绑定在 `proxy/runtime.rs` 的代理生命周期里（`server::serve` 或其同级任务）最稳妥，代理运行期间定时器才跑。
- **IPC 契约**：前端通过 `source_provider_id` 提交绑定，通过 `sync_group_now` 触发立即刷新。
- **防封号例外**：`upstream-access.md` 记录本次特性为合规例外，仅针对绑定分组且延时 24h 启动。

## 详细设计

### 1. 数据库改动 (`src-tauri/src/db/migrate.rs` / `group.rs`)

`ensure_group_columns` 增加检测并执行：
```sql
ALTER TABLE groups ADD COLUMN source_provider_id INTEGER
```
因为是 `INTEGER` 允许 `NULL`，直接添加即可，无需 `DEFAULT`（SQLite 对可空列默认 `NULL`）。

如果一个 provider 被删除了（`DELETE FROM providers WHERE id=?`），需要确保 `groups.source_provider_id` 被置空。由于 SQLite 不好在应用层不知情时直接级联改异表非外键列（若加外键 `SET NULL` 则需开启外键约束并修改建表语句，但之前 `groups` 是独立创建的），最简单的是在 `domain/provider.rs` 的 `delete_provider` 里手动补一刀：
```rust
conn.execute("UPDATE groups SET source_provider_id = NULL WHERE source_provider_id = ?1", [id])
```

`domain::Group` 结构体：
```rust
pub struct Group {
    // ...
    pub source_provider_id: Option<i64>,
}
```

### 2. 领域逻辑 (`src-tauri/src/domain/group.rs` 或同级)

新增方法：
```rust
pub async fn sync_bound_group(&self, group_id: i64) -> Result<(), AppError>
pub async fn sync_all_bound_groups(&self) -> Result<(), AppError>
```

由于 `fetch_upstream_model_ids` 是 `async`，`domain` 层必须提供异步上下文，或者在 command/proxy 顶层组织。
现有 `fetch_upstream_model_ids` 定义在 `domain/upstream_models.rs`，可以直接在 `commands.rs` 里或者独立的定时任务模块里组织：先从 `Stores` 查出 `source_provider_id`，再发起异步网络请求，拿到结果后回存 `Stores`。保持 `Stores` 本身不涉及网络（同现有模式），把网络和持久化的胶水代码写在 `proxy/runtime.rs` 启动的任务或者某个服务方法里。

**步骤**（胶水层）：
1. 获取 `group`（检查是否存在，以及 `source_provider_id`）。
2. 获取 `provider`（检查是否存在，以及 `enabled`）。
3. 如果 `!provider.enabled`，跳过。
4. 调用 `fetch_upstream_model_ids(provider.base_url, provider.api_key).await`。
5. 成功则组装 `Vec<GroupItemInput>`，调 `stores.update_group_items`（原内部 `replace_items` 可略微重构复用）。
6. 失败则返回/记录错误。

### 3. 定时调度 (`src-tauri/src/proxy/runtime.rs`)

代理成功启动时（`self.tokio_rt.spawn` 内，与 `server::serve` 并列或在内部）：
```rust
let stores = stores_clone;
self.tokio_rt.spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
    interval.tick().await; // skip 第一次（启动时立刻触发的）
    loop {
        interval.tick().await; // 等待 24 小时
        if let Err(e) = perform_sync_all_bound_groups(&stores).await {
            tracing::warn!(error = %e, "后台定时同步供应商模型失败");
        }
    }
});
```
跟随代理的启停而启停（或者绑定在一颗取消信号树上，随 `shutdown_rx` 退出）。

### 4. IPC 命令 (`src-tauri/src/commands.rs`)

```rust
#[tauri::command]
pub async fn sync_group_now(proxy: State<'_, ProxyHandle>, group_id: i64) -> Result<Group, InvokeError> {
    // 胶水逻辑，成功后返回最新 Group
}
```
并且在 `create_group` / `update_group` payload 加入 `source_provider_id: Option<i64>`。

### 5. 前端适配 (`src/pages/GroupsPage.vue`)

- 编辑分组弹窗，在「名称」下方加一项：「绑定供应商自动同步」下拉框（含「无 / 不绑定」选项）。
- 切换为某个供应商时，提示「绑定后，该分组将被供应商托管，每 24h 自动全量覆盖更新模型。您无法再手动增删模型条目。」
- 列表区只读控制：`is_bound = form.values.source_provider_id != null`
  - 如果 `is_bound`：
    - 隐藏每行的垃圾桶和拖拽手柄。
    - 隐藏「添加模型」按钮、「批量添加」区。
    - 表格上方显示警告：本分组由供应商托管，仅供只读浏览。
    - 显示「立即同步」按钮（调用 `syncGroupNow`）。
  - 如果非 `is_bound`：恢复正常交互。

### 6. Spec 更新

- `upstream-access.md` 增加允许后台对配置了 `source_provider_id` 的启用供应商定时（24h）请求 `/models` 的例外规则。
- `component-guidelines.md` 略加标注。

## 兼容性与回滚

- 数据库升级只加可空列，不破坏旧数据。如果回滚应用版本，旧代码查询只是忽略多出的列，不影响运行。
- 如果前端取消绑定，只需把该列置 `NULL`，既有模型数据保留变为手编形态，符合心智。
