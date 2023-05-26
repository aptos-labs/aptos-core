---
title: "Install CLI by Script"
---

# Install CLI by script

The `aptos` tool is a command line interface (CLI) for developing on the Aptos blockchain, debugging Move contracts, and conducting node operations. This document describes how to install the `aptos` CLI tool using the automated install script.

## Prerequisites

First, ensure you have Python 3.6+ installed:
```
$ python3 --version
Python 3.9.13
```
If it is not installed, you can find installation instructions on [python.org](https://www.python.org/downloads/).

## Install

Follow these instructions to install the Aptos CLI on various operating systems. Regardless of the operating system, you will always be directed to the latest release of the Aptos CLI. 

<details>
<summary>macOS / Linux / Windows Subsystem for Linux (WSL)</summary>

:::tip
These instructions have been tested on Ubuntu 20.04, Ubuntu 22.04, Arch Linux, MacOS (ARM), and WSL and assume you have either `curl` or `wget` installed to download the script.
:::

In your terminal, run the following `curl` command:

```
curl -fsSL "https://aptos.dev/scripts/install_cli.py" | python3
```

Or with `wget`:
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

## Update

To trigger an update to the Aptos CLI, run `aptos update` and see output indicating success:
```
{
  "Result": "CLI already up to date (v1.0.4)"
}
```

Alternatively, you may update your CLI by running the `python3 install_cli.py` installation script again and receiving output resembling:

```
Latest CLI release: 1.0.4
Currently installed CLI: 1.0.4

The latest version (1.0.4) is already installed.
```



