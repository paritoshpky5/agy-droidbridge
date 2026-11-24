# Benchmark report

## Android size and idle memory

Measured on November 24, 2026. The Android 14 emulator used an x86-64 guest on a Windows host with a Ryzen 5 3550H.

| Measurement | Result |
| --- | --- |
| Android ARM64 optimized executable | 1,615,544 bytes (1.54 MiB) |
| Android x86-64 optimized executable | 1,710,968 bytes (1.63 MiB) |
| Idle bridge RSS on the Android emulator, first trial | 4,372 KiB (4.27 MiB) median, 4,120–4,432 KiB range |
| Idle bridge RSS on the Android emulator, repeat trial | 4,316 KiB (4.21 MiB) median, 4,212–4,568 KiB range |

The probe starts the x86-64 bridge three times per trial and checks `/healthz` after each start. One second after the check passes, it reads `VmRSS` from `/proc/<pid>/status` five times at roughly 250 ms intervals. Each median pools 15 readings. The probe sends no generation requests, so the reading covers the idle bridge process.

To reproduce on Windows, build the x86-64 Android release using the [emulator instructions](TEST_REPORT.md#android-emulator) and run this command with that emulator booted.

```powershell
pwsh ./benchmarks/android-idle.ps1 -Serial emulator-5554
```

These measurements cover executable size on disk and emulator idle memory. Physical-device RAM, full Termux and CLI memory, battery use, and live generation performance remain unmeasured.

## One-paste install footprint

| Item | Size |
| --- | ---: |
| DroidBridge ARM64 download | 1.54 MiB |
| DroidBridge executable files after setup, including the copy cached for retries | 3.08 MiB |
| Official Antigravity 1.2.7 ARM64 download, when needed | 55.39 MiB |
| Official Antigravity executable before Termux preparation | 203.27 MiB |

On a fresh setup, Termux also installs `clang`, `python`, and `patchelf` for Antigravity compatibility. Their download and disk usage depend on what is already installed. With a working compatible CLI, setup reuses it and skips those downloads. The Windows comparison below measures the bridge processes with a stub CLI. [DroidBridge release](https://github.com/paritoshpky5/agy-droidbridge/releases/tag/v0.1.0) · [Antigravity CLI release](https://github.com/google-antigravity/antigravity-cli/releases/tag/1.2.7)

## Windows comparison with Node and Python

On November 24, 2026, all three servers ran on the same Windows x64 host with a Ryzen 5 3550H. DroidBridge used its optimized Windows build. The [Node reference](https://github.com/anyrobert/agy-api-proxy/tree/98107d5cdb9e1bb7ed43e88b5e62e0adcaede43f) ran at commit `98107d5` on Node.js 24.14.0. The [Python reference](https://pypi.org/project/antigravity-proxy/0.1.1/) ran on CPython 3.14.3 with FastAPI 0.165.0 and Uvicorn 0.54.0.

Each workload ran three rounds of 24 non-streaming Chat Completions requests per server. Requests used the same prompt and model ID, and all three servers called the same mock CLI with a 175 ms delay per generation. The runner rotated server order between rounds.

### Four concurrent requests

| Metric | AGY DroidBridge | Node reference | Python reference |
| --- | ---: | ---: | ---: |
| Idle working set | 6.29 MiB | 55.48 MiB | 50.68 MiB |
| Peak server working set | 7.23 MiB | 58.96 MiB | 54.28 MiB |
| Peak server plus mock CLI working set | 10.97 MiB | 63.29 MiB | 71.60 MiB |
| Startup to liveness | 45.54 ms | 146.20 ms | 672.31 ms |
| Request latency p50 | 387.15 ms | 411.10 ms | 213.83 ms |
| Request latency p95 | 755.00 ms | 792.42 ms | 238.11 ms |
| Throughput | 5.32 req/s | 5.06 req/s | 17.40 req/s |
| Peak concurrent mock CLI processes | 1 | 1 | 4 |

The Python server handled four CLI jobs in parallel. DroidBridge and the Node reference ran one generation at a time. The concurrent results show how the implementations handle the same client load, including their different CLI scheduling.

### One request at a time

| Metric | AGY DroidBridge | Node reference | Python reference |
| --- | ---: | ---: | ---: |
| Request latency p50 | 187.86 ms | 196.61 ms | 195.93 ms |
| Request latency p95 | 195.27 ms | 202.38 ms | 198.63 ms |
| Throughput | 5.31 req/s | 5.07 req/s | 5.09 req/s |

Working set is sampled by process. Peak server-plus-mock-CLI working set is the median of each run's highest sampled total for the server and its children. Startup includes HTTP liveness polling. Latency percentiles pool requests across three rounds. Other statistics use the median of the three runs.

See the [test report](TEST_REPORT.md#optional-compatibility-and-benchmark-runs) for the Windows comparison commands.
