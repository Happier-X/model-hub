<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import {
  createApiKey,
  deleteApiKey,
  exportToPiAgent,
  extractInvokeError,
  listApiKeys,
  updateApiKey,
  type ApiKeyPublic,
} from "../api/tauri";

const items = ref<ApiKeyPublic[]>([]);
const error = ref("");
const message = ref("");
const createdRaw = ref("");
const piKeyInput = ref("");
const exporting = ref(false);
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

async function exportPi() {
  exporting.value = true;
  message.value = "";
  try {
    // 若刚创建过 Key 且未单独填写，优先用明文
    const key = piKeyInput.value.trim() || createdRaw.value.trim();
    const result = await exportToPiAgent(key || undefined);
    error.value = "";
    message.value = `已写入 Pi 配置：${result.path}\n供应商 ${result.provider_id}，共 ${result.model_count} 个模型，Base URL ${result.base_url}${result.used_placeholder_key ? "（未填 Key，已写占位 apiKey）" : ""}。请在 Pi 中打开 /model 选择 model-hub/<分组名>。`;
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    exporting.value = false;
  }
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
      <h2 class="mb-2 text-base font-semibold">配置到 Pi Agent</h2>
      <p class="mb-3 text-sm text-slate-500">
        一键将当前代理地址与全部分组写入本机
        <code class="rounded bg-slate-100 px-1 text-xs">~/.pi/agent/models.json</code>
        的
        <code class="rounded bg-slate-100 px-1 text-xs">model-hub</code>
        供应商。Key 可留空（本地场景）；代理需强制鉴权时请填客户端 Key 或先上方创建。
      </p>
      <div class="flex flex-wrap items-end gap-3">
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">客户端 Key（可选）</span>
          <input
            v-model="piKeyInput"
            type="password"
            autocomplete="off"
            placeholder="可留空；有刚创建的明文会自动使用"
            class="w-72 rounded-lg border border-slate-300 px-3 py-2"
          />
        </label>
        <button
          type="button"
          class="rounded-lg bg-cyan-700 px-4 py-2 text-sm text-white hover:bg-cyan-600 disabled:opacity-50"
          :disabled="exporting"
          @click="exportPi"
        >
          {{ exporting ? "写入中…" : "一键配置到 Pi" }}
        </button>
      </div>
      <p v-if="message" class="mt-3 whitespace-pre-line text-sm text-emerald-700">{{ message }}</p>
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
