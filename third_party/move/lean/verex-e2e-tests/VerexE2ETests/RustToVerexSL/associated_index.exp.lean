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

    trait impl «<std::vec::Vec<T, A> as std::ops::IndexMut<I>>» where
      @[shape_only]
      native fun index_mut {T} {I} {A} {Output}(
        _1 : &mut Vector<T>, _2 : I
      ) -> &mut Output

verex namespace associated_index where
  use alloc::alloc::Global
  use alloc::vec::«<std::vec::Vec<T, A> as std::ops::Index<I>>»::index
  use alloc::vec::«<std::vec::Vec<T, A> as std::ops::IndexMut<I>>»::index_mut

  fun element(values : &Vector<u32>, index : usize) -> u32 := do
    let _3 := index::<u32, usize, Global, &u32>(values, index)
    *_3

  fun first(values : &Vector<u32>) -> u32 := do
    let _2 := index::<u32, usize, Global, &u32>(values, 0usize)
    *_2

  fun reference(values : &Vector<u32>, index : usize) -> &u32 :=
    index::<u32, usize, Global, &u32>(values, index)

  fun store(values : &mut Vector<u32>, index : usize, value : u32) -> Unit := do
    let _4 := index_mut::<u32, usize, Global, u32>(values, index)
    *_4 := value
