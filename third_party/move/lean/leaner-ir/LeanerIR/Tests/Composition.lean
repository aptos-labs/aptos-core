-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Composition

namespace LeanerIR.Tests.Composition

open LeanerIR.SemanticOperations LeanerIR.Proofs.Denotation

-- Sequential allocations retain their nested addition spelling. Registry
-- preservation must decide that bound before expanding global write-back.
set_option maxHeartbeats 1000 in
example (initial final : RuntimeState) (key : GlobalKey)
    (fresh : FreshGlobalLoanIds initial)
    (discipline : LoanDiscipline
      { initial with
        globalLoans := (initial.nextLoan, key) :: initial.globalLoans
        nextLoan := initial.nextLoan + 1 + 1 } final) :
    globalLoanKeyIn? final.globalLoans initial.nextLoan = some key := by
  leaner_call_normalize []

-- Removing a caller-minted global loan after a call preserves the older
-- registry, even though the callee's registry itself remains opaque.
set_option maxHeartbeats 1000 in
example (initial final : RuntimeState) (key : GlobalKey) (loan : Nat)
    (fresh : FreshGlobalLoanIds initial) (older : loan < initial.nextLoan)
    (discipline : LoanDiscipline
      { initial with
        globalLoans := (initial.nextLoan, key) :: initial.globalLoans
        nextLoan := initial.nextLoan + 1 + 1 } final) :
    globalLoanKeyIn? (removeGlobalLoan final.globalLoans initial.nextLoan) loan =
      globalLoanKeyIn? initial.globalLoans loan := by
  leaner_call_normalize []

-- Advancing across multiple new global registrations never inspects the
-- symbolic registry tail or any resource payload.
set_option maxHeartbeats 1000 in
example (initial final : RuntimeState) (firstKey secondKey : GlobalKey)
    (callee : LoanDiscipline
      { initial with
        globalLoans := (initial.nextLoan + 1, secondKey) ::
          (initial.nextLoan, firstKey) :: initial.globalLoans
        nextLoan := initial.nextLoan + 2 } final) :
    LoanDiscipline initial final := by
  apply RowSpec.discipline_prefix_advance initial
    [(initial.nextLoan + 1, secondKey), (initial.nextLoan, firstKey)] callee rfl
  · simp only [List.forall_mem_cons, List.mem_nil_iff, false_implies, implies_true, and_true]
    omega
  · dsimp only
    omega

-- Nested branch normalization can leave two guards with the same displayed
-- name. The arithmetic inventory must retain the earlier proof's identity.
set_option maxHeartbeats 1000 in
example (initial : Nat) (n : Int) (branchGuard : n ≠ 0) :
    initial = initial ∧ n ≠ 0 := by
  have branchGuard : True := trivial
  leaner_normalize

set_option maxHeartbeats 1000 in
example (returned flag : Bool) (summary : returned = true ↔ flag = true) :
    (if returned then (1 : Int) else 0) = (if flag then 1 else 0) := by
  leaner_call_normalize []

-- Literal preconditions decide computed indices before symbolic evaluation.
-- Both orientations and negative literals use checked substitution only.
set_option maxHeartbeats 1000 in
example (index : Int) (fixed : index = 1) : index + 1 = 2 := by
  leaner_subst_decided
  rfl

set_option maxHeartbeats 1000 in
example (index : Int) (fixed : -1 = index) : index + 1 = 0 := by
  leaner_subst_decided
  rfl

/-- A failed loan comparison can become decidable after a call adds its
monotonicity fact. Negative evaluator caches must not hide the new fact. -/
example (a b : Nat) (bound : Unit → a < b) :
    (if a = b then 1 else 2) = 2 := by
  simp (config := { failIfUnchanged := false }) only [lir_eval]
  have monotone := bound ()
  simp only [lir_eval, ite_false]

