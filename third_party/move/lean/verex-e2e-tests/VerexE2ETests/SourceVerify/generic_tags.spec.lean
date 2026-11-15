-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec push_only where
  ensures tags.length == old(tags).length + 1

spec recurse where
  ensures result == x

spec recurse_tag where
  ensures result == v
