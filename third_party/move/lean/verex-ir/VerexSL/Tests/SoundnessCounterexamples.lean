-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# The counterexamples of the independent soundness reviews

Each negative here was a program the verifier **accepted** at some point, and
each is kept with the *authored* proof attempt that exploited the hole, not
only with the automatic tactic. That distinction is the point of this file: in
the first review the automatic tactic already rejected the mixed-loan
counterexample while a three-line authored proof still proved `ensures false`,
so a test that merely expects automatic verification to fail cannot guard these
holes. Every negative therefore runs its authored proof, pins the rejection
with `#guard_msgs`, and asserts that no proof artifact was published.

The controls that must verify are kept beside them, so that an unsupported
construct or a compile failure cannot pass for enforcement.
-/

open Lean Meta Elab Tactic

/-! ## Review 1, P1 — a join of a global borrow and a reborrow

The holder may hold either alternative, so both lenders' obligations are owed
before the holder's prophecy equation can prune a run. Certifying only one of
them left the state contradictory and `ensures false` provable. Both branch
orders are kept because the defect made the verdict depend on the order. -/

verex module 0x42::review1_join where
  pragma verify = false
  struct S has Key, Store where
    v : u64
  spec S where
    invariant v == 0
  struct Outer where
    inner : S
  fun mixed(b : Bool, addr : Address, o : &mut Outer) -> Unit := do
    let r := if b then &mut S[addr] else &mut o.inner
    r.v := 1
  spec mixed where
    requires exists<S>(addr)
    ensures false
    modifies *
    aborts_if false
  fun mixed_swapped(b : Bool, addr : Address, o : &mut Outer) -> Unit := do
    let r := if b then &mut o.inner else &mut S[addr]
    r.v := 1
  spec mixed_swapped where
    requires exists<S>(addr)
    ensures false
    modifies *
    aborts_if false
  fun mixed_ok(b : Bool, addr : Address, o : &mut Outer) -> Unit := do
    let r := if b then &mut S[addr] else &mut o.inner
    r.v := 0
  spec mixed_ok where
    requires exists<S>(addr)
    modifies *
    aborts_if false
  fun mixed_ok_swapped(b : Bool, addr : Address, o : &mut Outer) -> Unit := do
    let r := if b then &mut o.inner else &mut S[addr]
    r.v := 0
  spec mixed_ok_swapped where
    requires exists<S>(addr)
    modifies *
    aborts_if false

/--
error: omega could not prove the goal:
No usable constraints found. You may need to unfold definitions so `omega` can see linear arithmetic facts about `Nat` and `Int`, which may also involve multiplication, division, and modular remainder by constants.
---
error: omega could not prove the goal:
No usable constraints found. You may need to unfold definitions so `omega` can see linear arithmetic facts about `Nat` and `Int`, which may also involve multiplication, division, and modular remainder by constants.
---
error: verex verification failed: the proof of `mixed` does not establish its specification
-/
#guard_msgs in
verify 0x42::review1_join::mixed by lean =>
  verex_obligations
  all_goals (verex_prepare <;> omega)

/--
error: omega could not prove the goal:
No usable constraints found. You may need to unfold definitions so `omega` can see linear arithmetic facts about `Nat` and `Int`, which may also involve multiplication, division, and modular remainder by constants.
---
error: omega could not prove the goal:
No usable constraints found. You may need to unfold definitions so `omega` can see linear arithmetic facts about `Nat` and `Int`, which may also involve multiplication, division, and modular remainder by constants.
---
error: verex verification failed: the proof of `mixed_swapped` does not establish its specification
-/
#guard_msgs in
verify 0x42::review1_join::mixed_swapped by lean =>
  verex_obligations
  all_goals (verex_prepare <;> omega)

run_cmd do
  let env ← getEnv
  for name in [`«0x42».review1_join.mixed.typedVerified,
      `«0x42».review1_join.mixed.verified,
      `«0x42».review1_join.mixed_swapped.typedVerified,
      `«0x42».review1_join.mixed_swapped.verified] do
    if env.contains name then throwError "rejected proof published {name}"

