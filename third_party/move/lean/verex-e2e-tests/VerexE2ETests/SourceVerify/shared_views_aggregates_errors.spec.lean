-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec option_final where
  ensures o is Some ==> final((o.0).r) == 0

spec struct_final where
  ensures final(v.h.r) == 0
