-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A refused function is an error at what refused it, and every other function verifies or
-- fails on its own. A caller gets only a contract the author states, never a body's meaning.

spec through_dyn where
  ensures result == s

spec calls_without_contract where
  ensures result == a -- error: a call of a refused function without a contract

spec calls_with_contract where
  ensures result == a

spec through_closure where
  ensures result == x -- error: the closure is refused, so its call has no meaning

spec plain where
  ensures result == a / 2
