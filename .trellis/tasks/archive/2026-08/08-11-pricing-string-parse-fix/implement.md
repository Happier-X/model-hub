# 执行计划：OpenRouter 价格字符串解析修复

## 实施清单

### 1. 解析兼容
- [ ] 在 `src-tauri/src/domain/pricing.rs` 增加私有价格值解析 helper。
- [ ] 兼容 JSON number 与 JSON string；字符串先去除首尾空白再解析为 `f64`。
- [ ] 缺失、空字符串、非法字符串和其他 JSON 类型回退为 `0.0`。
- [ ] 保持每 token 转每百万 token及 `round6` 精度规则不变。

### 2. 回归测试
- [ ] 增加真实 OpenRouter 字符串价格测试，断言输入/输出单价换算正确。
- [ ] 增加空字符串、非法字符串与混合有效值测试，确认单个坏字段不影响其他字段。
- [ ] 保留并运行现有数字格式、免费/缺失字段、非法响应和全量替换测试。

### 3. 运行验证
- [ ] `cargo test` 全量通过。
- [ ] `cargo build` 通过。
- [ ] 通过设置页“立即同步价格”刷新当前数据库价格。
- [ ] 查询 `model_pricing`，确认存在非零价格行；选择非免费模型验证输入/输出价格非零。
- [ ] 刷新首页，确认已有成功请求的费用统计不再固定为 0；免费模型仍允许为 0。
- [ ] 确认 git diff 仅包含本任务 Rust 文件和任务文档，不包含 shadcn 任务前端改动。

## 验证命令

```powershell
cd src-tauri
cargo test
cargo build
```

数据库验收重点：

```sql
SELECT COUNT(*) FROM model_pricing
WHERE prompt_price_per_mtok > 0 OR completion_price_per_mtok > 0;
```

## 风险与回滚点

- 风险文件：`src-tauri/src/domain/pricing.rs`；解析 helper 若接受过宽类型，可能把异常价格写入数据库，因此只接受 JSON number 或可解析的十进制字符串。
- 外部同步依赖 OpenRouter 网络和接口可用性；单元测试不依赖网络，手动同步仅作为端到端验收。
- 旧的全 0 价格不会自动修复，必须执行一次既有立即同步。
- 回滚只需还原代码提交；无 schema 或数据迁移。

## 开始实施前检查

- [ ] `prd.md`、`design.md`、`implement.md` 已完成并经用户批准。
- [ ] `implement.jsonl`、`check.jsonl` 已填充真实规范条目。
- [ ] 确认不覆盖工作区中 shadcn 任务的前端改动。
