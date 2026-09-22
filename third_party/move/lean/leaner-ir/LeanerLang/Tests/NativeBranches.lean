-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_branches where
  fun maximum(left : u8, right : u8) -> u8 := if left < right then right else left
  spec maximum where
    ensures result == if left < right then right else left
    aborts_if false
  verify maximum

  fun indicator(left : u8, right : u8) -> u8 := if left != right then 1 else 0
  spec indicator where
    ensures left == right ==> result == 0
    ensures !(left == right) ==> result == 1
    aborts_if false
  verify indicator

  fun guarded(value : u8) -> u8 := if value < 255 then value + 1 else value
  spec guarded where
    ensures result == if value < 255 then value + 1 else value
    aborts_if false
  verify guarded

  fun nested(value : u8) -> u8 :=
    if value < 10 then (if value > 0 then value - 1 else 0) else value
  spec nested where
    ensures result == if value < 10 && value > 0 then value - 1 else value
    aborts_if false
  verify nested

  fun after_local(value : u8) -> u8 := do
    let next := value + 1
    return if next <= 10 then next + 1 else next
  spec after_local where
    ensures result == if value < 10 then value + 2 else value + 1
    aborts_if value == 255
  verify after_local

  fun branch_local(value : u8) -> u8 :=
    if value >= 10 then (do
      let next := value - 1
      return next - 1) else value
  spec branch_local where
    ensures result == if value >= 10 then value - 2 else value
    aborts_if false
  verify branch_local

  fun less(left : u8, right : u8) -> Bool := left < right
  spec less where
    ensures result == (left < right)
    aborts_if false
  verify less

  fun equal(left : u8, right : u8) -> Bool := left == right
  spec equal where
    ensures result == (left == right)
    aborts_if false
  verify equal

  fun comparison_after_local(value : u8) -> Bool := do
    let next := value + 1
    return next >= 10
  spec comparison_after_local where
    ensures result == (value >= 9)
    aborts_if value == 255
  verify comparison_after_local

  fun boolean_branch(value : u8) -> Bool :=
    if value < 10 then true else value == 20
  spec boolean_branch where
    ensures result == (value < 10 || value == 20)
    aborts_if false
  verify boolean_branch

  fun branch_overflow(value : u8) -> u8 := if value > 0 then value + 1 else value
  spec branch_overflow where
    ensures result == if value > 0 then value + 1 else value
    aborts_if value == 255
  verify branch_overflow

  fun flag_argument(flag : Bool, value : u8) -> u8 := if flag then value else 0
  spec flag_argument where
    ensures result == if flag then value else 0
    aborts_if false
  verify flag_argument

  fun flag_local(value : u8) -> u8 := do
    let flag := value < 255
    return if flag then value + 1 else value
  spec flag_local where
    ensures result == if value < 255 then value + 1 else value
    aborts_if false
  verify flag_local

  fun return_flag(value : u8) -> Bool := do
    let flag := value < 10
    return flag
  spec return_flag where
    ensures result == (value < 10)
    aborts_if false
  verify return_flag

  fun only_booleans(flag : Bool, left : Bool, right : Bool) -> Bool :=
    if flag then left else right
  spec only_booleans where
    ensures result == if flag then left else right
    aborts_if false
  verify only_booleans

  fun bad_abort(value : u8) -> u8 := if value > 0 then value + 1 else value
  spec bad_abort where
    ensures result == if value > 0 then value + 1 else value
    aborts_if false

  fun bad(value : u8) -> u8 := if value == 0 then 1 else value
  spec bad where
    ensures result == value
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_branches::bad

#guard_msgs (drop error) in
#leaner_verify 0x42::native_branches::bad_abort

#leaner_require_native 0x42::native_branches::maximum
#leaner_require_native 0x42::native_branches::nested
#leaner_require_native 0x42::native_branches::after_local
#leaner_require_native 0x42::native_branches::branch_local
#leaner_require_native 0x42::native_branches::less
#leaner_require_native 0x42::native_branches::comparison_after_local
#leaner_require_native 0x42::native_branches::boolean_branch
#leaner_require_native 0x42::native_branches::branch_overflow
#leaner_require_native 0x42::native_branches::flag_argument
#leaner_require_native 0x42::native_branches::flag_local
#leaner_require_native 0x42::native_branches::return_flag
#leaner_require_native 0x42::native_branches::only_booleans

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  let artifactRoot := `«0x42».native_branches
  for function in ["maximum", "indicator", "guarded", "nested", "after_local", "branch_local",
      "less", "equal", "comparison_after_local", "boolean_branch", "branch_overflow",
      "flag_argument", "flag_local", "return_flag", "only_booleans"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_branches::{function} ")
    unless measured.size == 2 do throwError "missing native branch cost stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native branch {function} exceeds aggregate 50M budget"
    for suffix in [`computation, `nativeSummary, `computationVerified] do
      let root := Name.str artifactRoot function ++ suffix
      let nativeValues := suffix == `computation
      let mut pending := #[root]
      let mut visited : Array Name := #[]
      while let some name := pending.back? do
        pending := pending.pop
        if visited.contains name then continue
        visited := visited.push name
        let some declaration := (← getEnv).find? name
          | throwError "missing native branch artifact {name}"
        let constants := declaration.type.getUsedConstants ++
          ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
        for dependency in constants do
          if dependency == ``LeanerIR.RuntimeFrame ||
              dependency == ``LeanerIR.Proofs.typedFunction ||
              dependency == ``LeanerIR.Proofs.decodeSpec ||
              (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
              (`LeanerIR.Proofs.ComputationAgreement).isPrefixOf dependency ||
              dependency.getString! == "computationRepresents" ||
              dependency.getString! == "computationState" || dependency == ``sorryAx ||
              (nativeValues && dependency == ``LeanerIR.RuntimeValue) then
            throwError "native branch {root} has forbidden dependency {dependency}"
          if artifactRoot.isPrefixOf dependency then pending := pending.push dependency
  for function in [`bad, `bad_abort] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (artifactRoot ++ function ++ suffix) then
        throwError "rejected branch leaked {function}.{suffix}"

open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) :
    («0x42».native_branches.branch_overflow.computation ⟨⟨255, by decide⟩⟩).aborts
      state (.abort, #[.integer 256]) := by
  simp [«0x42».native_branches.branch_overflow.computation,
    NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
    IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, Spec.abort]

open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) :
    («0x42».native_branches.guarded.computation ⟨⟨255, by decide⟩⟩).ok
      state ⟨255, by decide⟩ state := by
  simp [«0x42».native_branches.guarded.computation, Spec.pure]
