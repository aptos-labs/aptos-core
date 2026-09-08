-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.OperationWP
import LeanerIR.Proofs.Composition

namespace LeanerIR.Tests.OperationWP

open LeanerIR.Proofs LeanerIR.Proofs.Denotation
open LeanerIR.Proofs.Denotation.RowSpec

/-- Scalar result decoding remains available after the generated result
codec is exposed at the end of body normalization. -/
example (value : Bool) : Codec.bool.decode? (.bool value) = some value := by
  simp only [lir_data_norm]

/-- Literal native encoding remains reducible when an operation has already
normalized the list reverse. This also covers symbolic-index vector writes. -/
example (a b c : SpecInt (.bits 64) false) :
    (Codec.vector (Codec.specInt (.bits 64) false)).encode #[a, b, c] =
      .vector #[.integer a.val, .integer b.val, .integer c.val] := by
  simp only [lir_data_norm]

/-- Only the focused WP rule may execute this borrow; no evaluator fallback
is in the simp set. The payload is arbitrary, including other loans. -/
example (state : RuntimeState) (outer : Nat) (current : RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate (DerefLocalBorrowOperation.evaluate?
      { location := ⟨⟨0⟩⟩, fields := [],
        referenceType := ⟨.move, .mutable, ⟨0⟩, ⟨0⟩⟩,
        kind := .mutable, lexicalLoan := 1 }) #[])
      post aborts ⟨#[some (.borrow outer current)], ⟨#[], #[], #[]⟩, state⟩ =
    post (.value (.borrow state.nextLoan current))
      ⟨#[some (.borrow outer (.loanHole state.nextLoan))],
        ⟨#[(⟨1⟩, state.nextLoan)],
          #[(state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)], #[]⟩,
        { state with nextLoan := state.nextLoan + 1 }⟩ := by
  simp only [wpBorrowFocus]
  rfl

/-- A field focus preserves the variant tag, both siblings, and every
unrelated local. This covers structs and enum variants without evaluator
fallback; sibling size and contents do not enter the operation proof. -/
example (state : RuntimeState) (outer : Nat) (source : StructHandle)
    (variant : Option String)
    (left current right unrelated : RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate (DerefLocalBorrowOperation.evaluate?
      { location := ⟨⟨1⟩⟩, fields := [⟨source, variant, 1⟩],
        referenceType := ⟨.move, .mutable, ⟨0⟩, ⟨0⟩⟩,
        kind := .mutable, lexicalLoan := 3 }) #[])
      post aborts
      ⟨#[some .unit, some (.borrow outer (.nominal source variant #[left, current, right])),
          some unrelated], ⟨#[], #[], #[]⟩, state⟩ =
    post (.value (.borrow state.nextLoan current))
      ⟨#[some .unit, some (.borrow outer
          (.nominal source variant #[left, .loanHole state.nextLoan, right])), some unrelated],
        ⟨#[(⟨3⟩, state.nextLoan)],
          #[(state.nextLoan, ⟨.local ⟨1⟩, #[.deref, .field 1], true⟩)], #[]⟩,
        { state with nextLoan := state.nextLoan + 1 }⟩ := by
  simp only [wpBorrowFocus]
  rfl

/-- The focused rule must not turn a mismatched variant into a successful
borrow. The ordinary partial evaluator still rejects that projection. -/
example (state : RuntimeState) (outer : Nat) (current : RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate (DerefLocalBorrowOperation.evaluate?
      { location := ⟨⟨0⟩⟩, fields := [⟨⟨⟨0⟩, 0⟩, some "Missing", 0⟩],
        referenceType := ⟨.move, .mutable, ⟨0⟩, ⟨0⟩⟩,
        kind := .mutable, lexicalLoan := 1 }) #[])
      post aborts
      ⟨#[some (.borrow outer (.nominal ⟨⟨0⟩, 0⟩ (some "Present") #[current]))],
        ⟨#[], #[], #[]⟩, state⟩ ↔ True := by
  fail_if_success (simp only [wpBorrowFocus]; done)
  rw [wp_evaluate]
  rfl

/-- A cached resting loan updates one slot; the operand's old current is
irrelevant. Again, this test cannot succeed through evaluator fallback. -/
example (state : RuntimeState) (loan : Nat)
    (current argument replacement unrelated : RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate ReferenceLocationOperation.mutate.evaluate?
      #[.borrow loan argument, replacement]) post aborts
      ⟨#[some unrelated, some (.borrow loan current)],
        ⟨#[], #[(loan, ⟨.local ⟨1⟩, #[], true⟩)], #[]⟩, state⟩ =
    post (.value .unit)
      ⟨#[some unrelated, some (.borrow loan replacement)],
        ⟨#[], #[(loan, ⟨.local ⟨1⟩, #[], true⟩)], #[]⟩, state⟩ := by
  simp [wpMutateFocus]

/-- A moved borrow can leave a stale cache entry. The focused rule must
certify the first matching resting slot rather than trusting that entry. -/
example (state : RuntimeState) (current replacement : RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate ReferenceLocationOperation.mutate.evaluate?
      #[.borrow 1 current, replacement]) post aborts
      ⟨#[some (.integer 0), some (.borrow 1 current)],
        ⟨#[], #[(1, ⟨.local ⟨0⟩, #[], true⟩)], #[]⟩, state⟩ =
    post (.value .unit)
      ⟨#[some (.integer 0), some (.borrow 1 replacement)],
        ⟨#[], #[(1, ⟨.local ⟨0⟩, #[], true⟩)], #[]⟩, state⟩ := by
  fail_if_success (simp; done)
  simp [wpMutateFocus]

/-- A reborrow's cache points to the lender's hole, not its resting holder.
The shared mutation rule preserves the entire lender and unrelated suffix,
including arbitrary enum tags, without general evaluator fallback. -/
example (state : RuntimeState) (source : StructHandle) (variant : Option String)
    (current replacement unrelated : RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate ReferenceLocationOperation.mutate.evaluate?
      #[.borrow (state.nextLoan + 1) current, replacement]) post aborts
      ⟨#[some (.borrow state.nextLoan
          (.nominal source variant #[.loanHole (state.nextLoan + 1)])),
        some (.borrow (state.nextLoan + 1) current), some unrelated],
        ⟨#[], #[(state.nextLoan + 1, ⟨.local ⟨0⟩, #[.deref, .field 0], true⟩)], #[]⟩, state⟩ =
    post (.value .unit)
      ⟨#[some (.borrow state.nextLoan
          (.nominal source variant #[.loanHole (state.nextLoan + 1)])),
        some (.borrow (state.nextLoan + 1) replacement), some unrelated],
        ⟨#[], #[(state.nextLoan + 1, ⟨.local ⟨0⟩, #[.deref, .field 0], true⟩)], #[]⟩, state⟩ := by
  fail_if_success (simp; done)
  simp [wpMutateFocus]

/-- A scalar reborrow uses the direct cached-hole miss certificate. The
hole need not belong to the loan being mutated: the resting search still
selects the first actual holder, and preserves the unrelated lender. -/
example (state : RuntimeState) (hole : Nat) (current replacement : RuntimeValue) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow 1 current, replacement]
      { locals := #[some (.borrow 0 (.loanHole hole)), some (.borrow 1 current)]
        loanLocations := #[(1, ⟨.local ⟨0⟩, #[.deref], true⟩)] } state =
      some (.value
        { locals := #[some (.borrow 0 (.loanHole hole)), some (.borrow 1 replacement)]
          loanLocations := #[(1, ⟨.local ⟨0⟩, #[.deref], true⟩)] } state .unit) := by
  fail_if_success (simp; done)
  simp [lir_eval, -evalReference, -evalMutate]

/-- The same certificate applies after operands have been evaluated, even
when no enclosing WP is available for the earlier rule to recognize. -/
example (state : RuntimeState) (current replacement unrelated : RuntimeValue) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow 1 current, replacement]
      { locals := #[some (.integer 0), some (.borrow 1 current), some unrelated]
        loanLocations := #[(1, ⟨.local ⟨0⟩, #[], true⟩)] } state =
      some (.value
        { locals := #[some (.integer 0), some (.borrow 1 replacement), some unrelated]
          loanLocations := #[(1, ⟨.local ⟨0⟩, #[], true⟩)] } state .unit) := by
  fail_if_success (simp; done)
  -- Exercise automatic registration across the import boundary while
  -- disabling both generic mutation fallbacks for this test.
  simp [lir_eval, -evalReference, -evalMutate]

/-- No cache entry and a consumed cached local are both certified misses,
not reasons to unfold the complete read/rewrite/write implementation. -/
example (uncached : Bool) (state : RuntimeState) (current replacement : RuntimeValue) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow 1 current, replacement]
      { locals := #[none, some (.borrow 1 current)]
        loanLocations := if uncached then #[] else #[(1, ⟨.local ⟨0⟩, #[], true⟩)] } state =
      some (.value
        { locals := #[none, some (.borrow 1 replacement)]
          loanLocations := if uncached then #[] else #[(1, ⟨.local ⟨0⟩, #[], true⟩)] } state .unit) := by
  cases uncached <;>
    (fail_if_success (simp; done)
     simp [lir_eval, -evalReference, -evalMutate])

set_option linter.unusedSimpArgs false in
/-- Finding a direct holder is not enough: an earlier nested occurrence
wins the semantic search. The direct-slot rule must decline this row. -/
example (state : RuntimeState) (source : StructHandle) (variant : Option String)
    (earlier later replacement : RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate ReferenceLocationOperation.mutate.evaluate?
      #[.borrow 1 later, replacement]) post aborts
      ⟨#[some (.nominal source variant #[.borrow 1 earlier]), some (.borrow 1 later)],
        ⟨#[], #[], #[]⟩, state⟩ =
    post (.value .unit)
      ⟨#[some (.nominal source variant #[.borrow 1 replacement]), some (.borrow 1 later)],
        ⟨#[], #[], #[]⟩, state⟩ := by
  fail_if_success (simp [wpMutateFocus]; done)
  simp [lir_wp_norm, lir_eval]

set_option maxHeartbeats 5000 in
/-- A symbolic vector has no loan entries, regardless of its length.
Only the typed collector certificate is allowed to discharge this test. -/
example (values : Array (SpecInt (.bits 8) false)) :
    SemanticOperations.collectPruned SemanticOperations.borrowEntry?
      (.vector (values.map (fun value => .integer value.val))) = #[] := by
  simp only [collectPlainAggregate]

/-- The collector must not discard an actual borrow embedded in a tuple. -/
example (loan : Nat) (current : RuntimeValue) :
    SemanticOperations.collectPruned SemanticOperations.borrowEntry?
      (.tuple #[.borrow loan current]) = #[some (loan, current)] := by
  fail_if_success (simp only [collectPlainAggregate]; done)
  simp [SemanticOperations.collectPruned, SemanticOperations.collectPrunedList,
    SemanticOperations.borrowEntry?]

set_option maxHeartbeats 1000 in
/-- Runtime equality and inequality expose logical guards in constant
proof size, even when vector elements are arbitrary runtime values. -/
example (left right : Array RuntimeValue) :
    ((RuntimeValue.vector left == .vector right) = true ↔ left = right) ∧
    ((RuntimeValue.vector left != .vector right) = true ↔ left ≠ right) := by
  simp

set_option maxHeartbeats 1000 in
/-- The certified Move boundary has a constant-size roundtrip proof for
arbitrary vectors; its length bound is not recomputed element by element. -/
example (codec : Codec α RuntimeValue) (values : SpecVector α) :
    (Codec.boundedVector codec).decode? ((Codec.boundedVector codec).encode values) =
      some values := by simp

/-- The added length certificate does not bypass element decoding. -/
example : (Codec.boundedVector (Codec.specInt (.bits 8) false)).decode?
    (.vector #[.integer 256]) = none := by
  decide

/-- Runtime shape remains checked independently of the bound. -/
example (codec : Codec α RuntimeValue) :
    (Codec.boundedVector codec).decode? (.tuple #[]) = none := rfl

/- The variant rule does not inspect arbitrary payloads or use the general
evaluator simplifier. It also covers a missing (struct) variant tag. -/
set_option maxHeartbeats 1000 in
example (source : StructHandle) (variants : Array String) (variant : Option String)
    (fields : Array RuntimeValue) (state : RowState)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate (NominalVariantTest.evaluate? ⟨source, variants⟩)
      #[.nominal source variant fields]) post aborts state ↔
      post (.value (.bool (variant.any variants.contains))) state := by
  simp only [wp_evaluate_variant_test]

/- The source handle remains checked, independently of the tag. -/
set_option maxHeartbeats 1000 in
example (fields : Array RuntimeValue) (state : RowState)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate (NominalVariantTest.evaluate? ⟨⟨⟨0⟩, 0⟩, #["First"]⟩)
      #[.nominal ⟨⟨0⟩, 1⟩ (some "First") fields]) post aborts state ↔ True := by
  fail_if_success (simp only [wp_evaluate_variant_test]; done)
  rw [wp_evaluate]
  rfl

/- A missing payload is partial, not an invented field value. -/
set_option maxHeartbeats 1000 in
example (source : StructHandle) (state : RowState)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate (NominalVariantFieldLocation.evaluateSelect?
      ⟨source, #[("First", 0)]⟩) #[.nominal source (some "First") #[]])
      post aborts state ↔ True := by
  rw [wp_evaluate_variant_field]
  rfl

/- An unselected computation must remain folded, even while an earlier
local read and variant test determine which arm runs. Eager normalization
expands this large list before it learns that the arm is unreachable. -/
private def expensiveUnusedBranch : Tree :=
  .value (.integer (Int.ofNat (List.replicate 100000 ()).length))

set_option maxHeartbeats 1000 in
example (state : RowState) (post : Control → RowState → Prop)
    (aborts : Failure → Prop) :
    wp (Tree.branchSome (.value (.bool true)) (.value (.integer 7))
      expensiveUnusedBranch).compute post aborts state ↔
      post (.value (.integer 7)) state := by
  leaner_normalize_lazy [expensiveUnusedBranch]

set_option maxHeartbeats 1000 in
example (runtime : RuntimeState) (post : Control → RowState → Prop)
    (aborts : Failure → Prop) :
    let state : RowState :=
      ⟨#[some (.nominal ⟨⟨0⟩, 0⟩ (some "First") #[])], ⟨#[], #[], #[]⟩, runtime⟩
    wp (Tree.branchSome
      (.variantTest { source := ⟨⟨0⟩, 0⟩, variants := #["First"] }
        (.cons (.localVar ⟨0⟩) .nil))
      (.value (.integer 7)) expensiveUnusedBranch).compute post aborts state ↔
      post (.value (.integer 7)) state := by
  dsimp only
  leaner_normalize_lazy [expensiveUnusedBranch]

set_option maxHeartbeats 1000 in
example (runtime : RuntimeState) (post : Control → RowState → Prop)
    (aborts : Failure → Prop) :
    let state : RowState :=
      ⟨#[some (.nominal ⟨⟨0⟩, 0⟩ (some "First") #[])], ⟨#[], #[], #[]⟩, runtime⟩
    wp (Tree.branchSome
      (.variantTest { source := ⟨⟨0⟩, 0⟩, variants := #["Second"] }
        (.cons (.localVar ⟨0⟩) .nil))
      expensiveUnusedBranch (.value (.integer 7))).compute post aborts state ↔
      post (.value (.integer 7)) state := by
  dsimp only
  leaner_normalize_lazy [expensiveUnusedBranch]

/- Indexed evaluators retain the VM's partial behavior at invalid places.
The new dispatch must not invent an element or accept a wrong enum tag. -/
set_option maxHeartbeats 1000 in
example (runtime : RuntimeState) (post : Control → RowState → Prop)
    (aborts : Failure → Prop) :
    wp (evaluate (IndexedLocalBorrowOperation.evaluate?
      { location := ⟨⟨0⟩⟩, dereference := false, index := 1,
        referenceType := ⟨.move, .mutable, ⟨0⟩, ⟨0⟩⟩,
        kind := .mutable, lexicalLoan := 0 }) #[]) post aborts
      ⟨#[some (.vector #[.integer 7])], ⟨#[], #[], #[]⟩, runtime⟩ ↔ True := by
  leaner_normalize

set_option maxHeartbeats 1000 in
example (runtime : RuntimeState) (post : Control → RowState → Prop)
    (aborts : Failure → Prop) :
    wp (evaluate (IndexedLocalFieldBorrowOperation.evaluate?
      { location := ⟨⟨0⟩⟩, index := 0,
        field := ⟨⟨⟨0⟩, 0⟩, some "Missing", 0⟩,
        referenceType := ⟨.move, .mutable, ⟨0⟩, ⟨0⟩⟩,
        kind := .mutable, lexicalLoan := 0 }) #[]) post aborts
      ⟨#[some (.vector #[.nominal ⟨⟨0⟩, 0⟩ (some "Present") #[.integer 7]])],
        ⟨#[], #[], #[]⟩, runtime⟩ ↔ True := by
  leaner_normalize

/- A symbolic vector needs one update certificate, not a traversal of its
unknown prefix and suffix, to locate or refill the inserted hole. -/
set_option maxHeartbeats 1000 in
example (values : Array Int) (index loan : Nat) (bound : index < values.size) :
    SemanticOperations.findFirstList (SemanticOperations.holeMark? loan)
      ((values.toList.map RuntimeValue.integer).set index (.loanHole loan)) = some () := by
  rw [findFirstList_set_of_absent]
  · simp [SemanticOperations.findFirst_loanHole]
  · apply findFirstList_map_none
    intro value
    simp [SemanticOperations.findFirst_integer]
  · simpa using bound

set_option maxHeartbeats 1000 in
example (values : Array Int) (index loan : Nat) (replacement : RuntimeValue) :
    SemanticOperations.rewriteFirstList (SemanticOperations.borrowRewrite? loan replacement)
      ((values.toList.map RuntimeValue.integer).set index (.loanHole loan)) = none := by
  apply rewriteFirstList_set_none
  · apply rewriteFirstList_map_none
    intro value
    simp [SemanticOperations.rewriteFirst_integer]
  · simp [SemanticOperations.rewriteFirst_loanHole]

set_option maxHeartbeats 1000 in
example (values : Array Int) (index loan : Nat) (replacement : RuntimeValue)
    (bound : index < values.size) :
    SemanticOperations.rewriteFirstList (SemanticOperations.holeFill? loan replacement)
      ((values.toList.map RuntimeValue.integer).set index (.loanHole loan)) =
      some ((values.toList.map RuntimeValue.integer).set index replacement) := by
  rw [rewriteFirstList_set_of_absent]
  · simp [SemanticOperations.rewriteFirst_loanHole, SemanticOperations.holeFill?]
  · apply rewriteFirstList_map_none
    intro value
    simp [SemanticOperations.rewriteFirst_integer, SemanticOperations.holeFill?]
  · simpa using bound

set_option maxHeartbeats 1000 in
example (left inserted right : RuntimeValue) (state : RuntimeState)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate PrimitiveLocationOperation.insertVector.evaluate?
      #[.vector #[left, right], .integer 1, inserted]) post aborts
      ⟨#[], ⟨#[], #[], #[]⟩, state⟩ =
    post (.value (.vector #[left, inserted, right])) ⟨#[], ⟨#[], #[], #[]⟩, state⟩ := by
  leaner_normalize

set_option maxHeartbeats 1000 in
example (left removed right : RuntimeValue) (state : RuntimeState)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate PrimitiveLocationOperation.removeVector.evaluate?
      #[.vector #[left, removed, right], .integer 1]) post aborts
      ⟨#[], ⟨#[], #[], #[]⟩, state⟩ =
    post (.value (.tuple #[removed, .vector #[left, right]]))
      ⟨#[], ⟨#[], #[], #[]⟩, state⟩ := by
  leaner_normalize

set_option maxHeartbeats 1000 in
example (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.removeVector.evaluate? #[.vector #[], .integer 0] frame state =
      some (.throw_ frame state .abort #[.integer 0]) := by
  rfl

set_option maxHeartbeats 1000 in
example (state : RuntimeState) (value : RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (Tree.compute (.assignLocal ⟨0⟩ (.value value))) post aborts
      ⟨#[none], ⟨#[], #[], #[]⟩, state⟩ =
    post (.value .unit) ⟨#[some value], ⟨#[], #[], #[]⟩, state⟩ := by
  leaner_normalize

set_option maxHeartbeats 1000 in
example (state : RowState) :
    wp (loop ⟨0⟩ (break_ 0))
      (fun control final => control = .value .unit ∧ final = state)
      (fun _ => False) state := by
  apply wp_loop_of_invariant (invariant := fun current => current = state)
  · rfl
  · intro current equal
    subst current
    leaner_normalize

set_option maxHeartbeats 1000 in
example (state : RowState) :
    wp (loop ⟨0⟩ (continue_ 1))
      (fun control final => control = .continue_ 0 ∧ final = state)
      (fun _ => False) state := by
  apply wp_loop_of_invariant (invariant := fun current => current = state)
  · rfl
  · intro current equal
    subst current
    leaner_normalize

set_option maxHeartbeats 1000 in
example (state : RowState) :
    wp (loop ⟨0⟩ (continue_ 0)) (fun _ _ => False) (fun _ => False) state := by
  -- Partial correctness: an unconditional continue has no finite exit.
  apply wp_loop_of_invariant (invariant := fun _ => True)
  · trivial
  · intro current _
    leaner_normalize

set_option maxHeartbeats 1000 in
example (state : RowState) :
    wp (loop ⟨0⟩ (break_ 1))
      (fun control final => control = .break_ 0 none ∧ final = state)
      (fun _ => False) state := by
  apply wp_loop_of_invariant (invariant := fun current => current = state)
  · rfl
  · intro current equal
    subst current
    leaner_normalize

set_option maxHeartbeats 1000 in
example (state : RowState) (values : List RuntimeValue) :
    wp (loop ⟨0⟩ (return_ (pure (.ok values))))
      (fun control final => control = .return_ values.toArray ∧ final = state)
      (fun _ => False) state := by
  apply wp_loop_of_invariant (invariant := fun current => current = state)
  · rfl
  · intro current equal
    subst current
    leaner_normalize

set_option maxHeartbeats 1000 in
example (state : RowState) :
    wp (loop ⟨0⟩ (throw_ .abort (pure (.ok [.integer 7]))))
      (fun control final => control = .throw_ .abort #[.integer 7] ∧ final = state)
      (fun _ => False) state := by
  apply wp_loop_of_invariant (invariant := fun current => current = state)
  · rfl
  · intro current equal
    subst current
    leaner_normalize

/- Inactive loop locals are optional but loan-free. Observing them must not
enumerate their combinations or inspect their runtime payloads. -/
set_option maxHeartbeats 1000 in
example (loan : Nat) (value : RuntimeValue) (a b c d e f g h : Option RuntimeValue)
    (_ha : SemanticOperations.Plain (a.getD .unit))
    (_hb : SemanticOperations.Plain (b.getD .unit))
    (_hc : SemanticOperations.Plain (c.getD .unit))
    (_hd : SemanticOperations.Plain (d.getD .unit))
    (_he : SemanticOperations.Plain (e.getD .unit))
    (_hf : SemanticOperations.Plain (f.getD .unit))
    (_hg : SemanticOperations.Plain (g.getD .unit))
    (_hh : SemanticOperations.Plain (h.getD .unit)) :
    SemanticOperations.frameBorrows
      (rowFrame #[some (.borrow loan value), a, b, c, d, e, f, g, h] ⟨#[], #[], #[]⟩) =
      #[(loan, value)] := by
  leaner_normalize

set_option maxHeartbeats 1000 in
example (loan : Nat) (a b : Option RuntimeValue)
    (_ha : SemanticOperations.Plain (a.getD .unit))
    (_hb : SemanticOperations.Plain (b.getD .unit)) :
    SemanticOperations.holeInFrame
      (rowFrame #[a, b] ⟨#[], #[], #[]⟩) loan = false := by
  simp only [rowFrame]
  leaner_normalize

/- The loop guard uses a u64 length; the resolver uses a natural index into
the current typed vector. Consume their invariant equation before opening
the borrower, without a case split on the index or vector contents. -/
set_option maxHeartbeats 1000 in
example (index : Int) (initial current : SpecVector Bool) (nonnegative : 0 ≤ index)
    (lengths : (current.values.map Codec.bool.encode).size =
      initial.values.size % Int.natAbs (18446744073709551616 : Int)) :
    index < (initial.values.size : Int) % 18446744073709551616 →
      (if index < 0 then none else
        if index.toNat < current.values.size then some index else none) = some index := by
  intro guard
  leaner_normalize

set_option maxHeartbeats 1000 in
example (index : Int) (values : SpecVector Bool) (state : RowState)
    (nonnegative : 0 ≤ index) :
    wp (if index < values.values.size then pure (Control.value (.integer index))
        else pure (Control.value .unit))
      (fun control _ => match control with
        | .value (.integer _) => index.toNat < values.values.size
        | .value .unit => True
        | _ => False)
      (fun _ : Failure => False) state := by
  leaner_compose [] []

/- A lowered source branch first selects a control constructor. Preserve
the branch boundary when that match reduces to an if, before touching the
continuation that needs its guard. -/
set_option maxHeartbeats 1000 in
example (index : Int) (values : SpecVector Bool) (state : RowState)
    (nonnegative : 0 ≤ index) :
    wp (branch (pure (.value (.bool (decide (index < values.values.size)))))
        (pure (.value (.integer index))) none)
      (fun control _ => match control with
        | .value (.integer _) => index.toNat < values.values.size
        | .value .unit => True
        | _ => False)
      (fun _ : Failure => False) state := by
  set_option leaner.branchBoundaries true in
    leaner_normalize
  leaner_compose [] []

/- The dynamic indexed borrow is independent of both vector length and
unrelated locals; the selected slot need not be the first local. -/
set_option maxHeartbeats 1000 in
example (values : SpecVector Bool) (index : Int) (outer : Nat)
    (unrelated : RuntimeValue) (inactive : Option RuntimeValue) (state : RuntimeState)
    (nonnegative : 0 ≤ index) (bound : index.toNat < values.values.size)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate (IndexedLocalBorrowOperation.evaluate?
      { location := ⟨⟨1⟩⟩, dereference := true, index := 0,
        referenceType := ⟨.move, .mutable, ⟨0⟩, ⟨0⟩⟩,
        kind := .mutable, lexicalLoan := 7, indexLocal := some ⟨2⟩ }) #[])
      post aborts
      ⟨#[some unrelated, some (.borrow outer (.vector (values.values.map .bool))),
        some (.integer index), inactive], ⟨#[], #[], #[]⟩, state⟩ =
    post (.value (.borrow state.nextLoan (.bool values.values[index.toNat])))
      ⟨#[some unrelated,
          some (.borrow outer (.vector ((values.values.map .bool).set! index.toNat
            (.loanHole state.nextLoan)))), some (.integer index), inactive],
        ⟨#[(⟨7⟩, state.nextLoan)],
          #[(state.nextLoan, ⟨.local ⟨1⟩, #[.deref, .index index.toNat], true⟩)], #[]⟩,
        { state with nextLoan := state.nextLoan + 1 }⟩ := by
  leaner_normalize

set_option maxHeartbeats 1000 in
example (payload unrelated : RuntimeValue) (outer : Nat) (state : RuntimeState)
    (registries : Registries) (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    let row := #[some unrelated,
      some (.borrow outer (.nominal ⟨⟨0⟩, 0⟩ none #[payload]))]
    wp (evaluate (DerefLocalBorrowOperation.evaluate?
      { location := ⟨⟨1⟩⟩, fields := [⟨⟨⟨0⟩, 0⟩, none, 0⟩],
        referenceType := ⟨.move, .shared, ⟨0⟩, ⟨0⟩⟩,
        kind := .immutable, lexicalLoan := 7 }) #[])
      post aborts ⟨row, registries, state⟩ =
      post (.value payload) ⟨row, registries, state⟩ := by
  dsimp only
  leaner_normalize

-- A concrete optional slot must still expose its native aggregate before
-- deciding loan-freedom; `getD` itself does not make a value opaque.
set_option maxHeartbeats 1000 in
example (flags : SpecVector Bool) :
    SemanticOperations.collectPruned SemanticOperations.borrowEntry?
      ((some (RuntimeValue.vector (flags.values.map .bool))).getD .unit) = #[] := by
  leaner_normalize

end LeanerIR.Tests.OperationWP
