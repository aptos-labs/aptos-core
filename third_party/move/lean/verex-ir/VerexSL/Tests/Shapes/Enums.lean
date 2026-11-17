-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Enums: matching, variant construction, shared fields, optional results

An enum whose data invariant has a clause per variant, matched and built by
branches; an enum whose variants share a field, read and written through a
mutable reference; and an optional result of an intrinsic map, in the two
representations a declaration may give it: an enum with a payload variant
and a struct over a vector of at most one element. A positive states what
the variant's clause or the map's operation gives; the negative asks for
more.

A field read of an enum held by a resource is `Storage.read_enum_ok`.
-/

namespace VerexSL.Tests.Shapes.Enums

set_option Elab.async false

verex module 0x42::shapes_enums where
  pragma verify = false

  enum Payment has Copy, Drop, Store where
    | None
    | Direct (amount : u64)
    | Split (left : u64, right : u64)
  spec Payment where
    invariant match this with
      | Payment::None {} => true
      | Payment::Direct { amount := amount } => amount > 0
      | Payment::Split { left := left, right := right } => left > 0 && right > 0

  enum Slot has Copy, Drop where
    | Empty (val : u64)
    | Full (val : u64)

  -- c.1 a match that reads each variant's clause
  fun match_bad(payment : Payment) -> u64 :=
    match payment with
      | Payment::None {} => 0
      | Payment::Direct { amount := amount } => amount
      | Payment::Split { left := left, right := right } => left
  spec match_bad where
    aborts_if false
    ensures result > 0

  fun match_ok(payment : Payment) -> u64 :=
    match payment with
      | Payment::None {} => 0
      | Payment::Direct { amount := amount } => amount
      | Payment::Split { left := left, right := right } => left
  spec match_ok where
    aborts_if false
    ensures match payment with
      | Payment::None {} => result == 0
      | Payment::Direct { amount := _ } => result > 0
      | Payment::Split { left := _, right := _ } => result > 0

  -- c.2 a variant chosen by a branch, each owing its own clause
  fun branch_bad(amount : u64) -> Payment :=
    if 0 < amount then new Payment::Direct { amount } else new Payment::Direct { amount := 0 }
  spec branch_bad where
    aborts_if false

  fun branch_ok(amount : u64) -> Payment :=
    if 0 < amount then new Payment::Direct { amount } else new Payment::None {}
  spec branch_ok where
    aborts_if false
    ensures 0 < amount ==> result is Direct

  -- c.3 a field the variants share, through a mutable reference
  fun shared_read_bad(self : &mut Slot) -> u64 := self.val
  spec shared_read_bad where
    aborts_if false
    ensures result == self.val + 1

  fun shared_read_ok(self : &mut Slot) -> u64 := self.val
  spec shared_read_ok where
    aborts_if false
    ensures result == self.val

  fun shared_write_bad(self : &mut Slot, v : u64) -> Unit := self.val := v
  spec shared_write_bad where
    aborts_if false
    ensures self.val == v + 1

  fun shared_write_ok(self : &mut Slot, v : u64) -> Unit := self.val := v
  spec shared_write_ok where
    aborts_if false
    ensures self.val == v

verex module 0x42::shapes_optional_enum where
  pragma verify = false

  enum Maybe {V} has Copy, Drop, Store where
    | Absent
    | Present (value : V)

  @[intrinsic_map]
  struct Map {K} {V} has Copy, Drop, Store where
    placeholder : Bool

  @[map_new (Map)]
  native fun new {K} {V}() -> Map<K, V>
  @[map_get (Map)]
  native fun get {K} {V}(m : &Map<K, V>, k : &K) -> Maybe<V>
  @[map_upsert (Map)]
  native fun upsert {K} {V}(m : &mut Map<K, V>, k : K, v : V) -> Maybe<V>

  -- c.4 an optional map result as an enum: absent, present, replaced
  fun enum_missing_ok() -> Maybe<u64> := do
    let m := new::<u64, u64>()
    get(&m, &7u64)
  spec enum_missing_ok where
    aborts_if false
    ensures result is Absent

  fun enum_present_ok() -> Maybe<u64> := do
    let mut m := new::<u64, u64>()
    let _ := upsert(&mut m, 7u64, 11u64)
    get(&m, &7u64)
  spec enum_present_ok where
    aborts_if false
    ensures result is Present && result.value == 11

  fun enum_replaced_bad() -> Maybe<u64> := do
    let mut m := new::<u64, u64>()
    let _ := upsert(&mut m, 7u64, 11u64)
    upsert(&mut m, 7u64, 13u64)
  spec enum_replaced_bad where
    aborts_if false
    ensures result is Present && result.value == 13

  fun enum_replaced_ok() -> Maybe<u64> := do
    let mut m := new::<u64, u64>()
    let _ := upsert(&mut m, 7u64, 11u64)
    upsert(&mut m, 7u64, 13u64)
  spec enum_replaced_ok where
    aborts_if false
    ensures result is Present && result.value == 11

