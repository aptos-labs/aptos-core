-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A reference the caller lends, overwritten through a local it moved to: rejected,
-- since no loan of the frame ends it and nothing resolves its lender's prophecy.

spec caller where
  ensures spec.result[1] == 9
  ensures spec.result[0] == old(a)
  ensures spec.result[0] == 7
  ensures false
