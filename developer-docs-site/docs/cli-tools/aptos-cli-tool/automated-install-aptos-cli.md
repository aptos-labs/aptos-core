---
title: "Automated Aptos CLI Installation"
id: "automated-install-aptos-cli"
---

# Automated Aptos CLI Installation

The `aptos` tool is a command line interface (CLI) for developing on the Aptos blockchain, debugging Move contracts, and conducting node operations. This document describes how to install the `aptos` CLI tool using the automated install script.

First, ensure you have Python 3.6+ installed:
```
$ python3 --version
Python 3.9.13
```

<details>
<summary>MacOS / Linux / Windows Subsystem for Linux (WSL)</summary>

:::tip
These instructions have been tested on Ubuntu 20.04, Ubuntu 22.04, Arch Linux, MacOS (ARM), and WSL.
:::

In your terminal:

```
curl -fsSL "https://aptos.dev/scripts/install_cli.py" | python3
```

If you don't have curl installed, try using wget:
```
wget -qO- "https://aptos.dev/scripts/install_cli.py" | python3
```

</details>

<details>

<summary>Windows (NT)</summary>

:::tip
These instructions have been tested on Windows 11.
:::

In Powershell:
```
iwr "https://aptos.dev/scripts/install_cli.py" -useb | Select-Object -ExpandProperty Content | python3
```

</details>
