import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import "happier-ui/tokens.css";
import "happier-ui/style.css";
import "./index.css";

createApp(App).use(router).mount("#app");
