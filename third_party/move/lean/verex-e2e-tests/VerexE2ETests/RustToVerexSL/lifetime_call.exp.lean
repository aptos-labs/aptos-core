-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace lifetime_call where
  struct Shared {a : lifetime} where
    r : &[a] u32

  struct Generic {a : lifetime} {T} where
    r : &[a] T

  impl Shared where
    fun get {a : lifetime}(self : &[a] Shared<lifetime a>) -> u32 := do
      let _2 := copy((*self).r)
      *_2

    fun inner {a : lifetime} {l1 : lifetime}(
      self : &[l1] Shared<lifetime a>
    ) -> &[a] u32 := copy((*self).r)

  impl Generic where
    fun inner {a : lifetime} {T} {l2 : lifetime}(
      self : &[l2] Generic<lifetime a, T>
    ) -> &[a] T := copy((*self).r)

  fun shrink {s : lifetime} {l : lifetime outlives s}(
    h : Shared<lifetime _>
  ) -> Shared<lifetime _> := h

  fun method(x : u32) -> u32 := do
    let h := new Shared<lifetime _> { r := &x }
    Shared::get(&h)

  fun shrunk(x : u32) -> u32 := do
    let h := shrink(new Shared<lifetime _> { r := &x })
    let _5 := copy(h.r)
    *_5

  fun generic(x : u32) -> u32 := do
    let h := new Generic<lifetime _, u32> { r := &x }
    let _4 := Generic::inner(&h)
    *_4
