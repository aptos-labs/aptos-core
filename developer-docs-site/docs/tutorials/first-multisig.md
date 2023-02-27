---
title: "Your First Multisig"
slug: "your-first-multisig"
---

# Your First Multisig

This tutorial introduces assorted [K-of-N multi-signer authentication](../concepts/accounts.md#multisigner-authentication) operations, and supplements content from the following tutorials:

* [Your First Transaction](./first-transaction.md)
* [Your First Coin](./first-coin.md)
* [Your First Move Module](./first-move-module.md)

:::tip
Try out the above tutorials (which include dependency installations) before moving on to multisig operations.
:::

## Step 1: Pick an SDK

This tutorial, a community contribution, was created for the [Python SDK](../sdks/python-sdk.md).

Other developers are invited to add support for the [TypeScript SDK](../sdks/ts-sdk/index.md) and the [Rust SDK](../sdks/rust-sdk.md)!

## Step 2: Start the example

Navigate to the Python SDK examples directory:

```zsh
cd <aptos-core-parent-directory>/aptos-core/ecosystem/python/sdk/examples
```

Run the `multisig.py` example:

```zsh
poetry run python multisig.py
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
Alice: 0x9635724a9e3f7997c9975c76398e434504544d4044a6c34d5125f7825574e861
Bob:   0x4509faa7d24179368cc340548345acb7fde777191f1a69bda0fad619f0df67fd
Chad:  0x20454430a49bb2c7e4b768bfe586a543f4b2b25386e35a40aab18c1fa3413fd2

=== Authentication keys ===
Alice: 0x9635724a9e3f7997c9975c76398e434504544d4044a6c34d5125f7825574e861
Bob:   0x4509faa7d24179368cc340548345acb7fde777191f1a69bda0fad619f0df67fd
Chad:  0x20454430a49bb2c7e4b768bfe586a543f4b2b25386e35a40aab18c1fa3413fd2

=== Public keys ===
Alice: 0x850a1cb34a627ddca5f392c8039621dbfa737068fa08b179484cbe3d0edc31f8
Bob:   0x30f1d8c34526625998ca7edd8a5fae6d9930588c12158510fe9ad54b9e9b3a0f
Chad:  0x710790fa9cc2cb2540889b3c487d3417147dd6fbfa9f9dccf8da057cf34ca3cd
```

For each user, note the [account address](../concepts/accounts.md#account-address) and [authentication key](../concepts/accounts.md#single-signer-authentication) are identical, but the [public key](../concepts/accounts.md#creating-an-account) is different.

## Step 4: Generate a multisig account

Next generate a [K-of-N multi-signer](../concepts/accounts.md#multisigner-authentication) public key and account address for a multisig account requiring two of the three signatures:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_2
```

The multisig account address depends on the public keys of the single signers. (Hence, it will be different for each example.) But the output should resemble:

```zsh title=Output
=== 2-of-3 Multisig account ===
Account public key: 2-of-3 Multi-Ed25519 public key
Account address:    0x4779464145bf769bfaa550f95b473e8ea895fd1fd59d188a489cecf4b55601aa
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
Since it is a two-of-three multisig account, signatures are required only from two individual signers.

### Step 6.1: Gather individual signatures

First generate a raw transaction, signed by Alice and Bob, but not by Chad.

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_4
```

Again, signatures vary for each example run:

```zsh title=Output
=== Individual signatures ===
Alice: 0x370c9bdd467bb7de36ade1bffe66f29de153b0f4807a2a964d4c63f3e2c4dc0f7032e42db1f3c45ae55c6ea0cde0802ba1ccd4bfa655dcbd0040e507a7eea800
Bob:   0xf8bf5676affd2a8f5d594844c5fed63c3b02c599b9f9e8067b941169366dc72257ae31313332d4fa52f877b709cddcb80719c44a0ca9870f9c886f90286f9000
```

### Step 6.2: Submit the multisig transaction

Next generate a [multisig authenticator](../guides/sign-a-transaction.md#multisignature-transactions) and submit the transaction:


```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_5
```

```zsh title=Output
=== Submitting transaction ===
Transaction hash: 0x4da4bb16ff481e22f22cf049cda3faa2df60a4dbd2c40aabee441d890b23c1dd
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
Multisig balance: 39945700
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
Deedee's address:    0xdd2f5d6df096759915ffadfe6e73898117a40a4a99852fcdf44c05d23e794211
Deedee's public key: 0x90b17d64843ca7ece0f96498bca49c269f585eda24105b0b9894ab7976c52c73
Deedee's balance: 50000000
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
cap_rotate_key:   0x71e3b8357051409881de4286046b477612d59ee9826d03f017ade18b0b5025801eef302a113dc687bceec9618fe43f684f53d6eea48309ef7a146ac606180b0d
cap_update_table: 0xa3c0f67cbe4a98cc3956373a62e15ca03f1d8b2cc61c5650e85157a84806fa9e8834197b75fd7cfc9d2b15230ef96ceed7f8e35e4ec0775c4fef2ebdf4ef15048cea52b850bc688ff658c643aa6974bd4b734d2d700f45c977a69bf7491fdb888b18f9ed28c317b91e37153920339534278bcacb985f1fbd500457628311590c60000000
```

### Step 7.3 Rotate the authentication key

Now that the relevant signatures have been gathered, the authentication key rotation transaction can be submitted.
After it executes, the rotated authentication key matches the address of the first multisig account (the one that sent octas to Chad):

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_9
```

```zsh title=Output
=== Submitting authentication key rotation transaction ===
Auth key pre-rotation: 0xdd2f5d6df096759915ffadfe6e73898117a40a4a99852fcdf44c05d23e794211

Waiting for API server to update...

New auth key:         0x4779464145bf769bfaa550f95b473e8ea895fd1fd59d188a489cecf4b55601aa
1st multisig address: 0x4779464145bf769bfaa550f95b473e8ea895fd1fd59d188a489cecf4b55601aa
```

In other words, Deedee generated an account with a vanity address so that Alice, Bob, and Chad could use it as a multisig account.
Then Deedee and the Alice/Bob/Chad group (under the authority of Bob and Chad) approved to rotate the vanity account's authentication key to the authentication key of the first multisig account.

## Step 8: Perform Move package governance

In this section the multisig vanity account will publish a simple package, upgrade it, then invoke a [Move governance](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/upgrade_and_govern) script.

Here, [semantic versioning](https://semver.org/) is used to distinguish between versions `v1.0.0` and `v1.1.0` of the `UpgradeAndGovern` example package from the `move-examples` folder.

### Step 8.1: Review v1.0.0

Version 1.0.0 of the `UpgradeAndGovern` package contains a simple manifest and a single Move source file:

```toml title="Move.toml"
:!: static/move-examples/upgrade_and_govern/v1_0_0/Move.toml manifest
```

```rust title="parameters.move"
:!: static/move-examples/upgrade_and_govern/v1_0_0/sources/parameters.move module
```


As soon as the package is published, a `GovernanceParameters` resource is moved to the package account with the values specified by `GENESIS_PARAMETER_1` and `GENESIS_PARAMETER_2`.
Then, the `get_parameters()` function can be used to look up the governance parameters, but note that in this version there is no setter function.
The setter function will be added later.

### Step 8.2: Publish v1.0.0

Here, Alice and Chad will sign off on the publication transaction.

All compilation and publication operations are handled via the ongoing Python script:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_10
```

```zsh title=Output
=== Publishing v1.0.0 ===
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/v1_0_0 --named-addresses upgrade_and_govern=0xdd2f5d6df096759915ffadfe6e73898117a40a4a99852fcdf44c05d23e794211

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0x3b7b974b3d4e525f21c1da2f083e01648476dd416c3c3ce7b2c44a4600692a38

Waiting for API server to update...

Package name from on-chain registry: UpgradeAndGovern
On-chain upgrade number: 0
```

### Step 8.3: Review v1.1.0

Version 1.1.0 of the `UpgradeAndGovern` packages adds the following parameter setter functionality at the end of `parameters.move`:

```rust title=parameters.move
:!: static/move-examples/upgrade_and_govern/v1_1_0/sources/parameters.move appended
```

Here, the account that the package is published under has the authority to change the `GovernanceParameter` values from the genesis values set in `v1.0.0` to the new `parameter_1` and `parameter_2` values.

There is also a new module, `transfer.move`:

```rust title=transfer.move
:!: static/move-examples/upgrade_and_govern/v1_1_0/sources/transfer.move module
```

This module simply looks up the `GovernanceParameter` values, and treats them as the amount of octas to send to two recipients.

Lastly, the manifest has been updated with the new version number:

```toml title=Move.toml
:!: static/move-examples/upgrade_and_govern/v1_1_0/Move.toml manifest
```

### Step 8.4: Upgrade to v1.1.0

Alice, Bob, and Chad will all sign off on this publication transaction, which results in an upgrade.
This process is almost identical to that of the `v1.0.0` publication:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_11
```

```zsh title=Output
=== Publishing v1.1.0 ===
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/v1_1_0 --named-addresses upgrade_and_govern=0xdd2f5d6df096759915ffadfe6e73898117a40a4a99852fcdf44c05d23e794211

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0xcfbb3207169a0058fb43467df4964b4d52855259bd183c648e8b358c1899190e

Waiting for API server to update...

On-chain upgrade number: 1
```

Note that the on-chain upgrade number has been incremented by 1.

### Step 8.6: Review the governance script

`UpgradeAndGovern` version 1.1.0 also includes a Move script defined in `set_and_transfer.move`:

```rust title=set_and_transfer.move
:!: static/move-examples/upgrade_and_govern/v1_1_0/scripts/set_and_transfer.move script
```

This script calls the governance parameter setter using hard-coded values defined at the top of the script, then calls the octa transfer function.
The script accepts as arguments the signature of the account hosting the package, as well as two target addresses for the transfer operation.

Note that both functions in the script are `public entry fun` functions, which means that everything achieved in the script could be performed without a script.
However, a non-script approach would require two transactions instead of just one, and would complicate the signature aggregation process:
in practical terms, Alice, Bob, and/or Chad would likely have to send single-signer transaction signatures around through off-chain communication channels, and a *scribe* for the group would then have to submit a multisig `Authenticator` (for *each* `public entry fun` call).
Hence in a non-script approach, extra operational complexity can quickly introduce opportunities for consensus failure.

A Move script, by contrast, collapses multiple governance function calls into a single transaction, and moreover, Move scripts can be published in a public forum like GitHub so that all signatories can review the actual function calls before they sign the script.

### Step 8.5: Execute the governance script

Alice and Bob sign off on the Move script, which sends coins from the vanity multisig account to their personal accounts.
Here, the amounts sent to each account are specified in the hard-coded values from the script.

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_12
```

```zsh title=Output
=== Invoking Move script ===
Transaction hash: 0x05903b30e3a88829d5ba802a671ba5b692c53bb870e2fb393a2b9cbdf153b120

Waiting for API server to update...

Alice's balance: 10000300
Bob's balance:   20000200
Chad's balance:  30000100
```

---

Congratulations on completing the tutorial on K-of-N multi-signer authentication operations!
