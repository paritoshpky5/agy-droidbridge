import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

async function availablePort() {
  const socket = createServer().listen(0, "127.0.0.1");
  await once(socket, "listening");
  const { port } = socket.address();
  await new Promise((done) => socket.close(done));
  return port;
}

export async function startProxy(kind, overrides = {}) {
  const env = Object.fromEntries(Object.entries(process.env).filter(([name]) => !/^AGY_|^NODE_OPTIONS$/i.test(name)));
  const fixture = process.env.AGY_TEST_MOCK_BIN;
  const entry = process.env[kind === "rust" ? "AGY_TEST_RUST_BIN" : "AGY_TEST_NODE_ENTRY"];
  if (!fixture || !entry) throw new Error("Set AGY_TEST_MOCK_BIN and the matching proxy executable or Node entry.");
  const directory = await mkdtemp(join(tmpdir(), "agy-contract-"));
  const settings = join(directory, "settings.json");
  await writeFile(settings, "{}\n");
  const port = await availablePort();
  const url = "http://127.0.0.1:" + port;
  const start = performance.now();
  const child = spawn(kind === "rust" ? resolve(entry) : process.execPath,
    kind === "rust" ? [] : [resolve(entry)], {
      cwd: directory,
      env: {
        ...env, AGY_BIN: resolve(fixture), AGY_SETTINGS_PATH: settings,
        AGY_PORT: String(port), AGY_HOST: "127.0.0.1",
        UNRELATED_PRIVATE_KEY: "not-a-real-key",
        ...overrides,
      },
      stdio: ["ignore", "pipe", "pipe"],
    });
  let logs = "";
  for (const output of [child.stdout, child.stderr]) {
    output.setEncoding("utf8").on("data", (chunk) => {
      logs = (logs + chunk).slice(-65_536);
    });
  }
  let processError;
  const exited = new Promise((done) => {
    child.once("error", (error) => { processError = error; done(); });
    child.once("close", done);
  });

  async function stop() {
    child.kill();
    await exited;
    await rm(directory, { recursive: true, force: true });
  }

  try {
    for (let attempt = 0; attempt < 160; attempt++) {
      if (processError || child.exitCode !== null || child.signalCode !== null) {
        throw processError ?? new Error("Proxy exited early: " + logs);
      }
      try {
        if ((await fetch(url + "/healthz", { signal: AbortSignal.timeout(500) })).ok) {
          return { url, pid: child.pid, settings, startupMs: performance.now() - start, stop, logs: () => logs };
        }
      } catch (error) {
        if (!["TypeError", "TimeoutError"].includes(error.name)) throw error;
      }
      await delay(25);
    }
    throw new Error("Proxy did not start: " + logs);
  } catch (error) {
    await stop();
    throw error;
  }
}

export function chat(content, other = {}) {
  return { messages: [{ role: "user", content }], ...other };
}

export function post(service, path, value, options = {}) {
  const { headers, ...other } = options;
  return fetch(service.url + path, {
    method: "POST",
    body: typeof value === "string" ? value : JSON.stringify(value),
    headers: { "Content-Type": "application/json", ...headers },
    ...other,
  });
}
