# aptos-keyless-verify

Standalone Aptos **keyless signature verification** for off-chain consumers.

## Why this crate exists

Verifying a keyless signature in Rust today requires depending on
`aptos-types`, which transitively pulls the full aptos-core build environment:

- `[patch.crates-io]` forks: `merlin`, `futures`, `ark-ec` / `ark-ff` /
  `ark-poly` / `ark-serialize`, `serde-reflection`, `dudect-bencher`.
- `--cfg tokio_unstable` rustflag (required by `aptos-runtimes`).
- `pkcs8 = 0.11.0-rc.11` pre-release pin (required by `slh-dsa`).
- 90+ transitive crates including `aptos-dkg`, `aptos-runtimes`,
  `aptos-batch-encryption`, `move-binary-format`, `move-vm-types`,
  `poem-openapi`, …

That's a lot of build-environment surface for a downstream service that only
wants to verify a signature. Cold builds in our internal services take
several minutes; CI matrices that exercise multiple Rust versions are
prohibitive.

`aptos-keyless-verify` carves out just the keyless verification surface.
Deps: `ark-bn254`, `ark-groth16`, `ark-ec`, `ark-ff`, `ark-serialize`,
`ark-std`, `ed25519-dalek`, `p256`, `jsonwebtoken`, `ring`, `serde`,
`serde_bytes`, `bcs`, `sha2`, `sha3`, `base64`, `thiserror`, `anyhow`. No
aptos-\* deps. No forked patches. No tokio_unstable.

## Public API surface

```rust
pub fn verify_keyless(
    pk: &KeylessPublicKey,
    signature: &KeylessSignature,
    message: &[u8],
    jwk: &RsaJwk,
    groth16_vk: &Groth16VerificationKey,
    config: &Configuration,
    now_unix_secs: u64,
) -> Result<(), VerifyError>;
```

All chain-dependent inputs are taken by reference — the caller is
responsible for fetching the JWK / VK / configuration from a fullnode (or
from a cache). The function is pure: no async, no clock, no I/O.

The auth-key binding (computing the keyless account address and comparing
against the on-chain `authentication_key`) is also pure but lives on the
type:

```rust
let auth_key: [u8; 32] = pk.account_authentication_key();
```

The worker (or other caller) compares this against the on-chain auth key.

## Relationship with `aptos-types`

This crate is **additive** in this PR — `aptos-types::keyless` continues to
own the canonical implementation. The follow-up plan is:

1. **This PR (skeleton):** crate exists, public API surface is locked in,
   no implementation yet. Reviewers can verify the dep tree.
2. **Subsequent PRs on this branch:** port verification logic module by
   module (`bn254_circom::get_public_inputs_hash`, Groth16 verification,
   JWT verification, ephemeral signature dispatch).
3. **Eventually:** migrate `aptos-types::keyless` to re-export from this
   crate, eliminating duplication.

Until step 3 lands, the contract between the two crates is **BCS wire
format**: signatures and public keys round-trip cleanly via `bcs::to_bytes`
/ `bcs::from_bytes` between the two crates' types. The struct definitions
are intentionally mirrored field-for-field.

## Status

🚧 **Skeleton.** `verify_keyless` returns `VerifyError::Unsupported` until
the verification logic is ported in follow-up commits on this branch. The
type definitions are complete and BCS-compatible with `aptos-types`.
