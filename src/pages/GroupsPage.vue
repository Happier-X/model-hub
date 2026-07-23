<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  createGroup,
  deleteGroup,
  exportGroupToPiAgent,
  extractInvokeError,
  fetchProviderModels,
  getModelLeaderboard,
  listGroups,
  listHealth,
  listProviders,
  updateGroup,
  type Group,
  type HealthSnapshot,
  type ModelLeaderboardSnapshot,
  type Provider,
} from "../api/tauri";
import HealthBadge from "../components/HealthBadge.vue";
import AppDialog from "../components/AppDialog.vue";
import { findHealth } from "../utils/health";
import {
  buildExternalScoreIndex,
  hybridSortKey,
  scoreModelCapability,
  sortByHybridCapability,
  sortByModelCapability,
  type ExternalSortMetric,
  type MatchedExternalScore,
  type QueueSortMode,
} from "../utils/modelCapability";
import { getGroupSaveMode } from "../utils/groupSaveMode";

const groups = ref<Group[]>([]);
const providers = ref<Provider[]>([]);
const health = ref<HealthSnapshot[]>([]);
const error = ref("");
const message = ref("");
/** 正在导出到 Pi 的分组 id */
const exportingPiId = ref<number | null>(null);
/** 稳定编辑目标 id；null 表示新建态。不得依赖列表对象引用。 */
const editingGroupId = ref<number | null>(null);
const isEditing = computed(() => editingGroupId.value !== null);
const editingGroupName = computed(() => {
  if (editingGroupId.value === null) return "";
  const g = groups.value.find((item) => item.id === editingGroupId.value);
  return g?.name ?? form.name;
});
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
const sortMode = ref<QueueSortMode>("local");
const leaderboard = ref<ModelLeaderboardSnapshot | null>(null);
const leaderboardLoading = ref(false);
const leaderboardError = ref("");
let nextItemUid = 1;

type QueueItemDraft = {
  uid: number;
  provider_id: number;
  upstream_model: string;
};

const form = reactive({
  name: "",
  auto_failover: true,
  items: [] as QueueItemDraft[],
});

function createQueueItem(providerId: number, upstreamModel: string): QueueItemDraft {
  return {
    uid: nextItemUid++,
    provider_id: providerId,
    upstream_model: upstreamModel,
  };
}

const providerMap = computed(() => new Map(providers.value.map((p) => [p.id, p])));

const externalIndex = computed(() => {
  if (sortMode.value === "local" || !leaderboard.value) return null;
  const metric: ExternalSortMetric =
    sortMode.value === "external_coding" ? "coding" : "intelligence";
  return buildExternalScoreIndex(leaderboard.value.models, metric);
});

const leaderboardStatusText = computed(() => {
  if (leaderboardLoading.value) return "榜单加载中…";
  if (!leaderboard.value) {
    return leaderboardError.value || "尚未加载外部榜单（使用外部排序时将自动拉取）";
  }
  const t = formatUnix(leaderboard.value.fetched_at_unix);
  const parts = [
    `来源 ${leaderboard.value.source === "openrouter" ? "OpenRouter" : leaderboard.value.source}`,
    `${leaderboard.value.models.length} 条`,
    `更新于 ${t}`,
  ];
  if (leaderboard.value.cache_hit) parts.push("缓存命中");
  if (leaderboard.value.stale) parts.push("陈旧缓存（网络失败已回退）");
  // 强制刷新失败但仍有旧快照时，补充错误提示
  if (leaderboardError.value) parts.push(`刷新失败：${leaderboardError.value}`);
  return parts.join(" · ");
});

/** 每条队列的展示分（按 index 缓存，避免模板内多次调用 displayScoreOf）。 */
const queueDisplayScores = computed(() =>
  form.items.map((item) => displayScoreOf(item.upstream_model)),
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
  if (sortMode.value === "local") return true;
  if (leaderboard.value && !leaderboardLoading.value) return true;
  await loadLeaderboard(false);
  return !!leaderboard.value;
}

