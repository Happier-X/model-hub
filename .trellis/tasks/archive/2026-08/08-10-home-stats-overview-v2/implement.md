# 08-10-home-stats-overview-v2 — Implement

## 步骤

1. **纯函数工具** `src/utils/formatTokenCount.ts` + 单测：
   - `formatTokenCount(n)`: <1000 原样；≥1000 `x.xk`（1 位小数，去尾 0）
   - `formatNumber(n)`: `toLocaleString('zh-CN')` 千分位
   - `node:test` 单测（含 0、999、1000、12345、负数钳制）
2. **新建组件** `src/components/StatsCards.vue`：
   - props：`overview: RequestOverview | null`、`error: string`、`onRefresh: () => void`
   - 内部：`mode` ref（total/today）、`displayRow` computed、AnimatedNumber（rAF）、入场动画 flag
   - 4 卡片模板（见 design.md 映射表），图标从 `@lucide/vue` 导入
   - tab 用 shadcn Button variant（ghost/outline 切换态），刷新按钮 outline sm
3. **HomePage.vue**：
   - import StatsCards；模板删除旧「统计总览」两行表格卡片，替换 `<StatsCards :overview :error :on-refresh="refreshStats" />`
   - 确认 `overview`/`overviewError`/`refreshStats` 复用，无残留未用 import
4. **验证**：`pnpm typecheck` → `pnpm lint` → `pnpm test:unit`（新增 formatTokenCount 测试全绿）→ `pnpm build`
5. **浏览器验证**（可选）：`pnpm preview --port 5199` + headless Edge dump DOM 确认 4 卡片渲染（无 invoke 数据时为 0/-）
6. **spec 同步**：`frontend/component-guidelines.md` 若有组件目录约定，确认 StatsCards 符合；无则跳过
7. **commit + journal**：`feat(frontend)` commit；`add_session.py` 记录

## 验证命令

```bash
pnpm typecheck
pnpm lint
pnpm test:unit
pnpm build
```

## 回滚点

- commit 前 `git checkout -- src/pages/HomePage.vue src/components` 可整体回退；组件为新文件，删除即回滚
