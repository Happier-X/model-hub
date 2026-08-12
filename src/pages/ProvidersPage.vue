<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Plus } from "@lucide/vue";
import { useForm } from "@tanstack/vue-form";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Empty } from "@/components/ui/empty";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import {
  createProvider,
  deleteProvider,
  extractInvokeError,
  listProviders,
  syncProviderNow,
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
  /** 自动同步开关：新建默认开，编辑保留原值（编辑表单不展示该字段） */
  auto_sync: boolean;
};

const defaultFormValues: ProviderFormValues = {
  name: "",
  base_url: "https://api.openai.com/v1",
  api_key: "",
  enabled: true,
  auto_sync: true,
};

const items = ref<Provider[]>([]);
const page = ref(1);
const pageSize = 10;
const pagedItems = computed(() => items.value.slice((page.value - 1) * pageSize, page.value * pageSize));

const providerColumns: { key: string; title: string }[] = [
  { key: "name", title: "名称" },
  { key: "base_url", title: "Base URL" },
  { key: "enabled", title: "启用" },
  { key: "last_sync_at", title: "上次同步" },
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
// 「立即同步」进行中的 id 集合，按钮 loading/disabled 防重复点击
const syncingIds = ref<Set<number>>(new Set());

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
          auto_sync: value.auto_sync,
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
    page.value = 1;
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
    auto_sync: p.auto_sync,
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

/** 上次同步时间展示：null/0 → 「未同步」；否则本地时间格式化。 */
function formatSyncTime(unix: number | null | undefined): string {
  if (!unix || unix <= 0) return "未同步";
  try {
    return new Date(unix * 1000).toLocaleString("zh-CN", { hour12: false });
  } catch {
    return String(unix);
  }
}

// 立即同步：按钮 loading/disabled，成功后刷新列表以更新 last_sync_at
async function syncNow(p: Provider) {
  if (syncingIds.value.has(p.id)) return;
  syncingIds.value = new Set(syncingIds.value).add(p.id);
  error.value = "";
  try {
    await syncProviderNow(p.id);
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    const nextSet = new Set(syncingIds.value);
    nextSet.delete(p.id);
    syncingIds.value = nextSet;
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
      auto_sync: p.auto_sync,
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
  <div class="h-full flex flex-col overflow-hidden">
    <AppDialog
      :open="dialogOpen"
      :title="editingProviderId === null ? '新建供应商' : '编辑供应商'"
      :close-disabled="saving"
      @close="closeDialog"
    >
      <section>
        <p v-if="editingProviderId !== null" class="mb-4 text-sm text-info">正在编辑供应商</p>
        <div class="mb-4 rounded-lg border border-dashed border-info/30 bg-info/5 p-3">
          <div class="mb-2 text-sm font-medium text-foreground">粘贴快速添加</div>
          <p class="mb-2 text-xs text-muted-foreground">
            支持 NewAPI 分享 JSON（含
            <code class="rounded bg-card px-1">newapi_channel_conn</code>）、环境变量、curl 与普通文本。仅本地解析，不会上传。
          </p>
          <!-- Textarea 支持 class 透传，可直接加 font-mono -->
          <Textarea
            v-model="pasteText"
            :rows="4"
            :spellcheck="false"
            placeholder='例如：{"_type":"newapi_channel_conn","key":"sk-...","url":"https://..."}'
            class="font-mono"
          />
          <div class="mt-2 flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" type="button" @click="applyPaste">
              识别并填入表单
            </Button>
            <Button variant="outline" size="sm" type="button" @click="pasteText = ''">
              清空粘贴框
            </Button>
          </div>
        </div>
        <form
          class="grid gap-3 md:grid-cols-2"
          @submit.prevent="form.handleSubmit()"
        >
          <form.Field name="name">
            <template #default="{ field }">
              <Field>
                <FieldLabel>名称</FieldLabel>
                <Input
                  :model-value="field.state.value"
                  @update:model-value="(v) => field.handleChange(v as string)"
                />
              </Field>
            </template>
          </form.Field>
          <form.Field name="base_url">
            <template #default="{ field }">
              <Field>
                <FieldLabel>Base URL</FieldLabel>
                <Input
                  :model-value="field.state.value"
                  @update:model-value="(v) => field.handleChange(v as string)"
                />
              </Field>
            </template>
          </form.Field>
          <div class="md:col-span-2">
            <form.Field name="api_key">
              <template #default="{ field }">
                <Field>
                  <FieldLabel>上游 API Key</FieldLabel>
                  <Input
                    :model-value="field.state.value"
                    type="password"
                    autocomplete="off"
                    @update:model-value="(v) => field.handleChange(v as string)"
                  />
                </Field>
              </template>
            </form.Field>
          </div>
          <form.Field name="enabled">
            <template #default="{ field }">
              <Field orientation="horizontal">
                <Checkbox
                  id="provider-enabled"
                  :model-value="field.state.value"
                  @update:model-value="(v) => field.handleChange(v === true)"
                />
                <FieldLabel for="provider-enabled">启用</FieldLabel>
              </Field>
            </template>
          </form.Field>
          <div class="mt-1 flex flex-wrap gap-2 md:col-span-2">
            <Button variant="default" type="submit" :disabled="saving">
              {{ saving ? "保存中…" : "保存" }}
            </Button>
            <Button variant="outline" type="button" :disabled="saving" @click="closeDialog">
              取消
            </Button>
          </div>
        </form>
        <p v-if="message" class="mt-3 text-sm text-success">{{ message }}</p>
        <p v-if="error" class="mt-3 text-sm text-destructive">{{ error }}</p>
      </section>
    </AppDialog>

    <Card class="min-h-0 flex-1 flex flex-col">
      <CardHeader class="shrink-0 py-3">
        <div class="flex items-center justify-between gap-2">
          <h2 class="text-base font-semibold">供应商</h2>
          <Button
            variant="ghost"
            size="icon"
            title="新建供应商"
            aria-label="新建供应商"
            type="button"
            @click="openCreate"
          >
            <Plus aria-hidden="true" />
          </Button>
        </div>
      </CardHeader>
      <CardContent class="flex min-h-0 flex-1 flex-col gap-3">
        <p v-if="error && !dialogOpen" class="text-sm text-destructive">{{ error }}</p>
        <p v-if="items.length > 0" class="text-sm text-muted-foreground">共 {{ items.length }} 个供应商</p>
        <Empty v-if="items.length === 0" class="app-empty-compact" title="暂无供应商" />
        <template v-else>
          <!-- 表格滚动区：flex-1 撑满，min-h-0 overflow-y-auto 仅表格 body 滚动 -->
          <div class="min-h-0 flex-1 overflow-y-auto">
            <Table class="text-sm">
              <TableHeader>
                <TableRow>
                  <TableHead v-for="col in providerColumns" :key="col.key">{{ col.title }}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                <TableRow v-for="row in pagedItems" :key="row.id">
                  <TableCell v-for="col in providerColumns" :key="col.key">
                    <template v-if="col.key === 'name'">
                      <span class="font-medium">{{ row.name }}</span>
                    </template>
                    <template v-else-if="col.key === 'base_url'">
                      <span class="font-mono text-xs">{{ row.base_url }}</span>
                    </template>
                    <template v-else-if="col.key === 'enabled'">
                      <Switch
                        :model-value="row.enabled"
                        :disabled="togglingIds.has(row.id) || saving"
                        :aria-label="`${row.name} 启用`"
                        @update:model-value="toggleProviderEnabled(row, $event)"
                      />
                    </template>
                    <template v-else-if="col.key === 'last_sync_at'">
                      <span class="text-xs text-muted-foreground">
                        {{ formatSyncTime(row.last_sync_at) }}
                      </span>
                    </template>
                    <template v-else-if="col.key === 'actions'">
                      <span class="inline-flex items-center gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          type="button"
                          :disabled="syncingIds.has(row.id)"
                          @click="syncNow(row)"
                        >
                          {{ syncingIds.has(row.id) ? "同步中…" : "立即同步" }}
                        </Button>
                        <Button variant="outline" size="sm" type="button" @click="startEdit(row)">
                          编辑
                        </Button>
                        <Button
                          variant="destructive"
                          size="sm"
                          type="button"
                          @click="remove(row.id)"
                        >
                          删除
                        </Button>
                      </span>
                    </template>
                    <template v-else>{{ (row as Record<string, unknown>)[col.key] }}</template>
                  </TableCell>
                </TableRow>
                <TableRow v-if="pagedItems.length === 0">
                  <TableCell :colspan="providerColumns.length" class="py-8 text-center text-muted-foreground">
                    暂无数据
                  </TableCell>
                </TableRow>
              </TableBody>
            </Table>
          </div>
          <!-- 分页器：表格滚动区之后，不随表格滚动 -->
          <div v-if="items.length > pageSize" class="flex shrink-0 justify-end">
            <Pagination
              v-model:page="page"
              :total="items.length"
              :items-per-page="pageSize"
            >
              <PaginationContent v-slot="{ items: pageItems, page }" class="gap-0.5">
                <PaginationPrevious>
                  <span class="hidden sm:block">上一页</span>
                </PaginationPrevious>
                <template v-for="item in pageItems" :key="item.type + item.value">
                  <PaginationItem
                    v-if="item.type === 'page'"
                    :value="item.value"
                    :is-active="item.value === page"
                  >
                    {{ item.value }}
                  </PaginationItem>
                  <PaginationEllipsis v-else-if="item.type === 'ellipsis'" />
                </template>
                <PaginationNext>
                  <span class="hidden sm:block">下一页</span>
                </PaginationNext>
              </PaginationContent>
            </Pagination>
          </div>
        </template>
      </CardContent>
    </Card>
  </div>
</template>
