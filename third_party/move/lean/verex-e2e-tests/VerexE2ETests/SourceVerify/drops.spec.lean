-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec pick where
  ensures first ==> result == left
  ensures !first ==> result == right

spec keep_first where
  ensures true

spec replace_once where
  ensures count <= 1 ==> result == initial
  ensures count > 1 ==> result == next
  loop 1 where
    invariant index <= 1
    invariant index <= count
    invariant result == initial

spec forget where
  ensures true

spec forget_row where
  ensures true

spec forget_holder where
  ensures true

spec second_of_holders where
  ensures result == second

spec pick_number where
  ensures first ==> result == left
  ensures !first ==> result == right

spec pick_reference where
  ensures first ==> result == left
  ensures !first ==> result == right
