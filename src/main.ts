import { createApp } from "vue";
import App from "./App.vue";
import OverlayApp from "./OverlayApp.vue";
import { router } from "./router";
// index.css 内含 `@import "tailwindcss"`，必须先加载以声明 CSS layer 顺序
// （theme, base, components, utilities）。若 happier-ui 的 styles.css 先加载，
// 其裸 `@layer components` 会被排到最前，导致 Tailwind preflight（base 层）
// 反而覆盖 .h-button 等组件样式。顺序敏感，勿调换。
import "./index.css";
import "happier-ui/tokens.css";
import "happier-ui/styles.css";

const isOverlay = new URLSearchParams(window.location.search).get("overlay") === "1";

if (isOverlay) {
  document.documentElement.classList.add("overlay-root");
  createApp(OverlayApp).mount("#app");
} else {
  createApp(App).use(router).mount("#app");
}
