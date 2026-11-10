-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace unsize where
  fun len_of(s : &Slice<u32>) -> usize := s.length

  fun write_first(s : &mut Slice<u32>) -> Unit := s[0usize] := 7u32

  fun shared_local(a : Vector<u32, const 3>) -> usize := len_of(&(unsize a))

  fun shared_reference(a : &Vector<u32, const 3>) -> usize :=
    len_of(&(unsize *a))

  fun mutable_local(mut a : Vector<u32, const 3>) -> u32 := do
    write_first(&mut (unsize a))
    a[0usize]

  fun mutable_reference(a : &mut Vector<u32, const 3>) -> Unit := do
    write_first(&mut (unsize *a))

  fun field_place(p : &mut (Vector<u32, const 2>, u32)) -> Unit := do
    write_first(&mut (unsize p.0))
