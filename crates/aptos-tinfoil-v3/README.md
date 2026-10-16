# Tinfoil v3 verifier for Aptos

This crate checks an Atlas Tinfoil attestation v3 **envelope with legacy
collateral FormatV2** offline. An absent `collateral_format` defaults to V2;
an explicit `https://tinfoil.sh/predicate/attestation-collaterals/v2` is also
accepted. The newer config/runtime
`https://tinfoil.sh/predicate/attestation-collaterals/v3` format is rejected.

The Aptos Move
state machine in `aptos_framework::tinfoil_v3` supplies a fresh 32-byte nonce,
chain time, an allowlisted repository, and the effective roots published by
`aptos_framework::trust_observation`: `intel_sgx_root_ca` (DER) and
`sigstore_trusted_root` (JSON). A missing, malformed, or changed root fails
verification. Production verification does not use bundled fallback roots or
network requests.

## Verified scope

`verify_v3` checks the strict envelope and nonce; Intel DCAP quote, PCS
collateral, TCB and REPORTDATA; Sigstore code provenance against the trusted
repository; RTMR1/RTMR2; code freshness; signed platform endorsements and
host/TDX policy against PPID, MRTD, RTMR0 and TCB; platform freshness; and
RTMR3 pinned to zero. It returns only authenticated TLS/HPKE keys, release
identity, measurements and TCB status.

Current Tinfoil collateral FormatV3 follows a different config and runtime
reference flow. It needs separately trusted config and freshness signing keys,
Rekor v2 inclusion proofs, and RFC 3161 timestamps in addition to the Intel
and Sigstore roots. Those checks and the corresponding on-chain stages are not
implemented here. The [official format registry](https://github.com/tinfoilsh/tinfoil-go/blob/main/verify/document/collateral/collateral.go)
and [verifier dispatch](https://github.com/tinfoilsh/tinfoil-go/blob/main/verify/verifier.go)
define the distinction.

**GPU trust boundary:** the verifier checks the signed VM shape against the
GPU/NVSwitch topology and nonce in the CPU-quote-bound device section. The
measured [Tinfoil guest boot program](https://github.com/tinfoilsh/cvmimage/blob/ad1a411ed483ac249237bbaf11376dd1c2e9cc30/tinfoil/cmd/boot/gpuattest.go#L189-L249) performs local NVIDIA appraisal before it
enables GPUs. This crate does not itself verify NVIDIA certificate chains,
signatures, RIMs, or revocation. The returned GPU count means a verified guest
release reported and bound that device evidence, not independent on-chain GPU
attestation. A consumer must approve the guest release that performs the boot
check. Strict independent hardware appraisal would require more signed device
collateral in the report and separately observed NVIDIA device and RIM roots.

## Staged on-chain use

`split_stage_fragments` can produce five fragments, each retaining the same
challenge, quote and endorsed crypto/device sections while carrying only one
collateral entry. The Move state machine uploads a fragment in chunks and
invokes these native stages in order:

1. `verify_quote_stage` authenticates the quote with the Intel root and stores
   REPORTDATA, quote hash, TLS/HPKE keys, TDX measurements, TCB and PPID.
2. `verify_code_stage` authenticates the code bundle with the Sigstore root,
   matches RTMR1/RTMR2 and stores the release artifact and VM shape.
3. `verify_code_freshness_stage` authenticates a separate witness for that
   release artifact at the pinned proof time.
4. `verify_platform_stage` authenticates platform endorsements, appraises the
   quoted host and TDX module, and resolves MRTD/RTMR0 for the signed shape.
5. `verify_platform_freshness_stage` authenticates the policy artifact's
   independent freshness witness.

Every later stage reparses its fragment, checks the same nonce, and compares
both REPORTDATA and the exact quote hash with stage 1. Move stores stage
outputs in private request state, pins trust-root hashes and proof time across
transactions, and publishes claims only after all five stages succeed.

## Verification status

The test suite has positive verification of a genuine Intel-signed TDX quote
with PCS collateral, a genuine Rekor bundle with the supplied Sigstore root,
and negative tests for changed roots, stage commitments, envelope fields and
measurements, including a test that explicitly rejects collateral FormatV3.
**There is no complete positive fixture containing one valid v3 envelope with
legacy V2 code, platform and both freshness Sigstore bundles.** Full
end-to-end on-chain verification, including transaction gas cost, remains
unproven until such a fixed-time fixture is tested.

Source was adapted from the Apache-2.0 `cvm-attestation` and `cvm-sigstore`
crates in `aptos-labs/atlas-main`, which port Tinfoil's v3 Go verifier.
Attribution and license details are in `NOTICE` and `LICENSE`. Intel DCAP
verification uses vendored `dcap-qvl` 0.6.1 (MIT) with the offline native
feature set; its manifest changes are recorded in `third_party/dcap-qvl/NOTICE`.
