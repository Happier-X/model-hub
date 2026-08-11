# 修复 OpenRouter 价格解析：字符串价格导致费用全为 0

## Goal

修复首页费用统计始终为 0 的问题。OpenRouter `/api/v1/models` 当前返回的 `pricing.prompt` 和 `pricing.completion` 是数字字符串，例如 `"0.00000095"`；现有解析使用 `serde_json::Value::as_f64()`，无法读取字符串，随后回退为 `0.0`，导致 `model_pricing` 表中的价格均为 0，历史请求费用也无法计算。

修复后，价格同步应正确保存每百万 token 单价，统计查询应根据既有请求日志中的 token 数重新计算费用。

## Confirmed Facts

- 解析代码位于 `src-tauri/src/domain/pricing.rs:34-64` 的 `parse_openrouter_pricing`。
- 当前实现仅通过 `as_f64()` 读取价格；OpenRouter 实际响应的价格字段为字符串。
- 数据库 `model_pricing` 已有约 400 行，但实测非零价格行数为 0。
- `request_logs` 保存 `input_tokens` / `output_tokens`，不保存费用；费用在统计查询时按 `model_pricing` 临时计算，因此修复并重新同步后可以补算历史请求。
- 现有同步入口为设置页的 `sync_pricing_now`，同时存在启动后的 24 小时后台同步。
- 当前工作区存在 shadcn 任务的前端改动；本任务只修改 Rust 价格解析及测试，不触碰这些改动。

## Requirements

- R1：价格解析同时支持 JSON number 和 JSON string 两种格式。
- R2：无法解析、缺失或空字符串价格继续安全回退为 `0.0`，不能导致同步命令崩溃。
- R3：保持现有单位和精度：OpenRouter 每 token 美元价格乘以 `1_000_000`，再按现有 6 位小数规则取整。
- R4：补充回归测试，覆盖真实字符串格式，并保留数字格式、免费模型/缺失字段、非法响应等既有行为。
- R5：不改变价格同步的全量替换策略、费用统计口径、前端费用展示格式，也不在本任务引入缓存读价格或其他额外计费项。
- R6：修复后通过立即同步刷新本地价格表，并验证已有成功请求的费用不再全部为 0。

## Acceptance Criteria

- [ ] AC1：`pricing.prompt` / `pricing.completion` 为字符串的响应可正确解析，例如 `"0.00000125"` → 输入单价 `1.25`。
- [ ] AC2：JSON number、免费模型、缺失字段、空字符串和非法数字均按约定处理；非法值回退为 0，不影响其他模型解析。
- [ ] AC3：新增回归单元测试通过，且 `cargo test` 全量通过。
- [ ] AC4：`cargo build` 通过；不修改前端费用卡片、费用公式或同步全量替换行为。
- [ ] AC5：执行一次设置页“立即同步价格”后，`model_pricing` 中存在非零价格行；首页已有成功请求的总费用、输入费用或输出费用至少一项不再固定为 0（免费模型除外）。

## Out of Scope

- 不接入 `input_cache_read`、web search 或其他 OpenRouter 额外计费字段。
- 不调整费用卡片的格式、精度或展示文案。
- 不改变价格同步周期、网络地址、全量替换策略或模型匹配规则。
- 不修改当前工作区中属于 shadcn 任务的前端文件。

## Risks / Deferred Items

- OpenRouter 的免费模型价格合法为 0；验证时应选取至少一个非免费模型，不能仅依据任意单条价格判断同步失败。
- OpenRouter 价格接口是外部数据源；自动同步失败时仍沿用现有错误处理，本任务不增加新的重试策略。
- 价格同步后历史费用会按当前价格重算，这是既有设计，不做历史价格快照。

## Open Questions

无阻塞问题。