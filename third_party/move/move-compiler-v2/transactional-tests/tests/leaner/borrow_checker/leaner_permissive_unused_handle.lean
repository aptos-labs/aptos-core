-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowPermissiveUnused where
  -- Leaner permits overlapping handles when the competing handle dies unused
  -- before the selected handle is written.
  fun run() -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let _competing := &mut owner
    *selected := 5
    let result := *selected
    result

--# run 0x0::LeanerBorrowPermissiveUnused::run
