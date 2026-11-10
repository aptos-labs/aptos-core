-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace alloc where
  pragma verify = false

  mod alloc where
    opaque struct Global has Copy where

  mod vec where
    trait impl «<&'a std::vec::Vec<T, A> as std::iter::IntoIterator>» where
      @[shape_only]
      native fun into_iter {a : lifetime} {T} {A} {IntoIter}(
        _1 : &[a] Vector<T>
      ) -> IntoIter

verex namespace core where
  pragma verify = false

  mod iter where
    mod adapters where
      mod enumerate where
        struct Enumerate {I} where
          iter : I
          count : usize

        trait impl «<std::iter::Enumerate<I> as std::iter::Iterator>» where
          @[shape_only]
          native fun next {I} {Item}(
            _1 : &mut Enumerate<I>
          ) -> option::Option<(usize, Item)>

      mod rev where
        struct Rev {T} where
          iter : T

        trait impl «<std::iter::Rev<I> as std::iter::Iterator>» where
          @[shape_only]
          native fun next {I} {Item}(_1 : &mut Rev<I>) -> option::Option<Item>

      mod step_by where
        struct StepBy {I} where
          iter : I
          step_minus_one : usize
          first_take : Bool

        trait impl «<std::iter::StepBy<I> as std::iter::Iterator>» where
          @[shape_only]
          native fun next {I} {Item}(
            _1 : &mut StepBy<I>
          ) -> option::Option<Item>

    mod traits where
      mod collect where
        trait impl «<I as std::iter::IntoIterator>» where
          native fun into_iter {I}(_1 : I) -> I

      mod iterator where
        trait Iterator where
          native fun enumerate {Self}(
            _1 : Self
          ) -> adapters::enumerate::Enumerate<Self>

          native fun rev {Self}(_1 : Self) -> adapters::rev::Rev<Self>

          native fun step_by {Self}(
            _1 : Self, _2 : usize
          ) -> adapters::step_by::StepBy<Self>

    mod range where
      trait impl «std::iter::range::<impl std::iter::Iterator for std::ops::RangeInclusive<A>>» where
        native fun next {A}(
          _1 : &mut ops::range::RangeInclusive<A>
        ) -> option::Option<A>

  mod ops where
    mod range where
      struct «Range» {Idx} where
        start : Idx
        end : Idx

      struct RangeInclusive {Idx} where
        start : Idx
        end : Idx
        exhausted : Bool

      impl RangeInclusive where
        native fun new {Idx}(_1 : Idx, _2 : Idx) -> RangeInclusive<Idx>

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

  mod slice where
    mod iter where
      struct Iter {a : lifetime} {T} where
        slice : &[a] Slice<T>
        pos : usize
        end : usize

      trait impl «<std::slice::Iter<'a, T> as std::iter::Iterator>» where
        native fun next {a : lifetime} {T} {l2 : lifetime}(
          _1 : &[l2] mut Iter<lifetime a, T>
        ) -> option::Option<&[a] T>

  mod primitive where
    impl slice where
      native fun iter {T}(_1 : &Slice<T>) -> slice::iter::Iter<lifetime _, T>

verex namespace associated_iterators where
  use alloc::alloc::Global
  use core::iter::adapters::enumerate::Enumerate
  use core::iter::adapters::rev::Rev
  use core::iter::adapters::step_by::StepBy
  use core::iter::traits::iterator::Iterator::enumerate
  use core::iter::traits::iterator::Iterator::rev
  use core::iter::traits::iterator::Iterator::step_by
  use core::ops::range::RangeInclusive
  use core::ops::range::RangeInclusive::new
  use core::ops::range::«Range»
  use core::option::Option
  use core::primitive::slice::iter
  use core::slice::iter::Iter

  fun sum_slice(values : &Slice<u32>) -> u64 := do
    let mut total := 0u64
    let mut iter :=
      core::iter::traits::collect::«<I as std::iter::IntoIterator>»::into_iter(
        iter(values)
      )
    loop do
      let _7 := &mut iter
      let _6 :=
        core::slice::iter::«<std::slice::Iter<'a, T> as std::iter::Iterator>»::next(
          _7
        )
      match core.readDiscriminant[isize](_6) with
        | 0 => return total
        | 1 => do
          let x := copy(_6.0)
          total := core.prim.checkedAddPanic(total, *x as u64)
          continue

  fun sum_vec(values : &Vector<u32>) -> u64 := do
    let mut total := 0u64
    let mut iter :=
      alloc::vec::«<&'a std::vec::Vec<T, A> as std::iter::IntoIterator>»::into_iter::<u32, Global, Iter<lifetime _, u32> >(
        values
      )
    loop do
      let _6 := &mut iter
      let _5 :=
        core::slice::iter::«<std::slice::Iter<'a, T> as std::iter::Iterator>»::next(
          _6
        )
      match core.readDiscriminant[isize](_5) with
        | 0 => return total
        | 1 => do
          let x := copy(_5.0)
          total := core.prim.checkedAddPanic(total, *x as u64)
          continue

  fun inclusive(n : u32) -> u32 := do
    let mut total := 0u32
    let mut iter :=
      core::iter::traits::collect::«<I as std::iter::IntoIterator>»::into_iter(
        new(0u32, n)
      )
    loop do
      let _7 := &mut iter
      let _6 :=
        core::iter::range::«std::iter::range::<impl std::iter::Iterator for std::ops::RangeInclusive<A>>»::next(
          _7
        )
      match core.readDiscriminant[isize](_6) with
        | 0 => return total
        | 1 => do
          let i := _6.0
          total := core.prim.checkedAddPanic(total, i)
          continue

  fun reversed(n : u32) -> u32 := do
    let mut total := 0u32
    let mut iter :=
      core::iter::traits::collect::«<I as std::iter::IntoIterator>»::into_iter(
        rev(new «Range»<u32> { start := 0u32, end := n })
      )
    loop do
      let _8 := &mut iter
      let _7 :=
        core::iter::adapters::rev::«<std::iter::Rev<I> as std::iter::Iterator>»::next::<«Range»<u32>, u32>(
          _8
        )
      match core.readDiscriminant[isize](_7) with
        | 0 => return total
        | 1 => do
          let i := _7.0
          total := core.prim.checkedAddPanic(total, i)
          continue

  fun position(values : &Slice<u32>, target : u32) -> usize := do
    let mut iter :=
      core::iter::traits::collect::«<I as std::iter::IntoIterator>»::into_iter(
        enumerate(iter(values))
      )
    let mut i : usize
    loop do
      let _8 := &mut iter
      let _7 :=
        core::iter::adapters::enumerate::«<std::iter::Enumerate<I> as std::iter::Iterator>»::next::<Iter<lifetime _, u32>, &u32>(
          _8
        )
      match core.readDiscriminant[isize](_7) with
        | 0 => return values.length
        | 1 => do
          i := (_7.0).0
          let x := copy((_7.0).1)
          if *x == target then break else continue
    i

  fun stepped(n : u32) -> u32 := do
    let mut total := 0u32
    let mut iter :=
      core::iter::traits::collect::«<I as std::iter::IntoIterator>»::into_iter(
        step_by(new «Range»<u32> { start := 0u32, end := n }, 2usize)
      )
    loop do
      let _8 := &mut iter
      let _7 :=
        core::iter::adapters::step_by::«<std::iter::StepBy<I> as std::iter::Iterator>»::next::<«Range»<u32>, u32>(
          _8
        )
      match core.readDiscriminant[isize](_7) with
        | 0 => return total
        | 1 => do
          let i := _7.0
          total := core.prim.checkedAddPanic(total, i)
          continue
