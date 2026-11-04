# Assumptions

The hypotheses of the public theorems that no proof discharges. The
artifact audit (`verify`) admits the named axioms below and nothing else;
every `verified` theorem names them under `#print axioms`. The others are
hypotheses the theorems take.

- **`compileFunction_agrees`** (axiom). The denotation of a compiled
  function refines its prophetic big-step meaning, over every family and
  instantiation, for `compileFunctionAt unit handle σ = .ok f`: without a substitution
  `σ` the statement of `compileFunction` and `propheticMeaning`; at type arguments
  holding a mutable reference, over the substituted rows, with the frame's agreement at
  the family they induce (`propheticMeaningAt`). One family. Establishing it: the induction of
  `denote_agrees` against `BigStep.EvalFunction`, one case per construct
  ([`denotation.md`](../designs/denotation.md), "The design").
  Until then the Rust fixtures test it on concrete inputs
  (`verex-rust/VerexRust/Tests/DifferentialCheck.lean`).
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
- **The Rust standard-library contracts** (`verex-rust/lib`; each a `native_f`
  of the theorems reaching it). Trusted and listed; verifying them against the
  generic MIR of their bodies needs the unsafe profile for most of `alloc`.
  Capacity and allocation are outside the model: `Vec::push`, `insert`,
  `extend_from_slice` and `vec!` have no failure of their own, and one that returns
  had room (`old(self).length < MAX_U64`), and a string's UTF-8 bytes fit in a `usize`,
  which `String::as_bytes`, `push` and `push_str` state of its length. For a `Vec`
  of a zero-sized type this is a real assumption and not only a physical one:
  its length can reach `usize::MAX`, where `push` panics "capacity overflow". The
  pointer-width integers' contracts state 64 bits, as the rendering's profile does.
  `Clone` is the identity at the integers, `bool` and `char`, as is `Default` zero or
  `false`, the only shapes with a contract: a shape over a type parameter would trust
  an impl the unit does not see. `abs` panics at the minimum, as the caller's overflow
  checks do in the profile's `panic=abort` debug semantics, like its other arithmetic.
- **The contract of a refused function** (a `native_f` of the theorems reaching it).
  A function whose body the unit cannot state (`function.refused`) is an error of
  its crate's verification, and its callers assume a contract the user gives it.
- **An empty enum is an empty struct** (the export of `Infallible`). This is sound,
  since it only adds a value, but a proof has no fact of an empty enum's
  impossibility: code matching on a value of one is not known to be unreachable.
- **The models of `core`'s iterators** (`@[model] struct`, `verex-rust/lib/core`).
  A model states a private representation. `Rev`, `Enumerate`, `RangeInclusive` and
  `StepBy` hold `core`'s own fields; `slice::Iter` holds `slice`, `pos` and `end` for
  its pointers, the elements left being `slice[pos .. end]`. `next` follows the pinned
  toolchain's source, which leaves `start` and `end` unspecified after exhaustion: it
  wraps the bound and sets `exhausted` only when stepping overflows, and `StepBy`
  over an unsigned `Range` turns `end` into a count, so a toolchain update re-checks
  these. `Enumerate::next` panics when `count` overflows
  (`rustc_inherit_overflow_checks`), which assumes overflow checks are on. A model
  states no destructor, which is the library's word that `core` declares none on
  these types.
- **`NativesTyped`, `NativesShift`** (hypotheses of theorems assuming
  typing). Natives preserve typing and the loan shift.
- **A native declared `-> !` does not return** (inside
  `compileFunction_agrees`, as no hypothesis states it). Nothing follows a
  call returning `!` (`Term.unreachable`, no outcome). A unit-defined callee
  cannot return, as static typing leaves `!` uninhabited. Of a native, only
  `NativesTyped` says so, which the axiom does not take, and
  `NativesTyped` is not discharged.
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
- **A dropped type argument's destructor** (no hypothesis states it; Rust
  functions dropping a type parameter). The argument's destructor has no
  observable effect and does not panic: the runtime and the denotation read the
  drop as removing the value. A call in the unit instantiating such a parameter
  at a type with a destructor has no denotation (`dropsDestructor`,
  `Denote/Compile.lean`), so only a use of the theorem outside the unit relies
  on this. Discharged by M4 of
  [`rust-mir-design.md`](../designs/rust-mir-design.md), where a drop runs the
  destructor.
- **A diverging region is a panic** (no hypothesis states it; Rust functions).
  The import collapses the blocks all of whose paths reach a call that does not
  return into `abort` (`VerexIR/Import/PanicRegions.lean`): what a region computes
  is unobservable under `panic=abort` and a stateless failure. A diverging callee
  that does not panic (`exit`, a loop) and a call inside a region that does not
  return become a panic path, and the callee's contract is not consulted, which
  is sound and incomplete: the path did not return, and the panic is reported or
  has to be permitted. An effect of such a callee that outlives the process is
  not modeled.
- **A callee's abstract contract view**, and the contract of a callee
  whose verification failed or did not run, are hypotheses of the caller's
  theorem; the callee's own failure is reported where it is verified.
- **The decoder of native function values** (planned): assumed inside
  `compileFunction_agrees` until the agreement is proved.
- **`-Zub-checks=no`** (Rust exporter). The exported MIR omits the alignment and null checks
  rustc inserts at a dereference of a raw pointer and at a reference made from one, so
  verification reads no panic of theirs. Under `unsafe=reject` the only such pointers a body
  holds are rustc's elaboration of a `Box` dereference (`BoxFolds`), and a box's pointer is
  aligned and non-null by the type's own invariant, so no check of them can fail. Revisit for
  the unsafe profile, where a check does fail on a wrong pointer and its panic is behavior
  ([`unsafe-pointers.md`](../designs/unsafe-pointers.md)).
