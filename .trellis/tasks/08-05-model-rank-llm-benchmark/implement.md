# 执行计划：模型能力排序改用 llm_benchmark 榜单

## 前置

- [x] PRD 决策 D1–D4 已关闭
- [x] design.md 已写
- [ ] 用户审阅本计划后 `task.py start`（未批准前不写产品代码）

## 清单

### 0. 准备

1. [x] 读 backend spec：`upstream-access`；frontend：`model-queue-sort`、`quality-guidelines`
2. [x] 确认 `datasets.json` 实际返回结构（可临时 curl raw URL 验证字段名 `category/reportDate/csv`）

### 1. 后端（leaderboard.rs）

3. [x] 常量替换：删 OpenRouter URL，加 llm_benchmark datasets/base URL + `LEADERBOARD_CACHE_FILE` 新名
4. [x] `parse_llm_benchmark_csv`：手写 CSV（表头定位 `模型`/`极限分数` 列索引；引号字段；数值白名单；空榜错误）
5. [x] `locate_latest_logic_csv`：解析 datasets.json 数组，取 category=logic 最新 reportDate 的 csv 路径
6. [x] `fetch_llm_benchmark_models`：两步 GET（datasets → csv），复用超时/错误 sanitize；文案改 llm_benchmark
7. [x] `source` 字符串改 `"llm_benchmark"`（fresh/stale/error 三处）
8. [x] 错误文案（超时/连接/HTTP/空榜）全面替换 OpenRouter → llm_benchmark

### 2. 前端

9. [x] `modelCapability.ts`：`normalizeModelIdForMatch` 加展示名净化（剥括号档位、剥 4 位日期段）；`sourceLabel` → `"llm_benchmark"`
10. [x] `GroupsPage.vue`：状态文案/分数徽章/排序提示 3 处 OpenRouter → llm_benchmark
11. [x] `modelCapability.test.ts`：新增净化与匹配用例（GPT-5.5/Kimi-K3/DeepSeek V4 Flash）；sourceLabel 断言更新；护栏用例保留
12. [x] Rust 单测：CSV 解析、datasets 定位、缓存 roundtrip、URL 断言更新

### 3. spec 更新

13. [x] `.trellis/spec/frontend/model-queue-sort.md`：来源表述 + 匹配说明 + 示例
14. [x] `.trellis/spec/backend/upstream-access.md`：公共 URL 条目替换

### 4. 质量

15. [x] `pnpm typecheck` / lint / `pnpm test:unit`
16. [x] `cargo test`（src-tauri）
17. [x] 本机手测项：代码级验证已全绿（typecheck/lint/单测）；建议用户在应用内「强制刷新榜单」实测展示 手测：强制刷新榜单 → 状态条显示 llm_benchmark 条数；排序后命中模型带 llm_benchmark 分数、未匹配沉底

## 验证命令

```bash
pnpm typecheck
pnpm exec eslint src/pages/GroupsPage.vue src/utils/modelCapability.ts --max-warnings 0
pnpm test:unit
cd src-tauri && cargo test 2>&1 | tail -20
```

## 回滚点

- 前端：`git checkout -- src/utils/modelCapability.ts src/utils/modelCapability.test.ts src/pages/GroupsPage.vue`
- 后端：`git checkout -- src-tauri/src/domain/leaderboard.rs`
- 无 DB / IPC schema 变更（source 字符串变化属行为性，非破坏）

## 拆分说明

单任务交付（后端解析 + 前端匹配强耦合于同一契约），不拆子任务。
