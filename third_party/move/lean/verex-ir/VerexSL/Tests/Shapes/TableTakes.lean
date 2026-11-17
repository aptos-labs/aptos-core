-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# A value taken out of a stored map, unpacked, and handed on

A map held by a resource gives up an entry as a pair of its key and its
value; the value is a struct, unpacked by a pattern, one field compared with
the result of an opaque generic call and another handed to a second one. The
clauses state all of it of the map's read before the take: the equation of
the pair with the read is at its components, the pattern's locals are the
fields of that read, and each opaque call binds its result at the caller's
own type parameter. Then a callee's tuple result equated to an uninterpreted
function's, returned swapped.

A positive states the aborts and the result through the read; the negatives
read the map after the removal, decode another field, and leave the
components unswapped.
-/

namespace VerexSL.Tests.Shapes.TableTakes

set_option Elab.async false

verex module 0x42::shapes_table_takes where
  pragma verify = false

  @[intrinsic_map]
  struct Map {K : phantom type has Copy, Drop} {V : phantom type} has Store where
    dummy_field : Bool

  @[map_del_return_key (Map)]
  native fun take {K has Copy, Drop} {V}(self : &mut Map<K, V>, key : &K) -> (K, V)

  @[map_spec_has_key (Map)]
  opaque spec fun spec_contains {K} {V}(t : Map<K, V>, k : K) : Bool

  @[map_spec_get (Map)]
  opaque spec fun spec_get {K} {V}(t : Map<K, V>, k : K) : V

  @[map_spec_del (Map)]
  opaque spec fun spec_remove {K} {V}(t : Map<K, V>, k : K) : Map<K, V>

  struct Name has Copy, Drop, Store where
    bytes : Vector<u8>

  struct Packed has Drop, Store where
    name : Name
    data : Vector<u8>

  struct Pending has Key where
    configs : Map<Name, Packed>

  native fun name_of {T}() -> Name
  spec name_of where
    pragma opaque
    aborts_if false
    ensures result == spec_name_of::<T>()

  opaque spec fun spec_name_of {T}() : Name

  native fun decode {T}(bytes : Vector<u8>) -> T
  spec decode where
    pragma opaque
    aborts_if !spec_decodable::<T>(bytes)
    ensures result == spec_decode::<T>(bytes)

  opaque spec fun spec_decode {T}(bytes : Vector<u8>) : T

  opaque spec fun spec_decodable {T}(bytes : Vector<u8>) : Bool

  opaque spec fun split_of(x : u64) : (u64, u64)

  native fun split(x : u64) -> (u64, u64)
  spec split where
    pragma opaque
    aborts_if false
    ensures (result, spec.result[1]) == split_of(x)

  -- 7.20 take, unpack, compare a field with an opaque call's result, hand
  -- another field to a second opaque call
  fun take_decode_bad {T}() -> T := do
    let configs := &mut Pending[@0x1]
    let key := name_of::<T>()
    let (_, packed) := take(&mut configs.configs, &key)
    let Packed { name := name, data := data } := packed
    assert!(name == name_of::<T>(), 1)
    decode::<T>(data)
  spec take_decode_bad where
    pragma aborts_if_is_partial
    let_pre key := spec_name_of::<T>()
    let_pre stored := spec_get(global<Pending>(@0x1).configs, key)
    ensures result == spec_decode::<T>(stored.name.bytes)
    modifies global<Pending>(@0x1)

  fun take_decode_ok {T}() -> T := do
    let configs := &mut Pending[@0x1]
    let key := name_of::<T>()
    let (_, packed) := take(&mut configs.configs, &key)
    let Packed { name := name, data := data } := packed
    assert!(name == name_of::<T>(), 1)
    decode::<T>(data)
  spec take_decode_ok where
    let_pre key := spec_name_of::<T>()
    let_pre stored := spec_get(global<Pending>(@0x1).configs, key)
    aborts_if !exists<Pending>(@0x1)
    aborts_if !spec_contains(global<Pending>(@0x1).configs, key)
    aborts_if stored.name != key
    aborts_if !spec_decodable::<T>(stored.data)
    ensures result == spec_decode::<T>(stored.data)
    ensures !spec_contains(global<Pending>(@0x1).configs, key)
    modifies global<Pending>(@0x1)

  -- 7.21 take, unpack, return a field of the value
  fun take_field_bad {T}() -> Vector<u8> := do
    let configs := &mut Pending[@0x1]
    let key := name_of::<T>()
    let (_, packed) := take(&mut configs.configs, &key)
    let Packed { name := _, data := data } := packed
    data
  spec take_field_bad where
    pragma aborts_if_is_partial
    let_post key := spec_name_of::<T>()
    ensures result == spec_get(global<Pending>(@0x1).configs, key).data
    modifies global<Pending>(@0x1)

  fun take_field_ok {T}() -> Vector<u8> := do
    let configs := &mut Pending[@0x1]
    let key := name_of::<T>()
    let (_, packed) := take(&mut configs.configs, &key)
    let Packed { name := name, data := data } := packed
    assert!(name == name_of::<T>(), 1)
    data
  spec take_field_ok where
    let_pre key := spec_name_of::<T>()
    let_pre stored := spec_get(global<Pending>(@0x1).configs, key)
    aborts_if !exists<Pending>(@0x1)
    aborts_if !spec_contains(global<Pending>(@0x1).configs, key)
    aborts_if stored.name != key
    ensures result == stored.data
    ensures !spec_contains(global<Pending>(@0x1).configs, key)
    modifies global<Pending>(@0x1)

  -- 4.g' a callee's tuple result equated to an uninterpreted function's
  fun swapped_bad(x : u64) -> (u64, u64) := do
    let (a, b) := split(x)
    (b, a)
  spec swapped_bad where
    aborts_if false
    ensures (result, spec.result[1]) == split_of(x)

  fun swapped_ok(x : u64) -> (u64, u64) := do
    let (a, b) := split(x)
    (b, a)
  spec swapped_ok where
    aborts_if false
    ensures (spec.result[1], result) == split_of(x)

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_table_takes.take_decode_bad
    ["ensures result == spec_decode::<T>(stored.name.bytes)"]
    (← `(command| verify 0x42::shapes_table_takes::take_decode_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_table_takes.take_field_bad
    ["ensures result == spec_get(global<Pending>(@0x1).configs, key).data"]
    (← `(command| verify 0x42::shapes_table_takes::take_field_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_table_takes.swapped_bad
    ["ensures (result, spec.result[1]) == split_of(x)"]
    (← `(command| verify 0x42::shapes_table_takes::swapped_bad))

/-! ## Positive controls -/

verify 0x42::shapes_table_takes::take_decode_ok
verify 0x42::shapes_table_takes::take_field_ok
verify 0x42::shapes_table_takes::swapped_ok

end VerexSL.Tests.Shapes.TableTakes
