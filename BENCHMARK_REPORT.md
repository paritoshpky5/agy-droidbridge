# Benchmark report

On November 24, 2026, both servers ran on the same Windows x64 host with a Ryzen 5 3550H. The reference server ran on Node.js 24.14.0 at [98107d5](https://github.com/anyrobert/agy-api-proxy/tree/98107d5cdb9e1bb7ed43e88b5e62e0adcaede43f). DroidBridge used the optimized Windows binary. Both used the same mock CLI with a 175 ms delay per request. Each server ran three rounds with 24 requests per round and four concurrent client requests per batch.

| Metric | Node reference | AGY DroidBridge |
| --- | ---: | ---: |
| Idle resident memory | 55.28 MiB | 6.28 MiB |
| Peak resident memory | 58.63 MiB | 7.07 MiB |
| Peak server plus CLI memory | 62.96 MiB | 10.81 MiB |
| Startup to liveness | 142.67 ms | 48.46 ms |
| Executable on disk | 87.15 MiB | 1.66 MiB |
| Request latency p50 | 410.46 ms | 383.49 ms |
| Request latency p95 | 792.67 ms | 748.23 ms |
| Throughput | 5.10 req/s | 5.34 req/s |
| Server CPU time per run | 234.38 ms | 78.13 ms |

Memory is Windows working set. The server-plus-CLI number is the median of each run's peak simultaneous working set. Startup includes health-check polling. Latency percentiles pool request-level observations. Other statistics use the median of the three runs. The executable sizes are `node.exe` and `agy-droidbridge.exe`. The Android ARM64 release is 1.54 MiB.

The test measures a fixed-delay mock and a Windows process. Android memory, battery use, and live model latency remain unmeasured. Reproduction commands are in the [test report](TEST_REPORT.md).
