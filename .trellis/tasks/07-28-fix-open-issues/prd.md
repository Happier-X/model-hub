# 修复 3 个 open issues (#1 #2 #3)

## Goal

修复 `Happier-X/model-hub` 仓库当前 3 个 open GitHub Issues，每个 issue 一个独立子任务，
可分别修+验证+回滚。本父任务只统领进度与跨子任务一致性，不做实施目标。

## 子任务映射

| 子任务 | Issue | 简述 | 主要涉及层 |
|--------|-------|------|-----------|
| `07-28-issue1-today-requests-1000` | [#1](https://github.com/Happier-X/model-hub/issues/1) | 首页"今日请求"总请求数总显示 1000 | backend 统计逻辑 + frontend 展示（待研究确认） |
| `07-28-issue2-heatmap-only-today` | [#2](https://github.com/Happier-X/model-hub/issues/2) | 每日请求量热力图只展示当天色块，未展示一整年 | backend 按天聚合 + frontend HHeatmap 数据源（待研究确认） |
| `07-28-issue3-overlay-screen-change` | [#3](https://github.com/Happier-X/model-hub/issues/3) | 桌面悬浮条从双屏切到单屏后变小、无法拖动 | backend overlay 窗口尺寸适配 + 拖动坐标 clamp（待研究确认） |

## 父任务职责

- 串起三个子任务的总体进度（每个子任务独立 brainstorm → 实现 → 验证 → 归档）。
- 跨子任务一致性：三个 fix 不能相互破坏（例如改后端统计查询时不能同时影响 #1 和 #2 的语义）。
- 不直接实施；实施在子任务里。

## 跨子任务验收

- [ ] 三个子任务各自归档
- [ ] `npm run build` / `npm run lint` / `npm run test:unit` 全部通过
- [ ] 后端测试（如果存在）通过
- [ ] 三个 issue 在 GitHub 上可由用户关闭（实际关闭由用户在 GitHub 操作，本会话不自动关 issue）

## Notes

- 三个 issue 正文在 GitHub 未登录抓取下不可见，仅有标题线索。每个子任务 brainstorm 阶段
  需先**研究代码定位根因**（用 grep / 读源码确认是前端还是后端 bug），再决定 PRD 范围。
- 执行顺序：建议按 **#1 → #2**（都涉及首页统计，可能共享后端查询逻辑，可一并研究）→
  **#3**（独立 overlay 模块，互不影响）。但子任务独立可分别归档，顺序仅为方便。
- 每个子任务最终 PRD 写明：根因、修复点（具体文件/函数）、验收条件、回归测试（如有）。