# Conversation Summary: Transaction Formats and Signing Specification

## Session Overview
**Date:** January 28, 2026  
**Objective:** Create comprehensive specification documentation for Aptos transaction formats and signing mechanisms

---

## User Request Summary

The user requested the following documentation:

### Part 1: Transaction Format Specifications (BCS)
1. Ed25519 transaction format
2. SingleKey transaction format
3. MultiKey transaction format
4. MultiEd25519 transaction format

### Part 2: Signing Documentation
1. Ed25519 signing
2. MultiEd25519 signing
3. Secp256k1 signing
4. Secp256r1 signing
5. Keyless signing

### Part 3: Advanced Transaction Building
1. How to hash transactions for signing
2. Multi-agent transaction building
3. Fee payer (sponsored) transaction building
4. Multisig transaction building

---

## Commands Executed

| # | Command/Action | Purpose |
|---|----------------|---------|
| 1 | `Grep "struct.*Transaction"` | Find transaction-related structures |
| 2 | `Grep "Ed25519\|SingleKey\|MultiKey\|MultiEd25519"` | Find authentication types |
| 3 | `Grep "secp256k1\|secp256r1\|Keyless"` | Find additional key types |
| 4 | `Read types/src/transaction/authenticator.rs` | Read authenticator definitions |
| 5 | `Read types/src/transaction/mod.rs` | Read transaction module |
| 6 | `Read types/src/keyless/mod.rs` | Read keyless authentication |
| 7 | `Read types/src/transaction/multisig.rs` | Read multisig structures |
| 8 | `Grep "RawTransactionWithData\|signing_message"` | Find signing message creation |
| 9 | `Read crates/aptos-crypto/src/traits/mod.rs` | Read crypto traits |
| 10 | `Read crates/aptos-crypto/src/multi_ed25519.rs` | Read multi-ed25519 implementation |
| 11 | `git checkout -B cursor/transaction-formats-and-signing-6309` | Create feature branch |

---

## Key Findings from Codebase Analysis

### Transaction Authenticator Types (Enum Variants)
```
TransactionAuthenticator:
  - Ed25519 { public_key, signature }
  - MultiEd25519 { public_key, signature }
  - MultiAgent { sender, secondary_signer_addresses, secondary_signers }
  - FeePayer { sender, secondary_signer_addresses, secondary_signers, fee_payer_address, fee_payer_signer }
  - SingleSender { sender: AccountAuthenticator }
```

### Account Authenticator Types
```
AccountAuthenticator:
  - Ed25519 { public_key, signature }
  - MultiEd25519 { public_key, signature }
  - SingleKey { authenticator: SingleKeyAuthenticator }
  - MultiKey { authenticator: MultiKeyAuthenticator }
  - NoAccountAuthenticator
  - Abstract { authenticator: AbstractAuthenticator }
```

### AnyPublicKey Variants
```
AnyPublicKey:
  - Ed25519 { public_key }
  - Secp256k1Ecdsa { public_key }
  - Secp256r1Ecdsa { public_key }
  - Keyless { public_key }
  - FederatedKeyless { public_key }
  - SlhDsa_Sha2_128s { public_key }
```

### Signing Message Construction
The signing message is constructed as:
```
signing_message = Hasher::seed() || BCS(message)
```

Where `Hasher::seed()` is a domain separation prefix.

---

## Plan Status

- [ ] Interview user for additional details
- [ ] Create specifications/ folder
- [ ] Write Ed25519 transaction format spec
- [ ] Write SingleKey transaction format spec
- [ ] Write MultiKey transaction format spec
- [ ] Write MultiEd25519 transaction format spec
- [ ] Write signing documentation for each key type
- [ ] Write transaction hashing documentation
- [ ] Write multi-agent transaction documentation
- [ ] Write fee payer transaction documentation
- [ ] Write multisig transaction documentation

---

## Interview Questions & Answers

### 1. Target Audience
**Q:** Who is the primary audience?  
**A:** SDK developers and application integrators. Purpose is to provide proper specifications for new language integrations.

### 2. Programming Language Examples
**Q:** Should code examples be language-agnostic or specific languages?  
**A:** Provide examples in **Rust**, **Python**, and **TypeScript**.

### 3. BCS Encoding Detail Level
**Q:** How detailed should the BCS encoding explanations be?  
**A:** BCS is handled in a different repository (will be referenced later). However, documentation should be comprehensive at the **byte level** showing exactly what is needed.

### 4. Test Vectors
**Q:** Should test vectors be included?  
**A:** **Yes**, generate test vectors from the actual codebase.

### 5. Diagram Format
**Q:** What diagram format?  
**A:** **Mermaid diagrams** (renders in GitHub markdown).

### 6. Authentication Key Derivation
**Q:** Document address derivation from public keys?  
**A:** **Yes**, include full documentation.

### 7. WebAuthn/Passkey Details
**Q:** Scope for Secp256r1/WebAuthn?  
**A:** **Full coverage** - include complete WebAuthn assertion format and cryptographic signing.

### 8. Keyless Depth
**Q:** Scope for Keyless signing?  
**A:** **All paths** - cover both OpenIdSig and ZeroKnowledgeSig, including pepper, ephemeral keys, and JWT structure.

### 9. Version/Chain Specificity
**Q:** Chain-specific or generic?  
**A:** **Generic** with examples for both mainnet and testnet.

### 10. Error Handling
**Q:** Document error cases and validation rules?  
**A:** **Yes**, include common errors and validation rules.

---

## Documentation Created

### Transaction Format Specifications (`specifications/transaction-formats/`)

1. **01-raw-transaction.md** - Core RawTransaction BCS format with byte-level details
2. **02-ed25519-authenticator.md** - Ed25519 authentication format
3. **03-single-key-authenticator.md** - SingleKey unified authentication format
4. **04-multi-key-authenticator.md** - MultiKey K-of-N multi-signature format
5. **05-multi-ed25519-authenticator.md** - Legacy MultiEd25519 format

### Signing Process Documentation (`specifications/signing/`)

1. **01-transaction-hashing.md** - Domain separation and signing message construction
2. **02-ed25519-signing.md** - Ed25519 EdDSA signing process
3. **03-multi-ed25519-signing.md** - K-of-N multi-signature coordination
4. **04-secp256k1-signing.md** - Secp256k1 ECDSA with low-S normalization
5. **05-secp256r1-signing.md** - Secp256r1/WebAuthn/Passkey signing
6. **06-keyless-signing.md** - OIDC-based keyless authentication (ZK and OpenID paths)

### Advanced Transaction Documentation (`specifications/advanced/`)

1. **01-multi-agent-transactions.md** - Multiple independent signers
2. **02-fee-payer-transactions.md** - Sponsored/gasless transactions
3. **03-multisig-transactions.md** - On-chain multisig workflow

### Overview Documentation

- **specifications/README.md** - Navigation guide and quick reference tables

---

## Commits Made

| Commit Hash | Description |
|-------------|-------------|
| (pending) | Initial documentation structure |

---

*This document is updated as the conversation progresses.*
