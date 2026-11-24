# AGY DroidBridge

![AGY DroidBridge connects an Android chat app to Antigravity running in Termux](docs/media/overview.png)

**Use the Antigravity CLI from the chat app on your Android phone.**

AGY DroidBridge is a small Rust server that runs in Termux. Point an OpenAI-compatible chat app at `http://127.0.0.1:8766/v1` and the bridge passes its text requests to `agy`. The Antigravity CLI handles Google sign-in and generates the replies.

## Why I built it

Antigravity already runs in a Termux terminal. I wanted a way to use the same session from an Android chat app. I built the bridge in Rust so the server can run as one native executable on the phone.

## Get started

Use an ARM64 Android phone with Termux and a working Antigravity CLI. In Termux, `uname -m` should print `aarch64` and `agy --version` should show a recent CLI version. Run `agy models` once and finish Google sign-in if prompted.

```sh
pkg install git
git clone https://github.com/paritoshpky5/agy-droidbridge.git
cd agy-droidbridge
./agy-droidbridge install
```

The installer builds the server and creates your private API key. Allow a few minutes for the first build. If setup reports that the text agent has not loaded, run the following once. Approve the DroidBridge folder in Antigravity and exit it before running the second command.

```sh
./agy-droidbridge trust
./agy-droidbridge start
```

The running server reports its URL. The key is stored in the file listed below.

Open the chat app on **the same phone** and add an OpenAI-compatible provider.

| App setting | Enter this |
| --- | --- |
| API type | Chat Completions |
| Base URL | `http://127.0.0.1:8766/v1` |
| API key | The contents of `~/.local/share/agy-droidbridge/api-key` |
| Model | If the app asks for one, run `./agy-droidbridge models` and pick an ID from the refreshed list. |

`./agy-droidbridge status` checks the running server. Use `./agy-droidbridge stop`, `start`, or `restart` to manage it. See the [setup guide](docs/setup.md) for a first request with curl, model refresh, and troubleshooting.

## What you can do

- Use OpenAI-compatible text chat and model listing from a native Android client.
- Use the Anthropic Messages endpoint for text with an Anthropic-compatible client.
- Keep requests on localhost between the app and the bridge. The Antigravity CLI uses its normal connection to Google for model requests.

The OpenAI API can return buffered streaming chunks after generation completes. Images, tool calls, and the OpenAI Responses API are outside the current interface.

## What has been tested

The HTTP contract and the official OpenAI JavaScript SDK have been exercised with a mock CLI. An Android 14 emulator app reached the bridge over localhost and repeated the check after a restart. A physical ARM64 Termux run with an authenticated Antigravity session is still pending. See the [test report](TEST_REPORT.md).

If you try it, [open an issue](https://github.com/paritoshpky5/agy-droidbridge/issues) with your Android version, Android architecture, `agy --version`, and the step you reached. Keep API keys and account details out of reports.

Built by [Paritosh](https://github.com/paritoshpky5).
