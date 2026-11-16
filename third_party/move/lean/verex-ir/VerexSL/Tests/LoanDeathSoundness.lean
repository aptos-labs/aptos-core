-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

/-!
# A loan's death certifies before it equates: the soundness controls

The controls of the loan-death rule. Each negative was a program the verifier
**accepted** — most through an authored proof, two through the automatic
tactic — and is kept with the proof attempt that exploited the hole; each runs
that attempt, pins the rejection, and asserts that no proof artifact was
published. The matching controls that must verify are kept beside them, so
that an unsupported construct or a compile failure cannot pass for
enforcement. The other soundness reviews' counterexamples are in
`SoundnessCounterexamples.lean`.
-/

set_option Elab.async false

open Lean Meta Elab Tactic

/-! ## A loan's death certifies before it equates

The prophecy of a mutable borrow carries `NTy.Holds`, so the equation a
loan's death assumes (`current = prophecy`) is valid only when `current`'s
data invariants hold. The death of a reborrow through a reference
(`&mut b.f`, `&mut *b`, where `b` is a reference) must check `NTy.Holds` on
`current` before assuming `current = prophecy`: without that certifying step, a
run that left `current` invalid would be pruned at the equation, making
every later obligation — the owner's exit check among them — vacuous. The
automatic tactic rejected such a function by failing to find the proof, while
the authored script below would otherwise discharge it from the contradiction.
A holder joined from two reborrows must likewise check before equating.

Now every death certifies the side of each equation it assumes
(`Lender.resolve` and `holderResolves`).
Each negative runs the automatic tactic, which must name the cut where the
reborrow dies, and then the authored script, which must fail on arithmetic it
can no longer do. `repaired_through_owner` is rejected on purpose: the value
is invalid where the field loan ends, although the owner repairs it before its
own loan ends. The Move Prover, which checks a write-back into a root and a
`&mut` parameter's death only, accepts that program. -/

