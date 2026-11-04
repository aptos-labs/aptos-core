-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Contracts at a shape of a function's generics

A dependency's function has no body, and its callers verify through contracts.
A contract may name a shape of the function's generics, over the shape's own
variables, and a call uses the shape its instantiation matches. A hand-written
`native fun` here stands for what a Rust dependency renders as, with an
associated type as an extra generic parameter.
-/

namespace VerexSL.Tests.Check.Generics.ShapeContracts

verex namespace shape_lib where
  pragma verify = false

  enum Opt {T} has Copy where
    | None
    | Some (value : T)

  opaque struct Store {T} {A} has Copy where

  opaque spec fun store_at {T} {A} (store : Store<T, A>, position : usize) : T

  mod primitive where
    impl «u32» where
      native fun checked_add(_1 : u32, _2 : u32) -> Opt<u32>
      spec checked_add where
        ensures (_1 + _2 < 4294967296) ==> (result is Some && result.value == _1 + _2)
        ensures (4294967296 <= _1 + _2) ==> result is None

  -- A contract over all generics, read at a caller's type arguments.
  native fun keep {T}(_1 : T) -> T
  spec keep where
    ensures result == _1

  -- A shape of concrete types only.
  native fun unwrap_u32 {T}(_1 : Opt<T>, _2 : T) -> T
  spec unwrap_u32 at {u32} where
    ensures (_1 is None) ==> result == _2
    ensures (_1 is Some) ==> result == _1.value

  -- The binders in the exporter's order, an associated type last: a shape's
  -- positions follow them, and `T` twice makes the output the element type.
  native fun index {T} {I} {A} {Output}(_1 : &Store<T, A>, _2 : I) -> &Output
  spec index at {T} {usize} {A} {T} where
    requires _2 < 10
    ensures result == store_at(_1, _2)

  -- A variable shape, and shapes apart.
  native fun unwrap_any {T}(_1 : Opt<T>, _2 : T) -> T
  spec unwrap_any at {T} where
    ensures (_1 is None) ==> result == _2

  native fun of {T}(_1 : T) -> u32
  spec of at {u32} where
    ensures result == _1
  spec of at {Bool} where
    ensures result == (if _1 then 1 else 0)
  spec of at {Opt<T>} where
    ensures result == 7

  -- A variable twice in a shape.
  native fun second {A} {B}(_1 : A, _2 : B) -> B
  spec second at {A} {A} where
    ensures result == _2

  native fun same_width {T}(_1 : T) -> T
  spec same_width at {usize} where
    ensures result == _1
  spec same_width at {u64} where
    ensures result == _1

  -- A contract over a lifetime binder and a type binder.
  native fun through {a : lifetime} {T}(_1 : &T) -> T
  spec through where
    ensures result == _1

  -- A shape spells no lifetime binder, whose position holds a placeholder.
  native fun pick {a : lifetime} {T} {Output}(_1 : &T, _2 : Output) -> Output
  spec pick at {T} {u32} where
    ensures result == _2

verex namespace shape_callers where
  use shape_lib::same_width
  use shape_lib::Opt
  use shape_lib::Store
  use shape_lib::keep
  use shape_lib::unwrap_u32
  use shape_lib::index
  use shape_lib::unwrap_any
  use shape_lib::of
  use shape_lib::second
  use shape_lib::primitive::«u32»::checked_add
  use shape_lib::store_at
  use shape_lib::through
  use shape_lib::pick

  fun add(left : u32, right : u32) -> Opt<u32> :=
    checked_add(left, right)
  spec add where
    ensures (left + right < 4294967296) ==> result is Some

  fun keep_u32(value : u32) -> u32 := keep::<u32>(value)
  spec keep_u32 where
    ensures result == value

  fun keep_generic {T}(value : T) -> T := keep::<T>(value)
  spec keep_generic where
    ensures result == value

  fun unwrap_or_five(value : Opt<u32>) -> u32 := unwrap_u32::<u32>(value, 5)
  spec unwrap_or_five where
    ensures (value is None) ==> result == 5

  fun at_position {T} {A}(store : &Store<T, A>, position : usize) -> &T :=
    index::<T, usize, A, T>(store, position)
  spec at_position where
    requires position < 5
    ensures result == store_at(store, position)

  fun at_position_u32(store : &Store<u32, Bool>) -> &u32 :=
    index::<u32, usize, Bool, u32>(store, 3)
  spec at_position_u32 where
    ensures result == store_at(store, 3)

  -- The call's type argument is still a parameter, which `{T}` matches.
  fun unwrap_generic {T}(value : Opt<T>, other : T) -> T :=
    unwrap_any::<T>(value, other)
  spec unwrap_generic where
    pragma opaque
    ensures (value is None) ==> result == other

  -- Through the theorem of a generic callee, which assumes the shape.
  fun unwrap_through(value : Opt<u32>) -> u32 := unwrap_generic::<u32>(value, 9)
  spec unwrap_through where
    ensures (value is None) ==> result == 9

  -- Inlined, at the caller's frame: the call in its body is at its own parameter.
  fun unspecified {T}(value : Opt<T>, other : T) -> T := unwrap_any::<T>(value, other)

  fun unwrap_inlined(value : Opt<u32>) -> u32 := unspecified::<u32>(value, 4)
  spec unwrap_inlined where
    ensures (value is None) ==> result == 4

  fun of_u32(value : u32) -> u32 := of::<u32>(value)
  spec of_u32 where
    ensures result == value

  fun of_bool(value : Bool) -> u32 := of::<Bool>(value)
  spec of_bool where
    ensures value ==> result == 1

  fun of_opt {T}(value : Opt<T>) -> u32 := of::<Opt<T> >(value)
  spec of_opt where
    ensures result == 7

  fun second_u32(left : u32, right : u32) -> u32 := second::<u32, u32>(left, right)
  spec second_u32 where
    ensures result == right

  fun second_generic {T}(left : T, right : T) -> T := second::<T, T>(left, right)
  spec second_generic where
    ensures result == right

  -- A call matches on VIR types, so these are two shapes, stating one contract.
  fun both_widths(left : usize, right : u64) -> u64 := do
    let kept := same_width::<usize>(left)
    let other := same_width::<u64>(right)
    if kept == 0usize then other else other
  spec both_widths where
    ensures result == right

  fun through_u64(value : &u64) -> u64 := through::<u64>(value)
  spec through_u64 where
    ensures result == value

  fun pick_u32(value : &u64, other : u32) -> u32 := pick::<u64, u32>(value, other)
  spec pick_u32 where
    ensures result == other

end VerexSL.Tests.Check.Generics.ShapeContracts
