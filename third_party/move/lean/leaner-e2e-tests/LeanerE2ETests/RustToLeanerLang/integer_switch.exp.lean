-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace integer_switch where
  fun select(tag : u32, zero : u32, one : u32, fallback : u32) -> u32 :=
    match tag with
      | 0 => zero
      | 1 => one
      | _ => fallback
