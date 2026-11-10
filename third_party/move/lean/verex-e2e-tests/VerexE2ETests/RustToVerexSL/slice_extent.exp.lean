-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace slice_extent where
  fun set_first(values : &mut Slice<u32>, value : u32) -> Unit :=
    if values.length > 0usize then values[0usize] := value

  fun array_update() -> u32 := do
    let mut values : Vector<u32, const 2> := #[1u32, 2u32]
    set_first(&mut (unsize values), 9u32)
    values[0usize]

  fun array_length_after(values : &mut Vector<u32, const 2>) -> usize := do
    set_first(&mut (unsize *values), 9u32)
    (&(unsize *values)).length

  fun reborrow(values : &mut Slice<u32>) -> usize := do
    set_first(values, 9u32)
    values.length

  fun redirect {a : lifetime}(
    left : &[a] mut Slice<u32>, right : &[a] mut Slice<u32>
  ) -> &[a] mut Slice<u32> := do
    let mut selected := left
    selected := right
    &mut *selected

  fun sequential_reborrow(values : &mut Slice<u32>) -> usize := do
    set_first(values, 7u32)
    set_first(values, 9u32)
    values.length

  fun loop_reborrow(values : &mut Slice<u32>) -> usize := do
    let mut rounds := 0u32
    while rounds < 2u32 do
      set_first(values, 11u32)
      rounds := core.prim.checkedAddPanic(rounds, 1u32)
      continue
    values.length

  fun redirect_nested {a : lifetime}(
    selected : &[a] mut &[a] mut Slice<u32>,
    replacement : &[a] mut Slice<u32>
  ) -> Unit := *selected := replacement
