---
title: "Cryptography"
---

# Cryptography in Move

Cryptography plays an integral role in ensuring the security, integrity, confidentiality, and immutability of data in blockchain systems. The Aptos adapter for Move provides developers with an array of cryptographic primitives to cater to this need. This document delves into the cryptographic functionalities offered by Move on Aptos and elucidates the principles that drive their design.

## Cryptographic primitives

Move, through the Aptos adapter, encompasses several fundamental cryptographic tools:

1. [Cryptographic Hash Functions](#cryptographic-hash-functions) – Algorithms that produce a fixed-size output (hash) from variable-sized input data. Supported functions include SHA2-256, SHA3-256, Keccak256, and Blake2b-256.
2. [Digital Signature Verification](#digital-signature-verification) – Algorithms for signing a message so as to ensure its integrity, authenticate its sender, ensure non-repudiation, or any combination thereof. Supported signature schemes include Ed25519, ECDSA, and BLS.
3. [Elliptic Curve Arithmetic](#elliptic-curve-arithmetic) – Elliptic curves are one of the building blocks of advanced cryptographic primitives, such as digital signatures, public-key encryption or verifiable secret sharing. Supported curves include Ristretto255 and BLS12-381.
4. [Zero-Knowledge Proofs (ZKP)](#building-powerful-cryptographic-applications) – These cryptographic techniques enable a party to prove that a relation $R(x; w)$ is satisfied on a public statement $x$ without leaking the secret witness $w$ that makes it hold. Currently, we support Groth16 ZKP verification and Bulletproofs ZK range proof verification.

Three fundamental principles guide the design and integration of the Aptos cryptographic extensions into Move:

1. **Economic Gas Usage** – Striving to minimize gas costs for Move developers by implementing key primitives as [Move native functions](../book/functions.md#native-functions). For example, see the module for [BLS signatures over BLS12-381 elliptic curves](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/bls12381.move).
2. **Type-Safe APIs** – Ensuring that APIs are resistant to common mistakes, type-safety enhances code reliability and promotes an efficient development process. For an example, see the [Ed25519 signature module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/ed25519.move).
3. **Empowerment of Developers** – In instances where native functions are unavailable, we empower developers to build their own cryptographic primitives on top of abstract cryptographic building blocks such as _finite fields_ and _Abelian groups_. Refer to the [`aptos_std::crypto_algebra`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/crypto_algebra.move) module for more insights.

Continue reading to delve a bit deeper and uncover some of the intricacies behind these extensions, as well as the range of applications they empower. For the most comprehensive understanding of this subject, refer to the [cryptography Move modules code](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/framework/aptos-stdlib/sources/cryptography).

## Cryptographic hash functions

Developers can now use more cryptographic hash functions in Move via the [`aptos_std::aptos_hash`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/hash.move) module:

| Hash function | Hash size (bits) | Cost for hashing 1KiB (in internal gas units) | Collision-resistance security (bits) |
|---------------|------------------|-----------------------------------------------|--------------------------------------|
| Keccak256     | 256              | 1,001,600                                     | 128                                  |
| SHA2-256      | 256              | 1,084,000                                     | 128                                  |
| SHA2-512      | 512              | 1,293,600                                     | 256                                  |
| SHA3-256      | 256              | 1,001,600                                     | 128                                  |
| SHA3-512      | 512              | 1,114,000                                     | 256                                  |
| RIPEMD160     | 160              | 1,084,000                                     | 80 (**weak**)                        |
| Blake2b-256   | 256              | 342,200                                       | 128                                  |

All hash functions have the same security properties (e.g., one-wayness, collision resistance, etc.), but their security levels are different.

:::caution
RIPEMD160 should be avoided as a collision-resistant function due to its 80-bit security level. It is mainly supported for backward-compatibility reasons: e.g., Bitcoin address derivation relies on RIPEMD160.
:::

Some of these functions can be used for interoperability with other chains (e.g., verifying Ethereum Merkle proofs via [`aptos_std::aptos_hash::keccak256`](https://github.com/aptos-labs/aptos-core/blob/137acee4c6dddb1c86398dce25b041d78a3028d3/aptos-move/framework/aptos-stdlib/sources/hash.move#L35)).
Others, have lower gas costs, such as [`aptos_std::aptos_hash::blake2b_256`](https://github.com/aptos-labs/aptos-core/blob/137acee4c6dddb1c86398dce25b041d78a3028d3/aptos-move/framework/aptos-stdlib/sources/hash.move#L69).
In general, a wider variety of hash functions give developers additional freedom in terms of both security and interoperability with other off-chain cryptographic systems.

## Digital signature verification

Developers can now use a *type-safe* API for verifying many kinds of digital signatures in Move:

| Signature scheme                                                                                                                                          | Curve         | Sig. size (bytes) | PK size (bytes) | Malleability | Assumptions | Pros          | Cons                |
|-----------------------------------------------------------------------------------------------------------------------------------------------------------|---------------|-------------------|-----------------|--------------|-------------|---------------|---------------------|
| [ECDSA](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/secp256k1.move)                         | secp256k1     | 64                | 64              | Yes          | GGM         | Wide adoption | Security proof      |
| [Ed25519](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/ed25519.move)                         | Edwards 25519 | 64                | 32              | No           | DLA, ROM    | Fast          | Subtleties          |
| [MultiEd25519](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/multi_ed25519.move)              | Edwards 25519 | $4 + t \cdot 64$  | $n \cdot 32$    | No           | DLA, ROM    | Easy-to-adopt | Large sig. size     |
| [MinPK BLS](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/bls12381.move)                      | BLS12-381     | 96                | 48              | No           | CDH, ROM    | Versatile     | Slower verification |
| [MinSig BLS](https://github.com/aptos-labs/aptos-core/blob/7d4fb98c6604c67e526a96f55668e7add7aaebf6/aptos-move/move-examples/drand/sources/drand.move#L57) | BLS12-381     | 48                | 96              | No           | CDH, ROM    | Versatile     | Slower verification |


:::note
 - CDH stands for the _"Computational Diffie-Hellman Assumption"_
 - DLA stands for the _"Discrete Log Assumption"_
 - GGM stands for the _"Generic Group Model"_
 - ROM stands for the _"Random Oracle Model"_
:::

The digital signature modules above can be used to build smart contract-based wallets, secure claiming mechanisms for airdrops, or any digital-signature-based access-control mechanism for dapps.

The right choice of a signature scheme in your dapp could depend on many factors:
1. **Backwards-compatibility** 
   - If your dapp's user base predominantly uses a particular signing mechanism, it would be prudent to support that mechanism for ease of transition and adoption.
     - Example: If users mainly sign using Ed25519, it becomes a logical choice.
2. **Ease-of-implementation**
   - While theoretically sound, complex protocols may be challenging to implement in practice.
     - Example: Even though $t$-out-of-$n$ threshold protocols for Ed25519 exist, their intricacy on the signer's side might push developers toward MultiEd25519 due to its more straightforward signing implementation.
3. **Efficiency** 
   - Depending on the dapp's requirements, you might prioritize one aspect of efficiency over another.
     - Signature size vs. public key size: Some applications might prioritize a smaller signature footprint, while others might emphasize a compact PK.
     - Signing time vs. verification time: For certain dapps, the signing speed might be more crucial, while for others, rapid signature verification could be the priority.
4. **Security analysis**
   - It is essential to consider the underlying assumptions and potential vulnerabilities of a signature scheme.
     - Example: ECDSA's security is proven under strong assumptions such as the Generic Group Model (GGM).
     - Malleability concerns: Some signature schemes are susceptible to malleability, where a valid signature, $\sigma$, can be mauled into a different yet still valid signature, $\sigma'$, for the same message $m$.
5. **Versatility**
   - The adaptability and flexibility of signature schemes are important to consider so you may properly accommodate the cryptographic needs of your dapp.
     - Example: $t$-out-of-$n$ threshold BLS signatures are very simple to implement.

:::caution
Despite its careful, principled design[^ed25519], Ed25519 has known implementation subtleties. For example, different implementations could easily disagree on the validity of signatures, especially when batch verification is employed[^devalence]$^,$[^eddsa].
:::

:::tip 
Our [`aptos_std::bls12381`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/bls12381.move) module for [MinPK BLS](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-bls-signature-05#name-variants) supports verification of individual signatures, **multi**-signatures, **aggregate** signatures and **threshold** signatures.
:::

## Elliptic curve arithmetic

While the [hash function](#cryptographic-hash-functions) and [digital signature](#digital-signature-verification) modules should provide enough functionality for most applications, some applications will require more powerful cryptography.
Normally, developers of such applications would have to wait until their desired cryptographic functionality is implemented efficiently as a [Move native function](https://github.com/aptos-labs/aptos-core/blob/main/developer-docs-site/docs/move/book/functions.md#native-functions) in the [Aptos Move framework](/reference/move).
Instead, we expose basic building blocks that developers can use to implement their own cryptographic primitives directly in the Move language and do so **efficiently**.  

Specifically, we currently expose low-level arithmetic operations on two popular elliptic curve groups and their associated finite fields:

 1. Ristretto255, via [`aptos_std::ristretto255`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/ristretto255.move)
 2. BLS12-381, via [`aptos_std::crypto_algebra`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/crypto_algebra.move) 
    and [`aptos_std::bls12381_algebra`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/bls12381_algebra.move)

These modules support low-level operations such as:

 * scalar multiplication of elliptic curve points
 * multi-scalar multiplications (MSMs)
 * pairings
 * scalar addition, multiplication, inversion
 * hashing to a scalar or to a point
 * and many more

Examples of powerful applications that can be built on top include:  

 1. **Validity rollups** – See the [`groth16` zkSNARK verifier example](#groth16-zksnark-verifier).
 2. **Randomness-based games** – See the [`drand` verifier example](#verifying-randomness-from-the-drand-beacon).
 3. **Privacy-preserving applications** – See the [`veiled_coin` example](#veiled-coins).

### Ristretto255 arithmetic

The [`aptos_std::ristretto255`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/ristretto255.move) module provides support for elliptic curve arithmetic on the popular [Ristretto255 curve](https://ristretto.group/).
One of the main advantages of Ristretto255 is that it is a prime order group (unlike the Edwards 25519 curve), which obviates small-subgroup attacks on higher-level cryptosystems built on top of it.
Furthermore, Ristretto255 serialization is canonical and deserialization only accepts canonical encodings, which obviates malleability issues in higher-level protocols.

This module has proven useful for implementing several cryptographic primitives:

 1. **Zero-knowledge $\Sigma$-protocols** – See the [`veiled_coin` example](#veiled-coins).
 2. **ElGamal** encryption – See [`aptos_std::ristretto255_elgamal`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/ristretto255_elgamal.move)
 3. **Pedersen** commitments – See [`aptos_std::ristretto255_pedersen`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/ristretto255_pedersen.move)
 4. **Bulletproofs** ZK range proofs[^bulletproofs] – See [`aptos_std::ristretto255_bulletproofs`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/ristretto255_bulletproofs.move)

Need ideas for a cryptosystem to build on top of `ristretto255`?
A popular primitive that you could easily build would be the [schnorrkel](https://github.com/w3f/schnorrkel) signature scheme, which is a hardended version of Schnorr signatures over Ristretto255 groups.

### Generic elliptic curve arithmetic

What is better than one curve? More curves!

The [`aptos_std::crypto_algebra`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/crypto_algebra.move) provides elliptic curve arithmetic operations for **any** supported elliptic curve, including pairing-friendly curves.
As a consequence, Move developers can implement a cryptosystem generically over **any** curve that is or will be supported in the future.
Compared to fixing a particular curve in the code (e.g., by implementing against the [Ristretto255 module](#ristretto255-arithmetic)), this approach provides more flexibility and lowers development time when migrating to a different curve.

Although currently the `crypto_algebra` module only supports arithmetic over BLS12-381 curves (via the marker types declared in [`aptos_std::bls12381_algebra`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/sources/cryptography/bls12381_algebra.move)), more curves will be supported into the future (e.g., BN254, Ristretto255, BLS12-377, BW6-761, secp256k1, secp256r1).

As an example, a Move developer can implement the popular Boneh-Lynn-Shacham (BLS) signature scheme generically over **any** curve by using [type arguments](../../../move/book/functions#type-parameters) for the curve type in their implementation:

```rust title="Generic BLS signature verification over any curve"
use std::option;
use aptos_std::crypto_algebra::{eq, pairing, one, deserialize, hash_to};

/// Example of a BLS signature verification function that works over any pairing-friendly
/// group triple `Gr1`, `Gr2`, `GrT` where signatures are in `Gr1` and PKs in `Gr2`.
/// Points are serialized using the format in `FormatG1` and `FormatG2` and the hashing 
/// method is `HashMethod`.
/// 
/// WARNING: This example is type-unsafe and probably not a great fit for production code.
public fun bls_verify_sig<Gr1, Gr2, GrT, FormatG1, FormatG2, HashMethod>(
    dst:        vector<u8>,
    signature:  vector<u8>,
    message:    vector<u8>,
    public_key: vector<u8>): bool
{
    let sig  = option::extract(&mut deserialize<Gr1, FormatG1>(&signature));
    let pk   = option::extract(&mut deserialize<Gr2, FormatG2>(&public_key));
    let hash = hash_to<Gr1, HashMethod>(&dst, &message);
    
    // Checks if $e(H(m), pk) = e(sig, g_2)$, where $g_2$ generates $\mathbb{G}_2$
    eq(
        &pairing<Gr1, Gr2, GrT>(&hash, &pk), 
        &pairing<Gr1, Gr2, GrT>(&sig, &one<Gr2>())
    )
}
```

Using the `bls_verify_sig` _generic_ function from above, developers can verify BLS signatures over **any** of the supported (pairing-friendly) curves.
For example, one can verify [MinSig BLS](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-bls-signature-05#name-variants) signatures over BLS12-381 curves by calling the function above with the right BLS12-381 marker types as its type arguments:

```rust title="MinSig BLS signature verification over BLS12-381"
use aptos_std::bls12381_algebra::{
    G1, G2, Gt, FormatG1Compr, FormatG2Compr, HashG1XmdSha256SswuRo
};

// Aborts with code 1 if the MinSig BLS signature over the BLS12-381 curve fails to verify. 
assert(
    bls_verify_sig<G1, G2, Gt, FormatG1Compr, FormatG2Compr, HashG1XmdSha256SswuRo>(
        dst, signature, message, public_key
    ),
    1
);
```

For more use cases of the `crypto_algebra` module, check out some Move examples:

1. [Verifying Groth16 zkSNARK proofs](#groth16-zksnark-verifier) over **any** curve 
2. [Verifying randomness from the `drand` beacon](#verifying-randomness-from-the-drand-beacon)

## Building powerful cryptographic applications

### Veiled coins

The [`veiled_coin` example](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/veiled_coin/sources) demonstrates how to use [the Ristretto255 modules from above](#ristretto255-arithmetic) to add a reasonable layer of confidentiality to coin balances and transactions.

Specifically, users can **veil** their balance, keeping it hidden from everyone, including validators.
Furthermore, a user can send a **veiled transaction** that hides the transaction amount from everybody, including validators.
An important caveat is that veiled transactions do **not** hide the identities of the sender or the recipient.

:::danger
This module is educational. It is **not** production-ready. Using it could lead to loss of funds.
:::

### Groth16 zkSNARK verifier

The [`groth16` example](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/move-examples/groth16_example/sources/groth16.move) demonstrates how to verify Groth16 zkSNARK proofs[^groth16], which are the shortest, fastest-to-verify, general-purpose zero-knowledge proofs.
Importantly, as explained [above](#generic-elliptic-curve-arithmetic), this implementation is *generic* over **any** curve, making it very easy for Move developers to use it with their favorite (supported) curves.

:::caution
This code has not been audited by a third-party organization. If using it in a production system, proceed at your own risk.
:::

### Verifying randomness from the `drand` beacon

The [`drand` example](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/drand/sources) shows how to verify public randomness from the [drand](https://drand.love) randomness beacon.
This randomness can be used in games or any other chance-based smart contract.
We give a simple example of a lottery implemented on top of `drand` randomness in [`lottery.move`](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/drand/sources/lottery.move).

:::caution
This code has not been audited by a third-party organization. If using it in a production system, proceed at your own risk.
:::

Another application that can be built on top of `drand` is time-lock encryption[^tlock], which allows users to encrypt information such that it can only be decrypted in a future block.
We do not currently have an implementation but the reader is encouraged to write one!

[^bulletproofs]: _bulletproofs:_ **Bulletproofs: Short Proofs for Confidential Transactions and More**; by B. Bünz and J. Bootle and D. Boneh and A. Poelstra and P. Wuille and G. Maxwell; in 2018 IEEE Symposium on Security and Privacy
[^devalence]: _devalence:_ **It’s 255:19AM. Do you know what your validation criteria are?**, by Henry de Valence, [https://hdevalence.ca/blog/2020-10-04-its-25519am](https://hdevalence.ca/blog/2020-10-04-its-25519am)
[^ed25519]: _ed25519:_ **Ed25519: high-speed high-security signatures**, by Daniel J. Bernstein, Niels Duif, Tanja Lange, Peter Schwabe, Bo-Yin Yang, [https://ed25519.cr.yp.to/](https://ed25519.cr.yp.to/)
[^eddsa]: _eddsa:_ **Taming the Many EdDSAs**, by Konstantinos Chalkias, François Garillot, Valeria Nikolaenko, in SSR 2020, [https://dl.acm.org/doi/abs/10.1007/978-3-030-64357-7_4](https://dl.acm.org/doi/abs/10.1007/978-3-030-64357-7_4)
[^groth16]: _groth16:_ **On the Size of Pairing-Based Non-interactive Arguments**; by Groth, Jens; in EUROCRYPT 2016
[^tlock]: _tlock:_ **tlock: Practical Timelock Encryption from Threshold BLS**; by Nicolas Gailly and Kelsey Melissaris and Yolan Romailler; [https://eprint.iacr.org/2023/189](https://eprint.iacr.org/2023/189)
