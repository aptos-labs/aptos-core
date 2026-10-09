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

Generic module invariants (2026-10-09): `invariant<T>` (LeanerLang
`invariant {T}`) is monomorphized as the Prover does, by unifying the memory
it reads with a function's, with ghost type parameters for what a write
leaves undetermined. New fixtures:
`SourceVerify/generic_invariants_false.move`,
`SourceVerify/generic_invariant_ghosts_false.move`.

Callee preconditions (2026-10-09): an inlined callee's `requires` is owed
where it starts (`Contract.startPrecondition`, the closer's `requiring`
table), and a function without a specification calling one is verified.
Benchmark +0.15% (`ordered_map`, `GenericStorage`), same outcomes.

Inline specifications (2026-10-08): a non-opaque inline function with its
own specification is verified against it, as in the Prover; its calls stay
expanded. New fixture: `SourceVerify/inline_spec_false.move`.

Loop invariant placement (2026-10-08): a loop invariant no loop header
begins with is an error, as in the Prover (`loop_invariant_invalid`); a
loop's annotations together form its specification, where a second one had
silently replaced the first. New fixtures:
`SourceVerify/loop_invariant_placement{,_false}.move`, and a case in
`Check/Control/LoopInvariantErrors`.

Test-driver parity (2026-10-08): `--verify-only` selects the verified
functions under `--lean`; Move scripts, and any module whose rendered name is
quoted (`std::string`), are verified again: the verifier had looked them up
by their quoted spelling and silently verified nothing. The general
pipeline's stages are timed (`pipe-…`). New fixture:
`SourceVerify/script_false.move`.

Generic module axioms (2026-10-08): `axiom<T>` (LeanerLang `axiom {T} e`) is
assumed at the instantiations a verification applies, as the Prover
monomorphizes it; `num` is a specification function's type argument of its
own; a module's abort strictness holds for its functions. Four more registry
files verify; benchmark unchanged. New fixtures:
`SourceVerify/generic_axioms{,_false}.move`,
`SourceVerify/inherited_strictness{,_false}.move`.

Registry follow-up (2026-10-08): module axioms over values are assumed by
every verification and printed as axioms; the export follows specifications'
module references; arbitrary values, `int2bv` of literals and schema
equalities import exactly; the closer relates a conjunction's operand orders
and a nonnegative shift's two remainders. `abort_in_fun`, `bitwise_operators`,
`bv_internal` and `defines` verify; the benchmark keeps every outcome at
-0.58% heartbeats. New fixtures: `SourceVerify/module_axioms{,_false}.move`,
`SourceVerify/schema_equality.move`.

Partial specification functions (2026-10-08, user decisions): an `int2bv` the
compiler types at `num` wraps at `u64`, so `MoveToLeanerLang/constants.move`
translates again; one at a type parameter is still rejected. A specification
function is defined where its parameters fit their declared fixed-width types
and unspecified elsewhere (G15): the importer keeps the declared types, the
body reads them as `Int`, and the verifier derives the domain, guarding
recursive definitions and expansions alike. The closer treats a domain guard as
transparent when it decides unfoldings. Full registry, all four suites and both
cost gates pass with no regressed function; the benchmark keeps every outcome
at a heartbeat cost recorded in the handoff. New fixtures:
`SourceVerify/spec_fun_domain{,_false}.move`,
`Check/Specifications/SpecFunctionDomain{s,Errors}.lean`.

