-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Check

namespace VerexIR.Tests.ReceiverNoEscape
open VerexIR VerexIR.Import VerexIR.Validation

private def use (id : Nat) : TypeUse := { typeId := ⟨id⟩, loc := ⟨0⟩ }
private def declaration (name root result : Nat) (parameters : Array Nat) : FunctionDecl RawBody := {
  loc := ⟨0⟩, name := ⟨name⟩, profile := .rust
  signature := {
    parameters := parameters.zipIdx.map fun (type, index) =>
      { name := s!"p{index}", typeUse := use type, mutable := true }
    results := #[use result] }
  locals := parameters.zipIdx.map fun (type, index) =>
    { id := ⟨index⟩, name := s!"p{index}", type := use type, mutable := true, loc := ⟨0⟩ }
  body := .structured ⟨root⟩, origin := ⟨0⟩, alignment := ⟨0⟩ }

private def fixture : RawUnit := {
  tables := {
    files := #[{ name := "receiver_noescape.rs" }]
    locations := #[{ primary := some { file := ⟨0⟩, startByte := 0, endByte := 1 } }]
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "receiver refinement" }]
    lifetimes := #[{ kind := .inference, loc := ⟨0⟩ }]
    types := #[.bool, .reference { profile := .rust, kind := .mutable, referent := ⟨0⟩, lifetime := ⟨0⟩ }, .unit,
      .reference { profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨0⟩ }]
    namespaces := #[{ segments := #["receiver_noescape"] }]
    names := #["identity", "scalar", "reborrow", "writeback", "mutate"].map
      fun name => { namespaceId := ⟨0⟩, name } }
  profiles := #[{ profile := .rust, name := "rust-test" }]
  namespaces := #[{
    loc := ⟨0⟩, identity := ⟨0⟩, profile := some .rust
    places := #[.localVar ⟨0⟩, .deref ⟨0⟩, .localVar ⟨1⟩, .deref ⟨2⟩]
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .localVar ⟨0⟩ },
      { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.bool true) },
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .operation (.borrow .mutable ⟨1⟩) #[] #[] },
      { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .operation (.write ⟨3⟩) #[] #[⟨0⟩] },
      { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .operation (.write ⟨1⟩) #[] #[⟨1⟩] }]
    functions := #[declaration 0 0 1 #[1], declaration 1 1 0 #[1],
      declaration 2 2 1 #[1], declaration 3 3 2 #[1, 3], declaration 4 4 2 #[1]] }] }

private def checked := validate #[{ profile := .rust, name := "rust-test" }] fixture

#guard checked.isOk
#guard match checked with
  | .ok unit =>
      let ns := unit.namespaces[0]!
      -- Returning the original receiver or a fresh reborrow is refused;
      -- storing it through another parameter is refused even with unit result.
      (ns.functions.map (Validation.ReceiverNoEscape.checkedParameters unit ns)) ==
        #[#[], #[0], #[], #[1], #[0]]
  | .error _ => false

private def requirements : Array Attribute := #[
  .assign "method_evidence" (.constant (.integer 1)),
  .assign "scoped_method_evidence" (.constant (.integer 1))]

private def suppliedFixture (forwarded marked : Bool) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let runner := { declaration 5 6 1 #[1, 4] with attributes := requirements }
  let caller := { declaration 6 8 1 (if forwarded then #[1, 4] else #[1]) with
    attributes := if marked then requirements else #[] }
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.function #[⟨1⟩] ⟨1⟩ #[.copy, .drop])
      names := fixture.tables.names ++ #[
        { namespaceId := ⟨0⟩, name := "runner" },
        { namespaceId := ⟨0⟩, name := "caller" }] }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨4⟩, kind := .localVar ⟨1⟩ },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .operation (.call .invoke) #[] #[⟨5⟩, ⟨0⟩] },
        { loc := ⟨0⟩, typeId := ⟨4⟩, kind := .operation
            (.call (.closure { namespaceId := ⟨0⟩, name := ⟨0⟩ } 0)) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .operation
            (.call (.function { namespaceId := ⟨0⟩, name := ⟨5⟩ })) #[]
              #[⟨0⟩, if forwarded then ⟨5⟩ else ⟨7⟩] }]
      functions := ns.functions ++ #[runner, caller] }] }

private def checkSupplied (forwarded marked : Bool) :=
  validate #[{ profile := .rust, name := "rust-test" }] (suppliedFixture forwarded marked)

-- A literal returning its receiver cannot forge the scoped requirement.
#guard match checkSupplied false false with
  | .error errors => errors.any (·.code == "VIR-SCOPED-METHOD-EVIDENCE")
  | .ok _ => false
-- Structural function type alone cannot establish a forwarded refinement.
#guard match checkSupplied true false with
  | .error errors => errors.any (·.code == "VIR-SCOPED-METHOD-EVIDENCE")
  | .ok _ => false
-- An explicitly scoped, unmodified dictionary can be forwarded.
#guard (checkSupplied true true).isOk

private def overwrittenFixture : RawUnit :=
  let raw := suppliedFixture true true
  let ns := raw.namespaces[0]!
  { raw with namespaces := #[{ ns with
    expressions := ns.expressions ++ #[
      { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .assign ⟨2⟩ ⟨7⟩ },
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .block #[⟨9⟩] (some ⟨8⟩) }]
    functions := ns.functions.set! 6 { ns.functions[6]! with body := .structured ⟨10⟩ } }] }

