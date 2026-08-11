# 技术设计：OpenRouter 价格字段兼容解析

## 1. 边界

只修改 `src-tauri/src/domain/pricing.rs` 的价格值解析与单元测试。`commands.rs` 的网络请求、`replace_pricing` 的全量替换、`request_overview` 的费用公式和前端展示均保持不变。

## 2. 数据契约

OpenRouter 模型响应中的价格字段允许以下形态：

```json
{"pricing":{"prompt":"0.00000125","completion":"0.00000425"}}
```

兼容已有测试和潜在响应：

```json
{"pricing":{"prompt":0.00000125,"completion":0.00000425}}
```

解析后统一为 `ModelPrice`：

- `prompt_price_per_mtok = prompt_per_token * 1_000_000`
- `completion_price_per_mtok = completion_per_token * 1_000_000`
- 结果沿用 `round6`
- 缺失、空字符串、非法字符串或不支持的 JSON 类型 → `0.0`

## 3. 实现方案

增加一个私有小 helper（例如 `parse_price_value`），先尝试 `as_f64()`，再对 `as_str()` 做 `trim().parse::<f64>()`；调用方统一 `unwrap_or(0.0)`。不改变模型 ID 过滤和价格表写入逻辑。

## 4. 测试方案

- 为真实字符串格式增加测试：验证输入/输出单价换算和 6 位精度。
- 扩展异常值测试：空字符串、非法字符串不会污染其他模型结果。
- 保留已有 JSON number、free/missing fields、invalid body、replace/prune 测试。
- 使用临时数据库或现有数据库测试验证费用统计不在本次改动范围内；真实价格刷新由手动同步验收。

## 5. 兼容性与回滚

这是纯解析兼容修复，无 schema、IPC、网络地址或数据迁移变化。旧数据库中的 0 价数据不会自动变更，必须执行一次既有“立即同步价格”；回滚代码后不会破坏数据库结构。
