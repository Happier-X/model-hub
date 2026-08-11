# 首页统计实时更新 — 技术设计

## 1. 架构与边界

```
请求完成
  └─ proxy/forward.rs / server.rs ── insert_log_best_effort
        └─ domain/log.rs Stores::insert_log ── 写 SQLite 成功
              └─ notify_changed() 遍历订阅回调（领域层，不依赖 tauri）
                    └─ proxy/runtime.rs 注册的 listener ── app.emit("stats-changed")
                          └─ HomePage.vue listen("stats-changed") ── 立即刷新 overview
```

- **领域层**（`Stores`）：只加订阅机制（std 实现），不引入 tauri 依赖。空订阅时零开销。
- **代理层**（`ProxyHandle`）：定义变更回调契约（`Box<dyn Fn() + Send + Sync>`），**不依赖 tauri**。
- **壳层**（`lib.rs`）：注入 emit 闭包，把领域变更转成 tauri 全局事件。
- **前端**（`HomePage.vue`）：监听事件 + 恢复可见刷新 + 保留 5s 轮询兜底。

## 2. 数据流与契约

### 事件契约
- 事件名：`stats-changed`（字符串常量，Rust 与前端各自定义，防拼写错误用 `pub const STATS_CHANGED_EVENT: &str = "stats-changed"`）。
- payload：空（`()`）。前端收到后自行 `invoke("get_request_overview")` 拉最新数据，避免 payload 与查询逻辑耦合。
- 语义：**日志写入成功后**才 emit；写入失败不 emit（`insert_log_best_effort` 失败静默）。

### Stores 变更订阅
```rust
// domain/mod.rs
pub struct Stores {
    pub db: DbConn,
    change_listeners: Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>,
}
impl Stores {
    pub fn new(db: DbConn) -> Self { /* 空 listeners */ }
    pub fn subscribe_change(&self, listener: impl Fn() + Send + Sync + 'static) {
        self.change_listeners.lock()...push(Arc::new(listener));
    }
    fn notify_changed(&self) {
        let listeners = self.change_listeners.lock()...clone(); // 锁外调用回调，防重入死锁
        for f in &listeners { f(); }
    }
}
```
- `insert_log` 成功后（`with_conn` 返回 Ok 之后、锁已释放）调用 `self.notify_changed()`。
- 注意 `Stores { db }` 字面量构造点（测试等）需同步补字段或提供 `new` 之外的构造路径（本仓均走 `new`，无字面量构造）。

### ProxyHandle 回调注入（壳层转 tauri 事件）
```rust
// proxy/runtime.rs —— 不 import 任何 tauri 类型
struct RuntimeInner { /* ... */ change_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>> }
pub fn set_change_callback(&self, cb: Option<Box<dyn Fn() + Send + Sync>>)
// ensure_stores 创建 stores 时注册 listener（惰性读取 change_callback，注入先后均可生效）

// lib.rs setup —— start 之后注入 emit 闭包
let app_handle = app.handle().clone();
proxy.set_change_callback(Some(Box::new(move || {
    let _ = app_handle.emit(STATS_CHANGED_EVENT, ());
})));
```
- **为何不用 `RuntimeInner.app: Option<AppHandle>`**：直接引用 tauri 类型会让 cargo test 的 test harness 链接 wry/tao 的 comctl32 v6 专属符号（无 manifest 的 test exe 加载时 `STATUS_ENTRYPOINT_NOT_FOUND 0xc0000139` 崩溃）。回调注入后，emit 闭包在 lib.rs（test 下被 DCE 裁剪），代理/领域层保持 tauri 无关。

## 3. 前端刷新策略（HomePage.vue）

```
优先级：事件/可见性即时刷新 > 5s 轮询兜底
```

- `listen("stats-changed", () => void refreshOverviewOnly())`（mounted 时注册，unmounted 时 `unlisten()`）。
- `visibilitychange → visible` 与 `window focus` 时立即 `refreshOverviewOnly()`，解决托盘恢复旧数据。
- 保留现有 5s `setInterval`。
- **防重入**：`refreshOverviewOnly` 加 in-flight 标志（`if (refreshing) return; ... finally { refreshing = false }`），事件驱动与轮询并发时只发一次 invoke。
- 事件频率 = 请求完成频率（人类节奏），不做额外节流；若未来高频请求，可在此函数内加 1s 最小间隔。

## 4. 兼容性与回滚

- 领域层 `Stores` 字段新增：所有 `Stores::new` 路径自动覆盖；直接字面量构造处（测试）需同步改。
- `subscribe_change` 是新增 API，旧调用不受影响；listener 泄漏风险低（Stores 生命周期 = 应用生命周期）。
- 事件是新增推送，前端不 listen 时（旧页面缓存）无影响；前端 listen 失败时 5s 轮询兜底。
- 回滚：Revert 提交即可，无数据迁移、无 schema 变更。

## 5. 风险

- **emit 频率**：流式请求仅结束时写一条日志（`defer_request_log`），非流式每请求一条 → 事件频率与请求频率一致，可接受。
- **WebView2 隐藏期间事件丢失**：隐藏时 JS 暂停，emit 到 webview 的事件可能丢弃 → 由"恢复可见刷新"（visibilitychange/focus）兜底，恢复即拉最新。
- **token/耗时 M 级舍入**：单次请求增量在 `"16.82M"`/`"1.06h"` 下不可见，属已知显示精度限制，本期不改（prd.md Out of Scope）。
