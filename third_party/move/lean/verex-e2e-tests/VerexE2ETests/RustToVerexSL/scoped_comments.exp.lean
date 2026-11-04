-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace scoped_comments where
  mod m where
    -- Before the module.
    /--
    /// The module's first function.
    -/
    fun one() -> u32 := 1u32

    -- Between the functions of the module.
    fun two() -> u32 := 2u32

  -- In a body.
  /--
  /// A counter.
  -/
  struct Counter where
    value : u32

  impl Counter where
    -- Before a method.
    fun get(self : Counter) -> u32 := self.value

  -- Before the root's function.
  fun root() -> u32 := m::one()
