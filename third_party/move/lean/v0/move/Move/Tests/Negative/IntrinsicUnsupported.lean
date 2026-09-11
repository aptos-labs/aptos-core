-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: negative.

import Move

/-! Intrinsic attributes fail explicitly while their semantics await unified LIR. -/

namespace Tests.Negative.IntrinsicUnsupported

open Move
open scoped Move Move.Compiler

/-- error: intrinsic declarations are temporarily unsupported; support is suspended until intrinsic validation is implemented on the unified LIR -/
#guard_msgs in
module Owner where

  @[intrinsic_map]
  struct Table (K V) where
    marker : Bool

/-- error: intrinsic declarations are temporarily unsupported; support is suspended until intrinsic validation is implemented on the unified LIR -/
#guard_msgs in
module Role where

  struct Table (K V) where
    marker : Bool

  @[map_spec_get (Table)]
  spec opaque spec_get {K} {V} (map : Table K V) (key : K) : V

end Tests.Negative.IntrinsicUnsupported
