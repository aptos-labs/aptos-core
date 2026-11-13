-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace moved_reference where
  struct Holder {a : lifetime} where
    r : &[a] mut u32

  fun into_tuple(mut a : u32, mut b : u32) -> u32 := do
    let _6 := &mut a
    let _5 := (_6, &mut b)
    let p := move(_5.0)
    let q := move(_5.1)
    *p := 1u32
    *q := 2u32
    a

  fun into_field {a : lifetime}(
    h : &[a] mut Holder<lifetime a>, y : &[a] mut u32
  ) -> Unit := (*h).r := y

  fun element(v : &mut Slice<u32>, k : usize) -> &mut u32 := &mut v[k]

  fun written(v : &mut Slice<u32>, k : usize) -> Unit := do
    let e := &mut v[k]
    *e := 1u32
