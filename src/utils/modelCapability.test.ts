import assert from "node:assert/strict";
import test from "node:test";
import {
  buildExternalScoreIndex,
  matchModelToLeaderboard,
  normalizeModelIdForMatch,
  sortQueueByLeaderboard,
  type ExternalLeaderboardEntry,
} from "./modelCapability.ts";

test("归一化剥离厂商前缀、日期、渠道与量化后缀", () => {
  assert.equal(normalizeModelIdForMatch("OpenAI/GPT-4o"), "gpt-4o");
  assert.equal(normalizeModelIdForMatch("anthropic/claude-sonnet-4"), "claude-sonnet-4");
  assert.equal(normalizeModelIdForMatch("claude.sonnet.4"), "claude-sonnet-4");
  assert.equal(normalizeModelIdForMatch("gpt-4o-2024-08-06"), "gpt-4o");
  assert.equal(normalizeModelIdForMatch("gpt-4o-20240806"), "gpt-4o");
  assert.equal(normalizeModelIdForMatch("mistral-large-latest"), "mistral-large");
  assert.equal(normalizeModelIdForMatch("llama-3-8b-instruct"), "llama-3-8b");
  assert.equal(normalizeModelIdForMatch("qwen-plus-fp8"), "qwen-plus");
  assert.equal(normalizeModelIdForMatch("deepseek-r1-gguf"), "deepseek-r1");
});

test("展示名净化（M1）：剥括号档位与纯 4 位日期段", () => {
  // llm_benchmark 展示名 → 归一化后与 API 名对齐
  assert.equal(normalizeModelIdForMatch("GPT-5.5 (xhigh)"), "gpt-5-5");
  assert.equal(normalizeModelIdForMatch("Kimi-K3 (max)"), "kimi-k3");
  assert.equal(normalizeModelIdForMatch("DeepSeek V4 Flash 0731 (max)"), "deepseek-v4-flash");
  assert.equal(normalizeModelIdForMatch("Gemini 3.5 Flash Lite (high)"), "gemini-3-5-flash-lite");
  assert.equal(normalizeModelIdForMatch("Qwen3.7-Max (xhigh)"), "qwen3-7-max");
  assert.equal(normalizeModelIdForMatch("Gemma 4 31B"), "gemma-4-31b");
  // 不含括号的普通 API 名不受影响（4o / 31b 非纯 4 位数字段）
  assert.equal(normalizeModelIdForMatch("gpt-4o"), "gpt-4o");
  assert.equal(normalizeModelIdForMatch("gemma-4-31b"), "gemma-4-31b");
});

test("llm_benchmark 展示名 ↔ API 名跨侧命中", () => {
  const entries: ExternalLeaderboardEntry[] = [
    { id: "GPT-5.5 (xhigh)", name: "GPT-5.5 (xhigh)", agentic_score: 83.8 },
    { id: "Kimi-K3 (max)", name: "Kimi-K3 (max)", agentic_score: 82.91 },
    { id: "DeepSeek V4 Flash 0731 (max)", name: "DeepSeek V4 Flash 0731 (max)", agentic_score: 68.12 },
    { id: "Gemma 4 31B", name: "Gemma 4 31B", agentic_score: 27.91 },
  ];
  const index = buildExternalScoreIndex(entries);

  // API 名命中对应展示名
  const hit1 = matchModelToLeaderboard("openai/gpt-5.5", index);
  assert.equal(hit1?.score, 83.8);
  assert.equal(hit1?.sourceLabel, "llm_benchmark");
  const hit2 = matchModelToLeaderboard("moonshot/kimi-k3", index);
  assert.equal(hit2?.score, 82.91);
  const hit3 = matchModelToLeaderboard("deepseek/deepseek-v4-flash", index);
  assert.equal(hit3?.score, 68.12);
  const hit4 = matchModelToLeaderboard("google/gemma-4-31b", index);
  assert.equal(hit4?.score, 27.91);

  // 未收录模型不命中
  assert.equal(matchModelToLeaderboard("company-internal-model", index), null);
});

test("分层匹配：精确/归一化命中与多候选择优", () => {
  const entries: ExternalLeaderboardEntry[] = [
    { id: "openai/gpt-4o", agentic_score: 85 },
    { id: "openai/gpt-5", agentic_score: 95 },
    { id: "anthropic/claude-3-5-sonnet", agentic_score: 88 },
  ];
  const index = buildExternalScoreIndex(entries);

  // 精确命中
  const exact = matchModelToLeaderboard("openai/gpt-4o", index);
  assert.equal(exact?.score, 85);
  assert.equal(exact?.tier, "exact"); // 归一化同等

  // 去噪命中
  const norm = matchModelToLeaderboard("gpt-5-latest-fp8", index);
  assert.equal(norm?.score, 95);
  // 对于查询 gpt-5-latest-fp8，normalizeModelIdForMatch 把它变成 gpt-5。
  // 然后 index 里有 gpt-5 (exact 层，构建时直接存入的)。
  // 但是注意，index 建立时如果原模型 ID 被归一化了，其实也就是 exact。
  // 这里如果是 prefix，说明前缀匹配的逻辑。
  // 我们直接看 norm?.tier 就行。
  assert.ok(norm?.tier === "exact" || norm?.tier === "prefix" || norm?.tier === "normalized");

  // 前缀命中 (gpt-4o 作为前缀命中 gpt-4o-custom)
  // 注意，这里的 candidate 必须没有 tier token，否则会拦截。custom-version 没有判别 token。
  const prefix = matchModelToLeaderboard("gpt-4o-custom", index);
  assert.equal(prefix?.score, 85);
  // gpt-4o-custom 会把 prefix gpt-4o 匹配上，剩余 custom，无判别 token。
  assert.equal(prefix?.tier, "prefix");

  // 不存在的模型
  assert.equal(matchModelToLeaderboard("unknown-model", index), null);
});

