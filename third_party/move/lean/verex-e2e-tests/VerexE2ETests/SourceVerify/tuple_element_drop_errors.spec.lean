-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec tuple_element_drop where
  ensures true

spec tuple_element_drop_false where
  ensures false -- error: test

spec tuple_element_drop_vacuous where
  ensures false -- error: test

spec vec_in_tuple_drop where
  ensures true

spec vec_in_tuple_drop_false where
  ensures false -- error: test
