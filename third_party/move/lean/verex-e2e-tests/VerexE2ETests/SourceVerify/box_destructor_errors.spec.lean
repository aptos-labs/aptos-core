-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec whole where
  ensures result == 1

spec read_then_drop where
  ensures result == value.0

spec in_a_field where
  ensures result == 1

spec behind_flag where
  ensures flag ==> result is Some

spec part_behind_flag where
  ensures !flag ==> result == 0
