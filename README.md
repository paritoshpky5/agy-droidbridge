# AGY DroidBridge

**Use your Antigravity plan in BYOK apps and tools on Android.**

AGY DroidBridge gives apps, scripts, and automations a local API for the Antigravity CLI. If your tool lets you bring your own key and set a custom API URL, point it at DroidBridge running in Termux. The bridge handles OpenAI Chat Completions and Anthropic Messages for text requests.

The ARM64 bridge executable is **1.54 MiB**. Google sign-in and model usage go through the Antigravity CLI.

[Get started](#get-started) · [Connect a BYOK client](#connect-a-byok-client) · [Size and memory](#size-and-memory)

![AGY DroidBridge connects BYOK apps and tools to Antigravity running in Termux](docs/media/overview.png)

## Get started

Paste this line into Termux on your ARM64 Android phone.

```sh
(command -v curl >/dev/null || pkg install -y curl) && curl -fsSL https://raw.githubusercontent.com/paritoshpky5/agy-droidbridge/v0.1.0/install.sh | bash
```

Setup downloads the bridge and reuses a compatible Antigravity CLI when it finds one. Otherwise it downloads the official Antigravity binary and prepares it for Termux. The first install can take a few minutes.

When Antigravity opens, sign in if prompted, approve the DroidBridge workspace, and exit Antigravity. The same setup command resumes and starts DroidBridge automatically. The terminal prints the server URL and where to find your API key.

After setup, `agy-droidbridge status` checks the local API. You can also make a health request from Termux.

```sh
curl -fsS http://127.0.0.1:8766/healthz
```

The response should be `{"ok":true,"service":"agy-droidbridge"}`. If setup stops, check the [setup guide](docs/setup.md#common-issues) for the error shown above it.

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
| Idle working set in the Windows mock benchmark | **6.29 MiB** for DroidBridge, **55.48 MiB** for the Node reference, **50.68 MiB** for the Python reference |

The Windows comparison measured **about 88% lower idle working set** for DroidBridge against the Python reference. The Python server finished the four-client workload sooner by launching CLI processes in parallel. The idle measurements cover the bridge process alone. See the [benchmark report](BENCHMARK_REPORT.md) for both workloads and reproduction commands.

## Use your Antigravity allowance

DroidBridge sends requests through your signed-in Antigravity account. Requests within your included allowance can avoid a separate metered API bill. A paid plan can cost less when it covers the workload and its price comes in below your usual API spend. Models, quotas, and credit overages depend on your plan. [Antigravity plans and overages](https://antigravity.google/docs/plans)

**Already claimed Google AI Pro through Jio?** Sign in with the Google account you used to redeem it. Jio closed the offer to new claims, and eligible users who already redeemed it can retain up to 18 months under the offer terms. Google lists different Antigravity limits for promotional and paid plans. [Jio offer terms](https://www.jio.com/jcms/en-in/jio-google-gemini-terms-and-conditions/) · [Antigravity plan limits](https://antigravity.google/docs/plans#baseline-quota)

## Testing and development

The HTTP contract and the official OpenAI JavaScript SDK passed against a mock CLI. An Android 14 emulator app reached the bridge over localhost before and after a restart. A live Antigravity session on a physical ARM64 phone is still pending. See the [test report](TEST_REPORT.md) and [architecture](ARCHITECTURE.md).

If you try it, [open an issue](https://github.com/paritoshpky5/agy-droidbridge/issues) with your Android version, Android architecture, `agy --version`, and the step you reached. Keep API keys and account details out of reports.

Built by [Paritosh](https://github.com/paritoshpky5).
