-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec head where
  ensures result is Some -- error: not of an empty slice

spec tail where
  ensures values.length > 0 ==> result is Some && result.0 == values[0] -- error: it is the last

spec parts where
  requires values.length <= 1000
  panics_if mid >= values.length -- error: splitting at the end is allowed

spec flip where
  ensures values == old(values) -- error: it is reversed

spec has where
  ensures result -- error: not of an absent value

spec vec_head where
  ensures values.length > 0 ==> result is None -- error: it has one
