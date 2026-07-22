export interface ModelCapability {
  /** 仅用于队列启发式排序；不是官方基准分。未识别固定为 0。 */
  score: number;
  label: string;
  family: string;
  recognized: boolean;
}

/** 外部榜单一条模型记录（与 IPC 白名单字段对齐）。 */
export interface ExternalLeaderboardEntry {
  id: string;
  canonical_slug?: string | null;
  name?: string | null;
  intelligence_score?: number | null;
  coding_score?: number | null;
  agentic_score?: number | null;
}

export type ExternalSortMetric = "intelligence" | "coding";

export type QueueSortMode = "local" | "external_intelligence" | "external_coding";

export interface MatchedExternalScore {
  /** 用于展示的分数（对应指标）。 */
  score: number;
  /** 榜单条目 id。 */
  leaderboardId: string;
  sourceLabel: string;
}

/** 外部命中后的排序 key 基数，确保高于本地启发式（本地通常 < 1000）。 */
export const EXTERNAL_SORT_BASE = 1_000_000;

function includesAny(name: string, values: string[]): boolean {
  return values.some((value) => name.includes(value));
}

function parameterBonus(name: string): number {
  // 识别 405b / 72b / 32b / 7b；MoE 的 8x7b 取最后一个参数量作为保守参考。
  const matches = [...name.matchAll(/(?:^|[-_/\s])([0-9]+(?:\.[0-9]+)?)b(?:$|[-_/\s])/g)];
  const raw = matches.at(-1)?.[1];
  if (!raw) return 0;
  const billions = Number(raw);
  if (!Number.isFinite(billions)) return 0;
  if (billions >= 400) return 140;
  if (billions >= 200) return 120;
  if (billions >= 100) return 105;
  if (billions >= 70) return 90;
  if (billions >= 30) return 65;
  if (billions >= 13) return 40;
  if (billions >= 7) return 20;
  return 5;
}

function variantPenalty(name: string): number {
  let penalty = 0;
  if (includesAny(name, ["nano", "tiny"])) penalty -= 230;
  else if (includesAny(name, ["mini", "small", "lite"])) penalty -= 130;
  if (includesAny(name, ["preview", "experimental", "exp-"])) penalty -= 10;
  return penalty;
}

function result(score: number, label: string, family: string): ModelCapability {
  return { score: Math.max(1, Math.round(score)), label, family, recognized: true };
}

