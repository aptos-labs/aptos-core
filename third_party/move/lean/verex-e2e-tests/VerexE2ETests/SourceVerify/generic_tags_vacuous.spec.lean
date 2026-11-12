-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec push_only where
  ensures false -- error: the call has an outcome

spec recurse where
  ensures result == x

spec recurse_tag where
  ensures false -- error: the call has an outcome

spec recurse_own where
  ensures false -- error: the call has an outcome
