-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Syntax

/-!
# Enum payload access

Move reaches the payload of an enum value through its variants:

* `x.f` on an enum *value* selects the field `f` of the variants that have
  it — the specification language's partial reading: the field when the
  value is one of those variants, an unspecified value (`Move.Spec.arbitrary`)
  otherwise.  Elaborated here for a local (or a parenthesized term) of an
  enum type; `x.f` on anything else is Lean's.
* `&r.f` / `&mut r.f` through a reference to an enum borrows that field
  (`Move.borrowVariantField`, in `Move.Syntax`): the VM fails on any other
  variant.
* `match r with …` through a reference to an enum binds the selected
  variant's payload by reference.  Leaner writes it as a plain `match` whose
  scrutinee is a local of type `Ref E` / `MutRef E`: the elaborators here
  turn it into a variant dispatch on the referent (`testVariantRef`) whose
  alternatives borrow their payload fields through the reference
  (`borrowVariantField`).  The alternatives are the plain forms
  `| .Variant x y => rhs`, `| Enum.Variant x _ => rhs` and `| _ => rhs`, one
  pattern each, covering every variant or ending in `_`.  Inside `do` the
  right-hand sides are `do` sequences (so `break`/`continue`/`return` in
  them are the enclosing block's); as a term the right-hand sides are
  `Action` terms.

The helpers building the variant dispatches are shared with the verifier's
source translation (`Move.Verify.Paths`).

In its own module so that its `do` blocks elaborate with `Move.Syntax`'s
`do`-element hooks imported (compiled) rather than interpreted in the module
that defines them.
-/

open Lean Elab Term Meta

namespace Move

/-- The variants of the Move enum `enumName`: constructor, index, payload
arity. -/
def enumVariants (enumName : Name) : MetaM (Array (Name × Nat × Nat)) := do
  let env ← getEnv
  let some (.inductInfo info) := env.find? enumName | return #[]
  info.ctors.toArray.mapIdxM fun index ctor => do
    let some (.ctorInfo ctorInfo) := env.find? ctor | throwError "`{ctor}` is not a constructor"
    return (ctor, index, ctorInfo.numFields)

/-- `Ctor a₀ … aₙ₋₁` with `argument i` at each position — a pattern or a
value of the variant. -/
def variantApplication (ctor : Name) (arity : Nat) (argument : Nat → TermElabM Term) :
    TermElabM Term := do
  let head : Term := mkIdent ctor
  if arity == 0 then return head
  let mut arguments : Array Term := #[]
  for position in [:arity] do
    arguments := arguments.push (← argument position)
  `($head $arguments*)

/-- The Move enum `ownerType` is an application of, with the variants that
have payload field `field` (as a bit set), the field's offset, and its type;
`none` when `ownerType` is not a Move enum with that field. -/
def enumField? (ownerType : Expr) (field : Name) :
    MetaM (Option (Name × Nat × Nat × Expr)) := do
  let ownerType ← whnf ownerType
  let .const enumName _ := ownerType.getAppFn | return none
  let some (variants, offset, fieldType) ← variantField? ownerType field | return none
  return some (enumName, variants, offset, fieldType)

/-- `match owner with | V₁ … x … => x | … | _ => Move.Spec.arbitrary _ site`:
the payload field `field` of the enum value `owner : ownerType` — Move's
`SelectVariants`, read partially.  `none` when `ownerType` is not a Move
enum with that field. -/
def enumFieldSelectTerm (owner : Term) (ownerType : Expr) (field : Name) (site : Nat) :
    TermElabM (Option Term) := do
  let some (enumName, variants, offset, _) ← enumField? ownerType field | return none
  let selected : Term := mkIdent `_moveSelectedField
  let mut alternatives : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  let all ← enumVariants enumName
  for (ctor, index, arity) in all do
    if (variants >>> index) % 2 == 1 then
      let pattern ← variantApplication ctor arity fun position =>
        if position == offset then pure selected else `(_)
      alternatives := alternatives.push (← `(Lean.Parser.Term.matchAltExpr| | $pattern:term => $selected))
  unless variants == (1 <<< all.size) - 1 do
    let siteLiteral := Syntax.mkNumLit (toString site)
    alternatives := alternatives.push
      (← `(Lean.Parser.Term.matchAltExpr| | _ => Move.Spec.arbitrary _ $siteLiteral))
  return some (← `(match $owner:term with $alternatives:matchAlt*))

