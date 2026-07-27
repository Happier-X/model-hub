# Journal - happier (Part 1)

> AI development session journal
> Started: 2026-07-17

---



## Session 1: Tauri 移植 octopus M1 完成

**Date**: 2026-07-17
**Task**: Tauri 移植 octopus M1 完成
**Branch**: `master`

### Summary

完成脚手架、侧车启停、管理 UI、客户端文档四个子任务；父任务归档。真机 octopus.exe 联调待本机验证。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `b17ffc6` | (see git log) |
| `d4e7e83` | (see git log) |
| `f5e8ac0` | (see git log) |
| `3296785` | (see git log) |
| `c560d40` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete

---

## Session: Vue3 内嵌代理迭代（Pi 导出 + 无 Key + 文档）

**Date**: 2026-07-22
**Branch**: `master`

### Summary

在 Vue3 重写与内嵌代理基线之上，继续修旧库兼容、管理台能力，并落地：

1. API Key 页 **一键配置到 Pi Agent**（合并 `~/.pi/agent/models.json` 的 `model-hub`）
2. 本机 `/v1` **允许无客户端 API Key**；错误 Key 仍 401；占位 `model-hub` 放行
3. 补齐发布说明、README、客户端/上手文档与本 journal

此前同周期已归档任务包括：分组/日志/api_keys/group_items 迁移、故障转移与流式空闲超时、管理台 UX、更新检查、上游模型拉取、日志分页、今日统计、批量加模型、队列拖拽等。

### Main Changes

- `src-tauri/src/pi_export.rs` + `export_to_pi_agent` 命令
- `src/pages/ApiKeysPage.vue` 一键配置 UI
- `proxy/server.rs`：`require_key` 无 Key / 占位 Key 放行
- 文档：`docs/release-notes-v0.1.0.md`、`README.md`、`docs/client-integration.md`、`docs/chat-onboarding.md` 等

### Git Commits（节选）

| Hash | Message |
|------|---------|
| `028289c` | feat: API Key 页一键配置到 Pi Agent |
| `680cffa` | feat: 本机 /v1 允许无客户端 API Key |
| `9896b22` | fix: 将 Pi 占位 Key 视为本机无鉴权 |
| （本提交） | docs: 同步 v0.1.0 说明与会话 journal |

### Testing

- `cargo test --lib`（含 `pi_export`）
- `cargo test --test proxy_failover`（无 Key / 占位 Key / 错误 Key）
- `pnpm typecheck` / `pnpm lint`（导出功能合入时）

### Status

[OK] 文档与 journal 同步完成

### Next Steps

- 本机完全重启应用后验证 Pi 无 Key 调用
- 可选：打 `v0.1.0` tag / CI 发版（需 Secrets）


## Session 2: 接入 OpenRouter 模型榜单排序

**Date**: 2026-07-22
**Task**: 接入 OpenRouter 模型榜单排序
**Branch**: `master`

### Summary

实现 OpenRouter 公共榜单拉取与 24h 文件缓存、白名单解析与 stale 回退；前端混合排序（本地/外部通用/编码）、高置信匹配与 GroupsPage UI；更新 backend/frontend code-spec 并提交。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `a3a2302` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 3: 分组页配置到 Pi 与 v0.1.1 发布

**Date**: 2026-07-23
**Task**: 分组页配置到 Pi 与 v0.1.1 发布
**Branch**: `master`

### Summary

完成 OpenRouter 榜单排序与 v0.1.1 发版推送；将 Pi 配置入口迁到分组页，按分组 upsert model-hub、固定占位 Key、移除 API 密钥页全局导出。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `8719a09` | (see git log) |
| `a3a2302` | (see git log) |
| `579e60d` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 4: 移除客户端 API Key 管理与鉴权

**Date**: 2026-07-23
**Task**: 移除客户端 API Key 管理与鉴权
**Branch**: `master`

### Summary

删除客户端 API Key 页面、路由、前后端 IPC 与 domain/apikey，移除代理客户端 Key 校验与 api_keys schema/迁移/测试，保留供应商上游 Key 与 Pi 占位配置；同步文档与 specs。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `f0d8adc` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 5: 禁止对用户上游测活

**Date**: 2026-07-23
**Task**: 禁止对用户上游测活
**Branch**: `master`

### Summary

移除供应商页测试连接；固化禁止自动/后台/AI 默认对用户上游测活的 code-spec；保留分组页点击拉模型、真实 Chat 转发与熔断内存健康展示。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `7cbe744` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 6: 修复退出后端口占用与单实例

**Date**: 2026-07-23
**Task**: 修复退出后端口占用与单实例
**Branch**: `master`

### Summary

