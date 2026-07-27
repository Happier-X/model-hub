# 自动同步供应商模型到分组

## Goal

给分组新增「绑定供应商 + 后台定时全量同步该供应商模型」的能力。用户把某个分组绑定到一个供应商后，应用每 24 小时自动用该供应商上游 `GET /models` 的最新模型列表全量刷新这个分组，让分组内容持续反映供应商当前可用模型，无需手动拉取和保存。

用户原话：「现在会隔一段时间自动拉取供应商里面所有的模型到分组中吗」→ 当前没有该能力，本任务新增。

## Background（已确认事实）

- 当前**无任何自动/定时同步**。拉取模型全靠手动：
  - `pullModels()`（GroupsPage.vue）：分组编辑里点某行「拉取模型」。
  - `bulkAddProviderModels()`（GroupsPage.vue）：选供应商→拉全部模型→加进编辑分组队列，仍需点「保存」落库。
- 拉取实现：`fetch_upstream_model_ids(base_url, api_key) -> Result<Vec<String>, AppError>`（`domain/upstream_models.rs`）。GET `{base_url}/models`，解析 OpenAI 风格 `data[].id`，有序去重；超时 15s / 连接 10s；管理侧直连不走故障转移。
- 数据模型：
  - `providers`：id, name, base_url, api_key, enabled, created_at。
  - `groups`：id, name, items[], created_at, thinking_effort（`db/migrate.rs` schema v1 + `ensure_group_columns` 加列模式）。
  - `group_items`：id, group_id, provider_id, upstream_model, sort_order（旧 gateway-rust 表还有 `channel_id/model_name/priority/weight` NOT NULL，`replace_items` 检测旧列并双写兼容）。
- 分组 CRUD：`Stores::create_group / update_group`（`domain/group.rs`），事务内 `replace_items` 全量重写 group_items。
- 壳配置持久化：`shell.json` / `ShellConfig`（`settings.rs`），已有 `gateway_port`、`check_update_on_startup`、`overlay_enabled`、`overlay_x/y`；原子写（tmp→bak→rename）。
- 后台任务：无通用调度器。proxy 有独立 tokio runtime（`proxy/runtime.rs`，`ProxyHandle.tokio_rt.spawn`）。前端有 setInterval 用例（LogsPage、OverlayApp）。
- Tauri 命令注册在 `lib.rs` 的 `invoke_handler!`；启动逻辑在 `.setup(|app| ...)`；`ProxyHandle` 通过 `app.manage` 注入。
- 前端命令封装集中在 `src/api/tauri.ts`；分组页 `src/pages/GroupsPage.vue` 用 TanStack Form。

## Key Decisions

1. **组织形态：供应商专属分组**。分组与供应商一对一绑定，全量刷新该分组模型列表；不影响用户手动编排的混合分组。
2. **绑定入口：在已有分组上绑定（A2）**。用户先建分组，再在分组编辑里选「绑定供应商并自动同步」。给 `groups` 加可空列 `source_provider_id` 标记归属；同步按此列找目标分组。
3. **触发：后台定时，固定 24 小时**。
   - **不在应用启动时拉**。定时器从应用启动时刻起算，24h 后第一次触发，之后每 24h 一轮。
   - 无可配周期、无设置页周期下拉。
   - 定时器复用 proxy 的 `tokio_rt`（或应用级 tokio runtime）承载。
4. **写入语义：完全托管纯镜像（S1）**。每次同步用上游最新 `data[].id` 全量覆盖绑定分组：上游新增的进入、下线的删除。绑定状态下分组为「只读托管」——前端禁止手动增/删/改排该分组的模型条目；解绑后恢复普通可编辑分组。
5. **空 vs 失败区分**：
   - 上游 **HTTP 200 且空列表** → 视为供应商当前无可用模型，**清空该分组**（保留旧的会挡住故障转移，无意义）。
   - **拉取失败**（网络/超时/HTTP 4xx/5xx，即 `fetch_upstream_model_ids` 返回 `Err`）→ **保留上一次模型列表不动**，写 tracing 日志，等下一轮重试。
6. **供应商 enabled 语义**：仅对 `enabled = true` 的供应商发起同步；被禁用供应商的绑定分组本轮跳过（不清空、不拉取）。
7. **合规例外（upstream-access.md）**：本任务在既有「禁止后台/定时访问用户上游」约定上，新增一条**记录在案的例外**——仅当分组 `source_provider_id` 非空（用户主动绑定，视为显式授权）且供应商 enabled 时，后台定时器才对该供应商发起 `GET /models`。无绑定分组时后台零上游访问。需同步更新 `upstream-access.md`。
8. **手动「立即同步」**：绑定分组支持用户点击「立即同步」按钮，即时触发一次全量刷新（复用同一同步逻辑），不依赖 24h 周期。

## Requirements

### R1 数据库：groups 增加 source_provider_id
- 给 `groups` 表加可空列 `source_provider_id INTEGER`（NULL = 未绑定/普通分组）。
- 用 `ensure_group_columns` 同款「PRAGMA table_info 检测缺列 → ALTER TABLE ADD COLUMN」加列模式，幂等，不重建表、不丢数据。
- `Group` 结构体、`load_items`/`list_groups`/`get_group_by_name` 查询、create/update payload 均带上该字段。
- 供应商删除时，其绑定分组的 `source_provider_id` 行为需明确：`group_items` 已有 `ON DELETE CASCADE` 到 providers，删供应商会清空该分组条目；`source_provider_id` 列本身建议置空（解绑），避免悬空引用。

