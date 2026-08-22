-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Verify.SourceProgram

/-!
# Poison-aware source borrow checking

This module is deliberately independent of compiler IR.  It is the small,
executable policy core used to check the typed borrow view retained from Move
source.  Mutable handles are inert until their first destructive use.  Such a
use activates the handle and poisons competing overlapping handles; failure is
reported only if a poisoned handle is subsequently used.

The certificate checker replays every transfer.  Certificates contain useful
analysis facts, but no trusted Boolean saying that an access-path comparison
or state transition was valid.
-/

namespace Move.Verify.Borrow

instance [DecidableEq ε] [DecidableEq α] : DecidableEq (Except ε α)
  | .error left, .error right =>
      if h : left = right then isTrue (by cases h; rfl)
      else isFalse fun equality => h (Except.error.inj equality)
  | .ok left, .ok right =>
      if h : left = right then isTrue (by cases h; rfl)
      else isFalse fun equality => h (Except.ok.inj equality)
  | .error _, .ok _ | .ok _, .error _ => isFalse nofun

/-- Roots of source access-path trees.  Reference parameters may alias one
another; local roots cannot, and globals alias at resource-family granularity. -/
inductive Root where
  | local (name : String)
  | global (family : String)
  | parameter (index : Nat)
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

/-- Vector indices are intentionally collapsed. -/
inductive Step where
  | field (name : String)
  | anyIndex
  | unknownSuffix
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure Place where
  root : Root
  path : Array Step := #[]
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

def Root.mayShareTree : Root → Root → Bool
  | .local left, .local right => left == right
  | .global left, .global right => left == right
  | .parameter _, .parameter _ => true
  | _, _ => false

private def Step.compatible : Step → Step → Bool
  | .unknownSuffix, _ | _, .unknownSuffix => true
  | .anyIndex, .anyIndex => true
  | .field left, .field right => left == right
  | _, _ => false

/-- Prefix overlap in one access-path tree. -/
def Place.overlaps (left right : Place) : Bool :=
  if !left.root.mayShareTree right.root then false
  else
    let common := min left.path.size right.path.size
    (List.range common).all fun index =>
      Step.compatible left.path[index]! right.path[index]!

def Place.sameTree (left right : Place) : Bool :=
  left.root.mayShareTree right.root

inductive RefKind where
  | immutable
  | mutable
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

inductive Phase where
  | unactivated
  | activated
  | suspended
  | poisoned
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure RefInfo where
  id : String
  kind : RefKind
  place : Place
  phase : Phase
  parent? : Option String := none
  poisoners : Array String := #[]
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure Separation where
  left : Nat
  right : Nat
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

def Separation.normalized (left right : Nat) : Separation :=
  if left ≤ right then { left, right } else { left := right, right := left }

inductive CallEffect where
  | ignore
  | read
  | write
  | consume
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

/-- A returned reference expressed relative to a reference parameter. -/
structure ReturnDerivation where
  parameter : Nat
  path : Array Step := #[]
  kind : RefKind
  phase : Phase := .unactivated
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure State where
  refs : Array RefInfo := #[]
  requiredSeparations : Array Separation := #[]
  parameterEffects : Array CallEffect := #[]
  returns : Array ReturnDerivation := #[]
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure CallArgument where
  reference : String
  parameter : Nat := 0
  effect : CallEffect
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure CallResult where
  destination : String
  derivation : ReturnDerivation
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

