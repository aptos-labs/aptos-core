-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Cached verification must pass the same native-artifact audit as a fresh
denotation proof. A same-name declaration is not a verification certificate. -/

namespace LeanerLang.Tests.DenoteCache

set_option Elab.async false

leaner module 0x42::denote_cache where
  fun valid(value : u64) -> u64 := value
  spec valid where
    ensures result == value
    aborts_if false
  verify valid

  fun forged(value : u64) -> u64 := value
  spec forged where
    ensures result == value
    aborts_if false

-- Genuine certificates remain reusable, including inside a Lean namespace.
#leaner_verify 0x42::denote_cache::valid
#leaner_require_native 0x42::denote_cache::valid

theorem «0x42».denote_cache.forged.typedVerified : True := True.intro

/-- error: `forged` was verified by a retired route -/
#guard_msgs in
#leaner_verify 0x42::denote_cache::forged

open Lean Elab Command in
run_cmd do
  for suffix in [`compiled_eq, `verified] do
    if (← getEnv).contains
        (`LeanerLang.Tests.DenoteCache.«0x42».denote_cache.forged ++ suffix) then
      throwError "rejected cache entry leaked {suffix}"

end LeanerLang.Tests.DenoteCache
