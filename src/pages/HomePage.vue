<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import Heatmap from "@/components/Heatmap.vue";
import StatsCards from "@/components/StatsCards.vue";
import { type HeatmapValue } from "@/utils/heatmap";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
  extractInvokeError,
  getRequestDailyCounts,
  getRequestOverview,
  proxyStart,
  proxyStatus,
  proxyStop,
  type ProxyStatus,
  type RequestDailyCounts,
  type RequestOverview,
} from "../api/tauri";

const status = ref<ProxyStatus | null>(null);
const loading = ref(false);
const message = ref("");
const error = ref("");
const overview = ref<RequestOverview | null>(null);
const overviewError = ref("");
const daily = ref<RequestDailyCounts | null>(null);
const dailyError = ref("");
const dailyLoading = ref(false);

const heatmapData = computed<HeatmapValue[]>(() => {
  const counts = daily.value;
  if (!counts) return [];
  // 后端只返回有记录的日（count>0）；补全 365 天全网格，让热力图不依赖数据的实际时间范围。
  const dayMs = 86_400_000;
  // end_unix 是“今日次日 00:00”的 unix 秒；向前取 365 天（含今日）。
  const endMs = counts.end_unix * 1000;
  const startMs = endMs - 365 * dayMs;
  // 后端 days 由 day_start_unix 升序，借成 Map 供 O(1) 查。
  const byDay = new Map<number, number>();
  for (const d of counts.days) byDay.set(d.day_start_unix, d.count);
  const out: HeatmapValue[] = [];
  for (let t = startMs; t < endMs; t += dayMs) {
    // counts 的 day_start_unix 是 unix 秒，除以 1000 换回去。
    const dayStartUnix = Math.round(t / 1000);
    // 自研 Heatmap 要求 { date, count }；date 传 'YYYY-MM-DD'（本地自然日）。
    const d = new Date(t);
    const yyyy = d.getFullYear();
    const mm = String(d.getMonth() + 1).padStart(2, "0");
    const dd = String(d.getDate()).padStart(2, "0");
    out.push({ date: `${yyyy}-${mm}-${dd}`, count: byDay.get(dayStartUnix) ?? 0 });
  }
  return out;
});

const statusBadgeVariant = computed<"success" | "danger" | "default">(() => {
  if (status.value?.state === "running") return "success";
  if (status.value?.state === "error") return "danger";
  return "default";
});

async function refreshStats() {
  const overviewPromise = getRequestOverview()
    .then((value) => {
      overview.value = value;
      overviewError.value = "";
    })
    .catch((e) => {
      overviewError.value = extractInvokeError(e);
    });
  dailyLoading.value = true;
  const dailyPromise = getRequestDailyCounts()
    .then((value) => {
      daily.value = value;
      dailyError.value = "";
    })
    .catch((e) => {
      dailyError.value = extractInvokeError(e);
    })
    .finally(() => {
      dailyLoading.value = false;
    });
  await Promise.all([overviewPromise, dailyPromise]);
}

async function refresh() {
  try {
    status.value = await proxyStatus();
    if (status.value.port_note) {
      message.value = status.value.port_note;
    }
    await refreshStats();
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

/** 仅轮询刷新统计总览（5s），避免频繁刷新每日热力图。 */
async function refreshOverviewOnly() {
  try {
    overview.value = await getRequestOverview();
    overviewError.value = "";
  } catch (e) {
    overviewError.value = extractInvokeError(e);
  }
}

let overviewTimer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  refresh();
  overviewTimer = setInterval(refreshOverviewOnly, 5000);
});
onUnmounted(() => {
  if (overviewTimer !== undefined) clearInterval(overviewTimer);
});
async function start() {
  loading.value = true;
  message.value = "";
  try {
    status.value = await proxyStart();
    message.value = status.value.port_note || "代理已启动";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    loading.value = false;
  }
}

async function stop() {
  loading.value = true;
  message.value = "";
  try {
    status.value = await proxyStop();
    message.value = "代理已停止";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    loading.value = false;
  }
}

async function copyBaseUrl() {
  if (!status.value?.base_url) return;
  await navigator.clipboard.writeText(status.value.base_url);
  message.value = "Base URL 已复制";
}