verex module 0x42::reborrow_death where
  pragma verify = false
  struct Percent where
    value : u64
  spec Percent where
    invariant value <= 100
  struct Pair where
    left : Percent
    right : Percent
  struct Box {T} where
    value : T
    other : T
  struct Ordered where
    low : Percent
    high : Percent
  spec Ordered where
    invariant low.value <= high.value
  struct Nested where
    inner : Ordered
    extra : Percent

  fun set_left(pair : &mut Pair, amount : u64) -> Unit := do
    let field := &mut pair.left
    field.value := amount
  spec set_left where
    requires amount <= 100
    ensures pair.left.value == amount
    ensures pair.right.value == old(pair).right.value
  fun set_left_bad(pair : &mut Pair) -> Unit := do
    let field := &mut pair.left
    field.value := 101

  fun set_box(box : &mut Box<Percent>, amount : u64) -> Unit := do
    let field := &mut box.value
    field.value := amount
  spec set_box where
    requires amount <= 100
    ensures box.value.value == amount
    ensures box.other.value == old(box).other.value
  fun set_box_bad(box : &mut Box<Percent>) -> Unit := do
    let field := &mut box.value
    field.value := 101
  -- Valid, and not verified automatically before or after the change: the exit
  -- obligation of a generic owner is left as `Box<T>.Holds`. Kept so that the
  -- generic borrow compiles beside the concrete one.
  fun set_value {T}(box : &mut Box<T>, value : T) -> Unit := do
    let field := &mut box.value
    *field := value

  fun set_low(ordered : &mut Ordered, amount : u64) -> Unit := do
    let field := &mut ordered.low
    field.value := amount
  spec set_low where
    requires amount <= ordered.high.value
    ensures ordered.low.value == amount
    ensures ordered.high.value == old(ordered).high.value
  fun set_low_bad(ordered : &mut Ordered) -> Unit := do
    let field := &mut ordered.low
    field.value := 101

  fun whole(percent : &mut Percent, amount : u64) -> Unit := do
    let all := &mut *percent
    all.value := amount
  spec whole where
    requires amount <= 100
    ensures percent.value == amount
  fun whole_bad(percent : &mut Percent) -> Unit := do
    let all := &mut *percent
    all.value := 101

  fun nested_low(nested : &mut Nested, amount : u64) -> Unit := do
    let field := &mut nested.inner.low
    field.value := amount
  spec nested_low where
    requires amount == 0
  fun nested_low_bad(nested : &mut Nested) -> Unit := do
    let field := &mut nested.inner.low
    field.value := 101

  fun ancestor_open_then_repaired(ordered : &mut Ordered) -> Unit := do
    let low := &mut ordered.low
    low.value := 100
    let high := &mut ordered.high
    high.value := 100
  fun ancestor_left_open_bad(ordered : &mut Ordered) -> Unit := do
    let low := &mut ordered.low
    low.value := 100

  fun repaired_before_death(pair : &mut Pair) -> Unit := do
    let field := &mut pair.left
    field.value := 101
    field.value := 100
  fun repaired_through_owner(pair : &mut Pair) -> Unit := do
    let field := &mut pair.left
    field.value := 101
    pair.left.value := 100

  fun scoped(pair : &mut Pair, amount : u64) -> u64 := do
    let field := &mut pair.left
    field.value := amount
    pair.right.value
  spec scoped where
    requires amount <= 100
    ensures result == old(pair).right.value
    ensures pair.left.value == amount
  fun scoped_bad(pair : &mut Pair) -> u64 := do
    let field := &mut pair.left
    field.value := 101
    pair.right.value

  fun observe(percent : &mut Percent) -> u64 := percent.value
  fun observe_pair(pair : &mut Pair) -> u64 := pair.left.value
  fun hand_over(pair : &mut Pair) -> u64 := do
    let field := &mut pair.left
    field.value := 100
    observe(field)
  fun hand_over_bad(pair : &mut Pair) -> u64 := do
    let field := &mut pair.left
    field.value := 101
    observe(field)
  fun escapes_to_callee(pair : &mut Pair) -> u64 := do
    let field := &mut pair.left
    field.value := 101
    let observed := observe_pair(pair)
    pair.left.value := 100
    observed

  fun left_of(pair : &mut Pair, amount : u64) -> &mut Percent := do
    let field := &mut pair.left
    field.value := amount
    field
  spec left_of where
    requires amount <= 100
  fun left_of_bad(pair : &mut Pair) -> &mut Percent := do
    let field := &mut pair.left
    field.value := 101
    field

  fun set_either(flag : Bool, pair : &mut Pair, amount : u64) -> Unit := do
    let field := if flag then &mut pair.left else &mut pair.right
    field.value := amount
  spec set_either where
    requires amount <= 100
  fun set_either_bad(flag : Bool, pair : &mut Pair) -> Unit := do
    let field := if flag then &mut pair.left else &mut pair.right
    field.value := 101
  fun set_either_swapped_bad(flag : Bool, pair : &mut Pair) -> Unit := do
    let field := if flag then &mut pair.right else &mut pair.left
    field.value := 101
  fun set_either_first_bad(flag : Bool, pair : &mut Pair) -> Unit := do
    let field := if flag then &mut pair.left else &mut pair.right
    field.value := 101
  spec set_either_first_bad where
    requires flag
  fun set_either_second_bad(flag : Bool, pair : &mut Pair) -> Unit := do
    let field := if flag then &mut pair.left else &mut pair.right
    field.value := 101
  spec set_either_second_bad where
    requires !flag
  fun set_two(flag : Bool, first : &mut Pair, second : &mut Pair, amount : u64) -> Unit := do
    let field := if flag then &mut first.left else &mut second.left
    field.value := amount
  spec set_two where
    requires amount <= 100
  fun set_two_bad(flag : Bool, first : &mut Pair, second : &mut Pair) -> Unit := do
    let field := if flag then &mut first.left else &mut second.left
    field.value := 101
  fun set_two_swapped_bad(flag : Bool, first : &mut Pair, second : &mut Pair) -> Unit := do
    let field := if flag then &mut second.left else &mut first.left
    field.value := 101

  fun roots(flag : Bool, amount : u64) -> u64 := do
    let mut first := new Percent { value := 0 }
    let mut second := new Percent { value := 1 }
    let chosen := if flag then &mut first else &mut second
    chosen.value := amount
    first.value + second.value
  spec roots where
    requires amount <= 100
  fun roots_bad(flag : Bool) -> u64 := do
    let mut first := new Percent { value := 0 }
    let mut second := new Percent { value := 1 }
    let chosen := if flag then &mut first else &mut second
    chosen.value := 101
    first.value + second.value

  fun root_or_field(flag : Bool, pair : &mut Pair, amount : u64) -> u64 := do
    let mut first := new Percent { value := 0 }
    let chosen := if flag then &mut first else &mut pair.left
    chosen.value := amount
    first.value
  spec root_or_field where
    requires amount <= 100
  fun root_or_field_bad(flag : Bool, pair : &mut Pair) -> u64 := do
    let mut first := new Percent { value := 0 }
    let chosen := if flag then &mut first else &mut pair.left
    chosen.value := 101
    first.value

  fun whole_either(flag : Bool, first : &mut Percent, second : &mut Percent, amount : u64) -> Unit := do
    let chosen := if flag then &mut *first else &mut *second
    chosen.value := amount
  spec whole_either where
    requires amount <= 100
  fun whole_either_bad(flag : Bool, first : &mut Percent, second : &mut Percent) -> Unit := do
    let chosen := if flag then &mut *first else &mut *second
    chosen.value := 101
  fun whole_either_swapped_bad(flag : Bool, first : &mut Percent, second : &mut Percent) -> Unit := do
    let chosen := if flag then &mut *second else &mut *first
    chosen.value := 101

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  let ordered := "invariant low.value <= high.value"
  expectCuts `«0x42».reborrow_death.set_left_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_left_bad))
  expectScriptFails `«0x42».reborrow_death.set_left_bad
    (← `(command| verify 0x42::reborrow_death::set_left_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.set_box_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_box_bad))
  expectScriptFails `«0x42».reborrow_death.set_box_bad
    (← `(command| verify 0x42::reborrow_death::set_box_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.set_low_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_low_bad))
  expectScriptFails `«0x42».reborrow_death.set_low_bad
    (← `(command| verify 0x42::reborrow_death::set_low_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.whole_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::whole_bad))
  expectScriptFails `«0x42».reborrow_death.whole_bad
    (← `(command| verify 0x42::reborrow_death::whole_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.nested_low_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::nested_low_bad))
  expectScriptFails `«0x42».reborrow_death.nested_low_bad
    (← `(command| verify 0x42::reborrow_death::nested_low_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.whole_either_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::whole_either_bad))
  expectScriptFails `«0x42».reborrow_death.whole_either_bad
    (← `(command| verify 0x42::reborrow_death::whole_either_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.whole_either_swapped_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::whole_either_swapped_bad))
  expectScriptFails `«0x42».reborrow_death.whole_either_swapped_bad
    (← `(command| verify 0x42::reborrow_death::whole_either_swapped_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.ancestor_left_open_bad [(ordered, exit)]
    (← `(command| verify 0x42::reborrow_death::ancestor_left_open_bad)) (some [])
  expectScriptFails `«0x42».reborrow_death.ancestor_left_open_bad
    (← `(command| verify 0x42::reborrow_death::ancestor_left_open_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.repaired_through_owner [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::repaired_through_owner))
  expectScriptFails `«0x42».reborrow_death.repaired_through_owner
    (← `(command| verify 0x42::reborrow_death::repaired_through_owner by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.scoped_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::scoped_bad))
  expectScriptFails `«0x42».reborrow_death.scoped_bad
    (← `(command| verify 0x42::reborrow_death::scoped_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.hand_over_bad [(percent, handOver)]
    (← `(command| verify 0x42::reborrow_death::hand_over_bad))
  expectScriptFails `«0x42».reborrow_death.hand_over_bad
    (← `(command| verify 0x42::reborrow_death::hand_over_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.escapes_to_callee [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::escapes_to_callee))
  expectScriptFails `«0x42».reborrow_death.escapes_to_callee
    (← `(command| verify 0x42::reborrow_death::escapes_to_callee by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.left_of_bad [(percent, exit)]
    (← `(command| verify 0x42::reborrow_death::left_of_bad)) (some [])
  expectScriptFails `«0x42».reborrow_death.left_of_bad
    (← `(command| verify 0x42::reborrow_death::left_of_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.set_either_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_either_bad))
  expectScriptFails `«0x42».reborrow_death.set_either_bad
    (← `(command| verify 0x42::reborrow_death::set_either_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.set_either_swapped_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_either_swapped_bad))
  expectScriptFails `«0x42».reborrow_death.set_either_swapped_bad
    (← `(command| verify 0x42::reborrow_death::set_either_swapped_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.set_either_first_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_either_first_bad))
  expectScriptFails `«0x42».reborrow_death.set_either_first_bad
    (← `(command| verify 0x42::reborrow_death::set_either_first_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.set_either_second_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_either_second_bad))
  expectScriptFails `«0x42».reborrow_death.set_either_second_bad
    (← `(command| verify 0x42::reborrow_death::set_either_second_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.set_two_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_two_bad))
  expectScriptFails `«0x42».reborrow_death.set_two_bad
    (← `(command| verify 0x42::reborrow_death::set_two_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.set_two_swapped_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::set_two_swapped_bad))
  expectScriptFails `«0x42».reborrow_death.set_two_swapped_bad
    (← `(command| verify 0x42::reborrow_death::set_two_swapped_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.roots_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::roots_bad))
  expectScriptFails `«0x42».reborrow_death.roots_bad
    (← `(command| verify 0x42::reborrow_death::roots_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `«0x42».reborrow_death.root_or_field_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::reborrow_death::root_or_field_bad))
  expectScriptFails `«0x42».reborrow_death.root_or_field_bad
    (← `(command| verify 0x42::reborrow_death::root_or_field_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  -- The script with evidence at the base: the one `verex_on` form that
  -- proved `set_left_bad` before the repair.
  expectScriptFails `«0x42».reborrow_death.set_left_bad
    (← `(command| verify 0x42::reborrow_death::set_left_bad by lean =>
      verex_obligations
      verex_on
      | data_invariant_1 => verex_prepare <;> (exfalso; omega)
      | _ => verex_decide))

-- The valid twins. A reborrow that leaves its component valid verifies, with
-- an enclosing clause left open meanwhile (`ancestor_open_then_repaired`) and
-- a value repaired before the same loan dies (`repaired_before_death`).
verify 0x42::reborrow_death::set_left
verify 0x42::reborrow_death::set_box
verify 0x42::reborrow_death::whole
verify 0x42::reborrow_death::ancestor_open_then_repaired
verify 0x42::reborrow_death::repaired_before_death
verify 0x42::reborrow_death::scoped
verify 0x42::reborrow_death::observe
verify 0x42::reborrow_death::observe_pair
verify 0x42::reborrow_death::left_of
verify 0x42::reborrow_death::set_either
verify 0x42::reborrow_death::set_two
verify 0x42::reborrow_death::roots
verify 0x42::reborrow_death::root_or_field
verify 0x42::reborrow_death::set_low
verify 0x42::reborrow_death::nested_low
verify 0x42::reborrow_death::hand_over
verify 0x42::reborrow_death::whole_either
-- Valid and generic, not verified automatically before the change or after
-- it: the exit obligation of a generic owner is left as `Box<T>.Holds`. Kept
-- so that the generic borrow compiles beside the concrete one.
verify 0x42::reborrow_death::set_value

run_cmd do
  let env ← getEnv
  for name in [`«0x42».reborrow_death.set_left.typedVerified,
      `«0x42».reborrow_death.set_box.typedVerified,
      `«0x42».reborrow_death.whole.typedVerified,
      `«0x42».reborrow_death.ancestor_open_then_repaired.typedVerified,
      `«0x42».reborrow_death.repaired_before_death.typedVerified,
      `«0x42».reborrow_death.scoped.typedVerified,
      `«0x42».reborrow_death.observe.typedVerified,
      `«0x42».reborrow_death.observe_pair.typedVerified,
      `«0x42».reborrow_death.left_of.typedVerified,
      `«0x42».reborrow_death.set_either.typedVerified,
      `«0x42».reborrow_death.set_two.typedVerified,
      `«0x42».reborrow_death.roots.typedVerified,
      `«0x42».reborrow_death.root_or_field.typedVerified,
      `«0x42».reborrow_death.set_low.typedVerified,
      `«0x42».reborrow_death.nested_low.typedVerified,
      `«0x42».reborrow_death.hand_over.typedVerified,
      `«0x42».reborrow_death.whole_either.typedVerified,
      `«0x42».reborrow_death.set_value.typedVerified] do
    unless env.contains name do throwError "the control {name} did not verify"

/-! ## A loan of a slice element

A slice reference keeps its current certified (`NTy.carrier`, `.sliceRef`), so
the death of a loan of one of its elements, or of a field of one, owes the
slice reference rebuilt with the final current before the write-back equates
it with the certified slice the owner holds. The path of such a loan counted
as a dereference, and its death owed nothing: `elem_bad` was provable by the
script below, and `elem_field_bad`, which breaks the element's own clause with
a valid field, by honest proofs of its two bounds goals alone. Each script now
leaves a certifying step of the death unsolved. -/

verex namespace slice_loans where
  pragma verify = false
  struct Percent where
    value : u32
  spec Percent where
    invariant value <= 100
  struct Pair where
    low : Percent
    high : Percent
  spec Pair where
    invariant low.value <= high.value
  struct Holder where
    arr : Vector<Percent, const 2>

  fun elem_bad(s : &mut Slice<Percent>) -> Unit := do
    let r := &mut s[0usize]
    (*r).value := 101u32
  spec elem_bad where
    requires s.length > 0

  fun elem_field_bad(s : &mut Slice<Pair>) -> Unit := do
    let r := &mut s[0usize].low
    (*r).value := 100u32
  spec elem_field_bad where
    requires s.length > 0


open LoanDeathControls in
run_cmd do
  -- The scripts that proved these two before the repair. Each must now be
  -- rejected with no artifact published; what the cut is called is pinned by
  -- `rejectedDiagnostics`' own requirement of a `verex verification failed`
  -- report, and the script's own failure is recorded beside it.
  for (base, command) in [
      (`slice_loans.elem_bad, ← `(command| verify slice_loans::elem_bad by lean =>
        verex_obligations
        verex_on
        | data_invariant_1 => verex_prepare <;> (exfalso; omega)
        | no_panic => verex_prepare <;> omega
        | leaf => verex_prepare <;> (
            simp (failIfUnchanged := false) only [
              VerexIR.SpecSliceRef.map_extent] at heq
            omega))),
      (`slice_loans.elem_field_bad, ← `(command| verify slice_loans::elem_field_bad by lean =>
        verex_obligations
        verex_on
        | no_panic => verex_prepare <;> omega
        | leaf => verex_prepare <;> (
            simp (failIfUnchanged := false) only [
              VerexIR.SpecSliceRef.map_extent] at heq
            omega)))] do
    let diagnostics ← rejectedDiagnostics base command
    if let some other := diagnostics.find? fun message =>
        !(message.startsWith "unsolved goals" ||
          message.startsWith "omega could not prove the goal" ||
          message.startsWith "verex verification failed: the proof of ") then
      throwError "{base}: the script failed elsewhere: {other}"

/-! ## A reference stored in an aggregate

A loan through a reference that a local aggregate holds (`&mut *slot.target`)
writes back into a root: its death owes the aggregate rebuilt and, the
component lying below a dereference that the aggregate's certificate does not
cover, the reference's own current. It owed neither, and the automatic tactic
verified both functions below with no authored proof. -/

verex namespace stored_reference where
  pragma verify = false
  struct Percent where
    value : u32
  spec Percent where
    invariant value <= 100

  struct Slot {a : lifetime} where
    target : &[a] mut Percent

  fun store_bad(p : &mut Percent) -> Unit := do
    let slot := new Slot<lifetime _> { target := &mut *p }
    let inner := &mut *(slot.target)
    (*inner).value := 101u32

  fun store_field_bad(pair : &mut (Percent, Percent)) -> Unit := do
    let first := &mut pair.0
    let slot := new Slot<lifetime _> { target := &mut *first }
    let inner := &mut *(slot.target)
    (*inner).value := 101u32


open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `stored_reference.store_bad [(percent, mutationEnd)]
    (← `(command| verify stored_reference::store_bad))
  expectScriptFails `stored_reference.store_bad
    (← `(command| verify stored_reference::store_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
  expectCuts `stored_reference.store_field_bad [(percent, mutationEnd)]
    (← `(command| verify stored_reference::store_field_bad))
  expectScriptFails `stored_reference.store_field_bad
    (← `(command| verify stored_reference::store_field_bad by lean =>
      verex_obligations
      all_goals (verex_prepare <;> (exfalso; omega))))
