# 组件规范

> Vue 3 管理台组件的编写约定。

## 基本模式

1. 使用 Vue 单文件组件与 `<script setup lang="ts">`。
2. Props 使用 `defineProps` 声明明确类型；事件使用 `defineEmits` 声明名称和参数。
3. 页面负责加载与提交，通用组件负责展示和用户交互；复杂领域操作下沉到 `src/api/tauri.ts` 或组合式函数。
3.1 **shadcn-vue（reka-ui 底层，源码入仓）**：组件源码位于 `src/components/ui/*`（`shadcn-vue add` 拷入，`components.json` 管理别名）；`cn` 工具在 `src/lib/utils.ts`（clsx + tailwind-merge）。**依赖**：`reka-ui`、`clsx`、`tailwind-merge`、`class-variance-authority`、`tw-animate-css`（index.css 已 `@import`）、图标 `@lucide/vue`。样式变量（oklch `--background` 等）在 `src/index.css`，风格 `reka-nova`、baseColor `neutral`。**不再使用 happier-ui**（已移除依赖与 `tokens.css`/`styles.css` 导入）。映射：按钮 → `Button`（variant：default/outline/secondary/ghost/destructive/link；尺寸含 `size="icon"`；旧 `isIconOnly`/`shape="circle"` 无此概念，图标按钮用 `size="icon"`）；单行输入 → `Input`（无 `label` prop，用 `<label>` 包裹 + `span` 文本，或 `:model-value` + `@update:model-value`）；布尔 → `Switch`/`Checkbox`（均 v-model，`label` 文本由外层 `<label class="flex items-center gap-2">` 承载）；空列表 → `Empty`；**分区卡片 → `Card`**（`CardHeader`/`CardContent` 子组件，`class` 直接透传可写 `flex flex-col min-h-0 flex-1`，不再需要 `.h-card__body` 深选择器 hack）；**侧栏壳 → `Sidebar` 全家桶**（`SidebarProvider > Sidebar collapsible="none" > SidebarHeader + SidebarContent + SidebarMenu + SidebarMenuItem + SidebarMenuButton(as-child → RouterLink)`，`:is-active` 高亮当前项；**必须 `collapsible="none"`**——`icon` 折叠态纯文字导航（无图标）会整体消失，`offcanvas` 则 <768px 时隐藏且需 SidebarTrigger 才能唤出；尺寸变量 `--sidebar-width: 16rem` 等必须在 index.css 的 `:root` 定义（shadcn-vue init 若中断不会写入，缺失时 `w-(--sidebar-width)` 塌缩为 0，导航全部消失——见下方代码示例）；`none` 模式是普通文档流 div（无 fixed/Sheet），后续 `main` 自然占位，无需 `SidebarInset` padding 补偿）；**下拉选择 → `Select` 全家桶**（`Select > SelectTrigger > SelectValue + SelectContent > SelectItem`；`:model-value` + `@update:model-value`；string 联合类型需 `v as XxxType` 断言、数字选项需 `String(v)`/`Number(v)` 转换——reka-ui SelectItem 的 value 是 string）；**状态徽章 → `Badge`**（variant：default/secondary/destructive/outline/ghost/link；旧 success/warning/danger 映射：success→secondary（或 outline+emerald class）、warning→outline、danger→destructive）；**多行输入 → `Textarea`**（`v-model` + `:rows`，class 透传到 `<textarea>` 本体，`font-mono` 可直接加）；**分页 → `Pagination` 全家桶**（reka-ui 分页：`Pagination(:page :total :page-size @update:page) > PaginationContent v-slot="{ items }" > PaginationFirst/Previous + v-for PaginationItem(点击传 `item.value`) + Ellipsis + Next/Last`；「筛选 N 条」等统计文本不属分页职责，保留独立 span）；**数据表格 → `Table` 结构**（`Table > TableHeader(TableRow>TableHead v-for) + TableBody(TableRow v-for > TableCell v-for)`；`columns` 数组 `{ key, title }[]` 驱动表头与单元格 v-for，复杂/条件渲染按 `column.key` 分支，`row` 已是泛型对象无需断言（`(row as Record<string, unknown>)[col.key]`）；空态用 `<TableRow v-if="items.length===0"><TableCell :colspan>` 或外层 `Empty`；loading 用文本/`Spinner`）。**额外组件**：`Spinner`（替代旧 `HLoading`）、`Item` 全家桶（`Item + ItemContent + ItemTitle + ItemDescription`，替代旧 `HCell`，供应商手风琴行用：Item 默认 slot 放展开箭头、ItemTitle 放名称、ItemDescription 放自动同步开关行）、`Badge` 兼作状态标签（替代旧 `HTag`，用 `variant="outline"` + 自定义 class 表达成功/默认）、`Dialog`（见对话框合同）、`Tooltip`、`Progress`（替代旧 HProgress：`:model-value` + `:max` + `:indeterminate` + class 调高度）。热力图用 **vue3-calendar-heatmap**（`CalendarHeatmap`，props `:values="{ date:'YYYY-MM-DD', count }[]"`、`:end-date="new Date(...)"`、`:range-color="string[]"`、`:tooltip="false"` 可关提示）；首页 `HHeatmapData {timestamp,value}` 需 computed 转为 `{date,count}`。**因 shadcn 无 `label` 概念，所有带 label 的控件统一 `<label class="block text-sm"><span class="mb-1 block text-slate-600">…</span><Input/></label>` 模式**。
3.2 **业务表单（TanStack Form）**：供应商对话框表单与分组表单页（`GroupFormPage`）用 `@tanstack/vue-form` 的 `useForm` + `form.Field` 管理字段与提交；控件用 `Input`/`Checkbox`/`Select` 等，绑定 `field.state.value` + `field.handleChange`（或 `:model-value` + `@update:model-value`），**禁止**再用独立 `reactive` 作为提交字段真源。粘贴识别、拖拽排序、批量添加等通过 `form.setFieldValue` / 整体替换数组写回。保存走 `form.handleSubmit` / `onSubmit`；打开新建 `form.reset(defaults)`，打开编辑 `form.reset(entityFields)`；保存失败保留 values 与 `editing*Id`。日志筛选、设置页端口/偏好等非对话框表单可用 `ref`，不强制迁 Form。不强制 Zod。
4. 代理运行状态、Base URL 和最后错误必须使用清晰、可行动的中文文案。
5. 列表必须覆盖加载、空数据和错误状态。
6. 表单中的上游 Key 输入使用密码类型；不向用户展示完整上游 Key。
7. 应用无登录页，首屏直接进入主布局。
8. 分组队列「按模型能力排序」只可修改当前表单，不得自动保存；排序依据为 llm_benchmark 外部榜单（logic 综合榜「极限分数」），未命中模型稳定排后，用户仍可拖拽微调。合同见 [model-queue-sort.md](./model-queue-sort.md)。
9. **配置到 Pi**：入口在**分组页**列表行「配置到 Pi」；调用 `exportGroupToPiAgent(groupId)`；**无 Key UI / 无 Key 入参**；模型名=分组名，写入本机 `~/.pi/agent/models.json` 的单一 `providers.model-hub`（按 id upsert）。
10. 信息架构无「API 密钥 / 客户端 Key」页面与导航。
11. **上游访问**：禁止供应商页「测试连接」及任何自动/后台对用户上游的测活；**不**展示供应商熔断健康徽章，**不**调用 `listHealth`（已删除）；分组页「拉取模型」**仅**用户点击触发，不得在 `onMounted`/保存时自动拉取。**例外**：供应商开启了「自动同步」时，代理在后台每 24h 自动拉取其 `/models` 并全量覆盖本地 `provider_models` 表（`sync_provider_now` 可手动触发，不受 24h 限制）；分组页左侧展开供应商**优先读本地持久化** `provider_models`（不发网络请求），仅本地无数据时才实时拉取一次兑底。合同见 backend [upstream-access.md](../backend/upstream-access.md)。
12. **故障转移**：分组队列始终按顺序故障转移，UI **无** `auto_failover` 开关；创建/更新分组 payload 不得再传该字段。
13. **首页「最近成功请求」**：展示全局最近一次成功日志的分组 / 供应商 / 上游模型 / 时间（日志态，非队列首选）；调用 `getLastSuccessRequest()`；空态「暂无成功请求」；与今日统计一并刷新，独立错误文案；不轮询、不按分组展开。成功语义见 backend [logging-guidelines.md](../backend/logging-guidelines.md)。
14. **页面职责**：首页只承载代理运行状态、Base URL、启停/刷新、请求统计与接入指引；端口修改、数据目录、应用更新和自动检查偏好统一放在设置页。
15. **启动更新检查**：应用壳层仅在挂载时读取一次 `check_update_on_startup`；发现新版本只展示可关闭提示和设置页入口，探测后立即关闭 `Update` 资源，不自动下载或安装。
16. **分组卡片内即时编辑**：分组页卡片支持拖拽排序 / 删成员即时保存（`update_group` 全量 `items` 替换）。子组件（`src/components/groups/GroupCard.vue`）用**乐观本地态**展示新顺序：`localItems` 初始化自 props、`watch(() => props.group.items)` 随服务端回写同步；操作时先改 `localItems` 再 `emit('persist-items', items)`，页面收到后组装完整 payload 并 `updateGroup`，成功后用服务端返回替换 `groups[idx]`，失败 `error + refresh()` 回滚。同一分组保存中通过 `cardSavingIds`（`Set<number>`）禁用冲突操作，避免全量替换写穿。分组队列纯手动维护（无绑定态只读限制）。删除分组用卡片内覆盖层二次确认，**禁止** `window.confirm`。
17. **分组双栏选模独立页（octopus 风格）**：新建/编辑复用 `src/pages/GroupFormPage.vue`（路由 `/groups/new`、`/groups/:id/edit`，不再使用 `AppDialog`）+ `@tanstack/vue-form`；左栏按供应商手风琴选模、右栏已选队列拖拽/删除/清空。**左栏模型加载合同**：`useProviderModelCache`（`src/composables/useProviderModelCache.ts`）按 `provider_id` 缓存，`ensure(id)` **先读本地持久化** `getProviderModels`（`provider_models` 表），非空即 ready（离线可用，不发网络请求），空则实时拉取一次兑底并防并发（inflight map）；`refresh(id)` 强制实时拉取。**只在用户展开手风琴 / 点刷新 / 点「全部加入」时调用**，打开页面、`onMounted`、保存均不预拉（D4=L1，见 upstream-access）。去重 key 为 `` `${provider_id}\u0000${upstream_model.trim()}` ``；「全部加入」只追加未在队列中的模型。队列始终可交互（无绑定态只读）。**双栏 Card 滚动链（整页占满版）**：页面根 `flex h-full min-h-0 flex-col gap-4 overflow-hidden`（自身不滚动），表单 `flex min-h-0 flex-1 flex-col`，双栏容器必须用 **flex 而非 grid**——`grid item` 上 `flex-1`（flex 属性）不生效，卡片高度回退内容高会撑高整页产生滚动条；正确写法 `flex min-h-0 flex-1 flex-col gap-4 lg:flex-row`（响应式等价 grid-cols）。shadcn `Card` 根自带 `flex flex-col`，`CardContent` 直接加 `min-h-0 flex-1 flex flex-col p-0` 接出内部滚动区（`min-h-0 flex-1 overflow-y-auto p-3`）即可，**无需任何深选择器 hack**（历史 `.h-card__body` 约束已随 happier-ui 移除而消亡）。保存成功 / 取消均回 `/groups`；无 dirty 守卫，未保存直接丢弃；编辑页用 `listGroups` 按 id 定位，找不到展示错误 + 返回列表。

