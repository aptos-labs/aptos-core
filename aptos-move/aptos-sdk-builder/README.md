---
id: aptos-sdk-builder
title: Supra SDK Builder
custom_edit_url: https://github.com/aptos-labs/aptos-core/edit/main/language/aptos-sdk-builder/README.md
---

# Supra SDK Builder

A *transaction builder* is a helper function that converts its arguments into the payload of an Supra transaction calling a particular Move script.

In Rust, the signature of such a function typically looks like this:
```rust
pub fn encode_peer_to_peer_with_metadata_script(
    token: TypeTag,
    payee: AccountAddress,
    amount: u64,
    metadata: Vec<u8>,
    metadata_signature: Vec<u8>,
) -> Script;
```

This crate provide a library to generate transaction builders in one programming language.

The tool will also generate and install type definitions for Supra types such as `TypeTag`, `AccountAddress`, and `Script`.

In practice, hashing and signing Supra transactions additionally requires a runtime library for Binary Canonical Serialization ("BCS").
Such a library will be installed together with the Supra types.


## Supported Languages

The following languages are currently supported:
* Rust
