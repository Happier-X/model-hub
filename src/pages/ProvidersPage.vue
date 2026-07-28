<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Plus } from "@lucide/vue";
import { useForm } from "@tanstack/vue-form";
import {
  HButton,
  HCard,
  HCheckbox,
  HEmpty,
  HInput,
  HSwitch,
  HTable,
  type HTableColumn,
  HTextarea,
} from "happier-ui";
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

const providerColumns: HTableColumn[] = [
  { key: "name", title: "名称" },
  { key: "base_url", title: "Base URL" },
  { key: "enabled", title: "启用" },
  { key: "actions", title: "操作" },
];
const error = ref("");
const message = ref("");
const editingProviderId = ref<number | null>(null);
const dialogOpen = ref(false);
const saving = ref(false);
const pasteText = ref("");
// 行内启用开关进行中的 id 集合，用于 disabled 防重复点击
const togglingIds = ref<Set<number>>(new Set());

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

// 行内开关启停：乐观更新本地 -> 整行更新到后端 -> 成功用返回值同步 / 失败回滚并报错
async function toggleProviderEnabled(p: Provider, next: boolean) {
  if (togglingIds.value.has(p.id)) return;
  const previous = p.enabled;
  // 乐观更新
  const target = items.value.find((it) => it.id === p.id);
  if (target) target.enabled = next;
  togglingIds.value = new Set(togglingIds.value).add(p.id);
  try {
    const updated = await updateProvider({
      id: p.id,
      name: p.name,
      base_url: p.base_url,
      api_key: p.api_key,
      enabled: next,
    });
    // 以服务端返回为准同步
    const sync = items.value.find((it) => it.id === p.id);
    if (sync) Object.assign(sync, updated);
  } catch (e) {
    const failed = items.value.find((it) => it.id === p.id);
    if (failed) failed.enabled = previous;
    error.value = extractInvokeError(e);
  } finally {
    const nextSet = new Set(togglingIds.value);
    nextSet.delete(p.id);
    togglingIds.value = nextSet;
  }
}

onMounted(refresh);
</script>

<template>
  <div class="space-y-6">
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
          <!-- HTextarea 内部 textarea 无法接收等宽字体（class 落到外层 div，表单元素不继承 font-family）；
               原 font-mono 暂时降级，等 happier-ui#8 补 monospace 支持后恢复。 -->
          <HTextarea
            v-model="pasteText"
            :rows="4"
            :spellcheck="false"
            placeholder='例如：{"_type":"newapi_channel_conn","key":"sk-...","url":"https://..."}'
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

    <HCard variant="outlined" padding="md">
      <template #header>
        <div class="flex items-center justify-between gap-2">
          <h2 class="text-base font-semibold">供应商</h2>
          <HButton
            variant="ghost"
            size="sm"
            isIconOnly
            shape="circle"
            title="新建供应商"
            aria-label="新建供应商"
            type="button"
            @click="openCreate"
          >
            <Plus :size="18" aria-hidden="true" />
          </HButton>
        </div>
      </template>
      <p v-if="error && !dialogOpen" class="mb-3 text-sm text-rose-600">{{ error }}</p>
      <HEmpty v-if="items.length === 0" class="app-empty-compact" title="暂无供应商" />
      <!-- HTable data 只接受 Record<string, unknown>[]，interface 无索引签名需双重断言；等 happier-ui#9 泛型化后简化 -->
      <HTable
        v-else
        :columns="providerColumns"
        :data="items as unknown as Record<string, unknown>[]"
        row-key="id"
        class="text-sm"
      >
        <template #cell="{ column, row }">
          <template v-if="column.key === 'name'">
            <span class="font-medium">{{ (row as Provider).name }}</span>
          </template>
          <template v-else-if="column.key === 'base_url'">
            <span class="font-mono text-xs">{{ (row as Provider).base_url }}</span>
          </template>
          <template v-else-if="column.key === 'enabled'">
            <HSwitch
              :model-value="(row as Provider).enabled"
              :disabled="togglingIds.has((row as Provider).id) || saving"
              :aria-label="`${(row as Provider).name} 启用`"
              @update:model-value="toggleProviderEnabled(row as Provider, $event)"
            />
          </template>
          <template v-else-if="column.key === 'actions'">
            <span class="space-x-2">
              <HButton variant="ghost" size="sm" type="button" @click="startEdit(row as Provider)">
                编辑
              </HButton>
              <HButton
                variant="danger-soft"
                size="sm"
                type="button"
                @click="remove((row as Provider).id)"
              >
                删除
              </HButton>
            </span>
          </template>
          <template v-else>{{ (row as Provider)[column.key as keyof Provider] }}</template>
        </template>
      </HTable>
    </HCard>
  </div>
</template>