## 应用外壳布局（AppShell）

固定框架「上 titlebar + 下(左侧栏 + 右主区)」，滚动只发生在右主区内容容器。契约：

- 最外层锁视口高度并禁止自身滚动：`h-screen overflow-hidden` + `flex-col`（**不要**用 `min-h-screen`，它允许整体高度撑破视口，导致 body/最外层出现滚动条，把 titlebar、侧栏一起滚走）。
- 顶部 `AppTitleBar` 固定高度（`h-11 shrink-0`），不参与滚动。
- 下方横向区域 `flex min-h-0 flex-1 overflow-hidden`：左侧 `SidebarProvider > Sidebar` 固定，右侧 `main` 用 `flex min-w-0 flex-1 flex-col` 占满剩余宽度。
- 右主区内：更新提示条（如有）+ 页面标题 `header` 固定不滚，只有最下方的 `RouterView` 容器可纵向溢出滚动。

> **Warning**: flex 子项默认 `min-height: auto` / `min-width: auto`，内容超长时会撑高/撑宽父项，使祖先的 `overflow-auto` / `overflow-hidden` 失效——滚动条跑到外层而非目标容器。
>
> 修复：在 flex 链路每一层补 `min-h-0`（纵向）/ `min-w-0`（横向），把子项约束回可用空间内。**滚动容器自身也要带 `min-h-0`**：`RouterView` 外层必须写 `min-h-0 flex-1 overflow-auto`，只写 `flex-1 overflow-auto` 会漏——内容超长时 main 整体溢出、`overflow-auto` 不生效。

