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
  move; the transport of a definition taking types but no frame (SC2);
  addresses as `u256` values.
- **Quantifier syntax aligned with Move.** VerexSL's `∀ (i : T), P` should
  move toward Move's spelling, `forall i: T @ P`, with `forall` and `@`
  also accepted as symbol tokens.
- **Function values as native data**: [`native-function-values.md`](native-function-values.md),
  steps 2b and 3. Ordered with the proof language
  ([`proof-language.md`](proof-language.md)): that plan's L0–L2
  interleaved with steps 2b and 3, then the language's function-value
  constructs (its L4). L0's keys of calls are done; its path decisions and
  selectors are next.
- **Function-typed variant fields (open decision).** A specification that
  reads an enum variant's function-typed field as a value has no native
  read (`let f = old(State[a]).Continuation.0` in the paper example
  `calculator`): the read under another variant needs a value of the
  field's type, and a function type has none by its type alone
  (`NTy.inhabitable`), and the function carrier interface of
  [`native-function-values.md`](native-function-values.md) provides none;
  a behavioral predicate of such a field is reported as not carried.
  Either the read stays partial under its variant guard, or the carrier
  provides a value of the type.
- **Turned off, to fix.** The benchmark leaves out the paper example `amm`
  (`higher-order-paper-26/examples`): its authored proof
  (`amm_example.proof.lean`) is stated over runtime values and is written
  again in the proof language once closures are data
  ([`native-function-values.md`](native-function-values.md), steps 2b and
  3). `pool_u64::deduct_shares` sets `pragma verex_verify = false`: the
  obligation that a removed shareholder is no longer in `shareholders` (a
  quantified read after `eraseIdx`) falls to the general pipeline, which
  does not decide it; its authored proof is written in the proof language.
- **Key-instance spelling.** The normal form spells the `DecidableEq`
  instance of a concrete key type as `NTy.decEq carriers τ` (typed views,
  `NTy.readDecEq`); a term elaborated outside the normal form gets the
  synthesized instance (`instDecidableEqString` for an address). They are
  definitionally equal, not syntactically. Integer keys already use the
  synthesized instance. A reducible `NTy.decEq` changes
  discrimination-tree keys and breaks automatic proofs (`bitwise_table`,
  `data_invariant_in_loop`); declaring it an instance does not help.
  Decided for authored proofs: the proof language's elaborator spells the
  instance as the leaf does ([`proof-language.md`](proof-language.md),
  "Semantics and lowering", 2); until then a script works on the leaf's
  own terms (as `pool_u64`'s `redeem_shares` does). Open for the normal
  form: whether the synthesized instance at every closed concrete type,
  through the typed reads and the frame transports, is worth its change
  to the automatic proofs.
- **Cost (P3, P4).** Re-measure the benchmark, `DenotePerformance`, and the
  map modules on native leaves; the P4 goal is all 38 BigOrderedMap targets
  below roughly 1.9G heartbeats including authored proofs. Then the levers of
  [`performance.md`](../designs/performance.md), "Open levers", in its order,
  and the opt-in join scheduler (`verex.sharedContinuations`) enabled for
  automatic verification or removed. Debts:
  [`measurement-debts.md`](../status/measurement-debts.md).
- **Assumptions.** Discharge the hypotheses in
  [`assumptions.md`](../status/assumptions.md): the compilation agreement
  (`compileFunction_agrees`, `compileFunction_least_cycle`, which need a
  continuity proof over the denotation), the connection between native
  storage and verification values, the remaining global-preservation
  hypotheses, `FailuresIntegral`, `RunsAgree` and `OwnersRepresentable` for
  units whose owners are all tables.
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

- **R3, proof reuse.** An opaque callee of another module is verified again
  in every importer, and every linked module's unit certificates are checked
  again (7.5 s for a client of `ordered_map`). Needs the semantics invariant
  under consistent renumbering, or semantics over namespace-local identities.
- **R4, shipped namespaces.** The exchange format carries relocatable
  namespaces so a frontend ships a precompiled package.

## 3. The Move standard library as the corpus

Design: [`source-verification.md`](../designs/source-verification.md).

- Reduce proof and context-preparation cost (track 1); reuse imported
  proofs (R3); locate residual obligations without a clause marker at the
  function rather than the module header.

## 4. Shared checking

Design: [`vir-design.md`](../designs/vir-design.md).

- The checked trait-selection evidence table (also blocking the Rust
  frontend).
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

- The rest of the metatheory of the big-step semantics: determinism,
  completeness of the interpreter up to fuel, and no-stuck for prepared
  invocations.
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
- **C4.** Prover tests `closures/stored_fun_values` and
  `behavioral_target_two_masks`.
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

## Not scheduled

- Phase 0 of `vir-design.md`, a multi-party freeze of RawUnit JSON v1: the
  "no construct unclassified" gate is mechanized by `Capability.lean`, and
  publishing v1 becomes work only when an external consumer depends on it.
- The design documents inside `v0/` describe the deprecated stack; nothing
  is scheduled from them.
