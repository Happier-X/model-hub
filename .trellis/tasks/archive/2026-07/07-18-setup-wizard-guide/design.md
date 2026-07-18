# 设计：首次配置向导与闭环引导

## 边界

| 层 | 职责 |
|----|------|
| 前端 | 仪表盘清单组件、状态聚合、导航回调、复制 |
| 现有 API | 复用 `listChannels` / `listGroups` / `listApiKeys` |
| 壳 | 仅读 `gateway` / `authOk`；无新 invoke |

## 组件结构

```text
DashboardPage (或 SetupGuidePage)
  props: running, authOk, authMessage, baseUrl, onNavigate(item)
  SetupChecklist
    steps[] → status + action
  ClientQuickstart
    curl templates + copy
```

建议：

- 将现有 `DashboardPage` 替换为独立文件 `src/pages/DashboardPage.tsx`，避免 `App.tsx` 继续膨胀。
- `App` 传入 `setActiveItem` 包装为 `onNavigate`。

## 状态机（检测）

```text
if !running → step1 fail; step2–5 blocked
if running && !authOk → step1 ok; step2 fail; step3–5 blocked
if running && authOk → fetch lists in parallel
  step3 = channels.length > 0
  step4 = groups.length > 0
  step5 = apiKeys.length > 0
step6 = always "info" (展示示例，不记为阻塞)
```

整体就绪：`step1..5` 全 true 时展示绿色「配置闭环已就绪」横幅。

## 数据流

```text
Dashboard mount / refresh click / running|authOk change
  → Promise.allSettled([listChannels, listGroups, listApiKeys])
  → set counts / errors
  → render
跳转 → onNavigate("渠道" | "分组" | "API 密钥" | "设置")
```

错误处理：list 失败显示步骤级错误文案，不崩溃整页。

## 与 GatewayGate

- 仪表盘**不**用 GatewayGate 整页挡住：即使用户未启动网关，仍应看到步骤 1 与去设置引导。
- 仅在拉取 list 时判断 auth。

## 回滚

- 删除/还原 Dashboard 组件与 App 挂载即可；无持久化 schema 变更（localStorage 可选）。