Scalar specification calls (2026-10-07): nonrecursive scalar helpers now share
representation slots across callers, preserve `num` sinks, and check that the
signature does not depend on traversal order. Three positive source roots and
two false-clause controls agree with MVP. Substituting literal scalar inputs
before arithmetic search fixes a nested-spec-call timeout: 763,888 raw heartbeats
instead of exhausting approximately 1.5G. All four full Lean suites pass,
including performance gates. `math_fixed8` gets past import but still exhausts
the registry's proof budget; the then-ignored unroll pragma is corrected in the
focused checkpoint described in the handoff, with performance still open. Representation
guards decrease from 27 to 26. The benchmark/main HTML was refreshed first:
24 verified, one expected rejection, six import failures, 9,516,016,778 raw
heartbeats. All 437 normal registry checks pass after recording the newly exposed
math_fixed8 timeout; this is not a claim that its three functions verify.
The remaining obligation-preparation issue is recorded in the handoff.
The Move Prover's bit-vector representation is no longer modeled (user
decision 2026-10-07, G14): spec arithmetic stays mathematical and `int2bv` is
exact. The representation guard is gone and all six affected benchmark modules
verify again (30 verified, one expected rejection). Width-less `int2bv`
(contextual width) and several newly exposed registry gaps are open; see the
handoff.

The subsequent checkpoint adds sound bounded unrolling. Literal arithmetic
results are no longer named, so ground nested unrolling verifies at 25k; an
outrun bound is reported as such and ends the attempt. All four suites, the
registry and the benchmark pass (24 verified, one expected rejection, six C17
import failures, −0.48% heartbeats). `math_fixed8` and unbounded vector loops
with `pragma unroll` remain open; see the handoff.

Closed scalar-package integration (2026-10-07): production import now connects
classification, physical binding widths and mathematical rewriting across all
owned/dependency scalar functions and their derived spec bodies. Unsupported
declaration families and dynamic narrowing stay guarded. Seven new source
controls pass both MVP and Leaner; the six false representation regressions now
reach verification and fail their postconditions. Checked multiplication still
aborts. Focused analysis/lowering checks and full Move/E2E suites pass. `test_bitvector`
verifies again, and `bv_mul_overflow` rejects its false no-abort contract;
representation rejections fall from 29 to 27. The final normal registry recheck passes all 437 tests
without updates (`/tmp/bv-scalar-registry-final.log`). Benchmark/main HTML was regenerated first: 24 verified, one expected
rejection, six unchanged import failures, 9,509,422,287 raw heartbeats.
Recursive/stateful spec-function calls, fields/containers, generics, inline specs/proofs and
independent arbitrary narrowing declarations remain open.

Numeric lowering builders (2026-10-07): unsigned specification operations now
have explicit mathematical expansions, including negative-input conversion,
SMT division/remainder by zero, shift-count widths, and arbitrary-overflow
narrowing casts. Twenty-four focused guards include 4,116 BitVec comparisons;
a kernel proof covers conversion for every integer. Six generated backend
verification controls pass and two false claims are rejected. The production
importer still needs representation/width integration and independent arbitrary
declarations before these builders can restore guarded registry cases. Reports
remain at the last full checkpoint; see `numeric-representation.md`.

Numeric analysis infrastructure (2026-10-07): scalar function classification now
has shared call slots, distinct body/integer-contract slots and occurrence
identities. A following width pass resolves nested bitwise spec arithmetic and
compiler-defaulted literals, retaining explicit suffix/range mismatch errors.
All 66 focused guards/kernel examples pass; seven exported source regressions
and the two-module literal registry example additionally exercise the assembled
analysis. This remains outside the production importer: no new registry
verification success or benchmark change is claimed. Whole-package collection,
binder/physical-local widths and representation-aware lowering remain open;
see `numeric-representation.md` and `/tmp/bv-width-tests.log`.

Table representation revision (2026-10-07): target an owned identity+contents
value in the denotation, keeping native storage in its runtime agreement boundary.
The exploratory source-frame extension was removed. `StateView` supplies checked
state-dependent recovery, preservation of undefined outcomes, and input consistency;
`TableValueBoundary` connects recursive snapshots and now carries typed retained
observations, generic transport and two-state argument/result rows (19 focused
checks pass). Compiler/contract integration, loan reconciliation and native
allocation/mutation dispatch remain open; see `intrinsic-maps.md`. Existing typed
footprint laws remain useful for runtime agreement. These are test-only additions
following the full checkpoint, not new registry successes.