-- The controls: the same joins, in both orders, writing a value the invariant
-- allows. These must verify, so that the negatives above cannot be passing
-- because the construct is unsupported.
verify 0x42::review1_join::mixed_ok
verify 0x42::review1_join::mixed_ok_swapped

/-! ## Review 1, P2 — an explicit early return

`Term.returned` certified only the fall-through outcome, and `Flow.bind` passes
`return` through without running a continuation, so a reference whose referent
violated its invariant escaped. The nested and loop forms are kept because the
frontend normalizes a function-tail return into a value expression, which masks
the defect in a simple tail-return test. -/

verex module 0x42::review1_return where
  pragma verify = false
  struct S where
    v : u64
  spec S where
    invariant v == 0
  fun early_return(b : Bool, s : &mut S) -> &mut S := do
    if b then
      s.v := 1
      return s
    s
  fun nested_return(b : Bool, c : Bool, s : &mut S) -> &mut S := do
    if b then
      if c then
        s.v := 1
        return s
      s
    else
      s
  fun loop_return(n : u64, s : &mut S) -> &mut S := do
    let i := 0
    while i < n do
      s.v := 1
      return s
    s
  fun early_return_ok(b : Bool, s : &mut S) -> &mut S := do
    if b then
      s.v := 0
      return s
    s
  fun nested_return_ok(b : Bool, c : Bool, s : &mut S) -> &mut S := do
    if b then
      if c then
        s.v := 0
        return s
      s.v := 0
      return s
    s
  fun loop_return_ok(n : u64, s : &mut S) -> &mut S := do
    let i := 0
    while i < n do
      s.v := 0
      return s
    s

/--
error: omega could not prove the goal:
No usable constraints found. You may need to unfold definitions so `omega` can see linear arithmetic facts about `Nat` and `Int`, which may also involve multiplication, division, and modular remainder by constants.
---
error: verex verification failed: the proof of `early_return` does not establish its specification
-/
#guard_msgs in
verify 0x42::review1_return::early_return by lean =>
  verex_obligations
  all_goals (verex_prepare <;> omega)

/--
error: omega could not prove the goal:
No usable constraints found. You may need to unfold definitions so `omega` can see linear arithmetic facts about `Nat` and `Int`, which may also involve multiplication, division, and modular remainder by constants.
---
error: verex verification failed: the proof of `nested_return` does not establish its specification
-/
#guard_msgs in
verify 0x42::review1_return::nested_return by lean =>
  verex_obligations
  all_goals (verex_prepare <;> omega)

/--
error: omega could not prove the goal:
a possible counterexample may satisfy the constraints
  1 ≤ a ≤ 18446744073709551615
where
 a := n.val
---
error: verex verification failed: the proof of `loop_return` does not establish its specification
-/
#guard_msgs in
verify 0x42::review1_return::loop_return by lean =>
  verex_obligations
  all_goals (verex_prepare <;> omega)

run_cmd do
  let env ← getEnv
  for name in [`«0x42».review1_return.early_return.typedVerified,
      `«0x42».review1_return.early_return.verified,
      `«0x42».review1_return.nested_return.typedVerified,
      `«0x42».review1_return.nested_return.verified,
      `«0x42».review1_return.loop_return.typedVerified,
      `«0x42».review1_return.loop_return.verified] do
    if env.contains name then throwError "rejected proof published {name}"

verify 0x42::review1_return::early_return_ok
verify 0x42::review1_return::nested_return_ok
verify 0x42::review1_return::loop_return_ok

/-! ## Review 2 — a place's index operand is read at the loan's death

A local index compiles to `PlaceIndex.slot`, which is resolved against the
environment supplied at resolution, while the reference denotes the element
chosen when the borrow happened; the runtime resolves the index once and
records the resolved place. Until the lender carries the operands as evaluated
at the borrow, assigning such a local is refused, so these four programs are
rejected with the unsupported-construct diagnostic rather than verified against
the wrong element. The borrow analysis decides it with its own liveness, so a
loop that borrows anew each iteration and increments the index after the loan
died — `bit_vector`'s shape — is carried and verifies (the controls below). The joined form is the one this session's canonical lender
identity newly routed through precise resolution; the single-borrow form was
already accepted before it, which is why both are kept. -/

