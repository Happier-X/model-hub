export type GroupSaveMode = "create" | "update";

/** 根据稳定的编辑分组 id 判定保存模式，避免依赖列表对象引用。 */
export function getGroupSaveMode(editingGroupId: number | null): GroupSaveMode {
  return editingGroupId === null ? "create" : "update";
}
