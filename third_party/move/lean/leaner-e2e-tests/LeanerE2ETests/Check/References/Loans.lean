-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Independent mutable loans

Vector append, composed reborrows,
and reconciliation of an indexed element loan beside a live mutable
parameter before moving its owner. `clear` is specified but not verified
(`pragma verify = false`).
-/

namespace LeanerLang.Tests.Check.References.Loans

leaner module 0x42::loans where
  struct Buffer has Copy, Drop, Store where
    bytes : Vector<u8>

  struct Bits has Copy, Drop, Store where
    length : u64
    bit_field : Vector<Bool>

  fun extend(self : &mut Vector<u8>, value : u8) -> Unit :=
    *self := core.prim.pushVector(*self, value)
  spec extend where
    requires self.length < MAX_U64
    ensures self == core.prim.pushVector(old(self), value)
    aborts_if false

  fun splice(self : &mut Buffer, a : u8, b : u8) -> Unit := do
    let mut front : Vector<u8> := vector<u8>[]
    let r := &mut front
    extend(r, a)
    extend(r, b)
    let bytes := *r
    *self := new Buffer { bytes }
  spec splice where
    ensures self.bytes == vector<u8>[a, b]
    aborts_if false

  fun independent_element(self : &mut Buffer) -> Unit := do
    let mut values : Vector<u8> := vector<u8>[0]
    let element := &mut values[0]
    *element := 7
    *self := new Buffer { bytes := values }
  spec independent_element where
    ensures self.bytes == vector<u8>[7]
    aborts_if false

  fun clear(self : &mut Bits) -> Unit := do
    let field := &self.bit_field
    let len := field.length
    let mut i : u64 := 0
    loop
      if !(i < len) then break
      self.bit_field[i] := false
      i := i + 1
  spec clear where
    pragma verify = false
    ensures self.length == old(self).length
    aborts_if false

end LeanerLang.Tests.Check.References.Loans