verex module 0x42::review2_index where
  pragma verify = false
  struct Box where
    values : Vector<u64>
  fun changed_index(b : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0]
    let mut i : u64 := 0
    let r := if b then &mut values[i] else &mut values[i]
    i := 1
    *r := 7
    values[1]
  spec changed_index where
    ensures false
    aborts_if false
  fun changed_index_single() -> u64 := do
    let mut values := vector<u64>[0, 0]
    let mut i : u64 := 0
    let r := &mut values[i]
    i := 1
    *r := 7
    values[1]
  spec changed_index_single where
    ensures false
    aborts_if false
  fun changed_index_under_field() -> u64 := do
    let mut box := new Box { values := vector<u64>[0, 0] }
    let mut i : u64 := 0
    let r := &mut box.values[i]
    i := 1
    *r := 7
    box.values[1]
  spec changed_index_under_field where
    ensures false
    aborts_if false
  fun changed_index_under_deref(box : &mut Box) -> u64 := do
    let mut i : u64 := 0
    let r := &mut box.values[i]
    i := 1
    *r := 7
    box.values[1]
  spec changed_index_under_deref where
    requires box.values.length == 2
    ensures false
    aborts_if false

/--
error: no denotation for `changed_index`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_index::changed_index by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

/--
error: no denotation for `changed_index_single`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_index::changed_index_single by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

/--
error: no denotation for `changed_index_under_field`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_index::changed_index_under_field by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

/--
error: no denotation for `changed_index_under_deref`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_index::changed_index_under_deref by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

run_cmd do
  let env ← getEnv
  for name in [`«0x42».review2_index.changed_index.typedVerified,
      `«0x42».review2_index.changed_index.verified,
      `«0x42».review2_index.changed_index_single.typedVerified,
      `«0x42».review2_index.changed_index_single.verified,
      `«0x42».review2_index.changed_index_under_field.typedVerified,
      `«0x42».review2_index.changed_index_under_field.verified,
      `«0x42».review2_index.changed_index_under_deref.typedVerified,
      `«0x42».review2_index.changed_index_under_deref.verified] do
    if env.contains name then throwError "rejected proof published {name}"

-- The control: the index is not reassigned while the loan is live, so the
-- operand read at the death is the one the borrow chose, the construct is
-- carried, and the true postcondition is provable.
verex module 0x42::review2_captured where
  struct Box where
    values : Vector<u64>
  fun captured_index(b : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0]
    let i : u64 := 0
    let r := if b then &mut values[i] else &mut values[i]
    *r := 7
    values[1]
  spec captured_index where
    ensures result == 0
    aborts_if false
  fun captured_index_under_field() -> u64 := do
    let mut box := new Box { values := vector<u64>[0, 0] }
    let i : u64 := 0
    let r := &mut box.values[i]
    *r := 7
    box.values[1]
  spec captured_index_under_field where
    ensures result == 0
    aborts_if false

/-! ## Review 2 — the other write paths, and the liveness controls

The index local can change without a plain assignment: through a mutable
reference to it, or in a callee that takes it by `&mut`. Both are refused while
the indexing loan is live, with the same authored proof as above. -/

verex module 0x42::review2_write_paths where
  pragma verify = false
  fun write_via_ref() -> u64 := do
    let mut values := vector<u64>[0, 0]
    let mut i : u64 := 0
    let r := &mut values[i]
    let p := &mut i
    *p := 1
    *r := 7
    values[1]
  spec write_via_ref where
    ensures false
    aborts_if false
  fun bump(x : &mut u64) -> Unit := do
    *x := 1
  fun write_via_callee() -> u64 := do
    let mut values := vector<u64>[0, 0]
    let mut i : u64 := 0
    let r := &mut values[i]
    bump(&mut i)
    *r := 7
    values[1]
  spec write_via_callee where
    ensures false
    aborts_if false

/--
error: no denotation for `write_via_ref`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_write_paths::write_via_ref by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

/--
error: no denotation for `write_via_callee`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_write_paths::write_via_callee by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

