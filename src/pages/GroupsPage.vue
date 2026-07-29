<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Plus } from "@lucide/vue";
import { useForm } from "@tanstack/vue-form";
import { HBadge, HButton, HCard, HEmpty, HInput, HSelect, type HSelectOption } from "happier-ui";
import {
  createGroup,
  deleteGroup,
  exportGroupToPiAgent,
  extractInvokeError,
  fetchProviderModels,
  getModelLeaderboard,
  listGroups,
  listProviders,
  syncGroupNow,
  updateGroup,
  type Group,
  type ModelLeaderboardSnapshot,
  type Provider,
  type ThinkingEffort,
} from "../api/tauri";
import AppDialog from "../components/AppDialog.vue";
import {
  buildExternalScoreIndex,
  matchModelToLeaderboard,
  sortQueueByLeaderboard,
  type ExternalLeaderboardEntry,
  type LeaderboardIndex,
} from "../utils/modelCapability";
import { getGroupSaveMode } from "../utils/groupSaveMode";

type QueueItemDraft = {
  uid: number;
  provider_id: number;
  upstream_model: string;
};

type GroupFormValues = {
  name: string;
  thinking_effort: ThinkingEffort;
  items: QueueItemDraft[];
  source_provider_id?: number | null;
};

const defaultFormValues: GroupFormValues = {
  name: "",
  thinking_effort: "off",
  items: [],
  source_provider_id: null,
};

const thinkingEffortOptions: HSelectOption[] = [
  { value: "off", label: "关闭（不注入）" },
  { value: "auto", label: "自动最佳" },
  { value: "minimal", label: "最小" },
  { value: "low", label: "低" },
  { value: "medium", label: "中" },
  { value: "high", label: "高" },
];

const thinkingEffortLabels: Record<ThinkingEffort, string> = {
  off: "关闭",
  auto: "自动最佳",
  minimal: "最小",
  low: "低",
  medium: "中",
  high: "高",
};

const groups = ref<Group[]>([]);
const providers = ref<Provider[]>([]);
const error = ref("");
const message = ref("");
/** 正在导出到 Pi 的分组 id */
const exportingPiId = ref<number | null>(null);
/** 稳定编辑目标 id；null 表示新建态。不得依赖列表对象引用。 */
const editingGroupId = ref<number | null>(null);
const isEditing = computed(() => editingGroupId.value !== null);
const saving = ref(false);
const dialogOpen = ref(false);
/** 每条队列条目拉取到的上游模型 id 列表 */
const modelOptions = ref<Record<number, string[]>>({});
const fetchingModels = ref<Record<number, boolean>>({});
const bulkProviderId = ref(0);
const bulkAddingModels = ref(false);
const bulkMessage = ref("");
const dragFromIndex = ref<number | null>(null);
const dragOverIndex = ref<number | null>(null);

const providerSelectOptions = computed<HSelectOption[]>(() => [
  { value: 0, label: "选择供应商" },
  ...providers.value.map((p) => ({ value: p.id, label: p.name })),
]);

const bindProviderOptions = computed<HSelectOption[]>(() => [
  { value: 0, label: "不绑定" },
  ...providers.value.map((p) => ({ value: p.id, label: p.name })),
]);
const leaderboard = ref<ModelLeaderboardSnapshot | null>(null);
const leaderboardLoading = ref(false);
const leaderboardError = ref("");
let nextItemUid = 1;

