<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { Plus } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Empty } from "@/components/ui/empty";
import {
  deleteGroup,
  exportGroupToPiAgent,
  extractInvokeError,
  listGroups,
  listProviders,
  updateGroup,
  type Group,
  type GroupItem,
  type Provider,
  type ThinkingEffort,
} from "../api/tauri";
import GroupCard from "../components/groups/GroupCard.vue";

const router = useRouter();

const groups = ref<Group[]>([]);
const providers = ref<Provider[]>([]);
const error = ref("");
const message = ref("");
/** 正在导出到 Pi 的分组 id */
const exportingPiId = ref<number | null>(null);
/** 卡片即时保存中的分组 id 集合 */
const cardSavingIds = ref<Set<number>>(new Set());

const thinkingEffortLabels: Record<ThinkingEffort, string> = {
  off: "关闭",
  auto: "自动最佳",
  minimal: "最小",
  low: "低",
  medium: "中",
  high: "高",
};

const providerMap = computed(() => new Map(providers.value.map((p) => [p.id, p])));

function providerName(providerId: number, fallbackName?: string): string {
  return providerMap.value.get(providerId)?.name || fallbackName || String(providerId);
}

async function refresh() {
  try {
    [groups.value, providers.value] = await Promise.all([listGroups(), listProviders()]);
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

function openCreate() {
  void router.push({ name: "groups-new" });
}

function startEdit(g: Group) {
  void router.push({ name: "groups-edit", params: { id: String(g.id) } });
}

// ---------------------------------------------------------------------------
// 卡片内即时编辑
// ---------------------------------------------------------------------------

function persistGroupItems(group: Group, nextItems: GroupItem[]) {
  if (cardSavingIds.value.has(group.id)) return;
  cardSavingIds.value = new Set(cardSavingIds.value).add(group.id);
  error.value = "";
  const payload = {
    id: group.id,
    name: group.name,
    thinking_effort: group.thinking_effort,
    items: nextItems.map((i) => ({ provider_id: i.provider_id, upstream_model: i.upstream_model })),
  };
  void (async () => {
    try {
      const updated = await updateGroup(payload);
      // 以服务端返回为准替换本地，避免假成功
      const idx = groups.value.findIndex((g) => g.id === group.id);
      if (idx >= 0) {
        groups.value = [...groups.value.slice(0, idx), updated, ...groups.value.slice(idx + 1)];
      }
    } catch (e) {
      error.value = extractInvokeError(e);
      await refresh();
    } finally {
      cardSavingIds.value = new Set([...cardSavingIds.value].filter((id) => id !== group.id));
    }
  })();
}

async function removeGroup(id: number) {
  if (cardSavingIds.value.has(id)) return;
  cardSavingIds.value = new Set(cardSavingIds.value).add(id);
  error.value = "";
  try {
    await deleteGroup(id);
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
    await refresh();
  } finally {
    cardSavingIds.value = new Set([...cardSavingIds.value].filter((gid) => gid !== id));
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
  <div class="h-full flex flex-col overflow-hidden">
    <Card class="min-h-0 flex-1 flex flex-col">
      <CardHeader class="flex shrink-0 flex-row items-center justify-between gap-2 py-3">
        <h2 class="text-base font-semibold">分组</h2>
        <Button
          variant="ghost"
          size="icon"
          title="新建分组"
          aria-label="新建分组"
          type="button"
          @click="openCreate"
        >
          <Plus aria-hidden="true" />
        </Button>
      </CardHeader>
      <CardContent class="flex min-h-0 flex-1 flex-col gap-3">
        <p v-if="message" class="shrink-0 whitespace-pre-line text-sm text-success">{{ message }}</p>
        <p v-if="error" class="shrink-0 text-sm text-destructive">{{ error }}</p>
        <Empty v-if="groups.length === 0" class="app-empty-compact shrink-0" title="暂无分组" />
        <div v-if="groups.length > 0" class="min-h-0 flex-1 overflow-y-auto pr-1">
          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
            <GroupCard
              v-for="g in groups"
              :key="g.id"
              :group="g"
              :provider-name="providerName"
              :thinking-effort-labels="thinkingEffortLabels"
              :saving="cardSavingIds.has(g.id)"
              :exporting-pi="exportingPiId === g.id"
              @edit="startEdit(g)"
              @export-pi="exportToPi(g.id)"
              @delete-group="removeGroup(g.id)"
              @persist-items="persistGroupItems(g, $event)"
            />
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
