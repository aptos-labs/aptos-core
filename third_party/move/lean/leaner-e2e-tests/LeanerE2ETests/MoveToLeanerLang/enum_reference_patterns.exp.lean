-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::enum_reference_patterns where
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
    if value is One then value.value else value.value

  enum Slot {T} has Copy, Drop where
    | Empty
    | Filled (value : T)

  fun borrow_or {T}(slot : &Slot<T>, fallback : &T) -> &T :=
    if slot is Empty then fallback else slot.value

  fun value_or {T has Copy, Drop}(slot : &Slot<T>, fallback : T) -> T :=
    if slot is Empty then fallback else slot.value

  fun borrow_mut {T}(slot : &mut Slot<T>) -> &mut T :=
    if slot is Empty then abort(7)
    else
      let value := &mut slot.value
      return value

  fun replace(slot : &mut Slot<u64>, replacement : u64) -> u64 :=
    if slot is Empty then abort(7)
    else
      let value := &mut slot.value
      let previous := *value
      *value := replacement
      return previous