/-- Borrow-relevant retained-source events.  Control flow is represented by
`Block`, below, rather than flattened compiler blocks. -/
inductive Event where
  | borrowMut (destination : String) (place : Place)
      (parent? : Option String := none)
  | borrowImm (destination : String) (place : Place)
      (parent? : Option String := none)
  | read (reference : String)
  | write (reference : String)
  | freeze (source destination : String)
  | call (callee : String) (arguments : Array CallArgument)
      (requiredSeparations : Array Separation := #[])
      (results : Array CallResult := #[])
  | drop (reference : String)
  | ownerWrite (place : Place)
  | returnRef (reference : String)
  | nop
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

/-- The borrow-effect projection uses the shared retained-source control
topology. -/
abbrev Block := Move.Verify.Source.Control Event

structure Parameter where
  name : String
  kind : RefKind
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure Summary where
  parameterEffects : Array CallEffect := #[]
  requiredSeparations : Array Separation := #[]
  returns : Array ReturnDerivation := #[]
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure Program where
  declaration : String
  parameters : Array Parameter := #[]
  body : Block
  summary : Summary := {}
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

inductive ErrorKind where
  | duplicateReference
  | unknownReference
  | immutableWrite
  | poisonedUse
  | suspendedUse
  | immutableDuringMutation
  | mutationDuringImmutable
  | ownerInvalidation
  | localReferenceEscape
  | invalidCallEffect
  | invalidSummary
  | invalidLoopCertificate
  | certificateMismatch
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure BorrowError where
  point : Nat
  kind : ErrorKind
  reference? : Option String := none
  conflicting? : Option String := none
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

private def State.findRef? (state : State) (id : String) : Option RefInfo :=
  state.refs.find? (·.id == id)

private def State.replaceRef (state : State) (updated : RefInfo) : State :=
  { state with refs := state.refs.map fun info => if info.id == updated.id then updated else info }

private def State.eraseRef (state : State) (id : String) : State :=
  { state with refs := state.refs.filter (·.id != id) }

private def CallEffect.join : CallEffect → CallEffect → CallEffect
  | .write, _ | _, .write => .write
  | .consume, _ | _, .consume => .consume
  | .read, _ | _, .read => .read
  | _, _ => .ignore

private def State.recordEffect (state : State) (info : RefInfo)
    (effect : CallEffect) : State :=
  match info.place.root with
  | .parameter index =>
      match state.parameterEffects[index]? with
      | none => state
      | some old => { state with parameterEffects :=
          state.parameterEffects.set! index (old.join effect) }
  | _ => state

private def fail (point : Nat) (kind : ErrorKind) (reference? : Option String := none)
    (conflicting? : Option String := none) : Except BorrowError α :=
  .error { point, kind, reference?, conflicting? }

private def usable (point : Nat) (info : RefInfo) : Except BorrowError Unit :=
  match info.phase with
  | .poisoned => fail point .poisonedUse (some info.id) info.poisoners[0]?
  | .suspended => fail point .suspendedUse (some info.id)
  | _ => pure ()

private def lookupUsable (point : Nat) (state : State) (id : String) :
    Except BorrowError RefInfo := do
  let some info := state.findRef? id
    | fail point .unknownReference (some id)
  usable point info
  pure info

private partial def isAncestor (state : State) (ancestor child : String) : Bool :=
  if ancestor == child then true
  else
    match (state.findRef? child).bind (·.parent?) with
    | none => false
    | some parent => isAncestor state ancestor parent

private def conditionalSeparation? (left right : Place) : Option Separation :=
  match left.root, right.root with
  | .parameter left, .parameter right =>
      if left != right then some (Separation.normalized left right) else none
  | _, _ => none

private def State.addSeparation (state : State) (requirement : Separation) : State :=
  if state.requiredSeparations.contains requirement then state
  else { state with requiredSeparations := state.requiredSeparations.push requirement }

private def State.addReturn (state : State) (derivation : ReturnDerivation) : State :=
  if state.returns.contains derivation then state
  else { state with returns := state.returns.push derivation }

private def activate (point : Nat) (state : State) (writer : RefInfo) :
    Except BorrowError State := do
  unless writer.kind == .mutable do
    fail point .immutableWrite (some writer.id)
  let mut requirements := state.requiredSeparations
  for other in state.refs do
    if other.id != writer.id && other.kind == .immutable &&
        other.place.sameTree writer.place then
      match conditionalSeparation? writer.place other.place with
      | some requirement =>
          if !requirements.contains requirement then
            requirements := requirements.push requirement
      | none => fail point .mutationDuringImmutable (some writer.id) (some other.id)
  let mut refs := #[]
  for other in state.refs do
    if other.id == writer.id then
      refs := refs.push { writer with phase := .activated }
    else if isAncestor state other.id writer.id && other.place.overlaps writer.place then
      refs := refs.push { other with phase := .suspended }
    else if other.kind == .mutable && other.place.overlaps writer.place &&
        !isAncestor state writer.id other.id then
      match writer.place.root, other.place.root with
      | .parameter left, .parameter right =>
          if left != right then
            let requirement := Separation.normalized left right
            if !requirements.contains requirement then
              requirements := requirements.push requirement
            refs := refs.push other
          else
            let poisoned := { other with phase := .poisoned }
            refs := refs.push { poisoned with poisoners :=
              (if other.poisoners.contains writer.id then other.poisoners
              else other.poisoners.push writer.id) }
      | _, _ =>
          let poisoned := { other with phase := .poisoned }
          refs := refs.push { poisoned with poisoners :=
            (if other.poisoners.contains writer.id then other.poisoners
            else other.poisoners.push writer.id) }
    else
      refs := refs.push other
  pure <| ({ state with refs, requiredSeparations := requirements }).recordEffect
    writer .write

private def addReference (point : Nat) (state : State) (info : RefInfo) :
    Except BorrowError State := do
  if (state.findRef? info.id).isSome then
    fail point .duplicateReference (some info.id)
  pure { state with refs := state.refs.push info }

private def borrowMutable (point : Nat) (state : State) (destination : String)
    (place : Place) (parent? : Option String) : Except BorrowError State := do
  if let some parent := parent? then
    let parentInfo ← lookupUsable point state parent
    unless parentInfo.kind == .mutable do
      fail point .immutableWrite (some parent)
  addReference point state {
    id := destination, kind := .mutable, place, phase := .unactivated, parent? }

private def borrowImmutable (point : Nat) (state : State) (destination : String)
    (place : Place) (parent? : Option String) : Except BorrowError State := do
  if let some parent := parent? then
    discard <| lookupUsable point state parent
  let mut state := state
  for other in state.refs do
    if other.phase == .activated && other.place.sameTree place then
      match conditionalSeparation? place other.place with
      | some requirement => state := state.addSeparation requirement
      | none => fail point .immutableDuringMutation (some destination) (some other.id)
  addReference point state {
    id := destination, kind := .immutable, place, phase := .unactivated, parent? }

private def dropReference (_point : Nat) (state : State) (id : String) :
    Except BorrowError State := do
  let some info := state.findRef? id
    | pure state
  let state := state.eraseRef id
  match info.parent? with
  | none => pure state
  | some parent =>
      match state.findRef? parent with
      | some parentInfo =>
          let stillActiveChild := state.refs.any fun child =>
            child.parent? == some parent && child.phase == .activated
          if parentInfo.phase == .suspended && !stillActiveChild then
            pure <| state.replaceRef { parentInfo with phase := .activated }
          else pure state
      | none => pure state

private def transfer (point : Nat) (event : Event) (state : State) :
    Except BorrowError State := do
  match event with
  | .borrowMut destination place parent? =>
      borrowMutable point state destination place parent?
  | .borrowImm destination place parent? =>
      borrowImmutable point state destination place parent?
  | .read reference =>
      let info ← lookupUsable point state reference
      pure <| state.recordEffect info .read
  | .write reference =>
      let writer ← lookupUsable point state reference
      activate point state writer
  | .freeze source destination =>
      let info ← lookupUsable point state source
      unless info.kind == .mutable do
        fail point .invalidCallEffect (some source)
      let state ← dropReference point state source
      borrowImmutable point state destination info.place info.parent?
  | .call _ arguments requiredSeparations results =>
      let mut state := state
      for requirement in requiredSeparations do
        let some leftArgument := arguments.find? (·.parameter == requirement.left)
          | fail point .invalidCallEffect
        let some rightArgument := arguments.find? (·.parameter == requirement.right)
          | fail point .invalidCallEffect
        let left ← lookupUsable point state leftArgument.reference
        let right ← lookupUsable point state rightArgument.reference
        if left.place.overlaps right.place then
          match conditionalSeparation? left.place right.place with
          | some requirement => state := state.addSeparation requirement
          | none => fail point .invalidCallEffect (some left.id) (some right.id)
      for argument in arguments do
        match argument.effect with
        | .ignore =>
            discard <| lookupUsable point state argument.reference
        | .read =>
            let info ← lookupUsable point state argument.reference
            state := state.recordEffect info .read
        | .write =>
            let writer ← lookupUsable point state argument.reference
            state ← activate point state writer
        | .consume =>
            discard <| lookupUsable point state argument.reference
            state ← dropReference point state argument.reference
      for result in results do
        let some argument := arguments.find? (·.parameter == result.derivation.parameter)
          | fail point .invalidCallEffect (some result.destination)
        let parent ← lookupUsable point state argument.reference
        unless parent.kind == result.derivation.kind ||
            (parent.kind == .mutable && result.derivation.kind == .immutable) do
          fail point .invalidCallEffect (some argument.reference)
        let place := { parent.place with
          path := parent.place.path ++ result.derivation.path }
        if result.derivation.phase == .activated then
          state := state.replaceRef { parent with phase := .suspended }
        state ← addReference point state {
          id := result.destination
          kind := result.derivation.kind
          place
          phase := result.derivation.phase
          parent? := some parent.id }
      pure state
  | .drop reference => dropReference point state reference
  | .ownerWrite place =>
      if let some conflict := state.refs.find? (·.place.sameTree place) then
        fail point .ownerInvalidation none (some conflict.id)
      pure state
  | .returnRef reference =>
      let info ← lookupUsable point state reference
      match info.place.root with
      | .parameter parameter =>
          pure <| state.addReturn {
            parameter, path := info.place.path, kind := info.kind, phase := info.phase }
      | _ => fail point .localReferenceEscape (some reference)
  | .nop => pure state

private def Phase.join : Phase → Phase → Phase
  | .poisoned, _ | _, .poisoned => .poisoned
  | .suspended, _ | _, .suspended => .suspended
  | .activated, _ | _, .activated => .activated
  | _, _ => .unactivated

private def RefInfo.join (left right : RefInfo) : RefInfo :=
  { left with
    phase := left.phase.join right.phase
    poisoners := right.poisoners.foldl (fun accumulated poisoner =>
      if accumulated.contains poisoner then accumulated else accumulated.push poisoner)
      left.poisoners }

/-- May-state union at source joins.  A reference present on only one edge is
kept: using it after the join will conservatively retain its conflicts. -/
def State.join (left right : State) : State :=
  let withRight := right.refs.foldl (fun refs info =>
    match refs.findIdx? (·.id == info.id) with
    | none => refs.push info
    | some index => refs.set! index (RefInfo.join refs[index]! info)) left.refs
  let requirements := right.requiredSeparations.foldl (fun accumulated requirement =>
    if accumulated.contains requirement then accumulated else accumulated.push requirement)
    left.requiredSeparations
  let effectCount := max left.parameterEffects.size right.parameterEffects.size
  let effects := (List.range effectCount).toArray.map fun index =>
    (left.parameterEffects[index]?.getD .ignore).join
      (right.parameterEffects[index]?.getD .ignore)
  let returns := right.returns.foldl (fun accumulated returned =>
    if accumulated.contains returned then accumulated else accumulated.push returned)
    left.returns
  { refs := withRight, requiredSeparations := requirements,
    parameterEffects := effects, returns }

def State.le (left right : State) : Bool := left.join right == right

structure LoopFact where
  point : Nat
  invariant : State
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

structure Analysis where
  finalState : State
  loops : Array LoopFact := #[]
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

private partial def runAnalyze (block : Block) (state : State)
    (loops : Array LoopFact) (fuel : Nat) : Except BorrowError Analysis := do
  if fuel == 0 then fail 0 .invalidLoopCertificate
  match block with
  | .done | .abort => pure { finalState := state, loops }
  | .event point event next =>
      runAnalyze next (← transfer point event state) loops (fuel - 1)
  | .branch _ thenBranch elseBranch next =>
      let thenResult ← runAnalyze thenBranch state loops (fuel - 1)
      let elseResult ← runAnalyze elseBranch state thenResult.loops (fuel - 1)
      runAnalyze next (thenResult.finalState.join elseResult.finalState)
        elseResult.loops (fuel - 1)
  | .loop point body next =>
      let rec iterate (candidate : State) (remaining : Nat) : Except BorrowError Analysis := do
        if remaining == 0 then fail point .invalidLoopCertificate
        let bodyResult ← runAnalyze body candidate loops (fuel - 1)
        let enlarged := state.join bodyResult.finalState
        if enlarged == candidate then
          let loops := bodyResult.loops.push { point, invariant := candidate }
          runAnalyze next candidate loops (fuel - 1)
        else iterate enlarged (remaining - 1)
      iterate state (state.refs.size * 4 + 32)

/-- Generate the least structured post-fixpoint used in a certificate. -/
def Program.initialState (program : Program) : State :=
  { refs := program.parameters.mapIdx fun index parameter => {
      id := parameter.name
      kind := parameter.kind
      place := { root := .parameter index }
      phase := .unactivated }
    parameterEffects := program.parameters.map fun _ => .ignore }

def analyze (program : Program) : Except BorrowError Analysis :=
  runAnalyze program.body program.initialState #[] 100000

structure Certificate where
  version : Nat := 1
  program : Program
  analysis : Analysis
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

def makeCertificate (program : Program) : Except BorrowError Certificate := do
  pure { program, analysis := ← analyze program }

/-- The independent checker replays the analysis; neither loop invariants nor
the final state are accepted on trust. -/
def Certificate.check (certificate : Certificate) (program : Program) :
    Except BorrowError Unit :=
  if certificate.version != 1 || certificate.program != program then
    fail 0 .certificateMismatch
  else
    match analyze program with
    | .error error => .error error
    | .ok replay =>
        if replay != certificate.analysis then fail 0 .certificateMismatch
        else if program.summary.parameterEffects != replay.finalState.parameterEffects ||
            program.summary.requiredSeparations != replay.finalState.requiredSeparations ||
            program.summary.returns != replay.finalState.returns then
          fail 0 .invalidSummary
        else .ok ()

def Certificate.Checks (certificate : Certificate) (program : Program) : Prop :=
  certificate.check program = .ok ()

def PoisonSafe (program : Program) : Prop := (analyze program).isOk

def WellBorrowed (program : Program) : Prop :=
  ∃ certificate, Certificate.Checks certificate program

theorem Certificate.sound {certificate : Certificate} {program : Program}
    (checked : certificate.Checks program) : PoisonSafe program := by
  unfold Certificate.Checks Certificate.check at checked
  cases mismatch : (certificate.version != 1 || certificate.program != program) with
  | true => simp [mismatch, fail] at checked
  | false =>
      simp only [mismatch, Bool.false_eq] at checked
      cases replay : analyze program with
      | error error => simp [replay] at checked
      | ok analysis =>
          unfold PoisonSafe
          rw [replay]
          rfl

theorem soundChecked {program : Program} {certificate : Certificate}
    (checked : certificate.check program = .ok ()) : WellBorrowed program :=
  ⟨certificate, checked⟩

def ErrorKind.message : ErrorKind → String
  | .duplicateReference => "reference name is already live"
  | .unknownReference => "reference is not live"
  | .immutableWrite => "an immutable reference cannot be written"
  | .poisonedUse => "reference was poisoned by an overlapping write"
  | .suspendedUse => "reference is suspended by an active child borrow"
  | .immutableDuringMutation => "cannot create an immutable reference while a prophecy is active in the same tree"
  | .mutationDuringImmutable => "cannot activate a mutable reference while an immutable reference is live in the same tree"
  | .ownerInvalidation => "cannot move or overwrite an owner while a reference into it is live"
  | .localReferenceEscape => "a returned reference must derive from a reference parameter"
  | .invalidCallEffect => "call summary is incompatible with its reference argument"
  | .invalidSummary => "borrow summary is not a post-fixpoint of the retained source body"
  | .invalidLoopCertificate => "borrow analysis did not produce a valid loop post-fixpoint"
  | .certificateMismatch => "borrow certificate does not match the retained source program"

end Move.Verify.Borrow
