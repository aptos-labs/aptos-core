-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Vectors in specification functions. A specification function compares a
vector with a literal or with another vector through the vector's encoding;
the clause applying it is decided against the function's typed vector. A
quantifier over a short range with literal bounds is decided position by
position. -/

leaner module 0x4A::encoded_vectors where
  struct Digest has Copy, Drop where
    data : Vector<u8>

  spec fun is_zero(b : Digest) : Bool := b.data == #[0, 0, 0, 0]

  spec fun same(a : Digest, b : Digest) : Bool := a.data == b.data

  fun zero() -> Digest := new Digest { data := #[0, 0, 0, 0] }

  spec zero where
    ensures is_zero(result)

  fun check_zero(b : &Digest) -> Bool := b.data == #[0, 0, 0, 0]

  spec check_zero where
    ensures result == is_zero(b)

  fun equal(a : &Digest, b : &Digest) -> Bool := a.data == b.data

  spec equal where
    aborts_if false
    ensures result == same(a, b)

  fun from_byte(byte : u8) -> Digest := do
    let b := zero()
    b.data[0] := byte
    b

  spec from_byte where
    aborts_if false
    ensures result.data[0] == byte
    ensures ∀ (i in 1 .. result.data.length), result.data[i] == 0