修复 stop 超时未 abort 导致端口残留；ProxyHandle Drop 时 best-effort stop；接入 tauri-plugin-single-instance 防止多开；托盘/概览文案区分关窗隐藏与退出停代理。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `98bbd10` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 7: 修复编辑分组误创建重复分组

**Date**: 2026-07-23
**Task**: 修复编辑分组误创建重复分组
**Branch**: `master`

### Summary

使用稳定的编辑分组 ID 隔离创建与更新路径，增加防重复提交及前后端回归测试，并同步前端组件规范。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `c84dff7` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 8: 新建供应商和分组使用对话框

**Date**: 2026-07-23
**Task**: 新建供应商和分组使用对话框
**Branch**: `master`

### Summary

新增通用可访问对话框，将供应商与分组的新建、编辑表单改为按需打开的 Dialog；保存失败保留输入，保存成功刷新列表，分组对话框不自动访问上游。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `22c7cbf` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 9: 移除供应商刷新健康功能

**Date**: 2026-07-23
**Task**: 移除供应商刷新健康功能
**Branch**: `master`

### Summary

移除供应商页面的刷新健康按钮、专用加载状态与点击处理，保留初始健康快照加载和健康状态展示，并同步前端规范。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `dbb6bf0` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 10: 移除分组刷新健康功能

**Date**: 2026-07-23
**Task**: 移除分组刷新健康功能
**Branch**: `master`

### Summary

移除分组页面的刷新健康按钮、专用加载状态与点击处理，保留初始健康快照加载和健康状态展示，并同步前端规范。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `baa31df` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 11: 重置发布为 v0.0.1

**Date**: 2026-07-23
**Task**: 重置发布为 v0.0.1
**Branch**: `master`

### Summary

完整重置历史 tag/Release 与旧 release-notes，版本改回 0.0.1，改用 changelog 维护更新日志，推送 master 与 v0.0.1 触发 Windows 发布工作流。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `f289196` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 12: 修复配置到 Pi 的 Tauri 参数名

**Date**: 2026-07-23
**Task**: 修复配置到 Pi 的 Tauri 参数名
**Branch**: `master`

### Summary

将 export_group_to_pi_agent 与 get_model_leaderboard 的 invoke 参数键改为 camelCase，并同步前端 type-safety 规范，修复配置到 Pi 缺失 groupId 报错。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `df58edb` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 13: 默认端口改为 8888

**Date**: 2026-07-23
**Task**: 默认端口改为 8888
**Branch**: `master`

### Summary

将代理 DEFAULT_PORT 与概览页/文档/规范默认端口从 8080 改为 8888，保留已持久化 shell.json 端口不被覆盖。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `7bc7272` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 14: 发布 v0.0.2

**Date**: 2026-07-23
**Task**: 发布 v0.0.2
**Branch**: `master`

### Summary

将版本统一升至 0.0.2，新增 changelog/v0.0.2.md，推送 master 与 v0.0.2 并成功触发 Windows Release。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `5cc5855` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 15: 发布 v0.0.2

**Date**: 2026-07-23
**Task**: 发布 v0.0.2
**Branch**: `master`

### Summary

将版本统一升至 0.0.2，新增 changelog/v0.0.2.md，推送 master 与 v0.0.2 并成功触发 Windows Release。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `5cc5855` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 16: 修复私有成员运行时错误

**Date**: 2026-07-23
**Task**: 修复私有成员运行时错误
**Branch**: `master`

### Summary

修复 Vue 深层代理 Tauri Update 导致的 private member 错误，改用 shallowRef，增加资源释放逻辑和 23 项回归测试。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `a2806ac` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 17: 取消熔断与 auto_failover，错误即顺序故障转移

**Date**: 2026-07-24
**Task**: 取消熔断与 auto_failover，错误即顺序故障转移
**Branch**: `master`

### Summary

删除供应商熔断与分组 auto_failover；响应提交前任意错误按队列顺序换源；清理 list_health/健康徽章；同步 backend/frontend spec 与迁移删列契约；集成测覆盖模型不支持、普通 4xx、2xx 错误信封与全失败透传。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `e9aa28b` | (see git log) |
| `413ed89` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 18: 渐进接入 happier-ui 替换可映射控件

**Date**: 2026-07-24
**Task**: 渐进接入 happier-ui 替换可映射控件
**Branch**: `master`

### Summary

安装 npm happier-ui 与 @lucide/vue；AppDialog 薄封装 HDialog；四页主要按钮/输入/布尔/空状态渐进替换；保留 Tailwind 与表格/select/侧栏；同步 frontend spec。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `cd12f94` | (see git log) |
| `e7bac07` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 19: 发布 v0.0.3

**Date**: 2026-07-24
**Task**: 发布 v0.0.3
**Branch**: `master`

### Summary

