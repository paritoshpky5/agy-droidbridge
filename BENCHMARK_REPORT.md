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

## Windows comparison with the Node reference

On November 24, 2026, both servers ran on the same Windows x64 host with a Ryzen 5 3550H. The reference server ran on Node.js 24.14.0 at [98107d5](https://github.com/anyrobert/agy-api-proxy/tree/98107d5cdb9e1bb7ed43e88b5e62e0adcaede43f). DroidBridge used the optimized Windows binary. Both used the same mock CLI with a 175 ms delay per request. Each server ran three rounds with 24 requests per round and four concurrent client requests per batch.

| Metric | Node reference | AGY DroidBridge |
| --- | ---: | ---: |
| Idle working set | 55.28 MiB | 6.28 MiB |
| Peak working set | 58.63 MiB | 7.07 MiB |
| Peak server plus mock CLI working set | 62.96 MiB | 10.81 MiB |
| Startup to liveness | 142.67 ms | 48.46 ms |
| Executable on disk | 87.15 MiB | 1.66 MiB |
| Request latency p50 | 410.46 ms | 383.49 ms |
| Request latency p95 | 792.67 ms | 748.23 ms |
| Throughput | 5.10 req/s | 5.34 req/s |
| Server CPU time per run | 234.38 ms | 78.13 ms |

The server-plus-mock-CLI number is the median of each run's peak simultaneous working set. Startup includes health-check polling. Latency percentiles pool request-level observations. Other statistics use the median of the three runs. The executable sizes cover `node.exe` and `agy-droidbridge.exe`.

See the [test report](TEST_REPORT.md#optional-compatibility-and-benchmark-runs) for the Windows comparison commands.
