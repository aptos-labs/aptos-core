-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

/-!
# Source-level `validResult` call-result invariant controls

Verifies that `ResultShape.Holds` in generated `Contract.validResult`
(`VerexSL/Contract.lean`) supplies `NTy.Holds` of an invariant-bearing return value
to callers of bodiless `native fun` and unverified `pragma opaque = true` callees ONLY
through the explicit `Satisfies` premise carried on the caller's exported theorems:
1. **Negative controls (first)**:
   - Without assuming `Satisfies` on the callee (`make_percent` or `opaque_percent`), an
     arbitrary callee meaning returning `(101, ())` refutes `ensures result.value <= 100`
     (`without_premise_native_result_holds_unavailable`,
     `without_premise_opaque_result_holds_unavailable`).
   - Neither `use_native` nor `use_opaque` may verify `ensures result.value <= 99` (only the
     invariant `value <= 100` from `validResult` is available).
2. **Positive controls (second)**:
   - `use_native` (calling bodiless `native fun make_percent() -> Percent`) and `use_opaque`
     (calling unverified `pragma opaque = true` `opaque_percent() -> Percent`) both verify
     `ensures result.value <= 100` using `Holds` of the result.
   - Both exported theorems (`verified` and `typedVerified`) carry the explicit `Satisfies`
     premise (`native_make_percent` / `native_opaque_percent`) in their binder telescopes,
     with zero new axioms and `nativeDependencies` recorded.
3. **A result a contract equates with an opaque specification function's value**: the
   clauses of a generated contract read the result at its native type's carrier
   (`NTy.carrier`), the type every other value of a clause has, so the equation is a row's
   and the result's fact is a fact of the specification value.
   - Negatives: a result type without an invariant gives the specification value no bound
     (`use_stated_raw`); the fact does not reach the value of another specification
     function (`use_stated_other`); the invariant gives no stronger bound
     (`use_stated_too_strong`).
   - Positives: the clause binder of the result has that domain, and `use_stated`
     verifies `ensures spec_percent().value <= 100`.
-/

namespace VerexSL.Tests.CallResultHolds

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexSL.Verify
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

verex module 0x42::call_result_holds where
  pragma verify = false

  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  -- 1. Bodiless native returning invariant-bearing `Percent`, with no explicit `ensures`.
  native fun make_percent() -> Percent
  spec make_percent where
    aborts_if false

  -- 2. Unverified `pragma opaque = true` callee returning invariant-bearing `Percent`,
  --    with no explicit `ensures`.
  fun opaque_percent() -> Percent := new Percent { value := 50 }
  spec opaque_percent where
    pragma opaque = true
    pragma verify = false
    aborts_if false

  -- Negative callers (strengthened claim `<= 99` is NOT entailed by `validResult`).
  fun use_native_too_strong() -> Percent := make_percent()
  spec use_native_too_strong where
    aborts_if false
    ensures result.value <= 99

  fun use_opaque_too_strong() -> Percent := opaque_percent()
  spec use_opaque_too_strong where
    aborts_if false
    ensures result.value <= 99

  -- Positive callers: `ensures result.value <= 100` comes solely from `validResult`
  -- (`ResultShape.Holds` of `Percent`) supplied by the explicit `Satisfies` premise.
  fun use_native() -> Percent := make_percent()
  spec use_native where
    aborts_if false
    ensures result.value <= 100

  fun use_opaque() -> Percent := opaque_percent()
  spec use_opaque where
    aborts_if false
    ensures result.value <= 100

  -- 3. A result equated with an opaque specification function's value.
  struct Raw has Copy, Drop where
    value : u64

  opaque spec fun spec_percent() : Percent
  opaque spec fun spec_other() : Percent
  opaque spec fun spec_raw() : Raw

  native fun stated_percent() -> Percent
  spec stated_percent where
    aborts_if false
    ensures result == spec_percent()

  native fun stated_raw() -> Raw
  spec stated_raw where
    aborts_if false
    ensures result == spec_raw()

  fun use_stated_raw() -> Raw := stated_raw()
  spec use_stated_raw where
    aborts_if false
    ensures spec_raw().value <= 100

  fun use_stated_other() -> Percent := stated_percent()
  spec use_stated_other where
    aborts_if false
    ensures spec_other().value <= 100

  fun use_stated_too_strong() -> Percent := stated_percent()
  spec use_stated_too_strong where
    aborts_if false
    ensures spec_percent().value <= 99

  fun use_stated() -> Percent := stated_percent()
  spec use_stated where
    aborts_if false
    ensures spec_percent().value <= 100

/-! ## 1. Negative Controls (First) -/

