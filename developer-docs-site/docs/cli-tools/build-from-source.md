---
title: "Build CLI from Source Code"
id: "build-aptos-cli"
---

# Build Aptos CLI from Source Code

If you are an advanced user and would like to build the CLI binary by downloading the source code, follow the below steps, [selecting the network branch](../guides/system-integrators-guide.md#choose-a-network) that meets your use case. For example, `main` contains the latest code yet poses a greater risk of bugs.

**Building the Aptos CLI is not recommended** unless you are on a platform unsupported by the prebuilt binaries. Otherwise, [install the prebuilt CLI binaries](aptos-cli-tool/install-aptos-cli.md) to ease ramp up and reduce variables in your environment.

:::tip Configure your development environment first
See [Setup build dependencies](../guides/getting-started.md#set-up-build-dependencies) to install the necessary dependencies for your development environment.
:::

<details>
<summary>macOS</summary>

### macOS

#### Building the Aptos CLI

1. [Clone the Aptos-core repo.](../guides/getting-started.md#clone-the-aptos-core-repo)
1. [Check out a release branch.](../guides/getting-started.md#check-out-release-branch)
1. Build the CLI tool: `cargo build --package aptos --release`
1. The binary will be available in at `target/release/aptos`
1. (Optional) Move this executable to a place on your path. For example: `~/bin/aptos`
1. View help instructions by running `~/bin/aptos help`

</details>

<details>
<summary>Linux</summary>

### Linux

#### Building the Aptos CLI

1. [Clone the Aptos-core repo.](../guides/getting-started.md#clone-the-aptos-core-repo)
1. [Check out a release branch.](../guides/getting-started.md#check-out-release-branch)
1. Build the CLI tool: `cargo build --package aptos --release`
1. The binary will be available at `target/release/aptos`
1. (Optional) Move this executable to a place on your path. For example: `~/bin/aptos`
1. View help instructions by running `~/bin/aptos help`

</details>

<details>
<summary>Windows</summary>

### Windows

#### Building aptos-core
    
1. [Clone the Aptos-core repo.](../guides/getting-started.md#clone-the-aptos-core-repo)
1. [Check out a release branch.](../guides/getting-started.md#check-out-release-branch)
1. Build the CLI tool: `cargo build --package aptos --release`
1. The binary will be available at `target\release\aptos.exe`
1. View help instructions by running `target\release\aptos.exe`

</details>
