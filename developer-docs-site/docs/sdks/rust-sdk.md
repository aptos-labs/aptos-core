---
title: "Rust SDK"
slug: "rust-sdk"
---

# Aptos Rust SDK

## Installing Rust SDK

Aptos provides an official Rust SDK in the [Aptos-core GitHub](https://github.com/aptos-labs/aptos-core/tree/main/sdk) repository. To use the Rust SDK, add the following dependency and patches on the git repo directly in your `Cargo.toml`, like this:

```toml
[dependencies]
aptos-sdk = { git = "https://github.com/aptos-labs/aptos-core", branch = "devnet" }

[patch.crates-io]
merlin = { git = "https://github.com/aptos-labs/merlin" }
```

You must also create a `.cargo/config.toml` file with this content:
```toml
[build]
rustflags = ["--cfg", "tokio_unstable"]
```

The source code for the official Rust SDK is available in the [aptos-core GitHub repository](https://github.com/aptos-labs/aptos-core/tree/main/sdk).

## Using Rust SDK

See the [Developer Tutorials](../tutorials/index.md) for code examples showing how to use the Rust SDK.
