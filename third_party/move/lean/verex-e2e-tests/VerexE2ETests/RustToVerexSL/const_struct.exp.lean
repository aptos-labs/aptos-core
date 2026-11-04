-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace const_struct where
  struct Point where
    x : u32
    flag : Bool

  fun origin() -> Point := new Point { x := 3u32, flag := true }
