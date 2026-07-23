<script setup lang="ts">
import { nextTick, onBeforeUnmount, watch, ref } from "vue";

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
const dialog = ref<HTMLElement | null>(null);
const titleId = `app-dialog-title-${Math.random().toString(36).slice(2)}`;
let restoreFocus: HTMLElement | null = null;

function close() {
  if (!props.closeDisabled) emit("close");
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
    return;
  }
  if (event.key !== "Tab" || !dialog.value) return;
  const focusable = Array.from(
    dialog.value.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"]), [contenteditable="true"]',
    ),
  );
  if (focusable.length === 0) {
    event.preventDefault();
    dialog.value.focus();
    return;
  }
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      document.addEventListener("keydown", onKeydown);
      await nextTick();
      dialog.value?.focus();
    } else {
      document.removeEventListener("keydown", onKeydown);
      await nextTick();
      restoreFocus?.focus();
      restoreFocus = null;
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => document.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center p-4" @click.self="close">
      <div class="absolute inset-0 bg-slate-900/50" aria-hidden="true" @click="close" />
      <section
        ref="dialog"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        tabindex="-1"
        class="relative flex w-full flex-col rounded-xl bg-white shadow-xl outline-none"
        :class="size === 'wide' ? 'max-w-6xl max-h-[90vh]' : 'max-w-xl max-h-[90vh]'"
        @click.stop
      >
        <header class="flex shrink-0 items-center justify-between border-b border-slate-200 px-5 py-4">
          <h2 :id="titleId" class="text-lg font-semibold text-slate-900">{{ title }}</h2>
          <button
            type="button"
            aria-label="关闭对话框"
            class="rounded-lg p-1.5 text-xl leading-none text-slate-500 hover:bg-slate-100 disabled:opacity-40"
            :disabled="closeDisabled"
            @click="close"
          >
            ×
          </button>
        </header>
        <div class="min-h-0 overflow-y-auto p-5"><slot /></div>
      </section>
    </div>
  </Teleport>
</template>
