-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace return_in_nested_branch where
  -- The `return` closes a branch nested in a statement: its value is not the branch's.
  fun in_while(a : Bool, b : Bool, c : Bool) -> u64 := do
    let mut x := 0u64
    while c do
      if a then
        if b then break
        else
          x := 1u64
          let _0 := x
          return _0;
      else
        x := 2u64
        continue
    x

  -- The same with the `return` in a block of its own, folded with its value.
  fun after_a_branch(a : Bool, b : Bool, c : Bool, d : Bool) -> u64 := do
    let mut x := 0u64
    while c do
      if a then
        if b then break
        else
          if d then x := 1u64
          return x;
      else
        x := 2u64
        continue
    x

  fun in_loop(a : Bool, b : Bool) -> u64 := do
    let mut x := 0u64
    loop if a then
      if b then continue else break
    else
      x := 2u64
      continue
    x := core.prim.checkedAddPanic(x, 1u64)
    x
