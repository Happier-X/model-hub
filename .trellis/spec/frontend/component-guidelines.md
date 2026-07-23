# 组件规范

> Vue 3 管理台组件的编写约定。

## 基本模式

1. 使用 Vue 单文件组件与 `<script setup lang="ts">`。
2. Props 使用 `defineProps` 声明明确类型；事件使用 `defineEmits` 声明名称和参数。
3. 页面负责加载与提交，通用组件负责展示和用户交互；复杂领域操作下沉到 `src/api/tauri.ts` 或组合式函数。
4. 代理运行状态、Base URL 和最后错误必须使用清晰、可行动的中文文案。
5. 列表必须覆盖加载、空数据和错误状态。
6. 表单中的上游 Key 输入使用密码类型；不向用户展示完整上游 Key。
7. 应用无登录页，首屏直接进入主布局。
8. 分组队列「按模型能力排序」只可修改当前表单，不得自动保存；支持本地启发式 / 外部通用 / 外部编码；外部分需标注 OpenRouter 来源与缓存状态，未匹配回退本地启发式；未知模型稳定排后，用户仍可拖拽微调。合同见 [model-queue-sort.md](./model-queue-sort.md)。
9. **配置到 Pi**：入口在**分组页**列表行「配置到 Pi」；调用 `exportGroupToPiAgent(groupId)`；**无 Key UI / 无 Key 入参**；模型名=分组名，写入本机 `~/.pi/agent/models.json` 的单一 `providers.model-hub`（按 id upsert）。
10. 信息架构无「API 密钥 / 客户端 Key」页面与导航。
11. **上游访问**：禁止供应商页「测试连接」及任何自动/后台对用户上游的测活；供应商页与分组页健康状态只读展示，不提供手动刷新入口；分组页「拉取模型」**仅**用户点击触发，不得在 `onMounted`/保存时自动拉取。健康展示只读熔断内存（`listHealth`），不打上游。合同见 backend [upstream-access.md](../backend/upstream-access.md)。

## 状态与生命周期

- 局部交互使用 `ref` / `reactive` / `computed`。
- 异步加载在 `onMounted` 中触发；定时器和事件订阅在 `onUnmounted` 中清理。
- 提交期间禁用重复操作，并在失败时保留用户可修正的输入。
- 编辑已有分组表单必须使用稳定的 `editingGroupId: number | null` 表达编辑目标；保存时先快照 id，id 非空只能调用更新，只有新建态才调用创建。添加条目、拉取模型、批量添加、排序、健康刷新等异步/局部操作不得清空编辑 id。
- 新建/编辑供应商与分组复用 `AppDialog`，页面以稳定实体 id 区分创建和更新。打开新建 Dialog 前重置默认值；保存失败保留 Dialog 与输入；保存成功后关闭并刷新列表；保存期间禁止重复提交和关闭。

## 对话框合同

- 通用外壳使用 `src/components/AppDialog.vue`，页面保留表单和领域保存逻辑，不引入页面专用遮罩实现。
- 必须提供语义化标题、`role="dialog"`、`aria-modal="true"`、Escape/遮罩/关闭按钮、焦点循环及关闭后的焦点恢复。
- 普通表单使用 `size="default"`；分组等长表单使用 `size="wide"`，限制视口高度并在内容区内部滚动。
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
- **关窗 vs 退出**：概览/托盘等须写明——关闭窗口 = 隐藏到托盘、代理继续；仅托盘「退出」停止代理并释放端口；自动改口可提示意外多开时托盘退出旧实例。

## 禁止模式

- 使用 Options API 与 Composition API 混写同一组件。
- 在多个页面重复手写 Tauri 命令名和返回类型。
- 为未规划的多用户功能预埋权限组件。
- 用空数组或空对象吞掉加载失败。
- 对用户配置的上游做「测试连接」、定时探测或打开页面自动拉 `/models`。
