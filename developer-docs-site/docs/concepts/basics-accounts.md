---
title: "Accounts"
id: "basics-accounts"
---

# Accounts

An account on the Aptos blockchain contains blockchain assets. These assets, for example, coins and NFTs, are by nature scarce and must be access-controlled. Any such asset is represented in the blockchain account as a **resource**. A resource is a Move language primitive that emphasizes access control and scarcity in its representation. However, a resource can also be used to represent other on-chain capabilities, identifying information, and access control. 

Each account on the Aptos blockchain is identified by a 32-byte account address. An account can store data and it stores this data in resources. The initial resource is the account data itself (authentication key and sequence number). Additional resources like currency or NFTs are added after creating the account. 

:::tip Account address example
Account addresses are 32-bytes. They are usually shown as 64 hex characters, with each hex character a nibble. See the [Your First Transaction](/tutorials/first-transaction.md#output) for an example of how an address looks like, reproduced below:
```text
Alice: eeff357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b
Bob: 19aadeca9388e009d136245b9a67423f3eee242b03142849eb4f81a4a409e59c
```
:::

## Creating an account

When a user requests to create an account, for example by using the [Aptos SDK](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosAccount.html), the following cryptographic steps are executed:

- Start by generating a new private key, public key pair.
- From the user get the preferred signature scheme for the account: If the account should use a single signature or if it should require multiple signatures for signing a transaction. 
- Combine the public key with the user's signature scheme to generate a 32-byte authentication key. 
- Initialize the account sequence number to 0. Both the authentication key and the sequence number are stored in the account as an initial account resource. 
- Create the 32-byte account address from the initial authentication key. 

From now on, the user should use the private key for signing the transactions with this account. 

## Account sequence number

The sequence number for an account indicates the number of transactions that have been submitted and committed on chain from that account. It is incremented every time a transaction sent from that account is executed or aborted and stored in the blockchain.

Every transaction submitted must contain the current sequence number for the sender account. When the Aptos blockchain processes the transaction, it looks at the sequence number in the transaction and compares it with the sequence number in the account (as stored on the blockchain at the current ledger version). The transaction is executed only if the sequence number in the transaction is the same as the sequence number for the sender account, and rejects if they do not match. In this way past transactions, which necessarily contain older sequence numbers, cannot be replayed, hence preventing replay attacks. 

These transactions will be held in mempool until they are the next sequence number for that account (or until they expire). When the transaction is applied, the sequence number of the account will be incremented by 1. The account has a strictly increasing sequence number.

## Account address

During the new account creation process, a 32-byte authentication key comes to exist first. This authentication key is then returned as it is as the 32-byte account address. 

However, the authentication key may subsequently change, for example, when you generate a new pair of private key, public keys, to rotate the keys. But the account address will not change. Hence, **only initially** the 32-byte authentication key will be the same as the 32-byte account address. After an account is created, the account address will remain unchanged even though the private key, public key and authentication keys may change. There is nothing called changing the address of the existing account. 

## Signature schemes

An account can send transactions. The Aptos blockchain supports the following signature schemes: 

1. The [Ed25519](https://ed25519.cr.yp.to/) for single signature transactions, and
2. The MultiEd25519, for multi-signature transactions. 

:::note
The Aptos blockchain defaults to single signature transactions.
:::

## Signature scheme identifiers

Generating the authentication key for an account requires that you provide one of the below 1-byte signature scheme identifiers for this account, i.e., whether the account is a single signature or a multisig account:

- **1-byte single-signature scheme identifier**: `0x00`.
- **1-byte multisig scheme identifier**: `0x01`. Make sure to also provide the value of `K` to generate the K-of-N multisig authentication key.

## Single signer authentication

To generate an authentication key and the account address for a single signature account:

1. **Generate a key-pair**: Generate a fresh key-pair (`privkey_A`, `pubkey_A`). The Aptos blockchain uses the PureEdDSA scheme over the Ed25519 curve, as defined in RFC 8032.
2. **Derive a 32-byte authentication key**: Derive a 32-byte authentication key from the `pubkey_A`:
     ```
     auth_key = sha3-256(pubkey_A | 0x00)
     ```
     where `|` denotes concatenation. The `0x00` is the 1-byte single-signature scheme identifier. 
3. Use this initial authentication key as the permanent account address.

## Multisigner authentication

With K-of-N multisig authentication, there are a total of N signers for the account, and at least K of those N signatures must be used to authenticate a transaction.

To generate a K-of-N multisig account's authentication key and the account address:

1. **Generate key-pairs**: Generate `N` ed25519 public keys `p_1`, ..., `p_n`.
2. Decide on the value of `K`, the threshold number of signatures needed for authenticating the transaction.
3. **Derive a 32-byte authentication key**: Compute the authentication key as described below:
    ```
    auth_key = sha3-256(p_1 | . . . | p_n | K | 0x01)
    ```
    The `0x01` is the 1-byte multisig scheme identifier.
4. Use this initial authentication key as the permanent account address.

:::tip Accounts on Aptos Testnet
In order to create accounts, the Aptos testnet requires the account's public key and an amount of `Coin<TestCoin>` to add to that account, resulting in the creation of a new account with those two resources.
:::

## Access control with signer

The sender of a transaction is represented by a signer. When a function in a Move module takes `signer` as an argument, then the Aptos Move VM translates the identity of the account that signed the transaction into a signer in a Move module entry point. See the below Move example code with `signer` in the `initialize` and `withdraw` functions. When a `signer` is not specified in a function, for example, the below `deposit` function, then no access controls exist for this function:

```rust
module Test::Coin {
  struct Coin has key { amount: u64 }

  public fun initialize(account: &signer) {
    move_to(account, Coin { amount: 1000 });
  }

  public fun withdraw(account: &signer, amount: u64): Coin acquires Coin {
    let balance = &mut borrow_global_mut<Coin>(Signer::address_of(account)).amount;
    *balance = *balance - amount;
    Coin { amount }
  }

  public fun deposit(account: address, coin: Coin) acquires Coin {
      let balance = &mut borrow_global_mut<Coin>(account).amount;
      *balance = *balance + coin.amount;
      Coin { amount: _ } = coin;
  }
}
```

## State of the account

The state of each account comprises both the code (Move modules) and the data (Move resources). An account may contain an arbitrary number of Move modules and Move resources:

- **Move modules**: Move modules contain code, for example, type and procedure declarations, but they do not contain data. A Move module encodes the rules for updating the Aptos blockchain's global state.
- **Move resources**: Move resources contain data but no code. Every resource value has a type that is declared in a module published in the Aptos blockchain's distributed database.