verex module 0x42::shapes_optional_vector where
  pragma verify = false

  struct Maybe {V} has Copy, Drop, Store where
    values : Vector<V>

  @[intrinsic_map]
  struct Map {K} {V} has Copy, Drop, Store where
    placeholder : Bool

  @[map_new (Map)]
  native fun new {K} {V}() -> Map<K, V>
  @[map_get (Map)]
  native fun get {K} {V}(m : &Map<K, V>, k : &K) -> Maybe<V>
  @[map_upsert (Map)]
  native fun upsert {K} {V}(m : &mut Map<K, V>, k : K, v : V) -> Maybe<V>

  -- c.5 the same result as a struct over a vector
  fun vector_missing_ok() -> Maybe<u64> := do
    let m := new::<u64, u64>()
    get(&m, &7u64)
  spec vector_missing_ok where
    aborts_if false
    ensures len(result.values) == 0

  fun vector_replaced_bad() -> Maybe<u64> := do
    let mut m := new::<u64, u64>()
    let _ := upsert(&mut m, 7u64, 11u64)
    upsert(&mut m, 7u64, 13u64)
  spec vector_replaced_bad where
    aborts_if false
    ensures len(result.values) == 1 && result.values[0] == 13

  fun vector_replaced_ok() -> Maybe<u64> := do
    let mut m := new::<u64, u64>()
    let _ := upsert(&mut m, 7u64, 11u64)
    upsert(&mut m, 7u64, 13u64)
  spec vector_replaced_ok where
    aborts_if false
    ensures len(result.values) == 1 && result.values[0] == 11

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_enums.match_bad ["ensures result > 0"]
    (← `(command| verify 0x42::shapes_enums::match_bad))

open LoanDeathControls in
run_cmd do
  let payment := "invariant match this with\n" ++
    "      | Payment::None {} => true\n" ++
    "      | Payment::Direct { amount := amount } => amount > 0\n" ++
    "      | Payment::Split { left := left, right := right } => left > 0 && right > 0"
  expectCuts `«0x42».shapes_enums.branch_bad [(payment, construction)]
    (← `(command| verify 0x42::shapes_enums::branch_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_enums.shared_read_bad ["ensures result == self.val + 1"]
    (← `(command| verify 0x42::shapes_enums::shared_read_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_enums.shared_write_bad ["ensures self.val == v + 1"]
    (← `(command| verify 0x42::shapes_enums::shared_write_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_optional_enum.enum_replaced_bad
    ["ensures result is Present && result.value == 13"]
    (← `(command| verify 0x42::shapes_optional_enum::enum_replaced_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_optional_vector.vector_replaced_bad
    ["ensures len(result.values) == 1 && result.values[0] == 13"]
    (← `(command| verify 0x42::shapes_optional_vector::vector_replaced_bad))

/-! ## Positive controls -/

verify 0x42::shapes_enums::match_ok
verify 0x42::shapes_enums::branch_ok
verify 0x42::shapes_enums::shared_read_ok
verify 0x42::shapes_enums::shared_write_ok
verify 0x42::shapes_optional_enum::enum_missing_ok
verify 0x42::shapes_optional_enum::enum_present_ok
verify 0x42::shapes_optional_enum::enum_replaced_ok
verify 0x42::shapes_optional_vector::vector_missing_ok
verify 0x42::shapes_optional_vector::vector_replaced_ok

end VerexSL.Tests.Shapes.Enums
