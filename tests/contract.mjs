import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import test from "node:test";
import { get } from "node:http";
import { join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chat, post, startProxy } from "./harness.mjs";

const chatPath = "/v1/chat/completions";
const messagePath = "/v1/messages";
const secret = "local-key";
const bearer = { Authorization: "Bearer " + secret };

async function proxy(t, kind, options) {
  const service = await startProxy(kind, options);
  t.after(service.stop);
  return service;
}

async function json(response, status) {
  assert.equal(response.status, status);
  return response.json();
}

async function completion(service, prompt, options) {
  return json(await post(service, chatPath, chat(prompt), options), 200);
}

test("standalone health, model listing, chat and Anthropic messages", async (t) => {
  const bridge = await proxy(t, "rust");
  assert.deepEqual(await json(await fetch(bridge.url + "/healthz"), 200), {
    ok: true, service: "agy-droidbridge",
  });
  const catalog = await json(await fetch(bridge.url + "/v1/models"), 200);
  assert.equal(catalog.object, "list");
  assert(catalog.data.length > 0);
  const model = catalog.data[0].id;
  const reply = await json(await post(bridge, chatPath, {
    model,
    messages: [
      { role: "developer", content: "Be concise." },
      { role: "user", content: [{ type: "text", text: "Say hello." }] },
    ],
  }), 200);
  assert.equal(reply.object, "chat.completion");
  assert.equal(reply.choices[0].message.content, "Mock reply: System: Be concise.\n\nUser: Say hello.");
  const message = await json(await post(bridge, messagePath, chat("hello", { system: "Be concise." })), 200);
  assert.equal(message.type, "message");
  assert.equal(message.content[0].text, "Mock reply: System: Be concise.\n\nUser: hello");
  assert.equal((await fetch(bridge.url + "/missing")).status, 404);
});

test("unsupported request features fail clearly and unsuccessful models do not stick", async (t) => {
  const bridge = await proxy(t, "rust");
  for (const [request, reason] of [
    [chat([{ type: "image_url", image_url: { url: "https://example.test/image" } }]), /text content/i],
    [chat("hello", { tools: [{ type: "function" }] }), /tool calling/i],
    [chat("hello", { n: 2 }), /one completion/i],
    [chat("hello", { response_format: { type: "json_object" } }), /text responses/i],
  ]) {
    const error = await json(await post(bridge, chatPath, request), 400);
    assert.match(error.error.message, reason);
  }
  const first = "gemini-3.5-flash-low";
  await json(await post(bridge, chatPath, chat("MOCK_MODEL", { model: first })), 200);
  await json(await post(bridge, chatPath, chat("MOCK_FAIL", { model: "gemini-3.5-flash-high" })), 502);
  assert.equal((await completion(bridge, "MOCK_MODEL")).choices[0].message.content, first);
});

test("the model-list validator rejects invalid catalog files", async (t) => {
  const bridge = await proxy(t, "rust");
  const executable = process.env.AGY_TEST_RUST_BIN;
  await writeFile(bridge.settings, "gemini-3.8-flash-medium  Gemini 3.8 Flash (Medium)\n");
  const valid = spawnSync(executable, ["--check-models", bridge.settings], { encoding: "utf8" });
  assert.equal(valid.status, 0);
  assert.match(valid.stdout, /1 models/);
  await writeFile(bridge.settings, "--invalid flag\n");
  const invalid = spawnSync(executable, ["--check-models", bridge.settings], { encoding: "utf8" });
  assert.equal(invalid.status, 1);
  assert.match(invalid.stderr, /Invalid model list/);
});

test("a persistent workspace is reusable and refuses a changed agent profile", async (t) => {
  const bridge = await proxy(t, "rust");
  const workspace = join(bridge.settings, "..", "workspace");
  const executable = process.env.AGY_TEST_RUST_BIN;
  const args = ["--prepare-workspace", workspace];
  assert.equal(spawnSync(executable, args).status, 0);
  assert.equal(spawnSync(executable, args).status, 0);
  const profile = join(workspace, ".agents", "agents", "agy-droidbridge-text.md");
  assert.match(await readFile(profile, "utf8"), /tools: \[\]/);
  await writeFile(profile, "local modification\n");
  assert.equal(spawnSync(executable, args).status, 1);
  assert.equal(await readFile(profile, "utf8"), "local modification\n");
});

