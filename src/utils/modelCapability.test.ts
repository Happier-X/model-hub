import assert from "node:assert/strict";
import test from "node:test";
import {
  buildExternalScoreIndex,
  EXTERNAL_SORT_BASE,
  hybridSortKey,
  matchExternalScore,
  normalizeModelIdForMatch,
  scoreModelCapability,
  sortByHybridCapability,
  sortByModelCapability,
  type ExternalLeaderboardEntry,
} from "./modelCapability.ts";

test("Claude 定位顺序为 Opus > Sonnet > Haiku", () => {
  const opus = scoreModelCapability("claude-opus-4");
  const sonnet = scoreModelCapability("claude-sonnet-4");
  const haiku = scoreModelCapability("claude-3-5-haiku");
  assert.ok(opus.score > sonnet.score);
  assert.ok(sonnet.score > haiku.score);
});

test("主流模型按启发式能力降序", () => {
  const input = [
    "custom-unknown-model",
    "gpt-4o-mini",
    "deepseek-r1",
    "gpt-5",
    "gemini-2.5-pro",
  ];
  const sorted = sortByModelCapability(input, (id) => id);
  assert.deepEqual(sorted, [
    "gpt-5",
    "deepseek-r1",
    "gemini-2.5-pro",
    "gpt-4o-mini",
    "custom-unknown-model",
  ]);
});

test("Qwen 与 Llama 参数量越大分数越高", () => {
  assert.ok(scoreModelCapability("qwen2.5-72b").score > scoreModelCapability("qwen2.5-7b").score);
  assert.ok(scoreModelCapability("llama-3.1-405b").score > scoreModelCapability("llama-3.1-8b").score);
});

test("识别常见连字符版本、路径前缀与 Mistral", () => {
  const sonnet37 = scoreModelCapability("claude-3-7-sonnet").score;
  const sonnet35 = scoreModelCapability("claude-3-5-sonnet").score;
  assert.equal(sonnet37 > sonnet35, true, `expected ${sonnet37} > ${sonnet35}`);
  assert.equal(scoreModelCapability("vendor/o3-mini").family, "openai");
  assert.equal(scoreModelCapability("mistral-large-latest").label, "Mistral Large");
});

test("未知模型排在后面且保持相对顺序", () => {
  const input = ["unknown-b", "claude-sonnet-4", "unknown-a", "unknown-c"];
  const sorted = sortByModelCapability(input, (id) => id);
  assert.deepEqual(sorted, ["claude-sonnet-4", "unknown-b", "unknown-a", "unknown-c"]);
  assert.equal(scoreModelCapability("unknown-b").recognized, false);
});

test("同分模型稳定排序", () => {
  const input = [
    { id: 1, model: "claude-sonnet-4" },
    { id: 2, model: "claude-sonnet-4" },
    { id: 3, model: "claude-sonnet-4" },
  ];
  const sorted = sortByModelCapability(input, (item) => item.model);
  assert.deepEqual(
    sorted.map((item) => item.id),
    [1, 2, 3],
  );
});

test("归一化剥离厂商前缀、日期与分隔符变体", () => {
  assert.equal(normalizeModelIdForMatch("OpenAI/GPT-4o"), "gpt-4o");
  assert.equal(normalizeModelIdForMatch("anthropic/claude-sonnet-4"), "claude-sonnet-4");
  assert.equal(normalizeModelIdForMatch("claude.sonnet.4"), "claude-sonnet-4");
  assert.equal(normalizeModelIdForMatch("gpt-4o-2024-08-06"), "gpt-4o");
  assert.equal(normalizeModelIdForMatch("gpt-4o-20240806"), "gpt-4o");
  assert.equal(normalizeModelIdForMatch("mistral-large-latest"), "mistral-large");
});

