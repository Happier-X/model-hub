# PRD: 修复 happier-ui 样式被 Tailwind preflight 覆盖（CSS layer 顺序）

## 问题描述

页面上 happier-ui 的组件（首先暴露在 `HButton`）样式**未生效**——按钮呈现浏览器/Tailwind preflight 的裸样式（无背景色、无内边距、无圆角），而非 happier-ui 设计的外观。

排查结论：**不是组件用法错误**，而是 `main.ts` 里的 CSS 引入顺序打乱了 CSS 层叠层（cascade layer）优先级。

### 根因

happier-ui 的 `dist/styles.css`：

- 前 ~392 行是 `:root` 里的 `--h-*` token 变量；
- 所有组件样式（含 `.h-button` 等 51 处）裸包在 `@layer components { ... }` 里；
- **包内不声明 layer 顺序**（没有 `@layer theme, base, components, utilities;` 这类前置声明）。

当前 `main.ts` 引入顺序：

```js
import "happier-ui/tokens.css";
import "happier-ui/styles.css";   // 先加载 → 首次注册 @layer components
import "./index.css";              // 后加载 → 内部才 @import "tailwindcss"
```

CSS 层叠层顺序由**首次出现顺序**决定，于是实际注册顺序为：

1. happier `styles.css` 首次遇到 `components` 层 → 排在最前；
2. `index.css` 的 `@import "tailwindcss"` 展开 `@layer theme, base, components, utilities;` → `theme`/`base`/`utilities` 作为新层追加到 `components` **之后**。

最终优先级（低→高）：`components(happier) < theme < base < utilities`。

Tailwind 的 preflight reset 位于 `base` 层，现在排在 `components` **之后**，优先级更高，把 `.h-button` 的 `background` / `border` / `padding` 等全部覆盖 → 按钮"没样式"。

## 方案

调整 `src/main.ts` 引入顺序：让含 `@import "tailwindcss"` 的 `./index.css` **先于** happier-ui 的 `styles.css` 加载，使 Tailwind 首先声明 `theme, base, components, utilities` 的正确层顺序：

```js
import "./index.css";              // 先声明 layer 顺序（theme, base, components, utilities）
import "happier-ui/tokens.css";
import "happier-ui/styles.css";    // .h-button 正确归入已声明的 components 层
```

恢复后优先级为 `theme < base < components < utilities`，preflight（base）在 components 之前，不再覆盖组件样式。

## 影响范围

- 主要：`src/main.ts`（仅 import 顺序，无逻辑改动）
- 文档：`.trellis/spec/frontend/component-guidelines.md`（记录该 layer-order 坑与正确引入顺序）
- **不改**：任何组件调用、overlay 引入分支、happier-ui 版本、`index.css` 内容

## 约束

1. 不改 overlay 挂载分支逻辑（`isOverlay` 判定与 class 注入保持不变）。
2. 不改 happier-ui 版本，不改 `index.css` / `tokens.css` / `styles.css` 内容。
3. 不为个别组件写覆盖样式绕过问题——从 layer 顺序根治。
4. 保持既有前端质量门禁（typecheck / lint / test:unit / build）全绿。

## 验收标准

- [ ] `src/main.ts` 中 `./index.css` 先于 `happier-ui/styles.css` 引入。
- [ ] 构建产物中，Tailwind `base`（preflight）层优先级低于 happier-ui 组件所在的 `components` 层（按钮等组件样式正常呈现）。
- [ ] 主应用页面 `HButton` 各 variant（primary/secondary/outline/ghost/danger 等）恢复设计样式；overlay 窗口不受影响。
- [ ] `pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿。
- [ ] `.trellis/spec/frontend/component-guidelines.md` 记录：happier-ui 样式必须在 Tailwind layer 声明之后引入，附根因与正确顺序。

## 非目标

- 不升级或改动 happier-ui 包本身（layer 顺序声明缺失属库侧问题，可另行提 issue）。
- 不重构 `index.css`。
- 不改后端 / overlay 生命周期。

## Notes

- 可考虑给 happier-ui 提 issue：`styles.css` 应在文件顶部声明 `@layer theme, base, components, utilities;`（或至少 `@layer components;`）以避免消费方引入顺序敏感。本任务先在消费侧修复。
- 现场证据：`main.ts` 现顺序为 tokens → styles → index.css；`index.css` 首行 `@import "tailwindcss";`；happier `styles.css` 第一个 `@layer components {` 在第 393 行，之前无 layer 顺序声明。
