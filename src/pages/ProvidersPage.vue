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
import {
  describeProviderPasteSource,
  parseProviderPaste,
} from "../utils/providerPaste";

const items = ref<Provider[]>([]);
const health = ref<HealthSnapshot[]>([]);
const error = ref("");
const message = ref("");
const healthLoading = ref(false);
const editing = ref<Provider | null>(null);
const pasteText = ref("");
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
  pasteText.value = "";
}

function applyPaste() {
  message.value = "";
  error.value = "";
  const parsed = parseProviderPaste(pasteText.value);
  if (!parsed) {
    error.value =
      "未能识别 Base URL 或 API Key。可粘贴 NewAPI 分享 JSON、环境变量、curl 或普通文本。";
    return;
  }
  if (parsed.baseUrl) form.base_url = parsed.baseUrl;
  if (parsed.apiKey) form.api_key = parsed.apiKey;
  // 编辑时保留原名称；新建且名称为空时用域名建议名。
  if (!editing.value && !form.name.trim() && parsed.suggestedName) {
    form.name = parsed.suggestedName;
  }
  const sourceLabel = describeProviderPasteSource(parsed.source);
  if (parsed.warnings.length > 0) {
    error.value = `${sourceLabel} 部分识别：${parsed.warnings.join("；")}。请补全后保存。`;
  } else {
    message.value = `已从${sourceLabel}识别并填入表单，请确认后保存。`;
  }
}

function startEdit(p: Provider) {
  editing.value = p;
  form.name = p.name;
  form.base_url = p.base_url;
  form.api_key = p.api_key;
  form.enabled = p.enabled;
}

async function save() {
  message.value = "";
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
      <div class="mb-4 rounded-lg border border-dashed border-cyan-300 bg-cyan-50/40 p-3">
        <div class="mb-2 text-sm font-medium text-slate-700">粘贴快速添加</div>
        <p class="mb-2 text-xs text-slate-500">
          支持 NewAPI 分享 JSON（含
          <code class="rounded bg-white px-1">newapi_channel_conn</code>）、环境变量、curl 与普通文本。仅本地解析，不会上传。
        </p>
        <textarea
          v-model="pasteText"
          rows="4"
          spellcheck="false"
          placeholder='例如：{"_type":"newapi_channel_conn","key":"sk-...","url":"https://..."}'
          class="w-full rounded-lg border border-slate-300 bg-white px-3 py-2 font-mono text-xs"
        />
        <div class="mt-2 flex flex-wrap gap-2">
          <button
            type="button"
            class="rounded-lg bg-cyan-700 px-3 py-1.5 text-sm text-white hover:bg-cyan-600"
            @click="applyPaste"
          >
            识别并填入表单
          </button>
          <button
            type="button"
            class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm hover:bg-white"
            @click="pasteText = ''"
          >
            清空粘贴框
          </button>
        </div>
      </div>
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
      <div class="mt-4 flex flex-wrap gap-2">
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
      <p v-if="message" class="mt-3 text-sm text-emerald-700">{{ message }}</p>
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
