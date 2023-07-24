---
title: "Install the Move Prover"
---

# Install the Move Prover

If you want to use the [Move Prover](../../../move/prover/index.md), install the Move Prover dependencies after [installing the CLI binary](.).

1. See [Building Aptos From Source](../../../guides/building-from-source.md)

1. Then, in the checked out aptos-core directory, install additional Move tools:
   <details>
   <summary>Linux / macOS</summary>

   1. Open a Terminal session.
   1. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh -yp`
   1. Update your current shell environment: `source ~/.profile`

   :::tip
   `dev_setup.sh -p` updates your `~./profile` with environment variables to support the installed Move Prover tools. You may need to set `.bash_profile` or `.zprofile` or other setup files for your shell.
   :::

   </details>
   <details>
   <summary>Windows</summary>

   1. Open a PowerShell terminal as an administrator.
   1. Run the dev setup script to prepare your environment: `PowerShell -ExecutionPolicy Bypass -File ./scripts/windows_dev_setup.ps1 -y`

   </details>

1. You can now run the Move Prover to prove an example:
   ```bash
   aptos move prove --package-dir aptos-move/move-examples/hello_prover/
   ```
