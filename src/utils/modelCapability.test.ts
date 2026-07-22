import assert from "node:assert/strict";
import test from "node:test";
import { scoreModelCapability, sortByModelCapability } from "./modelCapability.ts";

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
