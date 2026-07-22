<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  createGroup,
  deleteGroup,
  extractInvokeError,
  fetchProviderModels,
  listGroups,
  listHealth,
  listProviders,
  updateGroup,
  type Group,
  type HealthSnapshot,
  type Provider,
} from "../api/tauri";
import HealthBadge from "../components/HealthBadge.vue";
import { findHealth } from "../utils/health";

const groups = ref<Group[]>([]);
const providers = ref<Provider[]>([]);
const health = ref<HealthSnapshot[]>([]);
const error = ref("");
const healthLoading = ref(false);
const editing = ref<Group | null>(null);
/** 每条队列条目拉取到的上游模型 id 列表 */
const modelOptions = ref<Record<number, string[]>>({});
const fetchingModels = ref<Record<number, boolean>>({});

const form = reactive({
  name: "",
  auto_failover: true,
  items: [] as { provider_id: number; upstream_model: string }[],
});

const providerMap = computed(() => new Map(providers.value.map((p) => [p.id, p])));

async function refresh() {
  try {
    [groups.value, providers.value, health.value] = await Promise.all([
      listGroups(),
      listProviders(),
      listHealth(),
    ]);
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function refreshHealth() {
  healthLoading.value = true;
  try {
    health.value = await listHealth();
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    healthLoading.value = false;
  }
}

function resetForm() {
  editing.value = null;
  form.name = "";
  form.auto_failover = true;
  form.items = [];
}

function startEdit(g: Group) {
  editing.value = g;
  form.name = g.name;
  form.auto_failover = g.auto_failover;
  form.items = g.items.map((i) => ({
    provider_id: i.provider_id,
    upstream_model: i.upstream_model,
  }));
}

function addItem() {
  const first = providers.value[0];
  form.items.push({
    provider_id: first?.id ?? 0,
    upstream_model: "gpt-4o-mini",
  });
}

function moveItem(index: number, delta: number) {
  const next = index + delta;
  if (next < 0 || next >= form.items.length) return;
  const arr = form.items;
  const tmp = arr[index];
  arr[index] = arr[next];
  arr[next] = tmp;
}

function removeItem(index: number) {
  form.items.splice(index, 1);
  delete modelOptions.value[index];
  delete fetchingModels.value[index];
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

async function save() {
  try {
    const payload = {
      name: form.name,
      auto_failover: form.auto_failover,
      items: form.items.filter((i) => i.provider_id > 0 && i.upstream_model.trim()),
    };
    if (editing.value) {
      await updateGroup({ id: editing.value.id, ...payload });
    } else {
      await createGroup(payload);
    }
    resetForm();
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
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

onMounted(refresh);
</script>

<template>
  <div class="space-y-6">
    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-4 text-base font-semibold">{{ editing ? "编辑分组" : "新建分组" }}</h2>
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
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-medium">故障转移队列</h3>
          <button type="button" class="text-sm text-cyan-700 hover:underline" @click="addItem">添加条目</button>
        </div>
        <div
          v-for="(item, index) in form.items"
          :key="index"
          class="flex flex-wrap items-center gap-2 rounded-lg border border-slate-200 p-3"
        >
          <span class="w-8 text-xs text-slate-400">#{{ index + 1 }}</span>
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
        <button type="button" class="rounded-lg bg-slate-800 px-4 py-2 text-sm text-white" @click="save">
          保存
        </button>
        <button
          v-if="editing"
          type="button"
          class="rounded-lg border border-slate-300 px-4 py-2 text-sm"
          @click="resetForm"
        >
          取消
        </button>
      </div>
      <p v-if="error" class="mt-3 text-sm text-rose-600">{{ error }}</p>
    </section>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <div class="mb-4 flex flex-wrap items-center justify-between gap-2">
        <h2 class="text-base font-semibold">分组列表</h2>
        <button
          type="button"
          class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm hover:bg-slate-50 disabled:opacity-50"
          :disabled="healthLoading"
          @click="refreshHealth"
        >
          {{ healthLoading ? "刷新中…" : "刷新健康" }}
        </button>
      </div>
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
