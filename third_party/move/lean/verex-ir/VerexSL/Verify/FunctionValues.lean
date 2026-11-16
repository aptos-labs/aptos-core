-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.Contracts
import VerexIR.Proofs.Denote.FunctionValues

/-!
# Generated function values

A unit's function values (`doc/designs/higher-order-functions.md`, "Function
values as native data"), generated once per link set before the unit's first
theorem: one indexed family with a constructor per closure site at its closed
rows, holding the site's captures natively, and `foreign` at every function
type (`ForeignClosure`); its encoder, structural on the family; and the
`SiteFamily` the library derives the decoder and the laws of `FunctionCarriers`
from. A unit without closure sites keeps the library's
function values (`FunctionCarriers.runtime`).

Each constructor's captured carrier is spelled concretely, since the
family's own `NTy.carrier` is no strictly positive occurrence, and checked
once by `rfl` against the base frame's carrier. Construction of a site at
the base frame is its constructor (`construct…` equations, at any
instantiation the site's key names); of a site without type arguments the
constructor's closure, its invocation as its target's call on the woven
arguments, the unit's check, and the typing of the value naming its target
are generated and registered (`SiteEquations`).
-/

namespace VerexSL.Verify

open Lean Meta Elab Command
open VerexIR (FunctionHandle TypeId)
open VerexIR.Validation (ValidatedUnit)
open VerexIR.Proofs.Denote
open VerexSL.Contract

/-- Name of a unit's generated family of function values. -/
def functionValuesName (segments : Array String) : Name :=
  (unitName segments).getPrefix ++ `FunctionValue

/-- Name of a unit's requires meaning over its function values, given the
reading of a value no site encodes (`RequiresMeaning`): every unit's takes it,
so that a statement reading the meaning spells it the same way whether or not
the unit has closure sites. -/
def functionRequiresName (segments : Array String) : Name :=
  (unitName segments).getPrefix ++ `functionRequires

/-- The keyed site specifications of a unit's closure sites. -/
def siteSpecs (sites : Array ClosureSite) : Array SiteSpec :=
  sites.map fun site =>
    { key := { function := site.target, mask := site.mask
               typeInstantiation := site.typeInstantiation }
      parameters := site.type.parameters, shared := site.type.shared
      results := site.type.results, captured := site.captured }

/-- Why a captured type cannot be carried natively, if it cannot: a type
parameter left open, or a function value under a vector or a map, where the
kernel admits no nested occurrence of the family. -/
partial def capturedUnsupported? : NTy → Option String
  | τ@(.param _) => if τ.paramFree then none else some "a type parameter"
  | .function parameters _ results =>
      if parameters.paramFree && results.paramFree then none else some "a type parameter"
  | .nominal _ _ _ => some "a recursive type"
  | .tuple elements | .struct _ _ elements => rowUnsupported? elements
  | .enum _ _ _ rows _ => rowsUnsupported? rows
  | .ghost slot => capturedUnsupported? slot
  | τ@(.vector _) | τ@(.slice _) | τ@(.table _ _) =>
      if !τ.paramFree then some "a type parameter"
      else if τ.closureFree then none
      else some "a function value inside a vector or a map"
  | .ref _ | .sliceRef _ => some "a reference"
  | _ => none
where
  rowUnsupported? : NRow → Option String
    | .nil => none
    | .cons τ rest => (capturedUnsupported? τ).orElse fun _ => rowUnsupported? rest
  rowsUnsupported? : NRows → Option String
    | .nil => none
    | .cons fields rest => (rowUnsupported? fields).orElse fun _ => rowsUnsupported? rest

/-- The literals a generated block names: rows, sharing marks, and types,
each an abbreviation of its quoted value. -/
private structure Literals where
  rows : Array (NRow × Name) := #[]
  sharings : Array (Sharing × Name) := #[]
  types : Array (NTy × Name) := #[]

private abbrev GenM := StateRefT Literals CommandElabM

/-- Declare a reducible abbreviation of a quoted literal. -/
private def defineLiteral (name : Name) (type value : Lean.Expr) : CommandElabM Unit :=
  liftTermElabM do
    addDecl (.defnDecl {
      name, levelParams := [], type, value, hints := .abbrev, safety := .safe })
    setReducibilityStatus name .reducible

private def rowIdent (family : Name) (row : NRow) : GenM Ident := do
  if let some (_, name) := (← get).rows.find? (·.1 == row) then return rootIdent name
  let name := family ++ Name.mkSimple s!"row{(← get).rows.size}"
  defineLiteral name (mkConst ``NRow) (← liftTermElabM (quoteRow row))
  modify fun literals => { literals with rows := literals.rows.push (row, name) }
  return rootIdent name

private def sharingIdent (family : Name) (shared : Sharing) : GenM Ident := do
  if let some (_, name) := (← get).sharings.find? (·.1 == shared) then return rootIdent name
  let name := family ++ Name.mkSimple s!"sharing{(← get).sharings.size}"
  defineLiteral name (mkConst ``Sharing) (toExpr shared)
  modify fun literals => { literals with sharings := literals.sharings.push (shared, name) }
  return rootIdent name

private def typeIdent (family : Name) (τ : NTy) : GenM Ident := do
  if let some (_, name) := (← get).types.find? (·.1 == τ) then return rootIdent name
  let name := family ++ Name.mkSimple s!"type{(← get).types.size}"
  defineLiteral name (mkConst ``NTy) (← liftTermElabM (quoteNTy τ))
  modify fun literals => { literals with types := literals.types.push (τ, name) }
  return rootIdent name

mutual
/-- The carrier of a captured type at the family under definition, spelled
concretely: a function type as the family at its index, a type holding
function values as the products and sums down to them, any other type as
its carrier at the ground family.

A closure-free type's carrier is the same at every family, so the spelling
names a family it can name before the family under definition exists. It must:
the carrier of a struct or a variant is a row (`HList`), which does not reduce
away the family it is given, so spelling such a capture at the family under
definition would put that family under the `Prod` of its row — a nested
occurrence the kernel admits only in a positive position. -/
private partial def spellType (family : Name) (reference : Ident) (τ : NTy) : GenM Term := do
  if τ.closureFree then
    return ← `(@VerexIR.Proofs.Denote.NTy.carrier VerexIR.Proofs.Denote.Carriers.ground
      $(← typeIdent family τ))
  match τ with
  | .function parameters shared results =>
      `($reference $(← rowIdent family parameters) $(← sharingIdent family shared)
        $(← rowIdent family results))
  | .tuple elements | .struct _ _ elements => spellRow family reference elements
  | .enum _ _ names rows _ => spellVariants family reference names rows
  | .ghost slot => spellType family reference slot
  | _ => throwError "internal: a captured type is not carried natively"

private partial def spellRow (family : Name) (reference : Ident) : NRow → GenM Term
  | .nil => `(Unit)
  | .cons τ rest => do
      `(($(← spellType family reference τ)) × ($(← spellRow family reference rest)))

private partial def spellVariants (family : Name) (reference : Ident) :
    List String → NRows → GenM Term
  | _ :: names, .cons fields rest => do
      `(($(← spellRow family reference fields)) ⊕
        ($(← spellVariants family reference names rest)))
  | _, _ => `(Empty)
end

/-- An encoding transformed where it exists. -/
private def mapEncoded {α β : Type} (encoded : Except Ident α) (f : α → GenM β) :
    GenM (Except Ident β) :=
  match encoded with
  | .ok value => return .ok (← f value)
  | .error absent => return .error absent

/-- A pattern over a captured value and what its encoding is from the
pattern's variables: the runtime value, or the variable of an empty type
the pattern cannot match. -/
private abbrev Alternative := Term × Except Ident Term

private def quoteHandleSyntax (source : VerexIR.StructHandle) : Term :=
  Syntax.mkApp (mkCIdent ``VerexIR.StructHandle.mk)
    #[Syntax.mkApp (mkCIdent ``VerexIR.NamespaceId.mk) #[quote source.namespaceId.index],
      quote source.structId]

