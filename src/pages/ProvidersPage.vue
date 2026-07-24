<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useForm } from "@tanstack/vue-form";
import { HButton, HCheckbox, HEmpty, HInput } from "happier-ui";
import {
  createProvider,
  deleteProvider,
  extractInvokeError,
  listProviders,
  updateProvider,
  type Provider,
} from "../api/tauri";
import AppDialog from "../components/AppDialog.vue";
import {
  describeProviderPasteSource,
  parseProviderPaste,
} from "../utils/providerPaste";

type ProviderFormValues = {
  name: string;
  base_url: string;
  api_key: string;
  enabled: boolean;
};

const defaultFormValues: ProviderFormValues = {
  name: "",
  base_url: "https://api.openai.com/v1",
  api_key: "",
  enabled: true,
};

const items = ref<Provider[]>([]);
const error = ref("");
const message = ref("");
const editingProviderId = ref<number | null>(null);
const dialogOpen = ref(false);
const saving = ref(false);
const pasteText = ref("");

const form = useForm({
  defaultValues: { ...defaultFormValues },
  onSubmit: async ({ value }) => {
    if (saving.value) return;
    message.value = "";
    saving.value = true;
    try {
      const targetId = editingProviderId.value;
      if (targetId !== null) {
        await updateProvider({
          id: targetId,
          name: value.name,
          base_url: value.base_url,
          api_key: value.api_key,
          enabled: value.enabled,
        });
      } else {
        await createProvider({ ...value });
      }
      dialogOpen.value = false;
      resetForm();
      await refresh();
    } catch (e) {
      error.value = extractInvokeError(e);
    } finally {
      saving.value = false;
    }
  },
});

async function refresh() {
  try {
    items.value = await listProviders();
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

function resetForm() {
  editingProviderId.value = null;
  form.reset({ ...defaultFormValues });
  pasteText.value = "";
  error.value = "";
  message.value = "";
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
  if (parsed.baseUrl) form.setFieldValue("base_url", parsed.baseUrl);
  if (parsed.apiKey) form.setFieldValue("api_key", parsed.apiKey);
  // 编辑时保留原名称；新建且名称为空时用域名建议名。
  const currentName = String(form.state.values.name ?? "");
  if (
    editingProviderId.value === null &&
    !currentName.trim() &&
    parsed.suggestedName
  ) {
    form.setFieldValue("name", parsed.suggestedName);
  }
  const sourceLabel = describeProviderPasteSource(parsed.source);
  if (parsed.warnings.length > 0) {
    error.value = `${sourceLabel} 部分识别：${parsed.warnings.join("；")}。请补全后保存。`;
  } else {
    message.value = `已从${sourceLabel}识别并填入表单，请确认后保存。`;
  }
}

function openCreate() {
  resetForm();
  dialogOpen.value = true;
}

function startEdit(p: Provider) {
  error.value = "";
  message.value = "";
  editingProviderId.value = p.id;
  dialogOpen.value = true;
  form.reset({
    name: p.name,
    base_url: p.base_url,
    api_key: p.api_key,
    enabled: p.enabled,
  });
}

function closeDialog() {
  if (saving.value) return;
  dialogOpen.value = false;
  resetForm();
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
      <div class="flex items-center justify-between gap-2">
        <h2 class="text-base font-semibold">供应商管理</h2>
        <HButton variant="primary" type="button" @click="openCreate">新建供应商</HButton>
      </div>
    </section>

    <AppDialog
      :open="dialogOpen"
      :title="editingProviderId === null ? '新建供应商' : '编辑供应商'"
      :close-disabled="saving"
      @close="closeDialog"
    >
      <section>
        <p v-if="editingProviderId !== null" class="mb-4 text-sm text-cyan-800">正在编辑供应商</p>
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
            <HButton variant="secondary" size="sm" type="button" @click="applyPaste">
              识别并填入表单
            </HButton>
            <HButton variant="outline" size="sm" type="button" @click="pasteText = ''">
              清空粘贴框
            </HButton>
          </div>
        </div>
        <form
          class="grid gap-3 md:grid-cols-2"
          @submit.prevent="form.handleSubmit()"
        >
          <form.Field name="name">
            <template #default="{ field }">
              <HInput
                :model-value="field.state.value"
                label="名称"
                @update:model-value="field.handleChange"
              />
            </template>
          </form.Field>
          <form.Field name="base_url">
            <template #default="{ field }">
              <HInput
                :model-value="field.state.value"
                label="Base URL"
                @update:model-value="field.handleChange"
              />
            </template>
          </form.Field>
          <div class="md:col-span-2">
            <form.Field name="api_key">
              <template #default="{ field }">
                <HInput
                  :model-value="field.state.value"
                  type="password"
                  autocomplete="off"
                  label="上游 API Key"
                  @update:model-value="field.handleChange"
                />
              </template>
            </form.Field>
          </div>
          <form.Field name="enabled">
            <template #default="{ field }">
              <HCheckbox
                :model-value="field.state.value"
                label="启用"
                @update:model-value="field.handleChange"
              />
            </template>
          </form.Field>
          <div class="mt-1 flex flex-wrap gap-2 md:col-span-2">
            <HButton variant="primary" type="submit" :disabled="saving">
              {{ saving ? "保存中…" : "保存" }}
            </HButton>
            <HButton variant="outline" type="button" :disabled="saving" @click="closeDialog">
              取消
            </HButton>
          </div>
        </form>
        <p v-if="message" class="mt-3 text-sm text-emerald-700">{{ message }}</p>
        <p v-if="error" class="mt-3 text-sm text-rose-600">{{ error }}</p>
      </section>
    </AppDialog>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <div class="mb-4 flex flex-wrap items-center justify-between gap-2">
        <h2 class="text-base font-semibold">供应商列表</h2>
      </div>
      <p v-if="error && !dialogOpen" class="mb-3 text-sm text-rose-600">{{ error }}</p>
      <HEmpty v-if="items.length === 0" class="app-empty-compact" title="暂无供应商" />
      <div v-else class="overflow-x-auto">
        <table class="min-w-full text-left text-sm">
          <thead class="border-b text-slate-500">
            <tr>
              <th class="px-2 py-2">名称</th>
              <th class="px-2 py-2">Base URL</th>
              <th class="px-2 py-2">启用</th>
              <th class="px-2 py-2">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="p in items" :key="p.id" class="border-b border-slate-100">
              <td class="px-2 py-2 font-medium">{{ p.name }}</td>
              <td class="px-2 py-2 font-mono text-xs">{{ p.base_url }}</td>
              <td class="px-2 py-2">{{ p.enabled ? "启用" : "停用" }}</td>
              <td class="px-2 py-2 space-x-2">
                <HButton variant="ghost" size="sm" type="button" @click="startEdit(p)">
                  编辑
                </HButton>
                <HButton variant="danger-soft" size="sm" type="button" @click="remove(p.id)">
                  删除
                </HButton>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
