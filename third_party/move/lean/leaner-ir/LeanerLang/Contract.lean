-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Elab
import LeanerLang.Frame
import LeanerLang.Perf
import LeanerLang.Options
import LeanerLang.Quote
import LeanerLang.Registry
import LeanerLang.SpecTypes
import LeanerLang.Syntax
import LeanerLang.ValueRep
import LeanerIR.Proofs.Meaning
import LeanerIR.Proofs.IntegerArithmetic
import LeanerIR.Proofs.Denote.Types

/-!
# Generated contracts from LIR specifications

A validated function's declared contract is translated into an ordinary Lean
`LeanerIR.Proofs.FunctionContract` at command-elaboration time.  The
translation is deliberately shallow: a specification clause becomes a Lean
proposition over Lean-typed parameter and result binders, so a proof
obligation reads like the authored specification rather than like an
interpreter state.  The runtime value rows are tied to those binders by an
explicit shape equation.

The generator is untrusted glue: whatever it produces is a statement the
Lean kernel checks, and a clause it cannot translate is a reported error
rather than a silently weakened contract.
-/

namespace LeanerLang.Contract

open Lean Meta Elab Command
open LeanerIR (RuntimeValue ConstValue Operation PrimitiveOperation ExprId
  LocalId TypeId)
open LeanerIR.Validation (ValidatedUnit ValidatedNamespace)
open LeanerLang.Quote

/-- The shared IR type node; `LeanerLang.Ty` is the surface type. -/
private abbrev IrTy := LeanerIR.Ty

/-- The definition of a recursive specification function under
construction: its bundled argument, the recursion hypothesis over smaller
bundles, and its measure. -/
structure RecursiveDefinition where
  reference : LeanerIR.QualifiedRef
  argument : Lean.Expr
  recurse : Lean.Expr
  measure : Lean.Expr

/-- Translation context for one function's specification clauses. -/
structure Context where
  unit : ValidatedUnit
  /-- The namespace owning the specification, for reference resolution. -/
  namespaceId : LeanerIR.NamespaceId
  ns : ValidatedNamespace
  /-- Lean binder for each specification local, indexed by `LocalId`. -/
  locals : Array (Option Lean.Expr)
  /-- Logical type of each local binder. References are represented
  by their referent even when a specification expression retains the
  physical reference type. -/
  localTypes : Array IrTy := #[]
  /-- Source name of each local, for the binders a clause introduces. -/
  localNames : Array String := #[]
  /-- Lean binder each local denotes under `spec.old`: the entry value of a
  mutable reference, and the ordinary binder for everything else. -/
  oldLocals : Array (Option Lean.Expr) := #[]
  /-- Lean binder for each function result. -/
  results : Array Lean.Expr
  /-- The final value (prophecy) of each returned mutable reference, where
  the clause may read it with `final`. -/
  resultFinals : Array (Option Lean.Expr) := #[]
  /-- Logical referent type of each result binder.  The LIR type carried by
  `spec.result[i]` can still be the physical reference type, while the
  specification binder deliberately denotes its referent. -/
  resultTypes : Array IrTy := #[]
  /-- Runtime state a clause reads storage from in its current position:
  the entry state in a one-state clause, the final state in `ensures`. -/
  state : Option Lean.Expr := none
  /-- Runtime state `spec.old` reads storage from. -/
  oldState : Option Lean.Expr := none
  /-- Typed twins of the unit's struct declarations. -/
  twins : Array SpecTypes.TwinInfo := #[]
  /-- Storable families with typed accessors a clause reads through. -/
  families : Array SpecTypes.FamilyInfo := #[]
  /-- Native representation environment of a generic contract. -/
  carrier : Option Lean.Expr := none
  codecs : Option Lean.Expr := none
  /-- Sparse declaration-type substitution selected for this invocation. -/
  typeInstantiation : Option Lean.Expr := none
  /-- Derived specification calls are expanded from their already-lowered
  bodies. Keep the active identities so an accidental recursive model is
  rejected instead of recursing in the command elaborator. -/
  specCallStack : Array LeanerIR.QualifiedRef := #[]
  /-- The type arguments of an expanded generic specification function,
  resolved in the contract's frame: each parameter's type and native
  representation. -/
  typeArguments : Array (IrTy × Option Typed.ValueRep) := #[]
  /-- Inside the body of a recursive specification function's definition. -/
  definition : Option RecursiveDefinition := none
  /-- The conditions on the path to the current position, which prove a
  recursive call's measure smaller. -/
  facts : Array Lean.Expr := #[]

/-- The type a specification type denotes: a parameter of an expanded
specification function is its argument. -/
private def Context.typeOf? (context : Context) (typeId : TypeId) : Option IrTy := do
  let ty ← context.unit.tables.types[typeId.index]?
  match ty with
  | .typeParameter index => (context.typeArguments[index]?).map (·.1) <|> pure ty
  | _ => pure ty

/-- The native representation of a specification type, with the parameters
of an expanded specification function replaced by their arguments'. -/
private def Context.valueRep? (context : Context) (typeId : TypeId) :
    Option Typed.ValueRep :=
  (Typed.valueRep? context.unit context.twins typeId context.ns.profile).map
    (·.substitute (context.typeArguments.map (·.2)))

/-- The logical domain a specification value of one LIR type inhabits.
Scalars get the Lean type a clause reads naturally; every aggregate stays a
`RuntimeValue`, projected by the total specification accessors, because its
shape is exactly what a clause selects fields out of. -/
private inductive Domain where
  | integer
  | boolean
  /-- Textual scalars differ only in the constructor that encodes them. -/
  | text (constructor : Name)
  | aggregate
  deriving BEq

private def domainOf (ty : IrTy) : Domain :=
  match ty with
  | .integer _ _ => .integer
  | .bool => .boolean
  | .string => .text ``RuntimeValue.string
  | .address => .text ``RuntimeValue.address
  | .signer => .text ``RuntimeValue.signer
  | _ => .aggregate

/-- The Lean type of a binder in this domain. -/
private def Domain.leanType : Domain → Lean.Expr
  | .integer => mkConst ``Int
  | .boolean => mkConst ``Bool
  | .text _ => mkConst ``String
  | .aggregate => mkConst ``LeanerIR.RuntimeValue

/-- The runtime value a binder of this domain encodes. -/
private def Domain.encode (domain : Domain) (binder : Lean.Expr) :
    MetaM Lean.Expr := do
  match domain with
  | .integer => mkAppM ``RuntimeValue.integer #[binder]
  | .boolean => mkAppM ``RuntimeValue.bool #[binder]
  | .text constructor => mkAppM constructor #[binder]
  | .aggregate => return binder

/-- The clause-level term a binder denotes.  A clause is a proposition, so a
boolean reads as its truth; every other domain reads as itself. -/
private def Domain.ofBinder (domain : Domain) (binder : Lean.Expr) :
    MetaM Lean.Expr := do
  match domain with
  | .boolean => mkEq binder (mkConst ``Bool.true)
  | _ => return binder

