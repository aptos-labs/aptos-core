-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod ops where
    mod range where
      struct «Range» {Idx} where
        start : Idx
        end : Idx

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

  mod iter where
    mod traits where
      mod collect where
        trait impl «<I as std::iter::IntoIterator>» where
          native fun into_iter {I}(_1 : I) -> I

    mod range where
      trait impl «std::iter::range::<impl std::iter::Iterator for std::ops::Range<A>>» where
        native fun next {A}(
          _1 : &mut ops::range::«Range»<A>
        ) -> option::Option<A>

verex namespace associated_for_range where
  use core::iter::range::«std::iter::range::<impl std::iter::Iterator for std::ops::Range<A>>»::next
  use core::iter::traits::collect::«<I as std::iter::IntoIterator>»::into_iter
  use core::ops::range::«Range»
  use core::option::Option

  fun sum(n : u32) -> u32 := do
    let mut total := 0u32
    let mut iter := into_iter(new «Range»<u32> { start := 0u32, end := n })
    loop do
      let _7 := &mut iter
      let _6 := next(_7)
      match core.readDiscriminant[isize](_6) with
        | 0 => return total
        | 1 => do
          let i := _6.0
          total := core.prim.checkedAddPanic(total, i)
          continue
