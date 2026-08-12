<script setup lang="ts">
import { computed, nextTick, watch } from "vue";
import { XIcon } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

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
 * 对外 open ↔ Dialog 开合。
 * closeDisabled 时忽略关闭；仅在 v-model 路径 emit close，避免与 @close 重复。
 */
const modelOpen = computed({
  get: () => props.open,
  set: (value: boolean) => {
    if (!value && !props.closeDisabled) emit("close");
  },
});

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
  <!-- 宿主 class 挂在外层：Dialog 内容宽度由 class 控制 -->
  <Teleport to="body">
    <Dialog v-model:open="modelOpen">
      <DialogContent
        :class="size === 'wide' ? 'max-w-3xl' : 'max-w-lg'"
        :close-on-esc="!closeDisabled"
        :close-on-overlay="!closeDisabled"
        :show-close-button="false"
      >
        <DialogHeader>
          <DialogTitle>
            <div class="app-dialog-title-row">
              <h2 class="app-dialog-title">{{ title }}</h2>
            </div>
          </DialogTitle>
        </DialogHeader>
        <slot />
        <DialogClose
          as-child
          :disabled="closeDisabled"
          @click="requestClose"
        >
          <Button
            variant="ghost"
            size="sm"
            type="button"
            class="absolute right-4 top-4 size-7 rounded-sm p-0 opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            aria-label="关闭对话框"
          >
            <XIcon class="size-4" />
          </Button>
        </DialogClose>
      </DialogContent>
    </Dialog>
  </Teleport>
</template>