/-- The clause-level term a runtime value denotes in this domain, through
the total specification accessors. -/
private def Domain.ofRuntime (domain : Domain) (value : Lean.Expr) :
    MetaM Lean.Expr := do
  match domain with
  | .integer => mkAppM ``LeanerIR.RuntimeValue.asInt #[value]
  | .boolean => mkEq (← mkAppM ``LeanerIR.RuntimeValue.asBool #[value]) (mkConst ``Bool.true)
  | .text _ => mkAppM ``LeanerIR.RuntimeValue.asString #[value]
  | .aggregate => return value

/-- Decode a runtime value to the binder type of one logical domain.  This
differs from `ofRuntime` only for booleans: a local binder stores `Bool`,
while a clause which reads that binder denotes the proposition that it is
true. -/
private def Domain.binderOfRuntime (domain : Domain) (value : Lean.Expr) :
    MetaM Lean.Expr := do
  match domain with
  | .integer => mkAppM ``LeanerIR.RuntimeValue.asInt #[value]
  | .boolean => mkAppM ``LeanerIR.RuntimeValue.asBool #[value]
  | .text _ => mkAppM ``LeanerIR.RuntimeValue.asString #[value]
  | .aggregate => return value

/- Closed projection equations belong in the data-normalization inventory.
Keeping the recursive definitions themselves folded avoids branching on an
unknown runtime value, while constructor images produced by typed codecs
still reduce to their fields in one rewrite. -/
@[simp] theorem runtimeFieldNominal
    (source : LeanerIR.StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) (index : Nat) :
    RuntimeValue.field (.nominal source variant fields) index =
      fields[index]?.getD .unit := rfl

/- Enum invariant matches immediately project constructor payloads.  Keep
these literal-array rows ahead of the generic nominal row so arithmetic sees
the payload itself instead of an intermediate `getElem?` application. -/
@[simp] theorem runtimeFieldNominalZero
    (source : LeanerIR.StructHandle) (variant : Option String)
    (first : RuntimeValue) (rest : List RuntimeValue) :
    RuntimeValue.field (.nominal source variant (Array.mk (first :: rest))) 0 =
      first := rfl

@[simp] theorem runtimeFieldNominalOne
    (source : LeanerIR.StructHandle) (variant : Option String)
    (first second : RuntimeValue) (rest : List RuntimeValue) :
    RuntimeValue.field
        (.nominal source variant (Array.mk (first :: second :: rest))) 1 =
      second := rfl

@[simp] theorem runtimeAsIntInteger (value : Int) :
    RuntimeValue.asInt (.integer value) = value := rfl

@[simp] theorem runtimeAsBoolBool (value : Bool) :
    RuntimeValue.asBool (.bool value) = value := rfl

@[simp] theorem runtimeAsStringString (value : String) :
    RuntimeValue.asString (.string value) = value := rfl

@[simp] theorem runtimeAsStringAddress (value : String) :
    RuntimeValue.asString (.address value) = value := rfl

@[simp] theorem runtimeAsStringSigner (value : String) :
    RuntimeValue.asString (.signer value) = value := rfl

/-- Total logical vector update used by generated clauses.  As with the
other specification projections, an ill-shaped value is junk; successful
execution and the authored bounds condition rule that case out. -/
def updateVector (value : RuntimeValue) (index : Nat)
    (replacement : RuntimeValue) : RuntimeValue :=
  match value with
  | .vector elements => .vector (elements.set! index replacement)
  | _ => .unit

/-- Total logical vector append used by specification-side `pushVector`.
The executable primitive has the same constructor equation. -/
@[simp] def pushVector (value element : RuntimeValue) : RuntimeValue :=
  match value with
  | .vector elements => .vector (elements.push element)
  | _ => .unit

/-- Total logical concatenation corresponding to the value-level primitive. -/
@[simp] def concatVector (left right : RuntimeValue) : RuntimeValue :=
  match left, right with
  | .vector left, .vector right => .vector (left ++ right)
  | _, _ => .unit

/-- Every carrier has decidable equality, so a clause can decide a
proposition about carrier values, as when a Boolean argument of a typed
contract is passed to an expanded specification function. -/
instance [LeanerIR.Proofs.Denote.Skolems] (τ : LeanerIR.Proofs.Denote.NTy) :
    DecidableEq τ.carrier :=
  τ.decEq

/-- The meaning of a specification function without a body: a fixed
function about which nothing is known, named by the declaration's qualified
spelling and applied to its encoded arguments. -/
opaque opaqueSpec (name : String) (result : Type) [Inhabited result]
    (arguments : List RuntimeValue) : result

/-- Logical membership corresponding to the value-level `containsVector`. -/
def containsVector (value element : RuntimeValue) : Prop :=
  match value with
  | .vector elements => element ∈ elements
  | _ => False

@[simp] theorem containsVector_vector
    (elements : Array RuntimeValue) (element : RuntimeValue) :
    containsVector (.vector elements) element = (element ∈ elements) := rfl

/-- Total logical vector length used by generated clauses.  Ill-shaped
specification operands denote the same default integer value as the other
total runtime projections. -/
def lengthVector (value : RuntimeValue) : Int :=
  match value with
  | .vector elements => (elements.size : Int)
  | _ => 0

@[simp] theorem lengthVector_vector (elements : Array RuntimeValue) :
    lengthVector (.vector elements) = (elements.size : Int) := rfl

/-- Decoding determines the vector behind a precondition's runtime length.
Expose just those hypotheses before reducing indexed operations; traversing
all local facts or the whole computational goal here is unnecessary. -/
elab "leaner_normalize_vector_lengths" : tactic => do
  Lean.Elab.Tactic.withMainContext do
    let mut hypotheses := #[]
    for declaration in ← Lean.getLCtx do
      if declaration.isImplementationDetail then continue
      if declaration.type.getUsedConstants.contains ``lengthVector then
        hypotheses := hypotheses.push (mkIdent declaration.userName)
    for hypothesis in hypotheses do
      Lean.Elab.Tactic.evalTactic (← `(tactic|
        simp only [lengthVector_vector, Array.size_map, Int.ofNat_eq_natCast, Int.natCast_pos,
          Int.ofNat_lt, Int.ofNat_le] at $hypothesis:ident))



/-- Closed list lookup for specification-side enum descriptors.  Using the
literal list directly avoids elaborating `Array.find?` into a `forIn` state
machine in proof obligations. -/
@[simp] def variantIndex (variant : String) :
    List (String × Nat) → Option Nat
  | [] => none
  | (candidate, index) :: rest =>
      if candidate == variant then some index else variantIndex variant rest

@[simp] def variantMember (variant : String) : List String → Bool
  | [] => false
  | candidate :: rest =>
      if candidate == variant then true else variantMember variant rest

/-- Total enum-payload selection used by specifications after the declaration
resolver has reduced a source field name to one payload offset per variant. -/
@[simp] def selectVariantField (value : RuntimeValue)
    (owner : LeanerIR.StructHandle)
    (variants : Array (String × Nat)) : RuntimeValue :=
  match value with
  | .nominal actual (some variant) fields =>
      if actual == owner then
        match variantIndex variant variants.toList with
        | some index => fields[index]?.getD .unit
        | none => .unit
      else .unit
  | _ => .unit

/-- Total enum-variant membership used by generated specification clauses. -/
@[simp] def testVariants (value : RuntimeValue)
    (owner : LeanerIR.StructHandle)
    (variants : Array String) : Bool :=
  match value with
  | .nominal actual (some variant) _ =>
      actual == owner && variantMember variant variants.toList
  | _ => false

theorem testVariants_nominal (actual owner : LeanerIR.StructHandle) (variant : String)
    (fields : Array RuntimeValue) (variants : Array String) :
    testVariants (.nominal actual (some variant) fields) owner variants =
      (actual == owner && variantMember variant variants.toList) := rfl

theorem testVariants_nominal_self (owner : LeanerIR.StructHandle)
    (variant : String) (fields : Array RuntimeValue) (variants : Array String) :
    testVariants (.nominal owner (some variant) fields) owner variants =
      variantMember variant variants.toList := by
  simp [testVariants]

private def typeOfExpr? (context : Context) (id : ExprId) : Option IrTy := do
  let expression ← context.ns.expressions[id.index]?
  context.typeOf? expression.typeId

private def describeType (ty : IrTy) : String :=
  toString (repr ty)

/-- The typed twin a specification expression's value inhabits: a storage
read is twin-typed, and the twin domain follows through `spec.old`,
dereference, and selection of a nominal field.  Every other expression stays
in its scalar or runtime-value domain. -/
private structure AppliedTwin where
  info : SpecTypes.TwinInfo
  arguments : Array SpecTypes.FieldRep := #[]

/-- Apply a generated family accessor, selecting its invocation-aware form
when the surrounding generic contract carries a type substitution. -/
private def familyAccessor (typeInstantiation : Option Lean.Expr)
    (family : SpecTypes.FamilyInfo)
    (suffix : Name) (arguments : Array Lean.Expr) : MetaM Lean.Expr := do
  let some instantiation := typeInstantiation
    | return ← mkAppM (family.accessorName suffix) arguments
  let name := family.accessorName (Name.mkSimple (suffix.toString ++ "At"))
  let arguments := match suffix with
    | `get => arguments.insertIdx (arguments.size - 1) instantiation
    | `read => arguments.insertIdx (arguments.size - 2) instantiation
    | `key => #[instantiation] ++ arguments
    | `contains => #[instantiation] ++ arguments
    | _ => arguments
  mkAppM name arguments

private partial def twinOfExpr? (context : Context) (id : ExprId) :
    Option AppliedTwin := do
  let expression ← context.ns.expressions[id.index]?
  match expression.kind with
  | .operation (.specification (.global _)) instantiations _ _ =>
      match instantiations.toList with
      | [.typeArg resource] =>
          (context.families.find?
            (·.typeIndex == resource.typeId.index)).map fun family =>
              { info := family.info, arguments := family.arguments }
      | _ => none
  | .operation (.specification .old) _ arguments _ => do
      twinOfExpr? context (← arguments[0]?)
  | .operation (.reference .dereference) _ arguments _ => do
      twinOfExpr? context (← arguments[0]?)
  | .operation (.data (.select _ field)) _ arguments _ => do
      let base ← twinOfExpr? context (← arguments[0]?)
      let (_, rep) ← base.info.fields.find? (·.1 == field)
      match rep.instantiate base.arguments with
      | .nominal twin arguments =>
          (context.twins.find? (·.twin == twin)).map fun info =>
            { info, arguments }
      | _ => none
  | _ => none

/-- The resolved twin named by an operation's nominal reference. -/
private def twinOfReference? (context : Context)
    (reference : LeanerIR.QualifiedRef) : Option SpecTypes.TwinInfo := do
  let handle ← LeanerIR.SemanticOperations.resolveStruct?
    context.unit context.namespaceId reference
  context.twins.find? fun info =>
    info.namespaceIndex == handle.namespaceId.index &&
      info.structIndex == handle.structId

/-- Encode a native generic value when entering the total runtime vocabulary
of aggregate clauses. Values already in that vocabulary are unchanged. -/
private def runtimeAggregateValue (context : Context) (id : ExprId)
    (translated : Lean.Expr) : MetaM Lean.Expr := do
  if (← inferType translated).isConstOf ``RuntimeValue then return translated
  match twinOfExpr? context id with
  | some applied =>
      let argumentCodecs ← applied.arguments.mapM (·.codec context.codecs)
      mkAppM (applied.info.twin ++ `erase) (argumentCodecs.push translated)
  | none =>
      let some expression := context.ns.expressions[id.index]? | return translated
      if let some rep := context.valueRep? expression.typeId then
        match rep with
        | .parameter _ | .twin _ _ | .vector _ _ | .tuple _ =>
            return ← rep.encode context.codecs translated
        | _ => pure ()
      return translated

/-- The specification function a call names, with its namespace. -/
private def specFunctionOf? (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    Option (ValidatedNamespace × LeanerIR.SpecFunctionDecl) := do
  let targetNs ← unit.namespaces[reference.namespaceId.index]?
  let functionId ← unit.resolution.specFunction? reference.name
  let declaration ← targetNs.specFunctions[functionId.index]?
  pure (targetNs, declaration)

/-- Whether a specification function reaches another through the
specification functions its body calls. -/
private partial def specReaches (unit : ValidatedUnit) (source target : LeanerIR.QualifiedRef) :
    Bool := Id.run do
  let mut work : List (LeanerIR.NamespaceId × ExprId) := match specFunctionOf? unit source with
    | some (_, { body := some root, .. }) => [(source.namespaceId, root)]
    | _ => []
  let mut visited : Array (Nat × Nat) := #[]
  let bound := unit.namespaces.foldl (fun total ns => total + ns.expressions.size) 1
  for _ in [0:bound + 1] do
    match work with
    | [] => break
    | (owner, id) :: rest =>
        work := rest
        if visited.contains (owner.index, id.index) then continue
        visited := visited.push (owner.index, id.index)
        let some expression := unit.namespaces[owner.index]?.bind (·.expressions[id.index]?)
          | continue
        if let .operation (.specification (.functionCall callee _)) _ _ _ := expression.kind then
          if callee == target then return true
          if let some (_, { body := some root, .. }) := specFunctionOf? unit callee then
            work := work ++ [(callee.namespaceId, root)]
        work := work ++ (LeanerIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·)
  return false

/-- Whether a specification function reaches itself through the
specification functions its body calls. -/
private def specRecursive (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) : Bool :=
  specReaches unit reference reference

/-- The spelling of a specification function, for a diagnostic. -/
private def specFunctionSpelling (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    String :=
  ((unit.tables.names[reference.name.index]?).map (·.name)).getD (toString (repr reference))

/-- The Lean name of a recursive specification function's definition. -/
private def specDefinitionName (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    MetaM Name := do
  let some path := unit.tables.namespaces[reference.namespaceId.index]?
    | throwError "a specification function's namespace has no path"
  let some qualified := unit.tables.names[reference.name.index]?
    | throwError "a specification function has no name"
  let base := path.segments.foldl (fun name segment => Name.str name segment) .anonymous
  return Name.str (Name.str base qualified.name) "spec"

/-- Translate one specification expression into a Lean term. -/
private partial def translate (context : Context) (id : ExprId) : MetaM Lean.Expr := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "specification expression {id.index} is out of range"
  let some ty := context.typeOf? expression.typeId
    | throwError "specification expression {id.index} has an unknown type"
  match expression.kind with
  | .value literal _ => translateLiteral literal ty
  | .constant reference =>
      -- A named constant denotes its defining literal, translated in the
      -- declaring namespace.  The definition is executable-typed, so the
      -- integer arm of the literal translation accepts fixed widths.
      let some handle := LeanerIR.SemanticOperations.resolveConstant?
          context.unit context.namespaceId reference
        | throwError "specification constant does not resolve in generated contracts"
      let some targetNs := context.unit.namespaces[handle.namespaceId.index]?
        | throwError "specification constant namespace is out of range"
      let some declaration := targetNs.constants[handle.constantId]?
        | throwError "specification constant is out of range"
      translate { context with
        ns := targetNs, namespaceId := handle.namespaceId
        locals := #[], localTypes := #[], oldLocals := #[], results := #[] }
        declaration.value
  | .localVar localId =>
      let some (some binder) := context.locals[localId.index]?
        | throwError "specification local {localId.index} has no binder"
      let logicalType := context.localTypes[localId.index]?.getD ty
      (domainOf logicalType).ofBinder binder
  | .operation operation instantiations arguments _ =>
      translateOperation operation instantiations arguments ty
  | .quantifier kind binders triggers condition body =>
      unless triggers.isEmpty do
        throwError "quantifier triggers are not supported in generated contracts"
      let rec bind (index : Nat) (active : Context) : MetaM Lean.Expr := do
        if h : index < binders.size then
          let binder := binders[index]
          let some pattern := active.ns.patterns[binder.pattern.index]?
            | throwError "quantifier pattern {binder.pattern.index} is out of range"
          let .variable localId := pattern.kind
            | throwError "generated contracts currently require a variable quantifier pattern"
          let some domainExpression := active.ns.expressions[binder.domain.index]?
            | throwError "quantifier domain {binder.domain.index} is out of range"
          let .operation (.specification .typeDomain) #[.typeArg domainType] #[] _ :=
              domainExpression.kind
            | throwError "generated contracts currently require a type-domain quantifier"
          let some patternType := active.typeOf? pattern.typeId
            | throwError "quantifier pattern type {pattern.typeId.index} is out of range"
          let some declaredDomain := active.typeOf? domainType.typeId
            | throwError "quantifier domain type {domainType.typeId.index} is out of range"
          -- A bounded integer domain ranges over its values, bound at the
          -- widened logical type.
          let range? : Option Lean.Expr ← if patternType == declaredDomain then pure none else
            match patternType, declaredDomain with
            | .integer .unbounded _, .integer (.bits width) signed =>
                pure (some (mkApp2 (mkConst ``LeanerIR.IntegerValueFits)
                  (mkApp (mkConst ``LeanerIR.IntWidth.bits) (toExpr width)) (toExpr signed)))
            | _, _ => throwError "quantifier pattern and type domain differ"
          let binderName := match active.localNames[localId.index]? with
            | some name => if name.isEmpty then Name.mkSimple s!"quantified_{localId.index}"
                else Name.mkSimple name
            | none => Name.mkSimple s!"quantified_{localId.index}"
          withLocalDeclD binderName
              (domainOf patternType).leanType fun value => do
            let inner ← bind (index + 1) { active with
              locals := active.locals.set! localId.index (some value)
              oldLocals := active.oldLocals.set! localId.index (some value) }
            match kind, range? with
            | .forall, none => mkForallFVars #[value] inner
            | .forall, some range => mkForallFVars #[value] (← mkArrow (mkApp range value) inner)
            | .exists, none => mkAppM ``Exists #[← mkLambdaFVars #[value] inner]
            | .exists, some range =>
                mkAppM ``Exists #[← mkLambdaFVars #[value] (← mkAppM ``And #[mkApp range value, inner])]
            | _, _ => throwError "choice quantifiers are not supported in generated contracts"
        else
          let proposition ← translate active body
          match condition with
          | none => pure proposition
          | some guard =>
              let guard ← translate active guard
              match kind with
              | .forall => pure (← mkArrow guard proposition)
              | .exists => mkAppM ``And #[guard, proposition]
              | _ => throwError "choice quantifiers are not supported in generated contracts"
      bind 0 context
  | .match_ scrutinee arms =>
      if arms.isEmpty then
        throwError "a specification match must contain an arm"
      -- The term below deliberately makes the final arm unconditional, as
      -- source matching does after an exhaustive decision tree. Establish
      -- that invariant here instead of giving a partial match a total but
      -- unintended logical meaning.
      let mut owner? : Option LeanerIR.StructHandle := none
      let mut declaredVariants : Array String := #[]
      let mut coveredVariants : Array String := #[]
      let mut hasWildcard := false
      for (arm, index) in arms.zipIdx do
        let some pattern := context.ns.patterns[arm.pattern.index]?
          | throwError "specification match pattern {arm.pattern.index} is out of range"
        match pattern.kind with
        | .wildcard =>
            if index + 1 < arms.size then
              throwError "a wildcard arm in a specification match must be last"
            hasWildcard := true
        | .constructor owner _ (some variant) _ =>
            let some qualified := context.unit.tables.names[owner.index]?
              | throwError "specification match constructor has an unknown owner"
            let reference : LeanerIR.QualifiedRef := {
              namespaceId := qualified.namespaceId, name := owner }
            let some handle := LeanerIR.SemanticOperations.resolveStruct?
                context.unit context.namespaceId reference
              | throwError "specification match constructor does not resolve"
            if let some previous := owner? then
              unless previous == handle do
                throwError "specification match arms name different enum owners"
            else
              owner? := some handle
              let some targetNs := context.unit.namespaces[handle.namespaceId.index]?
                | throwError "specification match constructor namespace is out of range"
              let some declaration := targetNs.structs[handle.structId]?
                | throwError "specification match constructor declaration is out of range"
              declaredVariants := declaration.variants.filterMap fun declaration =>
                (context.unit.tables.names[declaration.name.index]?).map (·.name)
            unless declaredVariants.contains variant do
              throwError "specification match names unknown variant `{variant}`"
            coveredVariants := coveredVariants.push variant
        | _ =>
            throwError "generated contracts require enum-constructor or wildcard match patterns"
      unless hasWildcard || declaredVariants.all coveredVariants.contains do
        let missing := declaredVariants.filter (!coveredVariants.contains ·)
        throwError "a specification match must cover every variant (missing: \
          {String.intercalate ", " missing.toList})"
      let scrutineeValue ← runtimeAggregate scrutinee
      let mut lowered : Array (Option Lean.Expr × Lean.Expr) := #[]
      for arm in arms do
        if arm.guard.isSome then
          throwError "guards in specification matches are not supported in generated contracts"
        let some pattern := context.ns.patterns[arm.pattern.index]?
          | throwError "specification match pattern {arm.pattern.index} is out of range"
        let (condition?, armContext) ← match pattern.kind with
          | .wildcard => pure (none, context)
          | .constructor owner _ (some variant) fields => do
              let some qualified := context.unit.tables.names[owner.index]?
                | throwError "specification match constructor has an unknown owner"
              let reference : LeanerIR.QualifiedRef := {
                namespaceId := qualified.namespaceId, name := owner }
              let some handle := LeanerIR.SemanticOperations.resolveStruct?
                  context.unit context.namespaceId reference
                | throwError "specification match constructor does not resolve"
              let variantLiteral ← mkArrayLit (mkConst ``String) [toExpr variant]
              let test ← mkAppM ``LeanerLang.Contract.testVariants
                #[scrutineeValue, toExpr handle, variantLiteral]
              let condition ← mkEq test (mkConst ``Bool.true)
              let mut armLocals := context.locals
              let mut armOldLocals := context.oldLocals
              for (childId, index) in fields.zipIdx do
                let some child := context.ns.patterns[childId.index]?
                  | throwError "specification match child pattern {childId.index} is out of range"
                match child.kind with
                | .wildcard => pure ()
                | .variable localId =>
                    let selected ← mkAppM ``LeanerIR.RuntimeValue.field
                      #[scrutineeValue, toExpr index]
                    let some childType := context.typeOf? child.typeId
                      | throwError "specification match child has an unknown type"
                    let value ← match domainOf childType with
                      | .integer => mkAppM ``LeanerIR.RuntimeValue.asInt #[selected]
                      | .boolean => mkAppM ``LeanerIR.RuntimeValue.asBool #[selected]
                      | .text _ => mkAppM ``LeanerIR.RuntimeValue.asString #[selected]
                      | .aggregate => pure selected
                    armLocals := armLocals.set! localId.index (some value)
                    armOldLocals := armOldLocals.set! localId.index (some value)
                | _ =>
                    throwError "generated contracts require variable or wildcard enum payload patterns"
              pure (some condition, { context with
                locals := armLocals, oldLocals := armOldLocals })
          | _ =>
              throwError "generated contracts require enum-constructor or wildcard match patterns"
        let body ← translate armContext arm.body
        let body ← if domainOf ty == .aggregate then
            runtimeAggregateValue armContext arm.body body else pure body
        lowered := lowered.push (condition?, body)
      let mut result? : Option Lean.Expr := none
      for (condition?, body) in lowered.reverse do
        result? ← match result?, condition? with
          | none, _ => pure (some body)
          | some _, none => pure (some body)
          | some fallback, some condition =>
              some <$> mkAppOptM ``ite #[none, condition, none, body, fallback]
      let some result := result? | unreachable!
      pure result
  | .ifElse condition thenBranch (some elseBranch) =>
      if context.definition.isSome then
        -- Inside a recursive definition the branches know their condition,
        -- which a recursive call's termination may need.
        let test ← translate context condition
        let branch (fact : Lean.Expr) (body : ExprId) : MetaM Lean.Expr :=
          withLocalDeclD `fact fact fun hypothesis => do
            let term ← translate { context with facts := context.facts.push hypothesis } body
            let term ← if domainOf ty == .aggregate then
                runtimeAggregateValue context body term else pure term
            mkLambdaFVars #[hypothesis] term
        let thenTerm ← branch test thenBranch
        let elseTerm ← branch (mkNot test) elseBranch
        let resultType := (← inferType thenTerm).bindingBody!
        return mkAppN (mkConst ``dite [← getLevel resultType])
          #[resultType, test, mkApp (mkConst ``Classical.propDecidable) test, thenTerm, elseTerm]
      -- A conditional in a clause is Lean's `ite` on the decided test.
      let test ← translate context condition
      let thenTerm ← translate context thenBranch
      let elseTerm ← translate context elseBranch
      let thenTerm ← if domainOf ty == .aggregate then
          runtimeAggregateValue context thenBranch thenTerm else pure thenTerm
      let elseTerm ← if domainOf ty == .aggregate then
          runtimeAggregateValue context elseBranch elseTerm else pure elseTerm
      mkAppOptM ``ite #[none, test, none, thenTerm, elseTerm]
  | .letDecl pattern (some initializer) body =>
      -- A binding names its initializer's value in the body, bound as a
      -- specification function argument is.
      let some patternNode := context.ns.patterns[pattern.index]?
        | throwError "specification binding pattern {pattern.index} is out of range"
      match patternNode.kind with
      | .wildcard => translate context body
      | .variable localId =>
          let some patternType := context.typeOf? patternNode.typeId
            | throwError "specification binding pattern has an unknown type"
          unless localId.index < context.locals.size do
            throwError "specification local {localId.index} has no binder slot"
          let value ← translate context initializer
          let value ← match domainOf patternType with
            | .boolean => mkDecide value
            | _ => pure value
          translate { context with
            locals := context.locals.set! localId.index (some value)
            localTypes := if localId.index < context.localTypes.size
              then context.localTypes.set! localId.index patternType else context.localTypes
            oldLocals := context.oldLocals.setIfInBounds localId.index (some value) } body
      | _ =>
          throwError "generated contracts require a variable or wildcard binding pattern"
  | .block statements (some result) =>
      -- A block denotes its result after its statements: an assignment to a
      -- local rebinds it for the rest, every other statement must be
      -- without logical effect.
      let mut active := context
      for statement in statements do
        let some node := active.ns.expressions[statement.index]?
          | throwError "specification statement {statement.index} is out of range"
        match node.kind with
        | .assign place value =>
            let some (.localVar localId) := active.ns.places[place.index]?
              | throwError "a specification reading assigns only locals, not a place \
                  through a reference, field, or element"
            unless localId.index < active.locals.size do
              throwError "specification local {localId.index} has no binder slot"
            let some valueType := typeOfExpr? active value
              | throwError "an assigned value has an unknown type"
            let translated ← translate active value
            let translated ← match domainOf valueType with
              | .boolean => mkDecide translated
              | _ => pure translated
            active := { active with
              locals := active.locals.set! localId.index (some translated)
              oldLocals := active.oldLocals.setIfInBounds localId.index (some translated) }
        | kind =>
            unless withoutLogicalEffect statement do
              throwError "specification statement {repr kind} is not supported in \
                generated contracts"
      translate active result
  | kind =>
      throwError "specification node {repr kind} is not supported in generated contracts"
where
  /-- A statement a specification reading may drop: its value is discarded,
  so only a local mutation or a control transfer could reach the rest of the
  block. A failure imposes no condition. -/
  withoutLogicalEffect (id : ExprId) : Bool :=
    match context.ns.expressions[id.index]? with
    | none => false
    | some expression =>
        match expression.kind with
        | .value .. | .constant _ | .localVar _ | .quantifier .. | .spec _ => true
        | .operation (.write _) _ _ _ => false
        | .operation _ _ arguments _ => arguments.all withoutLogicalEffect
        | .throw_ _ arguments => arguments.all withoutLogicalEffect
        | .block statements result =>
            statements.all withoutLogicalEffect && result.all withoutLogicalEffect
        | .letDecl _ initializer body =>
            initializer.all withoutLogicalEffect && withoutLogicalEffect body
        | .ifElse condition thenBranch elseBranch =>
            withoutLogicalEffect condition && withoutLogicalEffect thenBranch &&
              elseBranch.all withoutLogicalEffect
        | .match_ scrutinee arms =>
            withoutLogicalEffect scrutinee && arms.all fun arm =>
              arm.guard.all withoutLogicalEffect && withoutLogicalEffect arm.body
        | .loop .. | .break_ .. | .continue_ _ | .return_ _ | .assign .. | .assignPattern .. =>
            false
  translateLiteral (literal : ConstValue) (ty : IrTy) : MetaM Lean.Expr := do
    match literal, ty with
    | .integer value, .integer _ _ => return toExpr value
    | .bool true, .bool => return mkConst ``True
    | .bool false, .bool => return mkConst ``False
    | .string value, .string => return toExpr value
    | .address value, .address => return toExpr value
    | .bytes value, .bytes => mkAppM ``RuntimeValue.bytes #[toExpr value]
    | _, _ =>
        throwError "specification literal {repr literal} at type {describeType ty} \
          is not supported in generated contracts"
  /-- Translate an operand and re-encode it as a runtime value, for the
  positions — a storage key, a published resource — where a clause hands a
  value back to the runtime vocabulary. -/
  runtimeOperand (id : ExprId) : MetaM Lean.Expr := do
    let some expression := context.ns.expressions[id.index]?
      | throwError "a specification operand is out of range"
    let some ty := context.typeOf? expression.typeId
      | throwError "a specification operand has an unknown type"
    let translated ← translate context id
    -- Constructors and total field projections already produce runtime
    -- aggregates. Native generic binders need encoding, but encoding an
    -- already translated constructor again is a representation mismatch.
    if (← inferType translated).isConstOf ``RuntimeValue then return translated
    if let some rep := context.valueRep? expression.typeId then
      match rep with
      | .parameter _ | .twin _ _ | .vector _ _ | .tuple _ =>
          return ← rep.encode context.codecs translated
      | _ => pure ()
    match domainOf ty with
    | .boolean => mkAppM ``RuntimeValue.bool #[← mkDecide translated]
    | domain => domain.encode translated
  /-- Translate an aggregate and erase it when it is currently represented
  by a generated twin. -/
  runtimeAggregate (id : ExprId) : MetaM Lean.Expr := do
    runtimeAggregateValue context id (← translate context id)
  /-- The resource family a global operation is instantiated at. -/
  resourceType (instantiations : Array LeanerIR.GenericArgument) : MetaM TypeId := do
    match instantiations.toList with
    | [.typeArg resource] => return resource.typeId
    | _ => throwError "a global specification operation needs one resource type"
  /-- The typed family a storage clause reads through.  A resource without a
  typed twin cannot be reasoned about, and saying so beats a contract whose
  storage reads are opaque. -/
  storableFamily (instantiations : Array LeanerIR.GenericArgument)
      (what : String) : MetaM SpecTypes.FamilyInfo := do
    let resource ← resourceType instantiations
    let some family := context.families.find? (·.typeIndex == resource.index)
      | throwError "the resource type of a storage {what} has no typed \
          specification twin, so its contents cannot be reasoned about in \
          generated contracts"
    return family
  /-- The state a storage read observes: `spec.old` reads the entry state,
  every other position the clause's own state. -/
  currentState : MetaM Lean.Expr := do
    let some state := context.state
      | throwError "a storage read has no state in this specification position"
    return state
  /-- The parameter domains and the result domain of a specification
  function, as its definition takes them: no type argument is resolved, so
  a type parameter is the runtime value domain. -/
  definitionDomains (reference : LeanerIR.QualifiedRef) : MetaM (Array Domain × Domain) := do
    let some (_, declaration) := specFunctionOf? context.unit reference
      | throwError "specification function `{repr reference}` does not resolve"
    let parameters ← declaration.signature.parameters.mapM fun parameter => do
      let some parameterType := context.unit.tables.types[parameter.typeUse.typeId.index]?
        | throwError "a specification function parameter has an unknown type"
      pure (domainOf parameterType)
    let result ← match declaration.signature.results.toList with
      | [result] =>
          let some resultType := context.unit.tables.types[result.typeId.index]?
            | throwError "a specification function result has an unknown type"
          pure (domainOf resultType)
      | _ => throwError "a recursive specification function returns one value"
    pure (parameters, result)
  /-- The Lean type of a domain as a definition's parameter or result. -/
  definitionType (domain : Domain) : Lean.Expr :=
    match domain with
    | .boolean => mkSort .zero
    | _ => domain.leanType
  /-- The arguments of a call as the bundle a recursive definition takes:
  right-nested pairs closed by `()`. -/
  bundleArguments (reference : LeanerIR.QualifiedRef) (arguments : Array ExprId) :
      MetaM Lean.Expr := do
    let (domains, _) ← definitionDomains reference
    unless arguments.size == domains.size do
      throwError "specification function call argument count differs from its declaration"
    let mut values : Array Lean.Expr := #[]
    for (argument, domain) in arguments.zip domains do
      let value ← match domain with
        | .boolean => mkDecide (← translate context argument)
        | .aggregate => runtimeOperand argument
        | .integer | .text _ => translate context argument
      values := values.push value
    values.foldrM (fun value rest => mkAppM ``Prod.mk #[value, rest]) (mkConst ``Unit.unit)
  /-- A recursive definition's result at a call, in the call's domain. -/
  callResult (reference : LeanerIR.QualifiedRef) (ty : IrTy) (value : Lean.Expr) :
      MetaM Lean.Expr := do
    let (_, result) ← definitionDomains reference
    match result, domainOf ty with
    | .aggregate, domain => domain.ofRuntime value
    | .boolean, .boolean => pure value
    | _, _ => pure value
  /-- Define a recursive specification function once: over its bundled
  arguments by well-founded recursion on its `decreases` measure, with
  the unfolding theorem `f.unfold`. -/
  ensureDefinition (reference : LeanerIR.QualifiedRef) : MetaM Name := do
    let name ← specDefinitionName context.unit reference
    if (← getEnv).contains name then return name
    let some (targetNs, declaration) := specFunctionOf? context.unit reference
      | throwError "specification function `{repr reference}` does not resolve"
    let some root := declaration.body
      | throwError "a recursive specification function must have a body"
    let some measureSource := declaration.contract.conditions.find? (·.kind == .decreases)
      | throwError m!"the recursive specification function \
          `{specFunctionSpelling context.unit reference}` needs a `decreases` measure"
    let (domains, resultDomain) ← definitionDomains reference
    let bundleType ← domains.foldrM (fun domain rest => mkAppM ``Prod #[definitionType domain, rest])
      (mkConst ``Unit)
    let resultType := definitionType resultDomain
    let mut localTypes : Array IrTy := #[]
    for localDecl in declaration.locals do
      let some localType := context.unit.tables.types[localDecl.type.typeId.index]?
        | throwError "a specification function local has an unknown type"
      localTypes := localTypes.push localType
    -- The parameters are the projections of the bundle.
    let bodyContext (bundle : Lean.Expr) (definition : Option RecursiveDefinition) :
        MetaM Context := do
      let mut locals : Array (Option Lean.Expr) := Array.replicate declaration.locals.size none
      let mut rest := bundle
      for index in [:domains.size] do
        locals := locals.set! index (some (← mkAppM ``Prod.fst #[rest]))
        rest ← mkAppM ``Prod.snd #[rest]
      pure { context with
        namespaceId := reference.namespaceId, ns := targetNs
        locals, localTypes, localNames := declaration.locals.map (·.name), oldLocals := locals
        results := #[], resultTypes := #[]
        specCallStack := #[], typeArguments := #[]
        definition, facts := #[] }
    let measure ← withLocalDeclD `bundle bundleType fun bundle => do
      let value ← translate (← bodyContext bundle none) measureSource.expression
      mkLambdaFVars #[bundle] (← mkAppM ``Int.toNat #[value])
    let step ← withLocalDeclD `bundle bundleType fun bundle => do
      let recurseType ← withLocalDeclD `smaller bundleType fun smaller => do
        let decreasing ← mkAppM ``LT.lt #[mkApp measure smaller, mkApp measure bundle]
        mkForallFVars #[smaller] (← mkArrow decreasing resultType)
      withLocalDeclD `recurse recurseType fun recurse => do
        let body ← translate (← bodyContext bundle
          (some { reference, argument := bundle, recurse, measure })) root
        let body ← match resultDomain with
          | .aggregate => runtimeAggregateValue (← bodyContext bundle none) root body
          | _ => pure body
        mkLambdaFVars #[bundle, recurse] body
    let relation ← mkAppM ``measure #[measure]
    let wellFounded ← mkAppOptM ``WellFoundedRelation.wf #[bundleType, relation]
    let motive ← withLocalDeclD `bundle bundleType fun bundle => mkLambdaFVars #[bundle] resultType
    let value ← mkAppOptM ``WellFounded.fix #[bundleType, motive,
      ← mkAppOptM ``WellFoundedRelation.rel #[bundleType, relation], wellFounded, step]
    let type ← mkArrow bundleType resultType
    addDecl (.defnDecl { name, levelParams := [], type, value, hints := .opaque, safety := .safe })
    setIrreducibleAttribute name
    -- `f a = F a (fun b _ => f b)`: the definition unfolds once.
    let unfoldType ← withLocalDeclD `bundle bundleType fun bundle => do
      let recursion ← withLocalDeclD `smaller bundleType fun smaller => do
        let decreasing ← mkAppM ``LT.lt #[mkApp measure smaller, mkApp measure bundle]
        withLocalDeclD `less decreasing fun less =>
          mkLambdaFVars #[smaller, less] (mkApp (mkConst name) smaller)
      mkForallFVars #[bundle]
        (← mkEq (mkApp (mkConst name) bundle) (← Core.betaReduce (mkAppN step #[bundle, recursion])))
    let unfoldValue ← withLocalDeclD `bundle bundleType fun bundle => do
      let equation ← mkAppOptM ``WellFounded.fix_eq
        #[bundleType, motive, ← mkAppOptM ``WellFoundedRelation.rel #[bundleType, relation], wellFounded, step,
          bundle]
      mkLambdaFVars #[bundle] equation
    let unfoldName := Name.str name "unfold"
    addDecl (.thmDecl { name := unfoldName, levelParams := [], type := unfoldType, value := unfoldValue })
    return name
  translateOperation (operation : Operation)
      (instantiations : Array LeanerIR.GenericArgument) (arguments : Array ExprId)
      (ty : IrTy) : MetaM Lean.Expr := do
    match operation with
    | .specification (.functionCall reference _) =>
        if let some definition := context.definition then
          if definition.reference == reference then
            -- A recursive call: the bundled arguments, at a smaller measure.
            let bundle ← bundleArguments reference arguments
            let goal ← mkAppM ``LT.lt
              #[mkApp definition.measure bundle, mkApp definition.measure definition.argument]
            -- The measure at the bundled arguments, projected out: `omega`
            -- reads the arithmetic, not the pairing.
            let (goal, _) ← dsimp goal (← Simp.mkContext)
            let proof ← mkFreshExprMVar goal
            try
              -- As the tactic does: refute the negated goal from the local
              -- hypotheses, which include the path conditions.
              if let some refutation ← proof.mvarId!.falseOrByContra then
                let hypotheses ← refutation.withContext getLocalHyps
                Lean.Elab.Tactic.Omega.omega hypotheses.toList refutation
            catch failure =>
              let facts ← context.facts.mapM fun fact => do pure m!"{← inferType fact}"
              throwError m!"the recursive call of `{specFunctionSpelling context.unit reference}` \
                does not decrease its measure under the conditions on its path: \
                {failure.toMessageData}\ngoal: {goal}\nconditions: {facts.toList}"
            let value := mkApp2 definition.recurse bundle (← instantiateMVars proof)
            return ← callResult reference ty value
        if specRecursive context.unit reference then
          if let some definition := context.definition then
            if specReaches context.unit reference definition.reference then
              throwError m!"the specification functions \
                `{specFunctionSpelling context.unit definition.reference}` and \
                `{specFunctionSpelling context.unit reference}` are mutually recursive, which is \
                not carried"
          -- A recursive function is a Lean definition, not an expansion.
          let name ← ensureDefinition reference
          let bundle ← bundleArguments reference arguments
          return ← callResult reference ty (mkApp (mkConst name) bundle)
        if context.specCallStack.contains reference then
          throwError m!"the recursive specification function \
            `{specFunctionSpelling context.unit reference}` needs a `decreases` measure"
        -- A generic function is expanded at its arguments' types.
        let typeArguments ← instantiations.mapM fun instantiation => do
          let .typeArg argument := instantiation
            | throwError "a specification function call takes only type arguments in \
                generated contracts"
          let some argumentType := context.typeOf? argument.typeId
            | throwError "a specification function type argument has an unknown type"
          pure (argumentType, context.valueRep? argument.typeId)
        let some qualified := context.unit.tables.names[reference.name.index]?
          | throwError "specification function call has an unknown name"
        unless qualified.namespaceId == reference.namespaceId do
          throwError "specification function call name belongs to a different namespace"
        let some targetNs := context.unit.namespaces[reference.namespaceId.index]?
          | throwError "specification function call namespace is out of range"
        let some functionId := context.unit.resolution.specFunction? reference.name
          | if (context.unit.resolution.function? reference.name).isSome then
              throwError "`{qualified.name}` has no specification version: it has no \
                body (a native); `spec fun {qualified.name}` can declare one"
            else throwError "specification function call target does not resolve"
        let some declaration := targetNs.specFunctions[functionId.index]?
          | throwError "specification function declaration is out of range"
        unless arguments.size == declaration.signature.parameters.size do
          throwError "specification function call argument count differs from its declaration"
        let some body := declaration.body
          | do
            let some path := context.unit.tables.namespaces[reference.namespaceId.index]?
              | throwError "specification function namespace has no path"
            let instantiation := typeArguments.map fun (argumentType, rep) =>
              match rep with
              | some rep => toString (repr rep)
              | none => toString (repr argumentType)
            let name := "::".intercalate (path.segments.toList ++ [qualified.name]) ++
              (if instantiation.isEmpty then "" else s!"<{", ".intercalate instantiation.toList}>")
            let encoded ← arguments.mapM runtimeOperand
            let domain := domainOf ty
            let value ← mkAppOptM ``LeanerLang.Contract.opaqueSpec
              #[toExpr name, domain.leanType, none,
                ← mkListLit (mkConst ``RuntimeValue) encoded.toList]
            domain.ofBinder value
        unless declaration.signature.results.size == 1 do
          throwError "generated contracts currently expand specification functions with one result"
        let callee : Context := {
          context with
          namespaceId := reference.namespaceId
          ns := targetNs
          results := #[]
          resultTypes := #[]
          specCallStack := context.specCallStack.push reference
          typeArguments }
        let mut targetLocals : Array (Option Lean.Expr) :=
          Array.replicate declaration.locals.size none
        let mut targetTypes : Array IrTy := #[]
        for localDecl in declaration.locals do
          let some localType := callee.typeOf? localDecl.type.typeId
            | throwError "specification function local has an unknown type"
          targetTypes := targetTypes.push localType
        for (argument, index) in arguments.zipIdx do
          let some parameter := declaration.signature.parameters[index]?
            | throwError "specification function parameter is out of range"
          let some parameterType := callee.typeOf? parameter.typeUse.typeId
            | throwError "specification function parameter has an unknown type"
          let value ← translate context argument
          let value ← match domainOf parameterType with
            | .boolean => mkDecide value
            | _ => pure value
          targetLocals := targetLocals.set! index (some value)
        translate { callee with
          locals := targetLocals, localTypes := targetTypes, oldLocals := targetLocals
          localNames := declaration.locals.map (·.name) } body
    | .specification (.result index) =>
        match context.ns.profile, context.resultTypes[0]?, context.results[0]? with
        | some .move, some (.tuple _), some packed =>
            -- Move presents multiple returns as one tuple-typed physical
            -- result. `spec.result[i]` nevertheless denotes component `i`.
            -- Projecting from the represented runtime tuple preserves that
            -- source-level view without inventing extra result-row slots.
            let selected ← mkAppM ``LeanerIR.RuntimeValue.field
              #[packed, toExpr index]
            (domainOf ty).ofRuntime selected
        | _, _, _ =>
            let some binder := context.results[index]?
              | throwError "specification result {index} has no binder"
            let some logicalType := context.resultTypes[index]?
              | throwError "specification result {index} has no logical domain"
            (domainOf logicalType).ofBinder binder
    | .specification .final =>
        let some argument := arguments[0]?
          | throwError "`final` expects one argument"
        let index ← match context.ns.expressions[argument.index]?.map (·.kind) with
          | some (LeanerIR.ExprKind.operation (.specification (.result index)) _ _ _) =>
              pure index
          | _ => throwError "`final` reads a returned mutable reference (`result`)"
        let some (some final) := context.resultFinals[index]?
          | throwError "`final` has no final value for result {index} in this clause"
        match context.ns.profile, context.resultTypes[0]? with
        | some .move, some (.tuple _) => (domainOf ty).ofRuntime final
        | _, _ =>
            let some logicalType := context.resultTypes[index]?
              | throwError "specification result {index} has no logical domain"
            (domainOf logicalType).ofBinder final
    | .specification .old =>
        let some argument := arguments[0]?
          | throwError "spec.old expects one argument"
        translate { context with
          locals := context.oldLocals, state := context.oldState } argument
    | .specification .bitVectorToInt | .specification .intToBitVector =>
        -- The logical domain already represents every integer unbounded, so
        -- reading a fixed-width value at `Int` is the identity.
        let some argument := arguments[0]?
          | throwError "spec.bitVectorToInt expects one argument"
        translate context argument
    | .specification .indexVector =>
        let some vector := arguments[0]?
          | throwError "spec.indexVector expects a vector operand"
        let some index := arguments[1]?
          | throwError "spec.indexVector expects an index operand"
        let selected ← mkAppM ``LeanerIR.RuntimeValue.field
          #[← runtimeAggregate vector,
            ← mkAppM ``Int.toNat #[← translate context index]]
        (domainOf ty).ofRuntime selected
    | .specification .lengthVector =>
        let some vector := arguments[0]?
          | throwError "spec.lengthVector expects a vector operand"
        mkAppM ``LeanerLang.Contract.lengthVector #[← runtimeAggregate vector]
    | .specification .containsVector =>
        let some vector := arguments[0]?
          | throwError "spec.containsVector expects a vector operand"
        let some element := arguments[1]?
          | throwError "spec.containsVector expects an element operand"
        mkAppM ``LeanerLang.Contract.containsVector
          #[← runtimeAggregate vector, ← runtimeOperand element]
    | .specification .updateVector =>
        let some vector := arguments[0]?
          | throwError "spec.updateVector expects a vector operand"
        let some index := arguments[1]?
          | throwError "spec.updateVector expects an index operand"
        let some replacement := arguments[2]?
          | throwError "spec.updateVector expects a replacement operand"
        mkAppM ``LeanerLang.Contract.updateVector
          #[← runtimeAggregate vector,
            ← mkAppM ``Int.toNat #[← translate context index],
            ← runtimeOperand replacement]
    | .specification (.global _) =>
        let some key := arguments[0]?
          | throwError "a storage read expects one key"
        let family ← storableFamily instantiations "read"
        let globals ← mkAppM ``LeanerIR.RuntimeState.globals #[← currentState]
        let codecArguments ← family.outerCodecs context.codecs
        familyAccessor context.typeInstantiation family `get
          (codecArguments ++ #[globals, ← runtimeOperand key])
    | .global .contains =>
        let some key := arguments[0]?
          | throwError "a storage existence test expects one key"
        let family ← storableFamily instantiations "existence test"
        let globals ← mkAppM ``LeanerIR.RuntimeState.globals #[← currentState]
        let test ← familyAccessor context.typeInstantiation family `contains
          #[globals, ← runtimeOperand key]
        mkEq test (mkConst ``Bool.true)
    | .data (.select reference field) =>
        let some base := arguments[0]?
          | throwError "a field selection expects one operand"
        match twinOfExpr? context base with
        | some applied =>
            let some (fieldName, declarationRep) :=
                applied.info.fields.find? (·.1 == field)
              | throwError "field `{field}` does not exist on the typed \
                  twin of `{applied.info.name}`"
            let rep := declarationRep.instantiate applied.arguments
            let projection ← applied.arguments.foldlM
              (init := mkConst (applied.info.twin ++ Name.mkSimple fieldName))
              fun projection argument =>
                return mkApp projection (← argument.leanType context.carrier)
            let projected := mkApp projection (← translate context base)
            match rep with
            | .int _ _ => mkAppM ``LeanerIR.SpecInt.val #[projected]
            | .bool => mkEq projected (mkConst ``Bool.true)
            | _ => return projected
        | none =>
            let some index := LeanerIR.SemanticOperations.referencedFieldIndex?
                context.unit context.namespaceId reference none field
              | throwError "specification field `{field}` does not resolve in \
                  generated contracts"
            let selected ← mkAppM ``LeanerIR.RuntimeValue.field
              #[← runtimeAggregate base, toExpr index]
            (domainOf ty).ofRuntime selected
    | .data (.selectVariants reference fields) =>
        let some base := arguments[0]?
          | throwError "a variant field selection expects one operand"
        let some info := twinOfReference? context reference
          | throwError "the owner of a variant field selection has no typed twin"
        let handle ← mkAppM ``LeanerIR.StructHandle.mk
          #[← mkAppM ``LeanerIR.NamespaceId.mk #[toExpr info.namespaceIndex],
            toExpr info.structIndex]
        let mut choices : Array Lean.Expr := #[]
        for variant in info.variants do
          if let some index := variant.fields.findIdx? fun candidate =>
              fields.contains candidate.1 then
            choices := choices.push
              (← mkAppM ``Prod.mk #[toExpr variant.name, toExpr index])
        let pairType ← mkAppM ``Prod #[mkConst ``String, mkConst ``Nat]
        let choicesLiteral ← mkArrayLit pairType choices.toList
        let selected ← mkAppM ``LeanerLang.Contract.selectVariantField
          #[← runtimeAggregate base, handle, choicesLiteral]
        (domainOf ty).ofRuntime selected
    | .data (.testVariants reference variants) =>
        let some base := arguments[0]?
          | throwError "a variant test expects one operand"
        let some info := twinOfReference? context reference
          | throwError "the owner of a variant test has no typed twin"
        let handle ← mkAppM ``LeanerIR.StructHandle.mk
          #[← mkAppM ``LeanerIR.NamespaceId.mk #[toExpr info.namespaceIndex],
            toExpr info.structIndex]
        let variants ← mkArrayLit (mkConst ``String)
          (variants.toList.map toExpr)
        let test ← mkAppM ``LeanerLang.Contract.testVariants
          #[← runtimeAggregate base, handle, variants]
        mkEq test (mkConst ``Bool.true)
    | .call (.constructor reference variant) =>
        let some info := twinOfReference? context reference
          | throwError "a specification constructor has no typed twin"
        let handle ← mkAppM ``LeanerIR.StructHandle.mk
          #[← mkAppM ``LeanerIR.NamespaceId.mk #[toExpr info.namespaceIndex],
            toExpr info.structIndex]
        let variant ← match variant with
          | some name => mkAppM ``Option.some #[toExpr name]
          | none => pure (mkApp (mkConst ``Option.none [Lean.Level.zero])
              (mkConst ``String))
        let operands ← arguments.mapM runtimeOperand
        let operands ← mkArrayLit (mkConst ``RuntimeValue) operands.toList
        mkAppM ``RuntimeValue.nominal #[handle, variant, operands]
    | .reference .dereference =>
        -- Specifications see through references: the accessors read a
        -- borrow's content, so a dereference is the operand itself.
        let some argument := arguments[0]?
          | throwError "a dereference expects one operand"
        translate context argument
    | .primitive primitive => translatePrimitive primitive arguments ty
    | _ =>
        throwError "specification operation {repr operation} is not supported \
          in generated contracts"
  binary (arguments : Array ExprId) : MetaM (Lean.Expr × Lean.Expr) := do
    let some left := arguments[0]? | throwError "a binary specification operation needs two operands"
    let some right := arguments[1]? | throwError "a binary specification operation needs two operands"
    return (← translate context left, ← translate context right)
  unary (arguments : Array ExprId) : MetaM Lean.Expr := do
    let some only := arguments[0]? | throwError "a unary specification operation needs one operand"
    translate context only
  translatePrimitive (primitive : PrimitiveOperation) (arguments : Array ExprId)
      (ty : IrTy) : MetaM Lean.Expr := do
    match primitive with
    -- A signer's carrier is the address it holds.
    | .signerAddress => unary arguments
    | .vector =>
        let encoded ← arguments.mapM runtimeOperand
        let literal ← mkArrayLit (mkConst ``LeanerIR.RuntimeValue) encoded.toList
        mkAppM ``LeanerIR.RuntimeValue.vector #[literal]
    | .pushVector =>
        let some vector := arguments[0]?
          | throwError "pushVector expects a vector operand"
        let some element := arguments[1]?
          | throwError "pushVector expects an element operand"
        mkAppM ``LeanerLang.Contract.pushVector
          #[← runtimeAggregate vector, ← runtimeOperand element]
    | .concatVector =>
        let some left := arguments[0]?
          | throwError "concatVector expects two vector operands"
        let some right := arguments[1]?
          | throwError "concatVector expects two vector operands"
        mkAppM ``LeanerLang.Contract.concatVector
          #[← runtimeAggregate left, ← runtimeAggregate right]
    | .compare =>
        let (left, right) ← match arguments.toList with
          | [left, right] => pure (left, right)
          | _ => throwError "compare expects two operands"
        let rank := mkApp (mkConst ``LeanerIR.SemanticOperations.variantRank)
          (toExpr context.ns.variantOrders)
        let ordered := mkAppN (mkConst ``LeanerIR.RuntimeValue.order)
          #[rank, ← runtimeOperand left, ← runtimeOperand right]
        mkAppM ``LeanerIR.orderValue #[ordered]
    | .containsVector =>
        let some vector := arguments[0]?
          | throwError "containsVector expects a vector operand"
        let some element := arguments[1]?
          | throwError "containsVector expects an element operand"
        mkAppM ``LeanerLang.Contract.containsVector
          #[← runtimeAggregate vector, ← runtimeOperand element]
    | .add => let (l, r) ← binary arguments; mkAppM ``HAdd.hAdd #[l, r]
    | .subtract => let (l, r) ← binary arguments; mkAppM ``HSub.hSub #[l, r]
    | .multiply => let (l, r) ← binary arguments; mkAppM ``HMul.hMul #[l, r]
    | .divide => let (l, r) ← binary arguments; mkAppM ``Int.tdiv #[l, r]
    | .modulo => let (l, r) ← binary arguments; mkAppM ``Int.tmod #[l, r]
    | .negate => let value ← unary arguments; mkAppM ``Neg.neg #[value]
    | .bitwiseAnd =>
        let (l, r) ← binary arguments
        if ty == LeanerIR.Ty.bool then mkAppM ``And #[l, r]
        else mkAppM ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd #[l, r]
    | .bitwiseOr =>
        let (l, r) ← binary arguments
        if ty == LeanerIR.Ty.bool then mkAppM ``Or #[l, r]
        else mkAppM ``LeanerIR.Proofs.IntegerArithmetic.bitwiseOr #[l, r]
    | .bitwiseXor =>
        let (l, r) ← binary arguments
        if ty == LeanerIR.Ty.bool then mkAppM ``Not #[← mkAppM ``Iff #[l, r]]
        else mkAppM ``LeanerIR.Proofs.IntegerArithmetic.bitwiseXor #[l, r]
    | .bitwiseNot => let value ← unary arguments; mkAppM ``Int.not #[value]
    | .shiftLeft =>
        let (l, r) ← binary arguments
        mkAppM ``Int.shiftLeft #[l, ← mkAppM ``Int.toNat #[r]]
    | .shiftRight =>
        let (l, r) ← binary arguments
        mkAppM ``Int.shiftRight #[l, ← mkAppM ``Int.toNat #[r]]
    | .less => let (l, r) ← binary arguments; mkAppM ``LT.lt #[l, r]
    | .greater => let (l, r) ← binary arguments; mkAppM ``LT.lt #[r, l]
    | .lessEqual => let (l, r) ← binary arguments; mkAppM ``LE.le #[l, r]
    | .greaterEqual => let (l, r) ← binary arguments; mkAppM ``LE.le #[r, l]
    | .equal =>
        let some left := arguments[0]?
          | throwError "an equality needs two operands"
        let some right := arguments[1]?
          | throwError "an equality needs two operands"
        let l ← runtimeAggregate left
        let r ← runtimeAggregate right
        let some operandTy := arguments[0]?.bind (typeOfExpr? context)
          | throwError "an equality operand has an unknown type"
        if operandTy == LeanerIR.Ty.bool then mkAppM ``Iff #[l, r] else mkAppM ``Eq #[l, r]
    | .notEqual =>
        let some left := arguments[0]?
          | throwError "an inequality needs two operands"
        let some right := arguments[1]?
          | throwError "an inequality needs two operands"
        let l ← runtimeAggregate left
        let r ← runtimeAggregate right
        let some operandTy := arguments[0]?.bind (typeOfExpr? context)
          | throwError "an inequality operand has an unknown type"
        if operandTy == LeanerIR.Ty.bool then
          mkAppM ``Not #[← mkAppM ``Iff #[l, r]]
        else mkAppM ``Ne #[l, r]
    | .logicalAnd => let (l, r) ← binary arguments; mkAppM ``And #[l, r]
    | .logicalOr => let (l, r) ← binary arguments; mkAppM ``Or #[l, r]
    | .logicalNot => let value ← unary arguments; mkAppM ``Not #[value]
    | .implies => let (l, r) ← binary arguments; return ← mkArrow l r
    | .equivalent => let (l, r) ← binary arguments; mkAppM ``Iff #[l, r]
    | _ =>
        throwError "specification primitive {repr primitive} at type \
          {describeType ty} is not supported in generated contracts"