Scalar internal bit-vector support (2026-10-07): opaque `bv_internal` bodies
can use scalar executable bitwise operations without classifying their ordinary
contracts or callers. The wrapping registry example now verifies its valid
function and rejects its false postcondition; general representation propagation
remains open. Mutable-reference contracts additionally exposed and fixed a
missing implicit read before specification casts. Positive current/old reference
checks and a false-postcondition regression agree with MVP. Outgoing calls,
aggregates/generics, inline specs, concrete clauses and proof blocks remain
guarded. See C17 in `prover-test-problems.md`.
Full Move/E2E suites and all 437 registry checks pass without baseline updates
(`/tmp/bv-internal-validation.log`, exit 0). Benchmark JSON/main HTML was
regenerated first: 24 verified, one expected rejection, six unchanged import
failures, 9,509,276,380 raw heartbeats. Core/Rust were unchanged.

Numeric representation follow-up: XAST version 10 now retains the compiler's
defaulted numeric literal marker, so width inference can distinguish `3` from
explicit `3u256`. The producer/consumer tests cover this and schema expansion.
[`numeric-representation.md`](numeric-representation.md) specifies the remaining
whole-source analysis and lowering. Narrowing bitvector specification casts need
arbitrary out-of-range values, unlike wrapping `int2bv`; a new source regression
has positive and negative MVP controls. These are prerequisites for restoring
the guarded modules, not additional verified registry cases.
This checkpoint passes 59 exchange tests, full Move/E2E suites and all 437
registry checks without baseline updates (`/tmp/bv-literals-validation.log`,
exit 0). Benchmark JSON/main HTML was regenerated before the broad Lean checks:
24 verified, one expected rejection, six unchanged import failures,
9,509,559,974 raw heartbeats. The ci CLI and registry executable were rebuilt
for XAST v10. General representation analysis and lowering remain open.

Source bit-vector soundness correction (2026-10-07): the importer now rejects
numeric bitwise operators, `bv`/`bv_ret` pragmas and `int2bv`/`bv2int` instead
of erasing their representation semantics. MVP rejects
the unsigned overflow claims that Leaner previously accepted. Both conversion
truncation and subsequent arithmetic wrapping need explicit modeling, including
generic instantiation. This reopens `features` and affected importing modules;
restoring them is a priority. See C17 in `prover-test-problems.md` and the
`SourceVerify/bv_representation_overflow.move` and `bv_representation_arithmetic.move`
regressions, plus implicit/operator/pragma propagation cases. The expanded
seed preflight passes full Move/E2E suites and all 437 registry baseline checks
without updates (`/tmp/bv-seeds-validation.log`, exit 0). The measured benchmark
has 24 verified, one expected rejection and six import failures at 9.510G raw
heartbeats. This reflects reduced coverage, not a speed improvement. The
affected modules are `features`, `cmp`, `math_fixed`, `ristretto255`, `ed25519`
and framework `ordered_map`; authored LeanerLang `OrderedMap` still verifies.
Thirty registry files now stop at this boundary. See handoff.md.