test("optional comparison with the reference server", { skip: !process.env.AGY_TEST_NODE_ENTRY }, async (t) => {
  const rust = await proxy(t, "rust");
  const node = await proxy(t, "node");
  for (const path of ["/", "/healthz?x=y"]) {
    assert.equal((await json(await fetch(rust.url + path), 200)).ok, true);
    assert.equal((await json(await fetch(node.url + path), 200)).ok, true);
  }
  const [rustModels, nodeModels] = await Promise.all([rust, node].map(async (server) => {
    const models = await json(await fetch(server.url + "/v1/models"), 200);
    assert.equal(models.object, "list");
    assert.equal(models.data.length, 11);
    for (const model of models.data) {
      assert(Number.isInteger(model.created));
      delete model.created;
    }
    return models;
  }));
  assert.deepEqual(rustModels, nodeModels);

  const messages = [
    { role: "system", content: "System instructions" },
    { role: "assistant", content: "Earlier 😃" },
    { role: "tool", content: [{ type: "text", text: "Tool result" }] },
    { role: "user", content: ["First", { type: "text", text: "Second" }] },
  ];
  for (const [path, request] of [
    [chatPath, { model: "Gemini-3.5-Flash", messages }],
    [messagePath, { model: "gpt-oss-120b", system: [{ type: "text", text: "Be brief" }], messages, max_tokens: 20 }],
  ]) {
    const responses = await Promise.all([rust, node].map(async (server) => {
      const response = await json(await post(server, path, request), 200);
      assert.match(response.id, path === chatPath ? /^chatcmpl-/ : /^msg_/);
      delete response.id;
      delete response.created;
      return response;
    }));
    assert.deepEqual(...responses);
  }
  assert.deepEqual(JSON.parse(await readFile(rust.settings, "utf8")), {});
  assert.equal(JSON.parse(await readFile(node.settings, "utf8")).model, "GPT-OSS 120B (Medium)");
  assert.equal((await completion(rust, "MOCK_MODEL")).choices[0].message.content, "gpt-oss-120b-medium");

  for (const server of [rust, node]) {
    assert.equal((await fetch(server.url + "/does-not-exist")).status, 404);
    assert.equal((await fetch(server.url + chatPath, { method: "OPTIONS" })).status, 204);
    assert.equal((await post(server, chatPath, "{invalid")).status, 400);
    assert.equal((await post(server, chatPath, {})).status, 400);
    assert.equal((await post(server, chatPath, chat("bad", { model: "../private/file" }))).status, 400);
    assert.equal((await post(server, messagePath, chat("stream", { stream: true }))).status, 400);
    assert.equal((await post(server, chatPath, chat("MOCK_FAIL"))).status, 502);
    assert.equal((await post(server, chatPath, chat("MOCK_EMPTY"))).status, 502);
  }
  assert.equal((await post(node, chatPath, chat("stream", { stream: true }))).status, 400);
});

test("auth, browser isolation, body limits and literal prompt delivery", async (t) => {
  const rust = await proxy(t, "rust", { AGY_API_KEY: secret, AGY_MAX_BODY_BYTES: "512", AGY_MAX_PROMPT_BYTES: "100" });
  for (const headers of [{}, { Authorization: "Bearer wrong" }, { "x-api-key": secret }]) {
    assert.equal((await fetch(rust.url + "/v1/models", { headers })).status, 401);
  }
  assert.equal((await fetch(rust.url + "/v1/models", { headers: { ...bearer, Origin: "https://other.example" } })).status, 403);
  const rebindingStatus = await new Promise((resolve, reject) => {
    get(rust.url + "/v1/models", { headers: { ...bearer, Host: "other.example" } }, (response) => {
      response.resume();
      resolve(response.statusCode);
    }).on("error", reject);
  });
  assert.equal(rebindingStatus, 403);
  assert.equal((await fetch(rust.url + "/v1/models", { headers: bearer })).status, 200);
  assert.equal((await post(rust, messagePath, chat("test"), { headers: { "x-api-key": secret } })).status, 200);
  assert.equal((await post(rust, messagePath, chat("test"), { headers: { ...bearer, "x-api-key": secret } })).status, 401);
  assert.equal((await post(rust, chatPath, chat("a".repeat(540)), { headers: bearer })).status, 413);
  assert.equal((await post(rust, chatPath, chat("😀".repeat(50)), { headers: bearer })).status, 413);
  assert.equal((await post(rust, chatPath, chat("ABC\u0000DEF"), { headers: bearer })).status, 400);
  const literal = "Quotes \" and ' ; $(echo marker) \x60test\x60 & | > < \\\\ ";
  const reply = await completion(rust, literal, { headers: bearer });
  assert.equal(reply.choices[0].message.content, ("Mock reply: User: " + literal).trim());
});

