-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
Utility for converting a Move value to its binary representation in BCS (Binary Canonical
Serialization). BCS is the binary encoding for Move resources and other non-module values
published on-chain. See https://github.com/aptos-labs/bcs#binary-canonical-serialization-bcs for more
details on BCS.
-/
leaner module 0x1::bcs where
  use 0x1::std::option::Option

  /--
  Returns the binary representation of `v` in BCS (Binary Canonical Serialization) format.
  Aborts with `0x1c5` error code if serialization fails.
  -/
  public native fun to_bytes {MoveValue}(v : &MoveValue) -> Vector<u8>

  /--
  Returns the size of the binary representation of `v` in BCS (Binary Canonical Serialization) format.
  Aborts with `0x1c5` error code if there is a failure when calculating serialized size.
  -/
  public native fun serialized_size {MoveValue}(v : &MoveValue) -> u64

  spec serialized_size where
    pragma opaque
    aborts_if [abstract] false
    ensures result == serialize(v).length

  /--
  If the type has known constant (always the same, independent of instance) serialized size
  in BCS (Binary Canonical Serialization) format, returns it, otherwise returns None.
  Aborts with `0x1c5` error code if there is a failure when calculating serialized size.

  Note:
  For some types it might not be known they have constant size, and function might return None.
  For example, signer appears to have constant size, but it's size might change.
  If this function returned Some() for some type before - it is guaranteed to continue returning Some().
  On the other hand, if function has returned None for some type,
  it might change in the future to return Some() instead, if size becomes "known".
  -/
  public native fun constant_serialized_size {MoveValue}() -> Option<u64>

  spec constant_serialized_size where
    pragma opaque
    aborts_if [abstract] false

  -- ==============================
  -- Module Specification
  -- switch to module documentation context
  /--
  Native function which is defined in the prover's prelude.
  -/
  opaque spec fun serialize {MoveValue}(v : MoveValue) : Vector<u8>

  -- in practice, this function should never abort
  -- Safely assumed to never abort: the native only aborts on type-layout
  -- construction limits, unreachable for realistic types (same rationale
  -- as `serialized_size` above). The result is left unconstrained; a
  -- precise per-type model would need prover-backend support.
