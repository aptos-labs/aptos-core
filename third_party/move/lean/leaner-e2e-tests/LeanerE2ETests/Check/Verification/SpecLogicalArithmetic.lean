-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Port of v0's mathematical-integer specification arithmetic. The scan
body checks specification elaboration only, as in v0; it is not verified. -/

namespace LeanerLang.Tests.VerificationSpecLogicalArithmetic

leaner module 0x42::spec_logical_arithmetic where
  struct Bits has Copy, Drop, Store where
    length : u64
    values : Vector<Bool>
  spec Bits where
    invariant length == values.length

  spec fun is_set(bits : Bits, index : Int) : Bool :=
    0 <= index && index < bits.values.length && bits.values[index]
  spec fun logical_length(bits : Bits) : Int := bits.length
  spec fun logical_successor(value : Int) : Int := value + 1

  fun checked_successor(value : u64) -> u64 := value + 1
  spec checked_successor where
    requires value < 18446744073709551615
    ensures result == logical_successor(value)
    aborts_if false

  -- Source-level counterparts of v0's two logical-normalization lemmas.
  -- The boundary cases distinguish mathematical arithmetic from wrapping
  -- or aborting machine arithmetic in a specification.
  fun subtract_less(left : u64, right : u64) -> Bool :=
    if left == 0 then true else left - 1 < right
  spec subtract_less where
    ensures result == (left - 1 < right)
    aborts_if false
  verify subtract_less

  fun successor_equal(left : u64, right : u64) -> Bool :=
    if left < 18446744073709551615 then left + 1 == right else false
  spec successor_equal where
    ensures result == (left + 1 == right)
    aborts_if false
  verify successor_equal

  fun below(left : u64, right : u64) -> Bool := left < right
  fun scan(bits : &Bits, start : u64) -> u64 := do
    let length_ref := &bits.length
    let length := *length_ref
    let mut index := start
    loop do
      if !(index < length) then break
      index := index + 1
    spec do
      invariant index == start || is_set(bits, index - 1)
      invariant index == start || index - 1 < bits.values.length
      invariant index == start || below(index - 1, length)
      invariant forall (j : Int), start <= j && j < index ==> is_set(bits, j)
    return index
  spec scan where
    ensures start <= result && result <= logical_length(bits) &&
      (result == start || below(result - 1, logical_length(bits)))
    aborts_if false

-- Exercise the source example through the native arithmetic route; the
-- branch/comparison examples above still use the retiring route.
set_option leaner.route "native" in
#leaner_verify 0x42::spec_logical_arithmetic::checked_successor

end LeanerLang.Tests.VerificationSpecLogicalArithmetic
