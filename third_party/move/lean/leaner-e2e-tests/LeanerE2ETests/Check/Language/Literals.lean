-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/Literals. LeanerLang deliberately distinguishes
`Bytes` from `Vector<u8>`, so vector-returning literals are written as typed
vectors. The byte-string abort message remains exercised as a `Bytes` local. -/

leaner module 0x42::literals where
  fun fixed_address() -> Address := @0xCAFE
  spec fixed_address where
    ensures result == @0xCAFE
    aborts_if false

  fun ascii_bytes() -> Vector<u8> := vector<u8>[77, 111, 118, 101]
  fun hex_bytes() -> Vector<u8> := vector<u8>[0xDE, 0xAD, 0]

  fun classify_bytes(value : Vector<u8>) -> u64 :=
    if value == vector<u8>[103, 111] then 1
    else if value == vector<u8>[0] then 2
    else 0
  spec classify_bytes where
    ensures value == vector<u8>[103, 111] ==> result == 1
    ensures value == vector<u8>[0] ==> result == 2
    ensures !(value == vector<u8>[103, 111]) &&
      !(value == vector<u8>[0]) ==> result == 0
    aborts_if false
  verify classify_bytes

  fun abort_message() -> Unit := do
    let message : Bytes := b[115, 111, 109, 101, 116, 104, 105, 110, 103,
      32, 119, 101, 110, 116, 32, 119, 114, 111, 110, 103]
    if message.length == 20 then abort(0xCA26CBD9BE0B0000) else abort(0)
  spec abort_message where
    ensures false
    aborts_if true with 0xCA26CBD9BE0B0000

set_option leaner.route "native" in
#leaner_verify 0x42::literals::fixed_address
#leaner_require_native 0x42::literals::fixed_address

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».literals #[
    ⟨"fixed_address", #[], .returned #[.address "0xCAFE"], {}⟩,
    ⟨"ascii_bytes", #[], .returned #[.vector #[.integer 77, .integer 111,
      .integer 118, .integer 101]], {}⟩,
    ⟨"hex_bytes", #[], .returned #[.vector #[.integer 0xDE, .integer 0xAD,
      .integer 0]], {}⟩,
    ⟨"classify_bytes", #[.vector #[.integer 103, .integer 111]],
      .returned #[.integer 1], {}⟩,
    ⟨"classify_bytes", #[.vector #[.integer 0]], .returned #[.integer 2], {}⟩,
    ⟨"classify_bytes", #[.vector #[]], .returned #[.integer 0], {}⟩,
    ⟨"abort_message", #[], .threw .abort #[.integer 0xCA26CBD9BE0B0000], {}⟩]
