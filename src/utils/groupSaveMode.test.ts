import assert from "node:assert/strict";
import test from "node:test";
import { getGroupSaveMode } from "./groupSaveMode.ts";

test("editingGroupId 为 null 时为新建态", () => {
  assert.equal(getGroupSaveMode(null), "create");
});

test("editingGroupId 为数字时为编辑态", () => {
  assert.equal(getGroupSaveMode(1), "update");
  assert.equal(getGroupSaveMode(42), "update");
});

test("编辑 id 在模拟异步操作后保持不变且仍走 update", () => {
  // 模拟页面状态：startEdit 写入 id，后续 bulk/sort/pull 不得清空
  let editingGroupId: number | null = 7;
  const ops = [
    () => {
      /* bulkAddProviderModels 只改 form.items */
    },
    () => {
      /* sortQueueByCapability 只改 form.items 顺序 */
    },
    () => {
      /* pullModels / refreshHealth 不碰 editingGroupId */
    },
  ];
  for (const op of ops) {
    op();
    assert.equal(editingGroupId, 7);
    assert.equal(getGroupSaveMode(editingGroupId), "update");
  }

  // resetForm 才清空
  editingGroupId = null;
  assert.equal(getGroupSaveMode(editingGroupId), "create");
});

test("保存分支快照：快照后的 id 决定模式，与后续 reset 无关", () => {
  let editingGroupId: number | null = 3;
  const snapshot = editingGroupId;
  const mode = getGroupSaveMode(snapshot);
  assert.equal(mode, "update");

  // 成功保存后 reset
  editingGroupId = null;
  assert.equal(getGroupSaveMode(editingGroupId), "create");
  // 已快照的分支仍应按 update 执行
  assert.equal(getGroupSaveMode(snapshot), "update");
});
