-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerE2ETests.CheckSupport

/-! Port of v0's loop invariants: scalar loop state, a while loop, and a
mutable aggregate whose unrelated field and vector length are preserved. -/

namespace LeanerLang.Tests.VerificationLoopInvariants

leaner module 0x42::loop_invariants where
  struct Bits has Copy, Drop, Store where
    length : u64
    bit_field : Vector<Bool>

  fun count_to(n : u64) -> u64 := do
    let mut i : u64 := 0
    loop do
      if !(i < n) then break
      i := i + 1
    spec do
      invariant i <= n
    return i
  spec count_to where
    ensures result == n
    aborts_if false
  verify count_to

  fun sum_ones(n : u64) -> u64 := do
    let mut i : u64 := 0
    let mut total : u64 := 0
    while i < n do
      total := total + 1
      i := i + 1
    where
      invariant i <= n && total == i
    return total
  spec sum_ones where
    ensures result == n
    aborts_if false
  verify sum_ones

  fun clear(self : &mut Bits) -> Unit := do
    let field := &self.bit_field
    let len := field.length
    let length_ref := &self.length
    let length0 := *length_ref
    let mut i : u64 := 0
    loop do
      if !(i < len) then break
      let bits := &mut self.bit_field
      let bit := &mut bits[i]
      *bit := false
      i := i + 1
    spec do
      invariant i <= len && self.bit_field.length == len && self.length == length0
  spec clear where
    ensures self.length == old(self).length &&
      self.bit_field.length == old(self).bit_field.length
    aborts_if false
  verify clear

#leaner_require_native 0x42::loop_invariants::count_to
#leaner_require_native 0x42::loop_invariants::sum_ones

open Lean Elab Command in
run_cmd do
  for function in ["count_to", "sum_ones", "clear"] do
    let name := ((`LeanerLang.Tests.VerificationLoopInvariants.«0x42».loop_invariants).str function).str "verified"
    unless (← getEnv).contains name do throwError "missing loop verification: {name}"
    if (← collectAxioms name).contains ``sorryAx then
      throwError "generated loop proof contains an admission: {name}"

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».loop_invariants #[
    ⟨"count_to", #[.integer 0], .returned #[.integer 0], {}⟩,
    ⟨"count_to", #[.integer 4], .returned #[.integer 4], {}⟩,
    ⟨"sum_ones", #[.integer 0], .returned #[.integer 0], {}⟩,
    ⟨"sum_ones", #[.integer 4], .returned #[.integer 4], {}⟩]
  let bits : StructHandle := { namespaceId := ⟨0⟩, structId := 0 }
  let empty := RuntimeValue.nominal bits none #[.integer 17, .vector #[]]
  let full := RuntimeValue.nominal bits none #[.integer 17, .vector #[.bool true, .bool false, .bool true]]
  let cleared := RuntimeValue.nominal bits none #[.integer 17, .vector #[.bool false, .bool false, .bool false]]
  assertRunsState `«0x42».loop_invariants #[
    ⟨"clear", #[.borrow 0 empty], .returned #[], { nextLoan := 1 },
      { nextLoan := 1, pending := #[(0, empty)] }⟩,
    ⟨"clear", #[.borrow 0 full], .returned #[], { nextLoan := 1 },
      { nextLoan := 7, pending := #[(0, cleared)] }⟩]

end LeanerLang.Tests.VerificationLoopInvariants
