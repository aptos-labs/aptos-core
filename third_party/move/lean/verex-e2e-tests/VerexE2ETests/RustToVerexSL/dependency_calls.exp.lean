-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace alloc where
  pragma verify = false

  mod alloc where
    opaque struct Global has Copy where

  mod vec where
    impl Vec where
      native fun len {T} {A}(_1 : &Vector<T>) -> usize

      native fun push {T} {A}(_1 : &mut Vector<T>, _2 : T) -> Unit

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

    trait impl «<std::option::Option<T> as std::clone::Clone>» where
      native fun clone {T}(_1 : &Option<T>) -> Option<T>

    impl Option where
      native fun unwrap_or {T}(_1 : Option<T>, _2 : T) -> T

  mod primitive where
    impl «u32» where
      native fun checked_add(_1 : u32, _2 : u32) -> option::Option<u32>

verex namespace dependency_calls where
  use alloc::alloc::Global
  use alloc::vec::Vec::len
  use alloc::vec::Vec::push
  use core::option::Option
  use core::option::Option::unwrap_or
  use core::option::«<std::option::Option<T> as std::clone::Clone>»::clone
  use core::primitive::«u32»::checked_add

  fun checked(left : u32, right : u32) -> Option<u32> :=
    checked_add(left, right)

  fun or_zero(value : Option<u32>) -> u32 := unwrap_or(value, 0u32)

  fun length(values : &Vector<u32>) -> usize := len::<u32, Global>(values)

  fun push_one(values : &mut Vector<u32>) -> Unit := do
    push::<u32, Global>(values, 1u32)

  fun copied(value : &Option<u32>) -> Option<u32> := clone(value)
