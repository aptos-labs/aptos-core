-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace drop_flags where
  -- A value moved on some paths only is dropped under a flag that records
  -- whether it is still owned.
  fun pick {T}(first : Bool, left : T, right : T) -> T := do
    let mut _5 := false
    let mut _4 := false
    _5 := true
    _4 := true
    let mut _0 : T
    if first then
      _5 := false
      _0 := left
    else
      _4 := false
      _0 := right
    if _4 then drop(right)
    if _5 then drop(left)
    _0

  fun keep_first {T}(values : (T, T), swap : Bool) -> T := do
    let mut _6 := false
    let mut _5 := false
    _6 := true
    let left := move(values.0)
    _5 := true
    let right := move(values.1)
    let mut _0 : T
    if swap then
      _5 := false
      _0 := right
    else
      _6 := false
      _0 := left
    if _5 then drop(right)
    _5 := false
    if _6 then drop(left)
    _6 := false
    _0

  fun replace_once {T}(count : u32, initial : T, next : T) -> T := do
    let mut _12 := false
    _12 := true
    let mut result := initial
    let mut index := 0u32
    while index < count do
      if index == 1u32 then
        _12 := false
        drop(result)
        result := next
        break
      else
        index := core.prim.checkedAddPanic(index, 1u32)
        continue
    if _12 then drop(next)
    result