export function scoreModelCapability(modelId: string): ModelCapability {
  const name = modelId.trim().toLowerCase();
  if (!name) return { score: 0, label: "未识别", family: "unknown", recognized: false };

  // Claude：系列定位优先，版本用于小幅区分。
  if (name.includes("claude")) {
    // 用版本锚点，避免 `includes("4")` 误伤其它片段。
    const version = /(?:^|[-_/])4(?:\.1|-1)(?:$|[-_/])/.test(name)
      ? 45
      : /(?:^|[-_/])4(?:$|[-_/])/.test(name)
        ? 30
        : /(?:^|[-_/])3(?:\.7|-7)(?:$|[-_/])/.test(name)
          ? 20
          : /(?:^|[-_/])3(?:\.5|-5)(?:$|[-_/])/.test(name)
            ? 10
            : 0;
    if (name.includes("opus")) return result(900 + version, "Claude Opus", "claude");
    if (name.includes("sonnet")) return result(760 + version, "Claude Sonnet", "claude");
    if (name.includes("haiku")) return result(470 + version, "Claude Haiku", "claude");
    return result(620 + version, "Claude", "claude");
  }

  // OpenAI GPT / o 系列。
  if (/\bgpt[-_/]?5\b/.test(name) || name.startsWith("gpt-5")) {
    return result(930 + variantPenalty(name), "GPT-5", "openai");
  }
  if (/\bgpt[-_/]?4\.1\b/.test(name) || name.includes("gpt-4.1")) {
    return result(840 + variantPenalty(name), "GPT-4.1", "openai");
  }
  if (name.includes("gpt-4o")) {
    return result(790 + variantPenalty(name), "GPT-4o", "openai");
  }
  if (/(?:^|[-_/])(?:openai[-_/])?o[134](?:[-_/]|$)/.test(name)) {
    const base = name.includes("o3") ? 880 : name.includes("o4") ? 890 : 820;
    return result(base + variantPenalty(name), "OpenAI 推理", "openai");
  }
  if (name.includes("gpt-4")) return result(720 + variantPenalty(name), "GPT-4", "openai");
  if (name.includes("gpt-3.5")) return result(380 + variantPenalty(name), "GPT-3.5", "openai");

  // Gemini：Pro > Flash，版本越新略优。
  if (name.includes("gemini")) {
    const version = name.includes("2.5") ? 60 : name.includes("2.0") || name.includes("2-") ? 35 : name.includes("1.5") ? 15 : 0;
    if (name.includes("pro")) return result(790 + version + variantPenalty(name), "Gemini Pro", "gemini");
    if (name.includes("flash")) return result(610 + version + variantPenalty(name), "Gemini Flash", "gemini");
    return result(650 + version + variantPenalty(name), "Gemini", "gemini");
  }

  if (name.includes("deepseek")) {
    if (includesAny(name, ["r1", "reasoner"])) return result(860 + variantPenalty(name), "DeepSeek 推理", "deepseek");
    if (includesAny(name, ["v3", "chat"])) return result(740 + variantPenalty(name), "DeepSeek V3/Chat", "deepseek");
    if (name.includes("coder")) return result(650 + variantPenalty(name), "DeepSeek Coder", "deepseek");
    return result(620 + variantPenalty(name), "DeepSeek", "deepseek");
  }

  if (includesAny(name, ["qwen", "qwq"])) {
    const params = parameterBonus(name);
    if (includesAny(name, ["max", "qwq"])) return result(800 + params, "Qwen Max/推理", "qwen");
    if (name.includes("plus")) return result(680 + params, "Qwen Plus", "qwen");
    if (name.includes("turbo")) return result(500 + params, "Qwen Turbo", "qwen");
    return result(560 + params + variantPenalty(name), "Qwen", "qwen");
  }

  if (name.includes("llama")) {
    return result(500 + parameterBonus(name) + variantPenalty(name), "Llama", "llama");
  }

  if (includesAny(name, ["mistral", "mixtral", "codestral"])) {
    const params = parameterBonus(name);
    if (name.includes("large")) return result(690 + params, "Mistral Large", "mistral");
    if (name.includes("medium")) return result(570 + params, "Mistral Medium", "mistral");
    return result(460 + params + variantPenalty(name), "Mistral", "mistral");
  }

  return { score: 0, label: "未识别", family: "unknown", recognized: false };
}

/** 稳定降序：同分保持输入顺序；未识别（0 分）自然排在已识别之后。 */
export function sortByModelCapability<T>(
  items: readonly T[],
  getModelId: (item: T) => string,
): T[] {
  return items
    .map((item, index) => ({ item, index, score: scoreModelCapability(getModelId(item)).score }))
    .sort((a, b) => b.score - a.score || a.index - b.index)
    .map(({ item }) => item);
}

/** 常见厂商前缀（匹配时剥离，便于跨供应商 id 对齐）。 */
const VENDOR_PREFIXES = [
  "anthropic",
  "openai",
  "google",
  "google-ai-studio",
  "meta-llama",
  "meta",
  "mistralai",
  "mistral",
  "deepseek",
  "qwen",
  "alibaba",
  "x-ai",
  "xai",
  "cohere",
  "perplexity",
  "nvidia",
  "microsoft",
  "amazon",
  "ai21",
  "01-ai",
  "together",
  "fireworks",
  "groq",
  "openrouter",
  "vendor",
] as const;

/**
 * 高置信模型名归一化：小写、去厂商前缀、去日期/版本杂音后缀、统一分隔符。
 * 仅用于匹配，不用于展示。
 */
