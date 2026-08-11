# 首页统计实时更新 — 执行计划

## 实施清单（顺序执行）

### 1. Rust：Stores 变更订阅（领域层）
- [x] `src-tauri/src/domain/mod.rs`：`Stores` 增 `change_listeners: Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>`；`new` 初始化；增 `subscribe_change` 与私有 `notify_changed`（**锁外**克隆列表后调用回调，防重入死锁）。
- [x] `src-tauri/src/domain/log.rs`：`insert_log` 成功后调用 `self.notify_changed()`（with_conn 返回 Ok 之后、锁已释放）。
- [x] 检查全仓 `Stores { db }` 字面量构造点：均走 `Stores::new`，无字面量构造，无需补字段。

### 2. Rust：事件推送（壳层回调注入）
- [x] `src-tauri/src/proxy/runtime.rs`：定义 `pub const STATS_CHANGED_EVENT: &str = "stats-changed"`；`RuntimeInner` 增 `change_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>`（**不直接依赖 tauri**——直接引用 AppHandle 会使 test 二进制链接 wry/tao 的 comctl32 v6 符号，无 manifest 的 test harness 加载失败 0xC0000139）；`ensure_stores` 创建 stores 时注册 listener（惰性读取 change_callback，注入先后均可生效）；`set_change_callback` 供壳层注入。
- [x] `src-tauri/src/lib.rs`：setup 中 `proxy.start()` 后注入闭包 `move || { let _ = app_handle.emit(STATS_CHANGED_EVENT, ()); }`（`use tauri::Emitter`）。

### 3. Rust：测试
- [x] `src-tauri/src/domain/log.rs` 测试：`insert_log_notifies_change_subscribers`（`Arc<AtomicUsize>` 计数断言每次写入触发一次回调）；无订阅写入不 panic（既有测试覆盖）。
- [x] `cargo test` 全量通过（147 lib + 13 集成 + 9 其它）；`cargo build` 通过。

### 4. 前端：HomePage 即时刷新
- [x] `src/pages/HomePage.vue`：
  - `import { listen, type UnlistenFn } from "@tauri-apps/api/event"`；
  - `refreshOverviewOnly` 加 in-flight 防重（`refreshingOverview` 标志 + finally 复位）；
  - `onMounted`：`listen("stats-changed", ...)`（try/catch，失败不影响轮询）；`document.addEventListener("visibilitychange")` + `window.addEventListener("focus")`（visible 时立即刷新）；
  - `onUnmounted`：`unlisten?.()` + 移除两个监听。

### 5. 前端验证
- [x] `pnpm typecheck` 通过；改动文件（HomePage.vue）无 lint 错误（`src/components/ui/chart/utils.ts` 的 any 错误属 shadcn 任务文件，非本次范围）。
- [x] 端到端实测（vite dev + debug exe + WebView2 CDP）：发一次代理请求 → 首页请求次数 480.00 → 1.2s 内 481.00（事件驱动立即刷新，无需等 5s 轮询）。

## 验证命令（全部通过）

```bash
cargo test            # 147 + 13 + 9 all ok（含新增订阅测试）
cargo build           # 编译通过
pnpm typecheck        # 前端类型检查通过
```

## 风险文件 / 回滚点

- `src-tauri/src/domain/mod.rs`（Stores 结构变更；已确认全仓无 `Stores {}` 字面量构造）
- `src-tauri/src/domain/log.rs`（insert_log 触发点 + 订阅测试）
- `src-tauri/src/proxy/runtime.rs`（change_callback 回调注入；**保持不依赖 tauri**，避免 test 链接 comctl32 v6）
- `src-tauri/src/lib.rs`（注入 emit 闭包）
- `src/pages/HomePage.vue`（事件监听 + 可见性刷新 + 防重入）

回滚：整体 revert 提交，无 schema/数据迁移。

## 实施中发现的关键坑（已沉淀）

- **domain/代理层禁止直接依赖 tauri 类型**：`cargo test` 的 test harness 无 manifest（无 `.rsrc`），若链接 wry/tao 的 comctl32 v6 专属符号（`SetWindowSubclass`/`TaskDialogIndirect` 等），加载器绑定到 comctl32 v5 → `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)` 直接崩。解决：领域/代理层定义回调契约（`Box<dyn Fn() + Send + Sync>`），壳层（lib.rs）负责转 tauri 事件；test 编译时该闭包被 DCE 裁剪。