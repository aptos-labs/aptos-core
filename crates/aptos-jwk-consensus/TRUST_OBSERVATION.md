# Validator-observed trust material

`0x1::trust_observation` lets governance configure HTTPS resources whose bytes
are observed by validators and committed by a quorum-certified validator
transaction. It is separate from the existing JWK observer, which continues to
serve Keyless accounts.

## Target configuration

Governance stages each target for the next epoch with
`upsert_target_for_next_epoch(id, url, format, max_bytes)` or removes it with
`remove_target_for_next_epoch(id)`. The module assigns a configuration version
that changes when a target is edited. Up to 16 targets can be configured, and
each response is capped at 60 KiB or the target's lower `max_bytes` value.
The combined resource remains below Aptos's 1 MiB per-write limit even when
all 16 targets reach this cap.

The observer accepts only HTTPS, follows no redirects, and has a five-second
fetch timeout. The configured `format` selects a canonicalizer compiled into
the validators:

| Format | Value signed by validators |
| --- | --- |
| `1` | One DER certificate, as received, with its outer X.509 structure checked |
| `2` | JSON object with recursively sorted object keys and compact serialization |
| `3` | Nonempty response bytes, unchanged |

The three formats can cover an Intel DER root, a Sigstore TrustedRoot JSON
document, and other stable byte resources. A new canonicalization rule requires
validator code support; the target configuration cannot supply executable code.
The DER check is structural. An attestation verifier still checks the
certificate's signature, chain, validity period, and format-specific policy.

## Commit and read path

Each validator fetches and canonicalizes the configured target. When the value
differs from the current observation, validators sign the target ID,
configuration version, epoch, next observation version, and value. A quorum
certificate enters a validator transaction. VM execution checks the current
configuration, epoch, version, size, voting power, and aggregate signature
before updating `ObservedTrustData`.

`ObservedTrustData` records the validator observation. Governance can apply a
replacement or removal patch without changing that observation. Attestation
verifiers read `get_effective_value(target_id)`, which resolves the patch over
the latest observation. A patch stays in effect until governance clears it or
changes/removes the target configuration.

## Rollout

Genesis initializes the new resources empty. An existing chain must initialize
them through governance as part of its framework upgrade before staging any
targets. The validator, VM, and framework changes must be activated together.
The attestation-format-specific verification functions are separate consumers
of `get_effective_value`; this change supplies their observed trust material.