export function normalizeModelIdForMatch(raw: string): string {
  let s = raw.trim().toLowerCase();
  if (!s) return "";

  // 路径/命名空间：保留最后一段为主，同时记录全路径去前缀后的形式。
  s = s.replace(/\\/g, "/");
  // 统一分隔符为 `-`
  s = s.replace(/[_.\s]+/g, "-");
  s = s.replace(/\/+/g, "/");

  // 反复剥离已知厂商前缀（`openai/gpt-4o` → `gpt-4o`）
  let changed = true;
  while (changed) {
    changed = false;
    for (const vendor of VENDOR_PREFIXES) {
      const withSlash = `${vendor}/`;
      if (s.startsWith(withSlash)) {
        s = s.slice(withSlash.length);
        changed = true;
        break;
      }
    }
  }

  // 去掉路径中仍残留的段前缀，只保留最后一段再归一化一次
  if (s.includes("/")) {
    const parts = s.split("/").filter(Boolean);
    s = parts[parts.length - 1] ?? s;
  }

  // 去常见部署/渠道后缀
  s = s.replace(
    /-(?:latest|prod|production|stable|beta|alpha|chat|instruct|it|hf|gguf|fp8|fp16|bf16|int4|int8|awq|gptq)$/g,
    "",
  );

  // 去日期后缀：-20241022 / -2024-10-22 / -202410
  s = s.replace(/-\d{4}-\d{2}-\d{2}$/g, "");
  s = s.replace(/-\d{8}$/g, "");
  s = s.replace(/-\d{6}$/g, "");

  // 压缩连续分隔符
  s = s.replace(/-+/g, "-").replace(/^-|-$/g, "");
  return s;
}

function metricScore(
  entry: ExternalLeaderboardEntry,
  metric: ExternalSortMetric,
): number | null {
  const raw = metric === "coding" ? entry.coding_score : entry.intelligence_score;
  if (raw == null || !Number.isFinite(raw)) return null;
  return raw;
}

/**
 * 构建外部榜单查找表：归一化 key → 最佳条目（同 key 取更高分）。
 * 仅索引有对应指标分数的条目。
 */
export function buildExternalScoreIndex(
  models: readonly ExternalLeaderboardEntry[],
  metric: ExternalSortMetric,
): Map<string, MatchedExternalScore> {
  const index = new Map<string, MatchedExternalScore>();

  const consider = (key: string, entry: ExternalLeaderboardEntry, score: number) => {
    if (!key) return;
    const prev = index.get(key);
    if (!prev || score > prev.score) {
      index.set(key, {
        score,
        leaderboardId: entry.id,
        sourceLabel: "OpenRouter",
      });
    }
  };

  for (const entry of models) {
    const score = metricScore(entry, metric);
    if (score == null) continue;

    const keys = new Set<string>();
    keys.add(normalizeModelIdForMatch(entry.id));
    if (entry.canonical_slug) keys.add(normalizeModelIdForMatch(entry.canonical_slug));
    if (entry.name) keys.add(normalizeModelIdForMatch(entry.name));
    // 也索引「去掉厂商后的 id 最后一段」已由 normalize 完成

    for (const key of keys) {
      consider(key, entry, score);
    }
  }
  return index;
}

/**
 * 高置信匹配：仅当本地模型归一化 key 与榜单某 key **完全相等** 时命中。
 * 不做子串/模糊匹配，避免错配。
 */
export function matchExternalScore(
  modelId: string,
  index: Map<string, MatchedExternalScore>,
): MatchedExternalScore | null {
  const key = normalizeModelIdForMatch(modelId);
  if (!key) return null;
  return index.get(key) ?? null;
}

/** 混合排序 key：外部分命中 → `1_000_000 + score * 1000`；否则本地启发式。 */
export function hybridSortKey(
  modelId: string,
  index: Map<string, MatchedExternalScore> | null,
): { key: number; external: MatchedExternalScore | null; local: ModelCapability } {
  const local = scoreModelCapability(modelId);
  if (index) {
    const external = matchExternalScore(modelId, index);
    if (external) {
      return {
        key: EXTERNAL_SORT_BASE + external.score * 1_000,
        external,
        local,
      };
    }
  }
  return { key: local.score, external: null, local };
}

/**
 * 稳定混合排序：外部命中按外部分；未命中/无分回退本地启发式；同 key 保持原序。
 */
export function sortByHybridCapability<T>(
  items: readonly T[],
  getModelId: (item: T) => string,
  index: Map<string, MatchedExternalScore> | null,
): T[] {
  return items
    .map((item, indexIn) => ({
      item,
      indexIn,
      key: hybridSortKey(getModelId(item), index).key,
    }))
    .sort((a, b) => b.key - a.key || a.indexIn - b.indexIn)
    .map(({ item }) => item);
}
