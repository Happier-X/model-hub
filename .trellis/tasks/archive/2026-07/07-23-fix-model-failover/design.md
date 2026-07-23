# 技术设计：简化故障转移

## 设计目标

1. 每次请求始终从分组队列第一个启用候选项开始，按顺序尝试。
2. 响应提交客户端前，当前候选项任意失败都继续下一候选项。
3. 彻底移除供应商熔断与分组 `auto_failover`。
4. 修复“模型不支持却不换源”问题，作为“任意失败均换源”的子集一并覆盖。

## 边界与职责

| 区域 | 改动 |
|------|------|
| `proxy/forward.rs` | 删除熔断与 `auto_failover` 分支；错误即换源；保存最后原始响应 |
| `proxy/circuit.rs` | 删除模块或改为空壳后彻底移除引用 |
| `proxy/server.rs` / `proxy/runtime.rs` | 移除 `CircuitRegistry` 依赖 |
| `domain/group.rs` + 迁移 | 删除 `auto_failover` 读写与 schema |
| `commands.rs` / `lib.rs` | 删除 `list_health` 注册 |
| 前端 `tauri.ts`、分组/供应商页、健康组件 | 删除开关、健康徽章与相关类型 |
| 测试 | 重写为无熔断、无开关、任意失败换源 |

不新增后台测活、不改队列排序算法、不删除本机 `GET /health`。

## 运行时行为

### 候选选择

- 仅跳过 `enabled = false` 的供应商。
- 不再检查熔断状态，也不再占用 HalfOpen 探测位。
- 每次请求从队列第一项开始。

### 错误即换源

在响应尚未提交客户端前，下列结果均进入下一候选项：

1. 网络错误、连接失败、读 body 失败。
2. 首包超时（流式）与现有超时语义。
3. 任意非 `2xx` HTTP 状态，包括普通参数 `400`、`404`、模型不支持、鉴权、限流、`5xx`。
4. 明确的 `2xx` 结构化错误信封：
   - 非流式：完整 JSON body 可解析为错误信封。
   - 流式：首个数据块本身是明确的非 SSE JSON 错误信封。
5. 正常 `2xx` 成功 JSON 或正常 SSE 首包不得换源。

### 最终响应

- 任一候选项成功：返回该成功响应，并保留 `failover_from` / `failover_to` / `failover_reason`。
- 全部失败且最后一次有上游 HTTP 响应：透传最后候选项的状态、响应头、响应体。
- 全部失败且最后一次无上游响应：返回明确网关错误（如 `502` + 摘要）。

### 流式边界

- 首包提交前：可换源。
- 首包提交后：只透传当前上游；静默超时/读错误/客户端断开只记当前流的最终日志，禁止拼接第二家。

## 模型不支持与 2xx 错误信封

为兼容用户错误体 `{"error":"当前 API 不支持所选模型 gpt-5.6-sol","type":"error"}`：

- 提取路径：字符串 `error`、对象 `error.message`、顶层 `message` 等常见字段。
- 若 body 是明确错误信封，则即便状态码是 `2xx`，也按失败进入换源。
- 不依赖“仅出现 model 单词”做成功/失败判定；成功响应以正常 chat completion / SSE 语义为准。

## 删除熔断

移除：

- `CircuitRegistry` 及 `proxy/circuit.rs` 状态机。
- `forward` 中的 `allow_request` / `record_success` / `record_failure` / `release_probe` / `ProbeReleaseGuard`。
- 流式回调中的熔断成功/失败记账。
- `list_health` 命令、`HealthSnapshot` 类型、前端健康徽章与页面调用。

流式静默超时仍写失败日志，但不再推进熔断。

## 删除 `auto_failover`

### Schema

- 新建库：`groups` 不再包含 `auto_failover`。
- 旧库：迁移中停止添加该列；若列已存在，用兼容 SQLite 的重建表方式删除列，并完整拷贝 `id/name/created_at` 与 `group_items`。
- 迁移幂等；不得丢失分组与条目。

### 领域与契约

- `Group` / `CreateGroupPayload` / `UpdateGroupPayload` 删除字段。
- 前端 `Group` 类型、创建/更新 payload、`GroupsPage` 开关与列表文案删除。
- 所有测试去掉 `auto_failover` 构造与分支。

## 日志

保留字段：时间、分组、供应商、上游模型、状态码、耗时、error、failover_from/to/reason。

规则：

- 中间失败可写尝试摘要；最终结论仍清晰。
- 流式：首包成功后不得立刻记 200；终态只写一条结论。
- 禁止完整 API Key 与 messages。

## 风险与兼容

| 风险 | 控制 |
|------|------|
| 普通参数错误会打完整队列 | 已确认产品接受“任意失败都换源” |
| `2xx` 错误信封误判 | 仅识别明确错误信封；正常 completion/SSE 不换源；加正反例 |
| SQLite 删列 | 用重建表 + 数据拷贝；迁移测试覆盖新旧库 |
| 前端死代码 | 删除健康组件/工具与引用；类型检查兜底 |

## 回滚

可按模块回退：

1. 恢复 `auto_failover` 字段与开关。
2. 恢复 `CircuitRegistry` 与健康展示。
3. 恢复“仅部分状态可换源”的分类。

无外部服务依赖，不涉及云端迁移。
