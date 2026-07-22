<script setup lang="ts">
import { computed } from "vue";
import type { HealthSnapshot } from "../api/tauri";
import { healthStateClass, healthStateLabel } from "../utils/health";

const props = defineProps<{
  snapshot?: HealthSnapshot | null;
}>();

const state = computed(() => props.snapshot?.state);
const label = computed(() => healthStateLabel(state.value));
const badgeClass = computed(() => healthStateClass(state.value));
const failures = computed(() => props.snapshot?.consecutive_failures ?? 0);
</script>

<template>
  <span class="inline-flex flex-wrap items-center gap-1.5">
    <span
      class="inline-flex rounded-full px-2 py-0.5 text-xs font-medium"
      :class="badgeClass"
    >
      {{ label }}
    </span>
    <span class="text-xs text-slate-500" title="连续失败次数">
      失败 {{ failures }}
    </span>
  </span>
</template>
