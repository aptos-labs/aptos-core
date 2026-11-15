-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Contracts the code does not meet: each failure is reported at the clause in
-- this file.

spec reassigned where
  ensures result == 1

spec borrowed where
  ensures result == 0

spec field_borrowed where
  ensures result == 1

spec in_branch where
  ensures result == v
