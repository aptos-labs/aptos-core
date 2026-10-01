-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! A native without a specification that the Move Prover's prelude models
is read by that model: `hash::sha3_256` is an uninterpreted function of its
input with a 32-byte result, and does not abort. A native with neither a
specification nor a model is rejected, as the Prover rejects it. -/

leaner module std::hash where
  public native fun sha3_256(data : Vector<u8>) -> Vector<u8>

leaner module 0x49::digests where
  use std::hash::sha3_256

  public fun key(bytes : Vector<u8>) -> Vector<u8> := sha3_256(bytes)

  spec key where
    aborts_if false
    ensures result == sha3_256(bytes)
    ensures result.length == 32

  public fun stable(bytes : Vector<u8>) -> Bool := sha3_256(bytes) == sha3_256(bytes)

  spec stable where
    aborts_if false
    ensures result == true
