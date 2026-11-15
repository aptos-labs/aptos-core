-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A cast that wraps is not its operand.

spec truncate where
  ensures result == a

spec sign_extend where
  ensures result == a
