-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec value where
  ensures result == x

spec owned where
  ensures result == n

spec ends_holder where
  ensures result == a % 100 + 1

spec ends_reference where
  ensures result == a % 100 + 2

spec lets_writer_go where
  ensures result == 6
