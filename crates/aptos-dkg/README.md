# aptos-dkg

DKG (distributed key generation) and related crypto: PVSS schemes, weighted VUFs, range proofs, polynomial commitments, and supporting primitives. PKs in G₂, SKs in G₁

**PVSS:** `das` (weighted and unweighted); `chunky` (with and without pairing, always weighted).  
**Weighted VUF:** Pinkas WVUF.

---

## Crate layout

| Area | Path | Description |
|------|------|-------------|
| PVSS | `src/pvss/` | Secret-sharing schemes and shared machinery |
| Chunky | `src/pvss/chunky/` | Weighted non-malleable PVSS (see below) |
| Das | `src/pvss/das/` | Das PVSS (unweighted and weighted) |
| Signed | `src/pvss/signed/` | Generic signing of PVSS transcripts |
| Traits | `src/pvss/traits/` | Transcript, encryption, config traits |
| Weighting | `src/pvss/weighted/` | Generic weighting (not used and may not be safe for some schemes) |
| Range proofs | `src/range_proofs/` | DeKART (univariate / multivariate), used by chunky |
| PCS | `src/pcs/` | Polynomial commitments (hiding KZG, Zeromorph, Shplonked) |
| Sigma protocols | `src/sigma_protocol/` | Σ-protocols for homomorphisms (PVSS, range proofs, PCS) |
| DLOG | `src/dlog/` | Discrete-log helpers (BSGS, tables) |
| Weighted VUF | `src/weighted_vuf/` | Pinkas WVUF and BLS-based WVUF implementations |
| Utils | `src/utils/` | `blstrs` RNG, parallel multi-pairing, test helpers |

Tests: `tests/` (e.g. `pvss.rs`, `sigma_protocol.rs`). Benchmarks: `benches/` (e.g. `pvss.rs`, `serialization.rs`).

---

## Chunky (weighted PVSS)

