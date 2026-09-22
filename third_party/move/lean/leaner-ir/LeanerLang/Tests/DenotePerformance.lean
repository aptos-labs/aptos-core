-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Verification-cost benchmark of the denotation

The D0 targets of `designs/denotation.md`, one per operation class, the
D1 loop forms, and the D1 aggregates (tuples, structs, enums), each proved
by a bare `verify` through the denotation.  `#leaner_perf` compares
their cost with [`DenotePerformance.exp`](DenotePerformance.exp) and fails
on a regression.  The retiring route's benchmark (`Performance.lean`) is
kept until its targets are carried; this file gates what the denotation
carries today.
-/

namespace LeanerLang.Tests.DenotePerformance

set_option Elab.async false

#leaner_measure

leaner module 0x42::denote_perf where
  const COMPLEX : u64 := 1 + 2 * 3

  fun add_values(left : u64, right : u64) -> u64 := left + right
  spec add_values where
    ensures result == left + right
    aborts_if left + right > 18446744073709551615
  verify add_values

  fun subtract_values(left : u64, right : u64) -> u64 := left - right
  spec subtract_values where
    ensures result == left - right
    aborts_if left < right
  verify subtract_values

  fun divide_values(left : u64, right : u64) -> u64 := left / right
  spec divide_values where
    ensures result == left / right
    aborts_if right == 0
  verify divide_values

  fun at_most(left : u64, right : u64) -> u64 :=
    if left <= right then 1 else 0
  spec at_most where
    ensures result == if left <= right then 1 else 0
    aborts_if false
  verify at_most

  fun differs(left : u64, right : u64) -> u64 :=
    if left != right then 1 else 0
  spec differs where
    ensures left == right ==> result == 0
    ensures !(left == right) ==> result == 1
    aborts_if false
  verify differs

  fun is_less(left : u64, right : u64) -> Bool := left < right
  spec is_less where
    ensures result == (left < right)
  verify is_less

  fun narrow(value : u64) -> u8 := value as u8
  spec narrow where
    ensures result == value
    aborts_if value > 255
  verify narrow

  fun masked(value : u64, mask : u64) -> u64 := value & mask
  spec masked where
    ensures result == value & mask
    aborts_if false
  verify masked

  fun shifted(value : u64, amount : u8) -> u64 := value << amount
  spec shifted where
    ensures result == (value << amount) % 18446744073709551616
    aborts_if amount >= 64
  verify shifted

  fun halved(value : u16) -> u16 := value >> 1u8
  spec halved where
    ensures result == value >> 1u8
    aborts_if false
  verify halved

  fun complex_constant() -> u64 := COMPLEX
  spec complex_constant where
    ensures result == 7
    aborts_if false
  verify complex_constant

  fun classify_primitive(value : u64) -> u64 :=
    if value == 0 then 10
    else if 1 <= value && value < 4 && value != 2 then 20
    else if 4 <= value && value <= 6 then 30
    else 40
  spec classify_primitive where
    ensures result == if value == 0 then 10
      else if 1 <= value && value < 4 && value != 2 then 20
      else if 4 <= value && value <= 6 then 30
      else 40
    aborts_if false
  verify classify_primitive

  fun primitive_match_reference(value : u64) -> u64 := do
    let reference := &value
    if *reference == 0 then 1
    else if 1 <= *reference && *reference <= 9 then 2
    else 3
  spec primitive_match_reference where
    ensures result == if value == 0 then 1
      else if 1 <= value && value <= 9 then 2
      else 3
    aborts_if false
  verify primitive_match_reference

leaner module 0x44::denote_perf_loops where
  public fun count_to(limit : u64) -> u64 := do
    let mut current : u64 := 0
    loop if current < limit then do
      current := current + 1
    else break
    spec do
      invariant current <= limit
    return current
  spec count_to where
    ensures result == limit
    aborts_if false
  verify count_to

  fun return_in_loop(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      if remaining == 3 then return 1
      remaining := remaining - 1
    return remaining
  spec return_in_loop where
    ensures result <= 1
    aborts_if false
  verify return_in_loop

  fun count(limit : u64) -> u64 := do
    let mut i := 0
    while i < limit do
      i := i + 1
    spec do
      invariant i <= limit
    i
  spec count where
    ensures result == limit
    aborts_if false
  verify count

  fun keeps(n : u64, limit : u64) -> u64 := do
    let mut i := 0
    while i < limit do
      i := i + 1
    spec do
      invariant i <= limit
    n
  spec keeps where
    ensures result == n
    aborts_if false
  verify keeps

leaner module 0x45::denote_perf_aggregates where
  struct Pair has Copy, Drop, Store where
    first : u64
    second : Bool
  enum Action has Copy, Drop, Store where
    | Idle
    | Transfer (amount : u64)
    | Split (left : u64, right : u64)

  fun pure_pair(value : u64) -> (u64, Bool) := (value, true)
  spec pure_pair where
    ensures spec.result[0] == value && spec.result[1] == true
  verify pure_pair

  fun destructure_local(value : u64) -> u64 := do
    let pair := (value, false)
    let (first, _) := pair
    return first
  spec destructure_local where
    ensures result == value
  verify destructure_local

  fun read_pair(pair : Pair) -> u64 := pair.first
  spec read_pair where
    ensures result == pair.first
  verify read_pair

  fun make_pair(value : u64) -> Pair := new Pair { first := value, second := true }
  spec make_pair where
    ensures result.first == value
    ensures result.second == true
  verify make_pair

  fun make_transfer(amount : u64) -> Action := new Action::Transfer { amount }
  spec make_transfer where
    ensures result == new Action::Transfer { amount }
  verify make_transfer

  fun is_transfer(action : Action) -> Bool := action is Transfer
  spec is_transfer where
    ensures result == (action is Transfer)
  verify is_transfer

  fun total(action : Action) -> u64 :=
    match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := amount } => amount
      | Action::Split { left := left, right := right } => left + right
  spec total where
    ensures result == match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := amount } => amount
      | Action::Split { left := left, right := right } => left + right
    aborts_if match action with
      | Action::Split { left := left, right := right } => left + right > 18446744073709551615
      | _ => false
  verify total

#leaner_perf "LeanerLang/Tests/DenotePerformance.exp"

end LeanerLang.Tests.DenotePerformance