-- Without the callee `Satisfies` premise, `Holds` of the result (`value <= 100`) is
-- NOT available: an unconstrained plain callee meaning returning `(101, ())` violates
-- both `use_native.typedContract` and `use_opaque.typedContract`.
run_cmd do
  let env ← getEnv
  let curNs ← getCurrNamespace
  let modName := curNs ++ `«0x42».call_result_holds
  let some unit := (registeredUnit? env modName).orElse
      (fun _ => registeredUnit? env `«0x42».call_result_holds)
    | throwError "missing registered unit for 0x42::call_result_holds"
  let segments := if (registeredUnit? env modName).isSome then
      (curNs.components.map (·.toString)).toArray ++ #["0x42", "call_result_holds"]
    else #["0x42", "call_result_holds"]
  let unitDef ← ensureUnitInstance segments unit
  discard <| ensureUnitDefinitions segments unit
  for fn in ["use_native", "use_opaque"] do
    let some (namespaceIndex, ns, _, declaration) := findFunction? unit fn
      | throwError s!"missing `{fn}` in 0x42::call_result_holds"
    let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
      | throwError s!"no native signature for `{fn}`"
    ensureContracts segments fn unit namespaceIndex ns declaration params result
  let env ← getEnv
  let unitIdent := mkIdent (rootNamespace ++ unitDef)
  let useNativeTyped := mkIdent (if env.contains (modName ++ `use_native.typedContract) then
      rootNamespace ++ modName ++ `use_native.typedContract
    else `_root_.«0x42».call_result_holds.use_native.typedContract)
  let useOpaqueTyped := mkIdent (if env.contains (modName ++ `use_opaque.typedContract) then
      rootNamespace ++ modName ++ `use_opaque.typedContract
    else `_root_.«0x42».call_result_holds.use_opaque.typedContract)
  elabCommand (← `(command|
    theorem without_premise_native_result_holds_unavailable (initial : Memory $unitIdent) :
        let frame : Skolems $unitIdent := Skolems.runtime $unitIdent
        let badMeaning : Unit → Spec (Memory $unitIdent) Failure (SpecInt (.bits 64) false × PUnit) :=
          fun _ => Spec.pure (⟨101, by decide⟩, ())
        ¬ Satisfies badMeaning (@$useNativeTyped $unitIdent _ frame) := by
      intro frame badMeaning hSat
      have hRun := hSat () initial (by simp [$useNativeTyped:ident, HList.Holds])
        (by simp [$useNativeTyped:ident]) (by simp [$useNativeTyped:ident])
      have hPost := (hRun.1 (⟨101, by decide⟩, ()) initial ⟨rfl, rfl⟩).2.1
        (by simp [$useNativeTyped:ident])
      simp only [$useNativeTyped:ident] at hPost
      simp only [VerexIR.Proofs.Obligation_eq] at *
      omega))
  elabCommand (← `(command|
    theorem without_premise_opaque_result_holds_unavailable (initial : Memory $unitIdent) :
        let frame : Skolems $unitIdent := Skolems.runtime $unitIdent
        let badMeaning : Unit → Spec (Memory $unitIdent) Failure (SpecInt (.bits 64) false × PUnit) :=
          fun _ => Spec.pure (⟨101, by decide⟩, ())
        ¬ Satisfies badMeaning (@$useOpaqueTyped $unitIdent _ frame) := by
      intro frame badMeaning hSat
      have hRun := hSat () initial (by simp [$useOpaqueTyped:ident, HList.Holds])
        (by simp [$useOpaqueTyped:ident]) (by simp [$useOpaqueTyped:ident])
      have hPost := (hRun.1 (⟨101, by decide⟩, ()) initial ⟨rfl, rfl⟩).2.1
        (by simp [$useOpaqueTyped:ident])
      simp only [$useOpaqueTyped:ident] at hPost
      simp only [VerexIR.Proofs.Obligation_eq] at *
      omega))

-- Source-level rejection when the caller asks for more than `Percent`'s invariant (`<= 99`).
run_cmd do
  for (base, cmd) in [
    (`«0x42».call_result_holds.use_native_too_strong,
      ← `(command| verify 0x42::call_result_holds::use_native_too_strong)),
    (`«0x42».call_result_holds.use_opaque_too_strong,
      ← `(command| verify 0x42::call_result_holds::use_opaque_too_strong))
  ] do
    let saved ← get
    let priorCount := saved.messages.toList.length
    let cmd := Id.run <| cmd.raw.replaceM fun stx =>
      pure <| if stx.isIdent then some (mkIdentFrom stx stx.getId.eraseMacroScopes) else none
    try elabCommand cmd catch _ => pure ()
    let attempted ← get
    let curNs ← getCurrNamespace
    let freshErrors := (attempted.messages.toList.drop priorCount).filter (·.severity == .error)
    let errorTexts ← freshErrors.mapM (·.data.toString)
    let noProof := !attempted.env.contains (base ++ `verified) &&
      !attempted.env.contains (curNs ++ base ++ `verified) &&
      !attempted.env.contains (base ++ `typedVerified) &&
      !attempted.env.contains (curNs ++ base ++ `typedVerified)
    modify fun _ => saved
    unless noProof && errorTexts.any (·.contains "ensures result.value <= 99") do
      throwError "expected `{base}` to reject `ensures result.value <= 99`, got: {errorTexts}"

