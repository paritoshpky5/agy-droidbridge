import { spawn } from "node:child_process";
import { readFile, stat } from "node:fs/promises";
import { createInterface } from "node:readline";
import { setTimeout as delay } from "node:timers/promises";
import { chat, post, startProxy } from "../tests/harness.mjs";

const runs = 3;
const concurrency = 4;
const requests = 24;

function quantile(values, percentile) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted.length ? sorted[Math.max(0, Math.ceil(sorted.length * percentile) - 1)] : null;
}

function windowsSampler(pid) {
  const cmd = [
    "$ErrorActionPreference = 'Stop'",
    "while ($true) {",
    "$p = @(Get-CimInstance Win32_Process -Filter 'ProcessId = " + pid + " OR ParentProcessId = " + pid + "')",
    "$root = $p | Where-Object ProcessId -EQ " + pid,
    "if (!$root) { break }",
    "$tree = ($p | Measure-Object -Property WorkingSetSize -Sum).Sum",
    "[Console]::WriteLine(('{0},{1},{2},{3}' -f $root.WorkingSetSize,$tree,($root.UserModeTime+$root.KernelModeTime),($p.Count-1)))",
    "Start-Sleep -Milliseconds 40",
    "}",
  ].join("\n");
  const child = spawn("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", cmd],
    { windowsHide: true, stdio: ["ignore", "pipe", "ignore"] });
  const samples = [];
  child.on("error", () => {});
  const lines = createInterface({ input: child.stdout });
  lines.on("line", (line) => {
    const numbers = line.trim().split(",").map(Number);
    if (numbers.length === 4 && numbers.every(Number.isFinite)) {
      const [proxy, tree, cpuTicks, children] = numbers;
      samples.push({ proxy, tree, cpuMs: cpuTicks / 10_000, children });
    }
  });
  return { samples, stop: () => { child.kill(); lines.close(); } };
}

function linuxSampler(pid) {
  const samples = [];
  let finished = false;
  async function sample() {
    while (!finished) {
      try {
        const children = (await readFile("/proc/" + pid + "/task/" + pid + "/children", "utf8")).trim().split(/\s+/).filter(Boolean);
        const rss = async (id) => {
          const status = await readFile("/proc/" + id + "/status", "utf8");
          return 1024 * Number(status.match(/^VmRSS:\s+(\d+)\s+kB/m)?.[1] ?? 0);
        };
        const proxy = await rss(pid);
        const workerRss = await Promise.all(children.map((id) => rss(id).catch(() => 0)));
        samples.push({ proxy, tree: proxy + workerRss.reduce((a, b) => a + b, 0), children: workerRss.filter(Boolean).length });
      } catch {
        return;
      }
      await delay(30);
    }
  }
  void sample();
  return { samples, stop: () => { finished = true; } };
}

async function measure(kind) {
  const service = await startProxy(kind);
  const sampler = process.platform === "win32" ? windowsSampler(service.pid) :
    process.platform === "linux" ? linuxSampler(service.pid) : { samples: [], stop() {} };
  const latencies = [];
  try {
    await delay(750);
    const idleSamples = sampler.samples.slice();
    const baselineCpu = idleSamples.at(-1)?.cpuMs;
    const start = performance.now();
    for (let sent = 0; sent < requests; sent += concurrency) {
      await Promise.all(Array.from({ length: concurrency }, async () => {
        const before = performance.now();
        const result = await post(service, "/v1/chat/completions", chat("MOCK_SERIAL"));
        if (!result.ok) throw new Error("Benchmark request failed with HTTP " + result.status);
        const completion = await result.json();
        if (!/^\d+:\d+$/.test(completion.choices[0].message.content)) throw new Error("Unexpected fixture response");
        latencies.push(performance.now() - before);
      }));
    }
    const duration = performance.now() - start;
    await delay(180);
    const loadSamples = sampler.samples.slice(idleSamples.length);
    const finalCpu = sampler.samples.at(-1)?.cpuMs;
    return {
      startupMs: service.startupMs, requestsPerSecond: requests * 1000 / duration, wallMs: duration,
      idleProxyRss: quantile(idleSamples.map(({ proxy }) => proxy), 0.5),
      peakProxyRss: quantile(loadSamples.map(({ proxy }) => proxy), 1),
      peakProxyAndCliRss: quantile(loadSamples.filter(({ children }) => children).map(({ tree }) => tree), 1),
      proxyCpuMs: finalCpu === undefined || baselineCpu === undefined ? null : finalCpu - baselineCpu,
      latencies,
    };
  } finally {
    sampler.stop();
    await service.stop();
  }
}

const metrics = ["startupMs", "requestsPerSecond", "wallMs", "idleProxyRss", "peakProxyRss", "peakProxyAndCliRss", "proxyCpuMs"];
const measurements = { node: [], rust: [] };
for (let run = 0; run < runs; run++) {
  for (const kind of run % 2 ? ["rust", "node"] : ["node", "rust"]) {
    measurements[kind].push(await measure(kind));
    console.error(kind + " run " + (run + 1) + " complete");
  }
}
const summary = { platform: process.platform, arch: process.arch, runs, requests, concurrency };
for (const [kind, series] of Object.entries(measurements)) {
  const latencies = series.flatMap((run) => run.latencies);
  const executable = kind === "node" ? process.execPath : process.env.AGY_TEST_RUST_BIN;
  summary[kind] = {
    executableBytes: (await stat(executable)).size,
    p50Ms: quantile(latencies, 0.5),
    p95Ms: quantile(latencies, 0.95),
    ...Object.fromEntries(metrics.map((metric) =>
      [metric, quantile(series.map((item) => item[metric]).filter(Number.isFinite), 0.5)])),
  };
}
console.log(JSON.stringify(summary, null, 2));
