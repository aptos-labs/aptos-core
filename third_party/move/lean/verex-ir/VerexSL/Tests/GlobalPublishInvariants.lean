-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Global-publish (`move_to`) and ghost-store (`ghostStore`) data-invariant controls

Verifies data-invariant enforcement across global storage write boundaries for
both `Term.globalPublish` (`move_to<Cap>`) and `Term.ghostStore` (`pragma ghost_memory`
`update global<...>(@0x0).v := ...`):
1. **Negative controls (first)**:
   - Direct solver `.publish` controls (`Term.globalPublish` and `Term.ghostStore`
     at an uncertified local): rejected at `.publish` naming `` `invariant v <= 100` ``
     (`"the data invariant \`invariant v <= 100\` does not hold where a resource is published"`)
     when carrying a source obligation range, and reporting
     `"a data invariant where a resource is published is not established"` without
     source obligation ranges.
   - `publish_pack_bad` / `publish_ensures_false`: publishing a freshly packed
     invalid resource (`move_to<Cap>(s, new Cap { v := 101 })`) is rejected at
     `.construction` naming `` `invariant v <= 100` `` without vacuous pruning.
   - `publish_mut_bad`: mutating a local resource to `101` before publishing is
     rejected at `.mutationEnd` naming `` `invariant v <= 100` ``.
   - `publish_param_overclaim_bad`: publishing a parameter `c : Cap` supplies
     `c.v <= 100` (`validArgs`), rejecting stronger `ensures global<Cap>(s.address).v <= 99`.
   - `ghost_store_pack_bad` / `ghost_store_ensures_false`: storing a freshly packed
     invalid `Cap` (`v := 101`) into ghost global memory is rejected at `.construction`
     naming `` `invariant v <= 100` `` without vacuous pruning.
   - `ghost_store_param_overclaim_bad`: storing a parameter `c : Cap` into ghost
     global memory supplies `c.v <= 100`, rejecting `<= 99`.
2. **Positive controls (second)**:
   - `publish_ok`, `publish_mut_ok`, `publish_param_ok`, `publish_plain_ok` verify
     cleanly and export axiom-clean `typedVerified` and `verified` certificates.
   - `ghost_store_ok`, `ghost_store_param_ok`, `ghost_store_u64_ok` verify cleanly
     and export axiom-clean `typedVerified` model certificates.
-/

namespace VerexSL.Tests.GlobalPublishInvariants

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexSL.Verify
open VerexIR VerexIR.Validation VerexIR.Proofs VerexIR.Proofs.Denote

private def forbiddenSubstrings : List String := [
  "exceeded its budget", "exceeded its heartbeat budget", "maxHeartbeats",
  "maximum number of heartbeats", "heartbeats", "stopped by the budget",
  "deterministic timeout", "(deterministic) timeout",
  "maximum recursion depth", "deep recursion",
  "unknown identifier", "unknown constant", "unknown function",
  "no Verex module declares", "no denotation for",
  "given verification is unsupported", "given verification concrete transfer failed",
  "pragma verify = manual",
  "type mismatch", "application type mismatch", "failed to synthesize",
  "unsolved goals", "no goals to be solved",
  "kernel", "PANIC at", "internal exception"
]

