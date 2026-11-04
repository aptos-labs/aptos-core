-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec make where
  ensures result == value

spec read where
  ensures result == value

spec read_nested where
  ensures result == value

spec read_field where
  ensures result == value[1u32][0u32]

spec move_out where
  ensures result == value

spec move_guard where
  ensures result.0 == value.0

spec move_part where
  ensures result == value[0u32]

spec move_behind_flag where
  ensures flag ==> result == value.length
  ensures !flag ==> result == 0

spec write where
  ensures value == other

spec write_field where
  ensures value[1u32][1u32] == other
  ensures value[0u32] == old(value[0u32])

spec write_through where
  ensures value == other

spec write_owned where
  ensures result == other

spec borrow_mut where
  ensures result == old(value)
  ensures value == final(result)

spec view where
  ensures result == value

spec as_ref where
  ensures result == value

spec as_mut where
  ensures value == other

spec dropped where
  ensures result == 1

spec dropped_holder where
  ensures result == 1
