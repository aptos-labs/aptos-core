-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec push_only where
  ensures tags.length == old(tags).length -- error: the push adds an element

spec recurse where
  ensures result == x

spec recurse_tag where
  ensures result == v + 1 -- error: the callee returns its argument

spec recurse_own where
  ensures result != x -- error: the recursion returns its argument
