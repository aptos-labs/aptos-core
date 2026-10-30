-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.Contracts
import VerexIR.Proofs.Denote.FunctionValues

/-!
# Generated function values

A unit's function values (`doc/plan/native-function-values.md`, "Step 2b,
decided"), generated once per link set before the unit's first theorem: one
indexed family with a constructor per closure site at its closed rows,
holding the site's captures natively, and `foreign` at every function type
(`ForeignClosure`); its encoder, structural on the family; and the
`SiteFamily` the library derives the decoder and the laws of
`FunctionCarriers` from. A unit without closure sites keeps the library's
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
reading of a value no site encodes (`RequiresMeaning`). -/
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
  | .tuple elements | .struct _ _ elements => rowUnsupported? elements
  | .enum _ _ _ rows _ => rowsUnsupported? rows
  | .ghost slot => capturedUnsupported? slot
  | τ@(.vector _) | τ@(.table _ _) =>
      if !τ.paramFree then some "a type parameter"
      else if τ.closureFree then none
      else some "a function value inside a vector or a map"
  | .ref _ => some "a reference"
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
its carrier. -/
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
private partial def typeAlternatives (family : Name) (τ : NTy) (fresh : IO.Ref Nat) :
    GenM (Array Alternative) := do
  let freshVariable : GenM Ident := do
    let index ← fresh.modifyGet fun index => (index, index + 1)
    return mkIdent (Name.mkSimple s!"capture{index}")
  if τ.closureFree then
    let x ← freshVariable
    return #[(x, .ok (← `(VerexIR.Proofs.Denote.NTy.encodeWith (C := $(rootIdent family))
      (fun _ _ _ _ => default) $(← typeIdent family τ) $x)))]
  match τ with
  | .function _ _ _ =>
      let x ← freshVariable
      return #[(x, .ok (← `(VerexIR.Proofs.Denote.ClosureValue.encode
        ($(mkIdent (family ++ `encode)) $x))))]
  | .tuple elements =>
      (← rowAlternatives family elements fresh).mapM fun (pattern, encoded) => do
        return (pattern, ← mapEncoded encoded fun values =>
          `(VerexIR.RuntimeValue.tuple (List.toArray [$values,*])))
  | .struct source _ fields =>
      (← rowAlternatives family fields fresh).mapM fun (pattern, encoded) => do
        return (pattern, ← mapEncoded encoded fun values =>
          `(VerexIR.RuntimeValue.nominal $(quoteHandleSyntax source) none
            (List.toArray [$values,*])))
  | .enum source _ names rows _ => variantAlternatives family source names rows fresh
  | .ghost slot =>
      (← typeAlternatives family slot fresh).mapM fun (pattern, encoded) => do
        return (pattern, ← mapEncoded encoded fun value => `(VerexIR.RuntimeValue.ghost $value))
  | _ => throwError "internal: a captured type is not carried natively"

private partial def rowAlternatives (family : Name) (row : NRow) (fresh : IO.Ref Nat) :
    GenM (Array (Term × Except Ident (Array Term))) := do
  match row with
  | .nil => return #[(← `(()), .ok #[])]
  | .cons τ rest =>
      let heads ← typeAlternatives family τ fresh
      let tails ← rowAlternatives family rest fresh
      let mut combined := #[]
      for (headPattern, head) in heads do
        for (tailPattern, tail) in tails do
          let pattern ← `(($headPattern, $tailPattern))
          combined := combined.push (pattern, match head, tail with
            | .ok value, .ok values => .ok (#[value] ++ values)
            | .error absent, _ => .error absent
            | _, .error absent => .error absent)
      return combined

private partial def variantAlternatives (family : Name) (source : VerexIR.StructHandle)
    (names : List String) (rows : NRows) (fresh : IO.Ref Nat) : GenM (Array Alternative) := do
  match names, rows with
  | name :: names, .cons fields rest =>
      let here ← (← rowAlternatives family fields fresh).mapM fun (pattern, encoded) => do
        return (← `(Sum.inl $pattern), ← mapEncoded encoded fun values =>
          `(VerexIR.RuntimeValue.nominal $(quoteHandleSyntax source) (some $(quote name))
            (List.toArray [$values,*])))
      let later ← (← variantAlternatives family source names rest fresh).mapM
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
    let value := (mkApp2 requires (mkConst unitDefinition) (mkConst instanceName)).headBeta
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
  let closures := closureSites unit
  let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  let mut equations : Array (Nat × Term) := #[]
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
    -- A precondition stating a behavioral predicate would read the meaning
    -- itself: a contract reading the meaning is rejected (`requiresMeaningTerm`).
    if siteRequiresRejected unit site.target then
      everywhere := false
      continue
    let targetRequires := name "requiresTarget"
    defineSiteRequires unit unitDefinition instanceName targetRequires site.target instantiation?
    unless ← declaresNoRequires targetRequires do everywhere := false
    elabCommand (← `(command| attribute [vir_denote_norm] $(rootIdent targetRequires):ident))
    let fullRow ← literal "fullRow" fullOwn
    let capturedRow ← literal "capturedRow" capturedOwn
    let suppliedRow ← literal "suppliedRow" suppliedOwn
    let ctor := rootIdent (name "site")
    let captures := mkIdent `captures
    let args := mkIdent `args
    let state := mkIdent `state
    let body ← match instantiation? with
      | none => `($(rootIdent targetRequires)
          (@VerexIR.Proofs.Denote.Weave.compose $unitIdent _ $runtimeFrame $fullRow $capturedRow
            $suppliedRow $weave $captures $args) $state)
      | some arguments => `($(rootIdent targetRequires)
          (@VerexIR.Proofs.Denote.Weave.compose $unitIdent _
            (VerexIR.Proofs.Denote.Skolems.instantiate $(rootIdent arguments) $runtimeFrame)
            $fullRow $capturedRow $suppliedRow $weave
            (@VerexIR.Proofs.Denote.HList.toSkolem $runtimeCarriers $(rootIdent arguments)
              $capturedRow $captures)
            (@VerexIR.Proofs.Denote.HList.toSkolem $runtimeCarriers $(rootIdent arguments)
              $suppliedRow $args)) $state)
    arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $ctor $captures, $args => $body))
    equations := equations.push (index, body)
  let foreignCtor := rootIdent (family ++ `foreign)
  let (key, args, state, foreign) := (mkIdent `key, mkIdent `args, mkIdent `state, mkIdent `foreign)
  -- A site whose target's precondition no meaning reads: uninterpreted.
  for site in closures, index in [0:closures.size] do
    unless siteRequiresRejected unit site.target do continue
    let ctor := rootIdent (family ++ Name.mkSimple s!"site{index}")
    arms := arms.push (← `(Lean.Parser.Term.matchAltExpr|
      | $ctor $(mkIdent `captures), $args => VerexIR.Proofs.requiresForeign $unitIdent _ _ _
          ($ctor $(mkIdent `captures)) $args $state))
  arms := arms.push (← `(Lean.Parser.Term.matchAltExpr|
    | $foreignCtor $key, $args => $foreign _ _ _ ($foreignCtor $key) $args $state))
  elabCommand (← `(command|
    /-- The unit's requires meaning: a closure site's value reads as its
    target's declared precondition at the woven arguments, any other value by
    `foreign`. -/
    noncomputable def $requiresIdent:ident ($foreign : VerexIR.Proofs.RequiresMeaning $unitIdent) :
        VerexIR.Proofs.RequiresMeaning $unitIdent :=
      fun _ _ _ value $args:ident $state:ident => match value, $args:ident with $arms:matchAlt*))
  -- A normalization lemma per constructor.
  for (index, body) in equations do
    let ctor := rootIdent (family ++ Name.mkSimple s!"site{index}")
    elabCommand (← `(command|
      @[vir_denote_norm] theorem $(rootIdent (family ++ Name.mkSimple s!"requires{index}")):ident
          ($foreign : VerexIR.Proofs.RequiresMeaning $unitIdent) $(mkIdent `captures):ident $args:ident
          $state:ident :
          $requiresIdent $foreign _ _ _ ($ctor $(mkIdent `captures)) $args $state = $body := rfl))
  elabCommand (← `(command|
    @[vir_denote_norm] theorem $(rootIdent (family ++ `requiresForeign)):ident
        ($foreign : VerexIR.Proofs.RequiresMeaning $unitIdent) {parameters shared results} $key:ident
        $args:ident $state:ident :
        $requiresIdent $foreign parameters shared results ($foreignCtor $key) $args $state =
          $foreign parameters shared results ($foreignCtor $key) $args $state := rfl))
  if everywhere then
    let lemmas : Array Ident := equations.flatMap fun (index, _) =>
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
        cases value <;> simp only [$lemmas,*]))

