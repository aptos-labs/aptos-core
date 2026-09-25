-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Mutable references returned across the call boundary

Returned references and reference tuples, forwarding through further calls, recursion, loans
that end inside loops, early exits across structural, vector, payload, and
global loans, and disjoint mutable parameters written while a loan is live.
Every target verifies automatically.
-/

namespace LeanerLang.Tests.Check.References.ReturnedMutRefs

leaner module 0x42::returned_mut_refs where
  -- ## Types

  struct Cell has Copy, Drop, Store where
    value : u64

  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  struct ExitResource has Key where
    value : u64

  enum Maybe has Copy, Drop, Store where
    | Empty
    | Filled (value : u64)

  enum Duo has Copy, Drop, Store where
    | None
    | Both (left : u64, right : u64)

  -- ## Reference tuples

  fun return_pair(left : &mut u64, right : &mut u64) -> (&mut u64, &mut u64) :=
    (left, right)
  spec return_pair where
    ensures spec.result[0] == old(left) && spec.result[1] == old(right) &&
      left == old(left) && right == old(right)
    aborts_if false

  fun write_pair(left : &mut u64, right : &mut u64) -> Unit := do
    let (returnedLeft, returnedRight) := return_pair(left, right)
    *returnedLeft := 7
    *returnedRight := 9
  spec write_pair where
    ensures left == 7 && right == 9
    aborts_if false

  fun write_pair_then_resume(left : &mut u64, right : &mut u64) -> Unit := do
    let (returnedLeft, returnedRight) := return_pair(left, right)
    *returnedLeft := 7
    *returnedRight := 9
    *left := 11
    *right := 13
  spec write_pair_then_resume where
    ensures left == 11 && right == 13
    aborts_if false

  fun write_then_forward_pair(left : &mut u64, right : &mut u64) ->
      (&mut u64, &mut u64) := do
    let (returnedLeft, returnedRight) := return_pair(left, right)
    *returnedLeft := 5
    *returnedRight := 6
    (returnedRight, returnedLeft)
  spec write_then_forward_pair where
    ensures spec.result[0] == 6 && spec.result[1] == 5 && left == 5 && right == 6
    aborts_if false

  fun write_after_forwarded_pair(left : &mut u64, right : &mut u64) -> Unit := do
    let (first, second) := write_then_forward_pair(left, right)
    *first := 31
    *second := 37
  spec write_after_forwarded_pair where
    ensures left == 37 && right == 31
    aborts_if false

  fun return_pair_fields(pair : &mut Pair) -> (&mut u64, &mut u64) := do
    let left := &mut pair.left
    let right := &mut pair.right
    *left := 2
    *right := 3
    (left, right)
  spec return_pair_fields where
    ensures spec.result[0] == 2 && spec.result[1] == 3 &&
      pair == new Pair { left := 2, right := 3 }
    aborts_if false

  fun write_pair_fields(pair : &mut Pair) -> Unit := do
    let (left, right) := return_pair_fields(pair)
    *left := 41
    *right := 43
  spec write_pair_fields where
    ensures pair == new Pair { left := 41, right := 43 }
    aborts_if false

  fun order_pair(swap : Bool, left : &mut u64, right : &mut u64) ->
      (&mut u64, &mut u64) :=
    if swap then (right, left) else (left, right)
  spec order_pair where
    ensures spec.result[0] == (if swap then old(right) else old(left)) &&
      spec.result[1] == (if swap then old(left) else old(right)) &&
      left == old(left) && right == old(right)
    aborts_if false

  fun write_ordered_pair(swap : Bool, left : &mut u64, right : &mut u64) -> Unit := do
    let (first, second) := order_pair(swap, left, right)
    *first := 19
    *second := 23
  spec write_ordered_pair where
    ensures if swap then (left == 23 && right == 19) else (left == 19 && right == 23)
    aborts_if false

  fun return_mixed(left : &mut u64, flag : Bool, right : &mut u64) ->
      (&mut u64, Bool, &mut u64) :=
    (left, flag, right)
  spec return_mixed where
    ensures spec.result[0] == old(left) && spec.result[1] == flag &&
      spec.result[2] == old(right) && left == old(left) && right == old(right)
    aborts_if false

  fun write_mixed(left : &mut u64, right : &mut u64) -> Bool := do
    let (returnedLeft, flag, returnedRight) := return_mixed(left, true, right)
    *returnedLeft := 13
    *returnedRight := 17
    flag
  spec write_mixed where
    ensures result == true && left == 13 && right == 17
    aborts_if false

  -- ## Forwarding, recursion, and choice

  fun identity(slot : &mut u64) -> &mut u64 := slot
  spec identity where
    ensures result == old(slot) && slot == old(slot)
    aborts_if false

  /-- A returned loan can cross another function boundary without exposing
  its origin. The inner call's updated lender carrier becomes the outer
  mutation-level result's carrier directly. -/
  fun forward_identity(slot : &mut u64) -> &mut u64 := do
    let returned := identity(slot)
    returned
  spec forward_identity where
    ensures result == old(slot) && slot == old(slot)
    aborts_if false

  /-- Forwarding preserves the returned mutation's prophecy after local use;
  it is not restricted to an immediate syntactic return. -/
  fun write_then_forward(slot : &mut u64) -> &mut u64 := do
    let returned := identity(slot)
    *returned := 13
    returned
  spec write_then_forward where
    ensures result == 13 && slot == 13
    aborts_if false

  fun write_after_nontrivial_forward(slot : &mut u64) -> Unit := do
    let returned := write_then_forward(slot)
    *returned := 17
  spec write_after_nontrivial_forward where
    ensures slot == 17
    aborts_if false

  fun write_forwarded(slot : &mut u64) -> Unit := do
    let returned := forward_identity(slot)
    *returned := 11
  spec write_forwarded where
    ensures slot == 11
    aborts_if false

  fun recursive_identity(done : Bool, slot : &mut u64) -> &mut u64 :=
    if done then slot
    else do
      let returned := recursive_identity(true, slot)
      return returned
  spec recursive_identity where
    ensures result == old(slot) && slot == old(slot)
    aborts_if false

  fun mutual_return_left(done : Bool, slot : &mut u64) -> &mut u64 :=
    if done then slot
    else mutual_return_right(true, slot)
  spec mutual_return_left where
    ensures result == old(slot) && slot == old(slot)
    aborts_if false

  fun mutual_return_right(done : Bool, slot : &mut u64) -> &mut u64 :=
    if done then slot
    else do
      let returned := mutual_return_left(true, slot)
      return returned
  spec mutual_return_right where
    ensures result == old(slot) && slot == old(slot)
    aborts_if false

  /-- Move signatures carry no lifetime identifying the selected input.  The
  mutation relation can retain the branch-sensitive prophecy equations, while
  a caller must conservatively suspend both inputs. -/
  fun choose(takeLeft : Bool, left : &mut u64, right : &mut u64) -> &mut u64 :=
    if takeLeft then left else right
  spec choose where
    ensures result == (if takeLeft then old(left) else old(right)) &&
      left == old(left) && right == old(right)
    aborts_if false

  fun forward_choose(takeLeft : Bool, left : &mut u64, right : &mut u64) ->
      &mut u64 := do
    let returned := choose(takeLeft, left, right)
    returned
  spec forward_choose where
    ensures result == (if takeLeft then old(left) else old(right)) &&
      left == old(left) && right == old(right)
    aborts_if false

  fun write_identity(slot : &mut u64) -> Unit := do
    let returned := identity(slot)
    *returned := 9
  spec write_identity where
    ensures slot == 9
    aborts_if false

  /-- The input is revived from its prophecy once the returned loan reaches
  its last use, so it can be written again in the continuation. -/
  fun write_then_resume(slot : &mut u64) -> Unit := do
    let returned := identity(slot)
    *returned := 9
    *slot := 10
  spec write_then_resume where
    ensures slot == 10
    aborts_if false

  -- ## Loans ending in loops and early exits

  /-- Each returned loan ends inside the iteration. The outer mutation then
  resumes with its enclosing prophecy still open, so it can be passed again
  on the next iteration and written after the loop. -/
  fun repeat_returned_loan(slot : &mut u64, count : u64) -> Unit := do
    let mut i : u64 := 0
    while i < count do
      let returned := identity(slot)
      *returned := i
      i := i + 1
    where
      invariant i <= count
    *slot := 10
  spec repeat_returned_loan where
    ensures slot == 10
    aborts_if false

  /-- A returned mutation may itself remain live across loop iterations. Its
  possible lender stays suspended until the loop and the result's last use
  finish, then resumes for the continuation. -/
  fun loop_carried_return(slot : &mut u64, count : u64) -> Unit := do
    let returned := identity(slot)
    let mut i : u64 := 0
    while i < count do
      *returned := i
      i := i + 1
    where
      invariant i <= count
    *slot := 10
  spec loop_carried_return where
    ensures slot == 10
    aborts_if false

  /-- A loop exit unwinds a returned loan before running the post-loop
  continuation, so the conservatively poisoned input is revived there. -/
  fun break_with_returned_loan(slot : &mut u64, stop : Bool) -> Unit := do
    let mut i : u64 := 0
    while i < 2 do
      let returned := identity(slot)
      if stop then break
      *returned := 7
      i := i + 1
    where
      invariant i <= 2
    *slot := 10
  spec break_with_returned_loan where
    ensures slot == 10
    aborts_if false

  /-- Every component of a returned-reference tuple is resolved before a
  loop exit revives the shared conservative lender set. -/
  fun break_with_returned_pair(left : &mut u64, right : &mut u64, stop : Bool) ->
      Unit := do
    let mut i : u64 := 0
    while i < 2 do
      let (returnedLeft, returnedRight) := return_pair(left, right)
      if stop then break
      *returnedLeft := 7
      *returnedRight := 8
      i := i + 1
    where
      invariant i <= 2
    *left := 10
    *right := 11
  spec break_with_returned_pair where
    ensures left == 10 && right == 11
    aborts_if false

  /-- `continue` likewise reconciles a returned loan before invoking the next
  loop approximation; the next iteration receives the revived root. -/
  fun continue_with_returned_loan(slot : &mut u64, count : u64, skipWrite : Bool) ->
      Unit := do
    let mut i : u64 := 0
    while i < count do
      let returned := identity(slot)
      i := i + 1
      if skipWrite then continue
      *returned := i
    where
      invariant i <= count
    *slot := 10
  spec continue_with_returned_loan where
    ensures slot == 10
    aborts_if false

  /-- An early function return also reconciles the loan and skips the normal
  post-loan continuation. -/
  fun return_with_returned_loan(slot : &mut u64, early : Bool) -> Unit := do
    let returned := identity(slot)
    if early then return ();
    *returned := 1
    *slot := 2
  spec return_with_returned_loan where
    ensures if early then slot == old(slot) else slot == 2
    aborts_if false

  /-- A break targeting a loop created inside the loan does not unwind the
  loan; the returned reference remains usable after that inner loop. -/
  fun inner_break_with_returned_loan(slot : &mut u64) -> Unit := do
    let returned := identity(slot)
    let mut i : u64 := 0
    while i < 2 do
      if i == 1 then break
      i := i + 1
    where
      invariant i <= 2
    *returned := 3
    *slot := 4
  spec inner_break_with_returned_loan where
    ensures slot == 4
    aborts_if false

  /-- Exiting through nested returned loans unwinds them inside-out before the
  loop continuation sees the original root again. -/
  fun break_with_nested_returned_loans(slot : &mut u64, stop : Bool) -> Unit := do
    let mut i : u64 := 0
    while i < 2 do
      let outer := identity(slot)
      let inner := identity(outer)
      if stop then break
      *inner := 7
      i := i + 1
    where
      invariant i <= 2
    *slot := 10
  spec break_with_nested_returned_loans where
    ensures slot == 10
    aborts_if false

  /-- The same unwind rule applies to a direct loan of an owned local. -/
  fun break_with_local_loan(stop : Bool) -> u64 := do
    let mut owner : u64 := 0
    let mut i : u64 := 0
    while i < 2 do
      let borrowed := &mut owner
      if stop then break
      *borrowed := 7
      i := i + 1
    where
      invariant i <= 2
    owner := 10
    owner
  spec break_with_local_loan where
    ensures result == 10
    aborts_if false

  /-- A structural child loan rebuilds its parent mutation before leaving the
  loop, so a later borrow observes a live complete owner. -/
  fun break_with_field_loan(cell : &mut Cell, stop : Bool) -> Unit := do
    let mut i : u64 := 0
    while i < 2 do
      let value := &mut cell.value
      if stop then break
      *value := 7
      i := i + 1
    where
      invariant i <= 2
    let finalValue := &mut cell.value
    *finalValue := 10
  spec break_with_field_loan where
    ensures cell.value == 10
    aborts_if false

  /-- Vector-element loan reconciliation updates the vector mutation before
  control leaves the loop. -/
  fun break_with_vector_element_loan(values : &mut Vector<u64>, stop : Bool) ->
      Unit := do
    let mut i : u64 := 0
    while i < 2 do
      let first := &mut values[0]
      if stop then break
      *first := 7
      i := i + 1
    where
      invariant i <= 2 && 0 < values.length
    let finalFirst := &mut values[0]
    *finalFirst := 10
  spec break_with_vector_element_loan where
    requires 0 < values.length
    ensures 0 < values.length && values[0] == 10
    aborts_if false

  /-- A global resource is restored to its store before an early return; the
  normal post-loan continuation is skipped on that path. -/
  fun return_with_global_loan(address : Address, early : Bool) -> Unit := do
    let borrowed := &mut ExitResource[address].value
    if early then return ();
    *borrowed := 7
    let finalValue := &mut ExitResource[address].value
    *finalValue := 10
  spec return_with_global_loan where
    requires exists<ExitResource>(address)
    modifies global<ExitResource>(address)
    ensures global<ExitResource>(address).value ==
      (if early then old(global<ExitResource>(address).value) else 10)
    aborts_if false

  -- ## Writes to disjoint parameters

  /-- A disjoint outer mutation may be written while an owned-local loan is
  live; the post-loan closure transports both updates. -/
  fun write_outer_during_local_loan(outer : &mut u64) -> u64 := do
    let mut owner : u64 := 0
    let borrowed := &mut owner
    *outer := 5
    *borrowed := 7
    let result := *borrowed
    result
  spec write_outer_during_local_loan where
    ensures outer == 5 && result == 7
    aborts_if false

  fun set_five(value : &mut u64) -> Unit := do
    *value := 5

  /-- Updates performed by callees must also cross an unrelated live loan;
  inspecting only source assignments would miss this case. -/
  fun call_outer_during_local_loan(outer : &mut u64) -> u64 := do
    let mut owner : u64 := 0
    let borrowed := &mut owner
    set_five(outer)
    *borrowed := 7
    let result := *borrowed
    result
  spec call_outer_during_local_loan where
    ensures outer == 5 && result == 7
    aborts_if false

  /-- Structural reconciliation and a disjoint mutation update are both
  retained when the child loan dies. -/
  fun write_outer_during_field_loan(cell : &mut Cell, outer : &mut u64) -> Unit := do
    let value := &mut cell.value
    *outer := 5
    *value := 7
  spec write_outer_during_field_loan where
    ensures cell.value == 7 && outer == 5
    aborts_if false

  /-- An element loan similarly transports writes to an independent mutable
  parameter while rebuilding its vector owner. -/
  fun write_outer_during_vector_loan(values : &mut Vector<u64>, outer : &mut u64) ->
      Unit := do
    let first := &mut values[0]
    *outer := 5
    *first := 7
  spec write_outer_during_vector_loan where
    requires 0 < values.length
    ensures 0 < values.length && values[0] == 7 && outer == 5
    aborts_if false

  /-- A global loan keeps disjoint mutable parameters live in its returned
  continuation while the resource is checked out of the store. -/
  fun write_outer_during_global_loan(outer : &mut u64, address : Address) -> Unit := do
    let value := &mut ExitResource[address].value
    *outer := 5
    *value := 7
  spec write_outer_during_global_loan where
    requires exists<ExitResource>(address)
    modifies global<ExitResource>(address)
    ensures outer == 5 && global<ExitResource>(address).value == 7
    aborts_if false

  /-- A returned loan poisons only its possible lenders. Updates to another
  mutable parameter are captured until those lenders resume. -/
  fun write_outer_during_returned_loan(slot : &mut u64, outer : &mut u64) -> Unit := do
    let returned := identity(slot)
    *outer := 5
    *returned := 7
  spec write_outer_during_returned_loan where
    ensures slot == 7 && outer == 5
    aborts_if false

  /-- Multiple returned loans also retain an independent outer mutation. -/
  fun write_outer_during_returned_pair(left : &mut u64, right : &mut u64,
      outer : &mut u64) -> Unit := do
    let (returnedLeft, returnedRight) := return_pair(left, right)
    *outer := 5
    *returnedLeft := 7
    *returnedRight := 9
  spec write_outer_during_returned_pair where
    ensures left == 7 && right == 9 && outer == 5
    aborts_if false

  /-- Enum payload loans rebuild the variant while retaining a write to a
  disjoint mutable parameter. -/
  fun write_outer_during_payload_loan(choice : &mut Maybe, outer : &mut u64) -> Unit :=
    match choice with
      | Maybe::Empty {} => abort(7)
      | Maybe::Filled { value := value } => do
          *outer := 5
          *value := 7
  spec write_outer_during_payload_loan where
    requires choice is Filled
    ensures choice == new Maybe::Filled { value := 7 } && outer == 5
    aborts_if false

  -- ## Several possible lenders

  fun write_chosen(takeLeft : Bool, left : &mut u64, right : &mut u64) -> Unit := do
    let returned := forward_choose(takeLeft, left, right)
    *returned := 9
  spec write_chosen where
    ensures if takeLeft then (left == 9 && right == old(right))
      else (left == old(left) && right == 9)
    aborts_if false

  /-- The modular call rule suspends every one of three possible lenders;
  after the result resolves, all three carriers resume in parameter order. -/
  fun choose_three(takeFirst : Bool, takeSecond : Bool, first : &mut u64,
      second : &mut u64, third : &mut u64) -> &mut u64 :=
    if takeFirst then first
    else
      if takeSecond then second else third
  spec choose_three where
    ensures result == (if takeFirst then old(first)
        else (if takeSecond then old(second) else old(third))) &&
      first == old(first) && second == old(second) && third == old(third)
    aborts_if false

  fun write_chosen_three(takeFirst : Bool, takeSecond : Bool, first : &mut u64,
      second : &mut u64, third : &mut u64) -> Unit := do
    let returned := choose_three(takeFirst, takeSecond, first, second, third)
    *returned := 31
  spec write_chosen_three where
    ensures if takeFirst then
        (first == 31 && second == old(second) && third == old(third))
      else (if takeSecond then
        (first == old(first) && second == 31 && third == old(third))
      else (first == old(first) && second == old(second) && third == 31))
    aborts_if false

  /-- Mutable parameter opening is arity-independent. This regression crosses
  the former three-parameter limit and also checks that every unrelated lender
  resumes unchanged after the returned loan is resolved. -/
  fun return_one_of_four(first : &mut u64, second : &mut u64, third : &mut u64,
      fourth : &mut u64) -> &mut u64 :=
    first
  spec return_one_of_four where
    ensures result == old(first) && first == old(first) &&
      second == old(second) && third == old(third) && fourth == old(fourth)
    aborts_if false

  fun write_first_of_four(first : &mut u64, second : &mut u64, third : &mut u64,
      fourth : &mut u64) -> Unit := do
    let returned := return_one_of_four(first, second, third, fourth)
    *returned := 41
  spec write_first_of_four where
    ensures first == 41 && second == old(second) &&
      third == old(third) && fourth == old(fourth)
    aborts_if false

  -- ## Structural, vector, and payload children

  fun borrow_value(cell : &mut Cell) -> &mut u64 := do
    let value := &mut cell.value
    value
  spec borrow_value where
    ensures result == old(cell).value && cell == old(cell)
    aborts_if false

  /-- A field selected for return on one loop path is reconciled when another
  path breaks and selects a fresh loan after the loop. -/
  fun borrow_value_after_break(stop : Bool, cell : &mut Cell) -> &mut u64 := do
    loop do
      let value := &mut cell.value
      if stop then break
      return value;
    spec do
      invariant cell == old(cell)
    let afterValue := &mut cell.value
    afterValue
  spec borrow_value_after_break where
    ensures result == old(cell).value && cell == old(cell)
    aborts_if false

  /-- Continuing an iteration also resolves the temporary child before the
  next fixed-point invocation reopens it. The last iteration transfers it. -/
  fun borrow_value_after_continue(count : u64, cell : &mut Cell) -> &mut u64 := do
    let mut i : u64 := 0
    while i < count do
      let value := &mut cell.value
      i := i + 1
      if i < count then continue
      if i == count then return value;
      -- Retain a syntactic loop tail for executable lowering. The invariant
      -- and loop guard make this edge unreachable.
      let current := *value
      *value := current
    where
      invariant i <= count && cell == old(cell)
    let afterValue := &mut cell.value
    afterValue
  spec borrow_value_after_continue where
    ensures result == old(cell).value && cell == old(cell)
    aborts_if false

  /-- A labeled exit crosses both the returned field loan and its inner loop,
  then reopens the field in the named loop's continuation. -/
  fun borrow_value_after_labeled_break(stop : Bool, cell : &mut Cell) -> &mut u64 := do
    loop@outer do
      loop do
        let value := &mut cell.value
        if stop then break@outer
        return value;
      spec do
        invariant cell == old(cell)
    spec do
      invariant cell == old(cell)
    let afterValue := &mut cell.value
    afterValue
  spec borrow_value_after_labeled_break where
    ensures result == old(cell).value && cell == old(cell)
    aborts_if false

  /-- Returning a different mutable input abandons the selected structural
  child: its prophecy is resolved before the other root crosses the boundary. -/
  fun borrow_value_or_slot(takeSlot : Bool, cell : &mut Cell, slot : &mut u64) ->
      &mut u64 := do
    let value := &mut cell.value
    if takeSlot then return slot;
    value
  spec borrow_value_or_slot where
    ensures true
    aborts_if false

  fun write_value_or_slot(takeSlot : Bool, cell : &mut Cell, slot : &mut u64) ->
      Unit := do
    let selected := borrow_value_or_slot(takeSlot, cell, slot)
    *selected := 79
  spec write_value_or_slot where
    ensures if takeSlot then (cell == old(cell) && slot == 79)
      else (cell.value == 79 && slot == old(slot))
    aborts_if false

  /-- The forwarding boundary remains path-free even when the inner result was
  derived from a structure field. Only the updated root carrier crosses each
  call boundary. -/
  fun forward_value(cell : &mut Cell) -> &mut u64 := do
    let value := borrow_value(cell)
    value
  spec forward_value where
    ensures result == old(cell).value && cell == old(cell)
    aborts_if false

  fun write_then_forward_value(cell : &mut Cell) -> &mut u64 := do
    let value := borrow_value(cell)
    *value := 29
    value
  spec write_then_forward_value where
    ensures result == 29 && cell.value == 29
    aborts_if false

  fun write_after_nontrivial_value_forward(cell : &mut Cell) -> Unit := do
    let value := write_then_forward_value(cell)
    *value := 31
  spec write_after_nontrivial_value_forward where
    ensures cell.value == 31
    aborts_if false

  fun write_forwarded_value(cell : &mut Cell) -> Unit := do
    let value := forward_value(cell)
    *value := 19
  spec write_forwarded_value where
    ensures cell.value == 19
    aborts_if false

  fun write_value(cell : &mut Cell) -> Unit := do
    let value := borrow_value(cell)
    *value := 17
  spec write_value where
    ensures cell.value == 17
    aborts_if false

  fun borrow_first(values : &mut Vector<u64>) -> &mut u64 := do
    let first := &mut values[0]
    first
  spec borrow_first where
    requires 0 < values.length
    ensures true
    aborts_if false

  /-- A vector-element transfer is resolved on the break path before the
  element is borrowed again after the loop. -/
  fun borrow_first_after_break(stop : Bool, values : &mut Vector<u64>) -> &mut u64 := do
    loop do
      let first := &mut values[0]
      if stop then break
      return first;
    spec do
      invariant 0 < values.length
    let afterFirst := &mut values[0]
    afterFirst
  spec borrow_first_after_break where
    requires 0 < values.length
    ensures true
    aborts_if false

  fun write_first(values : &mut Vector<u64>) -> Unit := do
    let first := borrow_first(values)
    *first := 23
  spec write_first where
    requires 0 < values.length
    ensures 0 < values.length && values[0] == 23
    aborts_if false

  fun return_vector_element_and_slot(values : &mut Vector<u64>, slot : &mut u64) ->
      (&mut u64, &mut u64) := do
    let first := &mut values[0]
    (first, slot)
  spec return_vector_element_and_slot where
    requires 0 < values.length
    ensures true
    aborts_if false

  fun write_vector_element_and_slot(values : &mut Vector<u64>, slot : &mut u64) ->
      Unit := do
    let (first, returnedSlot) := return_vector_element_and_slot(values, slot)
    *first := 47
    *returnedSlot := 53
  spec write_vector_element_and_slot where
    requires 0 < values.length
    ensures 0 < values.length && values[0] == 47 && slot == 53
    aborts_if false

  fun borrow_payload(slot : &mut Maybe) -> &mut u64 :=
    match slot with
      | Maybe::Empty {} => abort(7)
      | Maybe::Filled { value := value } => value
  spec borrow_payload where
    pragma aborts_if_is_partial
    ensures (match old(slot) with
      | Maybe::Empty {} => true
      | Maybe::Filled { value := value } => result == value)
    aborts_if slot is Empty with 7

  /-- A variant payload transfer is resolved and its owner rebuilt on the
  break path before a second payload match returns a fresh loan. -/
  fun borrow_payload_after_break(stop : Bool, slot : &mut Maybe) -> &mut u64 := do
    loop do
      match slot with
        | Maybe::Empty {} => abort(7)
        | Maybe::Filled { value := value } => do
            if stop then break
            return value;
    spec do
      invariant slot is Filled
    match slot with
      | Maybe::Empty {} => abort(7)
      | Maybe::Filled { value := value } => value
  spec borrow_payload_after_break where
    requires slot is Filled
    ensures true
    aborts_if false

  /-- Every transferred payload child is resolved inside-out on the break
  edge; the post-loop match can then return a fresh pair. -/
  fun borrow_payload_pair_after_break(stop : Bool, duo : &mut Duo) ->
      (&mut u64, &mut u64) := do
    loop do
      match duo with
        | Duo::None {} => abort(9)
        | Duo::Both { left := left, right := right } => do
            if stop then break
            return (left, right);
    spec do
      invariant ∃ (left : u64; right : u64),
        duo == new Duo::Both { left := left, right := right }
    match duo with
      | Duo::None {} => abort(9)
      | Duo::Both { left := left, right := right } => (left, right)
  spec borrow_payload_pair_after_break where
    requires ∃ (left : u64; right : u64),
      duo == new Duo::Both { left := left, right := right }
    ensures true
    aborts_if false

  fun write_payload(slot : &mut Maybe) -> Unit := do
    let value := borrow_payload(slot)
    *value := 29
  spec write_payload where
    requires slot is Filled
    ensures slot == new Maybe::Filled { value := 29 }
    aborts_if false

  fun return_payload_and_slot(choice : &mut Maybe, slot : &mut u64) ->
      (&mut u64, &mut u64) :=
    match choice with
      | Maybe::Empty {} => abort(7)
      | Maybe::Filled { value := value } => (value, slot)
  spec return_payload_and_slot where
    requires choice is Filled
    ensures true
    aborts_if false

  fun write_payload_and_slot(choice : &mut Maybe, slot : &mut u64) -> Unit := do
    let (value, returnedSlot) := return_payload_and_slot(choice, slot)
    *value := 59
    *returnedSlot := 61
  spec write_payload_and_slot where
    requires choice is Filled
    ensures choice == new Maybe::Filled { value := 59 } && slot == 61
    aborts_if false

  fun return_payload_pair(duo : &mut Duo) -> (&mut u64, &mut u64) :=
    match duo with
      | Duo::None {} => abort(9)
      | Duo::Both { left := left, right := right } => (left, right)
  spec return_payload_pair where
    requires ∃ (left : u64; right : u64),
      duo == new Duo::Both { left := left, right := right }
    ensures true
    aborts_if false

  fun write_payload_pair(duo : &mut Duo) -> Unit := do
    let (left, right) := return_payload_pair(duo)
    *left := 67
    *right := 71
  spec write_payload_pair where
    requires ∃ (left : u64; right : u64),
      duo == new Duo::Both { left := left, right := right }
    ensures duo == new Duo::Both { left := 67, right := 71 }
    aborts_if false

/-! A contract stating a returned reference's final value (`final(result)`)
relates later writes through the reference to its lender, so callers use the
contract instead of the body: a native returning `&mut` is called through its
contract alone. -/

leaner module 0x42::returned_mut_refs_final where
  struct Cell has Copy, Drop, Store where
    value : u64

  fun borrow_value(cell : &mut Cell) -> &mut u64 := &mut cell.value
  spec borrow_value where
    ensures result == old(cell).value && cell.value == final(result)
    aborts_if false

  fun set_value(cell : &mut Cell) -> Unit := do
    let value := borrow_value(cell)
    *value := 9
  spec set_value where
    ensures cell.value == 9
    aborts_if false

  native fun summarized_native(slot : &mut u64) -> &mut u64
  spec summarized_native where
    ensures result == old(slot) && slot == final(result)
    aborts_if false

  fun call_summarized_native(slot : &mut u64) -> Unit := do
    let returned := summarized_native(slot)
    *returned := 37
  spec call_summarized_native where
    ensures slot == 37
    aborts_if false

end LeanerLang.Tests.Check.References.ReturnedMutRefs
