import { createRouter, createWebHistory } from "vue-router";
import HomePage from "../pages/HomePage.vue";
import ProvidersPage from "../pages/ProvidersPage.vue";
import GroupsPage from "../pages/GroupsPage.vue";
import LogsPage from "../pages/LogsPage.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "home", component: HomePage, meta: { title: "首页" } },
    { path: "/providers", name: "providers", component: ProvidersPage, meta: { title: "供应商" } },
    { path: "/groups", name: "groups", component: GroupsPage, meta: { title: "分组" } },
    { path: "/logs", name: "logs", component: LogsPage, meta: { title: "日志" } },
  ],
});
