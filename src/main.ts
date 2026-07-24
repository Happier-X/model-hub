import { createApp } from "vue";
import App from "./App.vue";
import OverlayApp from "./OverlayApp.vue";
import { router } from "./router";
import "happier-ui/tokens.css";
import "happier-ui/style.css";
import "./index.css";

const isOverlay = new URLSearchParams(window.location.search).get("overlay") === "1";

if (isOverlay) {
  document.documentElement.classList.add("overlay-root");
  createApp(OverlayApp).mount("#app");
} else {
  createApp(App).use(router).mount("#app");
}
