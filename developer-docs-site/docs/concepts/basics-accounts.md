---
title: "Accounts"
id: "basics-accounts"
---

# Accounts

An account on the Aptos Blockchain contains blockchain assets, which are, by nature, scarce and must be access-controlled. For example, coins and NFTs. Any such asset is represented in the blockchain account as a *resource*. A resource is a Move language primitive that emphasizes on access control and scarcity in its representation of an asset.  

Each account on the Aptos Blockchain is identified by a 32-byte account address. Every account on Aptos Blockchain can store data, and it stores this data in resources. The initial resource is the account data itself (authentication key and sequence number). Additional resources like currency or NFTs are added after creating the account. An account can send transactions. 

## Creating an account

When a new account is requested by the user, the Aptos standard Move library, or the Aptos SDK, will perform a series of cryptographic steps to create the account. Below is a conceptual view of these steps:

- Start by generating a new private key, public key pair.
- From the user get the preferred signature scheme for the account: If the account should use a single signature or if it should require multiple signatures, for signing a transaction. 
- Combine the public key with the user's signature scheme to generate a 32-byte authentication key. 
- Initialize the sequence number to 0. The sequence number represents the next transaction sequence number, to prevent replay attacks of transactions. Both the authentication key and the sequence number are stored in the account as an initial account resource. 
- Create the 32-byte account address from the initial authentication key. 

From now on, the user should use the private key for signing the transactions with this account. 

The Aptos Blockchain supports two signature schemes: 

1. The [Ed25519](https://ed25519.cr.yp.to/) for single signature transactions, and 
2. The MultiEd25519, for multi-signature transactions. 

:::note
Aptos Blockchain defaults to single signature transactions.
:::

  
## Rotating the keys

For an existing account you can rotate the private key, public key pair, i.e., use a new pair of private, public keys at regular intervals. To rotate the keys you will need to pass the current authentication key of your account. This will generate and store a new authentication key in the account. **However, the account address will remain unchanged.**

## Access control with signer

When a `signer` is specified in a function in a transaction, then the `signer` is the only entity capable of adding or removing resources into an account. The sender of a transaction is represented by a signer. See a Move example code below with `&signer` in the `initialize` and `withdraw` functions:

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


## Single signer authentication

To generate an authentication key and account address:

1. **Generate a key-pair**: Generate a fresh key-pair (`pubkey_A`, `privkey_A`). The Aptos Blockchain uses the PureEdDSA scheme over the Ed25519 curve, as defined in RFC 8032.
2. **Derive a 32-byte authentication key**: Derive a 32-byte authentication key `auth_key = sha3-256(pubkey_A | 0x00)`, where | denotes concatenation. The `0x00` is a 1-byte signature scheme identifier where 0x00 means single-signature. The first 16 bytes of `auth_key` is the "authentication key prefix".

## Multisigner authentication

The authentication key for an account may require either a single signature or multiple signatures ("multisig"). With K-of-N multisig authentication, there are a total of N signers for the account, and at least K of those N signatures must be used to authenticate a transaction.

Creating a K-of-N multisig authentication key is similar to creating a single signature one:
1. **Generate key-pairs**: Generate N ed25519 public keys `p_1`, ..., `p_n`.
2. **Derive a 32-byte authentication key**: Compute `auth_key = sha3-256(p_1 | … | p_n | K | 0x01)`. Derive an address and an auth key prefix as described above. `K` represents the K-of-N required for authenticating the transaction. The `0x01` is a 1-byte signature scheme identifier where `0x01` means multisignature.

:::tip Accounts on Aptos Testnet
In order to create accounts, the Aptos testnet requires the account's public key and an amount of `Coin<TestCoin>` to add to that account, resulting in the creation of a new account with those two resources.
:::

## State of the account

The state of each account comprises both the code (Move modules) and the data (Move resources). An account may contain an arbitrary number of Move modules and Move resources:

- **Move modules**: Move modules contain code, for example, type and procedure declarations, but they do not contain data. A Move module encodes the rules for updating the Aptos Blockchain's global state.
- **Move resources**: Move resources contain data but no code. Every resource value has a type that is declared in a module published in the Aptos Blockchain's distributed database.

