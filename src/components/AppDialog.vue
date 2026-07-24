<script setup lang="ts">
import { computed, nextTick, watch } from "vue";
import { HButton, HDialog } from "happier-ui";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    size?: "default" | "wide";
    closeDisabled?: boolean;
  }>(),
  { size: "default", closeDisabled: false },
);

const emit = defineEmits<{ close: [] }>();

/**
 * 对外 open ↔ HDialog modelValue。
 * closeDisabled 时忽略关闭；仅在 v-model 路径 emit close，避免与 @close 重复。
 */
const modelOpen = computed({
  get: () => props.open,
  set: (value: boolean) => {
    if (!value && !props.closeDisabled) emit("close");
  },
});

const allowClose = computed(() => !props.closeDisabled);

let restoreFocus: HTMLElement | null = null;

watch(
  () => props.open,
  async (open) => {
    if (open) {
      restoreFocus =
        document.activeElement instanceof HTMLElement ? document.activeElement : null;
    } else {
      await nextTick();
      restoreFocus?.focus();
      restoreFocus = null;
    }
  },
);

function requestClose() {
  if (!props.closeDisabled) emit("close");
}
</script>

<template>
  <!-- Teleport 到 body，避免被 AppShell 内容区 overflow 裁切 -->
  <Teleport to="body">
    <HDialog
      v-model="modelOpen"
      :aria-label="title"
      :close-on-overlay="allowClose"
      :close-on-esc="allowClose"
      :class="['app-dialog-host', size === 'wide' ? 'app-dialog-host--wide' : '']"
    >
      <template #title>
        <div class="app-dialog-title-row">
          <h2 class="app-dialog-title">{{ title }}</h2>
          <HButton
            variant="ghost"
            size="sm"
            type="button"
            aria-label="关闭对话框"
            :disabled="closeDisabled"
            @click="requestClose"
          >
            ×
          </HButton>
        </div>
      </template>
      <slot />
    </HDialog>
  </Teleport>
</template>
