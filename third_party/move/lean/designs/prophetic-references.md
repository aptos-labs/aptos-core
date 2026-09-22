# Prophetic reference semantics for validated LIR

Status: P1–P7 implemented. This document is the master
plan for replacing the concrete heap/generation reference model of validated
LIR with a prophecy-based ownership model, run uniformly by the interpreter,
the big-step relational semantics, and the verifier. Its milestone register
supersedes the M5 reference bullets of
[`elaboration-design.md`](elaboration-design.md).

## 1. Decision

After validation, references are not pointers. A mutable borrow transfers
ownership of the borrowed value into the reference; the lender's place is
left with the borrow's *prophecy* — the value the loan will have at its
death — and the death of the loan assumes the reconciliation equation
`current = prophecy`. Shared borrows are observations: plain values.

This is the RustHorn encoding (Matsushita/Tsukada/Kobayashi), the same model
the V0 stack (`move/Move/Semantics/Reference.lean`, `Verify/Borrow.lean`)
used successfully for full Move verification. It covers nested references,
reborrowing, and reference-returning functions, and it makes aliasing
*unrepresentable* after validation: exclusivity is structural, so no
separation logic and no per-point disjointness reasoning is needed anywhere
in the verifier.

Everything downstream of validation runs on this one model:

- the big-step relation and the fueled interpreter run the *same*
  deterministic rules: borrows leave loan holes, loan markers write the
  current value back at the recorded death point, and dying frames export
  unreconciled loans through the state's pending set (P3 note: the planned
  separate ghost judgment proved redundant — see §2.2);
- the WP/contract layer symbolically executes that judgment, so a
  reference contract names each loan's final value through the pending
  exports, with no refinement gap to a separate pointer semantics.

The borrow analysis remains the trust boundary: its certificate is what
*licenses* the elimination of pointers. A program the analysis cannot
certify never reaches the prophetic model.

### Options considered

1. **Concrete heap + certificate bridge theorems** (exclusivity and
   no-dangling proved from `BorrowCertificate`, prophecy layer as a proved
   refinement on top). Sound, but every reference proof pays for the heap:
   store typing over slots and generations, frame reasoning at calls, and a
   permanent refinement obligation between the interpreter's pointers and
   the verifier's values. Rejected as the end state; its artifacts
   (`Proofs/References.lean`) are subsumed.
2. **Write-back paths in the reference value** (Move Prover / Aeneas
   backward style, no prophecy). Executable and local, but a function like
   `fn pick(a: &mut T, b: &mut T) -> &mut T` makes the write-back
   destination dynamic, and callee summaries must speak about paths written
   rather than values related — the frame problem re-enters at each call.
   Retained only as the *erasure* device inside the interpreter (loan
   markers below), never in the semantics of specifications.
3. **Prophecies throughout** — chosen.

## 2. The model

### 2.1 Values

`RuntimeValue.reference`, `RuntimeReference`, `RuntimeReferenceTarget`,
`HeapSlot`, `RuntimeState.heap`, `RuntimeFrame.localSlots`, and the region
generation machinery are removed. Program values use two reference
constructors:

```lean
| borrow (loan : LoanId) (current : RuntimeValue)
| loanHole (loan : LoanId)
```

(P6 note: `borrow` carries no reference kind — it is always a mutable
loan, because shared references are fully erased at semantic
preparation.)

- A **mutable borrow in flight** is `borrow ℓ v`: it owns the
  current value. `LoanId` is the stable loan identity minted by validation
  (§3); it names the loan in ghost bookkeeping and in the interpreter's
  write-back, and is never observable by executable code.
- A **hole** `loanHole ℓ` sits where the borrowed value was taken from — inside
  a local, a vector element, a nominal field, or a global slot. Values with
  holes are exactly Aeneas' concrete-value discipline; well-formedness
  (each live `ℓ` has exactly one hole and one borrow) is an invariant the
  semantics preserves and validation's certificate guarantees at
  construction.
