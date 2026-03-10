# Sumcheck variable order: LSB vs MSB

## Summary

- **aptos-dkg** sumcheck uses **LSB-first** (variable 0 = least significant bit of the hypercube index).
- **aptos-crypto** sumcheck (Product4, BooleanityEq) uses **MSB-first** (variable 0 = most significant bit).
- Both are correct *within* their own convention; bugs arise when **mixing** conventions (e.g. feeding LSB-ordered MLEs into an MSB sumcheck without reindexing).

This crate provides:
- **MSB-first**: `BooleanityEqSumcheckProver` / `BooleanityEqSumcheckVerifier`, `Product4SumcheckProver` / `Product4SumcheckVerifier` (original Jolt-style binding).
- **LSB-first**: `BooleanityEqSumcheckProverLSB` / `BooleanityEqSumcheckVerifierLSB` (matches aptos-dkg and ark_poly).

---

## Why variable order matters

In sumcheck we have a multilinear polynomial over the boolean hypercube `{0,1}^n`. The **variable order** is the assignment of which coordinate corresponds to which bit of the index when we identify the hypercube with indices `0..2^n`.

- **LSB-first**: index `g` has bits `g_0, g_1, ..., g_{n-1}` with `g_i = (g >> i) & 1`. So variable 0 = LSB, variable 1 = next bit, ..., variable (n-1) = MSB.  
  - Folding: pair indices `(2b, 2b+1)` → first round binds the LSB.
- **MSB-first**: index `g` has bits so that variable 0 = MSB (e.g. `(g >> (n-1-i)) & 1`).  
  - Folding: pair indices `(g, g + half)` → first round binds the MSB.

The **same** polynomial (same hypercube sum) can be run with either order, but:
- The **round polynomials** and **verifier challenges** are different (different binding order).
- The **evaluation point** `r = (r_0,...,r_{n-1})` has different meaning: in LSB-first, `r_0` is the value for the LSB variable; in MSB-first, `r_0` is the value for the MSB variable.

So proofs are **not** interchangeable between the two orders.

---

## Why this is an issue in implementations

1. **Interop with ark_poly / aptos-dkg**  
   `ark_poly::DenseMultilinearExtension::from_evaluations_vec` stores evaluations in natural index order; the first dimension that varies (0→1, 2→3, …) is the **LSB**. So any code that builds MLEs from ark_poly and then runs sumcheck must use an LSB-first sumcheck (or reindex the evals) so that “variable 0” in the polynomial matches “round 1” in the sumcheck.

2. **Porting between codebases**  
   If you port a protocol from a codebase that uses LSB (e.g. aptos-dkg) to one that uses MSB (e.g. Jolt/aptos-crypto Product4), you must either:
   - Reindex the evaluation arrays (e.g. bit-reverse) so that the MSB-first sumcheck sees the same logical polynomial, or
   - Introduce an LSB-first sumcheck path (as in `BooleanityEqSumcheckProverLSB`) and keep your MLEs in LSB order.

3. **Verifier formulas without evals**  
   When the verifier does not have the full MLE evals (e.g. in Dekart it only has `y_js`, `y_g`, `c_zc` and a closed formula), the formula must use the **same** variable order as the sumcheck. Using `r[0]` as “MSB” in a formula while the sumcheck bound LSB first causes silent wrong results.

4. **Tests**  
   Tests that only run “prove with evals → verify with same evals” validate internal consistency of one convention. They do **not** catch a mismatch between an external formula and the binding order. Cross-checking “formula value at r” vs “binding result at r” (with the same evals) helps.

---

## Performance

The LSB version uses the same number of rounds and the same degree; only the binding step differs:

- **MSB binding**: `Z[0..half]` and `Z[half..]` are adjacent in memory → good cache behavior.
- **LSB binding**: pairs `Z[2i]`, `Z[2i+1]` → one pass with stride 2, still linear scan, similar cost. The implementation uses a single allocation for the folded vector.

So the LSB-first BooleanityEq prover/verifier are intended to be **performant** and on par with the MSB version, while matching aptos-dkg and ark_poly ordering.
