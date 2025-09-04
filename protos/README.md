# Protos
This directory contains the protobuf definitions for all Velor services. For the sake of simplifying release and minimizing potential version conflicts, we include all protos and code generated from those protos in one place.

Make sure to install buf, e.g. for Mac:
```
brew install bufbuild/buf/buf
```

If you see unexpected changes, make sure the version of buf you have matches the version we use in CI, see [`.github/workflows/check-protos.yaml`](../.github/workflows/check-protos.yaml).

If you update the proto definitions in `proto/`, you can regenerate the code for all languages based on those protos by running this script:
```bash
./scripts/build_protos.sh
```

If you haven't installed deps yet, run this script from this directory:
```bash
./scripts/install_deps.sh
```

# Transactions

## Signatures

Converted signatures part of transactions proto to uml (below)

<img width="1777" height="1251" alt="signatures_plantuml" src="https://github.com/user-attachments/assets/49d11153-1eec-46d3-86fb-4d2a85f1aba8" />

```uml
@startuml SignatureStructure

' Root Signature message
class Signature {
  +Type type
  .. oneof signature ..
  +Ed25519Signature ed25519
  +MultiEd25519Signature multi_ed25519
  +MultiAgentSignature multi_agent
  +FeePayerSignature fee_payer
  +SingleSender single_sender
}

enum Signature::Type {
  TYPE_UNSPECIFIED = 0
  TYPE_ED25519 = 1
  TYPE_MULTI_ED25519 = 2
  TYPE_MULTI_AGENT = 3
  TYPE_FEE_PAYER = 4
  TYPE_SINGLE_SENDER = 6
}

' Ed25519Signature
class Ed25519Signature {
  +bytes public_key
  +bytes signature
}

' MultiEd25519Signature
class MultiEd25519Signature {
  +repeated bytes public_keys
  +repeated bytes signatures
  +uint32 threshold
  +repeated uint32 public_key_indices
}

' MultiAgentSignature
class MultiAgentSignature {
  +AccountSignature sender
  +repeated string secondary_signer_addresses
  +repeated AccountSignature secondary_signers
}

' FeePayerSignature
class FeePayerSignature {
  +AccountSignature sender
  +repeated string secondary_signer_addresses
  +repeated AccountSignature secondary_signers
  +string fee_payer_address
  +AccountSignature fee_payer_signer
}

' SingleSender
class SingleSender {
  +AccountSignature sender
}

' AccountSignature
class AccountSignature {
  +Type type
  .. oneof signature ..
  +Ed25519Signature ed25519
  +MultiEd25519Signature multi_ed25519
  +SingleKeySignature single_key_signature
  +MultiKeySignature multi_key_signature
  +AbstractionSignature abstraction
}

enum AccountSignature::Type {
  TYPE_UNSPECIFIED = 0
  TYPE_ED25519 = 1
  TYPE_MULTI_ED25519 = 2
  TYPE_SINGLE_KEY = 4
  TYPE_MULTI_KEY = 5
  TYPE_ABSTRACTION = 6
}

' SingleKeySignature
class SingleKeySignature {
  +AnyPublicKey public_key
  +AnySignature signature
}

' MultiKeySignature
class MultiKeySignature {
  +repeated AnyPublicKey public_keys
  +repeated IndexedSignature signatures
  +uint32 signatures_required
}

' AbstractionSignature
class AbstractionSignature {
  +string function_info
  +bytes signature
}

' IndexedSignature
class IndexedSignature {
  +uint32 index
  +AnySignature signature
}

' AnyPublicKey
class AnyPublicKey {
  +Type type
  +bytes public_key
}

enum AnyPublicKey::Type {
  TYPE_UNSPECIFIED = 0
  TYPE_ED25519 = 1
  TYPE_SECP256K1_ECDSA = 2
  TYPE_SECP256R1_ECDSA = 3
  TYPE_KEYLESS = 4
  TYPE_FEDERATED_KEYLESS = 5
}

' AnySignature
class AnySignature {
  +Type type
  +bytes signature (deprecated)
  .. oneof signature_variant ..
  +Ed25519 ed25519
  +Secp256k1Ecdsa secp256k1_ecdsa
  +WebAuthn webauthn
  +Keyless keyless
}

enum AnySignature::Type {
  TYPE_UNSPECIFIED = 0
  TYPE_ED25519 = 1
  TYPE_SECP256K1_ECDSA = 2
  TYPE_WEBAUTHN = 3
  TYPE_KEYLESS = 4
}

class Ed25519 {
  +bytes signature
}

class Secp256k1Ecdsa {
  +bytes signature
}

class WebAuthn {
  +bytes signature
}

class Keyless {
  +bytes signature
}

' Associations
Signature --> Ed25519Signature
Signature --> MultiEd25519Signature
Signature --> MultiAgentSignature
Signature --> FeePayerSignature
Signature --> SingleSender

MultiAgentSignature --> AccountSignature
FeePayerSignature --> AccountSignature
SingleSender --> AccountSignature

AccountSignature --> Ed25519Signature
AccountSignature --> MultiEd25519Signature
AccountSignature --> SingleKeySignature
AccountSignature --> MultiKeySignature
AccountSignature --> AbstractionSignature

SingleKeySignature --> AnyPublicKey
SingleKeySignature --> AnySignature
MultiKeySignature --> AnyPublicKey
MultiKeySignature --> IndexedSignature
IndexedSignature --> AnySignature

AnySignature --> Ed25519
AnySignature --> Secp256k1Ecdsa
AnySignature --> WebAuthn
AnySignature --> Keyless

@enduml
```
