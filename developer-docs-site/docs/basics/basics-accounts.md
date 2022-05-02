---
title: "Accounts"
id: "basics-accounts"
---
An account represents a resource on the Aptos Blockchain that can send transactions. Each account is identified by a particular 32-byte account address and is a container for Move modules and Move resources.

# Introduction

The state of each account comprises both code and data:

- **Code**: Move modules contain code (type and procedure declarations), but they do not contain data. The module's procedures encode the rules for updating the blockchain's global state.
- **Data**: Move resources contain data but no code. Every resource value has a type that is declared in a module published in the blockchain's distributed database.

An account may contain an arbitrary number of Move resources and Move modules.

## Initial Account Setup

An Aptos account is referenced by an account address, which is a 32-byte value. The account address is derived from the initial public verification key(s) for that account. Specifically, the account address is the 32-byte of the SHA-3 256 cryptographic hash of the initial public verification key(s) concatenated with a signature scheme identifier byte. The Aptos Blockchain supports two signature schemes: [Ed25519](/reference/glossary#ed25519) and MultiEd25519 (for multi-signature transactions). The account's private key is necessary for signing transactions.

Each account also stores a `sequence_number`, which represents the next transaction sequence number to prevent replay attacks of transactions. This is initialized to `0` at account creation time.

## Authentication Keys

Each account stores an authentication key. This authentication key enables account owners to rotate their private key(s) associated with the account without changing the address that hosts their account. During rotation, the authentication key is updated based upon the newly-generated private, public key-pair(s).

### Single signer authentication

To generate an authentication key and account address:

1. **Generate a key-pair**: Generate a fresh key-pair (`pubkey_A`, `privkey_A`). The Aptos Blockchain uses the PureEdDSA scheme over the Ed25519 curve, as defined in RFC 8032.
2. **Derive a 32-byte authentication key**: Derive a 32-byte authentication key `auth_key = sha3-256(pubkey_A | 0x00)`, where | denotes concatenation. The `0x00` is a 1-byte signature scheme identifier where 0x00 means single-signature. The first 16 bytes of `auth_key` is the "authentication key prefix".

### Multisigner authentication

The authentication key for an account may require either a single signature or multiple signatures ("multisig"). With K-of-N multisig authentication, there are a total of N signers for the account, and at least K of those N signatures must be used to authenticate a transaction.

Creating a K-of-N multisig authentication key is similar to creating a single signature one:
1. **Generate key-pairs**: Generate N ed25519 public keys `p_1`, ..., `p_n`.
2. **Derive a 32-byte authentication key**: Compute `auth_key = sha3-256(p_1 | … | p_n | K | 0x01)`. Derive an address and an auth key prefix as described above. `K` represents the K-of-N required for authenticating the transaction. The `0x01` is a 1-byte signature scheme identifier where `0x01` means multisignature.

## Account resources

Every account on Aptos can store data, which it does so in resources. The initial resource is the account data itself (authentication key and sequence number). Additional resources like currency or NFTs can be added after account creation. In order to create accounts, the Aptos testnet requires the account's public key and an amount of TestCoin to add to that account, resulting in the creation of a new account with those two resources.