```vue
<!-- 正确：h-screen 锁死 + 每层 min-h-0/min-w-0，仅内容容器滚动 -->
<div class="flex h-screen flex-col overflow-hidden">
  <AppTitleBar />
  <div class="flex min-h-0 flex-1 overflow-hidden">
    <SidebarProvider>
      <Sidebar collapsible="none" class="border-r border-slate-200 bg-white">
        <SidebarHeader><!-- 品牌区 --></SidebarHeader>
        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupLabel>导航</SidebarGroupLabel>
            <SidebarMenu>
              <SidebarMenuItem v-for="item in navItems" :key="item.key">
                <SidebarMenuButton as-child :is-active="activeNavKey === item.key">
                  <RouterLink :to="item.key">{{ item.label }}</RouterLink>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroup>
        </SidebarContent>
      </Sidebar>
    </SidebarProvider>
    <main class="flex min-w-0 flex-1 flex-col">
      <!-- 更新提示条 / header：固定不滚 -->
      <div class="min-h-0 flex-1 overflow-auto p-6"><RouterView /></div>
    </main>
  </div>
</div>
```

## 页面内部表格滚动模式（AppShell 整体滚动的对偶）

默认 AppShell 滚动发生在右主区 `<div class="min-h-0 flex-1 overflow-auto p-6">` 包裹的 RouterView 容器
（页面整体滚动）。**例外场景**：当某页要求"表格恰好占满页高、仅表格 body 滚动、底部分页器不滚"
时（供应商页 `ProvidersPage`），该页走"表格内部滚动"模式，与整体滚动模式互斥。

