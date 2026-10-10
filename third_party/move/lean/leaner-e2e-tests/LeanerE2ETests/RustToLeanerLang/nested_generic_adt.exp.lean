-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace nested_generic_adt where
  struct Wrapper {T} where
    value : T

  struct Outer {T} where
    inner : Wrapper<T>

  fun read_u32(value : Outer<u32>) -> u32 := value.inner.value