### R2 领域：同步逻辑
- 新增同步函数（如 `Stores::sync_group_from_provider(group_id)` 或 domain 层 `sync_bound_group`），流程：
  1. 读分组的 `source_provider_id`，为空则报错/跳过。
  2. 读供应商；`enabled = false` 则跳过（不改分组）。
  3. `fetch_upstream_model_ids(base_url, api_key)`。
  4. `Ok(ids)`（含空）→ 事务内全量 `replace_items` 为 `ids` 映射的 `{provider_id: source_provider_id, upstream_model: id}`（保持上游返回顺序为 sort_order）。空列表 → 清空。
  5. `Err(_)` → 不改分组，返回错误/记日志。
- 新增「扫描所有绑定分组并逐个同步」的批量入口（如 `sync_all_bound_groups`），供后台定时器与手动全量触发调用；单个分组失败不影响其它分组。

### R3 后台定时器（24h）
- 应用启动后在 `tokio_rt` 起一个后台任务：`tokio::time::interval(Duration::from_secs(24*3600))`，首次 tick 立即返回需 skip（或用 sleep 先等 24h 再循环），确保**启动时不拉**。
- 每轮调用 `sync_all_bound_groups`；全程 best-effort，异常只记 tracing warn，不 panic、不影响代理主流程。
- 应用退出时任务随 runtime 结束即可（无需精细 join，但不得阻塞退出）。

### R4 IPC 命令
- `sync_group_now(group_id)`：手动触发单个绑定分组同步，返回刷新后的 `Group`（或同步结果）。
- create/update group payload 增加可选 `source_provider_id`；`list_groups` 返回值带该字段。
- 在 `lib.rs` `invoke_handler!` 注册新命令。

### R5 前端
- `src/api/tauri.ts`：`Group` 类型加 `source_provider_id?: number | null`；create/update payload 加该字段；新增 `syncGroupNow(groupId)` 封装。
- `GroupsPage.vue` 分组编辑：
  - 新增「绑定供应商并自动同步」控件（选供应商 → 设置 `source_provider_id`；可解绑设为 null）。
  - 绑定状态下模型队列区为**只读托管**：隐藏/禁用手动添加、删除、拖拽排序、批量添加、单行拉取；给出「本分组由供应商 X 托管，每 24h 自动同步」说明文案 + 「立即同步」按钮。
  - 解绑后恢复普通编辑能力。
- 列表行/编辑态标识哪些分组是托管分组（如徽章「自动同步」）。

### R6 Spec 更新
- `upstream-access.md`：新增「绑定分组后台定时同步」为记录在案的例外（触发条件、频率、enabled 约束）。
- `component-guidelines.md` 第 11 条相关：补充「绑定分组的后台自动同步是显式授权例外，手动拉取约定不变」。
- 视情况在 `database-guidelines.md` 记录 `source_provider_id` 列与只读托管语义。

## Acceptance Criteria

- [ ] AC1：`groups` 迁移后含 `source_provider_id` 列；旧库升级不丢分组/条目，migrate 幂等（新增迁移单测）。
- [ ] AC2：绑定分组调用同步，上游返回 `[m1,m2,m3]` → 分组条目正好是这三个、provider 均为绑定供应商、顺序与上游一致。
- [ ] AC3：上游 HTTP 200 空列表 → 绑定分组被清空（0 条）。
- [ ] AC4：拉取失败（`fetch_upstream_model_ids` 返回 `Err`）→ 分组条目保持同步前不变。
- [ ] AC5：供应商 `enabled = false` → 该绑定分组本轮跳过，条目不变。
- [ ] AC6：后台定时器**启动时不发起**任何上游请求；无绑定分组时后台零上游访问（审计 / 单测可验证 skip 首 tick 行为）。
- [ ] AC7：前端绑定分组为只读托管——无法手动增/删/改排模型；解绑后恢复可编辑。
- [ ] AC8：手动「立即同步」按钮触发单分组全量刷新并回显最新条目。
- [ ] AC9：`upstream-access.md` 与 `component-guidelines.md` 已记录该后台同步例外与只读托管约定。
- [ ] AC10：质量门禁通过——`cargo fmt` / `clippy` / `cargo test`（含新迁移与同步单测）、前端 `typecheck` / `lint` / `test:unit` / `build` 全绿。

## Out of Scope

- 可配同步周期 / 设置页周期下拉（固定 24h）。
- 启动时自动同步一轮（明确不做）。
- 上游下线模型的软删标记 / 僵尸条目保留（采用纯镜像，直接删）。
- 一个分组绑定多个供应商 / 多供应商混合自动同步（一对一）。
- 非 OpenAI 风格 `/models` 响应格式适配（沿用现有解析）。
- 同步失败的桌面通知 / 弹窗（仅 tracing 日志）。

## Notes

- 复杂任务，已具备 `design.md` 与 `implement.md`。
- 本任务需修改既有安全 spec（`upstream-access.md`），实现前须确认该例外可接受（已与用户确认「绑定即授权 + 固定 24h」方向）。