const exampleCurl = () => {
  const base = status.value?.base_url || "http://127.0.0.1:8888";
  return `curl ${base}/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"hi"}]}'`;
};

</script>

<template>
  <div class="space-y-6">
    <StatsCards :overview="overview" :error="overviewError" />

    <Card class="border border-slate-200 bg-white">
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">每日请求量（近一年）</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <p class="mb-3 text-xs text-slate-500">
        按本地自然日聚合的请求总条数。
      </p>
      <Heatmap :values="heatmapData" />
      <p v-if="dailyError" class="mt-3 text-sm text-rose-600">{{ dailyError }}</p>
    </CardContent>
    </Card>

    <Card class="border border-slate-200 bg-white">
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">本地代理</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <div class="grid gap-3 text-sm md:grid-cols-2">
        <div>
          <div class="text-slate-500">状态</div>
          <div class="mt-1 font-medium">
            <Badge :variant="statusBadgeVariant === 'danger' ? 'destructive' : statusBadgeVariant === 'success' ? 'secondary' : 'outline'">
              {{ status?.state || "未知" }}
            </Badge>
          </div>
        </div>
        <div>
          <div class="text-slate-500">Base URL</div>
          <div class="mt-1 flex items-center gap-2 font-mono text-sm">
            <span>{{ status?.base_url || "-" }}</span>
            <Button variant="secondary" size="sm" type="button" @click="copyBaseUrl">复制</Button>
          </div>
        </div>
        <div>
          <div class="text-slate-500">监听</div>
          <div class="mt-1 font-mono">{{ status?.host }}:{{ status?.port }}</div>
        </div>
      </div>

      <div class="mt-5 flex flex-wrap items-center gap-3">
        <Button variant="secondary" type="button" :disabled="loading" @click="start">启动</Button>
        <Button variant="danger" type="button" :disabled="loading" @click="stop">停止</Button>
        <Button variant="outline" type="button" @click="refresh">刷新</Button>
      </div>

      <p v-if="message" class="mt-3 whitespace-pre-line text-sm text-emerald-700">{{ message }}</p>
      <p
        v-if="status?.port_note && status.port_note !== message"
        class="mt-2 text-sm text-amber-800"
      >
        {{ status.port_note }}
      </p>
      <p v-if="error || status?.last_error" class="mt-3 text-sm text-rose-600">
        {{ error || status?.last_error }}
      </p>
      <p class="mt-2 text-xs text-slate-500">
        关闭窗口会隐藏到系统托盘，代理继续运行；仅托盘菜单「退出」会停止代理并释放端口。若首选端口被占用，会自动向后寻找可用端口；若意外多开旧实例，请在旧进程托盘选择「退出」。端口配置和数据目录可在「设置」页查看。
      </p>
    </CardContent>
    </Card>

    <Card class="border border-slate-200 bg-white">
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">本机接入步骤</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <ol class="list-decimal space-y-2 pl-5 text-sm text-slate-700">
        <li>
          <span class="font-medium">启动代理</span>
          ：确认状态为 running，记下或复制 Base URL（默认 127.0.0.1）。
        </li>
        <li>
          <span class="font-medium">新建供应商</span>
          ：填写上游 Base URL 与 API Key，并启用。
        </li>
        <li>
          <span class="font-medium">新建分组与队列</span>
          ：分组名即客户端 model；按优先级添加供应商与上游模型，失败时按队列顺序自动故障转移。
        </li>
        <li>
          <span class="font-medium">客户端 / curl 调用</span>
          ：Base URL 用本机地址，Authorization 可省略，body 中 model 填分组名。
        </li>
      </ol>
      <p class="mt-3 text-xs text-slate-500">
        完整可勾选验收步骤见仓库
        <code class="rounded bg-slate-100 px-1">docs/local-acceptance.md</code>。
      </p>
    </CardContent>
    </Card>

    <Card class="border border-slate-200 bg-white">
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">调用示例</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <p class="mb-2 text-sm text-slate-500">
        客户端使用统一 Base URL；请求体中的 model 填分组名，无需配置客户端密钥。
      </p>
      <pre class="overflow-x-auto rounded-lg bg-slate-900 p-4 text-xs text-slate-100">{{ exampleCurl() }}</pre>
    </CardContent>
    </Card>
  </div>
</template>
