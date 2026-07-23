# 技术设计

## 决策边界

| 允许 | 禁止 |
|------|------|
| 用户真实 Chat → 代理转发上游（含故障转移） | 启动/定时/后台对用户供应商 ping、空 chat、假 health |
| 熔断 HalfOpen：真实请求路径占用探测位 | 为恢复熔断而**单独**发起上游请求 |
| 用户点击「拉取模型」→ `fetch_provider_models` | 打开供应商/分组页、保存供应商时**自动**拉 `/models` |
| `list_health` 读内存熔断状态 | 把 list_health 做成打上游 |
| 本机 `GET /health` | 用用户 Key 打上游当 health |
| OpenRouter 公共榜单（无供应商 Key） | 用用户 `providers.api_key` 做榜单/测活 |

## Spec 落点

1. **新建** `.trellis/spec/backend/upstream-access.md`：上游访问合同（7 段可执行：范围、签名/路径、合同、错误矩阵、案例、测试、对错）。
2. **更新** `error-handling.md`：澄清熔断探测 ≠ 测活。
3. **更新** `frontend/component-guidelines.md`：禁止自动拉模型/测活 UI；拉取模型仅点击。
4. **可选** `guides/` 短检查项：改代理/供应商前是否引入自动上游请求。
5. **索引** backend/frontend `index.md` 链接新文档。

## 代码审计

搜索并确认无：

- 定时器 / on_start 调用 `fetch_upstream` / `reqwest` 打 `providers.base_url`
- 管理面启动批量健康检查

当前预期：仅 `forward.rs`（业务）、`upstream_models.rs`（用户触发 IPC）、`leaderboard`（公共 URL）。

## 实现量

以 **spec + 文档表述 + 审计** 为主；若审计发现自动探测再删代码。无行为变更则无需大改产品逻辑。
