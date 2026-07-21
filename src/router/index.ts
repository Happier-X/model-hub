import { createRouter, createWebHistory } from "vue-router";
import OverviewPage from "../pages/OverviewPage.vue";
import ProvidersPage from "../pages/ProvidersPage.vue";
import GroupsPage from "../pages/GroupsPage.vue";
import ApiKeysPage from "../pages/ApiKeysPage.vue";
import LogsPage from "../pages/LogsPage.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "overview", component: OverviewPage, meta: { title: "概览" } },
    { path: "/providers", name: "providers", component: ProvidersPage, meta: { title: "供应商" } },
    { path: "/groups", name: "groups", component: GroupsPage, meta: { title: "分组" } },
    { path: "/api-keys", name: "api-keys", component: ApiKeysPage, meta: { title: "API Key" } },
    { path: "/logs", name: "logs", component: LogsPage, meta: { title: "日志" } },
  ],
});
