-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec reassigned where
  ensures result == v

spec borrowed where
  ensures result == v

spec field_borrowed where
  ensures result == v

spec in_branch where
  ensures c ==> result == v
  ensures !c ==> result == 0