run_cmd do
  let env ← getEnv
  for name in [`«0x42».review2_write_paths.write_via_ref.typedVerified,
      `«0x42».review2_write_paths.write_via_ref.verified,
      `«0x42».review2_write_paths.write_via_callee.typedVerified,
      `«0x42».review2_write_paths.write_via_callee.verified] do
    if env.contains name then throwError "rejected proof published {name}"

-- The liveness controls. `clear_all` is `bit_vector::shift_left_for_verification_only`'s
-- loop: borrow the element, write through it, increment the index after the
-- loan has died. `write_after_death` does the same in straight-line code and
-- proves the true postcondition. Both must verify: the refusal is only for a
-- write while the loan is live.
verex module 0x42::review2_liveness where
  fun clear_all(v : &mut Vector<Bool>) -> Unit := do
    let len := v.length
    let mut i : u64 := 0
    while i < len do
      let elem := &mut v[i]
      *elem := false
      i := i + 1
  fun write_after_death() -> u64 := do
    let mut values := vector<u64>[0, 0]
    let mut i : u64 := 0
    let r := &mut values[i]
    *r := 7
    i := 1
    values[i]
  spec write_after_death where
    ensures result == 0
    aborts_if false

/-! ## Loops — a loan live across a loop, and a `let` re-binding of the index

Measured with `Term.describe`: a loan whose last use precedes a loop dies at the
loop statement, and its `resolve` is placed *before* the `loop`, so it runs once
before any write to the index (`entry_dead`, which must verify). A loan used
*after* the loop is live through its body, so a write to the index there is
refused (`live_across`). And in a loop, a `let` binds the same local again on
each iteration while the previous iteration's loan — indexed by it — is still
held across the back edge; that loan's death runs after the re-binding and would
read the new value, so the re-binding is refused (`rebind_in_loop`). -/

verex module 0x42::review2_loops where
  pragma verify = false
  fun live_across(n : u64) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut i : u64 := 0
    let r := &mut values[i]
    while i < n do
      i := i + 1
    *r := 7
    values[1]
  spec live_across where
    requires n == 2
    ensures false
    aborts_if false
  fun rebind_in_loop(n : u64) -> u64 := do
    let mut values := vector<u64>[0, 0]
    let mut r := &mut values[0]
    let mut k : u64 := 0
    while k < n do
      let i : u64 := k
      *r := 7
      r := &mut values[i]
      k := k + 1
    *r := 9
    values[1]
  spec rebind_in_loop where
    requires n == 2
    ensures false
    aborts_if false

/--
error: no denotation for `live_across`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_loops::live_across by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

-- `rebind_in_loop`: `r` is read at the top of the body, so the loan of
-- `values[i]` it takes in one iteration stays live across the back edge, and
-- the next iteration's `let i` binds its index again while it is live.
/--
error: no denotation for `rebind_in_loop`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_loops::rebind_in_loop by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

run_cmd do
  let env ← getEnv
  for name in [`«0x42».review2_loops.live_across.typedVerified,
      `«0x42».review2_loops.live_across.verified,
      `«0x42».review2_loops.rebind_in_loop.typedVerified,
      `«0x42».review2_loops.rebind_in_loop.verified] do
    if env.contains name then throwError "rejected proof published {name}"

/-! ## Loops — a match arm re-binding the index

A match arm binds the locals of its pattern as a `let` does, so in a loop the
arm binds the index again while the previous iteration's loan, indexed by it
and held across the back edge, is still live (`rebind_in_arm`). The control
takes and drops the loan inside the arm, so no loan is live at the binding
(`borrow_in_arm`). -/

verex module 0x42::review2_match_rebind where
  pragma verify = false
  enum Slot has Copy, Drop where
    | At (index : u64)
  fun rebind_in_arm(n : u64) -> u64 := do
    let mut values := vector<u64>[0, 0]
    let mut r := &mut values[0]
    let mut k : u64 := 0
    while k < n do
      match new Slot::At { index := k } with
        | Slot::At { index := i } => do
          *r := 7
          r := &mut values[i]
      k := k + 1
    *r := 9
    values[1]
  spec rebind_in_arm where
    requires n == 2
    ensures false
    aborts_if false

