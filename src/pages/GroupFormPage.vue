<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ChevronDown } from "@lucide/vue";
import { useForm } from "@tanstack/vue-form";
import { HButton, HCard, HCell, HEmpty, HInput, HLoading, HSelect, HTag, type HSelectOption } from "happier-ui";
import {
  createGroup,
  extractInvokeError,
  getModelLeaderboard,
  listGroups,
  listProviders,
  updateGroup,
  type Group,
  type ModelLeaderboardSnapshot,
  type Provider,
  type ThinkingEffort,
} from "../api/tauri";
import { useProviderModelCache } from "../composables/useProviderModelCache";
import {
  buildExternalScoreIndex,
  matchModelToLeaderboard,
  sortQueueByLeaderboard,
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
};

const defaultFormValues: GroupFormValues = {
  name: "",
  thinking_effort: "off",
  items: [],
};

const thinkingEffortOptions: HSelectOption[] = [
  { value: "off", label: "关闭（不注入）" },
  { value: "auto", label: "自动最佳" },
  { value: "minimal", label: "最小" },
  { value: "low", label: "低" },
  { value: "medium", label: "中" },
  { value: "high", label: "高" },
];

const route = useRoute();
const router = useRouter();

/** 新建页固定 `/groups/new`；编辑页从路由参数取 id。 */
const isCreate = computed(() => route.name === "groups-new");
const editingGroupId = computed<number | null>(() => {
  if (isCreate.value) return null;
  const raw = route.params.id;
  const n = typeof raw === "string" ? Number(raw) : NaN;
  return Number.isInteger(n) && n > 0 ? n : null;
});
const isEditing = computed(() => editingGroupId.value !== null);

const providers = ref<Provider[]>([]);
const error = ref("");
const saving = ref(false);
/** 编辑页加载中（避免闪空表单）；新建页初始即就绪 */
const loading = ref(isEditing.value);
/** 编辑页加载失败（分组不存在或列表加载失败），展示错误 + 返回入口 */
const loadFailed = ref(false);
/** 双栏：左侧供应商模型缓存（仅展开/刷新触发拉取，禁止预拉） */
const modelCache = useProviderModelCache();
const expandedProviders = ref<Set<number>>(new Set());
const leftFilter = ref("");
/** 表单内提示（排序/全部加入/同步反馈） */
const formMessage = ref("");

const leaderboard = ref<ModelLeaderboardSnapshot | null>(null);
const leaderboardLoading = ref(false);
const leaderboardError = ref("");
let nextItemUid = 1;

const form = useForm({
  defaultValues: {
    name: defaultFormValues.name,
    thinking_effort: defaultFormValues.thinking_effort,
    items: [] as QueueItemDraft[],
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
      };
      if (mode === "update" && targetId !== null) {
        await updateGroup({ id: targetId, ...payload });
      } else {
        await createGroup(payload);
      }
      await router.push({ name: "groups" });
    } catch (e) {
      // 失败保留编辑态与表单，便于重试
      error.value = extractInvokeError(e);
    } finally {
      saving.value = false;
    }
  },
});

/** 订阅表单 values，供队列操作与模板读取 */
const formValues = form.useSelector((s) => s.values);

const editingGroupName = computed(() => {
  if (!isEditing.value) return "";
  return formValues.value.name;
});

const providerMap = computed(() => new Map(providers.value.map((p) => [p.id, p])));

function providerName(providerId: number, fallbackName?: string): string {
  return providerMap.value.get(providerId)?.name || fallbackName || String(providerId);
}

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

/** 已选队列的去重 key：provider_id + upstream_model。 */
const selectedItemKeys = computed(
  () => new Set(formValues.value.items.map((i) => `${i.provider_id}\u0000${i.upstream_model.trim()}`)),
);

const externalIndex = computed<LeaderboardIndex | null>(() => {
  if (!leaderboard.value) return null;
  return buildExternalScoreIndex(leaderboard.value.models);
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
  } catch (e) {
    leaderboardError.value = extractInvokeError(e);
  } finally {
    leaderboardLoading.value = false;
  }
}

async function ensureLeaderboardForExternalSort() {
  if (leaderboard.value && !leaderboardLoading.value) return true;
  await loadLeaderboard(false);
  return !!leaderboard.value;
}