将版本统一升至 0.0.3，新增 changelog/v0.0.3.md，同步 README 版本文案与故障转移描述；推送 master 与 v0.0.3，release-windows 成功并发布 NSIS/签名/latest.json/SHA256SUMS。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `73b17ae` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 20: 概览展示最近成功模型

**Date**: 2026-07-24
**Task**: 概览展示最近成功模型
**Branch**: `master`

### Summary

新增 get_last_success_request IPC 与请求日志查询，概览展示全局最近一次成功请求的分组、供应商、上游模型和时间；无成功记录显示空态，与今日统计并行刷新；补充 Rust 测试、前后端类型接线及相关 spec。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `bdebbb7` | (see git log) |
| `f870b4f` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 21: 概览改名为首页

**Date**: 2026-07-24
**Task**: 概览改名为首页
**Branch**: `master`

### Summary

将应用中「概览」统一更名为「首页」：路由 name=home、OverviewPage→HomePage、侧栏与页面文案、README/docs/changelog/注释及相关 Trellis spec 同步更新；typecheck 与 lint 通过。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `fea1216` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 22: 桌面悬浮状态条显示最近成功模型

**Date**: 2026-07-24
**Task**: 桌面悬浮状态条显示最近成功模型
**Branch**: `master`

### Summary

实现 Windows 桌面悬浮状态条：默认关闭、设置页开关、最近成功模型与代理状态展示、主屏工作区和 DPI 定位、拖动位置持久化、最小权限 capability；完成跨层契约规格与自动化验证。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `97051ce` | (see git log) |
| `1d4e568` | (see git log) |
| `21b8cc3` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## 2026-07-24 - 发布 v0.0.4

**Task**: 发布新版本
**Branch**: `master`

### Summary

完成 Windows 稳定版 v0.0.4 发布，包含最近成功模型展示、首页与设置调整、桌面悬浮状态条。版本材料已提交、推送并通过 GitHub Actions 发布。

### Git Commits

| Hash | Message |
|------|---------|
| `8357e88` | chore(release): v0.0.4 |
| `4cd7817` | chore(task): 落入 release-new-version 任务文档 |

### Testing

- pnpm lint
- pnpm typecheck
- pnpm test:unit（23 项）
- pnpm build
- cargo fmt --check
- cargo check
- cargo test（89 项）
- GitHub Actions release-windows（run 30089675626）成功
- GitHub Release 与 latest.json 资产核验通过

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 23: 主窗口无边框 + 自定义标题栏

**Date**: 2026-07-24
**Task**: 主窗口无边框 + 自定义标题栏
**Branch**: `master`

### Summary

主窗口改 decorations:false，新增 AppTitleBar.vue 全宽标题栏（最小化/最大化还原/关闭三按钮），AppShell 布局改为上标题栏+下侧栏主区。窗口控制走 @tauri-apps/api/window，关闭按钮命中现有 CloseRequested 拦截保持隐藏到托盘语义，Rust 侧零改动。补 6 项 core:window capability 权限，沉淀前端 spec desktop-titlebar.md。lint/typecheck/cargo build 全绿。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `c7b76c8` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 24: 升级 happier-ui 0.0.2 + HCard/HSidebar 替换

**Date**: 2026-07-25
**Task**: 升级 happier-ui 0.0.2 + HCard/HSidebar 替换
**Branch**: `master`

### Summary

happier-ui 0.0.1→0.0.2，修复破坏性 CSS 入口改名 style.css→styles.css；各页面外层 section 卡片改 HCard（标题进 header slot，接受无阴影），AppShell 侧栏改 HSidebar（model-value=route.path + router.push 路由联动）；HIconButton 实测放弃（无 hover 背景态、固定正方圆角、ghost 显蓝、danger 常驻红底，与 AppTitleBar/更新提示/overlay 图标钮交互模型冲突），三处保留原生 button+Tailwind；更新 component-guidelines.md 3.1 组件面边界。lint/typecheck/cargo build 全绿，trellis-check 修复两处 HCard header slot 一致性。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `c66feb3` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete

---

## 2025-07-25 修复故障转移耗尽时 2xx 错误信封透传问题

### 问题
上游供应商（xAI/Grok）返回 HTTP 200 但 body 为结构化 JSON 错误时，检测正确触发故障转移，但所有候选耗尽后 exhausted 分支原样透传最后的 HTTP 200 + 错误体，客户端误以为成功。

### 修改
- **forward.rs**：exhausted 分支新增判断，若最后 HTTP 响应为 2xx 且含结构化错误信封，升级为 502 + 汇总错误摘要
- **failover_any_error.rs**：新增 exhausted_2xx_error_envelopes_return_502 集成测试
- **error-handling.md**：更新 exhausted 行为描述

### 验证
- 74 单元测试通过
- 10/10 failover_any_error 集成测试通过
- 8/8 proxy_failover 集成测试通过

[OK] **Completed**