/--
error: no denotation for `rebind_in_arm`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::review2_match_rebind::rebind_in_arm by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

run_cmd do
  let env ← getEnv
  for name in [`«0x42».review2_match_rebind.rebind_in_arm.typedVerified,
      `«0x42».review2_match_rebind.rebind_in_arm.verified] do
    if env.contains name then throwError "rejected proof published {name}"

verex module 0x42::review2_match_control where
  enum Slot has Copy, Drop where
    | At (index : u64)
  fun borrow_in_arm(n : u64) -> Unit := do
    let mut values := vector<u64>[0, 0]
    let mut k : u64 := 0
    while k < n do
      match new Slot::At { index := 0 } with
        | Slot::At { index := i } => do
          let r := &mut values[i]
          *r := 7
      k := k + 1

-- The control: the loan dies before the loop, the index is written in the
-- body, and the true postcondition is provable.
verex module 0x42::review2_loop_control where
  fun entry_dead(n : u64) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut i : u64 := 0
    let r := &mut values[i]
    *r := 7
    while i < n do
      i := i + 1
    values[1]
  spec entry_dead where
    ensures result == 0
    aborts_if false

/-! ## A loan born inside a branch or loop body, held by a local declared outside

The borrow analysis decided a loan's liveness from uses of its holders in the
rest of the current sequence and from `preserved`, a set of loans the enclosing
construct computed on entry. A loan born inside a branch or a loop body is in
no such set, so it died at the next statement not naming its holder, although
the holder still held its reference and the code after the branch, or the next
iteration, resolved it again. Each resolution after the first constrained the
owner a second time, so with no index written at all the state was
contradictory (`lit_after`, `requires c; ensures false` proved), an element
write under the live reference was not a conflict (`if_conflict`), and with an
index written in between the second resolution read the new index
(`if_after`, `inc_after`). The analysis now keeps such a loan live while the
continuation — the rest of every enclosing sequence, the loop body again across
a back edge, and the code after the branch or loop — may read its holder before
overwriting it. -/