async function refreshProviders() {
  try {
    providers.value = await listProviders();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

/** 编辑态：加载分组列表并按 id 定位。找不到给出错误 + 返回入口。 */
async function loadEditingGroup(targetId: number) {
  loading.value = true;
  loadFailed.value = false;
  error.value = "";
  try {
    const groups = await listGroups();
    const g = groups.find((item) => item.id === targetId);
    if (!g) {
      error.value = `分组 #${targetId} 不存在或已被删除。`;
      loadFailed.value = true;
      return;
    }
    applyGroupToForm(g);
  } catch (e) {
    error.value = extractInvokeError(e);
    loadFailed.value = true;
  } finally {
    loading.value = false;
  }
}

function applyGroupToForm(g: Group) {
  form.reset({
    name: g.name,
    thinking_effort: g.thinking_effort,
    items: g.items.map((i) => createQueueItem(i.provider_id, i.upstream_model)),
  });
  expandedProviders.value = new Set();
  leftFilter.value = "";
  dragFromIndex.value = null;
  dragOverIndex.value = null;
}

function resetFormToDefault() {
  form.reset({ name: "", thinking_effort: "off", items: [] });
  expandedProviders.value = new Set();
  leftFilter.value = "";
  dragFromIndex.value = null;
  dragOverIndex.value = null;
  formMessage.value = "";
  error.value = "";
}

onMounted(async () => {
  await refreshProviders();
  if (route.name !== "groups-edit") {
    // 新建：直接展示空表单
    resetFormToDefault();
    return;
  }
  const targetId = editingGroupId.value;
  if (targetId === null) {
    // 编辑路由但 id 非法（如 0）：明确报错 + 返回入口，绝不落回新建态
    error.value = "分组 id 无效，无法编辑。";
    loadFailed.value = true;
    loading.value = false;
    return;
  }
  await loadEditingGroup(targetId);
});

/** 取消 / 返回列表：未保存直接丢弃，不做二次确认。 */
function goBack() {
  if (saving.value) return;
  void router.push({ name: "groups" });
}

// ---------------------------------------------------------------------------
// 双栏：左侧供应商手风琴
// ---------------------------------------------------------------------------

function toggleProvider(providerId: number) {
  const next = new Set(expandedProviders.value);
  if (next.has(providerId)) {
    next.delete(providerId);
  } else {
    next.add(providerId);
    // D4=L1：首次展开才拉取；已缓存则直接展示
    void modelCache.ensure(providerId).catch(() => {});
  }
  expandedProviders.value = next;
}

function addModelFromLeft(providerId: number, modelId: string) {
  const m = modelId.trim();
  if (!m) return;
  if (selectedItemKeys.value.has(`${providerId}\u0000${m}`)) return;
  setItems([...formValues.value.items, createQueueItem(providerId, m)]);
}

async function addAllFromProvider(providerId: number) {
  let models = modelCache.getModels(providerId);
  if (models.length === 0) {
    try {
      models = await modelCache.ensure(providerId);
    } catch {
      return;
    }
  }
  const items = formValues.value.items.slice();
  const existing = new Set(items.map((i) => `${i.provider_id}\u0000${i.upstream_model.trim()}`));
  let added = 0;
  for (const raw of models) {
    const m = raw.trim();
    if (!m) continue;
    const key = `${providerId}\u0000${m}`;
    if (existing.has(key)) continue;
    items.push(createQueueItem(providerId, m));
    existing.add(key);
    added += 1;
  }
  setItems(items);
  formMessage.value =
    added > 0
      ? `已加入 ${added} 个模型，点击“保存”后生效`
      : "队列已包含该供应商全部模型";
}

/** 左侧已加载模型：关键词过滤 + 已选剔除。 */
function filteredProviderModels(providerId: number): string[] {
  const models = modelCache.getModels(providerId);
  const kw = leftFilter.value.trim().toLowerCase();
  return models.filter((m) => {
    if (selectedItemKeys.value.has(`${providerId}\u0000${m.trim()}`)) return false;
    return !kw || m.toLowerCase().includes(kw);
  });
}

// ---------------------------------------------------------------------------
// 双栏：右侧队列
// ---------------------------------------------------------------------------

const dragFromIndex = ref<number | null>(null);
const dragOverIndex = ref<number | null>(null);

function reorderQueue(from: number, to: number) {
  const items = formValues.value.items;
  if (from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) return;
  const nextItems = items.slice();
  const [movedItem] = nextItems.splice(from, 1);
  nextItems.splice(to, 0, movedItem);
  setItems(nextItems);
}

function removeQueueItem(index: number) {
  setItems(formValues.value.items.filter((_, i) => i !== index));
}

function clearQueue() {
  setItems([]);
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
  setItems(sorted);
  dragFromIndex.value = null;
  dragOverIndex.value = null;
  formMessage.value = msg;
}

async function sortQueueByCapability() {
  const items = formValues.value.items;
  if (items.length < 2) {
    formMessage.value = "队列条目少于 2 条，无需排序";
    return;
  }

  const ok = await ensureLeaderboardForExternalSort();
  if (!ok) {
    formMessage.value = "外部榜单不可用，已保持当前顺序。请检查网络后重试。";
    return;
  }

  const before = items.map((item) => item.uid);
  const sorted = sortQueueByLeaderboard(items, (item) => item.upstream_model, externalIndex.value);

  const after = sorted.map((item) => item.uid);
  if (before.every((uid, index) => uid === after[index])) {
    formMessage.value = "当前顺序已符合 llm_benchmark 榜单排序（未匹配项保持原序）";
    return;
  }

  applySortedItems(sorted, "已按 llm_benchmark 综合能力排序；未匹配项已沉底。点击“保存”后生效，仍可拖拽微调。");
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
    <!-- 顶部：说明 + 返回列表 -->
    <div class="flex flex-wrap items-center justify-between gap-2">
      <p class="text-sm text-slate-500">
        <span v-if="isEditing" class="mr-1 font-medium text-cyan-800">
          正在编辑：{{ editingGroupName || `分组 #${editingGroupId}` }}
        </span>
        分组名 = 客户端 model；队列顺序即故障转移优先级。
      </p>
      <HButton
        variant="ghost"
        size="sm"
        type="button"
        :disabled="saving"
        @click="goBack"
      >
        返回列表
      </HButton>
    </div>

    <!-- 编辑态加载中 -->
    <HLoading
      v-if="loading"
      mode="local"
      label="正在加载分组…"
      class="py-6"
    />

    <!-- 编辑态加载失败：分组不存在 / 加载失败 -->
    <HCard
      v-else-if="loadFailed"
      variant="outlined"
      class="border-rose-200 bg-rose-50"
    >
      <p class="text-sm text-rose-700">{{ error }}</p>
      <HButton
        variant="outline"
        size="sm"
        type="button"
        class="mt-3"
        @click="goBack"
      >
        返回列表
      </HButton>
    </HCard>

    <div v-else class="flex min-h-0 flex-1 flex-col gap-4">
      <form class="flex min-h-0 flex-1 flex-col gap-4" @submit.prevent="form.handleSubmit()">
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
                  代理转发时按上游模型家族翻译为对应字段；客户端自带则不覆盖。
                </span>
              </label>
            </template>
          </form.Field>
        </div>

        <p v-if="formMessage" class="text-sm text-emerald-700">{{ formMessage }}</p>

        <!-- 双栏：左可选模型 / 右已选队列（flex 而非 grid：grid item 上 flex-1 不生效，会回退到内容高度导致整页滚动） -->
        <div class="flex min-h-0 flex-1 flex-col gap-4 lg:flex-row">
          <!-- 左：按供应商手风琴选模 -->
          <HCard variant="outlined" padding="none" class="flex min-h-0 flex-1 flex-col">
            <template #header>
              <div class="flex items-center justify-between px-3 py-2">
                <h3 class="text-sm font-medium">可选模型</h3>
                <span class="text-xs text-slate-400">展开供应商以加载其模型</span>
              </div>
            </template>
            <div class="min-h-0 flex-1 space-y-2 overflow-y-auto p-3">
              <div
                v-for="p in providers"
                :key="p.id"
                class="rounded-lg border border-slate-200"
              >
                <HCell
                  clickable
                  :show-chevron="false"
                  :title="p.name"
                  @click="toggleProvider(p.id)"
                >
                  <template #prefix>
                    <ChevronDown
                      :size="14"
                      class="text-slate-400 transition-transform"
                      :class="{ '-rotate-90': !expandedProviders.has(p.id) }"
                    />
                  </template>
                  <template #suffix>
                    <!-- 模型已加载显示数量；未加载显示同步状态（数据来自 list_providers 返回的 last_sync_at） -->
                    <span v-if="modelCache.getStatus(p.id) === 'ready'" class="text-xs text-slate-400">
                      {{ modelCache.getModels(p.id).length }} 个模型
                    </span>
                    <span v-else class="text-xs text-slate-400">
                      {{ p.last_sync_at ? `已同步 ${formatUnix(p.last_sync_at)}` : "未同步" }}
                    </span>
                  </template>
                </HCell>

                <div v-if="expandedProviders.has(p.id)" class="border-t border-slate-100 px-3 py-2">
                  <div v-if="modelCache.getStatus(p.id) === 'loading'" class="py-2">
                    <HLoading mode="local" size="sm" label="正在拉取模型…" />
                  </div>
                  <div v-else-if="modelCache.getStatus(p.id) === 'error'" class="py-2">
                    <p class="text-xs text-rose-600">{{ modelCache.getError(p.id) }}</p>
                    <HButton
                      variant="ghost"
                      size="sm"
                      type="button"
                      class="mt-1"
                      @click="modelCache.refresh(p.id)"
                    >
                      重试
                    </HButton>
                  </div>
                  <template v-else>
                    <HInput
                      v-model="leftFilter"
                      placeholder="过滤该供应商已加载模型"
                      class="mb-2"
                    />
                    <HEmpty v-if="modelCache.getModels(p.id).length === 0" class="app-empty-compact" title="上游未返回模型" />
                    <div
                      v-else
                      class="flex max-h-56 flex-col gap-1 overflow-y-auto"
                    >
                      <button
                        v-for="m in filteredProviderModels(p.id)"
                        :key="m"
                        type="button"
                        class="rounded bg-slate-100 px-2 py-1 text-left font-mono text-xs text-slate-700 hover:bg-cyan-100 disabled:opacity-50"
                        :title="m"
                        @click="addModelFromLeft(p.id, m)"
                      >
                        {{ m }}
                      </button>
                      <p v-if="filteredProviderModels(p.id).length === 0" class="py-1 text-xs text-slate-400">
                        无匹配（已选或关键词过滤）
                      </p>
                    </div>
                    <div class="mt-2 flex justify-end">
                      <HButton
                        variant="outline"
                        size="sm"
                        type="button"
                        @click="addAllFromProvider(p.id)"
                      >
                        全部加入
                      </HButton>
                    </div>
                  </template>
                </div>
              </div>
              <HEmpty v-if="providers.length === 0" class="app-empty-compact" title="暂无供应商，请先到「供应商」页添加" />
            </div>
          </HCard>

          <!-- 右：已选故障转移队列 -->
          <HCard variant="outlined" padding="none" class="flex min-h-0 flex-1 flex-col">
            <template #header>
              <div class="flex items-center justify-between px-3 py-2">
                <h3 class="text-sm font-medium">故障转移队列</h3>
                <span class="flex items-center gap-1">
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
                    :disabled="formValues.items.length === 0"
                    @click="clearQueue"
                  >
                    清空
                  </HButton>
                </span>
              </div>
            </template>
            <div class="min-h-0 flex-1 space-y-1 overflow-y-auto p-3">
              <div
                v-for="(item, index) in formValues.items"
                :key="item.uid"
                class="flex items-center gap-1.5 rounded-md border border-slate-200 px-2 py-1.5"
                :class="
                  dragOverIndex === index
                    ? 'border-cyan-400 bg-cyan-50'
                    : dragFromIndex === index
                      ? 'border-slate-300 bg-slate-50 opacity-80'
                      : 'bg-white'
                "
                @dragover="onDragOver(index, $event)"
                @drop="onDrop(index, $event)"
              >
                <button
                  type="button"
                  class="cursor-grab select-none rounded border border-slate-200 bg-slate-50 px-1 py-0.5 text-[10px] text-slate-500 active:cursor-grabbing"
                  title="拖动排序"
                  :draggable="!saving"
                  @dragstart="onDragStart(index, $event)"
                  @dragend="onDragEnd"
                >
                  ⋮⋮
                </button>
                <span class="w-5 shrink-0 text-xs tabular-nums text-slate-400">{{ index + 1 }}.</span>
                <div class="min-w-0 flex-1">
                  <span class="block truncate text-xs text-slate-600">
                    {{ providerName(item.provider_id) }}
                  </span>
                  <span class="block truncate font-mono text-xs text-slate-500">{{ item.upstream_model }}</span>
                </div>
                <HTag
                  size="sm"
                  :variant="queueDisplayScores[index] ? 'success' : 'default'"
                  :title="
                    queueDisplayScores[index]
                      ? `llm_benchmark 分数 ${queueDisplayScores[index]?.score}（匹配层级：${queueDisplayScores[index]?.tier}）`
                      : '未匹配到 llm_benchmark 榜单数据'
                  "
                >
                  <template v-if="queueDisplayScores[index]">
                    llm_benchmark · {{ queueDisplayScores[index]?.score }}
                  </template>
                  <template v-else>未匹配</template>
                </HTag>
                <HButton
                  variant="ghost"
                  size="sm"
                  type="button"
                  class="shrink-0 text-rose-600 hover:bg-rose-50 hover:text-rose-700"
                  title="删除成员"
                  @click="removeQueueItem(index)"
                >
                  ×
                </HButton>
              </div>
              <HEmpty
                v-if="formValues.items.length === 0"
                class="app-empty-compact"
                title="队列为空：从左侧选择模型加入"
              />
            </div>
          </HCard>
        </div>

        <div class="flex gap-2">
          <HButton variant="primary" type="submit" :disabled="saving">
            {{ saving ? "保存中…" : isEditing ? "保存修改" : "创建分组" }}
          </HButton>
          <HButton variant="outline" type="button" :disabled="saving" @click="goBack">
            取消
          </HButton>
        </div>
        <p v-if="error" class="text-sm text-rose-600">{{ error }}</p>
      </form>
    </div>
  </div>
</template>

<style scoped>
/* 让 HCard (.h-card) 内部 .h-card__body slot 容器参与 flex 列布局并撑满高度，
   否则双栏内部 overflow-y-auto 滚动区的 flex-1 失效，内容被 max-h 截断无法滚动。 */
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
