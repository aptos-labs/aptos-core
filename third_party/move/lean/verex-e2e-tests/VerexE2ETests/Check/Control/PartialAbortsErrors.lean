-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- Partial mode changes abort coverage only: strict functions still cannot
-- abort, postconditions still hold on return, sufficient conditions still
-- force aborts, and declared abort codes still constrain failures.
verex module 0x99::partial_aborts_errors where
  pragma aborts_if_is_strict
  pragma verify = false

  public fun strict(value : u64) -> u64 := value + 1

  public fun wrong_result(value : u64) -> u64 := value
  spec wrong_result where
    pragma aborts_if_is_partial
    ensures result == value + 1

  public fun missing_abort(value : u64) -> u64 := value
  spec missing_abort where
    pragma aborts_if_is_partial
    aborts_if value == 0

  public fun wrong_code() -> u64 := abort(8)
  spec wrong_code where
    pragma aborts_if_is_partial
    aborts_with 7

verify 0x99::partial_aborts_errors::strict
verify 0x99::partial_aborts_errors::wrong_result
verify 0x99::partial_aborts_errors::missing_abort
verify 0x99::partial_aborts_errors::wrong_code
