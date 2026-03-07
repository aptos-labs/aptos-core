# The `FPTX` batch threshold encryption scheme

## Overview

This crate includes the batch threshold encryption scheme used in the Aptos
encrypted mempool. There are several variants of the scheme:

* `FPTX` is the original unweighted version of the scheme.
* `FPTXWeighted` is a wrapper around `FPTX` where each master secret key
  share and decryption key share is weighted. This is in order to support
  the weighted setting of a proof-of-stake blockchain. **`FPTXWeighted` is
  the scheme used in production.**
* `FPTXSuccinct` is a modified version of the scheme which saves a single
  group element per ciphertext. However, it requires a DKG which outputs
  $[\tau \cdot \mathsf{sk}]_2$, where $\mathsf{sk}$ is the per-epoch master
  secret key, which our DKG does not currently do. **Because of this, it is
  not currently used in production.**

## Repo structure

- `src/traits.rs` defines the interface which this crate exports to be used
  during consensus. Each of the schemes implement this trait.
- The individual schemes are in `src/schemes`.
- The elliptic curve group used in this crate is defined in `src/group.rs`.
- Ciphertexts are defined in `src/shared/ciphertext`. Each scheme's ciphertext
  consists of two layers: an inner "batch IBE" ciphertext, defined in
  `src/shared/ciphertext/{bibe.rs,bibe_succinct.rs}`, and a wrapper, in
  `src/shared/ciphertext/mod.rs`, which generically turns a batch IBE ciphertext
  into a non-malleable (non-IBE) ciphertext. To do this, it uses various
  symmetric crypto primitives defined in `src/shared/symmetric.rs`.
  - **Note: `src/shared/ciphertext/bibe_succinct.rs` is part of
    `FPTXSuccinct`, and is not used in production.**
- Block-specific decryption key reconstruction and verification are in
  `src/shared/key_derivation.rs`.
- Types/code related to digest computation are in `src/shared/digest.rs`
  and `src/shared/ids.rs`.
- Various required algebraic operations are implemented in
  `src/shared/algebra`. Most notably of these, two implementations of the
  generalized FK algorithm are in `src/shared/algebra/fk_algorithm.rs`. We
  currently only use the more naive of these two implementations, as it is
  faster for our batch size.
