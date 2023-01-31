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

Run the [`multisig.py`](../../../ecosystem/python/sdk/examples/multisig.py) example:

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
Alice: 0xf07c9ef4c19a60462441e3cab0c823ef4c3e74fafa0504eab21a8f6ef623d3f4
Bob:   0x7c7beba6f5d597c34d1fc8ab123d766382009482d1b7735e57a8b7ad657db0b3
Chad:  0x5f587d416b661e115d14153dd711368f328d64a716525027be6ffce7e8239cd8

=== Authentication keys ===
Alice: 0xf07c9ef4c19a60462441e3cab0c823ef4c3e74fafa0504eab21a8f6ef623d3f4
Bob:   0x7c7beba6f5d597c34d1fc8ab123d766382009482d1b7735e57a8b7ad657db0b3
Chad:  0x5f587d416b661e115d14153dd711368f328d64a716525027be6ffce7e8239cd8

=== Public keys ===
Alice: 0xb2cf7ad02bcea120d25a5c1250235d759ec652c07967f38048988d7be41967d1
Bob:   0xce1912ed9a6ecadca634112c60dba9e124884e9b67006a9e089df299824f3a23
Chad:  0x6f941d55c33dcc3be13d34cc5d168dbf129ebf1208fa087dc4031160b2dd298a
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
Account address:    0xeec99e7567cceebcbdffdffd5248d5c6dde6e332ca7c1fa04d5cc42f228fb3c2
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
Alice: 0x0564f37fdff4d59fd75e604f2c895329004e3c36b4943f5760b8fd0632155f53b8ddbe7132e874bf22be5af32de212d6670bd07f285128c5148a33fee7fdcd0f
Bob:   0x935e5aaab2bdfd3a8c97ad91491092a259203d7ff11cd5522af1bcdda8be0fb92712ba3eb65ba51cd03686570284e2cde78d1c6d2574eb39ce8b0662c9abe10a
```

### Step 6.2: Submit the multisig transaction

Next generate a [multisig authenticator](../guides/sign-a-transaction.md#multisignature-transactions) and submit the transaction:


```python
:!: static/sdks/python/examples/multisig.py section_5
```

```zsh
=== Submitting transaction ===
Transaction hash: 0x2e41b00a8d32e1bbb9369aa3fa5b633c9f413adf3a2feb007bb1446551072358
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
Deedee's address:    0xddb819d72eca401eb4d5d894a3d72540fd1ee47fef2daa0867c9c05fc469925f
Deedee's public key: 0x6b4cd64ae76b751c0ee3d475cb024b6774a45030a5a2a127ff9e01777cadffe9
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
After it executes, the rotated authentication key matches the address of the multisig account that sent octas to Chad:

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

In this section the multisig vanity account will publish a simple package, upgrade it, then invoke a Move governance script.

Here, [semantic versioning](https://semver.org/) is used to distinguish between versions `v1.0.0` and `v1.1.0` of the `UpgradeAndGovern` example package from the `move-examples` folder.

### Step 8.1: Review v1.0.0

Version 1.0.0 of the `UpgradeAndGovern` package contains a simple manifest and a single Move source file:

```toml
[package]
name = 'UpgradeAndGovern'
version = '1.0.0'

[addresses]
upgrade_and_govern = '_'

[dependencies.AptosFramework]
local = '../../../framework/aptos-framework'
```

```rust
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
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/v1_0_0 --named-addresses upgrade_and_govern=0xddc1b52bdb4b24771029e6781aac88ccc42d166a83659ea3a279a81aaa4ff0d5

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0xd30797298c7bed5a302aad0b0ec7f3bc1a108234a64ed06f0a009e3db6f5e9fe

Waiting for client to update...

Package name from on-chain registry: UpgradeAndGovern
On-chain upgrade number: 0
```

### Step 8.3: Upgrade to v1.1.0

### Step 8.4: Invoke a governance script