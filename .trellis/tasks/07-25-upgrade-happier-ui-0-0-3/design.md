# 技术设计：升级 happier-ui 0.0.3 + 手写控件替换

## 1. 边界与目标

- 升级依赖 0.0.2 → 0.0.3，CSS 入口名不变（已确认 `styles.css` + `tokens.css`）。
- 「能换都换」的落地原则：真实契合就换，缺功能则提 issue 并保留手写，不硬套。
- 不改后端 Rust；不改 overlay；不改现有交互语义与可见文案。

## 2. 依赖升级

| 项 | 改动 |
|----|------|
| `package.json` | `happier-ui`: `0.0.2` → `0.0.3` |
| `pnpm-lock.yaml` | `pnpm add happier-ui@0.0.3` 同步 |
| `src/main.ts` | 无改动（CSS 入口未变） |

## 3. 组件替换详案

### 3.1 HSelect（5 处）

| 位置 | 现状 | HSelect 映射 |
|------|------|--------------|
| LogsPage statusClass | 4 静态选项（all/2xx/4xx/5xx/error） | `options` 常量 + `v-model="statusClass"` |
| LogsPage pageSize | 3 静态选项（20/50/100）+ `@change` | 若与 HPagination 合并则删除此 select；否则 `options` 常量 + `v-model.number` |
| GroupsPage sortMode | 3 静态选项（local/external_intelligence/external_coding） | `options` 常量 + `v-model="sortMode"` |
| GroupsPage bulkProviderId | 动态 `providers` + value=0 占位 | 用 `placeholder="选择供应商"` + `clearable`（省略 value=0 空选项）；`v-model.number="bulkProviderId"` |
| GroupsPage 队列行内 provider_id | `:value` + `@change` 手动模式 | 改 `:model-value="item.provider_id"` + `@update:model-value="v => updateItemAt(index, { provider_id: Number(v) })"` |

**风险**：HSelect 的 placeholder + clearable 语义是「无值时显 placeholder / 清空回无值」，与项目当前用 value=0 显式空选项的模式不同；如无法优雅取代 value=0 保留原写法（保留 value=0 常规选项，不用 placeholder）。

### 3.2 HTextarea（1 处，ProvidersPage 粘贴框）

- 现状：`<textarea rows=4 spellcheck=false class="w-full ... font-mono text-xs">`。
- HTextarea 映射：`v-model="pasteText"` + `rows="4"` + `spellcheck="false"` + `placeholder`。
- **风险**：等宽字体（`font-mono`）与内部尺寸。HTextarea 有 `size: sm/md/lg` 但字体族由库控制。若无法保留等宽字体（对 JSON/curl 识别友好），提 issue「HTextarea 建议支持 monospace variant 或类透传」，本处降级为 HTextarea 但接受非等宽；不接受则保留手写并注明。

### 3.3 HBadge（2 处）

| 位置 | 颜色 → variant 映射 |
|------|--------------------|
| HomePage 代理状态 | running→success, error→danger, 其他→default |
| LogsPage 状态码色标 | 2xx→success, 4xx→warning, 5xx→danger, 其他→default |

- 契合度高，直接映射。文本进默认 slot。

### 3.4 HPagination（1 处，LogsPage）

- 现状：文本「筛选 N 条 · 库内 M 条 · 第 P/T 页」+ 两个 HButton「上一页/下一页」。
- HPagination 映射：`:current="page"` + `:total="total"` + `:page-size="pageSize"` + `show-total` + `@change="({current}) => goPage(current)"`。
- **合并 pageSize select**：HPagination 有 `show-size-changer` + `page-size-options`，可以合并。合并后删除 LogsPage 每页条数 HSelect。若合并导致布局塞不下或视觉杂乱，保守只换翻页部分，pageSize 保留独立 HSelect。
- 「筛选 N 条 · 库内 M 条」文本不属分页职责，保留独立 span 展示。

### 3.5 HTable（2 处，最高风险）

#### LogsPage（8 列）
- `columns`（不使用 `render`，用 `cell` slot 承接彩标 / 多行内容）：
  - time、group_name、provider_name、upstream_model、status_code（cell slot → HBadge）、use_time_ms、error（cell slot → 彩色 span）、failover（cell slot → 多行）
- `data="items"` + `rowKey="id"`。
- loading slot → 「加载中…」；empty slot → HEmpty。

#### ProvidersPage（4 列）
- `columns`：name、base_url、enabled、操作
- 操作列 cell slot → 两个 HButton（编辑 / 删除）。
- `data="items"` + `rowKey="id"`。

**风险**：`cell` slot 类型签名是 `{ column, row: Record<string, unknown>, index }`，`row` 是 unknown 需类型断言（`row as RequestLog` / `row as Provider`）。若 cell slot 承接不了原有交互（如 whitespace/break-words 精细控制），保留手写 table 并注明。独立回滚点。

### 3.6 HTag / 其他不启用

- **HTag**：项目无「可关闭标签」场景，本轮不启用。
- 沿用上轮不启用清单（HIconButton / HToast / HProgress / HCell / HCellGroup / HFloatingBubble / HRange）。

## 4. spec 更新

- `component-guidelines.md` 3.1：把组件面更新到 0.0.3，纳入 HSelect / HTable / HBadge / HTextarea / HPagination 为可映射组件。
- HTag 不启用理由入 spec；如有缺功能保留手写的位置，注明「等 happier-ui issue #N 补齐」。

## 5. 缺功能 → 提 issue 流程

替换过程中若发现缺功能：
1. 记录现象（哪个组件 + 缺什么 + 项目哪里用不到）。
2. 判断能否降级：能→降级替换 + spec 注明；不能→保留手写 + 打 TODO。
3. 用 `gh issue create --repo Happier-X/happier-ui`（沿用 v0.0.2→0.0.3 那 5 个 issue 的模板风格）提交。
4. Issue 链接回填到 spec 或代码注释。

## 6. 验证

- `pnpm lint`、`pnpm typecheck`、`pnpm test:unit`、`pnpm build`；`cargo build`。
- 手动（需 `pnpm tauri dev`）：LogsPage 筛选/翻页/表格；ProvidersPage 表格/粘贴框；GroupsPage 排序/批量/队列；HomePage 状态徽章。

## 7. 回滚

- 分文件改动，`git checkout <file>` 可回退。
- HTable 是独立回滚点：不佳则单独回退这两个页面的 table 部分，不影响其他替换。
- 5 处 select、textarea、badge、pagination 也可各自独立回退。
