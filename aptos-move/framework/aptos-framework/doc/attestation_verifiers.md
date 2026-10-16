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

## Tinfoil v3 envelope with FormatV2 collateral

Governance first approves the expected GitHub repository through
`tinfoil_v3::set_repository_approved`. The requesting account calls
`begin_entry(trusted_repo, expected_tls_spki_sha256, lifetime_secs)` and reads
the 32-byte validator-random nonce from `pending_nonce` or the emitted event. The workload
obtains a document for that nonce.

The document is split into five fragments with
`aptos_tinfoil_v3::split_stage_fragments`. Each retains the same challenge,
quote and quote-bound cryptographic material, but carries the collateral for
one stage. For each fragment, the submitter uploads at most 48 KiB per
`append_chunk` transaction, then calls the corresponding verification entry
point:

1. `verify_quote`: Intel TDX quote, Intel PCS collateral, REPORTDATA, TLS and
   HPKE keys. This stage pins the current Intel and Sigstore roots and chain
   time for the request.
2. `verify_code`: signed code release and measured code registers.
3. `verify_code_freshness`: code-release freshness witness.
4. `verify_platform`: signed platform policy and host measurements.
5. `verify_entry`: platform-policy freshness witness; this commits the result.

The module keeps authenticated intermediate claims between transactions. Each
later fragment must match the authenticated quote and REPORTDATA of stage 1.
A change to either effective root or revocation of the approved repository
invalidates the request or stored result. `clean_up` removes uploaded chunks
in bounded calls after completion, cancellation, or expiry.

The present verifier is **CPU-only**. It rejects GPU and NVSwitch device
evidence, and any release whose authenticated VM shape requires GPUs. The
hardware quote, collateral signatures, Sigstore signatures, trusted roots,
freshness and workload measurements all need to verify before a result is
published. An application must still pin the returned TLS certificate
fingerprint or HPKE key to the connection it actually uses.

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