test("高置信匹配：跨厂商 id 对齐且不模糊误配", () => {
  const entries: ExternalLeaderboardEntry[] = [
    {
      id: "anthropic/claude-sonnet-4",
      canonical_slug: "anthropic/claude-sonnet-4",
      name: "Claude Sonnet 4",
      intelligence_score: 72.5,
      coding_score: 68,
    },
    {
      id: "openai/gpt-5",
      intelligence_score: 90,
      coding_score: 88,
    },
  ];
  const index = buildExternalScoreIndex(entries, "intelligence");
  assert.equal(matchExternalScore("claude-sonnet-4", index)?.score, 72.5);
  assert.equal(matchExternalScore("anthropic/claude-sonnet-4", index)?.score, 72.5);
  assert.equal(matchExternalScore("gpt-5", index)?.score, 90);
  // 不得模糊匹配到 sonnet
  assert.equal(matchExternalScore("claude-sonnet-3", index), null);
  assert.equal(matchExternalScore("claude", index), null);
  assert.equal(matchExternalScore("unknown-model", index), null);
});

test("编码指标索引与通用能力索引独立", () => {
  const entries: ExternalLeaderboardEntry[] = [
    {
      id: "openai/gpt-5",
      intelligence_score: 90,
      coding_score: 50,
    },
    {
      id: "deepseek/deepseek-r1",
      intelligence_score: 70,
      coding_score: 95,
    },
  ];
  const intel = buildExternalScoreIndex(entries, "intelligence");
  const coding = buildExternalScoreIndex(entries, "coding");
  assert.equal(matchExternalScore("gpt-5", intel)?.score, 90);
  assert.equal(matchExternalScore("gpt-5", coding)?.score, 50);
  assert.equal(matchExternalScore("deepseek-r1", coding)?.score, 95);
});

test("混合排序：外部命中优先，未匹配回退本地，稳定同 key", () => {
  const entries: ExternalLeaderboardEntry[] = [
    { id: "openai/gpt-4o-mini", intelligence_score: 40, coding_score: 30 },
    { id: "deepseek/deepseek-r1", intelligence_score: 85, coding_score: 80 },
  ];
  const index = buildExternalScoreIndex(entries, "intelligence");
  const input = [
    "custom-unknown-model",
    "gpt-4o-mini",
    "gpt-5", // 无外部条目，走本地（很高）
    "deepseek-r1",
    "another-unknown",
  ];
  const sorted = sortByHybridCapability(input, (id) => id, index);

  // deepseek-r1 外部 85 → key 远高于本地 gpt-5
  assert.equal(sorted[0], "deepseek-r1");
  // gpt-5 仅本地启发式；外部 key = 1_000_000 + 40*1000 = 1_040_000，本地 gpt-5 ≈ 930 → 外部命中整体仍更高
  assert.equal(sorted[1], "gpt-4o-mini");
  assert.equal(sorted[2], "gpt-5");
  // 未知模型保持相对顺序
  assert.deepEqual(sorted.slice(3), ["custom-unknown-model", "another-unknown"]);

  const hit = hybridSortKey("deepseek-r1", index);
  assert.ok(hit.external);
  assert.equal(hit.key, EXTERNAL_SORT_BASE + 85 * 1000);

  const miss = hybridSortKey("gpt-5", index);
  assert.equal(miss.external, null);
  assert.equal(miss.key, scoreModelCapability("gpt-5").score);
});

test("无外部索引时混合排序等价于本地", () => {
  const input = ["gpt-4o-mini", "gpt-5", "unknown-x"];
  const local = sortByModelCapability(input, (id) => id);
  const hybrid = sortByHybridCapability(input, (id) => id, null);
  assert.deepEqual(hybrid, local);
});

test("混合排序同分稳定", () => {
  const entries: ExternalLeaderboardEntry[] = [
    { id: "a/model-x", intelligence_score: 50 },
    { id: "b/model-y", intelligence_score: 50 },
  ];
  const index = buildExternalScoreIndex(entries, "intelligence");
  const input = [
    { id: 1, model: "model-x" },
    { id: 2, model: "model-y" },
    { id: 3, model: "model-x" },
  ];
  const sorted = sortByHybridCapability(input, (i) => i.model, index);
  assert.deepEqual(
    sorted.map((i) => i.id),
    [1, 2, 3],
  );
});
