-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace closures where
  fun direct(c : u32) -> u32 := do
    let g :=
      closure «direct::{closure#0}» {u0 : lifetime} has Copy (
        c : &[u0] u32 := &c
      )
        fun call(
          self : &«direct::{closure#0}»<lifetime _>, a : u32
        ) -> u32 := do
          let _5 := copy((*self).c)
          core.prim.checkedAddPanic(a, *_5)
    let _4 := «direct::{closure#0}»::call(&g, 1u32)
    core.prim.checkedAddPanic(_4, «direct::{closure#0}»::call(&g, 2u32))

  fun no_upvars(x : u32) -> u32 := do
    let _2 :=
      closure «no_upvars::{closure#0}» has Copy ()
        fun call(self : &«no_upvars::{closure#0}», a : u32, b : u32) -> u32 :=
          core.prim.checkedAddPanic(a, b)
    «no_upvars::{closure#0}»::call(&new «no_upvars::{closure#0}» {}, x, 1u32)

  fun counter(n0 : u32) -> u32 := do
    let mut n := n0
    let mut inc :=
      closure «counter::{closure#0}» {u0 : lifetime} (
        n : &[u0] mut u32 := &mut n
      )
        fun call_mut(
          self : &mut «counter::{closure#0}»<lifetime _>, d : u32
        ) -> Unit := do
          let _4 := &mut *((*self).n)
          let _3 := overflowing_add(*_4, d)
          let _5 := &mut *((*self).n)
          if move(_3.1) then panic()
          else
            let _6 := &mut *((*self).n)
            *_6 := move(_3.0)
    «counter::{closure#0}»::call_mut(&mut inc, 1u32)
    «counter::{closure#0}»::call_mut(&mut inc, 2u32)
    n

  fun consume(v : Vector<u32>) -> Vector<u32> := do
    let take :=
      closure «consume::{closure#0}» (v : Vector<u32> := v)
        fun call_once(
          self : «consume::{closure#0}»
        ) -> Vector<u32> := move(self.v)
    «consume::{closure#0}»::call_once(take)

  fun moved_counter(start : u32) -> u32 := do
    let mut next :=
      closure «moved_counter::{closure#0}» has Copy (count : u32 := start)
        fun call_mut(self : &mut «moved_counter::{closure#0}») -> u32 := do
          (*self).count := core.prim.checkedAddPanic((*self).count, 1u32)
          (*self).count
    let _3 := «moved_counter::{closure#0}»::call_mut(&mut next)
    «moved_counter::{closure#0}»::call_mut(&mut next)

  fun nested(x : u32) -> u32 := do
    let outer :=
      closure «nested::{closure#0}» {u0 : lifetime} has Copy (
        x : &[u0] u32 := &x
      )
        fun call(
          self : &«nested::{closure#0}»<lifetime _>, y : u32
        ) -> u32 := do
          let inner :=
            closure «nested::{closure#0}::{closure#0}» {u0 : lifetime} {u1 : lifetime} has Copy (
              y : &[u0] u32 := &y, x : &[u1] u32 := copy(
                (*self).x
              )
            )
              fun call(
                self : &«nested::{closure#0}::{closure#0}»<lifetime _, lifetime _>,
                z : u32
              ) -> u32 := do
                let _8 := copy((*self).y)
                let _9 := copy((*self).x)
                core.prim.checkedAddPanic(
                  core.prim.checkedAddPanic(z, *_8), *_9
                )
          «nested::{closure#0}::{closure#0}»::call(&inner, 1u32)
    «nested::{closure#0}»::call(&outer, 2u32)

  fun copied(c : u32) -> u32 := do
    let g :=
      closure «copied::{closure#0}» {u0 : lifetime} has Copy (
        c : &[u0] u32 := &c
      )
        fun call(
          self : &«copied::{closure#0}»<lifetime _>, a : u32
        ) -> u32 := do
          let _5 := copy((*self).c)
          core.prim.checkedAddPanic(a, *_5)
    let h := g
    let _5 := «copied::{closure#0}»::call(&g, 1u32)
    core.prim.checkedAddPanic(_5, «copied::{closure#0}»::call(&h, 2u32))

  fun zero_sized_copy(x : u32) -> u32 := do
    let _2 :=
      closure «zero_sized_copy::{closure#0}» has Copy ()
        fun call(self : &«zero_sized_copy::{closure#0}», a : u32) -> u32 :=
          core.prim.checkedAddPanic(a, 1u32)
    let _3 := new «zero_sized_copy::{closure#0}» {}
    let _4 :=
      «zero_sized_copy::{closure#0}»::call(
        &new «zero_sized_copy::{closure#0}» {}, x
      )
    core.prim.checkedAddPanic(
      _4,
      «zero_sized_copy::{closure#0}»::call(
        &new «zero_sized_copy::{closure#0}» {}, x
      )
    )

  fun zero_sized_capture(x : u32) -> u32 := do
    let _2 :=
      closure «zero_sized_capture::{closure#1}» has Copy (
        0 : «zero_sized_capture::{closure#0}» := closure «zero_sized_capture::{closure#0}» has Copy ()
          fun call(self : &«zero_sized_capture::{closure#0}», a : u32) -> u32 :=
            core.prim.checkedAddPanic(a, 1u32))
        fun call(self : &«zero_sized_capture::{closure#1}», b : u32) -> u32 :=
          «zero_sized_capture::{closure#0}»::call(&(*self).0, b)
    «zero_sized_capture::{closure#1}»::call(
      &new «zero_sized_capture::{closure#1}» {
        0 := new «zero_sized_capture::{closure#0}» {}
      },
      x
    )

  fun make() -> «make::{closure#0}» :=
    closure «make::{closure#0}» has Copy ()
      fun call(self : &«make::{closure#0}», a : u32) -> u32 :=
        core.prim.checkedAddPanic(a, 1u32)

  fun use_make(x : u32) -> u32 := do
    let _3 := new «make::{closure#0}» {}
    let _2 := make()
    «make::{closure#0}»::call(&new «make::{closure#0}» {}, x)

  fun unused(x : u32) -> u32 := x

  -- Its parent never uses the closure, so the closure stands at the parent's entry.
  fun never_makes() -> «never_makes::{closure#0}» := do
    closure «never_makes::{closure#0}» has Copy ()
      fun call(self : &«never_makes::{closure#0}», a : u32) -> u32 :=
        core.prim.checkedAddPanic(a, 1u32)
    panic()

  fun use_never_makes(x : u32) -> u32 :=
    «never_makes::{closure#0}»::call(&never_makes(), x)

  fun looped(n : u8) -> u8 := do
    let _2 :=
      closure «looped::{closure#0}» has Copy ()
        fun call(self : &«looped::{closure#0}», m : u8) -> u8 := do
          let mut s := 0u8
          let mut i := 0u8
          loop if 8u8 == 0u8 then panic()
          else
            if i < core.prim.checkedModuloPanic(m, 8u8) then
              s := core.prim.checkedAddPanic(s, i)
              i := core.prim.checkedAddPanic(i, 1u8)
              continue
            else
              return s;
    let mut total := 0u8
    let mut j := 0u8
    loop if 8u8 == 0u8 then panic()
    else
      if j < core.prim.checkedModuloPanic(n, 8u8) then
        total := core.prim.checkedAddPanic(
          total, «looped::{closure#0}»::call(
            &new «looped::{closure#0}» {}, j
          )
        )
        j := core.prim.checkedAddPanic(j, 1u8)
        continue
      else
        return total;

  mod m where
    struct S where
      v : u32

    impl S where
      fun get(self : &S) -> u32 := do
        let k :=
          closure «S::get::{closure#0}» {u0 : lifetime} has Copy (
            self : &[u0] S := self
          )
            fun call(
              self : &«S::get::{closure#0}»<lifetime _>, a : u32
            ) -> u32 := do
              let _5 := copy((*self).self)
              core.prim.checkedAddPanic(a, (*_5).v)
        «S::get::{closure#0}»::call(&k, 1u32)

    fun inside(c : u32) -> u32 := do
      let g :=
        closure «inside::{closure#0}» {u0 : lifetime} has Copy (
          c : &[u0] u32 := &c
        )
          fun call(
            self : &«inside::{closure#0}»<lifetime _>, a : u32
          ) -> u32 := do
            let _5 := copy((*self).c)
            core.prim.checkedAddPanic(a, *_5)
      «inside::{closure#0}»::call(&g, 1u32)
