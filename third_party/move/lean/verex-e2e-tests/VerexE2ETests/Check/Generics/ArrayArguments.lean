-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Arrays as type arguments

A fixed-length array instantiates a generic struct and enum, and a generic
function's type parameter, as any inhabited type does.
-/

verex namespace array_arguments where
  struct Holder {T} where
    value : T

  enum Maybe {T} where
    | Nothing
    | Just (value : T)

  fun identity {T}(value : T) -> T := value

  fun through_generic(pair : Vector<u8, const 2>) -> u8 := do
    let held := core.call identity::<Vector<u8, const 2> >(pair)
    held[1usize]
  spec through_generic where
    ensures result == pair[1]

  fun first_held(holder : Holder<Vector<u8, const 2> >) -> u8 := holder.value[0usize]
  spec first_held where
    ensures result == holder.value[0]

  fun first_or(maybe : Maybe<Vector<u8, const 2> >, fallback : u8) -> u8 :=
    if maybe is Just then maybe.value[0usize] else fallback
  spec first_or where
    ensures maybe is Nothing ==> result == fallback
    ensures maybe is Just ==> result == maybe.value[0]
