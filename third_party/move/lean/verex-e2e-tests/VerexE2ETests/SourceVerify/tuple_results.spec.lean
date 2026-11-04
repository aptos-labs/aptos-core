-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec swapped where
  ensures spec.result[0] == b && spec.result[1] == a
  ensures result == (b, a)

spec same where
  ensures result == t

spec first where
  ensures result == t[0]

spec nested where
  ensures result == t[0][1]

spec written where
  ensures t[0] == 1 && t[1] == old(t[1])
