import { invoke } from "@tauri-apps/api/core";

export interface AppPaths {
  app_data_dir: string;
  config_dir: string;
  gateway_dir: string;
  bin_dir: string;
}

export type GatewayPhase =
  | "idle"
  | "starting"
  | "running"
  | "stopping"
  | "error";

export interface GatewayStatus {
  state: GatewayPhase;
  host: string;
  port: number;
  pid: number | null;
  last_error: string | null;
  base_url: string;
  data_dir: string;
  binary_path: string | null;
  /** 网关实现：`octopus` | `rust`；旧壳可能缺省 */
  impl_name?: string;
}

const browserPreviewPaths: Readonly<AppPaths> = {
  app_data_dir: "浏览器预览模式：请在桌面应用中查看实际路径",
  config_dir: "—",
  gateway_dir: "—",
  bin_dir: "—",
};

const browserPreviewGateway: Readonly<GatewayStatus> = {
  state: "idle",
  host: "127.0.0.1",
  port: 8080,
  pid: null,
  last_error: "浏览器预览模式：网关启停仅在桌面应用中可用",
  base_url: "http://127.0.0.1:8080",
  data_dir: "—",
  binary_path: null,
};

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function formatInvokeError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object") {
    const record = error as { message?: unknown; code?: unknown };
    if (typeof record.message === "string") {
      return record.message;
    }
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

export async function getPaths(): Promise<AppPaths> {
  if (!isTauriRuntime()) {
    return { ...browserPreviewPaths };
  }

  return invoke<AppPaths>("get_paths");
}

export async function gatewayStatus(): Promise<GatewayStatus> {
  if (!isTauriRuntime()) {
    return { ...browserPreviewGateway };
  }

  return invoke<GatewayStatus>("gateway_status");
}

export async function gatewayStart(): Promise<GatewayStatus> {
  if (!isTauriRuntime()) {
    throw new Error("浏览器预览模式无法启动网关，请使用 pnpm tauri dev。");
  }

  try {
    return await invoke<GatewayStatus>("gateway_start");
  } catch (error) {
    throw new Error(formatInvokeError(error));
  }
}

export async function gatewayStop(): Promise<GatewayStatus> {
  if (!isTauriRuntime()) {
    throw new Error("浏览器预览模式无法停止网关，请使用 pnpm tauri dev。");
  }

  try {
    return await invoke<GatewayStatus>("gateway_stop");
  } catch (error) {
    throw new Error(formatInvokeError(error));
  }
}

export async function gatewaySetPort(port: number): Promise<GatewayStatus> {
  if (!isTauriRuntime()) {
    throw new Error("浏览器预览模式无法保存网关端口，请使用桌面应用。");
  }

  try {
    return await invoke<GatewayStatus>("gateway_set_port", { port });
  } catch (error) {
    throw new Error(formatInvokeError(error));
  }
}

export function gatewayStateLabel(state: GatewayPhase): string {
  switch (state) {
    case "idle":
      return "未运行";
    case "starting":
      return "启动中";
    case "running":
      return "运行中";
    case "stopping":
      return "停止中";
    case "error":
      return "错误";
    default:
      return state;
  }
}
