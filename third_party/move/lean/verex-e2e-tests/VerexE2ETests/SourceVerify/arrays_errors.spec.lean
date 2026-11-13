-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- An index past the array's length panics, and the element read is not the other.

spec get where
  ensures result == values[index]

spec first_of_pair where
  ensures result == right
