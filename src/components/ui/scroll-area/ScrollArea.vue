<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import { reactiveOmit } from "@vueuse/core";
import {
  ScrollAreaCorner,
  ScrollAreaRoot,
  ScrollAreaViewport,
  type ScrollAreaRootProps,
} from "reka-ui";
import { cn } from "@/lib/utils";
import { ScrollBar } from ".";

const props = withDefaults(
  defineProps<ScrollAreaRootProps & { class?: HTMLAttributes["class"] }>(),
  {
    type: "hover",
    orientation: "vertical",
    scrollHideDelay: 600,
  },
);

const delegatedProps = reactiveOmit(props, "class");
</script>

<template>
  <ScrollAreaRoot
    v-bind="delegatedProps"
    :class="cn('relative overflow-hidden', props.class)"
  >
    <ScrollAreaViewport class="h-full w-full rounded-[inherit]">
      <slot />
    </ScrollAreaViewport>
    <ScrollBar />
    <ScrollAreaCorner />
  </ScrollAreaRoot>
</template>
