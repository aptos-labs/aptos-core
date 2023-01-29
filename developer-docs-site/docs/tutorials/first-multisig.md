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
Alice: 0x0d13819690aaf14c00538bd50879d96d4763690d112390b7d7994766201e75fe
Bob:   0xe8221d30a0f50586d193c5293dd3d89f768a6f13e089aec3c55c0d4a9c748f4d
Chad:  0xdb3ed5b3e53c9793eaba242cbc8e4e776c0eb8f7e10341784de7961fa5599dac

=== Authentication keys ===
Alice: 0x0d13819690aaf14c00538bd50879d96d4763690d112390b7d7994766201e75fe
Bob:   0xe8221d30a0f50586d193c5293dd3d89f768a6f13e089aec3c55c0d4a9c748f4d
Chad:  0xdb3ed5b3e53c9793eaba242cbc8e4e776c0eb8f7e10341784de7961fa5599dac

=== Public keys ===
Alice: 0xdd14d7b52d8e120ccf8cbad2e51ed49b87554adf96f5df216d77a9103ee01cf4
Bob:   0x798d0eeaddbe0d078088034b35d36e77c522e76a072aa3ff127b2e3e13b50446
Chad:  0xd386988ee35e709a721c848976d143cdc83bf67295f4089558135364aeab22b7
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
Account address:    0x61c6fc315d6a96bd84b93c3ee89466a7d9323c425499687b1c4e275942443bac
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
Alice: 0x7960fb9dac861fb46b43c31275df88f24e309d0c94d0a07c7898644044d200213286ed64aef37172e9ae58cbc1dc224e925c657c8f796899d1edc0f41f3fe30f
Bob:   0x6197b02eaa8e5d378e8edc7993065e88d8d3ce52a1193bc1bb5ad3ed91e37157a02fd37cb2ef3475d3b05083e3b3b9731f9da43afee110047d95bd9296896d08
```

### Step 6.2: Submit the multisig transaction

Next generate a [multisig authenticator](../guides/sign-a-transaction.md#multisignature-transactions) and submit the transaction:


```python
:!: static/sdks/python/examples/multisig.py section_5
```

```zsh
=== Submitting transaction ===
Transaction hash: 0x3a65087a4e5e6ab3b3eb56d222509090d3cdafbdb907deb2c233069d308d8a81
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
Deedee's address:    0xdd063617b654750fa97ae2bc82f85efe79770d08029aa9aa8f8c4f4f1271e05e
Deedee's public key: 0xca8b97f8efc2b0be7a241479802cfd39b70969da1fcfacb08aabd79fa0fb6d73
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
cap_rotate_key:   0x61ad912862c3647c9bb1f10d88494f7b157eeccc685e1cb45c338c3b9ea2d1c3a9e0473378f1e1d340b66eea352c961901ef6c5eaf25b7ba439cfa1e32014b03
cap_update_table: 0xcb49863bbc000a1c62f780115843b7b7c06661525d9195c3b3518aebd9541d73e48e298ef877c114314e91cc1b9001c01ab0bcbf889119350274a3cd447cf40bdade8af39ed873c913ddcd3de4c8a7aa453278664c374190936e7d974a021408c32b53f07c0d21b74cad2b429b3b48285c61be40d636feb6e0d10f4b695cb00360000000
```

### Step 7.3 Rotate the authentication key

```python
:!: static/sdks/python/examples/multisig.py section_9
```

:::warning
Move abort in 0x1::account: EINVALID_PROOF_OF_KNOWLEDGE(0x10008): Specified proof of knowledge required to prove ownership of a public key is invalid
:::