- A **shared borrow** is a copy of the observed value — no constructor, no
  hole. Exclusivity (certified) makes the snapshot sound: nothing mutates
  the lender while shared loans live. `&signer` at Move entry boundaries is
  therefore just a signer value. As of P6 this erasure is literal: shared
  borrows produce the bare value, shared dereference and shared-source
  freeze are rewritten to `copyValue` at semantic preparation (place
  dereferences of shared-typed bases collapse to their bases), typing has
  a transparency rule (`value : T` implies `value : &T`), and no loan
  instance is minted — so loan identity can never leak into equality.

### 2.2 Ghost prophecies

> **P3 consolidation.** As implemented, the ghost judgment described here
> is not a separate relation. P2 landed BigStep and the interpreter on one
> deterministic rule set (holes, write-backs, pending exports), under
> which every reconciliation premise below holds by construction and the
> agreement theorem is exactly `run_sound`/`run_complete`. The prophecy
> survives where it is observable: a function's contract names each dying
> loan's final value through the pending exports, and a caller's wp
> reasoning resolves its holes by `applyPending` at the call boundary.
> The rule-by-rule reading below remains the design rationale.

The big-step relation attaches to every mutable-borrow creation a prophecy
value, universally quantified in the rule, and threads a ghost environment
`prophecies : LoanId → RuntimeValue`:

- **Borrow** `let b = &mut place`: with `v` the value at `place`, the rule
  yields `b = borrow ℓ .mutable v` and — *relationally* — stores the
  prophecy `π ℓ` into the lender's hole immediately. The lender's
  continuation computes with the final value of the loan, which is the
  entire point of the encoding.
- **Dereference / mutate** touch only `current`.
- **Death** (an explicit marker, §3): the rule's premise is the
  reconciliation `current = π ℓ`. A derivation exists only for the correct
  guess, so the relation stays deterministic in its observables.
- **Reborrow** `let b' = &mut *b` with `b = borrow ℓ _ v`: fresh loan `ℓ'`,
  `b' = borrow ℓ' .mutable v`, and `b` continues as `borrow ℓ _ (π ℓ')` —
  the outer's current becomes the inner's prophecy. Nested references
  (`&mut &mut T`, references inside ADTs, `generic_lifetime_adt`) need
  nothing further: borrows nest as values nest.
- **Freeze** (`&mut` → `&`, explicit or implicit): kills the mutable loan at
  the freeze point — reconciliation `v = π ℓ` — and yields the shared
  observation `v`.
- **Returned references**: a reference crossing a call boundary is an
  ordinary value. The callee's summary relates argument and result
  borrow/prophecy pairs; `pick` needs no special treatment because which
  prophecy got tied to which result is part of the relation, not of any
  path. Executably, callee finalization exports
  `(outerLoan, loanHole returnedLoan)`. Caller reconciliation reads that
  prophecy hole to transfer the dynamic identity; it does not recover an
  owner root or projection path from the reference.
- **Globals**: `borrow_global_mut` leaves the prophecy (relationally) or the
  hole (executably) in the global slot. `RuntimeState.globalLoans` separately
  records the storage key for a live global loan and rekeys it from an outer
  loan to a returned reborrow. The key registry is write-back metadata, not
  part of the reference value; take/publish interact with holes only through
  certified exclusivity.

### 2.3 Executable erasure

A deterministic interpreter cannot guess `π ℓ`. It runs the *same* rules
with the ghost erased:

- borrow leaves `loanHole ℓ` as the hole;
- death writes `current` back into the unique hole `ℓ` (found through the
  validated frame-local address cache, the separate global key registry, or
  the semantic value-walk fallback — implementation details of
  `Operations.lean` rather than paths stored in the reference);
- reconciliation is trivially satisfied by construction.

The agreement theorem (the prophetic analogue of today's
`run_sound`/`run_complete`) states: interpreter runs and big-step
derivations coincide, where completeness *constructs* each prophecy witness
as the value the run actually wrote back at that loan's death, and
soundness erases ghosts. This replaces — it does not sit beside — the old
concrete/prophecy refinement obligation: there is only one semantics, with
a ghost annotation.

