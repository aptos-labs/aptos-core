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
Lemmas and proof blocks ([`lemmas.md`](lemmas.md), L1–L4 done
2026-10-03): `spec lemma`s are theorems over their statements, recursion
groups by well-founded recursion on their measure; a proof's steps are
in-body `assert`, `assume`, `apply`, and `split` statements, an `assume`
in a lemma's proof a hypothesis of every theorem using it. Mutually
recursive specification functions are defined together. A lemma
parameter or quantifier binder of an aggregate type ranges over the
values of its native type.

Open:

- **Static global memory** ([`static-memory.md`](static-memory.md),
  decided 2026-10-02; S1–S5 done 2026-10-02). The denotation's global
  memory is one typed memory per resource type, related to runtime globals
  once, in the agreement; nothing about memory typing is proved per
  function. Open: proving `GlobalsPreserved`, which the public theorem
  takes, from `preservation`, and the gaps the design's status lists.
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
  the instance, lookup, and write-split deciders, and last core `grind`,
  whose congruence closure omega lacks) before the saturating
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
- **Closed (2026-09-30): the preparation certificates.** Every module and
  every importer replayed loan-death marking and shared-reference erasure
  in the kernel, 14 s before a module's first target. The semantics reads
  the validated unit now: loan deaths end at their anchors, and shared
  references are decided in place (`perf-notes.md`, "No preparation to
  certify"; `prophetic-references.md`, P1 and P6).
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
- **Closed (2026-09-30): discarded statement values.** A Move branch that
  discards a value (`v.remove(index);` in `capability::remove_element`,
  `pool_u64::deduct_shares`) failed with LEANER-TYPE-MISMATCH. The exchange
  and the LIR keep the source's unit; the canonical rendering writes the
  branch as an `if` without `else`, whose branch value LeanerLang discards
  where the `if` supplies none, but lowering checked that branch against
  the expected unit first. It now lowers such a branch without an expected
  type (test `discarded_branches` in `LeanerLang/Tests/Frontend.lean`).
- **Closed (2026-09-30): `math128::sqrt` exceeded its budget in
  normalization.** `aptos_std::math128` now verifies completely, `sqrt`
  included, in 9 s for the module (`leaner-move verify` of `aptos-stdlib`
  with `--modules aptos_std::math128`). `ristretto255::scalar_invert`
  verifies in 50s.
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
does not verify yet, and is absent while everything verifies. Every clause
of every module verifies within the default budget (2026-09-28). Open:

- `features` and `bit_vector` verify completely within the default budget,
  `apply_diff`, `change_feature_flags_internal`, and
  `shift_left_for_verification_only` included: 59 s for the two modules
  (`leaner-move verify` of the framework's `move-stdlib` with
  `--modules std::bit_vector,std::features`, 2026-09-30).
- `acl` verifies (2026-09-28). The exchange maps `vector::contains`,
  `index_of`, and `remove` to the primitives the denotation states
  directly, as the Move Prover treats them, so their loops are not
  inlined and the search result is characterized rather than unrolled.
  Since 2026-10-02 every intrinsic `vector` function with an LIR
  counterpart is mapped so (source-verification.md, "Move vector
  intrinsics").
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
  (2026-09-28). The e2e `verify` suite verifies the framework's
  `move-stdlib` in place (2026-09-30); the copy under
  `MoveToLeanerLang/MoveStdlib` serves only the Move-to-LeanerLang
  baselines.

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
  A holder overwritten whole (an assignment or a binding of the local) no
  longer keeps its loans live, and an assignment ends the loans neither its
  place nor what follows uses once its value is produced, before the write
  (2026-10-02); a unit with a rejected function is not verified.
- Strict external JSON compatibility of RawUnit and the full staged API.

## 5. Runtime semantics

Design: [`elaboration-design.md`](elaboration-design.md) (runtime model,
big-step semantics, interpreter).

Done, 2026-10-01: preservation of runtime typing for prepared units
(`Semantics/Preservation.lean`, [`static-typing.md`](static-typing.md)), and
loan independence: runs from starts differing in their loan bookkeeping
agree up to the loans they mint (`Semantics/LoanIndependence.lean`).

Open: the rest of the metatheory of the big-step semantics — determinism,
completeness of the interpreter up to fuel, and no-stuck for prepared
invocations.

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

## 9. Higher-order functions

Design: [`higher-order-functions.md`](higher-order-functions.md).

Done, 2026-09-30: H1, closures at run time (masks, instantiations fixed at
construction, capture validation, `invoke` in the borrow analysis, name
ordering); H2, the Move path (closure construction in the exchange, the
Move persistence rule, LeanerLang mask and generic-target spellings, XIR
lowering, MonoVM differential and compiler-v2 transactional fixtures); H3,
the denotation (function values as closures, invocations of known closures
verified through their targets' contracts, generic targets included).

Done, 2026-10-01: H4a, the behavioral predicates in contracts; of H4c, the
dispatch of a literal closure's predicates to its target's precondition and
theorem.

Open: the rest of H4 — typing of invocation outcomes (H4b, planned in
[`static-typing.md`](static-typing.md), Phases 1–4 done: the checker at
preparation, defensive semantics, the operation lemmas, and preservation;
Phase 5 in progress: the invocation rule's core and loan independence done,
next the typing hypotheses, the closer rule, and the callers' discharge),
the rule for
invoking an unknown closure and the closed-world theorem (H4c), state
labels, frames, and `&mut` arguments (H4d) — then H5, the re-entrancy lock.
A function type gets a typed carrier, as `u64` has one (decided
2026-10-04, "Closures in memory: a typed carrier", milestones C1–C4): no
function value needs reasoning about types, and memory with a
function-typed field is typed for free. Done, 2026-10-04: C1 (families,
memory, and executable units at a unit), C2a (compiled code at its unit),
C2b1 (closures of targets without type parameters typed in native types,
their nodes carrying their rows), C2b2 (closures of generic targets typed at
their frames, the target frame's coherence taken at creation; coherence over
the types a frame requires and its type parameters, which real frames
satisfy), C2c (generic public theorems at every instantiation coherent with
the frame their type arguments induce), C3a (`NTy.TypedAs` for every native
type, decided by evaluation; memory with enum resources and instances of
generic declarations typed), C3b0 (native function types keep which
parameters are shared references), C3b1 (a closure's native typing checks
that its instantiation is faithful), C3a2 (`TypedAs` functional, unique,
and under substitution), C3b2 (values a closed native type admits typed
semantically where the unit's readings agree, closures included), C3c0
(native nominal types record their type arguments, so a native type reads as
one semantic type in every unit, phantom parameters included), C3c (memory
with function-typed fields typed; a function value the proof does not see
typed by its carrier, a generic function assuming only that its frame
resolves an invocation's rows to scalars); next C4, the Prover tests on
stored function values. Open, not scheduled: closures whose targets return
references, typed as function types marking their shared results.

State labels (`..S |~ …`, `exists S in *`) have their own design,
[`state-labels.md`](state-labels.md): labels as `Memory` variables, defined by
state-change predicates or quantified over all memories, milestones S1–S4
(round trip, contracts, the closer's witnesses, the rest of the
`state_labels` directory). Done, 2026-10-05: S1, labels named in the
exchange, bound at state-domain binders in LIR, spelled in LeanerLang
(`S |~`, `..S |~`, `S.. |~`, `S..T |~`, `∃ (S : StateDomain)`, `publish`,
`remove`, `update`), validated; every `state_labels` test re-elaborates.
S2a: quantified labels over memory bound in contracts as `Memory`
variables, ranges selecting the states predicates and specification
functions read; S3a, a label also binds the values of the `&mut`
parameters at its state; S3b, the program points (the values and states
handed to continuations at call boundaries) as hypotheses of the leaf,
tried jointly as a label's witnesses, and a copy witnessed from the bounds
its existential states where no call separates the states
(`Check/Specifications/StateLabels.lean`; the labeled `state_labels` tests
over `&mut` parameters verify as the Prover does). Open: S2b (defined
labels), S4.

## Tests

### Gates

| Gate | Result |
|---|---|
| `leaner-ir` build and `lake test`, with the `DenotePerformance` and `CompositionPerformance` cost gates | PASS |
| `leaner-move`, `leaner-rust` builds and tests | PASS |
| `leaner-e2e-tests` `lake test`: suites `move`, `rust`, `verify`, `check`, `monodiff` | PASS |
| Check fixtures | 87 of 87 pass exactly |
| Move-to-LeanerLang corpus round trip | every module is a typed fixed point |
| Move Prover unit tests, `MVP_TEST_FEATURE=lean` (not in CI) | 112 of 414 verify (3 with the Move compiler's warnings); the problems are registered in [`prover-test-problems.md`](prover-test-problems.md) |

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
| Scalars | `Addresses`, `Arithmetic`, `BitVectors`, `Division`, `Integers`, `Literals`, `Order`, `Signed`, `SpecLogicalArithmetic` | `BitVectors`: `pragma bv`; a wrong postcondition and the same leaf without the pragma fail (intended). `Division`: the division algorithm, the remainder's range, and cancellation at a divisor that is not a literal; a strict bound fails (intended). `Order`: an authored proof of transitivity. |
| Structs | `Abilities`, `Invariants`, `PositionalStructs`, `Tuples` | `Abilities` has no `verify` targets. |
| Enums | `EnumPatterns`, `EnumPayloads`, `EnumRefContracts`, `EnumRefs`, `EnumResources`, `Enums`, `MatchPatterns`, `VariantFields` | `EnumRefs` is execution only. `MatchPatterns` and `VariantFields`: a value no pattern or field fits aborts with Move's incomplete-match code, in the denotation and at run time. |
| Vectors | `VectorBounds`, `VectorOperations`, `Vectors` | A target whose guards stay undecided exhausts heartbeats rather than failing fast. |
| Control | `Aborts`, `ControlForms`, `LiteralLoops`, `LoopControlErrors`, `LoopInvariantErrors`, `LoopInvariants`, `Loops`, `LoopVerification` | `Aborts`: a false contract (intended). `LiteralLoops`: loops over literal vectors with recursive specification functions in their invariants, and a nested loop with a nonlinear invariant; a wrong value fails (intended). `LoopInvariantErrors`: an invariant not established at entry and at an iteration. |
| References | `BorrowCertificates`, `BorrowChecker`, `BorrowErrors`, `BorrowGlobalErrors`, `CorePrimitives`, `Freeze`, `Loans`, `OperandLoans`, `Prophecies`, `ReferencePatterns`, `References`, `ReturnedMutRefErrors`, `ReturnedMutRefs` | `BorrowCertificates` asserts certificates, no `verify`. `BorrowChecker`: twelve rejections at preparation, the accepted programs verified. |
| Storage | `CalleeFrames`, `CrossInv`, `GenericSpecReads`, `GlobalBorrows`, `GlobalInv`, `LooseFrame`, `Normalized`, `Read`, `ResourceComposition`, `Storage`, `StorageSpecErrors` | `LooseFrame` carries an authored proof. `GenericSpecReads`: a generic specification function reads the resources of its call's type arguments; a claim about another type fails (intended); a generic caller calls a generic function twice at its own type parameter. |
| Calls | `AbstractClauses`, `Callees`, `Calls`, `Composition`, `InlinedCallees`, `Modules`, `OpaqueGeneric`, `PreludeNatives`, `Summaries` | Intended negatives: a caller does not see a `[concrete]` result (`AbstractClauses`), an opaque callee's body (`InlinedCallees`), a postcondition a callee of another module refutes (`Modules`). |
| Generics | `GenericCycles`, `GenericFunctions`, `GenericScalarCalls`, `Generics`, `GenericStorage` | `Generics`: a generic function's public theorem applied at a concrete instantiation, coherent by evaluation. |
| Closures | `Carriers`, `Frames`, `GenericHofs`, `Known` | `Carriers`: a function value the proof does not see is typed by its carrier, without a typing assumption, in a unit declaring a phantom type parameter and resources with function-typed fields; a wrong result fails (intended). `Known`: known closures invoked through their targets' contracts, a generic target included, at a concrete caller and at a generic caller's own parameters, beside a function type over those parameters; a wrong postcondition fails (intended). `Frames`: function-valued parameters keep memory, established from a target's theorem or body, over memory holding an enum resource and an instance of a generic declaration; a closure that changes memory fails (intended). `GenericHofs`: generic higher-order functions invoking a function value over their type parameters, callers at type arguments, and a value passed through; a wrong result fails (intended). |
| Specifications | `AssertionErrors`, `Assertions`, `ContractErrors`, `DomainQuantifiers`, `EncodedVectors`, `Increment`, `Lemmas`, `RecursiveSpecFunctions`, `SpecFunctionErrors`, `SpecFunctions`, `SpecTuples`, `StateLabels`, `VectorSlices`, `WrongIncrement` | `AssertionErrors`: an in-body assertion that does not hold, in a function with and without a specification, a loop body, an inlined callee, a generic function, and over a state anchor. `DomainQuantifiers`: an existential no index satisfies and one over an empty vector (intended); a frame over a vector write verifies; quantifiers over a vector type bind vectors of its element type. `EncodedVectors`: specification functions comparing vectors with literals and with each other, and a quantifier over a literal range, all verify. `Lemmas`: `spec lemma`s, a recursive one by its integer parameters and one by `decreases` with a `split`, applied in functions (`apply`, `∀ … apply`), and a proof's steps at entry and before a return; a false lemma, a false recursive lemma, an unmet premise, and an argument out of a parameter's range are reported (intended); a lemma not established gives no fact, so a function needing its conclusion fails (intended); a lemma over a vector whose induction needs its elements' bounds, and a function needing it fails without it (intended). `RecursiveSpecFunctions`: a lemma by induction and a `verify … by` using it; mutually recursive functions with lemmas recursing through each other; Boolean parameters of a function and a lemma; mutually recursive functions reading storage. `StateLabels`: a quantified state label — an existential split of a two-state specification function over two opaque calls, witnessed at the program point after the first, a one-state read at the label, and the split over two inlined calls through a `&mut` parameter, witnessed by the parameter's value at the point between them; the splits over one call fail (intended). |
| Modules | `Attributes`, `EmptyModule`, `IntrinsicErrors`, `LoweringErrors`, `Rust` | `Attributes` and `EmptyModule` have no `verify` targets. |
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
