-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Source-level `Given` invariant-instantiation controls

Verifies the `Given` concrete-export boundary when generic `Given`-proved functions
(`identity<T>` and `touch_mut<T>`) are instantiated at a concrete nominal struct carrying
a data invariant (`Percent` with `invariant value <= 100`):
1. **Negative controls (first)**:
   - Source-level call-site violation (`launder_open_bad`) attempting to copy `*p`
     (`p.value == 101`) into `identity::<Percent>(*p)` before repairing `p` fails at the
     call boundary naming `` `invariant value <= 100` `` (never for a missing transfer dependency).
   - Source-level mutable-reference call-site violation (`launder_handover_bad`) attempting
     to pass `p : &mut Percent` (`p.value == 101`) into `touch_mut::<Percent>(p)` before
     repairing `p` fails at the named `.handOver` call boundary
     (`"the data invariant \`invariant value <= 100\` does not hold where a mutable reference is passed to a call"`).
   - Public export theorems (`identity.verified`, `identity.typedVerified`) require
     `identity.givenTransfer` (`Contract.RelationalCertificate`) as an explicit premise.
   - Contract-level uncertified `Eq` transfer at `Percent` is uninhabited
     (`uncertified_percent_given_transfer_refused`).
2. **Positive controls (second)**:
   - Concrete caller `roundtrip(p : Percent) -> Percent` verifies through
     `identity.typedSemanticsVerified`, proving `result == p && result.value <= 100`
     from `identity`'s `validResult` while propagating `identity.givenTransfer`
     (with zero `nativeDependencies`).
-/

-- Select the Given route explicitly so `identity` and `touch_mut` are proved on the Given route
-- while `verex.givenRoute` defaults to `false`.
set_option verex.givenRoute true

namespace VerexSL.Tests.GivenInvariantInstantiation

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexSL.Verify
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

private def forbiddenSubstrings : List String := [
  "exceeded its budget", "exceeded its heartbeat budget", "maxHeartbeats",
  "maximum number of heartbeats", "heartbeats", "stopped by the budget",
  "deterministic timeout", "(deterministic) timeout",
  "maximum recursion depth", "deep recursion",
  "unknown identifier", "unknown constant", "unknown function",
  "no Verex module declares", "no denotation for",
  "given verification is unsupported", "given verification concrete transfer failed",
  "givenTransfer", "transfer",
  "pragma verify = manual",
  "type mismatch", "application type mismatch", "failed to synthesize",
  "unsolved goals", "no goals to be solved",
  "kernel", "PANIC at", "internal exception"
]

private def assertRejectedAtBoundary (base : Name) (command : TSyntax `command)
    (expectedDiag : String) (label : String) : CommandElabM Unit := do
  let saved ← get
  let priorCount := saved.messages.toList.length
  let command := Id.run <| command.raw.replaceM fun stx =>
    pure <| if stx.isIdent then some (mkIdentFrom stx stx.getId.eraseMacroScopes) else none
  let thrown ← try
    elabCommand command
    pure none
  catch error =>
    pure (some (error.isMaxHeartbeat || error.isMaxRecDepth, ← error.toMessageData.toString))
  let attempted ← get
  let curNs ← getCurrNamespace
  let fresh := attempted.messages.toList.drop priorCount
  let freshErrors := fresh.filter (·.severity == .error)
  let hasHeartbeatTag := fresh.any (·.data.hasTag (· == `runtime.maxHeartbeats))
  let errorTexts := (← freshErrors.mapM fun message => message.data.toString) ++
    (thrown.map (·.2)).toList
  let noProof := !attempted.env.contains (base ++ `verified) &&
    !attempted.env.contains (curNs ++ base ++ `verified) &&
    !attempted.env.contains (base ++ `givenTypedVerified) &&
    !attempted.env.contains (curNs ++ base ++ `givenTypedVerified) &&
    !attempted.env.contains (base ++ `typedVerified) &&
    !attempted.env.contains (curNs ++ base ++ `typedVerified) &&
    !attempted.env.contains (base ++ `givenTransfer) &&
    !attempted.env.contains (curNs ++ base ++ `givenTransfer) &&
    ((givenProofArtifacts.getState attempted.env).find? base).isNone &&
    ((givenProofArtifacts.getState attempted.env).find? (curNs ++ base)).isNone
  modify fun _ => saved
  unless noProof do
    throwError "expected `{label}` ({base}) not to publish any proof artifact"
  if thrown.any (·.1) || hasHeartbeatTag then
    throwError "expected semantic rejection for `{label}` ({base}), got heartbeat/recursion limit: {errorTexts}"
  if let some bad := errorTexts.find? (fun text => forbiddenSubstrings.any (text.contains ·)) then
    throwError "expected call-boundary rejection for `{label}` ({base}), got forbidden/transfer/infrastructure diagnostic: {bad}"
  let fn := base.getString!
  let summaryPrefix := s!"verex verification failed: the automatic verification of `{fn}` failed;"
  let hasSummary := errorTexts.any (·.startsWith summaryPrefix)
  let hasExpectedDiag := errorTexts.any (· == expectedDiag)
  let hasDataInvariantLeaf := errorTexts.any fun text =>
    text.startsWith summaryPrefix && text.contains "| data_invariant_1 =>"
  let allExpected := errorTexts.all fun text =>
    text.startsWith summaryPrefix || text == expectedDiag ||
      text == "the precondition of `identity` is not established"
  unless !errorTexts.isEmpty && hasSummary && hasExpectedDiag && hasDataInvariantLeaf && allExpected do
    throwError "expected `{label}` ({base}) to fail at `{expectedDiag}` with `data_invariant_1`, got: {errorTexts}"

