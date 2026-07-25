# 技术设计：首页每日请求量热力图

## 1. 边界与分层

| 层 | 文件 | 改动 |
|----|------|------|
| 领域/持久化 | `src-tauri/src/domain/log.rs` | 新增 `request_daily_counts`、`RequestDailyCounts`、`DailyCount`；新增单测 |
| IPC | `src-tauri/src/commands.rs` | 新增 `get_request_daily_counts` command |
| 注册 | `src-tauri/src/lib.rs` | `invoke_handler` 追加 `commands::get_request_daily_counts` |
| 前端 API | `src/api/tauri.ts` | 新增 `DailyCount` / `RequestDailyCounts` 类型 + `getRequestDailyCounts(days?)` |
| 前端 UI | `src/pages/HomePage.vue` | 引入 `HHeatmap`；拉取数据、映射为 `HHeatmapData`、渲染、错误/加载态、接入刷新 |

## 2. 后端聚合设计

### 2.1 时区归桶策略

`request_logs.time` 是写入时 `chrono::Local::now().timestamp()`（本地 unix 秒）。既有
`request_stats_between` 已按 `time >= start AND time < end` 半开区间过滤，
`local_day_bounds_unix()` 提供本地日边界，本设计复用同一时区语义。

**不使用 SQLite 的 `date(time,'unixepoch','localtime')` 分组**：`localtime` 依赖运行时
系统时区且在不同平台行为易漂移，测试不可控。改为**应用层分桶**：

1. Rust 侧算出窗口 `[start_unix, end_unix)`：
   - `end_unix` = 今日次日 00:00（即 `local_day_bounds_unix().1`）
   - `start_unix` = 从今日 00:00 向前推 `days-1` 天的那一天 00:00
2. 单条 SQL 只取窗口内 `time`：`SELECT time FROM request_logs WHERE time >= ?1 AND time < ?2`
3. 遍历每个 `time`，用 `chrono::Local` 还原出当天 00:00 的 unix 秒作为桶键，`HashMap<i64, i64>` 计数
4. 仅输出 `count > 0` 的桶，按 `day_start_unix` 升序

行数量级：默认日志保留 30 天、上限数万行，窗口最多 400 天，应用层遍历 O(n) 可接受（目标 < 100ms）。

### 2.2 桶键计算辅助

新增私有函数：

```rust
/// 把任意本地 unix 秒归一化到「所在本地自然日 00:00」的 unix 秒。
fn local_day_start_unix(ts: i64) -> i64 {
    use chrono::{Local, TimeZone};
    let dt = Local.timestamp_opt(ts, 0).single();
    match dt {
        Some(dt) => {
            let day = dt.date_naive();
            let midnight = day.and_hms_opt(0, 0, 0).expect("midnight");
            Local
                .from_local_datetime(&midnight)
                .single()
                .map(|d| d.timestamp())
                .unwrap_or(ts)
        }
        None => ts,
    }
}
```

### 2.3 窗口起点计算

```rust
fn daily_window_bounds(days: u32) -> (i64, i64) {
    use chrono::{Duration, Local, TimeZone};
    let (today_start, tomorrow_start) = local_day_bounds_unix();
    let back = days.saturating_sub(1) as i64;
    let start_naive = Local
        .timestamp_opt(today_start, 0)
        .single()
        .map(|dt| dt.date_naive() - Duration::days(back));
    let start_unix = start_naive
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|nd| Local.from_local_datetime(&nd).single())
        .map(|dt| dt.timestamp())
        .unwrap_or(today_start);
    (start_unix, tomorrow_start)
}
```

### 2.4 入参钳制

```rust
const DAILY_COUNTS_DEFAULT_DAYS: u32 = 365;
const DAILY_COUNTS_MAX_DAYS: u32 = 400;

// command 层：days 传 None → 365；传 0 → 1；> 400 → 400
let days = days.unwrap_or(DAILY_COUNTS_DEFAULT_DAYS).clamp(1, DAILY_COUNTS_MAX_DAYS);
```

### 2.5 返回结构

```rust
#[derive(Debug, Clone, Serialize)]
pub struct DailyCount {
    pub day_start_unix: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestDailyCounts {
    pub days: Vec<DailyCount>,
    pub start_unix: i64,
    pub end_unix: i64,
}
```

