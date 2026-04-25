# The `TRX` batch threshold encryption scheme

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
- Block-specific decryption key reconstruction and verification are in
  `src/shared/key_derivation.rs`.
- Types/code related to digest computation are in `src/shared/digest.rs`
  and `src/shared/ids.rs`.
- Various required algebraic operations are implemented in
  `src/shared/algebra`. Most notably of these, two implementations of the
  generalized FK algorithm are in `src/shared/algebra/fk_algorithm.rs`. 
