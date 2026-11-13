-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! Generic opaque calls preserve the declared nominal argument type when
one side constructs the value and the other reads a native call result.
Changing the payload or the nominal declaration must still be rejected. -/

verex module 0x48::nominal_opaque where
  struct Box {T} has Copy, Drop, Store where
    inner : T

  struct Other {T} has Copy, Drop, Store where
    inner : T

  native fun observe {T}(value : &T) -> u64

  spec observe where
    pragma opaque
    aborts_if false
    ensures result == spec_observe(value)

  opaque spec fun spec_observe {T}(value : T) : Int

  fun constructed {T has Copy, Drop, Store}(value : T) -> u64 := do
    let box := new Box<T> { inner := value }
    observe(&box)

  spec constructed where
    aborts_if false
    ensures result == spec_observe(new Box<T> { inner := value })

  fun changed {T has Copy, Drop, Store}(left : T, right : T) -> u64 := do
    let box := new Box<T> { inner := left }
    observe(&box)

  spec changed where
    pragma verify = false
    aborts_if false
    ensures result == spec_observe(new Box<T> { inner := right })

  fun other {T has Copy, Drop, Store}(value : T) -> u64 := do
    let box := new Box<T> { inner := value }
    observe(&box)

  spec other where
    pragma verify = false
    aborts_if false
    ensures result == spec_observe(new Other<T> { inner := value })

/--
error: the specification clause `ensures result == spec_observe(new Box<T> { inner := right })` is not established
---
error: verex verification failed: the automatic verification of `changed` failed; provide a proof in the module:
  verify changed by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x48::nominal_opaque::changed

/--
error: the specification clause `ensures result == spec_observe(new Other<T> { inner := value })` is not established
---
error: verex verification failed: the automatic verification of `other` failed; provide a proof in the module:
  verify other by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x48::nominal_opaque::other
