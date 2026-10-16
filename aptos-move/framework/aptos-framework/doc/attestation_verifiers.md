# On-chain attestation verifiers

These Framework modules consume effective trust material maintained by
`trust_observation` on chain. The VM does not fetch a URL or consult the host
clock while verifying a proof. Governance must configure the corresponding
observation targets before either verifier can succeed.

| Module | Effective target | Value |
| --- | --- | --- |
| `google_space` | `google_space_jwks` | Google Confidential Space signing JWKS, canonical JSON |
| `tinfoil_v3` | `intel_sgx_root_ca` | Intel SGX Provisioning Certification Root CA certificate, DER |
| `tinfoil_v3` | `sigstore_trusted_root` | Sigstore trusted root, canonical JSON |

The Tinfoil module currently accepts the **v3 document envelope with the
legacy FormatV2 collateral flow used by Atlas**. That flow carries Intel PCS
collateral, Sigstore code and platform references, and separate freshness
witnesses. It rejects the newer `collateral_format=v3` config/runtime flow.
This distinction is part of the API contract; a new format requires its own
verification stages and trust material.

## Google Confidential Space

1. The account that will hold the verification result calls
   `google_space::begin_entry(audience, expected_image_digest,
   key_commitment, lifetime_secs)`. The module emits a one-use lowercase-hex
   nonce from validator randomness and exposes it through `pending_nonce`.
   This entry approves only empty command arguments and environment claims.
   For a workload with explicit launch settings, call the private transaction
   entry `begin_entry_with_launch_claims` with a JSON object containing the
   exact expected `args`, `cmd_override`, `env`, and `env_override` values.
   Real workloads commonly have nonempty `args` or `env`; configure those
   expectations explicitly rather than relying on the strict empty policy.
2. The workload requests a Google Confidential Space token using that exact
   nonce and audience. It submits the token with `verify_entry` before the
   challenge or token expires.
3. The native checks the RS256 signature against the **current chain JWKS**,
   issuer, audience, nonce, issuance time after the challenge, chain time,
   Confidential Space claims, image digest, and approved launch settings. It
   rejects signed memory monitoring. The module records and emits the verified
   claims. Consumers can use
   `last_verified` and `is_current` and should compare the result's program
   identity with their own policy.

`key_commitment` ties the challenge to an application-selected worker key. The
token does not attest possession of that key by itself. An application that
uses the result to register a worker must bind the key to the workload that
requested the token and verify possession before trusting the registration.

## Tinfoil v3 verification objects

Governance first approves the expected GitHub repository through
`tinfoil_v3::set_repository_approved`. A CVM submits its complete attestation
with `tinfoil_v3::new_verification(attestation)`. The
`VerificationCreated` event gives the new object's address. A normal Aptos
transaction is limited to 64 KiB by the default gas schedule, so this entry
caps the report at 48 KiB. Reports up to 256 KiB use
`new_verification_chunked`, repeated `append_verification_chunk` calls, and
`seal_verification`. Both paths produce the same verification object. Chunks
occupy separate table items because a single chain state item is limited to
1 MiB. The 256 KiB bound keeps the per-byte native charge below the current
execution gas ceiling; realistic reports still need end-to-end gas measurement.
Larger reports require a stage-fragment object format and are rejected at
creation.

The CVM or any other account then calls
`tinfoil_v3::advance_verification(verification_address)` until its
`VerificationAdvanced` event reports `PASS` or `FAIL`. A `CONTINUE` event
identifies that another transaction is needed. The object owns the exact
submitted bytes, their hash, the current stage, authenticated intermediate
claims, and the terminal outcome. An invalid proof produces a persistent
`FAIL` rather than aborting the transaction and losing its outcome.

The stages check the Intel TDX quote and REPORTDATA, signed code release,
code freshness, signed platform endorsement, and platform freshness. They use
the effective Intel and Sigstore trust values observed in chain state and
reject root changes or repository revocation during verification. A business
contract takes the object address and calls the public `verified_report_*`
getters. Those getters abort unless the object is `PASS`, recent, and still
uses the effective roots and approved repository. The business contract must
check the returned workload identity and key against its own policy. The event
alone never authorizes a worker.

The verifier consumes the attestation's 32-byte nonce without issuing its own
challenge. An application that needs transaction-specific freshness can issue
a challenge before the CVM creates its report, then compare that challenge
with `verified_nonce` when consuming the PASS object. It must also bind the
verified TLS or HPKE key to the registering CVM and check proof of possession.
The verifier checks collateral and witness time against chain time, but a
PASS object alone does not prove who submitted or controls the attested keys.

The currently supported Tinfoil input is the **v3 envelope with FormatV2
collateral used by Atlas**. Newer `collateral_format=v3` config/runtime
reports need separate verification logic. GPU-shaped reports do not reach
`PASS` yet: an NVIDIA NRAS EAT signs an appraisal result and GPU identities,
but the current producer does not bind those identities or the appraised raw
GPU evidence to the raw GPU evidence in the CPU quote. Checking only a shared
nonce and GPU count would allow evidence substitution. The offline NRAS EAT
signature verifier is available as a building block; this cross-evidence
binding must be established before the chain reports GPU verification.

The older `begin_entry` / `append_chunk` / `verify_*` entry points remain
available for existing callers during this API transition.

## Integration status

The modules expose reusable verified claims; they do not register a c26t
worker or define a c26t policy. Gas coefficients are provisional until measured
against representative full-size reports. A production deployment also needs
a valid end-to-end Tinfoil fixture for the exact collateral format and device
profile it intends to accept.

These natives are disabled before gas feature version 55. Version 55 adds four
attestation verification gas keys. A network must install a version 55 gas
schedule containing all four keys as part of its coordinated gas upgrade before
publishing and using these modules. Existing version 54 schedules remain valid
when nodes load this binary; the natives fail closed there rather than execute
expensive cryptography without a gas charge.
