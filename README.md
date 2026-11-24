# AGY DroidBridge

**Use your Antigravity plan in BYOK apps and tools on Android.**

AGY DroidBridge gives apps, scripts, and automations a local API for the Antigravity CLI. If your tool lets you bring your own key and set a custom API URL, point it at DroidBridge running in Termux. The bridge handles OpenAI Chat Completions and Anthropic Messages for text requests.

The ARM64 bridge executable is **1.54 MiB**. Google sign-in and model usage go through the Antigravity CLI.

[Get started](#get-started) · [Connect a BYOK client](#connect-a-byok-client) · [Size and memory](#size-and-memory)

![AGY DroidBridge connects BYOK apps and tools to Antigravity running in Termux](docs/media/overview.png)

## Get started

Paste this line into Termux on your ARM64 Android phone.

```sh
(command -v curl >/dev/null || pkg install -y curl) && curl -fsSL https://raw.githubusercontent.com/paritoshpky5/agy-droidbridge/v0.1.2/install.sh | bash
```

Setup reuses a compatible Antigravity CLI when it finds one. On a fresh install it downloads Google's Linux CLI and runs it through Termux's PRoot package. [See the download and disk sizes](BENCHMARK_REPORT.md#one-paste-install-footprint).

### Finish setup in Antigravity

1. If you see **Choose your color scheme**, pick one, press Enter, and continue through **Next**.
2. Sign in if prompted. When Antigravity asks which folder to trust, approve the DroidBridge workspace at `~/.local/share/agy-droidbridge/workspace`.
3. At the Antigravity prompt, type **`/exit`** and press Enter. Keep the same Termux session open. The installer is waiting for Antigravity to close, then it starts the bridge and prints **`Bridge running`**.

### Check that it works

After you see **`Bridge running`**, open [the health check](http://127.0.0.1:8766/healthz) in a browser on the same phone, or run this in Termux.

```sh
curl -fsS http://127.0.0.1:8766/healthz
```

You should see `{"ok":true,"service":"agy-droidbridge"}`. If the page still fails to load after you exit Antigravity, check the last message in Termux or see the [setup guide](docs/setup.md#common-issues).

## Connect a BYOK client

Your client must accept a **custom base URL and an API key** and send its HTTP requests from the same Android device.

| Provider format | Custom base URL | Text API |
| --- | --- | --- |
| OpenAI-compatible | `http://127.0.0.1:8766/v1` | Chat Completions and model listing |
| Anthropic-compatible | `http://127.0.0.1:8766` | Messages |

Use the key from `~/.local/share/agy-droidbridge/api-key` in the client's API key or token field. If the client asks for a model, run `agy-droidbridge models` and select an ID from the refreshed list.

OpenAI streaming clients receive buffered chunks after generation finishes. See the [setup guide](docs/setup.md) for a complete curl request, lifecycle commands, and troubleshooting.

## Size and memory

| Measurement | Result |
| --- | --- |
| Android ARM64 executable on disk | **1.54 MiB** |
| Idle bridge memory on an Android 14 x86-64 emulator | **4.21–4.27 MiB** median RSS across two trials |

### Windows comparison

All three proxies ran on the same Windows PC with the same mock Antigravity CLI.

| Metric | DroidBridge | Node reference | Python reference |
| --- | ---: | ---: | ---: |
| Idle process memory | **6.29 MiB** | 55.48 MiB | 50.68 MiB |
| Startup to a healthy HTTP response | **45.54 ms** | 146.20 ms | 672.31 ms |
| Throughput with four concurrent requests | 5.32 req/s | 5.06 req/s | **17.40 req/s** |

DroidBridge used **about 89% less idle memory** than the Node reference and took **about 69% less time to start**. Python finished the four-client workload sooner by running four mock CLI processes in parallel. Real Antigravity response times are outside this measurement. [Benchmark method and reproduction commands](BENCHMARK_REPORT.md) · [Node reference](https://github.com/anyrobert/agy-api-proxy)

## Use your Antigravity allowance

DroidBridge sends requests through your signed-in Antigravity account. Requests within your included allowance can avoid a separate metered API bill. A paid plan can cost less when it covers the workload and its price comes in below your usual API spend. Models, quotas, and credit overages depend on your plan. [Antigravity plans and overages](https://antigravity.google/docs/plans)

**Already claimed Google AI Pro through Jio?** Sign in with the Google account you used to redeem it. Jio closed the offer to new claims, and eligible users who already redeemed it can retain up to 18 months under the offer terms. Google lists different Antigravity limits for promotional and paid plans. [Jio offer terms](https://www.jio.com/jcms/en-in/jio-google-gemini-terms-and-conditions/) · [Antigravity plan limits](https://antigravity.google/docs/plans#baseline-quota)

## Testing and development

The HTTP contract and the official OpenAI JavaScript SDK passed against a mock CLI. An Android 14 emulator app reached the bridge over localhost before and after a restart. The Antigravity CLI reached a signed-in prompt in the DroidBridge workspace on a physical ARM64 phone. A live API completion through the bridge on that phone is still pending. See the [test report](TEST_REPORT.md) and [architecture](ARCHITECTURE.md).

If you try it, [open an issue](https://github.com/paritoshpky5/agy-droidbridge/issues) with your Android version, Android architecture, `agy --version`, and the step you reached. Keep API keys and account details out of reports.

Built by [Paritosh](https://github.com/paritoshpky5).
