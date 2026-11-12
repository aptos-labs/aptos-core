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

    impl Vec where
      native fun len {T} {A}(_1 : &Vector<T>) -> usize

      native fun push {T} {A}(_1 : &mut Vector<T>, _2 : T) -> Unit

verex namespace two_phase where
  use alloc::alloc::Global
  use alloc::vec::Vec::len
  use alloc::vec::Vec::push
  use alloc::vec::«<std::vec::Vec<T, A> as std::ops::Index<I>>»::index

  struct C where
    n : u32

  impl C where
    fun get(self : &C) -> u32 := (*self).n

    fun add(self : &mut C, d : u32) -> Unit :=
      (*self).n := core.prim.checkedAddPanic((*self).n, d)

  struct Pair where
    a : C
    b : C

  struct Holder {a : lifetime} where
    r : &[a] mut u32

  fun add_to(target : &mut u32, d : u32) -> Unit :=
    *target := core.prim.checkedAddPanic(*target, d)

  fun two_phase_local(n : u32) -> u32 := do
    let mut c := new C { n }
    let _5 := C::get(&c)
    C::add(&mut c, _5)
    c.n

  fun two_phase_free(c : &mut C) -> Unit := do
    let _3 := C::get(&*c)
    C::add(c, _3)

  fun push_len(v : &mut Vector<usize>) -> Unit := do
    let _3 := len::<usize, Global>(&*v)
    push::<usize, Global>(v, _3)

  fun holder(h : Holder<lifetime _>) -> Unit := do
    let _5 := &mut *(h.r)
    let _3 := *_5
    add_to(&mut *(h.r), _3)

  fun index_operand(c : &mut C, a : &Slice<u32>, i : usize) -> Unit := do
    let _5 := a[i]
    C::add(c, _5)

  fun index_self(v : &mut Vector<u32>) -> Unit := do
    let _4 := index::<u32, usize, Global, &u32>(&*v, 0usize)
    let _3 := *_4
    push::<u32, Global>(v, _3)

  fun local_across(b : Bool, n : u32) -> u32 := do
    let mut c := new C { n }
    let mut _6 : u32
    if b then _6 := C::get(&c) else _6 := 1u32
    C::add(&mut c, _6)
    c.n

  fun disjoint_fields(mut p : Pair) -> u32 := do
    let _4 := C::get(&p.b)
    C::add(&mut p.a, _4)
    C::add(&mut p.b, 1u32)
    C::add(&mut p.b, 1u32)
    let _10 := C::get(&p.b)
    C::add(&mut p.a, _10)
    p.a.n
