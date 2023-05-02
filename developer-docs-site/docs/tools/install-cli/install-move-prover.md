---
title: "Install the Move Prover"
---

# Install the Move Prover

If you want to use the [Move Prover](../../move/prover/index.md), install the Move Prover dependencies after [installing the CLI binary](.).

:::tip
Currently, Windows is not supported by the Move Prover.
:::

1. See [Building Aptos From Source](../../guides/building-from-source.md)

1. Then, in the checked out aptos-core directory, install additional Move tools:

   ```bash
   ./scripts/dev_setup.sh -yp
   source ~/.profile
   ```

   :::info
   `dev_setup.sh -p` updates your `~./profile` with environment variables to support the installed Move Prover tools. You may need to set `.bash_profile` or `.zprofile` or other setup files for your shell.
   :::

1. You can now run the Move Prover to prove an example:
    ```bash
    aptos move prove --package-dir aptos-move/move-examples/hello_prover/
