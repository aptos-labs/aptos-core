-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Quantifiers over a range, over a vector, and over a type. A range binds
the integers from its lower bound below its upper bound; a vector, as in the
Move Prover, binds the element at each index below its length; an aggregate
type binds its values, whose elements are bounded by their type. -/

leaner module 0x49::quantifiers where
  fun below(n : u64) -> u64 := n

  spec below where
    ensures ∀ (i in 0 .. n), i < result
    ensures ∀ (i in n .. n), false
    ensures ∃ (i in 0 .. n), i >= result -- error: no index of the range reaches its bound

  fun length_of(v : &Vector<u64>) -> u64 := v.length

  spec length_of where
    ensures ∀ (y in v), y <= MAX_U64 || result > 0
    ensures ∃ (y in v), true -- error: an empty vector has no element

  -- A frame over a vector write: every other index keeps its element, and
  -- the written index holds the written value.
  fun write_at(v : &mut Vector<u64>, i : u64) -> Unit := do
    assert!(v.length > i, 1)
    v[i] := 7

  spec write_at where
    aborts_if i >= v.length with 1
    ensures v[i] == 7
    ensures ∀ (k in 0 .. v.length), k != i ==> v[k] == old(v)[k]

  fun any_vector() -> u64 := 1
  spec any_vector where
    ensures ∀ (w : Vector<u64>), w.length == 0 || w[0] <= MAX_U64

  fun some_vector(v : Vector<u64>) -> u64 := 1
  spec some_vector where
    ensures ∃ (w : Vector<u64>), w == v
