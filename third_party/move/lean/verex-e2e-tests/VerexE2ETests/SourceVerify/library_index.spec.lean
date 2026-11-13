-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec element where
  panics_if position >= values.length
  ensures result == values[position]

spec first_pushed where
  ensures result == value

spec reference where
  panics_if position >= values.length
  ensures result == values[position]

spec lookup where
  ensures position < values.length ==> result == values[position]
  ensures position >= values.length ==> result == 0

spec found where
  ensures result == (position < values.length)