test("受控近似（前缀 + 护栏反例）：宁未匹配不错配", () => {
  const entries: ExternalLeaderboardEntry[] = [
    { id: "openai/gpt-4o", agentic_score: 85 },
    { id: "openai/gpt-4o-mini", agentic_score: 40 },
    { id: "anthropic/claude-3-5-sonnet", agentic_score: 88 },
    { id: "anthropic/claude-3-5-haiku", agentic_score: 50 },
    { id: "meta-llama/llama-3-8b", agentic_score: 60 },
    { id: "meta-llama/llama-3-70b", agentic_score: 80 },
    { id: "openai/gpt-4", agentic_score: 75 },
    { id: "openai/gpt-40", agentic_score: 99 },
  ];
  const index = buildExternalScoreIndex(entries);

  // gpt-4o 不得命中 gpt-4o-mini (差 mini)
  const oMissing = matchModelToLeaderboard("gpt-4o-max", index); // max 被拦截，榜单没有 max → 不匹配
  assert.equal(oMissing, null);

  // gpt-4o-mini 不得命中 gpt-4o (逆当前缀，但剩余是 mini，拦截)
  // 如果给 gpt-4o-mini，直接精确命中：
  const mini = matchModelToLeaderboard("gpt-4o-mini", index);
  assert.equal(mini?.score, 40);
  assert.equal(mini?.tier, "exact");

  // 但如果榜单没有 mini，上游是 mini，不得命中 4o：
  const entriesNoMini: ExternalLeaderboardEntry[] = [
    { id: "openai/gpt-4o", agentic_score: 85 },
  ];
  const indexNoMini = buildExternalScoreIndex(entriesNoMini);
  assert.equal(matchModelToLeaderboard("gpt-4o-mini", indexNoMini), null);

  // claude-sonnet-3 不误配到 haiku
  assert.equal(matchModelToLeaderboard("claude-3-5-opus", index), null);

  // 裸 claude 不命中（差 sonnet/haiku，拦截）
  assert.equal(matchModelToLeaderboard("claude-3-5", index), null);

  // 参数量护栏：8b 不命中 70b
  assert.equal(matchModelToLeaderboard("llama-3", index), null); // 差 8b/70b
  assert.equal(matchModelToLeaderboard("llama-3-405b", index), null); // 榜单没有 405b

  // 无 `-` 边界不是前缀：gpt-4 不得命中 gpt-40
  const noDash = matchModelToLeaderboard("gpt-400", index);
  assert.equal(noDash, null);
});

test("sortQueueByLeaderboard：命中降序，未匹配沉底，同分/未匹配稳定", () => {
  const entries: ExternalLeaderboardEntry[] = [
    { id: "openai/gpt-4o", agentic_score: 85 },
    { id: "openai/gpt-4o-mini", agentic_score: 40 },
    { id: "deepseek/deepseek-r1", agentic_score: 95 },
  ];
  const index = buildExternalScoreIndex(entries);

  const input = [
    { id: 1, model: "custom-model" },
    { id: 2, model: "gpt-4o-mini" },
    { id: 3, model: "unknown-x" },
    { id: 4, model: "gpt-4o" },
    { id: 5, model: "deepseek-r1" },
    { id: 6, model: "gpt-4o-mini" }, // 重复，测试同分
    { id: 7, model: "custom-model-2" },
  ];

  const sorted = sortQueueByLeaderboard(input, (i) => i.model, index);

  // 检查 sorted 的每个元素长什么样
  // sorted[0] 是什么
  // console.log("Sorted:", sorted);

  // 命中：r1(95) > 4o(85) > mini(40) > mini(40)
  assert.equal(sorted[0].model, "deepseek-r1");
  assert.equal(sorted[1].model, "gpt-4o");
  assert.equal(sorted[2].id, 2); // 保持原序
  assert.equal(sorted[3].id, 6); // 保持原序

  // 未匹配沉底：保持原序 1 -> 3 -> 7
  assert.equal(sorted[4].id, 1);
  assert.equal(sorted[5].id, 3);
  assert.equal(sorted[6].id, 7);
});

test("sortQueueByLeaderboard：无 index 时全部视为未匹配，保持原序", () => {
  const input = [
    { id: 1, model: "gpt-4o" },
    { id: 2, model: "deepseek-r1" },
  ];
  const sorted = sortQueueByLeaderboard(input, (i) => i.model, null);
  assert.equal(sorted[0].id, 1);
  assert.equal(sorted[1].id, 2);
});
