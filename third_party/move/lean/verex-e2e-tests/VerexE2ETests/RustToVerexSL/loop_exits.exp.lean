-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace loop_exits where
  fun first_index(values : &Slice<u32>, target : u32) -> u32 := do
    let mut index := 0u32
    while values.length > index as usize do
      let _10 := index as usize
      if values[_10] == target then
        return index;
      else
        index := core.prim.checkedAddPanic(index, 1u32)
        continue
    index

  fun two_returns(values : &Slice<u32>) -> u32 := do
    let mut index := 0
    while values.length > index as usize do
      let _9 := index as usize
      let mut _0 : u32
      if values[_9] == 0u32 then _0 := 0u32
      else
        let _15 := index as usize
        if values[_15] == 1u32 then
          return 1u32;
        else
          index := core.prim.checkedAddPanic(index, 1)
          continue
      return _0;
    2u32

  fun nested(rows : u32, columns : u32) -> u32 := do
    let mut row := 0u32
    while row < rows do
      let mut column := 0u32
      while column < columns do
        if row == column then
          return row;
        else
          column := core.prim.checkedAddPanic(column, 1u32)
          continue
      row := core.prim.checkedAddPanic(row, 1u32)
      continue
    rows

  fun break_outer(rows : u32) -> u32 := do
    let mut row := 0u32
    loop@l0 if row < rows then
      let mut column := 0u32
      while column < rows do
        if column > row then break@l0
        else
          column := core.prim.checkedAddPanic(column, 1u32)
          continue
      row := core.prim.checkedAddPanic(row, 1u32)
      continue
    else break
    row

  fun skip_rows(rows : u32, columns : u32) -> u32 := do
    let mut count := 0u32
    let mut row := 0u32
    loop@l0 if row < rows then
      row := core.prim.checkedAddPanic(row, 1u32)
      let mut column := 0u32
      while column < columns do
        column := core.prim.checkedAddPanic(column, 1u32)
        if column == row then continue@l0
        else
          count := core.prim.checkedAddPanic(count, 1u32)
          continue
      count := core.prim.checkedAddPanic(count, 2u32)
      continue
    else break
    count

  fun classify(values : &Slice<u32>, limit : u32) -> u32 := do
    let mut index := 0
    let mut kind : u32
    loop if values.length <= index as usize then
      kind := 0u32
      break
    else
      let _11 := index as usize
      if values[_11] > limit then
        kind := 1u32
        break
      else
        index := core.prim.checkedAddPanic(index, 1)
        continue
    let mut total := kind
    while total < limit do
      total := core.prim.checkedAddPanic(total, 1u32)
      continue
    total