verex module 0x42::given_invariant_instantiation where
  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  -- Opaque contract so concrete callers reason through `identity.typedSemanticsVerified`
  -- rather than inlining `identity`'s body, while `identity` itself is verified on Given.
  fun identity {T}(value : T) -> T := value
  spec identity where
    pragma opaque = true
    aborts_if false
    ensures result == value

  -- Generic mutable-reference callee verified on Given (`pragma opaque = true`),
  -- exercising the `.handOver` call boundary on `&mut Percent`.
  fun touch_mut {T}(slot : &mut T) -> Unit := ()
  spec touch_mut where
    pragma opaque = true
    aborts_if false
    ensures slot == old(slot)

  -- Negative 1 (first): open `&mut Percent` breaks `value <= 100` and attempts to
  -- copy `*p` into `identity::<Percent>(*p)` before repairing `p` (`.copyOut` cut).
  fun launder_open_bad(p : &mut Percent) -> Percent := do
    p.value := 101
    let leaked := identity::<Percent>(*p)
    p.value := 50
    leaked
  spec launder_open_bad where
    pragma verify = false
    aborts_if false
    ensures result.value == 101

  -- Negative 2 (second): open `&mut Percent` breaks `value <= 100` and attempts to
  -- pass `p` directly to `touch_mut::<Percent>(p)` before repairing `p` (`.handOver` cut).
  fun launder_handover_bad(p : &mut Percent) -> Unit := do
    p.value := 101
    touch_mut::<Percent>(p)
    p.value := 50
  spec launder_handover_bad where
    pragma verify = false
    aborts_if false
    ensures p.value == 50

  -- Positive concrete caller (third): instantiates Given-proved `identity` at `Percent`,
  -- owing `validArgs` (`Percent.Holds p`) and receiving `validResult` (`Percent.Holds result`)
  -- while inheriting `identity.givenTransfer`.
  fun roundtrip(p : Percent) -> Percent := identity::<Percent>(p)
  spec roundtrip where
    pragma verify = false
    aborts_if false
    ensures result == p && result.value <= 100

-- Concrete callers instantiate `identity` and `touch_mut` on the concrete verification route.
set_option verex.givenRoute false

/-! ## 1. Negative Controls (First) -/

-- Source-level call-site rejections when `Percent`'s invariant is violated at the call
-- (both copy-out `*p` and mutable-reference hand-over `p`), never failing for transfer deps.
run_cmd do
  assertRejectedAtBoundary `«0x42».given_invariant_instantiation.launder_open_bad
    (← `(command| verify 0x42::given_invariant_instantiation::launder_open_bad))
    "the data invariant `invariant value <= 100` does not hold where a value is copied out of a \
      mutable reference"
    "Given invariant instantiation: launder_open_bad (.copyOut)"
  assertRejectedAtBoundary `«0x42».given_invariant_instantiation.launder_handover_bad
    (← `(command| verify 0x42::given_invariant_instantiation::launder_handover_bad))
    "the data invariant `invariant value <= 100` does not hold where a mutable reference is passed to a call"
    "Given invariant instantiation: launder_handover_bad (.handOver)"

