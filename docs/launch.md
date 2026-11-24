# Launch copy

## LinkedIn draft

I built AGY DroidBridge because I wanted an Android chat app to use the Antigravity CLI I already had in Termux.

The bridge is a small Rust server on the phone. Point a chat app at localhost and it forwards text to `agy`. Antigravity still handles Google sign-in and generates the replies.

I tested the API with the OpenAI SDK and checked that an Android emulator app could connect before and after a restart. I'm looking for feedback from people who can try it on an ARM64 Termux phone.

[See the source and setup guide](https://github.com/paritoshpky5/agy-droidbridge)

## Product Hunt draft

### Name

AGY DroidBridge

### Tagline

Connect Android chat apps to Antigravity in Termux

### Description

Use a chat app on your Android phone with the Antigravity CLI you already run in Termux. AGY DroidBridge runs a small Rust server on localhost and exposes text chat through an OpenAI-compatible API. The CLI manages Google sign-in and model requests. Install from one repository on ARM64 Android.

**First comment**

Hey, I'm Paritosh. I built this because I wanted to use Antigravity from a chat app on my phone when I already had it set up in Termux. I kept the interface small: install the bridge, copy the localhost URL and key into the app, and send a message. The app uses a familiar chat API and Antigravity handles the model call.

The API and Android emulator checks are in the GitHub repo. If you try it on a physical phone, I'd like to hear which chat app you used and where setup was clear or confusing.

**Media and link**

- [Logo](media/logo.png)
- [Gallery overview](media/overview.png)
- [Android emulator test](media/emulator-check.png)
- [Product URL](https://github.com/paritoshpky5/agy-droidbridge)

Before you schedule the launch, confirm public access to the repo and finish the physical device check. Product Hunt's [submission guide](https://help.producthunt.com/en/articles/17834537-4-build-your-product-page) covers the remaining account and launch fields.
