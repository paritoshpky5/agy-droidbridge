# Verification checklist

- [x] Implement the loopback Rust server and text chat endpoints
- [x] Add installation, lifecycle, and model refresh commands
- [x] Add standalone contract tests, with an optional reference comparison
- [x] Prepare the README, installation guide, and launch copy
- [x] Pass the Rust and standalone HTTP contract tests
- [x] Compare the optional reference API and run the OpenAI SDK check
- [x] Verify the Termux command flow with shims and agent discovery with the official CLI
- [x] Build both Android targets and test localhost with an emulator app
- [x] Document ARM64 executable size and reproduce idle memory measurements in the emulator
- [x] Measure the Rust, Node, and Python bridges with a shared mock CLI
- [ ] Install and run on a physical ARM64 Android phone
- [ ] Verify generation through a signed-in Antigravity CLI on that phone

See the [test report](TEST_REPORT.md) and [benchmark report](BENCHMARK_REPORT.md).
