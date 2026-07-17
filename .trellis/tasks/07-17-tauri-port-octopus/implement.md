# 执行计划：Tauri 移植 octopus（父任务）

> 父任务处于 **planning**。实现在子任务中进行；本文件是地图与门禁。

## 0. 前置（B1，可与父任务规划并行收尾）

- [x] 产品决策 D1–D7 写入 `prd.md`
- [x] 目标栈最小 spec 写入 `.trellis/spec/{backend,frontend}/`
- [x] bootstrap 已归档：`00-bootstrap-guidelines`
- [x] 父任务规划产物 + 子任务地图已执行
- [x] 配置 `implement.jsonl` / `check.jsonl`
- [x] 四个子任务均已实现并归档（scaffold / sidecar / admin-ui / e2e-docs）

## 1. 子任务执行顺序

### 1.1 脚手架（建议 slug: `mvp-scaffold`）

- 初始化 Tauri 2 + React + Vite + TS + Tailwind（Windows）
- 建立 `src-tauri` paths 模块与 README 开发说明
- 验证：`pnpm tauri dev`（或等价）窗口可开
- 依赖：无
- 回滚：删除业务目录，保留 `.trellis`

### 1.2 侧车集成（建议 slug: `mvp-gateway-sidecar`）

- 钉扎侧车版本与获取方式（`gateway/README.md`）
- 实现 start/stop/status/health、默认 127.0.0.1、数据目录注入
- 处理与 D3/D6 冲突的登录/Key 适配
- 验证：AC6；状态条
- 依赖：1.1
- 回滚：停用自动启动，保留 UI 壳

### 1.3 管理 UI MVP（建议 slug: `mvp-admin-ui`）

- 渠道 / 分组 / 日志 / 设置页；无登录
- HTTP client 对接侧车；错误与空态
- 验证：AC1；可完成配置闭环
- 依赖：1.2（可先 mock HTTP，但验收需真侧车）

### 1.4 E2E 与文档（建议 slug: `mvp-e2e-docs`）

- OpenAI Chat 真上游或 mock 验收 AC2–AC5
- 客户端对接文档（base_url、model=分组名、占位 api_key）
- 许可证致谢
- 依赖：1.2 + 1.3

## 2. 验证命令（脚手架落地后替换为真实脚本）

```bash
# 前端
pnpm lint
pnpm build

# Tauri / Rust
cd src-tauri && cargo check

# 手动
# 1) 启动应用 → 网关 running
# 2) 配置渠道与分组
# 3) curl 或 SDK 调 /v1/chat/completions
```

## 3. 父任务完成定义

- M1 验收 AC1–AC6 全部勾选
- 子任务均已 archive 或明确遗留
- spec 已按真实目录修订一轮（`trellis-update-spec`）
- 用户确认后父任务 finish/archive

## 4. 风险检查点

| 检查点 | 动作 |
|--------|------|
| 侧车无法关登录 | 评估最小补丁 vs 反代 vs 换实现；更新 PRD |
| 端口占用 | UI 提示；设置改端口 |
| AGPL 分发 | README 与发布说明 |

## 5. 明确不做（实现时拒绝范围蔓延）

- 管理台登录、强制网关 Key
- macOS/Linux 验收
- MySQL/PG、全协议、价格同步
