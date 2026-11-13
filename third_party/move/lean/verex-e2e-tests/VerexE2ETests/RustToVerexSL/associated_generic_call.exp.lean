-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace alloc where
  pragma verify = false

  mod alloc where
    opaque struct Global has Copy where

  mod vec where
    trait impl «<std::vec::Vec<T, A> as std::ops::Index<I>>» where
      @[shape_only]
      native fun index {T} {I} {A} {BorrowedOutput}(
        _1 : &Vector<T>, _2 : I
      ) -> BorrowedOutput

verex namespace associated_generic_call where
  use alloc::alloc::Global
  use alloc::vec::«<std::vec::Vec<T, A> as std::ops::Index<I>>»::index

  fun first {T}(values : &Vector<T>) -> &T :=
    index::<T, usize, Global, &T>(values, 0usize)
