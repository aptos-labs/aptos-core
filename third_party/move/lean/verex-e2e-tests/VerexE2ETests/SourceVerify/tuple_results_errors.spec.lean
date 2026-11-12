-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Contracts the code does not meet: each failure is reported at the clause in
-- this file.

spec swapped where
  ensures result == (a, b)

spec same where
  ensures spec.result[0] == spec.result[1]

spec first where
  ensures result == t[1]

spec nested where
  ensures result == t[1]

spec written where
  ensures t[0] == old(t[0])
