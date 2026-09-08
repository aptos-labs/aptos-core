-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x1::cmp where
  enum Ordering has Copy, Drop where
    | Less
    | Equal
    | Greater

  spec Ordering where
    pragma intrinsic

  /--
  Compares two values with the natural ordering:
  - native types are compared identically to `<` and other operators
  - complex types
    - Structs and vectors - are compared lexicographically - first field/element is compared first,
      and if equal we proceed to the next.
    - enum's are compared first by their variant, and if equal - they are compared as structs are.
  -/
  public native fun compare {T}(first : &T, second : &T) -> Ordering

  spec compare where
    pragma intrinsic

  public fun is_eq(self : &Ordering) -> Bool := self is Equal

  spec is_eq where
    pragma opaque
    pragma intrinsic
    pragma verify = false

  public fun is_ne(self : &Ordering) -> Bool := !(self is Equal)

  spec is_ne where
    pragma opaque
    pragma intrinsic
    pragma verify = false

  public fun is_lt(self : &Ordering) -> Bool := self is Less

  spec is_lt where
    pragma opaque
    pragma intrinsic
    pragma verify = false

  public fun is_le(self : &Ordering) -> Bool := !(self is Greater)

  spec is_le where
    pragma opaque
    pragma intrinsic
    pragma verify = false

  public fun is_gt(self : &Ordering) -> Bool := self is Greater

  spec is_gt where
    pragma opaque
    pragma intrinsic
    pragma verify = false

  public fun is_ge(self : &Ordering) -> Bool := !(self is Less)

  spec is_ge where
    pragma opaque
    pragma intrinsic
    pragma verify = false

  -- here for typing, for the second line
  fun test_verify_compare_preliminary_types() -> Unit := spec do
    assert compare(1, 5).is_ne()
    assert !compare(1, 5).is_eq()
    assert compare(1, 5).is_lt()
    assert compare(1, 5).is_le()
    assert compare(5, 5).is_eq()
    assert !compare(5, 5).is_ne()
    assert !compare(5, 5).is_lt()
    assert compare(5, 5).is_le()
    assert !compare(7, 5).is_eq()
    assert compare(7, 5).is_ne()
    assert !compare(7, 5).is_lt()
    assert !compare(7, 5).is_le()
    assert compare(false, true).is_ne()
    assert compare(false, true).is_lt()
    assert compare(true, false).is_ge()
    assert compare(true, true).is_eq()

  fun test_verify_compare_vectors() -> Unit := do
    let empty := vector<u64>[]
    let v1 := vector<u64>[1 as u64]
    let v8 := vector<u8>[1 as u8, 2u8]
    let v32_1 := vector<u32>[1 as u32, 2u32, 3u32]
    let v32_2 := vector<u32>[5 as u32]
    spec do
      assert compare(empty, v1) == new Ordering::Less {}
      assert compare(empty, empty) == new Ordering::Equal {}
      assert compare(v1, empty) == new Ordering::Greater {}
      assert compare(v8, v8) == new Ordering::Equal {}
      assert compare(v32_1, v32_2) is Less
      assert compare(v32_2, v32_1) == new Ordering::Greater {}

  struct SomeStruct has Drop where
    field_1 : u64
    field_2 : u64

  fun test_verify_compare_structs() -> Unit := do
    let s1 := new SomeStruct { field_1 := 1, field_2 := 2 }
    let s2 := new SomeStruct { field_1 := 1, field_2 := 3 }
    let s3 := new SomeStruct { field_1 := 1, field_2 := 1 }
    let s4 := new SomeStruct { field_1 := 2, field_2 := 1 }
    spec do
      assert compare(s1, s1) == new Ordering::Equal {}
      assert compare(s1, s2) == new Ordering::Less {}
      assert compare(s1, s3) == new Ordering::Greater {}
      assert compare(s4, s1) == new Ordering::Greater {}

  fun test_verify_compare_vector_of_structs() -> Unit := do
    let v1 := vector<SomeStruct>[new SomeStruct { field_1 := 1, field_2 := 2 }]
    let v2 := vector<SomeStruct>[new SomeStruct { field_1 := 1, field_2 := 3 }]
    spec do
      assert compare(v1, v2) == new Ordering::Less {}
      assert compare(v1, v1) == new Ordering::Equal {}

  enum SomeEnum has Drop where
    | V1 (field_1 : u64)
    | V2 (field_2 : u64)
    | V3 (field_3 : SomeStruct)
    | V4 (field_4 : Vector<u64>)
    | V5 (field_5 : SimpleEnum)

  enum SimpleEnum has Drop where
    | V (field : u64)

  fun test_verify_compare_enums() -> Unit := do
    let e1 := new SomeEnum::V1 { field_1 := 6 }
    let e2 := new SomeEnum::V2 { field_2 := 1 }
    let e3 :=
      new SomeEnum::V3 {
        field_3 := new SomeStruct { field_1 := 1, field_2 := 2 }
      }
    let e4 := new SomeEnum::V4 { field_4 := vector<u64>[1, 2] }
    let e5 := new SomeEnum::V5 { field_5 := new SimpleEnum::V { field := 3 } }
    spec do
      assert compare(e1, e1) == new Ordering::Equal {}
      assert compare(e1, e2) == new Ordering::Less {}
      assert compare(e2, e1) == new Ordering::Greater {}
      assert compare(e3, e4) == new Ordering::Less {}
      assert compare(e5, e4) == new Ordering::Greater {}

  struct SomeStruct_BV has Copy, Drop where
    field : u64

  spec SomeStruct_BV where
    pragma bv = b"0"

  fun test_compare_bv() -> Unit := do
    let a := 1
    let b := 5
    let se_a := new SomeStruct_BV { field := a }
    let se_b := new SomeStruct_BV { field := b }
    let v_a := vector<u64>[a]
    let v_b := vector<u64>[b]
    spec do
      assert compare(a, b) == new Ordering::Less {}
      assert compare(se_a, se_b) == new Ordering::Less {}
      assert compare(v_a, v_b) == new Ordering::Less {}

  -- Different masks: capture pos 0 vs capture pos 1.
  -- Must not be equal because different masks mean different behavior.
  -- Identical closures (same function, same mask, same captured value) must be equal.