/-- Retire the local parent after two reborrows, beside an untouched native
vector parameter. Check the entire resulting frame, not just that evaluation
produces some result. Payload size does not enter the loan search. -/
example (state : RuntimeState) (source : StructHandle)
    (selfLoan parent : Nat) (distinct : selfLoan < parent)
    (values : Array (SpecInt (.bits 8) false)) (a b : Int) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[]
      { locals := #[some (.borrow selfLoan (.nominal source none
            #[.vector (values.map fun v => .integer v.val)])),
          some (.integer a), some (.integer b), some (.loanHole parent),
          some (.borrow parent (.vector #[.integer a, .integer b])),
          some (.vector #[.integer a, .integer b])]
        activeLoans := #[(⟨0⟩, parent)]
        loanLocations := #[(selfLoan, { root := .local ⟨0⟩ }),
          (parent, { root := .local ⟨3⟩ })] }
      state = some (.value
        { locals := #[some (.borrow selfLoan (.nominal source none
              #[.vector (values.map fun v => .integer v.val)])),
            some (.integer a), some (.integer b), some (.vector #[.integer a, .integer b]),
            some .unit, some (.vector #[.integer a, .integer b])]
          loanLocations := #[(selfLoan, { root := .local ⟨0⟩ }),
            (parent, { root := .local ⟨3⟩ })] }
        state .unit) := by
  simp [lir_eval]

/-- Dynamic call facts must reach the normalizer even when the caller
supplies no extra rules. Two independent syntax splices used to lose them. -/
example (initial final : RuntimeState) (fresh : FreshGlobalLoanIds initial)
    (discipline : LoanDiscipline { initial with nextLoan := initial.nextLoan + 1 + 1 } final) :
    LoanDiscipline initial final := by
  leaner_call_normalize []

set_option maxHeartbeats 1000 in
example (initial final : RuntimeState) (globals : GlobalMap)
    (pending : Array (Nat × RuntimeValue))
    (discipline : LoanDiscipline initial final) :
    LoanDiscipline initial
      { globals, globalLoans := final.globalLoans, nextLoan := final.nextLoan + 1, pending } := by
  leaner_certified_close!

/-- A throwing call with no exported writes preserves every caller slot and
registry. This is quantified over the whole row, not a particular layout. -/
example (caller : RowState) (final : RuntimeState) (kind : ThrowKind)
    (payload : Array RuntimeValue) (lexical : Option Nat)
    (noWrites : final.pending = caller.state.pending) :
    RowSpec.resumeCall lexical caller final (.threw kind payload) =
      RowState.ofFrame caller.frame { final with pending := caller.state.pending } := by
  simp only [RowSpec.resumeCall, applyPendingFrom_none noWrites, callFrame_threw]

/-- A call reconciles its exported write even on a throwing outcome. The
inherited prefix is not replayed, and reconciliation uses the caller frame. -/
example (caller : RowState) (final : RuntimeState) (kind : ThrowKind)
    (payload : Array RuntimeValue) (loan : Nat) (value : RuntimeValue)
    (write : final.pending = caller.state.pending.push (loan, value)) :
    RowSpec.resumeCall none caller final (.threw kind payload) =
      let resumed := applyPendingWriteBack caller.frame
        { final with pending := caller.state.pending } loan value
      RowState.ofFrame resumed.1 resumed.2 := by
  simp only [RowSpec.resumeCall, applyPendingFrom_single write, callFrame_threw]

/-- Returned-reference summaries can equate a value to a resolution that
contains it. That expanding direction must not become a simp rule. -/
example (value : Int) (loan : Nat)
    (_resolution : RuntimeValue.integer value =
      resolveReturnedBorrows #[.borrow loan (.integer value)] (.loanHole loan)) :
    readLocal? (rowFrame #[some (.integer value)]
      { activeLoans := #[], loanLocations := #[] }) ⟨0⟩ =
      some (.integer value) := by
  leaner_call_normalize []

/-- Mutable decoding maps the decoded payload into a loan/value record.
Expose that map before attempting to normalize the caller continuation. -/
example (runtime : RuntimeValue) (loan : Nat)
    (decoded : Proofs.MutableArgument (SpecInt (.bits 64) false))
    (decodedEq : (decodeInt? (.bits 64) false runtime).map
      (fun value => (⟨loan, value⟩ : Proofs.MutableArgument (SpecInt (.bits 64) false))) =
        some decoded) :
    runtime = .integer decoded.value.val ∧ loan = decoded.loan := by
  leaner_call_results
  all_goals trivial

set_option maxHeartbeats 1000 in
example (runtime : RuntimeValue)
    (decoded : SpecVector (SpecInt (.bits 64) false))
    (decodedEq : (Proofs.Codec.boundedVector (Proofs.Codec.specInt (.bits 64) false)).decode?
      runtime = some decoded)
    (contents : decoded.values.map (fun value => RuntimeValue.integer value.val) =
      #[.integer 10, .integer 20, .integer 30]) :
    runtime = .vector #[.integer 10, .integer 20, .integer 30] := by
  leaner_call_results
  leaner_call_normalize []

/-- The callee constrains an untouched sibling through its resolved export.
That fact must become available before the caller's arithmetic closes. -/
example (source : StructHandle) (runtime : RuntimeValue) (loan : Nat)
    (decoded : Proofs.MutableArgument (SpecInt (.bits 64) false))
    (sibling expected : Int)
    (decodedEq : (decodeInt? (.bits 64) false runtime).map
      (fun value => (⟨loan, value⟩ : Proofs.MutableArgument (SpecInt (.bits 64) false))) =
        some decoded)
    (preserved : ((resolveReturnedBorrows
      #[.borrow decoded.loan (.integer decoded.value.val)]
      (focusValue [⟨source, #[], #[.integer sibling], none⟩]
        (.loanHole decoded.loan))).field 1).asInt = expected) :
    sibling = expected := by
  leaner_call_results
  assumption

/-- More than two exports use the same suffix law, even when two entries
name the same loan. Order matters; the inherited writes are not replayed. -/
example (frame : RuntimeFrame) (state : RuntimeState)
    (inherited : Array (Nat × RuntimeValue)) (firstLoan secondLoan : Nat)
    (firstValue secondValue thirdValue : RuntimeValue)
    (pending : state.pending = ((inherited.push (firstLoan, firstValue)).push
      (secondLoan, secondValue)).push (firstLoan, thirdValue)) :
    applyPendingFrom inherited frame state =
      let first := applyPendingWriteBack frame { state with pending := inherited }
        firstLoan firstValue
      let second := applyPendingWriteBack first.1 first.2 secondLoan secondValue
      applyPendingWriteBack second.1 second.2 firstLoan thirdValue := by
  simp only [lir_eval, List.foldl_cons, List.foldl_nil]

/-- Registration of a returned loan preserves opaque locals and records
only its dynamic identity, without searching the borrowed payload. -/
example (frame : RuntimeFrame) (lexical loan : Nat) (current : RuntimeValue) :
    registerReturnedLoan (some lexical) #[.borrow loan current] frame =
      { frame with activeLoans :=
        (frame.activeLoans.filter (·.1 != ⟨lexical⟩)).push (⟨lexical⟩, loan) } := by
  simp only [lir_eval]

/-- A global returned-reference summary supplies the registry equation as
well as the storage equation; retirement must see both. -/
example (initial final : RuntimeState) (loan : Nat) (key : GlobalKey)
    (registered : final.globalLoans = (loan, key) :: initial.globalLoans) :
    globalLoanKey? final loan = some key := by
  leaner_call_normalize []

/-- Stand-in for a generated resource decoder. Keeping it irreducible
ensures the boundary test needs its decoding certificate. -/
@[irreducible] def SummaryPair.decode? (runtime : RuntimeValue) : Option (Int × Int) :=
  match runtime with
  | .nominal _ _ fields =>
      match fields.toList with
      | [.integer left, .integer right] => some (left, right)
      | _ => none
  | _ => none

example (source : StructHandle) (runtime : RuntimeValue) (loan : Nat)
    (decoded : Proofs.MutableArgument (SpecInt (.bits 64) false))
    (sibling expected : Int)
    (decodedEq : (decodeInt? (.bits 64) false runtime).map
      (fun value => (⟨loan, value⟩ : Proofs.MutableArgument (SpecInt (.bits 64) false))) =
        some decoded)
    (preserved : ((SummaryPair.decode? (resolveReturnedBorrows
      #[.borrow decoded.loan (.integer decoded.value.val)]
      (focusValue [⟨source, #[], #[.integer sibling], none⟩]
        (.loanHole decoded.loan)))).getD (0, 0)).2 = expected) :
    sibling = expected := by
  leaner_call_results
  assumption

/-- Retire the inner loan before its parent and preserve the marker's
argument. Disabling the general reference evaluator pins the staged rule. -/
example (state : RuntimeState) (parent child : Nat) (ordered : parent < child)
    (value : Int) (flag : Bool) (locations : Array (Nat × RuntimePlace)) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩]).evaluate? #[.bool flag]
      { locals := #[some (.loanHole parent), some (.borrow parent (.loanHole child)),
          some (.borrow child (.integer value))]
        activeLoans := #[(⟨0⟩, parent), (⟨1⟩, child)]
        loanLocations := locations } state =
      some (.value
        { locals := #[some (.integer value), some .unit, some .unit]
          loanLocations := locations } state (.bool flag)) := by
  simp [lir_eval, lir_call_eval, -RowSpec.evalReference]

set_option maxHeartbeats 1000 in
/-- Dependent certificates can retain an injected equation. The result
exposer must not repeatedly inject that same equation. -/
example (left right : Int) (h : RuntimeValue.integer left = .integer right)
    (certificate : (RuntimeValue.integer left = .integer right) → Prop)
    (_dependent : certificate h) : left = right := by
  leaner_call_results
  all_goals rfl

set_option maxHeartbeats 1000 in
/-- Inspect runtime summaries only. Unrelated semantic equations must not
be weak-head normalized while searching for enum constructor facts. -/
example (_unrelated : (List.replicate 100000 ()).length = 100000)
    (left right : Int) (h : RuntimeValue.integer left = .integer right) : left = right := by
  leaner_call_results
  all_goals rfl

end LeanerIR.Tests.Composition
