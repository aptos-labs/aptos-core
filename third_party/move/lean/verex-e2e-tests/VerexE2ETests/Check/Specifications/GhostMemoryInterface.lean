-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! A spec variable's memory stays one in the interface an importing module
checks against: its abilities ask nothing of its contents there either. -/

verex module 0x48::ghost_holder where
  struct Data has Key where
    x : u64

  struct «Ghost$ghost_data» has Copy, Drop, Store, Key where
    v : Data

  spec «Ghost$ghost_data» where
    pragma ghost_memory

  public fun read(a : Address) -> u64 := do
    let data := &Data[a]
    spec update global<«Ghost$ghost_data»>(@0x0).v := data
    data.x

  spec read where
    aborts_if !exists<Data>(a)
    ensures global<«Ghost$ghost_data»>(@0x0).v == global<Data>(a)
    modifies global<«Ghost$ghost_data»>(@0x0)

verex module 0x48::ghost_reader where
  use 0x48::ghost_holder::Data
  use 0x48::ghost_holder::read
  use 0x48::ghost_holder::«Ghost$ghost_data»

  fun forward(a : Address) -> u64 := read(a)

  spec forward where
    aborts_if !exists<Data>(a)
    modifies global<«Ghost$ghost_data»>(@0x0)