verex module 0x42::carried_holder where
  pragma verify = false
  fun lit_after(c : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut k : u64 := 0
    let mut r := &mut values[0]
    if c then
      r := &mut values[1]
      k := 1
    *r := 9
    values[1]
  spec lit_after where
    requires c
    ensures false
    aborts_if false
  fun if_after(c : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut i : u64 := 0
    let mut r := &mut values[i]
    if c then
      r := &mut values[i]
      i := 1
    *r := 9
    values[1]
  spec if_after where
    requires c
    ensures false
    aborts_if false
  fun inc_after(n : u64) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut i : u64 := 0
    let mut k : u64 := 0
    let mut r := &mut values[i]
    while k < n do
      *r := 7
      r := &mut values[i]
      i := i + 1
      k := k + 1
    *r := 9
    values[1]
  spec inc_after where
    requires n == 2
    ensures false
    aborts_if false
  fun inc_after_no_final(n : u64) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut i : u64 := 0
    let mut k : u64 := 0
    let mut r := &mut values[i]
    while k < n do
      *r := 7
      r := &mut values[i]
      i := i + 1
      k := k + 1
    values[1]
  spec inc_after_no_final where
    requires n == 2
    ensures false
    aborts_if false

/--
error: no denotation for `if_after`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::carried_holder::if_after by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

/--
error: no denotation for `inc_after`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::carried_holder::inc_after by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

/--
error: no denotation for `inc_after_no_final`: a write to a local a live mutable borrow uses as an element index is not carried by the denotation
-/
#guard_msgs in
verify 0x42::carried_holder::inc_after_no_final by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

-- `lit_after` writes no index, so it is not refused: the loan born in the
-- branch now dies only after `*r := 9`, the holder is resolved once on each
-- path, and the authored proof of `ensures false` (which closed it before)
-- fails. Its residual goal is a bare `False`.
#guard_msgs (drop error) in
verify 0x42::carried_holder::lit_after by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

run_cmd do
  let env ← getEnv
  for name in [`«0x42».carried_holder.lit_after, `«0x42».carried_holder.if_after,
      `«0x42».carried_holder.inc_after, `«0x42».carried_holder.inc_after_no_final] do
    for suffix in [`typedVerified, `verified] do
      if env.contains (name ++ suffix) then throwError "rejected proof published {name ++ suffix}"

-- An element write under the reference the branch re-pointed `r` to, which
-- the code after the branch writes through: a borrow conflict, so the unit is
-- refused. Before, the loan had died inside the branch and the authored proof
-- of `requires c; ensures false` succeeded.
verex module 0x42::carried_conflict where
  pragma verify = false
  fun if_conflict(c : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut r := &mut values[0]
    if c then
      r := &mut values[1]
      values[1] := 5
    *r := 9
    values[1]
  spec if_conflict where
    requires c
    ensures false
    aborts_if false

#guard_msgs (drop error) in
verify 0x42::carried_conflict::if_conflict by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set])

run_cmd do
  let env ← getEnv
  for name in [`«0x42».carried_conflict.if_conflict.typedVerified,
      `«0x42».carried_conflict.if_conflict.verified] do
    if env.contains name then throwError "rejected proof published {name}"

-- Controls. `lit_after_true` states what the program computes and must verify.
-- `repointed` carries `r` across the back edge but re-points it at another
-- local before the index is incremented, so the indexed loan dies, and is
-- resolved, before the write: it is carried, not refused.
verex module 0x42::carried_holder_control where
  fun lit_after_true(c : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut k : u64 := 0
    let mut r := &mut values[0]
    if c then
      r := &mut values[1]
      k := 1
    *r := 9
    values[1]
  spec lit_after_true where
    ensures c ==> result == 9
    ensures !c ==> result == 0
    aborts_if false
  fun repointed(n : u64) -> u64 := do
    let mut values := vector<u64>[0, 0, 0, 0]
    let mut w : u64 := 0
    let mut i : u64 := 0
    let mut k : u64 := 0
    let mut r := &mut values[i]
    while k < n do
      *r := 7
      r := &mut values[i]
      *r := 8
      r := &mut w
      i := i + 1
      k := k + 1
    *r := 9
    values[1]
  spec repointed where
    ensures true

/-! ## Review 4, C1 — a reference field of a Move-profile struct

Move has no reference fields, and the borrow analysis treats a Move nominal
value as loan-free on that ground. Direct VerexSL could declare one anyway: the
loan stored in `h` went untracked, the lender was read while the loan was
live, and the authored proof established `result == 7` of a run returning
`0`. The declaration is now refused at validation. The control stores a plain
value in the same shape and verifies. -/

/--
error: VIR-MOVE-REFERENCE-FIELD: unsupported construct: field `r` of Move declaration `Holder` holds a reference; Move nominal declarations have no reference fields
-/
#guard_msgs in
verex module 0x42::review4_struct_holder where
  pragma verify = false
  struct Holder where
    r : &mut u64
  fun f(c : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0, 0]
    let mut i : u64 := 0
    let r := &mut values[i]
    let h := new Holder { r }
    let before := values[0]
    *h.r := 7
    before
  spec f where
    pragma unroll = 3
    ensures result == 7
    aborts_if false

#guard_msgs (drop error) in
verify 0x42::review4_struct_holder::f by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set] <;> omega)

run_cmd do
  let env ← getEnv
  for name in [`«0x42».review4_struct_holder.f.typedVerified,
      `«0x42».review4_struct_holder.f.verified] do
    if env.contains name then throwError "rejected proof published {name}"

-- A reference at depth, and one in an enum variant, are refused alike.
/--
error: VIR-MOVE-REFERENCE-FIELD: unsupported construct: field `rs` of Move declaration `Holders` holds a reference; Move nominal declarations have no reference fields
---
error: VIR-MOVE-REFERENCE-FIELD: unsupported construct: field `r` of variant `Some` of Move declaration `MaybeHolder` holds a reference; Move nominal declarations have no reference fields
-/
#guard_msgs in
verex module 0x42::review4_nested_holders where
  pragma verify = false
  struct Holders where
    rs : Vector<&u64>
  enum MaybeHolder where
    | None
    | Some (r : &mut u64)

