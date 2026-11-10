-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace lifetime_constructor where
  struct Holder {a : lifetime} where
    r : &[a] mut u32

  struct Shared {a : lifetime} where
    r : &[a] u32

  struct Generic {a : lifetime} {T} where
    r : &[a] T

  struct Nested {a : lifetime} where
    holder : Holder<lifetime a>
    k : u32

  enum Slot {a : lifetime} where
    | Empty = 0
    | Full (0 : &[a] u32) = 1

  fun hold(mut x : u32) -> u32 := do
    let _3 := &mut x
    let h := new Holder<lifetime _> { r := &mut *_3 }
    let _4 := &mut *(h.r)
    *_4 := 5u32
    x

  fun read(x : u32) -> u32 := do
    let h := new Shared<lifetime _> { r := &x }
    let _4 := copy(h.r)
    *_4

  fun generic(x : u32) -> u32 := do
    let h := new Generic<lifetime _, u32> { r := &x }
    let _4 := copy(h.r)
    *_4

  fun nested(mut x : u32) -> u32 := do
    let _4 := &mut x
    let n :=
      new Nested<lifetime _> {
        holder := new Holder<lifetime _> { r := &mut *_4 }, k := 4u32
      }
    let _6 := &mut *((n.holder).r)
    *_6 := n.k
    x

  fun slot(x : u32) -> u32 := do
    let s := new Slot<lifetime _>::Full { 0 := &x }
    let mut _0 : u32
    match core.readDiscriminant[isize](s) with
      | 0 => _0 := 0u32
      | 1 => do
        let r := copy(s.0)
        _0 := *r
    _0
