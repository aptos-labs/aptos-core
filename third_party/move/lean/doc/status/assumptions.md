# Assumptions

The hypotheses of the public theorems that no proof discharges. The
artifact audit (`verify`) admits the named axioms below and nothing else;
every `verified` theorem names them under `#print axioms`. The others are
hypotheses the theorems take.

- **`compileFunction_agrees`** (axiom). The denotation of a compiled
  function is equivalent to its prophetic big-step meaning, over every
  family and instantiation. Establishing it: the induction of
  `denote_agrees` against `BigStep.EvalFunction`, one case per construct
  ([`denotation.md`](../designs/denotation.md), "The design").
- **`compileFunction_least_cycle`** (axiom; members of cycles of calls).
  The closed meanings of a cycle's members refine the least fixed point of
  their denotations. Establishing it: a continuity proof over the
  denotation (a meaning's outcomes use finitely many calls), from which
  the closed agreement follows.
- **`bv_decide` certificates** (`_native.bv_decide.ax_*`; functions under
  `pragma bv` only). The native evaluation of the verified checker
  `Std.Tactic.BVDecide.Reflect.verifyBVExpr` on a certificate is `true`;
  trusts the Lean compiler for that evaluation, as the Move Prover trusts
  its SMT solver. A forged certificate is rejected by re-evaluating the
  checker (`Check/Scalars/ForgedCertificate`).
- **`GlobalsPreserved`** (hypothesis). Runs preserve the typing of global
  memory. Establishing it: the remaining global-preservation obligations
  of [`static-memory.md`](../designs/static-memory.md) and the static
  typing of [`static-typing.md`](../historical/static-typing.md).
- **`FailuresIntegral`** (hypothesis). The unit's runs fail with integer
  payloads. Validation does not constrain a throw's operands; establishing
  it would take a validation rule that they are integers, a lemma over the
  big-step semantics, and the natives' integral failures as an obligation
  of the semantics registry. Kept as an assumption deliberately: failures
  are to carry more than integers later
  ([`higher-order-functions.md`](../designs/higher-order-functions.md),
  "Function values as native data").
- **`Terminating`** (hypothesis; functions whose proof reads `result_of`
  of a known function value). Runs of the unit's functions end, as the
  Move Prover assumes of every function.
- **`native_f`** (hypothesis per native reached). Each native satisfies
  its contract; a caller of a verified function assumes that function's
  natives too.
- **`NativesTyped`, `NativesShift`** (hypotheses of theorems assuming
  typing). Natives preserve typing and the loan shift.
- **`RunsAgree`** (hypothesis of theorems assuming typing). Runs of the
  function meaning from starts with the same globals and table handles
  agree up to loan renaming: a run depends on its maps' abstract values
  alone. Holds of a unit whose owners' Move bodies refine their roles'
  contracts up to normalization
  ([`intrinsic-maps.md`](../designs/intrinsic-maps.md)).
- **`OwnersRepresentable`** (same). Every value a closed native type admits
  has a typed runtime representation normalizing to it. Both map
  hypotheses are to be discharged by a proof for a unit whose owners are
  all tables, where normalization is the identity.
- **`AssumptionsHold`** (hypothesis of functions with in-body `assume`).
  Every outcome of a run is an outcome of the reading in which the runs
  pass each assumption only where its condition holds. A caller using the
  theorem takes the same hypothesis; one inlining the function takes none.
- **A callee's abstract contract view**, and the contract of a callee
  whose verification failed or did not run, are hypotheses of the caller's
  theorem; the callee's own failure is reported where it is verified.
- **The decoder of native function values** (planned): assumed inside
  `compileFunction_agrees` until the agreement is proved.
