-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A drop that could run a destructor has no denotation, at its location: of a
nominal stating one, of one holding it, and of a call dropping a type parameter
instantiated with it, however the call reaches the drop. A nominal stating none
drops, and a call that does not drop its type argument passes it. -/

verex namespace destructors where
  @[destructor]
  struct Guard where
    value : u32

  struct Field where
    guard : Guard

  struct Holder {T} where
    value : T

  struct Plain where
    value : u32

  fun consume(guard : Guard) -> Unit := do
    drop(guard)
  spec consume where
    ensures true

  fun consume_field(field : Field) -> Unit := do
    drop(field)
  spec consume_field where
    ensures true

  fun consume_holder(holder : Holder<Guard>) -> Unit := do
    drop(holder)
  spec consume_holder where
    ensures true

  fun forget {T}(value : T) -> Unit := do
    drop(value)
  spec forget where
    ensures true

  fun forget_holder {T}(holder : Holder<T>) -> Unit := do
    drop(holder)
  spec forget_holder where
    ensures true

  fun forget_through {T}(value : T) -> Unit := do
    forget::<T>(value)
  spec forget_through where
    ensures true

  fun identity {T}(value : T) -> T := value
  spec identity where
    ensures result == value

  fun forget_guard(guard : Guard) -> Unit := do
    forget::<Guard>(guard)
  spec forget_guard where
    ensures true

  fun forget_holder_guard(holder : Holder<Guard>) -> Unit := do
    forget_holder::<Guard>(holder)
  spec forget_holder_guard where
    ensures true

  fun forget_guard_through(guard : Guard) -> Unit := do
    forget_through::<Guard>(guard)
  spec forget_guard_through where
    ensures true

  fun forget_plain(plain : Plain) -> Unit := do
    forget::<Plain>(plain)
  spec forget_plain where
    ensures true

  fun drop_plain(plain : Plain) -> Unit := do
    drop(plain)
  spec drop_plain where
    ensures true

  fun keep_guard(guard : Guard) -> Guard := identity::<Guard>(guard)
  spec keep_guard where
    ensures result == guard
