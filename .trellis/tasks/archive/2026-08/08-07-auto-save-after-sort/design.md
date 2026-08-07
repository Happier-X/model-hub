# 设计：排序后自动保存

## 数据流

```
sortQueueByCapability
  ├─ items < 2 → 提示，return
  ├─ ensureLeaderboardForExternalSort 失败 → 提示，return
  ├─ sortQueueByLeaderboard
  │    ├─ 顺序未变 → 提示「已符合」，return
  │    └─ 顺序变了 → applySortedItems(sorted, ...)
  │         └─ isEditing ? autoSaveAfterSort() : formMessage「保存后生效」
  │              ├─ updateGroup({ id, name, thinking_effort, items })
  │              ├─ 成功 → formMessage「已保存，可继续拖拽微调」（不跳转）
  │              └─ 失败 → error.value（保留排序，可再保存）
```

## 关键实现

### payload 构建复用
onSubmit 与 autoSaveAfterSort 共用同一 payload 逻辑，避免漂移：
```ts
function buildGroupPayload(value: typeof formValues.value) {
  return {
    name: value.name,
    thinking_effort: value.thinking_effort,
    items: value.items.filter((i) => i.provider_id > 0 && i.upstream_model.trim()),
  };
}
```
- onSubmit 改用 `buildGroupPayload(value)`（行为不变）
- autoSaveAfterSort 用 `buildGroupPayload(formValues.value)`

### autoSaveAfterSort
```ts
async function autoSaveAfterSort(): Promise<boolean> {
  if (saving.value) return false;
  const targetId = editingGroupId.value;
  if (targetId === null) return false; // 新建态不走此函数
  saving.value = true;
  try {
    await updateGroup({ id: targetId, ...buildGroupPayload(formValues.value) });
    formMessage.value = "已按 llm_benchmark 排序并保存，可继续拖拽微调。";
    return true;
  } catch (e) {
    error.value = extractInvokeError(e);
    return false;
  } finally {
    saving.value = false;
  }
}
```

### sortQueueByCapability 尾部
```ts
applySortedItems(sorted, ""); // 不展示旧文案
if (isEditing.value) {
  await autoSaveAfterSort();
} else {
  formMessage.value = "已按 llm_benchmark 综合能力排序；未匹配项已沉底。点击“保存”后生效，仍可拖拽微调。";
}
```
- 注意：`applySortedItems` 内部会设置 formMessage（旧文案），需要调整：改为 applySortedItems 只设中间态或传空，最终提示由上面分支覆盖。

## 风险

- `saving` 与表单提交按钮共用：自动保存期间按钮 disabled，防重复提交 ✓
- 自动保存期间用户拖拽：顺序基于 formValues，保存的是当前最新值 ✓（updateGroup 用最新 formValues）
- 新建态排序不自动建分组：符合决策（避免误创建）✓
- applySortedItems 的 formMessage 与最终提示竞争：统一在分支里最终赋值覆盖

## 测试

- 无新增单测（纯页面逻辑迁移）；跑 typecheck/lint/unit/build
- 手工：编辑态排序 → 提示已保存 → 重开顺序保留；新建态排序 → 提示保存后生效
