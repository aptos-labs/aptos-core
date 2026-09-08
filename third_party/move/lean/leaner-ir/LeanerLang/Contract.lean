-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Elab
import LeanerLang.Frame
import LeanerLang.Perf
import LeanerLang.Denotation
import LeanerLang.Options
import LeanerLang.Quote
import LeanerLang.Registry
import LeanerLang.SpecTypes
import LeanerLang.Syntax
import LeanerLang.Typed
import LeanerLang.Computation
import LeanerLang.NativeLoopInfo
import LeanerLang.NativeMutable
import LeanerIR.Proofs.Certify
import LeanerIR.Proofs.Represent
import LeanerIR.Proofs.Composition
import LeanerIR.Proofs.Computation
import LeanerIR.Proofs.NativeBoundary
import LeanerIR.Proofs.Meaning
import LeanerIR.Proofs.DenotationWP

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
  /-- Lean binder each local denotes under `spec.old`: the entry value of a
  mutable reference, and the ordinary binder for everything else. -/
  oldLocals : Array (Option Lean.Expr) := #[]
  /-- Lean binder for each function result. -/
  results : Array Lean.Expr
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
@[simp, lir_data_norm] theorem runtimeFieldNominal
    (source : LeanerIR.StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) (index : Nat) :
    RuntimeValue.field (.nominal source variant fields) index =
      fields[index]?.getD .unit := rfl

/- Enum invariant matches immediately project constructor payloads.  Keep
these literal-array rows ahead of the generic nominal row so arithmetic sees
the payload itself instead of an intermediate `getElem?` application. -/
@[simp, lir_data_norm high] theorem runtimeFieldNominalZero
    (source : LeanerIR.StructHandle) (variant : Option String)
    (first : RuntimeValue) (rest : List RuntimeValue) :
    RuntimeValue.field (.nominal source variant (Array.mk (first :: rest))) 0 =
      first := rfl

@[simp, lir_data_norm high] theorem runtimeFieldNominalOne
    (source : LeanerIR.StructHandle) (variant : Option String)
    (first second : RuntimeValue) (rest : List RuntimeValue) :
    RuntimeValue.field
        (.nominal source variant (Array.mk (first :: second :: rest))) 1 =
      second := rfl

@[simp, lir_data_norm] theorem runtimeAsIntInteger (value : Int) :
    RuntimeValue.asInt (.integer value) = value := rfl

@[simp, lir_data_norm] theorem runtimeAsBoolBool (value : Bool) :
    RuntimeValue.asBool (.bool value) = value := rfl

@[simp, lir_data_norm] theorem runtimeAsStringString (value : String) :
    RuntimeValue.asString (.string value) = value := rfl

@[simp, lir_data_norm] theorem runtimeAsStringAddress (value : String) :
    RuntimeValue.asString (.address value) = value := rfl

@[simp, lir_data_norm] theorem runtimeAsStringSigner (value : String) :
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
@[simp, lir_data_norm] def pushVector (value element : RuntimeValue) : RuntimeValue :=
  match value with
  | .vector elements => .vector (elements.push element)
  | _ => .unit

/-- Total logical concatenation corresponding to the value-level primitive. -/
@[simp, lir_data_norm] def concatVector (left right : RuntimeValue) : RuntimeValue :=
  match left, right with
  | .vector left, .vector right => .vector (left ++ right)
  | _, _ => .unit

/-- Total logical vector length used by generated clauses.  Ill-shaped
specification operands denote the same default integer value as the other
total runtime projections. -/
def lengthVector (value : RuntimeValue) : Int :=
  match value with
  | .vector elements => Int.ofNat elements.size
  | _ => 0

@[simp, lir_data_norm, lir_spec_norm] theorem lengthVector_vector (elements : Array RuntimeValue) :
    lengthVector (.vector elements) = Int.ofNat elements.size := rfl

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

