import { defineComponent, h, onMounted, onUnmounted, ref, watch } from "vue";

/**
 * octopus 同款数值滚动：入参为已格式化的 value 字符串（如 "12.35"、"500.00"），
 * 内部 parseFloat 后 rAF 滚动（600ms easeOutCubic），按原串是否含小数点显示 0/2 位小数。
 * 单位（unit）由调用方单独渲染。
 */
export const AnimatedNumber = defineComponent({
  name: "AnimatedNumber",
  props: {
    value: { type: String, required: true },
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
        display.value = from + (to - from) * eased;
        if (progress < 1) raf = requestAnimationFrame(step);
      };
      raf = requestAnimationFrame(step);
    };

    const targetOf = (value: string): number => {
      const parsed = parseFloat(value.replace(/,/g, ""));
      return Number.isFinite(parsed) ? parsed : 0;
    };

    onMounted(() => {
      animate(0, targetOf(props.value));
    });
    watch(
      () => props.value,
      (value) => {
        animate(display.value, targetOf(value));
      },
    );
    onUnmounted(() => cancelAnimationFrame(raf));

    return () => {
      const decimals = props.value.includes(".") ? 2 : 0;
      return h(
        "span",
        { class: "tabular-nums" },
        display.value.toLocaleString("en-US", {
          minimumFractionDigits: decimals,
          maximumFractionDigits: decimals,
        }),
      );
    };
  },
});
