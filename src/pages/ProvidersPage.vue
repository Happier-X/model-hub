<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import {
  createProvider,
  deleteProvider,
  extractInvokeError,
  listHealth,
  listProviders,
  updateProvider,
  type HealthSnapshot,
  type Provider,
} from "../api/tauri";
import HealthBadge from "../components/HealthBadge.vue";
import { findHealth } from "../utils/health";

const items = ref<Provider[]>([]);
const health = ref<HealthSnapshot[]>([]);
const error = ref("");
const healthLoading = ref(false);
const editing = ref<Provider | null>(null);
const form = reactive({
  name: "",
  base_url: "https://api.openai.com/v1",
  api_key: "",
  enabled: true,
});

async function refresh() {
  try {
    [items.value, health.value] = await Promise.all([listProviders(), listHealth()]);
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
  form.base_url = "https://api.openai.com/v1";
  form.api_key = "";
  form.enabled = true;
}

function startEdit(p: Provider) {
  editing.value = p;
  form.name = p.name;
  form.base_url = p.base_url;
  form.api_key = p.api_key;
  form.enabled = p.enabled;
}

async function save() {
  try {
    if (editing.value) {
      await updateProvider({
        id: editing.value.id,
        name: form.name,
        base_url: form.base_url,
        api_key: form.api_key,
        enabled: form.enabled,
      });
    } else {
      await createProvider({ ...form });
    }
    resetForm();
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function remove(id: number) {
  if (!confirm("确认删除该供应商？")) return;
  try {
    await deleteProvider(id);
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
      <h2 class="mb-4 text-base font-semibold">{{ editing ? "编辑供应商" : "新建供应商" }}</h2>
      <div class="grid gap-3 md:grid-cols-2">
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">名称</span>
          <input v-model="form.name" class="w-full rounded-lg border border-slate-300 px-3 py-2" />
        </label>
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">Base URL</span>
          <input v-model="form.base_url" class="w-full rounded-lg border border-slate-300 px-3 py-2" />
        </label>
        <label class="text-sm md:col-span-2">
          <span class="mb-1 block text-slate-500">上游 API Key</span>
          <input
            v-model="form.api_key"
            type="password"
            autocomplete="off"
            class="w-full rounded-lg border border-slate-300 px-3 py-2"
          />
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input v-model="form.enabled" type="checkbox" />
          启用
        </label>
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
        <h2 class="text-base font-semibold">供应商列表</h2>
        <button
          type="button"
          class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm hover:bg-slate-50 disabled:opacity-50"
          :disabled="healthLoading"
          @click="refreshHealth"
        >
          {{ healthLoading ? "刷新中…" : "刷新健康" }}
        </button>
      </div>
      <div v-if="items.length === 0" class="text-sm text-slate-500">暂无供应商</div>
      <div v-else class="overflow-x-auto">
        <table class="min-w-full text-left text-sm">
          <thead class="border-b text-slate-500">
            <tr>
              <th class="px-2 py-2">名称</th>
              <th class="px-2 py-2">Base URL</th>
              <th class="px-2 py-2">启用</th>
              <th class="px-2 py-2">健康</th>
              <th class="px-2 py-2">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="p in items" :key="p.id" class="border-b border-slate-100">
              <td class="px-2 py-2 font-medium">{{ p.name }}</td>
              <td class="px-2 py-2 font-mono text-xs">{{ p.base_url }}</td>
              <td class="px-2 py-2">{{ p.enabled ? "启用" : "停用" }}</td>
              <td class="px-2 py-2">
                <HealthBadge :snapshot="findHealth(health, p.id)" />
              </td>
              <td class="px-2 py-2 space-x-2">
                <button type="button" class="text-cyan-700 hover:underline" @click="startEdit(p)">编辑</button>
                <button type="button" class="text-rose-600 hover:underline" @click="remove(p.id)">删除</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
