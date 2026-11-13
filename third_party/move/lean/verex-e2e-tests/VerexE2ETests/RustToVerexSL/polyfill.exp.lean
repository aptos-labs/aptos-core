-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod panicking where
    enum AssertKind where
      | Eq = 0
      | Ne = 1
      | Match = 2

    native fun assert_failed {T} {U}(
      _1 : AssertKind, _2 : &T, _3 : &U,
      _4 : option::Option<fmt::Arguments<lifetime _> >
    ) -> Never

  mod fmt where
    opaque struct Arguments {a : lifetime} has Copy where

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

    trait impl «<std::option::Option<T> as std::cmp::PartialEq>» where
      native fun eq {T}(_1 : &Option<T>, _2 : &Option<T>) -> Bool

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

  mod iter where
    mod traits where
      mod collect where
        trait impl «<I as std::iter::IntoIterator>» where
          native fun into_iter {I}(_1 : I) -> I

  mod primitive where
    impl slice where
      native fun iter {T}(_1 : &Slice<T>) -> slice::iter::Iter<lifetime _, T>

verex namespace polyfill where
  use core::iter::traits::collect::«<I as std::iter::IntoIterator>»::into_iter
  use core::option::Option
  use core::primitive::slice::iter
  use core::slice::iter::Iter
  use core::slice::iter::«<std::slice::Iter<'a, T> as std::iter::Iterator>»::next

  fun constant {N : const usize}() -> usize := 7usize

  fun bool_constant {B : const Bool}() -> Bool := true

  fun promoted() -> &u64 := &5u64

  fun promoted_array() -> u32 := do
    let _1 := &#[1u32, 2u32, 3u32]
    _1[1usize]

  fun is_none(x : Option<u32>) -> Bool :=
    core::option::«<std::option::Option<T> as std::cmp::PartialEq>»::eq(
      &x, &new Option<u32>::None {}
    )

  trait impl «<polyfill::Unit as std::cmp::PartialEq>» where
    fun eq(self : &«Unit», other : &«Unit») -> Bool := do
      let __self_discr := core.readDiscriminant[isize](*self)
      let __arg1_discr := core.readDiscriminant[isize](*other)
      __self_discr == __arg1_discr

  enum «Unit» where
    | A = 0
    | B = 1

  fun is_a(x : «Unit») -> Bool :=
    «<polyfill::Unit as std::cmp::PartialEq>»::eq(&x, &new «Unit»::A {})

  fun assertion(x : u32) -> Unit := do
    let _2 := (&x, &1u32)
    let left_val := copy(_2.0)
    let right_val := copy(_2.1)
    if !(*left_val == *right_val) then panic()

  fun iter_count {K}(slice : &Slice<K>) -> usize := do
    let mut count := 0usize
    let mut iter := into_iter(iter(slice))
    loop do
      let _7 := &mut iter
      let _6 := next(_7)
      match core.readDiscriminant[isize](_6) with
        | 0 => return count
        | 1 => do
          count := core.prim.checkedAddPanic(count, 1usize)
          continue

  fun twice {T has Copy}(x : T) -> (T, T) := (x, x)

  fun unused_copy {T has Copy}() -> Unit := ()

  fun super_copy {T has Copy}(x : T) -> (T, T) := (x, x)

  fun shorten {a : lifetime outlives b} {b : lifetime}(
    x : &[a] u32
  ) -> &[b] u32 :=
    x

  struct Bounded {a : lifetime outlives b} {b : lifetime} where
    x : &[a] u32
    y : &[b] u32

  impl Bounded where
    fun get {a : lifetime outlives b} {b : lifetime}(
      self : Bounded<lifetime a, lifetime b>
    ) -> &[b] u32 := copy(self.x)
