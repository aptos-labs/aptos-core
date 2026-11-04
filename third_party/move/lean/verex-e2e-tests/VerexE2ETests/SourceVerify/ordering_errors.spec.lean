-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec comparator where
  aborts_if false
  ensures result == (a <= b)

spec order where
  aborts_if false
  ensures a < b ==> result == 1
