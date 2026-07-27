# 技术设计：跨重启的 24 小时倒计时同步

## 架构边界

沿用上个任务的分层：`Stores`（纯 DB，不碰网络）+ `commands.rs` 胶水（网络 + 持久化）+ `proxy/runtime.rs` 调度器。本次只改「时间记录」与「调度判定」，不动镜像写入语义。

## 时间存储格式

新增列 `groups.last_sync_at INTEGER`（unix 秒，可空；NULL = 从未同步）。
选 INTEGER unix 秒而非 RFC3339 文本，便于 SQL 直接算差值、与 `request_logs.time` 口径一致，避免时区/解析歧义。

## 详细设计

### 1. 数据库（`db/migrate.rs`）

`ensure_group_columns` 追加幂等加列：
```sql
ALTER TABLE groups ADD COLUMN last_sync_at INTEGER
```
可空列直接加、无需 DEFAULT。新增单测 `migrate_adds_missing_last_sync_at_without_losing_data`，比照现有 `source_provider_id` 单测结构。

### 2. 领域层（`domain/group.rs`）

- `Group` 结构体加 `pub last_sync_at: Option<i64>`。
- `list_groups` / `get_group_by_name` 的 SELECT 与行映射补该列。
- 新增写时间的原子方法（不触碰 items）：
  ```rust
  pub fn touch_group_synced_at(&self, group_id: i64, unix: i64) -> Result<(), AppError>
  ```
  执行 `UPDATE groups SET last_sync_at = ?1 WHERE id = ?2`。
- create/update payload **不**接收 last_sync_at（它只由同步流程写入，用户表单不可编辑）；create 默认 NULL。

### 3. 同步胶水（`commands.rs`）

`perform_sync_bound_group` 在 `replace_group_items` 成功后追加：
```rust
let now = chrono::Utc::now().timestamp();
stores.touch_group_synced_at(group.id, now)?;
```
注意：供应商禁用时提前 `return Ok(())`（既有逻辑），此路径**不写时间**——禁用不算一次成功同步，解禁后应尽快补。fetch 失败走 `?` 提前返回，也不写时间。

新增后台专用的「过期判定 + 错峰」调度函数，替换现有 `perform_sync_all_bound_groups` 的无条件全量：
```rust
pub const SYNC_STALE_AFTER_SECS: i64 = 24 * 3600;
pub const SYNC_STAGGER: Duration = Duration::from_secs(5);

pub async fn perform_due_bound_groups(stores: &Stores) {
    let groups = match stores.list_groups() { ... };
    let now = chrono::Utc::now().timestamp();
    let mut first = true;
    for group in groups {
        if group.source_provider_id.is_none() { continue; }
        let due = match group.last_sync_at {
            None => true,
            Some(t) => now - t >= SYNC_STALE_AFTER_SECS,
        };
        if !due { continue; }
        if !first { tokio::time::sleep(SYNC_STAGGER).await; }
        first = false;
        if let Err(e) = perform_sync_bound_group(stores, group.id).await {
            tracing::warn!(...);
        }
    }
}
```
说明：错峰只在「本批实际发起的分组之间」插入，用 `first` 标志避免第一个也白等 5 秒。禁用供应商在 `perform_sync_bound_group` 内部提前返回，几乎零耗时，不会触发有意义的上游访问，但为简单起见仍计入 `first` 序列亦可接受（不发网络请求）。为精确起见，判定 due 时不预读 provider enabled，交由内部处理，保持单一职责。

`sync_group_now`（手动）继续直接调 `perform_sync_bound_group`，不受 24h 限制——内部会写时间，手动同步也刷新倒计时。

### 4. 调度器（`proxy/runtime.rs`）

把 24h 大睡改成「启动延迟 + 每小时检查」：
```rust
pub const SYNC_STARTUP_DELAY: Duration = Duration::from_secs(5 * 60);
pub const SYNC_CHECK_INTERVAL: Duration = Duration::from_secs(3600);

let timer_fut = async move {
    tokio::time::sleep(SYNC_STARTUP_DELAY).await; // 启动后先静默 5 分钟
    let mut interval = tokio::time::interval(SYNC_CHECK_INTERVAL);
    loop {
        interval.tick().await; // 首个 tick 立即返回：即启动延迟后立刻做一次检查
        crate::commands::perform_due_bound_groups(&stores).await;
    }
};
```
与现有 `tokio::select!(serve_fut, timer_fut)` 结构不变：代理 stop → serve 完成 → select 返回 → timer_fut drop，无泄漏。启停语义与之前一致，退出安全性不变。

设计取舍：首个 `interval.tick()` 立即返回是有意的——启动延迟 5 分钟后我们**希望**立刻检查一次过期项（此时距上次可能已远超 24h），这正是修复目标。防封号靠的是「启动后 5 分钟静默」而非「跳过首个 tick」，与旧机制的防护点不同但更契合需求。

### 5. 前端（`api/tauri.ts` / `GroupsPage.vue`）

- `Group` 接口加 `last_sync_at?: number | null`。
- 绑定态只读区可展示「上次同步：<本地时间> / 尚未同步」，复用页面已有的 `formatUnix`。纯展示，非必需交互。

## 兼容性与回滚

- 仅加可空列，旧库无损；回滚旧代码忽略多余列。
- 老绑定分组 `last_sync_at` 为 NULL → 首次检查即视为过期，会在启动延迟后同步一次并落时间，符合预期。
