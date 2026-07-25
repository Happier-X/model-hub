# 执行计划：模型思考强度设置

## 前置

- 分支：在 `master` 上直接改（沿项目习惯，最后单 commit）
- 校验命令：
  - 前端：`pnpm lint` / `pnpm typecheck` / `pnpm test:unit`
  - 后端：`cd src-tauri && cargo test` / `cargo check`

## 步骤

### 1. 后端数据层

- [ ] `db/migrate.rs`：在 `ensure_group_columns` 内追加 `thinking_effort` 加列逻辑（缺列则 `ALTER TABLE groups ADD COLUMN thinking_effort TEXT NOT NULL DEFAULT 'off'`）
  - 新增迁移测试：老库无该列 → 迁移后有列且默认 `off`；幂等重跑值不变
- [ ] `domain/group.rs`：
  - `Group` 加 `thinking_effort: String`
  - `CreateGroupPayload` / `UpdateGroupPayload` 加 `thinking_effort: Option<String>`
  - 新增 `normalize_effort(&str) -> &'static str`（未知 → `off`）
  - group 查询 SELECT 增加该列（`load_groups`/`get_group_by_name` 对应 row.get 索引）
  - `create_group` / `update_group` 写入归一化后的值
  - 扩展/新增单测：创建带档位、更新档位、未知值回退 off

### 2. 后端转发层

- [ ] `proxy/forward.rs`：
  - 新增 `apply_thinking_effort(obj, upstream_model, effort)` + 家族识别 `thinking_family(model)`
  - `rewrite_model` 改签名为 `(body, upstream_model, effort)`，在 `strip_tool_strict` 后调用 `apply_thinking_effort`
  - `attempt_non_stream` / `attempt_stream_prime` 增加 `effort` 参数并透传
  - `forward_with_failover` 增加 `effort: &str` 参数
  - 单测覆盖：
    - GPT-5 high → `reasoning_effort:"high"`
    - o3 minimal → `reasoning_effort:"low"`（不支持 minimal 降级）
    - Claude sonnet-4 medium → `thinking.budget_tokens:8192`
    - Qwen3 auto → `enable_thinking:true`
    - Qwen3 off → 不注入（off 入口即 return，Qwen 不例外）
    - GPT-4o high → 不注入（None 家族）
    - Claude off → 不注入
    - 客户端已带 `reasoning_effort` → 不覆盖
    - effort=off（任意家族）→ body 与只过 strip_tool_strict 后逐字节相同

- [ ] `proxy/server.rs`：`chat_completions` 读 `group.thinking_effort`，传入 `forward_with_failover`
- [ ] 修正 `tests/proxy_failover.rs` / `tests/failover_any_error.rs` 若因签名变更编译失败（补默认 `"off"` 实参）

### 3. 前端

- [ ] `api/tauri.ts`：
  - 加 `ThinkingEffort` 类型
  - `Group` 接口加 `thinking_effort`
  - `createGroup` / `updateGroup` payload 加 `thinking_effort`
- [ ] `pages/GroupsPage.vue`：
  - `GroupFormValues` + 表单默认值加 `thinking_effort`（默认 `off`）
  - `startEdit` 回填、`resetForm` 归零、`onSubmit` payload 带上
  - 对话框内加 `HSelect`（档位选择，中文标签）
  - 分组列表：非 off 显示思考徽章

### 4. 校验与文档

- [ ] `pnpm lint && pnpm typecheck && pnpm test:unit`
- [ ] `cd src-tauri && cargo test && cargo check`
- [ ] README / docs 补一句：思考强度为分组级，客户端自带字段优先；Claude 需客户端给足 `max_tokens`
- [ ] changelog 追加条目（若发版）

## 验证门（review gate）

- 后端全部 `cargo test` 通过，含新增家族映射单测
- 前端三项校验通过
- 手动：新建分组选「高」，对 GPT-5 分组发一次请求，日志/上游确认 `reasoning_effort` 已注入（可留待联调）

## 回滚点

- 数据层与转发层解耦：转发层 `apply_thinking_effort` 若出问题，`effort` 传 `"off"` 即恢复原行为
- schema 新列默认 off，回退代码无副作用