async function refresh() {
  try {
    [groups.value, providers.value, health.value] = await Promise.all([
      listGroups(),
      listProviders(),
      listHealth(),
    ]);
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
  form.name = "";
  form.auto_failover = true;
  form.items = [];
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
  form.name = g.name;
  form.auto_failover = g.auto_failover;
  form.items = g.items.map((i) => createQueueItem(i.provider_id, i.upstream_model));
  modelOptions.value = {};
  fetchingModels.value = {};
  dragFromIndex.value = null;
  dragOverIndex.value = null;
  bulkMessage.value = "";
}

function addItem() {
  const first = providers.value[0];
  form.items.push(createQueueItem(first?.id ?? 0, "gpt-4o-mini"));
}

function reorderQueue(from: number, to: number) {
  if (
    from === to ||
    from < 0 ||
    to < 0 ||
    from >= form.items.length ||
    to >= form.items.length
  ) {
    return;
  }

  const nextItems = form.items.slice();
  const [movedItem] = nextItems.splice(from, 1);
  nextItems.splice(to, 0, movedItem);

  const indexOrder = form.items.map((_, i) => i);
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

  form.items.splice(0, form.items.length, ...nextItems);
  modelOptions.value = nextOptions;
  fetchingModels.value = nextFetching;
  bulkMessage.value = "队列顺序已调整，点击“保存”后生效";
}

function moveItem(index: number, delta: number) {
  reorderQueue(index, index + delta);
}

function removeItem(index: number) {
  form.items.splice(index, 1);
  const nextOptions: Record<number, string[]> = {};
  const nextFetching: Record<number, boolean> = {};
  form.items.forEach((_, newIndex) => {
    const oldIndex = newIndex >= index ? newIndex + 1 : newIndex;
    if (modelOptions.value[oldIndex]) {
      nextOptions[newIndex] = modelOptions.value[oldIndex];
    }
    if (fetchingModels.value[oldIndex]) {
      nextFetching[newIndex] = fetchingModels.value[oldIndex];
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

function capabilityOf(modelId: string) {
  return scoreModelCapability(modelId);
}

function displayScoreOf(modelId: string): {
  label: string;
  score: number;
  source: "local" | "openrouter";
  recognized: boolean;
  external: MatchedExternalScore | null;
} {
  const local = scoreModelCapability(modelId);
  if (sortMode.value !== "local" && externalIndex.value) {
    const { external } = hybridSortKey(modelId, externalIndex.value);
    if (external) {
      return {
        label: local.recognized ? local.label : "外部榜单",
        score: external.score,
        source: "openrouter",
        recognized: true,
        external,
      };
    }
  }
  return {
    label: local.label,
    score: local.score,
    source: "local",
    recognized: local.recognized,
    external: null,
  };
}

function applySortedItems(sorted: typeof form.items, message: string) {
  const oldIndexByUid = new Map(form.items.map((item, index) => [item.uid, index]));
  const nextOptions: Record<number, string[]> = {};
  const nextFetching: Record<number, boolean> = {};
  sorted.forEach((item, newIndex) => {
    const oldIndex = oldIndexByUid.get(item.uid);
    if (oldIndex === undefined) return;
    if (modelOptions.value[oldIndex]) nextOptions[newIndex] = modelOptions.value[oldIndex];
    if (fetchingModels.value[oldIndex]) nextFetching[newIndex] = fetchingModels.value[oldIndex];
  });

  form.items.splice(0, form.items.length, ...sorted);
  modelOptions.value = nextOptions;
  fetchingModels.value = nextFetching;
  dragFromIndex.value = null;
  dragOverIndex.value = null;
  bulkMessage.value = message;
}

async function sortQueueByCapability() {
  if (form.items.length < 2) {
    bulkMessage.value = "队列条目少于 2 条，无需排序";
    return;
  }

  if (sortMode.value !== "local") {
    const ok = await ensureLeaderboardForExternalSort();
    if (!ok) {
      bulkMessage.value =
        "外部榜单不可用，已保持当前顺序。可改用「本地启发式」排序，或检查网络后强制刷新榜单。";
      return;
    }
  }

  const before = form.items.map((item) => item.uid);
  const sorted =
    sortMode.value === "local"
      ? sortByModelCapability(form.items, (item) => item.upstream_model)
      : sortByHybridCapability(form.items, (item) => item.upstream_model, externalIndex.value);

  const after = sorted.map((item) => item.uid);
  if (before.every((uid, index) => uid === after[index])) {
    bulkMessage.value =
      sortMode.value === "local"
        ? "当前顺序已符合本地启发式排序"
        : "当前顺序已符合所选外部榜单排序（未匹配项已回退本地）";
    return;
  }

  const modeHint =
    sortMode.value === "local"
      ? "已按本地模型名启发式排序（分数越高越优先）；未识别模型排后。"
      : sortMode.value === "external_coding"
        ? "已按 OpenRouter 编码能力排序；未匹配或无分模型回退本地启发式。"
        : "已按 OpenRouter 通用能力排序；未匹配或无分模型回退本地启发式。";
  applySortedItems(sorted, `${modeHint}点击“保存”后生效，仍可拖拽微调。`);
}

async function pullModels(index: number) {
  const item = form.items[index];
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
  const item = form.items[index];
  if (!item) return;
  item.upstream_model = modelId;
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

    const existing = new Set(
      form.items.map((item) => `${item.provider_id}\u0000${item.upstream_model.trim()}`),
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
      form.items.push(createQueueItem(providerId, modelId));
      existing.add(key);
      added += 1;
    }
    error.value = "";
    bulkMessage.value = `已添加 ${added} 个模型${skipped > 0 ? `，跳过 ${skipped} 个重复或空模型` : ""}；点击“保存”后生效`;
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    bulkAddingModels.value = false;
  }
}

async function save() {
  if (saving.value) return;
  // 快照编辑 id，避免异步期间状态漂移误走 create
  const targetId = editingGroupId.value;
  const mode = getGroupSaveMode(targetId);
  saving.value = true;
  try {
    const payload = {
      name: form.name,
      auto_failover: form.auto_failover,
      items: form.items.filter((i) => i.provider_id > 0 && i.upstream_model.trim()),
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

onMounted(async () => {
  await refresh();
});
</script>

<template>
  <div class="space-y-6">
    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <div class="flex items-center justify-between gap-2">
        <h2 class="text-base font-semibold">分组管理</h2>
        <button type="button" class="rounded-lg bg-slate-800 px-4 py-2 text-sm text-white" @click="openCreate">新建分组</button>
      </div>
    </section>

    <AppDialog :open="dialogOpen" :title="isEditing ? '编辑分组' : '新建分组'" size="wide" :close-disabled="saving" @close="closeDialog">
    <section>
      <h2 class="sr-only">{{ isEditing ? "编辑分组" : "新建分组" }}</h2>
      <p v-if="isEditing" class="mb-2 text-sm text-cyan-800">
        正在编辑：{{ editingGroupName || form.name || `分组 #${editingGroupId}` }}
      </p>
      <p class="mb-3 text-sm text-slate-500">分组名 = 客户端 model；队列顺序即故障转移优先级。</p>
      <div class="grid gap-3 md:grid-cols-2">
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">分组名（对外 model）</span>
          <input v-model="form.name" class="w-full rounded-lg border border-slate-300 px-3 py-2" />
        </label>
        <label class="flex items-center gap-2 self-end text-sm">
          <input v-model="form.auto_failover" type="checkbox" />
          开启自动故障转移
        </label>
      </div>

      <div class="mt-4 space-y-2">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h3 class="text-sm font-medium">故障转移队列</h3>
          <div class="flex flex-wrap items-center gap-3">
            <label class="flex items-center gap-1.5 text-sm text-slate-600">
              <span class="text-slate-500">排序方式</span>
              <select
                v-model="sortMode"
                class="rounded border border-slate-300 bg-white px-2 py-1 text-sm"
              >
                <option value="local">本地启发式</option>
                <option value="external_intelligence">外部通用能力</option>
                <option value="external_coding">外部编码能力</option>
              </select>
            </label>
            <button
              type="button"
              class="text-sm text-cyan-700 hover:underline disabled:opacity-40"
              :disabled="form.items.length < 2 || leaderboardLoading"
              @click="sortQueueByCapability"
            >
              按模型能力排序
            </button>
            <button
              type="button"
              class="text-sm text-cyan-700 hover:underline disabled:opacity-40"
              :disabled="leaderboardLoading"
              @click="loadLeaderboard(true)"
            >
              {{ leaderboardLoading ? "刷新榜单中…" : "强制刷新榜单" }}
            </button>
            <button type="button" class="text-sm text-cyan-700 hover:underline" @click="addItem">添加条目</button>
          </div>
        </div>
        <p class="text-xs text-slate-500">{{ leaderboardStatusText }}</p>
        <div class="flex flex-wrap items-end gap-2 rounded-lg border border-cyan-100 bg-cyan-50/60 p-3">
          <label class="text-sm">
            <span class="mb-1 block text-slate-600">批量添加供应商全部模型</span>
            <select
              v-model.number="bulkProviderId"
              class="min-w-48 rounded border border-slate-300 bg-white px-2 py-1.5 text-sm"
            >
              <option :value="0">选择供应商</option>
              <option v-for="p in providers" :key="p.id" :value="p.id">{{ p.name }}</option>
            </select>
          </label>
          <button
            type="button"
            class="rounded border border-cyan-600 bg-white px-3 py-1.5 text-sm text-cyan-700 hover:bg-cyan-50 disabled:opacity-50"
            :disabled="!bulkProviderId || bulkAddingModels"
            @click="bulkAddProviderModels"
          >
            {{ bulkAddingModels ? "拉取添加中…" : "拉取并全部添加" }}
          </button>
          <span class="pb-1 text-xs text-slate-500">按供应商 + 模型名去重，仅修改当前表单。</span>
        </div>
        <p v-if="bulkMessage" class="text-sm text-emerald-700">{{ bulkMessage }}</p>
        <p class="text-xs text-slate-500">
          可拖动左侧手柄调整故障转移优先级；上移/下移与「按模型能力排序」仅作用于当前表单，需点保存写入。本地分来自模型名启发式；外部分为 OpenRouter 公开指标，未匹配回退本地。
        </p>
        <div
          v-for="(item, index) in form.items"
          :key="item.uid"
          class="flex flex-wrap items-center gap-2 rounded-lg border p-3 transition"
          :class="
            dragOverIndex === index
              ? 'border-cyan-400 bg-cyan-50'
              : dragFromIndex === index
                ? 'border-slate-300 bg-slate-50 opacity-80'
                : 'border-slate-200 bg-white'
          "
          @dragover="onDragOver(index, $event)"
          @drop="onDrop(index, $event)"
        >
          <button
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
              queueDisplayScores[index]?.source === 'openrouter'
                ? 'bg-emerald-50 text-emerald-800'
                : queueDisplayScores[index]?.recognized
                  ? 'bg-violet-50 text-violet-700'
                  : 'bg-slate-100 text-slate-500'
            "
            :title="
              queueDisplayScores[index]?.source === 'openrouter'
                ? `OpenRouter 分数 ${queueDisplayScores[index]?.score}（本地启发式 ${capabilityOf(item.upstream_model).score}）`
                : `本地启发式能力分 ${queueDisplayScores[index]?.score}`
            "
          >
            {{ queueDisplayScores[index]?.source === "openrouter" ? "OpenRouter" : "本地" }}
            ·
            {{ queueDisplayScores[index]?.label }}
            ·
            {{ queueDisplayScores[index]?.score }}
          </span>
          <select v-model.number="item.provider_id" class="rounded border border-slate-300 px-2 py-1 text-sm">
            <option :value="0">选择供应商</option>
            <option v-for="p in providers" :key="p.id" :value="p.id">{{ p.name }}</option>
          </select>
          <div class="flex min-w-[200px] flex-1 flex-col gap-1">
            <div class="flex flex-wrap items-center gap-2">
              <input
                v-model="item.upstream_model"
                :list="`upstream-models-${index}`"
                placeholder="上游模型名"
                class="min-w-[160px] flex-1 rounded border border-slate-300 px-2 py-1 text-sm"
              />
              <datalist :id="`upstream-models-${index}`">
                <option v-for="mid in modelOptions[index] || []" :key="mid" :value="mid" />
              </datalist>
              <button
                type="button"
                class="shrink-0 rounded border border-cyan-600 px-2 py-1 text-xs text-cyan-700 hover:bg-cyan-50 disabled:opacity-50"
                :disabled="!item.provider_id || fetchingModels[index]"
                @click="pullModels(index)"
              >
                {{ fetchingModels[index] ? "拉取中…" : "拉取模型" }}
              </button>
            </div>
            <div
              v-if="modelOptions[index]?.length"
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
          <button type="button" class="text-xs text-slate-600" @click="moveItem(index, -1)">上移</button>
          <button type="button" class="text-xs text-slate-600" @click="moveItem(index, 1)">下移</button>
          <button type="button" class="text-xs text-rose-600" @click="removeItem(index)">删除</button>
        </div>
      </div>

      <div class="mt-4 flex gap-2">
        <button
          type="button"
          class="rounded-lg bg-slate-800 px-4 py-2 text-sm text-white disabled:opacity-50"
          :disabled="saving"
          @click="save"
        >
          {{ saving ? "保存中…" : isEditing ? "保存修改" : "创建分组" }}
        </button>
        <button
          type="button"
          class="rounded-lg border border-slate-300 px-4 py-2 text-sm disabled:opacity-50"
          :disabled="saving"
          @click="closeDialog"
        >
          取消
        </button>
      </div>
      <p v-if="error" class="mt-3 text-sm text-rose-600">{{ error }}</p>
    </section>
    </AppDialog>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <div class="mb-4 flex flex-wrap items-center justify-between gap-2">
        <h2 class="text-base font-semibold">分组列表</h2>
      </div>
      <p class="mb-3 text-xs text-slate-500">
        「配置到 Pi」会将该分组名写入本机
        <code class="rounded bg-slate-100 px-1">~/.pi/agent/models.json</code>
        的
        <code class="rounded bg-slate-100 px-1">model-hub</code>
        （固定占位 Key，无需客户端密钥）。
      </p>
      <p v-if="message" class="mb-3 whitespace-pre-line text-sm text-emerald-700">{{ message }}</p>
      <p v-if="error && !dialogOpen" class="mb-3 text-sm text-rose-600">{{ error }}</p>
      <div v-if="groups.length === 0" class="text-sm text-slate-500">暂无分组</div>
      <div v-for="g in groups" :key="g.id" class="mb-4 rounded-lg border border-slate-100 p-4 last:mb-0">
        <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
          <div>
            <span class="font-semibold">{{ g.name }}</span>
            <span class="ml-2 text-xs text-slate-500">
              {{ g.auto_failover ? "自动故障转移：开" : "自动故障转移：关" }}
            </span>
          </div>
          <div class="space-x-2 text-sm">
            <button
              type="button"
              class="text-cyan-700 hover:underline disabled:opacity-50"
              :disabled="exportingPiId === g.id"
              @click="exportToPi(g.id)"
            >
              {{ exportingPiId === g.id ? "配置中…" : "配置到 Pi" }}
            </button>
            <button type="button" class="text-cyan-700 hover:underline" @click="startEdit(g)">编辑</button>
            <button type="button" class="text-rose-600 hover:underline" @click="remove(g.id)">删除</button>
          </div>
        </div>
        <ol class="space-y-2 text-sm">
          <li v-for="(item, idx) in g.items" :key="item.id" class="flex flex-wrap items-center gap-2 text-slate-700">
            <span class="text-slate-400">{{ idx + 1 }}.</span>
            <span>{{ providerMap.get(item.provider_id)?.name || item.provider_name || item.provider_id }}</span>
            <span class="font-mono text-xs text-slate-500">{{ item.upstream_model }}</span>
            <HealthBadge :snapshot="findHealth(health, item.provider_id)" />
          </li>
        </ol>
      </div>
    </section>
  </div>
</template>