/-- Whether the enum value `owner : ownerType` is one of the variants that
have payload field `field`, as a `Bool` term (`true` when all do). -/
def enumFieldHoldsTerm (owner : Term) (ownerType : Expr) (field : Name) :
    TermElabM (Option Term) := do
  let some (enumName, variants, _, _) ← enumField? ownerType field | return none
  let all ← enumVariants enumName
  if variants == (1 <<< all.size) - 1 then return some (← `(true))
  let mut alternatives : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  for (ctor, index, arity) in all do
    if (variants >>> index) % 2 == 1 then
      let pattern ← variantApplication ctor arity fun _ => `(_)
      alternatives := alternatives.push (← `(Lean.Parser.Term.matchAltExpr| | $pattern:term => true))
  alternatives := alternatives.push (← `(Lean.Parser.Term.matchAltExpr| | _ => false))
  return some (← `(match $owner:term with $alternatives:matchAlt*))

/-- The enum value `owner : ownerType` with payload field `field` replaced by
`newValue` in the variants that have it, unchanged otherwise. -/
def enumFieldUpdateTerm (owner : Term) (ownerType : Expr) (field : Name) (newValue : Term) :
    TermElabM (Option Term) := do
  let some (enumName, variants, offset, _) ← enumField? ownerType field | return none
  let all ← enumVariants enumName
  let mut alternatives : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  for (ctor, index, arity) in all do
    if (variants >>> index) % 2 == 1 then
      let binder (position : Nat) : Term := mkIdent (Name.mkSimple s!"_moveField{position}")
      let pattern ← variantApplication ctor arity fun position => pure (binder position)
      let rebuilt ← variantApplication ctor arity fun position =>
        if position == offset then pure newValue else pure (binder position)
      alternatives := alternatives.push (← `(Lean.Parser.Term.matchAltExpr| | $pattern:term => $rebuilt))
  unless variants == (1 <<< all.size) - 1 do
    alternatives := alternatives.push (← `(Lean.Parser.Term.matchAltExpr| | _ => $owner))
  return some (← `(match $owner:term with $alternatives:matchAlt*))

/-- The byte position of `stx`, distinguishing the sites of unspecified
values. -/
def siteOf (stx : Syntax) : Nat := (stx.getPos?).map (·.byteIdx) |>.getD 0

/-- `x.f` as a dotted identifier, `x` a local of a Move enum type with
payload field `f`: the field's partial selection. -/
@[term_elab ident]
def elabMoveEnumFieldOfIdent : TermElab := fun stx expectedType? => do
  -- The owner keeps the identifier's macro scopes; the field is its last
  -- component.
  let view := extractMacroScopes stx.getId
  let .str ownerBase field := view.name | throwUnsupportedSyntax
  if ownerBase.isAnonymous then throwUnsupportedSyntax
  let owner := { view with name := ownerBase }.review
  let context ← getLCtx
  unless (context.findFromUserName? owner.getRoot).isSome do throwUnsupportedSyntax
  -- A local named `x.f` itself is that local.
  if (context.findFromUserName? stx.getId).isSome then throwUnsupportedSyntax
  let ownerExpr ← elabTerm (mkIdentFrom stx owner) none
  let ownerType ← whnf (← instantiateMVars (← inferType ownerExpr))
  if ownerType.getAppFn.isMVar then tryPostpone
  let some selection ← enumFieldSelectTerm (← exprToSyntax ownerExpr) ownerType
      (Name.mkSimple field) (siteOf stx) | throwUnsupportedSyntax
  elabTerm selection expectedType?

/-- `(e).f`, `e` of a Move enum type with payload field `f`: the field's
partial selection. -/
@[term_elab Lean.Parser.Term.proj]
def elabMoveEnumFieldOfProj : TermElab := fun stx expectedType? => do
  let field := stx[2]
  unless field.isIdent do throwUnsupportedSyntax
  let ownerExpr ← elabTerm stx[0] none
  let ownerType ← whnf (← instantiateMVars (← inferType ownerExpr))
  if ownerType.getAppFn.isMVar then tryPostpone
  let some selection ← enumFieldSelectTerm (← exprToSyntax ownerExpr) ownerType
      field.getId (siteOf stx) | throwUnsupportedSyntax
  elabTerm selection expectedType?

/-- The scrutinee of a `match` through a Move reference. -/
private structure RefMatchScrutinee where
  discriminant : Ident
  mutable : Bool
  enumName : Name
  levels : List Level
  args : Array Expr
  info : InductiveVal

/-- One alternative: its variant (`none` for `_`), its payload binders
(`none` for `_`), and its right-hand side (a term or a `do` sequence). -/
private structure RefMatchAlternative where
  variant? : Option (Nat × Name)
  payload : Array (Option Ident)
  rhs : Syntax

/-- `discriminant` as the scrutinee of a `match` through a Move reference: a
local of type `Ref E` / `MutRef E` with `E` a Move enum. -/
private def refMatchScrutinee? (discriminant : Syntax) :
    TermElabM (Option RefMatchScrutinee) := do
  unless discriminant.isIdent do return none
  let some declaration := (← getLCtx).findFromUserName? discriminant.getId | return none
  let type ← whnf declaration.type
  let (mutable, owner) ←
    if type.isAppOfArity ``MutRef 1 then pure (true, type.appArg!)
    else if type.isAppOfArity ``Ref 1 then pure (false, type.appArg!)
    else return none
  let owner ← whnf owner
  let .const enumName levels := owner.getAppFn | return none
  let env ← getEnv
  unless moveEnumAttr.hasTag env enumName do return none
  let some (.inductInfo info) := env.find? enumName | return none
  return some { discriminant := ⟨discriminant⟩, mutable, enumName, levels, args := owner.getAppArgs, info }

/-- The alternative `alternative` (a `matchAlt` node, whose right-hand side
is a term or a `do` sequence) of a `match` through `scrutinee`. -/
private def parseRefMatchAlternative (scrutinee : RefMatchScrutinee) (alternative : Syntax) :
    TermElabM RefMatchAlternative := do
  let patterns := alternative[1]
  unless patterns.getNumArgs == 1 && patterns[0].getNumArgs == 1 do
    throwErrorAt alternative "a `match` through a Move reference takes one pattern per alternative"
  let pattern := patterns[0][0]
  let rhs := alternative[3]
  let (head, binders) : Syntax × Array Syntax :=
    if pattern.isOfKind ``Lean.Parser.Term.app && pattern.getNumArgs == 2 then
      (pattern[0], pattern[1].getArgs)
    else (pattern, #[])
  if head.isOfKind ``Lean.Parser.Term.hole then
    unless binders.isEmpty do throwErrorAt pattern "a wildcard pattern takes no binders"
    return { variant? := none, payload := #[], rhs }
  let ctor ←
    if head.isOfKind ``Lean.Parser.Term.dotIdent then
      pure (scrutinee.enumName ++ head[1].getId)
    else if head.isIdent then
      resolveGlobalConstNoOverload head
    else throwErrorAt pattern "unsupported pattern in a `match` through a Move reference"
  let some (.ctorInfo ctorInfo) := (← getEnv).find? ctor
    | throwErrorAt head "`{ctor}` is not a variant of `{scrutinee.enumName}`"
  unless ctorInfo.induct == scrutinee.enumName do
    throwErrorAt head "`{ctor}` is not a variant of `{scrutinee.enumName}`"
  let payload ← binders.mapM fun binder => do
    if binder.isOfKind ``Lean.Parser.Term.hole then pure none
    else if binder.isIdent then pure (some (⟨binder⟩ : Ident))
    else throwErrorAt binder "a payload binder of a `match` through a Move reference is a name or `_`"
  return { variant? := some (ctorInfo.cidx, ctor), payload, rhs }

/-- The payload borrows of an alternative, as `do` elements: one
`let x ← borrowVariantField[Mut] (FieldTy := T) variants offset r` per named
binder. -/
private def payloadBorrows (scrutinee : RefMatchScrutinee) (index : Nat) (ctor : Name)
    (payload : Array (Option Ident)) (rhs : Syntax) : TermElabM (Array Syntax) := do
  let ctorType ← instantiateForall (← inferType (mkConst ctor scrutinee.levels)) scrutinee.args
  let fieldTypes ← forallTelescope ctorType fun fields _ =>
    fields.mapM fun fieldVar => do instantiateMVars (← fieldVar.fvarId!.getDecl).type
  unless payload.size == fieldTypes.size do
    throwErrorAt rhs "variant `{ctor}` has {fieldTypes.size} payload field(s), the pattern binds {payload.size}"
  let primitive := mkIdent (if scrutinee.mutable then ``borrowVariantFieldMut else ``borrowVariantField)
  let mask := Syntax.mkNumLit (toString (1 <<< index))
  let discriminant := scrutinee.discriminant
  let mut borrows : Array Syntax := #[]
  for position in [:payload.size] do
    let some binder := payload[position]! | pure ()
    let offset := Syntax.mkNumLit (toString position)
    let fieldType ← exprToSyntax fieldTypes[position]!
    let borrow ← `(doElem| let $binder:ident ← $primitive:ident (FieldTy := $fieldType) $mask $offset $discriminant)
    borrows := borrows.push borrow.raw
  return borrows

/-- Checks that `alternatives` cover every variant of the scrutinee, or end
in a wildcard. -/
private def checkRefMatchCoverage (ref : Syntax) (scrutinee : RefMatchScrutinee)
    (alternatives : List RefMatchAlternative) : TermElabM Unit := do
  let covered := alternatives.foldl (init := (0 : Nat)) fun acc alternative =>
    match alternative.variant? with
    | some (index, _) => acc ||| (1 <<< index)
    | none => acc
  let all := (1 <<< scrutinee.info.ctors.length) - 1
  let exhaustive := alternatives.any (·.variant?.isNone) || covered == all
  unless exhaustive do
    throwErrorAt ref "a `match` through a Move reference must cover every variant (or end in `_`)"
  for alternative in alternatives.dropLast do
    if alternative.variant?.isNone then
      throwErrorAt alternative.rhs "a wildcard alternative must be the last"

private def variantTestElem (scrutinee : RefMatchScrutinee) (test : Ident) (index : Nat) :
    TermElabM Syntax := do
  let primitive := mkIdent (if scrutinee.mutable then ``testVariantMutRef else ``testVariantRef)
  let variant := Syntax.mkNumLit (toString index)
  let discriminant := scrutinee.discriminant
  return (← `(doElem| let $test:ident ← $primitive:ident $variant $discriminant)).raw

/-- The dispatch of a term `match`: a test per alternative but the last, each
alternative's payload borrows before its right-hand side (a term). -/
private partial def refMatchTerm (ref : Syntax) (scrutinee : RefMatchScrutinee) :
    List RefMatchAlternative → TermElabM Term
  | [] => throwErrorAt ref "a `match` needs at least one alternative"
  | [alternative] => armTerm alternative
  | alternative :: rest => do
      let some (index, _) := alternative.variant?
        | throwErrorAt alternative.rhs "a wildcard alternative must be the last"
      let test := mkIdentFrom ref `_moveVariantTest
      let testElem ← variantTestElem scrutinee test index
      let body ← armTerm alternative
      let remaining ← refMatchTerm ref scrutinee rest
      let branch ← `(doElem| if $test:ident then $body:term else $remaining:term)
      `(do $(mkDoSeq #[testElem, branch.raw]):doSeq)
where
  armTerm (alternative : RefMatchAlternative) : TermElabM Term := do
    let rhs : Term := ⟨alternative.rhs⟩
    let some (index, ctor) := alternative.variant? | pure rhs
    let borrows ← payloadBorrows scrutinee index ctor alternative.payload alternative.rhs
    if borrows.isEmpty then pure rhs
    else
      let rhsElem ← `(doElem| $rhs:term)
      `(do $(mkDoSeq (borrows.push rhsElem.raw)):doSeq)

open Lean.Elab.Do Lean.Parser.Term in
/-- The dispatch of a `match` `do` element, as `do` elements: a test per
alternative but the last, each alternative's payload borrows before its
right-hand side (a `do` sequence). -/
private partial def refMatchDoElems (ref : Syntax) (scrutinee : RefMatchScrutinee) :
    List RefMatchAlternative → TermElabM (Array Syntax)
  | [] => throwErrorAt ref "a `match` needs at least one alternative"
  | [alternative] => armElems alternative
  | alternative :: rest => do
      let some (index, _) := alternative.variant?
        | throwErrorAt alternative.rhs "a wildcard alternative must be the last"
      let test := mkIdentFrom ref `_moveVariantTest
      let testElem ← variantTestElem scrutinee test index
      let body := mkDoSeq (← armElems alternative)
      let remaining := mkDoSeq (← refMatchDoElems ref scrutinee rest)
      let branch ← `(doElem| if $test:ident then $body:doSeq else $remaining:doSeq)
      return #[testElem, branch.raw]
where
  armElems (alternative : RefMatchAlternative) : TermElabM (Array Syntax) := do
    let rhs : TSyntax ``doSeq := ⟨alternative.rhs⟩
    let elements := getDoElems rhs |>.map (·.raw)
    let some (index, ctor) := alternative.variant? | pure elements
    let borrows ← payloadBorrows scrutinee index ctor alternative.payload alternative.rhs
    return borrows ++ elements

/-- A term `match r with …` through a Move reference `r`. -/
@[term_elab Lean.Parser.Term.match]
def elabMoveRefMatch : TermElab := fun stx expectedType? => do
  -- `match` generalizingParam? motive? discrs "with" matchAlts
  unless stx[1].isNone && stx[2].isNone && stx[3].getNumArgs == 1 && stx[3][0][0].isNone do
    throwUnsupportedSyntax
  let some scrutinee ← refMatchScrutinee? stx[3][0][1] | throwUnsupportedSyntax
  let alternatives ← stx[5][0].getArgs.toList.mapM (parseRefMatchAlternative scrutinee)
  checkRefMatchCoverage stx scrutinee alternatives
  let expanded ← refMatchTerm stx scrutinee alternatives
  elabTerm expanded expectedType?

open Lean.Elab.Do in
/-- A `match r with …` `do` element through a Move reference `r`. -/
@[doElem_elab Lean.Parser.Term.doMatch]
def elabMoveRefDoMatch : DoElab := fun element cont => do
  let stx := element.raw
  -- `match` dependentParam? generalizingParam? motive? discrs "with" doMatchAlts
  unless stx[1].isNone && stx[2].isNone && stx[3].isNone && stx[4].getNumArgs == 1 &&
      stx[4][0][0].isNone do
    throwUnsupportedSyntax
  let some scrutinee ← refMatchScrutinee? stx[4][0][1] | throwUnsupportedSyntax
  let alternatives ← (stx[6][0].getArgs.toList.mapM (parseRefMatchAlternative scrutinee) :
    TermElabM _)
  checkRefMatchCoverage stx scrutinee alternatives
  let elements ← refMatchDoElems stx scrutinee alternatives
  elabDoElems1 (elements.map fun e => (⟨e⟩ : DoElem)) cont

end Move
