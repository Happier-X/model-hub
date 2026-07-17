import { invoke } from "@tauri-apps/api/core";

export interface AppPaths {
  app_data_dir: string;
  config_dir: string;
  gateway_dir: string;
  bin_dir: string;
}

const browserPreviewPaths: Readonly<AppPaths> = {
  app_data_dir: "浏览器预览模式：请在桌面应用中查看实际路径",
  config_dir: "—",
  gateway_dir: "—",
  bin_dir: "—",
};

export async function getPaths(): Promise<AppPaths> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    return { ...browserPreviewPaths };
  }

  return invoke<AppPaths>("get_paths");
}
