-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace string_length where
  fun byte_length(value : &string) -> usize := value.length
