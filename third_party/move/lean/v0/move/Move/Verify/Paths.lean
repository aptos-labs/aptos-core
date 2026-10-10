-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.EnumAccess
import Move.Semantics.Enum

/-!
# Field paths in generated source specifications

The source translator (`Move.Verify.Syntax`) works on syntax and does not
know the types along a field path `owner.f.g`; the elaborators here decide
each step from the owner's type when the generated specification is
elaborated.  A step through a structure field is the projection (and a
structure update on write-back); a step through the payload field of an
enum is the field's partial selection (`Move.Spec.arbitrary` for the other
variants), guarded by the variant test that the VM performs — the borrow
aborts with `Move.Semantics.variantMismatch` on any other variant.

* `selectPath% owner [f, g]` — the value at the path (for a focus or an
  immutable read; the guards are the caller's).
* `focusPath% owner [f, g]` — the value at a path with no enum step
  (a global resource's mutable focus, which has no room for a guard).
* `updatePath% owner [f, g] value` — `owner` with `value` written at the
  path.
* `bindSelectPath% owner [f, g] x => body` — `let x := owner.f.g; body`, or
  the guarded selection bound by `Spec.bind` when the path crosses an enum.
* `guardPath% owner [f, g] spec` — `spec` after the path's guards.
-/

open Lean Elab Term Meta

namespace Move.Verify

scoped syntax (name := selectPathTerm) "selectPath% " term:max " [" ident,* "]" : term
scoped syntax (name := focusPathTerm) "focusPath% " term:max " [" ident,* "]" : term
scoped syntax (name := updatePathTerm) "updatePath% " term:max " [" ident,* "] " term:max : term
scoped syntax (name := bindSelectPathTerm)
  "bindSelectPath% " term:max " [" ident,* "] " ident " => " term : term
scoped syntax (name := guardPathTerm) "guardPath% " term:max " [" ident,* "] " term:max : term

/-- The field idents of a path node. -/
private def pathFields (node : Syntax) : Array Ident :=
  node.getSepArgs.map (⟨·⟩)

/-- The value of `owner` (an elaborated term) at field `field`, and the
variant test guarding it when `field` is an enum payload field. -/
private def pathStep (owner : Expr) (field : Ident) (site : Nat) (strict : Bool) :
    TermElabM (Expr × Option Term) := do
  let ownerType ← whnf (← instantiateMVars (← inferType owner))
  if ownerType.getAppFn.isMVar then tryPostpone
  let ownerStx ← exprToSyntax owner
  match ← enumFieldSelectTerm ownerStx ownerType field.getId site with
  | some selection =>
      if strict then
        throwErrorAt field "a mutable borrow focused through the enum payload field `{field.getId}` of a global resource is not yet supported by source specification generation"
      let some holds ← enumFieldHoldsTerm ownerStx ownerType field.getId
        | throwErrorAt field "no variant test for `{field.getId}`"
      return (← elabTerm selection none, some holds)
  | none =>
      return (← elabTerm (← `($ownerStx.$field)) none, none)

/-- Walk a path: the value at its end and the guards of its enum steps. -/
private def walkPath (owner : Syntax) (fields : Array Ident) (site : Nat)
    (strict : Bool := false) : TermElabM (Expr × Array Term) := do
  let mut current ← elabTerm owner none
  let mut guards : Array Term := #[]
  for field in fields do
    let (next, guard?) ← pathStep current field site strict
    current := next
    if let some guard := guard? then guards := guards.push guard
  return (current, guards)

/-- The conjunction of the guards. -/
private def conjunction (guards : Array Term) : TermElabM Term := do
  let mut result := guards[0]!
  for guard in guards.extract 1 guards.size do
    result ← `($result && $guard)
  pure result

@[term_elab selectPathTerm]
def elabSelectPath : TermElab := fun stx expectedType? => do
  let (value, _) ← walkPath stx[1] (pathFields stx[3]) (siteOf stx)
  ensureHasType expectedType? value

@[term_elab focusPathTerm]
def elabFocusPath : TermElab := fun stx expectedType? => do
  let (value, _) ← walkPath stx[1] (pathFields stx[3]) (siteOf stx) (strict := true)
  ensureHasType expectedType? value

@[term_elab bindSelectPathTerm]
def elabBindSelectPath : TermElab := fun stx expectedType? => do
  let name : Ident := ⟨stx[5]⟩
  let body : Term := ⟨stx[7]⟩
  let (value, guards) ← walkPath stx[1] (pathFields stx[3]) (siteOf stx)
  let valueStx ← exprToSyntax value
  if guards.isEmpty then
    elabTerm (← `(let $name := $valueStx; $body)) expectedType?
  else
    let holds ← conjunction guards
    elabTerm (← `(Move.Semantics.Spec.bind
      (Move.Semantics.variantFieldSpec $holds $valueStx) (fun $name => $body))) expectedType?

@[term_elab guardPathTerm]
def elabGuardPath : TermElab := fun stx expectedType? => do
  let body : Term := ⟨stx[5]⟩
  let (_, guards) ← walkPath stx[1] (pathFields stx[3]) (siteOf stx)
  if guards.isEmpty then
    elabTerm body expectedType?
  else
    let holds ← conjunction guards
    elabTerm (← `(Move.Semantics.Spec.bind
      (Move.Semantics.variantFieldSpec $holds ()) (fun _ => $body))) expectedType?

/-- Replace the identifier `name` by `replacement` throughout `stx`. -/
private partial def replacePlaceholder (name : Name) (replacement stx : Syntax) : Syntax :=
  if stx.isIdent && stx.getId == name then replacement
  else stx.setArgs (stx.getArgs.map (replacePlaceholder name replacement))

/-- `{ owner with field := value }` for a structure owner. -/
private def structureUpdate (owner : Syntax) (field : Ident) (value : Term) :
    TermElabM Term := do
  let source := "{ _moveSpecUpdateOwner with " ++ field.getId.toString ++
    " := _moveSpecUpdateValue }"
  let parsed ← match Lean.Parser.runParserCategory (← getEnv) `term source with
    | .ok parsed => pure parsed
    | .error message => throwErrorAt field
        "failed to generate update for source field `{field.getId}`: {message}"
  let parsed := replacePlaceholder `_moveSpecUpdateOwner owner parsed
  let parsed := replacePlaceholder `_moveSpecUpdateValue value.raw parsed
  pure ⟨parsed⟩

/-- `owner` (an elaborated term) with `newValue` written at `fields`. -/
private partial def updateAlong (owner : Expr) (fields : List Ident) (newValue : Term)
    (site : Nat) : TermElabM Term := do
  match fields with
  | [] => pure newValue
  | field :: rest =>
      let ownerType ← whnf (← instantiateMVars (← inferType owner))
      if ownerType.getAppFn.isMVar then tryPostpone
      let ownerStx ← exprToSyntax owner
      match ← enumFieldSelectTerm ownerStx ownerType field.getId site with
      | some selection =>
          let inner ← elabTerm selection none
          let updatedInner ← updateAlong inner rest newValue site
          let some updated ← enumFieldUpdateTerm ownerStx ownerType field.getId updatedInner
            | throwErrorAt field "no variant update for `{field.getId}`"
          pure updated
      | none =>
          let inner ← elabTerm (← `($ownerStx.$field)) none
          let updatedInner ← updateAlong inner rest newValue site
          structureUpdate ownerStx field updatedInner

@[term_elab updatePathTerm]
def elabUpdatePath : TermElab := fun stx expectedType? => do
  let newValue : Term := ⟨stx[5]⟩
  let owner ← elabTerm stx[1] none
  let updated ← updateAlong owner (pathFields stx[3]).toList newValue (siteOf stx)
  elabTerm updated expectedType?

end Move.Verify
