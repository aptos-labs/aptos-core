---
id: generate-format
title: Generate Format
custom_edit_url: https://github.com/velor-chain/velor-core/edit/main/testsuite/generate-format/README.md
---

`generate-format` hosts the Velor core type checker to ensure compatibility and uses 
[`serde-reflection`](https://github.com/velor-chain/serde-reflection) to properly track type changes over time. 

## How to make a change

When you introduce a new struct, enum, variant, or other type, ensure you make changes to the following files:
- [x] api.rs
- [x] velor.rs
- [x] consensus.rs

as well as
- [x] api.yaml
- [x] velor.yaml
- [x] consensus.yaml

## Example
As an example, we will walk through a real-life example to demonstrate how to make the appropriate changes.
Feel free to follow along here: https://github.com/velor-chain/velor-core/pull/10755/files

Suppose you're adding a new `secp256r1_ecdsa` crypto library with new structs for the following keys and signatures:
- `PublicKey` 
- `Signature` 
- `PrivateKey`

In addition, you are creating a set of structs to support WebAuthn transactions,
such as `PartialAuthenticatorAssertionResponse` and `AssertionSignature`.

Below we'll walk through the necessary changes to ensure these types are tracked appropriately

### Crypto library changes

The following changes should be made to support `secp256r1_ecdsa` keys and signatures properly

In the following files
- [x] api.rs
- [x] velor.rs
- [x] consensus.rs

add `tracer.trace_value` for secp256r1_ecdsa

```rust
fn trace_crypto_values(tracer: &mut Tracer, samples: &mut Samples) -> Result<()> {
    ...
    // Add tracing for secp256r1_ecdsa keys and sigs
    tracer.trace_value(samples, &secp256r1_ecdsa_private_key)?;
    tracer.trace_value(samples, &secp256r1_ecdsa_public_key)?;
    tracer.trace_value(samples, &secp256r1_ecdsa_signature)?;

    Ok(())
}
```

> Note: You may also want to add the `key_name` macro if you are using a different name than your struct name 

```rust
#[key_name("Secp256r1EcdsaPrivateKey")]
pub struct PublicKey {...}
```

Additionally, in the following files
- [x] api.yaml
- [x] velor.yaml
- [x] consensus.yaml

add the following yaml

```yaml
...
Secp256r1EcdsaPrivateKey:
  NEWTYPESTRUCT: BYTES
Secp256r1EcdsaPublicKey:
  NEWTYPESTRUCT: BYTES
Secp256r1EcdsaSignature:
  NEWTYPESTRUCT: BYTES
...
```

### Struct and enum changes

`Secp256r1Ecdsa` was also added as an enum variant on `AnyPublicKey`, so the appropriate updates must be made to the `AnyPublicKey` enum:

```yaml
AnyPublicKey:
  ENUM:
    ...
    2:
      Secp256r1Ecdsa:
        STRUCT:
          - public_key:
              TYPENAME: Secp256r1EcdsaPublicKey
```

Additionally, a new enum variant - `WebAuthn` - was added to `AnySignature` to support WebAuthn signatures. 
The `WebAuthn` variant takes in a `PartialAuthenticatorAssertionResponse` struct.  

```yaml
AnySignature:
  ENUM:
    ...
    2:
      WebAuthn:
        STRUCT:
          - signature:
              TYPENAME: PartialAuthenticatorAssertionResponse
PartialAuthenticatorAssertionResponse:
  STRUCT:
    - signature:
        TYPENAME: AssertionSignature
    - authenticator_data: BYTES
    - client_data_json: BYTES
AssertionSignature:
  ENUM:
    0:
      Secp256r1Ecdsa:
        STRUCT:
          - signature:
              TYPENAME: Secp256r1EcdsaSignature
```

Ensure that the changes above are synchronized across all of these files:
- [x] api.yaml
- [x] velor.yaml
- [x] consensus.yaml

> Note: Because `[api|velor|consensus].rs`, are already tracking the `AnyPublicKey` and `AnySignature` struct, no further tracers are necessary here

Additionally, ensure that `enums` are tracked correctly across `[api|velor|consensus].rs`
```rust
fn get_registry(){
    ...
    tracer.trace_type::<transaction::webauthn::AssertionSignature>(&samples)?;
    ...
}
```

Lastly ensure that the struct has the appropriate serde macros if needed. In this case, we want to serialize
`PartialAuthenticatorAssertionResponse`'s `authenticator_data` and `client_data_json`'s `Vec<u8>` to `BYTES` 
so we will need to add the `serde_bytes` macro, like so

```rust
/// `PartialAuthenticatorAssertionResponse` includes a subset of the fields returned from
/// an [`AuthenticatorAssertionResponse`](passkey_types::webauthn::AuthenticatorAssertionResponse)
///
/// See <https://www.w3.org/TR/webauthn-3/#authenticatorassertionresponse>
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]

pub struct PartialAuthenticatorAssertionResponse {
    /// This attribute contains the raw signature returned from the authenticator.
    /// NOTE: Many signatures returned from WebAuthn assertions are not raw signatures.
    /// As an example, secp256r1_ecdsa signatures are encoded as an [ASN.1 DER Ecdsa-Sig_value](https://www.w3.org/TR/webauthn-3/#sctn-signature-attestation-types)
    /// If the signature is encoded, the client is expected to convert the encoded signature
    /// into a raw signature before including it in the transaction
    signature: AssertionSignature,
    /// This attribute contains the authenticator data returned by the authenticator.
    /// See [`AuthenticatorData`](passkey_types::ctap2::AuthenticatorData).
    #[serde(with = "serde_bytes")]
    authenticator_data: Vec<u8>,
    /// This attribute contains the JSON byte serialization of [`CollectedClientData`](CollectedClientData) passed to the
    /// authenticator by the client in order to generate this credential. The exact JSON serialization
    /// MUST be preserved, as the hash of the serialized client data has been computed over it.
    #[serde(with = "serde_bytes")]
    client_data_json: Vec<u8>,
}
```