Preceding validation (2026-10-07): all four full suites and all 437 registry
baseline checks pass after the conditional-range change. That benchmark had
30 verified + one expected rejection at 15.066G raw heartbeats; measured JSON
and main-relative HTML were refreshed before the suites. Remaining registry
failures are tracked in `prover-test-problems.md`; baseline agreement is not a
claim of full MVP parity. See `/tmp/conditional-range-full-tests.log`.

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
  Existing call observations are tried before reconstructing behavioral facts:
  `aborts_if_at_state_label::caller` and `aliasing::remove_then_try_read` verify
  at native 25k. The registered StateLabelCallObservations fixture includes
  an opaque caller and a rejected false postcondition. Focused cost/label
  gates pass; all four full suites subsequently pass. The registry exposed
  a constructor-budget regression; restricting eligibility to direct invocation
  facts restores AMM create_pool's original baseline. The follow-up Boolean
  observer also verifies intermediate_states::test_config_preserved at 25k
  (8.794M typed heartbeats), preserving its two intended negative siblings.
  StateLabelBooleanObservations registers conditional-read coverage. Its
  refreshed benchmark preserves 30 verified + one expected rejection, and the
  full registry matches 437/437 baselines. The subsequent aliasing fix makes
  the complete native aliasing module verify at 25k. StateLabelMemoryEquality
  checks sequential writes, an opaque caller with only labeled contract facts,
  and rejection of a wrong update. Conditional self-referential frames remain
  facts but cannot drive recursive simplification. Focused cost/label checks
  pass. The refreshed benchmark preserves 30 verified + one expected rejection
  at 15,696,214,192 raw heartbeats (+0.1242%). All four full suites and all 437
  registry baseline checks subsequently pass (handoff.md).
  Optional certified integer reads now normalize with their actual fallback.
  LiteralOptionalReads covers signed/unsigned and symbolic values. The native
  vector_hofs_fold sum_concrete, sum_inferred, and sum_scaled verify at 25k
  using companion proofs; their benchmark and all 437 registry checks pass.
  The following range-bound congruence fix also verifies product_concrete
  (23.961M whole typed heartbeats); even-count remains open. Positive and
  missing-premise regressions and both cost gates pass; its benchmark/registry
  refresh is running (handoff.md).
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
  Move profile (`LeanerLang/Profile.lean`: `hash::sha2_256`, `sha3_256` as
  uninterpreted 32-byte functions), and any other is rejected. Still
  unmodelled and needed by `aptos-stdlib`: the `table` natives (`add_box`,
  `remove_box`, `borrow_box`, …; the prelude's `table_module`), on which
  `big_vector`, `smart_table`, `smart_vector`, `storage_slots_allocator`
  and `pool_u64*` wait, together with an `Inhabited` twin for structs
  holding a table; and the hashes' injectivity axiom.
  Table foundation (2026-10-06, [`intrinsic-maps.md`](intrinsic-maps.md)):
  identity/content snapshots, owned entry reconciliation, logical fresh
  allocation, and typed content-memory frame laws are kernel-proved. Native
  contents and allocation history use unbounded typed collections, independently
  of ordinary Move vector bounds and of a Table variant's cached length field.
  Table entry borrows are tied to the whole owner by Move's call rule;
  focused compiler probes confirm conflicts even at distinct keys. Call
  boundaries and behavioral predicates now encode native storage, including
  holes observed through returned current/prophecy rows. Result-aware runtime
  frames constrain the actual returned row; loan independence and labeled
  post-state uniqueness cover it. Tagged stored-loan targets now route entry
  write-back into Table contents and preserve unrelated storage and allocation
  history. Native entry-loan primitives now register those destinations and
  prove lookup, loan discipline, renaming, and reconciliation. Allocation,
  insertion, removal and checked retirement primitives now have freshness,
  frame and renaming proofs. Typed lookup/add/remove agreement and typed mutable
  entry reconciliation pass the focused operation checks and axiom audit.
  The separate `SnapshotValue` carrier now preserves nested observations per
  occurrence, proves physical-codec round trips and generic caller/callee
  transport, and passes eleven focused regressions plus its axiom audit.
  Contract translation now carries observations through aggregate expressions,
  expanded specification functions, old/labeled inputs and functional Table roles.
  `table_contais_to_length` verifies at its original budget, and its baseline
  refresh and recheck pass. Source regressions cover caller-side labels and
  per-occurrence functional snapshots. Five source proofs and six compatibility
  checks pass. The full benchmark retains 30 verified samples and the expected
  AMM rejection at 15.808G heartbeats. All four suites pass (125 Check fixtures
  and both cost gates); all 437 registry checks pass after the scoped baseline
  refresh. `map_equality_encoding` also verifies. Remaining Table fixtures now
  reach the native-model/reference-denotation gaps.
  The canonical-slot follow-up excludes shared-reference aliases and resolves
  caller/callee names to one owner/type slot, with a uniqueness proof that does
  not assume interning. Whole-heap insert/erase agreement and the new
  `TableStorageKeys` source regression pass focused checks. This follow-up has
  not yet had another full benchmark or suite run (126 Check fixtures now).
  The next storage increment proves whole-storage allocation/add/remove/retire
  agreement, including active unrelated loans observed through returned references.
  Actual entry borrowing and write-back agree with the typed contents update.
  Five focused test roots and a 31-theorem standard-axiom audit pass; no additional
  registry fixture is closed by these adapter prerequisites.
  Shared Table lookup and membership use explicit intrinsic-contract hypotheses.
  Caller/callee observation transport and snapshot/typed lookup agreement are
  kernel-proved. `TableReads` covers generic/nested reads, an opaque caller's
  old-state contract and aggregate payload equality; incorrect lookup and
  payload-bound claims remain rejected.
  Generated collection predicates now carry deep stored-entry invariants with
  resolved generic and phantom arguments. Table calls in code or specifications
  extend memory reach even for parameters with no global access. Nine
  `TableStoredInvariants` positive proofs and two false-claim rejections pass at
  25k. The replacement theorem requires every written key/value's invariant.
  A frontend rejection guards Table identity against physical-metadata equality;
  Table identity/content invariants and carrier-dependent generic callbacks
  still need extensions.
  `table_option` now verifies at the unchanged native 25k setting. Its obsolete
  failure baseline was removed by the owning runner; the ordinary recheck
  passes. Whole typed-theorem cost falls 28.690M → 25.254M raw heartbeats
  (−11.9749%), with 21,500 proof objects instead of 22,076. The diagnostic sample
  includes work outside the local proof-budget scope. Both cost gates, all
  Table invariant positives/rejections, the identity guard, call-range cases,
  TableReads and TableSnapshots pass (`handoff.md`). The field optimization is
  restricted to known aggregates so generic observations retain their old form.
  The first normalization benchmark exposed an ordered_map/OrderedMap
  regression, so the checkpoint correctly stopped before broad suites. The
  original call-range eligibility guard is restored; Table's stored-read/aggregate
  optimizations remain. Native table_option and both cost gates pass. The corrected
  benchmark has 30 verified plus one expected rejection, no timeouts, at
  15,646,929,683 heartbeats. Main-relative JSON/HTML are refreshed first. All
  four builds and full suites pass; all 437 registry baselines match without
  refresh. Final Table theorem cost with the guard restored is 26,045,418
  raw heartbeats and 21,500 objects; native 25k verification passes (`handoff.md`).
  The preceding call-range checkpoint passes all four full suites and all
  437 registry baselines. The subsequent computed-index benchmark preserves
  30 verified problems plus one expected AMM rejection at 15,646,803,818
  heartbeats; main-relative JSON/HTML are refreshed with type_info excluded.
  Mutating/allocation native contracts and
  adapters, whole-memory ownership frames, Table type-domain
  quantification, future `final` contents and pure `spec_new` identity remain open.
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
does not verify yet, and is absent while everything verifies. The former full
verification claim is superseded by the source bit-vector soundness correction
above (2026-10-07). Open:

- `features` is now rejected at import because `spec_contains` uses `int2bv`.
  Its earlier verification erased the conversion semantics and is not a valid
  completion claim. `bit_vector` still verifies. The earlier measurement for
  both modules was within the default budget,
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
over `&mut` parameters verify as the Prover does). S2b implemented:
state-change and invocation-defined labels are memory expressions in
contracts, including when an opaque callee's contract replaces a call.
Presence and success conditions remain proof obligations (`DefinedStateLabels`,
`DefinedStateLabelErrors`, `InvocationStateLabels`, `InvocationStateLabelErrors`).
The decoded-state follow-up (2026-10-07) verifies `two_state_labels::two_increments`
at the native 25k limit (18.270M typed heartbeats). Known memory and decoder
values are normalized before integer range checks. The registered IR fixture
`StateLabelDecoding` covers an opaque caller without callee program points,
missing-resource rejection and overflowing-update rejection. All 30 labeled
registry baselines and both cost gates pass. The regular benchmark passes with
30 verified + one expected rejection at 15,647,129,560 heartbeats (+0.0013%).
Main-relative JSON/HTML are refreshed; the prior checkpoint is the latest
full-suite validation (`handoff.md`).
The witness-search follow-up verifies `spec_fun_old_param_labeled_with_memory`
at 25k (24.132M typed heartbeats, down from 30.912M). Constructive/context
witnesses precede program points; `StateLabelWitnesses`, all 30 state-label
registry checks and both cost gates pass. The benchmark retains 30 verified
+ one expected rejection at 15,647,293,919 heartbeats (+0.00105%). Main-relative
JSON/HTML were refreshed before all four builds/full suites and 437/437
registry checks passed (`handoff.md`).
Open: the proof budget of `aborts_if_at_state_label::caller`, reads of removed
resource contents (`unmodified_memory_at_label::swap`, see the design), S4.

