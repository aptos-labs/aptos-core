-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec insert_at where
  panics_if at >= old(values).length -- error: inserting at the end is allowed
  ensures values[at] == value

spec remove_at where
  panics_if at > old(values).length -- error: removing at the end panics

spec keep_two where
  ensures values.length == 2 -- error: not if it was shorter

spec clear_all where
  ensures values == old(values) -- error: it is emptied

spec whole where
  ensures result.length == 0 -- error: it is the whole vector

spec append where
  ensures values.length == old(values).length -- error: it grows

spec filled where
  ensures ∀ (k in 0 .. count), result[k] == 8 -- error: it is 7
