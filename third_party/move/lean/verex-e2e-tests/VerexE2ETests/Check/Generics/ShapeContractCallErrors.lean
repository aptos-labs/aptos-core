-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Calls through contracts at a shape, rejected or unproved

A false `ensures` at the caller fails; a call its instantiation does not
match to a shape has no contract and is rejected at its location.
-/

namespace VerexSL.Tests.Check.Generics.ShapeContractCallErrors

verex namespace shape_call_lib where
  pragma verify = false

  enum Opt {T} has Copy where
    | None
    | Some (value : T)

  native fun unwrap_u32 {T}(_1 : Opt<T>, _2 : T) -> T
  spec unwrap_u32 at {u32} where
    ensures (_1 is None) ==> result == _2

  native fun second {A} {B}(_1 : A, _2 : B) -> B
  spec second at {A} {A} where
    ensures result == _2

  opaque struct Store {T} {A} has Copy where

  opaque spec fun store_at {T} {A} (store : Store<T, A>, position : usize) : T

  native fun pick {a : lifetime} {T} {Output}(_1 : &T, _2 : Output) -> Output
  spec pick at {T} {u32} where
    ensures result == _2

  native fun index {T} {I} {A} {Output}(_1 : &Store<T, A>, _2 : I) -> &Output
  spec index at {T} {usize} {A} {T} where
    requires _2 < 10
    ensures result == store_at(_1, _2)

  -- One meaning at one carrier: the verifier's types do not tell the two apart.
  native fun apart {T}(_1 : T) -> T
  spec apart at {usize} where
    ensures result == _1
  spec apart at {u64} where
    ensures result == 0

verex namespace shape_call_errors where
  use shape_call_lib::apart
  use shape_call_lib::Opt
  use shape_call_lib::unwrap_u32
  use shape_call_lib::second
  use shape_call_lib::Store
  use shape_call_lib::index
  use shape_call_lib::pick

  fun false_ensures(value : Opt<u32>) -> u32 := unwrap_u32::<u32>(value, 5)
  spec false_ensures where
    ensures (value is None) ==> result == 6 -- error: the shape says 5

  fun at_bool(value : Opt<Bool>) -> Bool := unwrap_u32::<Bool>(value, true)
  spec at_bool where
    ensures true

  -- The parameter is not `u32`.
  fun at_parameter {T}(value : Opt<T>, other : T) -> T := unwrap_u32::<T>(value, other)
  spec at_parameter where
    ensures true

  -- A variable twice meets two types.
  fun second_apart(left : u32, right : Bool) -> Bool := second::<u32, Bool>(left, right)
  spec second_apart where
    ensures true

  fun second_parameters {T} {U}(left : T, right : U) -> U := second::<T, U>(left, right)
  spec second_parameters where
    ensures true

  fun disagreeing(value : usize) -> usize := apart::<usize>(value)
  spec disagreeing where
    ensures result == value

  -- A call meets the shape's precondition.
  fun position_too_far(store : &Store<u32, Bool>) -> &u32 :=
    index::<u32, usize, Bool, u32>(store, 20)
  spec position_too_far where
    ensures true

  -- The output is not the element type, the second `T` of the shape.
  fun other_output(store : &Store<u32, Bool>) -> &Bool :=
    index::<u32, usize, Bool, Bool>(store, 3)
  spec other_output where
    ensures true

  -- The index is not a `usize`.
  fun other_index(store : &Store<u32, Bool>) -> &u32 :=
    index::<u32, u32, Bool, u32>(store, 3)
  spec other_index where
    ensures true

  -- Nor is a `u64`, though the verifier's types do not tell them apart.
  fun u64_index(store : &Store<u32, Bool>) -> &u32 :=
    index::<u32, u64, Bool, u32>(store, 3)
  spec u64_index where
    ensures true

  -- A shape over a lifetime binder: its contract holds, the claim here does not.
  fun pick_other(value : &u64, other : u32) -> u32 := pick::<u64, u32>(value, other)
  spec pick_other where
    ensures result == other + 1

end VerexSL.Tests.Check.Generics.ShapeContractCallErrors