/-! ## 2. Positive Controls (Second) -/

verify 0x42::call_result_holds::use_native
verify 0x42::call_result_holds::use_opaque

-- Audit that BOTH `use_native` and `use_opaque` verified conditionally on an explicit
-- `Satisfies` premise in `typedVerified` and `verified` (not discharged silently, not turned
-- into an axiom, and not refused).
run_cmd liftTermElabM do
  let env ← getEnv
  let curNs ← getCurrNamespace
  let modBase := if env.contains (curNs ++ `«0x42».call_result_holds.use_native.verified) then
      curNs ++ `«0x42».call_result_holds
    else `«0x42».call_result_holds
  for (caller, calleeStr, contractSuffix) in [
    (`use_native, "make_percent", `make_percent.typedContract),
    (`use_opaque, "opaque_percent", `opaque_percent.typedInterfaceContract)
  ] do
    let callerBase := modBase ++ caller
    let deps := ((nativeDependencies.getState env).find? callerBase).getD #[]
    unless deps.any (·.2 == calleeStr) do
      throwError "{callerBase} did not record `{calleeStr}` in nativeDependencies: {deps.map (·.2)}"
    for suffix in [`typedVerified, `verified] do
      let thm := callerBase ++ suffix
      let info ← getConstInfo thm
      let c1 := modBase ++ contractSuffix
      let c2 := `«0x42».call_result_holds ++ contractSuffix
      forallTelescope info.type fun binders _ => do
        let hasPremise ← binders.anyM fun b => do
          let bTy ← inferType b
          let used := bTy.getUsedConstants
          return used.contains ``VerexIR.Proofs.Satisfies &&
            (used.contains c1 || used.contains c2)
        unless hasPremise do
          throwError "{thm} is missing the explicit `Satisfies ... {c2}` premise!"
      let axioms ← collectAxioms thm
      let allowed := if suffix == `verified then
          [``propext, ``Classical.choice, ``Quot.sound, ``VerexIR.Proofs.Denote.compileFunction_agrees]
        else [``propext, ``Classical.choice, ``Quot.sound]
      unless axioms.all (allowed.contains ·) do
        throwError "unexpected axioms in {thm}: {axioms}"

/-! ## 3. A result equated with an opaque specification function's value -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».call_result_holds.use_stated_raw
    ["ensures spec_raw().value <= 100"]
    (← `(command| verify 0x42::call_result_holds::use_stated_raw))
  expectUnestablished `«0x42».call_result_holds.use_stated_other
    ["ensures spec_other().value <= 100"]
    (← `(command| verify 0x42::call_result_holds::use_stated_other))
  expectUnestablished `«0x42».call_result_holds.use_stated_too_strong
    ["ensures spec_percent().value <= 99"]
    (← `(command| verify 0x42::call_result_holds::use_stated_too_strong))

verify 0x42::call_result_holds::use_stated

-- The clauses of the callee's contract bind the result at its native type's carrier, and
-- no clause states a value at a result shape's carrier.
run_cmd liftTermElabM do
  let env ← getEnv
  let curNs ← getCurrNamespace
  let local_ := curNs ++ `«0x42».call_result_holds.stated_percent.typedContract
  let name := if env.contains local_ then local_
    else `«0x42».call_result_holds.stated_percent.typedContract
  let info ← getConstInfo name
  lambdaTelescope info.value! fun _ contract => do
    unless contract.isAppOf ``VerexIR.Proofs.Contract.mk do
      throwError "{name} is not a constructed contract: {contract}"
    for field in [6, 10] do
      let clause := contract.getArg! field
      lambdaTelescope clause fun binders body => do
        let some result := binders[2]? | throwError "a clause over a result has four binders"
        let domain ← inferType result
        unless domain.isAppOfArity ``NTy.carrier 2 do
          throwError "the result is bound at {domain}"
        if (body.find? (·.isConstOf ``ResultShape.carrier)).isSome then
          throwError "a clause states a value at a result shape's carrier: {body}"

end VerexSL.Tests.CallResultHolds
