# 组件规范

> Vue 3 管理台组件的编写约定。

## 基本模式

1. 使用 Vue 单文件组件与 `<script setup lang="ts">`。
2. Props 使用 `defineProps` 声明明确类型；事件使用 `defineEmits` 声明名称和参数。
3. 页面负责加载与提交，通用组件负责展示和用户交互；复杂领域操作下沉到 `src/api/tauri.ts` 或组合式函数。
3.1 **happier-ui（渐进，0.1.1）**：入口导入 `happier-ui/tokens.css` 与 `happier-ui/styles.css`（**CSS 入口为复数 `styles.css`**，旧名 `style.css` 已不存在）；peer 提供 `@lucide/vue`。**CSS 引入顺序**：`main.ts` 保持先 `import "./index.css"`（含 `@import "tailwindcss"`）再 `import "happier-ui/styles.css"`。历史原因：0.1.0 之前 `styles.css` 把组件样式裸包在 `@layer components{}` 且未声明 layer 顺序，若先加载会被注册成首个层，Tailwind preflight（base 层 `button` reset）反而覆盖 `.h-button` 等组件样式；**0.1.1 已在 `styles.css` 顶部自带 `@layer theme, base, components, utilities;` 声明（上游 [happier-ui#10](https://github.com/Happier-X/happier-ui/issues/10) 已修复），消费侧顺序敏感解除**，现有顺序无害，保持不动。`tokens.css` 纯 `:root` 变量，与 `styles.css` 相对顺序无关，统一放 index.css 之后。映射：按钮 → `HButton`；单行输入 → `HInput`；布尔 → `HSwitch`/`HCheckbox`；空列表 → `HEmpty`；**分区卡片 → `HCard`**（`variant="outlined"` + `padding`，标题进 `#header`；库卡无 box-shadow，靠 border+surface 表达层次）；**侧栏壳 → `HSidebar`**（`items` + `:model-value="route.path"` + `@update:model-value` 里 `router.push`，品牌区进 `#header`，`:show-collapse-toggle="false"`）；**下拉选择 → `HSelect`**（`:options="HSelectOption[]"` + `:model-value` + `@update:model-value`；string 联合类型需 `v as XxxType` 断言、数字选项需 `Number(v)`；动态列表保留 `value=0` 占位选项时不用 `placeholder`）；**状态徽章 → `HBadge`**（`variant` 映射：running/2xx→success、4xx→warning、error/5xx→danger、其余→default；文本进默认 slot）；**多行输入 → `HTextarea`**（`v-model` + `:rows` + `:spellcheck`）；**分页 → `HPagination`**（`:current`/`:total`/`:page-size` + `@change="({current})=>goPage(current)"`；「筛选 N 条」等统计文本不属分页职责，保留独立 span；每页条数选择保留独立 `HSelect`，不并入 `show-size-changer` 以免改布局）；**数据表格 → `HTable`**（`:columns="HTableColumn[]"` + `:data` + `row-key`；复杂/条件渲染统一走 `#cell="{ column, row }"` slot 按 `column.key` 分支，`row` 需 `row as Xxx` 断言；空态用 `empty-text` 或外层 `HEmpty` v-if/v-else；loading 用 `:loading`）。内层小卡与改造收益低的保留 Tailwind，不硬套。**因库缺功能保留手写/降级**：`HTextarea` 内部 `<textarea>` 无法接收等宽字体（`class` 落到外层 `div`，表单元素不继承 `font-family`），粘贴框 `font-mono` 暂降级，等 [happier-ui#8](https://github.com/Happier-X/happier-ui/issues/8) 补 monospace；`HTable` 的 `:data` 只接受 `Record<string, unknown>[]`，interface 无索引签名须 `as unknown as Record<string, unknown>[]` 双重断言，等 [happier-ui#9](https://github.com/Happier-X/happier-ui/issues/9) 泛型化后简化。**本轮不启用**：`HTag`（项目无「可关闭标签」场景，状态展示用 `HBadge` 更贴）、`HIconButton`（实测不契合：固定正方形+圆角、只有 `:active` 无 `:hover` 背景、深底 ghost 显蓝——做不出标题栏 OS 惯例贴边矩形与关闭 hover 变红，故 AppTitleBar 三窗控件、AppShell 更新提示钮、OverlayApp 打开钮均保留原生 `<button>`）、`HToast`（更新提示是持久横幅非自动关闭浮层；另见上游 issue #7）、`HProgress`（文本进度改进度条属新增非替换）、`HCell`/`HCellGroup`（设置页复合行非标准列表项）、`HFloatingBubble`（overlay 是 Tauri 独立窗口非 DOM 气泡）、`HRange`（无场景）。
3.2 **业务表单（TanStack Form）**：供应商对话框表单与分组表单页（`GroupFormPage`）用 `@tanstack/vue-form` 的 `useForm` + `form.Field` 管理字段与提交；控件仍用 `HInput`/`HCheckbox` 等，绑定 `field.state.value` + `field.handleChange`（或 `:model-value` + `@update:model-value`），**禁止**再用独立 `reactive` 作为提交字段真源。粘贴识别、拖拽排序、批量添加等通过 `form.setFieldValue` / 整体替换数组写回。保存走 `form.handleSubmit` / `onSubmit`；打开新建 `form.reset(defaults)`，打开编辑 `form.reset(entityFields)`；保存失败保留 values 与 `editing*Id`。日志筛选、设置页端口/偏好等非对话框表单可用 `ref`，不强制迁 Form。不强制 Zod。
4. 代理运行状态、Base URL 和最后错误必须使用清晰、可行动的中文文案。
5. 列表必须覆盖加载、空数据和错误状态。
6. 表单中的上游 Key 输入使用密码类型；不向用户展示完整上游 Key。
7. 应用无登录页，首屏直接进入主布局。
8. 分组队列「按模型能力排序」只可修改当前表单，不得自动保存；排序依据为 llm_benchmark 外部榜单（logic 综合榜「极限分数」），未命中模型稳定排后，用户仍可拖拽微调。合同见 [model-queue-sort.md](./model-queue-sort.md)。
9. **配置到 Pi**：入口在**分组页**列表行「配置到 Pi」；调用 `exportGroupToPiAgent(groupId)`；**无 Key UI / 无 Key 入参**；模型名=分组名，写入本机 `~/.pi/agent/models.json` 的单一 `providers.model-hub`（按 id upsert）。
10. 信息架构无「API 密钥 / 客户端 Key」页面与导航。
11. **上游访问**：禁止供应商页「测试连接」及任何自动/后台对用户上游的测活；**不**展示供应商熔断健康徽章，**不**调用 `listHealth`（已删除）；分组页「拉取模型」**仅**用户点击触发，不得在 `onMounted`/保存时自动拉取。**例外**：当分组绑定了供应商开启「自动同步」时，代理在后台每 24h 自动全量覆盖分组，该模式下分组模型列表变为只读，禁用拖拽、编辑和删除条目。合同见 backend [upstream-access.md](../backend/upstream-access.md)。
12. **故障转移**：分组队列始终按顺序故障转移，UI **无** `auto_failover` 开关；创建/更新分组 payload 不得再传该字段。
13. **首页「最近成功请求」**：展示全局最近一次成功日志的分组 / 供应商 / 上游模型 / 时间（日志态，非队列首选）；调用 `getLastSuccessRequest()`；空态「暂无成功请求」；与今日统计一并刷新，独立错误文案；不轮询、不按分组展开。成功语义见 backend [logging-guidelines.md](../backend/logging-guidelines.md)。
14. **页面职责**：首页只承载代理运行状态、Base URL、启停/刷新、请求统计与接入指引；端口修改、数据目录、应用更新和自动检查偏好统一放在设置页。
15. **启动更新检查**：应用壳层仅在挂载时读取一次 `check_update_on_startup`；发现新版本只展示可关闭提示和设置页入口，探测后立即关闭 `Update` 资源，不自动下载或安装。
16. **分组卡片内即时编辑**：分组页卡片支持拖拽排序 / 删成员即时保存（`update_group` 全量 `items` 替换）。子组件（`src/components/groups/GroupCard.vue`）用**乐观本地态**展示新顺序：`localItems` 初始化自 props、`watch(() => props.group.items)` 随服务端回写同步；操作时先改 `localItems` 再 `emit('persist-items', items)`，页面收到后组装完整 payload 并 `updateGroup`，成功后用服务端返回替换 `groups[idx]`，失败 `error + refresh()` 回滚。同一分组保存中通过 `cardSavingIds`（`Set<number>`）禁用冲突操作，避免全量替换写穿。绑定分组（`source_provider_id` 非空）卡片内不渲染拖拽/删除且拖拽事件守卫返回。删除分组用卡片内覆盖层二次确认，**禁止** `window.confirm`。
17. **分组双栏选模独立页（octopus 风格）**：新建/编辑复用 `src/pages/GroupFormPage.vue`（路由 `/groups/new`、`/groups/:id/edit`，不再使用 `AppDialog`）+ `@tanstack/vue-form`；左栏按供应商手风琴选模、右栏已选队列拖拽/删除/清空。**左栏模型加载合同**：`useProviderModelCache`（`src/composables/useProviderModelCache.ts`）按 `provider_id` 缓存，`ensure(id)` 仅 `ready` 时直接返回、否则拉取并防并发（inflight map）；**只在用户展开手风琴 / 点刷新 / 点「全部加入」时调用**，打开页面、`onMounted`、保存均不预拉（D4=L1，见 upstream-access）。去重 key 为 `` `${provider_id}\u0000${upstream_model.trim()}` ``；「全部加入」只追加未在队列中的模型。绑定态左栏禁用（含展开按钮）、右栏只读，保留「立即同步」。保存成功 / 取消均回 `/groups`；无 dirty 守卫，未保存直接丢弃；编辑页用 `listGroups` 按 id 定位，找不到展示错误 + 返回列表。

## 应用外壳布局（AppShell）

固定框架「上 titlebar + 下(左侧栏 + 右主区)」，滚动只发生在右主区内容容器。契约：

- 最外层锁视口高度并禁止自身滚动：`h-screen overflow-hidden` + `flex-col`（**不要**用 `min-h-screen`，它允许整体高度撑破视口，导致 body/最外层出现滚动条，把 titlebar、侧栏一起滚走）。
- 顶部 `AppTitleBar` 固定高度（`h-11 shrink-0`），不参与滚动。
- 下方横向区域 `flex min-h-0 flex-1 overflow-hidden`：左侧 `HSidebar` 固定，右侧 `main` 用 `flex min-w-0 flex-1 flex-col` 占满剩余宽度。
- 右主区内：更新提示条（如有）+ 页面标题 `header` 固定不滚，只有最下方的 `RouterView` 容器可纵向溢出滚动。

> **Warning**: flex 子项默认 `min-height: auto` / `min-width: auto`，内容超长时会撑高/撑宽父项，使祖先的 `overflow-auto` / `overflow-hidden` 失效——滚动条跑到外层而非目标容器。
>
> 修复：在 flex 链路每一层补 `min-h-0`（纵向）/ `min-w-0`（横向），把子项约束回可用空间内。**滚动容器自身也要带 `min-h-0`**：`RouterView` 外层必须写 `min-h-0 flex-1 overflow-auto`，只写 `flex-1 overflow-auto` 会漏——内容超长时 main 整体溢出、`overflow-auto` 不生效。

```vue
<!-- 正确：h-screen 锁死 + 每层 min-h-0/min-w-0，仅内容容器滚动 -->
<div class="flex h-screen flex-col overflow-hidden">
  <AppTitleBar />
  <div class="flex min-h-0 flex-1 overflow-hidden">
    <HSidebar ... />
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
    <!-- 2. HCard：flex-1 min-h-0 吃掉页高；class fallthrough 落到 <article class="h-card"> -->
    <HCard variant="outlined" padding="md" class="min-h-0 flex-1 flex flex-col">
      <template #header><!-- 标题 + 新建按钮（固定不滚） --></template>
      <HEmpty v-if="items.length === 0" ... />
      <template v-else>
        <!-- 3. 表格滚动区：min-h-0 flex-1 overflow-y-auto，仅此区滚动 -->
        <div class="min-h-0 flex-1 overflow-y-auto">
          <HTable :data="pagedItems" :sticky-header="true" ... />
        </div>
        <!-- 4. 分页器：在滚动区之外，shrink-0 不被压缩，mt-3 右对齐 -->
        <div v-if="items.length > pageSize" class="mt-3 flex justify-end shrink-0">
          <HPagination :current="page" :total="items.length" :page-size="pageSize"
            @change="({ current }) => goPage(current)" />
        </div>
      </template>
    </HCard>
  </div>
</template>

<style scoped>
/* 5. HCard 内部 .h-card__body slot 容器默认是普通 div（无 flex），
   必须用 :deep 让它 flex-1 + min-h-0 参与卡片 flex 列布局，否则 body 不撑高、表格滚动区塌缩为 0。 */
:deep(.h-card) { display: flex; flex-direction: column; }
:deep(.h-card__body) { flex: 1; min-height: 0; display: flex; flex-direction: column; }
</style>
```

### 关键点（坑）

- **`:sticky-header` 需滚动祖先**：`.h-table--sticky .h-table__th` 用 `position: sticky; top: 0`，
  sticky 相对最近滚动祖先定位。`.h-table-wrapper` 只有 `overflow-x: auto`（无纵向滚动），
  所以**必须在 wrapper 外再套一层 `overflow-y-auto` 容器**，sticky 表头才会粘住。
- **`.h-card__body` 默认不 flex**：HCard 库 CSS 中 `.h-card__header/.h-card__body/.h-card__footer` 仅
  `padding` + `border-top`，无 `display:flex`。直接在 HCard 上加 `flex flex-col` 只让 `<article>`
  变 flex 列，**body 容器自身仍是块级且无 `flex-1`，不会撑满剩余高度**——必须 `:deep(.h-card__body)`。
- **分页器必须在滚动区外**：若放进 `overflow-y-auto` 内，会随表格行一起滚动消失。用 `shrink-0` 防止被压缩。
- **前端假分页**：后端 `list_providers` 无分页参数，前端对全量 `items` 切片传 `HTable :data`，
  `total` 传 `items.length`（全量数），`page` 为页面级 ref，`refresh()` 后重置 `page=1`。
- **乐观更新改 `items` 非 `pagedItems`**：行内开关等局部更新操作真源是 `items`（全量），
  `pagedItems` 是 computed slice 自动反映；分页与乐观更新正交。
- **不破坏 AppShell 契约**：此模式不改动 AppShell `.overflow-auto .p-6` 容器，只在页面内用
  `h-full overflow-hidden` + flex 列把滚动从主区"接管"到表格内部；其他页面仍走整体滚动模式。

## 状态与生命周期

- 局部交互使用 `ref` / `computed`；**禁止使用 `reactive`，一律用 `ref`**（含 `shallowRef`）；**对话框业务表单字段**用 TanStack Form（见 3.2），不与页面级 `ref` 双源。
- 异步加载在 `onMounted` 中触发；定时器和事件订阅在 `onUnmounted` 中清理。
- 提交期间禁用重复操作，并在失败时保留用户可修正的输入（Form values 与编辑 id）。
- 编辑已有分组表单必须使用稳定的 `editingGroupId: number | null` 表达编辑目标；保存时先快照 id，id 非空只能调用更新，只有新建态才调用创建。添加条目、拉取模型、批量添加、排序等异步/局部操作不得清空编辑 id。
- 供应商新建/编辑复用 `AppDialog`（分组已迁独立页 `GroupFormPage`，见 §17）；页面以稳定实体 id 区分创建和更新。打开新建 Dialog 前 `form.reset` 默认值；保存失败保留 Dialog 与 Form 输入；保存成功后关闭并刷新列表；保存期间禁止重复提交和关闭（`closeDisabled`）。

## 对话框合同

- 通用外壳使用 `src/components/AppDialog.vue`（**内部**基于 `HDialog` 的薄封装），页面保留表单和领域保存逻辑，不引入页面专用遮罩实现。**当前仅供应商页使用**；分组新建/编辑已迁独立路由页（`GroupFormPage`，见 §17），不得回归对话框。
- 对外 props 保持：`open` / `title` / `size`（`default`|`wide`）/ `closeDisabled`、`@close`。
- 适配：`open` ↔ `modelValue`；`closeDisabled` 时 `closeOnOverlay`/`closeOnEsc` 为 false 并忽略关闭更新；`wide` 由 **Teleport 外层宿主** class（`app-dialog-host--wide`）约束宽度与内容滚动——勿把 class 直接挂在 `HDialog` 上（库根节点 class 写死为 `h-dialog`）。
- 必须 Teleport 到 `body`（避免主区 `overflow` 裁切）；提供关闭按钮；关闭后恢复焦点。焦点陷阱以 `HDialog` 行为为准。
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
