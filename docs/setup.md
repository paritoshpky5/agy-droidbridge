# Setup and troubleshooting

Start with the [one-paste setup](../README.md#get-started). If Termux needs a CLI, setup checks the download and runs [Google's Linux CLI 1.5.16](https://github.com/google-antigravity/antigravity-cli/releases/tag/1.5.16) through PRoot. PRoot lets the Linux executable start in Termux without loading it through Android's Bionic linker.

## Check the Antigravity CLI

Run `agy --version` and `agy models` from Termux. The bridge needs a signed-in CLI with `--agent`, `--disable-slash-commands`, `--output-format`, and `--model`. See the [Antigravity command reference](https://antigravity.google/docs/cli/reference/#command-line-flags).

Google lists its supported CLI installers in the [Antigravity CLI guide](https://antigravity.google/docs/getting-started?tab=cli). For a CLI in a custom location, set `AGY_BIN` to its executable path.

## Build from source

Use this path if you want to work from a checkout with an existing Antigravity CLI.

```sh
pkg install git
git clone https://github.com/paritoshpky5/agy-droidbridge.git &&
cd agy-droidbridge &&
./agy-droidbridge install --interactive
```

## Approve the bridge workspace once

Antigravity only loads the DroidBridge text agent from an approved workspace. When setup reports the agent is missing, run `./agy-droidbridge trust`, approve its dedicated folder in Antigravity, exit Antigravity, and run `./agy-droidbridge start`. Approve the folder printed by the CLI. The bridge uses that same workspace after restart.

## Set the URL and key in your tool

Your BYOK client needs a custom endpoint and a key setting, and it must make requests from the phone running Termux. For OpenAI-compatible clients, use `http://127.0.0.1:8766/v1` and select Chat Completions. For a client that uses the Anthropic Messages API, use `http://127.0.0.1:8766`. If a client asks for a full request URL, append `/chat/completions` to the OpenAI base URL or `/v1/messages` to the Anthropic base URL.

Use the key from `~/.local/share/agy-droidbridge/api-key` as the client's token. OpenAI-compatible requests use bearer authentication. Anthropic-compatible requests can use `x-api-key` or bearer authentication.

## Make a request from Termux

```sh
curl -fsS http://127.0.0.1:8766/healthz

state="${AGY_STATE_DIR:-$HOME/.local/share/agy-droidbridge}"
auth_header() { printf 'Authorization: Bearer %s\n' "$(cat "$state/api-key")"; }
auth_header | curl -fsS -H @- http://127.0.0.1:8766/v1/models
auth_header | curl -fsS -H @- -H 'Content-Type: application/json' \
  -d '{"messages":[{"role":"user","content":"Reply with READY"}]}' \
  http://127.0.0.1:8766/v1/chat/completions
```

The example uses the CLI's current model selection when the bridge has no previously successful selection. Requests use the signed-in Antigravity account's allowance. For a particular model, pass an ID from `/v1/models` in the `model` field.

## Refresh available models

Run `./agy-droidbridge models` while signed into Antigravity. It reads `agy models`, validates the catalog, saves it, and restarts an active bridge. If the listing fails or fails validation, your saved list stays in place. Until you import a catalog, `/v1/models` uses built-in IDs that can differ from your account's current selection.

Set `AGY_MODELS_FILE` to use your own plain-text list of model IDs and labels. Use `./agy-droidbridge restart` after editing it. Unset the variable before using the built-in refresh command.

## Common issues

See [Start or restart DroidBridge](../README.md#start-or-restart-droidbridge) for the commands to use after reopening Termux.

| Symptom | What to check |
| --- | --- |
| Setup cannot find `agy` | Use the [one-paste setup](../README.md#get-started) to install it. For a source checkout and an executable outside your `PATH`, use `AGY_BIN=/path/to/agy ./agy-droidbridge install --interactive`. |
| Setup reports an underaligned TLS segment | This can happen with the v0.1.0 installer on Google Play Termux. Run the [updated one-paste setup](../README.md#get-started). |
| Antigravity has not loaded the DroidBridge text agent | Run `./agy-droidbridge trust` and complete the workspace approval before starting the bridge. |
| Health check reports `curl: (7) Failed to connect` | Finish [setup](../README.md#get-started) first. If setup had succeeded, run `agy-droidbridge start` and use the port it prints when checking `/healthz`. |
| The tool cannot connect | Run `./agy-droidbridge status`. Check that the tool sends requests from the phone, allows HTTP for localhost, and uses the base URL for its API format. |
| The API returns `401` | Copy the key from the path printed by setup. OpenAI clients send it as bearer authentication. |
| Model is unavailable | Run `agy models` to check sign-in, then run `./agy-droidbridge models` to refresh the list. |
| The bridge exits on startup | Check `~/.local/share/agy-droidbridge/bridge.log`. If the port is in use, run `AGY_PORT=8767 ./agy-droidbridge install` and update the client URL. |

## Run from an existing binary

If you have a trusted Android ARM64 build, run `./agy-droidbridge install /path/to/agy-droidbridge-aarch64-linux-android` from the checkout. Setup still checks the installed CLI and creates the application key.

## API and configuration

| Route | Behavior |
| --- | --- |
| `GET /healthz` or `GET /` | Public liveness check |
| `GET /v1/models` | Available IDs and display labels |
| `POST /v1/chat/completions` | Text generation with optional buffered SSE |
| `POST /v1/messages` | Anthropic text messages with `x-api-key` or bearer authentication |

Include previous messages to continue a conversation. OpenAI streaming sends SSE chunks after Antigravity finishes. The bridge handles one generation at a time and bounds the queue, request body, prompt, and process duration.

| Setting | Default |
| --- | --- |
| `AGY_PORT` | `8766` |
| `AGY_BIN` | `agy` on `PATH` |
| `AGY_STATE_DIR` | `~/.local/share/agy-droidbridge` for the shell commands |
| `AGY_WORKSPACE` | `workspace` under the state directory when using the launcher. Direct callers may set the path to an approved DroidBridge workspace. |
| `AGY_HOST` | `127.0.0.1`. Direct launches accept loopback IPs. |
| `AGY_API_KEY` | Generated by setup. Unset for a direct Rust binary launch. |
| `AGY_MODELS_FILE` | Saved model list if present, otherwise built-in IDs |
| `AGY_TIMEOUT_SECS` | `120` |
| `AGY_MAX_PENDING` | `8` waiting requests plus one active request |
| `AGY_MAX_BODY_BYTES` | `262144` |
| `AGY_MAX_PROMPT_BYTES` | `16384` |
| `AGY_MAX_OUTPUT_BYTES` | `1048576` captured stdout bytes |

See the [architecture](../ARCHITECTURE.md) and [test report](../TEST_REPORT.md) for development details.