-- A marker does not survive replacement with an arbitrary dictionary.
#guard match validate #[{ profile := .rust, name := "rust-test" }] overwrittenFixture with
  | .error errors => errors.any (·.code == "VIR-SCOPED-METHOD-EVIDENCE")
  | .ok _ => false

private def aliasedFixture : RawUnit :=
  let raw := suppliedFixture true true
  let ns := raw.namespaces[0]!
  let caller := ns.functions[6]!
  { raw with namespaces := #[{ ns with
    patterns := #[{ loc := ⟨0⟩, typeId := ⟨4⟩, kind := .variable ⟨2⟩ }]
    expressions := ns.expressions ++ #[
      { loc := ⟨0⟩, typeId := ⟨4⟩, kind := .localVar ⟨2⟩ },
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .operation
            (.call (.function { namespaceId := ⟨0⟩, name := ⟨5⟩ })) #[] #[⟨0⟩, ⟨9⟩] },
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .letDecl ⟨0⟩ (some ⟨5⟩) ⟨10⟩ }]
    functions := ns.functions.set! 6 { caller with
      body := .structured ⟨11⟩
      locals := caller.locals.push {
        id := ⟨2⟩, name := "alias", type := use 4, mutable := false, loc := ⟨0⟩ } } }] }

#guard (validate #[{ profile := .rust, name := "rust-test" }] aliasedFixture).isOk

