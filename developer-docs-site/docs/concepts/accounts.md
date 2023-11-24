---
title: "Accounts"
id: "accounts"
---

# Accounts

An account on the Aptos blockchain represents access control over a set of assets including on-chain currency and NFTs. In Aptos, these assets are represented by a Move language primitive known as a **resource** that emphasizes both access control and scarcity.

Each account on the Aptos blockchain is identified by a 32-byte account address. You can employ the [Aptos Name Service](../integration/aptos-name-service-connector.md) at [www.aptosnames.com](https://www.aptosnames.com/) to secure .apt domains for key accounts to make them memorable and unique.
 
Different from other blockchains where accounts and addresses are implicit, accounts on Aptos are explicit and need to be created before they can execute transactions. The account can be created explicitly or implicitly by transferring Aptos tokens (APT) there.  See the [Creating an account](#creating-an-account) section for more details. In a way, this is similar to other chains where an address needs to be sent funds for gas before it can send transactions.

Explicit accounts allow first-class features that are not available on other networks such as:
* Rotating authentication key. The account's authentication key can be changed to be controlled via a different private key. This is similar to changing passwords in the web2 world.
* Native multisig support. Accounts on Aptos support k-of-n multisig using both Ed25519 and Secp256k1 ECDSA signature schemes when constructing the authentication key.

There are three types of accounts on Aptos:
  * *Standard account* - This is a typical account corresponding to an address with a corresponding pair of public/private keys.
  * [*Resource account*](../move/move-on-aptos/resource-accounts.md) - An autonomous account without a corresponding private key used by developers to store resources or publish modules on-chain.
  * [*Object*](../standards/aptos-object.md) - A complex set of resources stored within a single address representing a single entity.

:::tip Account address example
Account addresses are 32-bytes. They are usually shown as 64 hex characters, with each hex character a nibble.
Sometimes the address is prefixed with a 0x. See the [Your First Transaction](../tutorials/first-transaction.md) for an example
of how an address appears, reproduced below:

```text
Alice: 0xeeff357ea5c1a4e7bc11b2b17ff2dc2dcca69750bfef1e1ebcaccf8c8018175b
Bob: 0x19aadeca9388e009d136245b9a67423f3eee242b03142849eb4f81a4a409e59c
```
:::

## Account address

Currently, Aptos supports only a single, unified identifier for an account. Accounts on Aptos are universally represented as a 32-byte hex string. A hex string shorter than 32-bytes is also valid; in those scenarios, the hex string can be padded with leading zeroes, e.g., `0x1x` => `0x0000000000000...01`. While Aptos standards indicate leading zeroes may be removed from an Address, most applications attempt to eschew that legacy behavior and only support the removal of 0s for special addresses ranging from `0x0` to `0xa`.

## Creating an account

When a user requests to create an account, for example by using the [Aptos SDK](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosAccount.html), the following steps are executed:

- Select the authentication scheme for managing the user's account, e.g., Ed25519 or Secp256k1 ECDSA.
- Generate a new private key, public key pair.
- Combine the public key with the public key's authentication scheme to generate a 32-byte authentication key and the account address.

The user should use the private key for signing the transactions associated with this account.

## Account sequence number

The sequence number for an account indicates the number of transactions that have been submitted and committed on-chain from that account. Committed transactions either execute with the resulting state changes committed to the blockchain or abort wherein state changes are discarded and only the transaction is stored.

Every transaction submitted must contain a unique sequence number for the given sender's account. When the Aptos blockchain processes the transaction, it looks at the sequence number in the transaction and compares it with the sequence number in the on-chain account. The transaction is processed only if the sequence number is equal to or larger than the current sequence number. Transactions are only forwarded to other mempools or executed if there is a contiguous series of transactions from the current sequence number. Execution rejects out of order sequence numbers preventing replay attacks of older transactions and guarantees ordering of future transactions.

## Authentication key

The initial account address is set to the authentication key derived during account creation. However, the authentication key may subsequently change, for example when you generate a new public-private key pair, public keys to rotate the keys. An account address never changes.

The Aptos blockchain supports the following authentication schemes:

1. [Ed25519](https://ed25519.cr.yp.to/)
2. [Secp256k1 ECDSA](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-49.md)
3. [K-of-N multi-signatures](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-55.md)
4. A dedicated, now legacy, MultiEd25519 scheme

:::note
The Aptos blockchain defaults to Ed25519 signature transactions.
:::

### Ed25519 authentication

To generate an authentication key and the account address for an Ed25519 signature:

1. **Generate a key-pair**: Generate a fresh key-pair (`privkey_A`, `pubkey_A`). The Aptos blockchain uses the PureEdDSA scheme over the Ed25519 curve, as defined in RFC 8032.
2. **Derive a 32-byte authentication key**: Derive a 32-byte authentication key from the `pubkey_A`:
     ```
     auth_key = sha3-256(pubkey_A | 0x00)
     ```
     where `|` denotes concatenation. The `0x00` is the 1-byte single-signature scheme identifier. 
3. Use this initial authentication key as the permanent account address.

### MultiEd25519 authentication

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

### Generalized authentication

Generalized authentication supports both Ed25519 and Secp256k1 ECDSA. Like the previous authentication schemes, these schemes contain a scheme value, `0x02` and `0x03` for single and multikey respectively, but also each key contains a prefix value to indicate its key type:

- **1-byte Ed25519 generalized scheme**: `0x00`,
- **1-byte Secp256k1 ECDSA generalized scheme**: `0x01`.

For a single key Secp256k1 ECDSA account, using public key `pubkey`, the authentication key would be derived as follows:
```
auth_key = sha3-256(0x01 | pubkey | 0x02)
```
Where
* the first entry, `0x01`, represents the use of a Secp256k1 ECDSA key;
* the last entry, `0x02`, represents the authentication scheme.

For a multi-key account containing, a single Secp256k1 ECDSA public key, `pubkey_0`, and a single Ed25519 public key, `pubkey_1`, where one signature suffices, the authentication key would be derived as follows:
```
auth_key = sha3-256(0x02 | 0x01 | pubkey_0 | 0x02 | pubkey_2 | 0x01 | 0x03)
```
Where
* the first entry, `0x02`, represents the total number of keys as a single byte;
* the second to last entry, `0x01`, represents the required number of singatures as a single byte;
* the last entry, `0x03`, represents the authentication scheme.

## Rotating the keys
An Account on Aptos has the ability to rotate keys so that potentially compromised keys cannot be used to access the accounts.  Keys can be rotated via the `account::rotate_authentication_key` function.

Refreshing the keys is generally regarded as good hygiene in the security field. However, this presents a challenge for system integrators who are used to using a mnemonic to represent both a private key and its associated account. To simplify this for the system integrators, Aptos provides an on-chain mapping via aptos account lookup-address. The on-chain data maps an effective account address as defined by the current mnemonic to the actual account address.

For more information, see [`account.move`](https://github.com/aptos-labs/aptos-core/blob/a676c1494e246c31c5e96d3363d99e2422e30f49/aptos-move/framework/aptos-framework/sources/account.move#L274).

## State of an account

The state of each account comprises both the code (Move modules) and the data (Move resources). An account may contain an arbitrary number of Move modules and Move resources:

- **Move modules**: Move modules contain code, for example, type and procedure declarations; but they do not contain data. A Move module encodes the rules for updating the Aptos blockchain's global state.
- **Move resources**: Move resources contain data but no code. Every resource value has a type that is declared in a module published on the Aptos blockchain.

## Access control with signers

The sender of a transaction is represented by a signer. When a function in a Move module takes `signer` as an argument, the Aptos Move VM translates the identity of the account that signed the transaction into a signer in a Move module entry point.  See the below Move example code with `signer` in the `initialize` and `withdraw` functions. When a `signer` is not specified in a function, for example, the below `deposit` function, then no signer-based access controls will be provided for this function:

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
