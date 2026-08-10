# 08-10-cost-tracking — Design

## 架构与边界

```
OpenRouter API ──(24h 到期检查/手动按钮)──> commands.perform_due_price_syncs / sync_pricing_now
                                              └─> domain/pricing.rs: 解析 + upsert model_pricing 表
                                                        │
request_overview 统计时 ──LEFT JOIN model_pricing──> 费用 = Σ(tokens × price_per_mtok / 1e6)
                                                        │
                                            OverviewRow{ input_cost, output_cost, cost(总) }
                                                        │
                                  前端 StatsCards 三费用项显示 formatCost($)
```

- 费用**只在统计时计算**，`request_logs` 不加列；改价/补同步历史自动重算。
- 同步失败静默（warn 日志），绝不影响启动、代理与首页。

## DB：model_pricing 表

```sql
CREATE TABLE IF NOT EXISTS model_pricing (
  model_name TEXT PRIMARY KEY,
  prompt_price_per_mtok REAL NOT NULL DEFAULT 0,   -- 每百万输入 token 美元
  completion_price_per_mtok REAL NOT NULL DEFAULT 0, -- 每百万输出 token 美元
  updated_at INTEGER NOT NULL DEFAULT 0
);
```

- `migrate.rs` 幂等 ensure（`ensure_model_pricing_table`，`CREATE TABLE IF NOT EXISTS`）。
- 同步为全量 upsert（`INSERT ... ON CONFLICT(model_name) DO UPDATE`），删除表内不再出现的模型（replace 语义，用事务 delete+insert 或 upsert+清理过期）。

## 同步链路（复用 provider auto_sync 模式）

- `domain/pricing.rs`：
  - `parse_openrouter_pricing(&[u8]) -> Vec<ModelPrice>` 纯函数：解析 `data[].{id, pricing.prompt, pricing.completion}`，每 token → 每百万（×1e6），取整到 6 位小数；非法行跳过。
  - `ModelPrice { model_name, prompt_price_per_mtok, completion_price_per_mtok }`。
  - Stores 方法：`replace_pricing(&[ModelPrice])`（事务：upsert + 清理不在列表中的旧模型）、`list_pricing() -> Vec<ModelPrice>`、`get_pricing_sync_info() -> Option<(i64 /*count*/, i64 /*last_updated*/)>`。
- `commands.rs`：
  - `pub async fn fetch_openrouter_pricing() -> Result<Vec<ModelPrice>, AppError>`：reqwest GET `https://openrouter.ai/api/v1/models`（超时 15s，复用现有 HTTP client 或新建）。
  - `pub async fn perform_due_price_syncs(stores: &Stores)`：`now - last_updated >= SYNC_STALE_AFTER_SECS(24h)` 才拉；失败 warn。
  - `#[tauri::command] pub async fn sync_pricing_now(proxy) -> Result<PricingSyncInfo, InvokeError>`（设置页「立即同步」按钮）：强制拉取 + 返回 `{ count, updated_at }`。
  - `#[tauri::command] pub fn get_model_pricing(proxy) -> Result<PricingInfo, InvokeError>`：`{ items: Vec<ModelPrice>, count, updated_at }`。
- `proxy/runtime.rs`：timer_fut 的 interval tick 内追加 `crate::commands::perform_due_price_syncs(&stores).await`（与供应商同步同一节奏，启动静默 5 分钟沿用）。

## 统计费用（log.rs）

`overview_row` SQL 改造（两段：range/全部共用）：
```sql
SELECT
  COUNT(*),
  COALESCE(SUM(l.input_tokens), 0),
  COALESCE(SUM(l.output_tokens), 0),
  COALESCE(SUM(l.use_time_ms), 0),
  COALESCE(SUM(CAST(l.input_tokens AS REAL) * COALESCE(p.prompt_price_per_mtok, 0) / 1000000.0), 0)  AS input_cost,
  COALESCE(SUM(CAST(l.output_tokens AS REAL) * COALESCE(p.completion_price_per_mtok, 0) / 1000000.0), 0) AS output_cost
FROM request_logs l
LEFT JOIN model_pricing p
  ON l.upstream_model = p.model_name
  OR p.model_name LIKE '%/' || l.upstream_model
WHERE status_code BETWEEN 200 AND 299 AND (error IS NULL OR length(error) = 0)
  [AND time >= ?1 AND time < ?2]
```
- 别名匹配：`p.model_name LIKE '%/' || l.upstream_model` 使 `deepseek/deepseek-chat` 匹配日志 `deepseek-chat`；SQLite LIKE 默认 ASCII 大小写不敏感。
- `OverviewRow` 加 `input_cost: f64`、`output_cost: f64`（`cost` 保留 = input+output 总费用）；`request_overview` 无范围分支同样处理。

## 前端

- `tauri.ts`：`OverviewRow` 加 `input_cost/output_cost`；新增 `ModelPrice`/`PricingInfo` 类型 + `getModelPricing`/`syncPricingNow`。
- `src/utils/formatCost.ts`：`formatCost(n)` —— 0 → `$0`；>0 → `$` + 至多 4 位小数去尾 0（如 `$1.25`、`$0.0012`）+ 单测。
- `StatsCards.vue`：三项费用 value 从 `"-"` 改为 `formatCost(...)`：
  - 总费用 = `r.cost`（后端已 = input+output）
  - 输入费用 = `r.input_cost`、输出费用 = `r.output_cost`（kind raw 直接显示字符串）
- 设置页新增「模型单价」卡片：同步状态行（共 N 个模型 · 最后同步时间）+「立即同步」按钮（`syncPricingNow`，loading 态）+ 搜索框 + shadcn Table（模型名/输入价/输出价）；空态提示「尚未同步，点击立即同步或等待后台自动同步」。

## 兼容与回滚

- 新表 `CREATE TABLE IF NOT EXISTS` 幂等；旧库无表直接创建。
- 费用逻辑全在统计查询内，移除 model_pricing 表即可回退到「-」展示（前端改回常量）。
- 同步为纯增量增强：失败不影响代理；无网络时 `updated_at` 不变，下次到期再试。
- 金额精度：REAL 8 字节浮点，统计级精度足够（7 天万条内）。

## 风险与缓解

- OpenRouter id 与日志模型名不一致：别名 LIKE 匹配缓解；仍不中 → 0 价（用户已接受）。
- 网络不可达/OpenRouter 变更：同步失败静默 warn；解析纯函数单测锁定格式；`sync_pricing_now` 手动触发时向用户报错提示。
- 表数据 400+ 行前端渲染：设置页表格用 v-for + 搜索过滤（前端过滤即可，不引入分页）。
