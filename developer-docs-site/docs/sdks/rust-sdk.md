---
title: "Rust SDK"
slug: "rust-sdk"
---

# Aptos Rust SDK

Aptos provides an official Rust SDK. The Rust SDK is tested carefully, though it isn't as popular as the [Typescript SDK](/sdks/typescript-sdk).

For now the best way to use the Rust SDK is to add a dependency on the git repo directly, like this:
```toml
aptos-sdk = { git = "https://github.com/aptos-labs/aptos-core", branch = "devnet" }
```

The source code is available in the [aptos-core GitHub repository](https://github.com/aptos-labs/aptos-core/tree/main/sdk).
