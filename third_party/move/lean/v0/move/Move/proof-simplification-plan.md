# Plan: Simplifying the `Tests/Verification` Proofs

Status: proposal (not started)

This plan is based on a survey of all manual `verify … by` proofs and their
supporting private theorems in `Tests/Verification` (Quicksort, OrderedMap, Calls,
Loops, VectorOperations, Read, Generics, EnumPayloads, Vectors), together
with the library surface they use (`Move/Verify/Contract.lean`,
`Move/Verify/Borrow.lean`, `Move/Semantics/{Spec,Checked,Vector,Global,
Reference}.lean`, and the `spec`/`verify` elaborator in
`Move/Verify/Syntax.lean`).

## 1. The recurring patterns (evidence)

The manual proofs are dominated by a small number of idioms, repeated
almost verbatim:

**P1 — Contract-opening prologue.** Every manual proof starts with the same
shape: `unfold f.contract f.sourceSpec [f.bodySpec] Move.Verify.Satisfies`,
`intro` of type parameters / instances / `State`, `rintro ⟨args⟩ initial
permitted`, and for recursive functions `apply satisfies_fix[_of_wp]; intro
recursive recursiveVerified`. ~25 occurrences.

**P2 — Duplicated ok/aborts halves.** Proofs written directly against
`Satisfies` prove two obligations (`constructor; · intro result final
execution …; · intro code execution …`) and must re-normalize the source
spec in *both* branches. In `OrderedMap.borrow`, `add`, and `remove` the
abort branch replays the entire ok-branch `simp only [...] at
restExecution` chain (~40 lines each, twice). By contrast,
`quickSortRange` (Quicksort.lean:1254) is written in `wp` style and steps
through each call once. The `wp` route exists but is not used uniformly.

**P3 — Checked-arithmetic stepping.** 31+ occurrences of the block

```lean
have addStep : (Move.Semantics.Checked.addSpec x 1 :
    Move.Semantics.Spec State U64) =
  Move.Semantics.Spec.pure (U64.ofNat (x.toNat + 1)) := by
  apply Move.Semantics.Checked.addSpec_eq_pure
  change x.toNat + 1 < U64.size
  omega
rw [addStep, Move.Semantics.Spec.pure_bind]
```

plus the mirrored `by_cases safe : …` overflow split that then proves the
abort obligation by hand (Calls.lean `countdown`/`effectCountdown`,
Loops.lean `skipEvens`/`countdownTail` are four near-identical ~50-line
instances of this).

**P4 — Condition case-split dance.** 25+ occurrences of

```lean
by_cases h : Move.Verify.Source.logicalLT a b
· simp only [h, if_true]
  change a.toNat < b.toNat at h
  ...
```

and the loop-exit variant `change ¬0 < n.toNat at h; have := 
Move.U64.eq_zero_of_not_pos h; subst n` (14 occurrences).

**P5 — U64 numeral/`toNat` glue.** 35+ local facts of the form
`have oneToNat : (1 : U64).toNat = 1 := rfl`, `have twoNonzero : (2 :
U64).toNat ≠ 0 := by decide`, `apply U64.ext; change …; omega`, and
per-test lemmas such as `singletonVectorLength`, `fourElementVectorLength`,
`make3Length`, `insertEdgesResultFitsU64`.

**P6 — Element-at-index naming.** 12+ occurrences of

```lean
let entry := values.toList[i]'inBounds
have atI : values.toList[i]? = some entry := by simp [entry, inBounds]
rw [Move.Semantics.Vector.borrowElemSpec_eq_pure atI, Spec.pure_bind]
```

