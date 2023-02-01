---
title: "Your First Multisig"
slug: "your-first-multisig"
---

# Your First Multisig

This tutorial introduces assorted [K-of-N multisigner authentication](../concepts/accounts.md#multisigner-authentication) operations, and supplements content from the following tutorials:

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

Navigate to the Python SDK directory:

```zsh
cd <aptos-core-parent-directory>/aptos-core/ecosystem/python/sdk/examples
```

Run the `multisig.py` example:

```zsh
python multisig.py
```

## Step 3: Generate single signer accounts

First, we will generate single signer accounts for Alice, Bob, and Chad:

```python
:!: static/sdks/python/examples/multisig.py section_1
```

Fresh accounts are generated for each example run, but the output should resemble:

```zsh
=== Account addresses ===
Alice: 0xb6a60cf0af64152a0c305dc4dc93a0298c2b373782cd4880e10091acdcc7f315
Bob:   0x41ac06b3faceef0d018b8e9be59b3e5cfee5a858e420f25b703b37a9af6ef718
Chad:  0xf4f6c6949a903e3055ab05d220eea75d29ba56abb5a744476fd37214b3d11000

=== Authentication keys ===
Alice: 0xb6a60cf0af64152a0c305dc4dc93a0298c2b373782cd4880e10091acdcc7f315
Bob:   0x41ac06b3faceef0d018b8e9be59b3e5cfee5a858e420f25b703b37a9af6ef718
Chad:  0xf4f6c6949a903e3055ab05d220eea75d29ba56abb5a744476fd37214b3d11000

=== Public keys ===
Alice: 0xe44af056dc3eea75d2c750ace58a8d5f5164a7f5fd52eda18c16d1c7cc355c59
Bob:   0x554868a66a50a220f1b1065572406ec5ecb587b4bcab2e6434b022303b3094bc
Chad:  0x2f414aecc6a7dd204ae81fd2e4f33934cdb00b80d1abe2a7568c85c43870373f
```

For each user, note the [account address](../concepts/accounts.md#account-address) and [authentication key](../concepts/accounts.md#single-signer-authentication) are identical, but the [public key](../concepts/accounts.md#creating-an-account) is different.

## Step 4: Generate a multisig account

Next generate a [K-of-N multisigner](../concepts/accounts.md#multisigner-authentication) public key and account address for a multisig account requiring two of the three signatures:

```python
:!: static/sdks/python/examples/multisig.py section_2
```

The multisig account address depends on the public keys of the single signers. (Hence, it will be different for each example.) But the output should resemble:

```zsh
=== 2-of-3 Multisig account ===
Account public key: 2-of-3 Multi-Ed25519 public key
Account address:    0x12133ea3b7d710216881a01f9f9cd59693f6a9c81c88a200b90123fc11a2b984
```

## Step 5: Fund all accounts

Next fund all accounts:

```python
:!: static/sdks/python/examples/multisig.py section_3
```

```zsh
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

```python
:!: static/sdks/python/examples/multisig.py section_4
```

Again, signatures vary for each example run:

```zsh
=== Individual signatures ===
Alice: 0x8ccfbab4eb72fda7d10c4b9e4dfc4c2f34bcef78cccef4bb7e3d6666b778de70b9368429dcbd04e677c0e283a27cdfe72622aaf6e99d7dd7a002c9d8adac440c
Bob:   0xfa9bec52c7d1c8b8a68691a1a08c7e41cb6f66037e717cc99c533c5e7059c527077749403968c2b673e5460dc23f34af34f88158535f26fd796f8b4d1f6ad104
```

### Step 6.2: Submit the multisig transaction

Next generate a [multisig authenticator](../guides/sign-a-transaction.md#multisignature-transactions) and submit the transaction:


```python
:!: static/sdks/python/examples/multisig.py section_5
```

```zsh
=== Submitting transaction ===
Transaction hash: 0xa0d59cb79b4a6ea5392381d2de1140adf9a779d3541d25d3a6d39ebf5ac4c2f5
```

### Step 6.3: Check balances

Check the new account balances:

```python
:!: static/sdks/python/examples/multisig.py section_6
```

```zsh
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

```python
:!: static/sdks/python/examples/multisig.py section_7
```

```zsh
=== Funding vanity address ===
Deedee's address:    0xdd41eb16f1ec28fdaa816a95c47c611e7ff24bb5e864daa8bb42cc8c2e723689
Deedee's public key: 0x54daba7e8c6e54217ff54cd2af3735099f5fb89dcbb22f5c0f670203b449b6c4
Deedee's balance: 50000000
```

### Step 7.2 Sign a rotation proof challenge

Deedee and the two-of-three multisig must both sign a `RotationProofChallenge`, yielding two signatures.
Deedee's signature, `cap_rotate_key`, verifies that she approves of the authentication key rotation.
The multisig signature, `cap_update_table`, verifies that the multisig approves of the authentication key rotation.
Here, Bob and Chad provide individual signatures for the multisig:

```python
:!: static/sdks/python/examples/multisig.py section_8
```

```zsh
=== Signing rotation proof challenge ===
cap_rotate_key:   0x6d4bdafa2eed35a32314730e8d892d94fc9318c25b0f3527fefd8d2b0f29853cdfdcd9560f469ddbba7b1107c661036155f10e340abdca15c97c7129bd0cc603
cap_update_table: 0x0702a4754c7653fe8e56e82af631a21db4a2672f01a5443bb86638aea0351b79a074be81d48745387433dcf9b4e22b8dd0f7d35c592e510673a3d17dd0ea040ec0a531af37259332902a9140822a4f7af5e40186da512343d7e503228797272130729118bdee53383c17c7ad0a549f0283f78c2da8a958c38e5c528caa7de00360000000
```

### Step 7.3 Rotate the authentication key

Now that the relevant signatures have been gathered, the authentication key rotation transaction can be submitted.
After it executes, the rotated authentication key matches the address of the first multisig account (the one that sent octas to Chad):

```python
:!: static/sdks/python/examples/multisig.py section_9
```

```zsh
=== Submitting authentication key rotation transaction ===
Auth key pre-rotation: 0xddb819d72eca401eb4d5d894a3d72540fd1ee47fef2daa0867c9c05fc469925f

Waiting for client to update...

Auth key post-rotation: 0xeec99e7567cceebcbdffdffd5248d5c6dde6e332ca7c1fa04d5cc42f228fb3c2
First multisig address: 0xeec99e7567cceebcbdffdffd5248d5c6dde6e332ca7c1fa04d5cc42f228fb3c2
```

In other words, Deedee generated an account with a vanity address so that Alice, Bob, and Chad could use it as a multisig account.
Then Deedee and the Alice/Bob/Chad group (under the authority of Bob and Chad) approved to rotate the vanity account's authentication key to the authentication key of the first multisig account.

## Step 8: Perform Move package governance

In this section the multisig vanity account will publish a simple package, upgrade it, then invoke a Move governance](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/upgrade_and_govern) script.

Here, [semantic versioning](https://semver.org/) is used to distinguish between versions `v1.0.0` and `v1.1.0` of the `UpgradeAndGovern` example package from the `move-examples` folder.

### Step 8.1: Review v1.0.0

Version 1.0.0 of the `UpgradeAndGovern` package contains a simple manifest and a single Move source file:

```toml title="Move.toml"
[package]
name = 'UpgradeAndGovern'
version = '1.0.0'

[addresses]
upgrade_and_govern = '_'

[dependencies.AptosFramework]
local = '../../../framework/aptos-framework'
```

```rust title="parameters.move"
/// Mock on-chain governance parameters.
module upgrade_and_govern::parameters {

    struct GovernanceParameters has key {
        parameter_1: u64,
        parameter_2: u64
    }

    const GENESIS_PARAMETER_1: u64 = 123;
    const GENESIS_PARAMETER_2: u64 = 456;

    fun init_module(
        upgrade_and_govern: &signer
    ) {
        let governance_parameters = GovernanceParameters{
            parameter_1: GENESIS_PARAMETER_1,
            parameter_2: GENESIS_PARAMETER_2};
        move_to<GovernanceParameters>(
            upgrade_and_govern, governance_parameters);
    }

    public fun get_parameters():
    (u64, u64)
    acquires GovernanceParameters {
        let governance_parameters_ref =
            borrow_global<GovernanceParameters>(@upgrade_and_govern);
        (governance_parameters_ref.parameter_1,
         governance_parameters_ref.parameter_2)
    }

}
```

As soon as the package is published, a `GovernanceParameters` resource is moved to the package account with the values specified by `GENESIS_PARAMETER_1` and `GENESIS_PARAMETER_2`.
Then, the `get_parameters()` function can be used to look up the governance parameters, but note that in this version there is no setter function.
The setter function will be added later.

### Step 8.2: Publish v1.0.0

Here, Alice and Chad will sign off on the publication transaction.

All compilation and publication operations are handled via the ongoing Python script:

```python
:!: static/sdks/python/examples/multisig.py section_10
```

```zsh
=== Publishing v1.0.0 ===
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/v1_0_0 --named-addresses upgrade_and_govern=0xdd41eb16f1ec28fdaa816a95c47c611e7ff24bb5e864daa8bb42cc8c2e723689

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0xa71189804cf90ecbe8ec011a3e89feddc8910529a390ce6bdd311c3224ad9d48

Waiting for API server to update...

Package name from on-chain registry: UpgradeAndGovern
On-chain upgrade number: 0
```

### Step 8.3: Review v1.1.0

Version 1.1.0 of the `UpgradeAndGovern` packages adds the following parameter setter functionality at the end of `parameters.move`:

```rust
use std::signer::address_of;

const E_INVALID_AUTHORITY: u64 = 0;

public entry fun set_parameters(
    upgrade_and_govern: &signer,
    parameter_1: u64,
    parameter_2: u64
) acquires GovernanceParameters {
    assert!(address_of(upgrade_and_govern) == @upgrade_and_govern,
            E_INVALID_AUTHORITY);
    let governance_parameters_ref_mut =
        borrow_global_mut<GovernanceParameters>(@upgrade_and_govern);
    governance_parameters_ref_mut.parameter_1 = parameter_1;
    governance_parameters_ref_mut.parameter_2 = parameter_2;
}
```

Here, the account that the package is published under has the authority to change the `GovernanceParameter` values from the genesis values set in `v1.0.0` to the new `parameter_1` and `parameter_2` values.

There is also a new module, `transfer.move`:

```rust
/// Mock coin transfer module that invokes governance parameters.
module upgrade_and_govern::transfer {

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use upgrade_and_govern::parameters;

    public entry fun transfer_octas(
        from: &signer,
        to_1: address,
        to_2: address
    ) {
        let (amount_1, amount_2) = parameters::get_parameters();
        coin::transfer<AptosCoin>(from, to_1, amount_1);
        coin::transfer<AptosCoin>(from, to_2, amount_2);
    }

}
```

This module simply looks up the `GovernanceParameter` values, and treats them as the amount of octas to send to two recipients.

Lastly, the manifest has been updated with the new version number:

```toml
[package]
name = 'UpgradeAndGovern'
version = '1.1.0'

[addresses]
upgrade_and_govern = '_'

[dependencies.AptosFramework]
local = '../../../framework/aptos-framework'
```

### Step 8.4: Upgrade to v1.1.0

Alice, Bob, and Chad will all sign off on this publication transaction, which results in an upgrade.
This process is almost identical to that of the `v1.0.0` publication:

```python
:!: static/sdks/python/examples/multisig.py section_11
```

```zsh
=== Publishing v1.1.0 ===
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/v1_1_0 --named-addresses upgrade_and_govern=0xdd41eb16f1ec28fdaa816a95c47c611e7ff24bb5e864daa8bb42cc8c2e723689

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0x874fdb388b64dfaccd27a8a12a85cab59237bf172ddca3f1c16e1f09db466a9f

Waiting for API server to update...

On-chain upgrade number: 1
```

Note that the on-chain upgrade number has been incremented by 1.

### Step 8.6: Review the governance script

`UpgradeAndGovern` version 1.1.0 also includes a Move script defined in `set_and_transfer.move`:

```rust
script {
    use upgrade_and_govern::parameters;
    use upgrade_and_govern::transfer;

    const PARAMETER_1: u64 = 300;
    const PARAMETER_2: u64 = 200;

    fun main(
        upgrade_and_govern: &signer,
        to_1: address,
        to_2: address
    ) {
        parameters::set_parameters(
            upgrade_and_govern, PARAMETER_1, PARAMETER_2);
        transfer::transfer_octas(upgrade_and_govern, to_1, to_2);
    }
}
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

```python
:!: static/sdks/python/examples/multisig.py section_12
```

```zsh
=== Invoking Move script ===
Transaction hash: 0x98f8e74a505f5905b0336d97df60f3dd8972214a154d4f5fa945c1335814a74b

Waiting for API server to update...

Alice's balance: 10000300
Bob's balance:   20000200
Chad's balance:  30000100
```

Congratulations on completing the tutorial on K-of-N multisigner authentication operations!