### 适用判定

- 页面内容仅一个表格 + 分页器（无长表单 / 无多分区卡片堆叠）。
- 用户预期"表头钉住、行区域滚动、翻页紧贴表底"，而非整页卷动。
- 供应商页（数十条数据）从全量加载切换到前端假分页时使用此模式。

### 实现合同（供应商页模式）

```vue
<template>
  <!-- 1. 页面根节点：h-full 撑满 + overflow-hidden 防整页滚动 -->
  <div class="h-full flex flex-col overflow-hidden">
    <!-- 2. Card：flex-1 min-h-0 吃掉页高；class 透传落到 Card 根（自身即 flex flex-col） -->
    <Card class="min-h-0 flex-1 flex flex-col border border-slate-200 bg-white">
      <CardHeader class="shrink-0 py-3"><!-- 标题 + 新建按钮（固定不滚） --></CardHeader>
      <CardContent class="flex min-h-0 flex-1 flex-col gap-3">
        <Empty v-if="items.length === 0" ... />
        <template v-else>
          <!-- 3. 表格滚动区：min-h-0 flex-1 overflow-y-auto，仅此区滚动 -->
          <div class="min-h-0 flex-1 overflow-y-auto">
            <Table>
              <TableHeader>…</TableHeader>
              <TableBody>…</TableBody>
            </Table>
          </div>
          <!-- 4. 分页器：在滚动区之外，shrink-0 不被压缩，右对齐 -->
          <div v-if="items.length > pageSize" class="flex shrink-0 justify-end">
            <Pagination …/>
          </div>
        </template>
      </CardContent>
    </Card>
  </div>
</template>
```

