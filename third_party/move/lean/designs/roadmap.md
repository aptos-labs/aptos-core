# Roadmap and test ledger

Status: 2026-09-28. This is the one place for project status: what is
still open across the designs, in priority order, and the ledger of the
acceptance fixtures that measures it ([Tests](#tests)). Each linked design
stays authoritative for its own scope and rationale; when a status
changes, update it here, not in the design. Done work is named in one line
per track so the open items read in context; the history lives in
[`historical/`](historical/).

The design documents inside `v0/` (`move/`, `move-model/`, `transpiler/`)
describe the deprecated stack. Nothing is scheduled from them; they are
mined for solutions, and all their verification fixtures are ported to the
ledger below.

## 1. Verification of validated LIR

Design: [`denotation.md`](denotation.md), with the reference model of
[`prophetic-references.md`](prophetic-references.md).

Done: one `denote` of validated LIR into `Spec`, compiled per target and
certified by the kernel (D0–D6: control, loops with invariants, references
as prophecies, storage and resource invariants, vectors, Move generics,
recursion and cycles of calls). Calls follow the Move Prover: callees are
inlined unless native, opaque, or on a cycle; contracts have `[abstract]`
and `[concrete]` views; `pragma bv` restates bit arithmetic over bit
vectors for `bv_decide`. Every Check fixture passes ([Tests](#tests)).

Open:

- **The agreement theorem.** `compileFunction_agrees` and
  `compileFunction_least_cycle` are axioms by decision; proving them by
  induction over `compileFunction` removes the last project-specific
  assumptions besides the natively checked `bv_decide` certificates.
- **Quantifier automation.** Specification quantifiers over types, ranges,
  and vectors (by index, as in the Move Prover) translate. A leaf whose
  deciders fail on a quantified or conditional goal is retried at an
  arbitrary instance, within its own heartbeat budget, and a lookup after
  a vector write is resolved where omega tells the positions apart or
  equal: frames over a write verify (`bit_vector::set`, `unset`). A goal
  one position past a quantified hypothesis's range splits on the open
  range premise and is decided at the bound, which extends a range
  invariant by one iteration, in an authored proof and in automatic mode
  alike (the instance deciders see a goal whose binders the loop step
  introduced, and see through `Obligation` markers). A lookup at a
  position the context does not tell apart from a written one splits on
  the positions being equal, reads the written element or the array
  before the write in each case, and pins the variable the read position
  offsets to the written one (`leaner_denote_split_write`); a lookup into
  an array with an element removed or inserted reads the array before
  where omega places the position (`leaner_denote_lookups_after_writes`).
  A quantified equation among the hypotheses rewrites the goal once at an
  instance whose premises omega proves (`leaner_denote_rewrite_instance`;
  bounded, unlike rewriting with the equations as rules, which cycles).
  An existential is witnessed by the hypothesis stating its body, by a
  context variable, by a written position, or by a position a lookup fact
  reads, its premises decided by the context (`leaner_denote_witness`,
  in the cheap deciders and the pipeline).
  The result of a vector search (`contains`, `index_of`) is split into
  "nothing" and "a position", the latter characterized by its bound, the
  element there, and every element before it (`leaner_denote_split_search`,
  `findIndex?_eq_some_iff`); a lookup into an array with an element
  removed or inserted splits on which side of that position it reads
  (`leaner_denote_split_write`, at integer positions); an in-bounds lookup
  the context states nothing about names its element
  (`leaner_denote_name_lookups`); and, last, the hypotheses quantified
  over positions that read an array are instantiated at every position
  the context reads, bounds closed by a hypothesis or omega, the equations
  left as premises, and the instances rewritten by the context
  (`leaner_denote_instantiate_positions`, at most 64 instances per
  hypothesis). Membership and search results normalize to one form, the
  position of the element (`mem_iff_exists_int_index`,
  `forall_imp_ne_iff`). `acl` verifies with this (its injectivity
  invariant over two indices, through a push and a removal). Open: the
  instantiation is the costly step (the rewriting of each instance by the
  context, a third of a million heartbeats each), and it fires at every
  read position rather than at the patterns of the hypothesis.
- **Not carried by `denote`:** signed bitwise operations, `bytes` and
  fixed-length vectors, the generic residuals of D3 (aliasing instances), unspecified pure callees used as summaries, references
  inside an aggregate, and nested destructuring patterns.
- **Cost** (measurement notes, rejected experiments, and leads:
  [`perf-notes.md`](perf-notes.md)). Across the Check fixtures the closer spends 2.5G heartbeats:
  leaves 0.8G, loop exits 0.5G, binds 0.45G. A step normalizes only the
  structure it creates: the parts a rule rearranges, the values a
  continuation is applied to, and the conditions a folded continuation
  passes on are normal already, and the continuation a bind rule builds is
  normalized where it is applied (binds −55%, exits −18%). Leaves share
  their bounds: a bound asserted on a goal is not asserted again on the
  goals it leaves, and a saturation round that only reorders hypotheses
  has reached the fixed point (leaves −17%). Two costs now lead and
  heartbeats do not show the first:
  - kernel checking, the largest part of the slowest fixtures (31s of
    `ReturnedMutRefs`, 16s of `VectorOperations`, 11s of `OrderedMap`).
    The denotation rewrites by its equations, which the kernel checks by
    instantiation instead of reducing `Term.denote`'s 61-way recursion
    (kernel −14%, closer −20% across the fixtures). The preparation
    certificates, which the kernel checks by evaluating the validation
    passes, avoid array indexing, which the kernel pays for by walking a
    list: loan marking iterates lists, an arena index is built in one
    traversal, and the erasure analysis reads places and types through
    arena indexes (kernel −19%, then `ReturnedMutRefs` 47s to 31s;
    `marked_eq` 19s to 2s, the erasure chunks 10s to 2s). What remains: a
    `compiled_eq` per function (up to 1s), which does not come from
    deriving native types (a per-view table of them saves 1% of its
    unfolds and costs more to certify than it saves), and the proofs
    themselves;
  - the price of a `simp` step: every lookup reduces the reducible carriers
    in implicit type arguments (`NTy.carrier`, `HEnv`) by smart unfolding,
    about half of each normalization.

  A leaf no decider closes costs most: each saturation round over a large
  context costs 8–18M heartbeats. A leaf therefore tries the prepared
  deciders (`leaner_denote_decide_prepared`: one normalization pass, then
  the instance, lookup, and write-split deciders) before the saturating
  pipeline, where its context holds a quantified hypothesis or a written
  vector; the normalization pass skips the hypotheses a pass of this
  target already left normal, up to the names of their free variables
  (`normalHypotheses`), and a loop step reduces the entry row's
  projections structurally before normalizing. Every target runs under a
  budget (`leaner.verifyHeartbeats`, 1.5G heartbeats by default: about a
  minute), so a target that does not verify fails within it, and the
  failure asks for a proof in the source's proof file or a larger budget
  (`pragma heartbeats = N`; see `source-verification.md`, Proofs;
  2026-09-28). The costliest standard-library targets verify within the
  default: `bit_vector::shift_left_for_verification_only` (284M),
  `features::change_feature_flags_for_next_epoch` (215M),
  `change_feature_flags_internal` (391M), and `apply_diff` (714M); their
  cost is in the inlined `set`'s loop and the many-slot rows of the
  `for_each` expansion, and it grew during the acl round (see
  `perf-notes.md`, the open lead).
- **Closed (2026-09-29): frame-instantiation certificates on large units.**
  A generic call at the runtime family is certified by the kernel; read
  through the namespace's view (2026-09-29) the certificate no longer
  stalled but cost 25–30 s each on `aptos-stdlib`'s crypto modules
  (`ristretto255::multi_scalar_mul`: three of them). Now a witness
  certificate: a key map over the callee namespace's types decided once
  per namespace, a witness per type checked with logarithmic reads
  (`denotation.md`, "Instantiation certificates by witness";
  `perf-notes.md`, kernel calibration). The three certificates of
  `multi_scalar_mul` and its closer cost 17 s together.
- **Closed (2026-09-29): the whole-package `prove --lean` of `aptos-stdlib`
  exceeded 30 minutes** while its module extracts summed to 20. The cause
  was the namespace command's comment scanner, quadratic in the file and
  run twice per module; it is linear now and the package's module commands
  elaborate in under a second each (`perf-notes.md`, "Stage log").
- **Closed (2026-09-30): importing a large module stalled.** A module
  importing `ordered_map` did not finish in 5 minutes: the dependency's
  interface spelling walked the namespace per printed name. It renders in
  6 ms now, generic frame certificates are shared per unit, and the bound
  collectors walk shared terms once (`perf-notes.md`, "Modules,
  generic frames, and shared terms").
- **Open (2026-09-29): natives without a specification.** The verifier now
  mirrors the Prover: a native with a Move specification uses it, one
  without uses the model the Boogie prelude gives it, registered by the
  Move profile (`LeanerMove/Natives.lean`: `hash::sha2_256`, `sha3_256` as
  uninterpreted 32-byte functions), and any other is rejected. Still
  unmodelled and needed by `aptos-stdlib`: the `table` natives (`add_box`,
  `remove_box`, `borrow_box`, …; the prelude's `table_module`), on which
  `big_vector`, `smart_table`, `smart_vector`, `storage_slots_allocator`
  and `pool_u64*` wait, together with an `Inhabited` twin for structs
  holding a table; and the hashes' injectivity axiom.
- **Open (2026-09-29): discarded statement values.** The exchange encodes a
  Move sequence's last expression as the block's value even where the Move
  source discards it (`v.remove(index);` in `capability::remove_element`,
  `pool_u64::deduct_shares`), so LeanerLang rejects the `if` without `else`
  whose branch then has a non-unit value (LEANER-TYPE-MISMATCH). Fix at the
  frontend's sequence encoding (`Frontend/LIR/Encode.lean`), keeping the
  unit the source states.
- **Open (2026-09-29): `math128::sqrt` exceeds its budget in normalization**
  (`whnf`, before any decision; the skip proof times out as well) —
  a normalizer gap on its shifts and divisions of `u128`, to find with a
  reduced module. `ristretto255::scalar_invert` verifies in 50s.
- **Closed (2026-09-29): opaque specification functions under generic
  callees.** `Contract.opaqueSpec` keyed a call by its qualified name with
  the type arguments rendered as text, so a generic native's contract named
  `<parameter 0, …>` where its caller named the concrete types. The type
  arguments are now native types under the contract's family
  (`Skolems.type`, `NTy.substWith`), which the instantiation resolves
  (`perf-notes.md`, "multi_scalar_mul"; fixture
  `Check/Calls/OpaqueGeneric`). `ristretto255` verifies completely.
  With the view certificate `ristretto255` completes in 399s (25 targets;
  `scalar_invert` alone 514M heartbeats). Its targets returning
  `Option<Scalar>` through the trusted `option::some`/`none`
  (`new_scalar_from_bytes` and two more) had failed with a kernel
  application type mismatch: the closer's encoding facts took their
  `Skolems` instance from the context (the runtime family) rather than
  from the hypothesis (the callee's instantiated family), which the
  elaborator's unifier let pass and the kernel refused; and the transport
  of a trusted callee's value into the caller's carriers
  (`NTy.ofSkolem`, `variantCarrier.ofSkolem`) had no reduction lemmas, so a
  variant name or field read through it was undecidable. Both fixed
  (2026-09-29; `Check/Calls/TrustedGeneric.lean`). Also open there: the encoded-vector-literal equality
  (`scalar_is_zero`), the byte-vector `spec fun` shape of every crypto
  module.

## 2. Verification across modules

Design: [`relocatable-namespaces.md`](relocatable-namespaces.md).

Done: R1 (relocation: namespaces carry their own tables and validation
certificates, and a unit links them without re-validating) and R2
(a `leaner module` links the registered modules it uses; verification
inlines across modules; a clause is reported in the file that authored
it).

Open:

- **R3: proof reuse.** An opaque callee of another module is verified again
  in every importer, and the unit certificates of every linked module are
  checked again (7.5 s for a client of `ordered_map`). Reusing its theorem
  needs the semantics invariant under consistent renumbering, or semantics
  over namespace-local identities; either makes a package's verification
  cost linear.
- **R4: shipped namespaces.** The exchange format carries relocatable
  namespaces, so a frontend can ship a precompiled package.

## 3. The Move standard library as the verification corpus

Design: [`source-verification.md`](source-verification.md).

The package verifies module by module; the baseline
`leaner-e2e-tests/LeanerE2ETests/SourceVerify/MoveStdlib.exp` lists what
does not verify yet. Every clause of every module verifies within the
default budget (2026-09-28). Open:

- `features`: `is_enabled`, `on_new_epoch`, and
  `change_feature_flags_for_next_epoch` verify; `apply_diff` (its
  `for_each` expansion reads locals under `old` inside state anchors,
  which a loop invariant now reads at the loop's entry) and
  `change_feature_flags_internal` verify above the budget (section 1);
  three errors reported at the module remain to be attributed to their
  declarations: "maximum recursion depth", which a larger `maxRecDepth`
  turns into `isDefEq` timeouts at the default 200M heartbeats, so they
  arise outside a target's budgeted proof, in the certificates the kernel
  checks (section 1, cost).
- `bit_vector`: everything verifies but
  `shift_left_for_verification_only`, above the budget (section 1).
- `acl` verifies (2026-09-28). The exchange maps `vector::contains`,
  `index_of`, and `remove` to the primitives the denotation states
  directly, as the Move Prover treats them, so their loops are not
  inlined and the search result is characterized rather than unrolled.
- A residual obligation that no clause marker locates is reported at the
  module header, not at the function.
- **Verification in the Move CLI.** `move prove --lean` (and `aptos move
  prove --lean`) verifies a package with Leaner instead of Boogie: the
  framework's `ProverOptions::prove` exports the package's typed AST and
  runs `leaner-move verify --export` on it, so a framework package is
  verified in place with the usual tool set, from the CLI and from the
  framework's prover tests alike (2026-09-28; see
  [`source-verification.md`](source-verification.md)), and
  `move_stdlib_lean_prover_tests` verifies the standard library in place.
  A function the automatic verification does not establish within its
  budget is proved in the LeanerLang proof file beside its Move file
  (`foo.proof.lean`), whose `verify f by …` items and lemmas the driver
  splices into the module; the failure message names the file
  (2026-09-28). Open: the e2e
  baseline still verifies the checked-in copy under
  `MoveToLeanerLang/MoveStdlib`, which drifts from the framework (`cmp`,
  `signer`); the copy is to serve only the Move-to-LeanerLang baselines.

## 4. Shared checking

Design: [`lir-design.md`](lir-design.md), Phases 1–2.

Done: a single-run `validate` with authoritative name resolution,
unification typing of executable and specification bodies, and the
initialization and borrow analyses run once with certificates on
`ValidatedUnit`.

Open:

- The checked trait-selection evidence table (also blocking
  [`rust-mir-design.md`](rust-mir-design.md)).
- The certification pass that turns `prepare*` into pure mode filters.
- Borrow checking: general non-lexical loan death, dependency and unknown
  aggregate alias propagation, nominal variance, use-site region solving.
- Strict external JSON compatibility of RawUnit and the full staged API.

## 5. Runtime semantics

Design: [`elaboration-design.md`](elaboration-design.md) (runtime model,
big-step semantics, interpreter).

Open: the metatheory of the big-step semantics — determinism,
completeness of the interpreter up to fuel, preservation, and no-stuck for
prepared invocations.

## 6. MonoVM differential harness

Design: [`monovm-link-design.md`](monovm-link-design.md).

Done: M0–M3 and M4a (the linked adapter, marshalling, the three-way
differential driver, composite values); M4b in part.

Open: resources and events, and mutable-reference out-parameters (M5).

## 7. Rust frontend

Design: [`rust-mir-design.md`](rust-mir-design.md).

Open: the M0 gate (blocked upstream, below), then M1 mapper completion and
Charon differential tests, M1.5 prebuilt exporter distribution, M2
contracts and `verify` for Rust through `denote`, M3 regions over the
prophetic model, M4 drops and panics, M5 the unsafe profile
([`unsafe-pointers.md`](unsafe-pointers.md), a proposal scheduled here),
M6 library models, M7 alignment proofs.

## 8. Surface and backends

Designs: [`leaner-lang.md`](leaner-lang.md) and
[`lir-design.md`](lir-design.md) Phases 4, 6, 7.

Done: validated LIR lowers to XIR, and compiler-v2 compiles `.lean`
sources through it (2026-09-25).

Open: the LeanerLang forms still stated as target language (`core.*`,
`spec.*`, extension and dependency forms); retiring the NSIR bridge;
corpus-wide semantic alpha-equivalence and source maps; a
compiler-correctness theorem connecting source verification to the
emitted bytecode.

## Tests

### Gates

| Gate | Result |
|---|---|
| `leaner-ir` build and `lake test`, with the `DenotePerformance` and `CompositionPerformance` cost gates | PASS |
| `leaner-move`, `leaner-rust` builds and tests | PASS |
| `leaner-e2e-tests` `lake test`: suites `move`, `rust`, `verify`, `check`, `monodiff` | PASS |
| Check fixtures | 76 of 76 pass exactly |
| Move-to-LeanerLang corpus round trip | every module is a typed fixed point |

The `verify` suite verifies the staged Move standard library; its baseline
records what does not verify yet (section 3).

### Check ledger

Each name is a `.lean` file under
[`leaner-e2e-tests/LeanerE2ETests/Check/<Folder>/`](../leaner-e2e-tests/LeanerE2ETests/Check/).
Every file passes: its whole output matches the adjacent `.exp` at the
driver's caps (180k verification heartbeats per target). Notes name what
a file covers beyond its title: intended negatives, authored proofs, and
files without `verify` targets.

| Folder | Files | Notes |
|---|---|---|
| Scalars | `Addresses`, `Arithmetic`, `BitVectors`, `Integers`, `Literals`, `Order`, `Signed`, `SpecLogicalArithmetic` | `BitVectors`: `pragma bv`; a wrong postcondition and the same leaf without the pragma fail (intended). `Order`: an authored proof of transitivity. |
| Structs | `Abilities`, `Invariants`, `PositionalStructs`, `Tuples` | `Abilities` has no `verify` targets. |
| Enums | `EnumPatterns`, `EnumPayloads`, `EnumRefContracts`, `EnumRefs`, `Enums` | `EnumRefs` is execution only. |
| Vectors | `VectorBounds`, `VectorOperations`, `Vectors` | A target whose guards stay undecided exhausts heartbeats rather than failing fast. |
| Control | `Aborts`, `ControlForms`, `LoopControlErrors`, `LoopInvariantErrors`, `LoopInvariants`, `Loops`, `LoopVerification` | `Aborts`: a false contract (intended). `LoopInvariantErrors`: an invariant not established at entry and at an iteration. |
| References | `BorrowCertificates`, `BorrowChecker`, `BorrowErrors`, `BorrowGlobalErrors`, `CorePrimitives`, `Freeze`, `Loans`, `Prophecies`, `References`, `ReturnedMutRefErrors`, `ReturnedMutRefs` | `BorrowCertificates` asserts certificates, no `verify`. `BorrowChecker`: twelve rejections at preparation, the accepted programs verified. |
| Storage | `CrossInv`, `GlobalBorrows`, `GlobalInv`, `LooseFrame`, `Normalized`, `Read`, `ResourceComposition`, `Storage`, `StorageSpecErrors` | `LooseFrame` carries an authored proof. |
| Calls | `AbstractClauses`, `Callees`, `Calls`, `Composition`, `InlinedCallees`, `Modules`, `OpaqueGeneric`, `PreludeNatives`, `Summaries` | Intended negatives: a caller does not see a `[concrete]` result (`AbstractClauses`), an opaque callee's body (`InlinedCallees`), a postcondition a callee of another module refutes (`Modules`). |
| Generics | `GenericCycles`, `GenericFunctions`, `GenericScalarCalls`, `Generics`, `GenericStorage` | |
| Specifications | `ContractErrors`, `DomainQuantifiers`, `EncodedVectors`, `Increment`, `RecursiveSpecFunctions`, `SpecFunctionErrors`, `SpecFunctions`, `WrongIncrement` | `DomainQuantifiers`: an existential no index satisfies and one over an empty vector (intended); a frame over a vector write verifies. `EncodedVectors`: specification functions comparing vectors with literals and with each other, and a quantifier over a literal range, all verify. `RecursiveSpecFunctions`: a lemma by induction and a `verify … by` using it. |
| Modules | `Attributes`, `EmptyModule`, `IntrinsicErrors`, `LoweringErrors`, `PreparationRetry`, `Rust` | `Attributes` and `EmptyModule` have no `verify` targets. |
| Examples | `Account`, `Corpus`, `OrderedMap`, `Quicksort` | `OrderedMap` and `Quicksort` verify through authored lemmas over arrays; `Quicksort`'s `partition` runs at a raised cap of 400k heartbeats (277M in total). |

### Conventions

A Check file is LeanerLang source with contracts, theorems, authored
proofs, and execution or diagnostic assertions. The driver discovers every
`Check/**/*.lean`, runs each in its own Lean process at the driver's caps,
and compares all diagnostics to the adjacent `.exp` (no `.exp` means empty
output). Run it from `leaner-e2e-tests` with
`LEANER_E2E_SUITE=check lake test`; `UB=1` regenerates baselines, and every
regenerated diff is reviewed.

- A baseline records intended behavior. A negative case's expected
  diagnostic names the construct or clause, and its line carries a
  trailing `-- error: <reason>`; an unsupported positive proof is never
  recorded as an expected failure.
- A file is promoted only by the driver at the unchanged caps; a passing
  pilot promotes nothing. Caps are not raised to count a port as done.
- Every `verify` audits its artifacts (denotation route, approved axioms
  only); a fixture adds no audit of its own.
- A module lays out each function as its `fun`, its `spec`, the
  `spec fun`s that specification needs (at their first use), the theorems
  its proof uses, and its `verify … by`, in this order; a function with a
  `spec` verifies without a `verify` item. Larger modules separate groups
  of functions with `-- ## Title` headers.
- Assertion-style IR, Move, and Rust tests stay in their owning packages.
  The deprecated packages are reference material and are not run.
- Source verification is not a compiler-correctness theorem for emitted
  bytecode.

## Out-of-band items

- **Upstream rustc ask, filed.** The two Rustc Public API gaps blocking
  the Rust M0 gate (`FnDef` exposes no predicates; no const-fn binder type
  query) are
  [rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892).
  Until it lands the generic trait RawUnit fixture cannot satisfy the
  gate; the answer stays an upstream query, not a private `rustc_middle`
  read.
- **Security boundary.** Lean-source package discovery executes arbitrary
  host code at compile time; see the TODO in
  [`lir-design.md`](lir-design.md#todo-secure-lean-source-discovery-and-elaboration).
  It must be fixed before Lean-source discovery is a production compiler
  feature.

## Deliberately not scheduled

Phase 0 of [`lir-design.md`](lir-design.md) is a multi-party freeze with
one owner on both sides: the "no construct unclassified" gate is mechanized
by [`Capability.lean`](../leaner-ir/LeanerIR/Validation/Capability.lean),
and publishing RawUnit JSON v1 becomes work only when a consumer outside
this repository depends on it.

## Historical designs

[`historical/`](historical/) holds executed or superseded designs and
archives. They keep their rationale and measurements and are not updated:
[`verification-v2.md`](historical/verification-v2.md),
[`certifying-execution.md`](historical/certifying-execution.md),
[`generic-route.md`](historical/generic-route.md), and
[`verification-perf-audit.md`](historical/verification-perf-audit.md),
replaced by [`denotation.md`](denotation.md);
[`test-organization.md`](historical/test-organization.md) and
[`test-organization-history.md`](historical/test-organization-history.md),
the check ledger's earlier per-file notes, chronology, and v0 mappings,
now kept [here](#tests).
