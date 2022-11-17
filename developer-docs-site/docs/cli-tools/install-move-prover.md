---
title: "Install Move Prover"
id: "install-move-prover"
---

# Install Move Prover to Validate Code

If you want to use the [Move Prover](https://github.com/move-language/move/blob/main/language/move-prover/doc/user/prover-guide.md) to validate your Move code, install the Move Prover dependencies after [installing the CLI binary](aptos-cli-tool/install-aptos-cli.md).

For some sweet examples, see:
https://github.com/Zellic/move-prover-examples

:::tip
Currently, Windows is not supported by the Move Prover.
:::

<details>
<summary>Prover macOS installation</summary>

### macOS

:::tip
These instructions have been tested on macOS Monterey (12.6)
:::

1. Ensure you have `brew` installed https://brew.sh/.
1. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
1. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
1. Change directory into `aptos-core`: `cd aptos-core`
1. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh -yp`
1. Source the profile file: `source ~/.profile`.

   :::info
   Note that you have to include environment variable definitions in `~/.profile` into your shell. Depending on your setup, the  `~/.profile` may be already automatically loaded for each login shell, or it may not. If not, you may
   need to add `. ~/.profile` to your `~/.bash_profile` or other shell configuration manually.
   :::

1. You can now run the Move Prover to prove an example:
    ```bash
    aptos move prove --package-dir aptos-move/move-examples/hello_prover/
    ```
   
</details>

<details>
<summary>Prover Linux installation</summary>

### Linux

:::tip 
Some Linux distributions are not supported. Currently, OpenSUSE and Amazon Linux do not support the automatic installation via the `dev_setup.sh` script.
:::

1. Ensure you have `git` installed https://git-scm.com/book/en/v2/Getting-Started-Installing-Git.
1. Clone the Aptos core repo:  `git clone https://github.com/aptos-labs/aptos-core.git`.
1. Change directory into `aptos-core`: `cd aptos-core`
1. Run the dev setup script to prepare your environment: `./scripts/dev_setup.sh -yp`
1. Source the profile file: `source ~/.profile`.

   :::info
   Note that you have to include environment variable definitions in `~/.profile` into your shell. Depending on your setup, the  `~/.profile` may be already automatically loaded for each login shell, or it may not. If not, you may
   need to add `. ~/.profile` to your `~/.bash_profile` or other shell configuration manually.
   :::

1. You can now run the Move Prover to prove an example:
    ```bash
    aptos move prove --package-dir aptos-move/move-examples/hello_prover/
    ```

</details>
