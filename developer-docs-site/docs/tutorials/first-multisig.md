---
title: "Your First Multisig"
slug: "your-first-multisig"
---

# Your First Multisig

This tutorial introduces assorted [K-of-N multisigner authentication](../concepts/accounts#multisigner-authentication) operations, and supplements content from the following tutorials:

* [Your First Transaction](your-first-transaction)
* [Your First Coin](your-first-coin)
* [Your First Move Module](first-move-module)

:::tip
Try out the above tutorials (which include dependency installations) before moving on to multisig operations.
:::

## Step 1: Pick an SDK

This tutorial, a community contribution, was created for the [Python SDK](../sdks/python-sdk).

Other developers are invited to add support for the [TypeScript SDK](../sdks/ts-sdk/index) and the [Rust SDK](../sdks/rust-sdk)!

## Step 2: Start the example

Navigate to the Python SDK directory:

```zsh
cd <aptos-core-parent-directory>/aptos-core/ecosystem/python/sdk/examples
```

Run the [`multisig.py`](../../../ecosystem/python/sdk/examples/multisig.py) example:

```zsh
python multisig.py
```

## Step 3: Generate signers

First the example will generate single signer accounts for Alice, Bob, and Chad:

```python
:!: static/sdks/python/examples/multisig.py section_1
```

Fresh accounts are generated for each example run, but the printout should look something like the following:

```zsh
=== Account addresses ===
Alice: 0x4730cd9ecf497ead90e2fb90e9fdf734df1735815e60440b0806e30cfd3877aa
Bob:   0x682e4177c9acaecc14b5e3c1abad0f5a7caf653ab05b4093f8f80954037df446
Chad:  0x5edbae9ecf1f06498a5316f16a4545dd83ba3fcbef532f1b4ceabf3430648e76

=== Authentication keys ===
Alice: 0x4730cd9ecf497ead90e2fb90e9fdf734df1735815e60440b0806e30cfd3877aa
Bob:   0x682e4177c9acaecc14b5e3c1abad0f5a7caf653ab05b4093f8f80954037df446
Chad:  0x5edbae9ecf1f06498a5316f16a4545dd83ba3fcbef532f1b4ceabf3430648e76

=== Public keys ===
Alice: 0x44952324f5fa35bf15dc495f914864165b820a4fef39a4d2b0238168981519fe
Bob:   0x089d6e00e946af8d372ef4ef7f26e21a08cb856747014d15525180bf37f31ef5
Chad:  0x784508b54b812f89a6cb6c47010a5c389ff25316feed7f923b9e1489f7772acf
```

For each user note that the [account address](../concepts/accounts#account-address) and [authentication key](../concepts/accounts#single-signer-authentication) are identical, but the [public key](../concepts/accounts#creating-an-account) is different.

## Step 4: Generate a multisig account

Next generate a [K-of-N multisigner](../concepts/accounts#multisigner-authentication) public key and account address for a multisig account requiring 2 of the 3 signatures:

```python
:!: static/sdks/python/examples/multisig.py section_2
```

The multisig account address depends on the public keys of the single signers (hence it will be different for each example), but your printout should look something like the following:

```zsh
=== 2-of-3 Multisig account ===
Account public key: 0x1ee12500a32d35cae7788966edd224768f201f63c329c2881ceed089c113bbc4
Account address:    0x1ee12500a32d35cae7788966edd224768f201f63c329c2881ceed089c113bbc4
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
Since it is a 2-of-3 multisig account, signatures are only required from 2 individual signers.

### Step 6.1 Gather individual signatures

First generate a raw transaction, signed by Alice and by Bob, but not by Chad.

```python
:!: static/sdks/python/examples/multisig.py section_4
```

Again, signatures vary for each example run:

```zsh
=== Individual signatures ===
Alice: 0x223fd617e4fb82fe211c4067356dc1d9e84c0bcc65cfdbb8da75f58c27273c55e3ed28704cb2bf8cd053ec24ac62ebc0467d3d630622f745618ad9e626c43004
Bob:   0xc8120ebeebff07b647666431ceb66fd5043635e8ccc836b99c525185b3bf5a8293de63068a06c78ce5082e5b1d4894ed0b80f34800cc3272c990d775d641740e
```

### Step 6.2 Generate a multisig authenticator

Next generate a [multisig authenticator](../guides/creating-a-signed-transaction#multisignature-transactions):


```python
:!: static/sdks/python/examples/multisig.py section_5
```