/-- Declare a unit's function values once, before its first theorem: the
generated family where the link set has closure sites, the library's
otherwise. -/
def ensureFunctionCarriers (segments : Array String) (unit : ValidatedUnit) :
    CommandElabM Name := do
  let name := functionCarriersName segments
  if (← getEnv).contains name then return name
  let unitDefinition ← ensureUnitDefinition segments unit
  let unitIdent := rootIdent unitDefinition
  let instanceId ← `(Lean.Parser.Command.declId| $(rootIdent name):ident)
  let sites := siteSpecs (closureSites unit)
  let requiresIdent := rootIdent (functionRequiresName segments)
  if sites.isEmpty then
    elabCommand (← `(command|
      noncomputable instance $instanceId:declId :
          VerexIR.Proofs.Denote.FunctionCarriers $unitIdent :=
        VerexIR.Proofs.Denote.FunctionCarriers.runtime $unitIdent))
    -- Every value is one no site encodes.
    elabCommand (← `(command|
      /-- The unit's requires meaning: no closure site encodes a value. -/
      noncomputable def $requiresIdent:ident (foreign : VerexIR.Proofs.RequiresMeaning $unitIdent) :
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
      let alternatives := (← rowAlternatives family site.captured fresh).map
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
          VerexIR.Proofs.Denote.HListWith $familyIdent (($sitesIdent).get index).captured →
            $familyIdent (($sitesIdent).get index).parameters (($sitesIdent).get index).shared
              (($sitesIdent).get index).results :=
        fun index captures => match index, captures with $siteAlts:matchAlt*))
    elabCommand (← `(command|
      theorem $(rootIdent (family ++ `encode_site)):ident :
          ∀ (index : Fin ($sitesIdent).length)
            (captures : VerexIR.Proofs.Denote.HListWith $familyIdent
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
      noncomputable instance $instanceId:declId :
          VerexIR.Proofs.Denote.FunctionCarriers $unitIdent :=
        VerexIR.Proofs.Denote.SiteFamily.functionCarriers $(rootIdent (family ++ `family))))
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
    for site in sites, index in [0:sites.size] do
      let row ← rowIdent family site.captured
      let parameters ← rowIdent family site.parameters
      let shared ← sharingIdent family site.shared
      let results ← rowIdent family site.results
      let key := rootIdent (family ++ Name.mkSimple s!"key{index}")
      let ctor := rootIdent (family ++ Name.mkSimple s!"site{index}")
      elabCommand (← `(command|
        theorem $(rootIdent (family ++ Name.mkSimple s!"construct{index}")):ident
            (mask : Nat) (masked : mask = ($key).mask)
            (instantiation : Array (VerexIR.TypeId × VerexIR.TypeId))
            (instantiated : instantiation = ($key).typeInstantiation)
            (captures : @VerexIR.Proofs.Denote.HList $runtimeCarriers $row)
            (typed : @VerexIR.Proofs.Denote.SiteTyped $unitIdent _ $runtimeFrame $parameters $shared
              $results ($key).function mask instantiation $row) :
            @VerexIR.Proofs.Denote.Skolems.construct $unitIdent _ $runtimeFrame $parameters $shared
              $results ($key).function mask instantiation $row captures typed = $ctor captures := by
          subst masked instantiated
          exact VerexIR.Proofs.Denote.SiteFamily.construct_eq $(rootIdent (family ++ `family))
            (σs := $row) captures typed
            (VerexIR.Proofs.Denote.SiteFamily.site $(rootIdent (family ++ `family))
              ⟨$(quote index), by decide⟩ captures)
            (VerexIR.Proofs.Denote.SiteFamily.encode_site_eq $(rootIdent (family ++ `family))
              ⟨$(quote index), by decide⟩ captures).symm))
    -- The equations of a site without type arguments: its weave, its
    -- constructor's closure, the unit's check, its invocation, and the typing
    -- of the value naming its target.
    for site in sites, index in [0:sites.size] do
      unless site.key.typeInstantiation.isEmpty do continue
      let some (full, _) := closureSignature? unit site.key.function | continue
      unless closureTypingCheck unit site.key.function site.key.mask site.captured site.parameters
          site.results site.shared do continue
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
          show VerexIR.Proofs.Denote.SiteFamily.encode $(rootIdent (family ++ `family))
              (VerexIR.Proofs.Denote.SiteFamily.site $(rootIdent (family ++ `family))
                ⟨$(quote index), by decide⟩ captures) = _
          rw [VerexIR.Proofs.Denote.SiteFamily.encode_site_eq]
          rfl))
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
