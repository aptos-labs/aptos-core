# Test fixtures

## `tdx_quote.bin`, `tdx_quote_collateral.json`

A genuine, Intel-signed Intel TDX quote and its matching DCAP verification
collateral (TCB info, QE identity, CRLs).

Vendored verbatim from the [`dcap-qvl`](https://crates.io/crates/dcap-qvl)
crate's `sample/` directory (v0.6.1), which is MIT licensed,
Copyright (c) Phala Network.

Used by `src/attestation/tdx.rs` and `tests/attestation.rs`. Unlike the
synthetic fixtures in `testdata.rs` — which are *structurally* genuine but carry
no valid signature — this quote really was signed by Intel, so it exercises the
full DCAP chain up to the pinned Intel SGX Root CA.

Two properties shape how tests use it:

1. **Fixed `REPORTDATA`.** Nothing can make a genuine Intel signature commit to
   a nonce we choose, so a positive test passes the quote's own report data in
   as the expected challenge binding
   (`testdata::tdx_quote_report_data()`).
2. **Expired collateral.** The collateral's validity window closed on
   2025-07-19, so verification is pinned to
   `testdata::TDX_COLLATERAL_VALID_AT` (2025-07-01). Using the real clock would
   fail on collateral expiry rather than on whatever the test means to assert —
   and `rejects_expired_collateral` covers the expiry path deliberately.

`tdx_quote_collateral.json` is kept for provenance: it is the upstream
`dcap-qvl` sample and the source the `intel-pcs/v1` fixture below was derived
from. The vault itself no longer parses this shape.

## `tdx_pcs_collateral.json`

The same genuine Intel collateral, re-expressed in the `intel-pcs/v1` wire
format that a real `cvmimage` document actually carries: Intel PCS responses
captured verbatim, keyed by request URL, with the issuer chains in response
headers.

**Read this before trusting it.** The *payloads* are genuine — the TCB info,
QE identity, CRLs, issuer chains and Intel's signatures over them are the
unmodified bytes from the `dcap-qvl` sample, so verification really does run
Intel's signature chain up to the pinned SGX Root CA. But the *envelope* around
them is reconstructed, not captured:

- The request URLs are plausible reconstructions (Intel's documented v4
  endpoints, with the `fmspc` recovered from the TCB info). No live PCS
  request produced them.
- Response headers carry only the issuer chains, percent-encoded as Intel
  encodes them. A real capture would carry the full header set.
- The root CA CRL is filed under Intel's `.der` distribution-point URL, which
  is where `dcap-qvl` sources it when a PCCS mirror is absent.

The signed documents (`tcbInfo`, `enclaveIdentity`) are spliced in **byte for
byte**. They are deliberately not re-serialized from parsed JSON: Intel's
signature covers the exact bytes, and re-encoding would reorder members and
invalidate it. `authenticates_a_genuine_intel_signed_quote` is what proves the
splice is correct — if the bytes drifted, Intel's signature would not verify.

Regenerate it from `tdx_quote_collateral.json` rather than editing it by hand;
hand-editing the base64 bodies will silently break signature verification.

This fixture closes the gap where the vault expected a collateral shape no real
document carries, but it is still not a substitute for testing against a
genuine end-to-end `cvmimage` document.
