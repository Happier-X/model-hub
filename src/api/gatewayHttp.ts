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

export interface GatewayRequestOptions {
  /** 兼容旧调用；本地开放模式忽略鉴权。 */
  authMode?: "admin" | "none" | "bearer";
  auth?: boolean;
  bearer?: string;
}

export interface ClientProbeResult {
  status: number;
  ok: boolean;
  message: string;
  body: unknown;
}

let baseUrlProvider: (() => string | null) | null = null;

export function setBaseUrlProvider(provider: () => string | null): void {
  baseUrlProvider = provider;
}

/** @deprecated 本地开放模式无管理 Token */
export function getAdminToken(): string | null {
  return null;
}

/** @deprecated 本地开放模式无管理 Token */
export function setAdminToken(_token?: string | null, _persistManual?: boolean): void {
  void _token;
  void _persistManual;
}

/** @deprecated 本地开放模式无管理 Token */
export function loadManualToken(): string | null {
  return null;
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

async function gatewayRequest<T>(
  method: string,
  path: string,
  body?: unknown,
  options?: GatewayRequestOptions,
): Promise<T> {
  void options;
  const base = resolveBaseUrl();
  const url = `${base}${path.startsWith("/") ? path : `/${path}`}`;
  const headers = new Headers({ Accept: "application/json" });
  const init: RequestInit = { method, headers };
  if (body !== undefined) {
    headers.set("Content-Type", "application/json");
    init.body = JSON.stringify(body);
  }

  let response: Response;
  try {
    response = await fetch(url, init);
  } catch (cause: unknown) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    throw new GatewayHttpError(
      `无法连接网关（${detail}）。请确认状态条显示运行中，且请求地址与设置端口一致（默认 127.0.0.1）。若 8080 被其他程序占用，请在设置中更换端口。`,
      0,
    );
  }

  const data = await parseBody(response);
  if (!response.ok) {
    throw new GatewayHttpError(
      extractErrorMessage(data, `请求失败 HTTP ${response.status}`),
      response.status,
    );
  }

  if (data && typeof data === "object" && "data" in data) {
    return (data as { data: T }).data;
  }
  return data as T;
}

export const gatewayHttp = {
  get: <T>(path: string, options?: GatewayRequestOptions) =>
    gatewayRequest<T>("GET", path, undefined, options),
  post: <T>(path: string, body?: unknown, options?: GatewayRequestOptions) =>
    gatewayRequest<T>("POST", path, body, options),
  put: <T>(path: string, body?: unknown, options?: GatewayRequestOptions) =>
    gatewayRequest<T>("PUT", path, body, options),
  delete: <T>(path: string, options?: GatewayRequestOptions) =>
    gatewayRequest<T>("DELETE", path, undefined, options),
  postPublic: <T>(path: string, body?: unknown) =>
    gatewayRequest<T>("POST", path, body, { authMode: "none" }),
  withBearer: <T>(
    method: string,
    path: string,
    body: unknown | undefined,
    bearer?: string,
  ) => {
    void bearer;
    return gatewayRequest<T>(method, path, body, { authMode: "none" });
  },
};

/**
 * 客户端路径探测：本地开放模式无需 API Key。
 */
export async function clientProbe(
  path: string,
  _clientKey?: string,
  init?: { method?: string; body?: unknown },
): Promise<ClientProbeResult> {
  try {
    const base = resolveBaseUrl();
    const url = `${base}${path.startsWith("/") ? path : `/${path}`}`;
    const method = init?.method ?? "GET";
    const headers = new Headers({ Accept: "application/json" });
    const requestInit: RequestInit = { method, headers };
    if (init?.body !== undefined) {
      headers.set("Content-Type", "application/json");
      requestInit.body = JSON.stringify(init.body);
    }
    const response = await fetch(url, requestInit);
    const body = await parseBody(response);
    return {
      status: response.status,
      ok: response.ok,
      message: response.ok
        ? "请求成功"
        : extractErrorMessage(body, `HTTP ${response.status}`),
      body,
    };
  } catch (cause: unknown) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    return {
      status: 0,
      ok: false,
      message: detail,
      body: null,
    };
  }
}
