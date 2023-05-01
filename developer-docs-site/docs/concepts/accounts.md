---
title: "Accounts"
id: "accounts"
---

# Accounts

An account on the Aptos blockchain contains blockchain assets. These assets, for example, coins and NFTs, are by nature
scarce and must be access-controlled. Any such asset is represented in the blockchain account as a **resource**.
A resource is a Move language primitive that emphasizes access control and scarcity in its representation.
However, a resource can also be used to represent other on-chain capabilities, identifying information, and access control.

Each account on the Aptos blockchain is identified by a 32-byte account address. An account can store data, and the account stores
this data in resources. The initial resource is the account data itself (authentication key and sequence number).
Additional resources like currency or NFTs are added after creating the account. And you can employ the [Aptos Name Service](../integration/aptos-name-service-connector.md) at [www.aptosnames.com](https://www.aptosnames.com/) to secure .apt domains for key accounts to make them memorable and unique.

Different from other blockchains where accounts and addresses are implicit, accounts on Aptos are explicit and need to be
created before they can hold resources and modules. The account can be created explicitly or implicitly by transferring Aptos tokens (APT) there.
See the [Creating an account](#creating-an-account) section for more details. In a way, this is similar to other chains where an address needs
to be sent funds for gas before it can send transactions.

Explicit accounts allow first-class features that are not available on other networks such as:
* Rotating authentication key. The account's authentication key can be changed to be controlled via a different private 
key. This is similar to changing passwords in the web2 world.
* Native multisig support. Accounts on Aptos support multi-ed25519 which allows for a multisig authentication scheme when
constructing the authentication key. In the future, more authentication schemes can be introduced easily.
* More integration with the rest of ecosystem to bring in features such as profiles, domain names, etc. that can be seamlessly
integrated with the Aptos account.

There are two types of accounts in Aptos:
  * *Standard account* - This is a typical account corresponding to an address with a corresponding pair of public/private keys.
  * *Resource account* - An autonomous account without a corresponding private key used by developers to store resources or publish modules on chain.

:::tip Account address example
Account addresses are 32-bytes. They are usually shown as 64 hex characters, with each hex character a nibble.
Sometimes the address is prefixed with a 0x. See the [Your First Transaction](../tutorials/first-transaction.md) for an example
of how an address appears, reproduced below:

```text
Alice: 0xeeff357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b
Bob: 0x19aadeca9388e009d136245b9a67423f3eee242b03142849eb4f81a4a409e59c
```

If there are leading 0s, they may be excluded:
```text
Dan: 0000357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b 
Dan: 0x0000357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b
Dan: 0x357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b
```
:::

## Account identifiers
Currently, Aptos supports only a single, unified identifier for an account. Accounts on Aptos are universally
represented as a 32-byte hex string. A hex string shorter than 32-bytes is also valid; in those scenarios,
the hex string can be padded with leading zeroes, e.g., 0x1 => 0x0000000000000...01.

## Creating an account

When a user requests to create an account, for example by using the [Aptos SDK](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosAccount.html), the following cryptographic steps are executed:

- Start by generating a new private key, public key pair.
- From the user, get the preferred signature scheme for the account: If the account should use a single signature or if
it should require multiple signatures for signing a transaction. 
- Combine the public key with the user's signature scheme to generate a 32-byte authentication key. 
- Initialize the account sequence number to 0. Both the authentication key and the sequence number are stored in the
account as an initial account resource. 
- Create the 32-byte account address from the initial authentication key. 

From now on, the user should use the private key for signing the transactions with this account. 

## Account sequence number

The sequence number for an account indicates the number of transactions that have been submitted and committed on chain
from that account. It is incremented every time a transaction sent from that account is executed or aborted and stored in
the blockchain.

Every transaction submitted must contain the current sequence number for the sender account. When the Aptos blockchain
processes the transaction, it looks at the sequence number in the transaction and compares it with the sequence number in
the account (as stored on the blockchain at the current ledger version). The transaction is executed only if the sequence
number in the transaction is the same as the sequence number for the sender account; and it is rejected if they do not match.
In this way, past transactions - which necessarily contain older sequence numbers - cannot be replayed, hence preventing replay attacks. 

These transactions will be held in mempool until they are the next sequence number for that account (or until they expire).
When the transaction is applied, the sequence number of the account will be incremented by 1. The account has a strictly
increasing sequence number.

## Account address

During the account creation process, a 32-byte authentication key comes to exist first. This authentication key is
then returned as it is as the 32-byte account address. 

However, the authentication key may subsequently change, for example when you generate a new public-private key pair,
public keys to rotate the keys. But the account address will not change. Hence, **only initially** the 32-byte authentication
key will be the same as the 32-byte account address. After an account is created, the account address will remain unchanged
even though the private key, public key and authentication keys may change. There is nothing called that changes the address
of the existing account. 

## Signature schemes

An account can send transactions. The Aptos blockchain supports the following signature schemes: 

1. The [Ed25519](https://ed25519.cr.yp.to/) for single signature transactions, and
2. The MultiEd25519, for multi-signature transactions. 

:::note
The Aptos blockchain defaults to single signature transactions.
:::

### Signature scheme identifiers

Generating the authentication key for an account requires that you provide one of the below 1-byte signature scheme
identifiers for this account, i.e., whether the account is a single signature or a multisig account:

- **1-byte single-signature scheme identifier**: `0x00`.
- **1-byte multisig scheme identifier**: `0x01`. Make sure to also provide the value of `K` to generate the K-of-N multisig authentication key.

### Single-signer authentication

To generate an authentication key and the account address for a single signature account:

1. **Generate a key-pair**: Generate a fresh key-pair (`privkey_A`, `pubkey_A`). The Aptos blockchain uses the PureEdDSA scheme over the Ed25519 curve, as defined in RFC 8032.
2. **Derive a 32-byte authentication key**: Derive a 32-byte authentication key from the `pubkey_A`:
     ```
     auth_key = sha3-256(pubkey_A | 0x00)
     ```
     where `|` denotes concatenation. The `0x00` is the 1-byte single-signature scheme identifier. 
3. Use this initial authentication key as the permanent account address.

### Multi-signer authentication

With K-of-N multisig authentication, there are a total of N signers for the account, and at least K of those N signatures
must be used to authenticate a transaction.

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
In order to create accounts, the Aptos testnet requires the account's public key and an amount of `Coin<TestCoin>` to
add to that account, resulting in the creation of a new account with those two resources.
:::

## Rotating the keys
An Account on Aptos has the ability to rotate keys so that potentially compromised keys cannot be used to access the accounts.
Keys can be rotated via the account::rotate_authentication_key function.

Refreshing the keys is generally regarded as good hygiene in the security field. However, this presents a challenge for
system integrators who are used to using a mnemonic to represent both a private key and its associated account.
To simplify this for the system integrators, Aptos provides an on-chain mapping via aptos account lookup-address.
The on-chain data maps an effective account address as defined by the current mnemonic to the actual account address.

For more information, see [`account.move`](https://github.com/aptos-labs/aptos-core/blob/d4a859bb0987f8e35e7471469c3bcd4ae4b49855/aptos-move/framework/aptos-framework/sources/account.move#L251).

## Access control with signers

The sender of a transaction is represented by a signer. When a function in a Move module takes `signer` as an argument,
then the Aptos Move VM translates the identity of the account that signed the transaction into a signer in a Move module entry point.
See the below Move example code with `signer` in the `initialize` and `withdraw` functions. When a `signer` is not specified
in a function, for example, the below `deposit` function, then no access controls exist for this function:

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

## State of an account

The state of each account comprises both the code (Move modules) and the data (Move resources). An account may contain an
arbitrary number of Move modules and Move resources:

- **Move modules**: Move modules contain code, for example, type and procedure declarations; but they do not contain data.
A Move module encodes the rules for updating the Aptos blockchain's global state.
- **Move resources**: Move resources contain data but no code. Every resource value has a type that is declared in a module
published in the Aptos blockchain's distributed database.

## Preventing replay attacks
When the Aptos blockchain processes the transaction, it looks at the sequence number in the transaction and compares it
with the sequence number in the sender’s account (as stored on the blockchain at the current ledger version).

The transaction is executed only if the sequence number in the transaction is the same as the sequence number for the
sender account; and the transaction is rejected if those two numbers do not match. In this way, past transactions - which
necessarily contain older sequence numbers - cannot be replayed, hence preventing replay attacks.

## Resource accounts
A resource account is a developer feature used to manage resources independent of an account managed by a user, specifically
publishing modules and automatically signing for transactions.

For example, a developer may use a resource account to manage an account for module publishing, say managing a contract. The contract itself does not require a signer post initialization. A resource account gives you the means for the module to provide a signer to other modules and sign transactions on behalf of the module.

Typically, a resource account is used for two main purposes:

- Store and isolate resources; a module creates a resource account just to host specific resources.
- Publish module as a standalone (resource) account, a building block in a decentralized design where no private keys can control the resource account. The ownership (SignerCap) can be kept in another module, such as governance.

Find more details on creating and using these at [using resource accounts in your app](../move/move-on-aptos/resource-accounts.md).
