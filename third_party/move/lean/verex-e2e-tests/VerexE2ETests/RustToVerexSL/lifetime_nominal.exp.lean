-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace lifetime_nominal where
  struct Shared {a : lifetime} where
    r : &[a] u32

  fun make {a : lifetime}(x : &[a] u32) -> Shared<lifetime a> :=
    new Shared<lifetime _> { r := x }

  fun value(h : Shared<lifetime _>) -> u32 := do
    let _2 := copy(h.r)
    *_2

  fun through(x : u32) -> u32 := do
    let h := make(&x)
    value(h)

  fun joined(c : Bool, a : u32, b : u32) -> u32 := do
    let mut h : Shared<lifetime _>
    if c then h := new Shared<lifetime _> { r := &a }
    else h := new Shared<lifetime _> { r := &b }
    let _7 := copy(h.r)
    *_7

  fun larger {a : lifetime}(
    a : Shared<lifetime a>, b : Shared<lifetime a>
  ) -> Shared<lifetime a> := do
    let _6 := copy(a.r)
    let _7 := copy(b.r)
    if *_6 > *_7 then a else b

  fun replace {a : lifetime}(
    h : &[a] mut Shared<lifetime a>, y : &[a] u32
  ) -> Unit := (*h).r := y
