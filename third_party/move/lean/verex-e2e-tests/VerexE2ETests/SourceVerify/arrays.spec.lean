-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec get where
  panics_if index >= 4
  ensures result == values[index]

spec get_third where
  ensures result == values[2]

spec first_of_pair where
  ensures result == left

spec repeated where
  ensures result == value

spec set_first where
  ensures result == value