Chunky is a **weighted, non-malleable PVSS** scheme. Design reference: [Chunky: A Weighted Non-Malleable PVSS](https://alinush.github.io/chunky#chunky-a-weighted-non-malleable-pvss). All implementation is under `src/pvss/chunky/`.

**Transcripts**

- **v1** (`weighted_transcript.rs`): Verifier uses one pairing equation (one G1 MSM, one G2 MSM).
- **v2** (`weighted_transcript_v2.rs`): Verifier uses pairings only indirectly (e.g. in range proof), so might be used with a different range proof over a pairingless curve in the distant future. *Not used in production.*

Public types: `UnsignedWeightedTranscript`, `UnsignedWeightedTranscriptv2`, and signed variants `SignedWeightedTranscript`, `SignedWeightedTranscriptv2` (via `pvss::signed::GenericSigning`).

---

## Sigma protocols (`src/sigma_protocol/`)

Σ-protocols for proving knowledge of preimages under **group homomorphisms**. Used by PVSS (e.g. chunky’s SoK), range proofs (DeKART PoK), and sometimes by PCS components. Supports **Fiat–Shamir** and **batched verification**.

- **`homomorphism/`** — Homomorphism trait (domain → codomain, with normalized codomain for transcripts), plus `fixed_base_msms` (making use of MSM structure to do batch verification) and `tuple` (combining homomorphisms into a larger homomorphism).
- **`proof`** — prover commitment + response. (May be extended to store challenge instead of commitment.)
- **`traits`** — Prover/verifier API, challenge derivation, and batched MSM checks.

---

## DeKART v2 (`src/range_proofs/dekart_univariate_v2.rs`)

Univariate **DeKART** range proof: proves that committed values lie in a prescribed range. Design: [DeKART](https://alinush.github.io/dekart). This is the **univariate** implementation; it is the one used by **chunky** (both transcript v1 and v2) to prove that chunked ElGamal ciphertexts encrypt values in the correct base-*B* chunks.

- **Building blocks:** Univariate hiding KZG (commitments and opening proof) and a small sigma protocol (`two_term_msm`). 
- **Building blocks:** Contains commitments, scalar openings, the sigma proof, and a hiding KZG opening.
- **Timing:** Optional feature `range_proof_timing_univariate_v2` prints per-phase timing (setup / prove / verify / commit). Usage e.g.:

`RAYON_NUM_THREADS=1 cargo bench --bench range_proof --features range_proof_timing_univariate_v2 -- --nocapture`

or

`RAYON_NUM_THREADS=1 N=1023 L=8 cargo bench -p aptos-dkg --bench range_proof --features range_proof_timing_univariate_v2 -- 'dekart-rs.*verify' --nocapture`

---

## Implementation notes

**Serialization & safety**  
We (mostly) rely on the `aptos-crypto` `SerializeKey` and `DeserializeKey` derives for safety during deserialization.
Specifically, each cryptographic object (e.g., public key, public parameters, etc) must implement `ValidCryptoMaterial` for serialization and `TryFrom` for deserialization when these derives are used.

The G1/G2 group elements in `blstrs` are deserialized safely via calls to `from_[un]compressed` rather than calls to `from_[un]compressed_unchecked` which does not check prime-order subgroup membership.

Some of our structs use $(x, y, z)$ projective coordinates, for faster arithmetic operations.
During serialization, we convert to more succinct $(x, y)$ affine coordinates.

**TODOs**

- Sometimes the `chunky` PVSS code uses `Pairing<ScalarField = Fp<P, N>>`, and sometimes it uses the `Scalar` wrapper. This is a bit inconsistent.
- `accumulator_poly`: replace hard-coded FFT threshold for slow/fast switch; add multi-threading; reduce allocations.
- Consider affine in-memory representation for `blstrs` transcript/VUF to speed up multiexps / multipairings.

---

## Workarounds

### rand_core / rand versions
`aptos_crypto` uses `rand_core 0.5.1` and `rand 0.7.3`; `blstrs` uses `rand_core 0.6`, `arkworks 0.5.0` uses `rand 0.8`. We cannot pass those RNGs into `aptos_crypto` traits. Workaround: generate random seeds with the 0.5.1 RNG from `aptos_crypto`, then derive scalars/points by hashing the seed.

### blstrs quirks


**Size-1 multiexps**

`blstrs 0.7.0` had a bug (originally from `blst`) where size-1 multiexps (sometimes) don't output the correct result: see [this issue](https://github.com/filecoin-project/blstrs/issues/57) opened by Sourav Das.

As a result, some of our 1 out of 1 weighted PVSS tests which did a secret reconstruction via a size-1 multiexp in G2 failed intermittently. (This test was called `weighted_fail` at commit `5cd69cba8908b6676cf4481457aae93850b6245e`; it runs in a loop until it fails; sometimes it doesn't fail; most of the times it does though.)

We patched this by clumsily checking for the input size before calling `blstrs`'s multiexp wrapper.

**$g_1^0$ and $g_2^0$ multiexps can fail**

`test_crypto_g1_multiexp_less_points`. See `test_crypto_g_2_to_zero_multiexp` and `test_crypto_g_1_to_zero_multiexp`.

**Multiexps with more exponents than bases fail**

See `test_crypto_g1_multiexp_less_points`. Instead, they should truncate the exponents to be the size of the bases.

**Support for generics is lacking**

e.g., no multiexp trait on G1, G2 and GT

**Cannot sample GT from random bytes due to unexposed `fp12::Fp12`**


---

## Development

### Cargo flamegraphs

Example: You indicate the benchmark group with `--bench` and then you append part of the benchmark name at the end (e.g., `accumulator_poly/` so as to exclude `accumulator_poly_slow/`)
```
sudo cargo flamegraph --bench 'crypto' -- --bench accumulator_poly/
```

---

## References

- [GJM+21e] *Aggregatable Distributed Key Generation*; Kobi Gurkan, Philipp Jovanovic, Mary Maller, Sarah Meiklejohn, Gilad Stern, Alin Tomescu; ePrint 2021/005; https://eprint.iacr.org/2021/005