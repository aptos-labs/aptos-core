-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Verifying Rust-profile functions

Verification is stated over the shared IR, so it should not care which
frontend produced a unit.  This file is the first evidence that it does
not: the function below is authored in the Rust profile — Rust integer
widths, Rust references, a target pointer width — and proved by the same
bare `verify` the Move fixtures use.

Arithmetic is where the profiles genuinely differ, and the contracts here
say so: Move's `+` is checked and aborts on overflow, while Rust's is
modular, so a Rust contract states the wrapped result and declares no
abort.  A Move-shaped `aborts_if` on the same function would be
unprovable, and correctly so.

One shape does not yet verify in this profile and is recorded with its
evidence in `designs/historical/verification-v2.md`: a field read from a struct
passed by reference.
-/

namespace LeanerLang.Tests.VerificationRust

leaner namespace rust_verification where
  fun replace(value : &mut u32, replacement : u32) -> u32 := do
    *value := replacement
    return *value

  spec replace where
    ensures result == replacement
    ensures value == replacement

  fun sum(left : u32, right : u32) -> u32 := left + right

  spec sum where
    ensures result == (left + right) % 4294967296

  fun increment(slot : &mut u32) -> Unit := do
    *slot := *slot + 1

  spec increment where
    ensures slot == (old(slot) + 1) % 4294967296

  verify replace
  verify sum
  verify increment

end LeanerLang.Tests.VerificationRust
