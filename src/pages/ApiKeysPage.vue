<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import {
  createApiKey,
  deleteApiKey,
  extractInvokeError,
  listApiKeys,
  updateApiKey,
  type ApiKeyPublic,
} from "../api/tauri";

const items = ref<ApiKeyPublic[]>([]);
const error = ref("");
const createdRaw = ref("");
const form = reactive({ name: "", enabled: true });

async function refresh() {
  try {
    items.value = await listApiKeys();
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function create() {
  try {
    const created = await createApiKey({ name: form.name, enabled: form.enabled });
    createdRaw.value = created.raw_key;
    form.name = "";
    form.enabled = true;
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function toggle(item: ApiKeyPublic) {
  try {
    await updateApiKey({ id: item.id, name: item.name, enabled: !item.enabled });
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function remove(id: number) {
  if (!confirm("确认删除该 Key？")) return;
  try {
    await deleteApiKey(id);
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function copyRaw() {
  if (!createdRaw.value) return;
  await navigator.clipboard.writeText(createdRaw.value);
}

onMounted(refresh);
</script>

<template>
  <div class="space-y-6">
    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-4 text-base font-semibold">创建客户端 API Key</h2>
      <div class="flex flex-wrap items-end gap-3">
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">名称</span>
          <input v-model="form.name" class="rounded-lg border border-slate-300 px-3 py-2" />
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input v-model="form.enabled" type="checkbox" />
          启用
        </label>
        <button type="button" class="rounded-lg bg-slate-800 px-4 py-2 text-sm text-white" @click="create">
          创建
        </button>
      </div>
      <div v-if="createdRaw" class="mt-4 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm">
        <div class="mb-1 font-medium text-amber-800">请立即复制明文 Key（仅展示一次）</div>
        <div class="flex flex-wrap items-center gap-2">
          <code class="break-all font-mono text-xs">{{ createdRaw }}</code>
          <button type="button" class="rounded bg-amber-200 px-2 py-1 text-xs" @click="copyRaw">复制</button>
        </div>
      </div>
      <p v-if="error" class="mt-3 text-sm text-rose-600">{{ error }}</p>
    </section>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-4 text-base font-semibold">Key 列表</h2>
      <div v-if="items.length === 0" class="text-sm text-slate-500">暂无 Key</div>
      <table v-else class="min-w-full text-left text-sm">
        <thead class="border-b text-slate-500">
          <tr>
            <th class="px-2 py-2">名称</th>
            <th class="px-2 py-2">脱敏</th>
            <th class="px-2 py-2">状态</th>
            <th class="px-2 py-2">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in items" :key="item.id" class="border-b border-slate-100">
            <td class="px-2 py-2">{{ item.name }}</td>
            <td class="px-2 py-2 font-mono text-xs">{{ item.masked }}</td>
            <td class="px-2 py-2">{{ item.enabled ? "启用" : "停用" }}</td>
            <td class="px-2 py-2 space-x-2">
              <button type="button" class="text-cyan-700 hover:underline" @click="toggle(item)">
                {{ item.enabled ? "停用" : "启用" }}
              </button>
              <button type="button" class="text-rose-600 hover:underline" @click="remove(item.id)">删除</button>
            </td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>
