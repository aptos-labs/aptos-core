-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec

/-!
# N1: progress of the semantics

UNTRUSTED (checked by the kernel). Every state has a step whenever natives
always produce some result. The deterministic core is a total function, so
the only way to be stuck would be a native call with no result. There is no
silently stuck state, so `NoWrong` cannot hold vacuously.
-/

namespace MoveBytecode.Proofs

theorem progress (P : Program) (N : NativeRel) : Progress P N := by
  intro hN s
  cases h : step P s with
  | done o => exact ⟨o, .core h⟩
  | callNative s' f args views =>
    obtain ⟨r, hr⟩ := hN f views
    exact ⟨_, .native h hr⟩

end MoveBytecode.Proofs