verex module 0x42::review4_struct_holder_control where
  pragma verify = false
  struct Holder where
    v : u64
  fun f(c : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0, 0]
    let v : u64 := 7
    let h := new Holder { v }
    let before := values[0]
    values[0] := h.v
    before + values[0]
  spec f where
    ensures result == 7
    aborts_if false

verify 0x42::review4_struct_holder_control::f by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set] <;> omega)

run_cmd do
  let env ← getEnv
  unless env.contains `«0x42».review4_struct_holder_control.f.verified do
    throwError "the control did not verify"

/-! ## Review 4, C1 — a reference as a Move type argument

The same loan stored through a generic holder: the field's declared type is a
type parameter, instantiated at `&mut u64`. Move has no reference type
arguments, and the borrow analysis treats a value of a Move type parameter as
loan-free on that ground, so a reference type argument is refused at
validation wherever it occurs: a type, a constructor, a call (explicit or
inferred). The control instantiates the same shapes at `u64` and verifies. -/

/--
error: VIR-MOVE-REFERENCE-TYPE-ARGUMENT: in fun f: unsupported construct: type argument 0 of the type `Holder` holds a reference; Move type arguments cannot be references
---
error: VIR-MOVE-REFERENCE-TYPE-ARGUMENT: in fun f: unsupported construct: type argument 0 of the type `Holder` holds a reference; Move type arguments cannot be references
---
error: VIR-MOVE-REFERENCE-TYPE-ARGUMENT: in fun f: unsupported construct: type argument 0 of the constructor `Holder` holds a reference; Move type arguments cannot be references
---
error: VIR-MOVE-REFERENCE-TYPE-ARGUMENT: in spec fun f: unsupported construct: type argument 0 of the type `Holder` holds a reference; Move type arguments cannot be references
-/
#guard_msgs in
verex module 0x42::review4_generic_holder where
  pragma verify = false
  struct Holder {T} where
    r : T
  fun f(c : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0, 0]
    let mut i : u64 := 0
    let r := &mut values[i]
    let h := new Holder<&mut u64> { r }
    let before := values[0]
    *h.r := 7
    before
  spec f where
    pragma unroll = 3
    ensures result == 7
    aborts_if false

#guard_msgs (drop error) in
verify 0x42::review4_generic_holder::f by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set] <;> omega)

run_cmd do
  let env ← getEnv
  for name in [`«0x42».review4_generic_holder.f.typedVerified,
      `«0x42».review4_generic_holder.f.verified] do
    if env.contains name then throwError "rejected proof published {name}"

/--
error: VIR-MOVE-REFERENCE-TYPE-ARGUMENT: in fun explicit: unsupported construct: type argument 0 of the call of `id` holds a reference; Move type arguments cannot be references
---
error: VIR-MOVE-REFERENCE-TYPE-ARGUMENT: in fun inferred: unsupported construct: type argument 0 of the call of `id` holds a reference; Move type arguments cannot be references
-/
#guard_msgs in
verex module 0x42::review4_generic_call where
  pragma verify = false
  fun id {T}(x : T) -> T := x
  fun explicit() -> u64 := do
    let mut values := vector<u64>[0, 0, 0]
    let r := id::<&mut u64>(&mut values[0])
    *r := 7
    values[0]
  fun inferred() -> u64 := do
    let mut values := vector<u64>[0, 0, 0]
    let r := id(&mut values[0])
    *r := 7
    values[0]

verex module 0x42::review4_generic_control where
  pragma verify = false
  struct Holder {T} where
    r : T
  fun id {T}(x : T) -> T := x
  fun f(c : Bool) -> u64 := do
    let mut values := vector<u64>[0, 0, 0]
    let v : u64 := id::<u64>(7)
    let h := new Holder<u64> { r := v }
    let before := values[0]
    values[0] := h.r
    before + values[0]
  spec f where
    ensures result == 7
    aborts_if false

verify 0x42::review4_generic_control::f by lean =>
  verex_obligations
  all_goals (verex_prepare <;> simp_all [VerexIR.SpecVector.set] <;> omega)

run_cmd do
  let env ← getEnv
  unless env.contains `«0x42».review4_generic_control.f.verified do
    throwError "the control did not verify"
