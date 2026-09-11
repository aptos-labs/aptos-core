-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace generic_const_adt where
  struct Tagged {T} {N : const usize} {ENABLED : const Bool} where
    value : T

  fun read_u32(value : Tagged<u32, const 3, const true>) -> u32 := value.value