-- Check the independent analysis's fail-closed cases directly against checked
-- tables; these cases must not acquire a receipt from result-region filtering.
#guard match checked with
  | .error _ => false
  | .ok unit =>
      let ns := unit.namespaces[0]!
      let ns := { ns with expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .operation
            (.call (.function { namespaceId := ⟨0⟩, name := ⟨0⟩ })) #[] #[⟨0⟩] },
        { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .break_ 0 (some ⟨0⟩) },
        { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .block #[⟨6⟩] none },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .loop none ⟨7⟩ }] }
      !Validation.ReceiverNoEscape.checks unit 10 ns
        { ns.functions[0]! with body := .structured ⟨5⟩ } 0 &&
      !Validation.ReceiverNoEscape.checks unit 10 ns
        { ns.functions[0]! with body := .structured ⟨8⟩ } 0

private def graphFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let call := fun (name : Nat) => ({
    loc := ⟨0⟩, typeId := ⟨0⟩
    kind := .operation (.call (.function { namespaceId := ⟨0⟩, name := ⟨name⟩ }))
      #[] #[⟨0⟩] } : Expr)
  { fixture with namespaces := #[{ ns with
    expressions := ns.expressions ++ #[call 0, call 1, call 2,
      { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .block #[⟨6⟩] (some ⟨7⟩) }, call 4]
    functions := #[declaration 0 1 0 #[1], declaration 1 5 0 #[1],
      declaration 2 5 0 #[1], declaration 3 8 0 #[1], declaration 4 9 0 #[1]] }] }

-- A repeated diamond has one summary per function. The cycle cannot assume
-- its own noescape guarantee, even though its result type is scalar.
#guard match validate #[{ profile := .rust, name := "rust-test" }] graphFixture with
  | .error _ => false
  | .ok unit =>
      let summaries := Validation.ReceiverNoEscape.summaries unit
      summaries.size == 5 &&
        (Array.range 5).map (fun index =>
          Validation.ReceiverNoEscape.summaryParameters summaries ⟨0⟩ ⟨index⟩) ==
          #[#[0], #[0], #[0], #[0], #[]]

-- Place indices are outside expressionChildren. A hidden side-effecting
-- expression there must fail closed, even for a scalar read or a drop.
#guard match checked with
  | .error _ => false
  | .ok unit =>
      let ns := unit.namespaces[0]!
      let ns := { ns with
        places := ns.places.push (.index ⟨0⟩ ⟨3⟩)
        expressions := ns.expressions ++ #[
          { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation (.read ⟨4⟩) #[] #[] },
          { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .operation (.drop ⟨4⟩) #[] #[] }] }
      !Validation.ReceiverNoEscape.checks unit 10 ns
        { ns.functions[1]! with body := .structured ⟨5⟩ } 0 &&
      !Validation.ReceiverNoEscape.checks unit 10 ns
        { ns.functions[4]! with body := .structured ⟨6⟩ } 0

private def borrowedDictionaryFixture : RawUnit :=
  let raw := suppliedFixture true true
  let ns := raw.namespaces[0]!
  { raw with
    tables := { raw.tables with
      types := raw.tables.types.push
        (.reference { profile := .rust, kind := .mutable, referent := ⟨4⟩, lifetime := ⟨0⟩ }) }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨5⟩, kind := .operation (.borrow .mutable ⟨2⟩) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .block #[⟨9⟩] (some ⟨8⟩) }]
      functions := ns.functions.set! 6 { ns.functions[6]! with body := .structured ⟨10⟩ } }] }

-- An address-taken dictionary may have been overwritten through an alias;
-- the initial bounded gate does not preserve its refinement.
#guard match validate #[{ profile := .rust, name := "rust-test" }] borrowedDictionaryFixture with
  | .error errors => errors.any (·.code == "VIR-SCOPED-METHOD-EVIDENCE")
  | .ok _ => false

private def reboundDictionaryFixture : RawUnit :=
  let raw := suppliedFixture true true
  let ns := raw.namespaces[0]!
  { raw with namespaces := #[{ ns with
    patterns := #[{ loc := ⟨0⟩, typeId := ⟨4⟩, kind := .variable ⟨1⟩ }]
    expressions := ns.expressions.push {
      loc := ⟨0⟩, typeId := ⟨1⟩, kind := .letDecl ⟨0⟩ (some ⟨7⟩) ⟨8⟩ }
    functions := ns.functions.set! 6 { ns.functions[6]! with body := .structured ⟨9⟩ } }] }

#guard match validate #[{ profile := .rust, name := "rust-test" }] reboundDictionaryFixture with
  | .error errors => errors.any (·.code == "VIR-SCOPED-METHOD-EVIDENCE")
  | .ok _ => false

-- A native intrinsic may execute instead of its apparently harmless source
-- body. Never use that source body as a noescape receipt for the override.
#guard match checked with
  | .error _ => false
  | .ok unit =>
      let ns := unit.namespaces[0]!
      let ns := { ns with intrinsics := #[{
        loc := ⟨0⟩, owner := ⟨0⟩, profile := .rust, model := "map"
        executableBindings := #[{
          loc := ⟨0⟩, role := "map_borrow_mut", target := { namespaceId := ⟨0⟩, name := ⟨1⟩ } }] }] }
      !Validation.ReceiverNoEscape.checks unit 10 ns ns.functions[1]! 0

private def otherArgumentFixture (incoming : Bool) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let consumer := declaration 0 8 1 #[0, if incoming then 1 else 0, 3]
  { fixture with
    tables := { fixture.tables with
      locations := fixture.tables.locations.push {
        primary := some { file := ⟨0⟩, startByte := 2, endByte := 3 } }
      types := #[.bool, fixture.tables.types[1]!, .unit,
        .function #[⟨1⟩, ⟨1⟩] ⟨1⟩ #[.copy, .drop]]
      names := #["retain_other", "return_other"].map fun name => { namespaceId := ⟨0⟩, name } }
    namespaces := #[{ ns with
      patterns := #[{ loc := ⟨0⟩, typeId := ⟨1⟩, kind := .variable ⟨3⟩ }]
      places := #[.localVar ⟨0⟩, .localVar ⟨1⟩, .localVar ⟨2⟩, .localVar ⟨3⟩]
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .operation (.borrow .mutable ⟨0⟩) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := if incoming then .localVar ⟨1⟩
            else .operation (.borrow .mutable ⟨1⟩) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨3⟩, kind := .localVar ⟨2⟩ },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .operation (.call .invoke) #[] #[⟨2⟩, ⟨0⟩, ⟨1⟩] },
        { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .operation (.drop ⟨0⟩) #[] #[] },
        { loc := ⟨1⟩, typeId := ⟨2⟩, kind := if incoming then .value .unit
            else .operation (.drop ⟨1⟩) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .localVar ⟨3⟩ },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .block #[⟨4⟩, ⟨5⟩] (some ⟨6⟩) },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .letDecl ⟨0⟩ (some ⟨3⟩) ⟨7⟩ },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .localVar ⟨1⟩ }]
      functions := #[
        { consumer with
          attributes := #[.assign "method_evidence" (.constant (.integer 2)),
            .assign "scoped_method_evidence" (.constant (.integer 2))]
          locals := consumer.locals.push {
            id := ⟨3⟩, name := "result", type := use 1, mutable := false, loc := ⟨0⟩ } },
        declaration 1 9 1 #[1, 1]] }] }

private def prepareOtherArgument (incoming : Bool) : Except (Array Diagnostic) Unit := do
  let unit ← validate #[{ profile := .rust, name := "rust-test" }] (otherArgumentFixture incoming)
  discard <| prepareExecution #[{ profile := .rust, name := "rust-test", classify := fun _ _ => none }] unit

-- The callback is permitted to return its other argument: only the receiver
-- slot earns a receipt. Dropping the private receiver is safe when that other
-- argument is an incoming borrow whose lender remains outside this frame.
#guard match validate #[{ profile := .rust, name := "rust-test" }] (otherArgumentFixture true) with
  | .error _ => false
  | .ok unit => unit.namespaces[0]!.receiverNoEscape ⟨1⟩ == #[0]
#guard (prepareOtherArgument true).isOk

-- A distinct local lender stays borrowed through the returned reference. The
-- scoped receiver refinement must not free its loan when the invocation ends.
#guard match prepareOtherArgument false with
  | .error errors => errors.any (fun error =>
      error.code == "VIR-SEMANTIC-BORROW-CONFLICT" && error.primary == some ⟨1⟩)
  | .ok _ => false

-- The expression arena is a DAG, not a tree. Each level shares its two
-- branches and adds a nested let: recursive re-evaluation would take
-- exponential time. Both cases have a reference receiver, so skipping
-- scalar-only declarations cannot make this regression pass.
private def deepSharedReceipt (escaping : Bool) : Bool := Id.run do
  let .ok unit := checked | return false
  let original := unit.namespaces[0]!
  let mut expressions := original.expressions
  let mut root : ExprId := if escaping then ⟨0⟩ else ⟨1⟩
  let typeId : TypeId := if escaping then ⟨1⟩ else ⟨0⟩
  for _ in [:64] do
    let branch : ExprId := ⟨expressions.size⟩
    expressions := expressions.push {
      loc := ⟨0⟩, typeId, kind := .ifElse ⟨1⟩ root (some root) }
    root := ⟨expressions.size⟩
    expressions := expressions.push {
      loc := ⟨0⟩, typeId, kind := .letDecl ⟨0⟩ none branch }
  let ns := { original with
    expressions := expressions
    patterns := #[{ loc := ⟨0⟩, typeId := ⟨2⟩, kind := .wildcard }] }
  let function := { original.functions[if escaping then 0 else 1]! with
    body := .structured root }
  return Validation.ReceiverNoEscape.checks unit 10 ns function 0

#guard deepSharedReceipt false
#guard !deepSharedReceipt true

end VerexIR.Tests.ReceiverNoEscape
