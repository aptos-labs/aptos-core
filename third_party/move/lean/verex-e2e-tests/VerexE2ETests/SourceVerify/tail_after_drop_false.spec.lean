-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec tail_cast where
  ensures result == tags.length + 1 -- error: off by one

spec tail_truncate where
  ensures result == 1 -- error: not every length
