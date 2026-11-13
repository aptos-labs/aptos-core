-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace alloc where
  pragma verify = false

  mod alloc where
    opaque struct Global has Copy where

  mod vec where
    trait impl «<std::vec::Vec<T, A> as std::ops::IndexMut<I>>» where
      @[shape_only]
      native fun index_mut {T} {I} {A} {Output}(
        _1 : &mut Vector<T>, _2 : I
      ) -> &mut Output

verex namespace declared_lifetimes where
  use alloc::alloc::Global
  use alloc::vec::«<std::vec::Vec<T, A> as std::ops::IndexMut<I>>»::index_mut

  -- Signatures keep their declared lifetimes, and their bodies stay inferred. An
  -- elided lifetime is named only where it tells a result from a parameter.
  struct Holder {a : lifetime} where
    r : &[a] mut u32

  fun sel {a : lifetime} {b : lifetime}(
    x : &[a] mut u32, _y : &[b] mut u32
  ) -> &[a] mut u32 := &mut *x

  fun named_unit {a : lifetime}(x : &[a] mut u32) -> Unit := *x := 1u32

  fun elided_unit(x : &mut u32, y : &u32) -> Unit := *x := *y

  fun elided_one(x : &mut u32) -> &mut u32 := &mut *x

  fun elided_with_named {a : lifetime} {l1 : lifetime}(
    x : &[a] mut u32, _y : &[l1] mut u32
  ) -> &[a] mut u32 := &mut *x

  -- The bridge keeps early-bound lifetimes and their explicit outlives bound.
  fun outlives {s : lifetime} {l : lifetime outlives s}(
    x : &[l] mut u32
  ) -> &[s] mut u32 := &mut *x

  fun outlives_static {a : lifetime outlives static}(
    x : &[a] u32
  ) -> &[static] u32 := x

  fun generic {T} {a : lifetime} {l2 : lifetime}(
    x : &[a] mut T, _y : &[l2] u32
  ) -> &[a] mut T := &mut *x

  fun static_result {a : lifetime}(
    _x : &[a] u32, y : &[static] u32
  ) -> &[static] u32 := y

  fun holder {a : lifetime}(x : &[a] mut u32) -> Holder<lifetime a> :=
    new Holder<lifetime _> { r := &mut *x }

  fun inner {s : lifetime} {a : lifetime}(
    h : &[s] mut Holder<lifetime a>
  ) -> &[s] mut u32 := &mut *((*h).r)

  -- Trait-bound early lifetimes remain distinct in the declared signature.
  fun bounded {a : lifetime} {b : lifetime} {T} {l3 : lifetime}(
    _value : &[l3] T, _x : &[a] mut u32, y : &[b] mut u32
  ) -> &[b] mut u32 := &mut *y

  struct S where
    v : u32

  impl S where
    fun first {l0 : lifetime} {l1 : lifetime}(
      self : &[l0] mut S, _other : &[l1] u32
    ) -> &[l0] mut u32 := &mut (*self).v

  impl Holder where
    fun get {a : lifetime} {l1 : lifetime}(
      self : &[l1] mut Holder<lifetime a>
    ) -> &[l1] mut u32 := &mut *((*self).r)

    fun take {a : lifetime}(self : Holder<lifetime a>) -> &[a] mut u32 :=
      &mut *(self.r)

  fun caller(mut a : u32, mut b : u32) -> u32 := do
    let _4 := &mut a
    let _5 := &mut b
    let r := sel(_4, _5)
    b := 2u32
    *r := 1u32
    core.prim.checkedAddPanic(a, b)

  fun vec_index(v : &mut Vector<u32>, i : usize) -> &mut u32 :=
    index_mut::<u32, usize, Global, u32>(v, i)