/-- How one parameter reaches the function: by value, by shared borrow,
or by mutable borrow. -/
private inductive SlotKind where
  | plain
  | sharedRef
  | mutableRef
  deriving BEq

/-- Logical binder type and physical encoding of one parameter or result.
For a reference slot, `physical` is the referent type: specifications see
through references, so the logical binders live at the borrowed type. -/
private structure Slot where
  name : Name
  kind : SlotKind := .plain
  physical : IrTy
  leanType : Lean.Expr
  /-- Native generic representation, when this slot depends on a function
  type parameter. -/
  rep : Option Typed.ValueRep := none
  codec : Option Lean.Expr := none
  /-- A result that is a tuple of mutable references to scalars is one
  runtime value whose components each carry a loan: the components are
  slots of their own, so a summary can name each returned loan. -/
  components : Array Slot := #[]

/-- The mutable-reference-to-scalar components of a tuple type, when every
element is one. -/
private def tupleReferenceComponents? (context : Context) (name : Name) (declared : IrTy) :
    Option (Array Slot) := do
  let .tuple elements := declared | none
  guard !elements.isEmpty
  elements.mapIdxM fun index element => do
    let .reference reference ← context.unit.tables.types[element.index]? | none
    guard (reference.kind == .mutable)
    let referent ← context.unit.tables.types[reference.referent.index]?
    match referent with
    | .integer _ _ | .bool => pure ()
    | _ => none
    pure { name := name.appendAfter s!"_{index}", kind := .mutableRef, physical := referent
           leanType := (domainOf referent).leanType }