/-- Keep preparation outside the large generated-theorem quotation. -/
macro "leaner_prepare_normalization" : tactic =>
  `(tactic| (leaner_subst_decided; leaner_normalize_vector_lengths))

/-- Expose the data-domain certificate of a native aggregate once, before
walking its fields. Only ownership hypotheses are simplified here. -/
elab "leaner_prepare_plain_data" " [" facts:Lean.Parser.Tactic.simpLemma,* "]" : tactic => do
  Lean.Elab.Tactic.withMainContext do
    let mut hypotheses := #[]
    for declaration in ← Lean.getLCtx do
      if declaration.isImplementationDetail then continue
      if declaration.type.isAppOfArity ``LeanerIR.SemanticOperations.Plain 1 then
        hypotheses := hypotheses.push (mkIdent declaration.userName)
    for hypothesis in hypotheses do
      Lean.Elab.Tactic.evalTactic (← `(tactic|
        simp (config := { failIfUnchanged := false }) [$facts,*, leaner_plain] at $hypothesis:ident))
      Lean.Elab.Tactic.evalTactic (← `(tactic| try leaner_cases $hypothesis:ident))

attribute [lir_data_norm] List.size_toArray List.length_cons List.length_nil
  Array.size_empty Array.size_push Array.size_singleton

attribute [lir_spec_norm] List.size_toArray List.length_cons List.length_nil
  Array.size_empty Array.size_push Array.size_singleton Int.ofNat_eq_natCast

/-- Closed list lookup for specification-side enum descriptors.  Using the
literal list directly avoids elaborating `Array.find?` into a `forIn` state
machine in proof obligations. -/
@[simp, lir_data_norm] def variantIndex (variant : String) :
    List (String × Nat) → Option Nat
  | [] => none
  | (candidate, index) :: rest =>
      if candidate == variant then some index else variantIndex variant rest

@[simp, lir_data_norm, lir_spec_norm] def variantMember (variant : String) : List String → Bool
  | [] => false
  | candidate :: rest =>
      if candidate == variant then true else variantMember variant rest

/-- Total enum-payload selection used by specifications after the declaration
resolver has reduced a source field name to one payload offset per variant. -/
@[simp, lir_data_norm] def selectVariantField (value : RuntimeValue)
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
@[simp, lir_data_norm] def testVariants (value : RuntimeValue)
    (owner : LeanerIR.StructHandle)
    (variants : Array String) : Bool :=
  match value with
  | .nominal actual (some variant) _ =>
      actual == owner && variantMember variant variants.toList
  | _ => false

@[lir_spec_norm] theorem testVariants_nominal_self (owner : LeanerIR.StructHandle)
    (variant : String) (fields : Array RuntimeValue) (variants : Array String) :
    testVariants (.nominal owner (some variant) fields) owner variants =
      variantMember variant variants.toList := by
  simp [testVariants]

private def typeOfExpr? (context : Context) (id : ExprId) : Option IrTy := do
  let expression ← context.ns.expressions[id.index]?
  context.unit.tables.types[expression.typeId.index]?

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
      if let some rep := Typed.valueRep? context.unit context.twins
          expression.typeId context.ns.profile then
        match rep with
        | .parameter _ | .twin _ _ | .vector _ _ | .tuple _ =>
            return ← rep.encode context.codecs translated
        | _ => pure ()
      return translated

/-- Translate one specification expression into a Lean term. -/
private partial def translate (context : Context) (id : ExprId) : MetaM Lean.Expr := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "specification expression {id.index} is out of range"
  let some ty := context.unit.tables.types[expression.typeId.index]?
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
          let some patternType := active.unit.tables.types[pattern.typeId.index]?
            | throwError "quantifier pattern type {pattern.typeId.index} is out of range"
          let some declaredDomain := active.unit.tables.types[domainType.typeId.index]?
            | throwError "quantifier domain type {domainType.typeId.index} is out of range"
          unless patternType == declaredDomain do
            throwError "quantifier pattern and type domain differ"
          withLocalDeclD (Name.mkSimple s!"quantified_{localId.index}")
              (domainOf patternType).leanType fun value => do
            let inner ← bind (index + 1) { active with
              locals := active.locals.set! localId.index (some value)
              oldLocals := active.oldLocals.set! localId.index (some value) }
            match kind with
            | .forall => mkForallFVars #[value] inner
            | .exists => mkAppM ``Exists #[← mkLambdaFVars #[value] inner]
            | _ => throwError "choice quantifiers are not supported in generated contracts"
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
                    let some childType := context.unit.tables.types[child.typeId.index]?
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
      -- A conditional in a clause is Lean's `ite` on the decided test.
      let test ← translate context condition
      let thenTerm ← translate context thenBranch
      let elseTerm ← translate context elseBranch
      let thenTerm ← if domainOf ty == .aggregate then
          runtimeAggregateValue context thenBranch thenTerm else pure thenTerm
      let elseTerm ← if domainOf ty == .aggregate then
          runtimeAggregateValue context elseBranch elseTerm else pure elseTerm
      mkAppOptM ``ite #[none, test, none, thenTerm, elseTerm]
  | kind =>
      throwError "specification node {repr kind} is not supported in generated contracts"
where
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
    let some ty := context.unit.tables.types[expression.typeId.index]?
      | throwError "a specification operand has an unknown type"
    let translated ← translate context id
    -- Constructors and total field projections already produce runtime
    -- aggregates. Native generic binders need encoding, but encoding an
    -- already translated constructor again is a representation mismatch.
    if (← inferType translated).isConstOf ``RuntimeValue then return translated
    if let some rep := Typed.valueRep? context.unit context.twins
        expression.typeId context.ns.profile then
      match rep with
      | .parameter _ | .twin _ _ | .vector _ _ | .tuple _ =>
          return ← rep.encode context.codecs translated
      | _ => pure ()
    match domainOf ty with
    | .boolean => mkAppM ``RuntimeValue.bool #[← mkAppM ``Decidable.decide #[translated]]
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
  translateOperation (operation : Operation)
      (instantiations : Array LeanerIR.GenericArgument) (arguments : Array ExprId)
      (ty : IrTy) : MetaM Lean.Expr := do
    match operation with
    | .specification (.functionCall reference _) =>
        if context.specCallStack.contains reference then
          throwError "recursive specification function `{repr reference}` is not supported in generated contracts"
        unless instantiations.isEmpty do
          throwError "generic specification function calls are not supported in generated contracts"
        let some qualified := context.unit.tables.names[reference.name.index]?
          | throwError "specification function call has an unknown name"
        unless qualified.namespaceId == reference.namespaceId do
          throwError "specification function call name belongs to a different namespace"
        let some targetNs := context.unit.namespaces[reference.namespaceId.index]?
          | throwError "specification function call namespace is out of range"
        let some functionId := context.unit.resolution.specFunction? reference.name
          | throwError "specification function call target does not resolve"
        let some declaration := targetNs.specFunctions[functionId.index]?
          | throwError "specification function declaration is out of range"
        let some body := declaration.body
          | throwError "an opaque specification function cannot be expanded in a generated contract"
        unless arguments.size == declaration.signature.parameters.size do
          throwError "specification function call argument count differs from its declaration"
        unless declaration.signature.results.size == 1 do
          throwError "generated contracts currently expand specification functions with one result"
        let mut targetLocals : Array (Option Lean.Expr) :=
          Array.replicate declaration.locals.size none
        let mut targetTypes : Array IrTy := #[]
        for localDecl in declaration.locals do
          let some localType := context.unit.tables.types[localDecl.type.typeId.index]?
            | throwError "specification function local has an unknown type"
          targetTypes := targetTypes.push localType
        for (argument, index) in arguments.zipIdx do
          let some parameter := declaration.signature.parameters[index]?
            | throwError "specification function parameter is out of range"
          let some parameterType := context.unit.tables.types[parameter.typeUse.typeId.index]?
            | throwError "specification function parameter has an unknown type"
          let value ← translate context argument
          let value ← match domainOf parameterType with
            | .boolean => mkAppM ``Decidable.decide #[value]
            | _ => pure value
          targetLocals := targetLocals.set! index (some value)
        translate {
          context with
          namespaceId := reference.namespaceId
          ns := targetNs
          locals := targetLocals
          localTypes := targetTypes
          oldLocals := targetLocals
          results := #[]
          resultTypes := #[]
          specCallStack := context.specCallStack.push reference } body
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

/-- The inclusive bounds a physical integer type imposes on its logical value,
as an explicit conjunction over the binder. -/
private def rangeConstraint? (ty : IrTy) (binder : Lean.Expr) : MetaM (Option Lean.Expr) := do
  match ty.integerBounds? with
  | some (lower, upper) =>
      let lowerBound ← mkAppM ``LE.le #[toExpr lower, binder]
      let upperBound ← mkAppM ``LE.le #[binder, toExpr upper]
      return some (← mkAppM ``And #[lowerBound, upperBound])
  | none => return none

/-- The runtime encoding of one logical binder at a physical type. -/
private def encodeValue? (ty : IrTy) (binder : Lean.Expr) : MetaM (Option Lean.Expr) :=
  return some (← (domainOf ty).encode binder)

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
  /-- Move type arguments denote data, not runtime reference/loan wrappers.
  Raw generic contracts state this domain restriction explicitly. -/
  plainData : Bool := false
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
  let plainData := context.ns.profile == some .move && rep.isSome
  return { name, kind, physical, leanType, rep, codec, components, plainData }

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

/-- The logical binders of one slot. A plain slot — and a shared
reference, which is the observed value itself — has one value; a mutable
reference adds its loan instance and, on the ensures side, the exit value
— the loan's value at its death. The bare parameter name denotes the
post-state value there (`spec.old` reaches the entry), so the exit binder
carries the parameter's name and the entry binder is suffixed. -/
private structure SlotBinders where
  loan : Option Lean.Expr := none
  entry : Lean.Expr
  exit : Option Lean.Expr := none
  /-- Raw value exported for a mutable parameter before returned-reference
  prophecies are resolved.  Keeping this separate from `exit` is what lets
  one contract describe dynamic selection and projected/multiple results. -/
  pending : Option Lean.Expr := none
  /-- The loan and value binders of a tuple result's reference components,
  in component order. -/
  componentBinders : Array (Lean.Expr × Lean.Expr) := #[]

/-- The binders of one slot in introduction order. -/
private def SlotBinders.flat (binders : SlotBinders) : Array Lean.Expr :=
  binders.loan.toArray ++ #[binders.entry] ++ binders.exit.toArray ++
    binders.pending.toArray ++
    binders.componentBinders.flatMap fun (loan, value) => #[loan, value]

/-- The binder a specification local denotes in the current state. -/
private def SlotBinders.current (binders : SlotBinders) : Lean.Expr :=
  binders.exit.getD binders.entry

/-- Bind the logical binders of every slot and continue. -/
private def withSlotBinders (slots : Array Slot) (withExit : Bool)
    (k : Array SlotBinders → MetaM α) : MetaM α :=
  let rec go (index : Nat) (bound : Array SlotBinders) : MetaM α := do
    if h : index < slots.size then
      let slot := slots[index]
      match slot.kind with
      | .plain =>
          withLocalDeclD slot.name slot.leanType fun entry =>
            /- A tuple of returned references: its components' loans and
            values, so the transfer of each lender's prophecy can be
            stated.  Bound in continuation-passing order over the
            components, ending in the next slot. -/
            (slot.components.foldr
              (fun component continue_ bound' =>
                withLocalDeclD (component.name.appendAfter "_loan") (mkConst ``Nat) fun loan =>
                  withLocalDeclD component.name component.leanType fun value =>
                    continue_ (bound'.push (loan, value)))
              (fun bound' => go (index + 1) (bound.push { entry, componentBinders := bound' })))
              #[]
      | .sharedRef =>
          -- A shared reference is the observed value itself; no loan binder.
          withLocalDeclD slot.name slot.leanType fun entry =>
            go (index + 1) (bound.push { entry })
      | .mutableRef =>
          withLocalDeclD (slot.name.appendAfter "_loan") (mkConst ``Nat) fun loan =>
            if withExit then
              withLocalDeclD (slot.name.appendAfter "_entry") slot.leanType fun entry =>
                withLocalDeclD slot.name slot.leanType fun exit =>
                  withLocalDeclD (slot.name.appendAfter "_pending")
                      (mkConst ``LeanerIR.RuntimeValue) fun pending =>
                    go (index + 1) (bound.push {
                      loan := some loan, entry, exit := some exit,
                      pending := some pending })
            else
              withLocalDeclD slot.name slot.leanType fun entry =>
                go (index + 1) (bound.push { loan := some loan, entry })
    else k bound
  go 0 #[]

/-- The runtime argument a slot's binders encode. -/
private def slotRuntimeValue (slot : Slot) (binders : SlotBinders) :
    MetaM Lean.Expr := do
  let encoded ← match slot.rep with
    | some _ =>
        let some codec := slot.codec
          | throwError "a native generic slot has no codec"
        mkAppM ``LeanerIR.Proofs.Codec.encode #[codec, binders.entry]
    | none =>
        let some encoded ← encodeValue? slot.physical binders.entry
          | throwError "a value of type {describeType slot.physical} has no runtime \
              encoding in generated contracts"
        pure encoded
  match slot.kind, binders.loan with
  | .plain, _ | .sharedRef, _ => return encoded
  | .mutableRef, some loan =>
      mkAppM ``LeanerIR.RuntimeValue.borrow #[loan, encoded]
  | .mutableRef, none => throwError "internal: a mutable slot has no loan binder"

/-- The row equation tying a runtime value row to its logical binders. -/
private def rowEquation (row : Lean.Expr) (slots : Array Slot)
    (bound : Array SlotBinders) : MetaM Lean.Expr := do
  let encoded ← (slots.zip bound).mapM fun (slot, binders) =>
    slotRuntimeValue slot binders
  let literal ← mkArrayLit (mkConst ``LeanerIR.RuntimeValue) encoded.toList
  mkAppM ``Eq #[row, literal]

/-- Range constraints of the given binders, as separate conjuncts. -/
private def slotRanges (slots : Array Slot)
    (binderOf : SlotBinders → Option Lean.Expr) (bound : Array SlotBinders) :
    MetaM (Array Lean.Expr) := do
  let mut parts : Array Lean.Expr := #[]
  for (slot, binders) in slots.zip bound do
    if let some binder := binderOf binders then
      if let some constraint ← rangeConstraint? slot.physical binder then
        parts := parts.push constraint
  return parts

/-- Runtime ownership preconditions.  Future dynamic loan identifiers are
absent from the global registry, which is the reachable-state invariant that
routes locally minted loans back to caller locations.  Mutable parameters'
holes likewise live in a caller frame rather than a global slot, and distinct
mutable parameters carry distinct loan identities.  `SatisfiesFunction`
ranges over raw runtime rows and states these source-level ownership facts
explicitly. -/
private def mutableLoanFacts (state : Lean.Expr) (slots : Array Slot)
    (bound : Array SlotBinders) : MetaM (Array Lean.Expr) := do
  let fresh ← mkAppM ``LeanerIR.SemanticOperations.FreshGlobalLoanIds #[state]
  let mut parts : Array Lean.Expr := #[fresh]
  for (slot, binders) in slots.zip bound do
    if slot.plainData then
      let some codec := slot.codec
        | throwError "a generic data argument has no codec"
      let encoded ← mkAppM ``LeanerIR.Proofs.Codec.encode #[codec, binders.entry]
      parts := parts.push (← mkAppM ``LeanerIR.SemanticOperations.Plain #[encoded])
  let mutableSlots := (slots.zip bound).filter fun (slot, _) =>
    slot.kind == .mutableRef
  for (_, binders) in mutableSlots do
    let some loan := binders.loan
      | throwError "internal: a mutable slot has no loan binder"
    let nextLoan ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[state]
    parts := parts.push (← mkAppM ``LT.lt #[loan, nextLoan])
    let lookup ← mkAppM ``LeanerIR.SemanticOperations.globalLoanKey?
      #[state, loan]
    parts := parts.push (← mkAppM ``Eq
      #[lookup, ← mkAppOptM ``Option.none #[mkConst ``LeanerIR.GlobalKey]])
  for (left, index) in mutableSlots.zipIdx do
    let some leftLoan := left.2.loan
      | throwError "internal: a mutable slot has no loan binder"
    for right in mutableSlots.extract (index + 1) mutableSlots.size do
      let some rightLoan := right.2.loan
        | throwError "internal: a mutable slot has no loan binder"
      parts := parts.push (← mkAppM ``Ne #[leftLoan, rightLoan])
  return parts

/-- One carried loan: the parameter lending it, and the result component
that carries it — `none` when the result is the reborrow itself.  This is
the prophecy transfer a modular caller must be told — the lender's export
*is* the returned loan's hole — which the `resolveReturnedBorrows` clause
alone leaves open (a callee returning a fresh reference satisfies it
too). -/
private structure LenderFocus where
  twin : SpecTypes.TwinInfo
  field : Nat

private structure Lender where
  parameter : Nat
  component : Option Nat
  /-- The field of the parameter's struct the reborrow projects, when it
  is not the whole parameter. -/
  focus : Option LenderFocus := none

/-- The carried loans of a reference-returning body, each with its lender.
A carried loan records no death: every other mutable loan dies at a known
point, and immutable loans are not death-marked at all.  The result is
either one whole reborrow of a parameter, or a tuple whose components are
such reborrows, or one reborrow of a field of a parameter's struct; a
result reached through a global or a dynamic selection keeps the value
form. -/
private def reborrowLenders (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (twins : Array SpecTypes.TwinInfo)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array Lender := Id.run do
  let certificate? : Option LeanerIR.Validation.BorrowCertificate := do
    guard (declaration.signature.results.size == 1)
    let functionIndex ← ns.functions.findIdx? fun function => function.name == declaration.name
    unit.borrowCertificates.find? fun certificate =>
      certificate.namespaceId == namespaceId &&
        certificate.functionId.index == functionIndex
  let some certificate := certificate? | return #[]
  let parameterOf (placeId : LeanerIR.PlaceId) : Option Nat := do
    let LeanerIR.Place.deref base ← ns.places[placeId.index]? | none
    let LeanerIR.Place.localVar lender ← ns.places[base.index]? | none
    guard (lender.index < declaration.signature.parameters.size)
    pure lender.index
  let lenderOf (expressionId : LeanerIR.ExprId) : Option Nat := do
    let expression ← ns.expressions[expressionId.index]?
    let LeanerIR.ExprKind.operation (.borrow .mutable place) _ _ _ := expression.kind
      | none
    parameterOf place
  /- A reborrow of one field of a parameter's struct: the lender and the
  focus, by the twin's field row. -/
  let projectedLenderOf (expressionId : LeanerIR.ExprId) : Option (Nat × LenderFocus) := do
    let expression ← ns.expressions[expressionId.index]?
    let LeanerIR.ExprKind.operation (.borrow .mutable place) _ _ _ := expression.kind
      | none
    let LeanerIR.Place.field base owner fieldName ← ns.places[place.index]? | none
    let parameter ← parameterOf base
    let qualified ← unit.tables.names[owner.name.index]?
    let twin ← twins.find? (·.qualified == qualified)
    let fieldQualified ← unit.tables.names[fieldName.index]?
    let field ← twin.fields.findIdx? (·.1 == fieldQualified.name)
    pure (parameter, { twin, field })
  let carried := certificate.loans.filter (·.deaths.isEmpty)
  let some root := (match declaration.body with
      | .structured root => some root
      | .absent => none)
    | return #[]
  let some rootExpression := ns.expressions[root.index]? | return #[]
  match rootExpression.kind with
  | .operation (.primitive .tuple) _ arguments _ =>
      /- Each component that is a carried whole reborrow of a parameter. -/
      let mut lenders : Array Lender := #[]
      for (argument, index) in arguments.zipIdx do
        if carried.any (·.expression == argument) then
          if let some parameter := lenderOf argument then
            lenders := lenders.push { parameter, component := some index }
      /- Only when every component is such a reborrow does the summary name
      the transfer; otherwise it keeps the value form throughout. -/
      if lenders.size == arguments.size then lenders else #[]
  | _ =>
      /- One carried loan, wherever the body mints it — the reborrow may be
      the argument of a forwarding call — is the whole result. -/
      match carried with
      | #[loan] =>
          match lenderOf loan.expression with
          | some parameter => #[{ parameter, component := none }]
          | none =>
              match projectedLenderOf loan.expression with
              | some (parameter, focus) => #[{ parameter, component := none, focus }]
              | none => #[]
      | _ => #[]

/-- The expressions of one body, from its root. -/
private partial def bodyExpressions (ns : ValidatedNamespace) (root : ExprId)
    (seen : Array Nat := #[]) : Array LeanerIR.Expr := Id.run do
  if seen.contains root.index then return #[]
  let some expression := ns.expressions[root.index]? | return #[]
  let seen := seen.push root.index
  let mut found := #[expression]
  for child in LeanerIR.Validation.expressionChildren expression.kind do
    found := found ++ bodyExpressions ns child seen
  return found

/-- A returned reborrow projected from a global borrow: the family borrowed,
the parameter holding its key, and the focus inside the resource. -/
private structure GlobalLender where
  family : SpecTypes.FamilyInfo
  keyParameter : Nat
  focus : LenderFocus

/-- The global lender of a reference-returning body: its one carried loan
is a reborrow of a field of a local that a `let` bound to a mutable global
borrow keyed by a parameter. -/
private def globalReborrowLender? (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (twins : Array SpecTypes.TwinInfo)
    (families : Array SpecTypes.FamilyInfo)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Option GlobalLender := do
  guard (declaration.signature.results.size == 1)
  let functionIndex ← ns.functions.findIdx? fun function => function.name == declaration.name
  let certificate ← unit.borrowCertificates.find? fun certificate =>
    certificate.namespaceId == namespaceId &&
      certificate.functionId.index == functionIndex
  /- The outer global borrow carries no recorded death either — the frame
  exit settles it — so two loans are carried: the resource's, and the
  returned field reborrow's. -/
  let carried := certificate.loans.filter (·.deaths.isEmpty)
  let projected? (loan : LeanerIR.Validation.CheckedLoanFact) :
      Option (LeanerIR.PlaceId × LeanerIR.QualifiedRef × LeanerIR.NameId) := do
    let expression ← ns.expressions[loan.expression.index]?
    let LeanerIR.ExprKind.operation (.borrow .mutable place) _ _ _ := expression.kind
      | none
    let LeanerIR.Place.field base owner fieldName ← ns.places[place.index]? | none
    pure (base, owner, fieldName)
  let #[loan] := carried.filter fun loan => (projected? loan).isSome | none
  let (base, owner, fieldName) ← projected? loan
  let LeanerIR.Place.deref holderPlace ← ns.places[base.index]? | none
  let LeanerIR.Place.localVar holder ← ns.places[holderPlace.index]? | none
  guard (holder.index ≥ declaration.signature.parameters.size)
  /- The holder's `let` is searched in this body alone: local indices are
  per function, and the namespace's expression table holds every body. -/
  let LeanerIR.Validation.FunctionBody.structured root := declaration.body | none
  let initializer ← (bodyExpressions ns root).findSome? fun candidate => do
    let LeanerIR.ExprKind.letDecl pattern (some initializer) _ := candidate.kind | none
    let bound ← ns.patterns[pattern.index]?
    let LeanerIR.PatternKind.variable local_ := bound.kind | none
    guard (local_ == holder)
    ns.expressions[initializer.index]?
  let LeanerIR.ExprKind.operation (.global (.borrow .mutable)) instantiations arguments _ :=
      initializer.kind
    | none
  /- Every other carried loan is the resource's own borrow. -/
  guard (carried.all fun other =>
    other.expression == loan.expression ||
      (ns.expressions[other.expression.index]?.map (·.kind) == some initializer.kind))
  let [.typeArg resource] := instantiations.toList | none
  let family ← families.find? (·.typeIndex == resource.typeId.index)
  let keyId ← arguments[0]?
  let keyExpression ← ns.expressions[keyId.index]?
  let LeanerIR.ExprKind.localVar keyLocal := keyExpression.kind | none
  guard (keyLocal.index < declaration.signature.parameters.size)
  let qualified ← unit.tables.names[owner.name.index]?
  let twin ← twins.find? (·.qualified == qualified)
  let fieldQualified ← unit.tables.names[fieldName.index]?
  let field ← twin.fields.findIdx? (·.1 == fieldQualified.name)
  pure { family, keyParameter := keyLocal.index, focus := { twin, field } }

/-- The equation spelling a tuple result from its components' loans and
values. -/
private def componentEquations (slots : Array Slot) (bound : Array SlotBinders) :
    MetaM (Array Lean.Expr) := do
  let mut parts : Array Lean.Expr := #[]
  for (slot, binders) in slots.zip bound do
    if slot.components.isEmpty then continue
    let encoded ← (slot.components.zip binders.componentBinders).mapM
      fun (component, (loan, value)) => do
        let some encodedValue ← encodeValue? component.physical value
          | throwError "a value of type {describeType component.physical} has no \
              runtime encoding in generated contracts"
        mkAppM ``LeanerIR.RuntimeValue.borrow #[loan, encodedValue]
    let literal ← mkArrayLit (mkConst ``LeanerIR.RuntimeValue) encoded.toList
    parts := parts.push
      (← mkAppM ``Eq #[binders.entry, ← mkAppM ``LeanerIR.RuntimeValue.tuple #[literal]])
  return parts

/-- Existentially close a body over the logical slots. -/
private def existsOver (binders : Array Lean.Expr) (body : Lean.Expr) :
    MetaM Lean.Expr := do
  binders.foldrM (fun binder accumulated => do
    mkAppM ``Exists #[← mkLambdaFVars #[binder] accumulated]) body

/-- The scalar domain a twin field is spelled in, if it has one, with the
LIR type its binder ranges over. -/
private def fieldDomain? : SpecTypes.FieldRep → Option (Domain × IrTy)
  | .int width signed => some (.integer, .integer width signed)
  | .bool => some (.boolean, .bool)
  | .string => some (.text ``RuntimeValue.string, .string)
  | .address => some (.text ``RuntimeValue.address, .address)
  | .signer => some (.text ``RuntimeValue.signer, .signer)
  | _ => none

/-- The transfer of a projected lender: its export is its struct with the
returned loan's hole at the focus, the siblings named existentially.  It
is spelled as the focus of the hole among the siblings — the spelling the
laws and the resolution rows share.  A struct with a field outside the
scalar domains states no transfer. -/
private def focusedTransfer? (replacement hole : Lean.Expr) (focus : LenderFocus) :
    MetaM (Option Lean.Expr) := do
  let some domains := focus.twin.fields.mapM fun (_, rep) => fieldDomain? rep
    | return none
  let rec bind (index : Nat) (siblings : Array Lean.Expr) : MetaM Lean.Expr := do
    if h : index < domains.size then
      if index == focus.field then
        bind (index + 1) siblings
      else
        let (domain, ty) := domains[index]
        withLocalDeclD (Name.mkSimple s!"sibling_{index}") domain.leanType
          fun sibling => do
            let body ← bind (index + 1) (siblings.push (← domain.encode sibling))
            /- A sibling's range travels with it: a caller rebuilding the
            typed resource around its write needs the certificate. -/
            let body ← match ← rangeConstraint? ty sibling with
              | some range => mkAppM ``And #[body, range]
              | none => pure body
            existsOver #[sibling] body
    else
      let handle ← mkAppM ``LeanerIR.StructHandle.mk
        #[← mkAppM ``LeanerIR.NamespaceId.mk #[toExpr focus.twin.namespaceIndex],
          toExpr focus.twin.structIndex]
      let runtimeValue := mkConst ``LeanerIR.RuntimeValue
      let before ← mkArrayLit runtimeValue (siblings.extract 0 focus.field).toList
      let after ← mkArrayLit runtimeValue (siblings.extract focus.field siblings.size).toList
      let step ← mkAppM ``LeanerIR.SemanticOperations.FocusStep.mk
        #[handle, before, after, toExpr (none : Option String)]
      let steps ← mkListLit (mkConst ``LeanerIR.SemanticOperations.FocusStep) [step]
      let focused ← mkAppM ``LeanerIR.SemanticOperations.focusValue #[steps, hole]
      mkAppM ``Eq #[replacement, focused]
  return some (← bind 0 #[])
  termination_by domains.size - index

/-- The pending-export equation of mutable parameters.  The frame exports one
raw replacement per parameter loan.  Returned mutable references may leave
their prophecy holes anywhere inside those replacements: dynamic selection,
multiple results, and projected owners are all described by resolving the
returned row's `(loan,current)` pairs into each replacement.  This relation
contains values and loan identities only—never an owner root or path. -/
private def pendingEquation? (initial final : Lean.Expr)
    (parameterSlots : Array Slot) (parameterBound : Array SlotBinders)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders)
    (lenders : Array Lender) :
    MetaM (Option Lean.Expr) := do
  let mut pending ← mkAppM ``LeanerIR.RuntimeState.pending #[initial]
  let mut exported := false
  let encodedResults ← (resultSlots.zip resultBound).mapM fun (slot, binders) =>
    slotRuntimeValue slot binders
  let resultRow ← mkArrayLit (mkConst ``LeanerIR.RuntimeValue)
    encodedResults.toList
  let mut transferRelations : Array Lean.Expr := #[]
  let mut transferredLoans : Array Lean.Expr := #[]
  for ((slot, binders), index) in (parameterSlots.zip parameterBound).zipIdx do
    if slot.kind == .mutableRef then
      let some loan := binders.loan
        | throwError "internal: a mutable slot has no loan binder"
      let some replacement := binders.pending
        | throwError "internal: a mutable slot has no pending replacement binder"
      let some exit := binders.exit
        | throwError "internal: a mutable parameter has no exit binder"
      let some encodedExit ← encodeValue? slot.physical exit
        | throwError "a value of type {describeType slot.physical} has no \
            runtime encoding in generated contracts"
      let resolved ← mkAppM
        ``LeanerIR.SemanticOperations.resolveReturnedBorrows
        #[resultRow, replacement]
      transferRelations := transferRelations.push
        (← mkAppM ``Eq #[encodedExit, resolved])
      if let some lender := lenders.find? (·.parameter == index) then
        let some resultLoan := (do
            let binders ← resultBound[0]?
            match lender.component with
            | none => binders.loan
            | some component => binders.componentBinders[component]?.map (·.1))
          | throwError "internal: a reference result has no loan binder"
        /- The transfer, and the returned loan's freshness: minted inside
        the callee, so after the entry's and before the exit's next loan.
        A caller settling the returned loan into its own frame needs both. -/
        let hole ← mkAppM ``LeanerIR.RuntimeValue.loanHole #[resultLoan]
        let transfer ← match lender.focus with
          | none => mkAppM ``Eq #[replacement, hole]
          | some focus => do
              let some transfer ← focusedTransfer? replacement hole focus
                | throwError "internal: a projected lender's struct has a field \
                    without a scalar domain"
              pure transfer
        transferRelations := transferRelations.push transfer
        transferRelations := transferRelations.push
          (← mkAppM ``LE.le #[← mkAppM ``LeanerIR.RuntimeState.nextLoan #[initial], resultLoan])
        transferRelations := transferRelations.push
          (← mkAppM ``LT.lt #[resultLoan, ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[final]])
        transferredLoans := transferredLoans.push resultLoan
      pending ← mkAppM ``Array.push
        #[pending, ← mkAppM ``Prod.mk #[loan, replacement]]
      exported := true
  /- Distinct carried loans are distinct: a caller settling two returned
  loans into two parameters needs their separation. -/
  for (first, i) in transferredLoans.zipIdx do
    for (second, j) in transferredLoans.zipIdx do
      if i < j then
        transferRelations := transferRelations.push (← mkAppM ``Ne #[first, second])
  /- A function without a reference parameter exports no write-back at
  all; saying so is what lets a caller consume its summary without
  re-executing it. -/
  if !exported then
    return some (← mkAppM ``Eq
      #[← mkAppM ``LeanerIR.RuntimeState.pending #[final],
        ← mkAppM ``LeanerIR.RuntimeState.pending #[initial]])
  let pendingEquality ← mkAppM ``Eq
    #[← mkAppM ``LeanerIR.RuntimeState.pending #[final], pending]
  return some (← conjunction (#[pendingEquality] ++ transferRelations))

/-! ## Native loop invariants -/

/-- Find source-normalized `spec invariant; loop` pairs.  The expression id
identifies the generated predicate over the loop's typed header product. -/
private partial def loopSpecifications (ns : ValidatedNamespace)
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

/-- Locals lexically available at a loop header. Body-local slots exist in
the runtime row but are not initialized on the first visit. -/
private partial def loopHeaderLocals (ns : ValidatedNamespace) (root target : ExprId)
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

/-- Authored invariants over a heterogeneous product of native header locals.
Fixed-width integers carry their ranges in their types. Unchanged locals are
tied to their entry values; old parameters select the original native argument.
No frame, decoder, runtime-value binder, or slot-shape existential is generated. -/
private def buildNativeLoopInvariants (unit : ValidatedUnit)
    (namespaceId : LeanerIR.NamespaceId) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (twins : Array SpecTypes.TwinInfo) (artifacts : Typed.Artifacts) :
    MetaM (Array (NativeLoopInfo.Loop × Lean.Expr)) := do
  let .structured root := declaration.body | return #[]
  let specifications := loopSpecifications ns root
  if specifications.isEmpty then return #[]
  checkLoopAnnotations ns root specifications
  unless artifacts.signature.typeParameterCount == 0 do
    throwError "native loop invariants require a monomorphic typed header"
  let logical (rep : Typed.ValueRep) (value : Lean.Expr) : MetaM Lean.Expr := do
    match rep with
    | .int .. => mkAppM ``LeanerIR.SpecInt.val #[value]
    | .bool => pure value
    | _ => throwError "native loop invariants require integer or Boolean header values"
  let productType (reps : Array Typed.ValueRep) : MetaM Lean.Expr := do
    let mut result := mkConst ``Unit
    for rep in reps.reverse do
      result := mkApp2 (mkConst ``Prod [Level.zero, Level.zero]) (← rep.leanType none) result
    return result
  let projections (value : Lean.Expr) (count : Nat) : MetaM (Array Lean.Expr) := do
    let mut rest := value
    let mut fields := #[]
    for _ in [:count] do
      fields := fields.push (← mkAppM ``Prod.fst #[rest])
      rest ← mkAppM ``Prod.snd #[rest]
    return fields
  let localTypes := declaration.locals.map fun decl =>
    unit.tables.types[decl.type.typeId.index]!
  let mut predicates := #[]
  for (site, block) in specifications do
    let available := (loopHeaderLocals ns root site
      (declaration.locals.extract 0 declaration.signature.parameters.size |>.map (·.id))).getD #[]
    let declarations := declaration.locals.filter (fun decl => available.contains decl.id)
    let reps ← declarations.mapM fun decl => do
      let slot := artifacts.signature.locals[decl.id.index]!
      unless slot.kind == .plain do
        throwError "native loop invariants do not yet carry reference headers"
      match slot.rep with
      | .int (.bits _) _ | .bool => pure slot.rep
      | _ => throwError "native loop invariants require integer or Boolean header values"
    let headerType ← productType reps
    let predicate ← withLocalDeclD `args (mkConst artifacts.argumentsType) fun args =>
      withLocalDeclD `loopEntry headerType fun initial =>
      withLocalDeclD `loopLocals headerType fun current =>
      withLocalDeclD `loopState (mkConst ``LeanerIR.RuntimeState) fun state => do
        let initialValues ← projections initial reps.size
        let currentValues ← projections current reps.size
        let mut locals := Array.replicate declaration.locals.size none
        let mut oldLocals := Array.replicate declaration.locals.size none
        let mut unchanged := #[]
        for ((decl, rep), index) in (declarations.zip reps).zipIdx do
          locals := locals.set! decl.id.index (some (← logical rep currentValues[index]!))
          unless decl.mutable do
            unchanged := unchanged.push (← mkEq
              (← logical rep currentValues[index]!) (← logical rep initialValues[index]!))
        for (argument, index) in artifacts.signature.arguments.zipIdx do
          let value ← mkAppM (artifacts.argumentsType ++ argument.name) #[args]
          oldLocals := oldLocals.set! index (some (← logical argument.rep value))
        let context : Context := {
          unit, namespaceId, ns, twins, locals, oldLocals, localTypes, results := #[]
          state := some state, oldState := none }
        let clauses ← block.conditions.mapM (translate context ·.expression)
        mkLambdaFVars #[args, initial, current, state] (← conjunction (unchanged ++ clauses))
    predicates := predicates.push
      (⟨site, declarations.map (·.id), reps, .anonymous⟩, predicate)
  return predicates

/-- One `aborts_if` clause: its condition, the abort code the failure
outcome must carry when declared with `with`, and its authored byte range. -/
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
of its body or a specification clause names.  A summary mentions only
what the function can read or write, so a caller consuming it is not
made to speak about families the callee never sees. -/
private def familiesUsed (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (families : Array SpecTypes.FamilyInfo) : Array SpecTypes.FamilyInfo := Id.run do
  let mut roots : Array ExprId :=
    declaration.contract.conditions.flatMap fun condition =>
      #[condition.expression] ++ condition.auxiliary.map (·.2)
  roots := roots ++ ns.invariants.flatMap fun invariant =>
    #[invariant.condition.expression] ++ invariant.condition.auxiliary.map (·.2)
  if let .structured root := declaration.body then
    roots := roots.push root
  let mut work := roots.toList
  let mut visited : Array Nat := #[]
  let mut used : Array Nat := #[]
  for _ in [0:ns.expressions.size + roots.size + 1] do
    match work with
    | [] => break
    | id :: rest =>
        work := rest
        if visited.contains id.index then continue
        visited := visited.push id.index
        let some expression := ns.expressions[id.index]? | continue
        match expression.kind with
        | .operation operation instantiations _ _ =>
            let touchesStorage := match operation with
              | .global _ | .specification (.global _) => true
              | _ => false
            if touchesStorage then
              for instantiation in instantiations do
                if let .typeArg typeUse := instantiation then
                  used := used.push typeUse.typeId.index
        | _ => pure ()
        work := work ++ (LeanerIR.Validation.expressionChildren expression.kind).toList
  return families.filter fun family => used.contains family.typeIndex

/-- What a global lender adds to a summary: the storage slot's export
binder, the facts over it, and the final state a clause reads through. -/
private structure GlobalLenderBound where
  binders : Array Lean.Expr := #[]
  facts : Array Lean.Expr := #[]
  /-- The final state with the lender's slot resolved through the returned
  row: what a clause's storage read denotes. -/
  resolvedFinal : Lean.Expr

/-- Bind the global lender's export and continue.  The callee leaves the
resource in storage with the returned loan's hole at the focus and the
key registered to that loan; a clause reading the resource sees the
resolution of that export through the returned row, as a parameter's
clauses see its pending export. -/
private def withGlobalLender (state final : Lean.Expr)
    (parameterSlots : Array Slot) (parameterBound : Array SlotBinders)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders)
    (typeInstantiation : Option Lean.Expr)
    (lender? : Option GlobalLender)
    (k : GlobalLenderBound → MetaM Lean.Expr) : MetaM Lean.Expr := do
  let some lender := lender? | k { resolvedFinal := final }
  let some keySlot := parameterSlots[lender.keyParameter]?
    | throwError "internal: a global lender's key parameter is out of range"
  let some keyBinders := parameterBound[lender.keyParameter]?
    | throwError "internal: a global lender's key parameter has no binders"
  let some resultLoan := resultBound[0]?.bind (·.loan)
    | throwError "internal: a global lender's result has no loan binder"
  let encodedKey ← (domainOf keySlot.physical).encode keyBinders.current
  let key ← familyAccessor typeInstantiation lender.family `key #[encodedKey]
  let encodedResults ← (resultSlots.zip resultBound).mapM fun (slot, binders) =>
    slotRuntimeValue slot binders
  let resultRow ← mkArrayLit (mkConst ``LeanerIR.RuntimeValue) encodedResults.toList
  withLocalDeclD (Name.mkSimple (lender.family.info.name ++ "_pending"))
      (mkConst ``LeanerIR.RuntimeValue) fun keyPending => do
    let finalGlobals ← mkAppM ``LeanerIR.RuntimeState.globals #[final]
    let lookup ← mkAppM ``LeanerIR.GlobalMap.lookup #[finalGlobals, key]
    let stored ← mkAppM ``Eq #[lookup, ← mkAppM ``Option.some #[keyPending]]
    let hole ← mkAppM ``LeanerIR.RuntimeValue.loanHole #[resultLoan]
    let some transfer ← focusedTransfer? keyPending hole lender.focus
      | throwError "internal: a global lender's struct has a field without a \
          scalar domain"
    /- The registry is the entry's plus the returned loan's registration:
    exactly the export's, and the shape a caller's settling marker reads. -/
    let registered ← mkAppM ``Eq
      #[← mkAppM ``LeanerIR.RuntimeState.globalLoans #[final],
        ← mkAppM ``List.cons
          #[← mkAppM ``Prod.mk #[resultLoan, key],
            ← mkAppM ``LeanerIR.RuntimeState.globalLoans #[state]]]
    let freshLo ← mkAppM ``LE.le
      #[← mkAppM ``LeanerIR.RuntimeState.nextLoan #[state], resultLoan]
    let freshHi ← mkAppM ``LT.lt
      #[resultLoan, ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[final]]
    let resolved ← mkAppM ``LeanerIR.SemanticOperations.resolveReturnedBorrows
      #[resultRow, keyPending]
    let resolvedFinal ← mkAppM ``LeanerIR.RuntimeState.mk
      #[← mkAppM ``LeanerIR.GlobalMap.insert #[finalGlobals, key, resolved],
        ← mkAppM ``LeanerIR.RuntimeState.globalLoans #[final],
        ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[final],
        ← mkAppM ``LeanerIR.RuntimeState.pending #[final]]
    k { binders := #[keyPending], facts := #[stored, transfer, registered, freshLo, freshHi],
        resolvedFinal }

/-- Build the Lean `FunctionContract` term of one validated function. -/
def buildContract (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (twins : Array SpecTypes.TwinInfo := #[])
    (families : Array SpecTypes.FamilyInfo := #[])
    (carrier : Option Lean.Expr := none)
    (codecs : Option Lean.Expr := none)
    (typeInstantiation : Option Lean.Expr := none) :
    MetaM Lean.Expr := do
  let families := familiesUsed ns declaration families
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
  let runtimeValues := mkApp (mkConst ``Array [.zero]) (mkConst ``LeanerIR.RuntimeValue)
  let runtimeState := mkConst ``LeanerIR.RuntimeState
  let failure := mkConst ``LeanerIR.Proofs.Failure
  /- Bind the argument row, state, and (for `ensures`) result row, then the
  logical slots under them. In a one-state clause a parameter denotes its
  entry value; in `ensures` it denotes the current value — the exit of a
  mutable reference — and `spec.old` reaches the entry. -/
  let contextOf (state : Lean.Expr) (bound : Array SlotBinders) : Context :=
    { unit, namespaceId, ns, twins, families
      carrier := carrier, codecs := codecs,
      typeInstantiation := typeInstantiation
      locals := padLocals (bound.map fun binders => some binders.entry)
      localTypes := functionLocalTypes
      oldLocals := padLocals (bound.map fun binders => some binders.entry)
      results := #[]
      state := some state, oldState := some state }
  let translateAll (context : Context) (clauses : Array ExprId) :
      MetaM (Array Lean.Expr) :=
    clauses.mapM (translate context)
  let flatten (bound : Array SlotBinders) : Array Lean.Expr :=
    bound.flatMap SlotBinders.flat
  let requiresTerm ← withLocalDeclD `arguments runtimeValues fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withFamilyContents families carrier fun familyBound =>
        withSlotBinders parameterSlots (withExit := false) fun bound => do
          let context := contextOf state bound
          let represented ← familyConjuncts families familyBound state codecs typeInstantiation
          let equation ← rowEquation arguments parameterSlots bound
          let ranges ← slotRanges parameterSlots (some ·.entry) bound
          let ownership ← mutableLoanFacts state parameterSlots bound
          let dataInvariants ← slotDataInvariantTerms context parameterSlots bound
            (some ·.entry)
          let clauses ← translateAll context groups.requires
          let invariants ← namespaceInvariantTerms
            context invariantResources .entry
          let body ← conjunction
            (represented ++ #[equation] ++ ranges ++ ownership ++ clauses ++
              dataInvariants.map (·.1) ++ invariants.map (·.1))
          let closed ← existsOver (familyBound ++ flatten bound) body
          mkLambdaFVars #[arguments, state] closed
  let ensuresTerm ← withLocalDeclD `arguments runtimeValues fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `results runtimeValues fun results =>
        withLocalDeclD `final runtimeState fun final =>
          withSlotBinders parameterSlots (withExit := true) fun parameterBound =>
            withSlotBinders resultSlots (withExit := false) fun resultBound =>
            withGlobalLender state final parameterSlots parameterBound resultSlots resultBound
              typeInstantiation
              (globalReborrowLender? unit namespaceId ns twins families declaration)
              fun globalBound => do
              let argumentEquation ← rowEquation arguments parameterSlots parameterBound
              let resultEquation ← rowEquation results resultSlots resultBound
              let componentEquations ← componentEquations resultSlots resultBound
              let ranges ← slotRanges resultSlots (some ·.entry) resultBound
              let componentRanges ← (resultSlots.zip resultBound).flatMapM fun (slot, binders) =>
                slotRanges slot.components (some ·.entry)
                  (binders.componentBinders.map fun (loan, value) => { loan := some loan, entry := value })
              let exitRanges ← slotRanges parameterSlots (·.exit) parameterBound
              let pending ← pendingEquation? state final parameterSlots parameterBound
                resultSlots resultBound (reborrowLenders unit namespaceId ns twins declaration)
              let context : Context :=
                { unit, namespaceId, ns, twins, families
                  carrier := carrier, codecs := codecs,
                  typeInstantiation := typeInstantiation
                  locals := padLocals (parameterBound.map fun binders => some binders.current)
                  localTypes := functionLocalTypes
                  oldLocals := padLocals (parameterBound.map fun binders => some binders.entry)
                  results := resultBound.map (·.entry)
                  resultTypes := resultSlots.map (·.physical)
                  state := some globalBound.resolvedFinal, oldState := some state }
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
                (#[argumentEquation, resultEquation] ++ componentEquations ++ pending.toArray ++
                  globalBound.facts ++ ranges ++ componentRanges ++ exitRanges ++ clauses ++
                  dataInvariantObligations ++ modifiedDataInvariants ++ invariantObligations)
              let closed ← existsOver
                (flatten parameterBound ++ flatten resultBound ++ globalBound.binders) body
              mkLambdaFVars #[arguments, state, results, final] closed
  /- The three failure components follow the reference stack's Move-style
  reading of `aborts_if Pᵢ [with Cᵢ]`.  Without clauses the behavior is
  uninterpreted (with `aborts_if_is_strict`: never fails).  With clauses,
  every Pᵢ both forces a failure and excuses the postcondition; a non-partial
  list also permits only outcomes matching a clause — with its code, when one
  is declared — while `aborts_if_is_partial` additionally permits any outcome
  in states where no Pᵢ holds. -/
  let declaredAborts := !groups.abortsIf.isEmpty
  let abortConditionTerm ← withLocalDeclD `arguments runtimeValues fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withSlotBinders parameterSlots (withExit := false) fun bound => do
        let equation ← rowEquation arguments parameterSlots bound
        let clauses ← translateAll (contextOf state bound) (groups.abortsIf.map (·.condition))
        let body ← conjunction #[equation, ← disjunction clauses]
        let closed ← if declaredAborts then existsOver (flatten bound) body
          else pure (mkConst ``False)
        mkLambdaFVars #[arguments, state] closed
  let abortsTerm ← withLocalDeclD `arguments runtimeValues fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `failure failure fun failureBinder => do
        if !declaredAborts then
          let permitted := if isStrict then mkConst ``False else mkConst ``True
          mkLambdaFVars #[arguments, state, failureBinder] permitted
        else
          withSlotBinders parameterSlots (withExit := false) fun bound => do
            let context := contextOf state bound
            let equation ← rowEquation arguments parameterSlots bound
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
            let body ← conjunction #[equation, permitted]
            let closed ← existsOver (flatten bound) body
            mkLambdaFVars #[arguments, state, failureBinder] closed
  /- The frame is stated over global memory: a successful execution leaves
  the globals it does not declare as modified alone.  Local heap slots are
  not part of it — finalization releases the slots a call borrowed.  With no
  `modifies` clause the whole map is unchanged; with clauses, every key
  other than the declared ones reads the same, which the keyed map laws
  discharge and a caller frames with.

  The clause also carries the loan discipline a modular caller needs of the
  state a call hands back: freshness of unminted loan ids is preserved, the
  registration of every loan minted before the call reads the same after
  it, and loan ids only ever grow.  Registry equality would be wrong — a
  callee returning a global `&mut` legitimately exits with its returned
  loan registered — and these three are exactly what lets the caller
  re-establish its own `requires`-side loan facts for the code after the
  call. -/
  let frameTerm ← withLocalDeclD `arguments runtimeValues fun arguments =>
    withLocalDeclD `initial runtimeState fun initial =>
      withLocalDeclD `final runtimeState fun final => do
        let finalGlobals ← mkAppM ``LeanerIR.RuntimeState.globals #[final]
        let initialGlobals ← mkAppM ``LeanerIR.RuntimeState.globals #[initial]
        let loanDiscipline ← mkAppM
          ``LeanerIR.SemanticOperations.LoanDiscipline #[initial, final]
        let body ←
          if declaration.contract.modifiesAll then
            pure (mkConst ``True)
          else if declaration.contract.modifies.isEmpty then
            mkAppM ``Eq #[finalGlobals, initialGlobals]
          else
            withSlotBinders parameterSlots (withExit := false) fun bound => do
              let context := contextOf initial bound
              let equation ← rowEquation arguments parameterSlots bound
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
              let quantified ← if hasLooseFrame declaration.contract then do
                  -- Quantify inside each listed family, as in v0. Its
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
              existsOver (flatten bound) (← conjunction #[equation, quantified])
        mkLambdaFVars #[arguments, initial, final]
          (← mkAppM ``And #[body, loanDiscipline])
  mkAppOptM ``LeanerIR.Proofs.Contract.mk
    #[some runtimeState, some failure, some runtimeValues, some runtimeValues,
      some requiresTerm, some ensuresTerm, some abortsTerm,
      some abortConditionTerm, some abortConditionTerm, some frameTerm]

/-- Translate authored clauses over the typed entry and updated owner.
This contract is independent of the assignment expression. The runtime
contract remains a separate boundary obligation, never the native postcondition.
The initial fragment has one integer owner and no returned references. -/
private def buildNativeOwnerContract (unit : ValidatedUnit)
    (namespaceId : LeanerIR.NamespaceId) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (artifacts : Typed.Artifacts) (typedContract : Name) : MetaM Lean.Expr := do
  let argument := artifacts.signature.arguments[0]!
  let valueType ← argument.rep.leanType none
  let ownerType := mkApp (mkConst ``LeanerIR.Proofs.MutableArgument) valueType
  let argsType := mkConst artifacts.argumentsType
  let stateType := mkConst ``LeanerIR.RuntimeState
  let failureType := mkConst ``LeanerIR.Proofs.Failure
  let groups ← groupConditions unit declaration.contract.conditions
  let declaredAborts := !groups.abortsIf.isEmpty
  let isPartial := pragmaEnabled declaration.contract "aborts_if_is_partial"
  let isStrict := pragmaEnabled declaration.contract "aborts_if_is_strict"
  let input (args : Lean.Expr) := mkAppM (artifacts.argumentsType ++ argument.name) #[args]
  let logical (owner : Lean.Expr) : MetaM Lean.Expr := do
    mkAppM ``LeanerIR.SpecInt.val #[← mkAppM ``LeanerIR.Proofs.MutableArgument.value #[owner]]
  let .reference reference := unit.tables.types[declaration.signature.parameters[0]!.typeUse.typeId.index]!
    | throwError "native owner contract requires a reference parameter"
  let physical := unit.tables.types[reference.referent.index]!
  let context (args initial current state : Lean.Expr) : MetaM Context := do
    return {
      unit, namespaceId, ns, locals := #[some (← logical current)],
      oldLocals := #[some (← logical (← input args))], localTypes := #[physical],
      results := #[], state := some state, oldState := some initial }
  let condition (args initial : Lean.Expr) : MetaM Lean.Expr := do
    let ctx ← context args initial (← input args) initial
    disjunction (← groups.abortsIf.mapM (translate ctx ·.condition))
  let ensures ← withLocalDeclD `args argsType fun args =>
    withLocalDeclD `initial stateType fun initial =>
    withLocalDeclD `output ownerType fun output =>
    withLocalDeclD `final stateType fun final => do
      let ctx ← context args initial output final
      let clauses ← groups.ensures.mapM fun (clause, range) =>
        return markObligation range (← translate ctx clause)
      let sameLoan ← mkEq (← mkAppM ``LeanerIR.Proofs.MutableArgument.loan #[output])
        (← mkAppM ``LeanerIR.Proofs.MutableArgument.loan #[← input args])
      let post ← conjunction clauses
      if post.getUsedConstants.contains ``LeanerIR.RuntimeValue then
        throwError "native owner clauses require native value projections"
      mkLambdaFVars #[args, initial, output, final] (← mkAppM ``And #[sameLoan, post])
  let mayAbort ← withLocalDeclD `args argsType fun args =>
    withLocalDeclD `initial stateType fun initial => do
      mkLambdaFVars #[args, initial] (← condition args initial)
  let aborts ← withLocalDeclD `args argsType fun args =>
    withLocalDeclD `initial stateType fun initial =>
    withLocalDeclD `failure failureType fun failure => do
      let ctx ← context args initial (← input args) initial
      let matched ← groups.abortsIf.mapM fun clause => do
        let cond ← translate ctx clause.condition
        let cond ← match clause.code with
          | none => pure cond
          | some code => do
            let encoded ← mkAppM ``LeanerIR.RuntimeValue.integer #[← translate ctx code]
            let payload ← mkArrayLit (mkConst ``LeanerIR.RuntimeValue) [encoded]
            let outcome ← mkAppM ``Prod.mk #[mkConst ``LeanerIR.ThrowKind.abort, payload]
            mkAppM ``And #[cond, ← mkEq failure outcome]
        return markObligation clause.range cond
      let mut allowed ← disjunction matched
      if !declaredAborts then allowed := mkConst (if isStrict then ``False else ``True)
      else if isPartial then
        allowed ← mkAppM ``Or #[allowed, ← mkAppM ``Not #[← condition args initial]]
      mkLambdaFVars #[args, initial, failure] allowed
  let frame ← withLocalDeclD `args argsType fun args =>
    withLocalDeclD `initial stateType fun initial =>
    withLocalDeclD `final stateType fun final => do
      mkLambdaFVars #[args, initial, final] (← mkEq final initial)
  mkAppOptM ``LeanerIR.Proofs.Contract.mk
    #[some stateType, some failureType, some argsType, some ownerType,
      some (← mkAppM ``LeanerIR.Proofs.Contract.requires #[mkConst typedContract]),
      some ensures, some aborts, some mayAbort, some mayAbort, some frame]

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

/-- Clause-translated runtime contract underlying a V3 native view. -/
def rawContractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "rawContract"

/-- Native argument/result contract exposed by V3. -/
def typedContractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedContract"

private def rootIdent (name : Name) : Ident :=
  mkIdent (rootNamespace ++ name)

/-- Resolve a `namespace::function` path against the registered units. -/
def resolvePath (stx : Syntax) :
    CommandElabM (Array String × String × ValidatedUnit × Nat × ValidatedNamespace × Nat ×
      LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) := do
  let segments := pathSegments stx
  unless segments.size ≥ 2 do
    throwErrorAt stx "a verification path must name a namespace and a function"
  let function := segments[segments.size - 1]!
  let namespaceSegments := segments.pop
  let some unit := LeanerLang.registeredUnit? (← getEnv) (pathName namespaceSegments)
    | throwErrorAt stx s!"unknown Leaner namespace `{pathName namespaceSegments}`"
  let some (namespaceIndex, ns, index, declaration) := findFunction? unit function
    | throwErrorAt stx s!"unknown function `{function}` in `{pathName namespaceSegments}`"
  return (namespaceSegments, function, unit, namespaceIndex, ns, index, declaration)

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
    namespaces := namespaces.push (mkAppN nsTemplate.getAppFn #[core, tables])
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

/-- Generate the Lean contract of one specified Leaner function. -/
syntax (name := leanerContractCommand) "#leaner_contract" leanerPath : command

/-- Define the generated contract of one function, once. -/
def ensureContractDefinition (namespaceSegments : Array String)
    (function : String) (unit : ValidatedUnit)
    (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    CommandElabM Name := do
  let name := contractName namespaceSegments function
  if (← getEnv).contains name then return name
  let (twins, families) ← SpecTypes.ensureSpecTypes namespaceSegments unit
  let signatureResult ← Typed.ensureSignatureDefinitions unit namespaceSegments
    function declaration twins
  match signatureResult with
  | .error reason =>
      logInfo m!"V3 native wrapper unavailable for `{function}`: {reason}; using runtime rows"
      liftTermElabM do
        let value ← buildContract unit ⟨namespaceIndex⟩ ns declaration twins families
        addDecl (.defnDecl {
          name, levelParams := [], type := mkConst ``LeanerIR.Proofs.FunctionContract, value
          hints := .abbrev, safety := .safe })
        enableRealizationsForConst name
  | .ok artifacts =>
      let rawName := rawContractName namespaceSegments function
      let generic := artifacts.signature.typeParameterCount != 0
      liftTermElabM do
        let value ← if !generic then
          buildContract unit ⟨namespaceIndex⟩ ns declaration twins families
        else
          let carrierType ← mkArrow (mkConst ``Nat) (mkSort (.succ .zero))
          withLocalDecl `Carrier .implicit carrierType fun carrier => do
            let inhabitedType ← withLocalDeclD `index (mkConst ``Nat) fun index => do
              let inhabited ← mkAppM ``Inhabited #[mkApp carrier index]
              mkForallFVars #[index] inhabited
            let codecsType ← withLocalDeclD `index (mkConst ``Nat) fun index => do
              let codec ← mkAppM ``LeanerIR.Proofs.Codec
                #[mkApp carrier index, mkConst ``LeanerIR.RuntimeValue]
              mkForallFVars #[index] codec
            let instantiationType ← mkAppM ``Array
              #[← mkAppM ``Prod #[mkConst ``LeanerIR.TypeId, mkConst ``LeanerIR.TypeId]]
            withLocalDecl `carrierInhabited .instImplicit inhabitedType fun carrierInhabited =>
              withLocalDeclD `typeInstantiation instantiationType fun typeInstantiation =>
                withLocalDeclD `codecs codecsType fun codecs => do
                  let body ← buildContract unit ⟨namespaceIndex⟩ ns declaration twins families
                    (some carrier) (some codecs) (some typeInstantiation)
                  mkLambdaFVars
                    #[carrier, carrierInhabited, typeInstantiation, codecs] body
        let type ← inferType value
        addDecl (.defnDecl {
          name := rawName, levelParams := [], type, value
          hints := .abbrev, safety := .safe })
        enableRealizationsForConst rawName
      let carrier := mkIdent `Carrier
      let codecs := mkIdent `codecs
      let typedName := rootIdent (typedContractName namespaceSegments function)
      let rawIdent := rootIdent rawName
      let publicName := rootIdent name
      let argumentsType ← if artifacts.signature.typeParameterCount == 0 then
        pure (⟨(rootIdent artifacts.argumentsType).raw⟩ : Term)
      else
        ``($(rootIdent artifacts.argumentsType) $carrier)
      let resultsType ← match artifacts.resultsType? with
        | some results =>
            if artifacts.signature.typeParameterCount == 0 then
              pure (⟨(rootIdent results).raw⟩ : Term)
            else
              ``($(rootIdent results) $carrier)
        | none => Typed.resultTypeSyntax artifacts.signature carrier
      if artifacts.signature.typeParameterCount == 0 then
        elabCommand (← `(def $typedName:ident :
            LeanerIR.Proofs.Contract LeanerIR.RuntimeState
              LeanerIR.Proofs.Failure $argumentsType $resultsType :=
          LeanerIR.Proofs.Contract.typed
            $(rootIdent artifacts.argumentsCodec)
            $(rootIdent artifacts.resultsCodec) $rawIdent))
        elabCommand (← `(def $publicName:ident :
            LeanerIR.Proofs.FunctionContract :=
          LeanerIR.Proofs.Contract.runtime
            $(rootIdent artifacts.argumentsCodec)
            $(rootIdent artifacts.resultsCodec) $typedName))
      else
        elabCommand (← `(def $typedName:ident
            {$carrier:ident : Nat → Type}
            [carrierInhabited : ∀ index, Inhabited ($carrier index)]
            (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId))
            ($codecs:ident : ∀ index,
              LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue) :
            LeanerIR.Proofs.Contract LeanerIR.RuntimeState
              LeanerIR.Proofs.Failure $argumentsType $resultsType :=
            LeanerIR.Proofs.Contract.typed
            ($(rootIdent artifacts.argumentsCodec) $codecs)
            ($(rootIdent artifacts.resultsCodec) $codecs)
            ($rawIdent typeInstantiation $codecs)))
        let runtimeCodecs ← `(term|
          fun (_ : Nat) => LeanerIR.Proofs.Codec.identity LeanerIR.RuntimeValue)
        elabCommand (← `(def $publicName:ident :
            LeanerIR.Proofs.FunctionContract :=
          LeanerIR.Proofs.Contract.runtime
            ($(rootIdent artifacts.argumentsCodec) $runtimeCodecs)
            ($(rootIdent artifacts.resultsCodec) $runtimeCodecs)
            ($typedName #[] $runtimeCodecs)))
  return name

@[command_elab leanerContractCommand]
def elabLeanerContract : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let (namespaceSegments, function, unit, namespaceIndex, ns, _, declaration) ←
    resolvePath pathSyntax
  discard <| ensureContractDefinition namespaceSegments function unit namespaceIndex
    ns declaration

/-- Name of the V3 theorem over the generated native signature. -/
def typedVerifiedName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedVerified"

/-- Destructure a generated argument record and the representation wrappers
whose projections should disappear before arithmetic.  In particular, a
mutable integer becomes its loan id, mathematical value, and range proof;
`omega` then sees one atom instead of alternate projection spellings of the
same `SpecInt`. -/
elab "leaner_native_cases" hypothesis:ident : tactic => do
  let env ← Lean.getEnv
  let isNativeWrapper (type : Lean.Expr) : Bool :=
    type.getAppFn.constName? == some ``LeanerIR.SpecInt ||
      type.getAppFn.constName? == some ``LeanerIR.Proofs.MutableArgument ||
      type.getAppFn.constName?.any (LeanerIR.Proofs.leanerTwinAttribute.hasTag env)
  /- Each field is named by its path: a parameter by its name, a mutable
  reference's loan as `<p>_loan` and its value as `<p>`, a certified
  integer's value as `<p>` and its certificate as `<p>_fits`.  Generated
  scripts refer to these names. -/
  let nameFields (goal : Lean.MVarId) (fields : Array Lean.Expr)
      (names : Array String) : Lean.MetaM (Lean.MVarId × Array (Lean.FVarId × String)) := do
    let mut goal := goal
    let mut named : Array (Lean.FVarId × String) := #[]
    for (field, name) in fields.zip names do
      if let .fvar id := field then
        goal ← goal.rename id (Lean.Name.mkSimple name)
        named := named.push (id, name)
    pure (goal, named)
  let rec destructWrappers (goal : Lean.MVarId) (fields : Array (Lean.FVarId × String))
      (fuel : Nat) : Lean.MetaM (Array Lean.MVarId) := do
    match fuel with
    | 0 => return #[goal]
    | fuel + 1 =>
        let some (field, base) := fields[0]? | return #[goal]
        let rest := fields.extract 1 fields.size
        let type ← goal.withContext do
          Lean.instantiateMVars (← field.getType)
        if !isNativeWrapper type then
          destructWrappers goal rest fuel
        else
          let cases ← goal.cases field
          let mut goals := #[]
          for subgoal in cases do
            let subnames : Array String :=
              if type.getAppFn.constName? == some ``LeanerIR.Proofs.MutableArgument then
                #[s!"{base}_loan", base]
              else if type.getAppFn.constName? == some ``LeanerIR.SpecInt then
                #[base, s!"{base}_fits"]
              else
                subgoal.fields.mapIdx fun index _ => s!"{base}_field{index}"
            let (goal, named) ← nameFields subgoal.mvarId subgoal.fields subnames
            goals := goals ++ (← destructWrappers goal (rest ++ named) fuel)
          return goals
  Lean.Elab.Tactic.liftMetaTactic fun goal => do
    let declaration ← Lean.Meta.getLocalDeclFromUserName hypothesis.getId
    let structName? ← goal.withContext do
      pure (← Lean.instantiateMVars declaration.type).getAppFn.constName?
    let fieldNames := match structName? with
      | some structName => (Lean.getStructureFields env structName).map (·.toString)
      | none => #[]
    match ← goal.cases declaration.fvarId with
    | #[subgoal] =>
        let (goal, named) ← nameFields subgoal.mvarId subgoal.fields fieldNames
        return (← destructWrappers goal named 64).toList
    | subgoals => return subgoals.toList.map fun subgoal => subgoal.mvarId
  /- Reduce the projection redexes the substitution leaves in retained
  facts, so `omega` sees the destructured components as atoms.  The goal is
  left to the drive's own normalization order. -/
  Lean.Elab.Tactic.evalTactic (← `(tactic| all_goals try simp only [] at *))
  -- Reconstruct certified range conjunctions before route vocabulary is
  -- assigned. Scalar call routes consume both the split bounds and their
  -- compact conjunction.
  Lean.Elab.Tactic.evalTactic (← `(tactic| all_goals try leaner_certify!))
  Lean.Elab.Tactic.evalTactic (← `(tactic| all_goals leaner_name_facts))

/-- The errors a message log holds, reported or not. -/
private def countErrors (log : MessageLog) : Nat :=
  log.reportedPlusUnreported.toList.filter (·.severity == .error) |>.length

/-- Native transport either preserves the state representation directly or
uses both an exact ownership-output agreement and a checked contract bridge.
A boundary certificate alone never qualifies a source theorem. -/
private def hasNativeTransport (proof : Lean.Expr) : Bool :=
  let constants := proof.getUsedConstants
  constants.contains ``LeanerIR.Proofs.Represents.typed ||
    (constants.contains ``LeanerIR.Proofs.NativeBoundary.Represents.typed &&
      constants.contains ``LeanerIR.Proofs.NativeBoundary.Transports.satisfies)

/-- Prove the native V3 view directly over the shallow denotation.  Generic
functions quantify over the abstract carrier and its certified codec; the
proof is shared by every concrete instantiation. -/
private def ensureTypedVerificationTheorem (reference : Syntax)
    (namespaceSegments : Array String) (function : String)
    (namespaceIndex functionIndex : Nat)
    (unitDefinition semanticsEq : Name)
    (preparedUnit : ValidatedUnit) (preparedNs : ValidatedNamespace)
    (preparedDeclaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (generated : LeanerLang.Denotation.Generated)
    (artifacts : Typed.Artifacts)
    (twins : Array SpecTypes.TwinInfo)
    (script? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq)) : CommandElabM Name := do
  let theoremName := typedVerifiedName namespaceSegments function
  let qualifiedTheoremName := (← getCurrNamespace) ++ theoremName
  if let some existing := (← getEnv).find? qualifiedTheoremName then
    if !(existing.value? (allowOpaque := true)).any hasNativeTransport then
      throwErrorAt reference m!"`{function}` was already verified by another route; \
        the native route cannot reuse its frame/row proof"
    return theoremName
  let typedContract := typedContractName namespaceSegments function
  let rawContract := rawContractName namespaceSegments function
  if script?.isSome then
    throwErrorAt reference "the native route consumes computation certificates, not row scripts"
  let loopSites := match preparedDeclaration.body with
    | .structured root => loopSpecifications preparedNs root
    | _ => #[]
  unless loopSites.isEmpty do
    let loopBase := (← getCurrNamespace) ++ Name.str (pathName namespaceSegments) function
    let names := loopSites.map fun (site, _) => loopBase ++ Name.mkSimple s!"nativeLoopInvariant_{site.index}"
    Perf.measureArtifacts s!"{pathName namespaceSegments}::{function} typed" names do
      let loopInvariants ← liftTermElabM <|
        buildNativeLoopInvariants preparedUnit ⟨namespaceIndex⟩ preparedNs
          preparedDeclaration twins artifacts
      let mut loopMetadata := #[]
      for (info, predicate) in loopInvariants do
        let localName := Name.str (Name.str (pathName namespaceSegments) function)
          s!"nativeLoopInvariant_{info.site.index}"
        let qualifiedName := (← getCurrNamespace) ++ localName
        unless (← getEnv).contains qualifiedName do
          liftTermElabM do
            let predicate ← Lean.instantiateMVars predicate
            let type ← Lean.instantiateMVars (← Lean.Meta.inferType predicate)
            addDecl (.defnDecl {
              name := qualifiedName, levelParams := [], type, value := predicate
              hints := .abbrev, safety := .safe })
            enableRealizationsForConst qualifiedName
        loopMetadata := loopMetadata.push { info with predicate := qualifiedName }
      modifyEnv (NativeLoopInfo.entries.addEntry ·
        ⟨(← getCurrNamespace) ++ Name.str (pathName namespaceSegments) function, loopMetadata⟩)
  let publicContract := contractName namespaceSegments function
  let carrierInhabitedIdent := mkIdent `carrierInhabited
  let executableIdent := mkIdent `executable
  let registryIdent := mkIdent `registry
  let command ← do
    let ownerBase := (← getCurrNamespace) ++ Name.str (pathName namespaceSegments) function
    let some bodyValue := (← getEnv).find? generated.body |>.bind (·.value?)
      | throwErrorAt reference "missing native denotation body"
    let body := bodyValue.bindingBody!
    if !(← getEnv).contains (ownerBase ++ `computation) &&
        NativeMutable.eligible artifacts.signature body then
      let ownerErrorsBefore := countErrors (← get).messages
      let names := #[`nativeContract, `computation, `computationVerified,
        `exitFrame, `project, `commit, `commit_eq, `computationBoundary,
        `computationRepresents].map (ownerBase ++ ·)
      let ownerOptions := maxHeartbeats.set ((← getOptions).setBool `Elab.async false)
        (leaner.verifyHeartbeats.get (← getOptions))
      withScope (fun scope => { scope with opts := ownerOptions }) do
        Perf.measureArtifacts s!"{pathName namespaceSegments}::{function} typed" names do
          liftTermElabM do
            let value ← buildNativeOwnerContract preparedUnit ⟨namespaceIndex⟩ preparedNs
              preparedDeclaration artifacts typedContract
            let name := ownerBase ++ `nativeContract
            addDecl (.defnDecl {
              name, levelParams := [], type := ← Lean.Meta.inferType value,
              value, hints := .abbrev, safety := .safe })
            enableRealizationsForConst name
          NativeMutable.generate ownerBase generated artifacts rawContract typedContract body
      if countErrors (← get).messages > ownerErrorsBefore then
        throwErrorAt reference "native owner generation failed"
    LeanerLang.Computation.ensure namespaceSegments function generated artifacts twins
      rawContract typedContract
      (leaner.verifyHeartbeats.get (← getOptions))
    /- Native computations and exact execution certificates are separate
    artifacts. A missing certificate is a generation error, never a fallback. -/
    let base := (← getCurrNamespace) ++ Name.str (pathName namespaceSegments) function
    let computation := rootIdent (base ++ `computation)
    let verified := rootIdent (base ++ `computationVerified)
    let represents := rootIdent (base ++ `computationRepresents)
    let budget := Syntax.mkNumLit (toString (leaner.verifyHeartbeats.get (← getOptions)))
    for suffix in [`computation, `computationVerified, `computationRepresents] do
      unless (← getEnv).contains (base ++ suffix) do
        throwErrorAt reference m!"native route for `{function}` requires `{base ++ suffix}`; \
          frame/row fallback is disabled"
    let hasBoundary := (← getEnv).contains (base ++ `computationBoundary)
    if hasBoundary && artifacts.signature.typeParameterCount != 0 then
      throwErrorAt reference "generic ownership-output boundary transport is not implemented"
    if artifacts.signature.typeParameterCount == 0 then
      let proof ← if hasBoundary then
        `(tactic| exact (LeanerIR.Proofs.satisfies_congr
          (LeanerIR.Proofs.NativeBoundary.Represents.typed
            (body := $computation) ($represents $executableIdent))
          $(rootIdent typedContract)).mpr
            (LeanerIR.Proofs.NativeBoundary.Transports.satisfies
              $(rootIdent (base ++ `computationBoundary)) $verified))
      else
        `(tactic| exact (LeanerIR.Proofs.satisfies_congr
          (LeanerIR.Proofs.Represents.typed
            (computation := $computation) ($represents $executableIdent))
          $(rootIdent typedContract)).mpr $verified)
      `(command|
        set_option Elab.async false in
        set_option maxHeartbeats $budget:num in
        theorem $(mkIdent theoremName)
            {$registryIdent:ident : LeanerIR.Validation.SemanticsRegistry}
            {$executableIdent:ident : LeanerIR.Validation.ExecutableUnit}
            (_prepared : LeanerIR.Validation.prepareExecution $registryIdent
              $(mkIdent unitDefinition) = .ok $executableIdent) :
            LeanerIR.Proofs.Satisfies
              ($(rootIdent artifacts.denotation) $executableIdent)
              $(rootIdent typedContract) := by
          $proof:tactic)
    else
      let carrier := mkIdent `Carrier
      let codecs := mkIdent `codecs
      `(command|
        set_option Elab.async false in
        set_option maxHeartbeats $budget:num in
        theorem $(mkIdent theoremName)
            {$carrier:ident : Nat → Type}
            [$carrierInhabitedIdent:ident : ∀ index, Inhabited ($carrier index)]
            ($codecs:ident : ∀ index,
              LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue)
            (types : Array (LeanerIR.TypeId × LeanerIR.TypeId))
            {$registryIdent:ident : LeanerIR.Validation.SemanticsRegistry}
            {$executableIdent:ident : LeanerIR.Validation.ExecutableUnit}
            (_prepared : LeanerIR.Validation.prepareExecution $registryIdent
              $(mkIdent unitDefinition) = .ok $executableIdent) :
            LeanerIR.Proofs.Satisfies
              ($(rootIdent artifacts.denotation) $codecs types $executableIdent)
              ($(rootIdent typedContract) types $codecs) := by
          exact (LeanerIR.Proofs.satisfies_congr
            (LeanerIR.Proofs.Represents.typed
              (computation := $computation) ($represents $codecs types $executableIdent))
            ($(rootIdent typedContract) types $codecs)).mpr ($verified $codecs types))
  /- A clause the closing could not establish was reported at its range
  and admitted; the command fails on that report, before anything
  transports the admitted theorem. -/
  let errorsBefore := countErrors (← get).messages
  try
    Perf.measure s!"{pathName namespaceSegments}::{function} typed"
      ((← getCurrNamespace) ++ theoremName) (elabCommand command)
  catch error =>
    throwErrorAt reference m!"failed to prove V3 native wrapper for `{function}`:\n{error.toMessageData}"
  if countErrors (← get).messages > errorsBefore then
    throwErrorAt reference "leaner verification failed"
  /- The public theorem is now only a certified transport: native contract
  satisfaction crosses the codecs, then the V1 agreement theorem crosses
  from the denotation to the authoritative big-step meaning. -/
  let namespaceName := pathName namespaceSegments
  let verifiedName := Name.str (Name.str namespaceName function) "verified"
  let instantiatedVerifiedName :=
    Name.str (Name.str namespaceName function) "instantiatedVerified"
  let nativeBase := (← getCurrNamespace) ++ Name.str namespaceName function
  let nativeRepresents := rootIdent (nativeBase ++ `computationRepresents)
  let nativeVerified := rootIdent (nativeBase ++ `computationVerified)
  if artifacts.signature.typeParameterCount != 0 then
    let carrier := mkIdent `Carrier
    let carrierInhabited := mkIdent `carrierInhabited
    let concreteCodecs := mkIdent `concreteCodecs
    let typeInstantiation := mkIdent `typeInstantiation
    let registry := mkIdent `registry
    let executable := mkIdent `executable
    let proof ← `(tactic| exact LeanerIR.Proofs.Represents.satisfies
      ($nativeRepresents $concreteCodecs $typeInstantiation $executable)
      ($nativeVerified $concreteCodecs $typeInstantiation))
    let command ← `(theorem $(mkIdent instantiatedVerifiedName)
        {$carrier:ident : Nat → Type}
        [$carrierInhabited:ident : ∀ index, Inhabited ($carrier index)]
        ($concreteCodecs:ident : ∀ index,
          LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue)
        ($typeInstantiation:ident :
          Array (LeanerIR.TypeId × LeanerIR.TypeId))
        {$registry:ident : LeanerIR.Validation.SemanticsRegistry}
        {$executable:ident : LeanerIR.Validation.ExecutableUnit}
        ($(mkIdent `prepared) : LeanerIR.Validation.prepareExecution $registry
          $(mkIdent unitDefinition) = .ok $executable) :
        LeanerIR.Proofs.Satisfies
          ($(mkIdent generated.denotation) $executable $typeInstantiation)
          (LeanerIR.Proofs.Contract.runtime
            ($(rootIdent artifacts.argumentsCodec) $concreteCodecs)
            ($(rootIdent artifacts.resultsCodec) $concreteCodecs)
            ($(rootIdent typedContract) $typeInstantiation $concreteCodecs)) := by
      $proof:tactic)
    Perf.measure s!"{namespaceName}::{function} typed"
      ((← getCurrNamespace) ++ instantiatedVerifiedName) (elabCommand command)
  let agreementApplication ← if generated.typeParameterCount == 0 &&
      generated.requiresUnitEquality then
    `(term| $(mkIdent generated.agreement) hu)
  else if generated.typeParameterCount == 0 then
    `(term| $(mkIdent generated.agreement) hn)
  else if generated.requiresUnitEquality then
    `(term| $(mkIdent generated.agreement)
      (Carrier := fun _ => PUnit) hu)
  else
    `(term| $(mkIdent generated.agreement)
      (Carrier := fun _ => PUnit) hn)
  let runtimeCommand ← if artifacts.signature.typeParameterCount == 0 then
    `(command|
      theorem $(mkIdent verifiedName)
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) :
          LeanerIR.Proofs.SatisfiesFunction executable
            ⟨⟨$(Syntax.mkNatLit namespaceIndex)⟩,
              ⟨$(Syntax.mkNatLit functionIndex)⟩⟩
            $(rootIdent publicContract) := by
        have hu := (LeanerIR.Validation.prepareExecution_unit prepared).trans
          $(mkIdent semanticsEq)
        have hn : executable.unit.namespaces[$(Syntax.mkNatLit namespaceIndex)]? =
            some $(mkIdent generated.namespaceDef) := by
          rw [hu]
          rfl
        apply (LeanerIR.Proofs.satisfies_congr
          $agreementApplication $(rootIdent publicContract)).mp
        simpa only [$(rootIdent publicContract):term] using
          (LeanerIR.Proofs.satisfies_runtime
            $(rootIdent artifacts.argumentsCodec)
            $(rootIdent artifacts.resultsCodec)
            ($(mkIdent generated.denotation) executable)
            $(rootIdent (typedContractName namespaceSegments function))
            ($(mkIdent theoremName) prepared)))
  else
    let runtimeCodecs := mkIdent `runtimeCodecs
    let proof ← `(tactic| exact LeanerIR.Proofs.Represents.satisfies
      ($nativeRepresents $runtimeCodecs #[] $(mkIdent `executable))
      ($nativeVerified $runtimeCodecs #[]))
    `(command|
      theorem $(mkIdent verifiedName)
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {$(mkIdent `executable) : LeanerIR.Validation.ExecutableUnit}
          ($(mkIdent `prepared) : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok $(mkIdent `executable)) :
          LeanerIR.Proofs.SatisfiesFunction $(mkIdent `executable)
            ⟨⟨$(Syntax.mkNatLit namespaceIndex)⟩,
              ⟨$(Syntax.mkNatLit functionIndex)⟩⟩
            $(rootIdent publicContract) := by
        have hu := (LeanerIR.Validation.prepareExecution_unit $(mkIdent `prepared)).trans
          $(mkIdent semanticsEq)
        have hn : ($(mkIdent `executable):ident).unit.namespaces[$(Syntax.mkNatLit namespaceIndex)]? =
            some $(mkIdent generated.namespaceDef) := by
          rw [hu]
          rfl
        apply (LeanerIR.Proofs.satisfies_congr
          $agreementApplication $(rootIdent publicContract)).mp
        let $runtimeCodecs:ident : ∀ _ : Nat,
            LeanerIR.Proofs.Codec LeanerIR.RuntimeValue LeanerIR.RuntimeValue :=
          fun _ => LeanerIR.Proofs.Codec.identity LeanerIR.RuntimeValue
        $proof:tactic)
  try
    Perf.measure s!"{pathName namespaceSegments}::{function} transport"
      ((← getCurrNamespace) ++ verifiedName) (elabCommand runtimeCommand)
  catch error =>
    throwErrorAt reference m!"failed to transport V3 theorem for `{function}`:\n{error.toMessageData}"
  return theoremName

/-- Checked source artifacts for native verification and execution agreement.
Preparing these does not prove the function's authored contract. -/
structure VerificationInput where
  namespaceIndex : Nat
  functionIndex : Nat
  unitDefinition : Name
  semanticsEq : Name
  widthEq : Name
  preparedUnit : ValidatedUnit
  preparedNs : ValidatedNamespace
  preparedDeclaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody
  generated : LeanerLang.Denotation.Generated
  artifacts : Typed.Artifacts
  twins : Array SpecTypes.TwinInfo

/-- Materialize verification inputs without choosing or running a VC route.
Native computation generation can use this boundary without first proving
the same function through the retiring frame route. -/
def prepareVerification (reference : Syntax) (namespaceSegments : Array String)
    (function : String) : CommandElabM VerificationInput := do
  let namespaceName := pathName namespaceSegments
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Leaner namespace `{namespaceName}`"
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwErrorAt reference s!"unknown function `{function}` in `{namespaceName}`"
  let unitDefinition ← ensureUnitDefinition namespaceSegments unit
  let (semanticsEq, widthEq) ← ensureSemanticsDefinitions namespaceSegments unit
  discard <| ensureContractDefinition namespaceSegments function unit namespaceIndex
    ns declaration
  let preparedUnit := (LeanerIR.Validation.prepareSemantics unit).1
  let some preparedNs := preparedUnit.namespaces[namespaceIndex]?
    | throwErrorAt reference "semantic preparation removed the function namespace"
  let some preparedDeclaration := preparedNs.functions[functionIndex]?
    | throwErrorAt reference "semantic preparation removed the function declaration"
  let denotationResult ← LeanerLang.Denotation.ensureDefinitions
    preparedUnit namespaceSegments function namespaceIndex functionIndex preparedNs
      preparedDeclaration (some (semanticsName namespaceSegments))
  let (generated, artifacts) ← match denotationResult with
    | .unsupported reason =>
        throwErrorAt reference m!"no native denotation for `{function}`: {reason}"
    | .generated generated =>
        let twins := SpecTypes.twinInfos namespaceSegments unit
        match ← Typed.ensureDefinitions preparedUnit namespaceSegments function
            preparedDeclaration twins generated.denotation with
        | .ok artifacts => pure (generated, artifacts)
        | .error reason =>
            throwErrorAt reference m!"no native wrapper for `{function}`: {reason}"
  let twins := SpecTypes.twinInfos namespaceSegments unit
  return {
    namespaceIndex, functionIndex, unitDefinition, semanticsEq, widthEq,
    preparedUnit, preparedNs, preparedDeclaration, generated, artifacts, twins }

/-- Shared no-fallback audit for one generated source function. -/
def requireNativeArtifacts (base : Name) : CommandElabM Unit := do
  let function := base.getString!
  let theoremName := base ++ `typedVerified
  let env ← getEnv
  unless (env.find? theoremName |>.bind (·.value? (allowOpaque := true))).any
      hasNativeTransport do
    throwError m!"`{function}` has no native computation transport; \
      a frame/row typed theorem does not qualify"
  unless env.contains (base ++ `computationRepresents) do
    throwError m!"missing native execution agreement for `{function}`"
  for suffix in [`computation, `computationVerified] do
    let mut pending := #[base ++ suffix]
    let mut visited : NameSet := {}
    while let some name := pending.back? do
      pending := pending.pop
      if visited.contains name then continue
      visited := visited.insert name
      let some declaration := env.find? name
        | throwError m!"missing native artifact `{name}`"
      let constants := declaration.type.getUsedConstants ++
        ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
      for dependency in constants do
        if dependency == ``LeanerIR.RuntimeFrame ||
            dependency == ``LeanerIR.Proofs.typedFunction ||
            dependency == ``LeanerIR.Proofs.decodeSpec ||
            (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
            (`LeanerIR.Proofs.ComputationAgreement).isPrefixOf dependency ||
            (`LeanerIR.Proofs.NativeBoundary).isPrefixOf dependency ||
            dependency.getString! == "computationRepresents" ||
            dependency.getString! == "computationBoundary" ||
            dependency.getString! == "computationState" || dependency == ``sorryAx ||
            (suffix == `computation && dependency == ``LeanerIR.RuntimeValue) then
          throwError m!"native artifact `{name}` retains forbidden dependency `{dependency}`"
        if base.isPrefixOf dependency then pending := pending.push dependency

/-- Verify and transport a function natively. Reject retired route selections
before preparation or reuse of any previously published proof. -/
def verifyFunction (reference : Syntax) (namespaceSegments : Array String)
    (function : String)
    (script? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) := none) :
    CommandElabM Unit := do
  let route := leaner.route.get (← getOptions)
  unless route == "native" do
    throwErrorAt reference m!"legacy verification route `{route}` is disabled; \
      use `native` and migrate unsupported computations"
  if script?.isSome then
    throwErrorAt reference "native verification consumes computation certificates, not row scripts"
  let input ← prepareVerification reference namespaceSegments function
  let saved ← get
  try
    discard <| ensureTypedVerificationTheorem reference namespaceSegments function
      input.namespaceIndex input.functionIndex input.unitDefinition input.semanticsEq
      input.preparedUnit input.preparedNs input.preparedDeclaration
      input.generated input.artifacts input.twins script?
    let base := (← getCurrNamespace) ++ Name.str (pathName namespaceSegments) function
    withRef reference <| requireNativeArtifacts base
  catch failure =>
    modify fun state => { saved with messages := state.messages }
    throw failure

/-- Expose generated contracts, representations and execution agreement for
an explicitly supplied native computation. This does not verify a contract. -/
syntax (name := leanerPrepareCommand) "#leaner_prepare" leanerPath : command

@[command_elab leanerPrepareCommand]
def elabLeanerPrepare : CommandElab := fun stx => do
  let path := stx[1]
  let (segments, function, _, _, _, _, _) ← resolvePath path
  discard <| prepareVerification path segments function

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

/-- The optional authored script of a `verify` form. -/
def scriptOfOptional (optional : Syntax) :
    Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) :=
  match optional.getArgs with
  | #[_, script] => some ⟨script⟩
  | _ => none

/-- Elaborate one in-module `verify` item once its module is registered. -/
def elabVerifyItem (namespaceSegments : Array String) (item : Syntax) :
    CommandElabM Unit := do
  let some identifier := item[1]? | throwErrorAt item "expected a function name"
  let function := identifier.getId.toString (escape := false)
  verifyFunction identifier namespaceSegments function
    (scriptOfOptional (item[2]?.getD .missing))

/-- Verify one function of a registered unit from outside its module, with
an optional authored proof script; the in-module `verify` item is the
normal spelling. -/
syntax (name := leanerVerifyCommand)
  "#leaner_verify" leanerPath ("by" Lean.Parser.Tactic.tacticSeq)? : command

@[command_elab leanerVerifyCommand]
def elabLeanerVerify : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let segments := pathSegments pathSyntax
  unless segments.size ≥ 2 do
    throwErrorAt pathSyntax "a verification path must name a namespace and a function"
  let function := segments[segments.size - 1]!
  verifyFunction pathSyntax segments.pop function
    (scriptOfOptional (stx[2]?.getD .missing))

/-- Check native computation/VC artifacts, not merely the existence of the
typed wrapper theorem (which the retiring routes also generate). -/
syntax (name := leanerRequireNativeCommand) "#leaner_require_native" leanerPath : command


@[command_elab leanerRequireNativeCommand]
def elabLeanerRequireNative : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let (namespaceSegments, function, _, _, _, _, _) ← resolvePath pathSyntax
  let base := (← getCurrNamespace) ++ Name.str (pathName namespaceSegments) function
  withRef pathSyntax <| requireNativeArtifacts base

/-- Audit every successfully verified source function declared in this file.
Imported declarations are excluded by the environment's current-module stage;
an omitted per-target assertion cannot hide a legacy-backed proof. -/
syntax (name := leanerRequireNativeAllCommand) "#leaner_require_native_all" : command

@[command_elab leanerRequireNativeAllCommand]
def elabLeanerRequireNativeAll : CommandElab := fun _ => do
  let env ← getEnv
  let bases : Array Name := env.constants.foldStage2 (fun bases name _ =>
    if name.getString! == "typedVerified" then bases.push name.getPrefix else bases) #[]
  unless !bases.isEmpty do throwError "native audit found no verified source functions in this file"
  for base in bases.qsort Name.quickLt do requireNativeArtifacts base

/-- The in-module `verify` items of a namespace command, in source order. -/
private partial def verifyItems (stx : Syntax) : Array Syntax :=
  if stx.isOfKind ``leanerVerifyItem then #[stx]
  else stx.getArgs.flatMap verifyItems

/-- Elaborate a Leaner namespace command and then its in-module `verify`
items: the namespace registers first, and each item proves its function
against the generated contract exactly as `#leaner_verify` does.  This
registration shadows the plain namespace elaborator, which stays the
entry point for every consumer that does not verify. -/
@[command_elab leanerNamespaceCommand, command_elab leanerMoveModuleCommand,
  command_elab leanerRustNamespaceCommand]
def elaborateNamespaceWithVerification : CommandElab := fun stx => do
  LeanerLang.elaborateNamespace stx
  let items := verifyItems stx
  if items.isEmpty then return
  let some pathSyntax := stx.getArgs.find? (·.isOfKind ``leanerPathSyntax)
    | throwErrorAt stx "a Leaner namespace requires a path"
  /- Every item reports on its own: a function that fails to verify does
  not hide the verdicts of the functions after it. -/
  for item in items do
    try elabVerifyItem (pathSegments pathSyntax) item
    catch error => logException error

end LeanerLang.Contract
