---
title: "Build CLI from Source Code"
---

# Build Aptos CLI from Source Code

If you are an advanced user and would like to build the CLI binary by downloading the source code, follow the below steps, [selecting the network branch](../../../guides/system-integrators-guide.md#choose-a-network) that meets your use case. Otherwise, [install the prebuilt CLI binaries](./download-cli-binaries.md) to ease ramp up and reduce variables in your environment.

Begin by preparing your environment by following the instructions in [building Aptos from source](../../../guides/building-from-source.md), note, you can skip the last section on _Building Aptos_ as the instructions below build in release mode.

<details>
<summary>Linux / macOS</summary>

### Linux / macOS

#### Building the Aptos CLI

1. Build the CLI tool: `cargo build --package aptos --release`
1. The binary will be available in at `target/release/aptos`
1. (Optional) Move this executable to a place on your path. For example: `~/bin/aptos`
1. View help instructions by running `~/bin/aptos help`

</details>

<details>
<summary>Windows</summary>

### Windows

#### Building aptos-core

1. Build the CLI tool: `cargo build --package aptos --release`
1. The binary will be available at `target\release\aptos.exe`
1. View help instructions by running `target\release\aptos.exe`

</details>
