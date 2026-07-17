import {
  gatewayHttp,
  getAdminToken,
  loadManualToken,
  setAdminToken,
} from "./gatewayHttp";

export interface SilentAuthResult {
  ok: boolean;
  source: "manual" | "silent" | "none";
  message: string;
}

interface LoginResponse {
  token: string;
  expire_at: string;
}

const DEFAULT_USER = "admin";
const DEFAULT_PASSWORD = "admin";
const DEFAULT_EXPIRE_SECONDS = 86400;

export async function ensureAdminSession(): Promise<SilentAuthResult> {
  loadManualToken();
  if (getAdminToken()) {
    try {
      await gatewayHttp.get<string>("/api/v1/user/status");
      return {
        ok: true,
        source: "manual",
        message: "已使用本地保存的管理 Token",
      };
    } catch {
      // fall through to silent login
    }
  }

  try {
    const data = await gatewayHttp.postPublic<LoginResponse>("/api/v1/user/login", {
      username: DEFAULT_USER,
      password: DEFAULT_PASSWORD,
      expire: DEFAULT_EXPIRE_SECONDS,
    });
    setAdminToken(data.token, false);
    return {
      ok: true,
      source: "silent",
      message: "已静默登录本机侧车（默认 admin，无登录页）",
    };
  } catch (error: unknown) {
    const detail = error instanceof Error ? error.message : String(error);
    setAdminToken(null, false);
    return {
      ok: false,
      source: "none",
      message: `管理 API 鉴权失败：${detail}。可在设置中粘贴管理 Token，或确认侧车默认账号未改密。`,
    };
  }
}

export function applyManualAdminToken(token: string): void {
  const trimmed = token.trim();
  if (!trimmed) {
    setAdminToken(null, true);
    return;
  }
  setAdminToken(trimmed, true);
}