private def slotOf (context : Context) (name : Name) (typeId : TypeId)
    (allowReference : Bool := false) : MetaM Slot := do
  let some declared := context.unit.tables.types[typeId.index]?
    | throwError "slot type {typeId.index} is out of range"
  let components := (tupleReferenceComponents? context name declared).getD #[]
  let (kind, physical, physicalId) ← match declared with
    | .reference reference => do
        unless allowReference do
          throwError "a result of reference type is not supported in           generated contracts"
        let some referent := context.unit.tables.types[reference.referent.index]?
          | throwError "reference referent type {reference.referent.index} is           out of range"
        let kind := match reference.kind with
          | .shared => SlotKind.sharedRef
          | .mutable => SlotKind.mutableRef
        pure (kind, referent, reference.referent)
    | declared => pure (SlotKind.plain, declared, typeId)
  let rep := (Typed.valueRep? context.unit context.twins physicalId context.ns.profile).filter
    (·.mentionsParameter)
  let leanType ← match rep with
    | some rep => rep.leanType context.carrier
    | none => pure (domainOf physical).leanType
  let codec ← rep.mapM (·.codec context.codecs)
  return { name, kind, physical, leanType, rep, codec, components }

private def conjunction (parts : Array Lean.Expr) : MetaM Lean.Expr := do
  match parts.toList with
  | [] => return mkConst ``True
  | head :: tail =>
      tail.foldlM (fun accumulated part => mkAppM ``And #[accumulated, part]) head

private def disjunction (parts : Array Lean.Expr) : MetaM Lean.Expr := do
  match parts.toList with
  | [] => return mkConst ``False
  | head :: tail =>
      tail.foldlM (fun accumulated part => mkAppM ``Or #[accumulated, part]) head

/-- The clause binders of one slot: its value, and for a mutable reference
its exit value, the prophecy.  The bare parameter name denotes the exit in
`ensures`, and `spec.old` reaches the entry. -/
private structure SlotBinders where
  entry : Lean.Expr
  exit : Option Lean.Expr := none

/-- The binder a specification local denotes in the current state. -/
private def SlotBinders.current (binders : SlotBinders) : Lean.Expr :=
  binders.exit.getD binders.entry

/-- Existentially close a body over the logical slots. -/
private def existsOver (binders : Array Lean.Expr) (body : Lean.Expr) :
    MetaM Lean.Expr := do
  binders.foldrM (fun binder accumulated => do
    mkAppM ``Exists #[← mkLambdaFVars #[binder] accumulated]) body

/-! ## Native loop invariants -/

/-- Find source-normalized `spec invariant; loop` pairs.  The expression id
identifies the generated predicate over the loop's typed header product. -/
partial def loopSpecifications (ns : ValidatedNamespace)
    (root : ExprId) (seen : Array Nat := #[]) :
    Array (ExprId × LeanerIR.SpecBlock) := Id.run do
  if seen.contains root.index then return #[]
  let some expression := ns.expressions[root.index]? | return #[]
  let seen := seen.push root.index
  let mut found := #[]
  -- A while annotation is carried by its condition, not a preceding
  -- statement. Discover that representation without rewriting the loop.
  if let .loop _ body := expression.kind then
    if let some { kind := .ifElse condition _ _, .. } := ns.expressions[body.index]? then
      if let some { kind := .block statements _, .. } := ns.expressions[condition.index]? then
        for statement in statements do
          if let some { kind := .spec block, .. } := ns.expressions[statement.index]? then
            if block.conditions.any (·.kind == .loopInvariant) then
              found := found.push (root, block)
  if let .block statements _ := expression.kind then
    for index in [0:statements.size] do
      if h : index + 1 < statements.size then
        let specId := statements[index]
        let loopId := statements[index + 1]
        if let some { kind := .spec block, .. } := ns.expressions[specId.index]? then
          if block.conditions.any (fun condition => condition.kind == .loopInvariant) then
            if let some { kind := .loop .., .. } := ns.expressions[loopId.index]? then
              found := found.push (loopId, block)
  for child in LeanerIR.Validation.expressionChildren expression.kind do
    found := found ++ loopSpecifications ns child seen
  -- Unannotated loops still need a typed header invariant. Authored clauses
  -- strengthen that invariant, but are not required for type/range facts.
  if let .loop .. := expression.kind then
    unless found.any (fun (site, _) => site == root) do
      found := found.push (root, { loc := expression.loc })
  -- An annotation preceding a loop and the recursive visit name the same
  -- site. Keep the authored entry, discovered before its default.
  found := found.foldl (fun acc item =>
    if acc.any (fun previous => previous.1 == item.1) then acc else acc.push item) #[]
  return found

/-- The conjunction of a loop annotation's `invariant` clauses over the
given local binders: the clause translation the contracts use, with no
runtime row, frame, or route-specific representation. -/
def translateLoopInvariants (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (block : LeanerIR.SpecBlock)
    (locals : Array (Option Lean.Expr)) (localTypes : Array IrTy) (codecs : Option Lean.Expr)
    (localNames : Array String) (oldLocals : Array (Option Lean.Expr))
    (oldState : Lean.Expr) (twins : Array SpecTypes.TwinInfo)
    (families : Array SpecTypes.FamilyInfo) : MetaM Lean.Expr := do
  let context : Context := {
    unit, namespaceId, ns, locals, oldLocals, localTypes, localNames, results := #[],
    codecs, oldState := some oldState, twins, families }
  let clauses ← block.conditions.filterMapM fun condition => do
    if condition.kind == .loopInvariant then some <$> translate context condition.expression
    else pure none
  conjunction clauses

/-- Locals lexically available at a loop header. Body-local slots exist in
the runtime row but are not initialized on the first visit. -/
partial def loopHeaderLocals (ns : ValidatedNamespace) (root target : ExprId)
    (available : Array LocalId) : Option (Array LocalId) := do
  if root == target then return available
  let expression ← ns.expressions[root.index]?
  if let .letDecl pattern value body := expression.kind then
    if let some value := value then
      if let some found := loopHeaderLocals ns value target available then return found
    let rec boundLocals (pattern : LeanerIR.PatternId) : Array LocalId :=
      match ns.patterns[pattern.index]? with
      | some { kind := .variable localId, .. } => #[localId]
      | some { kind := .tuple children, .. }
      | some { kind := .constructor _ _ _ children, .. } => children.flatMap boundLocals
      | _ => #[]
    loopHeaderLocals ns body target (available ++ boundLocals pattern)
  else
    (LeanerIR.Validation.expressionChildren expression.kind).findSome?
      fun child => loopHeaderLocals ns child target available

/-- Every erased in-body specification must be accounted for. Until inline
assertions/assumptions have native obligations, only linked loop annotations
may accompany a generated loop computation. -/
private partial def checkLoopAnnotations (ns : ValidatedNamespace) (root : ExprId)
    (specifications : Array (ExprId × LeanerIR.SpecBlock)) : MetaM Unit := do
  let some expression := ns.expressions[root.index]? | return
  if let .spec block := expression.kind then
    unless block.frame.isNone && block.pragmas.isEmpty &&
        block.conditions.all (·.kind == .loopInvariant) do
      throwError "native loops do not yet translate inline specification obligations"
    unless block.conditions.isEmpty || specifications.any (·.2 == block) do
      throwError "native loop invariant has no consuming loop"
  for child in LeanerIR.Validation.expressionChildren expression.kind do
    checkLoopAnnotations ns child specifications

private structure AbortClause where
  condition : ExprId
  code : Option ExprId := none
  range : Nat × Nat := (0, 0)

private structure ClauseGroups where
  requires : Array ExprId := #[]
  ensures : Array (ExprId × Nat × Nat) := #[]
  abortsIf : Array AbortClause := #[]

/-- The authored byte range of a condition, when the unit records one. -/
private def conditionRange (unit : ValidatedUnit) (condition : LeanerIR.Condition) :
    Nat × Nat :=
  match unit.tables.locations[condition.loc.index]? with
  | some location =>
      match location.primary with
      | some range => (range.startByte, range.endByte)
      | none => (0, 0)
  | none => (0, 0)

/-- One total resource read in a module invariant.  `old` selects whether its
implicit existence guard observes the entry or current state. -/
private structure InvariantValueRead where
  typeIndex : Nat
  key : ExprId
  atEntry : Bool := false
  deriving BEq

private structure ModifiedResource where
  typeIndex : Nat
  key : ExprId

/-- Resource reads whose totalized values occur in an invariant.  Existence
tests are deliberately absent: unlike a value read, they do not need an
implicit guard. -/
private partial def invariantValueReads (ns : ValidatedNamespace) (root : ExprId)
    (inOld : Bool := false) : Array InvariantValueRead :=
  match ns.expressions[root.index]? with
  | none => #[]
  | some expression =>
      match expression.kind with
      | .operation (.specification .old) _ arguments _ =>
          arguments.flatMap fun child => invariantValueReads ns child true
      | .operation (.specification (.global _)) instantiations arguments _ =>
          let read := match instantiations.toList, arguments[0]? with
            | [.typeArg resource], some key =>
                #[{ typeIndex := resource.typeId.index, key, atEntry := inOld }]
            | _, _ => #[]
          read ++ arguments.flatMap fun child => invariantValueReads ns child inOld
      | kind =>
          (LeanerIR.Validation.expressionChildren kind).flatMap fun child =>
            invariantValueReads ns child inOld

/-- The implicit existence premise for one totalized invariant read. -/
private def invariantReadGuard (context : Context) (read : InvariantValueRead) :
    MetaM Lean.Expr := do
  let some family := context.families.find? (fun family =>
      family.typeIndex == read.typeIndex)
    | throwError "a resource read in a namespace invariant has no typed family"
  let readContext := if read.atEntry then
      { context with locals := context.oldLocals, state := context.oldState }
    else context
  let some state := readContext.state
    | throwError "a namespace invariant resource read has no state"
  let some keyTy := typeOfExpr? readContext read.key
    | throwError "a namespace invariant resource key has an unknown type"
  let encodedKey ← (domainOf keyTy).encode (← translate readContext read.key)
  let globals ← mkAppM ``LeanerIR.RuntimeState.globals #[state]
  let contains ← familyAccessor context.typeInstantiation family `contains
    #[globals, encodedKey]
  mkEq contains (mkConst ``Bool.true)

/-- Translate the body at one fixed quantifier assignment and add Move's
implicit guard: a value-based invariant constrains an address only while all
resources it reads exist in the selected pre/post states. -/
private def guardedInvariantBody (context : Context) (root body : ExprId)
    (condition : Option ExprId) : MetaM Lean.Expr := do
  let proposition ← translate context body
  let mut guards := #[]
  if let some condition := condition then
    guards := guards.push (← translate context condition)
  let reads := (invariantValueReads context.ns root).foldl
    (fun unique read => if unique.contains read then unique else unique.push read) #[]
  for read in reads do
    guards := guards.push (← invariantReadGuard context read)
  if guards.isEmpty then return proposition
  mkArrow (← conjunction guards) proposition

private inductive InvariantPhase where
  | entry
  | exit
  deriving BEq

/-- Translate this namespace's invariants in the state selected by `context`.
Invariant locals are declaration-owned, so their quantifier binders start
unbound even when the surrounding function has locals with the same IDs.
Update invariants are obligations only at exit; regular invariants are also
assumptions at entry. -/
private def namespaceInvariantTerms (context : Context)
    (modifiedResources : Option (Array ModifiedResource)) (phase : InvariantPhase) :
    MetaM (Array (Lean.Expr × Nat × Nat)) := do
  let mut terms := #[]
  for declaration in context.ns.invariants do
    let (typeParameters, isUpdate) ← match declaration.condition.kind with
      | .globalInvariant typeParameters => pure (typeParameters, false)
      | .globalInvariantUpdate typeParameters => pure (typeParameters, true)
      | kind => throwError "namespace condition {repr kind} is not a global invariant"
    if phase == .entry && isUpdate then continue
    unless typeParameters.isEmpty do
      throwError "generic namespace invariants are not supported in generated contracts"
    unless declaration.condition.auxiliary.isEmpty do
      throwError "namespace invariants cannot carry auxiliary expressions"
    let localTypes ← declaration.locals.mapM fun localDecl => do
      let some ty := context.unit.tables.types[localDecl.type.typeId.index]?
        | throwError "namespace invariant local type {localDecl.type.typeId.index} is out of range"
      pure ty
    let emptyLocals := Array.replicate declaration.locals.size none
    let invariantContext := { context with
      locals := emptyLocals
      localTypes
      oldLocals := emptyLocals
      results := #[]
      resultTypes := #[] }
    let range := conditionRange context.unit declaration.condition
    let invariantFamilies := (invariantValueReads context.ns
      declaration.condition.expression).map (fun read => read.typeIndex)
    let modifiedKeys := modifiedResources.map fun resources =>
      resources.filterMap fun resource =>
        if invariantFamilies.contains resource.typeIndex then some resource.key else none
    match modifiedKeys with
    | none =>
        let some root := context.ns.expressions[declaration.condition.expression.index]?
          | throwError "namespace invariant expression is out of range"
        match root.kind with
        | .quantifier .forall #[binder] triggers condition body =>
            unless triggers.isEmpty do
              throwError "namespace invariant triggers are not supported in generated contracts"
            let some pattern := context.ns.patterns[binder.pattern.index]?
              | throwError "namespace invariant quantifier pattern is out of range"
            let .variable localId := pattern.kind
              | throwError "namespace invariants require a variable binder"
            let some patternType := context.unit.tables.types[pattern.typeId.index]?
              | throwError "namespace invariant binder type is out of range"
            let proposition ← withLocalDeclD (Name.mkSimple s!"invariant_{localId.index}")
                (domainOf patternType).leanType fun value => do
              let active := { invariantContext with
                locals := invariantContext.locals.set! localId.index (some value)
                oldLocals := invariantContext.oldLocals.set! localId.index (some value) }
              let bodyProposition ← guardedInvariantBody active
                declaration.condition.expression body condition
              mkForallFVars #[value] bodyProposition
            terms := terms.push (proposition, range.1, range.2)
        | _ =>
            let proposition ← guardedInvariantBody invariantContext
              declaration.condition.expression declaration.condition.expression none
            terms := terms.push (proposition, range.1, range.2)
    | some keys =>
        if keys.isEmpty then continue
        let some root := context.ns.expressions[declaration.condition.expression.index]?
          | throwError "namespace invariant expression is out of range"
        match root.kind with
        | .quantifier .forall #[binder] triggers condition body =>
            unless triggers.isEmpty do
              throwError "namespace invariant triggers are not supported in generated contracts"
            let some pattern := context.ns.patterns[binder.pattern.index]?
              | throwError "namespace invariant quantifier pattern is out of range"
            let .variable localId := pattern.kind
              | throwError "modified-key invariant specialization requires a variable binder"
            let mut translatedKeys := #[]
            for key in keys do
              let key ← translate context key
              if translatedKeys.any (· == key) then continue
              translatedKeys := translatedKeys.push key
              let active := { invariantContext with
                locals := invariantContext.locals.set! localId.index (some key)
                oldLocals := invariantContext.oldLocals.set! localId.index (some key) }
              let proposition ← guardedInvariantBody active
                declaration.condition.expression body condition
              terms := terms.push (proposition, range.1, range.2)
        | _ =>
            let proposition ← guardedInvariantBody invariantContext
              declaration.condition.expression declaration.condition.expression none
            terms := terms.push (proposition, range.1, range.2)
  pure terms

/-- Source-key expressions named by the function's frame. Specializing a
universal invariant at precisely these keys is sufficient together with the
generated frame, and avoids duplicating a quantified final-state expression. -/
private def invariantModifiedResources (ns : ValidatedNamespace)
    (contract : LeanerIR.FunctionContract) : MetaM (Option (Array ModifiedResource)) := do
  if contract.modifiesAll || hasLooseFrame contract then return none
  let mut resources := #[]
  for modified in contract.modifies do
    let some expression := ns.expressions[modified.index]?
      | throwError "a modifies expression is out of range"
    let .operation (.specification (.global _)) instantiations arguments _ := expression.kind
      | throwError "a modifies clause must name a resource at a key"
    let [.typeArg resource] := instantiations.toList
      | throwError "a modifies clause must name one resource type"
    let some key := arguments[0]?
      | throwError "a modifies clause expects one key"
    resources := resources.push { typeIndex := resource.typeId.index, key }
  pure (some resources)

private def groupConditions (unit : ValidatedUnit)
    (conditions : Array LeanerIR.Condition) : MetaM ClauseGroups := do
  let mut groups : ClauseGroups := {}
  for condition in conditions do
    let range := conditionRange unit condition
    match condition.kind with
    | .requires => groups := { groups with requires := groups.requires.push condition.expression }
    | .ensures => groups := { groups with
        ensures := groups.ensures.push (condition.expression, range) }
    | .abortsIf =>
        let code := condition.auxiliary.findSome? fun (key, value) =>
          if key == "abortCode" then some value else none
        groups := { groups with
          abortsIf := groups.abortsIf.push
            { condition := condition.expression, code, range } }
    | kind =>
        throwError "specification clause {repr kind} is not supported in           generated contracts"
  return groups

/-- Wrap a translated clause with its source range so a residual obligation
can be reported at the authored clause. -/
private def markObligation (range : Nat × Nat) (clause : Lean.Expr) : Lean.Expr :=
  mkApp3 (mkConst ``LeanerIR.Proofs.Obligation)
    (mkRawNatLit range.1) (mkRawNatLit range.2) clause

/-! ## Data invariants

Data invariants belong to values, rather than to the function which happens
to consume or produce them.  The runtime representation deliberately erases
those proofs, so the generated contract restores their verification meaning:
incoming values provide invariant assumptions, while outgoing values and
modified resources owe the invariant again. -/

/-- The declaration and owning namespace of a nominal type. -/
private def nominalDeclaration? (context : Context) (ty : IrTy) :
    Option (LeanerIR.NamespaceId × ValidatedNamespace × LeanerIR.StructDecl) := do
  let .nominal name _ := ty | none
  let qualified ← context.unit.tables.names[name.index]?
  let ns ← context.unit.namespaces[qualified.namespaceId.index]?
  let declaration ← ns.structs.find? (·.name == name)
  some (qualified.namespaceId, ns, declaration)

/-- Whether a physical type actually contributes a data-invariant clause. -/
private def hasDataInvariant (context : Context) (ty : IrTy) : Bool :=
  match nominalDeclaration? context ty with
  | some (_, _, declaration) => declaration.contract.conditions.any
      (fun condition => condition.kind == .structInvariant)
  | none => false

/-- Specialize every invariant declared by `ty` to its erased runtime value.
Struct invariant locals are the fields in declaration order; enum invariants
receive the whole tagged value as local zero and bind payloads in their own
logical match. -/
private def dataInvariantTerms (context : Context) (ty : IrTy)
    (value : Lean.Expr) : MetaM (Array (Lean.Expr × Nat × Nat)) := do
  let some (namespaceId, ns, declaration) := nominalDeclaration? context ty
    | return #[]
  let conditions := declaration.contract.conditions.filter
    (fun condition => condition.kind == .structInvariant)
  if conditions.isEmpty then return #[]
  let localTypes ← declaration.locals.mapM fun localDecl => do
    let some localType := context.unit.tables.types[localDecl.type.typeId.index]?
      | throwError "data invariant local type {localDecl.type.typeId.index} is out of range"
    pure localType
  let mut locals := Array.replicate declaration.locals.size none
  if declaration.variants.isEmpty then
    unless declaration.fields.size <= declaration.locals.size do
      throwError "a struct invariant has fewer declaration locals than fields"
    for (_, index) in declaration.fields.zipIdx do
      let some localType := localTypes[index]?
        | throwError "a struct invariant field local is out of range"
      let selected ← mkAppM ``LeanerIR.RuntimeValue.field #[value, toExpr index]
      locals := locals.set! index
        (some (← (domainOf localType).binderOfRuntime selected))
  else
    unless !declaration.locals.isEmpty do
      throwError "an enum invariant has no `this` declaration local"
    locals := locals.set! 0 (some value)
  let invariantContext : Context := {
    context with
    namespaceId, ns, locals, localTypes, oldLocals := locals
    results := #[], resultTypes := #[] }
  conditions.mapM fun condition => do
    unless condition.auxiliary.isEmpty do
      throwError "a data invariant cannot carry auxiliary expressions"
    let proposition ← translate invariantContext condition.expression
    let range := conditionRange context.unit condition
    pure (proposition, range.1, range.2)

/-- Encode the logical value of a slot without adding the outer borrow which
appears in an argument row.  Data invariants constrain the referent. -/
private def slotValueRuntime (context : Context) (slot : Slot)
    (value : Lean.Expr) : MetaM Lean.Expr := do
  -- A native contract binds an aggregate by its runtime encoding already.
  if (← whnfR (← inferType value)).isConstOf ``RuntimeValue then return value
  match slot.rep with
  | some rep => rep.encode context.codecs value
  | none => (domainOf slot.physical).encode value

/-- Data invariants carried by selected binders of a parameter/result row. -/
private def slotDataInvariantTerms (context : Context) (slots : Array Slot)
    (bound : Array SlotBinders)
    (binderOf : SlotBinders → Option Lean.Expr) :
    MetaM (Array (Lean.Expr × Nat × Nat)) := do
  let mut terms := #[]
  for (slot, binders) in slots.zip bound do
    unless hasDataInvariant context slot.physical do continue
    let some binder := binderOf binders | continue
    let value ← slotValueRuntime context slot binder
    terms := terms ++ (← dataInvariantTerms context slot.physical value)
  pure terms

/-- Re-establish a resource invariant only when the modified slot exists in
the post-state.  Removal is therefore vacuous; publication and mutation owe
the invariant of the exact runtime value left at the key. -/
private def modifiedDataInvariantTerms (context : Context)
    (modifiedResources : Option (Array ModifiedResource)) :
    MetaM (Array Lean.Expr) := do
  let some resources := modifiedResources | return #[]
  /- Module invariants also populate `modifiedResources`; most such resource
  types have no data invariant.  Reject those before constructing post-state
  lookups, which keeps their existing verification path cost-neutral. -/
  let resources := resources.filter fun resource =>
    match context.unit.tables.types[resource.typeIndex]? with
    | some resourceType => hasDataInvariant context resourceType
    | none => false
  if resources.isEmpty then return #[]
  let some state := context.state
    | throwError "modified-resource invariants require a post-state"
  let globals ← mkAppM ``LeanerIR.RuntimeState.globals #[state]
  let mut obligations := #[]
  for resource in resources do
    let some family := context.families.find? (·.typeIndex == resource.typeIndex)
      | throwError "a modified resource with a data invariant has no typed family"
    let some resourceType := context.unit.tables.types[resource.typeIndex]?
      | throwError "a modified resource type is out of range"
    let some keyType := typeOfExpr? context resource.key
      | throwError "a modified resource key has an unknown type"
    let encodedKey ← (domainOf keyType).encode (← translate context resource.key)
    let key ← familyAccessor context.typeInstantiation family `key #[encodedKey]
    let lookup ← mkAppM ``LeanerIR.GlobalMap.lookup #[globals, key]
    let present ← mkEq (← mkAppM ``Option.isSome #[lookup]) (mkConst ``Bool.true)
    let value ← mkAppM ``Option.getD #[lookup, mkConst ``RuntimeValue.unit]
    let invariants ← dataInvariantTerms context resourceType value
    for (invariant, start, stop) in invariants do
      obligations := obligations.push
        (← mkArrow present (markObligation (start, stop) invariant))
  pure obligations

/-- The runtime key a `modifies global<T>(k)` clause names, as a
`GlobalKey`-valued term over the contract's logical binders. -/
private def modifiedKeyTerm (context : Context) (id : ExprId) :
    MetaM Lean.Expr := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "a modifies clause is out of range"
  match expression.kind with
  | .operation (.specification (.global _)) instantiations arguments _ =>
      let resource ← match instantiations.toList with
        | [.typeArg resource] => pure resource.typeId
        | _ => throwError "a modifies clause needs one resource type"
      let some family := context.families.find? (·.typeIndex == resource.index)
        | throwError "the resource type of a modifies clause has no typed \
            specification twin"
      let some key := arguments[0]?
        | throwError "a modifies clause expects one key"
      let some keyTy := typeOfExpr? context key
        | throwError "a modifies key has an unknown type"
      let encoded ← (domainOf keyTy).encode (← translate context key)
      familyAccessor context.typeInstantiation family `key #[encoded]
  | _ =>
      throwError "a modifies clause must name a resource at a key in \
        generated contracts"

/-- Bind one typed-contents binder per storable family and continue. -/
private def withFamilyContents (families : Array SpecTypes.FamilyInfo)
    (carrier : Option Lean.Expr)
    (k : Array Lean.Expr → MetaM α) : MetaM α :=
  let rec go (index : Nat) (bound : Array Lean.Expr) : MetaM α := do
    if h : index < families.size then
      let family := families[index]
      let contentsType ← mkArrow (mkConst ``LeanerIR.StorageKey)
        (← mkAppM ``Option #[← family.leanType carrier])
      withLocalDeclD (Name.mkSimple (family.info.name ++ "_contents"))
        contentsType fun binder => go (index + 1) (bound.push binder)
    else k bound
  go 0 #[]

/-- The representation conjuncts tying each family binder to the runtime
map: this is the well-typed-store precondition, in the shape that rewrites. -/
private def familyConjuncts (families : Array SpecTypes.FamilyInfo)
    (bound : Array Lean.Expr) (state : Lean.Expr)
    (codecs typeInstantiation : Option Lean.Expr) : MetaM (Array Lean.Expr) := do
  let globals ← mkAppM ``LeanerIR.RuntimeState.globals #[state]
  (families.zip bound).mapM fun (family, binder) => do
    let typeId := toExpr (⟨family.typeIndex⟩ : LeanerIR.TypeId)
    let typeId ← match typeInstantiation with
      | none => pure typeId
      | some instantiation =>
          mkAppM ``LeanerIR.SemanticOperations.instantiatedTypeId
            #[instantiation, typeId]
    mkAppM ``LeanerIR.FamilyRepresentation
      #[← family.erase codecs,
        toExpr (⟨family.info.namespaceIndex⟩ : LeanerIR.NamespaceId),
        typeId,
        binder, globals]

/-- Whether the lowered contract sets a boolean pragma.  Lowering already
merged the namespace's pragmas into the contract, contract-first, so the
declared attribute list is authoritative. -/
private def pragmaEnabled (contract : LeanerIR.FunctionContract) (name : String) : Bool :=
  contract.pragmas.any fun pragma =>
    match pragma with
    | .assign pragmaName (.constant (.bool true)) _ => pragmaName == name
    | _ => false

/-- The storable families a function can touch: those a global operation
of its body or a specification clause names, directly or through the
specification functions a clause expands.  A summary mentions only what
the function can read or write, so a caller consuming it is not made to
speak about families the callee never sees. -/
private def familiesUsed (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (families : Array SpecTypes.FamilyInfo) : Array SpecTypes.FamilyInfo := Id.run do
  let mut roots : Array ExprId :=
    declaration.contract.conditions.flatMap fun condition =>
      #[condition.expression] ++ condition.auxiliary.map (·.2)
  roots := roots ++ ns.invariants.flatMap fun invariant =>
    #[invariant.condition.expression] ++ invariant.condition.auxiliary.map (·.2)
  if let .structured root := declaration.body then
    roots := roots.push root
  let mut work : List (LeanerIR.NamespaceId × ExprId) := roots.toList.map (namespaceId, ·)
  let mut visited : Array (Nat × Nat) := #[]
  let mut used : Array Nat := #[]
  let bound := unit.namespaces.foldl (fun total ns => total + ns.expressions.size) roots.size
  for _ in [0:bound + 1] do
    match work with
    | [] => break
    | (owner, id) :: rest =>
        work := rest
        if visited.contains (owner.index, id.index) then continue
        visited := visited.push (owner.index, id.index)
        let some ownerNs := unit.namespaces[owner.index]? | continue
        let some expression := ownerNs.expressions[id.index]? | continue
        match expression.kind with
        | .operation operation instantiations _ _ =>
            match operation with
            | .global _ | .specification (.global _) =>
                for instantiation in instantiations do
                  if let .typeArg typeUse := instantiation then
                    used := used.push typeUse.typeId.index
            | .specification (.functionCall reference _) =>
                let body? := do
                  let targetNs ← unit.namespaces[reference.namespaceId.index]?
                  let functionId ← unit.resolution.specFunction? reference.name
                  let callee ← targetNs.specFunctions[functionId.index]?
                  callee.body
                if let some body := body? then
                  work := work ++ [(reference.namespaceId, body)]
            | _ => pure ()
        | _ => pure ()
        work := work ++ (LeanerIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·)
  return families.filter fun family => used.contains family.typeIndex

/-- The quoted native signature a contract is stated over. -/
structure NativeSignature where
  /-- The skolem family the carriers are taken at. -/
  skolems : Lean.Expr
  /-- The parameter row. -/
  params : Lean.Expr
  /-- Each parameter's native type, in order. -/
  argumentTypes : Array Lean.Expr
  /-- The result shape. -/
  shape : Lean.Expr
  /-- The result's native type, when the function has a result. -/
  resultType : Option Lean.Expr := none
  /-- The component types of a tuple result. -/
  resultComponentTypes : Array Lean.Expr := #[]

/-- The components of a native row value, as projections. -/
private def rowProjections (skolems row : Lean.Expr) (types : Array Lean.Expr) : Array Lean.Expr :=
  Id.run do
    let mut rest := row
    let mut values := #[]
    for index in [:types.size] do
      let tail := (types.extract (index + 1) types.size).foldr
        (fun ty row => mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NRow.cons) ty row)
        (mkConst ``LeanerIR.Proofs.Denote.NRow.nil)
      let carrier := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.carrier) skolems types[index]!
      let tailType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.HList) skolems tail
      values := values.push (mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) carrier tailType rest)
      rest := mkApp3 (mkConst ``Prod.snd [Level.zero, Level.zero]) carrier tailType rest
    return values

/-- Whether a quoted native type is a mutable reference. -/
private def isReferenceType (ty : Lean.Expr) : Bool :=
  ty.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.ref 1

/-- The current value and the prophecy of a native reference to `referent`. -/
private def referenceParts (skolems referent value : Lean.Expr) : Lean.Expr × Lean.Expr :=
  let carrier := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.carrier) skolems referent
  (mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) carrier carrier value,
    mkApp3 (mkConst ``Prod.snd [Level.zero, Level.zero]) carrier carrier value)

/-- The clause binder of a native value at a physical type: an integer's
value, a boolean or text as itself, and an aggregate's encoding. -/
private def nativeBinder (skolems : Lean.Expr) (physical : IrTy) (ty value : Lean.Expr) :
    MetaM Lean.Expr :=
  match domainOf physical with
  | .integer => mkAppM ``LeanerIR.SpecInt.val #[value]
  | .boolean | .text _ => pure value
  | .aggregate => pure (mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) skolems ty value)

/-- The clause binders of the parameters, read off the native arguments.  A
mutable reference's entry is its current value and its exit its prophecy. -/
private def parameterBinders (skolems : Lean.Expr) (slots : Array Slot) (types : Array Lean.Expr)
    (arguments : Lean.Expr) : MetaM (Array SlotBinders) := do
  let values := rowProjections skolems arguments types
  unless slots.size == types.size do
    throwError "internal: a parameter row differs from its declaration"
  (slots.zip (types.zip values)).mapM fun (slot, ty, value) => do
    match slot.kind with
    | .mutableRef =>
        let referent := ty.appArg!
        let (current, prophecy) := referenceParts skolems referent value
        return { entry := ← nativeBinder skolems slot.physical referent current,
                 exit := some (← nativeBinder skolems slot.physical referent prophecy) }
    | .plain | .sharedRef => return { entry := ← nativeBinder skolems slot.physical ty value }

/-- Whether the expression tree at `root` reads a returned reference's final
value. -/
def mentionsFinal (ns : ValidatedNamespace) (root : ExprId) : Bool := Id.run do
  let mut pending := #[root]
  let mut visited : Array Nat := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if let .operation (.specification .final) .. := expression.kind then return true
    pending := pending ++ LeanerIR.Validation.expressionChildren expression.kind
  return false

/-- Whether a function's contract relates a returned mutable reference's final
value to its lenders: then callers use the contract, not the body. -/
def hasFinalContract (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  declaration.contract.conditions.any fun condition =>
    condition.kind == .ensures && mentionsFinal ns condition.expression

/-- The clause binders of the result, read off the native result, with the
value view of every reference it returns: its prophecy is its current
value.  A clause sees a returned reference as its current value, and reads
its prophecy with `final`. -/
private def resultBinders (slots : Array Slot) (signature : NativeSignature)
    (result : Lean.Expr) :
    MetaM (Array SlotBinders × Array Lean.Expr × Array (Option Lean.Expr)) := do
  match slots.toList, signature.resultType with
  | [], _ => return (#[], #[], #[])
  | [slot], some ty =>
      if slot.kind == .mutableRef then
        let referent := ty.appArg!
        let (current, prophecy) := referenceParts signature.skolems referent result
        return (#[{ entry := ← nativeBinder signature.skolems slot.physical referent current }],
          #[← mkEq prophecy current],
          #[some (← nativeBinder signature.skolems slot.physical referent prophecy)])
      if signature.resultComponentTypes.any isReferenceType then
        let values := rowProjections signature.skolems result signature.resultComponentTypes
        let mut viewed := #[]
        let mut premises := #[]
        let mut finals := #[]
        for (componentType, value) in signature.resultComponentTypes.zip values do
          if isReferenceType componentType then
            let referent := componentType.appArg!
            let (current, prophecy) := referenceParts signature.skolems referent value
            viewed := viewed.push
              (mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) signature.skolems referent current)
            premises := premises.push (← mkEq prophecy current)
            finals := finals.push (some
              (mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) signature.skolems referent prophecy))
          else
            finals := finals.push none
            viewed := viewed.push
              (mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) signature.skolems componentType
                value)
        let tuple ← mkAppM ``LeanerIR.RuntimeValue.tuple
          #[← mkArrayLit (mkConst ``LeanerIR.RuntimeValue) viewed.toList]
        return (#[{ entry := tuple }], premises, finals)
      return (#[{ entry := ← nativeBinder signature.skolems slot.physical ty result }], #[], #[none])
  | _, _ => throwError "internal: a result row differs from its declaration"

/-- The recursive specification functions of a unit that name a measure:
those with a definition. Without a measure there is no definition; a
contract using the function reports the missing measure. -/
def recursiveSpecFunctions (unit : ValidatedUnit) : Array LeanerIR.QualifiedRef := Id.run do
  let mut found := #[]
  for ns in unit.namespaces, namespaceIndex in [0:unit.namespaces.size] do
    for declaration in ns.specFunctions do
      let some qualified := unit.tables.names[declaration.name.index]? | continue
      let reference : LeanerIR.QualifiedRef := ⟨⟨namespaceIndex⟩, declaration.name⟩
      unless qualified.namespaceId == reference.namespaceId && declaration.body.isSome do continue
      unless declaration.contract.conditions.any (·.kind == .decreases) do continue
      unless specRecursive unit reference do continue
      found := found.push reference
  return found

/-- Define the recursive specification functions of a unit, so that lemmas
about a definition can precede the verification using it. -/
def ensureSpecFunctionDefinitions (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (families : Array SpecTypes.FamilyInfo) : MetaM Unit := do
  for reference in recursiveSpecFunctions unit do
    let some ns := unit.namespaces[reference.namespaceId.index]? | continue
    let context : Context := {
      unit, ns, twins, families
      namespaceId := reference.namespaceId
      locals := #[]
      results := #[] }
    let _ ← translate.ensureDefinition context reference

/-- The contract of a function over its native arguments and result. -/
def buildContract (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (signature : NativeSignature)
    (twins : Array SpecTypes.TwinInfo := #[])
    (families : Array SpecTypes.FamilyInfo := #[])
    (carrier : Option Lean.Expr := none)
    (codecs : Option Lean.Expr := none)
    (typeInstantiation : Option Lean.Expr := none) :
    MetaM Lean.Expr := do
  let families := familiesUsed unit namespaceId ns declaration families
  let groups ← groupConditions unit declaration.contract.conditions
  let invariantResources ← invariantModifiedResources ns declaration.contract
  let isPartial := pragmaEnabled declaration.contract "aborts_if_is_partial"
  let isStrict := pragmaEnabled declaration.contract "aborts_if_is_strict"
  let parameterSlots ← declaration.signature.parameters.mapIdxM fun index parameter =>
    slotOf { unit := unit, namespaceId := namespaceId, ns := ns,
             locals := #[], results := #[], twins := twins, families := families,
             carrier := carrier, codecs := codecs,
             typeInstantiation := typeInstantiation }
      (Name.mkSimple (if parameter.name.isEmpty then s!"argument{index}" else parameter.name))
      parameter.typeUse.typeId (allowReference := true)
  let resultSlots ← declaration.signature.results.mapIdxM fun index result =>
    slotOf { unit := unit, namespaceId := namespaceId, ns := ns,
             locals := #[], results := #[], twins := twins, families := families,
             carrier := carrier, codecs := codecs,
             typeInstantiation := typeInstantiation }
      (Name.mkSimple (if declaration.signature.results.size == 1 then "result"
        else s!"result{index}"))
      result.typeId (allowReference := true)
  let functionLocalTypes ← declaration.locals.mapM fun localDecl => do
    let some ty := unit.tables.types[localDecl.type.typeId.index]?
      | throwError "function local has an unknown type"
    match ty with
    | .reference reference =>
        let some referent := unit.tables.types[reference.referent.index]?
          | throwError "function reference local has an unknown referent type"
        pure referent
    | _ => pure ty
  let padLocals (values : Array (Option Lean.Expr)) : Array (Option Lean.Expr) :=
    values ++ Array.replicate (declaration.locals.size - values.size) none
  let argumentsType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.HList) signature.skolems
    signature.params
  let resultType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.ResultShape.carrier) signature.skolems
    signature.shape
  let runtimeState := mkConst ``LeanerIR.RuntimeState
  let failure := mkConst ``LeanerIR.Proofs.Failure
  /- In a one-state clause a parameter denotes its entry value; in
  `ensures` it denotes the current value — the prophecy of a mutable
  reference — and `spec.old` reaches the entry. -/
  let contextOf (state : Lean.Expr) (bound : Array SlotBinders) : Context :=
    { unit, namespaceId, ns, twins, families
      carrier := carrier, codecs := codecs,
      typeInstantiation := typeInstantiation
      locals := padLocals (bound.map fun binders => some binders.entry)
      localTypes := functionLocalTypes
      localNames := declaration.locals.map (·.name)
      oldLocals := padLocals (bound.map fun binders => some binders.entry)
      results := #[]
      state := some state, oldState := some state }
  let translateAll (context : Context) (clauses : Array ExprId) :
      MetaM (Array Lean.Expr) :=
    clauses.mapM (translate context)
  let requiresTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withFamilyContents families carrier fun familyBound => do
        let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
        let context := contextOf state bound
        let represented ← familyConjuncts families familyBound state codecs typeInstantiation
        let dataInvariants ← slotDataInvariantTerms context parameterSlots bound
          (some ·.entry)
        let clauses ← translateAll context groups.requires
        let invariants ← namespaceInvariantTerms
          context invariantResources .entry
        let body ← conjunction
          (represented ++ clauses ++ dataInvariants.map (·.1) ++ invariants.map (·.1))
        let closed ← existsOver familyBound body
        mkLambdaFVars #[arguments, state] closed
  let ensuresTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `result resultType fun result =>
        withLocalDeclD `final runtimeState fun final => do
          let parameterBound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
          let (resultBound, prophecies, finals) ← resultBinders resultSlots signature result
          let context : Context :=
            { unit, namespaceId, ns, twins, families
              carrier := carrier, codecs := codecs,
              typeInstantiation := typeInstantiation
              locals := padLocals (parameterBound.map fun binders => some binders.current)
              localTypes := functionLocalTypes
              localNames := declaration.locals.map (·.name)
              oldLocals := padLocals (parameterBound.map fun binders => some binders.entry)
              results := resultBound.map (·.entry)
              resultFinals := finals
              resultTypes := resultSlots.map (·.physical)
              state := some final, oldState := some state }
          let clauses ← groups.ensures.mapM fun (clause, range) =>
            return markObligation range (← translate context clause)
          let resultInvariants ← slotDataInvariantTerms context resultSlots resultBound
            (some ·.entry)
          let exitInvariants ← slotDataInvariantTerms context parameterSlots parameterBound
            (·.exit)
          let dataInvariantObligations :=
            (resultInvariants ++ exitInvariants).map fun (clause, start, stop) =>
              markObligation (start, stop) clause
          let modifiedDataInvariants ←
            modifiedDataInvariantTerms context invariantResources
          let invariants ← namespaceInvariantTerms context invariantResources .exit
          let invariantObligations := invariants.map fun (clause, start, stop) =>
            markObligation (start, stop) clause
          let body ← conjunction
            (clauses ++ dataInvariantObligations ++ modifiedDataInvariants ++ invariantObligations)
          -- The value view: a returned reference is read at its current
          -- value, as if it died on return, unless the clauses read its
          -- final value.
          let readsFinal := groups.ensures.any fun (clause, _) => mentionsFinal ns clause
          let body ← if prophecies.isEmpty || readsFinal then pure body
            else mkArrow (← conjunction prophecies) body
          mkLambdaFVars #[arguments, state, result, final] body
  /- The three failure components follow the Move-style
  reading of `aborts_if Pᵢ [with Cᵢ]`.  Without clauses the behavior is
  uninterpreted (with `aborts_if_is_strict`: never fails).  With clauses,
  every Pᵢ both forces a failure and excuses the postcondition; a non-partial
  list also permits only outcomes matching a clause — with its code, when one
  is declared — while `aborts_if_is_partial` additionally permits any outcome
  in states where no Pᵢ holds. -/
  let declaredAborts := !groups.abortsIf.isEmpty
  let abortConditionTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state => do
      let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
      let clauses ← translateAll (contextOf state bound) (groups.abortsIf.map (·.condition))
      let body ← if declaredAborts then disjunction clauses else pure (mkConst ``False)
      mkLambdaFVars #[arguments, state] body
  let abortsTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `failure failure fun failureBinder => do
        if !declaredAborts then
          let permitted := if isStrict then mkConst ``False else mkConst ``True
          mkLambdaFVars #[arguments, state, failureBinder] permitted
        else
          let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
          let context := contextOf state bound
          let mut matched : Array Lean.Expr := #[]
          let mut conditions : Array Lean.Expr := #[]
          for clause in groups.abortsIf do
            let condition ← translate context clause.condition
            conditions := conditions.push condition
            match clause.code with
            | some code =>
                let codeTerm ← translate context code
                let encoded ← mkAppM ``LeanerIR.RuntimeValue.integer #[codeTerm]
                let payload ← mkArrayLit (mkConst ``LeanerIR.RuntimeValue) [encoded]
                let outcome ← mkAppM ``Prod.mk
                  #[mkConst ``LeanerIR.ThrowKind.abort, payload]
                matched := matched.push (markObligation clause.range (← mkAppM ``And
                  #[condition, ← mkAppM ``Eq #[failureBinder, outcome]]))
            | none => matched := matched.push (markObligation clause.range condition)
          let mut permitted ← disjunction matched
          if isPartial then
            permitted ← mkAppM ``Or
              #[permitted, ← mkAppM ``Not #[← disjunction conditions]]
          mkLambdaFVars #[arguments, state, failureBinder] permitted
  /- The frame is stated over global memory: a successful execution leaves
  the globals it does not declare as modified alone.  With no `modifies`
  clause the whole map is unchanged; with clauses, every key other than the
  declared ones reads the same, which the keyed map laws discharge and a
  caller frames with. -/
  let frameTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `initial runtimeState fun initial =>
      withLocalDeclD `final runtimeState fun final => do
        let finalGlobals ← mkAppM ``LeanerIR.RuntimeState.globals #[final]
        let initialGlobals ← mkAppM ``LeanerIR.RuntimeState.globals #[initial]
        let body ←
          if declaration.contract.modifiesAll then
            pure (mkConst ``True)
          else if declaration.contract.modifies.isEmpty then
            mkAppM ``Eq #[finalGlobals, initialGlobals]
          else do
            let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
            let context := contextOf initial bound
            let keys ← declaration.contract.modifies.mapM
              (modifiedKeyTerm context)
            let frameAt (key : Lean.Expr)
                (family : Option (Lean.Expr × Lean.Expr × Lean.Expr) := none) : MetaM Lean.Expr := do
              let mut implication ← mkAppM ``Eq
                #[← mkAppM ``LeanerIR.GlobalMap.lookup #[finalGlobals, key],
                  ← mkAppM ``LeanerIR.GlobalMap.lookup #[initialGlobals, key]]
              for modified in keys.reverse do
                let mut query := key
                let mut written := modified
                if let some (namespaceId, typeId, storageKey) := family then
                  let modifiedNamespace ← withTransparency .default <|
                    whnf (← mkAppM ``LeanerIR.GlobalKey.namespaceId #[modified])
                  let modifiedType ← withTransparency .default <|
                    whnf (← mkAppM ``LeanerIR.GlobalKey.typeId #[modified])
                  if namespaceId == modifiedNamespace && typeId == modifiedType then
                    query := storageKey
                    written ← withTransparency .default <|
                      whnf (← mkAppM ``LeanerIR.GlobalKey.key #[modified])
                implication ← mkArrow (← mkAppM ``Ne #[query, written]) implication
              pure implication
            if hasLooseFrame declaration.contract then do
              -- Quantify inside each listed family. Its
              -- namespace/type are structural, not hypotheses a closer
              -- must rediscover when framing an unrelated-family write.
              let mut seen : Std.HashSet (Lean.Expr × Lean.Expr) := {}
              let mut frames := #[]
              for modified in keys do
                let namespaceId ← withTransparency .default <|
                  whnf (← mkAppM ``LeanerIR.GlobalKey.namespaceId #[modified])
                let typeId ← withTransparency .default <|
                  whnf (← mkAppM ``LeanerIR.GlobalKey.typeId #[modified])
                if seen.contains (namespaceId, typeId) then continue
                seen := seen.insert (namespaceId, typeId)
                let frame ← withLocalDeclD `key (mkConst ``LeanerIR.StorageKey) fun key => do
                  let globalKey ← mkAppM ``LeanerIR.GlobalKey.mk #[namespaceId, typeId, key]
                  mkForallFVars #[key] (← frameAt globalKey (some (namespaceId, typeId, key)))
                frames := frames.push frame
              conjunction frames
            else
              withLocalDeclD `key (mkConst ``LeanerIR.GlobalKey) fun key => do
                mkForallFVars #[key] (← frameAt key)
        mkLambdaFVars #[arguments, initial, final] body
  mkAppOptM ``LeanerIR.Proofs.Contract.mk
    #[some runtimeState, some failure, some argumentsType, some resultType,
      some requiresTerm, some ensuresTerm, some abortsTerm,
      some abortConditionTerm, some abortConditionTerm, some frameTerm]

/-- Segments of a `leanerPath`, ignoring separators. -/
private partial def pathSegments (stx : Syntax) : Array String :=
  match stx with
  | .ident _ raw _ _ => #[raw.toString]
  | .atom _ value => if value == "::" then #[] else #[value]
  | .node _ _ arguments => arguments.flatMap pathSegments
  | _ => #[]

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

/-- Locate one function of a registered unit by its final path segment. -/
def findFunction? (unit : ValidatedUnit) (name : String) :
    Option (Nat × ValidatedNamespace × Nat ×
      LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) := do
  for h : namespaceIndex in [0:unit.namespaces.size] do
    let ns := unit.namespaces[namespaceIndex]
    for h : index in [0:ns.functions.size] do
      let declaration := ns.functions[index]
      if let some qualified := unit.tables.names[declaration.name.index]? then
        if qualified.name == name then
          return (namespaceIndex, ns, index, declaration)
  none

/-- Name of the generated contract definition. -/
def contractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "contract"

/-- Name of the contract over a function's native arguments and result. -/
def typedContractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedContract"

private def rootIdent (name : Name) : Ident :=
  mkIdent (rootNamespace ++ name)

/-- Name of the generated validated-unit definition. -/
def unitName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "unit"

/-- Define the registered unit as a Lean literal, once per namespace. -/
def ensureUnitDefinition (namespaceSegments : Array String)
    (unit : ValidatedUnit) : CommandElabM Name := do
  let name := unitName namespaceSegments
  if (← getEnv).contains name then return name
  liftTermElabM do
    addDecl (.defnDecl {
      name, levelParams := []
      type := mkConst ``LeanerIR.Validation.ValidatedUnit
      value := toExpr unit
      hints := .abbrev, safety := .safe })
    enableRealizationsForConst name
  return name

/-- Name of the generated prepared-semantics definition. -/
def semanticsName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "semantics"

/-- Name of the equation identifying the semantic preparation of the unit
with its quoted literal. -/
def semanticsEqName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "semantics_eq"

/-- Name of the equation evaluating the unit's target pointer width. -/
def pointerWidthEqName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "targetPointerWidth_eq"

initialize Lean.registerTraceClass `leaner.preparation

/-- Quote a preparation update while sharing unchanged metadata with the
preceding unit. Re-quoting every declaration, table, and borrow certificate
made the kernel compare enormous copies of fields neither pass changes. -/
private def quotePreparationUpdate (parentName : Name)
    (parent result : ValidatedUnit) : TermElabM Lean.Expr := do
  let parentExpr := mkConst parentName
  let template := toExpr (default : ValidatedUnit)
  let mut fields := template.getAppArgs
  for i in [:fields.size] do
    fields := fields.set! i (mkProj ``ValidatedUnit i parentExpr)
  let tables := if parent.tables == result.tables then fields[0]!
    else toExpr result.tables
  fields := fields.set! 0 tables
  let mut namespaces := #[]
  for i in [:result.namespaces.size] do
    let ns := result.namespaces[i]!
    let parentNs ← Lean.Elab.Term.elabTerm
      (← `(($(rootIdent parentName):term).namespaces[$(Syntax.mkNatLit i)]!))
      (some (mkConst ``ValidatedNamespace))
    Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
    let parentNs ← instantiateMVars parentNs
    let nsTemplate := toExpr (default : ValidatedNamespace)
    let nsCore := nsTemplate.getAppArgs[0]!
    let mut nsFields := nsCore.getAppArgs
    let originalCore := mkProj ``ValidatedNamespace 0 parentNs
    -- Namespace's first constructor argument is its body-stage type.
    for field in [1:nsFields.size] do
      nsFields := nsFields.set! field (mkProj ``LeanerIR.Namespace (field - 1) originalCore)
    if ns.expressions != parent.namespaces[i]!.expressions then
      nsFields := nsFields.set! 5 (toExpr ns.expressions)
    if ns.places != parent.namespaces[i]!.places then
      nsFields := nsFields.set! 7 (toExpr ns.places)
    let core := mkAppN nsCore.getAppFn nsFields
    namespaces := namespaces.push (mkAppN nsTemplate.getAppFn
      #[core, tables, mkProj ``ValidatedNamespace 2 parentNs])
  fields := fields.set! 2 (← mkArrayLit (mkConst ``ValidatedNamespace) namespaces.toList)
  return mkAppN template.getAppFn fields

/-- Completed stages survive a later failed certificate. Reuse them on a
retry instead of masking the original failure with duplicate declarations. -/
private def addPreparationDecl (declaration : Declaration) : TermElabM Unit := do
  let env ← getEnv
  if declaration.getTopLevelNames.all env.contains then return
  addDecl declaration
  match declaration with
  | .defnDecl value => enableRealizationsForConst value.name
  | _ => pure ()

/-- Quote the semantically prepared unit once per namespace. The elaborator
computes each pass natively and the kernel checks its result by definitional
equality. A small transitivity certificate connects the named intermediate
and final units to `prepareExecution`; there is no native-decision axiom.
Every verify shares these certificates, and goals carry constants instead
of repeatedly reducing the complete preparation pipeline. -/
def ensureSemanticsDefinitions (namespaceSegments : Array String)
    (unit : ValidatedUnit) : CommandElabM (Name × Name) := do
  trace[leaner.preparation] "unit quotation start: {← IO.getNumHeartbeats}"
  let unitDefinition ← ensureUnitDefinition namespaceSegments unit
  trace[leaner.preparation] "unit quotation done: {← IO.getNumHeartbeats}"
  let name := semanticsName namespaceSegments
  let eqName := semanticsEqName namespaceSegments
  let widthEqName := pointerWidthEqName namespaceSegments
  if (← getEnv).contains eqName && (← getEnv).contains widthEqName then
    return (eqName, widthEqName)
  let marked := (LeanerIR.Validation.markLoanDeaths unit).1
  let indexes := LeanerIR.Validation.sharedReferenceErasureIndexes marked
  let chunks := LeanerIR.Validation.sharedReferenceErasureChunks marked
  let plan := LeanerIR.Validation.erasurePlanFromChunks chunks
  let expressionArenas := LeanerIR.Validation.erasedExpressionArenas marked
  let prepared := LeanerIR.Validation.applySharedReferenceErasure marked plan
  liftTermElabM do
    let markedName := Name.str name "marked"
    let markedEqName := Name.str name "marked_eq"
    addPreparationDecl (.defnDecl {
      name := markedName, levelParams := []
      type := mkConst ``LeanerIR.Validation.ValidatedUnit
      value := ← quotePreparationUpdate unitDefinition unit marked
      hints := .abbrev, safety := .safe })
    trace[leaner.preparation] "marked quotation done: {← IO.getNumHeartbeats}"
    let markedTerm ← mkAppM ``Prod.fst
      #[mkApp (mkConst ``LeanerIR.Validation.markLoanDeaths)
        (mkConst unitDefinition)]
    addPreparationDecl (.thmDecl {
      name := markedEqName, levelParams := []
      type := ← mkEq markedTerm (mkConst markedName)
      value := ← mkEqRefl (mkConst markedName) })
    trace[leaner.preparation] "marked certificate done: {← IO.getNumHeartbeats}"
    let indexesName := Name.str name "erasureIndexes"
    let indexesEqName := Name.str name "erasureIndexes_eq"
    addPreparationDecl (.defnDecl {
      name := indexesName, levelParams := []
      type := mkConst ``LeanerIR.Validation.SharedReferenceErasureIndexes
      value := toExpr indexes
      hints := .abbrev, safety := .safe })
    addPreparationDecl (.thmDecl {
      name := indexesEqName, levelParams := []
      type := ← mkEq
        (mkApp (mkConst ``LeanerIR.Validation.sharedReferenceErasureIndexes) (mkConst markedName))
        (mkConst indexesName)
      value := ← mkEqRefl (mkConst indexesName) })
    trace[leaner.preparation] "erasure indexes certificate done: {← IO.getNumHeartbeats}"
    let chunksName := Name.str name "erasureChunks"
    let chunksEqName := Name.str name "erasureChunks_eq"
    let indexedChunksEqName := Name.str name "erasureIndexedChunks_eq"
    addPreparationDecl (.defnDecl {
      name := chunksName, levelParams := []
      type := mkConst ``LeanerIR.Validation.SharedReferenceErasureChunks
      value := toExpr chunks
      hints := .abbrev, safety := .safe })
    addPreparationDecl (.thmDecl {
      name := indexedChunksEqName, levelParams := []
      type := ← mkEq
        (mkAppN (mkConst ``LeanerIR.Validation.sharedReferenceErasureChunksIndexed)
          #[mkConst markedName, mkConst indexesName])
        (mkConst chunksName)
      value := ← mkEqRefl (mkConst chunksName) })
    addPreparationDecl (.thmDecl {
      name := chunksEqName, levelParams := []
      type := ← mkEq
        (mkApp (mkConst ``LeanerIR.Validation.sharedReferenceErasureChunks) (mkConst markedName))
        (mkConst chunksName)
      value := mkAppN (mkConst ``LeanerIR.Validation.sharedReferenceErasureChunks_eq_of_indexes)
        #[mkConst markedName, mkConst indexesName, mkConst chunksName,
          mkConst indexesEqName, mkConst indexedChunksEqName] })
    trace[leaner.preparation] "erasure chunks certificate done: {← IO.getNumHeartbeats}"
    let planName := Name.str name "erasurePlan"
    let planEqName := Name.str name "erasurePlan_eq"
    let sortedEqName := Name.str name "erasureSorted_eq"
    addPreparationDecl (.defnDecl {
      name := planName, levelParams := []
      type := mkConst ``LeanerIR.Validation.SharedReferenceErasurePlan
      value := toExpr plan
      hints := .abbrev, safety := .safe })
    addPreparationDecl (.thmDecl {
      name := sortedEqName, levelParams := []
      type := ← mkEq
        (mkApp (mkConst ``LeanerIR.Validation.erasurePlanFromChunks) (mkConst chunksName))
        (mkConst planName)
      value := ← mkEqRefl (mkConst planName) })
    addPreparationDecl (.thmDecl {
      name := planEqName, levelParams := []
      type := ← mkEq
        (mkApp (mkConst ``LeanerIR.Validation.sharedReferenceErasurePlan) (mkConst markedName))
        (mkConst planName)
      value := mkAppN (mkConst ``LeanerIR.Validation.sharedReferenceErasurePlan_eq_of_chunks)
        #[mkConst markedName, mkConst chunksName, mkConst planName,
          mkConst chunksEqName, mkConst sortedEqName] })
    trace[leaner.preparation] "erasure plan certificate done: {← IO.getNumHeartbeats}"
    let arenasName := Name.str name "expressionArenas"
    let arenasEqName := Name.str name "expressionArenas_eq"
    let indexedArenasEqName := Name.str name "expressionArenasIndexed_eq"
    addPreparationDecl (.defnDecl {
      name := arenasName, levelParams := []
      type := mkConst ``LeanerIR.Validation.ErasedExpressionArenas
      value := toExpr expressionArenas
      hints := .abbrev, safety := .safe })
    addPreparationDecl (.thmDecl {
      name := indexedArenasEqName, levelParams := []
      type := ← mkEq
        (mkAppN (mkConst ``LeanerIR.Validation.erasedExpressionArenasIndexed)
          #[mkConst markedName, mkConst indexesName]) (mkConst arenasName)
      value := ← mkEqRefl (mkConst arenasName) })
    addPreparationDecl (.thmDecl {
      name := arenasEqName, levelParams := []
      type := ← mkEq
        (mkApp (mkConst ``LeanerIR.Validation.erasedExpressionArenas) (mkConst markedName))
        (mkConst arenasName)
      value := mkAppN (mkConst ``LeanerIR.Validation.erasedExpressionArenas_eq_of_indexes)
        #[mkConst markedName, mkConst indexesName, mkConst arenasName,
          mkConst indexesEqName, mkConst indexedArenasEqName] })
    trace[leaner.preparation] "expression arenas certificate done: {← IO.getNumHeartbeats}"
    addPreparationDecl (.defnDecl {
      name, levelParams := []
      type := mkConst ``LeanerIR.Validation.ValidatedUnit
      value := ← quotePreparationUpdate markedName marked prepared
      hints := .abbrev, safety := .safe })
    trace[leaner.preparation] "erased quotation done: {← IO.getNumHeartbeats}"
    let preparedTerm ← mkAppM ``Prod.fst
      #[mkApp (mkConst ``LeanerIR.Validation.prepareSemantics)
        (mkConst unitDefinition)]
    let eraseEqName := Name.str name "erased_eq"
    let applyEqName := Name.str name "erasureApplied_eq"
    let applyArenasEqName := Name.str name "erasureArenasApplied_eq"
    addPreparationDecl (.thmDecl {
      name := applyArenasEqName, levelParams := []
      type := ← mkEq
        (mkAppN (mkConst ``LeanerIR.Validation.applySharedReferenceErasureArenas)
          #[mkConst markedName, mkConst planName, mkConst arenasName])
        (mkConst name)
      value := ← mkEqRefl (mkConst name) })
    addPreparationDecl (.thmDecl {
      name := applyEqName, levelParams := []
      type := ← mkEq
        (mkAppN (mkConst ``LeanerIR.Validation.applySharedReferenceErasure)
          #[mkConst markedName, mkConst planName])
        (mkConst name)
      value := mkAppN (mkConst ``LeanerIR.Validation.applySharedReferenceErasure_eq_of_arenas)
        #[mkConst markedName, mkConst name, mkConst planName, mkConst arenasName,
          mkConst arenasEqName, mkConst applyArenasEqName] })
    addPreparationDecl (.thmDecl {
      name := eraseEqName, levelParams := []
      type := ← mkEq
        (mkApp (mkConst ``LeanerIR.Validation.eraseSharedReferences) (mkConst markedName))
        (mkConst name)
      value := mkAppN (mkConst ``LeanerIR.Validation.eraseSharedReferences_eq_of_plan)
        #[mkConst markedName, mkConst name, mkConst planName,
          mkConst planEqName, mkConst applyEqName] })
    trace[leaner.preparation] "erased certificate done: {← IO.getNumHeartbeats}"
    addPreparationDecl (.thmDecl {
      name := eqName, levelParams := []
      type := ← mkEq preparedTerm (mkConst name)
      -- Supply the implicit units explicitly: inferring them from the
      -- certificates can reduce a preparation pass again during unification.
      value := mkAppN (mkConst ``LeanerIR.Validation.prepareSemantics_eq_of_stages)
        #[mkConst unitDefinition, mkConst markedName, mkConst name,
          mkConst markedEqName, mkConst eraseEqName] })
    let width := LeanerIR.Validation.targetPointerWidth? unit
    let widthTerm := mkApp (mkConst ``LeanerIR.Validation.targetPointerWidth?)
      (mkConst unitDefinition)
    addPreparationDecl (.thmDecl {
      name := widthEqName, levelParams := []
      type := ← mkEq widthTerm (toExpr width)
      value := ← mkEqRefl (toExpr width) })
  return (eqName, widthEqName)

/-- Materialize a registered unit as a Lean definition. -/
syntax (name := leanerUnitCommand) "#leaner_unit" leanerPath : command

@[command_elab leanerUnitCommand]
def elabLeanerUnit : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let segments := pathSegments pathSyntax
  let some unit := LeanerLang.registeredUnit? (← getEnv) (pathName segments)
    | throwErrorAt stx s!"unknown Leaner namespace `{pathName segments}`"
  -- A hand-written proof needs the same quoted preparation the generated
  -- script uses, so materializing the unit materializes it too, along with
  -- the typed twins contracts read storage through.
  discard <| ensureSemanticsDefinitions segments unit
  discard <| SpecTypes.ensureSpecTypes segments unit

/-- Name of the typed verification theorem over a function's native
signature. -/
def typedVerifiedName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedVerified"

/-- Turn verification-cost measurement on for the rest of this file.  Only
the benchmark file uses it, so an ordinary `verify` never pays for it. -/
elab "#leaner_measure" : command => do
  Perf.recorded.set #[]
  Perf.measuring.set true

/-- Compare the measured costs of this file with a baseline, or rewrite the
baseline when `UB=1` is set.  The comparison gates on the two reproducible
numbers and reports wall time beside them. -/
elab "#leaner_perf " path:str : command => do
  Perf.measuring.set false
  let samples ← Perf.recorded.get
  if samples.isEmpty then
    logError "no verification target was measured; \
      `#leaner_measure` must precede the modules"
    return
  let path := System.FilePath.mk path.getString
  let update := (← IO.getEnv "UB").isSome
  if update then
    IO.FS.writeFile path (Perf.baselineText samples)
    logInfo m!"verification-cost baseline updated:\n{Perf.baselineText samples}"
    return
  unless ← path.pathExists do
    logError m!"the verification-cost baseline {path} does not exist; \
      write it with `UB=1`"
    return
  let baseline := Perf.parseBaseline (← IO.FS.readFile path)
  let (text, ok) := Perf.report samples baseline 10
  if ok then logInfo m!"verification cost:\n{text}"
  else throwError m!"verification cost regressed:\n{text}"

end LeanerLang.Contract