### 关键点（坑）

- **shadcn Card 自身即 flex 列**：`Card` 根带 `flex flex-col`，`CardContent` 直接可加 `min-h-0 flex-1 flex flex-col`——不再需要 `.h-card__body` 深选择器 hack，高度链与双栏模式见 §17。
- **分页器必须在滚动区外**：若放进 `overflow-y-auto` 内，会随表格行一起滚动消失。用 `shrink-0` 防止被压缩。
- **前端假分页**：后端 `list_providers` 无分页参数，前端对全量 `items` 切片传表格 `v-for`，`total` 传 `items.length`（全量数），`page` 为页面级 ref，`refresh()` 后重置 `page=1`。
- **乐观更新改 `items` 非 `pagedItems`**：行内开关等局部更新操作真源是 `items`（全量），`pagedItems` 是 computed slice 自动反映；分页与乐观更新正交。
- **不破坏 AppShell 契约**：此模式不改动 AppShell `.overflow-auto .p-6` 容器，只在页面内用 `h-full overflow-hidden` + flex 列把滚动从主区"接管"到表格内部；其他页面仍走整体滚动模式。

## 状态与生命周期

- 局部交互使用 `ref` / `computed`；**禁止使用 `reactive`，一律用 `ref`**（含 `shallowRef`）；**对话框业务表单字段**用 TanStack Form（见 3.2），不与页面级 `ref` 双源。
- 异步加载在 `onMounted` 中触发；定时器和事件订阅在 `onUnmounted` 中清理。
- 提交期间禁用重复操作，并在失败时保留用户可修正的输入（Form values 与编辑 id）。
- 编辑已有分组表单必须使用稳定的 `editingGroupId: number | null` 表达编辑目标；保存时先快照 id，id 非空只能调用更新，只有新建态才调用创建。添加条目、拉取模型、批量添加、排序等异步/局部操作不得清空编辑 id。
- 供应商新建/编辑复用 `AppDialog`（分组已迁独立页 `GroupFormPage`，见 §17）；页面以稳定实体 id 区分创建和更新。打开新建 Dialog 前 `form.reset` 默认值；保存失败保留 Dialog 与 Form 输入；保存成功后关闭并刷新列表；保存期间禁止重复提交和关闭（`closeDisabled`）。

## 对话框合同

- 通用外壳使用 `src/components/AppDialog.vue`（**内部**基于 shadcn `Dialog` 的薄封装），页面保留表单和领域保存逻辑，不引入页面专用遮罩实现。**当前仅供应商页使用**；分组新建/编辑已迁独立路由页（`GroupFormPage`，见 §17），不得回归对话框。
- 对外 props 保持：`open` / `title` / `size`（`default`|`wide`）/ `closeDisabled`、`@close`。
- 适配：`open` ↔ `Dialog v-model:open`；`closeDisabled` 时 `DialogContent` 的 `close-on-esc`/`close-on-overlay` 为 false 并忽略关闭更新；宽度由 `DialogContent` 的 class 控制（`max-w-lg` / `max-w-3xl`），关闭按钮用 `DialogClose` + `Button`（`@click` 里按 `closeDisabled` 守卫 `emit("close")`）；标题区用 `DialogHeader > DialogTitle`。
- 必须 Teleport 到 `body`（`Dialog` 内部 `DialogPortal` 已默认 teleport 到 body，避免主区 `overflow` 裁切）；关闭后恢复焦点（`AppDialog` 用 watch 保存/恢复 `document.activeElement`）。焦点陷阱以 reka-ui `Dialog` 行为为准。
- 对话框打开不得隐式触发上游请求；分组拉模型仍只允许用户点击。

