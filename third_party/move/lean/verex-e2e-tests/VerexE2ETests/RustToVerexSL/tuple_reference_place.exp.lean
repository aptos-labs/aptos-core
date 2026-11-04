-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace tuple_reference_place where
  fun whole(mut a : u32, mut b : u32) -> u32 := do
    let _4 := &mut a
    let t := (_4, &mut b)
    let _6 := &mut *(t.0)
    *_6 := 1u32
    let _7 := &mut *(t.1)
    *_7 := 2u32
    a

  fun nested(mut a : u32, mut b : u32) -> u32 := do
    let _4 := (&mut a, 7u32)
    let t := (_4, &mut b)
    let _8 := &mut *((t.0).0)
    *_8 := (t.0).1
    let _9 := &mut *(t.1)
    *_9 := 2u32
    a

  fun read(a : u32, b : u32) -> u32 := do
    let t := ((a, b), a)
    (t.0).1

  fun through(t : &mut (u32, u32)) -> Unit := do
    let a := &mut t.0
    let b := &mut t.1
    *a := 1u32
    *b := 2u32
