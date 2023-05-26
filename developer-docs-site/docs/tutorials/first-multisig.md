---
title: "Your First Multisig"
slug: "your-first-multisig"
---

# Your First Multisig

This tutorial introduces assorted [K-of-N multi-signer authentication](../concepts/accounts.md#multi-signer-authentication) operations and supplements content from the following tutorials:

* [Your First Transaction](./first-transaction.md)
* [Your First Coin](./first-coin.md)
* [Your First Move Module](./first-move-module.md)

:::tip
Try out the above tutorials (which include dependency installations) before moving on to multisig operations.
:::

## Step 1: Pick an SDK

This tutorial, a community contribution, was created for the [Python SDK](../sdks/python-sdk.md).

Other developers are invited to add support for the [TypeScript SDK](../sdks/ts-sdk/index.md), [Rust SDK](../sdks/rust-sdk.md), and [Unity SDK](../sdks/unity-sdk.md)!

## Step 2: Start the example

Navigate to the Python SDK directory:

```zsh
cd <aptos-core-parent-directory>/aptos-core/ecosystem/python/sdk/
```

Run the `multisig.py` example:

```zsh
poetry run python -m examples.multisig
```

:::tip
This example uses the Aptos devnet, which has historically been reset each Thursday.
Make sure devnet is live when you try running the example!
:::

## Step 3: Generate single signer accounts

First, we will generate single signer accounts for Alice, Bob, and Chad:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_1
```

Fresh accounts are generated for each example run, but the output should resemble:

```zsh title=Output
=== Account addresses ===
Alice: 0x93c1b7298d53dd0d517f503f2d3188fc62f6812ab94a412a955720c976fecf96
Bob:   0x85eb913e58d0885f6a966d98c76e4d00714cf6627f8db5903e1cd38cc86d1ce0
Chad:  0x14cf8dc376878ac268f2efc7ba65a2ee0ac13ceb43338b6106dd88d8d23e087a

=== Authentication keys ===
Alice: 0x93c1b7298d53dd0d517f503f2d3188fc62f6812ab94a412a955720c976fecf96
Bob:   0x85eb913e58d0885f6a966d98c76e4d00714cf6627f8db5903e1cd38cc86d1ce0
Chad:  0x14cf8dc376878ac268f2efc7ba65a2ee0ac13ceb43338b6106dd88d8d23e087a

=== Public keys ===
Alice: 0x3f23f869632aaa4378f3d68560e08d18b1fc2e80f91d6f9d1b39d720b0ef7a3f
Bob:   0xcf21e85337a313bdac33d068960a3e52d22ce0e6190e9acc03a1c9930e1eaf3e
Chad:  0xa1a2aef8525eb20655387d3ed50b9a3ea1531ef6117f579d0da4bcf5a2e1f76d
```

For each user, note the [account address](../concepts/accounts.md#account-address) and [authentication key](../concepts/accounts.md#single-signer-authentication) are identical, but the [public key](../concepts/accounts.md#creating-an-account) is different.

## Step 4: Generate a multisig account

Next generate a [K-of-N multi-signer](../concepts/accounts.md#multi-signer-authentication) public key and account address for a multisig account requiring two of the three signatures:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_2
```

The multisig account address depends on the public keys of the single signers. (Hence, it will be different for each example.) But the output should resemble:

```zsh title=Output
=== 2-of-3 Multisig account ===
Account public key: 2-of-3 Multi-Ed25519 public key
Account address:    0x08cac3b7b7ce4fbc5b18bc039279d7854e4c898cbf82518ac2650b565ad4d364
```

## Step 5: Fund all accounts

Next fund all accounts:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_3
```

```zsh title=Output
=== Funding accounts ===
Alice's balance:  10000000
Bob's balance:    20000000
Chad's balance:   30000000
Multisig balance: 40000000
```

## Step 6: Send coins from the multisig

This transaction will send 100 octas from the multisig account to Chad's account.
Since it is a two-of-three multisig account, signatures are required from only two individual signers.

### Step 6.1: Gather individual signatures

First generate a raw transaction, signed by Alice and Bob, but not by Chad.

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_4
```

Again, signatures vary for each example run:

```zsh title=Output
=== Individual signatures ===
Alice: 0x41b9dd65857df2d8d8fba251336357456cc9f17974de93292c13226f560102eac1e70ddc7809a98cd54ddee9b79853e8bf7d18cfef23458f23e3a335c3189e0d
Bob:   0x6305101f8f3ad5a75254a8fa74b0d9866756abbf359f9e4f2b54247917caf8c52798a36c5a81c77505ebc1dc9b80f2643e8fcc056bcc4f795e80b229fa41e509
```

### Step 6.2: Submit the multisig transaction

Next generate a [multisig authenticator](../integration/sign-a-transaction.md#multisignature-transactions) and submit the transaction:


```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_5
```

```zsh title=Output
=== Submitting transfer transaction ===
Transaction hash: 0x3ff2a848bf6145e6df3abb3ccb8b94fefd07ac16b4acb0c694fa7fa30b771f8c
```

### Step 6.3: Check balances

Check the new account balances:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_6
```

```zsh title=Output
=== New account balances===
Alice's balance:  10000000
Bob's balance:    20000000
Chad's balance:   30000100
Multisig balance: 39999300
```

Note that even though Alice and Bob signed the transaction, their account balances have not changed.
Chad, however, has received 100 octas from the multisig account, which assumed the gas costs of the transaction and thus has had more than 100 octas deducted.

## Step 7: Create a vanity address multisig

In this section, a fourth user named Deedee will generate a vanity address, then rotate her account to the two-of-three multisig.

### Step 7.1 Generate a vanity address

A fourth user, Deedee, wants her account address to start with `0xdd..`, so she generates random accounts until she finds one with a matching account address:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_7
```

```zsh title=Output
=== Funding vanity address ===
Deedee's address:    0xdd86860ae7f77f58d08188e1c39fbc6a2f7cec782f09f6767f8367d84357ed57
Deedee's public key: 0xdbf02311c45903f0217e9ab76ca64007c2876363118bb422922410d4cfe9964c
Deedee's balance:    50000000
```

### Step 7.2 Sign a rotation proof challenge

Deedee and the two-of-three multisig must both sign a `RotationProofChallenge`, yielding two signatures.
Deedee's signature, `cap_rotate_key`, verifies that she approves of the authentication key rotation.
The multisig signature, `cap_update_table`, verifies that the multisig approves of the authentication key rotation.
Here, Bob and Chad provide individual signatures for the multisig:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_8
```

```zsh title=Output
=== Signing rotation proof challenge ===
cap_rotate_key:   0x3b2906df78bb79f210051e910985c358572c2ec7cdd03f688480fb6adf8d538df48a52787d5651d85f2959dcca88d58da49709c9c0dc9c3c58b67169ec1e1c01
cap_update_table: 0x965fd11d7afe14396e5af40b8ffb78e6eb6f7caa1f1b1d8c7b819fdd6045864e70258788ec1670a3989c85f8cc604f4b54e43e1ce173a59aa0a6f7cf124fd902dcbb2ad53467d05c144260b2be1814777c082d8db437698b00e6a2109a015a565ff5783e827a21a4c07ae332b56398b69dfdbcc08b8ad5585dc1ac649b74730760000000
```

### Step 7.3 Rotate the authentication key

Now that the relevant signatures have been gathered, the authentication key rotation transaction can be submitted.
After it executes, the rotated authentication key matches the address of the first multisig account (the one that sent octas to Chad):

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_9
```

```zsh title=Output
=== Submitting authentication key rotation transaction ===
Auth key pre-rotation: 0xdd86860ae7f77f58d08188e1c39fbc6a2f7cec782f09f6767f8367d84357ed57
Transaction hash:      0x57c66089a1b81e2895a2d6919ab19eb90c4d3c3cbe9fecab8169eaeedff2c6e6
New auth key:          0x08cac3b7b7ce4fbc5b18bc039279d7854e4c898cbf82518ac2650b565ad4d364
1st multisig address:  0x08cac3b7b7ce4fbc5b18bc039279d7854e4c898cbf82518ac2650b565ad4d364
```

In other words, Deedee generated an account with a vanity address so that Alice, Bob, and Chad could use it as a multisig account.
Then Deedee and the Alice/Bob/Chad group (under the authority of Bob and Chad) approved to rotate the vanity account's authentication key to the authentication key of the first multisig account.

## Step 8: Perform Move package governance

In this section, the multisig vanity account will publish a simple package, upgrade it, then invoke a Move script.

Move source code for this section is found in the [`upgrade_and_govern`](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/upgrade_and_govern) directory.

### Step 8.1: Review genesis package

The `UpgradeAndGovern` genesis package (version `1.0.0`) contains a simple `.toml` manifest and a single Move source file:

```toml title="Move.toml"
:!: static/move-examples/upgrade_and_govern/genesis/Move.toml manifest
```

```rust title="parameters.move"
:!: static/move-examples/upgrade_and_govern/genesis/sources/parameters.move module
```

As soon as the package is published, a `GovernanceParameters` resource is moved to the `upgrade_and_govern` package account with the values specified by `GENESIS_PARAMETER_1` and `GENESIS_PARAMETER_2`.
Then, the `get_parameters()` function can be used to look up the governance parameters, but note that in this version there is no setter function.
The setter function will be added later.

### Step 8.2: Publish genesis package

Here, Alice and Chad will sign off on the publication transaction.

All compilation and publication operations are handled via the ongoing Python script:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_10
```

```zsh title=Output
=== Genesis publication ===
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/genesis --named-addresses upgrade_and_govern=0xdd86860ae7f77f58d08188e1c39fbc6a2f7cec782f09f6767f8367d84357ed57

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0x3c65c681194d6c64d73dc5d0cbcbad62e99a997b8600b8edad6847285e580c13
Package name from on-chain registry: UpgradeAndGovern
On-chain upgrade number: 0
```

### Step 8.3: Review package upgrades

The `UpgradeAndGovern` upgrade package adds the following parameter setter functionality at the end of `parameters.move`:

```rust title=parameters.move
:!: static/move-examples/upgrade_and_govern/upgrade/sources/parameters.move appended
```

Here, the account that the package is published under, `upgrade_and_govern`, has the authority to change the `GovernanceParameter` values from the genesis values to the new `parameter_1` and `parameter_2` values.

There is also a new module, `transfer.move`:

```rust title=transfer.move
:!: static/move-examples/upgrade_and_govern/upgrade/sources/transfer.move module
```

This module simply looks up the `GovernanceParameter` values, and treats them as the amount of octas to send to two recipients.

Lastly, the manifest has been updated with the new version number `1.1.0`:

```toml title=Move.toml
:!: static/move-examples/upgrade_and_govern/upgrade/Move.toml manifest
```

### Step 8.4: Upgrade the package

Alice, Bob, and Chad will all sign off on this publication transaction, which results in an upgrade.
This process is almost identical to that of the genesis publication, with the new `transfer` module listed after the `parameters` module:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_11
```

:::tip
Modules that `use` other modules must be listed *after* the modules they use.
:::

```zsh title=Output
=== Upgrade publication ===
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/upgrade --named-addresses upgrade_and_govern=0xdd86860ae7f77f58d08188e1c39fbc6a2f7cec782f09f6767f8367d84357ed57

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0x0f0ea3bb7271ddeaceac5b49ff5503d6c652d4746c1510e47665ceee5a89aaf0
On-chain upgrade number: 1
```

Note that the on-chain upgrade number has been incremented by 1.

### Step 8.6: Review the governance script

The `UpgradeAndGovern` upgrade package also includes a Move script at `set_and_transfer.move`:

```rust title=set_and_transfer.move
:!: static/move-examples/upgrade_and_govern/upgrade/scripts/set_and_transfer.move script
```

This script calls the governance parameter setter using hard-coded values defined at the top of the script, then calls the octa transfer function.
The script accepts as arguments the signature of the account hosting the package, as well as two target addresses for the transfer operation.

Note that both functions in the script are `public entry fun` functions, which means that everything achieved in the script could be performed without a script.
However, a non-script approach would require two transactions instead of just one, and would complicate the signature aggregation process:
in practical terms, Alice, Bob, and/or Chad would likely have to send single-signer transaction signatures around through off-chain communication channels, and a *scribe* for the group would then have to submit a multisig `Authenticator` (for *each* `public entry fun` call).
Hence in a non-script approach, extra operational complexity can quickly introduce opportunities for consensus failure.

A Move script, by contrast, collapses multiple governance function calls into a single transaction; and moreover, Move scripts can be published in a public forum like GitHub so that all signatories can review the actual function calls before they sign the script.

### Step 8.5: Execute the governance script

Alice and Bob sign off on the Move script, which sends coins from the vanity multisig account to their personal accounts.
Here, the amounts sent to each account are specified in the hard-coded values from the script.

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_12
```

```zsh title=Output
=== Invoking Move script ===
Transaction hash: 0xd06de4bd9fb12a9f3cbd8ce1b9a9fd47ea2b923a8cfac21f9788869430e4149b
Alice's balance:  10000300
Bob's balance:    20000200
Chad's balance:   30000100
```

---

Congratulations on completing the tutorial on K-of-N multi-signer authentication operations!