### 2.6 方法签名

```rust
pub fn request_daily_counts(&self, days: u32) -> Result<RequestDailyCounts, AppError>
```

`days` 已在 command 层钳制；领域层再 `clamp(1, DAILY_COUNTS_MAX_DAYS)` 兜底一次。

## 3. IPC 契约

```rust
#[tauri::command]
pub fn get_request_daily_counts(
    proxy: State<'_, ProxyHandle>,
    days: Option<u32>,
) -> Result<crate::domain::log::RequestDailyCounts, InvokeError> {
    let days = days.unwrap_or(365);
    stores(&proxy)?.request_daily_counts(days).map_err(Into::into)
}
```

Tauri 参数名 `days` → 前端 invoke 传 `{ days }`（camelCase 无差异，单词无需转换）。

## 4. 前端设计

### 4.1 类型 + 调用（`api/tauri.ts`）

```ts
export interface DailyCount {
  day_start_unix: number;
  count: number;
}
export interface RequestDailyCounts {
  days: DailyCount[];
  start_unix: number;
  end_unix: number;
}
export const getRequestDailyCounts = (days?: number) =>
  invoke<RequestDailyCounts>("get_request_daily_counts", days == null ? {} : { days });
```

### 4.2 HomePage 集成

- `import { HHeatmap } from "happier-ui"`，类型 `import type { HHeatmapData } from "happier-ui"`
- 新增 ref：`daily = ref<RequestDailyCounts | null>(null)`、`dailyError = ref("")`、`dailyLoading = ref(false)`
- `heatmapData = computed<HHeatmapData>(...)`：`daily.value?.days.map(d => ({ timestamp: d.day_start_unix * 1000, value: d.count })) ?? []`
- `refreshStats()` 内并入热力图拉取（与 stats / lastSuccess 一起 `Promise.all`）：
  ```ts
  dailyLoading.value = true;
  const dailyPromise = getRequestDailyCounts()
    .then((v) => { daily.value = v; dailyError.value = ""; })
    .catch((e) => { dailyError.value = extractInvokeError(e); })
    .finally(() => { dailyLoading.value = false; });
  ```
- 卡片放在「今日请求」`HCard` 下方，独立 `HCard`：
  - header：`每日请求量（近一年）` + 复用刷新（或说明“随上方刷新”）
  - body：`<HHeatmap :data="heatmapData" :loading="dailyLoading" />`
  - `dailyError` 非空时渲染 `text-rose-600` 文案

### 4.3 HHeatmap Props（0.0.6）

已确认导出：`data?`, `firstDayOfWeek?`, `size?`, `colors?`, `showWeekLabels?`, `showMonthLabels?`, `showColorIndicator?`, `loading?`。本轮只用 `data` + `loading`，其余取默认。

## 5. 兼容性 / 回滚

- 纯新增：新 SQL 只读、新 command、新前端调用；不动表结构、不动既有 command，不影响现有转发/日志路径。
- 回滚：`git revert` 单 commit 即可，无迁移需要回退。

## 6. 测试策略

后端单测（`domain/log.rs` tests 模块，复用现有 `setup()` / `seed()`）：

- `daily_counts_buckets_by_local_day`：`seed` 若干条（都是“今天”写入，因 `insert_log` 用 `Utc::now`）→ 断言返回桶 `count` 合计 == 插入数、当天桶存在。
- `daily_counts_respects_window_and_clamp`：`request_daily_counts(0)` 与 `(9999)` 不 panic，`start_unix < end_unix`，`days` 长度合理。
- `daily_counts_empty_db_returns_no_buckets`：空库 `days` 为空、`start/end` 有值。

> 注：`insert_log` 内部用 `Utc::now().timestamp()` 落 `time`，无法在测试里伪造历史日期。因此
> 跨日桶断言以“同一天多条 → 单桶累加”为主；窗口/钳制/空库覆盖边界。历史多桶靠直接
> `with_conn` 手写不同 `time` 的 INSERT 兜底（参照现有测试里 `with_conn` 手动插入模式）。

前端：无独立单测（纯展示 + computed 映射），靠 `pnpm typecheck` 保证契约。
