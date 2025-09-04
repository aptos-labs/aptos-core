---
id: crypto
title: Crypto
custom_edit_url: https://github.com/velor-chain/velor-core/edit/main/crypto/crypto/README.md
---

The crypto component hosts all the implementations of cryptographic primitives we use in Velor: hashing, signatures, multisignatures, aggregate signatures, and key derivation/generation.

To enforce type-safety for signature schemes, we rely on traits from  [`traits.rs`](src/traits.rs) and [`validatable.rs`](src/validatable.rs).

## Overview

Velor makes use of several cryptographic algorithms:

- **SHA-3** as the main hash function
  + Standardized in [FIPS 202](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
  + Based on the [tiny_keccak](https://docs.rs/tiny-keccak/) crate
- **HKDF: HMAC-based Extract-and-Expand Key Derivation Function**
  + Standardized in [RFC 5869](https://tools.ietf.org/html/rfc5869)
  + Used to generate keys from a salt (optional), seed, and application-info (optional)
- **Ed25519** signatures and (naive) multisignatures
  + Based on the [ed25519-dalek](https://docs.rs/ed25519-dalek/) crate with additional security checks (e.g., for malleability)
- **Boneh-Shacham-Lynn (BLS) multisignatures and aggregate signatures**
  + Based on the [blst](https://docs.rs/blst/) crate
  + Implemented on top of Barreto-Lynn-Scott BLS12-381 elliptic curves
- The **[Noise Protocol Framework](http://www.noiseprotocol.org/)**
  - Used to create authenticated and encrypted communications channels between validators
- **X25519** key exchange
  + Based on the [x25519-dalek](https://docs.rs/x25519-dalek) crate
  + Used in our implementation of the [Noise Protocol Framework](http://www.noiseprotocol.org/)

## Traits for safer cryptography implementation

Before implementing a cryptographic primitive, be sure to read [`traits.rs`](src/traits.rs) and [`validatable.rs`](src/validatable.rs) to understand how to comply with our API as well as **some** of the security concerns involved.

## How is this module organized?
```
    crypto/src
    ├── bls12-381/          # Boneh-Lynn-Shacham (BLS) signatures over (Barreto-Lynn-Scott) BLS12-381 curves
    ├── unit_tests/         # Unit tests
    ├── lib.rs
    ├── ed25519/            # Ed25519 implementation of the signing/verification API in traits.rs
    ├── hash.rs             # Hash function (SHA-3)
    ├── hkdf.rs             # HKDF implementation
    ├── multi_ed25519.rs    # MultiEd25519 implementation of the signing/verification API in traits.rs
    ├── noise.rs            # Noise Protocol Framework implementation
    ├── test_utils.rs
    ├── traits.rs           # Traits for safer implementations of signature schemes
    ├── validatable.rs      # Traits for deferring validation of group elements (e.g., public keys, signatures)
    └── x25519.rs           # X25519 implementation

```

## Changelog

 - This crate historically had support for (a different) BLS12-381, [EC-VRF](https://tools.ietf.org/id/draft-goldbe-vrf-01.html#rfc.section.5), and [SLIP-0010](https://github.com/satoshilabs/slips/blob/master/slip-0010.md), though were removed due to lack of use. The last git revision before the removal is 00301524.
