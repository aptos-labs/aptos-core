-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace generic_adt where
  struct Wrapper {T} where
    value : T

  fun unwrap_u32(value : Wrapper<u32>) -> u32 := value.value
