-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Calls into other modules

A module using others links them into its unit, so a caller reasons about a
callee of another module as about its own: through its body, unless the
callee is `opaque`. Functions of different modules may share a name.
-/

namespace LeanerLang.Tests.Check.Calls.Modules

leaner module 0x46::codes where
  struct Box has Copy, Drop where
    value : u64

  public fun code(c : u64) -> u64 := c + 1

  public fun wrap(v : u64) -> Box := new Box { value := v }

  public fun canon(c : u64) -> u64 := c + 2
  spec canon where
    pragma opaque
    ensures result == c + 2

-- `code` here calls the other module's `code`, and an opaque callee.
leaner module 0x46::middle where
  use 0x46::codes

  public fun code(x : u64) -> u64 := codes::code(x) + codes::canon(x)
  spec code where
    ensures result == 2 * x + 3

-- The module `codes` is reached both directly and through `middle`.
leaner module 0x46::top where
  use 0x46::middle
  use 0x46::codes
  use 0x46::codes::Box

  public fun through(x : u64) -> u64 := middle::code(x)
  spec through where
    ensures result == 2 * x + 3

  public fun boxed(x : u64) -> Box := codes::wrap(x)
  spec boxed where
    ensures result.value == x

  public fun wrong(x : u64) -> u64 := codes::code(x)
  spec wrong where
    ensures result == x -- error: the callee adds one

end LeanerLang.Tests.Check.Calls.Modules
