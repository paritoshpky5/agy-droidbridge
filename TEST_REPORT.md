# Test report

Verification on November 24, 2026 used Windows x64, an Android 14 x86-64 emulator, and a local mock for model responses.

| Check | Result |
| --- | --- |
| Rust formatting, Clippy, unit tests, and release build | Passed. Five unit tests passed. |
| Standalone HTTP contract | Eight tests passed. The optional reference comparison was skipped. |
| HTTP contract with the reference comparison | All nine tests passed with a local mock CLI. |
| OpenAI JavaScript SDK | Model listing, completion, buffered streaming with usage, and invalid key handling passed. |
| Installer and lifecycle | Bash checks passed with local shims for install, workspace approval, start, model refresh, rejected catalog updates, restart, and stop. |
| Official Antigravity CLI 1.5.16 on Windows | Verified the required flags. In an isolated profile, the exact DroidBridge agent was absent before workspace approval and present after it. |
| Android 14 x86-64 emulator | An Android app passed localhost health, invalid-key handling, model listing, and Chat Completions before and after a bridge restart. The CLI was mocked. |
| Android ARM64 | Built an Android API 24 AArch64 ELF with 16 KiB segment alignment. The binary is 1,615,544 bytes. Execution on a phone is pending. |

The standalone suite covers text and developer messages, Anthropic system prompts, unsupported features, model selection after failed generation, input limits, browser isolation, key handling, timeout recovery, and workspace validation. The optional comparison uses the [Node reference pinned at 98107d5](https://github.com/anyrobert/agy-api-proxy/tree/98107d5cdb9e1bb7ed43e88b5e62e0adcaede43f).

## Run without the reference repository

Use recent Rust, Bash, and Node.js 22 or later. Node.js is used by the test harness.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
bash tests/lifecycle.sh

rustc --edition 2024 -O tests/fixtures/mock_agy.rs -o target/mock-agy
export AGY_TEST_RUST_BIN="$PWD/target/release/agy-droidbridge"
export AGY_TEST_MOCK_BIN="$PWD/target/mock-agy"
node --test tests/contract.mjs
```

On Windows, use absolute paths to the `.exe` files. The lifecycle test uses Bash and local shims for Termux and `agy`.

### Optional compatibility and benchmark runs

Build the pinned Node reference and point `AGY_TEST_NODE_ENTRY` at its compiled `dist/server.js`.

```sh
git clone https://github.com/anyrobert/agy-api-proxy.git /tmp/agy-node-reference
(cd /tmp/agy-node-reference && git checkout 98107d5cdb9e1bb7ed43e88b5e62e0adcaede43f && npm ci && npm run build)
export AGY_TEST_NODE_ENTRY=/tmp/agy-node-reference/dist/server.js
node --test tests/contract.mjs
node benchmarks/compare.mjs
```

### Android emulator

The probe builds a test APK locally. Use a booted x86-64 Android emulator, JDK 21, Android SDK Build Tools 36.0.0, Platform 36, and NDK 28.2.13676358. From PowerShell with `ANDROID_HOME` set to your SDK path, run:

```powershell
rustup target add x86_64-linux-android
$ndk = Join-Path $env:ANDROID_HOME 'ndk/28.2.13676358/toolchains/llvm/prebuilt/windows-x86_64/bin'
$env:CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER = Join-Path $ndk 'x86_64-linux-android24-clang.cmd'
cargo build --locked --release --target x86_64-linux-android
pwsh ./tests/emulator/verify.ps1 -Serial emulator-5554
```

## Physical phone check

On an ARM64 Android phone with Termux, follow the README through installation and workspace approval. Run `./agy-droidbridge models`, then send one text request using the [setup guide](docs/setup.md#make-a-request-from-termux). Check that your chosen BYOK client connects on the same device and can send a second request after `./agy-droidbridge restart`.

Live model generation, an authenticated Google session on the phone, and physical Termux execution remain pending.
