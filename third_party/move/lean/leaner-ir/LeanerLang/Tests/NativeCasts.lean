-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option leaner.route "native"
-- Lean's option is measured in thousands of raw heartbeats: 5M here.
set_option maxHeartbeats 5000
set_option leaner.verifyHeartbeats 5000

leaner module 0x42::native_casts where
  fun narrow(value : u64) -> u8 := value as u8
  spec narrow where
    ensures result == value
    aborts_if value > 255
  verify narrow

  fun widen(value : u8) -> u256 := value as u256
  spec widen where
    ensures result == value
    aborts_if false
  verify widen

  fun signed_to_unsigned(value : i16) -> u8 := value as u8
  spec signed_to_unsigned where
    ensures result == value
    aborts_if value < 0 || value > 255
  verify signed_to_unsigned
