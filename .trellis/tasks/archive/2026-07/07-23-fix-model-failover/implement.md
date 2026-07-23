# 实施计划：无熔断顺序故障转移

## 1. 后端转发路径

- [ ] 修改 `src-tauri/src/proxy/forward.rs`：移除 `CircuitRegistry` 参数、`ProbeReleaseGuard` 和所有熔断调用。
- [ ] 删除 `auto_failover` 参数和只尝试第一项的分支，始终按启用候选项顺序遍历。
- [ ] 将非 `2xx` HTTP 响应统一作为可换源失败；保存原始状态、响应头、响应体和错误摘要。
- [ ] 增加或调整 `2xx` 结构化错误信封识别；非流式返回前检查，流式首包提交前只在明确 JSON 错误信封时换源。
- [ ] 队列耗尽时：最后失败有原始 HTTP 响应则透传；最后失败为网络/超时/读取错误则返回明确 `502/504` 类网关错误。
- [ ] 保持流式首包提交后的静默超时/读取错误单日志规则，但移除熔断失败记录。

## 2. 后端运行时与命令清理

- [ ] 修改 `src-tauri/src/proxy/server.rs`：构造 `AppState` 和调用 `forward_with_failover` 时不再传递 `circuits` 或 `auto_failover`。
- [ ] 修改 `src-tauri/src/proxy/runtime.rs`：移除 `RuntimeInner.circuits`、`ProxyHandle::circuits` 和启动时的熔断状态传递。
- [ ] 修改 `src-tauri/src/proxy/mod.rs` 并删除 `src-tauri/src/proxy/circuit.rs`。
- [ ] 修改 `src-tauri/src/commands.rs` 和 `src-tauri/src/lib.rs`：删除 `HealthSnapshot`、`list_health` 和命令注册。

## 3. 分组模型与数据库迁移

- [ ] 修改 `src-tauri/src/domain/group.rs`：删除 `Group`、`CreateGroupPayload`、`UpdateGroupPayload` 中的 `auto_failover`，调整 SQL select/insert/update 与单测。
- [ ] 修改 `src-tauri/src/db/migrate.rs`：新 schema 删除 `groups.auto_failover`；旧库迁移移除该列并保留已有分组及条目。
- [ ] 修改 `src-tauri/src/db/mod.rs` 中建库测试与日志测试对分组结构的断言。
- [ ] 全局搜索 `auto_failover`，确保只在历史兼容测试说明中出现或完全消失。

## 4. 前端契约与 UI 清理

- [ ] 修改 `src/api/tauri.ts`：删除 `auto_failover` 字段、`HealthSnapshot` 类型和 `listHealth` 方法。
- [ ] 修改 `src/pages/GroupsPage.vue`：删除表单字段、开关 UI、卡片状态文案、保存 payload 字段和健康状态加载。
- [ ] 修改 `src/pages/ProvidersPage.vue`：删除健康状态加载和健康列。
- [ ] 删除无引用的 `src/components/HealthBadge.vue` 与 `src/utils/health.ts`。
- [ ] 全局搜索 `listHealth`、`HealthBadge`、`consecutive_failures`、`healthy` 等熔断展示遗留引用。

## 5. 测试改写与新增

- [ ] 改写 `src-tauri/tests/proxy_failover.rs` 测试环境，不再创建或断言 `CircuitRegistry`。
- [ ] 新增/调整测试：模型不支持 `400` → 第二候选成功。
- [ ] 新增/调整测试：普通参数 `400` / `404` / `5xx` → 第二候选成功。
- [ ] 新增/调整测试：多次请求均从第一候选开始，不存在熔断跳过。
- [ ] 新增/调整测试：全部候选 HTTP 错误时透传最后候选状态、响应头和错误体。
- [ ] 新增/调整测试：最后失败为网络/超时时返回明确网关错误。
- [ ] 保留并改写流式首包后静默超时测试：不换源、单日志、无熔断断言。
- [ ] 删除或改写关闭 `auto_failover`、熔断计数、HalfOpen 相关测试。

## 6. 质量验证

- [ ] `cd src-tauri && cargo fmt -- --check`
- [ ] `cd src-tauri && cargo test`
- [ ] `cd src-tauri && cargo check`
- [ ] `npm run lint`
- [ ] `npm run test:unit`
- [ ] `npm run typecheck`
- [ ] `npm run build`
- [ ] 检查日志未包含 API Key 或完整 messages。
- [ ] 检查没有后台测活或新增上游探测请求。

## 风险与回滚点

- SQLite 删除列迁移风险：先用测试覆盖旧库含 `auto_failover` 的升级路径，再改运行时查询。
- 流式首包前 `2xx` 错误信封识别风险：仅对明确 JSON 错误信封处理，SSE `data:` 不误判。
- 前后端契约同步风险：以 `rg auto_failover listHealth HealthSnapshot CircuitRegistry` 做最终扫尾。
- 所有错误换源可能增加上游请求次数：这是已确认产品行为，日志中保留故障转移摘要便于诊断。
