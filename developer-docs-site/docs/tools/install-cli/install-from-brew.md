---
title: "Install the CLI with Brew"
---

# Install the Aptos CLI with Brew

Recommended on macOS, `brew` is a package manager that allows for installing and updating packages in a single 
command.

:::tip Not supported on Windows
Brew is not supported fully on Windows
:::

## Installation

1. Ensure you have `brew` installed https://brew.sh/
2. Open a terminal and enter the following commands
```bash
    brew update        # Gets the latest updates for packages
    brew install aptos # Installs the Aptos CLI
```
3. You can now get help instructions by running `aptos help`. You may have to open a new terminal window.
```bash
   aptos help
```

## Upgrading the CLI

Upgrading the CLI with brew is very simple, simply run

```bash
  brew update        # Gets the latest updates for packages
  brew upgrade aptos # Upgrades the Aptos CLI
```

## Additional details

[Aptos CLI homebrew Readme](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/homebrew/README.md)