const form = useForm({
  defaultValues: {
    name: defaultFormValues.name,
    thinking_effort: defaultFormValues.thinking_effort,
    items: [] as QueueItemDraft[],
    source_provider_id: defaultFormValues.source_provider_id,
  },
  onSubmit: async ({ value }) => {
    if (saving.value) return;
    // 快照编辑 id，避免异步期间状态漂移误走 create
    const targetId = editingGroupId.value;
    const mode = getGroupSaveMode(targetId);
    saving.value = true;
    try {
      const payload = {
        name: value.name,
        thinking_effort: value.thinking_effort,
        items: value.items.filter((i) => i.provider_id > 0 && i.upstream_model.trim()),
        source_provider_id: value.source_provider_id || null,
      };
      if (mode === "update" && targetId !== null) {
        await updateGroup({ id: targetId, ...payload });
      } else {
        await createGroup(payload);
      }
      dialogOpen.value = false;
      resetForm();
      await refresh();
    } catch (e) {
      // 失败保留编辑态与表单，便于重试
      error.value = extractInvokeError(e);
    } finally {
      saving.value = false;
    }
  },
});

const isBound = computed(() => !!formValues.value.source_provider_id);

/** 绑定态「上次同步」文案：优先取当前编辑分组的 last_sync_at。 */
const boundLastSyncText = computed(() => {
  if (editingGroupId.value === null) return "尚未同步";
  const g = groups.value.find((item) => item.id === editingGroupId.value);
  const ts = g?.last_sync_at;
  if (ts == null || ts <= 0) return "尚未同步";
  return formatUnix(ts);
});

/** 订阅表单 values，供队列操作与模板读取 */
const formValues = form.useSelector((s) => s.values);

const editingGroupName = computed(() => {
  if (editingGroupId.value === null) return "";
  const g = groups.value.find((item) => item.id === editingGroupId.value);
  return g?.name ?? formValues.value.name;
});

function createQueueItem(providerId: number, upstreamModel: string): QueueItemDraft {
  return {
    uid: nextItemUid++,
    provider_id: providerId,
    upstream_model: upstreamModel,
  };
}

function setItems(next: QueueItemDraft[]) {
  form.setFieldValue("items", next);
}

function updateItemAt(
  index: number,
  patch: Partial<Pick<QueueItemDraft, "provider_id" | "upstream_model">>,
) {
  const items = formValues.value.items;
  const current = items[index];
  if (!current) return;
  const next = items.slice();
  next[index] = { ...current, ...patch };
  setItems(next);
}

const providerMap = computed(() => new Map(providers.value.map((p) => [p.id, p])));

const externalIndex = computed<LeaderboardIndex | null>(() => {
  if (!leaderboard.value) return null;
  return buildExternalScoreIndex(leaderboard.value.models);
});

const leaderboardStatusText = computed(() => {
  if (leaderboardLoading.value) return "榜单加载中…";
  if (!leaderboard.value) {
    return leaderboardError.value || "尚未加载外部榜单（排序时将自动拉取）";
  }
  const t = formatUnix(leaderboard.value.fetched_at_unix);
  const parts = [
    `OpenRouter ${leaderboard.value.models.length} 条`,
    `更新于 ${t}`,
  ];
  if (leaderboard.value.cache_hit) parts.push("缓存命中");
  if (leaderboard.value.stale) parts.push("陈旧缓存");
  if (leaderboardError.value) parts.push(`刷新失败：${leaderboardError.value}`);
  return parts.join(" · ");
});

/** 每条队列的展示分（按 index 缓存，避免模板内多次调用）。 */
const queueDisplayScores = computed(() =>
  formValues.value.items.map((item) => matchModelToLeaderboard(item.upstream_model, externalIndex.value)),
);

function formatUnix(unix: number): string {
  if (!unix || unix <= 0) return "未知时间";
  try {
    return new Date(unix * 1000).toLocaleString("zh-CN", { hour12: false });
  } catch {
    return String(unix);
  }
}

async function loadLeaderboard(forceRefresh = false) {
  leaderboardLoading.value = true;
  leaderboardError.value = "";
  try {
    leaderboard.value = await getModelLeaderboard(forceRefresh);
    if (leaderboard.value.stale) {
      // stale 状态由结构化快照展示；它仍是可用的成功结果。
      leaderboardError.value = "";
    }
  } catch (e) {
    leaderboardError.value = extractInvokeError(e);
    // 失败不影响本地排序；保留旧快照（若有）
  } finally {
    leaderboardLoading.value = false;
  }
}

