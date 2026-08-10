<script setup lang="ts">
import { ref, watch } from "vue";
import { Button } from "@/components/ui/button";
import type { Group, GroupItem, ThinkingEffort } from "../../api/tauri";

const props = defineProps<{
  group: Group;
  providerName: (providerId: number, fallbackName?: string) => string;
  thinkingEffortLabels: Record<ThinkingEffort, string>;
  saving: boolean;
  exportingPi: boolean;
}>();

const emit = defineEmits<{
  edit: [];
  "export-pi": [];
  "delete-group": [];
  "persist-items": [items: GroupItem[]];
}>();

/** 卡片内拖拽/删除的乐观本地队列：松手即展示，保存失败由页面 refresh 回滚。 */
const localItems = ref<GroupItem[]>(props.group.items);

watch(
  () => props.group.items,
  (items) => {
    localItems.value = items;
  },
);

const showDeleteConfirm = ref(false);
const dragFromIndex = ref<number | null>(null);
const dragOverIndex = ref<number | null>(null);

function openDeleteConfirm() {
  if (props.saving) return;
  showDeleteConfirm.value = true;
}

function cancelDelete() {
  showDeleteConfirm.value = false;
}

function confirmDelete() {
  if (props.saving) return;
  showDeleteConfirm.value = false;
  emit("delete-group");
}

function reorder(from: number, to: number) {
  if (props.saving) return;
  const items = localItems.value;
  if (from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) {
    return;
  }
  const next = items.slice();
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  localItems.value = next.map((item, index) => ({ ...item, sort_order: index }));
  emit("persist-items", localItems.value);
}

function removeMember(index: number) {
  if (props.saving) return;
  localItems.value = localItems.value
    .filter((_, i) => i !== index)
    .map((item, i) => ({ ...item, sort_order: i }));
  emit("persist-items", localItems.value);
}

function onDragStart(index: number, event: DragEvent) {
  if (props.saving) {
    event.preventDefault();
    return;
  }
  dragFromIndex.value = index;
  dragOverIndex.value = index;
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", String(index));
  }
}

function onDragOver(index: number, event: DragEvent) {
  if (props.saving) return;
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = "move";
  }
  if (dragFromIndex.value === null) return;
  dragOverIndex.value = index;
}

function onDrop(index: number, event: DragEvent) {
  event.preventDefault();
  const from = dragFromIndex.value;
  dragFromIndex.value = null;
  dragOverIndex.value = null;
  if (from === null) return;
  reorder(from, index);
}

function onDragEnd() {
  dragFromIndex.value = null;
  dragOverIndex.value = null;
}
</script>

<template>
  <article
    class="group-card relative flex flex-col rounded-xl border border-slate-200 bg-white p-4 transition hover:border-cyan-300 hover:bg-cyan-50/30"
    :class="{ 'opacity-70': saving }"
  >
    <!-- 删组二次确认覆盖层 -->
    <div
      v-if="showDeleteConfirm"
      class="absolute inset-0 z-10 flex flex-col items-center justify-center gap-3 rounded-xl bg-white/95 p-4 backdrop-blur-sm"
    >
      <p class="text-sm font-medium text-slate-800">确认删除分组「{{ group.name }}」？</p>
      <p class="text-xs text-slate-500">此操作不可恢复。</p>
      <div class="flex gap-2">
        <Button variant="outline" size="sm" type="button" :disabled="saving" @click="cancelDelete">
          取消
        </Button>
        <Button variant="destructive" size="sm" type="button" :disabled="saving" @click="confirmDelete">
          {{ saving ? "删除中…" : "确认删除" }}
        </Button>
      </div>
    </div>

    <!-- 头部：分组名 + 思考强度 + 自动同步标签 -->
    <div class="flex flex-wrap items-center gap-2">
      <span class="break-all text-base font-semibold text-slate-800">{{ group.name }}</span>
      <span
        v-if="group.thinking_effort && group.thinking_effort !== 'off'"
        class="rounded-full bg-violet-50 px-2 py-0.5 text-[11px] text-violet-700"
        title="思考强度档位"
      >
        思考 · {{ thinkingEffortLabels[group.thinking_effort] ?? group.thinking_effort }}
      </span>
      <span v-if="saving" class="text-[11px] text-cyan-700">保存中…</span>
    </div>

    <p class="mt-1 text-xs text-slate-500">
      {{ localItems.length }} 个模型 · 队列顺序即故障转移优先级
    </p>

    <!-- 模型队列 -->
    <ol class="mt-3 max-h-44 space-y-1 overflow-y-auto pr-1 text-sm">
      <li
        v-for="(item, idx) in localItems"
        :key="item.id"
        class="flex items-center gap-1.5 rounded-md px-1.5 py-1 text-slate-700 transition"
        :class="
          dragOverIndex === idx
            ? 'bg-cyan-50 ring-1 ring-cyan-300'
            : dragFromIndex === idx
              ? 'bg-slate-50 opacity-70'
              : 'hover:bg-slate-50'
        "
        @dragover="onDragOver(idx, $event)"
        @drop="onDrop(idx, $event)"
      >
        <button
          type="button"
          class="cursor-grab select-none rounded border border-slate-200 bg-slate-50 px-1 py-0.5 text-[10px] text-slate-500 active:cursor-grabbing disabled:cursor-not-allowed disabled:opacity-50"
          title="拖动排序"
          :draggable="!saving"
          :disabled="saving"
          @dragstart="onDragStart(idx, $event)"
          @dragend="onDragEnd"
        >
          ⋮⋮
        </button>
        <span class="w-5 shrink-0 text-xs tabular-nums text-slate-400">{{ idx + 1 }}.</span>
        <div class="min-w-0 flex-1">
          <span class="block truncate text-slate-600">
            {{ providerName(item.provider_id, item.provider_name) }}
          </span>
          <span class="block truncate font-mono text-xs text-slate-500">{{ item.upstream_model }}</span>
        </div>
        <button
          type="button"
          class="shrink-0 rounded px-1.5 py-0.5 text-xs text-rose-600 hover:bg-rose-50 disabled:opacity-50"
          title="删除成员"
          :disabled="saving"
          @click="removeMember(idx)"
        >
          ×
        </button>
      </li>
      <li v-if="localItems.length === 0" class="px-1.5 py-2 text-xs text-slate-400">暂无模型</li>
    </ol>

    <!-- 操作区 -->
    <div class="mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 border-t border-slate-100 pt-3">
      <Button
        variant="outline"
        size="sm"
        type="button"
        :disabled="exportingPi || saving"
        @click="emit('export-pi')"
      >
        {{ exportingPi ? "配置中…" : "配置到 Pi" }}
      </Button>
      <Button variant="ghost" size="sm" type="button" :disabled="saving" @click="emit('edit')">
        编辑
      </Button>
      <Button
        variant="destructive"
        size="sm"
        type="button"
        class="ml-auto"
        :disabled="saving"
        @click="openDeleteConfirm"
      >
        删除
      </Button>
    </div>
  </article>
</template>
