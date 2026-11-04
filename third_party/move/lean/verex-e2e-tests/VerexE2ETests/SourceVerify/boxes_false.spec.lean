-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec make where
  ensures result == value + 1 -- error: the box holds the value

spec read where
  ensures result == 0 -- error: the content is the argument

spec read_nested where
  ensures result == value + 1 -- error: the content is the argument

spec read_field where
  ensures result == value[0u32] -- error: another field

spec move_out where
  ensures result.length == 0 -- error: the content is the argument

spec move_guard where
  ensures result.0 == value.0 + 1 -- error: the content is the argument

spec move_part where
  ensures result.length == 0 -- error: the content's field

spec move_behind_flag where
  ensures result == value.length -- error: zero when the flag is unset

spec write where
  ensures value == old(value) -- error: written

spec write_field where
  ensures value[1u32][1u32] == old(value[1u32][1u32]) -- error: written

spec write_through where
  ensures value == old(value) -- error: written through the reborrow

spec write_owned where
  ensures result == value -- error: written

spec borrow_mut where
  ensures result == old(value) + 1 -- error: the content itself

spec view where
  ensures result == value + 1 -- error: the content itself

spec as_ref where
  ensures result == value + 1 -- error: the content itself

spec as_mut where
  ensures value == old(value) -- error: written

spec dropped where
  ensures result == 2 -- error: the result is one

spec dropped_holder where
  ensures result == 2 -- error: the result is one
