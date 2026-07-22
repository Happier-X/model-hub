export interface ModelCapability {
  /** 仅用于队列启发式排序；不是官方基准分。未识别固定为 0。 */
  score: number;
  label: string;
  family: string;
  recognized: boolean;
}

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
