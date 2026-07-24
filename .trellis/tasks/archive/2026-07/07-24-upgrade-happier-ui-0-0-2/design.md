# 技术设计：升级 happier-ui 0.0.2 + 手写控件替换

## 1. 边界与目标

- 升级依赖 0.0.1 → 0.0.2，修复 CSS 入口破坏性改名。
- 「能替换到都替换」的落地原则：**只在契合度真实存在的位置替换**，不为凑数硬套语义不符的组件。
- 不改后端 Rust；不改 overlay 窗口构造与生命周期；不改任何现有交互语义与可见文案。

## 2. 破坏性升级（必做）

| 项 | 改动 |
|----|------|
| `package.json` | `happier-ui` → `0.0.2`（已 pnpm add 完成） |
| `src/main.ts` | `import "happier-ui/style.css"` → `import "happier-ui/styles.css"` |
| `tokens.css` | 入口名未变，保持 |

## 3. 组件替换清单（换）

### 3.1 HIconButton —— 实测不契合，本轮不启用（原计划 5 处，已放弃）

**实测结论（基于编译后 CSS，非 API 签名）**：HIconButton 与项目现存图标钮交互模型冲突，硬换会导致交互退化，归入「实测不契合」（同 overlay 的物理不可换等级）。

HIconButton 硬约束：
- 尺寸仅固定正方形三档（sm=32 / md=40 / lg=48）且带 12px 圆角（square）；
- **只有 `:active` 背景反馈，无 `:hover` 背景态**；
- `ghost` = primary 蓝色文字（为浅底设计）；`danger` = 常驻红底白字。

对照放弃理由：

| 目标 | 放弃理由 |
|------|----------|
| `AppTitleBar` 三钮 | OS 级窗口 chrome：需满高 36px×44px 贴边矩形命中区 + 无圆角 + 关闭「hover 才变红」；HIconButton 固定正方圆角铺不满，无 hover 态做不出 hover 变红，深色底 ghost 显蓝色。形状/配色/交互全冲突。 |
| `AppShell` 更新提示关闭钮 | cyan 主题小圆钮 + `hover:bg-cyan-100`；换后丢 hover 背景反馈、图标变蓝。 |
| `OverlayApp` 打开钮 | overlay 是纯 scoped CSS 自成体系深色浮层，混入库组件破坏一致性；且约束限定 overlay 内部尽量不动。 |

→ 结论：HIconButton 无真正契合落点，整体不启用。三处保留原生 `<button>` + Tailwind。

### 3.2 HCard —— 手写 `section` 卡片（各页面外层分区，约 12+ 处）

- 目标：`<section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">` → `<HCard variant="outlined" padding="md">`，内部 `<h2>` 标题移到 `#header` slot。
- 覆盖：HomePage(4)、GroupsPage(2 外层)、LogsPage(2)、ProvidersPage(2)、SettingsPage(3)。
- **视觉差异（接受）**：HCard「无 elevation / box-shadow」，替换后丢 `shadow-sm`，改由 border + surface 表达层次，符合库设计语言。
- 内层小卡（`rounded-lg border ...`）：契合处用 `HCard variant="flat"/"filled"`；与输入/说明混排且改造收益低的保留 Tailwind，逐处判断。

### 3.3 HSidebar —— AppShell 手写侧栏（1 处，路由联动改造）

- 当前：`<aside>` + 品牌区 + `RouterLink v-for` 手写高亮。
- 目标：`<HSidebar :items="navItems" :model-value="route.path" @update:model-value="key => router.push(key)">`，品牌区放 `#header` slot。
- `navItems: HSidebarItem[]`：`{ key: '/xxx', label }`；当前导航无图标，`icon` 省略。
- 选中态由 `route.path` 驱动（`model-value` 绑 `route.path`）；点击 emit key → `router.push`。
- **风险点（最高）**：HSidebar 自带宽度/配色可能与当前深色 `bg-slate-900` 侧栏不同；full-width 标题栏 + 侧栏的纵向布局衔接需实测。若 HSidebar 视觉/布局与壳层冲突严重，回退保留手写侧栏并在 spec 注明。

## 4. 不替换清单（说明理由，避免"漏换"误解）

| 组件 | 为何不换 |
|------|----------|
| `HIconButton` | 见 3.1：与现存图标钮交互模型冲突（无 hover 态、固定正方圆角、ghost 显蓝、danger 常驻红底），三处硬换均交互退化，无真正契合落点。 |
| `HToast` | 更新提示是**持久顶部横幅**（跟随主区、不自动消失、含「前往设置」链接）；HToast 是 `duration` 自动关闭的浮层通知，语义与交互不符，替换会改变 UX。 |
| `HProgress` | 当前下载进度仅文本百分比；改成进度条属**新增增强**而非替换，超出本轮"替换手写控件"范畴。 |
| `HCell` / `HCellGroup` | 设置页行是「输入 + 按钮 + 多行说明」复合结构，非标准 `title + prefix/suffix` 列表项；硬套会破坏现有布局与说明文案。 |
| `HFloatingBubble` | overlay 是 **Tauri 独立窗口**（Rust `WebviewWindowBuilder` 建），非 DOM 内悬浮气泡，物理不可替换。 |
| `HRange` | 项目无滑块/区间输入场景。 |
| `select` / `textarea` / 表格 | 库未提供对应组件；继续 Tailwind（沿用 spec 3.1）。 |

## 5. spec 更新

- `component-guidelines.md` 3.1：把组件面更新到 0.0.2，纳入 `HIconButton`（图标钮）、`HCard`（分区卡片）、`HSidebar`（侧栏壳）为可映射组件；删除/修订"侧栏壳、卡片分区继续 Tailwind""不扩展库组件面"的旧约束，改为本轮划定的新边界，并写明 select/textarea/表格仍 Tailwind、HToast/HProgress/HCell 本轮不启用的理由。

## 6. 验证

- `pnpm lint`、`pnpm typecheck`、`cargo build`。
- 手动（需 `pnpm tauri dev`）：样式无缺失（styles.css 生效）；标题栏三钮功能与图标切换正常；更新提示关闭钮正常；overlay 打开主窗口正常；侧栏导航高亮与跳转正常；各页面卡片视觉可接受。

## 7. 回滚

- 改动集中在前端 + package.json/lockfile + spec。分文件 `git checkout` 可回退。
- HSidebar 若实测不佳，单独回退该文件保留手写侧栏，不影响其余替换。
