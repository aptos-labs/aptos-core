-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec store where
  requires position < values.length
  ensures values == old(values)

spec unguarded where
  ensures values.length == old(values).length

spec other_position where
  requires position < values.length
  ensures values[0] == old(values)[0]
