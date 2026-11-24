# Architecture

AGY DroidBridge runs an HTTP server inside Termux. An Android chat app sends a text request to localhost. The bridge validates the request, formats the conversation as text, and starts the installed Antigravity CLI with its headless JSON output mode. Antigravity manages sign-in and sends model requests through its own connection.

## HTTP and model selection

The server offers a public localhost health check and authenticated model listing, OpenAI Chat Completions, and Anthropic Messages. The installer generates an application key stored in a private state directory. A direct binary launch can take its own key from `AGY_API_KEY`. The server binds to loopback and rejects browser origins and non-local HTTP hosts.

A model ID from the catalog becomes a per-request CLI argument. The bridge remembers an explicit model after a successful answer. Account-specific catalogs come from the local `agy models` command and pass validation before replacing the saved list. The bridge leaves the CLI's model settings and credentials under the CLI's control.

## Process boundary

Axum handles HTTP and Tokio supervises one CLI process at a time. The launcher gives the bridge a stable private working directory and prepares a text agent with an empty tool list. Antigravity requires the operator to approve this workspace before it lists the agent. Startup checks for that agent, then each request disables slash commands and passes the prompt as a single argument. The child receives a limited environment and the bridge keeps raw CLI diagnostics out of responses and logs.

Body size, prompt size, child output, queue length, and generation time are bounded. On Unix, the child runs in its own process group for cleanup on timeout or disconnect. A client's request includes its own history. The bridge ends generation after the CLI's final result, then formats the response as JSON or buffered OpenAI SSE.

## Reference checks

The [reference Node API](https://github.com/anyrobert/agy-api-proxy) informs a separate compatibility comparison. DroidBridge's own test suite checks its API against a local fixture and runs without that repository. See the [test report](TEST_REPORT.md).
