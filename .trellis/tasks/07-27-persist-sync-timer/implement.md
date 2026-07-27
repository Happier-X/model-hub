# 执行计划：跨重启的 24 小时倒计时同步

## 1. 数据库迁移（migrate.rs）
- [ ] `ensure_group_columns` 追加：`if !columns.contains("last_sync_at")` 时 `ALTER TABLE groups ADD COLUMN last_sync_at INTEGER`。
- [ ] 新增单测 `migrate_adds_missing_last_sync_at_without_losing_data`（比照 source_provider_id 单测：建 legacy 表→migrate→断言列存在且 NULL、旧数据保留→二次 migrate 幂等）。

## 2. 领域层（domain/group.rs）
- [ ] `Group` 加 `pub last_sync_at: Option<i64>`。
- [ ] `list_groups` / `get_group_by_name` 的 SELECT 加 `last_sync_at`，行映射与结构体构造补该列。
- [ ] 新增 `pub fn touch_group_synced_at(&self, group_id: i64, unix: i64) -> Result<(), AppError>`。
- [ ] 现有单测中构造 `Group`（若有直接字面量构造）补字段；create 后断言 `last_sync_at` 默认 None。

## 3. 同步胶水（commands.rs）
- [ ] `perform_sync_bound_group`：`replace_group_items` 成功后 `touch_group_synced_at(group.id, now)`；禁用/fetch 失败路径不写时间（确认既有提前返回位置正确）。
- [ ] 加常量 `SYNC_STALE_AFTER_SECS = 24*3600`、`SYNC_STAGGER = Duration::from_secs(5)`。
- [ ] 新增 `pub async fn perform_due_bound_groups(stores: &Stores)`：过期判定（None 或 now-t≥24h）+ 实际发起项之间 5 秒错峰。
- [ ] 保留 `perform_sync_bound_group` 供手动/内部复用；决定是否移除旧 `perform_sync_all_bound_groups`（若仅调度器用，直接替换为 `perform_due_bound_groups`）。

## 4. 调度器（proxy/runtime.rs）
- [ ] 加常量 `SYNC_STARTUP_DELAY = 5min`、`SYNC_CHECK_INTERVAL = 3600s`。
- [ ] `timer_fut` 改为：先 `sleep(SYNC_STARTUP_DELAY)`，再 `interval(SYNC_CHECK_INTERVAL)` 循环 `tick → perform_due_bound_groups`（不再跳过首个 tick）。
- [ ] `tokio::select!` 结构与退出语义保持不变。

## 5. 前端（api/tauri.ts / GroupsPage.vue）
- [ ] `Group` 接口加 `last_sync_at?: number | null`。
- [ ] 绑定态只读区展示「上次同步：{formatUnix} / 尚未同步」（可选展示）。
- [ ] 确认所有构造/依赖 Group 类型处 typecheck 通过。

## 6. Spec 更新
- [ ] `upstream-access.md`：把后台同步例外从「24h 连续运行定时」更新为「每小时检查 + 启动后 5 分钟静默 + 距上次同步 ≥24h 才拉 + 多分组 5 秒错峰」。
- [ ] 若 `component-guidelines.md` 提到同步机制，同步措辞。

## 7. 质量门禁
- [ ] `cargo test`（含新迁移单测）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
- [ ] `pnpm typecheck`、`pnpm lint`、`pnpm build`。