mutual
/-- The patterns of a captured value and their encodings, as the
encoder's arms take them apart. -/
private partial def typeAlternatives (unitIdent : Ident) (family : Name) (τ : NTy) (fresh : IO.Ref Nat) :
    GenM (Array Alternative) := do
  let freshVariable : GenM Ident := do
    let index ← fresh.modifyGet fun index => (index, index + 1)
    return mkIdent (Name.mkSimple s!"capture{index}")
  if τ.closureFree then
    let x ← freshVariable
    return #[(x, .ok (← `(VerexIR.Proofs.Denote.NTy.encodeWith (unit := $unitIdent) (C := $(rootIdent family))
      (fun _ _ _ _ => default) $(← typeIdent family τ) $x)))]
  match τ with
  | .function _ _ _ =>
      let x ← freshVariable
      return #[(x, .ok (← `(VerexIR.Proofs.Denote.ClosureValue.encode
        ($(mkIdent (family ++ `encode)) $x))))]
  | .tuple elements =>
      (← rowAlternatives unitIdent family elements fresh).mapM fun (pattern, encoded) => do
        return (pattern, ← mapEncoded encoded fun values =>
          `(VerexIR.RuntimeValue.tuple (List.toArray [$values,*])))
  | .struct source _ fields =>
      (← rowAlternatives unitIdent family fields fresh).mapM fun (pattern, encoded) => do
        return (pattern, ← mapEncoded encoded fun values =>
          `(VerexIR.RuntimeValue.nominal $(quoteHandleSyntax source) none
            (List.toArray [$values,*])))
  | .enum source _ names rows _ => variantAlternatives unitIdent family source names rows fresh
  | .ghost slot =>
      (← typeAlternatives unitIdent family slot fresh).mapM fun (pattern, encoded) => do
        return (pattern, ← mapEncoded encoded fun value => `(VerexIR.RuntimeValue.ghost $value))
  | _ => throwError "internal: a captured type is not carried natively"

private partial def rowAlternatives (unitIdent : Ident) (family : Name) (row : NRow) (fresh : IO.Ref Nat) :
    GenM (Array (Term × Except Ident (Array Term))) := do
  match row with
  | .nil => return #[(← `(()), .ok #[])]
  | .cons τ rest =>
      let heads ← typeAlternatives unitIdent family τ fresh
      let tails ← rowAlternatives unitIdent family rest fresh
      let mut combined := #[]
      for (headPattern, head) in heads do
        for (tailPattern, tail) in tails do
          let pattern ← `(($headPattern, $tailPattern))
          combined := combined.push (pattern, match head, tail with
            | .ok value, .ok values => .ok (#[value] ++ values)
            | .error absent, _ => .error absent
            | _, .error absent => .error absent)
      return combined

private partial def variantAlternatives (unitIdent : Ident) (family : Name) (source : VerexIR.StructHandle)
    (names : List String) (rows : NRows) (fresh : IO.Ref Nat) : GenM (Array Alternative) := do
  match names, rows with
  | name :: names, .cons fields rest =>
      let here ← (← rowAlternatives unitIdent family fields fresh).mapM fun (pattern, encoded) => do
        return (← `(Sum.inl $pattern), ← mapEncoded encoded fun values =>
          `(VerexIR.RuntimeValue.nominal $(quoteHandleSyntax source) (some $(quote name))
            (List.toArray [$values,*])))
      let later ← (← variantAlternatives unitIdent family source names rest fresh).mapM
        fun (pattern, encoded) => do return (← `(Sum.inr $pattern), encoded)
      return here ++ later
  | _, _ =>
      let index ← fresh.modifyGet fun index => (index, index + 1)
      let absent := mkIdent (Name.mkSimple s!"absent{index}")
      return #[(absent, .error absent)]
end

/-- The theorem that a unit's requires meaning holds of every value where
values no closure site encodes read as `True`. -/
def everywhereName (segments : Array String) : Name :=
  (functionRequiresName segments).appendAfter "_everywhere"

/-- Whether a site target's generated precondition is `True`: its target
declares none. -/
private def declaresNoRequires (name : Name) : CommandElabM Bool := do
  let some info := (← getEnv).find? name | return false
  let some value := info.value? | return false
  let rec body : Lean.Expr → Lean.Expr
    | .lam _ _ inner _ => body inner
    | e => e
  return (body value).isConstOf ``True

/-- Whether a closure target's declared precondition states a behavioral
predicate, which would read the requires meaning itself. -/
def siteRequiresRejected (unit : ValidatedUnit) (target : FunctionHandle) : Bool :=
  match (unit.namespaces[target.namespaceId.index]?).bind (·.functions[target.functionId.index]?) with
  | some declaration => Contract.preconditionReadsUnit unit target.namespaceId declaration
  | none => false

/-- A site target's declared precondition, at the base frame or, of a
target with type arguments, at the frame its instantiation induces: a
definition over the arguments and the state, and its unfolding a
normalization lemma. -/
private def defineSiteRequires (unit : ValidatedUnit) (unitDefinition instanceName : Name)
    (name : Name) (target : FunctionHandle) (instantiation? : Option Name) : CommandElabM Unit := do
  let some ns := unit.namespaces[target.namespaceId.index]?
    | throwError "internal: a closure site's target has no namespace"
  let some declaration := ns.functions[target.functionId.index]?
    | throwError "internal: a closure site's target has no declaration"
  liftTermElabM do
    let frameOf := fun (unitExpr functions : Lean.Expr) => do
      let runtimeFrame := mkApp2 (mkConst ``Skolems.runtime) unitExpr functions
      match instantiation? with
      | some arguments =>
          pure (mkApp4 (mkConst ``Skolems.instantiate) unitExpr functions (mkConst arguments)
            runtimeFrame)
      | none => pure runtimeFrame
    let requires ← Contract.buildDeclaredRequires unit target.namespaceId ns declaration frameOf
    let unitExpr := mkConst unitDefinition
    let instanceExpr := mkConst instanceName
    let value := (mkApp2 requires unitExpr instanceExpr).headBeta
    let type ← inferType value
    addDecl (.defnDecl {
      name, levelParams := [], type, value, hints := .regular 0, safety := .safe })
    enableRealizationsForConst name

/-- Declare a unit's requires meaning over its function values
(`RequiresMeaning`), given the reading of a value no closure site encodes: a
site's constructor reads as its target's declared precondition at the woven
arguments, at its instantiation; `foreign` by the given reading. Its
equations per constructor are normalization lemmas. -/
private def ensureFunctionRequires (segments : Array String) (unit : ValidatedUnit)
    (unitDefinition instanceName : Name) : CommandElabM Unit := do
  let family := functionValuesName segments
  let unitIdent := rootIdent unitDefinition
  let requiresIdent := rootIdent (functionRequiresName segments)
  let runtimeFrame ← `(VerexIR.Proofs.Denote.Skolems.runtime $unitIdent)
  let runtimeCarriers ← `(VerexIR.Proofs.Denote.Carriers.runtime $unitIdent)
  let closures ← closureSitesOf segments unit
  let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  let mut equations : Array (Nat × Term × Ident) := #[]
  -- Whether every site's value reads as `True`: no target declares a precondition.
  let mut everywhere := true
  for site in closures, index in [0:closures.size] do
    let name (base : String) := family ++ Name.mkSimple s!"{base}{index}"
    let some (full, _) := closureSignature? unit site.target
      | throwError "internal: a closure site's target has no signature"
    let some (capturedOwn, suppliedOwn, _) := closureRows? unit site.target site.mask
      | throwError "internal: a closure site's target has no rows"
    let fullOwn := NRow.ofList full
    -- The target's own weave over its own rows.
    let bits := (List.range full.length).map fun position => site.mask / 2 ^ position % 2 == 1
    let mut weave ← `(VerexIR.Proofs.Denote.Weave.nil)
    for captured in bits.reverse do
      weave ← if captured then `(VerexIR.Proofs.Denote.Weave.captured $weave)
        else `(VerexIR.Proofs.Denote.Weave.supplied $weave)
    let literal (base : String) (row : NRow) : CommandElabM Ident := do
      let literalName := name base
      defineLiteral literalName (mkConst ``NRow) (← liftTermElabM (quoteRow row))
      return rootIdent literalName
    let instantiation? ← if site.instantiation != .nil then do
        let arguments := name "arguments"
        let row ← literal "instantiation" site.instantiation
        elabCommand (← `(command|
          /-- A closure site's type arguments. -/
          noncomputable def $(rootIdent arguments):ident : VerexIR.Proofs.Denote.TypeArgs :=
            ⟨$row, by decide⟩))
        pure (some arguments)
      else pure none
    let ctor := rootIdent (name "site")
    let captures := mkIdent `captures
    let args := mkIdent `args
    let state := mkIdent `state
    -- A precondition stating a behavioral predicate would read the meaning
    -- itself: a contract reading the meaning is rejected (`requiresMeaningTerm`),
    -- and no meaning reads the site's target's precondition: uninterpreted.
    if siteRequiresRejected unit site.target then
      everywhere := false
      arms := arms.push (← `(Lean.Parser.Term.matchAltExpr|
        | $ctor $captures, $args => VerexIR.Proofs.requiresForeign $unitIdent _ _ _
            ($ctor $captures) $args $state))
      continue
    let targetRequires := name "requiresTarget"
    defineSiteRequires unit unitDefinition instanceName targetRequires site.target instantiation?
    unless ← declaresNoRequires targetRequires do everywhere := false
    elabCommand (← `(command| attribute [vir_denote_norm] $(rootIdent targetRequires):ident))
    let fullRow ← literal "fullRow" fullOwn
    let capturedRow ← literal "capturedRow" capturedOwn
    let suppliedRow ← literal "suppliedRow" suppliedOwn
    let siteRow ← literal "siteRow" site.captured
    let bodyFor (capturesTerm : Term) : CommandElabM Term := match instantiation? with
      | none => `($(rootIdent targetRequires)
          (@VerexIR.Proofs.Denote.Weave.compose $unitIdent _ $runtimeFrame $fullRow $capturedRow
            $suppliedRow $weave $capturesTerm $args) $state)
      | some arguments => `($(rootIdent targetRequires)
          (@VerexIR.Proofs.Denote.Weave.compose $unitIdent _
            (VerexIR.Proofs.Denote.Skolems.instantiate $(rootIdent arguments) $runtimeFrame)
            $fullRow $capturedRow $suppliedRow $weave
            (@VerexIR.Proofs.Denote.HList.toSkolem $runtimeCarriers $(rootIdent arguments)
              $capturedRow $capturesTerm)
            (@VerexIR.Proofs.Denote.HList.toSkolem $runtimeCarriers $(rootIdent arguments)
              $suppliedRow $args)) $state)
    let body ← bodyFor captures
    arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $ctor $captures, $args => $body))
    equations := equations.push (index, body, siteRow)
  let foreignCtor := rootIdent (family ++ `foreign)
  let (key, args, state, foreign) := (mkIdent `key, mkIdent `args, mkIdent `state, mkIdent `foreign)
  arms := arms.push (← `(Lean.Parser.Term.matchAltExpr|
    | $foreignCtor $key, $args => $foreign _ _ _ ($foreignCtor $key) $args $state))
  elabCommand (← `(command|
    /-- The unit's requires meaning: a closure site's value reads as its
    target's declared precondition at the woven arguments, any other value by
    `foreign`. -/
    noncomputable def $requiresIdent:ident
        ($foreign : VerexIR.Proofs.RequiresMeaning $unitIdent) :
        VerexIR.Proofs.RequiresMeaning $unitIdent :=
      fun _ _ _ value $args:ident $state:ident => match value, $args:ident with $arms:matchAlt*))
  for (index, body, siteRow) in equations do
    let ctor := rootIdent (family ++ Name.mkSimple s!"site{index}")
    elabCommand (← `(command|
      @[vir_denote_norm] theorem $(rootIdent (family ++ Name.mkSimple s!"requires{index}")):ident
          ($foreign : VerexIR.Proofs.RequiresMeaning $unitIdent)
          ($(mkIdent `captures) : @VerexIR.Proofs.Denote.HList $runtimeCarriers $siteRow)
          $args:ident $state:ident :
          $requiresIdent $foreign _ _ _ ($ctor $(mkIdent `captures)) $args $state = $body := rfl))
  elabCommand (← `(command|
    @[vir_denote_norm] theorem $(rootIdent (family ++ `requiresForeign)):ident
        ($foreign : VerexIR.Proofs.RequiresMeaning $unitIdent) {parameters shared results} $key:ident
        $args:ident $state:ident :
        $requiresIdent $foreign parameters shared results ($foreignCtor $key) $args $state =
          $foreign parameters shared results ($foreignCtor $key) $args $state := rfl))
  if everywhere then
    let lemmas : Array Ident := equations.flatMap fun (index, _, _) =>
      #[rootIdent (family ++ Name.mkSimple s!"requires{index}"),
        rootIdent (family ++ Name.mkSimple s!"requiresTarget{index}")]
    let lemmas := lemmas.push (rootIdent (family ++ `requiresForeign))
    let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
      lemmas.mapM fun ident => `(Lean.Parser.Tactic.simpLemma| $ident:ident)
    elabCommand (← `(command|
      /-- No closure site's target declares a precondition: where any other
      value is defined, every value is. -/
      theorem $(rootIdent (everywhereName segments)):ident :
          ∀ parameters shared results value $args:ident $state:ident,
            $requiresIdent (fun _ _ _ _ _ _ => True) parameters shared results value $args
              $state := by
        intro _ _ _ value _ _
        cases value <;>
          simp only [$requiresIdent:ident, $lemmas,*]))

/-- Name of a unit's nominal values: the instance that supports no
declaration natively, which this stage gives every unit
(`NominalCarriers.none`). -/
def nominalCarriersName (segments : Array String) : Name :=
  (unitName segments).getPrefix ++ `nominalCarriers

/-- Declare the unit's native nominal types.  No declaration of this unit has
a generated native carrier yet, so the unit supports none
(`NominalCarriers.none`).  Declared before the unit's function family, whose
second interpretation reads the instance and whose function values carry it
(`SiteFamily.functionCarriers`). -/
private def ensureNominalCarriers (segments : Array String) (unitIdent : Ident) :
    CommandElabM Unit := do
  if (← getEnv).contains (nominalCarriersName segments) then return
  let nominalsId ← `(Lean.Parser.Command.declId|
    $(rootIdent (nominalCarriersName segments)):ident)
  elabCommand (← `(command|
    @[reducible] noncomputable instance $nominalsId:declId :
        VerexIR.Proofs.Denote.NominalCarriers $unitIdent :=
      VerexIR.Proofs.Denote.NominalCarriers.none $unitIdent))

/-- Declare the data invariants of a unit's declarations and the instance
that carries them, given its base function values: `ensureDataOwn` over the
base family, then the instance proper, the base with `own := dataOwn` where
any declaration contributes one and the base itself otherwise. -/
private def declareCarrierInstance (segments : Array String) (unit : ValidatedUnit)
    (unitIdent baseIdent : Ident)
    (instanceId : TSyntax ``Lean.Parser.Command.declId) : CommandElabM Unit := do
  let own ← ensureDataOwn segments unit
  match own.predicate with
  | some _ =>
      elabCommand (← `(command|
        /-- The unit's function values with the data invariants its
        declarations state (`dataOwn`). -/
        noncomputable instance $instanceId:declId :
            VerexIR.Proofs.Denote.FunctionCarriers $unitIdent :=
          { $baseIdent with own := $(rootIdent (dataOwnName segments)) }))
      -- The field of the instance, which `Carriers.own_runtime` leaves in an
      -- obligation at a pack or a loan's death: the projection of the
      -- structure update is iota, so the equation is `rfl`.
      elabCommand (← `(command|
        /-- The unit's `Carriers.own`: the data invariants its declarations
        state. -/
        @[vir_denote_norm] theorem $(rootIdent (ownEqName segments)):ident :
            @VerexIR.Proofs.Denote.FunctionCarriers.own $unitIdent
                $(rootIdent (functionCarriersName segments)) =
              $(rootIdent (dataOwnName segments)) := rfl))
  | none =>
      elabCommand (← `(command|
        /-- The unit's function values with its declarations' data
        invariants.  No declaration of this unit states one carried by
        `own`, so the instance is the base. -/
        noncomputable instance $instanceId:declId :
            VerexIR.Proofs.Denote.FunctionCarriers $unitIdent := $baseIdent))

/-- Generate the native family, its encoder, and the base carrier instance. -/
private def defineFunctionFamily (segments : Array String) (unit : ValidatedUnit)
    (unitIdent baseIdent familyIdent : Ident) (instanceId baseId : TSyntax ``Lean.Parser.Command.declId)
    (family : Name) (sites : Array SiteSpec) : GenM Unit := do
  ensureNominalCarriers segments unitIdent
  -- The sites, as the family's index and the decoder read them.
  let mut specs := #[]
  for site in sites, index in [0:sites.size] do
    let keyName := family ++ Name.mkSimple s!"key{index}"
    defineLiteral keyName (mkConst ``SiteKey) (mkAppN (mkConst ``SiteKey.mk)
      #[toExpr site.key.function, toExpr site.key.mask, toExpr site.key.typeInstantiation])
    specs := specs.push (mkAppN (mkConst ``SiteSpec.mk)
      #[mkConst keyName, ← liftTermElabM (quoteRow site.parameters), toExpr site.shared,
        ← liftTermElabM (quoteRow site.results), ← liftTermElabM (quoteRow site.captured)])
  defineLiteral (family ++ `sites) (← liftTermElabM (mkAppM ``List #[mkConst ``SiteSpec]))
    (← liftTermElabM (mkListLit (mkConst ``SiteSpec) specs.toList))
  let sitesIdent := rootIdent (family ++ `sites)
  -- The family.
  let mut constructors : Array (TSyntax ``Lean.Parser.Command.ctor) := #[]
  for site in sites, index in [0:sites.size] do
    let ctorId := mkIdent (Name.mkSimple s!"site{index}")
    let captured ← spellRow family (mkIdent family) site.captured
    let parameters ← rowIdent family site.parameters
    let shared ← sharingIdent family site.shared
    let results ← rowIdent family site.results
    constructors := constructors.push (← `(Lean.Parser.Command.ctor|
      | $ctorId:ident ($(mkIdent `captures) : $captured) :
          $(mkIdent family) $parameters $shared $results))
  constructors := constructors.push (← `(Lean.Parser.Command.ctor|
    | $(mkIdent `foreign):ident {$(mkIdent `parameters) : VerexIR.Proofs.Denote.NRow}
        {$(mkIdent `shared) : VerexIR.Proofs.Denote.Sharing}
        {$(mkIdent `results) : VerexIR.Proofs.Denote.NRow}
        ($(mkIdent `key) : VerexIR.Proofs.Denote.ForeignClosure $unitIdent
          (VerexIR.Proofs.Denote.isSiteWith $unitIdent $sitesIdent) $(mkIdent `parameters)
          $(mkIdent `shared) $(mkIdent `results)) :
        $(mkIdent family) $(mkIdent `parameters) $(mkIdent `shared) $(mkIdent `results)))
  elabCommand (← `(command|
    /-- The unit's function values: a constructor per closure site, holding its
    captures, and every other closure the unit types. -/
    inductive $familyIdent:ident : VerexIR.Proofs.Denote.NRow → VerexIR.Proofs.Denote.Sharing →
        VerexIR.Proofs.Denote.NRow → Type where
      $constructors:ctor*))
  -- The encoder, structural on the family.
  let fresh ← IO.mkRef 0
  let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  let mut siteArms : Array (Array Alternative) := #[]
  for site in sites, index in [0:sites.size] do
    let ctor := mkIdent (family ++ Name.mkSimple s!"site{index}")
    let key := rootIdent (family ++ Name.mkSimple s!"key{index}")
    let alternatives := (← rowAlternatives unitIdent family site.captured fresh).map
      fun (pattern, encoded) => (pattern, encoded.map fun values =>
        (Unhygienic.run `(VerexIR.Proofs.Denote.SiteKey.closure $key [$values,*])))
    siteArms := siteArms.push alternatives
    for (pattern, encoded) in alternatives do
      let body ← match encoded with
        | .ok closure => pure closure
        | .error absent => `(nomatch $absent:ident)
      arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $ctor $pattern => $body))
  arms := arms.push (← `(Lean.Parser.Term.matchAltExpr|
    | $(mkIdent (family ++ `foreign)) key => key.closure))
  elabCommand (← `(command|
    /-- A function value's runtime closure. -/
    noncomputable def $(rootIdent (family ++ `encode)):ident :
        {parameters : VerexIR.Proofs.Denote.NRow} → {shared : VerexIR.Proofs.Denote.Sharing} →
        {results : VerexIR.Proofs.Denote.NRow} → $familyIdent parameters shared results →
          VerexIR.Proofs.Denote.ClosureValue :=
      fun value => match value with $arms:matchAlt*))
  -- The constructors by index, and the cases of a value.
  let count := sites.size
  let outOfRange ← `(by simp only [$sitesIdent:ident, List.length_cons, List.length_nil] at *; omega)
  let mut siteAlts : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  let mut encodeAlts : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  let mut caseAlts : Array (TSyntax ``Lean.Parser.Tactic.inductionAlt) := #[]
  for index in [0:count] do
    let ctor := rootIdent (family ++ Name.mkSimple s!"site{index}")
    let ctorName := mkIdent (Name.mkSimple s!"site{index}")
    siteAlts := siteAlts.push (← `(Lean.Parser.Term.matchAltExpr|
      | ⟨$(quote index), _⟩, captures => $ctor captures))
    let mut proofAlts : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
    for (pattern, encoded) in siteArms[index]! do
      proofAlts := proofAlts.push (← match encoded with
        | .ok _ => `(Lean.Parser.Term.matchAltExpr| | $pattern => rfl)
        | .error absent => `(Lean.Parser.Term.matchAltExpr| | $pattern => nomatch $absent:ident))
    encodeAlts := encodeAlts.push (← `(Lean.Parser.Term.matchAltExpr|
      | ⟨$(quote index), _⟩, captures => by
          exact match captures with $proofAlts:matchAlt*))
    caseAlts := caseAlts.push (← `(Lean.Parser.Tactic.inductionAlt|
      | $ctorName:ident captures =>
          exact Or.inl ⟨⟨$(quote index), by decide⟩, captures, ⟨rfl, rfl, rfl⟩, rfl⟩))
  siteAlts := siteAlts.push (← `(Lean.Parser.Term.matchAltExpr|
    | ⟨_ + $(quote count), bound⟩, _ => absurd bound $outOfRange))
  encodeAlts := encodeAlts.push (← `(Lean.Parser.Term.matchAltExpr|
    | ⟨_ + $(quote count), bound⟩, _ => absurd bound $outOfRange))
  caseAlts := caseAlts.push (← `(Lean.Parser.Tactic.inductionAlt|
    | $(mkIdent `foreign):ident key => exact Or.inr ⟨key, rfl⟩))
  let siteIdent := rootIdent (family ++ `site)
  let encodeIdent := rootIdent (family ++ `encode)
  elabCommand (← `(command|
    /-- The constructor of a site, by its index. -/
    noncomputable def $siteIdent:ident :
        (index : Fin ($sitesIdent).length) →
        VerexIR.Proofs.Denote.HListWith $unitIdent $familyIdent (($sitesIdent).get index).captured →
          $familyIdent (($sitesIdent).get index).parameters (($sitesIdent).get index).shared
            (($sitesIdent).get index).results :=
      fun index captures => match index, captures with $siteAlts:matchAlt*))
  elabCommand (← `(command|
    theorem $(rootIdent (family ++ `encode_site)):ident :
        ∀ (index : Fin ($sitesIdent).length)
          (captures : VerexIR.Proofs.Denote.HListWith $unitIdent $familyIdent
            (($sitesIdent).get index).captured),
          $encodeIdent ($siteIdent index captures) =
            (($sitesIdent).get index).key.closure
              (VerexIR.Proofs.Denote.NRow.encodeWith (fun _ _ _ value => $encodeIdent value)
                (($sitesIdent).get index).captured captures) :=
      fun index captures => match index, captures with $encodeAlts:matchAlt*))
  elabCommand (← `(command|
    theorem $(rootIdent (family ++ `cases)):ident {parameters : VerexIR.Proofs.Denote.NRow}
        {shared : VerexIR.Proofs.Denote.Sharing} {results : VerexIR.Proofs.Denote.NRow}
        (value : $familyIdent parameters shared results) :
        (∃ index captures, ∃ same : parameters = (($sitesIdent).get index).parameters ∧
            shared = (($sitesIdent).get index).shared ∧
            results = (($sitesIdent).get index).results,
          value = VerexIR.Proofs.Denote.castIndex same ($siteIdent index captures)) ∨
        ∃ key, value = $(rootIdent (family ++ `foreign)) key := by
      cases value with
      $caseAlts:inductionAlt*))
  -- What the unit's evaluation decides of the sites, once.
  elabCommand (← `(command|
    theorem $(rootIdent (family ++ `distinct)):ident :
        VerexIR.Proofs.Denote.sitesDistinct $sitesIdent = true := by decide +kernel))
  elabCommand (← `(command|
    theorem $(rootIdent (family ++ `typed)):ident :
        ∀ index, (($sitesIdent).get index).typed $unitIdent = true :=
      VerexIR.Proofs.Denote.SiteSpec.typed_of_all (by decide +kernel)))
  elabCommand (← `(command|
    /-- The unit's function values as a family of closure sites. -/
    @[reducible] noncomputable def $(rootIdent (family ++ `family)):ident :
        VerexIR.Proofs.Denote.SiteFamily $unitIdent where
      F := $familyIdent
      sites := $sitesIdent
      site := $siteIdent
      foreign := $(rootIdent (family ++ `foreign))
      encode := $encodeIdent
      encode_site := $(rootIdent (family ++ `encode_site))
      encode_foreign := fun _ => rfl
      cases := $(rootIdent (family ++ `cases))
      distinct := $(rootIdent (family ++ `distinct))
      typed := $(rootIdent (family ++ `typed))))
  elabCommand (← `(command|
    /-- The link set's function values before its declarations' data
    invariants are stated of them: `Carriers.own` at its default, so that
    the invariants are generated over a family that exists before the
    instance proper (`doc/designs/data-invariants.md`, "Data
    invariants"). -/
    @[reducible] noncomputable def $baseId:declId :
        VerexIR.Proofs.Denote.FunctionCarriers $unitIdent :=
      VerexIR.Proofs.Denote.SiteFamily.functionCarriers $(rootIdent (family ++ `family))
        (fun _ _ _ => True)))
  declareCarrierInstance segments unit unitIdent baseIdent instanceId

mutual
private def tyNominalFree : NTy → Bool
  | .nominal _ _ _ => false
  | .tuple elements | .struct _ _ elements => rowNominalFree elements
  | .enum _ _ _ rows _ => rowsNominalFree rows
  | .vector element | .slice element | .array element _ | .ghost element
  | .observed element | .sliceRef element | .ref element => tyNominalFree element
  | .table key value => tyNominalFree key && tyNominalFree value
  | _ => true
termination_by structural τ => τ

private def rowNominalFree : NRow → Bool
  | .nil => true
  | .cons τ rest => tyNominalFree τ && rowNominalFree rest
termination_by structural row => row

private def rowsNominalFree : NRows → Bool
  | .nil => true
  | .cons fields rest => rowNominalFree fields && rowsNominalFree rest
termination_by structural rows => rows
end

private theorem congrArg₂ {α : Sort u} {β : Sort v} {γ : Sort w} (f : α → β → γ)
    {a₁ a₂ : α} {b₁ b₂ : β} (ha : a₁ = a₂) (hb : b₁ = b₂) : f a₁ b₁ = f a₂ b₂ := by
  subst ha; subst hb; rfl

private theorem heq_prod {α α' β β' : Type} (hα : α = α') (hβ : β = β')
    {w : α × β} {v : α' × β'} (same : HEq w v) : HEq w.1 v.1 ∧ HEq w.2 v.2 := by
  subst hα; subst hβ; cases eq_of_heq same; exact ⟨HEq.rfl, HEq.rfl⟩

private theorem heq_sum_inl {α α' β β' : Type} (hα : α = α') (hβ : β = β')
    {a : α} {v : α' ⊕ β'} (same : HEq (Sum.inl a : α ⊕ β) v) :
    ∃ a' : α', v = Sum.inl a' ∧ HEq a a' := by
  subst hα; subst hβ; cases eq_of_heq same; exact ⟨a, rfl, HEq.rfl⟩

private theorem heq_sum_inr {α α' β β' : Type} (hα : α = α') (hβ : β = β')
    {b : β} {v : α' ⊕ β'} (same : HEq (Sum.inr b : α ⊕ β) v) :
    ∃ b' : β', v = Sum.inr b' ∧ HEq b b' := by
  subst hα; subst hβ; cases eq_of_heq same; exact ⟨b, rfl, HEq.rfl⟩

private theorem heq_specVector {α α' : Type} (h : α = α') {f : α → VerexIR.RuntimeValue}
    {g : α' → VerexIR.RuntimeValue} (fg : ∀ x y, HEq x y → f x = g y)
    {w : VerexIR.SpecVector α} {v : VerexIR.SpecVector α'} (same : HEq w v) :
    w.values.map f = v.values.map g := by
  subst h; cases eq_of_heq same; exact Array.map_congr_left fun x _ => fg x x HEq.rfl

private theorem heq_specArray {α α' : Type} {n : Nat} (h : α = α') {f : α → VerexIR.RuntimeValue}
    {g : α' → VerexIR.RuntimeValue} (fg : ∀ x y, HEq x y → f x = g y)
    {w : VerexIR.SpecArray α n} {v : VerexIR.SpecArray α' n} (same : HEq w v) :
    w.vector.values.map f = v.vector.values.map g := by
  subst h; cases eq_of_heq same; exact Array.map_congr_left fun x _ => fg x x HEq.rfl

private theorem heq_specEntries {κ κ' ν ν' : Type} (hκ : κ = κ') (hν : ν = ν')
    {fk : κ → VerexIR.RuntimeValue} {gk : κ' → VerexIR.RuntimeValue}
    {fv : ν → VerexIR.RuntimeValue} {gv : ν' → VerexIR.RuntimeValue}
    (fgk : ∀ x y, HEq x y → fk x = gk y) (fgv : ∀ x y, HEq x y → fv x = gv y)
    {w : VerexIR.SpecEntries (κ × ν)} {v : VerexIR.SpecEntries (κ' × ν')} (same : HEq w v) :
    (w.entries.map fun e => VerexIR.RuntimeValue.tuple #[fk e.1, fv e.2]) =
      (v.entries.map fun e => VerexIR.RuntimeValue.tuple #[gk e.1, gv e.2]) := by
  subst hκ; subst hν; cases eq_of_heq same
  exact List.map_congr_left fun e _ => by rw [fgk e.1 e.1 HEq.rfl, fgv e.2 e.2 HEq.rfl]

private theorem heq_specSliceRef {α α' : Type} (h : α = α') {f : α → VerexIR.RuntimeValue}
    {g : α' → VerexIR.RuntimeValue} (fg : ∀ x y, HEq x y → f x = g y)
    {w : VerexIR.SpecSliceRef α} {v : VerexIR.SpecSliceRef α'} (same : HEq w v) :
    w.currentVector.values.map f = v.currentVector.values.map g ∧
      w.finalVector.values.map f = v.finalVector.values.map g := by
  subst h; cases eq_of_heq same
  exact ⟨Array.map_congr_left fun x _ => fg x x HEq.rfl,
    Array.map_congr_left fun x _ => fg x x HEq.rfl⟩

mutual
private theorem carrierWith_runtime {unit : ValidatedUnit} [NominalCarriers unit]
    (Φ : SiteFamily unit) (own : VerexIR.StructHandle → NRow → VerexIR.RuntimeValue → Prop) :
    (τ : NTy) → tyNominalFree τ = true →
    NTy.carrierWith unit Φ.F τ = @NTy.carrier (@Carriers.runtime unit (Φ.functionCarriers own)) τ
  | .tuple elements, free | .struct _ _ elements, free => HListWith_runtime Φ own elements free
  | .enum _ _ names rows _, free => variantCarrierWith_runtime Φ own names rows free
  | .vector element, free | .slice element, free =>
      congrArg VerexIR.SpecVector (carrierWith_runtime Φ own element free)
  | .array element length, free =>
      congrArg (VerexIR.SpecArray · length) (carrierWith_runtime Φ own element free)
  | .table key value, free => by
      simp only [tyNominalFree, Bool.and_eq_true] at free
      exact congrArg VerexIR.SpecEntries
        (congrArg₂ Prod (carrierWith_runtime Φ own key free.1) (carrierWith_runtime Φ own value free.2))
  | .ghost slot, free => carrierWith_runtime Φ own slot free
  | .observed referent, free => carrierWith_runtime Φ own referent free
  | .sliceRef element, free =>
      congrArg VerexIR.SpecSliceRef (carrierWith_runtime Φ own element free)
  | .ref referent, free =>
      congrArg₂ Prod (carrierWith_runtime Φ own referent free) (carrierWith_runtime Φ own referent free)
  | .nominal _ _ _, free => by simp [tyNominalFree] at free
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _
  | .string, _ | .bytes, _ | .char, _ | .param _, _ | .function _ _ _, _ => rfl
termination_by structural τ _ => τ

private theorem HListWith_runtime {unit : ValidatedUnit} [NominalCarriers unit]
    (Φ : SiteFamily unit) (own : VerexIR.StructHandle → NRow → VerexIR.RuntimeValue → Prop) :
    (row : NRow) → rowNominalFree row = true →
    HListWith unit Φ.F row = @HList (@Carriers.runtime unit (Φ.functionCarriers own)) row
  | .nil, _ => rfl
  | .cons τ rest, free => by
      simp only [rowNominalFree, Bool.and_eq_true] at free
      exact congrArg₂ Prod (carrierWith_runtime Φ own τ free.1) (HListWith_runtime Φ own rest free.2)
termination_by structural row _ => row

private theorem variantCarrierWith_runtime {unit : ValidatedUnit} [NominalCarriers unit]
    (Φ : SiteFamily unit) (own : VerexIR.StructHandle → NRow → VerexIR.RuntimeValue → Prop) :
    (names : List String) → (rows : NRows) → rowsNominalFree rows = true →
    variantCarrierWith unit Φ.F names rows =
      @variantCarrier (@Carriers.runtime unit (Φ.functionCarriers own)) names rows
  | _, .nil, _ | [], .cons _ _, _ => rfl
  | _ :: names, .cons fields rest, free => by
      simp only [rowsNominalFree, Bool.and_eq_true] at free
      exact congrArg₂ Sum (HListWith_runtime Φ own fields free.1)
        (variantCarrierWith_runtime Φ own names rest free.2)
termination_by structural _ rows _ => rows
end

mutual
private theorem tyEncodeWith_runtime {unit : ValidatedUnit} [NominalCarriers unit]
    (Φ : SiteFamily unit) (own : VerexIR.StructHandle → NRow → VerexIR.RuntimeValue → Prop) :
    (τ : NTy) → (free : tyNominalFree τ = true) →
    (w : NTy.carrierWith unit Φ.F τ) →
    (v : @NTy.carrier (@Carriers.runtime unit (Φ.functionCarriers own)) τ) →
    HEq w v →
    @NTy.encodeWith unit _ Φ.F (fun _ _ _ value => Φ.encode value) τ w =
      @NTy.encode (@Carriers.runtime unit (Φ.functionCarriers own)) τ v
  | .unit, _, _, _, same | .bool, _, _, _, same | .int _ _, _, _, _, same
  | .num, _, _, _, same | .address, _, _, _, same | .signer, _, _, _, same
  | .string, _, _, _, same | .bytes, _, _, _, same | .char, _, _, _, same
  | .param _, _, _, _, same | .function _ _ _, _, _, _, same => by
      cases eq_of_heq same; rfl
  | .tuple elements, free, w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      simp only [NTy.encodeWith, NTy.encode_tuple]
      congr 1; congr 1
      exact rowEncodeWith_runtime Φ own elements free w v same
  | .struct source _ fields, free, w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      simp only [NTy.encodeWith, NTy.encode_struct]
      congr 1; congr 1
      exact rowEncodeWith_runtime Φ own fields free w v same
  | .enum source arguments names rows distinct, free, w, v, same =>
      rowsEncodeWith_runtime Φ own source arguments names rows distinct free w v same
  | .slice element, free, w, v, same | .vector element, free, w, v, same => by
      simp only [NTy.encodeWith, NTy.encode, NTy.codec, VerexIR.Proofs.Codec.boundedVector]
      congr 1
      exact heq_specVector (carrierWith_runtime Φ own element free)
        (fun x y h => tyEncodeWith_runtime Φ own element free x y h) same
  | .array element _, free, w, v, same => by
      simp only [NTy.encodeWith, NTy.encode, NTy.codec, VerexIR.Proofs.Codec.array,
        VerexIR.Proofs.Codec.boundedVector]
      congr 1
      exact heq_specArray (carrierWith_runtime Φ own element free)
        (fun x y h => tyEncodeWith_runtime Φ own element free x y h) same
  | .table key value, free, w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      simp only [tyNominalFree, Bool.and_eq_true] at free
      simp only [NTy.encodeWith, NTy.encode_table]
      congr 1; congr 1
      exact heq_specEntries (carrierWith_runtime Φ own key free.1)
        (carrierWith_runtime Φ own value free.2)
        (fun x y h => tyEncodeWith_runtime Φ own key free.1 x y h)
        (fun x y h => tyEncodeWith_runtime Φ own value free.2 x y h) same
  | .ghost slot, free, w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      simp only [NTy.encodeWith, NTy.encode_ghost]
      congr 1
      exact tyEncodeWith_runtime Φ own slot free w v same
  | .observed referent, free, w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      rw [NTy.encodeWith, NTy.encode_observed]
      exact tyEncodeWith_runtime Φ own referent free w v same
  | .sliceRef element, free, w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      have ⟨hc, hf⟩ := heq_specSliceRef (carrierWith_runtime Φ own element free)
        (fun x y h => tyEncodeWith_runtime Φ own element free x y h) same
      have hc' : w.currentVector.values.map _ = v.currentVector.values.map element.codec.encode := hc
      have hf' : w.finalVector.values.map _ = v.finalVector.values.map element.codec.encode := hf
      dsimp only [NTy.encodeWith]
      rw [NTy.encode_sliceRef, NTy.encode_vector, NTy.encode_vector, hc', hf']
  | .ref referent, free, w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      have ⟨h1, h2⟩ := heq_prod (carrierWith_runtime Φ own referent free)
        (carrierWith_runtime Φ own referent free) same
      dsimp only [NTy.encodeWith]
      rw [NTy.encode_ref,
        tyEncodeWith_runtime Φ own referent free w.1 v.1 h1,
        tyEncodeWith_runtime Φ own referent free w.2 v.2 h2]
  | .nominal _ _ _, free, _, _, _ => by simp [tyNominalFree] at free
termination_by structural τ _ _ _ _ => τ

private theorem rowEncodeWith_runtime {unit : ValidatedUnit} [NominalCarriers unit]
    (Φ : SiteFamily unit) (own : VerexIR.StructHandle → NRow → VerexIR.RuntimeValue → Prop) :
    (row : NRow) → (free : rowNominalFree row = true) →
    (w : HListWith unit Φ.F row) →
    (v : @HList (@Carriers.runtime unit (Φ.functionCarriers own)) row) →
    HEq w v →
    @NRow.encodeWith unit _ Φ.F (fun _ _ _ value => Φ.encode value) row w =
      @HList.encode (@Carriers.runtime unit (Φ.functionCarriers own)) row v
  | .nil, _, _, _, _ => rfl
  | .cons τ rest, free, w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      simp only [rowNominalFree, Bool.and_eq_true] at free
      have ⟨h1, h2⟩ := heq_prod (carrierWith_runtime Φ own τ free.1)
        (HListWith_runtime Φ own rest free.2) same
      dsimp only [NRow.encodeWith]
      rw [HList.encode_cons,
        tyEncodeWith_runtime Φ own τ free.1 w.1 v.1 h1,
        rowEncodeWith_runtime Φ own rest free.2 w.2 v.2 h2]
termination_by structural row _ _ _ _ => row

private theorem rowsEncodeWith_runtime {unit : ValidatedUnit} [NominalCarriers unit]
    (Φ : SiteFamily unit) (own : VerexIR.StructHandle → NRow → VerexIR.RuntimeValue → Prop)
    (source : VerexIR.StructHandle) (arguments : NRow) :
    (names : List String) → (rows : NRows) → (distinct : names.Nodup) →
    (free : rowsNominalFree rows = true) →
    (w : variantCarrierWith unit Φ.F names rows) →
    (v : @variantCarrier (@Carriers.runtime unit (Φ.functionCarriers own)) names rows) →
    HEq w v →
    @NRows.encodeWith unit _ Φ.F (fun _ _ _ value => Φ.encode value) source names rows w =
      @NTy.encode (@Carriers.runtime unit (Φ.functionCarriers own))
        (.enum source arguments names rows distinct) v
  | _, .nil, _, _, w, _, _ => nomatch w
  | [], .cons _ _, _, _, w, _, _ => nomatch w
  | name :: names, .cons fields rest, distinct, free, .inl w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      simp only [rowsNominalFree, Bool.and_eq_true] at free
      obtain ⟨v', rfl, sameFields⟩ := heq_sum_inl
        (HListWith_runtime Φ own fields free.1)
        (variantCarrierWith_runtime Φ own names rest free.2) same
      rw [NRows.encodeWith, NTy.encode_enum_inl,
        rowEncodeWith_runtime Φ own fields free.1 w v' sameFields]
  | name :: names, .cons fields rest, distinct, free, .inr w, v, same => by
      letI : Carriers := @Carriers.runtime unit (Φ.functionCarriers own)
      simp only [rowsNominalFree, Bool.and_eq_true] at free
      obtain ⟨v', rfl, sameRest⟩ := heq_sum_inr
        (HListWith_runtime Φ own fields free.1)
        (variantCarrierWith_runtime Φ own names rest free.2) same
      rw [NRows.encodeWith, NTy.encode_enum_inr]
      exact rowsEncodeWith_runtime Φ own source arguments names rest
        (List.nodup_cons.mp distinct).2 free.2 w v' sameRest
termination_by structural _ rows _ _ _ _ _ => rows
end

/-- Generate construction and encoding equations, retaining the sites for equality. -/
private def defineSiteConstruction (unitIdent familyIdent : Ident) (family : Name)
    (sites : Array SiteSpec) :
    GenM (Array (Nat × SiteSpec × Ident × Ident × Ident × Ident)) := do
  -- Each spelled carrier is the base frame's.
  for site in sites, index in [0:sites.size] do
    let captured ← spellRow family familyIdent site.captured
    let row ← rowIdent family site.captured
    elabCommand (← `(command|
      theorem $(rootIdent (family ++ Name.mkSimple s!"carrier{index}")):ident :
          @VerexIR.Proofs.Denote.HList
            (VerexIR.Proofs.Denote.Carriers.runtime $unitIdent) $row = $captured := rfl))
  -- Construction at the base frame: a site's key is its constructor, the
  -- mask and instantiation evaluated by the caller.
  let runtimeFrame ← `(VerexIR.Proofs.Denote.Skolems.runtime $unitIdent)
  let runtimeCarriers ← `(VerexIR.Proofs.Denote.Carriers.runtime $unitIdent)
  let mut erasedSites : Array (Nat × SiteSpec × Ident × Ident × Ident × Ident) := #[]
  for site in sites, index in [0:sites.size] do
    let row ← rowIdent family site.captured
    let parameters ← rowIdent family site.parameters
    let shared ← sharingIdent family site.shared
    let results ← rowIdent family site.results
    let key := rootIdent (family ++ Name.mkSimple s!"key{index}")
    let ctor := rootIdent (family ++ Name.mkSimple s!"site{index}")
    let encodeCapturesIdent := rootIdent (family ++ Name.mkSimple s!"encodeCaptures{index}")
    elabCommand (← `(command|
      theorem $encodeCapturesIdent:ident
          (captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row) :
          @VerexIR.Proofs.Denote.NRow.encodeWith $unitIdent _ $familyIdent
              (fun _ _ _ value => $(rootIdent (family ++ `encode)) value) $row captures =
            @VerexIR.Proofs.Denote.HList.encode $runtimeCarriers $row captures :=
        rowEncodeWith_runtime $(rootIdent (family ++ `family))
          (@VerexIR.Proofs.Denote.FunctionCarriers.own $unitIdent _) $row (by decide)
          captures captures HEq.rfl))
    let encodeSite := rootIdent (family ++ Name.mkSimple s!"encodeSite{index}")
    elabCommand (← `(command|
      theorem $encodeSite:ident
          (captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row) :
          VerexIR.Proofs.Denote.SiteFamily.encode $(rootIdent (family ++ `family))
              (VerexIR.Proofs.Denote.SiteFamily.site $(rootIdent (family ++ `family))
                ⟨$(quote index), by decide⟩ captures) =
            @VerexIR.Proofs.Denote.closureOf $unitIdent _ $runtimeFrame ($key).function
              ($key).mask ($key).typeInstantiation $row captures := by
        rw [VerexIR.Proofs.Denote.SiteFamily.encode_site_eq]
        exact VerexIR.Proofs.Denote.ClosureValue.ext rfl rfl rfl
          (congrArg List.toArray ($encodeCapturesIdent:ident captures))))
    elabCommand (← `(command|
      theorem $(rootIdent (family ++ Name.mkSimple s!"construct{index}")):ident
          (mask : Nat) (masked : mask = ($key).mask)
          (instantiation : Array (VerexIR.TypeId × VerexIR.TypeId))
          (instantiated : instantiation = ($key).typeInstantiation)
          (captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row)
          (typed : @VerexIR.Proofs.Denote.SiteTyped $unitIdent _ $runtimeFrame $parameters $shared
            $results ($key).function mask instantiation $row) :
          @VerexIR.Proofs.Denote.Skolems.construct $unitIdent _ $runtimeFrame $parameters $shared
            $results ($key).function mask instantiation $row captures typed =
              $ctor captures := by
        subst masked instantiated
        exact VerexIR.Proofs.Denote.SiteFamily.construct_eq $(rootIdent (family ++ `family))
          (σs := $row) captures typed
          (VerexIR.Proofs.Denote.SiteFamily.site $(rootIdent (family ++ `family))
            ⟨$(quote index), by decide⟩ captures)
          ($encodeSite:ident captures).symm))
    -- What a program's equality of two function values decides: their
    -- closures without ghost slots (`TypedFunctions.erased`). A site's is
    -- its key over the erased encoding of its captures.
    let erasedKey := rootIdent (family ++ Name.mkSimple s!"erasedKey{index}")
    elabCommand (← `(command|
      theorem $erasedKey:ident
          (captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row) :
          (@VerexIR.Proofs.Denote.Carriers.functions $runtimeCarriers $parameters $shared
              $results).erased ($ctor captures) =
            VerexIR.Proofs.Denote.ErasedClosure.mk
              (VerexIR.RuntimeValue.closure ($key).function ($key).mask
                ($key).typeInstantiation
                (VerexIR.RuntimeValue.eraseGhostsAll
                  (@VerexIR.Proofs.Denote.HList.encode $runtimeCarriers $row
                    captures)).toArray) := by
        show VerexIR.Proofs.Denote.ErasedClosure.mk
          (VerexIR.Proofs.Denote.SiteFamily.encode $(rootIdent (family ++ `family))
            (VerexIR.Proofs.Denote.SiteFamily.site $(rootIdent (family ++ `family))
              ⟨$(quote index), by decide⟩ captures)).encode.eraseGhosts = _
        rw [$encodeSite:ident captures]
        simp only [VerexIR.Proofs.Denote.closureOf, VerexIR.Proofs.Denote.ClosureValue.encode,
          VerexIR.RuntimeValue.eraseGhosts]))
    erasedSites := erasedSites.push (index, site, row, parameters, shared, results)
  return erasedSites

/-- Generate equality equations from the erased closures of the sites. -/
private def defineSiteEquality (unitIdent : Ident) (family : Name)
    (erasedSites : Array (Nat × SiteSpec × Ident × Ident × Ident × Ident)) : GenM Unit := do
  let runtimeCarriers ← `(VerexIR.Proofs.Denote.Carriers.runtime $unitIdent)
  -- Values of two sites differ by their keys, and values of one site compare
  -- by the native equality of their captures, which is itself modulo ghost
  -- slots (`HList.equal`). The family's `site…injEq` is never reached:
  -- the equality reads the erased closures, not the family's values.
  for (index, site, row, parameters, shared, results) in erasedSites do
    for (other, otherSite, otherRow, _, _, _) in erasedSites do
      unless site.parameters == otherSite.parameters && site.shared == otherSite.shared
          && site.results == otherSite.results do continue
      let left := mkIdent `left
      let right := mkIdent `right
      let erased (which : Nat) (value : Ident) : CommandElabM Term :=
        `((@VerexIR.Proofs.Denote.Carriers.functions $runtimeCarriers $parameters $shared
              $results).erased
            ($(rootIdent (family ++ Name.mkSimple s!"site{which}")) $value))
      let statement ← if index == other then
          `(($(← erased index left) = $(← erased other right)) ↔
            @VerexIR.Proofs.Denote.HList.equal $runtimeCarriers false $row $left $right =
              true)
        else `(($(← erased index left) = $(← erased other right)) ↔ False)
      let theoremId := mkIdent (family ++ Name.mkSimple s!"erased{index}_{other}")
      let keys : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
        (if index == other then #[index] else #[index, other]).mapM fun which =>
          `(Lean.Parser.Tactic.simpLemma|
            $(rootIdent (family ++ Name.mkSimple s!"erasedKey{which}")):ident)
      if index == other then
        elabCommand (← `(command|
          @[vir_denote_norm] theorem $theoremId:ident
              ($left : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row)
              ($right : @VerexIR.Proofs.Denote.HList $runtimeCarriers $otherRow) :
              $statement := by
            simp only [$keys,*, VerexIR.Proofs.Denote.ErasedClosure.mk.injEq,
              VerexIR.RuntimeValue.closure.injEq, true_and, Array.mk.injEq,
              VerexIR.Proofs.Denote.HList.equal_codec, decide_eq_true_eq,
              VerexIR.Proofs.Denote.HList.encode]))
      else
        -- A site's entire key distinguishes it. The same target can occur
        -- at different capture masks or type instantiations.
        let distinct : TSyntax `tactic ←
          if site.key.function != otherSite.key.function then
            `(tactic| (intro same; exact absurd same (by decide)))
          else if site.key.mask != otherSite.key.mask then
            `(tactic| (intro _ same; exact absurd same (by decide)))
          else
            `(tactic| (intro _ _ same; exact absurd same (by decide)))
        elabCommand (← `(command|
          @[vir_denote_norm] theorem $theoremId:ident
              ($left : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row)
              ($right : @VerexIR.Proofs.Denote.HList $runtimeCarriers $otherRow) :
              $statement := by
            simp only [$keys,*, VerexIR.Proofs.Denote.ErasedClosure.mk.injEq,
              VerexIR.RuntimeValue.closure.injEq, iff_false, not_and]
            $distinct:tactic))

/-- Generate and register the invocation equations for closed sites. -/
private def defineSiteEquations (unit : ValidatedUnit) (unitDefinition : Name)
    (unitIdent : Ident) (family : Name) (sites : Array SiteSpec) : GenM Unit := do
  let runtimeFrame ← `(VerexIR.Proofs.Denote.Skolems.runtime $unitIdent)
  let runtimeCarriers ← `(VerexIR.Proofs.Denote.Carriers.runtime $unitIdent)
  -- The equations of a site without type arguments: its weave, its
  -- constructor's closure, the unit's check, its invocation, and the typing
  -- of the value naming its target.
  let mut equipped : Array (Nat × SiteSpec × NRow) := #[]
  for site in sites, index in [0:sites.size] do
    unless site.key.typeInstantiation.isEmpty do continue
    let some (full, _) := closureSignature? unit site.key.function | continue
    unless closureTypingCheck unit site.key.function site.key.mask site.captured site.parameters
        site.results site.shared do continue
    equipped := equipped.push (index, site, NRow.ofList full)
    let name (base : String) := family ++ Name.mkSimple s!"{base}{index}"
    let row ← rowIdent family site.captured
    let parameters ← rowIdent family site.parameters
    let shared ← sharingIdent family site.shared
    let results ← rowIdent family site.results
    let fullRow ← rowIdent family (NRow.ofList full)
    let key := rootIdent (name "key")
    let ctor := rootIdent (name "site")
    let weaveIdent := rootIdent (name "weave")
    let weave ← if site.key.mask == 0 then `(VerexIR.Proofs.Denote.Weave.supplying $fullRow)
      else
        let bits := (List.range full.length).map fun position => site.key.mask / 2 ^ position % 2 == 1
        let mut term ← `(VerexIR.Proofs.Denote.Weave.nil)
        for captured in bits.reverse do
          term ← if captured then `(VerexIR.Proofs.Denote.Weave.captured $term)
            else `(VerexIR.Proofs.Denote.Weave.supplied $term)
        pure term
    elabCommand (← `(command|
      @[reducible] noncomputable def $weaveIdent:ident :
          VerexIR.Proofs.Denote.Weave $fullRow $row $parameters := $weave))
    let maskTerm ← `(VerexIR.Proofs.Denote.Weave.mask $weaveIdent)
    elabCommand (← `(command|
      theorem $(rootIdent (name "closure")):ident
          (captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row) :
          (@VerexIR.Proofs.Denote.Carriers.functions $runtimeCarriers $parameters $shared
              $results).closure ($ctor captures) =
            @VerexIR.Proofs.Denote.closureOf $unitIdent _ $runtimeFrame ($key).function $maskTerm
              #[] $row captures := by
        exact $(rootIdent (family ++ Name.mkSimple s!"encodeSite{index}")):ident captures))
    elabCommand (← `(command|
      theorem $(rootIdent (name "check")):ident :
          VerexIR.Proofs.Denote.closureTypingCheck $unitIdent ($key).function $maskTerm $row
            $parameters $results $shared = true := by decide +kernel))
    let shape? ← match site.results with
      | .nil => some <$> `(VerexIR.Proofs.Denote.ResultShape.none)
      | .cons τ .nil => some <$> `(VerexIR.Proofs.Denote.ResultShape.one $(← typeIdent family τ))
      | _ => pure none
    let invoke? ← match shape? with
      | none => pure none
      | some shape => do
          elabCommand (← `(command|
            theorem $(rootIdent (name "invoke")):ident
                (executable : VerexIR.Validation.ExecutableUnit $unitIdent)
                (captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row)
                (args : @VerexIR.Proofs.Denote.HList $runtimeCarriers $parameters) :
                @VerexIR.Proofs.Denote.closureMeaning $unitIdent _ executable $runtimeFrame
                    $parameters $shared $shape ($ctor captures) args =
                  @VerexIR.Proofs.Denote.propheticMeaning $unitIdent _ executable $runtimeFrame #[]
                    ($key).function $fullRow $shape (@VerexIR.Proofs.Denote.Weave.compose $unitIdent _ $runtimeFrame _ _ _ $weaveIdent captures args) := by
              letI : VerexIR.Proofs.Denote.Skolems $unitIdent := $runtimeFrame
              exact VerexIR.Proofs.closureMeaning_closureOf_coherent executable $weaveIdent
                (by decide) (VerexIR.Proofs.Denote.coherent_runtime $unitIdent _) captures $shared
                $shape ($ctor captures) ($(rootIdent (name "closure")) captures) args))
          pure (some (name "invoke"))
    let named? ← if site.captured == .nil then pure none else do
      let namedShared ← sharingIdent family (closureShared unit site.key.function 0)
      elabCommand (← `(command|
        theorem $(rootIdent (name "named")):ident :
            @VerexIR.Proofs.Denote.SiteTyped $unitIdent _ $runtimeFrame $fullRow $namedShared
              $results ($key).function
              (VerexIR.Proofs.Denote.Weave.mask (VerexIR.Proofs.Denote.Weave.supplying $fullRow))
              #[] VerexIR.Proofs.Denote.NRow.nil :=
          @VerexIR.Proofs.Denote.SiteTyped.ofCheck $unitIdent _ $runtimeFrame _ _ _ _ _ _
            (by decide +kernel)))
      pure (some (name "named"))
    modifyEnv fun environment => siteEquationsExtension.addEntry environment {
      unit := unitDefinition, constructor := name "site", construct := name "construct"
      closure := name "closure", invoke := invoke?, check := name "check", named := named?
      target := functionKey unit site.key.function, foreign := family ++ `foreign }
  -- The behaviors a clause states of a site that captures, at the link set's
  -- value naming its target: the captures woven into the entry values the
  -- clause supplies (`abortsOfEntries_closureOf_woven`), so that the clause's
  -- predicate of the closure and its predicate of the bare target are one
  -- term (`doc/plan/one-spelling.md`, "A closure's behavior").  Generated
  -- where the link set holds the target's mask-0 site at its full row, which
  -- is the value a clause naming the target spells.
  for (index, site, fullValue) in equipped do
    if site.captured == .nil then continue
    let mut naming? := none
    for (other, otherSite, _) in equipped do
      if naming?.isNone && otherSite.key.function == site.key.function &&
          otherSite.key.mask == 0 && otherSite.captured == .nil &&
          otherSite.parameters == fullValue && otherSite.results == site.results then
        naming? := some (other, otherSite)
    let some (named, namedSite) := naming? | continue
    let shape? ← match site.results with
      | .nil => some <$> `(VerexIR.Proofs.Denote.ResultShape.none)
      | .cons τ .nil => some <$> `(VerexIR.Proofs.Denote.ResultShape.one $(← typeIdent family τ))
      | _ => pure none
    let some shape := shape? | continue
    let row ← rowIdent family site.captured
    let parameters ← rowIdent family site.parameters
    let shared ← sharingIdent family site.shared
    let fullRow ← rowIdent family fullValue
    let namedShared ← sharingIdent family namedSite.shared
    let ctor := rootIdent (family ++ Name.mkSimple s!"site{index}")
    let closure := rootIdent (family ++ Name.mkSimple s!"closure{index}")
    let weave := rootIdent (family ++ Name.mkSimple s!"weave{index}")
    let namedCtor := rootIdent (family ++ Name.mkSimple s!"site{named}")
    let namedClosure := rootIdent (family ++ Name.mkSimple s!"closure{named}")
    let executable := mkIdent `executable
    let captures := mkIdent `captures
    let entries := mkIdent `entries
    let finals := mkIdent `finals
    let result := mkIdent `result
    let state := mkIdent `state
    let pre := mkIdent `pre
    let post := mkIdent `post
    let held := mkIdent `held
    let meaning ← `(@VerexIR.Proofs.Denote.closureMeaning $unitIdent _ $executable $runtimeFrame)
    -- A behavior at the woven entry values is stated of a valid row, the
    -- captures among it, so the equation holds of captures that hold their
    -- data invariants: a premise where the captured row states one, and by
    -- the row's types alone where it states none.  An equation without the
    -- premise is a normalization rule; one with it is the site's, read by
    -- the leaf that holds the captures' fact (`guardedWovenExtension`).
    let stateEquation (name : Name) : CommandElabM Unit := do
      if site.captured.invariantFree then
        elabCommand (← `(command| attribute [vir_denote_norm] $(rootIdent name):ident))
      else
        modifyEnv fun environment => guardedWovenExtension.addEntry environment
          (family ++ Name.mkSimple s!"site{index}", name)
    let capturesValid : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ←
      if site.captured.invariantFree then pure #[]
      else do
        let binder ← `(Lean.Parser.Term.bracketedBinderF|
          ($held : @VerexIR.Proofs.Denote.HList.Holds $runtimeCarriers $row $captures))
        pure #[⟨binder.raw⟩]
    let heldTerm ← if site.captured.invariantFree then
        `((@VerexIR.Proofs.Denote.HList.holds_iff $runtimeCarriers $row $captures).mpr
          (@VerexIR.Proofs.Denote.HList.deepHolds_of_invariantFree $runtimeCarriers $row
            (by decide) $captures))
      else `($held)
    let woven ← `(@VerexIR.Proofs.Denote.Weave.compose $unitIdent _ $runtimeFrame $fullRow $row
      $parameters $weave $captures $entries)
    elabCommand (← `(command|
      theorem $(rootIdent (family ++ Name.mkSimple s!"wovenAborts{index}")):ident
          ($executable : VerexIR.Validation.ExecutableUnit $unitIdent)
          ($captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row)
          $capturesValid*
          ($entries : @VerexIR.Proofs.Denote.HList $runtimeCarriers $parameters)
          ($state : @VerexIR.Proofs.Denote.Memory $unitIdent _) :
          @VerexIR.Proofs.AbortsOfEntries $unitIdent _ $runtimeFrame $meaning $parameters
              $shared $shape ($ctor $captures) $entries $state =
            @VerexIR.Proofs.AbortsOfEntries $unitIdent _ $runtimeFrame $meaning $fullRow
              $namedShared $shape ($namedCtor ()) $woven $state := by
        letI : VerexIR.Proofs.Denote.Skolems $unitIdent := $runtimeFrame
        exact VerexIR.Proofs.abortsOfEntries_closureOf_woven (named_shared := $namedShared)
          $weave (by decide) #[] $captures $heldTerm $shared $shape ($ctor $captures)
          ($closure $captures) ($namedCtor ()) ($namedClosure ()) $entries $state))
    stateEquation (family ++ Name.mkSimple s!"wovenAborts{index}")
    elabCommand (← `(command|
      theorem $(rootIdent (family ++ Name.mkSimple s!"wovenEnsures{index}")):ident
          ($executable : VerexIR.Validation.ExecutableUnit $unitIdent)
          ($captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row)
          $capturesValid*
          ($entries : @VerexIR.Proofs.Denote.HList $runtimeCarriers $parameters)
          ($finals : @VerexIR.Proofs.Denote.HList $runtimeCarriers
            (VerexIR.Proofs.Denote.NRow.finals $parameters))
          ($result : @VerexIR.Proofs.Denote.ResultShape.carrier $runtimeCarriers $shape)
          ($pre $post : @VerexIR.Proofs.Denote.Memory $unitIdent _) :
          @VerexIR.Proofs.EnsuresOfEntries $unitIdent _ $runtimeFrame $meaning $parameters
              $shared $shape ($ctor $captures) $entries $finals $result
              $pre $post =
            @VerexIR.Proofs.EnsuresOfEntries $unitIdent _ $runtimeFrame $meaning $fullRow
              $namedShared $shape ($namedCtor ()) $woven $finals $result
              $pre $post := by
        letI : VerexIR.Proofs.Denote.Skolems $unitIdent := $runtimeFrame
        exact VerexIR.Proofs.ensuresOfEntries_closureOf_woven (named_shared := $namedShared)
          (finals := $finals) (finals' := $finals) $weave (by decide) #[] $captures $heldTerm
          $shared $shape ($ctor $captures) ($closure $captures) ($namedCtor ())
          ($namedClosure ()) $entries HEq.rfl $result $pre $post))
    stateEquation (family ++ Name.mkSimple s!"wovenEnsures{index}")
    -- A target returning a mutable reference: the clause reads the result as
    -- its current value (`EnsuresOfSettled`).
    if let .cons (.ref referent) .nil := site.results then
      let σ ← typeIdent family referent
      let current := mkIdent `current
      elabCommand (← `(command|
        theorem
            $(rootIdent (family ++ Name.mkSimple s!"wovenEnsuresSettled{index}")):ident
            ($executable : VerexIR.Validation.ExecutableUnit $unitIdent)
            ($captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row)
            $capturesValid*
            ($entries : @VerexIR.Proofs.Denote.HList $runtimeCarriers $parameters)
            ($finals : @VerexIR.Proofs.Denote.HList $runtimeCarriers
              (VerexIR.Proofs.Denote.NRow.finals $parameters))
            ($current : @VerexIR.Proofs.Denote.NTy.carrier $runtimeCarriers $σ)
            ($pre $post : @VerexIR.Proofs.Denote.Memory $unitIdent _) :
            @VerexIR.Proofs.EnsuresOfSettled $unitIdent _ $runtimeFrame $meaning $parameters
                $shared $σ ($ctor $captures) $entries $finals $current $pre $post =
              @VerexIR.Proofs.EnsuresOfSettled $unitIdent _ $runtimeFrame $meaning $fullRow
                $namedShared $σ ($namedCtor ()) $woven $finals $current $pre $post := by
          letI : VerexIR.Proofs.Denote.Skolems $unitIdent := $runtimeFrame
          exact VerexIR.Proofs.ensuresOfSettled_closureOf_woven (named_shared := $namedShared)
            (finals := $finals) (finals' := $finals) $weave (by decide) #[] $captures $heldTerm
            $shared $σ ($ctor $captures) ($closure $captures) ($namedCtor ()) ($namedClosure ())
            $entries HEq.rfl $current $pre $post))
      stateEquation (family ++ Name.mkSimple s!"wovenEnsuresSettled{index}")

/-- Declare a unit's function values once, before its first theorem: the
generated family where the link set has closure sites, the library's
otherwise.  In two steps, the base family whose data invariants are
trivially true and the instance proper (`functionCarriersBaseName`). -/
def ensureFunctionCarriers (segments : Array String) (unit : ValidatedUnit) :
    CommandElabM Name := withNominalUnitCommand (Contract.unitName segments) do
  let name := functionCarriersName segments
  if (← getEnv).contains name then return name
  let unitDefinition ← ensureUnitDefinition segments unit
  let unitIdent := rootIdent unitDefinition
  let instanceId ← `(Lean.Parser.Command.declId| $(rootIdent name):ident)
  let baseName := functionCarriersBaseName segments
  let baseId ← `(Lean.Parser.Command.declId| $(rootIdent baseName):ident)
  let baseIdent := rootIdent baseName
  let sites := siteSpecs (← closureSitesOf segments unit)
  let requiresIdent := rootIdent (functionRequiresName segments)
  if sites.isEmpty then
    elabCommand (← `(command|
      /-- The unit's function values before its declarations' data invariants
      are stated of them: `Carriers.own` at its default, so that the
      invariants are generated over a family that exists before the instance
      proper (`doc/designs/data-invariants.md`, "Data invariants"). -/
      @[reducible] noncomputable def $baseId:declId :
          VerexIR.Proofs.Denote.FunctionCarriers $unitIdent :=
        VerexIR.Proofs.Denote.FunctionCarriers.runtime $unitIdent))
    declareCarrierInstance segments unit unitIdent baseIdent instanceId
    -- Every value is one no site encodes.
    elabCommand (← `(command|
      /-- The unit's requires meaning: no closure site encodes a value. -/
      noncomputable def $requiresIdent:ident
          (foreign : VerexIR.Proofs.RequiresMeaning $unitIdent) :
          VerexIR.Proofs.RequiresMeaning $unitIdent := foreign))
    elabCommand (← `(command|
      /-- Where any value is defined, every value is. -/
      theorem $(rootIdent (everywhereName segments)):ident :
          ∀ parameters shared results value args state,
            $requiresIdent (fun _ _ _ _ _ _ => True) parameters shared results value args state :=
        fun _ _ _ _ _ _ => trivial))
    return name
  -- The sites must be closed, carried natively, keyed apart, and typed.
  for site in sites do
    let target := functionKey unit site.key.function
    unless site.parameters.paramFree && site.results.paramFree do
      throwError m!"the closure site of `{target}` leaves a type parameter open"
    for τ in site.captured.toList do
      if let some reason := capturedUnsupported? τ then
        throwError m!"the closure site of `{target}` captures {reason}, which a function \
          value of the unit cannot hold"
    unless site.typed unit do
      throwError m!"internal: the unit does not type the closure site of `{target}`"
  unless sitesDistinct sites.toList do
    throwError "internal: two closure sites share a target, mask, and instantiation"
  let family := functionValuesName segments
  let familyIdent := rootIdent family
  let generate : GenM Unit := do
    defineFunctionFamily segments unit unitIdent baseIdent familyIdent instanceId baseId family sites
    let erasedSites ← defineSiteConstruction unitIdent familyIdent family sites
    defineSiteEquality unitIdent family erasedSites
    defineSiteEquations unit unitDefinition unitIdent family sites
  discard <| generate.run {}
  ensureFunctionRequires segments unit unitDefinition name
  return name

/-- The unit's constant with its function values declared: what every
theorem about the unit is stated at. -/
def ensureUnitInstance (segments : Array String) (unit : ValidatedUnit) : CommandElabM Name := do
  let name ← ensureUnitDefinition segments unit
  discard <| ensureFunctionCarriers segments unit
  return name

/-- The function values of a unit's link set for display: per closure
site its constructor, function type, target, and captured row. -/
def displayFunctionValues (unit : ValidatedUnit) (sites : Array ClosureSite) : String :=
  "\n".intercalate <| sites.toList.zipIdx.map fun (site, index) =>
    let instantiation := if site.instantiation == .nil then ""
      else s!"<{displayRow unit site.instantiation}>"
    s!"site{index} : {displayFunctionType unit site.type} := \
      {functionKey unit site.target}{instantiation} mask {site.mask} \
      captures ({displayRow unit site.captured})"

end VerexSL.Verify