test("OpenAI buffered SSE emits role, JSON-safe text, finish, usage and terminator", async (t) => {
  const rust = await proxy(t, "rust");
  const response = await post(rust, chatPath, chat("hello\nsecond line", { stream: true, stream_options: { include_usage: true } }));
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type"), /^text\/event-stream/);
  const events = (await response.text()).trimEnd().split("\n\n").map((event) => event.slice(6));
  assert.equal(events.pop(), "[DONE]");
  const chunks = events.map((event) => JSON.parse(event));
  assert.equal(new Set(chunks.map((chunk) => chunk.id)).size, 1);
  assert.equal(chunks[0].choices[0].delta.role, "assistant");
  assert.equal(chunks[1].choices[0].delta.content, "Mock reply: User: hello\nsecond line");
  assert.equal(chunks[2].choices[0].finish_reason, "stop");
  assert.equal(chunks[3].choices.length, 0);
  assert(chunks[3].usage.total_tokens > 0);
});

test("sanitized CLI errors and account-specific model catalog", async (t) => {
  const absent = await proxy(t, "rust", { AGY_BIN: "missing-agy-executable" });
  assert.equal((await post(absent, chatPath, chat("hi"))).status, 500);
  const config = await proxy(t, "rust");
  await writeFile(config.settings, "gemini-3.8-flash-medium  Gemini 3.8 Flash (Medium)\n");
  const rust = await proxy(t, "rust", { AGY_MAX_OUTPUT_BYTES: "1024", AGY_MODELS_FILE: config.settings });
  const models = await json(await fetch(rust.url + "/v1/models"), 200);
  assert.deepEqual(models.data.map((model) => model.id), ["gemini-3.8-flash-medium"]);
  const accountModel = await json(await post(rust, chatPath, chat("MOCK_MODEL", { model: models.data[0].id })), 200);
  assert.equal(accountModel.choices[0].message.content, models.data[0].id);
  for (const marker of ["MOCK_FAIL", "MOCK_AUTH", "MOCK_QUOTA", "MOCK_CRASH", "MOCK_MALFORMED", "MOCK_EMPTY", "MOCK_LARGE"]) {
    const body = await json(await post(rust, chatPath, chat(marker)), 502);
    assert.doesNotMatch(JSON.stringify(body), /SECRET-FIXTURE|private\/mock|not an envelope|MOCK_/i);
  }
  assert.doesNotMatch(rust.logs(), /SECRET-FIXTURE|private\/mock|UNRELATED_PRIVATE_KEY|MOCK_/i);
});

test("bounded queue, serialized children and deadline or disconnect recovery", async (t) => {
  const rust = await proxy(t, "rust", { AGY_TIMEOUT_SECS: "1", AGY_MAX_PENDING: "1" });
  const results = await Promise.all(Array.from({ length: 6 }, () => post(rust, chatPath, chat("MOCK_SERIAL"))));
  assert.equal(results.filter((result) => result.status === 503).length, 4);
  const intervals = await Promise.all(results.filter((result) => result.ok).map(async (result) => {
    const body = await result.json();
    return body.choices[0].message.content.split(":").map(Number);
  }));
  intervals.sort(([a], [b]) => a - b);
  assert.equal(intervals.length, 2);
  assert(intervals[1][0] >= intervals[0][1]);
  assert.equal((await post(rust, chatPath, chat("MOCK_TIMEOUT"))).status, 504);
  assert.equal((await completion(rust, "after timeout")).choices[0].message.content, "Mock reply: User: after timeout");

  const cancel = new AbortController();
  const outstanding = post(rust, chatPath, chat("MOCK_TIMEOUT"), { signal: cancel.signal });
  await delay(150);
  cancel.abort();
  await assert.rejects(outstanding, { name: "AbortError" });
  const started = performance.now();
  await completion(rust, "after disconnect");
  assert(performance.now() - started < 650, "disconnected child should release the execution slot");
});
