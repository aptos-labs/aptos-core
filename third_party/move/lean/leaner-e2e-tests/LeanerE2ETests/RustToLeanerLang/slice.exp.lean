-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace slice where
  fun first(values : &Vector<u32>) -> u32 :=
    if values.length > 0usize then values[0usize] else panic()
