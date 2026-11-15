-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace alloc where
  pragma verify = false

  mod alloc where
    opaque struct Global has Copy where

  mod vec where
    trait impl «<std::vec::Vec<T, A> as std::ops::Deref>» where
      native fun deref {T} {A}(_1 : &Vector<T>) -> &Slice<T>

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

  mod primitive where
    impl slice where
      @[shape_only]
      native fun get {T} {I} {BorrowedOutput}(
        _1 : &Slice<T>, _2 : I
      ) -> option::Option<BorrowedOutput>

verex namespace associated_get where
  use alloc::alloc::Global
  use alloc::vec::«<std::vec::Vec<T, A> as std::ops::Deref>»::deref
  use core::option::Option
  use core::primitive::slice::get

  fun lookup(values : &Vector<u32>, index : usize) -> Option<&u32> :=
    get::<u32, usize, &u32>(deref::<u32, Global>(values), index)

  fun lookup_first(values : &Vector<u64>) -> Option<&u64> :=
    get::<u64, usize, &u64>(deref::<u64, Global>(values), 0usize)

  fun lookup_generic {T}(values : &Slice<T>, index : usize) -> Option<&T> :=
    get::<T, usize, &T>(values, index)
