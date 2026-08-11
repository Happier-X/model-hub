<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ChevronDown } from "@lucide/vue";
import { useForm } from "@tanstack/vue-form";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Empty } from "@/components/ui/empty";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Item, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import {
  createGroup,
  extractInvokeError,
  getModelLeaderboard,
  listGroups,
  listProviders,
  setProviderAutoSync,
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

const thinkingEffortOptions: { value: ThinkingEffort; label: string }[] = [
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
/** 行内自动同步开关进行中的 id 集合（disabled 防重复点击） */
const autoSyncTogglingIds = ref<Set<number>>(new Set());
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
      const payload = buildGroupPayload(value);
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

/** 表单值 → 后端分组 payload（onSubmit 与排序自动保存共用，避免漂移）。 */
function buildGroupPayload(value: { name: string; thinking_effort: ThinkingEffort; items: QueueItemDraft[] }) {
  return {
    name: value.name,
    thinking_effort: value.thinking_effort,
    items: value.items.filter((i) => i.provider_id > 0 && i.upstream_model.trim()),
  };
}

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

// 行内自动同步开关：乐观更新本地 -> 调后端就地切换 -> 以返回值为准同步 -> 失败回滚并报错
async function toggleProviderAutoSync(p: Provider, next: boolean) {
  if (autoSyncTogglingIds.value.has(p.id)) return;
  const previous = p.auto_sync;
  // 乐观更新
  const target = providers.value.find((it) => it.id === p.id);
  if (target) target.auto_sync = next;
  autoSyncTogglingIds.value = new Set(autoSyncTogglingIds.value).add(p.id);
  try {
    const updated = await setProviderAutoSync(p.id, next);
    // 以服务端返回为准同步
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

  applySortedItems(sorted, "");
  if (isEditing.value) {
    // 编辑态：自动落库并留在页面供继续拖拽微调
    await autoSaveAfterSort();
  } else {
    // 新建态：无分组 id，不自动创建；提示保存后生效
    formMessage.value = "已按 llm_benchmark 综合能力排序；未匹配项已沉底。点击“保存”后生效，仍可拖拽微调。";
  }
}

/** 排序成功后自动保存（仅编辑态调用）：updateGroup 落库，不跳转。 */
async function autoSaveAfterSort(): Promise<boolean> {
  if (saving.value) return false;
  const targetId = editingGroupId.value;
  if (targetId === null) return false;
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
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
    <!-- 顶部：说明 + 返回列表 -->
    <div class="flex flex-wrap items-center justify-between gap-2">
      <p class="text-sm text-muted-foreground">
        <span v-if="isEditing" class="mr-1 font-medium text-info">
          正在编辑：{{ editingGroupName || `分组 #${editingGroupId}` }}
        </span>
        分组名 = 客户端 model；队列顺序即故障转移优先级。
      </p>
      <Button
        variant="ghost"
        size="sm"
        type="button"
        :disabled="saving"
        @click="goBack"
      >
        返回列表
      </Button>
    </div>

    <!-- 编辑态加载中 -->
    <div v-if="loading" class="flex items-center gap-2 py-6 text-sm text-muted-foreground">
      <Spinner class="size-4" />
      正在加载分组…
    </div>

    <!-- 编辑态加载失败：分组不存在 / 加载失败 -->
    <Card v-else-if="loadFailed" class="border-destructive/20 bg-destructive/10">
      <CardContent class="py-4">
        <p class="text-sm text-destructive">{{ error }}</p>
        <Button
          variant="outline"
          size="sm"
          type="button"
          class="mt-3"
          @click="goBack"
        >
          返回列表
        </Button>
      </CardContent>
    </Card>

    <div v-else class="flex min-h-0 flex-1 flex-col gap-4">
      <form class="flex min-h-0 flex-1 flex-col gap-4" @submit.prevent="form.handleSubmit()">
        <div class="grid gap-3 md:grid-cols-2">
          <form.Field name="name">
            <template #default="{ field }">
              <Field>
                <FieldLabel>分组名（对外 model）</FieldLabel>
                <Input
                  :model-value="field.state.value"
                  @update:model-value="(v) => field.handleChange(v as string)"
                />
              </Field>
            </template>
          </form.Field>
          <form.Field name="thinking_effort">
            <template #default="{ field }">
              <Field>
                <FieldLabel>思考强度</FieldLabel>
                <Select
                  :model-value="field.state.value"
                  @update:model-value="(v) => field.handleChange(v as ThinkingEffort)"
                >
                  <SelectTrigger class="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem v-for="opt in thinkingEffortOptions" :key="opt.value" :value="opt.value">
                      {{ opt.label }}
                    </SelectItem>
                  </SelectContent>
                </Select>
                <FieldDescription>
                  代理转发时按上游模型家族翻译为对应字段；客户端自带则不覆盖。
                </FieldDescription>
              </Field>
            </template>
          </form.Field>
        </div>

        <p v-if="formMessage" class="text-sm text-success">{{ formMessage }}</p>

        <!-- 双栏：左可选模型 / 右已选队列（flex 而非 grid：grid item 上 flex-1 不生效，会回退到内容高度导致整页滚动） -->
        <div class="flex min-h-0 flex-1 flex-col gap-4 lg:flex-row">
          <!-- 左：按供应商手风琴选模 -->
          <Card class="flex min-h-0 flex-1 flex-col">
            <CardHeader class="shrink-0 py-0">
              <div class="flex items-center justify-between px-3 py-2">
                <h3 class="text-sm font-medium">可选模型</h3>
                <span class="text-xs text-muted-foreground">展开供应商以加载其模型</span>
              </div>
            </CardHeader>
            <CardContent class="flex min-h-0 flex-1 flex-col p-0">
            <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-3">
              <div
                v-for="p in providers"
                :key="p.id"
                class="rounded-lg border border-border"
              >
                <Item
                  class="w-full cursor-pointer rounded-lg border border-transparent hover:bg-muted"
                  @click="toggleProvider(p.id)"
                >
                  <ChevronDown
                    class="shrink-0 size-3.5 text-muted-foreground transition-transform"
                    :class="{ '-rotate-90': !expandedProviders.has(p.id) }"
                  />
                  <ItemContent class="min-w-0 flex-1">
                    <ItemTitle>{{ p.name }}</ItemTitle>
                    <ItemDescription>
                      <!-- 自动同步开关：点击不展开手风琴（stop），仅切换 auto_sync -->
                      <div class="flex items-center gap-2" @click.stop>
                        <Switch
                          :model-value="p.auto_sync"
                          :disabled="autoSyncTogglingIds.has(p.id)"
                          :aria-label="`${p.name} 自动同步`"
                          title="自动同步"
                          @update:model-value="toggleProviderAutoSync(p, $event)"
                        />
                        <!-- 模型已加载显示数量；未加载显示同步状态（数据来自 list_providers 返回的 last_sync_at） -->
                        <span v-if="modelCache.getStatus(p.id) === 'ready'" class="text-xs text-muted-foreground">
                          {{ modelCache.getModels(p.id).length }} 个模型
                        </span>
                        <span v-else class="text-xs text-muted-foreground">
                          {{ p.last_sync_at ? `已同步 ${formatUnix(p.last_sync_at)}` : "未同步" }}
                        </span>
                      </div>
                    </ItemDescription>
                  </ItemContent>
                </Item>

                <div v-if="expandedProviders.has(p.id)" class="border-t border-border px-3 py-2">
                  <div v-if="modelCache.getStatus(p.id) === 'loading'" class="flex items-center gap-2 py-2 text-xs text-muted-foreground">
                    <Spinner class="size-3" />
                    正在拉取模型…
                  </div>
                  <div v-else-if="modelCache.getStatus(p.id) === 'error'" class="py-2">
                    <p class="text-xs text-destructive">{{ modelCache.getError(p.id) }}</p>
                    <Button
                      variant="ghost"
                      size="sm"
                      type="button"
                      class="mt-1"
                      @click="modelCache.refresh(p.id)"
                    >
                      重试
                    </Button>
                  </div>
                  <template v-else>
                    <Input
                      v-model="leftFilter"
                      placeholder="过滤该供应商已加载模型"
                      class="mb-2"
                    />
                    <Empty v-if="modelCache.getModels(p.id).length === 0" class="app-empty-compact" title="上游未返回模型" />
                    <div
                      v-else
                      class="flex max-h-56 flex-col gap-1 overflow-y-auto"
                    >
                      <button
                        v-for="m in filteredProviderModels(p.id)"
                        :key="m"
                        type="button"
                        class="rounded bg-muted px-2 py-1 text-left font-mono text-xs text-foreground hover:bg-info/15 disabled:opacity-50"
                        :title="m"
                        @click="addModelFromLeft(p.id, m)"
                      >
                        {{ m }}
                      </button>
                      <p v-if="filteredProviderModels(p.id).length === 0" class="py-1 text-xs text-muted-foreground">
                        无匹配（已选或关键词过滤）
                      </p>
                    </div>
                    <div class="mt-2 flex justify-end">
                      <Button
                        variant="outline"
                        size="sm"
                        type="button"
                        @click="addAllFromProvider(p.id)"
                      >
                        全部加入
                      </Button>
                    </div>
                  </template>
                </div>
              </div>
              <Empty v-if="providers.length === 0" class="app-empty-compact" title="暂无供应商，请先到「供应商」页添加" />
            </div>
            </CardContent>
          </Card>

          <!-- 右：已选故障转移队列 -->
          <Card class="flex min-h-0 flex-1 flex-col">
            <CardHeader class="shrink-0 py-0">
              <div class="flex items-center justify-between px-3 py-2">
                <h3 class="text-sm font-medium">故障转移队列</h3>
                <span class="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    type="button"
                    :disabled="formValues.items.length < 2 || leaderboardLoading"
                    @click="sortQueueByCapability"
                  >
                    按模型能力排序
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    type="button"
                    :disabled="formValues.items.length === 0"
                    @click="clearQueue"
                  >
                    清空
                  </Button>
                </span>
              </div>
            </CardHeader>
            <CardContent class="flex min-h-0 flex-1 flex-col p-0">
            <div class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto p-3">
              <div
                v-for="(item, index) in formValues.items"
                :key="item.uid"
                class="flex items-center gap-1.5 rounded-md border border-border px-2 py-1.5"
                :class="
                  dragOverIndex === index
                    ? 'border-info/40 bg-info/10'
                    : dragFromIndex === index
                      ? 'border-border bg-muted opacity-80'
                      : 'bg-card'
                "
                @dragover="onDragOver(index, $event)"
                @drop="onDrop(index, $event)"
              >
                <button
                  type="button"
                  class="cursor-grab select-none rounded border border-border bg-muted px-1 py-0.5 text-[10px] text-muted-foreground active:cursor-grabbing"
                  title="拖动排序"
                  :draggable="!saving"
                  @dragstart="onDragStart(index, $event)"
                  @dragend="onDragEnd"
                >
                  ⋮⋮
                </button>
                <span class="w-5 shrink-0 text-xs tabular-nums text-muted-foreground">{{ index + 1 }}.</span>
                <div class="min-w-0 flex-1">
                  <span class="block truncate text-xs text-muted-foreground">
                    {{ providerName(item.provider_id) }}
                  </span>
                  <span class="block truncate font-mono text-xs text-muted-foreground">{{ item.upstream_model }}</span>
                </div>
                <Badge
                  variant="outline"
                  :class="queueDisplayScores[index] ? 'border-success/20 bg-success/15 text-success' : ''"
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
                </Badge>
                <Button
                  variant="ghost"
                  size="sm"
                  type="button"
                  class="shrink-0 text-destructive hover:bg-destructive/10 hover:text-destructive"
                  title="删除成员"
                  @click="removeQueueItem(index)"
                >
                  ×
                </Button>
              </div>
              <Empty
                v-if="formValues.items.length === 0"
                class="app-empty-compact"
                title="队列为空：从左侧选择模型加入"
              />
            </div>
            </CardContent>
          </Card>
        </div>

        <div class="flex gap-2">
          <Button variant="default" type="submit" :disabled="saving">
            {{ saving ? "保存中…" : isEditing ? "保存修改" : "创建分组" }}
          </Button>
          <Button variant="outline" type="button" :disabled="saving" @click="goBack">
            取消
          </Button>
        </div>
        <p v-if="error" class="text-sm text-destructive">{{ error }}</p>
      </form>
    </div>
  </div>
</template>


