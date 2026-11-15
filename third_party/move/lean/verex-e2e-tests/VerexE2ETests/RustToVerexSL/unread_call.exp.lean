-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace unread_call where
  struct Counter where
    value : u32

  fun reset(counter : &mut Counter) -> Unit := (*counter).value := 0u32

  fun reset_twice(counter : &mut Counter) -> Unit := do
    reset(counter)
    reset(counter)
