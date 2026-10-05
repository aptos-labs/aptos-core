-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Fields pushed into vectors, in the logical projection

A function's body is also read as its logical meaning, where integers are
mathematical. A field pushed into a vector keeps its physical type there, as
the vector's elements do, and a vector a specification builds for a field
takes the field's element type.
-/

namespace LeanerLang.Tests.Check.Vectors.PhysicalElements

leaner module 0x42::physical_elements where
  struct Buffer has Copy, Drop, Store where
    bytes : Vector<u8>

  struct Cell has Copy, Drop, Store where
    byte : u8

  public fun push_byte(target : &mut Buffer, value : Cell) -> Unit :=
    target.bytes := core.prim.pushVector(target.bytes, value.byte)
  spec push_byte where
    ensures target.bytes.length == old(target.bytes).length + 1

  public fun pushed(items : Vector<u8>, value : Cell) -> Vector<u8> :=
    core.prim.pushVector(items, value.byte)
  spec pushed where
    ensures result.length == items.length + 1

  fun make() -> Buffer := new Buffer { bytes := vector<u8>[1, 2] }
  spec make where
    let_pre expected := new Buffer { bytes := concat(vec(1), vec(2)) }
    ensures result == expected

end LeanerLang.Tests.Check.Vectors.PhysicalElements
