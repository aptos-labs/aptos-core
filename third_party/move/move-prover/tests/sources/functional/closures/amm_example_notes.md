# AMM example — "pollution" investigation notes

Companion to `amm_example.move`. Records what we tried to explain and
eliminate the timeout on `create_compliant_fee_pool` when the procedure
is verified as part of the full module rather than in isolation.

## Symptom

`create_compliant_fee_pool` verifies in ~1.2 s in isolation but times
out at ~40 s when its VC sits alongside the rest of the AMM module.
Same solver, same VC — something in the full-module context is slowing
Z3 down. We labeled this "pollution."

## What we tried

### 1. Encoding BP evaluators as uninterpreted + per-variant axioms (landed)

Replaced the original `function {:inline}` BP evaluator (with
`if f is V then ... else` dispatch) with uninterpreted
`$ensures_of'F'` / `$aborts_of'F'` / `$requires_of'F'` plus per-variant
axioms:

- Closures: trigger `{$ensures_of(…, $closure'V'_mask(c0..ck), …)}`,
  constructor-keyed, no guard.
- Fun-param: trigger `{$ensures_of(…, $param'F$p'(), …)}`, nullary
  ground constructor.
- Struct-field: guarded form `{$ensures_of(…, f, …)}` with
  `(f is $struct_field'…') ==>`.

Motivation: stop inlining every variant's spec body at every evaluator
use-site. **Necessary and kept.** Solved cross-variant term leakage in
the `swap` VC.

### 2. `--split-vcs-by-assert` flag (landed as opt-in)

Emit one VC per assert instead of one monolithic VC per procedure.
Cuts full-module time 85 s → 35 s single core. **Symptom mitigation,
not a root-cause fix.**

### 3. `--error-limit` flag (landed)

Makes diagnostic behavior predictable under split-VCs.

### 4. Hypothesis: fun-param variant axioms leak because they're ground

`$param'F$p'()` is nullary, so Z3 materializes its axiom body even in
VCs that never use it. Plausible but never cleanly proven — seed
sweeps confounded the measurement.

### 5. Hypothesis: datatype size matters (5-variant vs 2-variant BPL)

Shrank the fun-type datatype to only variants reachable from the
procedure under test. **Seed sweep (10 seeds) showed 5/10 timeouts on
both 2-variant and 5-variant BPLs.** Datatype size is not the dominant
factor. This invalidated the "nonlinear CP axioms from unrelated
variants fire and blow up" narrative as stated.

### 6. `smt.random_seed` as confounder — major finding

We had been running manual Boogie invocations without
`smt.random_seed=1`, the flag the prover actually passes. Timings were
wildly inconsistent across runs. Many earlier "pollution" claims were
seed noise. After fixing this, the real picture emerged: same VC, same
encoding, timeout rate depends heavily on the seed — Z3's nonlinear
arithmetic solver takes very different search paths per seed.

### 7. Prototype: un-inline closure BPs (reverted)

Closure `$bp_*'F'` functions were still `function {:inline}` with full
spec body, asymmetric to the fun-param/struct-field uninterpreted +
axiom form. Hypothesis: inlining forces the nonlinear CP-preservation
body into the E-graph everywhere, driving NLA search. Converted
`$bp_requires_of'F'`, `$bp_aborts_of'F'` to uninterpreted + trigger-
keyed axioms; `$bp_ensures_of'F'` to a thin wrapper over uninterpreted
`$bp_ensures_of_result'F'` with a result-term-keyed definitional
axiom. **Seed sweep (10 seeds): new encoding 8/10 timeouts, old 5/10.**
Made it worse. Reverted.

## Where we stand

- The "pollution = cross-variant constructor dispatch" story was
  correct early on and fixed by (1).
- The remaining timeout is **not** obviously explained by inlining,
  datatype size, or variant-axiom leakage. It's dominated by Z3's
  nonlinear search cost, whose cost function is seed-sensitive.
- `--split-vcs-by-assert` reliably masks it at the cost of a flag.

## Open questions

- Is the remaining slowdown really QI/term-database related at all, or
  is it Z3's nonlinear tactic cost that happens to correlate with
  module size via unrelated path effects (e.g., more hypotheses in
  scope)?
- Would we benefit from QI tracing on the full-module vs isolated VCs
  to measure actual instantiation counts on suspected culprits, rather
  than reasoning from encoding shape?
