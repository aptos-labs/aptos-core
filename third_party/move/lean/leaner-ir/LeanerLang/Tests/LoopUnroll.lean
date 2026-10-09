-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::loop_unroll where
  fun count(n : u8) -> u8 := do
    let mut i := 0u8
    while i < n do
      i := i + 1u8
    i
  spec count where
    pragma unroll = 3
    requires n <= 3
    ensures result == n
    aborts_if false

  fun no_iterations(n : u8) -> u8 := do
    let mut i := 0u8
    while i < n do
      i := i + 1u8
    i
  spec no_iterations where
    pragma unroll = 0
    requires n == 0
    ensures result == 0
    aborts_if false

  fun early_return(n : u8) -> u8 := do
    let mut i := 0u8
    while i < n do
      if i == 1u8 then return i
      i := i + 1u8
    i
  spec early_return where
    pragma unroll = 2
    requires n <= 3
    ensures result == if n == 0 then 0 else 1
    aborts_if false

  -- Computed literals decide the loop conditions: only the one feasible
  -- path of the nested loops is explored.
  fun nested() -> u8 := do
    let mut total := 0u8
    let mut i := 0u8
    while i < 2u8 do
      let mut j := 0u8
      while j < 2u8 do
        total := total + 1u8
        j := j + 1u8
      i := i + 1u8
    total
  spec nested where
    pragma unroll = 2
    ensures result == 4
    aborts_if false

-- A path that iterates beyond the bound is reported as such, not proved.
leaner module 0x42::loop_unroll_bound where
  pragma verify = false

  fun outrun(n : u8) -> u8 := do
    let mut i := 0u8
    while i < n do
      i := i + 1u8
    i
  spec outrun where
    pragma unroll = 1
    requires n <= 3
    ensures result == n
    aborts_if false

/--
error: the bound `pragma unroll = 1` of a loop is not established
---
error: leaner verification failed: the automatic verification of `outrun` failed; provide a proof: `verify outrun by …` in the module (`verify outrun by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::loop_unroll_bound::outrun

-- Exhausting the proof allowance must preserve normal executions, failures,
-- and undefinedness; replacing the continuation with bottom loses all three.
open LeanerIR LeanerIR.Proofs LeanerIR.Proofs.Denote in
example {unit : Validation.ValidatedUnit} [Skolems unit]
    (entry : HEnv .nil) (initial : Memory unit) :
    ¬wp (loopUnroll (ρ := .none) 0 0 1
      (fun _ env => Spec.pure (.value () env)) entry)
      (fun _ _ => False) (fun _ => True) initial := by
  intro verified
  exact verified.1 (.value () entry) initial (by
    simp [loopUnroll, Spec.fixApprox, Spec.pure])

open LeanerIR LeanerIR.Proofs LeanerIR.Proofs.Denote in
example {unit : Validation.ValidatedUnit} [Skolems unit]
    (entry : HEnv .nil) (initial : Memory unit) (error : Failure) :
    ¬wp (loopUnroll (ρ := .none) 0 0 1
      (fun _ _ => Spec.abort error) entry)
      (fun _ _ => True) (fun _ => False) initial := by
  intro verified
  exact verified.2.1 error (by simp [loopUnroll, Spec.fixApprox, Spec.abort])

open LeanerIR LeanerIR.Proofs LeanerIR.Proofs.Denote in
example {unit : Validation.ValidatedUnit} [Skolems unit]
    (entry : HEnv .nil) (initial : Memory unit) :
    ¬wp (loopUnroll (ρ := .none) 0 0 1
      (fun _ _ => ⟨fun _ _ _ => False, fun _ _ => False, fun _ => True⟩) entry)
      (fun _ _ => True) (fun _ => True) initial := by
  intro verified
  exact verified.2.2 (by simp [loopUnroll, Spec.fixApprox])
