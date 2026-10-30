# Prophetic reference semantics for validated VIR

References in validated VIR follow a prophecy-based ownership model, run
uniformly by the interpreter, the big-step relational semantics, and the
verifier (D5 of [`denotation.md`](denotation.md)). Open work:
[`roadmap.md`](../plan/roadmap.md).

## 1. Decision

After validation, references are not pointers. A mutable borrow transfers
ownership of the borrowed value into the reference; the lender's place is
left with the borrow's *prophecy*, the value the loan will have at its
death, and the death of the loan assumes the reconciliation equation
`current = prophecy`. Shared borrows are observations: plain values.

This is the RustHorn encoding (Matsushita/Tsukada/Kobayashi). It covers
nested references, reborrowing, and reference-returning functions, and it
makes aliasing unrepresentable after validation: exclusivity is
structural, so no separation logic and no per-point disjointness reasoning
is needed anywhere in the verifier.

Everything downstream of validation runs on this one model: the big-step
relation and the fueled interpreter run the same deterministic rules
(borrows leave loan holes, deaths write the current value back, dying
frames export unreconciled loans through the state's pending set), and the
contract layer symbolically executes that judgment, so a reference
contract names each loan's final value through the pending exports, with
no refinement gap to a separate pointer semantics.

The borrow analysis remains the trust boundary: its certificate licenses
the elimination of pointers. A program the analysis cannot certify never
reaches the prophetic model: preparation refuses a unit with a rejected
function, and verification refuses to state theorems about it
(`ensureUnitDefinition`).

### Options considered

1. **Concrete heap with certificate bridge theorems** (exclusivity and
   no-dangling proved from `BorrowCertificate`, prophecies as a proved
   refinement on top). Sound, but every reference proof pays for the heap:
   store typing over slots and generations, frame reasoning at calls, and
   a permanent refinement obligation between pointers and values.
2. **Write-back paths in the reference value** (Move Prover / Aeneas
   backward style, no prophecy). Executable and local, but
   `fn pick(a: &mut T, b: &mut T) -> &mut T` makes the write-back
   destination dynamic, and callee summaries must speak about paths
   written rather than values related: the frame problem re-enters at each
   call. Retained only as the erasure device inside the interpreter, never
   in the semantics of specifications.
3. **Prophecies throughout**: chosen.

## 2. The model

### 2.1 Values

Program values use two reference constructors:

```lean
| borrow (loan : LoanId) (current : RuntimeValue)
| loanHole (loan : LoanId)
```

- A **mutable borrow in flight** is `borrow ℓ v`: it owns the current
  value. `LoanId` is the stable loan identity minted by validation (§3);
  it names the loan in the interpreter's write-back and is never
  observable by executable code. `borrow` carries no reference kind: it is
  always a mutable loan.
- A **hole** `loanHole ℓ` sits where the borrowed value was taken from:
  inside a local, a vector element, a nominal field, or a global slot.
  Well-formedness (each live `ℓ` has exactly one hole and one borrow) is an
  invariant the semantics preserves and the certificate guarantees at
  construction.
- A **shared borrow** is a copy of the observed value: no constructor, no
  hole, no loan minted. Exclusivity makes the snapshot sound: nothing
  mutates the lender while shared loans live. `&signer` at Move entry
  boundaries is a signer value. A dereference or freeze reads a shared
  operand by its recorded type (`sharedOperandAt`), and validation records
  each function's places that dereference a shared reference in its borrow
  certificate (`sharedDereferences`), which place resolution and
  compilation read as their bases. Typing has the transparency rule
  `value : T` implies `value : &T`. Loan identity therefore never leaks
  into equality.
- A **mutable borrow** compared by `==`, `!=`, or `compare` is read as its
  current value (`RuntimeValue.observe`), as Move and Rust compare
  references by their referents, so no comparison reads a loan identity.

### 2.2 Prophecies

The big-step relation attaches to every mutable-borrow creation a prophecy
value, universally quantified in the rule:

- **Borrow** `let b = &mut place`: with `v` the value at `place`, the rule
  yields `b = borrow ℓ v` and, relationally, stores the prophecy `π ℓ`
  into the lender's hole immediately. The lender's continuation computes
  with the final value of the loan, which is the point of the encoding.
- **Dereference / mutate** touch only `current`.
- **Death**: the rule's premise is the reconciliation `current = π ℓ`. A
  derivation exists only for the correct guess, so the relation stays
  deterministic in its observables.
- **Reborrow** `let b' = &mut *b` with `b = borrow ℓ v`: fresh loan `ℓ'`,
  `b' = borrow ℓ' v`, and `b` continues as `borrow ℓ (π ℓ')`: the outer's
  current becomes the inner's prophecy. Nested references need nothing
  further: borrows nest as values nest. A marker ending several loans
  reconciles them in decreasing instance order, since a reborrow is minted
  after the loan it projects, so a hole is settled before the value
  holding it moves.
- **Freeze** (`&mut` to `&`): a shared reborrow yielding the observation
  `v` of the current value; the mutable loan stays live and is reconciled
  where its holder dies, so writes through the mutable reference after
  the freeze reach the lender. Killing the loan at the freeze would lose
  those writes.
- **Returned references**: a reference crossing a call boundary is an
  ordinary value. The callee's summary relates argument and result
  borrow/prophecy pairs; `pick` needs no special treatment because which
  prophecy got tied to which result is part of the relation, not of any
  path. Executably, callee finalization exports
  `(outerLoan, loanHole returnedLoan)` and caller reconciliation reads that
  hole to transfer the dynamic identity; nothing recovers an owner root or
  projection path from a reference. Loans carried by a normal function
  result are excluded from the deaths recorded at the function's exits, so
  their identities cross the call boundary.
- **Unheld results**: death reads the current value from the local that
  holds the reference. A reference a call returns that no local holds (a
  discarded result, one a wildcard matches, the operand of a dereference
  or a field read, the base a borrow selects from) is bound by validation
  to a local of its own (`holdMutableReferences`), which holds it until it
  dies; the printers spell the binding back as the source does
  (`ValidatedNamespace.holders`). A mutable borrow of a place that dies
  unheld (`*(&mut x)`, `(&mut x).f`, `let _ = &mut x`) writes nothing:
  once the borrow analysis has checked it as a mutable borrow, validation
  reads it as a shared borrow of its place, which mints no loan
  (`foldPlaceBorrows`). A mutable reference copied by name out of a local
  into a call's parameter or another local is a reborrow of that local, or
  a move where the copy is the local's last use
  (`normalizeReferenceCopies`), so one loan rests in one holder.
- **Globals**: `borrow_global_mut` leaves the prophecy (relationally) or
  the hole (executably) in the global slot. `RuntimeState.storageLoans`
  records the storage key of a live global loan and rekeys it from an
  outer loan to a returned reborrow. The key registry is write-back
  metadata, not part of the reference value.

### 2.3 Executable erasure

A deterministic interpreter cannot guess `π ℓ`. It runs the same rules
with the ghost erased: a borrow leaves `loanHole ℓ`; a death writes
`current` back into the unique hole `ℓ`; reconciliation holds by
construction. The agreement theorem (`run_sound`/`run_complete`) states
that interpreter runs and big-step derivations coincide: completeness
constructs each prophecy witness as the value the run wrote back at that
loan's death, and soundness erases ghosts. There is one semantics, with a
ghost annotation.

## 3. Validation: loan elimination

Validation runs the borrow analysis once and retains `BorrowCertificate`
(parameters, loans, lifetime closure):

1. **Loan ids.** One `LoanId` per accepted loan site.
2. **Deaths.** Loan death is lexical; the analysis records, per accepted
   loan, its death points as it releases the loan: after an anchor
   expression completes, or before one begins (`CheckedLoanFact.deaths`),
   at ordinary fallthrough and at explicit return and throw exits. The
   semantics ends the recorded loans at their anchors (`loanDeathsAt`):
   `EvalExprWith` settles the loans dying before a node, evaluates the node
   (`EvalNodeWith`, which a loop repeats), and settles the loans dying
   after it once it produced a value; compilation wraps the node's term the
   same way. Printers, the runtime, and verification read one unit with one
   numbering. When non-lexical loan death lands, only the placement of the
   deaths moves; the semantics does not change.
3. **Rejections.** Two-phase borrows, interior mutability, raw-pointer
   escapes, and shared-of-mutable overlaps beyond the supported reborrow
   discipline are explicit unsupported diagnostics at validation; such code
   never receives the prophetic semantics silently. A dereference place
   whose base type cannot be recovered is left unrewritten and reports
   stuck at run time rather than guessing a kind.

Carrier invalidation in the analysis is the directed `placeCovers`: a
write through a dereference does not release the dereferenced reference's
own loan.

## 4. Verification

A `&mut T` parameter contributes a loan binder and an entry/exit value
pair: in a one-state clause the bare name is the entry value; in `ensures`
the bare name is the exit value and `old` reaches the entry. The argument
row encodes the entry, the pending-push equation (in parameter order)
names the exit, and a `holeInGlobals … = false` precondition holds per
mutable loan. A `&T` parameter is a loan binder plus one unchanging value.
A contract names each dying loan's final value through the pending
exports (`final.pending = initial.pending.push (loan, final_value)`), and
a caller resolves its holes by `applyPending` at the call boundary.

The hole-visibility scans (`holeWithin`, `holeInFrame`, `holeInGlobals`)
are guarded by their predicates (`fillVisibleHole`), so a refuted
predicate decides a branch without the scan computing on a symbolic
state, and `holeInFrame`'s evaluation equation is keyed on the frame
constructor so a symbolic frame never exposes a search. Death
certificates are closed, so the verifier never unfolds a generic loan
search.

## 5. Typing

The reference-vocabulary typing lemmas: dereference inversion
(`borrow_current_typed`), borrow introduction (`borrow_value_typed`),
freeze (`freeze_typed`), and hole writes preserving frame and state
typing (`typedFrame_set_loanHole`, `typedState_set_loanHole`); the shared
transparency rule has introduction and inversion lemmas. Walker-level
preservation of write-back and mutation through a live borrow is not
stated: a hole types at every type, so only a loan-typing environment
tying each hole's position to its loan's referent can carry it, and full
frame typing must be stated up to the dead placeholder values that
consumed borrows leave ([`vir-design.md`](vir-design.md), deferred-work
register).

## 6. Boundaries

- The Rust exporter is untouched: it exports borrow-checked MIR, and the
  model is post-validation.
- Differential execution against MonoVM changes observables for
  reference-typed signatures (ownership passing returns values where the
  VM returns pointers); the adapter translates at the boundary, and a case
  it cannot translate is skipped with an explicit marker, never passed
  silently ([`monovm-link-design.md`](monovm-link-design.md)).
