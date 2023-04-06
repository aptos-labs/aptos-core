---
title: "Install the Move Prover"
id: "install-move-prover"
---

# Install the Move Prover to Validate Code

If you want to use the [Move Prover](https://github.com/move-language/move/blob/main/language/move-prover/doc/user/prover-guide.md) to validate your Move code, install the Move Prover dependencies after [installing the CLI binary](aptos-cli-tool/install-aptos-cli.md).

For use, see the [supporting resources](#supporting-resources) below.

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

## Supporting resources

### Documentation

- [Move Prover User Guide](https://github.com/move-language/move/tree/main/language/move-prover)
- [Move Spec Language Reference](https://github.com/move-language/move/blob/main/language/move-prover/doc/user/spec-lang.md)

### Frameworks

- [Diem Framework](https://github.com/move-language/move/tree/main/language/documentation/examples/diem-framework)
- [Aptos Framework](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/framework)

### Examples

- [Move Prover Examples by Zellic](https://github.com/zellic/move-prover-examples)
- [`basic-coin` example](https://github.com/move-language/move/tree/main/language/documentation/examples/experimental/basic-coin)
- [`math-puzzle` example](https://github.com/move-language/move/tree/main/language/documentation/examples/experimental/math-puzzle)
- [rounding-error` example](https://github.com/move-language/move/tree/main/language/documentation/examples/experimental/rounding-error)
- [`verify-sort` example](https://github.com/move-language/move/tree/main/language/documentation/examples/experimental/verify-sort)

### Tutorials
- [The Move Tutorial, chapter 7](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/move-tutorial#step-7--use-the-move-prover)
- [The Move Prover: A Practical Guide by OtterSec](https://osec.io/blog/tutorials/2022-09-16-move-prover/)
- [Verify Smart Contracts in Aptos with the Move Prover by MoveBit](https://blog.movebit.xyz/post/move-prover-tutorial-part-1.html)
- [Formal Verification, the Move Language, and the Move Prover by Certik](https://www.certik.com/resources/blog/2wSOZ3mC55AB6CYol6Q2rP-formal-verification-the-move-language-and-the-move-prover)
- [The Move Prover: Quality Assurance of Formal Verification by Certik](https://www.certik.com/resources/blog/1NygvVeqIwhbUk1U1q3vJF-the-move-prover-quality-assurance-of-formal-verification)

## Presentations

- [Verifying Smart Contracts with Move Prover by Wolfgang Grieskamp (video)](https://drive.google.com/file/d/1DpI-rQ25Kq1jqMGioLgVrG3YuCqJHVMm/view?usp=share_link)
- [Move Prover - Best Practices & Tricks - A User’s Perspective by Xu-Dong@MoveBit (slides)](https://docs.google.com/presentation/d/1SuV0m5gGxSN9SaLdj9lLmTjspJ2xN1TOWgnwvdWbKEY/edit?usp=sharing)
- [Formal verification of Move programs for the Libra blockchain by David Dill (video)](http://www.fields.utoronto.ca/talks/Formal-verification-Move-programs-Libra-blockchain)

## Conference papers

- Zhong, Jingyi Emma, Kevin Cheang, Shaz Qadeer, Wolfgang Grieskamp, Sam Blackshear, Junkil Park, Yoni Zohar, Clark Barrett, and David L. Dill. "The move prover." In *International Conference on Computer Aided Verification*, pp. 137-150. Springer, Cham, 2020.Harvard
    - https://research.facebook.com/publications/the-move-prover/
- Dill, David, Wolfgang Grieskamp, Junkil Park, Shaz Qadeer, Meng Xu, and Emma Zhong. "Fast and reliable formal verification of smart contracts with the Move prover." In *International Conference on Tools and Algorithms for the Construction and Analysis of Systems*, pp. 183-200. Springer, Cham, 2022.Harvard
    - https://arxiv.org/abs/2110.08362
