-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

  mod iter where
    mod traits where
      mod collect where
        trait impl «<I as std::iter::IntoIterator>» where
          native fun into_iter {I}(_1 : I) -> I

      mod iterator where
        trait Iterator where
          @[shape_only]
          native fun last {Self} {Item}(_1 : Self) -> option::Option<Item>

verex namespace associated_user_iterator where
  use core::iter::traits::collect::«<I as std::iter::IntoIterator>»::into_iter
  use core::iter::traits::iterator::Iterator::last
  use core::option::Option

  struct Countdown where
    remaining : u32

  trait impl «<associated_user_iterator::Countdown as std::iter::Iterator>» where
    fun next(self : &mut Countdown) -> Option<u32> := do
      let mut _0 : Option<u32>
      if (*self).remaining == 0u32 then _0 := new Option<u32>::None {}
      else
        (*self).remaining := core.prim.checkedSubtractPanic(
          (*self).remaining, 1u32
        )
        _0 := new Option<u32>::Some { 0 := (*self).remaining }
      _0

  fun total(countdown : Countdown) -> u32 := do
    let mut total := 0u32
    let mut iter := into_iter(countdown)
    loop do
      let _6 := &mut iter
      let _5 :=
        «<associated_user_iterator::Countdown as std::iter::Iterator>»::next(_6)
      match core.readDiscriminant[isize](_5) with
        | 0 => return total
        | 1 => do
          let value := _5.0
          total := core.prim.checkedAddPanic(total, value)
          continue

  fun final_value(countdown : Countdown) -> Option<u32> :=
    last::<Countdown, u32>(countdown)
