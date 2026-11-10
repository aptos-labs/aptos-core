-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec value where
  ensures result != x

spec owned where
  ensures result != n

spec ends_holder where
  ensures result == a

spec ends_reference where
  ensures result == a

spec lets_writer_go where
  ensures result == 5
