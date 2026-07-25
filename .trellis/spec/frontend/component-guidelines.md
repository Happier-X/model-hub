# 组件规范

> Vue 3 管理台组件的编写约定。

## 基本模式

1. 使用 Vue 单文件组件与 `<script setup lang="ts">`。
2. Props 使用 `defineProps` 声明明确类型；事件使用 `defineEmits` 声明名称和参数。
3. 页面负责加载与提交，通用组件负责展示和用户交互；复杂领域操作下沉到 `src/api/tauri.ts` 或组合式函数。
3.1 **happier-ui（渐进，0.0.3）**：入口导入 `happier-ui/tokens.css` 与 `happier-ui/styles.css`（**CSS 入口为复数 `styles.css`**，旧名 `style.css` 已不存在；0.0.3 未再改 CSS 入口名）；peer 提供 `@lucide/vue`。**CSS 引入顺序（强制）**：`main.ts` 必须先 `import "./index.css"`（含 `@import "tailwindcss"`）再 `import "happier-ui/styles.css"`。原因：happier-ui 的 `styles.css` 把组件样式裸包在 `@layer components{}` 且**未在文件顶部声明 layer 顺序**；若它先加载，`components` 会被注册成首个层，随后 Tailwind 展开的 `theme/base/utilities` 追加到其后，导致最终顺序 `components < base`，preflight（base 层的 `button` reset：清背景/边框/内边距）反而覆盖 `.h-button` 等组件样式，表现为「按钮没样式」。让 Tailwind 先声明 `@layer theme, base, components, utilities` 顺序后，happier-ui 的 `.h-button` 正确归入已声明的 `components` 层（`base < components`），preflight 不再覆盖。`tokens.css` 纯 `:root` 变量，与 `styles.css` 相对顺序无关，但统一放 index.css 之后。已提上游 [happier-ui#10](https://github.com/Happier-X/happier-ui/issues/10) 建议库在 `styles.css` 顶部声明 `@layer theme, base, components, utilities;` 以消除消费方引入顺序敏感；在库侧修复前，消费侧顺序约束长期有效。映射：按钮 → `HButton`；单行输入 → `HInput`；布尔 → `HSwitch`/`HCheckbox`；空列表 → `HEmpty`；**分区卡片 → `HCard`**（`variant="outlined"` + `padding`，标题进 `#header`；库卡无 box-shadow，靠 border+surface 表达层次）；**侧栏壳 → `HSidebar`**（`items` + `:model-value="route.path"` + `@update:model-value` 里 `router.push`，品牌区进 `#header`，`:show-collapse-toggle="false"`）；**下拉选择 → `HSelect`**（`:options="HSelectOption[]"` + `:model-value` + `@update:model-value`；string 联合类型需 `v as XxxType` 断言、数字选项需 `Number(v)`；动态列表保留 `value=0` 占位选项时不用 `placeholder`）；**状态徽章 → `HBadge`**（`variant` 映射：running/2xx→success、4xx→warning、error/5xx→danger、其余→default；文本进默认 slot）；**多行输入 → `HTextarea`**（`v-model` + `:rows` + `:spellcheck`）；**分页 → `HPagination`**（`:current`/`:total`/`:page-size` + `@change="({current})=>goPage(current)"`；「筛选 N 条」等统计文本不属分页职责，保留独立 span；每页条数选择保留独立 `HSelect`，不并入 `show-size-changer` 以免改布局）；**数据表格 → `HTable`**（`:columns="HTableColumn[]"` + `:data` + `row-key`；复杂/条件渲染统一走 `#cell="{ column, row }"` slot 按 `column.key` 分支，`row` 需 `row as Xxx` 断言；空态用 `empty-text` 或外层 `HEmpty` v-if/v-else；loading 用 `:loading`）。内层小卡与改造收益低的保留 Tailwind，不硬套。**因库缺功能保留手写/降级**：`HTextarea` 内部 `<textarea>` 无法接收等宽字体（`class` 落到外层 `div`，表单元素不继承 `font-family`），粘贴框 `font-mono` 暂降级，等 [happier-ui#8](https://github.com/Happier-X/happier-ui/issues/8) 补 monospace；`HTable` 的 `:data` 只接受 `Record<string, unknown>[]`，interface 无索引签名须 `as unknown as Record<string, unknown>[]` 双重断言，等 [happier-ui#9](https://github.com/Happier-X/happier-ui/issues/9) 泛型化后简化。**本轮不启用**：`HTag`（项目无「可关闭标签」场景，状态展示用 `HBadge` 更贴）、`HIconButton`（实测不契合：固定正方形+圆角、只有 `:active` 无 `:hover` 背景、深底 ghost 显蓝——做不出标题栏 OS 惯例贴边矩形与关闭 hover 变红，故 AppTitleBar 三窗控件、AppShell 更新提示钮、OverlayApp 打开钮均保留原生 `<button>`）、`HToast`（更新提示是持久横幅非自动关闭浮层；另见上游 issue #7）、`HProgress`（文本进度改进度条属新增非替换）、`HCell`/`HCellGroup`（设置页复合行非标准列表项）、`HFloatingBubble`（overlay 是 Tauri 独立窗口非 DOM 气泡）、`HRange`（无场景）。
3.2 **业务对话框表单（TanStack Form）**：供应商/分组等对话框表单用 `@tanstack/vue-form` 的 `useForm` + `form.Field` 管理字段与提交；控件仍用 `HInput`/`HCheckbox` 等，绑定 `field.state.value` + `field.handleChange`（或 `:model-value` + `@update:model-value`），**禁止**再用独立 `reactive` 作为提交字段真源。粘贴识别、拖拽排序、批量添加等通过 `form.setFieldValue` / 整体替换数组写回。保存走 `form.handleSubmit` / `onSubmit`；打开新建 `form.reset(defaults)`，打开编辑 `form.reset(entityFields)`；保存失败保留 values 与 `editing*Id`。日志筛选、设置页端口/偏好等非对话框表单可用 `ref`，不强制迁 Form。不强制 Zod。
4. 代理运行状态、Base URL 和最后错误必须使用清晰、可行动的中文文案。
5. 列表必须覆盖加载、空数据和错误状态。
6. 表单中的上游 Key 输入使用密码类型；不向用户展示完整上游 Key。
7. 应用无登录页，首屏直接进入主布局。
8. 分组队列「按模型能力排序」只可修改当前表单，不得自动保存；支持本地启发式 / 外部通用 / 外部编码；外部分需标注 OpenRouter 来源与缓存状态，未匹配回退本地启发式；未知模型稳定排后，用户仍可拖拽微调。合同见 [model-queue-sort.md](./model-queue-sort.md)。
9. **配置到 Pi**：入口在**分组页**列表行「配置到 Pi」；调用 `exportGroupToPiAgent(groupId)`；**无 Key UI / 无 Key 入参**；模型名=分组名，写入本机 `~/.pi/agent/models.json` 的单一 `providers.model-hub`（按 id upsert）。
10. 信息架构无「API 密钥 / 客户端 Key」页面与导航。
11. **上游访问**：禁止供应商页「测试连接」及任何自动/后台对用户上游的测活；**不**展示供应商熔断健康徽章，**不**调用 `listHealth`（已删除）；分组页「拉取模型」**仅**用户点击触发，不得在 `onMounted`/保存时自动拉取。合同见 backend [upstream-access.md](../backend/upstream-access.md)。
12. **故障转移**：分组队列始终按顺序故障转移，UI **无** `auto_failover` 开关；创建/更新分组 payload 不得再传该字段。
13. **首页「最近成功请求」**：展示全局最近一次成功日志的分组 / 供应商 / 上游模型 / 时间（日志态，非队列首选）；调用 `getLastSuccessRequest()`；空态「暂无成功请求」；与今日统计一并刷新，独立错误文案；不轮询、不按分组展开。成功语义见 backend [logging-guidelines.md](../backend/logging-guidelines.md)。
14. **页面职责**：首页只承载代理运行状态、Base URL、启停/刷新、请求统计与接入指引；端口修改、数据目录、应用更新和自动检查偏好统一放在设置页。
15. **启动更新检查**：应用壳层仅在挂载时读取一次 `check_update_on_startup`；发现新版本只展示可关闭提示和设置页入口，探测后立即关闭 `Update` 资源，不自动下载或安装。

## 状态与生命周期

- 局部交互使用 `ref` / `reactive` / `computed`；**对话框业务表单字段**用 TanStack Form（见 3.2），不与页面级 `reactive` 双源。
- 异步加载在 `onMounted` 中触发；定时器和事件订阅在 `onUnmounted` 中清理。
- 提交期间禁用重复操作，并在失败时保留用户可修正的输入（Form values 与编辑 id）。
- 编辑已有分组表单必须使用稳定的 `editingGroupId: number | null` 表达编辑目标；保存时先快照 id，id 非空只能调用更新，只有新建态才调用创建。添加条目、拉取模型、批量添加、排序等异步/局部操作不得清空编辑 id。
- 新建/编辑供应商与分组复用 `AppDialog`，页面以稳定实体 id 区分创建和更新。打开新建 Dialog 前 `form.reset` 默认值；保存失败保留 Dialog 与 Form 输入；保存成功后关闭并刷新列表；保存期间禁止重复提交和关闭（`closeDisabled`）。

## 对话框合同

- 通用外壳使用 `src/components/AppDialog.vue`（**内部**基于 `HDialog` 的薄封装），页面保留表单和领域保存逻辑，不引入页面专用遮罩实现。
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