**P7 — Calling a verified callee.** Two competing styles: the `wp` style
(`wp_of_satisfies` + `wp_bind`, quickSortRange) and the raw style that
destructures `Spec.bind`'s existentials and projects `.1`/`.2` of the
callee's `verified` fact manually (`OrderedMap.contains/borrow/add/remove`,
`Calls.addTwiceThenOne`). The raw style is the source of most of P2's
duplication. What is missing for the `wp` style to always win is a
weakening lemma (adapting the callee's postcondition to the local one) —
proofs currently do that adaptation inline in both halves.

**P8 — Duplicated private library lemmas.** `withBorrowElemMutAssign_eq_pure`
appears verbatim in Quicksort.lean:787 and VectorOperations.lean:265.
`bindPureMap_ok_iff`/`bindPureMap_aborts_iff` (VectorOperations.lean:245)
are general `Spec` facts. `insertSpec_eq_pure` / `removeSpec_eq_pure`
(VectorOperations.lean:299/338) generalize lemmas already half-present in
`Move/Verify/Borrow.lean`.

**P9 — Prophecy/mutation plumbing.** Proofs touching `&mut` destructure
`withMutation` by hand: `rcases execution with ⟨future, reference, …,
referenceCurrent, _, outputFinal⟩` followed by `subst`s
(OrderedMap `add`/`remove`, Loops `drain`, VectorOperations
`insertEdges`/`removeMiddle`). `Move/Verify/Borrow.lean` already has
`withMutation_assign_ok_iff`-style rules but only for the two trivial
bodies (`assign`, `read`); there is no `wp` rule for a general body.

**P10 — Mutual recursion metatheorems.** `evenOddNoAbort` /
`heterogeneousNoAbort` and `evenOddPreservesState` /
`heterogeneousPreservesState` (Calls.lean:670-745) are pairwise-identical
scripts modulo the family name; the four `verify` proofs consuming them
are also four copies of one script.

**P11 — Hardcoded automation.** The automatic `verify f` command
(Move/Verify/Syntax.lean:2653) inlines a ~40-lemma `simp_all` list. Several
manual proofs (`Generics.hasGeneric/takeVault/takeBoolVault`,
`VectorOperations.setEdges/mutateThenBorrowOther`) are essentially "that
list plus two extra rewrites", but users cannot reuse or extend the list —
so they fork it by hand.

## 2. Design principles

- Everything is proved, nothing trusted: new tactics are macros over
  existing lemmas; no new axioms, no `sorry`.
- `Spec`-level equations live in `Move/Semantics/*`; `wp` rules and proof
  interface in `Move/Verify/*`; tactics in a new `Move/Verify/Tactics.lean`
  imported from `Move.lean`.
- One source of truth for automation: the automatic `verify` command and
  the user-facing tactics must draw from the same named simp sets.
- Tactic-generated hypothesis names follow the existing full-word style
  (`execution`, `recursiveVerified`, …).

## 3. Workstream A — library consolidation (quick wins, no new tactics)

1. **Promote duplicated test lemmas.**
   - `withBorrowElemMutAssign_eq_pure` → `Move/Verify/Borrow.lean`
     (delete both test copies).
   - `Spec.bind_pure_ok_iff` / `Spec.bind_pure_aborts_iff` (from
     `bindPureMap_*`) → `Move/Semantics/Spec.lean`, `@[simp]`.
   - `Vector.insertSpec_eq_pure` / `Vector.removeSpec_eq_pure`
     (Mutation-level, from VectorOperations) → `Move/Semantics/Vector.lean`.
2. **U64 numeral normal forms** in `Move/Basic.lean`:
   - `@[simp] theorem U64.toNat_ofNat_numeral (n : Nat) :
       (no_index (OfNat.ofNat n) : U64).toNat = n` (kills every
     `have oneToNat … := rfl`).
   - `@[simp] theorem U64.lt_iff_toNat_lt`, `U64.beq_iff_toNat_eq` so
     `<`/`==` on `U64` normalize to `Nat` facts without `change`.
   - `theorem U64.size_eq : U64.size = 18446744073709551616` (or keep
     `2 ^ 64` but make the `u64_omega` tactic below unfold it), removing
     the scattered `simp [U64.size]; omega`.
3. **Vector literal normal forms** in `Move/Basic.lean`: simp lemmas so
   `(vector![a, b, c]).toList` reduces to `[a, b, c]` and
   `Move.Vector.length (vector![…])` to the literal length (via
   `length_toNat` + `toList` computation + `U64.ext`). Deletes
   `singletonVectorLength`, `fourElementVectorLength`, `make3Length`.
4. **Indexed-element introduction lemma** (for P6), e.g. in
   `Move/Semantics/Vector.lean`:
   `theorem List.exists_getElem?_eq_some_of_lt … : i < xs.length →
    ∃ x, xs[i]? = some x` phrased for direct `obtain ⟨entry, atI⟩ :=
   getElem_some inBounds` use (exact form to be settled during
   implementation; the point is one step instead of `let` + `have` +
   `simp`).
5. **Mutual-recursion contract family lemma** (for P10) in
   `Move/Semantics/Spec.lean` or `Move/Verify/Contract.lean`: generalize
   `fixFamily_not_aborts` / `fixFamily_preserves_state` into one
   fixpoint-induction principle over a per-tag contract family
   (`satisfies_fixFamily`), from which both current specializations are
   corollaries. The four scripts in Calls.lean become two short instances.

Validation: `lake build Move && lake build Tests` after each deletion of a
test-local lemma.

## 4. Workstream B — complete the `wp` calculus (the structural fix)

Goal: every manual proof can be conducted entirely at the `wp` level, so
the ok/aborts split (P2) happens exactly once, inside library lemmas.

New lemmas in `Move/Verify/` (all `@[simp]`-able or in a dedicated simp
set `wp_norm`, see Workstream C):

1. `wp_mono` (postcondition/abort weakening) — the missing piece that
   forces proofs back into raw style whenever a callee's contract does not
   syntactically match the local goal:

   ```lean
   theorem wp_mono (h : wp action ensures aborts initial)
       (himp : ∀ result final, ensures result final → ensures' result final)
       (habort : ∀ code, aborts code → aborts' code) :
       wp action ensures' aborts' initial
   ```

2. `wp_ite` for pure conditions:
   `wp (if c then a else b) ens ab s ↔ if c then wp a … else wp b …`
   (plus the `decide`-condition variant produced by the spec translator).
3. **Conditional wp rules for every primitive**, stating both proof
   obligations at once so P3's `by_cases safe`/`eq_pure` dance disappears:

   ```lean
   theorem wp_addSpec :
       wp (Checked.addSpec a b) ensures aborts initial ↔
         (a.toNat + b.toNat < U64.size →
            ensures (U64.ofNat (a.toNat + b.toNat)) initial) ∧
         (¬a.toNat + b.toNat < U64.size → aborts Checked.arithmeticAbortCode)
   ```

   and likewise `wp_subSpec`, `wp_mulSpec`, `wp_divSpec`, `wp_modSpec`,
   `wp_borrowElemSpec`, `wp_setSpec`, `wp_insertSpec`, `wp_removeSpec`,
   `wp_borrowSpec` (resource), `wp_containsSpec`, `wp_moveFromSpec`,
   `wp_moveToSpec`.
4. **`wp` rule for prophecy scopes** (P9):
   `wp_withMutation : wp (withMutation value body) ens ab s ↔
    ∀ reference …` reduced to a `wp` of the body against the reconciled
   output — one lemma replacing the `⟨future, reference, …⟩` destructuring;
   similarly `wp_withBorrowElemMutSpec` and
   `wp_withBorrowMutFocusSpec`. (`withReborrow_eq_scopedMutation` already
   bridges reborrows; keep using it as a rewrite before these rules.)
5. Trivial glue: `wp_fix_const` (corollary of `Spec.fix_const`),
   `wp_bind` is already present.

With B in place, the canonical proof skeleton for a function with calls,
arithmetic, and branches is: open the contract (P1 tactic below), `rw
[wp_bind]`/simp with `wp_norm`, discharge branch/side conditions with
`omega`/`grind`, and step over callee calls with `wp_of_satisfies` +
`wp_mono`. No `.ok`/`.aborts` destructuring anywhere in user proofs.

Validation: rewrite `OrderedMap.borrow` (currently ~110 lines, fully
duplicated halves) as the pilot; target ≤ 40 lines with no duplicated
branch. Then `Calls.countdown` (P3 pilot; target ≤ 15 lines).

## 5. Workstream C — named simp sets shared with the automatic verifier

1. Register three simp attributes (new `Move/Verify/SimpSets.lean`, or in
   `Move/Attributes.lean`):
   - `move_norm` — value-level normalization: `U64` numerals/`toNat`
     lemmas, `logicalLT_uint`/`logicalLE_uint`, vector-literal `toList`
     lemmas, `Spec.pure_bind`, `Spec.fix_const`, the `*Spec_eq_pure`
     conditional equations.
   - `wp_norm` — the Workstream B rules plus `wp_pure`, `wp_abort`,
     `wp_bind`, `wp_ite`.
   - `move_spec` — the raw-semantics unfolding list currently hardcoded in
     `elabAutomaticSourceVerify` (Move/Verify/Syntax.lean:2717-2750).
2. Change `elabAutomaticSourceVerify` to emit
   `simp_all [move_spec, <function-specific unfolds>] <;> grind`, so the
   command and users share one lemma inventory, and users extend it with
   attributes (as they already do with `@[grind .]` in tests) instead of
   forking the list.
3. Conditional rewrites (`addSpec_eq_pure` etc.) enter `move_norm` with
   side conditions discharged by the tactic wrapper (below) via
   `simp (disch := …)`.

## 6. Workstream D — tactics (`Move/Verify/Tactics.lean`)

All are thin macros/elabs over Workstreams A–C:

1. **`contract_intro`** (P1): opens the current `f.contract` goal —
   unfolds `f.contract`, `f.sourceSpec`, `f.bodySpec` when present, and
   `Move.Verify.Satisfies`; intros binders with the standard names; for
   recursive contracts applies `satisfies_fix_of_wp` and intros
   `recursive recursiveVerified`; otherwise applies `satisfies_of_wp`.
   Implemented as a command-aware macro: `verify f by contract_intro; …`.
   (Whether `verify … by` should do this implicitly is an open question,
   §8.)
2. **`spec_norm` / `spec_norm at h`** (P3, P5, P6): wrapper around
   `simp only [move_norm] (disch := first | omega | decide | assumption)`.
   Steps the source spec into `pure`/`abort` normal form wherever side
   conditions are provable, without the `have step … rw [step]` ritual.
3. **`move_cases h : cond`** (P4): `by_cases` on the condition, rewrites
   the scrutinized `if` in the goal, and normalizes `h` to its `Nat` form
   (`logicalLT_uint`, `toNat` lemmas) in both branches. Also handles the
   loop-exit corollary: when the false branch yields `¬0 < n.toNat` it
   offers `subst`-ready `n = 0` via `U64.eq_zero_of_not_pos`.
4. **`u64_omega`** (P5): translates `U64` hypotheses and goal to `Nat`
   (`U64.ext`-iff, `toNat_ofNat*`, `size_eq`) then calls `omega`. Replaces
   `apply U64.ext; change …; omega` and `simp […]; omega` chains.
5. **`wp_call callee.verified [pre]`** (P7): `rw [wp_bind]`, applies
   `wp_of_satisfies callee.verified` with the given precondition proof
   (default `trivial`/`assumption`), then `wp_mono`, leaving (a) the
   weakened-postcondition goal and (b) the abort-forwarding goal, the
   latter closed automatically when the callee's aborts clause is `False`.
6. **`move_vcgen`** (aspirational driver): `contract_intro` + repeat
   (`spec_norm`, `simp only [wp_norm]`, `move_cases` on frontier
   conditions) producing leaf verification conditions; the automatic
   `verify f` can then try `move_vcgen <;> grind` before falling back to
   the current monolithic `simp_all`, making automatic and manual
   verification two points on one spectrum rather than two code paths.

## 7. Workstream E — migrate the test proofs, measure

File-by-file rewrite once A–D land, in this order (rising difficulty):

| File | Today | Expected effect |
|---|---|---|
| `Vectors`, `EnumPayloads`, `Generics` | small manual proofs + per-test numeral lemmas | mostly `verify f` automatic after A + C |
| `Read.lean` | 2 proofs, 4 duplicated branches | wp style, ~half the lines |
| `Loops.lean` | ~500 proof lines | `countDown`/`countDownLoop`/`labeledExit`/`labeledContinue` ≈ 5 lines each; `skipEvens`, `drain` shrink via `wp_withMutation` + `spec_norm` |
| `Calls.lean` | ~550 proof lines | `countdown`/`effectCountdown`/`sumDown`/`alternate` via `wp_addSpec` rules; mutual-recursion pairs via `satisfies_fixFamily` |
| `VectorOperations.lean` | ~300 proof lines + private lemmas | private lemmas deleted (A), `insertEdges`/`removeMiddle` via `wp` rules |
| `OrderedMap.lean` | ~700 verification lines | `contains`/`borrow`/`add`/`remove` in wp style; abort branches no longer duplicated |
| `Quicksort.lean` | ~620 verification lines | `partitionLoop` keeps its mathematical content; the ~150 lines of stepping boilerplate replaced by `spec_norm`/`move_cases`/`wp_call` |

Measurements to report per file: proof line count before/after and
compile time (`lake build Tests` wall clock), to confirm the tactics do
not regress elaboration time (the `simp (disch := …)` sets need care
here; keep them `simp only`-scoped).

The mathematical model sections (`Model.*` in OrderedMap/Quicksort — list
lemmas, `Partition`, `Segment`, …) are *not* in scope: they are genuine
content, not boilerplate. Only the symbolic-execution and plumbing layers
are targeted.

## 8. Sequencing and open questions

Suggested order: **A → C (simp sets) → B → D → E**, with the two pilots
(`OrderedMap.borrow`, `Calls.countdown`) rewritten immediately after B to
validate the rule set before investing in tactics.

Open questions to settle before/while implementing:

1. Should `verify f by` pre-open the goal (implicit `contract_intro`,
   possibly straight to `wp` form)? It shortens every proof but changes
   the meaning of existing proof scripts; an explicit opt-in spelling
   (e.g. `verify f wp by …`) would preserve compatibility during
   migration and could become the default once migration completes.
2. Exact discharge strategy for conditional rewrites in `spec_norm`:
   a fixed `first | omega | decide | assumption` is predictable; making it
   user-extensible risks unpredictable simp behavior.
3. Whether `wp_withMutation` can be stated without exposing `Mutation`
   internals to user proofs at all (preferred), or whether reborrow-heavy
   bodies (`siblingFieldWrites`) still need the `withReborrow_*` bridge
   rewrites first.
4. Naming: `spec_norm` / `move_cases` / `u64_omega` / `wp_call` are
   placeholders; align with whatever convention the `Move` namespace
   adopts for tactics (scoped in `Move.Spec` like the existing syntax?).
