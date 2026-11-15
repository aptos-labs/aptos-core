-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace let_mut where
  struct P where
    a : u32

  fun reassigned() -> u32 := do
    let mut x := 1u32
    x := 2u32
    x

  fun borrowed(v : u32) -> u32 := do
    let mut x := 0u32
    let r := &mut x
    *r := v
    x

  fun counted(n : u32) -> u32 := do
    let mut i := 0u32
    while i < n do
      i := core.prim.checkedAddPanic(i, 1u32)
      continue
    i

  fun field_written(v : u32) -> u32 := do
    let mut p := new P { a := 1u32 }
    p.a := v
    p.a

  fun field_borrowed(v : u32) -> u32 := do
    let mut p := new P { a := 1u32 }
    let q := &mut p.a
    *q := v
    p.a

  fun borrowed_in_branch(c : Bool) -> u32 := do
    let mut x := 0u32
    if c then
      let r := &mut x
      *r := 2u32
    x

  fun one_binding(v : u32) -> u32 := do
    let _4 := (1u32, v)
    let mut a := _4.0
    let b := _4.1
    a := b
    a

  fun repointed(mut a : u32, mut b : u32, c : Bool) -> u32 := do
    let mut r := &mut a
    if c then
      let _5 := &mut b
      r := _5
    *r := 1u32
    a

  fun never_written(v : u32) -> u32 := do
    let y := (v, v)
    y.0

  fun assigned_once(c : Bool) -> u32 := do
    let mut x : u32
    if c then x := 1u32 else x := 2u32
    x