## 3. Validation: loan elimination stage

Validation already runs the borrow analysis once and retains
`BorrowCertificate` (parameters, loans, lifetime closure). A new final
validation stage consumes it:

1. **Mint `LoanId`s** — one per accepted loan site
   (`CheckedLoanFact.expression` keys them today).
2. **Materialize death points.** Loan death is currently lexical; the
   analysis records, per accepted loan, its death points as it releases the
   loan (a differential recorder at every expression boundary plus explicit
   records at the inter-child release sites), stored on
   `CheckedLoanFact.deaths` with two flavors: after an anchor expression
   completes, or before one begins. Materialization then synthesizes
   `endLoan` markers — an after-death wraps its anchor, a before-death
   becomes a zero-operand marker statement in a synthesized block around
   it. The `Operation` enum gains `reference .endLoan (loans : Array
   LoanId)` as a synthesized-only constructor: frontends never emit it and
   the raw checker rejects it, so the RawUnit wire schema is untouched.

   > Implemented 2026-08-28 with one deviation: markers materialize at
   > `prepareExecution`/`prepareVerification` (`markLoanDeaths`), not at
   > validation. The validated unit stays the marker-free authority that
   > printers and exchange consumers read — the surface printers
   > reconstruct loops from exact block shapes, which markers would break —
   > while everything that executes or verifies receives the marked copy.
   > Recording the deaths exposed and fixed a missed-conflict bug: a write
   > through a dereference released the dereferenced reference's own loan
   > (symmetric place overlap); carrier invalidation is now the directed
   > `placeCovers`. Marked arenas contain parent-to-appended-child forward
   > references; consumers are fuel- or relation-based.

   > A marker can end several loans at once, and the certificate lists
   > them in minting order. Reconciling them in that order writes a lender
   > back while it still carries the hole of a reborrow into it, so the
   > hole escapes with the lender's value. `endLoans?` therefore
   > reconciles in decreasing instance order: a reborrow is minted after
   > the loan it projects, so that order settles a hole before the value
   > holding it moves.

   When non-lexical loan death lands, only the placement of these markers
   moves; the semantics does not change.
3. **Reject what cannot be eliminated.** Two-phase borrows, interior
   mutability, raw-pointer escapes, shared-of-mutable overlaps beyond the
   supported reborrow discipline: explicit unsupported diagnostics at
   validation, per the change-discipline rule (never silently give such
   code the prophetic semantics).

`ValidatedUnit` keeps the certificate; `prepare*` no longer needs to replay
borrow diagnostics for the eliminated subset (the diagnostics-replay path
remains for what elimination rejects).

## 4. Changes by module

