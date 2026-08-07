# 设计：自动同步开关迁移

## 数据流

```
ProvidersPage（移除 auto_sync 列/开关）
  └─ setProviderAutoSync 调用移至 GroupFormPage
GroupFormPage「可选模型」供应商行 HCell
  └─ HSwitch (auto_sync) → toggleProviderAutoSync(p, next)
       ├─ 乐观更新 p.auto_sync
       ├─ await setProviderAutoSync(p.id, next)（IPC 不变）
       ├─ 成功：以返回值同步 p（Object.assign）
       └─ 失败：回滚 p.auto_sync = previous + error 提示
```

## GroupFormPage 改动点

### state
```ts
/** 行内自动同步开关进行中的 id 集合（防重复点击） */
const autoSyncTogglingIds = ref<Set<number>>(new Set());
```

### 函数（复制自 ProvidersPage，替换 items → providers）
```ts
async function toggleProviderAutoSync(p: Provider, next: boolean) {
  if (autoSyncTogglingIds.value.has(p.id)) return;
  const previous = p.auto_sync;
  const target = providers.value.find((it) => it.id === p.id);
  if (target) target.auto_sync = next;
  autoSyncTogglingIds.value = new Set(autoSyncTogglingIds.value).add(p.id);
  try {
    const updated = await setProviderAutoSync(p.id, next);
    const sync = providers.value.find((it) => it.id === p.id);
    if (sync) Object.assign(sync, updated);
  } catch (e) {
    const failed = providers.value.find((it) => it.id === p.id);
    if (failed) failed.auto_sync = previous;
    error.value = extractInvokeError(e);
  } finally {
    const nextSet = new Set(autoSyncTogglingIds.value);
    nextSet.delete(p.id);
    autoSyncTogglingIds.value = nextSet;
  }
}
```
（需确认 GroupFormPage 已导入 `setProviderAutoSync` 与 `extractInvokeError`；无则补）

### 模板：HCell suffix
```html
<template #suffix>
  <div class="flex items-center gap-2" @click.stop>
    <HSwitch
      :model-value="p.auto_sync"
      :disabled="autoSyncTogglingIds.has(p.id)"
      :aria-label="`${p.name} 自动同步`"
      @update:model-value="toggleProviderAutoSync(p, $event)"
    />
    <!-- 原有模型数 / 同步状态 span 保留 -->
  </div>
</template>
```
- `@click.stop`：点击开关不触发展开手风琴（HCell clickable）。
- HSwitch 默认有 title/aria，不必加文字标签（供应商行空间有限）；如需可见文案可加 title="自动同步"。

### error 提示
- GroupFormPage 已有 `error` ref + 展示位（需确认：表单页顶部错误提示是否存在；否则复用 formMessage）。

## ProvidersPage 改动点

- `providerColumns` 删除 `{ key: "auto_sync", title: "自动同步" }`
- 删除列模板 `v-else-if="column.key === 'auto_sync'"` 分支
- 删除 `toggleProviderAutoSync`、`autoSyncTogglingIds`
- 删除 `setProviderAutoSync` 导入（grep 确认无其他使用）
- `ProviderFormValues.auto_sync` / `defaultFormValues.auto_sync: true` / 提交透传**保留**

## 风险

- HCell clickable 与内部 HSwitch 事件冲突 → `@click.stop` 兜底（happier-ui HSwitch 内部应已 stopPropagation，双保险无害）。
- GroupFormPage 是**新建+编辑复用**页：开关作用于 providers 数组（全局列表），与分组表单值无关，位置天然合适。
- 供应商页「立即同步」按钮保留（与自动同步是两回事，用户未要求移除）。

## 测试

- 前端无新增单测（纯 UI 迁移，逻辑已有后端 IPC 单测覆盖）；跑既有 typecheck/lint/unit/build。
- 手工验证：供应商页无自动同步列；分组表单页开关切换成功/失败回滚。
