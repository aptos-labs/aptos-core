-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace const_item where
  fun below_limit(value : usize) -> Bool := value < 1024usize
