-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! A generic opaque callee returns a concrete enum. Its contract's field read
uses the callee's instantiated carrier family; the caller's arithmetic reads
that same field at the outer family. Normalize the payload and its encoding
before arithmetic, without unfolding the opaque callee. -/

leaner module 0x42::generic_cursor where
  enum Cursor has Copy, Drop where
    | End
    | Position (index : u64)

  fun locate {K}(keys : Vector<K>) -> Cursor := new Cursor::End {}
  spec locate where
    pragma opaque
    aborts_if false
    ensures !(result is End) ==> result.index < keys.length

  fun caller(keys : Vector<u64>) -> Cursor := locate(keys)
  spec caller where
    aborts_if false
    ensures !(result is End) ==> result.index + 1 <= keys.length
