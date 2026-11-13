-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Literal bounds from opaque callees still expose finite positions, including
large literals, strict bounds, generic instantiation and mutable results. -/

set_option maxHeartbeats 25000
set_option verex.verifyHeartbeats 25000

-- The constant-list shortcut must leave differing entries alone.
example (i : Nat) (h : i < 2) :
    ([7, 8] : List Nat)[i] = if i = 0 then 7 else 8 := by
  try simp only [vir_denote_norm]
  rcases (show i = 0 ∨ i = 1 by omega) with rfl | rfl <;> rfl

verex module 0x42::call_range_results where
  fun choose(flag : Bool) -> u64 := if flag then 0 else 1
  spec choose where
    pragma opaque
    aborts_if false
    ensures 0 <= result
    ensures result <= 1

  fun choose_large() -> u64 := 1000000000000
  spec choose_large where
    pragma opaque
    aborts_if false
    ensures 999999999999 < result
    ensures result < 1000000000001

  fun write_choice(slot : &mut u64, flag : Bool) -> Unit :=
    *slot := if flag then 0 else 1
  spec write_choice where
    pragma opaque
    aborts_if false
    ensures 0 <= *slot
    ensures *slot <= 1

  fun read_choice(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_choice where
    aborts_if false
    ensures result == 7

  fun read_large() -> u64 := do
    let index := choose_large()
    let values := vector<u64>[7]
    values[index - 1000000000000]
  spec read_large where
    aborts_if false
    ensures result == 7

  fun read_written(flag : Bool) -> u64 := do
    let mut index := 0u64
    write_choice(&mut index, flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_written where
    aborts_if false
    ensures result == 7

  fun choose_generic {T}(flag : Bool) -> u64 := if flag then 0 else 1
  spec choose_generic where
    pragma opaque
    aborts_if false
    ensures 0 <= result
    ensures result <= 1

  fun read_generic(flag : Bool) -> u64 := do
    let index := choose_generic::<Bool>(flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_generic where
    aborts_if false
    ensures result == 7

-- Computed positions retain the finite range through arithmetic temporaries.
verex module 0x42::computed_call_range_results where
  fun choose(flag : Bool) -> u64 := if flag then 10 else 11
  spec choose where
    pragma opaque
    aborts_if false
    ensures 10 <= result
    ensures result <= 11

  fun choose_large() -> u64 := 1000000000000
  spec choose_large where
    pragma opaque
    aborts_if false
    ensures 999999999999 < result
    ensures result < 1000000000001

  fun write_choice(slot : &mut u64, flag : Bool) -> Unit :=
    *slot := if flag then 10 else 11
  spec write_choice where
    pragma opaque
    aborts_if false
    ensures 10 <= *slot
    ensures *slot <= 11

  fun read_choice(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_choice where
    aborts_if false
    ensures result == 7

  fun read_large() -> u64 := do
    let index := choose_large()
    let values := vector<u64>[7]
    values[index - 1000000000000]
  spec read_large where
    aborts_if false
    ensures result == 7

  fun read_written(flag : Bool) -> u64 := do
    let mut index := 0u64
    write_choice(&mut index, flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_written where
    aborts_if false
    ensures result == 7

  fun choose_generic {T}(flag : Bool) -> u64 := if flag then 10 else 11
  spec choose_generic where
    pragma opaque
    aborts_if false
    ensures 10 <= result
    ensures result <= 11

  fun read_generic(flag : Bool) -> u64 := do
    let index := choose_generic::<Bool>(flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_generic where
    aborts_if false
    ensures result == 7

  fun wrong_value(flag : Bool) -> u64 := read_choice(flag)
  spec wrong_value where
    pragma verify = false
    aborts_if false
    ensures result == 8

  fun repeated_choices(flag : Bool) -> u64 := do
    let first := choose(flag)
    let second := choose(flag)
    let total := first + second
    spec do
      assert total <= 22
      assert 20 <= total
    total
  spec repeated_choices where
    pragma verify = false
    aborts_if false
    ensures result <= 22

  fun wrong_index(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index - 9]
  spec wrong_index where
    pragma verify = false
    aborts_if false

-- Earlier call results remain addressable when a later call uses the same callee.
-- Even a one-thousand-heartbeat automatic leaf cap does not constrain an
-- authored proof, including its explicitly requested cheap deciders.
set_option verex.leafHeartbeats 1 in
verify 0x42::computed_call_range_results::repeated_choices by lean =>
  verex_obligations
  verex_group range := assert_1, assert_2
  verex_on
  | range =>
    verex_prepare only
    have first_bound : old_choose_result.val <= 11 := by assumption
    have second_bound : choose_result.val <= 11 := by assumption
    constructor <;> omega
  | _ => first | verex_decide | (verex_prepare only; all_goals omega)

/--
error: the specification clause `ensures result == 8` is not established
---
error: verex verification failed: the automatic verification of `wrong_value` failed; provide a proof in the module:
  verify wrong_value by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::computed_call_range_results::wrong_value

/--
error: the specification clause `aborts_if false` is not established
---
error: verex verification failed: the automatic verification of `wrong_index` failed; provide a proof in the module:
  verify wrong_index by lean =>
    verex_obligations
    verex_on
    | aborts_if_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::computed_call_range_results::wrong_index

open VerexIR.Proofs VerexIR.Proofs.Denote

-- Type arguments do not make an empty array's size variable.
example : ∀ r : VerexIR.SpecInt (.bits 64) false,
    Obligation "probe" 0 0 (0 ≤ r.val ∧ r.val ≤ ((#[] : Array Nat).size : Int)) → r.val = 0 := by
  verex_call_result_cases

example (α : Type) : ∀ r : VerexIR.SpecInt (.bits 64) false,
    Obligation "probe" 0 0 (0 ≤ r.val ∧ r.val ≤ ((#[] : Array α).size : Int)) → r.val = 0 := by
  verex_call_result_cases

-- The target depends on the returned certificate as well as its scalar value.
example (α : Type) : ∀ r : VerexIR.SpecInt (.bits 64) false,
    ∀ h : Obligation "probe" 0 0 (0 ≤ r.val ∧ r.val ≤ ((#[] : Array α).size : Int)),
    (⟨r.val, (show 0 ≤ r.val ∧ r.val ≤ ((#[] : Array α).size : Int) from
      by simpa only [Obligation_eq] using h).1⟩ :
      {n : Int // 0 ≤ n}).val = 0 := by
  verex_call_result_cases

-- A variable array is not mistaken for an empty one.
example (α : Type) (values : Array α) : ∀ r : VerexIR.SpecInt (.bits 64) false,
    Obligation "probe" 0 0 (0 ≤ r.val ∧ r.val ≤ (values.size : Int)) → r.val ≤ values.size := by
  verex_call_result_cases
  guard_target =ₛ ∀ r : VerexIR.SpecInt (.bits 64) false,
    Obligation "probe" 0 0 (0 ≤ r.val ∧ r.val ≤ (values.size : Int)) → r.val ≤ values.size
  intro r h
  exact (show 0 ≤ r.val ∧ r.val ≤ (values.size : Int) from
    by simpa only [Obligation_eq] using h).2

-- A two-point interval remains two alternatives, rather than being pinned.
example (α : Type) : ∀ r : VerexIR.SpecInt (.bits 64) false,
    Obligation "probe" 0 0 (0 ≤ r.val ∧ r.val ≤ ((#[none] : Array (Option α)).size : Int)) →
    r.val.toNat = 0 ∨ r.val.toNat = 1 := by
  verex_call_result_cases
  all_goals first | exact Or.inl rfl | exact Or.inr rfl

-- A callee-spelled bound and a caller-spelled literal share one scalar size.

example [outer : Carriers] [CarriersNonempty outer] :
    ∀ r : VerexIR.SpecInt (.bits 64) false,
    Obligation "probe" 0 0 (0 ≤ r.val ∧ r.val ≤
      ((@Array.size
        (@NTy.carrier (Carriers.instantiate ⟨.cons (.param 0) .nil, rfl⟩ outer) (.param 0))
        (([] : List (@NTy.carrier outer (.param 0))).toArray)) : Int)) → r.val = 0 := by
  verex_call_result_cases

example [outer : Carriers] [CarriersNonempty outer] (r : VerexIR.SpecInt (.bits 64) false)
    (first _second : r.val ≤
      ((@Array.size
        (@NTy.carrier (Carriers.instantiate ⟨.cons (.param 0) .nil, rfl⟩ outer) (.param 0))
        (([] : List (@NTy.carrier outer (.param 0))).toArray)) : Int)) : r.val ≤ 0 := by
  verex_canonical_hypotheses [first]
  guard_hyp first :ₛ r.val ≤ ((([] : List (@NTy.carrier outer (.param 0))).toArray).size : Int)
  guard_hyp _second :ₛ r.val ≤
      ((@Array.size
        (@NTy.carrier (Carriers.instantiate ⟨.cons (.param 0) .nil, rfl⟩ outer) (.param 0))
        (([] : List (@NTy.carrier outer (.param 0))).toArray)) : Int)
  simpa only [List.size_toArray, List.length_nil, Int.natCast_zero] using first

-- A partition of literal entries determines an interior opaque result; other
-- contracts can leave several feasible results, all of which must be kept.
verex module 0x42::partitioned_call_range_results where
  fun interior() -> u64 := 1
  spec interior where
    pragma opaque
    ensures 0 <= result
    ensures result <= 2
    ensures forall (i : Int), 0 <= i && i < result ==>
      vector<u64>[10, 30][i] < 20
    ensures forall (i : Int), result <= i && i < 2 ==>
      20 <= vector<u64>[10, 30][i]
    aborts_if false

  fun read_interior() -> u64 := do
    let index := interior()
    let values := core.prim.insertVector(vector<u64>[10, 30], index, 20)
    values[1]
  spec read_interior where
    ensures result == 20
    aborts_if false

  fun alternatives(flag : Bool) -> u64 := if flag then 0 else 2
  spec alternatives where
    pragma opaque
    ensures 0 <= result
    ensures result <= 2
    ensures result != 1
    aborts_if false

  fun read_alternatives(flag : Bool) -> u64 := do
    let index := alternatives(flag)
    let values := vector<u64>[7, 9, 7]
    values[index]
  spec read_alternatives where
    ensures result == 7
    aborts_if false

  fun wrong_alternative(flag : Bool) -> u64 := do
    let index := alternatives(flag)
    let values := vector<u64>[7, 9, 8]
    values[index]
  spec wrong_alternative where
    pragma verify = false
    ensures result == 7
    aborts_if false

/--
error: the specification clause `ensures result == 7` is not established
---
error: verex verification failed: the automatic verification of `wrong_alternative` failed; provide a proof in the module:
  verify wrong_alternative by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::partitioned_call_range_results::wrong_alternative

namespace VerexSL.Tests.FinitePositions

example (fallback i j : Int) (hi : 0 ≤ i) (hij : i < j) (hj : j < 3) :
    (#[10, 20, 30][i.toNat]?).getD fallback < (#[10, 20, 30][j.toNat]?).getD fallback := by
  verex_decide_ranges

-- Nonliteral facts remain available in every complete position tuple.
example (fallback a b i j : Int) (ordered : a < b)
    (hi : 0 ≤ i) (hij : i < j) (hj : j < 2) :
    (#[a, b][i.toNat]?).getD fallback < (#[a, b][j.toNat]?).getD fallback := by
  verex_decide_ranges

-- Coupled positions retain the equation that rules out inconsistent tuples.
example (fallback i j : Int) (hi : 0 ≤ i) (hi' : i < 2) (next : j = i + 1) :
    (#[10, 20, 30][i.toNat]?).getD fallback < (#[10, 20, 30][j.toNat]?).getD fallback := by
  have hj : j < 3 := by omega
  verex_decide_ranges

-- The complete box contains a counterexample; it cannot be discarded.
-- The hypotheses are read only by the attempt that must fail.
set_option linter.unusedVariables false in
example (fallback i j : Int) (hi : 0 ≤ i) (hij : i < j) (hj : j < 3) : True := by
  fail_if_success
    have : (#[10, 30, 20][i.toNat]?).getD fallback <
        (#[10, 30, 20][j.toNat]?).getD fallback := by
      verex_decide_ranges
  trivial

end VerexSL.Tests.FinitePositions