## Tests

### Gates

| Gate | Result |
|---|---|
| `leaner-ir` build and `lake test`, with the `DenotePerformance` and `CompositionPerformance` cost gates | PASS |
| `leaner-move`, `leaner-rust` builds and tests | PASS |
| `leaner-e2e-tests` `lake test`: suites `move`, `rust`, `verify`, `check`, `monodiff` | PASS |
| Check fixtures | 122 of 122 pass exactly (2026-10-06) |
| Move-to-LeanerLang corpus round trip | every module is a typed fixed point |
| Move Prover unit tests, `MVP_TEST_FEATURE=lean` (not in CI) | Full 437-file baseline refresh passes (2026-10-06), after all four full Leaner suites: 119 without diagnostics, 4 with warnings only, 50 preverification rejections, 232 verification diagnostics, 32 other errors. Returned-reference freezing and quantifier scope fixes expose both C16 files; two previously blocked positives prove. Cross-resource labels prove 6/6 positives, bp_pure_callee::count_all proves with a companion at 25k, and anchor_mutation_pre_state’s valid derived claim now proves; intended negatives remain rejected. Fresh benchmark/main HTML precedes suites: 28/32, no newly failed targets. Logs: `/tmp/registry-prepared-behavior-<package>-{build,test}.log`, `/tmp/behavior-guard-prover-refresh.log`. The final small guard passed focused checks, both cost gates, a fresh benchmark, and the full registry refresh; the full suites passed immediately before it. Work paused at user request. Intentional negatives remain; see [`prover-test-problems.md`](prover-test-problems.md). |

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
| Scalars | `Addresses`, `Arithmetic`, `BitVectors`, `Division`, `Integers`, `Literals`, `Order`, `Signed`, `SpecLogicalArithmetic` | `BitVectors`: `pragma bv`; a one-bit membership test verifies without the pragma, while a wrong postcondition and a fact about toggled bits without the pragma fail (intended). `Division`: the division algorithm, the remainder's range, and cancellation at a divisor that is not a literal; a strict bound fails (intended). `Order`: an authored proof of transitivity. |
| Structs | `Abilities`, `Invariants`, `PositionalStructs`, `Tuples` | `Abilities` has no `verify` targets. |
| Enums | `EnumPatterns`, `EnumPayloads`, `EnumRefContracts`, `EnumRefs`, `EnumResources`, `Enums`, `MatchPatterns`, `VariantFields` | `EnumRefs` is execution only. `MatchPatterns` and `VariantFields`: a value no pattern or field fits aborts with Move's incomplete-match code, in the denotation and at run time. |
| Vectors | `CertifiedReads`, `ElementQuantifiers`, `VectorBounds`, `VectorOperations`, `Vectors`, `GenericSwapRemove` | `CertifiedReads`: signed/unsigned bounds through total integer reads, zero for missing entries, and rejection of a false signed nonnegativity claim at 25k. `ElementQuantifiers`: element-wise preconditions establish indexed-read properties for value/reference vectors, signed integers, Booleans and generic equality predicates; stronger bounds and a missing-entry claim remain rejected at 25k. A target whose guards stay undecided exhausts heartbeats rather than failing fast. |
| Control | `Aborts`, `ControlForms`, `LiteralLoops`, `LoopControlErrors`, `LoopInvariantErrors`, `LoopInvariants`, `Loops`, `LoopVerification` | `Aborts`: a false contract (intended). `LiteralLoops`: loops over literal vectors with recursive specification functions in their invariants, and a nested loop with a nonlinear invariant; a wrong value fails (intended). `LoopInvariantErrors`: an invariant not established at entry and at an iteration, and one of a loop's two annotations (a `where` region and a following `spec`), both of which form its specification. |
| References | `BorrowCertificates`, `BorrowChecker`, `BorrowErrors`, `BorrowGlobalErrors`, `CorePrimitives`, `Freeze`, `Loans`, `OperandLoans`, `Prophecies`, `ReferencePatterns`, `References`, `ReturnedMutRefErrors`, `ReturnedMutRefs` | `BorrowCertificates` asserts certificates, no `verify`. `BorrowChecker`: twelve rejections at preparation, the accepted programs verified. |
| Storage | `CalleeFrames`, `CrossInv`, `GenericSpecReads`, `GenericSpecResourceArguments`, `GlobalBorrows`, `GlobalInv`, `LooseFrame`, `Normalized`, `Read`, `ResourceComposition`, `Storage`, `StorageSpecErrors`, `TableReads`, `TableStorageKeys`, `TableStoredInvariants` | `TableStoredInvariants`: nine stored-entry proofs at 25k, including phantom arguments, computed keys and both handle layouts, plus two false-claim rejections. `TableReads`: membership, scalar/generic/nested lookup, opaque caller old-state contract and false-lookup rejection. `TableStorageKeys`: canonical Table slots across caller/callee namespaces, reference exclusion and distinct handles. `LooseFrame` carries an authored proof. `GenericSpecReads`: a generic specification function reads the resources of its call's type arguments; a claim about another type fails (intended); a generic caller calls a generic function twice at its own type parameter. |
| Calls | `AbstractClauses`, `Callees`, `Calls`, `Composition`, `InlinedCallees`, `Modules`, `OpaqueGeneric`, `PreludeNatives`, `Summaries` | Intended negatives: a caller does not see a `[concrete]` result (`AbstractClauses`), an opaque callee's body (`InlinedCallees`), a postcondition a callee of another module refutes (`Modules`). |
| Generics | `GenericCycles`, `GenericFunctions`, `GenericScalarCalls`, `Generics`, `GenericStorage` | `Generics`: a generic function's public theorem applied at a concrete instantiation, coherent by evaluation. |
| Closures | `Carriers`, `Frames`, `GenericHofs`, `GenericResources`, `InvariantBehavior`, `Known`, `StoredFrames`, `StoredFrameErrors`, `StoredWriteFrames`, `StoredWriteFrameErrors` | `StoredWriteFrames`: addressed and wildcard frames, generic resource instantiation, opaque results, and captured owners. `StoredWriteFrameErrors`: wrong resource, replacement, and false preservation claims are rejected.  `InvariantBehavior`: construction checks quantified behavioral invariants. `StoredFrames`: default function-field frames survive publication, removal, and invocation. `StoredFrameErrors`: packing and replacement reject memory-writing closures. `Carriers`: a function value the proof does not see is typed by its carrier, without a typing assumption, in a unit declaring a phantom type parameter and resources with function-typed fields; a wrong result fails (intended). `Known`: known closures invoked through their targets' contracts, a generic target included, at a concrete caller and at a generic caller's own parameters, beside a function type over those parameters; a wrong postcondition fails (intended). `Frames`: function-valued parameters keep memory, established from a target's theorem or body, over memory holding an enum resource and an instance of a generic declaration; a closure that changes memory fails (intended). `GenericHofs`: generic higher-order functions invoking a function value over their type parameters, callers at type arguments, and a value passed through; a wrong result fails (intended). |
| Specifications | `AssertionErrors`, `Assertions`, `CapturedFunctionValues`, `ContractErrors`, `DefinedStateLabelErrors`, `DefinedStateLabels`, `DomainQuantifiers`, `EncodedVectors`, `FieldUpdate`, `GenericEnumPayload`, `Increment`, `InsertionMap`, `InvocationStateLabelErrors`, `InvocationStateLabels`, `Lemmas`, `RecursiveSpecFunctions`, `SpecFunctionDomainErrors`, `SpecFunctionDomains`, `SpecFunctionErrors`, `SpecFunctions`, `SpecTuples`, `StateLabels`, `TableSnapshots`, `VectorRanges`, `VectorSlices`, `WrongIncrement` | `SpecFunctionDomains`: a fixed-width parameter type is the function's domain; expansions and a recursive definition inside it verify, and the body computes beyond the width. `SpecFunctionDomainErrors`: claims at arguments outside the domain fail (intended). `VectorRanges`: vector index domains in universal/existential quantifiers and slices, shared generic values, empty/nonempty vectors, old versus current mutable values, and an excluded-upper-endpoint rejection at 25k. `TableSnapshots`: old/nested observations, caller-side quantified state labels, and functional snapshots with equal identity but different contents in one vector. `GenericEnumPayload`: a generic opaque callee returns a concrete enum; its index bound supports caller-side arithmetic after payload and carrier-family normalization. `CapturedFunctionValues`: leading and trailing captures compare executable closures with specification literals. `InsertionMap`: insertion-position roles preserve descending sequence order, with both key/rank clauses and implicit validity verified. `InvocationStateLabels`: forward and chained invocation labels, opaque callers of both projections, and invocation through a function-valued parameter. `InvocationStateLabelErrors`: a labeled projection cannot assume that an aborting call succeeds. `DefinedStateLabels`: publication, removal, update, chained definitions, generic resources, lexical lets across clauses, and opaque callers using labeled contracts without callee program points. `DefinedStateLabelErrors`: negation, implication antecedents, and an aborting removal do not supply assumptions (intended failures). `AssertionErrors`: an in-body assertion that does not hold, in a function with and without a specification, a loop body, an inlined callee, a generic function, and over a state anchor. `DomainQuantifiers`: an existential no index satisfies and one over an empty vector (intended); a frame over a vector write verifies; quantifiers over a vector type bind vectors of its element type. `EncodedVectors`: specification functions comparing vectors with literals and with each other, and a quantifier over a literal range, all verify. `Lemmas`: `spec lemma`s, a recursive one by its integer parameters and one by `decreases` with a `split`, applied in functions (`apply`, `∀ … apply`), and a proof's steps at entry and before a return; a false lemma, a false recursive lemma, an unmet premise, and an argument out of a parameter's range are reported (intended); a lemma not established gives no fact, so a function needing its conclusion fails (intended); a lemma over a vector whose induction needs its elements' bounds, and a function needing it fails without it (intended). `RecursiveSpecFunctions`: a lemma by induction and a `verify … by` using it; mutually recursive functions with lemmas recursing through each other; Boolean parameters of a function and a lemma; mutually recursive functions reading storage. `StateLabels`: a quantified state label — an existential split of a two-state specification function over two opaque calls, witnessed at the program point after the first, a one-state read at the label, and the split over two inlined calls through a `&mut` parameter, witnessed by the parameter's value at the point between them; the splits over one call fail (intended). |
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