```vue
<AppDialog
  :open="dialogOpen"
  :title="editingId === null ? '新建' : '编辑'"
  :close-disabled="saving"
  @close="closeDialog"
>
  <!-- 页面拥有表单与保存逻辑 -->
</AppDialog>
```

## 文案

- 用户可见文案使用简体中文。
- 错误提示应说明下一步，例如检查端口、供应商 URL、Key 或模型映射。
- 不向用户展示完整上游 Key 或消息正文。
- **关窗 vs 退出**：首页/托盘等须写明——关闭窗口 = 隐藏到托盘、代理继续；仅托盘「退出」停止代理并释放端口；自动改口可提示意外多开时托盘退出旧实例。

## 禁止模式

- 使用 Options API 与 Composition API 混写同一组件。
- 在多个页面重复手写 Tauri 命令名和返回类型。
- 为未规划的多用户功能预埋权限组件。
- 用空数组或空对象吞掉加载失败。
- 对用户配置的上游做「测试连接」、定时探测或打开页面自动拉 `/models`。

## 统计卡片（StatsCards）

- 首页顶部统计用 `StatsCards.vue`（octopus 风格）：4 卡片网格（`lg:grid-cols-4`，窄屏降级 1 列），每卡左区竖排标题（`[writing-mode:vertical-lr]`）+ 头部图标，右区 2 个指标项（`w-10 h-10 rounded-xl` 图标块 + 标签 + 数值）。
- 数据自 `get_request_overview`（total/today 两组，成功口径），**只展示总计**；首页 `HomePage` 用 `setInterval` 每 5s 轮询 `get_request_overview` 实时刷新（组件卸载 `clearInterval`），统计卡片无刷新按钮、无今日 tab。
- 指标图标块统一用 `bg-primary/10 text-primary`（四卡一致）；图标用 `@lucide/vue`。
- 数值格式化对齐 octopus：`formatOctopus.ts` 提供 `formatCount`/`formatMoney`/`formatTime`，返回 `{ value, unit }` 分离（B/M/K、$、d/h/m/s/ms 单位，全部 toFixed(2)），unit 由调用方单独渲染。
- 数值动画：纯渲染函数组件 `AnimatedNumber.ts`（`.ts` + `defineComponent` + `h`，rAF 600ms easeOutCubic），入参为已格式化 value 字符串，按是否含小数点显示 0/2 位小数。动画属增强，数值始终由 props 决定。
- 数值格式工具在 `src/utils/formatTokenCount.ts`（`formatNumber` / `formatTokenCount`）+ `formatDuration.ts`，均可独立单测。

## 模型单价（设置页）

- 设置页「模型单价」卡片为**只读**：OpenRouter 自动同步（后台 24h 到期检查 + 启动静默 5 分钟，复用 provider auto_sync 模式），「立即同步」按钮手动触发（`sync_pricing_now`）；无手动编辑价格表单。
- 展示：同步状态行（模型数 + 最后同步时间）+ 搜索过滤 + shadcn Table（模型名/输入价/输出价，`$`/百万 token）；空态提示尚未同步。
- 费用计算在统计时（`request_overview` LEFT JOIN `model_pricing`，别名匹配 `xxx/model` ↔ `model`）；首页 StatsCards 三项费用用 `formatCost`（`$0` / `$x.xxxx` 去尾 0）。
