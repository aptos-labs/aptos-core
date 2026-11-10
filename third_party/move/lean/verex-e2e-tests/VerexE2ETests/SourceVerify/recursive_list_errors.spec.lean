-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- count and count_false have List parameters; the diagnostic names the recursive
-- type ("recursion through `List`") rather than the generic "type of a local".

spec count where
  ensures result == 0 -- error: recursion through List

spec count_false where
  ensures result == 1 -- error: recursion through List
