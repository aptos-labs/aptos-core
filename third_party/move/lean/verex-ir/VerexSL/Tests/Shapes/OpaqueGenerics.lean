-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# An opaque specification function under a generic native's type parameters

A generic native whose contract names an opaque specification function at
its type parameter, called at a struct, at a scalar, at the caller's own
parameter, and at a generic struct over it: the callee's application and the
caller's clause are one application once the type arguments are resolved.
Then the argument is a nominal value the caller constructs, and the clause
names the same construction; and a native whose contract names an opaque
function at a type argument alone, its result of the caller's own parameter
type. The negatives name another value, another nominal declaration of the
same shape, another type argument, and leave an abort out.
-/

namespace VerexSL.Tests.Shapes.OpaqueGenerics

set_option Elab.async false

verex module 0x42::shapes_opaque_generics where
  pragma verify = false

  struct Point has Copy, Drop, Store where
    x : u64

  struct Box {T} has Copy, Drop, Store where
    inner : T

  struct Other {T} has Copy, Drop, Store where
    inner : T

  native fun sum {T}(values : &Vector<T>) -> u64
  spec sum where
    pragma opaque
    aborts_if false
    ensures result == spec_sum(values)

  opaque spec fun spec_sum {T}(values : Vector<T>) : Int

  native fun observe {T}(value : &T) -> u64
  spec observe where
    pragma opaque
    aborts_if false
    ensures result == spec_observe(value)

  opaque spec fun spec_observe {T}(value : T) : Int

  -- 5.1 at a struct, a scalar, the caller's parameter, a struct over it
  fun struct_bad(points : &Vector<Point>) -> u64 := sum(points)
  spec struct_bad where
    aborts_if false
    ensures result == spec_sum(points) + 1

  fun struct_ok(points : &Vector<Point>) -> u64 := sum(points)
  spec struct_ok where
    aborts_if false
    ensures result == spec_sum(points)

  fun scalar_ok(weights : &Vector<u64>) -> u64 := sum(weights)
  spec scalar_ok where
    aborts_if false
    ensures result == spec_sum(weights)

  fun param_ok {T}(values : &Vector<T>) -> u64 := sum(values)
  spec param_ok where
    aborts_if false
    ensures result == spec_sum(values)

  fun instance_ok {T}(boxes : &Vector<Box<T> >) -> u64 := sum(boxes)
  spec instance_ok where
    aborts_if false
    ensures result == spec_sum(boxes)

  -- 5.2 a nominal argument the caller constructs
  fun constructed_ok {T has Copy, Drop, Store}(value : T) -> u64 := do
    let box := new Box<T> { inner := value }
    observe(&box)
  spec constructed_ok where
    aborts_if false
    ensures result == spec_observe(new Box<T> { inner := value })

  fun constructed_bad {T has Copy, Drop, Store}(left : T, right : T) -> u64 := do
    let box := new Box<T> { inner := left }
    observe(&box)
  spec constructed_bad where
    aborts_if false
    ensures result == spec_observe(new Box<T> { inner := right })

  fun nominal_bad {T has Copy, Drop, Store}(value : T) -> u64 := do
    let box := new Box<T> { inner := value }
    observe(&box)
  spec nominal_bad where
    aborts_if false
    ensures result == spec_observe(new Other<T> { inner := value })

  -- 5.3 an opaque specification function at a type argument alone, and a
  -- result of the caller's own parameter type
  native fun name_of {T}() -> u64
  spec name_of where
    pragma opaque
    aborts_if false
    ensures result == spec_name_of::<T>()

  opaque spec fun spec_name_of {T}() : Int

  native fun decode {T}(bytes : Vector<u8>) -> T
  spec decode where
    pragma opaque
    aborts_if !spec_decodable::<T>(bytes)
    ensures result == spec_decode::<T>(bytes)

  opaque spec fun spec_decode {T}(bytes : Vector<u8>) : T

  opaque spec fun spec_decodable {T}(bytes : Vector<u8>) : Bool

  fun named_bad {T}() -> u64 := name_of::<T>()
  spec named_bad where
    aborts_if false
    ensures result == spec_name_of::<u64>()

  fun named_ok {T}() -> u64 := name_of::<T>()
  spec named_ok where
    aborts_if false
    ensures result == spec_name_of::<T>()

  fun decoded_bad {T}(bytes : Vector<u8>) -> T := decode::<T>(bytes)
  spec decoded_bad where
    aborts_if false
    ensures result == spec_decode::<T>(bytes)

  fun decoded_ok {T}(bytes : Vector<u8>) -> T := decode::<T>(bytes)
  spec decoded_ok where
    aborts_if !spec_decodable::<T>(bytes)
    ensures result == spec_decode::<T>(bytes)

  -- The name tested before the decoding, as a body that unpacks a stored
  -- value does.
  fun decoded_named_ok {T}(name : u64, bytes : Vector<u8>) -> T := do
    assert!(name == name_of::<T>(), 1)
    decode::<T>(bytes)
  spec decoded_named_ok where
    aborts_if name != spec_name_of::<T>()
    aborts_if !spec_decodable::<T>(bytes)
    ensures result == spec_decode::<T>(bytes)

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_opaque_generics.struct_bad
    ["ensures result == spec_sum(points) + 1"]
    (← `(command| verify 0x42::shapes_opaque_generics::struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_opaque_generics.constructed_bad
    ["ensures result == spec_observe(new Box<T> { inner := right })"]
    (← `(command| verify 0x42::shapes_opaque_generics::constructed_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_opaque_generics.nominal_bad
    ["ensures result == spec_observe(new Other<T> { inner := value })"]
    (← `(command| verify 0x42::shapes_opaque_generics::nominal_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_opaque_generics.named_bad
    ["ensures result == spec_name_of::<u64>()"]
    (← `(command| verify 0x42::shapes_opaque_generics::named_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_opaque_generics.decoded_bad ["aborts_if false"]
    (← `(command| verify 0x42::shapes_opaque_generics::decoded_bad))

/-! ## Positive controls -/

verify 0x42::shapes_opaque_generics::struct_ok
verify 0x42::shapes_opaque_generics::scalar_ok
verify 0x42::shapes_opaque_generics::param_ok
verify 0x42::shapes_opaque_generics::instance_ok
verify 0x42::shapes_opaque_generics::constructed_ok
verify 0x42::shapes_opaque_generics::named_ok
verify 0x42::shapes_opaque_generics::decoded_ok
verify 0x42::shapes_opaque_generics::decoded_named_ok

end VerexSL.Tests.Shapes.OpaqueGenerics
