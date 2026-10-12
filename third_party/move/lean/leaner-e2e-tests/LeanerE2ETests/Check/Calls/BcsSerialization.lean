-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# BCS serialization

What the Move Prover's prelude states of `bcs::serialize` holds of the values
a function takes where it serializes: the serialization is injective and
never empty, and every address has one length. Values of one type other than
addresses may serialize to different lengths.
-/

namespace LeanerLang.Tests.Check.Calls.BcsSerialization

leaner module std::bcs where
  public native fun to_bytes {MoveValue}(v : &MoveValue) -> Vector<u8>

  opaque spec fun serialize {MoveValue}(v : MoveValue) : Vector<u8>

leaner module 0x49::serialization where
  use std::bcs::to_bytes

  struct Pair has Copy, Drop where
    left : u64
    right : Bool

  fun same_bytes(a : &Pair, b : &Pair) -> Bool := to_bytes(a) == to_bytes(b)

  spec same_bytes where
    ensures result <==> a == b

  fun nonempty {T}(value : &T) -> u64 := to_bytes(value).length

  spec nonempty where
    aborts_if false
    ensures result > 0

  fun address_lengths(a : &Address, b : &Address) -> Bool :=
    to_bytes(a).length == to_bytes(b).length

  spec address_lengths where
    ensures result

  fun value_lengths {T}(a : &T, b : &T) -> Bool :=
    to_bytes(a).length == to_bytes(b).length

  spec value_lengths where
    ensures result -- error: values of one type may serialize to different lengths

end LeanerLang.Tests.Check.Calls.BcsSerialization