-- Refusal at the concrete export boundary: `identity.verified` and `identity.typedVerified`
-- cannot be instantiated without supplying `identity.givenTransfer` (`Contract.RelationalCertificate`).
run_cmd liftTermElabM do
  let env ← getEnv
  let curNs ← getCurrNamespace
  let idBase := if env.contains (curNs ++ `«0x42».given_invariant_instantiation.identity.givenTransfer) then
      curNs ++ `«0x42».given_invariant_instantiation.identity
    else `«0x42».given_invariant_instantiation.identity
  let transferName := idBase ++ `givenTransfer
  let some transferInfo := env.find? transferName
    | throwError "missing {transferName}"
  let some body := transferInfo.value?
    | throwError "{transferName} has no definition body"
  unless body.getUsedConstants.contains ``Contract.RelationalCertificate do
    throwError "{transferName} does not require Contract.RelationalCertificate"
  for thm in [idBase ++ `typedVerified, idBase ++ `verified] do
    let info ← getConstInfo thm
    forallTelescope info.type fun binders _ => do
      let hasTransfer ← binders.anyM fun b =>
        return (← inferType b).isConstOf transferName
      unless hasTransfer do
        throwError "{thm} silently dropped explicit {transferName} premise"

-- Contract-level refusal of uncertified plain `Eq` instantiation at `Percent`:
-- `identity.givenTypedContract` supplies only `True` in `validResult`, whereas
-- `identity.typedContract` at `Percent` requires `NTy.Holds Percent` (`value <= 100`),
-- refuted at `badPercent := (101, ())`.
run_cmd do
  let env ← getEnv
  let curNs ← getCurrNamespace
  let modName := curNs ++ `«0x42».given_invariant_instantiation
  let some unit := (registeredUnit? env modName).orElse
      (fun _ => registeredUnit? env `«0x42».given_invariant_instantiation)
    | throwError "missing registered unit for 0x42::given_invariant_instantiation"
  let segments := (curNs.components.map (·.toString)).toArray ++ #["0x42", "given_invariant_instantiation"]
  let unitDef ← ensureUnitInstance
    (if (registeredUnit? env modName).isSome then segments else #["0x42", "given_invariant_instantiation"])
    unit
  let descriptor := nominalDescriptorName unitDef ⟨⟨0⟩, 0⟩
  let percentNTyName := curNs ++ `percentNTy
  liftTermElabM do
    unless (← getEnv).contains percentNTyName do
      let val := mkConst descriptor
      addDecl (.defnDecl {
        name := percentNTyName, levelParams := [], type := mkConst ``NTy, value := val,
        hints := .abbrev, safety := .safe })
  let env ← getEnv
  let unitIdent := mkIdent (rootNamespace ++ unitDef)
  let percentIdent := mkIdent (rootNamespace ++ percentNTyName)
  let idTyped := mkIdent (if env.contains (modName ++ `identity.typedContract) then
      rootNamespace ++ modName ++ `identity.typedContract
    else `_root_.«0x42».given_invariant_instantiation.identity.typedContract)
  let idGiven := mkIdent (if env.contains (modName ++ `identity.givenTypedContract) then
      rootNamespace ++ modName ++ `identity.givenTypedContract
    else `_root_.«0x42».given_invariant_instantiation.identity.givenTypedContract)
  elabCommand (← `(command|
    theorem uncertified_percent_given_transfer_refused :
        let concreteFrame : Skolems $unitIdent :=
          Skolems.instantiate (unit := $unitIdent) ⟨.cons $percentIdent .nil, by decide⟩
            (Skolems.runtime $unitIdent)
        let givenFrame : GivenFrame := ⟨concreteFrame.toCarriers⟩
        ¬ Nonempty (Contract.RelationalTransfer Eq Eq Eq Eq
            (@$idTyped $unitIdent _ concreteFrame #[])
            (@$idGiven $unitIdent _ givenFrame (Memory $unitIdent))) := by
      intro concreteFrame givenFrame ⟨compatible⟩
      let badPercent : SpecInt (.bits 64) false × PUnit := (⟨101, by decide⟩, ())
      have givenValid :
          (@$idGiven $unitIdent _ givenFrame (Memory $unitIdent)).validResult badPercent := by
        dsimp only [$idGiven:ident, ResultShape.Holds]
        exact GivenFrame.holds_param (givenFrame.withDeclarations _) 0 badPercent
      have concreteValid :
          (@$idTyped $unitIdent _ concreteFrame #[]).validResult badPercent :=
        compatible.validResult badPercent badPercent rfl givenValid
      simp only [$idTyped:ident, ResultShape.Holds] at concreteValid
      have concreteHolds : @NTy.Holds (Skolems.runtime $unitIdent).toCarriers $percentIdent badPercent :=
        @NTy.Holds.intro (Skolems.runtime $unitIdent).toCarriers $percentIdent badPercent concreteValid.1
      dsimp only [$percentIdent:ident] at concreteHolds
      clear concreteValid compatible givenValid givenFrame concreteFrame
      letI : Carriers := (Skolems.runtime $unitIdent).toCarriers
      verex_certified_clauses
      dsimp only [badPercent] at *
      try simp only [VerexIR.Proofs.Obligation_eq] at *
      omega))

/-! ## 2. Positive Controls (Second) -/

-- Concrete caller `roundtrip` verifies against `identity`'s contract at `Percent`,
-- deriving `result.value <= 100` from `validResult`.
verify 0x42::given_invariant_instantiation::roundtrip

-- Audit that `identity` and `touch_mut` used the Given route and `roundtrip` propagated
-- `identity.givenTransfer` into its concrete theorems with zero `nativeDependencies`.
run_cmd liftTermElabM do
  let env ← getEnv
  let curNs ← getCurrNamespace
  let modBase := if env.contains (curNs ++ `«0x42».given_invariant_instantiation.identity.givenTransfer) then
      curNs ++ `«0x42».given_invariant_instantiation
    else `«0x42».given_invariant_instantiation
  let idBase := modBase ++ `identity
  let touchBase := modBase ++ `touch_mut
  let callerBase := modBase ++ `roundtrip
  let transferName := idBase ++ `givenTransfer
  unless (givenProofArtifacts.getState env).find? idBase == some (idBase ++ `givenTypedVerified) do
    throwError "identity did not use the Given route"
  unless (givenProofArtifacts.getState env).find? touchBase == some (touchBase ++ `givenTypedVerified) do
    throwError "touch_mut did not use the Given route"
  unless ((givenProofArtifacts.getState env).find? callerBase).isNone do
    throwError "roundtrip unexpectedly used the Given route instead of concrete instantiation"
  unless ((nativeDependencies.getState env).find? callerBase).getD #[] |>.isEmpty do
    throwError "roundtrip treated identity as an unverified native instead of routing through Given"
  unless ((transferDependencies.getState env).find? callerBase) == some #[transferName] do
    throwError "roundtrip did not propagate identity.givenTransfer"
  for givenThm in [idBase ++ `givenTypedVerified, touchBase ++ `givenTypedVerified] do
    let axioms ← collectAxioms givenThm
    let stdAxioms := [``propext, ``Classical.choice, ``Quot.sound]
    unless axioms.all (stdAxioms.contains ·) do
      throwError "unexpected axioms in {givenThm}: {axioms}"
  for suffix in [`typedVerified, `verified] do
    let thm := callerBase ++ suffix
    let info ← getConstInfo thm
    forallTelescope info.type fun binders _ => do
      let hasTransfer ← binders.anyM fun b =>
        return (← inferType b).isConstOf transferName
      unless hasTransfer do
        throwError "{thm} silently dropped identity.givenTransfer!"
    let axioms ← collectAxioms thm
    let allowed := [``propext, ``Classical.choice, ``Quot.sound,
      ``VerexIR.Proofs.Denote.compileFunction_agrees]
    unless axioms.all (allowed.contains ·) do
      throwError "unexpected axioms in {thm}: {axioms}"

end VerexSL.Tests.GivenInvariantInstantiation
