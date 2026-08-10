import { createApp } from "vue";
import App from "./App.vue";
import OverlayApp from "./OverlayApp.vue";
import { router } from "./router";
// index.css 内含 `@import "tailwindcss"` + shadcn-vue 主题变量，加载顺序：
// 先 index.css（含 tailwind + tw-animate + shadcn-vue/tailwind.css），再应用级样式。
import "./index.css";
// vue3-calendar-heatmap 的 vch__* 方块/图例样式（组件自身不内联样式）
import "vue3-calendar-heatmap/dist/style.css";

const isOverlay = new URLSearchParams(window.location.search).get("overlay") === "1";

if (isOverlay) {
  document.documentElement.classList.add("overlay-root");
  createApp(OverlayApp).mount("#app");
} else {
  createApp(App).use(router).mount("#app");
}
