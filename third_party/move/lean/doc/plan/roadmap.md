# Roadmap

What is open, by track, in priority order. Each track names its design;
the design holds the model and the decisions, this file only what is not
done. Current state that is not derivable from the source is under
[`../status/`](../status/). Approved but unscheduled plans sit beside this
file.

## 1. Verification of validated VIR

Designs: [`denotation.md`](../designs/denotation.md),
[`native-verification.md`](../designs/native-verification.md),
[`prophetic-references.md`](../designs/prophetic-references.md),
[`proof-strategy.md`](../designs/proof-strategy.md),
[`lemmas.md`](../designs/lemmas.md).

- **Native verification, remaining items.** BigOrderedMap's
  `test_verify_modify`; the cost of generic map targets; reframe lemmas for
  enum and table fields of a parameter's type, and an opaque specification
  function read through the runtime frame for an invariant applying it to
  move; the transport of a definition taking types but no frame (SC2).
- **Address representation.** Addresses are strings in the runtime and the
  denotation (`RuntimeValue.address`, `NTy.address`); the Move Prover's are
  integers in `0 .. 2^256`. Integers (`u256` values) would remove the second
  spelling of an address key's `DecidableEq` instance ("Key-instance
  spelling" below) and bring addresses within reach of the arithmetic
  deciders.
- **Frames of loops.** A loop writing the store keeps no frame, where the
  Prover havocs only what the loop modifies; one that only reads it keeps it
  whole. A function updating a spec variable in a loop therefore has the
  frame `modifies *`, and any function's closed frame over a loop's real
  writes fails. A frame per loop, from what its body writes, removes both.
- **A generic callee without a contract takes its caller off the Given
  route.** `givenUnsupported?` (`VerexSL/Verify/Given.lean`) reports "calls
  to callees without a contract", and the caller is verified concretely with
  the callee inlined ([`given-parameters.md`](../designs/given-parameters.md),
  "Which members take the route, and what happens when they cannot"). A
  derived summary cannot fill the gap: summaries are the concrete route's,
  and the Given branch of `verifyMember` (`VerexSL/Verify/Members.lean`)
  excludes a member being summarized. A summary derived over a Given
  interpretation would keep such callers on the route; it is the same work
  as the generic-signature callees of "Cost" below.
- **Quantifier syntax aligned with Move.** VerexSL's `∀ (i : T), P` should
  move toward Move's spelling, `forall i: T @ P`, with `forall` and `@`
  also accepted as symbol tokens.
- **Function values as native data, remaining items**
  ([`higher-order-functions.md`](../designs/higher-order-functions.md),
  "Function values as native data"): the invocation equations of closure
  sites with type arguments, which keep `functionOfGeneric` and its
  invocation rule; captures holding a function value under a vector or a
  map, which the generator rejects because such a carrier's certificate is a
  proof over its elements — the decided remedy is to drop that certificate
  and make the bound a leaf fact, as an integer's range is; and the sharing
  marks of `NTy.function`, which should be derived data of the target.
- **Behaviour facts of closures are not resolved against the target's
  contract.** A closure's or function literal's behaviour facts at entries
  (`AbortsOfEntries`, `EnsuresOfEntries`, `AtEntries … CallAborts`,
  `KeepsMemoryAtOf`) stay as they are stated, while the goal states the same
  behaviour at other entries or through the target's contract, and nothing
  relates the two. At 821af8289ba against main: 46 headlines (31 clauses),
  17 functions, 10 tests. Reproducer:
  `verex-e2e-tests/VerexE2ETests/Check/Closures/WovenBehaviors.lean`,
  `leading`, whose leaf is
  `EnsuresOfEntries (closureMeaning) (site0 (x)) (y) … ↔ EnsuresOfEntries (closureMeaning) (site2 ()) (x, y) …`.
- **Turned off, to fix.** `pool_u64::deduct_shares` sets `pragma verex_verify = false`: the
  obligation that a removed shareholder is no longer in `shareholders` (a
  quantified read after `eraseIdx`) falls to the general pipeline, which
  does not decide it; its authored proof is written in the proof language.
