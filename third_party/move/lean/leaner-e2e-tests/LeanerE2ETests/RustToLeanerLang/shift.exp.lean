-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace shift where
  fun shifts(value : u32, distance : u8) -> (u32, u32) :=
    if distance < 32u8 then
      if distance < 32u8 then (value << distance, value >> distance)
      else panic()
    else panic()
