export interface ProviderPasteResult {
  baseUrl: string;
  apiKey: string;
  suggestedName: string;
  source: "json" | "env" | "curl" | "text";
  warnings: string[];
}

const BASE_URL_KEYS = ["base_url", "baseUrl", "url", "endpoint", "api_base", "apiBase"];
const API_KEY_KEYS = ["api_key", "apiKey", "key", "token", "secret"];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function stringField(record: Record<string, unknown>, keys: string[]): string {
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "string" && value.trim()) return value.trim();
  }
  return "";
}

function normalizeBaseUrl(raw: string): string {
  return raw.trim().replace(/^['"`]+|['"`,;]+$/g, "").replace(/\/+$/g, "");
}

function normalizeKey(raw: string): string {
  return raw.trim().replace(/^['"`]+|['"`,;]+$/g, "");
}

function looksLikeUrl(value: string): boolean {
  return /^https?:\/\/[^\s'"<>]+/i.test(value.trim());
}

function extractFirstUrl(text: string): string {
  const match = text.match(/https?:\/\/[^\s'"<>),;]+/i);
  return match ? normalizeBaseUrl(match[0]) : "";
}

function extractBearerKey(text: string): string {
  const bearer = text.match(/authorization\s*:\s*bearer\s+([^\s'"`\\]+)/i);
  if (bearer?.[1]) return normalizeKey(bearer[1]);
  const headerBearer = text.match(/Bearer\s+([^\s'"`\\]+)/i);
  if (headerBearer?.[1]) return normalizeKey(headerBearer[1]);
  return "";
}

function extractHeaderKey(text: string): string {
  const header = text.match(/(?:x-api-key|api-key)\s*:\s*([^\s'"`\\]+)/i);
  if (header?.[1]) return normalizeKey(header[1]);
  return "";
}

function extractPlainKey(text: string): string {
  const envLike = text.match(/(?:OPENAI_API_KEY|API_KEY|api_key|apiKey|key|token)\s*[:=]\s*['"]?([^\s'"`,}]+)/i);
  if (envLike?.[1]) return normalizeKey(envLike[1]);
  // 覆盖 sk-*、ak-*、sk-xxx:yyy 等常见 Key，不要求特定供应商前缀。
  const key = text.match(/\b(?:sk|ak|rk|pk)-[A-Za-z0-9._:-]{12,}\b/);
  if (key?.[0]) return normalizeKey(key[0]);
  return "";
}

function extractEnvValue(text: string, names: string[]): string {
  for (const name of names) {
    const re = new RegExp(`(?:^|\\n)\\s*(?:export\\s+)?${name}\\s*=\\s*['"]?([^'"\\n]+)`, "i");
    const match = text.match(re);
    if (match?.[1]) return match[1].trim();
  }
  return "";
}

function suggestedNameFromUrl(baseUrl: string): string {
  try {
    const host = new URL(baseUrl).hostname.replace(/^www\./, "");
    const first = host.split(".")[0];
    return first || host || "自定义供应商";
  } catch {
    return "自定义供应商";
  }
}

function buildResult(
  baseUrl: string,
  apiKey: string,
  source: ProviderPasteResult["source"],
): ProviderPasteResult | null {
  const normalizedUrl = normalizeBaseUrl(baseUrl);
  const normalizedKey = normalizeKey(apiKey);
  if (!normalizedUrl && !normalizedKey) return null;
  const warnings: string[] = [];
  if (!normalizedUrl) warnings.push("未识别到 Base URL");
  if (!normalizedKey) warnings.push("未识别到 API Key");
  if (normalizedUrl && !looksLikeUrl(normalizedUrl)) warnings.push("Base URL 看起来不是 http(s) 地址");
  return {
    baseUrl: normalizedUrl,
    apiKey: normalizedKey,
    suggestedName: normalizedUrl ? suggestedNameFromUrl(normalizedUrl) : "自定义供应商",
    source,
    warnings,
  };
}

function parseJson(text: string): ProviderPasteResult | null {
  const trimmed = text.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) return null;
  try {
    const parsed: unknown = JSON.parse(trimmed);
    const record = Array.isArray(parsed) ? parsed.find(isRecord) : parsed;
    if (!isRecord(record)) return null;
    const baseUrl = stringField(record, BASE_URL_KEYS);
    const apiKey = stringField(record, API_KEY_KEYS);
    return buildResult(baseUrl, apiKey, "json");
  } catch {
    return null;
  }
}

function parseEnv(text: string): ProviderPasteResult | null {
  const baseUrl = extractEnvValue(text, [
    "OPENAI_BASE_URL",
    "OPENAI_API_BASE",
    "OPENAI_API_URL",
    "BASE_URL",
    "API_BASE_URL",
  ]);
  const apiKey = extractEnvValue(text, ["OPENAI_API_KEY", "API_KEY", "OPENAI_KEY"]);
  return buildResult(baseUrl, apiKey, "env");
}

function parseCurl(text: string): ProviderPasteResult | null {
  if (!/\bcurl\b|-H\s+['"]/i.test(text)) return null;
  const baseUrl = extractFirstUrl(text);
  const apiKey = extractBearerKey(text) || extractHeaderKey(text) || extractPlainKey(text);
  return buildResult(baseUrl, apiKey, "curl");
}

function parsePlainText(text: string): ProviderPasteResult | null {
  const baseUrl = extractFirstUrl(text);
  const apiKey = extractBearerKey(text) || extractHeaderKey(text) || extractPlainKey(text);
  return buildResult(baseUrl, apiKey, "text");
}

export function parseProviderPaste(text: string): ProviderPasteResult | null {
  if (!text.trim()) return null;
  return parseJson(text) || parseEnv(text) || parseCurl(text) || parsePlainText(text);
}

export function describeProviderPasteSource(source: ProviderPasteResult["source"]): string {
  switch (source) {
    case "json":
      return "JSON / NewAPI";
    case "env":
      return "环境变量";
    case "curl":
      return "curl / Header";
    case "text":
      return "普通文本";
  }
}