private def assertRejectedAtInvariant (base : Name) (command : TSyntax `command)
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
    throwError "expected semantic rejection for `{label}` ({base}), got budget/infrastructure diagnostic: {bad}"
  let fn := base.getString!
  let summaryPrefix := s!"verex verification failed: the automatic verification of `{fn}` failed;"
  let isExpectedOther (text : String) : Bool :=
    text.startsWith summaryPrefix ||
    text == expectedDiag ||
    (text.startsWith "the specification clause " && text.endsWith " is not established")
  let hasSummary := errorTexts.any (·.startsWith summaryPrefix)
  let hasNamedInvariant := errorTexts.any (· == expectedDiag)
  let allExpected := errorTexts.all isExpectedOther
  unless !errorTexts.isEmpty && hasSummary && hasNamedInvariant && allExpected do
    throwError "expected `{label}` ({base}) to fail with `{expectedDiag}`, got: {errorTexts}"

private def assertPublishSolverDiag (command : TSyntax `command)
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
  let fresh := attempted.messages.toList.drop priorCount
  let freshErrors := fresh.filter (·.severity == .error)
  let hasHeartbeatTag := fresh.any (·.data.hasTag (· == `runtime.maxHeartbeats))
  let errorTexts := (← freshErrors.mapM fun message => message.data.toString) ++
    (thrown.map (·.2)).toList
  modify fun _ => saved
  if thrown.any (·.1) || hasHeartbeatTag then
    throwError "expected `.publish` rejection for `{label}`, got heartbeat/recursion limit: {errorTexts}"
  if let some bad := errorTexts.find? (fun text => forbiddenSubstrings.any (text.contains ·)) then
    throwError "expected `.publish` rejection for `{label}`, got budget/infrastructure diagnostic: {bad}"
  unless errorTexts == [expectedDiag] do
    throwError "expected `{label}` to fail with exact `[`{expectedDiag}`]`, got: {errorTexts}"

abbrev irPublishCertifyKind : VerexIR.Proofs.Denote.CertifyKind :=
  VerexIR.Proofs.Denote.CertifyKind.publish

abbrev irPublishProvenance : VerexIR.Proofs.Denote.Provenance :=
  VerexIR.Proofs.Denote.Provenance.publish

abbrev irCapHandle : StructHandle := ⟨⟨0⟩, 0⟩
abbrev irCapTy : NTy := .struct irCapHandle .nil (.cons .num .nil)
abbrev irCapLocalRow : NRow := .cons irCapTy .nil
abbrev irParamTy : NTy := .param 0
abbrev irParamLocalRow : NRow := .cons irParamTy .nil
abbrev irSnum (n : Int) : SpecInt .unbounded true := ⟨n, rfl⟩
abbrev irPublishAddr : String := "0x1"

@[reducible] def irCapCarriers (file : String) (startByte endByte : Nat) : Carriers :=
  { Carriers.ground with
    own := fun src _ raw =>
      if src == irCapHandle then
        match (@NTy.codec Carriers.ground irCapTy).decode? raw with
        | some row => Obligation file startByte endByte (row.1.val ≤ 100)
        | none => True
      else True }

@[vir_denote_norm] theorem irCapCarriers_own_encode (file : String) (startByte endByte : Nat)
    (row : @NTy.carrier (irCapCarriers file startByte endByte) irCapTy) :
    @Carriers.own (irCapCarriers file startByte endByte) irCapHandle .nil
      (@NTy.encode (irCapCarriers file startByte endByte) irCapTy row) =
      Obligation file startByte endByte (row.1.val ≤ 100) := by
  have hbeq : (irCapHandle == irCapHandle) = true := by decide
  have hdec : (@NTy.codec Carriers.ground irCapTy).decode?
      (@NTy.encode (irCapCarriers file startByte endByte) irCapTy row) = some row :=
    Codec.decode_encode _ _
  change (if (irCapHandle == irCapHandle) = true then
      match (@NTy.codec Carriers.ground irCapTy).decode?
        (@NTy.encode (irCapCarriers file startByte endByte) irCapTy row) with
      | some row => Obligation file startByte endByte (row.1.val ≤ 100)
      | none => True
    else True) = Obligation file startByte endByte (row.1.val ≤ 100)
  rw [hbeq, if_pos rfl, hdec]

def publishCapTerm (unit : ValidatedUnit) : Term unit .none irCapLocalRow .unit :=
  Term.globalPublish 1 .nil (.lit (τ := .address) irPublishAddr rfl) (.var (τ := irCapTy) .here)

def ghostStoreCapTerm (unit : ValidatedUnit) : Term unit .none irCapLocalRow .unit :=
  Term.ghostStore 1 .nil (.lit (τ := .address) irPublishAddr rfl) (.var (τ := irCapTy) .here)

def publishParamTerm (unit : ValidatedUnit) : Term unit .none irParamLocalRow .unit :=
  Term.globalPublish 1 .nil (.lit (τ := .address) irPublishAddr rfl) (.var (τ := irParamTy) .here)

def ghostStoreParamTerm (unit : ValidatedUnit) : Term unit .none irParamLocalRow .unit :=
  Term.ghostStore 1 .nil (.lit (τ := .address) irPublishAddr rfl) (.var (τ := irParamTy) .here)

verex module 0x42::global_publish_invariants where
  pragma verify = false

  struct Cap has Key where
    v : u64
  spec Cap where
    invariant v <= 100

  struct PlainCap has Key where
    v : u64

  -- Negative 1: freshly packing an invalid resource (`v := 101`) and publishing
  -- it to global storage is rejected at `.construction`.
  fun publish_pack_bad(s : &Signer) -> Unit :=
    move_to<Cap>(s, new Cap { v := 101 })
  spec publish_pack_bad where
    requires !exists<Cap>(s.address)
    modifies global<Cap>(s.address)
    aborts_if false

  -- Negative 2: mutating a valid local resource through `&mut Cap` to `101`
  -- before publishing is rejected at `.mutationEnd` when `r` settles.
  fun publish_mut_bad(s : &Signer) -> Unit := do
    let mut c := new Cap { v := 42 }
    let r := &mut c
    r.v := 101
    move_to<Cap>(s, c)
  spec publish_mut_bad where
    requires !exists<Cap>(s.address)
    modifies global<Cap>(s.address)
    aborts_if false

  -- Negative 3 (non-vacuity): invalid pack before `move_to` fails at `.construction`
  -- rather than pruning execution and vacuously discharging `ensures false`.
  fun publish_ensures_false(s : &Signer) -> Unit :=
    move_to<Cap>(s, new Cap { v := 101 })
  spec publish_ensures_false where
    requires !exists<Cap>(s.address)
    modifies global<Cap>(s.address)
    aborts_if false
    ensures false

  -- Negative 4: publishing parameter `c : Cap` supplies `c.v <= 100` (`validArgs`),
  -- not stronger (`<= 99`).
  fun publish_param_overclaim_bad(s : &Signer, c : Cap) -> Unit :=
    move_to<Cap>(s, c)
  spec publish_param_overclaim_bad where
    requires !exists<Cap>(s.address)
    modifies global<Cap>(s.address)
    aborts_if false
    ensures global<Cap>(s.address).v <= 99

  -- Positive 1: publishing a valid freshly packed resource (`v := 42`) establishes
  -- existence, field value, and data invariant in global storage.
  fun publish_ok(s : &Signer) -> Unit :=
    move_to<Cap>(s, new Cap { v := 42 })
  spec publish_ok where
    requires !exists<Cap>(s.address)
    modifies global<Cap>(s.address)
    aborts_if false
    ensures exists<Cap>(s.address)
    ensures global<Cap>(s.address).v == 42 && global<Cap>(s.address).v <= 100

  -- Positive 2: mutating a local resource within its invariant (`v := 99`)
  -- and publishing verifies cleanly.
  fun publish_mut_ok(s : &Signer) -> Unit := do
    let mut c := new Cap { v := 42 }
    let r := &mut c
    r.v := 99
    move_to<Cap>(s, c)
  spec publish_mut_ok where
    requires !exists<Cap>(s.address)
    modifies global<Cap>(s.address)
    aborts_if false
    ensures exists<Cap>(s.address)
    ensures global<Cap>(s.address).v == 99 && global<Cap>(s.address).v <= 100

  -- Positive 3: publishing a certified parameter `c : Cap` directly (without `pack`)
  -- establishes existence, field equality, and `global<Cap>(s.address).v <= 100`.
  fun publish_param_ok(s : &Signer, c : Cap) -> Unit :=
    move_to<Cap>(s, c)
  spec publish_param_ok where
    requires !exists<Cap>(s.address)
    modifies global<Cap>(s.address)
    aborts_if false
    ensures exists<Cap>(s.address)
    ensures global<Cap>(s.address).v == c.v && global<Cap>(s.address).v <= 100

  -- Positive 4: publishing an invariant-free resource (`PlainCap`) verifies cleanly.
  fun publish_plain_ok(s : &Signer, x : u64) -> Unit :=
    move_to<PlainCap>(s, new PlainCap { v := x })
  spec publish_plain_ok where
    requires !exists<PlainCap>(s.address)
    modifies global<PlainCap>(s.address)
    aborts_if false
    ensures exists<PlainCap>(s.address) && global<PlainCap>(s.address).v == x

verex module 0x42::ghost_store_invariants where
  pragma verify = false

  struct Cap has Copy, Drop, Store, Key where
    v : u64
  spec Cap where
    invariant v <= 100

  struct «Ghost$ghost_cap» has Copy, Drop, Store, Key where
    v : Cap
  spec «Ghost$ghost_cap» where
    pragma ghost_memory

  struct «Ghost$ghost_u64» has Copy, Drop, Store, Key where
    v : u64
  spec «Ghost$ghost_u64» where
    pragma ghost_memory

  -- Negative 1: updating ghost global memory with an invalid freshly packed `Cap`
  -- (`v := 101`) is rejected at `.construction` naming `` `invariant v <= 100` ``.
  fun ghost_store_pack_bad() -> Unit := do
    spec do
      update global<«Ghost$ghost_cap»>(@0x0).v := new Cap { v := 101 }
  spec ghost_store_pack_bad where
    modifies global<«Ghost$ghost_cap»>(@0x0)
    aborts_if false

  -- Negative 2 (non-vacuity): invalid ghost global store followed by `ensures false`
  -- fails at `.construction` rather than vacuously discharging `ensures false`.
  fun ghost_store_ensures_false() -> Unit := do
    spec do
      update global<«Ghost$ghost_cap»>(@0x0).v := new Cap { v := 101 }
  spec ghost_store_ensures_false where
    modifies global<«Ghost$ghost_cap»>(@0x0)
    aborts_if false
    ensures false

  -- Negative 3: storing parameter `c : Cap` into ghost global memory supplies
  -- `c.v <= 100` (`validArgs`), not stronger (`<= 99`).
  fun ghost_store_param_overclaim_bad(c : Cap) -> Unit := do
    spec do
      update global<«Ghost$ghost_cap»>(@0x0).v := c
  spec ghost_store_param_overclaim_bad where
    modifies global<«Ghost$ghost_cap»>(@0x0)
    aborts_if false
    ensures global<«Ghost$ghost_cap»>(@0x0).v.v <= 99

  -- Positive 1: storing a valid freshly packed `Cap` (`v := n` under `n <= 100`)
  -- into ghost global memory verifies and establishes value + data invariant.
  fun ghost_store_ok(n : u64) -> Unit := do
    spec do
      update global<«Ghost$ghost_cap»>(@0x0).v := new Cap { v := n }
  spec ghost_store_ok where
    requires n <= 100
    modifies global<«Ghost$ghost_cap»>(@0x0)
    aborts_if false
    ensures global<«Ghost$ghost_cap»>(@0x0).v.v == n
    ensures global<«Ghost$ghost_cap»>(@0x0).v.v <= 100

  -- Positive 2: storing a certified parameter `c : Cap` into ghost global memory
  -- verifies and preserves `c`'s data invariant.
  fun ghost_store_param_ok(c : Cap) -> Unit := do
    spec do
      update global<«Ghost$ghost_cap»>(@0x0).v := c
  spec ghost_store_param_ok where
    modifies global<«Ghost$ghost_cap»>(@0x0)
    aborts_if false
    ensures global<«Ghost$ghost_cap»>(@0x0).v.v == c.v
    ensures global<«Ghost$ghost_cap»>(@0x0).v.v <= 100

  -- Positive 3: storing an invariant-free `u64` into ghost global memory verifies cleanly.
  fun ghost_store_u64_ok(x : u64) -> Unit := do
    spec do
      update global<«Ghost$ghost_u64»>(@0x0).v := x
  spec ghost_store_u64_ok where
    modifies global<«Ghost$ghost_u64»>(@0x0)
    aborts_if false
    ensures global<«Ghost$ghost_u64»>(@0x0).v == x

/-! ## 1. Negative Controls (First) -/

run_cmd do
  let env ← getEnv
  let curNs ← getCurrNamespace
  -- 1a. Unconditional direct `.publish` solver diagnostic controls.
  unless irPublishCertifyKind == VerexIR.Proofs.Denote.CertifyKind.publish &&
      irPublishProvenance.describe == "a data invariant where a resource is published" do
    throwError "unexpected CertifyKind.publish / Provenance.publish description"
  let some unit := (VerexSL.registeredUnit? env `«0x42».global_publish_invariants).orElse
      (fun _ => VerexSL.registeredUnit? env (curNs ++ `«0x42».global_publish_invariants))
    | throwError "missing registered unit `0x42::global_publish_invariants`"
  let some ns := unit.namespaces[0]?
    | throwError "missing namespace 0 in `0x42::global_publish_invariants`"
  let some capDecl := ns.structs.find? (fun d => (ns.nameOf d.name).any (·.name == "Cap"))
    | throwError "missing `Cap` declaration in `0x42::global_publish_invariants`"
  let some invCond := capDecl.contract.conditions.find? (·.kind == .structInvariant)
    | throwError "missing struct invariant on `Cap`"
  let range := VerexSL.Contract.locRange ns invCond.loc
  let fileLit := Syntax.mkStrLit range.file
  let startLit := Syntax.mkNumLit (toString range.startByte)
  let endLit := Syntax.mkNumLit (toString range.endByte)

  assertPublishSolverDiag
    (← `(command|
      example (u : VerexIR.Validation.ValidatedUnit) (State : Type)
          (ops : VerexIR.Proofs.Denote.InterpreterOps u
            (irCapCarriers $fileLit $startLit $endLit) State)
          (initial : State)
          (hAbsent : ops.read irCapTy .nil (.address irPublishAddr) initial = none) :
          VerexIR.Proofs.wp
            (@VerexIR.Proofs.Denote.Term.denoteIn u
              (irCapCarriers $fileLit $startLit $endLit) State ops
              .none irCapLocalRow .unit (publishCapTerm u)
              (some (irSnum 101, ()), ()))
            (fun _ _ => True) (fun _ => False) initial := by
        letI : VerexIR.Proofs.Denote.Carriers := irCapCarriers $fileLit $startLit $endLit
        dsimp only [publishCapTerm, VerexIR.Proofs.Denote.Term.denoteIn,
          VerexIR.Proofs.Denote.Flow.bind, VerexIR.Proofs.Denote.anchorAt,
          VerexIR.Proofs.Denote.Var.get, VerexIR.Proofs.Denote.NTy.ofGround,
          VerexIR.Proofs.Denote.NTy.storageKey]
        simp only [VerexIR.Proofs.Spec.pure_bind]
        rw [VerexIR.Proofs.wp_bind_get, hAbsent]
        dsimp only
        rw [VerexIR.Proofs.wp_bind]
        verex_solve_goals []))
    "the data invariant `invariant v <= 100` does not hold where a resource is published"
    "Term.globalPublish uncertified local with source obligation range"

  assertPublishSolverDiag
    (← `(command|
      example (u : VerexIR.Validation.ValidatedUnit) (State : Type)
          (ops : VerexIR.Proofs.Denote.InterpreterOps u
            (irCapCarriers $fileLit $startLit $endLit) State)
          (initial : State) :
          VerexIR.Proofs.wp
            (@VerexIR.Proofs.Denote.Term.denoteIn u
              (irCapCarriers $fileLit $startLit $endLit) State ops
              .none irCapLocalRow .unit (ghostStoreCapTerm u)
              (some (irSnum 101, ()), ()))
            (fun _ _ => True) (fun _ => False) initial := by
        letI : VerexIR.Proofs.Denote.Carriers := irCapCarriers $fileLit $startLit $endLit
        dsimp only [ghostStoreCapTerm, VerexIR.Proofs.Denote.Term.denoteIn,
          VerexIR.Proofs.Denote.Flow.bind, VerexIR.Proofs.Denote.anchorAt,
          VerexIR.Proofs.Denote.Var.get, VerexIR.Proofs.Denote.NTy.ofGround,
          VerexIR.Proofs.Denote.NTy.storageKey]
        simp only [VerexIR.Proofs.Spec.pure_bind]
        rw [VerexIR.Proofs.wp_bind_get, VerexIR.Proofs.wp_bind]
        verex_solve_goals []))
    "the data invariant `invariant v <= 100` does not hold where a resource is published"
    "Term.ghostStore uncertified local with source obligation range"

  assertPublishSolverDiag
    (← `(command|
      example (C : VerexIR.Proofs.Denote.Carriers)
          (u : VerexIR.Validation.ValidatedUnit) (State : Type)
          (ops : VerexIR.Proofs.Denote.InterpreterOps u C State)
          (unheld : @VerexIR.Proofs.Denote.NTy.carrier C irParamTy)
          (initial : State)
          (hAbsent : ops.read irParamTy .nil (.address irPublishAddr) initial = none) :
          VerexIR.Proofs.wp
            (@VerexIR.Proofs.Denote.Term.denoteIn u C State ops
              .none irParamLocalRow .unit (publishParamTerm u)
              (some unheld, ()))
            (fun _ _ => True) (fun _ => False) initial := by
        letI : VerexIR.Proofs.Denote.Carriers := C
        dsimp only [publishParamTerm, VerexIR.Proofs.Denote.Term.denoteIn,
          VerexIR.Proofs.Denote.Flow.bind, VerexIR.Proofs.Denote.anchorAt,
          VerexIR.Proofs.Denote.Var.get, VerexIR.Proofs.Denote.NTy.ofGround,
          VerexIR.Proofs.Denote.NTy.storageKey]
        simp only [VerexIR.Proofs.Spec.pure_bind]
        rw [VerexIR.Proofs.wp_bind_get, hAbsent]
        dsimp only
        rw [VerexIR.Proofs.wp_bind]
        verex_solve_goals []))
    "a data invariant where a resource is published is not established"
    "Term.globalPublish uncertified local without source obligation range"

  assertPublishSolverDiag
    (← `(command|
      example (C : VerexIR.Proofs.Denote.Carriers)
          (u : VerexIR.Validation.ValidatedUnit) (State : Type)
          (ops : VerexIR.Proofs.Denote.InterpreterOps u C State)
          (unheld : @VerexIR.Proofs.Denote.NTy.carrier C irParamTy)
          (initial : State) :
          VerexIR.Proofs.wp
            (@VerexIR.Proofs.Denote.Term.denoteIn u C State ops
              .none irParamLocalRow .unit (ghostStoreParamTerm u)
              (some unheld, ()))
            (fun _ _ => True) (fun _ => False) initial := by
        letI : VerexIR.Proofs.Denote.Carriers := C
        dsimp only [ghostStoreParamTerm, VerexIR.Proofs.Denote.Term.denoteIn,
          VerexIR.Proofs.Denote.Flow.bind, VerexIR.Proofs.Denote.anchorAt,
          VerexIR.Proofs.Denote.Var.get, VerexIR.Proofs.Denote.NTy.ofGround,
          VerexIR.Proofs.Denote.NTy.storageKey]
        simp only [VerexIR.Proofs.Spec.pure_bind]
        rw [VerexIR.Proofs.wp_bind_get, VerexIR.Proofs.wp_bind]
        verex_solve_goals []))
    "a data invariant where a resource is published is not established"
    "Term.ghostStore uncertified local without source obligation range"

  -- 1b. Source-level `globalPublish` (`move_to`) negative controls.
  assertRejectedAtInvariant `«0x42».global_publish_invariants.publish_pack_bad
    (← `(command| verify 0x42::global_publish_invariants::publish_pack_bad))
    "the data invariant `invariant v <= 100` does not hold where a value is constructed"
    "globalPublish freshly packed invalid Cap"
  assertRejectedAtInvariant `«0x42».global_publish_invariants.publish_mut_bad
    (← `(command| verify 0x42::global_publish_invariants::publish_mut_bad))
    "the data invariant `invariant v <= 100` does not hold where a mutation ends"
    "globalPublish locally mutated invalid Cap"
  assertRejectedAtInvariant `«0x42».global_publish_invariants.publish_ensures_false
    (← `(command| verify 0x42::global_publish_invariants::publish_ensures_false))
    "the data invariant `invariant v <= 100` does not hold where a value is constructed"
    "globalPublish non-vacuity control (ensures false)"
  assertRejectedAtInvariant `«0x42».global_publish_invariants.publish_param_overclaim_bad
    (← `(command| verify 0x42::global_publish_invariants::publish_param_overclaim_bad))
    "the specification clause `ensures global<Cap>(s.address).v <= 99` is not established"
    "globalPublish parameter Cap overclaim (<= 99)"

  -- 1c. Source-level `ghostStore` (`pragma ghost_memory`) negative controls.
  assertRejectedAtInvariant `«0x42».ghost_store_invariants.ghost_store_pack_bad
    (← `(command| verify 0x42::ghost_store_invariants::ghost_store_pack_bad))
    "the data invariant `invariant v <= 100` does not hold where a value is constructed"
    "ghostStore freshly packed invalid Cap"
  assertRejectedAtInvariant `«0x42».ghost_store_invariants.ghost_store_ensures_false
    (← `(command| verify 0x42::ghost_store_invariants::ghost_store_ensures_false))
    "the data invariant `invariant v <= 100` does not hold where a value is constructed"
    "ghostStore non-vacuity control (ensures false)"
  assertRejectedAtInvariant `«0x42».ghost_store_invariants.ghost_store_param_overclaim_bad
    (← `(command| verify 0x42::ghost_store_invariants::ghost_store_param_overclaim_bad))
    "the specification clause `ensures global<«Ghost$ghost_cap»>(@0x0).v.v <= 99` is not established"
    "ghostStore parameter Cap overclaim (<= 99)"

/-! ## 2. Positive Controls (Second) -/

theorem provenance_publish_describe :
    VerexIR.Proofs.Denote.Provenance.publish.describe =
      "a data invariant where a resource is published" := rfl

theorem checkingAt_publish_eq (State : Type) [VerexIR.Proofs.Denote.Carriers]
    (site : Nat) (τ : NTy) (value : τ.carrier) :
    VerexIR.Proofs.Denote.checkingAt (State := State) site
      VerexIR.Proofs.Denote.CertifyKind.publish τ value =
      VerexIR.Proofs.Denote.checking (State := State) τ value := rfl

run_cmd do
  let env ← getEnv
  let curNs ← getCurrNamespace
  let some unit := (VerexSL.registeredUnit? env `«0x42».global_publish_invariants).orElse
      (fun _ => VerexSL.registeredUnit? env (curNs ++ `«0x42».global_publish_invariants))
    | throwError "missing registered unit `0x42::global_publish_invariants`"
  let some ns := unit.namespaces[0]?
    | throwError "missing namespace 0 in `0x42::global_publish_invariants`"
  let some capDecl := ns.structs.find? (fun d => (ns.nameOf d.name).any (·.name == "Cap"))
    | throwError "missing `Cap` declaration in `0x42::global_publish_invariants`"
  let some invCond := capDecl.contract.conditions.find? (·.kind == .structInvariant)
    | throwError "missing struct invariant on `Cap`"
  let range := VerexSL.Contract.locRange ns invCond.loc
  let fileLit := Syntax.mkStrLit range.file
  let startLit := Syntax.mkNumLit (toString range.startByte)
  let endLit := Syntax.mkNumLit (toString range.endByte)

  elabCommand (← `(command|
    example (u : VerexIR.Validation.ValidatedUnit) (State : Type)
        (ops : VerexIR.Proofs.Denote.InterpreterOps u
          (irCapCarriers $fileLit $startLit $endLit) State)
        (initial : State)
        (hAbsent : ops.read irCapTy .nil (.address irPublishAddr) initial = none) :
        VerexIR.Proofs.wp
          (@VerexIR.Proofs.Denote.Term.denoteIn u
            (irCapCarriers $fileLit $startLit $endLit) State ops
            .none irCapLocalRow .unit (publishCapTerm u)
            (some (irSnum 42, ()), ()))
          (fun _ _ => True) (fun _ => False) initial := by
      letI : VerexIR.Proofs.Denote.Carriers := irCapCarriers $fileLit $startLit $endLit
      dsimp only [publishCapTerm, VerexIR.Proofs.Denote.Term.denoteIn,
        VerexIR.Proofs.Denote.Flow.bind, VerexIR.Proofs.Denote.anchorAt,
        VerexIR.Proofs.Denote.Var.get, VerexIR.Proofs.Denote.NTy.ofGround,
        VerexIR.Proofs.Denote.NTy.storageKey]
      simp only [VerexIR.Proofs.Spec.pure_bind]
      rw [VerexIR.Proofs.wp_bind_get, hAbsent]
      dsimp only
      rw [VerexIR.Proofs.wp_bind]
      verex_solve_goals []))

  elabCommand (← `(command|
    example (u : VerexIR.Validation.ValidatedUnit) (State : Type)
        (ops : VerexIR.Proofs.Denote.InterpreterOps u
          (irCapCarriers $fileLit $startLit $endLit) State)
        (initial : State) :
        VerexIR.Proofs.wp
          (@VerexIR.Proofs.Denote.Term.denoteIn u
            (irCapCarriers $fileLit $startLit $endLit) State ops
            .none irCapLocalRow .unit (ghostStoreCapTerm u)
            (some (irSnum 42, ()), ()))
          (fun _ _ => True) (fun _ => False) initial := by
      letI : VerexIR.Proofs.Denote.Carriers := irCapCarriers $fileLit $startLit $endLit
      dsimp only [ghostStoreCapTerm, VerexIR.Proofs.Denote.Term.denoteIn,
        VerexIR.Proofs.Denote.Flow.bind, VerexIR.Proofs.Denote.anchorAt,
        VerexIR.Proofs.Denote.Var.get, VerexIR.Proofs.Denote.NTy.ofGround,
        VerexIR.Proofs.Denote.NTy.storageKey]
      simp only [VerexIR.Proofs.Spec.pure_bind]
      rw [VerexIR.Proofs.wp_bind_get, VerexIR.Proofs.wp_bind]
      verex_solve_goals []))

verify 0x42::global_publish_invariants::publish_ok
verify 0x42::global_publish_invariants::publish_mut_ok
verify 0x42::global_publish_invariants::publish_param_ok
verify 0x42::global_publish_invariants::publish_plain_ok

run_cmd liftTermElabM do
  let env ← getEnv
  let curNs ← getCurrNamespace
  for fn in [`publish_ok, `publish_mut_ok, `publish_param_ok, `publish_plain_ok] do
    for suffix in [`typedVerified, `verified] do
      let thm := if env.contains (curNs ++ `«0x42».global_publish_invariants ++ fn ++ suffix) then
          curNs ++ `«0x42».global_publish_invariants ++ fn ++ suffix
        else `«0x42».global_publish_invariants ++ fn ++ suffix
      unless env.contains thm do
        throwError "missing verification theorem {thm}"
      let axioms ← collectAxioms thm
      let allowed := if suffix == `verified then
          [``propext, ``Classical.choice, ``Quot.sound, ``VerexIR.Proofs.Denote.compileFunction_agrees]
        else [``propext, ``Classical.choice, ``Quot.sound]
      unless axioms.all (allowed.contains ·) do
        throwError "unexpected axioms in {thm}: {axioms}"

verify 0x42::ghost_store_invariants::ghost_store_ok
verify 0x42::ghost_store_invariants::ghost_store_param_ok
verify 0x42::ghost_store_invariants::ghost_store_u64_ok

run_cmd liftTermElabM do
  let env ← getEnv
  let curNs ← getCurrNamespace
  for fn in [`ghost_store_ok, `ghost_store_param_ok, `ghost_store_u64_ok] do
    let thm := if env.contains (curNs ++ `«0x42».ghost_store_invariants ++ fn ++ `typedVerified) then
        curNs ++ `«0x42».ghost_store_invariants ++ fn ++ `typedVerified
      else `«0x42».ghost_store_invariants ++ fn ++ `typedVerified
    unless env.contains thm do
      throwError "missing model verification theorem {thm}"
    let axioms ← collectAxioms thm
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {thm}: {axioms}"

end VerexSL.Tests.GlobalPublishInvariants
