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

export async function gatewayRequest<T>(
  method: string,
  path: string,
  body?: unknown,
  options?: { auth?: boolean },
): Promise<T> {
  const base = resolveBaseUrl();
  const url = `${base}${path.startsWith("/") ? path : `/${path}`}`;
  const headers = new Headers();
  if (body !== undefined) {
    headers.set("Content-Type", "application/json");
  }
  const useAuth = options?.auth !== false;
  if (useAuth && adminToken) {
    headers.set("Authorization", `Bearer ${adminToken}`);
  }

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

export const gatewayHttp = {
  get: <T>(path: string) => gatewayRequest<T>("GET", path),
  post: <T>(path: string, body?: unknown) => gatewayRequest<T>("POST", path, body),
  delete: <T>(path: string) => gatewayRequest<T>("DELETE", path),
  postPublic: <T>(path: string, body?: unknown) =>
    gatewayRequest<T>("POST", path, body, { auth: false }),
};
