-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex module 0x42::enum_reference_patterns where
  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- The same field name has distinct generic types in the two variants.
  -- An unqualified projection cannot replace this typed pattern.
  enum Either {T} {E} has Copy, Drop where
    | Left (value : T)
    | Right (value : E)

  fun unwrap_left {T} {E}(value : Either<T, E>) -> T :=
    match value with
      | Either<T, E>::Left { value := value } => value
      | _ => abort(7)

  fun unwrap_left_u64(value : Either<u64, u64>) -> u64 :=
    match value with
      | Either<u64, u64>::Left { value := value } => value
      | _ => abort(7)

  enum Counter has Copy, Drop where
    | One (value : u64)
    | Two (value : u64)

  fun counter(value : Counter) -> u64 :=
    match value with
      | Counter::One { value := _ } => value.value
      | Counter::Two { value := _ } => value.value

  enum Slot {T} has Copy, Drop where
    | Empty
    | Filled (value : T)

  fun borrow_or {T}(slot : &Slot<T>, fallback : &T) -> &T :=
    match slot with
      | Slot<T>::Empty {} => fallback
      | Slot<T>::Filled { value := _ } => do
        let value := &slot.value
        value

  fun value_or {T has Copy, Drop}(slot : &Slot<T>, fallback : T) -> T :=
    match slot with
      | Slot<T>::Empty {} => fallback
      | Slot<T>::Filled { value := _ } => do
        let value := &slot.value
        *value

  fun borrow_mut {T}(slot : &mut Slot<T>) -> &mut T :=
    match slot with
      | Slot<T>::Empty {} => abort(7)
      | Slot<T>::Filled { value := _ } => do
        let value := &mut slot.value
        value

  fun replace(slot : &mut Slot<u64>, replacement : u64) -> u64 :=
    match slot with
      | Slot<u64>::Empty {} => abort(7)
      | Slot<u64>::Filled { value := _ } => do
        let value := &mut slot.value
        let previous := *value
        *value := replacement
        previous

  -- A match through a reference to a stored value, its last arm a
  -- wildcard: the arms print from the variant tests.
  enum «Store» has Key where
    | Full (value : u64)
    | Empty

  fun full_value(addr : Address) -> u64 := do
    let _ref0 := &«Store»[addr]
    match _ref0 with
      | «Store»::Full { value := _ } => do
        let value := &_ref0.value
        *value
      | _ => abort(9)
