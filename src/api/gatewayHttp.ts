export class GatewayHttpError extends Error {
  readonly status: number;
  readonly code?: string;

  constructor(message: string, status: number, code?: string) {
    super(message);
    this.name = "GatewayHttpError";
    this.status = status;
    this.code = code;
  }
}

export type GatewayAuthMode = "admin" | "none" | "bearer";

export interface GatewayRequestOptions {
  /**
   * 鉴权模式。默认 admin。
   * - admin：附带管理 JWT
   * - none：不带 Authorization
   * - bearer：使用 options.bearer（客户端 sk-octopus，**不得**回落 adminToken）
   */
  authMode?: GatewayAuthMode;
  /** 兼容旧调用：auth:false 等价 authMode:'none' */
  auth?: boolean;
  /** authMode=bearer 时使用的客户端 Key */
  bearer?: string;
}

export interface ClientProbeResult {
  status: number;
  ok: boolean;
  message: string;
  body: unknown;
}

let adminToken: string | null = null;
let baseUrlProvider: (() => string | null) | null = null;

const MANUAL_TOKEN_KEY = "model-hub.admin-token";

export function setBaseUrlProvider(provider: () => string | null): void {
  baseUrlProvider = provider;
}

export function getAdminToken(): string | null {
  return adminToken;
}

export function setAdminToken(token: string | null, persistManual = false): void {
  adminToken = token;
  if (persistManual && token) {
    window.localStorage.setItem(MANUAL_TOKEN_KEY, token);
  }
  if (!token) {
    window.localStorage.removeItem(MANUAL_TOKEN_KEY);
  }
}

export function loadManualToken(): string | null {
  const token = window.localStorage.getItem(MANUAL_TOKEN_KEY);
  if (token) {
    adminToken = token;
  }
  return token;
}

function resolveBaseUrl(): string {
  const base = baseUrlProvider?.() ?? null;
  if (!base) {
    throw new GatewayHttpError("网关未就绪：缺少 base_url。请先启动网关。", 0);
  }
  return base.replace(/\/$/, "");
}

async function parseBody(response: Response): Promise<unknown> {
  const contentType = response.headers.get("content-type") ?? "";
  if (contentType.includes("application/json")) {
    return response.json();
  }
  return response.text();
}

function extractErrorMessage(data: unknown, fallback: string): string {
  if (data && typeof data === "object" && "message" in data) {
    const message = (data as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) {
      return message;
    }
  }
  if (typeof data === "string" && data.trim()) {
    return data;
  }
  return fallback;
}

function resolveAuthMode(options?: GatewayRequestOptions): GatewayAuthMode {
  if (options?.authMode) {
    return options.authMode;
  }
  if (options?.auth === false) {
    return "none";
  }
  return "admin";
}

function applyAuthHeaders(
  headers: Headers,
  mode: GatewayAuthMode,
  bearer?: string,
): void {
  if (mode === "none") {
    return;
  }
  if (mode === "bearer") {
    const key = bearer?.trim();
    if (!key) {
      throw new GatewayHttpError("缺少客户端 API Key（bearer）。", 0);
    }
    headers.set("Authorization", `Bearer ${key}`);
    return;
  }
  if (adminToken) {
    headers.set("Authorization", `Bearer ${adminToken}`);
  }
}

export async function gatewayRequest<T>(
  method: string,
  path: string,
  body?: unknown,
  options?: GatewayRequestOptions,
): Promise<T> {
  const base = resolveBaseUrl();
  const url = `${base}${path.startsWith("/") ? path : `/${path}`}`;
  const headers = new Headers();
  if (body !== undefined) {
    headers.set("Content-Type", "application/json");
  }
  applyAuthHeaders(headers, resolveAuthMode(options), options?.bearer);

  const response = await fetch(url, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  const data = await parseBody(response);
  if (!response.ok) {
    throw new GatewayHttpError(
      extractErrorMessage(data, response.statusText || "请求失败"),
      response.status,
    );
  }

  if (data && typeof data === "object" && "data" in data) {
    return (data as { data: T }).data;
  }
  return data as T;
}

/**
 * 客户端路径探测：使用用户提供的网关 Key，**永不**附带管理 JWT。
 * 不抛出 HTTP 错误，便于 UI 展示状态码。
 */
export async function clientProbe(
  method: string,
  path: string,
  options: { bearer: string; body?: unknown },
): Promise<ClientProbeResult> {
  const base = resolveBaseUrl();
  const url = `${base}${path.startsWith("/") ? path : `/${path}`}`;
  const headers = new Headers();
  const key = options.bearer.trim();
  if (!key) {
    return {
      status: 0,
      ok: false,
      message: "请填写网关 API Key（sk-octopus-...）",
      body: null,
    };
  }
  headers.set("Authorization", `Bearer ${key}`);
  if (options.body !== undefined) {
    headers.set("Content-Type", "application/json");
  }

  try {
    const response = await fetch(url, {
      method,
      headers,
      body:
        options.body === undefined ? undefined : JSON.stringify(options.body),
    });
    const data = await parseBody(response);
    return {
      status: response.status,
      ok: response.ok,
      message: response.ok
        ? "成功"
        : extractErrorMessage(data, response.statusText || "请求失败"),
      body: data,
    };
  } catch (err: unknown) {
    return {
      status: 0,
      ok: false,
      message: err instanceof Error ? err.message : String(err),
      body: null,
    };
  }
}

export const gatewayHttp = {
  get: <T>(path: string) => gatewayRequest<T>("GET", path),
  post: <T>(path: string, body?: unknown) => gatewayRequest<T>("POST", path, body),
  delete: <T>(path: string) => gatewayRequest<T>("DELETE", path),
  postPublic: <T>(path: string, body?: unknown) =>
    gatewayRequest<T>("POST", path, body, { authMode: "none" }),
  withClientKey: <T>(
    method: string,
    path: string,
    bearer: string,
    body?: unknown,
  ) =>
    gatewayRequest<T>(method, path, body, { authMode: "bearer", bearer }),
};