async function ensureLeaderboardForExternalSort() {
  if (leaderboard.value && !leaderboardLoading.value) return true;
  await loadLeaderboard(false);
  return !!leaderboard.value;
}

async function refresh() {
  try {
    [groups.value, providers.value] = await Promise.all([listGroups(), listProviders()]);
    if (!bulkProviderId.value && providers.value.length > 0) {
      bulkProviderId.value = providers.value[0]?.id ?? 0;
    }
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

function resetForm() {
  editingGroupId.value = null;
  form.reset({ name: "", thinking_effort: "off", items: [], source_provider_id: null });
  modelOptions.value = {};
  fetchingModels.value = {};
  bulkProviderId.value = providers.value[0]?.id ?? 0;
  bulkMessage.value = "";
  error.value = "";
  message.value = "";
}

function openCreate() {
  resetForm();
  dialogOpen.value = true;
}

function closeDialog() {
  if (saving.value) return;
  dialogOpen.value = false;
  resetForm();
}

function startEdit(g: Group) {
  error.value = "";
  message.value = "";
  editingGroupId.value = g.id;
  dialogOpen.value = true;
  form.reset({
    name: g.name,
    thinking_effort: g.thinking_effort,
    items: g.items.map((i) => createQueueItem(i.provider_id, i.upstream_model)),
    source_provider_id: g.source_provider_id || null,
  });
  modelOptions.value = {};
  fetchingModels.value = {};
  dragFromIndex.value = null;
  dragOverIndex.value = null;
  bulkMessage.value = "";
}

function addItem() {
  const first = providers.value[0];
  setItems([...formValues.value.items, createQueueItem(first?.id ?? 0, "gpt-4o-mini")]);
}

function reorderQueue(from: number, to: number) {
  const items = formValues.value.items;
  if (from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) {
    return;
  }

  const nextItems = items.slice();
  const [movedItem] = nextItems.splice(from, 1);
  nextItems.splice(to, 0, movedItem);

  const indexOrder = items.map((_, i) => i);
  const [movedIndex] = indexOrder.splice(from, 1);
  indexOrder.splice(to, 0, movedIndex);

  const nextOptions: Record<number, string[]> = {};
  const nextFetching: Record<number, boolean> = {};
  indexOrder.forEach((oldIndex, newIndex) => {
    if (modelOptions.value[oldIndex]) {
      nextOptions[newIndex] = modelOptions.value[oldIndex];
    }
    if (fetchingModels.value[oldIndex]) {
      nextFetching[newIndex] = fetchingModels.value[oldIndex];
    }
  });

  setItems(nextItems);
  modelOptions.value = nextOptions;
  fetchingModels.value = nextFetching;
  bulkMessage.value = "队列顺序已调整，点击“保存”后生效";
}

function moveItem(index: number, delta: number) {
  reorderQueue(index, index + delta);
}

function removeItem(index: number) {
  const items = formValues.value.items;
  setItems(items.filter((_, i) => i !== index));
  const nextOptions: Record<number, string[]> = {};
  const nextFetching: Record<number, boolean> = {};
  items.forEach((_, newIndex) => {
    if (newIndex === index) return;
    const mapped = newIndex > index ? newIndex - 1 : newIndex;
    const oldIndex = newIndex;
    if (modelOptions.value[oldIndex]) {
      nextOptions[mapped] = modelOptions.value[oldIndex];
    }
    if (fetchingModels.value[oldIndex]) {
      nextFetching[mapped] = fetchingModels.value[oldIndex];
    }
  });
  modelOptions.value = nextOptions;
  fetchingModels.value = nextFetching;
}

function onDragStart(index: number, event: DragEvent) {
  dragFromIndex.value = index;
  dragOverIndex.value = index;
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", String(index));
  }
}

function onDragOver(index: number, event: DragEvent) {
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
  reorderQueue(from, index);
}

function onDragEnd() {
  dragFromIndex.value = null;
  dragOverIndex.value = null;
}

function applySortedItems(sorted: QueueItemDraft[], msg: string) {
  const items = formValues.value.items;
  const oldIndexByUid = new Map(items.map((item, index) => [item.uid, index]));
  const nextOptions: Record<number, string[]> = {};
  const nextFetching: Record<number, boolean> = {};
  sorted.forEach((item, newIndex) => {
    const oldIndex = oldIndexByUid.get(item.uid);
    if (oldIndex === undefined) return;
    if (modelOptions.value[oldIndex]) nextOptions[newIndex] = modelOptions.value[oldIndex];
    if (fetchingModels.value[oldIndex]) nextFetching[newIndex] = fetchingModels.value[oldIndex];
  });

  setItems(sorted);
  modelOptions.value = nextOptions;
  fetchingModels.value = nextFetching;
  dragFromIndex.value = null;
  dragOverIndex.value = null;
  bulkMessage.value = msg;
}

async function sortQueueByCapability() {
  const items = formValues.value.items;
  if (items.length < 2) {
    bulkMessage.value = "队列条目少于 2 条，无需排序";
    return;
  }

  const ok = await ensureLeaderboardForExternalSort();
  if (!ok) {
    bulkMessage.value =
      "外部榜单不可用，已保持当前顺序。请检查网络后强制刷新榜单。";
    return;
  }

  const before = items.map((item) => item.uid);
  const sorted = sortQueueByLeaderboard(items, (item) => item.upstream_model, externalIndex.value);

  const after = sorted.map((item) => item.uid);
  if (before.every((uid, index) => uid === after[index])) {
    bulkMessage.value = "当前顺序已符合 OpenRouter 榜单排序（未匹配项保持原序）";
    return;
  }

  applySortedItems(sorted, "已按 OpenRouter 通用能力排序；未匹配项已沉底。点击“保存”后生效，仍可拖拽微调。");
}

async function pullModels(index: number) {
  const item = formValues.value.items[index];
  if (!item || !item.provider_id) {
    error.value = "请先选择供应商，再拉取模型";
    return;
  }
  fetchingModels.value = { ...fetchingModels.value, [index]: true };
  try {
    const ids = await fetchProviderModels({ provider_id: item.provider_id });
    modelOptions.value = { ...modelOptions.value, [index]: ids };
    error.value = "";
    if (ids.length === 0) {
      error.value = "上游返回空模型列表，请手填上游模型名";
    }
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    fetchingModels.value = { ...fetchingModels.value, [index]: false };
  }
}

function pickModel(index: number, modelId: string) {
  updateItemAt(index, { upstream_model: modelId });
}

async function bulkAddProviderModels() {
  const providerId = bulkProviderId.value;
  if (!providerId) {
    error.value = "请先选择要批量添加模型的供应商";
    return;
  }
  bulkAddingModels.value = true;
  bulkMessage.value = "";
  try {
    const ids = await fetchProviderModels({ provider_id: providerId });
    if (ids.length === 0) {
      error.value = "上游返回空模型列表，队列未修改";
      return;
    }

    const items = formValues.value.items.slice();
    const existing = new Set(
      items.map((item) => `${item.provider_id}\u0000${item.upstream_model.trim()}`),
    );
    let added = 0;
    let skipped = 0;
    for (const rawId of ids) {
      const modelId = rawId.trim();
      if (!modelId) {
        skipped += 1;
        continue;
      }
      const key = `${providerId}\u0000${modelId}`;
      if (existing.has(key)) {
        skipped += 1;
        continue;
      }
      items.push(createQueueItem(providerId, modelId));
      existing.add(key);
      added += 1;
    }
    setItems(items);
    error.value = "";
    bulkMessage.value = `已添加 ${added} 个模型${skipped > 0 ? `，跳过 ${skipped} 个重复或空模型` : ""}；点击“保存”后生效`;
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    bulkAddingModels.value = false;
  }
}

async function remove(id: number) {
  if (!confirm("确认删除该分组？")) return;
  try {
    await deleteGroup(id);
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function exportToPi(groupId: number) {
  exportingPiId.value = groupId;
  message.value = "";
  try {
    const result = await exportGroupToPiAgent(groupId);
    error.value = "";
    message.value = `已写入 Pi 配置：${result.path}\n模型 ${result.provider_id}/${result.group_name}（当前 model-hub 共 ${result.model_count} 个模型），Base URL ${result.base_url}。请在 Pi 中打开 /model 选择 model-hub/${result.group_name}。`;
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    exportingPiId.value = null;
  }
}

async function handleSyncNow() {
  if (saving.value || !editingGroupId.value) return;
  saving.value = true;
  error.value = "";
  message.value = "";
  try {
    const updatedGroup = await syncGroupNow(editingGroupId.value);
    form.reset({
      name: updatedGroup.name,
      thinking_effort: updatedGroup.thinking_effort,
      items: updatedGroup.items.map((i) => createQueueItem(i.provider_id, i.upstream_model)),
      source_provider_id: updatedGroup.source_provider_id || null,
    });
    modelOptions.value = {};
    fetchingModels.value = {};
    message.value = "同步完成！模型列表已更新。";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    saving.value = false;
  }
}

onMounted(async () => {
  await refresh();
});
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden">
    <AppDialog
      :open="dialogOpen"
      :title="isEditing ? '编辑分组' : '新建分组'"
      size="wide"
      :close-disabled="saving"
      @close="closeDialog"
    >
      <section>
        <h2 class="sr-only">{{ isEditing ? "编辑分组" : "新建分组" }}</h2>
        <p v-if="isEditing" class="mb-2 text-sm text-cyan-800">
          正在编辑：{{ editingGroupName || formValues.name || `分组 #${editingGroupId}` }}
        </p>
        <p class="mb-3 text-sm text-slate-500">分组名 = 客户端 model；队列顺序即故障转移优先级。</p>
        <form class="space-y-3" @submit.prevent="form.handleSubmit()">
          <div class="grid gap-3 md:grid-cols-2">
            <form.Field name="name">
              <template #default="{ field }">
                <HInput
                  :model-value="field.state.value"
                  label="分组名（对外 model）"
                  @update:model-value="field.handleChange"
                />
              </template>
            </form.Field>
            <form.Field name="thinking_effort">
              <template #default="{ field }">
                <label class="text-sm">
                  <span class="mb-1 block text-slate-600">思考强度</span>
                  <HSelect
                    :options="thinkingEffortOptions"
                    :model-value="field.state.value"
                    @update:model-value="(v) => field.handleChange(v as ThinkingEffort)"
                  />
                  <span class="mt-1 block text-xs text-slate-500">
                    代理转发时按上游模型家族翻译为对应字段；客户端自带则不覆盖。Claude 需自备足够 max_tokens。
                  </span>
                </label>
              </template>
            </form.Field>
            <form.Field name="source_provider_id">
              <template #default="{ field }">
                <label class="text-sm">
                  <span class="mb-1 block text-slate-600">绑定供应商自动同步</span>
                  <HSelect
                    :options="bindProviderOptions"
                    :model-value="field.state.value || 0"
                    @update:model-value="(v) => field.handleChange(Number(v))"
                  />
                  <span class="mt-1 block text-xs text-slate-500">
                    绑定后分组由选定的供应商托管，后台每 24h 自动全量同步其模型列表。
                  </span>
                </label>
              </template>
            </form.Field>
          </div>

          <div class="mt-4 space-y-2">
            <div class="flex flex-wrap items-center justify-between gap-2">
              <h3 class="text-sm font-medium">故障转移队列</h3>
              <div v-if="!isBound" class="flex flex-wrap items-center gap-3">
                <HButton
                  variant="ghost"
                  size="sm"
                  type="button"
                  :disabled="formValues.items.length < 2 || leaderboardLoading"
                  @click="sortQueueByCapability"
                >
                  按模型能力排序
                </HButton>
                <HButton
                  variant="ghost"
                  size="sm"
                  type="button"
                  :disabled="leaderboardLoading"
                  @click="loadLeaderboard(true)"
                >
                  {{ leaderboardLoading ? "刷新榜单中…" : "强制刷新榜单" }}
                </HButton>
                <HButton variant="ghost" size="sm" type="button" @click="addItem">添加条目</HButton>
              </div>
            </div>
            <p v-if="!isBound" class="text-xs text-slate-500">{{ leaderboardStatusText }}</p>
            <div v-if="isBound" class="rounded-lg border border-violet-100 bg-violet-50/60 p-3">
              <div class="flex items-center justify-between gap-2">
                <div class="space-y-1">
                  <p class="text-sm text-violet-800">
                    本分组由供应商托管，每 24h 自动同步，模型列表只读。
                  </p>
                  <p class="text-xs text-violet-600">
                    上次同步：{{ boundLastSyncText }}
                  </p>
                </div>
                <HButton v-if="isEditing" variant="outline" size="sm" type="button" :disabled="saving" @click="handleSyncNow">
                  立即同步
                </HButton>
              </div>
            </div>
            <div
              v-else
              class="flex flex-wrap items-end gap-2 rounded-lg border border-cyan-100 bg-cyan-50/60 p-3"
            >
              <label class="text-sm">
                <span class="mb-1 block text-slate-600">批量添加供应商全部模型</span>
                <HSelect
                  class="min-w-48"
                  :options="providerSelectOptions"
                  :model-value="bulkProviderId"
                  @update:model-value="(v) => (bulkProviderId = Number(v))"
                />
              </label>
              <HButton
                variant="outline"
                size="sm"
                type="button"
                :disabled="!bulkProviderId || bulkAddingModels"
                @click="bulkAddProviderModels"
              >
                {{ bulkAddingModels ? "拉取添加中…" : "拉取并全部添加" }}
              </HButton>
              <span class="pb-1 text-xs text-slate-500">按供应商 + 模型名去重，仅修改当前表单。</span>
            </div>
            <p v-if="bulkMessage" class="text-sm text-emerald-700">{{ bulkMessage }}</p>
            <p v-if="!isBound" class="text-xs text-slate-500">
              可拖动左侧手柄调整故障转移优先级；上移/下移与「按模型能力排序」仅作用于当前表单，需点保存写入。外部分数为 OpenRouter 公开智能指标。
            </p>
            <div
              v-for="(item, index) in formValues.items"
              :key="item.uid"
              class="flex flex-wrap items-center gap-2 rounded-lg border p-3 transition"
              :class="
                dragOverIndex === index
                  ? 'border-cyan-400 bg-cyan-50'
                  : dragFromIndex === index
                    ? 'border-slate-300 bg-slate-50 opacity-80'
                    : 'border-slate-200 bg-white'
              "
              @dragover="!isBound && onDragOver(index, $event)"
              @drop="!isBound && onDrop(index, $event)"
            >
              <button
                v-if="!isBound"
                type="button"
                class="cursor-grab select-none rounded border border-slate-200 bg-slate-50 px-2 py-1 text-xs text-slate-500 active:cursor-grabbing"
                title="拖动排序"
                draggable="true"
                @dragstart="onDragStart(index, $event)"
                @dragend="onDragEnd"
              >
                ⋮⋮
              </button>
              <span class="w-8 text-xs text-slate-400">#{{ index + 1 }}</span>
              <span
                class="rounded-full px-2 py-0.5 text-[11px] tabular-nums"
                :class="
                  queueDisplayScores[index]
                    ? 'bg-emerald-50 text-emerald-800'
                    : 'bg-slate-100 text-slate-500'
                "
                :title="
                  queueDisplayScores[index]
                    ? `OpenRouter 分数 ${queueDisplayScores[index]?.score}（匹配层级：${queueDisplayScores[index]?.tier}）`
                    : '未匹配到 OpenRouter 榜单数据'
                "
              >
                <template v-if="queueDisplayScores[index]">
                  OpenRouter · {{ queueDisplayScores[index]?.score }}
                </template>
                <template v-else>未匹配</template>
              </span>
              <HSelect
                :disabled="isBound"
                :options="providerSelectOptions"
                :model-value="item.provider_id"
                @update:model-value="(v) => updateItemAt(index, { provider_id: Number(v) })"
              />
              <div class="flex min-w-[200px] flex-1 flex-col gap-1">
                <div class="flex flex-wrap items-center gap-2">
                  <input
                    :disabled="isBound"
                    :value="item.upstream_model"
                    :list="`upstream-models-${index}`"
                    placeholder="上游模型名"
                    class="min-w-[160px] flex-1 rounded border border-slate-300 px-2 py-1 text-sm disabled:bg-slate-50 disabled:text-slate-500"
                    @input="
                      updateItemAt(index, {
                        upstream_model: ($event.target as HTMLInputElement).value,
                      })
                    "
                  />
                  <datalist :id="`upstream-models-${index}`">
                    <option v-for="mid in modelOptions[index] || []" :key="mid" :value="mid" />
                  </datalist>
                  <HButton
                    v-if="!isBound"
                    variant="outline"
                    size="sm"
                    type="button"
                    class="shrink-0"
                    :disabled="!item.provider_id || fetchingModels[index]"
                    @click="pullModels(index)"
                  >
                    {{ fetchingModels[index] ? "拉取中…" : "拉取模型" }}
                  </HButton>
                </div>
                <div
                  v-if="modelOptions[index]?.length && !isBound"
                  class="flex max-h-28 flex-wrap gap-1 overflow-y-auto"
                >
                  <button
                    v-for="mid in modelOptions[index]"
                    :key="mid"
                    type="button"
                    class="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[11px] text-slate-700 hover:bg-cyan-100"
                    :title="mid"
                    @click="pickModel(index, mid)"
                  >
                    {{ mid }}
                  </button>
                </div>
              </div>
              <template v-if="!isBound">
                <HButton variant="ghost" size="sm" type="button" @click="moveItem(index, -1)">
                  上移
                </HButton>
                <HButton variant="ghost" size="sm" type="button" @click="moveItem(index, 1)">
                  下移
                </HButton>
                <HButton variant="danger-soft" size="sm" type="button" @click="removeItem(index)">
                  删除
                </HButton>
              </template>
            </div>
          </div>

          <div class="mt-4 flex gap-2">
            <HButton variant="primary" type="submit" :disabled="saving">
              {{ saving ? "保存中…" : isEditing ? "保存修改" : "创建分组" }}
            </HButton>
            <HButton variant="outline" type="button" :disabled="saving" @click="closeDialog">
              取消
            </HButton>
          </div>
        </form>
        <p v-if="error" class="mt-3 text-sm text-rose-600">{{ error }}</p>
      </section>
    </AppDialog>

    <HCard variant="outlined" padding="md" class="min-h-0 flex-1 flex flex-col">
      <template #header>
        <div class="flex items-center justify-between gap-2">
          <h2 class="text-base font-semibold">分组</h2>
          <HButton
            variant="ghost"
            size="sm"
            isIconOnly
            shape="circle"
            title="新建分组"
            aria-label="新建分组"
            type="button"
            @click="openCreate"
          >
            <Plus :size="18" aria-hidden="true" />
          </HButton>
        </div>
      </template>
      <p class="mb-3 shrink-0 text-xs text-slate-500">
        「配置到 Pi」会将该分组名写入本机
        <code class="rounded bg-slate-100 px-1">~/.pi/agent/models.json</code>
        的
        <code class="rounded bg-slate-100 px-1">model-hub</code>
        （固定占位 Key，无需客户端密钥）。
      </p>
      <p v-if="message" class="mb-3 shrink-0 whitespace-pre-line text-sm text-emerald-700">{{ message }}</p>
      <p v-if="error && !dialogOpen" class="mb-3 shrink-0 text-sm text-rose-600">{{ error }}</p>
      <HEmpty v-if="groups.length === 0" class="app-empty-compact shrink-0" title="暂无分组" />
      <div
        v-if="groups.length > 0"
        class="min-h-0 flex-1 overflow-y-auto pr-1"
      >
        <!-- 卡片网格：每个分组一张卡片，自上而下 含 标题/标签 → 数量概览 → 模型队列 → 操作区。
             octopus 风格：卡片本体分层次表达（border + 浅 bg），无策略 tab。 -->
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
          <article
            v-for="g in groups"
            :key="g.id"
            class="group-card flex flex-col rounded-xl border border-slate-200 bg-white p-4 transition
                   hover:border-cyan-300 hover:bg-cyan-50/30"
          >
            <!-- 头部：分组名 + 思考强度 + 自动同步标签 -->
            <div class="flex flex-wrap items-center gap-2">
              <span class="break-all text-base font-semibold text-slate-800">{{ g.name }}</span>
              <span
                v-if="g.thinking_effort && g.thinking_effort !== 'off'"
                class="rounded-full bg-violet-50 px-2 py-0.5 text-[11px] text-violet-700"
                title="思考强度档位"
              >
                思考 · {{ thinkingEffortLabels[g.thinking_effort] ?? g.thinking_effort }}
              </span>
              <HBadge v-if="g.source_provider_id" variant="default">自动同步</HBadge>
            </div>

            <!-- 概览条：模型数量 + 故障转移说明 -->
            <p class="mt-1 text-xs text-slate-500">
              {{ g.items.length }} 个模型 · 队列顺序即故障转移优先级
            </p>

            <!-- 模型队列：固定最高高度 + 滚动，超出截断 -->
            <ol class="mt-3 max-h-44 space-y-1.5 overflow-y-auto pr-1 text-sm">
              <li
                v-for="(item, idx) in g.items"
                :key="item.id"
                class="flex items-start gap-2 rounded-md px-1.5 py-1 text-slate-700 hover:bg-slate-50"
              >
                <span class="w-5 shrink-0 text-xs tabular-nums text-slate-400">{{ idx + 1 }}.</span>
                <div class="min-w-0 flex-1">
                  <span class="block truncate text-slate-600">
                    {{
                      providerMap.get(item.provider_id)?.name || item.provider_name || item.provider_id
                    }}
                  </span>
                  <span class="block truncate font-mono text-xs text-slate-500">{{ item.upstream_model }}</span>
                </div>
              </li>
            </ol>

            <!-- 操作区：卡片底部，吸附对齐 -->
            <div class="mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 border-t border-slate-100 pt-3">
              <HButton
                variant="outline"
                size="sm"
                type="button"
                :disabled="exportingPiId === g.id"
                @click="exportToPi(g.id)"
              >
                {{ exportingPiId === g.id ? "配置中…" : "配置到 Pi" }}
              </HButton>
              <HButton variant="ghost" size="sm" type="button" @click="startEdit(g)">编辑</HButton>
              <HButton
                variant="danger-soft"
                size="sm"
                type="button"
                class="ml-auto"
                @click="remove(g.id)"
              >
                删除
              </HButton>
            </div>
          </article>
        </div>
      </div>
    </HCard>
  </div>
</template>

<style scoped>
:deep(.h-card) {
  display: flex;
  flex-direction: column;
}

:deep(.h-card__body) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
</style>