## Session 25: 发布 v0.0.5

**Date**: 2026-07-25
**Task**: 发布 v0.0.5
**Branch**: `master`

### Summary

Session summary was not supplied.

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `fa539df` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 26: 升级 happier-ui 0.0.3 + 新组件替换手写控件

**Date**: 2026-07-25
**Task**: 升级 happier-ui 0.0.3 + 新组件替换手写控件
**Branch**: `master`

### Summary

Session summary was not supplied.

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `ce7f322` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 27: 修复流式 SSE 帧错误信封漏检导致不换源

**Date**: 2026-07-25
**Task**: 修复流式 SSE 帧错误信封漏检导致不换源
**Branch**: `master`

### Summary

上游流式返回 HTTP 200 + data:{error} 帧时被当正常 SSE 放行、故障转移停止、日志误记 200。扩展 is_structured_error_body：抽出 classify_json_error_envelope 供裸 JSON 与 SSE payload 共用；新增 looks_like_sse / extract_sse_data_payload 剥 data: 帧再判定；[DONE]/纯注释/带 choices 正常 delta 一律放行。耗尽分支自动复用 2xx 信封升级 502。单测 21 + failover_any_error 13(含 3 新 SSE 场景) + proxy_failover 8 全绿；fmt/clippy/build 干净。同步 error-handling.md 判定表。

### Main Changes

- Detailed change bullets were not supplied; see the summary above.

### Git Commits

| Hash | Message |
|------|---------|
| `f392572` | (see git log) |

### Testing

- Validation was not recorded for this session.

### Status

[OK] **Completed**

### Next Steps

- None - task complete

---

**Date**: 2026-07-25
**Task**: 修复 happier-ui 样式被 Tailwind preflight 覆盖（CSS layer 顺序）
**Branch**: `master`

### Summary

用户报 HButton 等 happier-ui 组件「没样式」，怀疑用法错误。排查确认非用法问题，而是 main.ts 的 CSS 引入顺序打乱了 CSS 层叠层优先级。happier-ui 的 styles.css 把组件样式裸包在 `@layer components{}` 且未在文件顶部声明 layer 顺序；原顺序 tokens→styles→index.css 使 happier 的 `components` 被注册为首个层，Tailwind 展开的 theme/base/utilities 追加其后，最终 `components < base`，preflight（base 层 button reset：清背景/边框/内边距）反而覆盖 .h-button。改为 index.css（含 `@import "tailwindcss"`）先加载，Tailwind 先声明 `theme,base,components,utilities` 顺序，.h-button 正确归入 components 层（base < components），preflight 不再覆盖。

### Main Changes

- `src/main.ts`：调整 import 顺序为 index.css → tokens.css → styles.css；加 4 行注释护栏说明顺序敏感与根因。
- `.trellis/spec/frontend/component-guidelines.md` §3.1：新增「CSS 引入顺序（强制）」约定，含根因图示与 issue #10 链接。
- 给 happier-ui 提 issue [#10](https://github.com/Happier-X/happier-ui/issues/10)：建议 styles.css 顶部声明 layer 顺序，避免消费方顺序敏感。

### 构建产物验证

- dist CSS layer 顺序：`theme(2644) < base(5079) < components(8568) < utilities(8586)`；preflight `button` reset 在 pos=6688（base 层内），`.h-button` 在 pos=34872（components 层内）。base < components，preflight 不再覆盖组件样式。

### Git Commits

| Hash | Message |
|------|---------|
| `536f783` | fix(ui): 调整 CSS 引入顺序，修复 happier-ui 被 Tailwind preflight 覆盖 |
| `a1111e9` | docs: 记录 happier-ui#10 与 CSS layer 引入顺序约束 |
| `084780e` | chore(task): archive 07-25-fix-happier-ui-css-layer-order |

### Testing

- typecheck ✓ / lint ✓ / test:unit 23 ✓ / build 1858 modules ✓
- 构建产物 layer 顺序人工核对 ✓

### Status

[OK] **Completed & Archived**

### Next Steps

- None - task complete


## Session 28: 主窗口标题栏改用 happier-ui 图标按钮

**Date**: 2026-07-27
**Task**: 主窗口标题栏改用 happier-ui 图标按钮
**Branch**: `master`

### Summary

将主窗口自定义标题栏的窗口控制按钮（最小化/最大化-还原/关闭）从原生 button+lucide 改为 happier-ui 的 HButton（variant=ghost、isIconOnly、size=sm），标题栏背景由深色 bg-slate-900 改为浅色 bg-white + 底边框，高度调至 h-11。同步更新 desktop-titlebar.md spec 记录新的浅色主题与 HButton 契约。类型检查通过。

### Git Commits

| Hash | Message |
|------|---------|
| `17ff10d` | (see git log) |

### Status

[OK] **Completed**