| Module | Change |
|---|---|
| `Syntax.lean` | `LoanId`; `ReferenceOperation.endLoan`. |
| `Semantics/Runtime.lean` | Value constructors §2.1; delete heap/slot/region/`localSlots`; `ValueHasType` reference case becomes borrow/loan cases (referent typing through the loan's certified type). |
| `Semantics/Operations.lean` | The one heavy rewrite (~57 heap/region sites). Place resolution loses stabilization and generations; borrow/deref/mutate/freeze/endLoan over values-with-holes; read/write/move/drop unchanged in shape. |
| `Semantics/BigStep.lean` | Rules for the reference operations gain the ghost prophecy environment and the reconciliation premise at `endLoan`; everything else is untouched (the heap never appeared here). |
| `Interpreter/Interpreter.lean` | Same rules, ghost-erased; write-back at `endLoan`. |
| `Proofs/Fuel.lean`, `Proofs/Interpreter.lean`, `Proofs/Completeness.lean` | Mechanical repair plus the prophecy-witness construction in completeness (§2.3). |
| `Proofs/References.lean` | Deleted; subsumed by the agreement theorem. |
| `Semantics/Typing.lean` | Preservation for borrow/deref/mutate/endLoan replaces the deferred pointer cases; store typing shrinks to frame + globals. |
| `Proofs/WP.lean` | Reference arms of `wpExpr_place` become dedicated prophecy rules; `wpExprStep` gains the `endLoan` arm. |
| `Proofs/Contract.lean` + new `Proofs/Mutation.lean` | Port V0 `Mutation`, `mutationWP`, `withMutation`-style scoped rules, prophecy-elimination simp lemmas (already present), and the derived vector-element/global borrow rules. |
| `LeanerLang/Contract.lean` | `&mut` parameters/results in generated contracts translate to (current, prophecy) binder pairs; `old(x)` is the parameter's entry value, the prophecy is its exit value. |
| Frame clause | Back to `final = initial`: no released heap slots to trim, `finalizeFunctionState` loses its heap pass. |

## 5. Proof plan

1. **Invariant.** Well-holed values (unique hole/borrow per live loan) as a
   state invariant; preservation through every operation.
2. **Agreement.** Prophetic big-step ↔ erased interpreter (§2.3), replacing
   `run_sound`/`run_complete` in one motion — same joint-recursor structure
   as the existing completeness proof.
3. **WP rules.** `wp` of borrow introduces `∀ π, … → (reconciliation → …)`;
   death eliminates it via the ported `forall_imp_eq_left` family. The V0
   rule set transfers nearly verbatim because the judgment shapes match.
4. **Typing.** `WfPrimitive`-style lemmas for the reference vocabulary; the
   pointer-width deferral note in `elaboration-design.md` is closed by
   removal.

## 6. Test and migration impact

- `Tests/References.lean` (~1000 lines) is rewritten to the new
  observables; the certificate-shape assertions stay.
- Rust e2e fixtures exercising the model today: `reference`,
  `mutable_reference`, `nested_reference`, `reference_composite`,
  `generic_lifetime_adt` — these become the acceptance tests for nested and
  composite borrows. Move corpus (`MoveStdlib`, `AptosStdlib`,
  `AptosFramework`) revalidates unchanged at the source level; baselines
  regenerate only where printed semantics mention reference operations.
- Rust exporter is untouched (it exports borrow-checked MIR; the model
  change is post-validation).
- **MonoVM differential suite** (owned by the suspended agent; this work
  now owns the whole tree): observables at differential boundaries change
  for reference-typed signatures — ownership passing returns values where
  the VM returns pointers. The adapter must translate at the boundary;
  until it does, reference-signature cases are skipped with an explicit
  marker, not silently passed.

## 7. Milestones

- **DONE — P1: Loan elimination.** 2026-08-28: `LoanId`, death recording in
  the borrow analysis, `endLoan` synthesis at preparation, and the
  value-transparent marker semantics; every suite green. The
  unsupported-boundary diagnostics beyond the existing borrow rejections
  move to P2, where the ownership semantics defines what cannot be
  eliminated.
- **DONE — P2: Ownership semantics.** 2026-08-28: §2.1/§2.3 across
  Runtime/Operations/BigStep/Interpreter with ghost-erased rules; heap,
  slot generations, and `localSlots` removed; dereference is a value
  projection into a borrow's current; call boundaries export dying loans
  through the state's pending set, applied where their holes become
  visible; the borrow analysis gained global-resource loan roots so global
  borrows record deaths; soundness/completeness/fuel/WP/typing repaired;
  the reference suites rewritten to ownership observables. All four suites
  green, including the Rust reference fixtures and the MonoVM
  differential path. Notes: value-level `mutate` updates the live borrow
  at rest or reconciles a consumed temporary; shared borrows carried a loan
  instance for uniform typing (superseded by P6's full erasure); `nextLoan` is run-local bookkeeping and
  excluded from differential state comparison; duplicated mutable borrows
  (never produced by borrow-checked profiles) remain trusted to the
  certificate rather than dynamically rejected.
- **DONE — P3: Prophetic relation and symbolic execution.** 2026-08-28,
  with one deliberate deviation: the separate ghost judgment planned here
  proved redundant. P2 had already consolidated BigStep and the
  interpreter onto one deterministic rule set, so reconciliation holds by
  construction and the agreement theorem *is* the repaired
  `run_sound`/`run_complete` — there is no second judgment to relate.
  The prophecy surfaces where it is used: a contract names each loan's
  final value through the pending exports
  (`final.pending = initial.pending.push (loan, final_value)`), and a
  caller's wp reasoning resolves it by `applyPending` substituting the
  callee's exported finals into the caller's holes. What P3 delivered is
  the machinery that makes this run under `leaner_wp`:
  - the loan-bookkeeping walkers became total — `rewriteFirst`/
    `findFirst`/`collectPruned` as mutual value/list well-founded
    recursions with precise first-match semantics, fueled
    `resolvePlace?`/`bindPattern` (arena-size fuel is exact on acyclic
    arenas), structural projection walkers;
  - a reduction/simp division of labor: whnf cannot unfold well-founded
    recursion, so `leaner_reduce` keeps the walker heads folded (arguments
    still reduce) and their registered `lir_data_norm` equations — plus
    the `List.findIdx?`/`List.findSome?` stepping lemmas — execute them in
    the simp phase; the equations are shape-conditional, so they never
    unfold against symbolic values;
  - `leaner_intro` peels constructor-wrapped operand equations by
    injection before substituting (and re-locates the hypothesis after
    `changeLocalDecl` rebinds its id); a progress-failing `leaner_reduce!`
    branch resumes ground reduction after substitutions;
  - normal-form seals ending reduce/simp oscillation: array literals
    (`List.toArray`), numerals (`OfNat.ofNat`, `Int.ofNat`), and the
    definitional connectives (`Not`, `Ne`, order notations) stay in simp
    normal form.
  Gate met: `LeanerLang/Tests/Verification.lean` verified `replace`
  (`&mut u64` parameter) end-to-end through `leaner_wp` in seconds, sorry-
  free, with the contract phrased as a pending export.  (`leaner_wp` and
  that hand proof were retired with the frame route on 2026-09-02; the
  same functions verify through the generated row route as the check
  `Check/Verification/Prophecies.lean`.)
- **DONE — P4: Verification layer.** 2026-08-28. Contract generation in
  `LeanerLang/Contract.lean` covers reference parameters: a `&mut T`
  parameter contributes a loan binder and an entry/exit value pair — in a
  one-state clause the bare name is the entry value; in `ensures` the
  bare name is the exit value and `spec.old` reaches the entry — with the
  argument row encoding the entry, the pending-push equation (in
  parameter order) naming the exit, exit-range facts, and a
  `holeInGlobals … = false` precondition per mutable loan; a `&T`
  parameter is a loan binder plus one unchanging value. The
  `#leaner_verify` command materializes the unit and generated contract
  and proves `SatisfiesFunction` by the scripted symbolic execution
  (`leaner_cases` destructs the precondition tracking hypotheses by
  name across substitutions; `leaner_wp!` feeds every hypothesis to the
  simp phases; a final `simp_all [and_assoc]` introduces the ensures
  witnesses). Semantics-side enablers: `holeWithin`/`holeInFrame`/
  `holeInGlobals` name the visibility scans, `fillVisibleHole` guards
  each scan behind its predicate so a refuted predicate decides the
  branch without the scan computing on a symbolic state, and
  `holeInFrame`'s evaluation equation is keyed on the frame constructor
  so a symbolic frame never exposes a search. V0's `Mutation`/
  `mutationWP` vocabulary was not needed: ownership passing plus pending
  exports carries the same content through the one judgment. Gate met:
  `LeanerLang/Tests/Verification.lean` proves the generated contracts of
  `replace` (`&mut`), `swap_in` (`&mut` with `old`), and `observe`
  (`&`), sorry-free. Reference-typed results and multiple `&mut` parameters
  were deferred here and are closed by P7. `spec.old` in a one-state clause
  remains the identity; exit-range facts are asserted by the contract and
  must be dischargeable from the body.
- **DONE — P5: Typing and register.** 2026-08-28. The reference-vocabulary
  typing leaf lemmas landed in `Semantics/Typing.lean`: dereference
  inversion (`borrow_current_typed`), borrow introduction
  (`borrow_value_typed`), freeze (`freeze_typed`), and hole writes
  preserving frame and state typing (`typedFrame_set_loanHole`,
  `typedState_set_loanHole`). Walker-level preservation — write-back and
  mutation through a live borrow — is deliberately not stated: a hole
  types at every type, so only a loan-typing environment tying each
  hole's position to its loan's referent can carry it; that environment
  belongs to the deferred M2 stage-5 evaluator induction and is recorded
  in `lir-design.md`'s register, together with the dead-residue caveat
  (`endLoan`/freeze leave placeholder values where consumed borrows
  rested). The old heap-generation pointer-width typing obligation closed
  with the heap's removal; `targetPointerWidth` survives only as the
  `usize`/`isize` width resolver. Register updates landed:
  `elaboration-design.md` reads the one-semantics model (no
  prophecy-refinement obligation, no heap in runtime state),
  `lir-design.md` gained the prophetic-residue row (loan-typing
  environment, non-lexical death, two-phase/interior mutability), and
  `rust-mir-design.md`'s reference section points here with the two Rust
  requirements discharged by construction.

- **DONE — P6: Shared-reference erasure.** 2026-08-28. The P2 deviation
  (shared borrows kept a `.borrow loan .shared value` wrapper for uniform
  typing) is closed: `RuntimeValue.borrow` drops its kind field and is
  always a mutable loan, shared borrows return the observed value with no
  loan instance minted, and freeze consumes the mutable loan into the
  bare snapshot. Because dereference and freeze cannot see their operand
  kind in a value-blind runtime, semantic preparation erases the shared
  vocabulary type-directedly: `prepareSemantics` (markers, then erasure)
  rewrites shared-operand dereference/freeze to `copyValue` and collapses
  shared-based dereference places to their bases, typing each base with
  the owning function's locals. `ValueHasType` gains the shared
  transparency rule, with introduction/inversion lemmas alongside the
  reference leaf lemmas. This closes the latent loan-id observability
  hazard: `.equal` compares values structurally, and shared snapshots or
  freeze results no longer carry loan ids that could make equal referents
  compare unequal. Everything the preparation computes stays in the
  whnf-reducible vocabulary (folds, structural fuel — not `Array.map` or
  `Std.Range` loops, which are well-founded and opaque), so a prepared
  unit remains a computable value for symbolic execution;
  `staticPlaceInfo?` and the place-field instantiation helpers were
  totalized on the way. Deferred: a dereference place whose base type
  cannot be recovered is left unrewritten and reports stuck at run time
  rather than guessing a kind.

- **DONE — P7: Native returned-reference transfer and explicit lifetimes.**
  2026-09-01. Borrow analysis records deaths at ordinary fallthrough and
  explicit return/throw exits; loans carried by a normal function result are
  excluded so their dynamic identities cross the call boundary. Semantic
  preparation materializes those decisions as explicit `endLoan` operations,
  and the native WP uses closed death certificates rather than unfolding a
  generic loan search. Returned local reborrows transfer identity through
  prophecy holes. Returned global reborrows additionally transfer the storage
  key in the separate `globalLoans` registry from the completed outer loan to
  the returned loan. The runtime reference itself remains exactly
  `borrow loan current`, with no owning root or projection path. The native
  fixture covers scalar forwarding, mutation after a call, dynamic selection
  from two mutable inputs, multiple results, projected results, and global
  results; all twelve positive functions have native-only gates, and a
  negative contract rules out an unrelated existential prophecy.

P1–P6 were checkpointed with the full matrix green. P7 is gated by the core
reference tests and the native-only reference verification/performance suites.
