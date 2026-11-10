-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A shared view of a value holding a mutable reference.

impl Holder where
  spec get where
    ensures result == self.r

  spec add where
    ensures self.r == old(self).r % 100 + d % 100

  spec bump where
    ensures result == old(self).r
    ensures self.r == old(self).r % 100 + 1

impl VecHolder where
  spec len where
    ensures result == self.v.length

  spec push_len where
    ensures self.v.length == old(self).v.length + 1

spec read_view where
  ensures result == h.r

spec idx_len_field where
  ensures result == (i < h.v.length)

spec idx_len_field_false where
  ensures result == true -- error: not when i is past the end

spec idx_len_field_vacuous where
  ensures false -- error: the paths are not vacuous

spec view_of_ref where
  ensures result == a % 100 + 1

spec view_of_ref_false where
  ensures result == a -- error: written after the view

spec view_of_ref_vacuous where
  ensures false -- error: the paths are not vacuous

spec double_ref where
  ensures result == m

spec double_ref_false where
  ensures result == m + 1 -- error: it reads the referent

spec double_ref_vacuous where
  ensures false -- error: the paths are not vacuous

spec view_callee where
  ensures result == a % 100 + 1

spec view_callee_false where
  ensures result == a -- error: written after the view ends

spec view_callee_vacuous where
  ensures false -- error: the paths are not vacuous

spec two_phase where
  ensures result == a % 100 + a % 100

spec two_phase_false where
  ensures result == a -- error: added to itself

spec two_phase_vacuous where
  ensures false -- error: the paths are not vacuous

spec method_calls_view where
  ensures spec.result[0] == a
  ensures spec.result[1] == a % 100 + 1

spec method_calls_view_false where
  ensures spec.result[1] == a -- error: written by the method

spec method_calls_view_vacuous where
  ensures false -- error: the paths are not vacuous

spec view_misses_later_write where
  ensures result == a

spec view_misses_later_write_false where
  ensures result == 7 -- error: the view ended before the write

spec view_misses_later_write_vacuous where
  ensures false -- error: the paths are not vacuous

spec vec_len_holder where
  ensures result == old(v).length

spec vec_len_holder_false where
  ensures result == 0 -- error: any length

spec vec_len_holder_vacuous where
  ensures false -- error: the paths are not vacuous

spec vec_push_len where
  ensures v.length == old(v).length + 1

spec vec_push_len_false where
  ensures v.length == old(v).length -- error: it pushes

spec vec_push_len_vacuous where
  ensures false -- error: the paths are not vacuous

spec read_unit where
  ensures true

spec unit_under_view where
  ensures result == a

spec unit_under_view_false where
  ensures result == a + 1 -- error: it returns its argument

spec unit_under_view_vacuous where
  ensures false -- error: the paths are not vacuous

spec generic_at_unit where
  ensures result == a

spec generic_at_unit_false where
  ensures result == a + 1 -- error: it returns its argument

spec generic_at_unit_vacuous where
  ensures false -- error: the paths are not vacuous

spec generic_at_reference where
  ensures result == a % 100 + 1

spec generic_at_reference_false where
  ensures result == a -- error: written after the view

spec generic_at_reference_vacuous where
  ensures false -- error: the paths are not vacuous
