# Technical decisions

- A loopback listener lets an Android client and Termux communicate on the same phone. The install script generates an application key so requests from other local apps still require authorization.
- DroidBridge starts `agy` in headless JSON mode, supplies a workspace text agent, and disables slash command expansion. The CLI manages Google credentials and model calls. [Headless mode](https://antigravity.google/docs/cli/headless/) · [Custom agents](https://antigravity.google/docs/subagents?tab=cli)
- Each request passes its model as a CLI argument. A failed request leaves the last working selection in place. Model refresh validates new output before replacing the list.
- Text is the current API boundary. The bridge returns a client error for images, tool calls, structured output, and multiple completions rather than dropping those inputs or claiming to support them.
- Buffered SSE gives streaming clients OpenAI-shaped events after the CLI finishes. The API also returns ordinary JSON responses.
- Native builds target Android API 24. The emulator can verify the network path and mock CLI contract on x86-64. A physical ARM64 Termux device is required for the installed-CLI and live-session check.
