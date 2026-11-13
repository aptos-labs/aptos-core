-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Contracts of real dependency calls

The rendering of `RustToVerexSL/dependency_calls.rs`, with the items a library
would splice into its dependency namespaces: a plain contract of
`u32::checked_add` in its scope, and `Option::unwrap_or` at a shape.
-/

namespace VerexSL.Tests.Check.Generics.ShapeContractsRendered

verex namespace core where
  pragma verify = false

  mod option where
    @[destructor]
    opaque struct Option {T} has Copy where

    opaque spec fun is_none {T}(_1 : Option<T>) : Bool
    opaque spec fun held {T}(_1 : Option<T>) : T

    impl Option where
      native fun unwrap_or {T}(_1 : Option<T>, _2 : T) -> T
      spec unwrap_or at {u32} where
        ensures is_none(_1) ==> result == _2
        ensures !is_none(_1) ==> result == held(_1)

  mod primitive where
    impl «u32» where
      native fun checked_add(_1 : u32, _2 : u32) -> option::Option<u32>
      spec checked_add where
        ensures option::is_none(result) <==> 4294967296 <= _1 + _2
        ensures !option::is_none(result) ==> option::held(result) == _1 + _2

verex namespace dependency_calls where
  use core::option::Option
  use core::option::is_none
  use core::option::held
  use core::option::Option::unwrap_or
  use core::primitive::«u32»::checked_add

  fun checked(left : u32, right : u32) -> Option<u32> :=
    checked_add(left, right)
  spec checked where
    ensures left + right < 4294967296 ==> held(result) == left + right

  fun or_zero(value : Option<u32>) -> u32 := unwrap_or(value, 0u32)
  spec or_zero where
    ensures is_none(value) ==> result == 0
    ensures !is_none(value) ==> result == held(value)

end VerexSL.Tests.Check.Generics.ShapeContractsRendered
