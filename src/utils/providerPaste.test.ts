import assert from "node:assert/strict";
import test from "node:test";
import { describeProviderPasteSource, parseProviderPaste } from "./providerPaste.ts";

test("解析 NewAPI 标准分享格式", () => {
  const result = parseProviderPaste(
    '{"_type":"newapi_channel_conn","key":"sk-newapi-example-abcdefghijklmnop","url":"https://free.lyclaude.site"}',
  );
  assert.ok(result);
  assert.equal(result.source, "json");
  assert.equal(result.baseUrl, "https://free.lyclaude.site");
  assert.equal(result.apiKey, "sk-newapi-example-abcdefghijklmnop");
  assert.equal(result.suggestedName, "free");
  assert.equal(result.warnings.length, 0);
  assert.equal(describeProviderPasteSource(result.source), "JSON / NewAPI");
});

test("解析环境变量", () => {
  const result = parseProviderPaste(`
OPENAI_BASE_URL=https://api.openai.com/v1/
OPENAI_API_KEY=sk-test-key-1234567890
`);
  assert.ok(result);
  assert.equal(result.source, "env");
  assert.equal(result.baseUrl, "https://api.openai.com/v1");
  assert.equal(result.apiKey, "sk-test-key-1234567890");
});

test("解析 curl", () => {
  const result = parseProviderPaste(`
curl https://example.com/v1/chat/completions \\
  -H "Authorization: Bearer sk-curl-abcdefghijklmn"
`);
  assert.ok(result);
  assert.equal(result.source, "curl");
  assert.equal(result.baseUrl, "https://example.com/v1/chat/completions");
  assert.equal(result.apiKey, "sk-curl-abcdefghijklmn");
});

test("解析普通两行文本", () => {
  const result = parseProviderPaste(`https://demo.example.com/v1
sk-plain-abcdefghijkl`);
  assert.ok(result);
  assert.equal(result.source, "text");
  assert.equal(result.baseUrl, "https://demo.example.com/v1");
  assert.equal(result.apiKey, "sk-plain-abcdefghijkl");
});

test("无法识别时返回 null", () => {
  assert.equal(parseProviderPaste("hello world"), null);
  assert.equal(parseProviderPaste(""), null);
});
