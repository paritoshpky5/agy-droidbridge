# Launch copy

## LinkedIn draft

I built AGY DroidBridge for apps and tools that ask you to bring your own API key.

If your tool lets you set a custom endpoint, give it the DroidBridge URL and local key. The bridge runs in Termux on Android and sends text requests through the Antigravity CLI you already signed into.

That gives those tools a way to use your existing Antigravity allowance. A paid plan can be cheaper than a separate per-token API for regular use if its included usage covers your workload. If you already redeemed Google AI Pro through Jio, use that same Google account. Jio's offer ran for up to 18 months and has closed to new claims. [Jio offer terms](https://www.jio.com/jcms/en-in/jio-google-gemini-terms-and-conditions/) · [Antigravity plans and limits](https://antigravity.google/docs/plans)

I tested the API with the OpenAI SDK and verified localhost access from an Android emulator app before and after a restart. If you run a compatible tool on an ARM64 Termux phone, I'd like to hear what you tried.

[See the source and setup guide](https://github.com/paritoshpky5/agy-droidbridge)

## Product Hunt draft

### Name

AGY DroidBridge

### Tagline

Use your Antigravity plan in BYOK apps

### Description

Give BYOK apps, scripts, and tools a local URL and key that use your existing Antigravity allowance. AGY DroidBridge runs in Termux on ARM64 Android and supports OpenAI Chat Completions and Anthropic Messages for text. Clients need a custom endpoint and must send requests from the same phone.

**First comment**

Hey, I'm Paritosh. I kept seeing API key fields in tools I wanted to try while I already had Antigravity set up in Termux. I built DroidBridge so a compatible tool can use that allowance. Install it, copy the localhost URL and key into the tool, and send a text request. Antigravity still handles sign-in and the model call.

Existing Jio Google AI Pro redemptions can be used with the Google account that claimed the offer, subject to that account's Antigravity access and limits. Paid plans can also be good value when their included usage covers what you'd normally pay for through a metered API.

The API and Android emulator checks are in the GitHub repo. If you try it on a phone, which BYOK tool did you connect and how did setup go?

**Media and link**

- [Logo](media/logo.png)
- [Gallery overview](media/overview.png)
- [Android emulator test](media/emulator-check.png)
- [Product URL](https://github.com/paritoshpky5/agy-droidbridge)

Before you schedule the launch, confirm public access to the repo and finish the physical device check. Product Hunt's [submission guide](https://help.producthunt.com/en/articles/17834537-4-build-your-product-page) covers the remaining account and launch fields.
