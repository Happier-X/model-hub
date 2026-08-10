import {
  defineComponent,
  h,
  onMounted,
  onUnmounted,
  ref,
  watch,
  type PropType,
} from "vue";
import { formatNumber, formatTokenCount } from "@/utils/formatTokenCount";

type AnimatedKind = "count" | "token" | "raw";

function formatByKind(value: number, kind: AnimatedKind): string {
  if (kind === "count") return formatNumber(value);
  if (kind === "token") return formatTokenCount(value);
  return String(value);
}

/**
 * 数值滚动动画：number 时用 requestAnimationFrame 从上一值平滑滚动（600ms easeOutCubic）；
 * string 时直接渲染（用于耗时/费用等已格式化文本）。
 */
export const AnimatedNumber = defineComponent({
  name: "AnimatedNumber",
  props: {
    value: { type: [Number, String] as PropType<number | string>, required: true },
    kind: { type: String as PropType<AnimatedKind>, default: "raw" },
  },
  setup(props) {
    const display = ref(0);
    let raf = 0;
    let startedAt = 0;
    const duration = 600;

    const animate = (from: number, to: number) => {
      cancelAnimationFrame(raf);
      startedAt = performance.now();
      const step = (now: number) => {
        const progress = Math.min(1, (now - startedAt) / duration);
        const eased = 1 - Math.pow(1 - progress, 3);
        display.value = Math.round(from + (to - from) * eased);
        if (progress < 1) raf = requestAnimationFrame(step);
      };
      raf = requestAnimationFrame(step);
    };

    onMounted(() => {
      if (typeof props.value === "number") animate(0, props.value);
    });
    watch(
      () => props.value,
      (value) => {
        if (typeof value === "number") animate(display.value, value);
      },
    );
    onUnmounted(() => cancelAnimationFrame(raf));

    return () =>
      h(
        "span",
        { class: "tabular-nums" },
        typeof props.value === "number"
          ? formatByKind(display.value, props.kind)
          : props.value,
      );
  },
});
