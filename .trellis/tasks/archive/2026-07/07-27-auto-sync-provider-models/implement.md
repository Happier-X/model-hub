# 执行计划：自动同步供应商模型到分组

## 1. Spec 与指引更新 (Specs)
- [ ] 修改 `.trellis/spec/backend/upstream-access.md`，在允许列表中增加后台定时同步的记录，并明确限制（24h延迟、绑定分组、启用供应商）。
- [ ] 修改 `.trellis/spec/frontend/component-guidelines.md` 第 11 条，补充该例外。

## 2. 数据库迁移与模型 (Database & Domain CRUD)
- [ ] `src-tauri/src/db/migrate.rs`：在 `ensure_group_columns` 中补充检测和 `ALTER TABLE groups ADD COLUMN source_provider_id INTEGER`。
- [ ] `migrate.rs`：在测试模块增加迁移单测 `migrate_adds_missing_source_provider_id_without_losing_data`。
- [ ] `src-tauri/src/domain/group.rs`：更新 `Group`、`CreateGroupPayload`、`UpdateGroupPayload` 增加 `source_provider_id: Option<i64>`。
- [ ] `src-tauri/src/domain/group.rs`：修改 `list_groups`、`get_group_by_name` 的 SQL 提取并设置该列。修改 `create_group` / `update_group` 保存该列。
- [ ] `src-tauri/src/domain/provider.rs`：修改 `delete_provider`，增加 SQL：将 `groups` 里引用了该 provider 的 `source_provider_id` 置 NULL。

## 3. 同步业务胶水逻辑 (Sync Logic)
- [ ] `src-tauri/src/domain/group.rs`：因为 `fetch` 是异步网络请求，最好在 `Stores` 增加供异步外部调用的接口，比如将 `replace_items` 单独抽出一个供外部在事务外调用的 `update_group_items(group_id, items)`（或者直接由 command 层读老条目删写？不，由于有排序兼容，最好将内部 `replace_items` 开放为 `pub fn replace_group_items(&self, group_id: i64, items: &[GroupItemInput]) -> Result<(), AppError>`）。
- [ ] `src-tauri/src/commands.rs`：实现 `async fn perform_sync_bound_group(stores: &Stores, group_id: i64) -> Result<(), AppError>` 独立函数，包含（查 group、查 provider 启用态、调用 `fetch_upstream_model_ids`、组装 inputs、调 `replace_group_items`）。
- [ ] `src-tauri/src/commands.rs`：实现 `#[tauri::command] pub async fn sync_group_now(proxy, group_id) -> Result<Group, InvokeError>`，调用 `perform` 后重新获取最新 group 返回。

## 4. 后台定时任务 (Background Task)
- [ ] `src-tauri/src/proxy/runtime.rs`：在 `start` 内部生成代理的 `tokio_rt.spawn` 里，起一个定时循环：
  ```rust
  let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
  interval.tick().await; // 跳过第一次（即刻）
  loop {
      tokio::select! {
          _ = interval.tick() => {
              // 从 stores 取所有 source_provider_id IS NOT NULL 的分组并循环同步
              // 注意不要阻塞主 runtime，异常只记 tracing
          }
          _ = shutdown_rx_for_timer.notified() => break, // 需处理优雅退出
      }
  }
  ```
  *注意：为了接收退出信号，可以通过广播或者独立 channel。原 `LiveProxy` 用 `shutdown_tx: oneshot` 结束 `server::serve`，可以克隆一份接收端或让 server 任务去管定时器。建议在 `server::serve` 内部起或者传递一个 `CancellationToken`。*

## 5. 前端 API 与 UI (Frontend)
- [ ] `src/api/tauri.ts`：更新 `Group` 接口和增改 payload 类型。添加 `syncGroupNow`。
- [ ] `src/pages/GroupsPage.vue`：
  - 表单加 `source_provider_id` 的 `HSelect`。
  - 判定 `isBound = form.values.source_provider_id != null && form.values.source_provider_id !== 0`。
  - 模型列表区域：
    - `isBound` 为真时，禁用/隐藏拖拽（`draggable="false"`）、删除按钮。
    - 隐藏「添加模型」、「批量添加」区域。
    - 顶部插入提示：「本分组已绑定供应商，模型列表只读，每 24h 自动同步。」及「立即同步」按钮（点按触发 `syncGroupNow` 并覆盖表单队列，注意 saving 态处理）。
  - 分组列表视图：可以在「创建时间」等位置补充一个「自动同步」的 `HBadge` 或文本标识。

## 6. 后续质量检查 (Validation gates)
- [ ] `cargo test` 通过所有相关测试（特别是新加的迁移/保存单测）。
- [ ] `pnpm typecheck` 和 `pnpm lint` 检查前端类型安全（确保 camelCase 等规范）。
- [ ] 前端打包检查是否有打包层面的错误。