- **Structure of the solver and the step layer.** Found in review, each a
  change of mechanism rather than a cleanup:
  - the function-value codec (`Denote/FunctionValues.lean`,
    `carrierWith`/`encodeWith`/`decodeWith` and their laws) restates
    `NTy.codec` and its laws over an explicit function arm; one codec with
    the arm as a parameter removes the second stack;
  - the step normal form and the leaf's are two pipelines joined by
    refolding (`foldLeafHypotheses`, `renormalizeFacts`,
    `reduceDefinitions`); one entry point parametrized by the rule set;
  - rule orientation is decided in `factRule`, `scanRules`, `settle` and
    `settleCaseEquations` separately; one `ruleOf` over a declaration;
  - memory-case values are named after the fact (`nameSomeReads`,
    `renormalizeFacts`); naming them at the split removes the repairs;
  - positional argument matching on solver heads (`isAppOfArity`,
    `getArg!`) made the `FunctionCarriers` parameter a tree-wide edit;
    recognizers per head;
  - step caches keyed by goal survive backtracking (`setNGen` after
    restore); a backtrackable cache.
- **Key-instance spelling.** The normal form spells the `DecidableEq`
  instance of a concrete key type as `NTy.decEq carriers τ` (typed views,
  `NTy.readDecEq`); a term elaborated outside the normal form gets the
  synthesized instance (`instDecidableEqString` for an address). They are
  definitionally equal, not syntactically. Integer keys already use the
  synthesized instance. A reducible `NTy.decEq` changes
  discrimination-tree keys and breaks automatic proofs (`bitwise_table`,
  `data_invariant_in_loop`); declaring it an instance does not help.
  Decided for authored proofs: the proof language's elaborator spells the
  instance as the leaf does ([`proof-language.md`](../designs/proof-language.md),
  "Semantics and lowering", 2); until then a script works on the leaf's
  own terms (as `pool_u64`'s `redeem_shares` does). Open for the normal
  form: whether the synthesized instance at every closed concrete type,
  through the typed reads and the frame transports, is worth its change
  to the automatic proofs.
- **Cost (P3, P4).** Re-measure the benchmark, `DenotePerformance`, and the
  map modules on native leaves; the P4 goal is all 38 BigOrderedMap targets
  below roughly 1.9G heartbeats including authored proofs. Then the levers of
  [`performance.md`](../designs/performance.md), "Open levers", in its order,
  postponed for now; the next is fact dependencies of normalized subterms
  (Preparation, 6), then fail fast and ground evaluation of literal leaves.
  First: data and global invariants,
  [`data-and-global-invariants.md`](data-and-global-invariants.md).
  Debts:
  [`measurement-debts.md`](../status/measurement-debts.md).
  - **Callee summaries do not pay for themselves on a fresh run**
    ([`performance.md`](../designs/performance.md), "Callee summaries"; the
    measurements are in [`transitions.md`](transitions.md), "Verification
    cost and callee summaries"). Each item needs a design decision before
    code, in this order:
    1. *Application cost of four or more conjuncts.* Applying a large
       summary can cost more than inlining the callee. Measure which of
       `normalizeAround`, `introNP` and `releaseContinuations` dominates the
       per-conjunct loop, then design; without this, more coverage can cost
       more.
    2. *Derivations are not stored in compiled namespaces.* Every module's
       process derives its callees' summaries again, which is what makes
       summaries a net loss.
    3. *Generic-signature and loop callees get no summary.* The largest
       lever left: a summary quantified over the callee's instantiation, and
       loop invariants carrying provenance.
    4. *No summary inside a shared-fold region* (`activeSummary`), the
       largest group of skipped call sites. Some of these callees have no
       summary either; count them before designing.
    5. *Call sites skipped because the call lacks a hypothesis the summary
       needs* (`error::invalid_state`, `invalid_argument`); not examined.
    6. *The leaf side*, after the walk: opening only what a leaf inspects,
       guarded facts, the memory frame as a common fact at joins.
  - **Shared work across targets and leaves.** Closed normalized call
    equations are reused only within a target; sharing them across targets
    must keep the argument-shape and family-transport key. The release of a
    call's continuation is not measured separately from its no-held-head
    fast path. The deciders' successes and failures per leaf are not
    profiled, which any change of their order or a cheaper applicability
    check needs first. Relevance-directed preparation is unmeasured: only a
    minority of prepared hypotheses are used by the proof, which does not
    imply a proportional saving; a closure from the goal and the names the
    proof reads must keep the omitted facts explicitly reachable.
  - **OrderedMap costs two to four times main per target.** Of the 17
    targets of `verex-e2e-tests/VerexE2ETests/Check/Examples/OrderedMap.lean`,
    measured uncapped at 821af8289ba against main 7a89e8311c5: `empty` 2.7×,
    `borrow_key_at` 3.0×, `lower_bound` 2.0×, `add` 3.0× (243M),
    `remove` 2.4×, `missing_scenario` 3.4×, `absent_lookup_scenario` 3.0×
    (170M), `duplicate_scenario` 3.4× (199M), `bool_key_scenario` 4.3×
    (377M), `ordering_scenario` 3.5× (419M), `lookup_scenario` 3.6× (466M);
    `length`, `lower_bound_loop`, `contains` and `borrow` are within 1.0–1.6×.
    So `add` and four scenarios exceed the end-to-end driver's 180M
    per-target cap, and `remove_scenario` and `remove_edges_scenario` (165M
    and 202M at main) exceed their own `pragma heartbeats = 300` even
    without a cap. The file passes at main. The cause is not localized
    beyond the carriers commit; neither the removal nor the nominal
    declaration work is known to own it. Reproducer: the file under
    `-DmaxHeartbeats=180000 -Dweak.verex.verifyHeartbeats=180000`.
  - **Failures that are only cost.** Some failures against main close with
    the leaf budget doubled (`-Dweak.verex.leafHeartbeats=40000`): at
    821af8289ba, 13 headlines, 8 functions, 6 tests; three functions verify
    whole (`generic_function_literals` `via`, `spec_fun_cross_resource`
    `test_behavior_cross_resource`, `uninterpreted_spec_fun_congruence`
    `scramble_all`), the rest are single clauses in functions failing for
    other causes. 3bd43a06681 was measured to remove `scramble_all`'s
    errors and half of `spec_fun_cross_resource`'s. Each is a leaf whose
    decision got more expensive under the carriers, not a missing rule.
    Reproducer: `verex-e2e-tests/VerexE2ETests/SourceVerify/generic_function_literals.move`,
    `via`, with and without the doubled leaf budget.
- **Assumptions.** Discharge the hypotheses in
  [`assumptions.md`](../status/assumptions.md): the compilation agreement
  (`compileFunction_agrees`, `compileFunction_least_cycle`, which need a
  continuity proof over the denotation), the connection between native
  storage and verification values, the remaining global-preservation
  hypotheses, `FailuresIntegral`, `RunsAgree`, `OwnersRepresentable` for
  units whose owners are all tables, and `DataInvariantsSatisfiable`, which
  a witness per declaration would discharge.
- **Denotation coverage not carried.** Signed bitwise operations, `bytes`
  and fixed-length vectors, unspecified pure callees used as summaries,
  references inside an aggregate, range patterns over characters; generics:
  the storage-aliasing instances, a generic caller of a callee whose storage
  depends on a type parameter, generic storage instantiated at a struct, the
  Rust profile denotations. Unmeasured: kernel `rfl` against `simp only
  [denote]` as the unfolding certificate, fuel over the arena against a
  reified tree.
- **Quantifier automation.** Instantiating every position read in a context
  is costly: target relevant patterns and share preparation without
  enlarging budgets. Conditional self-referential equations cannot drive
  recursive simplification.
  A loop invariant `∀ j < i, …` over a vector is not instantiated at the
  element the body observes (`v.values[i]? = some val`), so the iteration
  leaf lacks the invariant at `j = i`; the instantiation of a *data*
  invariant at an observed element (`observedOwnInstances`) does not cover a
  loop invariant. At 821af8289ba against main: 23 headlines (18 clauses),
  7 functions in `vector_hofs_for_each` and `vector_hofs_mut_receiver`.
  Reproducer: `move-prover/tests/sources/functional/closures/inline/vector_hofs_for_each.move`,
  `increment_all`.
- **Abstract maps (A4).** `big_ordered_map`'s use-case tests and their cost;
  the construction, representation-reading and identity-update diagnostics;
  an equation between a nominal literal and a model operation projected
  onto its fields. BigOrderedMap's unsupported iterator models and
  mutable-reference behavioral predicates stay separate from the cost goal.
- **Unsupported forms.** The registry
  [`prover-test-problems.md`](../status/prover-test-problems.md) lists the
  denotation, native-model, generic-aliasing and closure gaps; keep
  explicit unsupported diagnostics and intended negatives while adding
  support.
- **Intrinsic maps, agreement side.** Re-check the roles where the model
  is exact and the Prover bounded (`values`, `append`, `upsert_all`,
  `trim`, `replace_key_inplace`) against each body before assuming its
  refinement; state `NativesTyped` of the host's natives and prove it of the
  table roles; prove the table-layout role contracts the runtime provides
  from the model; `ValueHasType` types no owner's entries; specification
  equality at a type parameter is structural where an instantiation makes
  the values tables, which the Prover compares by identity.
- **Static memory.** Prove `GlobalsPreserved` from `preservation`;
  `ResourcesTyped` is undecided for a unit with an enum or generic resource
  (V22), so such a unit's unseen function values are not carried; a stored
  invariant about a function value read from storage is not used
  (`stored_fun_values`, `use_reader`).
- **Ghost fields (G4b).** The unit model as one least fixed point, its
  assembly by fixed-point induction, the agreement up to ghost erasure;
  invocations lending mutable references, `result_of` of an unseen
  closure, and closures of generic targets in the model; `grind` reports an
  unknown free variable on two leaves of `loop_memory_havoc`; the remaining
  C8 tests; `big_ordered_map`'s iterator targets (8 of 43 verify).
- **Source verification gaps.** Import and validation errors report at the
  input path's line 1 (byte ranges unmapped); a residual obligation without
  a clause marker lands on the module, not the function; a clause's message
  quotes its VerexSL rendering, not the source; `verex.skip_verify` cannot
  be set for the language server until leanprover/lean4#15626 lands.
- **Static typing, remaining items** ([`static-typing.md`](../historical/static-typing.md)):
  rows of tuples, vectors and nominal types for an unseen invocation, typed
  today only where the frame resolves them to scalars (`ScalarAt`); a proof
  that a call settles the loans it mints, so memory after a call is
  hole-free without a defensive exit check.

## 2. Verification across modules

Design: [`relocatable-namespaces.md`](../designs/relocatable-namespaces.md).

- **R2, compiled namespaces everywhere.** `scripts/verify-modules.py` compiles a
  package's namespaces to `.olean` files and each module imports them. Open: the
  Rust driver's pass over crates (the same `compileNamespace`), and `verex-move
  verify <package>` and `move prove --lean`, which still elaborate all in one
  rendering.
- **R3, proof reuse.** An opaque callee of another module is verified again
  in every importer, and every linked module's unit certificates are checked
  again (7.5 s for a client of `ordered_map`). Needs the semantics invariant
  under consistent renumbering, or semantics over namespace-local identities.
- **R4, shipped namespaces.** The exchange format carries relocatable
  namespaces so a frontend ships a precompiled package.
- **What a module is verified against depends on the export.** A target's
  rendering imports every namespace rendered before it in its own export,
  because those decide which modules' invariants its unit links
  (`invariantRelatedModules`). So a module rendered after it, whose
  invariant reads memory the target writes, is not linked, and the function
  is verified against that invariant in that module's own command instead,
  which the rendering must therefore elaborate. Compiled namespaces keep
  this. The fix is a package-level memory summary, computed before any
  target, that makes the related set a function of the package.

## 3. The Move standard library as the corpus

Design: [`source-verification.md`](../designs/source-verification.md).

- Reduce proof and context-preparation cost (track 1); reuse imported
  proofs (R3); locate residual obligations without a clause marker at the
  function rather than the module header.

## 4. Shared checking

Design: [`vir-design.md`](../designs/vir-design.md).

- The checked trait-selection evidence table, associated-item
  normalization and dictionary semantics (`EvidenceId` is an opaque slot),
  before trait calls are interpreted or trait contracts transfer proofs
  (also blocking the Rust frontend).
- **Dependency interfaces as checked data.** A unit holds one `Tables`
  snapshot with a declared interface for every external import. Dependency
  interfaces (exported declarations, signatures, visible contracts,
  intrinsic declarations; [`vir-design.md`](../designs/vir-design.md),
  "Compilation units and dependencies") are not resolved authoritatively
  across namespaces, so dependency signatures or bodies cannot take part in
  typing, interpretation or verification. Changes `Validation/Resolve.lean`
  and `Validation/Link.lean`.
- The certification pass that turns `prepare*` into pure mode filters.
- Borrow checking: general non-lexical loan death, dependency and unknown
  aggregate alias propagation, nominal variance, use-site region solving.
- Lifetime constraint generation and solving; authoritative whole-unit type
  checking.
- Promote the Move constructs the XAST adapter still encodes as tags, and
  the known Move event and Rust cell and drop operations, to core.
- Import receipts binding evidence to source, configuration and semantic
  hashes; until then a frontend's `checked` label is a producer assertion.
- Materialize `StructuredUnit`/`ResolvedUnit`/`TypedUnit` and the full
  staged API; strict external JSON compatibility of RawUnit, only when a
  consumer outside this repository depends on it.
- Prophetic residue: a loan-typing environment for walker-level
  preservation through a live borrow; non-lexical loan death; a design for
  two-phase borrows and interior mutability before such constructs are
  admitted.
- The secure Lean-source discovery TODO in `vir-design.md`, before
  Lean-source discovery is a production compiler feature.

## 5. Runtime semantics

Design: [`elaboration-design.md`](../designs/elaboration-design.md).

- No-stuck for prepared invocations, the open part of the big-step
  metatheory (determinism and completeness of the interpreter up to fuel are
  proved, `Proofs/Completeness.lean`).
- **Stated obligations without a theorem**
  ([`elaboration-design.md`](../designs/elaboration-design.md), "Correctness
  obligations"): Move abort finalization rolls back exactly the required
  state; VIR → XIR lowering preserves outcomes (`VerexMove/Xir/Lower.lean`);
  interpreter outcomes, errors and generated obligations keep their VIR
  locations. None is a theorem, so a claim resting on them is unproved.
- The unsupported primitive and reference subset, trait and predicate
  satisfaction, non-nominal whole-unit type well-formedness, the remaining
  declaration families.
- Field projection through a mutable global borrow (`&mut Coin[addr].value`)
  lowers to a `select` on a borrow value the evaluator does not define; the
  whole-resource borrow is the supported form until the lowering emits the
  place-based pattern.

## 6. MonoVM differential harness

Design: [`monovm-link-design.md`](../designs/monovm-link-design.md).

- M5: resource state in the request and writes, deletions and events per
  call in semantic equality; mutable-reference out-parameters read back from
  the root frame and from the interpreter's reference outputs (no `--args`
  spelling for a `&mut` argument yet).
- Findings to settle: the engines classify arithmetic overflow differently
  (MonoVM `failed InvalidOperation`, Verex abort 300); `vector::pop_back`
  needs a value-yielding statement sequence the printer cannot render;
  MonoVM does not compare function values.
- Stdlib vertical: `pop_back`, `move_range`, `destroy_empty`, borrows
  through struct fields. Deferred: type arguments on generic entries, script
  transactions and publishing, shared named-address assignments, gas and GC
  modeling, the legacy-VM arbiter step, non-Linux link support.

## 7. Rust frontend

Design: [`rust-mir-design.md`](../designs/rust-mir-design.md); proposal
[`unsafe-pointers.md`](unsafe-pointers.md).

- The M0 gate is blocked upstream: `FnDef` exposes no predicates and there
  is no const-fn binder type query
  ([rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892));
  the answer stays an upstream query, not a private `rustc_middle` read.
- Then M1 mapper completion and Charon differential tests; M1.5 a prebuilt
  exporter and sysroot distribution (today the managed path builds the
  exporter from the checkout); M2 contracts and `verify` for Rust through
  `denote`, constrained and const-generic function metadata, semantic
  normalization of the remaining validation-only bodies, generic function
  contracts and evidence, proof summaries (general range indexing stays
  behind the standard-library dependency model); M3 regions over the
  prophetic model: dependency and unknown-aggregate alias propagation,
  nominal variance, the remaining use-site constraints, returned-reference
  region precision (argument-changing recursive and external result-source
  selection stays conservative); M4 drops and panics: cleanup edges,
  `Resume`, effectful drop glue and drop flags are rejection boundaries
  today; M5 the unsafe profile; M6 library models; M7 alignment proofs.
- Milestone contents the design no longer spells out: M0 is the exporter
  and import contract; M1.5 build integration with a managed toolchain. M3
  also covers moves and initializedness in the VIR borrow checker, with
  rustc's borrow check kept as admission evidence and differential oracle,
  and positive/negative agreement tests against Rust execution. M4 covers
  `panic=abort` only; unwinding and `catch_unwind` need their own approved
  milestone. M6 is reference-bearing values and interior mutability beyond
  library models: references nested in products and ADTs (the denotation
  rejects a reference inside an aggregate), signature region relationships
  at modular boundaries, and an abstract `RefCell` store with dynamic borrow
  state and guard drop ([`rust-mir-design.md`](../designs/rust-mir-design.md),
  "Interior mutability and `RefCell`"); it needs the two-phase and
  interior-mutability design of §4 first. M7: a theorem that generic
  type/const/evidence instantiation preserves function meaning, so generic
  theorems transfer to concrete calls, and import alignment composed with
  `f.verified` into a theorem about the Rust MIR body.
- Mapper forms still rejected: symbolic const-dependent types; destructive
  and provenance administration, pointer-alignment assertions and reachable
  `unreachable` blocks (M4 for destruction and cleanup, M5 for retag,
  provenance and alignment).
- Function values in Rust: the native model of
  [`higher-order-functions.md`](../designs/higher-order-functions.md),
  "Function values as native data", read for Rust — a closure type as a
  struct of its upvars, `dyn Fn*` as the link set's closure structs plus
  `foreign`, function pointers as nullary constructors per target and
  instantiation — is designed and not implemented.

## 8. Surface and backends

Designs: [`verex-sl.md`](../designs/verex-sl.md), [`vir-design.md`](../designs/vir-design.md).

- VerexSL forms designed but not implemented: profile headers beyond Move
  and Rust (`using none`, registered extensions); cross-profile adapters
  (declaration overrides, `&[rust] …`, calls, summaries); the trait and
  implementation surface with checked implementation selection; loop labels
  and profile-specific control recovery in the printer; full interpretation
  of conditions, frames, quantifiers and the spec-operation vocabulary;
  typed schema fields for function modifiers; general intrinsic declaration
  syntax; `GenericBinder` for namespace axioms and invariants; proof-local
  labels; a Rust-profile guide.
- A Move producer that builds raw VIR before compiler-v2 rejection; the
  XAST compatibility frontend's removal criteria.
- Normalization of validation-only bodies, Move/Verex-native
  alpha-equivalence, source-map coverage, cross-language location fidelity,
  corpus-wide round trips.
- A compiler-correctness theorem connecting source verification to the
  emitted bytecode.
- VerexSL type spellings `EventStore`, `TypeDomain<T>`, `ResourceDomain<R>`
  and `profile_type[p, tag, payload]` are in the `verex-sl.md` grammar but
  have no parser keyword, and the printer rejects them (`VerexSL/Print.lean`),
  so units with such types cannot round-trip through VerexSL. Changes
  `VerexSL/Syntax.lean`, `Elab.lean`, `Print.lean`.

## 9. Higher-order functions and state labels

Designs: [`higher-order-functions.md`](../designs/higher-order-functions.md),
[`state-labels.md`](../designs/state-labels.md),
[`ghost-fields.md`](../designs/ghost-fields.md).

- **H4.** Invoking an unknown closure and the closed-world theorem (H4c);
  a state label witnessed by `&mut` predicates of an inlined target,
  invoking an unseen function value with `&mut` parameters, and labels
  defined by such invocations (H4d; `prover-test-problems.md`, V20); frames
  without `modifies_of` as generated preconditions.
- **H5.** The re-entrancy lock.
- **Data invariants over function-valued fields.** A clause stating the
  behavior of a stored function value (`aborts_of`, a field frame) depends on
  memory, so it cannot be the type of a stored value; a known Move Prover
  issue. Probable solution: read such clauses at the big-step semantics,
  over runtime states, which mention no carrier — a spike
  ([`data-and-global-invariants.md`](data-and-global-invariants.md),
  "Behavioural clauses inside a data invariant").
- **C4.** Prover tests `closures/stored_fun_values` and
  `behavioral_target_two_masks`.
- **Closure frames of open generic targets; `result_of` and wildcard frames
  under state labels.** Focused probes around `functional/closures/amm_example.move`
  found proof gaps in both; the checked regressions cover only concrete
  generic instantiation and `ensures_of` invariants. No fixture isolates
  either gap.
- **Results as rows.** Function values of tuple-returning functions: `ResultShape`
  gains a row case and a tuple result reads as its elements with their sharing
  marks. Until then such a value is refused where it is constructed: "a function
  value of `f` is not supported: `f` returns a tuple"
  ([`higher-order-functions.md`](../designs/higher-order-functions.md)).
- Unsupported, diagnosed: explicit enum-field write frames; write frames at
  dependency-interface boundaries. Not scheduled: closures whose targets
  return references; function types as type arguments
  (`Option<|u64|u64>`); Rust capturing closures and `Fn*` traits.
- **State labels.** Reads of a removed resource's contents under a label
  (`unmodified_memory_at_label::swap`); a free label with no defining
  operation (`nonlinear_cfg_error`); a witness for a target without a
  theorem, and a positive `aborts_of`, which no rule establishes from a
  contract; the remaining `state_labels/` failures (`spec_functions`,
  `spec_fun_mixed_memory`, …).
- **Ghost fields (G4b).** Modular theorems, the unit model as one least
  fixed point, the agreement up to ghost contents; generic functions and
  closure targets in the model; the remaining C8 tests.
- **Ghost memory through `modifies *`.** A callee with frame `modifies *`
  leaves every spec variable unconstrained, where the Prover never havocs
  ghost memory, so `ensures supply<T> == old(supply<T>)` fails across such a
  call. The loose and open frames would gain one conjunct: every ghost
  resource instantiation other than the function's targets reads the same,
  owed by the callee and discharged by the `ghostWrites` fixed point; a
  callee of unknown body gets no such conjunct. It adds an obligation to
  every memory-writing function; size it first.

## Not scheduled

- A multi-party freeze of RawUnit JSON v1: the "no construct unclassified"
  gate is mechanized by `Capability.lean`
  ([`vir-design.md`](../designs/vir-design.md), "Boundary classification and
  adapters"), and publishing v1 becomes work only when an external consumer
  depends on it.
- Rust features outside the profile: trait objects, specialization,
  higher-ranked bounds outside the initial fragment, async and coroutines,
  FFI, inline assembly. Each needs a core/profile classification and a
  model; the exporter rejects them at the first node.
- The design documents inside `v0/` describe the deprecated stack; nothing
  is scheduled from them.
- **Per-target result reuse (S1).** Not a Move Prover feature and no help to
  a fresh CI run. The proposal: key a verified result by the declarations
  its proof uses (the theorem statement, referenced constant types and the
  definitions reached transitively, toolchain, verifier build, options),
  not by source files, compared at theorem start; an opaque callee's body
  change leaves callers cached when its contract is unchanged, while the
  module audit always runs; recursive cycle groups are cached together.
  Failed and budget-stopped targets have no proof term, so they would take a
  conservative solver-visible environment key, with `--fresh` to bypass
  reuse. Unresolved: how importing cached results avoids stale or
  conflicting declarations (import-and-alias against kernel replay), and
  what validation, loading and replay cost.
- **Sharing across authored points.** Arrivals waiting at different program
  points of an authored proof (`verex_obligations at`) do not share a join's
  suffix (`partitionArrivals`). They could, with the suffix running each of
  their hooks, as a suffix already owes the union of its arrivals' clauses.
  Open: whether a hook's result holds for arrivals that did not pass its point.
