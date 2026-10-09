-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Elab
import LeanerLang.Frame
import LeanerLang.Modifiers
import LeanerLang.Perf
import LeanerLang.Options
import LeanerLang.Quote
import LeanerLang.Registry
import LeanerLang.SpecTypes
import LeanerLang.Syntax
import LeanerLang.ValueRep
import LeanerIR.Proofs.Denote.Compile
import LeanerIR.Proofs.Invocation
import LeanerIR.Proofs.Meaning
import LeanerIR.Proofs.Behavior
import LeanerIR.Proofs.IntegerArithmetic
import LeanerIR.Proofs.Denote.Types
import LeanerIR.Proofs.Denote.SnapshotValue
import LeanerIR.Proofs.Denote.StoredValueInvariants
import LeanerIR.Proofs.Maps

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
open LeanerIR.Proofs (ObligationRange)
open LeanerLang.Quote

/-- Move's execution-failure code describes a runtime error, not a user
abort carrying an operand or result of the failed operation. -/
@[simp] def abortCodeMatches (failure : LeanerIR.ThrowKind × Array RuntimeValue)
    (code : Int) : Prop :=
  match failure with
  | (.abort, payload) => payload = #[.integer code]
  | (.profile value, _) =>
      (value = { profile := .move, tag := "runtime.arithmetic_error" } ∨
       value = { profile := .move, tag := "runtime.vector_error" }) ∧ code = -1
  | _ => False

/-- What a state label denotes: a memory, and the value of each local at
that state — a mutable reference parameter's own, the others as they are. -/
private structure LabelState where
  memory : Lean.Expr
  locals : Array (Option Lean.Expr)

/-- The shared IR type node; `LeanerLang.Ty` is the surface type. -/
private abbrev IrTy := LeanerIR.Ty

/-- A member of the group of recursive specification functions a
definition is built for, as a call to it reads it: the recursion hypothesis
over its bundles below the definition's argument, its measure, and its
result type. -/
structure RecursiveMember where
  reference : LeanerIR.QualifiedRef
  recurse : Lean.Expr
  measure : Lean.Expr
  /-- The member's result type, for the value of a guarded call. -/
  resultType : Lean.Expr

/-- The definition of a recursive specification function under
construction: its bundled argument, its measure, and the members of its
group, itself among them. -/
structure RecursiveDefinition where
  reference : LeanerIR.QualifiedRef
  argument : Lean.Expr
  measure : Lean.Expr
  members : Array RecursiveMember
  /-- Whether a recursive call whose decrease the path conditions do not
  prove is guarded by it, taking an arbitrary value where the measure does
  not descend, rather than rejected. -/
  guarded : Bool := false

/-! ## Quotation of native types -/

mutual
/-- The literal of a native type.  An enum's distinctness witness is decided. -/
partial def quoteNTy : LeanerIR.Proofs.Denote.NTy → MetaM Lean.Expr
  | .unit => return mkConst ``LeanerIR.Proofs.Denote.NTy.unit
  | .bool => return mkConst ``LeanerIR.Proofs.Denote.NTy.bool
  | .int width signed => return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.int) #[toExpr width, toExpr signed]
  | .address => return mkConst ``LeanerIR.Proofs.Denote.NTy.address
  | .signer => return mkConst ``LeanerIR.Proofs.Denote.NTy.signer
  | .string => return mkConst ``LeanerIR.Proofs.Denote.NTy.string
  | .bytes => return mkConst ``LeanerIR.Proofs.Denote.NTy.bytes
  | .tuple elements => return mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.tuple) (← quoteRow elements)
  | .struct source arguments fields =>
      return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.struct)
        #[toExpr source, ← quoteRow arguments, ← quoteRow fields]
  | .enum source arguments names rows _ => do
      let namesExpr := toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.enum)
        #[toExpr source, ← quoteRow arguments, namesExpr, ← quoteRows rows, distinct]
  | .vector element => return mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.vector) (← quoteNTy element)
  | .ref referent => return mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.ref) (← quoteNTy referent)
  | .param index => return mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.param) (toExpr index)
  | .function parameters shared results =>
      return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.function)
        #[← quoteRow parameters, toExpr shared, ← quoteRow results]

partial def quoteRow : LeanerIR.Proofs.Denote.NRow → MetaM Lean.Expr
  | .nil => return mkConst ``LeanerIR.Proofs.Denote.NRow.nil
  | .cons τ rest => return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NRow.cons) #[← quoteNTy τ, ← quoteRow rest]

partial def quoteRows : LeanerIR.Proofs.Denote.NRows → MetaM Lean.Expr
  | .nil => return mkConst ``LeanerIR.Proofs.Denote.NRows.nil
  | .cons fields rest => return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NRows.cons) #[← quoteRow fields, ← quoteRows rest]
end

def quoteShape : LeanerIR.Proofs.Denote.ResultShape → MetaM Lean.Expr
  | .none => return mkConst ``LeanerIR.Proofs.Denote.ResultShape.none
  | .one τ => return mkApp (mkConst ``LeanerIR.Proofs.Denote.ResultShape.one) (← quoteNTy τ)

/-! ## Typed memory in clauses

A clause reads global memory as the encoding of the typed value a slot holds
(`designs/static-memory.md`): nothing decodes. -/

/-- The unit a frame belongs to. -/
def frameUnit (frame : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfR (← inferType frame)
  unless type.isAppOfArity ``LeanerIR.Proofs.Denote.Skolems 1 do
    throwError "internal: {frame} is not a frame"
  return type.appArg!

/-- The validated unit an executable unit executes. -/
def executableUnit (executable : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfR (← inferType executable)
  unless type.isAppOfArity ``LeanerIR.Validation.ExecutableUnit 1 do
    throwError "internal: {executable} is not an executable unit"
  return type.appArg!

/-- The type of the executable units of a unit. -/
def executableType (unit : Lean.Expr) : Lean.Expr :=
  mkApp (mkConst ``LeanerIR.Validation.ExecutableUnit) unit

/-- The carriers of a frame. -/
def frameCarriers (frame : Lean.Expr) : MetaM Lean.Expr := do
  return mkApp2 (mkConst ``LeanerIR.Proofs.Denote.Skolems.toCarriers) (← frameUnit frame) frame

/-- What a frame's type parameters stand for. -/
def frameTypes (frame : Lean.Expr) : MetaM Lean.Expr := do
  return mkApp2 (mkConst ``LeanerIR.Proofs.Denote.Skolems.type) (← frameUnit frame) frame

/-- The resource type a type identifier of a namespace denotes, at a frame,
with its native type. -/
def quoteResource (skolems : Lean.Expr) (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (typeId : TypeId) (typeArguments : Option LeanerIR.Proofs.Denote.NRow := none) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let some resource := LeanerIR.Proofs.Denote.resourceOf unit namespaceId typeId
    | throwError "a storage clause names a type that is not a resource"
  -- An expanded generic specification function reads the resources of its
  -- type arguments.
  let resource := match typeArguments with
    | some row => resource.subst row
    | none => resource
  let nativeType ← quoteNTy resource.type
  return (mkAppN (mkConst ``LeanerIR.Proofs.Denote.Skolems.resource)
    #[← frameUnit skolems, skolems, nativeType, ← quoteRow resource.arguments], nativeType)

/-- A memory's slot at a resource type and a runtime key value. -/
def memorySlot (memory resource key : Lean.Expr) : MetaM Lean.Expr :=
  return mkApp2 memory resource (← mkAppM ``LeanerIR.RuntimeValue.storageKey #[key])

/-- The encoding of what a memory slot holds, read at a frame; `unit` where
it holds nothing. -/
def slotEncoding (skolems nativeType resource slot : Lean.Expr) : MetaM Lean.Expr := do
  let unit ← frameUnit skolems
  let carriers ← frameCarriers skolems
  let carrier := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.ResourceType.carrier) unit resource
  let encode ← withLocalDeclD `value carrier fun value =>
    mkLambdaFVars #[value] (mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.encode)
      #[carriers, nativeType,
        mkAppN (mkConst ``LeanerIR.Proofs.Denote.Skolems.ofRuntime) #[unit, skolems, nativeType, value]])
  let mapped := mkAppN (mkConst ``Option.map [Level.zero, Level.zero])
    #[carrier, mkConst ``LeanerIR.RuntimeValue, encode, slot]
  return mkAppN (mkConst ``Option.getD [Level.zero])
    #[mkConst ``LeanerIR.RuntimeValue, mapped, mkConst ``LeanerIR.RuntimeValue.unit]

/-- Whether a memory slot holds a value. -/
def slotPresent (slot : Lean.Expr) : MetaM Lean.Expr := do
  mkEq (← mkAppM ``Option.isSome #[slot]) (mkConst ``Bool.true)

open LeanerIR.Proofs.Denote (NRow Weave) in
def quoteWeave : {full captured supplied : NRow} → Weave full captured supplied →
    MetaM Lean.Expr
  | _, _, _, .nil => return mkConst ``Weave.nil
  | .cons τ full, .cons _ captured, supplied, .captured rest =>
      return mkAppN (mkConst ``Weave.captured)
        #[← quoteNTy τ, ← quoteRow full, ← quoteRow captured, ← quoteRow supplied,
          ← quoteWeave rest]
  | .cons τ full, captured, .cons _ supplied, .supplied rest =>
      return mkAppN (mkConst ``Weave.supplied)
        #[← quoteNTy τ, ← quoteRow full, ← quoteRow captured, ← quoteRow supplied,
          ← quoteWeave rest]

/-- A free label's defining operation and its lexical let bindings. -/
private structure StateDefinition where
  label : Nat
  operation : ExprId
  bindings : Array (LeanerIR.PatternId × ExprId)

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
  /-- The states the clause's state labels denote, by label (a name of the
  unit): those bound by quantifiers over the state domain. -/
  stateLabels : List (Nat × LabelState) := []
  /-- Free definitions denote memory expressions, never assumptions. -/
  labelDefinitions : Array StateDefinition := #[]
  resolvingLabels : List Nat := []
  /-- Contract defaults, preserved under ranges and `old`. -/
  labelEntry : Option LabelState := none
  labelExit : Option LabelState := none
  /-- The expression whose memory range the clause's states already follow:
  a ranged operation is translated once more under the range, then as an
  unranged one. -/
  rangeApplied : Option ExprId := none
  /-- Lean binder each local denotes under `spec.old` inside a state anchor,
  and the state the anchor saved: in a loop invariant, the loop's entry. -/
  anchorLocals : Array (Option Lean.Expr) := #[]
  anchorState : Option Lean.Expr := none
  /-- The locals and state each labeled anchor saved, read in preference to
  the anchor above: in an in-body assertion, those its anchors recorded. -/
  labeledAnchors : Array (Nat × Array (Option Lean.Expr) × Lean.Expr) := #[]
  /-- Typed twins of the unit's struct declarations. -/
  twins : Array SpecTypes.TwinInfo := #[]
  /-- Native representation environment of a generic contract. -/
  carrier : Option Lean.Expr := none
  codecs : Option Lean.Expr := none
  /-- What the family's type parameters stand for (`Skolems.type`), which
  names the type arguments of an opaque specification function. -/
  types : Option Lean.Expr := none
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
  /-- The native types of the same arguments, for the type arguments of an
  opaque specification function the expansion applies. -/
  typeArgumentTypes : Array (Option LeanerIR.Proofs.Denote.NTy) := #[]
  /-- Inside the body of a recursive specification function's definition. -/
  definition : Option RecursiveDefinition := none
  /-- The conditions on the path to the current position, which prove a
  recursive call's measure smaller. -/
  facts : Array Lean.Expr := #[]
  /-- The executable unit, which a behavioral predicate reads a function
  value's meaning from; set where the contract takes it. -/
  executable : Option Lean.Expr := none
  /-- The table of declared preconditions `requires_of` reads; set where the
  contract takes it. -/
  requiresTable : Option Lean.Expr := none
  /-- Inside a quantifier: a lemma instance there is its implication. -/
  lemmaQuantified : Bool := false
  /-- A stored declaration may read a vector field's physical length without
  observing its elements. This does not license physical equality or content
  observations of Tables hidden in generic fields. -/
  physicalInvariantLengths : Bool := false

/-- The skolem instance the clauses are stated over: the one whose
`Skolems.type` the context's `types` is. -/
private def Context.skolems? (context : Context) : Option Lean.Expr := do
  let types ← context.types
  if types.isAppOfArity ``LeanerIR.Proofs.Denote.Skolems.type 2 then some types.appArg! else none

/-- The frame a clause's storage reads resolve resource types at: the
contract's, the runtime frame where a contract states none. -/
private def Context.frame (context : Context) : MetaM Lean.Expr := do
  if let some frame := context.skolems? then return frame
  let some executable := context.executable
    | throwError "internal: a clause reads storage without a frame or a unit"
  return mkApp (mkConst ``LeanerIR.Proofs.Denote.Skolems.runtime) (← executableUnit executable)

/-- The unit a clause's memory and frame are at. -/
private def Context.unitExpr (context : Context) : MetaM Lean.Expr := do
  frameUnit (← context.frame)

/-- A storage clause's memory slot at a resource type identifier and a
runtime key value, with the resource type's native type. -/
private def Context.slot (context : Context) (memory : Lean.Expr) (typeId : TypeId)
    (key : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr × Lean.Expr) := do
  let typeArguments ← if context.typeArgumentTypes.isEmpty then pure none else
    let some row := context.typeArgumentTypes.toList.mapM id
      | throwError "a type argument of a specification function reading storage has no \
          native type"
    pure (some (LeanerIR.Proofs.Denote.NRow.ofList row))
  let (resource, nativeType) ← quoteResource (← context.frame) context.unit context.namespaceId typeId
    typeArguments
  return (← memorySlot memory resource key, resource, nativeType)

/-- The type a specification type denotes: a parameter of an expanded
specification function is its argument. -/
private def Context.typeOf? (context : Context) (typeId : TypeId) : Option IrTy := do
  let ty ← context.unit.tables.types[typeId.index]?
  match ty with
  | .typeParameter index => (context.typeArguments[index]?).map (·.1) <|> pure ty
  | _ => pure ty

/-- The type a specification reads a value of type `typeId` at:
specifications see through references, so a reference's values are its
referent's. -/
private partial def specTypeId (unit : ValidatedUnit) (typeId : TypeId) : TypeId :=
  match unit.tables.types[typeId.index]? with
  | some (.reference reference) => specTypeId unit reference.referent
  | _ => typeId

/-- The type of a specification value, through references. -/
private def Context.valueTypeOf? (context : Context) (typeId : TypeId) : Option IrTy :=
  context.typeOf? (specTypeId context.unit typeId)

/-- The native type of a specification type, with the parameters replaced
by the expansion's type arguments. -/
private def Context.ntyOf? (context : Context) (typeId : TypeId) :
    Option LeanerIR.Proofs.Denote.NTy := do
  let ty ← context.unit.tables.types[typeId.index]?
  match ty with
  | .typeParameter index =>
      -- The family's own parameter outside an expansion, the expansion's
      -- type argument inside one.
      if context.typeArgumentTypes.isEmpty then some (.param index)
      else (context.typeArgumentTypes[index]?).join
  | _ =>
      let nty ← LeanerIR.Proofs.Denote.ntyOf context.unit context.namespaceId typeId
      if context.typeArgumentTypes.isEmpty then some nty else
        let row ← context.typeArgumentTypes.toList.mapM id
        some (nty.subst (LeanerIR.Proofs.Denote.NRow.ofList row))

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
  deriving BEq, Inhabited

private def domainOf (ty : IrTy) : Domain :=
  match ty with
  | .integer _ _ => .integer
  | .bool => .boolean
  | .string => .text ``RuntimeValue.string
  | .address => .text ``RuntimeValue.address
  | .signer => .text ``RuntimeValue.signer
  | _ => .aggregate

/-- The domain of a type of a declaration's signature, through references;
`role` names the position in the error for an unknown type. -/
private def signatureDomain (unit : ValidatedUnit) (typeId : TypeId) (role : String) :
    MetaM Domain := do
  let some ty := unit.tables.types[(specTypeId unit typeId).index]?
    | throwError "{role} has an unknown type"
  pure (domainOf ty)

/-- The values of a bounded integer type, as a binder's membership. -/
private def boundedMembership? : IrTy → Option (Lean.Expr → MetaM Lean.Expr)
  | .integer (.bits width) signed => some fun value => pure (mkApp3
      (mkConst ``LeanerIR.IntegerValueFits)
      (mkApp (mkConst ``LeanerIR.IntWidth.bits) (toExpr width)) (toExpr signed) value)
  | _ => none

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
still reduce to their fields in one rewrite. `grind` takes them too, for the
constructor images its case splits expose. -/
@[simp, grind =] theorem runtimeFieldNominal
    (source : LeanerIR.StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) (index : Nat) :
    RuntimeValue.field (.nominal source variant fields) index =
      fields[index]?.getD .unit := rfl

/- Enum invariant matches immediately project constructor payloads.  Keep
these literal-array rows ahead of the generic nominal row so arithmetic sees
the payload itself instead of an intermediate `getElem?` application. -/
@[simp, grind =] theorem runtimeFieldNominalZero
    (source : LeanerIR.StructHandle) (variant : Option String)
    (first : RuntimeValue) (rest : List RuntimeValue) :
    RuntimeValue.field (.nominal source variant (Array.mk (first :: rest))) 0 =
      first := rfl

@[simp, grind =] theorem runtimeFieldNominalOne
    (source : LeanerIR.StructHandle) (variant : Option String)
    (first second : RuntimeValue) (rest : List RuntimeValue) :
    RuntimeValue.field
        (.nominal source variant (Array.mk (first :: second :: rest))) 1 =
      second := rfl

@[simp, grind =] theorem runtimeAsIntInteger (value : Int) :
    RuntimeValue.asInt (.integer value) = value := rfl

@[simp, grind =] theorem runtimeAsBoolBool (value : Bool) :
    RuntimeValue.asBool (.bool value) = value := rfl

@[simp, grind =] theorem runtimeAsStringString (value : String) :
    RuntimeValue.asString (.string value) = value := rfl

@[simp, grind =] theorem runtimeAsStringAddress (value : String) :
    RuntimeValue.asString (.address value) = value := rfl

@[simp, grind =] theorem runtimeAsStringSigner (value : String) :
    RuntimeValue.asString (.signer value) = value := rfl

/-- Total logical vector update used by generated clauses.  As with the
other specification projections, an ill-shaped value is junk; successful
execution and the authored bounds condition rule that case out. -/
def updateVector (value : RuntimeValue) (index : Nat)
    (replacement : RuntimeValue) : RuntimeValue :=
  match value with
  | .vector elements => .vector (elements.set! index replacement)
  | _ => .unit

@[simp] theorem updateVector_vector (elements : Array RuntimeValue) (index : Nat)
    (replacement : RuntimeValue) :
    updateVector (.vector elements) index replacement =
      .vector (elements.setIfInBounds index replacement) := rfl

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

/-- Total logical vector slice: the elements from `lower` up to `upper`,
cut at the vector's end. -/
@[simp] def sliceVector (value : RuntimeValue) (lower upper : Nat) : RuntimeValue :=
  match value with
  | .vector elements => .vector (elements.extract lower upper)
  | _ => .unit

theorem pushVector_vector (elements : Array RuntimeValue) (element : RuntimeValue) :
    pushVector (.vector elements) element = .vector (elements.push element) := rfl

theorem concatVector_vector (left right : Array RuntimeValue) :
    concatVector (.vector left) (.vector right) = .vector (left ++ right) := rfl

theorem sliceVector_vector (elements : Array RuntimeValue) (lower upper : Nat) :
    sliceVector (.vector elements) lower upper = .vector (elements.extract lower upper) := rfl

/-- Every carrier has decidable equality, so a clause can decide a
proposition about carrier values, as when a Boolean argument of a typed
contract is passed to an expanded specification function. -/
instance [LeanerIR.Proofs.Denote.Carriers] (τ : LeanerIR.Proofs.Denote.NTy) :
    DecidableEq τ.carrier :=
  τ.decEq

/-- A type argument of a specification function: a native type, or the
mathematical integers (`num`), which no native type is. -/
inductive SpecTypeArgument where
  | native (type : LeanerIR.Proofs.Denote.NTy)
  | integer

/-- The meaning of a specification function without a body: a fixed
function about which nothing is known, named by the declaration's qualified
spelling and applied to its encoded arguments. -/
opaque opaqueSpec (name : String) (typeArguments : List SpecTypeArgument)
    (result : Type) [Inhabited result] (arguments : List RuntimeValue) : result

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

@[simp, grind =] theorem lengthVector_vector (elements : Array RuntimeValue) :
    lengthVector (.vector elements) = (elements.size : Int) := rfl

/-- The length of an aggregate vector does not require encoding its elements.
The aggregate constructor matters: a Table's physical metadata is not its
logical collection length. -/
@[lir_denote_norm↓] theorem lengthVector_physical
    (values : List LeanerIR.Proofs.Denote.SnapshotValue.Value) :
    lengthVector (LeanerIR.Proofs.Denote.SnapshotValue.Value.aggregate .vector values).physical =
      (values.length : Int) := by
  simp [lengthVector, LeanerIR.Proofs.Denote.SnapshotValue.Value.physical]

/-- The elements of a runtime vector, which a quantifier over the vector
ranges over. -/
def elementsVector (value : RuntimeValue) : List RuntimeValue :=
  match value with
  | .vector elements => elements.toList
  | _ => []

@[simp, grind =] theorem elementsVector_vector (elements : Array RuntimeValue) :
    elementsVector (.vector elements) = elements.toList := rfl

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

/-- Payload offset of a specification field update, resolved at translation. -/
@[simp] def updateFieldIndex (variant : Option String) : List (Option String × Nat) → Option Nat
  | [] => none
  | (candidate, index) :: rest =>
      if candidate == variant then some index else updateFieldIndex variant rest

/-- Functional field replacement preserves the owner and variant. A missing
field has no value, like a missing specification field selection. -/
@[simp, irreducible] def updateNominalField (value : RuntimeValue) (owner : LeanerIR.StructHandle)
    (choices : List (Option String × Nat)) (replacement : RuntimeValue) : RuntimeValue :=
  match value with
  | .nominal actual variant fields =>
      if actual == owner then
        match updateFieldIndex variant choices with
        | some index => if index < fields.size then
            .nominal actual variant (fields.set! index replacement) else .unit
        | none => .unit
      else .unit
  | _ => .unit

/-- Normalize optional reads before inspecting the runtime constructor. This
keeps a missing resource as `unit` without splitting a typed resource read into
unrelated runtime-value cases during contract reduction. -/
@[simp high, lir_denote_norm high] theorem updateNominalField_read {α : Type}
    (entry : Option α) (encode : α → RuntimeValue) (owner : LeanerIR.StructHandle)
    (choices : List (Option String × Nat)) (replacement : RuntimeValue) :
    updateNominalField ((entry.map encode).getD .unit) owner choices replacement =
      (entry.map fun value => updateNominalField (encode value) owner choices replacement).getD
        .unit := by
  cases entry <;> simp only [Option.map_none, Option.map_some, Option.getD_none,
    Option.getD_some, updateNominalField]

@[lir_denote_norm] theorem updateNominalField_nominal
    (actual owner : LeanerIR.StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) (choices : List (Option String × Nat))
    (replacement : RuntimeValue) :
    updateNominalField (.nominal actual variant fields) owner choices replacement =
      (if actual == owner then
        match updateFieldIndex variant choices with
        | some index => if index < fields.size then
            .nominal actual variant (fields.set! index replacement) else .unit
        | none => .unit
      else .unit) := by
  unfold updateNominalField
  rfl

@[lir_denote_norm] theorem updateNominalField_unit (owner : LeanerIR.StructHandle)
    (choices : List (Option String × Nat)) (replacement : RuntimeValue) :
    updateNominalField .unit owner choices replacement = .unit := by
  unfold updateNominalField
  rfl

/-- Total enum-variant membership used by generated specification clauses. -/
@[simp] def testVariants (value : RuntimeValue)
    (owner : LeanerIR.StructHandle)
    (variants : Array String) : Bool :=
  match value with
  | .nominal actual (some variant) _ =>
      actual == owner && variantMember variant variants.toList
  | _ => false

theorem selectVariantField_nominal (actual owner : LeanerIR.StructHandle) (variant : String)
    (fields : Array RuntimeValue) (variants : Array (String × Nat)) :
    selectVariantField (.nominal actual (some variant) fields) owner variants =
      if actual == owner then
        match variantIndex variant variants.toList with
        | some index => fields[index]?.getD .unit
        | none => .unit
      else .unit := rfl

theorem selectVariantField_nominal_none (actual owner : LeanerIR.StructHandle)
    (fields : Array RuntimeValue) (variants : Array (String × Nat)) :
    selectVariantField (.nominal actual none fields) owner variants = .unit := rfl

theorem testVariants_nominal (actual owner : LeanerIR.StructHandle) (variant : String)
    (fields : Array RuntimeValue) (variants : Array String) :
    testVariants (.nominal actual (some variant) fields) owner variants =
      (actual == owner && variantMember variant variants.toList) := rfl

theorem testVariants_nominal_self (owner : LeanerIR.StructHandle)
    (variant : String) (fields : Array RuntimeValue) (variants : Array String) :
    testVariants (.nominal owner (some variant) fields) owner variants =
      variantMember variant variants.toList := by
  simp [testVariants]

/-- The value of a three-variant ordering enum at an order: its first,
second, or third variant for less, equal, greater, as `std::cmp::compare`
returns it. -/
def orderingVariant (owner : LeanerIR.StructHandle) (less equal greater : String) :
    Ordering → RuntimeValue
  | .lt => .nominal owner (some less) #[]
  | .eq => .nominal owner (some equal) #[]
  | .gt => .nominal owner (some greater) #[]

/- The variant at a decided order. An undecided order keeps the variant
folded, so that the order's operands can still be read at their type. -/
@[simp, lir_denote_norm] theorem orderingVariant_lt (owner : LeanerIR.StructHandle)
    (less equal greater : String) :
    orderingVariant owner less equal greater .lt = .nominal owner (some less) #[] := rfl
@[simp, lir_denote_norm] theorem orderingVariant_eq (owner : LeanerIR.StructHandle)
    (less equal greater : String) :
    orderingVariant owner less equal greater .eq = .nominal owner (some equal) #[] := rfl
@[simp, lir_denote_norm] theorem orderingVariant_gt (owner : LeanerIR.StructHandle)
    (less equal greater : String) :
    orderingVariant owner less equal greater .gt = .nominal owner (some greater) #[] := rfl

/-- An ordering variant is a given variant exactly at the order naming it.
Stated before the order is read, so that every comparison, of integers or
of a map's keys read at their type, becomes the order's relation. -/
@[lir_denote_norm ↓] theorem orderingVariant_eq_nominal (owner source : LeanerIR.StructHandle)
    (less equal greater variant : String) (order : Ordering) :
    orderingVariant owner less equal greater order = .nominal source (some variant) #[] ↔
      owner = source ∧ ((order = .lt ∧ less = variant) ∨ (order = .eq ∧ equal = variant) ∨
        (order = .gt ∧ greater = variant)) := by
  cases order <;> simp [orderingVariant, eq_comm]

@[lir_denote_norm ↓] theorem nominal_eq_orderingVariant (owner source : LeanerIR.StructHandle)
    (less equal greater variant : String) (order : Ordering) :
    .nominal source (some variant) #[] = orderingVariant owner less equal greater order ↔
      owner = source ∧ ((order = .lt ∧ less = variant) ∨ (order = .eq ∧ equal = variant) ∨
        (order = .gt ∧ greater = variant)) := by
  rw [eq_comm, orderingVariant_eq_nominal]

/-- A variant equated to a conditional variant holds in the branch naming it. -/
@[lir_denote_norm] theorem nominal_eq_ite (source : LeanerIR.StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) (condition : Prop) [Decidable condition]
    (yes no : RuntimeValue) :
    (RuntimeValue.nominal source variant fields = if condition then yes else no) ↔
      (condition ∧ RuntimeValue.nominal source variant fields = yes) ∨
        (¬condition ∧ RuntimeValue.nominal source variant fields = no) := by
  by_cases h : condition <;> simp [h]

@[lir_denote_norm] theorem ite_eq_nominal (source : LeanerIR.StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) (condition : Prop) [Decidable condition]
    (yes no : RuntimeValue) :
    ((if condition then yes else no) = RuntimeValue.nominal source variant fields) ↔
      (condition ∧ yes = RuntimeValue.nominal source variant fields) ∨
        (¬condition ∧ no = RuntimeValue.nominal source variant fields) := by
  by_cases h : condition <;> simp [h]

/-- The ordering variant at the order of two integers, by their comparison. -/
@[lir_denote_norm] theorem orderingVariant_compare_int (owner : LeanerIR.StructHandle)
    (less equal greater : String) (a b : Int) :
    orderingVariant owner less equal greater (compare a b) =
      if a < b then .nominal owner (some less) #[]
      else if a = b then .nominal owner (some equal) #[]
      else .nominal owner (some greater) #[] := by
  simp only [compare, compareOfLessAndEq]
  split <;> (try split) <;> rfl

private def typeOfExpr? (context : Context) (id : ExprId) : Option IrTy := do
  let expression ← context.ns.expressions[id.index]?
  context.valueTypeOf? expression.typeId

private def describeType (ty : IrTy) : String :=
  toString (repr ty)

/-- The runtime identity of the nominal declaration an operation names. -/
private def structHandleOf (context : Context) (reference : LeanerIR.QualifiedRef) :
    MetaM LeanerIR.StructHandle := do
  let some handle := LeanerIR.SemanticOperations.resolveStruct?
      context.unit context.namespaceId reference
    | throwError "a nominal operation's declaration does not resolve"
  return handle

/-- The declaration of a nominal type, with its runtime identity and owning
namespace. -/
def nominalDeclaration? (unit : ValidatedUnit) (ty : IrTy) :
    Option (LeanerIR.StructHandle × ValidatedNamespace × LeanerIR.StructDecl) := do
  let .nominal name _ := ty | none
  let qualified ← unit.tables.names[name.index]?
  let ns ← unit.namespaces[qualified.namespaceId.index]?
  let structId ← ns.structs.findIdx? (·.name == name)
  let declaration ← ns.structs[structId]?
  some ({ namespaceId := qualified.namespaceId, structId }, ns, declaration)

/-- The structural order of two runtime values, as `std::cmp::compare`
and the `compare` primitive observe it. -/
private def structuralOrderTerm (context : Context) (left right : Lean.Expr) :
    MetaM Lean.Expr := do
  let rank := mkApp (mkConst ``LeanerIR.SemanticOperations.valueRanks)
    (toExpr context.ns.orders)
  return mkAppN (mkConst ``LeanerIR.RuntimeValue.order) #[rank, left, right]

/-- The value of the three-variant ordering enum `ty` at `order`. -/
private def orderingValue (context : Context) (ty : IrTy) (order : Lean.Expr) :
    MetaM Lean.Expr := do
  let some (owner, _, declaration) := nominalDeclaration? context.unit ty
    | throwError "a structural order returns an ordering enum"
  let names := declaration.variants.map fun variant =>
    ((context.unit.tables.names[variant.name.index]?).map (·.name)).getD ""
  let #[less, equal, greater] := names
    | throwError "the result of a structural order has three variants"
  return mkAppN (mkConst ``LeanerLang.Contract.orderingVariant)
    #[toExpr owner, toExpr less, toExpr equal, toExpr greater, order]

/-! ## Intrinsic maps

An owner of an intrinsic map is read through the map model
(`designs/intrinsic-maps.md`): its layout, and its discipline, ordered when
the owner binds a role that enumerates it in key order. -/

/-- The roles of an intrinsic map that enumerate it in key order. Insertion
position roles deliberately do not select the ordered discipline. -/
private def orderingRoles : Array String := #["map_spec_key_at", "map_spec_rank",
  "map_borrow_front", "map_borrow_back", "map_front_key", "map_back_key", "map_pop_front",
  "map_pop_back", "map_prev_key", "map_next_key"]

/-- A map owner's model, quoted: its layout and discipline. -/
private structure MapModel where
  layout : Lean.Expr
  discipline : Lean.Expr

/-- The model of an intrinsic map owner holding its entries in one vector of
two-field entries, a plain struct or the single variant of an enum. -/
private def mapModel? (unit : ValidatedUnit) (ty : IrTy) : Option MapModel := do
  let (owner, ns, declaration) ← nominalDeclaration? unit ty
  let intrinsic ← ns.intrinsics.find? fun intrinsic =>
    intrinsic.model == "map" && intrinsic.owner == declaration.name
  let (variant, fields) ← match declaration.variants.toList with
    | [] => some (none, declaration.fields)
    | [variant] => some (some (((unit.tables.names[variant.name.index]?).map (·.name)).getD ""),
        variant.fields)
    | _ => none
  let [field] := fields.toList | none
  let .vector element _ ← unit.tables.types[field.type.typeId.index]? | none
  let entryType ← unit.tables.types[element.index]?
  let (entry, _, entryDeclaration) ← nominalDeclaration? unit entryType
  guard (entryDeclaration.variants.isEmpty && entryDeclaration.fields.size == 2)
  let ordered := (intrinsic.executableBindings ++ intrinsic.specBindings).any
    (orderingRoles.contains ·.role)
  let rank := mkApp (mkConst ``LeanerIR.SemanticOperations.valueRanks) (toExpr ns.orders)
  let discipline := if ordered then mkApp (mkConst ``LeanerIR.Maps.Discipline.ordered) rank
    else mkConst ``LeanerIR.Maps.Discipline.sequence
  let layout := mkApp3 (mkConst ``LeanerIR.Maps.Layout.mk) (toExpr owner) (toExpr variant)
    (toExpr entry)
  some { layout, discipline }

/-- A handle-backed map owner, independently of any synthetic entries layout. -/
private def tableModel? (unit : ValidatedUnit) (ty : IrTy) : Option LeanerIR.StructHandle := do
  let (owner, ns, declaration) ← nominalDeclaration? unit ty
  guard (declaration.variants.isEmpty && declaration.generics.size == 2)
  guard (ns.intrinsics.any fun intrinsic =>
    intrinsic.model == "map" && intrinsic.owner == declaration.name)
  let fields : List IrTy ← declaration.fields.toList.mapM fun field => unit.tables.types[field.type.typeId.index]?
  match fields with
  | [.address] | [.address, .integer (.bits 64) false] => some owner
  | _ => none

private def hasTableModel (unit : ValidatedUnit) : Bool :=
  unit.namespaces.any fun ns => ns.structs.any fun declaration =>
    (tableModel? unit (.nominal declaration.name #[])).isSome

mutual
private def snapshotType (unit : ValidatedUnit) : LeanerIR.Proofs.Denote.NTy → Bool
  | .struct source _ fields =>
      (LeanerIR.Proofs.Denote.SnapshotValue.tableOwner unit source &&
        LeanerIR.Proofs.Denote.TableMemory.handleFields fields) || snapshotRow unit fields
  | .tuple fields => snapshotRow unit fields
  | .enum _ _ _ rows _ => snapshotRows unit rows
  | .vector element | .ref element => snapshotType unit element
  | .param _ => hasTableModel unit
  | _ => false
private def snapshotRow (unit : ValidatedUnit) : LeanerIR.Proofs.Denote.NRow → Bool
  | .nil => false
  | .cons type rest => snapshotType unit type || snapshotRow unit rest
private def snapshotRows (unit : ValidatedUnit) : LeanerIR.Proofs.Denote.NRows → Bool
  | .nil => false
  | .cons fields rest => snapshotRow unit fields || snapshotRows unit rest
end

/-- The specification role a specification function plays for an intrinsic
map of its namespace, with the map's owner. -/
private def mapSpecRole? (ns : ValidatedNamespace) (reference : LeanerIR.QualifiedRef) :
    Option (String × LeanerIR.NameId) :=
  ns.intrinsics.findSome? fun intrinsic =>
    if intrinsic.model == "map" then
      (intrinsic.specBindings.find? (·.target == reference)).map (·.role, intrinsic.owner)
    else none

/-- The rank a map model's ordered discipline compares keys by. -/
private def MapModel.rank? (model : MapModel) : Option Lean.Expr :=
  if model.discipline.isAppOfArity ``LeanerIR.Maps.Discipline.ordered 1 then
    some (model.discipline.getArg! 0)
  else none

/-- The position variant of an iterator enum, with the enum's handle: the
variant holding one field, the position. -/
private def iteratorPosition? (unit : ValidatedUnit) (ty : IrTy) :
    Option (LeanerIR.StructHandle × String) := do
  let (owner, _, declaration) ← nominalDeclaration? unit ty
  let variant ← declaration.variants.find? (·.fields.size == 1)
  let name ← (unit.tables.names[variant.name.index]?).map (·.name)
  pure (owner, name)

/-- The position an iterator holds, as a specification reads its payload
field. -/
private def iteratorPosition (owner : LeanerIR.StructHandle) (variant : String)
    (iterator : Lean.Expr) : MetaM Lean.Expr := do
  let variants := #[(variant, (0 : Nat))]
  mkAppM ``LeanerIR.RuntimeValue.asInt
    #[← mkAppM ``LeanerLang.Contract.selectVariantField #[iterator, toExpr owner, toExpr variants]]

/-- Whether an iterator is at a position of a map: its position variant,
below the map's size. -/
private def iteratorInRange (owner : LeanerIR.StructHandle) (variant : String)
    (iterator map : Lean.Expr) : MetaM Lean.Expr := do
  let test ← mkEq (← mkAppM ``LeanerLang.Contract.testVariants
    #[iterator, toExpr owner, toExpr #[variant]]) (mkConst ``Bool.true)
  let position ← iteratorPosition owner variant iterator
  mkAppM ``And #[test, ← mkAppM ``LT.lt #[position, ← mkAppM ``LeanerIR.Maps.size #[map]]]

/-- The key at an iterator's position. -/
private def iteratorKey (owner : LeanerIR.StructHandle) (variant : String)
    (iterator map : Lean.Expr) : MetaM Lean.Expr := do
  mkAppM ``LeanerIR.Maps.keyAt #[map, ← iteratorPosition owner variant iterator]

private def isSnapshotValue (value : Lean.Expr) : MetaM Bool :=
  return (← inferType value).isConstOf ``LeanerIR.Proofs.Denote.SnapshotValue.Value

/-- Promote a plain logical operand when another operand already carries a
snapshot. Physical Table inputs must be observed at their origin, before this
point; promotion neither reads memory nor changes an existing observation. -/
private def snapshotOperand (value : Lean.Expr) : MetaM Lean.Expr := do
  if ← isSnapshotValue value then return value
  mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.scalar #[value]

private def aggregateField (value index : Lean.Expr) : MetaM Lean.Expr := do
  if ← isSnapshotValue value then
    mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.field #[value, index]
  else mkAppM ``LeanerIR.RuntimeValue.field #[value, index]

private def Domain.ofAggregate (domain : Domain) (value : Lean.Expr) : MetaM Lean.Expr := do
  if domain == .aggregate then return value
  let raw ← if ← isSnapshotValue value then
      mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.physical #[value]
    else pure value
  domain.ofRuntime raw

private def Domain.binderOfAggregate (domain : Domain) (value : Lean.Expr) : MetaM Lean.Expr := do
  if domain == .aggregate then return value
  let raw ← if ← isSnapshotValue value then
      mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.physical #[value]
    else pure value
  domain.binderOfRuntime raw

private def aggregateTestVariants (value source variants : Lean.Expr) : MetaM Lean.Expr := do
  if ← isSnapshotValue value then
    mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.testVariants #[value, source, variants]
  else mkAppM ``LeanerLang.Contract.testVariants #[value, source, variants]

private def aggregateBranches (left right : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  if (← isSnapshotValue left) || (← isSnapshotValue right) then
    return (← snapshotOperand left, ← snapshotOperand right)
  return (left, right)

private def aggregateConstructor (runtimeConstructor : Name) (parameters : Array Lean.Expr)
    (shape : Lean.Expr) (values : Array Lean.Expr) : MetaM Lean.Expr := do
  if ← values.anyM isSnapshotValue then
    mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.aggregate
      #[shape, ← mkListLit (mkConst ``LeanerIR.Proofs.Denote.SnapshotValue.Value)
        (← values.toList.mapM snapshotOperand)]
  else mkAppM runtimeConstructor (parameters.push (← mkArrayLit (mkConst ``RuntimeValue) values.toList))

/-- Encode a native generic binder while preserving logical snapshot values. -/
private def runtimeAggregateValue (context : Context) (id : ExprId)
    (translated : Lean.Expr) : MetaM Lean.Expr := do
  if ← isSnapshotValue translated then return translated
  if (← inferType translated).isConstOf ``RuntimeValue then return translated
  let some expression := context.ns.expressions[id.index]? | return translated
  if let some rep := context.valueRep? expression.typeId then
    match rep with
    | .parameter _ | .twin _ _ | .vector _ _ | .tuple _ =>
        return ← rep.encode context.codecs translated
    | _ => pure ()
  return translated

/-- Observe a physical input where its memory is selected. Old/labeled inputs
are converted here, before a surrounding expression can change that memory.
Already-logical let/callee binders retain their own snapshots. -/
private def Context.observeInput (context : Context) (typeId : TypeId)
    (value : Lean.Expr) (memory : Option Lean.Expr := none) : MetaM Lean.Expr := do
  if ← isSnapshotValue value then return value
  let some nativeType := context.ntyOf? (specTypeId context.unit typeId) | return value
  unless snapshotType context.unit nativeType do return value
  let some memory := memory.or context.state
    | throwError "a physical Table observation needs a specification memory"
  let encoded ← if (← inferType value).isConstOf ``RuntimeValue then pure value else do
    let some rep := context.valueRep? (specTypeId context.unit typeId)
      | throwError "a Table observation has no physical representation"
    rep.encode context.codecs value
  let observed ← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.observeRuntime?
    #[← context.frame, memory, ← quoteNTy nativeType, encoded]
  let junk ← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.scalar #[mkConst ``RuntimeValue.unit]
  mkAppM ``Option.getD #[observed, junk]

/-- The specification function a call names, with its namespace. -/
private def specFunctionOf? (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    Option (ValidatedNamespace × LeanerIR.SpecFunctionDecl) := do
  let targetNs ← unit.namespaces[reference.namespaceId.index]?
  let functionId ← unit.resolution.specFunction? reference.name
  let declaration ← targetNs.specFunctions[functionId.index]?
  pure (targetNs, declaration)

/-- The authored byte range of a location, when the unit records one. -/
private def locRange (unit : ValidatedUnit) (loc : LeanerIR.LocId) : ObligationRange :=
  match unit.tables.locations[loc.index]?.bind (·.primary) with
  | some range => {
      file := (unit.tables.files[range.file.index]?.map (·.name)).getD ""
      startByte := range.startByte, endByte := range.endByte }
  | none => {}

/-- The authored byte range of a condition. -/
private def conditionRange (unit : ValidatedUnit) (condition : LeanerIR.Condition) :
    ObligationRange :=
  locRange unit condition.loc

/-- Wrap a translated clause with its source range so a residual obligation
can be reported at the authored clause. -/
private def markObligation (range : ObligationRange) (clause : Lean.Expr) : Lean.Expr :=
  mkApp4 (mkConst ``LeanerIR.Proofs.Obligation) (mkStrLit range.file)
    (mkRawNatLit range.startByte) (mkRawNatLit range.endByte) clause

/-- The lemma a reference names, with its namespace. -/
def lemmaOf? (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    Option (ValidatedNamespace × LeanerIR.LemmaDecl) := do
  let targetNs ← unit.namespaces[reference.namespaceId.index]?
  let index ← unit.resolution.lemma? reference.name
  let declaration ← targetNs.lemmas[index]?
  pure (targetNs, declaration)

/-- The parameters of a lemma that range over the values of a native type:
by position, those of an aggregate type with one, which the premise types. -/
private def lemmaTypedParameters (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    Array (Nat × LeanerIR.Proofs.Denote.NTy) := Id.run do
  let some (_, declaration) := lemmaOf? unit reference | return #[]
  let mut typed := #[]
  for parameter in declaration.signature.parameters, index in [0:declaration.signature.parameters.size] do
    let typeId := specTypeId unit parameter.typeUse.typeId
    let some parameterType := unit.tables.types[typeId.index]? | continue
    unless domainOf parameterType matches .aggregate do continue
    let some nty := LeanerIR.Proofs.Denote.ntyOf unit reference.namespaceId typeId | continue
    typed := typed.push (index, nty)
  return typed

/-- Whether a lemma's definitions take a family: to read storage at, or to
type its parameters at. -/
private def lemmaTakesFrame (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef)
    (readsState : Bool) : Bool :=
  readsState || !(lemmaTypedParameters unit reference).isEmpty

/-- The definitions a lemma is stated over, each over the bundle of its
parameters: its premise, its conclusion, the proposition of each step of its
proof, and each component of its measure; and its theorem. -/
structure LemmaNames where
  requires : Name
  ensures : Name
  steps : Array Name
  measures : Array Name
  /-- For each step its proof assumes, by position: that it holds wherever
  the proof reaches it, a hypothesis of the theorem and its users. -/
  trusted : Array (Nat × Name)
  theorem_ : Name
  deriving Inhabited

/-- The names of a lemma's definitions and theorem. -/
def lemmaNames (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) : MetaM LemmaNames := do
  let some path := unit.tables.namespaces[reference.namespaceId.index]?
    | throwError "a lemma's namespace has no path"
  let some qualified := unit.tables.names[reference.name.index]?
    | throwError "a lemma has no name"
  let some (_, declaration) := lemmaOf? unit reference
    | throwError m!"lemma `{qualified.name}` does not resolve"
  let base := Name.str (path.segments.foldl (fun name segment => Name.str name segment) .anonymous)
    qualified.name
  let measures := (declaration.contract.conditions.filter (·.kind == .decreases)).size
  return {
    requires := Name.str base "lemmaRequires"
    ensures := Name.str base "lemmaEnsures"
    steps := (Array.range declaration.proof.size).map fun index => Name.str base s!"lemmaStep_{index}"
    measures := (Array.range measures).map fun index => Name.str base s!"lemmaMeasure_{index}"
    trusted := (declaration.proof.zipIdx.filter (·.1.kind == .assumption)).map fun (_, index) =>
      (index, Name.str base s!"lemmaTrusted_{index}")
    theorem_ := Name.str base "lemma" }

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

/-- What a specification function's body reads besides its arguments,
through the specification functions it calls: the executable unit (a
behavioral predicate other than `requires_of`), the table of declared
preconditions (`requires_of`), and the state (either, or storage). -/
private structure SpecReads where
  unit : Bool := false
  requires : Bool := false
  state : Bool := false
  /-- `aborts_of` or `result_of`: what holds of them rests on the
  determinism of runs from typed memory. -/
  determinism : Bool := false
  /-- `result_of`: what holds of it at a known function value rests on its
  runs ending. -/
  result : Bool := false
  /-- A binder over the state domain: a state label. -/
  labels : Bool := false
  /-- An uninterpreted value at type arguments: a specification function
  without a body (an unspecified value among them) or a Move function. -/
  types : Bool := false

/-- Whether a specification function is the unspecified value LeanerLang
declares for an aborting specification branch. It does not depend on its
instantiation, which may be a mathematical integer no native type stands for. -/
private def arbitraryValue (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) : Bool :=
  (unit.tables.names[reference.name.index]?).any (·.name.startsWith "__leaner_arbitrary_")

/-- What the expressions `start` reach read, through the specification
functions they call. -/
private def readsFrom (unit : ValidatedUnit) (start : List (LeanerIR.NamespaceId × ExprId)) :
    SpecReads := Id.run do
  let mut work := start
  let mut visited : Array (Nat × Nat) := #[]
  let mut reads : SpecReads := {}
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
        match expression.kind with
        | .operation (.specification (.behavior .requiresOf _)) .. =>
            reads := { reads with requires := true, state := true }
        | .operation (.specification (.behavior kind range)) .. =>
            reads := { reads with unit := true, state := true
                                  determinism := reads.determinism ||
                                    (kind matches .abortsOf | .resultOf) ||
                                    (kind == .ensuresOf && range.post.isSome)
                                  result := reads.result || kind matches .resultOf }
        | .operation (.specification (.global _)) .. | .operation (.global _) .. |
            .operation (.specification (.publish _)) .. |
            .operation (.specification (.remove _)) .. |
            .operation (.specification (.update _)) .. =>
            reads := { reads with state := true }
        | .operation (.specification (.functionCall callee _)) instantiations .. =>
            if let some (_, { body := some root, .. }) := specFunctionOf? unit callee then
              work := work ++ [(callee.namespaceId, root)]
            else if !instantiations.isEmpty && !arbitraryValue unit callee then
              reads := { reads with types := true }
        | .operation (.specification (.lemma callee _)) .. =>
            if let some (_, declaration) := lemmaOf? unit callee then
              work := work ++ (declaration.contract.conditions.toList.map
                fun condition => (callee.namespaceId, condition.expression))
        | .quantifier _ binders _ _ _ =>
            if binders.any (·.label.isSome) then reads := { reads with labels := true }
        | _ => pure ()
        work := work ++ (LeanerIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·)
  return reads

/-- What a declaration's data invariants read. -/
private def invariantReads (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.StructDecl) : SpecReads :=
  let reads := readsFrom unit <|
    (declaration.contract.conditions.filter (·.kind == .structInvariant)).toList.map
      fun condition => (namespaceId, condition.expression)
  if (unit.namespaces[namespaceId.index]?).any (LeanerIR.Proofs.Denote.declarationHasClosureFields · declaration) then
    { reads with unit := true, determinism := true }
  else reads

/-- Whether a data invariant of the unit states a behavioral predicate: the
predicate of stored invariants then takes the executable unit. -/
def storedInvariantReadsUnit (unit : ValidatedUnit) : Bool :=
  (unit.namespaces.toList.zipIdx).any fun (ns, index) =>
    ns.structs.any fun declaration =>
      let reads := invariantReads unit ⟨index⟩ declaration
      reads.unit || reads.requires

private def specReads (unit : ValidatedUnit) (source : LeanerIR.QualifiedRef) : SpecReads :=
  match specFunctionOf? unit source with
  | some (_, { body := some root, .. }) => readsFrom unit [(source.namespaceId, root)]
  | _ => {}

/-- Whether a recursive specification function's definition takes the types
its type parameters stand for: it reads an uninterpreted value at type
arguments, and no storage, whose family would supply them. -/
private def definitionTakesTypes (reads : SpecReads) : Bool :=
  reads.types && !reads.state

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

/-- The recursive specification functions defined together with one: those
it reaches that reach it back, itself among them, in declaration order. -/
private def specGroup (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    Array LeanerIR.QualifiedRef := Id.run do
  let mut group := #[]
  for ns in unit.namespaces, namespaceIndex in [0:unit.namespaces.size] do
    for declaration in ns.specFunctions do
      let other : LeanerIR.QualifiedRef := ⟨⟨namespaceIndex⟩, declaration.name⟩
      if other == reference ||
          (specReaches unit reference other && specReaches unit other reference) then
        group := group.push other
  return group

/-- A member of a group of recursive specification functions, as its
definition is built. -/
private structure SpecMember where
  reference : LeanerIR.QualifiedRef
  name : Name
  targetNs : ValidatedNamespace
  declaration : LeanerIR.SpecFunctionDecl
  root : ExprId
  domains : Array Domain
  resultDomain : Domain
  bundleType : Lean.Expr
  resultType : Lean.Expr
  localTypes : Array IrTy
  deriving Inhabited

/-- The sum of types from position `k` on: `PSum`, nested to the right. -/
private partial def sumType (types : Array Lean.Expr) (k : Nat) : MetaM Lean.Expr := do
  if k + 1 ≥ types.size then return types[k]!
  mkAppM ``PSum #[types[k]!, ← sumType types (k + 1)]

/-- A value of the type at position `index` in the sum from position `k`. -/
private partial def sumInject (types : Array Lean.Expr) (k index : Nat) (value : Lean.Expr) :
    MetaM Lean.Expr := do
  if k + 1 ≥ types.size then return value
  let rest ← sumType types (k + 1)
  if index == k then mkAppOptM ``PSum.inl #[types[k]!, rest, value]
  else mkAppOptM ``PSum.inr #[types[k]!, rest, ← sumInject types (k + 1) index value]

/-- `x` of the sum from position `k`, by its summand: `branch index value`
at the type at `index`, of the type `motive` gives the injection of
`value` into the sum from position 0. -/
private partial def sumCases (types : Array Lean.Expr) (k : Nat) (motive x : Lean.Expr)
    (branch : Nat → Lean.Expr → MetaM Lean.Expr) : MetaM Lean.Expr := do
  if k + 1 ≥ types.size then return ← branch k x
  let rest ← sumType types (k + 1)
  let left ← withLocalDeclD `value types[k]! fun value => do
    mkLambdaFVars #[value] (← branch k value)
  let right ← withLocalDeclD `rest rest fun inner => do
    let innerMotive ← withLocalDeclD `z rest fun z => do
      mkLambdaFVars #[z] (mkApp motive (← mkAppOptM ``PSum.inr #[types[k]!, rest, z])).headBeta
    mkLambdaFVars #[inner] (← sumCases types (k + 1) innerMotive inner branch)
  mkAppOptM ``PSum.casesOn #[types[k]!, rest, motive, x, left, right]

/-- The constant motive `fun _ : domain => value`. -/
private def mkConstMotive (domain value : Lean.Expr) : MetaM Lean.Expr :=
  withLocalDeclD `x domain fun x => mkLambdaFVars #[x] value

/-- The parameters that are mutable references: their entry binder is apart
from their exit binder. -/
private def Context.mutableParameters (context : Context) : Array Nat :=
  (Array.range context.locals.size).filter fun index =>
    context.locals[index]? != context.oldLocals[index]?

mutual
/-- Bind a specification let in its lexical scope, also used when a free
state-label definition is read from another clause. -/
private partial def Context.bindLet (context : Context) (pattern : LeanerIR.PatternId)
    (initializer : ExprId) : MetaM Context := do
  let some patternNode := context.ns.patterns[pattern.index]?
    | throwError "specification binding pattern {pattern.index} is out of range"
  match patternNode.kind with
  | .wildcard => pure context
  | .variable localId =>
      let some patternType := context.valueTypeOf? patternNode.typeId
        | throwError "specification binding pattern has an unknown type"
      unless localId.index < context.locals.size do
        throwError "specification local {localId.index} has no binder slot"
      let value ← translate context initializer
      let value ← match domainOf patternType with
        | .boolean => mkDecide value
        | _ => pure value
      pure { context with
        locals := context.locals.set! localId.index (some value)
        localTypes := if localId.index < context.localTypes.size
          then context.localTypes.set! localId.index patternType else context.localTypes
        oldLocals := context.oldLocals.setIfInBounds localId.index (some value) }
  | .tuple elements | .constructor _ _ _ elements =>
      let literal? : Option (Array ExprId) :=
        match patternNode.kind, context.ns.expressions[initializer.index]? with
        | .tuple _, some { kind := .operation (.primitive .tuple) _ components _, .. } =>
            some components
        | _, _ => none
      let some components := literal? | do
        -- Any other tuple or constructor value binds its components by
        -- position, read through the total field projection.
        let aggregate ← runtimeAggregateValue context initializer (← translate context initializer)
        let mut bound := context
        for (element, index) in elements.zipIdx do
          let some elementNode := context.ns.patterns[element.index]?
            | throwError "specification binding pattern {element.index} is out of range"
          match elementNode.kind with
          | .wildcard => pure ()
          | .variable localId =>
              let some elementType := context.valueTypeOf? elementNode.typeId
                | throwError "specification binding pattern has an unknown type"
              unless localId.index < context.locals.size do
                throwError "specification local {localId.index} has no binder slot"
              let component ← aggregateField aggregate (toExpr index)
              let value ← (domainOf elementType).binderOfAggregate component
              bound := { bound with
                locals := bound.locals.set! localId.index (some value)
                localTypes := if localId.index < bound.localTypes.size
                  then bound.localTypes.set! localId.index elementType else bound.localTypes
                oldLocals := bound.oldLocals.setIfInBounds localId.index (some value) }
          | _ => throwError "generated contracts require a variable or wildcard component \
              in a destructuring binding pattern"
        pure bound
      -- A tuple bound from a tuple literal binds component-wise, every
      -- component taken in the scope before the binding, as an inlined
      -- function's parameters are bound from its arguments.
      unless components.size == elements.size do
        throwError "a tuple pattern binds a tuple of another arity"
      let mut bound := context
      for (element, component) in elements.zip components do
        let some elementNode := context.ns.patterns[element.index]?
          | throwError "specification binding pattern {element.index} is out of range"
        match elementNode.kind with
        | .wildcard => pure ()
        | .variable localId =>
            let some elementType := context.valueTypeOf? elementNode.typeId
              | throwError "specification binding pattern has an unknown type"
            unless localId.index < context.locals.size do
              throwError "specification local {localId.index} has no binder slot"
            let value ← translate context component
            let value ← match domainOf elementType with
              | .boolean => mkDecide value
              | _ => pure value
            bound := { bound with
              locals := bound.locals.set! localId.index (some value)
              localTypes := if localId.index < bound.localTypes.size
                then bound.localTypes.set! localId.index elementType else bound.localTypes
              oldLocals := bound.oldLocals.setIfInBounds localId.index (some value) }
        | _ => throwError "generated contracts require a variable or wildcard component \
            in a tuple binding pattern"
      pure bound
  | _ =>
      throwError "generated contracts require a variable or wildcard binding pattern"

/-- The state a label denotes in this clause. -/
private partial def Context.labelState (context : Context) (label : Nat) : MetaM LabelState := do
  if let some (_, state) := context.stateLabels.find? (·.1 == label) then return state
  let name := (context.unit.tables.names[label]?.map (·.name)).getD s!"{label}"
  if context.resolvingLabels.contains label then
    throwError "cyclic definition of state label `{name}`"
  let some definition := context.labelDefinitions.find? (·.label == label)
    | throwError "state label `{name}` has no definition in this contract"
  let some expression := context.ns.expressions[definition.operation.index]?
    | throwError "state label definition is out of range"
  let .operation (.specification operation) instantiations arguments _ := expression.kind
    | throwError "state label definition is not a two-state operation"
  let context := { context with resolvingLabels := label :: context.resolvingLabels }
  let context := match context.labelEntry, context.labelExit with
    | some entry, some exit => { context with
        oldState := some entry.memory
        oldLocals := entry.locals.zipWith (·.or ·) context.oldLocals
        state := some exit.memory
        locals := exit.locals.zipWith (·.or ·) context.locals }
    | _, _ => context
  let context ← definition.bindings.foldlM (init := context) fun context (pattern, initializer) =>
    context.bindLet pattern initializer
  let memory ← match operation with
    | .behavior .ensuresOf range | .behavior .resultOf range => do
        let (executable, callable, inputs, pre) ← translateInvocation context range arguments
        mkAppM ``LeanerIR.Proofs.StateOf #[executable, callable, inputs, pre]
    | _ => (·.1) <$> translateStateChange context operation instantiations arguments
  return { memory, locals := context.oldLocals }

/-- The clause's states under a memory range: a pre-state label replaces the
state `old` reads and the locals' entry values, a post-state label the current
state and the locals' current values. -/
private partial def Context.atRange (context : Context) (range : LeanerIR.MemoryRange) : MetaM Context := do
  let (oldState, oldLocals) ← match range.pre with
    | some label => do
        let state ← context.labelState label
        pure (some state.memory, state.locals)
    | none => pure (context.oldState, context.oldLocals)
  let (state, locals) ← match range.post with
    | some label => do
        let state ← context.labelState label
        pure (some state.memory, state.locals)
    | none => pure (context.state, context.locals)
  return { context with oldState, oldLocals, state, locals }

/-- A state change computes a memory and a presence condition. Naming the
memory does not assume the condition; it remains an obligation of the predicate. -/
private partial def translateStateChange (context : Context)
    (operation : LeanerIR.SpecOperation) (instantiations : Array LeanerIR.GenericArgument)
    (arguments : Array ExprId) (post : Option Lean.Expr := none) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let range ← match operation with
    | .publish range | .remove range | .update range => pure range
    | _ => throwError "expected a state-change predicate"
  let pre ← match range.pre with
    | some label => (·.memory) <$> context.labelState label
    | none => match context.oldState with
      | some state => pure state
      | none => throwError "a state-change predicate has no pre-state"
  let [.typeArg resourceUse] := instantiations.toList
    | throwError "a state-change predicate needs one resource type"
  let some key := arguments[0]? | throwError "a state-change predicate needs an address"
  let keyValue ← mkAppM ``RuntimeValue.address #[← translate context key]
  let (slot, resource, nativeType) ← context.slot pre resourceUse.typeId keyValue
  let present ← slotPresent slot
  let condition := if operation matches .publish _ then mkNot present else present
  let frame ← context.frame
  let unit ← frameUnit frame
  let mut valueCondition := mkConst ``True
  let value ← if operation matches .remove _ then
      pure (mkApp (mkConst ``Option.none [Level.zero])
        (mkApp2 (mkConst ``LeanerIR.Proofs.Denote.ResourceType.carrier) unit resource))
    else do
      let some value := arguments[1]? | throwError "a state-change predicate needs a value"
      let encoded ← runtimeAggregateValue context value (← translate context value)
      if let some post := post then
        let slot ← memorySlot post resource keyValue
        -- Bind the slot's value explicitly. A caller can destruct this
        -- proposition and read its fields without a program point or a
        -- reconstruction of the native row (whose empty tail is opaque).
        let carrier := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.ResourceType.carrier) unit resource
        valueCondition ← withLocalDeclD `stored carrier fun stored => do
          let someStored ← mkAppM ``Option.some #[stored]
          let equalValue ← mkEq (← slotEncoding frame nativeType resource someStored) encoded
          mkAppM ``Exists #[← mkLambdaFVars #[stored]
            (← mkAppM ``And #[← mkEq slot someStored, equalValue])]
        pure slot
      else
        let codec := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.codec)
          (← frameCarriers frame) nativeType
        let decoded ← mkAppM ``LeanerIR.Proofs.Codec.decode? #[codec, encoded]
        let transport := mkAppN (mkConst ``LeanerIR.Proofs.Denote.Skolems.toRuntime)
          #[unit, frame, nativeType]
        mkAppM ``Option.map #[transport, decoded]
  let memory ← mkAppM ``LeanerIR.Proofs.Denote.Memory.set
    #[pre, resource, ← mkAppM ``RuntimeValue.storageKey #[keyValue], value]
  let condition ← if operation matches .remove _ then pure condition
    else mkAppM ``And #[condition, ← mkAppM ``And #[← slotPresent value, valueCondition]]
  return (memory, condition)

/-- The semantic invocation a behavioral predicate and its defined label share. -/
private partial def translateInvocation (context : Context) (range : LeanerIR.MemoryRange)
    (arguments : Array ExprId) : MetaM (Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr) := do
  let some executable := context.executable
    | throwError "a behavioral predicate reads the executable unit, which this contract \
        does not take"
  let some callable := arguments[0]?
    | throwError "a behavioral predicate names a function value"
  let some callableExpression := context.ns.expressions[callable.index]?
    | throwError "a behavioral predicate's function value is out of range"
  let some (.function parameters _ _) := context.valueTypeOf? callableExpression.typeId
    | throwError "a behavioral predicate's operand is not a function value"
  -- A shared reference is the observed value itself, at runtime as in a
  -- specification.
  for parameter in parameters do
    if let some (.reference reference) := context.typeOf? parameter then
      if reference.kind == .mutable then
        throwError "a behavioral predicate over a function with mutable reference parameters \
          is not carried yet"
  let callableValue ← translateRuntimeOperand context callable
  -- Keep the native function row visible for projections of a literal call.
  let callableValue ← if callableValue.isAppOfArity ``LeanerIR.Proofs.Denote.ClosureValue.encode 1 then do
      let some nativeType := LeanerIR.Proofs.Denote.ntyOf context.unit context.namespaceId
          callableExpression.typeId
        | throwError "a behavioral predicate's callable has no native type"
      pure (mkApp2 (mkConst ``LeanerIR.Proofs.Denote.ClosureValue.encodeFor)
        (← quoteNTy nativeType) callableValue.appArg!)
    else pure callableValue
  let inputs := (arguments.extract 1 (parameters.size + 1))
  let inputValues ← mkListLit (mkConst ``LeanerIR.RuntimeValue)
    (← inputs.toList.mapM (translateRuntimeOperand context))
  let pre ← match range.pre, context.oldState with
    | some label, _ => (·.memory) <$> context.labelState label
    | none, some state => pure state
    | none, none => do
        let some state := context.state | throwError "an invocation needs a pre-state"
        pure state
  return (executable, callableValue, inputValues, pre)

/-- Encode scalar operands while retaining already-observed aggregate values. -/
private partial def translateLogicalOperand (context : Context) (id : ExprId) : MetaM Lean.Expr := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "a specification operand is out of range"
  let some ty := context.valueTypeOf? expression.typeId
    | throwError "a specification operand has an unknown type"
  let translated ← translate context id
  if ← isSnapshotValue translated then return translated
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

/-- A boundary back to execution must not silently erase a snapshot. -/
private partial def translateRuntimeOperand (context : Context) (id : ExprId) : MetaM Lean.Expr := do
  let value ← translateLogicalOperand context id
  if ← isSnapshotValue value then
    throwError "a logical Table snapshot cannot be used as an executable operand without storage agreement"
  return value

/-- Logical ranges are either explicit bounds or the valid indices of a
vector. Observe the vector in the active context, including old/labeled
memory, just as a direct length expression does. -/
private partial def translateRangeBounds (context : Context) (id : ExprId) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "a specification range is out of range"
  match expression.kind with
  | .operation (.primitive .range) _ #[lower, upper] _ =>
      return (← translate context lower, ← translate context upper)
  | .operation (.specification .vectorRange) _ #[vector] _ =>
      let vector ← runtimeAggregateValue context vector (← translate context vector)
      let upper ← if ← isSnapshotValue vector then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorLength #[vector]
        else mkAppM ``LeanerLang.Contract.lengthVector #[vector]
      return (toExpr (0 : Int), upper)
  | _ => throwError "a specification range must supply bounds or a vector's indices"

/-- Translate one specification expression into a Lean term. -/
private partial def translate (context : Context) (id : ExprId) : MetaM Lean.Expr := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "specification expression {id.index} is out of range"
  let some ty := context.valueTypeOf? expression.typeId
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
      context.observeInput expression.typeId (← (domainOf logicalType).ofBinder binder)
  -- A specification function read at state labels: its arguments read the
  -- labels' states as the old and current ones.
  | .operation (.specification (.functionCall _ range)) _ _ _ =>
      if (range.pre.isSome || range.post.isSome) && context.rangeApplied != some id then
        translate { (← context.atRange range) with rangeApplied := some id } id
      else
        let .operation operation instantiations arguments _ := expression.kind | unreachable!
        translateOperation operation instantiations arguments ty expression.typeId
  | .operation operation instantiations arguments _ =>
      translateOperation operation instantiations arguments ty expression.typeId
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
          let some patternType := active.valueTypeOf? pattern.typeId
            | throwError "quantifier pattern type {pattern.typeId.index} is out of range"
          let some domainType := active.valueTypeOf? domainExpression.typeId
            | throwError "quantifier domain type {domainExpression.typeId.index} is out of range"
          let domain := domainOf patternType
          let binderName := match active.localNames[localId.index]? with
            | some name => if name.isEmpty then Name.mkSimple s!"quantified_{localId.index}"
                else Name.mkSimple name
            | none => Name.mkSimple s!"quantified_{localId.index}"
          let range (lower upper : Lean.Expr) (value : Lean.Expr) : MetaM Lean.Expr := do
            mkAppM ``And #[← mkAppM ``LE.le #[lower, value], ← mkAppM ``LT.lt #[value, upper]]
          -- What the quantifier binds, the condition a bound value satisfies
          -- to be in the domain when not every value is, and the value the
          -- pattern's local takes.
          let (binderName, binderType, membership?, element) ← match domainType with
            -- A type's domain; a bounded integer type ranges over its values,
            -- bound at the widened logical type, and an aggregate type with a
            -- native type over its native values, the pattern's local their
            -- encodings.
            | .typeDomain declaredId =>
                let some declared := active.typeOf? declaredId
                  | throwError "quantifier domain type {declaredId.index} is out of range"
                match patternType == declared, domain, active.ntyOf? declaredId with
                | true, .aggregate, some nty =>
                    if snapshotType active.unit nty then
                      throwError "quantification over Table snapshots requires a logical value domain, which is not carried yet"
                    -- A nominal's native carrier is its field row. For
                    -- function-valued fields that is not yet MVP's value
                    -- domain: MVP also ties the field to this nominal
                    -- instantiation. Quantifying over the unrestricted row
                    -- would let an invariant of G<u64> constrain a closure
                    -- taken from G<bool>. Reject until that domain is carried.
                    if declared matches .nominal .. then
                      unless nty.closureFree do
                        throwError "quantification over nominal values containing function fields requires a field-validity domain, which is not carried yet"
                    let carriers ← frameCarriers (← active.frame)
                    let ntyExpr ← quoteNTy nty
                    pure (binderName,
                      mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.carrier) carriers ntyExpr, none,
                      fun (value : Lean.Expr) => (pure (mkApp3
                        (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) carriers ntyExpr value) :
                          MetaM Lean.Expr))
                | _, _, _ =>
                -- A bounded integer type an instantiation gives the pattern
                -- ranges over its values.
                let membership? ← if patternType == declared then pure (boundedMembership? declared)
                  else
                  match patternType, declared with
                  | .integer .unbounded _, .integer (.bits width) signed =>
                      pure (some fun (value : Lean.Expr) => pure (mkApp3
                        (mkConst ``LeanerIR.IntegerValueFits)
                        (mkApp (mkConst ``LeanerIR.IntWidth.bits) (toExpr width)) (toExpr signed)
                        value))
                  | _, _ => throwError "quantifier pattern and type domain differ"
                pure (binderName, domain.leanType, membership?,
                  fun (value : Lean.Expr) => (pure value : MetaM Lean.Expr))
            -- A range ranges over the integers from its lower bound, below
            -- its upper bound.
            | .range =>
                let (lower, upper) ← translateRangeBounds active binder.domain
                pure (binderName, domain.leanType, some (range lower upper), fun value => pure value)
            -- A vector ranges over its elements: the pattern binds an element
            -- of the vector, the form the `contains` and `index_of`
            -- primitives denote to.
            | .vector _ _ =>
                let vector ← runtimeAggregateValue active binder.domain
                  (← translate active binder.domain)
                let snapshot ← isSnapshotValue vector
                let elements ← if snapshot then
                    mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.elements #[vector]
                  else mkAppM ``LeanerLang.Contract.elementsVector #[vector]
                let elementType := if snapshot then mkConst ``LeanerIR.Proofs.Denote.SnapshotValue.Value
                  else mkConst ``RuntimeValue
                pure (binderName, elementType,
                  some fun element => mkAppM ``Membership.mem #[elements, element],
                  fun element => domain.binderOfAggregate element)
            -- A binder over the state domain binds a memory, the state its
            -- label denotes.
            | .stateDomain =>
                let some skolems := active.skolems?
                  | throwError "a state label needs the contract's skolem instance"
                let memory := mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) (← frameUnit skolems)
                pure (binderName, memory, none, fun value => pure value)
            | _ => throwError "a quantifier domain must be a type, a range, or a vector"
          -- A state label also binds the value of each mutable reference
          -- parameter at its state, as the Prover keeps a copy per label.
          let mutable := if domainType matches .stateDomain then active.mutableParameters else #[]
          let copies ← mutable.mapM fun parameter => do
            let some (some entry) := active.oldLocals[parameter]?
              | throwError "a mutable reference parameter has no entry binder"
            let localName := (active.localNames[parameter]?).getD s!"local_{parameter}"
            pure (Name.mkSimple s!"{binderName}_{localName}",
              fun (_ : Array Lean.Expr) => inferType entry)
          withLocalDeclD binderName binderType fun value =>
          withLocalDeclsD copies fun copies => do
            let element ← element value
            let labelLocals := (mutable.zip copies).foldl
              (fun locals (parameter, copy) => locals.set! parameter (some copy)) active.locals
            let stateLabels := match binder.label with
              | some label => (label, { memory := value, locals := labelLocals }) :: active.stateLabels
              | none => active.stateLabels
            let inner ← bind (index + 1) { active with
              locals := active.locals.set! localId.index (some element)
              oldLocals := active.oldLocals.set! localId.index (some element)
              stateLabels
              lemmaQuantified := true }
            let membership ← membership?.mapM (· value)
            let bound := #[value] ++ copies
            let existsOver (body : Lean.Expr) : MetaM Lean.Expr :=
              bound.foldrM (init := body) fun binder body => do
                mkAppM ``Exists #[← mkLambdaFVars #[binder] body]
            match kind, membership with
            | .forall, none => mkForallFVars bound inner
            | .forall, some member => mkForallFVars bound (← mkArrow member inner)
            | .exists, none => existsOver inner
            | .exists, some member => existsOver (← mkAppM ``And #[member, inner])
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
              let test ← aggregateTestVariants scrutineeValue (toExpr handle) variantLiteral
              let condition ← mkEq test (mkConst ``Bool.true)
              let mut armLocals := context.locals
              let mut armOldLocals := context.oldLocals
              for (childId, index) in fields.zipIdx do
                let some child := context.ns.patterns[childId.index]?
                  | throwError "specification match child pattern {childId.index} is out of range"
                match child.kind with
                | .wildcard => pure ()
                | .variable localId =>
                    let selected ← aggregateField scrutineeValue (toExpr index)
                    let some childType := context.valueTypeOf? child.typeId
                      | throwError "specification match child has an unknown type"
                    let value ← (domainOf childType).binderOfAggregate selected
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
              let (body, fallback) ← aggregateBranches body fallback
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
      let (thenTerm, elseTerm) ← aggregateBranches thenTerm elseTerm
      mkAppOptM ``ite #[none, test, none, thenTerm, elseTerm]
  | .letDecl pattern (some initializer) body =>
      translate (← context.bindLet pattern initializer) body
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
    -- A vector is a runtime value in a clause.
    | .vector _, .vector .. => runtimeLiteral literal
    | _, _ =>
        throwError "specification literal {repr literal} at type {describeType ty} \
          is not supported in generated contracts"
  runtimeLiteral (literal : ConstValue) : MetaM Lean.Expr := do
    match literal with
    | .unit => return mkConst ``RuntimeValue.unit
    | .bool value => mkAppM ``RuntimeValue.bool #[toExpr value]
    | .character value => mkAppM ``RuntimeValue.character #[toExpr value]
    | .integer value => mkAppM ``RuntimeValue.integer #[toExpr value]
    | .address value => mkAppM ``RuntimeValue.address #[toExpr value]
    | .string value => mkAppM ``RuntimeValue.string #[toExpr value]
    | .bytes value => mkAppM ``RuntimeValue.bytes #[toExpr value]
    | .vector elements =>
        mkAppM ``RuntimeValue.vector
          #[← mkArrayLit (mkConst ``RuntimeValue) (← elements.toList.mapM runtimeLiteral)]
    | .tuple elements =>
        mkAppM ``RuntimeValue.tuple
          #[← mkArrayLit (mkConst ``RuntimeValue) (← elements.toList.mapM runtimeLiteral)]
    | .profile _ =>
        throwError "specification literal {repr literal} has no runtime value"
  /-- Translate an operand and re-encode it as a runtime value, for the
  positions — a storage key, a published resource — where a clause hands a
  value back to the runtime vocabulary. -/
  runtimeOperand (id : ExprId) : MetaM Lean.Expr := translateRuntimeOperand context id
  logicalOperand (id : ExprId) : MetaM Lean.Expr := translateLogicalOperand context id
  /-- Translate an aggregate and erase it when it is currently represented
  by a generated twin. -/
  runtimeAggregate (id : ExprId) : MetaM Lean.Expr := do
    runtimeAggregateValue context id (← translate context id)
  /-- The resource type a global operation is instantiated at. -/
  resourceType (instantiations : Array LeanerIR.GenericArgument) : MetaM TypeId := do
    match instantiations.toList with
    | [.typeArg resource] => return resource.typeId
    | _ => throwError "a global specification operation needs one resource type"
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
    let parameters ← declaration.signature.parameters.mapM fun parameter =>
      signatureDomain context.unit parameter.typeUse.typeId "a specification function parameter"
    let result ← match declaration.signature.results.toList with
      | [result] => signatureDomain context.unit result.typeId "a specification function result"
      | _ => throwError "a recursive specification function returns one value"
    pure (parameters, result)
  /-- The Lean type of a domain as a definition's result: a Boolean result
  is a proposition. A parameter has its binder type (`Domain.leanType`),
  which the body reads. -/
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
  /-- A specification function's value outside its domain, where an
  argument does not fit the fixed-width type of its parameter: an
  uninterpreted function of the bundled arguments, declared once. -/
  outsideValue (reference : LeanerIR.QualifiedRef) (bundle : Lean.Expr) : MetaM Lean.Expr := do
    let name := Name.str (← specDefinitionName context.unit reference) "outside"
    unless (← getEnv).contains name do
      let (domains, result) ← definitionDomains reference
      let bundleType ← domains.foldrM
        (fun domain rest => mkAppM ``Prod #[domain.leanType, rest]) (mkConst ``Unit)
      let resultType := definitionType result
      let type ← mkArrow bundleType resultType
      let value ← withLocalDeclD `bundle bundleType fun bundle => do
        mkLambdaFVars #[bundle] (← mkAppOptM ``Inhabited.default #[resultType, none])
      addDecl (.opaqueDecl
        { name, levelParams := [], type, value, isUnsafe := false, all := [name] })
    return mkApp (mkConst name) bundle
  /-- The types a recursive definition's type parameters stand for at a
  call: the call's type arguments, under the caller's family. -/
  definitionTypes (instantiations : Array LeanerIR.GenericArgument) : MetaM Lean.Expr := do
    let arguments ← instantiations.mapM fun instantiation => do
      let .typeArg argument := instantiation
        | throwError "a specification function call takes only type arguments in \
            generated contracts"
      let some nty := context.ntyOf? argument.typeId
        | throwError m!"a type argument of a recursive specification function has no \
            native type ({repr instantiation})"
      let quoted ← quoteNTy nty
      pure <| match context.types with
        | some types => mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.substWith) types quoted
        | none => quoted
    let list ← mkListLit (mkConst ``LeanerIR.Proofs.Denote.NTy) arguments.toList
    withLocalDeclD `index (mkConst ``Nat) fun index => do
      mkLambdaFVars #[index] (← mkAppOptM ``List.getD
        #[none, list, index, mkConst ``LeanerIR.Proofs.Denote.NTy.unit])
  /-- A recursive definition's result at a call, in the call's domain. -/
  callResult (reference : LeanerIR.QualifiedRef) (ty : IrTy) (value : Lean.Expr) :
      MetaM Lean.Expr := do
    let (_, result) ← definitionDomains reference
    match result, domainOf ty with
    | .aggregate, domain => domain.ofRuntime value
    | .boolean, .boolean => pure value
    | _, _ => pure value
  /-- A specification role of an intrinsic map, as the map model reads it. -/
  mapSpecCall (role : String) (owner : LeanerIR.NameId) (ty : IrTy) (arguments : Array ExprId) :
      MetaM Lean.Expr := do
    let operand (index : Nat) : MetaM ExprId := do
      let some id := arguments[index]?
        | throwError m!"the intrinsic map role `{role}` lacks operand {index}"
      pure id
    if (tableModel? context.unit (.nominal owner #[])).isSome then
      if context.physicalInvariantLengths then
        throwError "Table contents in stored data invariants are not carried yet"
      if role == "map_spec_new" then
        throwError "a pure Table constructor's logical identity is not carried yet"
      let map ← runtimeAggregate (← operand 0)
      unless ← isSnapshotValue map do
        throwError "a Table specification requires an observed content snapshot"
      let key : MetaM Lean.Expr := do
        mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.identity
          #[← snapshotOperand (← logicalOperand (← operand 1))]
      let size := mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.size #[map]
      let hasKey : MetaM Lean.Expr := do
        mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.hasKey #[map, ← key]
      return ← match role with
      | "map_spec_len" => size
      | "map_spec_is_empty" | "map_spec_aborts_empty" => mkEq (← size) (mkIntLit 0)
      | "map_spec_has_key" | "map_spec_aborts_add" => mkEq (← hasKey) (mkConst ``Bool.true)
      | "map_spec_aborts_del" | "map_spec_aborts_borrow" => mkEq (← hasKey) (mkConst ``Bool.false)
      | "map_spec_aborts_destroy_empty" => mkAppM ``Ne #[← size, mkIntLit 0]
      | "map_spec_get" =>
          (domainOf ty).ofAggregate (← mkAppM
            ``LeanerIR.Proofs.Denote.SnapshotValue.Value.getValue #[map, ← key])
      | "map_spec_set" =>
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.setValue
            #[map, ← key, ← snapshotOperand (← logicalOperand (← operand 2))]
      | "map_spec_del" =>
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.removeValue #[map, ← key]
      | _ => throwError "the Table specification role `{role}` is not carried yet"
    let some model := mapModel? context.unit (.nominal owner #[])
      | throwError m!"the intrinsic map role `{role}` belongs to a map whose representation \
          is not carried"
    let aggregate (index : Nat) : MetaM Lean.Expr := do runtimeAggregate (← operand index)
    let value (index : Nat) : MetaM Lean.Expr := do runtimeOperand (← operand index)
    let map := aggregate 0
    let key := value 1
    let size : MetaM Lean.Expr := do mkAppM ``LeanerIR.Maps.size #[← map]
    let hasKey : MetaM Lean.Expr := do mkAppM ``LeanerIR.Maps.hasKey #[← map, ← key]
    match role with
    | "map_spec_new" => mkAppM ``LeanerIR.Maps.empty #[model.layout]
    | "map_spec_len" => size
    | "map_spec_is_empty" | "map_spec_aborts_empty" => mkEq (← size) (mkIntLit 0)
    | "map_spec_has_key" | "map_spec_aborts_add" => mkEq (← hasKey) (mkConst ``Bool.true)
    | "map_spec_aborts_del" | "map_spec_aborts_borrow" => mkEq (← hasKey) (mkConst ``Bool.false)
    | "map_spec_aborts_destroy_empty" => mkAppM ``Ne #[← size, mkIntLit 0]
    | "map_spec_get" => (domainOf ty).ofRuntime (← mkAppM ``LeanerIR.Maps.valueAt #[← map, ← key])
    | "map_spec_set" =>
        mkAppM ``LeanerIR.Maps.update #[model.layout, model.discipline, ← map, ← key, ← value 2]
    | "map_spec_del" => mkAppM ``LeanerIR.Maps.remove #[model.layout, model.discipline, ← map, ← key]
    | "map_spec_key_at" | "map_spec_insertion_key_at" =>
        (domainOf ty).ofRuntime (← mkAppM ``LeanerIR.Maps.keyAt #[← map, ← translate context (← operand 1)])
    | "map_spec_rank" | "map_spec_insertion_rank" => mkAppM ``LeanerIR.Maps.rank #[← map, ← key]
    | "map_spec_aborts_add_all" => mkAppM ``LeanerIR.Maps.AbortsAddAll #[← map, ← aggregate 1, ← aggregate 2]
    | "map_spec_aborts_new_from" => mkAppM ``LeanerIR.Maps.AbortsNewFrom #[← aggregate 0, ← aggregate 1]
    | "map_spec_aborts_upsert_all" => mkAppM ``LeanerIR.Maps.AbortsUpsertAll #[← aggregate 1, ← aggregate 2]
    | "map_spec_aborts_append_disjoint" =>
        mkAppM ``LeanerIR.Maps.AbortsAppendDisjoint #[← map, ← aggregate 1]
    | "map_spec_aborts_trim" => mkAppM ``LT.lt #[← size, ← translate context (← operand 1)]
    | "map_spec_aborts_replace_key_inplace" =>
        mkAppM ``LeanerIR.Maps.AbortsReplaceKey
          #[model.layout, model.discipline, ← map, ← value 1, ← value 2]
    | "map_spec_aborts_iter_borrow_mut" =>
        let some iteratorType := arguments[0]?.bind (typeOfExpr? context)
          | throwError "an iterator operand has an unknown type"
        let some (iteratorOwner, variant) := iteratorPosition? context.unit iteratorType
          | throwError "an iterator of an intrinsic map has no position variant"
        mkAppM ``Not #[← iteratorInRange iteratorOwner variant (← aggregate 0) (← aggregate 1)]
    | _ => throwError m!"the intrinsic map role `{role}` is not carried"
  /-- The bounds the integer parameters of a bundle carry by their Move
  types, in the form `SpecInt.unsigned_bounds`/`signed_bounds` state them;
  `none` when no parameter is bounded. -/
  parameterBounds (bundle : Lean.Expr) (domains : Array Domain) (localTypes : Array IrTy) :
      MetaM (Option Lean.Expr) := do
    let mut values : Array Lean.Expr := #[]
    let mut rest := bundle
    for _ in [:domains.size] do
      values := values.push (← mkAppM ``Prod.fst #[rest])
      rest ← mkAppM ``Prod.snd #[rest]
    valueBounds values domains localTypes
  /-- Whether `value` fits the integer type of `width` and `signed` by its
  own form: a literal in range, the value of a certified integer of a type
  that fits, or the length of a vector at a width of at least 64 bits. -/
  fitsByType (value : Lean.Expr) (width : Nat) (signed : Bool) : MetaM Bool := do
    let (low, high) : Int × Int := if signed
      then (-(2 ^ (width - 1)), 2 ^ (width - 1) - 1) else (0, 2 ^ width - 1)
    if let some literal := value.int? then return low ≤ literal && literal ≤ high
    if value.isAppOfArity ``LeanerIR.SpecInt.val 3 then
      let type ← whnfR (← inferType (value.getArg! 2))
      unless type.isAppOfArity ``LeanerIR.SpecInt 2 do return false
      let bits ← whnfR (type.getArg! 0)
      unless bits.isAppOfArity ``LeanerIR.IntWidth.bits 1 do return false
      let some width' := (bits.getArg! 0).rawNatLit? <|> (bits.getArg! 0).nat? | return false
      let signed' := (type.getArg! 1).isConstOf ``Bool.true
      return (signed' == signed && width' ≤ width) || (!signed' && signed && width' < width)
    if value.isAppOfArity ``Nat.cast 3 then
      let size := value.getArg! 2
      return !signed && 64 ≤ width && size.isAppOfArity ``Array.size 2 &&
        (size.getArg! 1).isAppOfArity ``LeanerIR.SpecVector.values 2
    return false
  /-- The bounds `values` carry by the fixed-width integer types of the
  parameters they are bound to; `none` when no parameter is bounded. -/
  valueBounds (values : Array Lean.Expr) (domains : Array Domain) (localTypes : Array IrTy) :
      MetaM (Option Lean.Expr) := do
    let mut conjuncts : Array Lean.Expr := #[]
    for value in values, index in [:values.size] do
      match domains[index]?, localTypes[index]? with
      | some .integer, some (LeanerIR.Ty.integer (.bits width) signed) =>
          if width == 0 then continue
          -- A value whose type already bounds it needs no guard: the
          -- conjunct would be a theorem.
          if ← fitsByType value width signed then continue
          let power (exponent : Nat) : MetaM Lean.Expr :=
            mkAppM ``HPow.hPow #[mkIntLit 2, mkNatLit exponent]
          let (low, high) ← if signed then
              pure (← mkAppM ``Neg.neg #[← power (width - 1)],
                ← mkAppM ``HSub.hSub #[← power (width - 1), mkIntLit 1])
            else pure (mkIntLit 0, ← mkAppM ``HSub.hSub #[← power width, mkIntLit 1])
          conjuncts := conjuncts.push (← mkAppM ``And
            #[← mkAppM ``LE.le #[low, value], ← mkAppM ``LE.le #[value, high]])
      | _, _ => pure ()
    let some first := conjuncts[0]? | return none
    let mut bounds := first
    for conjunct in conjuncts[1:] do
      bounds ← mkAppM ``And #[bounds, conjunct]
    return some bounds
  /-- Define a recursive specification function once, with the others of
  its group (`specGroup`): over its bundled arguments by well-founded
  recursion on its measure, with the unfolding theorem `f.unfold`. A
  definition first takes what its body reads besides its arguments
  (`specReads`), in that order; the members of a group read alike. -/
  ensureDefinition (reference : LeanerIR.QualifiedRef) : MetaM Name := do
    let name ← specDefinitionName context.unit reference
    if (← getEnv).contains name then return name
    let group := specGroup context.unit reference
    let reads := specReads context.unit reference
    let typesType ← mkArrow (mkConst ``Nat) (mkConst ``LeanerIR.Proofs.Denote.NTy)
    let optionalLocal {α : Type} (present : Bool) (binder : Name) (type : Lean.Expr)
        (k : Option Lean.Expr → MetaM α) : MetaM α :=
      if present then withLocalDeclD binder type fun x => k (some x) else k none
    -- A definition reading the executable unit, the table of declared
    -- preconditions, or storage takes the unit they are at first.
    optionalLocal (reads.unit || reads.requires || reads.state) `unit (mkConst ``ValidatedUnit)
      fun unit? =>
    let atUnit (constant : Name) := mkApp (mkConst constant) (unit?.getD (mkConst ``Unit))
    optionalLocal reads.unit `executable (atUnit ``LeanerIR.Validation.ExecutableUnit)
      fun executable =>
    optionalLocal reads.requires `requiresTable (atUnit ``LeanerIR.Proofs.RequiresTable)
      fun requiresTable =>
    -- A definition reading storage takes the family its reads resolve
    -- resource types at, before the memory.
    optionalLocal reads.state `frame (atUnit ``LeanerIR.Proofs.Denote.Skolems) fun frame =>
    optionalLocal reads.state `state (atUnit ``LeanerIR.Proofs.Denote.Memory) fun state =>
    -- One reading no storage, but an uninterpreted value at its type
    -- parameters, takes the types they stand for (`definitionTypes`).
    optionalLocal (definitionTakesTypes reads) `types typesType fun instantiation => do
    let parameters := #[unit?, executable, requiresTable, frame, state, instantiation].filterMap
      fun parameter => parameter
    let types ← match instantiation with
      | some instantiation => pure (some instantiation)
      | none => frame.mapM frameTypes
    let members ← group.mapM fun member => do
      let memberName ← specDefinitionName context.unit member
      let some (targetNs, declaration) := specFunctionOf? context.unit member
        | throwError "specification function `{repr member}` does not resolve"
      let some root := declaration.body
        | throwError "a recursive specification function must have a body"
      let (domains, resultDomain) ← definitionDomains member
      let bundleType ← domains.foldrM
        (fun domain rest => mkAppM ``Prod #[domain.leanType, rest]) (mkConst ``Unit)
      let mut localTypes : Array IrTy := #[]
      for localDecl in declaration.locals do
        let some localType := context.unit.tables.types[localDecl.type.typeId.index]?
          | throwError "a specification function local has an unknown type"
        localTypes := localTypes.push localType
      pure ({
        reference := member, name := memberName, targetNs := targetNs
        declaration := declaration, root := root, domains := domains
        resultDomain := resultDomain, bundleType := bundleType
        resultType := definitionType resultDomain, localTypes := localTypes } : SpecMember)
    -- The parameters are the projections of the bundle.
    let bodyContext (member : SpecMember) (bundle : Lean.Expr)
        (definition : Option RecursiveDefinition) : MetaM Context := do
      let mut locals : Array (Option Lean.Expr) :=
        Array.replicate member.declaration.locals.size none
      let mut rest := bundle
      for index in [:member.domains.size] do
        locals := locals.set! index (some (← mkAppM ``Prod.fst #[rest]))
        rest ← mkAppM ``Prod.snd #[rest]
      pure { context with
        namespaceId := member.reference.namespaceId, ns := member.targetNs
        locals, localTypes := member.localTypes
        localNames := member.declaration.locals.map (·.name), oldLocals := locals
        results := #[], resultTypes := #[]
        specCallStack := #[], typeArguments := #[]
        definition, facts := #[]
        executable, requiresTable, state, oldState := none, types
        anchorLocals := #[], anchorState := none }
    -- A member's measures: its `decreases` clause, or, without one, each
    -- integer parameter in turn. A Move specification function states no
    -- measure; the Move Prover takes its recursion as given.
    let candidateMeasures ← members.mapM fun member => do
      let candidates : Array (Lean.Expr → MetaM Lean.Expr) ←
        match member.declaration.contract.conditions.find? (·.kind == .decreases) with
        | some measureSource => pure #[fun bundle => do
            translate (← bodyContext member bundle none) measureSource.expression]
        | none => pure <| (member.domains.zipIdx).filterMap fun (domain, index) =>
            if domain matches .integer then some (fun bundle => do
              let mut rest := bundle
              for _ in [:index] do rest ← mkAppM ``Prod.snd #[rest]
              mkAppM ``Prod.fst #[rest])
            else none
      if candidates.isEmpty then
        throwError m!"the recursive specification function \
          `{specFunctionSpelling context.unit member.reference}` needs a `decreases` measure"
      candidates.mapM fun candidate =>
        withLocalDeclD `bundle member.bundleType fun bundle => do
          mkLambdaFVars #[bundle] (← mkAppM ``Int.toNat #[← candidate bundle])
    -- The step function of a member at the members' measures: over its
    -- bundle and a recursion hypothesis per member, over the bundles below
    -- its own. Guarded, a recursive call the path conditions do not prove
    -- descending is guarded by its descent. Assuming non-negativity, the
    -- integer parameters are taken as non-negative — the Move types the
    -- specification's `num` projection widened — only to select a
    -- measure; the step built so is discarded.
    let buildStep (index : Nat) (measures : Array Lean.Expr) (guarded assumeNonNegative : Bool) :
        MetaM Lean.Expr := do
      let member := members[index]!
      withLocalDeclD `bundle member.bundleType fun bundle => do
        let recurseTypes ← members.zipIdx.mapM fun (other, otherIndex) =>
          withLocalDeclD `smaller other.bundleType fun smaller => do
            let decreasing ← mkAppM ``LT.lt
              #[mkApp measures[otherIndex]! smaller, mkApp measures[index]! bundle]
            pure (`recurse, ← mkForallFVars #[smaller] (← mkArrow decreasing other.resultType))
        withLocalDeclsDND recurseTypes fun recurses => do
          let recursiveMembers := members.zipIdx.map fun (other, otherIndex) =>
            ({
              reference := other.reference, recurse := recurses[otherIndex]!
              measure := measures[otherIndex]!, resultType := other.resultType } : RecursiveMember)
          let definition : RecursiveDefinition := {
            reference := member.reference, argument := bundle, measure := measures[index]!
            members := recursiveMembers, guarded := guarded }
          let translateBody : MetaM Lean.Expr := do
            let body ← translate (← bodyContext member bundle (some definition)) member.root
            match member.resultDomain with
            | .aggregate => runtimeAggregateValue (← bodyContext member bundle none) member.root body
            | _ => pure body
          let translateBody : MetaM Lean.Expr := if !assumeNonNegative then translateBody else do
            let mut nonNegative : Array Lean.Expr := #[]
            let mut rest := bundle
            for domain in member.domains do
              let value ← mkAppM ``Prod.fst #[rest]
              rest ← mkAppM ``Prod.snd #[rest]
              if domain matches .integer then
                nonNegative := nonNegative.push (← mkAppM ``LE.le #[mkIntLit 0, value])
            let rec assume (index : Nat) : MetaM Lean.Expr := do
              if h : index < nonNegative.size then
                withLocalDeclD `nonNegative nonNegative[index] fun _ => assume (index + 1)
              else translateBody
            assume 0
          -- The definition unfolds at well-typed arguments only: the
          -- parameters' fixed-width types are the function's domain, which
          -- the recursion's descent may rely on, and its value outside them
          -- is unspecified (`outsideValue`).
          let body ← match ← parameterBounds bundle member.domains member.localTypes with
            | none => translateBody
            | some bounds =>
                let decision ← synthInstance (← mkAppM ``Decidable #[bounds])
                let inside ← withLocalDeclD `bounds bounds fun boundsHypothesis => do
                  mkLambdaFVars #[boundsHypothesis] (← translateBody)
                let outside ← withLocalDeclD `outside (mkNot bounds) fun outsideHypothesis => do
                  mkLambdaFVars #[outsideHypothesis] (← outsideValue member.reference bundle)
                pure (mkApp5 (mkConst ``dite [1]) member.resultType bounds decision inside outside)
          mkLambdaFVars (#[bundle] ++ recurses) body
    -- Measures every recursive call provably descends on, a choice per
    -- member; failing that, ones they descend on for non-negative
    -- parameters, with the calls guarded.
    let combinations := candidateMeasures.foldl (init := #[#[]]) fun combinations options =>
      combinations.flatMap fun combination => options.map combination.push
    let buildSteps (measures : Array Lean.Expr) (guarded assumeNonNegative : Bool) :=
      (Array.range members.size).mapM fun index =>
        buildStep index measures guarded assumeNonNegative
    -- A body that does not translate fails at every measure: that failure
    -- is the reason.
    let mut attempts : Array MessageData := #[]
    let mut untranslatable : Option Exception := none
    let mut found : Option (Array Lean.Expr × Array Lean.Expr) := none
    for measures in combinations do
      if found.isSome || untranslatable.isSome then break
      try found := some (measures, ← buildSteps measures false false)
      catch failure =>
        if failure.toMessageData.hasTag (· == `leaner.measure) then
          attempts := attempts.push failure.toMessageData
        else untranslatable := some failure
    for measures in combinations do
      if found.isSome || untranslatable.isSome then break
      try
        discard <| buildSteps measures false true
        found := some (measures, ← buildSteps measures true false)
      catch failure =>
        unless failure.toMessageData.hasTag (· == `leaner.measure) do
          untranslatable := some failure
    if let some failure := untranslatable then throw failure
    let some (measures, steps) := found
      | let spelled := ", ".intercalate (members.map fun member =>
            s!"`{specFunctionSpelling context.unit member.reference}`").toList
        throwError m!"{if members.size == 1 then
            s!"no measure of the recursive specification function {spelled} decreases"
          else s!"no measures of the mutually recursive specification functions {spelled} \
            decrease"} at every recursive call: {MessageData.joinSep attempts.toList "\n"}"
    let bundleTypes := members.map (·.bundleType)
    -- A member's fixpoint, and its applications as a recursive call takes them.
    let (fixpoint, motive, relation, wellFounded, step, injections) ←
      if members.size == 1 then do
        let relation ← mkAppM ``measure #[measures[0]!]
        let motive ← withLocalDeclD `bundle bundleTypes[0]! fun bundle =>
          mkLambdaFVars #[bundle] members[0]!.resultType
        pure (none, motive, relation, ← mkAppOptM ``WellFoundedRelation.wf #[bundleTypes[0]!, relation],
          steps[0]!, #[fun (bundle : Lean.Expr) => pure bundle])
      else do
        -- The sum of the bundles: each member's summand, its measure there,
        -- and the result type there.
        let sum ← sumType bundleTypes 0
        let resultMotive ← withLocalDeclD `x sum fun x => do
          mkLambdaFVars #[x] (← sumCases bundleTypes 0 (← mkConstMotive sum (mkSort (.succ .zero)))
            x fun index _ => pure members[index]!.resultType)
        let measureSum ← withLocalDeclD `x sum fun x => do
          mkLambdaFVars #[x] (← sumCases bundleTypes 0 (← mkConstMotive sum (mkConst ``Nat)) x
            fun index bundle => pure (mkApp measures[index]! bundle))
        let relation ← mkAppM ``measure #[measureSum]
        let wellFounded ← mkAppOptM ``WellFoundedRelation.wf #[sum, relation]
        let relationOf ← mkAppOptM ``WellFoundedRelation.rel #[sum, relation]
        -- The step over the sum: each summand's member's step, its
        -- recursion hypotheses the sum's at the members' summands.
        let hypothesisType (x : Lean.Expr) : MetaM Lean.Expr :=
          withLocalDeclD `y sum fun y => do
            mkForallFVars #[y] (← mkArrow (mkApp2 relationOf y x) (mkApp resultMotive y))
        let stepMotive ← withLocalDeclD `x sum fun x => do
          mkLambdaFVars #[x] (← mkArrow (← hypothesisType x) (mkApp resultMotive x))
        let step ← withLocalDeclD `x sum fun x => do
          mkLambdaFVars #[x] (← sumCases bundleTypes 0 stepMotive x fun index bundle => do
            let injected ← sumInject bundleTypes 0 index bundle
            withLocalDeclD `recurse (← hypothesisType injected) fun recurse => do
              let recursions ← members.zipIdx.mapM fun (other, otherIndex) =>
                withLocalDeclD `smaller other.bundleType fun smaller => do
                  let decreasing ← mkAppM ``LT.lt
                    #[mkApp measures[otherIndex]! smaller, mkApp measures[index]! bundle]
                  withLocalDeclD `less decreasing fun less => do
                    mkLambdaFVars #[smaller, less]
                      (mkApp2 recurse (← sumInject bundleTypes 0 otherIndex smaller) less)
              mkLambdaFVars #[recurse] (mkAppN steps[index]! (#[bundle] ++ recursions)))
        let fixpoint ← mkAppOptM ``WellFounded.fix #[sum, resultMotive, relationOf, wellFounded, step]
        let groupName := Name.str members[0]!.name "group"
        let groupType ← mkForallFVars parameters (← withLocalDeclD `x sum fun x => do
          mkForallFVars #[x] (mkApp resultMotive x))
        let groupValue ← mkLambdaFVars parameters fixpoint
        addDecl (.defnDecl {
          name := groupName, levelParams := [], type := groupType
          value := groupValue, hints := .opaque, safety := .safe })
        setIrreducibleAttribute groupName
        pure (some (mkAppN (mkConst groupName) parameters), resultMotive, relation, wellFounded, step,
          (Array.range members.size).map fun index (bundle : Lean.Expr) =>
            sumInject bundleTypes 0 index bundle)
    let applied (index : Nat) := mkAppN (mkConst members[index]!.name) parameters
    for member in members, index in [0:members.size] do
      let type ← mkForallFVars parameters (← mkArrow member.bundleType member.resultType)
      let value ← match fixpoint with
        | none => mkLambdaFVars parameters (← mkAppOptM ``WellFounded.fix
            #[member.bundleType, motive, ← mkAppOptM ``WellFoundedRelation.rel
              #[member.bundleType, relation], wellFounded, step])
        | some group => withLocalDeclD `bundle member.bundleType fun bundle => do
            mkLambdaFVars (parameters.push bundle) (mkApp group (← injections[index]! bundle))
      addDecl (.defnDecl {
        name := member.name, levelParams := [], type := type, value := value
        hints := .opaque, safety := .safe })
      setIrreducibleAttribute member.name
    -- `f a = F a (fun b _ => g b) …`: a member unfolds once, its recursive
    -- calls to the members' definitions.
    for member in members, index in [0:members.size] do
      let unfoldType ← withLocalDeclD `bundle member.bundleType fun bundle => do
        let recursions ← members.zipIdx.mapM fun (other, otherIndex) =>
          withLocalDeclD `smaller other.bundleType fun smaller => do
            let decreasing ← mkAppM ``LT.lt
              #[mkApp measures[otherIndex]! smaller, mkApp measures[index]! bundle]
            withLocalDeclD `less decreasing fun less =>
              mkLambdaFVars #[smaller, less] (mkApp (applied otherIndex) smaller)
        mkForallFVars (parameters.push bundle) (← mkEq (mkApp (applied index) bundle)
          (← Core.betaReduce (mkAppN steps[index]! (#[bundle] ++ recursions))))
      let unfoldValue ← withLocalDeclD `bundle member.bundleType fun bundle => do
        let (domain, argument) ← match fixpoint with
          | none => pure (member.bundleType, bundle)
          | some _ => pure (← sumType bundleTypes 0, ← injections[index]! bundle)
        let equation ← mkAppOptM ``WellFounded.fix_eq
          #[domain, motive, ← mkAppOptM ``WellFoundedRelation.rel #[domain, relation], wellFounded,
            step, argument]
        mkLambdaFVars (parameters.push bundle) equation
      addDecl (.thmDecl {
        name := Name.str member.name "unfold", levelParams := []
        type := unfoldType, value := unfoldValue })
    return name
  /-- What a lemma's statement reads besides its parameters. -/
  lemmaReads (reference : LeanerIR.QualifiedRef) : SpecReads :=
    match lemmaOf? context.unit reference with
    | some (_, declaration) => readsFrom context.unit <|
        (declaration.contract.conditions.toList ++ declaration.proof.toList).map
          fun condition => (reference.namespaceId, condition.expression)
    | none => {}
  /-- The parameter domains of a lemma, as its definitions take them. -/
  lemmaDomains (reference : LeanerIR.QualifiedRef) : MetaM (Array Domain) := do
    let some (_, declaration) := lemmaOf? context.unit reference
      | throwError "lemma `{repr reference}` does not resolve"
    declaration.signature.parameters.mapM fun parameter =>
      signatureDomain context.unit parameter.typeUse.typeId "a lemma parameter"
  /-- The arguments of a lemma instance as the bundle its definitions take. -/
  lemmaBundle (reference : LeanerIR.QualifiedRef) (arguments : Array ExprId) :
      MetaM Lean.Expr := do
    let domains ← lemmaDomains reference
    unless arguments.size == domains.size do
      throwError "a lemma instance's argument count differs from the lemma's"
    let mut values : Array Lean.Expr := #[]
    for (argument, domain) in arguments.zip domains do
      let value ← match domain with
        | .boolean => mkDecide (← translate context argument)
        | .aggregate => runtimeOperand argument
        | .integer | .text _ => translate context argument
      values := values.push value
    values.foldrM (fun value rest => mkAppM ``Prod.mk #[value, rest]) (mkConst ``Unit.unit)
  /-- A lemma's definition applied to what the position reads. -/
  lemmaApplied (reference : LeanerIR.QualifiedRef) (name : Name) : MetaM Lean.Expr := do
    let reads := lemmaReads reference
    let mut applied := mkConst name
    if reads.unit || reads.requires || lemmaTakesFrame context.unit reference reads.state ||
        reads.state then
      applied := mkApp applied (← context.unitExpr)
    if reads.unit then
      let some executable := context.executable
        | throwError "a lemma reads the executable unit, which this position does not take"
      applied := mkApp applied executable
    if reads.requires then
      let some table := context.requiresTable
        | throwError "a lemma reads the table of declared preconditions, which this \
            position does not take"
      applied := mkApp applied table
    if lemmaTakesFrame context.unit reference reads.state then
      applied := mkApp applied (← context.frame)
    if reads.state then applied := mkApp applied (← currentState)
    return applied
  /-- Define a lemma's statement once, each part over the bundle of its
  parameters, after what it reads besides them: the premise (the bounds the
  integer parameters carry by their types, then the `requires`), the
  conclusion, each step's proposition, and each measure component. -/
  ensureLemmaDefinitions (reference : LeanerIR.QualifiedRef) : MetaM LemmaNames := do
    let names ← lemmaNames context.unit reference
    if (← getEnv).contains names.requires then return names
    let some (targetNs, declaration) := lemmaOf? context.unit reference
      | throwError "lemma `{repr reference}` does not resolve"
    let reads := lemmaReads reference
    let optionalLocal {α : Type} (present : Bool) (binder : Name) (type : Lean.Expr)
        (k : Option Lean.Expr → MetaM α) : MetaM α :=
      if present then withLocalDeclD binder type fun x => k (some x) else k none
    let takesFrame := lemmaTakesFrame context.unit reference reads.state
    -- What it reads besides its parameters is at a unit, taken first.
    optionalLocal (reads.unit || reads.requires || takesFrame || reads.state) `unit
      (mkConst ``ValidatedUnit) fun unit? =>
    let atUnit (constant : Name) := mkApp (mkConst constant) (unit?.getD (mkConst ``Unit))
    optionalLocal reads.unit `executable (atUnit ``LeanerIR.Validation.ExecutableUnit)
      fun executable =>
    optionalLocal reads.requires `requiresTable (atUnit ``LeanerIR.Proofs.RequiresTable)
      fun requiresTable =>
    -- The family its reads resolve resource types at and its parameters
    -- are typed at, before the memory.
    optionalLocal takesFrame `frame (atUnit ``LeanerIR.Proofs.Denote.Skolems) fun frame =>
    optionalLocal reads.state `state (atUnit ``LeanerIR.Proofs.Denote.Memory) fun state => do
    let parameters := #[unit?, executable, requiresTable, frame, state].filterMap
      fun parameter => parameter
    let types ← frame.mapM frameTypes
    let domains ← lemmaDomains reference
    let bundleType ← domains.foldrM (fun domain rest => mkAppM ``Prod #[domain.leanType, rest])
      (mkConst ``Unit)
    let mut localTypes : Array IrTy := #[]
    for localDecl in declaration.locals do
      let some localType := context.unit.tables.types[localDecl.type.typeId.index]?
        | throwError "a lemma local has an unknown type"
      localTypes := localTypes.push localType
    let bodyContext (bundle : Lean.Expr) : MetaM Context := do
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
        definition := none, facts := #[]
        executable, requiresTable, state, oldState := state, types
        anchorLocals := #[], anchorState := none, lemmaQuantified := false }
    let conjunction (clauses : Array Lean.Expr) : MetaM Lean.Expr :=
      match clauses.back? with
      | none => pure (mkConst ``True)
      | some last => clauses.pop.foldrM (fun clause rest => mkAppM ``And #[clause, rest]) last
    let define (name : Name) (resultType : Lean.Expr)
        (body : Lean.Expr → MetaM Lean.Expr) : MetaM Unit := do
      let value ← withLocalDeclD `bundle bundleType fun bundle => do
        mkLambdaFVars (parameters.push bundle) (← body bundle)
      let type ← mkForallFVars parameters (← mkArrow bundleType resultType)
      addDecl (.defnDecl
        { name, levelParams := [], type, value, hints := .abbrev, safety := .safe })
      enableRealizationsForConst name
    -- Each clause is marked with its range, which a failure is reported at.
    let clauses (kind : LeanerIR.ConditionKind) (bundle : Lean.Expr) : MetaM (Array Lean.Expr) := do
      let active ← bodyContext bundle
      (declaration.contract.conditions.filter (·.kind == kind)).mapM fun condition =>
        markObligation (conditionRange context.unit condition) <$>
          translate active condition.expression
    define names.requires (mkSort .zero) fun bundle => do
      let bounds ← parameterBounds bundle domains localTypes
      -- Each typed parameter encodes a value of its native type.
      let typing ← (lemmaTypedParameters context.unit reference).mapM fun (index, nty) => do
        let some frame := frame | throwError "a lemma typing its parameters takes no family"
        let mut component := bundle
        for _ in [:index] do component ← mkAppM ``Prod.snd #[component]
        component ← mkAppM ``Prod.fst #[component]
        let carriers := (← frameCarriers frame)
        let ntyExpr ← quoteNTy nty
        let carrierType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.carrier) carriers ntyExpr
        let encodes ← withLocalDeclD `value carrierType fun value => do
          mkLambdaFVars #[value] (← mkEq
            (mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) carriers ntyExpr value) component)
        mkAppM ``Exists #[encodes]
      conjunction (bounds.toArray ++ typing ++ (← clauses .requires bundle))
    define names.ensures (mkSort .zero) fun bundle => do
      conjunction (← clauses .ensures bundle)
    for (condition, name) in declaration.proof.zip names.steps do
      define name (mkSort .zero) fun bundle => markObligation (conditionRange context.unit condition) <$> do
        let active ← bodyContext bundle
        match condition.kind with
        | .split =>
            -- A split on a Boolean directs the cases; one on an enum is no step.
            let some expression := targetNs.expressions[condition.expression.index]?
              | throwError "a split's expression is out of range"
            if context.unit.tables.types[expression.typeId.index]? == some .bool then
              mkAppM ``LeanerIR.Proofs.CaseSplit #[← translate active condition.expression]
            else pure (mkConst ``True)
        | _ => translate active condition.expression
    let measures := declaration.contract.conditions.filter (·.kind == .decreases)
    for (condition, name) in measures.zip names.measures do
      define name (mkConst ``Int) fun bundle => do
        translate (← bodyContext bundle) condition.expression
    -- What the proof assumes at a step: of every bundle meeting the premise
    -- and the assumptions before it, closed over what it reads.
    for (index, name) in names.trusted do
      let type ← withLocalDeclD `bundle bundleType fun bundle => do
        let applied (definition : Name) := mkApp (mkAppN (mkConst definition) parameters) bundle
        let earlier := names.trusted.filter (·.1 < index) |>.map fun (earlier, _) =>
          applied names.steps[earlier]!
        let premises := #[applied names.requires] ++ earlier
        let body ← premises.foldrM (fun premise rest => mkArrow premise rest)
          (applied names.steps[index]!)
        mkForallFVars (parameters.push bundle) body
      addDecl (.defnDecl
        { name, levelParams := [], type := mkSort .zero, value := type, hints := .abbrev,
          safety := .safe })
      enableRealizationsForConst name
    return names
  translateOperation (operation : Operation)
      (instantiations : Array LeanerIR.GenericArgument) (arguments : Array ExprId)
      (ty : IrTy) (resultTypeId : TypeId) : MetaM Lean.Expr := do
    match operation with
    | .specification (.lemma reference _) =>
        let names ← ensureLemmaDefinitions reference
        let bundle ← lemmaBundle reference arguments
        let owes := mkApp (← lemmaApplied reference names.requires) bundle
        let gives := mkApp (← lemmaApplied reference names.ensures) bundle
        mkAppM (if context.lemmaQuantified then ``LeanerIR.Proofs.LemmaInstance
          else ``LeanerIR.Proofs.LemmaApplication) #[owes, gives]
    | .specification (.functionCall reference _) =>
        if let some definition := context.definition then
          if let some member := definition.members.find? (·.reference == reference) then
            -- A recursive call: the bundled arguments, at a smaller measure.
            let bundle ← bundleArguments reference arguments
            let goal ← mkAppM ``LT.lt
              #[mkApp member.measure bundle, mkApp definition.measure definition.argument]
            -- The measure at the bundled arguments, projected out: `omega`
            -- reads the arithmetic, not the pairing.
            let (goal, _) ← dsimp goal (← Simp.mkContext)
            let proof ← mkFreshExprMVar goal
            let decreases ← try
                -- As the tactic does: refute the negated goal from the local
                -- hypotheses, which include the path conditions.
                if let some refutation ← proof.mvarId!.falseOrByContra then
                  let hypotheses ← refutation.withContext getLocalHyps
                  Lean.Elab.Tactic.Omega.omega hypotheses.toList refutation
                pure true
              catch failure =>
                if definition.guarded then pure false else
                  let facts ← context.facts.mapM fun fact => do pure m!"{← inferType fact}"
                  throwError (MessageData.tagged `leaner.measure m!"the recursive call of \
                    `{specFunctionSpelling context.unit reference}` does not decrease its \
                    measure under the conditions on its path: \
                    {failure.toMessageData}\ngoal: {goal}\nconditions: {facts.toList}")
            let value ← if decreases then
                pure (mkApp2 member.recurse bundle (← instantiateMVars proof))
              else
                -- The Move Prover's axiom for the function holds unguarded; a
                -- definition holds it wherever the measure descends and takes
                -- an arbitrary value elsewhere.
                let decision ← synthInstance (← mkAppM ``Decidable #[goal])
                let inside ← withLocalDeclD `descends goal fun descends =>
                  mkLambdaFVars #[descends] (mkApp2 member.recurse bundle descends)
                let outside ← withLocalDeclD `stuck (mkNot goal) fun stuck => do
                  mkLambdaFVars #[stuck] (← mkAppOptM ``Inhabited.default #[member.resultType, none])
                pure (mkApp5 (mkConst ``dite [1]) member.resultType goal decision inside outside)
            return ← callResult reference ty value
        if specRecursive context.unit reference then
          -- A recursive function is a Lean definition, not an expansion,
          -- taking what its body reads from the caller's position.
          let name ← ensureDefinition reference
          let bundle ← bundleArguments reference arguments
          let reads := specReads context.unit reference
          let mut applied := mkConst name
          if reads.unit || reads.requires || reads.state then
            applied := mkApp applied (← context.unitExpr)
          if reads.unit then
            let some executable := context.executable
              | throwError "a behavioral predicate reads the executable unit, which this \
                  contract does not take"
            applied := mkApp applied executable
          if reads.requires then
            let some table := context.requiresTable
              | throwError "`requires_of` reads the table of declared preconditions, which \
                  this contract does not take"
            applied := mkApp applied table
          if reads.state then applied := mkApp (mkApp applied (← context.frame)) (← currentState)
          if definitionTakesTypes reads then
            applied := mkApp applied (← definitionTypes instantiations)
          return ← callResult reference ty (mkApp applied bundle)
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
        let typeArgumentTypes := instantiations.map fun instantiation =>
          match instantiation with
          | .typeArg argument => context.ntyOf? argument.typeId
          | _ => none
        let some qualified := context.unit.tables.names[reference.name.index]?
          | throwError "specification function call has an unknown name"
        unless qualified.namespaceId == reference.namespaceId do
          throwError "specification function call name belongs to a different namespace"
        let some targetNs := context.unit.namespaces[reference.namespaceId.index]?
          | throwError "specification function call namespace is out of range"
        if let some (role, owner) := mapSpecRole? targetNs reference then
          return ← mapSpecCall role owner ty arguments
        -- An uninterpreted value of the call: an opaque specification
        -- function's, or a native's, which has no body to derive a
        -- specification version from, as the Move Prover reads it.
        let opaqueCall : MetaM Lean.Expr := do
          let some path := context.unit.tables.namespaces[reference.namespaceId.index]?
            | throwError "specification function namespace has no path"
          let own := "::".intercalate (path.segments.toList ++ [qualified.name])
          -- A native read by its Prover model denotes the model's value.
          let model : Option NativeModel := do
            let functionId ← context.unit.resolution.function? reference.name
            let declaration ← targetNs.functions[functionId.index]?
            guard (declaration.body == LeanerIR.Validation.FunctionBody.absent)
            nativeModel? targetNs.profile own
          if let some .structuralOrder := model then
            let [left, right] := arguments.toList
              | throwError "a structural order compares two operands"
            return ← orderingValue context ty
              (← structuralOrderTerm context (← runtimeOperand left) (← runtimeOperand right))
          let name : String := match model with
            | some (.uninterpreted _ (some function) _) => function
            | _ => own
          -- The type arguments as native types under the contract's family,
          -- which an instantiation of the contract resolves to the caller's,
          -- or `num`.
          let typeArguments := if arbitraryValue context.unit reference then #[]
            else instantiations.zip typeArgumentTypes
          let typeArgumentTerms ← typeArguments.mapM
            fun (instantiation, nty?) => do
              let some nty := nty?
                | match instantiation with
                  | .typeArg argument =>
                      match context.typeOf? argument.typeId with
                      | some (.integer .unbounded _) =>
                          return mkConst ``LeanerLang.Contract.SpecTypeArgument.integer
                      | _ => throwError m!"a type argument of the specification function \
                          `{name}` has no native type ({repr instantiation})"
                  | _ => throwError m!"a type argument of the specification function \
                      `{name}` has no native type ({repr instantiation})"
              let some types := context.types
                | throwError "the specification function `{name}` is applied outside a family"
              return mkApp (mkConst ``LeanerLang.Contract.SpecTypeArgument.native)
                (mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.substWith) types (← quoteNTy nty))
          let encoded ← arguments.mapM runtimeOperand
          let domain := domainOf ty
          let value ← mkAppOptM ``LeanerLang.Contract.opaqueSpec
            #[toExpr name,
              ← mkListLit (mkConst ``LeanerLang.Contract.SpecTypeArgument) typeArgumentTerms.toList,
              domain.leanType, none, ← mkListLit (mkConst ``RuntimeValue) encoded.toList]
          domain.ofBinder value
        let some functionId := context.unit.resolution.specFunction? reference.name
          | if (context.unit.resolution.function? reference.name).isSome then opaqueCall
            else throwError "specification function call target does not resolve"
        let some declaration := targetNs.specFunctions[functionId.index]?
          | throwError "specification function declaration is out of range"
        unless arguments.size == declaration.signature.parameters.size do
          throwError "specification function call argument count differs from its declaration"
        let some body := declaration.body
          | opaqueCall
        unless declaration.signature.results.size == 1 do
          throwError "generated contracts currently expand specification functions with one result"
        let callee : Context := {
          context with
          namespaceId := reference.namespaceId
          ns := targetNs
          results := #[]
          resultTypes := #[]
          specCallStack := context.specCallStack.push reference
          typeArguments, typeArgumentTypes }
        let mut targetLocals : Array (Option Lean.Expr) :=
          Array.replicate declaration.locals.size none
        let mut targetTypes : Array IrTy := #[]
        for localDecl in declaration.locals do
          let some localType := callee.valueTypeOf? localDecl.type.typeId
            | throwError "specification function local has an unknown type"
          targetTypes := targetTypes.push localType
        let mut values : Array Lean.Expr := #[]
        let mut parameterTypes : Array IrTy := #[]
        for (argument, index) in arguments.zipIdx do
          let some parameter := declaration.signature.parameters[index]?
            | throwError "specification function parameter is out of range"
          let some parameterType := callee.valueTypeOf? parameter.typeUse.typeId
            | throwError "specification function parameter has an unknown type"
          let value ← translate context argument
          let value ← match domainOf parameterType with
            | .boolean => mkDecide value
            | _ => pure value
          targetLocals := targetLocals.set! index (some value)
          values := values.push value
          parameterTypes := parameterTypes.push parameterType
        let expanded ← translate { callee with
          locals := targetLocals, localTypes := targetTypes, oldLocals := targetLocals
          localNames := declaration.locals.map (·.name) } body
        -- The function is defined where its arguments fit the fixed-width
        -- types of its parameters, and unspecified elsewhere.
        let some bounds ← valueBounds values (parameterTypes.map domainOf) parameterTypes
          | return expanded
        let outside ← callResult reference ty
          (← outsideValue reference (← bundleArguments reference arguments))
        mkAppM ``ite #[bounds, expanded, outside]
    | .specification (.result index) =>
        match context.ns.profile, context.resultTypes[0]?, context.results[0]? with
        | some .move, some (.tuple _), some packed =>
            -- Move presents multiple returns as one tuple-typed physical
            -- result. `spec.result[i]` nevertheless denotes component `i`.
            -- Projecting from the represented runtime tuple preserves that
            -- source-level view without inventing extra result-row slots.
            let selected ← mkAppM ``LeanerIR.RuntimeValue.field
              #[packed, toExpr index]
            (domainOf ty).ofAggregate (← context.observeInput resultTypeId selected)
        | _, _, _ =>
            let some binder := context.results[index]?
              | throwError "specification result {index} has no binder"
            let some logicalType := context.resultTypes[index]?
              | throwError "specification result {index} has no logical domain"
            context.observeInput resultTypeId (← (domainOf logicalType).ofBinder binder)
    | .specification .final =>
        if let some type := context.ntyOf? (specTypeId context.unit resultTypeId) then
          if snapshotType context.unit type then
            throwError "a final Table reference needs its future contents observation, which is not carried yet"
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
    | .specification (.behavior kind range) => translateBehavior kind range arguments ty
    | .specification .old =>
        let some argument := arguments[0]?
          | throwError "spec.old expects one argument"
        translate { context with
          locals := context.oldLocals, state := context.oldState } argument
    | .specification (.withStateAnchor label) =>
        -- `old` inside the anchor reads the state the anchor saved.
        let some argument := arguments[0]?
          | throwError "spec.withStateAnchor expects one argument"
        let (anchorLocals, anchorState) ← match context.labeledAnchors.find? (·.1 == label) with
          | some (_, locals, state) => pure (locals, state)
          | none =>
              let some anchorState := context.anchorState
                | throwError "a state anchor is read where no anchor is recorded"
              pure (context.anchorLocals, anchorState)
        translate { context with
          oldLocals := anchorLocals, oldState := some anchorState } argument
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
        let selected ← aggregateField (← runtimeAggregate vector)
          (← mkAppM ``Int.toNat #[← translate context index])
        (domainOf ty).ofAggregate selected
    | .specification .lengthVector =>
        let some vector := arguments[0]?
          | throwError "spec.lengthVector expects a vector operand"
        -- A declaration local is its physical field (or an already logical
        -- binder). Length does not inspect generic elements, unlike equality,
        -- membership, or an opaque specification call. Restrict this shortcut
        -- to a bare local so it cannot change an operand's branching tests.
        if context.physicalInvariantLengths then
          if let some expression := context.ns.expressions[vector.index]? then
            if let .localVar localId := expression.kind then
              if let some (some binder) := context.locals[localId.index]? then
                let some localType := context.localTypes[localId.index]?
                  | throwError "a stored invariant field has no type"
                let value ← runtimeAggregateValue context vector
                  (← (domainOf localType).ofBinder binder)
                if ← isSnapshotValue value then
                  return ← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorLength #[value]
                return ← mkAppM ``LeanerLang.Contract.lengthVector #[value]
        let vector ← runtimeAggregate vector
        if ← isSnapshotValue vector then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorLength #[vector]
        else mkAppM ``LeanerLang.Contract.lengthVector #[vector]
    | .specification .containsVector =>
        let some vector := arguments[0]?
          | throwError "spec.containsVector expects a vector operand"
        let some element := arguments[1]?
          | throwError "spec.containsVector expects an element operand"
        let vector ← runtimeAggregate vector
        let element ← logicalOperand element
        if (← isSnapshotValue vector) || (← isSnapshotValue element) then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorContains
            #[← snapshotOperand vector, ← snapshotOperand element]
        else mkAppM ``LeanerLang.Contract.containsVector #[vector, element]
    | .specification .emptyVector =>
        mkAppM ``LeanerIR.RuntimeValue.vector #[← mkArrayLit (mkConst ``LeanerIR.RuntimeValue) []]
    | .specification .concatVector =>
        let (some left, some right) := (arguments[0]?, arguments[1]?)
          | throwError "spec.concatVector expects two vector operands"
        let (left, right) ← aggregateBranches (← runtimeAggregate left) (← runtimeAggregate right)
        if ← isSnapshotValue left then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorConcat #[left, right]
        else mkAppM ``LeanerLang.Contract.concatVector #[left, right]
    | .specification .sliceVector =>
        let (some vector, some range) := (arguments[0]?, arguments[1]?)
          | throwError "spec.sliceVector expects a vector and a range"
        let (lower, upper) ← translateRangeBounds context range
        let vector ← runtimeAggregate vector
        let lower ← mkAppM ``Int.toNat #[lower]
        let upper ← mkAppM ``Int.toNat #[upper]
        if ← isSnapshotValue vector then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorSlice #[vector, lower, upper]
        else mkAppM ``LeanerLang.Contract.sliceVector #[vector, lower, upper]
    | .specification .singletonVector =>
        let some element := arguments[0]?
          | throwError "a singleton vector expects one element"
        aggregateConstructor ``RuntimeValue.vector #[]
          (mkConst ``LeanerIR.Proofs.Denote.SnapshotValue.Shape.vector) #[← logicalOperand element]
    | .specification .updateVector =>
        let some vector := arguments[0]?
          | throwError "spec.updateVector expects a vector operand"
        let some index := arguments[1]?
          | throwError "spec.updateVector expects an index operand"
        let some replacement := arguments[2]?
          | throwError "spec.updateVector expects a replacement operand"
        let vector ← runtimeAggregate vector
        let index ← mkAppM ``Int.toNat #[← translate context index]
        let replacement ← logicalOperand replacement
        if (← isSnapshotValue vector) || (← isSnapshotValue replacement) then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorUpdate
            #[← snapshotOperand vector, index, ← snapshotOperand replacement]
        else mkAppM ``LeanerLang.Contract.updateVector #[vector, index, replacement]
    | .specification (.publish range) | .specification (.remove range) |
        .specification (.update range) =>
        let .specification operation := operation | unreachable!
        let post ← match range.post with
          | some label => (·.memory) <$> context.labelState label
          | none => currentState
        let (memory, condition) ← translateStateChange context operation instantiations arguments
          (some post)
        mkAppM ``And #[← mkEq post memory, condition]
    | .specification (.global label) =>
        let some key := arguments[0]?
          | throwError "a storage read expects one key"
        let state ← match label with
          | some label => (·.memory) <$> context.labelState label
          | none => currentState
        let (slot, resource, nativeType) ← context.slot state
          (← resourceType instantiations) (← runtimeOperand key)
        let encoded ← slotEncoding (← context.frame) nativeType resource slot
        (domainOf ty).ofAggregate (← context.observeInput (← resourceType instantiations) encoded (some state))
    | .specification (.exists label) =>
        let some key := arguments[0]?
          | throwError "a storage existence test expects one key"
        let state ← match label with
          | some label => (·.memory) <$> context.labelState label
          | none => currentState
        let (slot, _, _) ← context.slot state (← resourceType instantiations)
          (← runtimeOperand key)
        slotPresent slot
    | .global .contains =>
        let some key := arguments[0]?
          | throwError "a storage existence test expects one key"
        let (slot, _, _) ← context.slot (← currentState) (← resourceType instantiations)
          (← runtimeOperand key)
        slotPresent slot
    | .data (.select reference field) =>
        let some base := arguments[0]?
          | throwError "a field selection expects one operand"
        let some index := LeanerIR.SemanticOperations.referencedFieldIndex?
            context.unit context.namespaceId reference none field
          | throwError "specification field `{field}` does not resolve in \
              generated contracts"
        let selected ← aggregateField (← runtimeAggregate base) (toExpr index)
        (domainOf ty).ofAggregate selected
    | .data (.selectVariants reference fields) =>
        let some base := arguments[0]?
          | throwError "a variant field selection expects one operand"
        let handle ← structHandleOf context reference
        let some choices := LeanerIR.SemanticOperations.variantFieldChoices?
            context.unit handle fields
          | throwError "a variant field selection has no choices"
        let aggregate ← runtimeAggregate base
        let selected ← if ← isSnapshotValue aggregate then
            mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.selectVariantField
              #[aggregate, toExpr handle, toExpr choices]
          else mkAppM ``LeanerLang.Contract.selectVariantField #[aggregate, toExpr handle, toExpr choices]
        (domainOf ty).ofAggregate selected
    | .data (.updateField reference field) =>
        let #[base, replacement] := arguments
          | throwError "a field update expects two operands"
        let handle ← structHandleOf context reference
        let some owner := context.unit.namespaces[handle.namespaceId.index]?
          | throwError "a field update has no owner namespace"
        let some declaration := owner.structs[handle.structId]?
          | throwError "a field update has no owner declaration"
        if owner.intrinsics.any (fun intrinsic =>
            intrinsic.model == "map" && intrinsic.owner == declaration.name) then
          throwError "cannot update a field of an intrinsic map type in a specification"
        let variants : Array (Option String) := if declaration.variants.isEmpty then #[none]
          else declaration.variants.map fun variant =>
            (owner.tables.names[variant.name.index]?).map (·.name)
        let choices := variants.toList.filterMap fun variant =>
          (LeanerIR.SemanticOperations.handleFieldIndex? context.unit handle variant field).map
            (variant, ·)
        unless choices.length == variants.size do
          throwError "a field update on an enum whose variants do not all carry the field is not supported yet"
        let aggregate ← runtimeAggregate base
        let replacement ← logicalOperand replacement
        if (← isSnapshotValue aggregate) || (← isSnapshotValue replacement) then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.updateNominalField
            #[← snapshotOperand aggregate, toExpr handle, toExpr choices, ← snapshotOperand replacement]
        else mkAppM ``LeanerLang.Contract.updateNominalField #[aggregate, toExpr handle, toExpr choices, replacement]
    | .data (.testVariants reference variants) =>
        let some base := arguments[0]?
          | throwError "a variant test expects one operand"
        let handle ← structHandleOf context reference
        let variants ← mkArrayLit (mkConst ``String)
          (variants.toList.map toExpr)
        let test ← aggregateTestVariants (← runtimeAggregate base) (toExpr handle) variants
        mkEq test (mkConst ``Bool.true)
    | .call (.constructor reference variant) =>
        let handle ← structHandleOf context reference
        let variant ← match variant with
          | some name => mkAppM ``Option.some #[toExpr name]
          | none => pure (mkApp (mkConst ``Option.none [Lean.Level.zero])
              (mkConst ``String))
        let operands ← arguments.mapM logicalOperand
        let shape ← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Shape.nominal #[toExpr handle, variant]
        context.observeInput resultTypeId
          (← aggregateConstructor ``RuntimeValue.nominal #[toExpr handle, variant] shape operands)
    | .reference .dereference =>
        -- Specifications see through references: the accessors read a
        -- borrow's content, so a dereference is the operand itself.
        let some argument := arguments[0]?
          | throwError "a dereference expects one operand"
        translate context argument
    | .primitive primitive => translatePrimitive primitive arguments ty
    | .call (.closure reference mask) =>
        -- A function value has exactly the runtime closure's captures.
        -- Specification integers need not carry native boundedness proofs,
        -- so captured values are encoded without inventing those proofs.
        unless instantiations.isEmpty do
          throwError "a function value with type arguments is not carried in generated contracts"
        let some handle := LeanerIR.SemanticOperations.resolveFunction? context.unit
            context.namespaceId reference
          | throwError "a function value's target does not resolve"
        let some targetNs := context.unit.namespaces[handle.namespaceId.index]?
          | throwError "a function value's target namespace is out of range"
        let some declaration := targetNs.functions[handle.functionId.index]?
          | throwError "a function value's target is out of range"
        unless declaration.signature.generics.isEmpty do
          throwError "a function value of a generic target is not carried in generated contracts"
        if !arguments.isEmpty then
          return mkApp4 (mkConst ``LeanerIR.RuntimeValue.closure) (toExpr handle) (toExpr mask)
            (← mkArrayLit (← mkAppM ``Prod #[mkConst ``TypeId, mkConst ``TypeId]) [])
            (← mkArrayLit (mkConst ``RuntimeValue) (← arguments.toList.mapM runtimeOperand))
        let nativeTypes (typeUses : Array LeanerIR.TypeUse) :
            MetaM (List LeanerIR.Proofs.Denote.NTy) :=
          typeUses.toList.mapM fun typeUse => do
            let some nty :=
                LeanerIR.Proofs.Denote.ntyOf context.unit handle.namespaceId typeUse.typeId
              | throwError "a function value's target has a type without a native type"
            pure nty
        let full ← nativeTypes (declaration.signature.parameters.map (·.typeUse))
        let ⟨captured, supplied, weave⟩ := LeanerIR.Proofs.Denote.Weave.ofMask mask full
        let some skolems := context.skolems?
          | throwError "a function value in a specification needs the contract's skolem instance"
        let frameUnit ← frameUnit skolems
        let closure := mkAppN (mkConst ``LeanerIR.Proofs.Denote.closureOf)
          #[frameUnit, skolems, toExpr handle,
            mkAppN (mkConst ``LeanerIR.Proofs.Denote.Weave.mask)
              #[← quoteRow (.ofList full), ← quoteRow captured, ← quoteRow supplied,
                ← quoteWeave weave],
            ← mkArrayLit (← mkAppM ``Prod #[mkConst ``TypeId, mkConst ``TypeId]) [],
            ← quoteRow captured, mkConst ``Unit.unit]
        return mkApp (mkConst ``LeanerIR.Proofs.Denote.ClosureValue.encode) closure
    | _ =>
        throwError "specification operation {repr operation} is not supported \
          in generated contracts"
  /-- A behavioral predicate over the invocation of a function value
(`Proofs/Behavior.lean`): from the function's entry state, and in
`ensures` to its exit state, as the Move Book reads a predicate without
state labels. -/
  translateBehavior (kind : LeanerIR.BehaviorKind) (range : LeanerIR.MemoryRange)
      (arguments : Array ExprId) (ty : IrTy) : MetaM Lean.Expr := do
    let (executable, callableValue, inputValues, pre) ← translateInvocation context range arguments
    let post ← match range.post with
      | some label => (·.memory) <$> context.labelState label
      | none => currentState
    match kind with
    | .abortsOf =>
        mkAppM ``LeanerIR.Proofs.AbortsOf #[executable, callableValue, inputValues, pre]
    | .ensuresOf =>
        let some callableExpression := context.ns.expressions[arguments[0]!.index]?
          | throwError "a behavioral predicate's function value is out of range"
        let some (.function parameters _ _) := context.valueTypeOf? callableExpression.typeId
          | throwError "a behavioral predicate's operand is not a function value"
        let results ← mkArrayLit (mkConst ``LeanerIR.RuntimeValue)
          (← (arguments.extract (parameters.size + 1) arguments.size).toList.mapM runtimeOperand)
        mkAppM ``LeanerIR.Proofs.EnsuresOf
          #[executable, callableValue, inputValues, results, pre, post]
    | .resultOf =>
        let results ← mkAppM ``LeanerIR.Proofs.ResultOf
          #[executable, callableValue, inputValues, pre]
        (domainOf ty).ofRuntime (← mkAppM ``LeanerIR.SemanticOperations.packResults #[results])
    | .requiresOf =>
        let some table := context.requiresTable
          | throwError "`requires_of` reads the table of declared preconditions, which this \
              contract does not take"
        mkAppM ``LeanerIR.Proofs.RequiresOf #[table, callableValue, inputValues, pre]
    | .unchangedOf | .foldsOf | .writeOf _ =>
        throwError "behavioral predicate {repr kind} is not carried in generated contracts"
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
        aggregateConstructor ``RuntimeValue.vector #[]
          (mkConst ``LeanerIR.Proofs.Denote.SnapshotValue.Shape.vector) (← arguments.mapM logicalOperand)
    -- A tuple is a runtime value in a clause, as a vector is.
    | .tuple =>
        aggregateConstructor ``RuntimeValue.tuple #[]
          (mkConst ``LeanerIR.Proofs.Denote.SnapshotValue.Shape.tuple) (← arguments.mapM logicalOperand)
    | .pushVector =>
        let some vector := arguments[0]?
          | throwError "pushVector expects a vector operand"
        let some element := arguments[1]?
          | throwError "pushVector expects an element operand"
        let vector ← runtimeAggregate vector
        let element ← logicalOperand element
        if (← isSnapshotValue vector) || (← isSnapshotValue element) then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorPush
            #[← snapshotOperand vector, ← snapshotOperand element]
        else mkAppM ``LeanerLang.Contract.pushVector #[vector, element]
    | .concatVector =>
        let some left := arguments[0]?
          | throwError "concatVector expects two vector operands"
        let some right := arguments[1]?
          | throwError "concatVector expects two vector operands"
        let (left, right) ← aggregateBranches (← runtimeAggregate left) (← runtimeAggregate right)
        if ← isSnapshotValue left then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.vectorConcat #[left, right]
        else mkAppM ``LeanerLang.Contract.concatVector #[left, right]
    | .compare =>
        let (left, right) ← match arguments.toList with
          | [left, right] => pure (left, right)
          | _ => throwError "compare expects two operands"
        let rank := mkApp (mkConst ``LeanerIR.SemanticOperations.valueRanks)
          (toExpr context.ns.orders)
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
        if operandTy == LeanerIR.Ty.bool then mkAppM ``Iff #[l, r]
        else if (← isSnapshotValue l) || (← isSnapshotValue r) then
          mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.SameIdentity
            #[← snapshotOperand l, ← snapshotOperand r]
        else mkAppM ``Eq #[l, r]
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
        else if (← isSnapshotValue l) || (← isSnapshotValue r) then
          mkAppM ``Not #[← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.SameIdentity
            #[← snapshotOperand l, ← snapshotOperand r]]
        else mkAppM ``Ne #[l, r]
    | .logicalAnd => let (l, r) ← binary arguments; mkAppM ``And #[l, r]
    | .logicalOr => let (l, r) ← binary arguments; mkAppM ``Or #[l, r]
    | .logicalNot => let value ← unary arguments; mkAppM ``Not #[value]
    | .implies => let (l, r) ← binary arguments; return ← mkArrow l r
    | .equivalent => let (l, r) ← binary arguments; mkAppM ``Iff #[l, r]
    | _ =>
        throwError "specification primitive {repr primitive} at type \
          {describeType ty} is not supported in generated contracts"

end

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
    -- So is an annotation opening the loop's body, at the head of every
    -- iteration.
    if let some { kind := .block statements _, .. } := ns.expressions[body.index]? then
      if let some (first : ExprId) := statements[0]? then
        if let some { kind := .spec block, .. } := ns.expressions[first.index]? then
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
  -- Several annotations can name one site: one preceding the loop, found
  -- here, and the blocks its header begins with, found by the recursive
  -- visit. The site's specification is all of their conditions, in order;
  -- the default, discovered after them, adds none.
  found := found.foldl (fun acc (site, block) =>
    match acc.findIdx? (·.1 == site) with
    | some index => acc.modify index fun (site, previous) =>
        (site, { previous with
          pragmas := previous.pragmas ++ block.pragmas
          conditions := previous.conditions ++ block.conditions
          frame := previous.frame <|> block.frame })
    | none => acc.push (site, block)) #[]
  return found

/-- A condition's clause marked with the condition's range, so that a
failure inside it is reported at the condition. -/
def markCondition (unit : ValidatedUnit) (condition : LeanerIR.Condition)
    (clause : Lean.Expr) : Lean.Expr :=
  markObligation (conditionRange unit condition) clause

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
  range : ObligationRange := {}

/-- A contract's `let`: the local it binds, its value, and whether that is
read in the post-state. -/
private structure ContractLet where
  name : String
  value : ExprId
  post : Bool

private structure ClauseGroups where
  requires : Array ExprId := #[]
  ensures : Array (ExprId × ObligationRange) := #[]
  abortsIf : Array AbortClause := #[]
  abortsWith : Array (ExprId × ObligationRange) := #[]
  lets : Array ContractLet := #[]

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

/-- Whether the lowered contract sets a boolean pragma.  Lowering already
merged the namespace's pragmas into the contract, contract-first, so the
declared attribute list is authoritative. -/
private def pragmaEnabled (contract : LeanerIR.FunctionContract) (name : String) : Bool :=
  contract.pragmas.any fun pragma =>
    match pragma with
    | .assign pragmaName (.constant (.bool true)) _ => pragmaName == name
    | _ => false

/-- The expressions a function's verification reads: the conditions of its
contract and its body. -/
private def functionRoots (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array ExprId :=
  let roots := declaration.contract.conditions.flatMap fun condition =>
    #[condition.expression] ++ condition.auxiliary.map (·.2)
  match declaration.body with
  | .structured root => roots.push root
  | _ => roots

/-- Fold `visit` over the operations the expressions `roots` reach: in
them, in the specification functions they expand, and in the functions they
call, at any depth. `false` with the fold when the walk exceeds its bound. -/
private def reachFold {α : Type} (unit : ValidatedUnit) (roots : List (LeanerIR.NamespaceId × ExprId))
    (init : α) (visit : α → LeanerIR.Operation → Array LeanerIR.GenericArgument → α) :
    α × Bool := Id.run do
  let mut work : List (LeanerIR.NamespaceId × ExprId) := roots
  let mut visited : Std.HashSet (Nat × Nat) := {}
  let mut folded := init
  let bound := unit.namespaces.foldl (fun total ns => total + ns.expressions.size) 1
  for _ in [0:bound] do
    match work with
    | [] => break
    | (owner, id) :: rest =>
        work := rest
        if visited.contains (owner.index, id.index) then continue
        visited := visited.insert (owner.index, id.index)
        let some ownerNs := unit.namespaces[owner.index]? | continue
        let some expression := ownerNs.expressions[id.index]? | continue
        if let .operation operation instantiations _ _ := expression.kind then
          folded := visit folded operation instantiations
          match operation with
          | .specification (.functionCall reference _) =>
              let body? := do
                let targetNs ← unit.namespaces[reference.namespaceId.index]?
                let functionId ← unit.resolution.specFunction? reference.name
                let callee ← targetNs.specFunctions[functionId.index]?
                callee.body
              if let some body := body? then work := (reference.namespaceId, body) :: work
          | .call (.function reference) =>
              if let some calleeNs := unit.namespaces[reference.namespaceId.index]? then
                if let some functionId := unit.resolution.function? reference.name then
                  if let some callee := calleeNs.functions[functionId.index]? then
                    work := (functionRoots callee).toList.map (reference.namespaceId, ·) ++ work
          | _ => pure ()
        work := (LeanerIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·) ++ work
  return (folded, work.isEmpty)

/-- Whether a type mentions no type parameter. -/
private def closedType (unit : ValidatedUnit) (typeId : TypeId) : Bool :=
  go typeId 32
where
  go (typeId : TypeId) : Nat → Bool
    | 0 => false
    | fuel + 1 => match unit.tables.types[typeId.index]? with
      | some (.typeParameter _) | none => false
      | some (.vector element _) | some (.typeDomain element) => go element fuel
      | some (.reference reference) => go reference.referent fuel
      | some (.tuple elements) => elements.all (go · fuel)
      | some (.function arguments result _) => arguments.all (go · fuel) && go result fuel
      | some (.nominal _ arguments) => arguments.all fun
          | .typeArg typeUse => go typeUse.typeId fuel
          | _ => true
      | some _ => true

/-- The specification functions a function's verification applies, each at
its type arguments in the function's frame: in the function's body and
contract, in the contracts of the functions it calls, and in the
specification functions these expand. A callee's or an expansion's type
parameter is the argument its application gives it; an application at a type
that mentions another parameter is left out. -/
def specInstantiations (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array (LeanerIR.QualifiedRef × Array TypeId) := Id.run do
  -- Each expression with the arguments of its owner's type parameters, none
  -- in the function itself, whose parameters are its own.
  let mut work : List (LeanerIR.NamespaceId × ExprId × Option (Array (Option TypeId))) :=
    (functionRoots declaration).toList.map (namespaceId, ·, none)
  let mut visited : Std.HashSet (Nat × Nat × List (Option Nat)) := {}
  let mut applied : Array (LeanerIR.QualifiedRef × Array TypeId) := #[]
  let bound := unit.namespaces.foldl (fun total ns => total + ns.expressions.size) 1
  for _ in [0:bound] do
    match work with
    | [] => break
    | (owner, id, arguments?) :: rest =>
        work := rest
        let key := (owner.index, id.index,
          (arguments?.getD #[]).toList.map (·.map (·.index)))
        if visited.contains key then continue
        visited := visited.insert key
        let some ownerNs := unit.namespaces[owner.index]? | continue
        let some expression := ownerNs.expressions[id.index]? | continue
        let inFrame (typeId : TypeId) : Option TypeId := match arguments? with
          | none => some typeId
          | some arguments => match unit.tables.types[typeId.index]? with
            | some (.typeParameter index) => arguments[index]?.join
            | _ => if closedType unit typeId then some typeId else none
        if let .operation operation instantiations _ _ := expression.kind then
          let inner := instantiations.map fun
            | .typeArg typeUse => inFrame typeUse.typeId
            | _ => none
          match operation with
          | .specification (.functionCall reference _) =>
              if let some typeArguments := inner.mapM (fun argument => argument) then
                unless applied.contains (reference, typeArguments) do
                  applied := applied.push (reference, typeArguments)
              let body? := do
                let targetNs ← unit.namespaces[reference.namespaceId.index]?
                let functionId ← unit.resolution.specFunction? reference.name
                (← targetNs.specFunctions[functionId.index]?).body
              if let some body := body? then
                work := (reference.namespaceId, body, some inner) :: work
          | .call (.function reference) =>
              -- A callee's contract, not its body, which its own verification reads.
              if arguments?.isNone then
                if let some calleeNs := unit.namespaces[reference.namespaceId.index]? then
                  if let some functionId := unit.resolution.function? reference.name then
                    if let some callee := calleeNs.functions[functionId.index]? then
                      let conditions := callee.contract.conditions.flatMap fun condition =>
                        #[condition.expression] ++ condition.auxiliary.map (·.2)
                      work := conditions.toList.map (reference.namespaceId, ·, some inner) ++ work
          | _ => pure ()
        work := (LeanerIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·, arguments?) ++ work
  return applied

/-- The instantiations a generic axiom is assumed at: those that apply a
specification function the axiom applies at a type argument list
`applied` holds, as the Move Prover instantiates an axiom at the
instantiations a verification uses. -/
private def axiomInstances (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.NamespaceInvariant) (parameters : Nat)
    (applied : Array (LeanerIR.QualifiedRef × Array TypeId)) : Array (Array TypeId) := Id.run do
  let mut own : Array (LeanerIR.QualifiedRef × Array TypeId) := #[]
  let mut work := [declaration.condition.expression]
  for _ in [0:ns.expressions.size + 1] do
    match work with
    | [] => break
    | id :: rest =>
        work := rest
        let some expression := ns.expressions[id.index]? | continue
        if let .operation (.specification (.functionCall reference _)) instantiations _ _ :=
            expression.kind then
          if let some arguments := instantiations.mapM (fun
              | .typeArg typeUse => some typeUse.typeId
              | _ => none) then
            own := own.push (reference, arguments)
        work := (LeanerIR.Validation.expressionChildren expression.kind).toList ++ work
  let mut instances : Array (Array TypeId) := #[]
  for (reference, pattern) in own do
    for (target, arguments) in applied do
      unless target == reference && arguments.size == pattern.size do continue
      let mut binding : Array (Option TypeId) := Array.replicate parameters none
      let mut matched := true
      for (formal, actual) in pattern.zip arguments do
        match unit.tables.types[formal.index]? with
        | some (.typeParameter index) =>
            match binding[index]? with
            | some none => binding := binding.set! index (some actual)
            | some (some bound) => if bound != actual then matched := false
            | none => matched := false
        | _ => if formal != actual then matched := false
      if matched then
        if let some found := binding.mapM (fun argument => argument) then
          unless instances.contains found do instances := instances.push found
  return instances

/-- The resource declarations what the expressions `roots` reach reaches in
global memory, or with `writes` writes, `none` when it reaches none. Past the
walk's bound, memory is conservatively reachable. -/
private def reachFrom (unit : ValidatedUnit) (roots : List (LeanerIR.NamespaceId × ExprId))
    (writes : Bool := false) : Option (Array LeanerIR.StructHandle) :=
  let ((reached, resources), complete) := reachFold unit roots (false, #[])
    fun (reached, resources) operation instantiations =>
      let counts := match operation with
        | .global (.borrow .mutable) | .global .take | .global .publish => true
        | .global _ | .specification (.global _) | .specification (.publish _) |
            .specification (.remove _) | .specification (.update _) => !writes
        | _ => false
      if !counts then (reached, resources) else
        (true, instantiations.foldl (init := resources) fun resources instantiation =>
          match instantiation with
          | .typeArg typeUse =>
              match unit.tables.types[typeUse.typeId.index]?.bind (nominalDeclaration? unit) with
              | some (handle, _, _) =>
                  if resources.contains handle then resources else resources.push handle
              | none => resources
          | _ => resources)
  if reached || !complete then some resources else none

/-- Handle-backed map roles read external storage even when their arguments
are local Tables and there is no global-resource operation in the body. -/
private def tableOperationOwner? (unit : ValidatedUnit) (operation : LeanerIR.Operation) :
    Option LeanerIR.StructHandle := do
  let reference ← match operation with
    | .call (.function reference) | .specification (.functionCall reference _) => some reference
    | _ => none
  let ns ← unit.namespaces[reference.namespaceId.index]?
  let intrinsic ← ns.intrinsics.find? fun intrinsic =>
    intrinsic.model == "map" &&
      (intrinsic.executableBindings ++ intrinsic.specBindings).any (·.target == reference)
  tableModel? unit (.nominal intrinsic.owner #[])

private def tableReach (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array LeanerIR.StructHandle :=
  if !hasTableModel unit then #[] else
  (reachFold unit ((functionRoots declaration).toList.map (namespaceId, ·)) #[]
    fun owners operation _ => match tableOperationOwner? unit operation with
      | some owner => if owners.contains owner then owners else owners.push owner
      | none => owners).1

/-- The resource declarations a function can reach in global memory:
through a global operation or a storage clause in its body or contract, in a
specification function one expands, or in a function it calls, at any
depth. `none` when it reaches no global memory. -/
def memoryReach (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Option (Array LeanerIR.StructHandle) := Id.run do
  let globals := reachFrom unit ((functionRoots declaration).toList.map (namespaceId, ·))
  let tables := tableReach unit namespaceId declaration
  if tables.isEmpty then return globals
  return some (tables.foldl (fun owners owner =>
    if owners.contains owner then owners else owners.push owner) (globals.getD #[]))

/-- The qualified name of a function or specification function. -/
private def qualifiedName? (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    Option String := do
  let path ← unit.tables.namespaces[reference.namespaceId.index]?
  let qualified ← unit.tables.names[reference.name.index]?
  some ("::".intercalate (path.segments.toList ++ [qualified.name]))

/-- The functions of `names` a function applies, in code or in a
specification, at any depth (`reachFold`); past the walk's bound, all of
them. -/
private def reachedFunctions (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (names : List String) : List String :=
  if names.isEmpty then [] else
  let (reached, complete) :=
    reachFold unit ((functionRoots declaration).toList.map (namespaceId, ·)) []
      fun reached operation _ => match operation with
        | .call (.function reference) | .specification (.functionCall reference _) =>
            match qualifiedName? unit reference with
            | some name => if names.contains name && !reached.contains name then name :: reached
                else reached
            | none => reached
        | _ => reached
  if complete then names.filter reached.contains else names

/-- The implicit existence premise for one totalized invariant read. -/
private def invariantReadGuard (context : Context) (read : InvariantValueRead) :
    MetaM Lean.Expr := do
  let readContext := if read.atEntry then
      { context with locals := context.oldLocals, state := context.oldState }
    else context
  let some state := readContext.state
    | throwError "a namespace invariant resource read has no state"
  let some keyTy := typeOfExpr? readContext read.key
    | throwError "a namespace invariant resource key has an unknown type"
  let encodedKey ← (domainOf keyTy).encode (← translate readContext read.key)
  let (slot, _, _) ← context.slot state ⟨read.typeIndex⟩ encodedKey
  slotPresent slot

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
assumptions at entry; axioms are assumptions at entry and never obligations,
a generic one at each instantiation of `applied` it applies
(`axiomInstances`). -/
private def namespaceInvariantTerms (context : Context)
    (modifiedResources : Option (Array ModifiedResource)) (phase : InvariantPhase)
    (relevant : LeanerIR.NamespaceId → LeanerIR.NamespaceInvariant → Bool := fun _ _ => true)
    (applied : Array (LeanerIR.QualifiedRef × Array TypeId) := #[]) :
    MetaM (Array (Lean.Expr × ObligationRange)) := do
  let mut terms := #[]
  -- The invariants of every namespace of the unit, each read in its own.
  let invariants := (context.unit.namespaces.toList.zipIdx).flatMap fun (ns, index) =>
    ns.invariants.toList.map fun invariant => ((⟨index⟩ : LeanerIR.NamespaceId), ns, invariant)
  let invariants := invariants.flatMap fun (namespaceId, ns, invariant) =>
    match invariant.condition.kind with
    | .axiom_ typeParameters =>
        if typeParameters.isEmpty then [(namespaceId, ns, invariant, #[])] else
          (axiomInstances context.unit ns invariant typeParameters.size applied).toList.map
            fun arguments => (namespaceId, ns, invariant, arguments)
    | _ => [(namespaceId, ns, invariant, #[])]
  for (invariantNamespaceId, invariantNs, declaration, axiomArguments) in invariants do
    unless relevant invariantNamespaceId declaration do continue
    let (typeParameters, isUpdate, isAxiom) ← match declaration.condition.kind with
      | .globalInvariant typeParameters => pure (typeParameters, false, false)
      | .globalInvariantUpdate typeParameters => pure (typeParameters, true, false)
      | .axiom_ typeParameters => pure (typeParameters, false, true)
      | kind => throwError "namespace condition {repr kind} is not a global invariant"
    if phase == .entry && isUpdate then continue
    if phase == .exit && isAxiom then continue
    unless typeParameters.isEmpty || isAxiom do
      throwError "generic namespace invariants are not supported in generated contracts"
    unless declaration.condition.auxiliary.isEmpty do
      throwError "namespace invariants cannot carry auxiliary expressions"
    let localTypes ← declaration.locals.mapM fun localDecl => do
      let some ty := context.unit.tables.types[localDecl.type.typeId.index]?
        | throwError "namespace invariant local type {localDecl.type.typeId.index} is out of range"
      pure ty
    let emptyLocals := Array.replicate declaration.locals.size none
    -- A generic axiom's type parameters read as its instance's types, in the
    -- function's frame.
    let typeArguments ← axiomArguments.mapM fun typeId => do
      let some ty := context.typeOf? typeId
        | throwError "a type argument of a generic axiom has an unknown type"
      pure (ty, context.valueRep? typeId)
    let invariantContext := { context with
      namespaceId := invariantNamespaceId
      ns := invariantNs
      locals := emptyLocals
      localTypes
      oldLocals := emptyLocals
      results := #[]
      resultTypes := #[]
      typeArguments := if axiomArguments.isEmpty then context.typeArguments else typeArguments
      typeArgumentTypes := if axiomArguments.isEmpty then context.typeArgumentTypes
        else axiomArguments.map context.ntyOf? }
    let range := conditionRange context.unit declaration.condition
    -- The declarations the invariant reads, against those a write modifies.
    let handleOf (typeIndex : Nat) : Option LeanerIR.StructHandle := do
      let ty ← context.unit.tables.types[typeIndex]?
      let (handle, _, _) ← nominalDeclaration? context.unit ty
      some handle
    let invariantFamilies := (reachFrom context.unit
      [(invariantNamespaceId, declaration.condition.expression)]).getD #[]
    -- An axiom is not re-established at the modified keys: it is assumed
    -- as stated.
    let modifiedKeys := if isAxiom then none else modifiedResources.map fun resources =>
      resources.filterMap fun resource =>
        if (handleOf resource.typeIndex).any invariantFamilies.contains then some resource.key
        else none
    match modifiedKeys with
    | none =>
        let some root := invariantNs.expressions[declaration.condition.expression.index]?
          | throwError "namespace invariant expression is out of range"
        -- An axiom quantifies as any specification does, over its binders'
        -- domains at its instance's types.
        if isAxiom then
          let proposition ← guardedInvariantBody invariantContext
            declaration.condition.expression declaration.condition.expression none
          terms := terms.push (proposition, range)
          continue
        match root.kind with
        | .quantifier .forall #[binder] triggers condition body =>
            unless triggers.isEmpty do
              throwError "namespace invariant triggers are not supported in generated contracts"
            let some pattern := invariantNs.patterns[binder.pattern.index]?
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
            terms := terms.push (proposition, range)
        | _ =>
            let proposition ← guardedInvariantBody invariantContext
              declaration.condition.expression declaration.condition.expression none
            terms := terms.push (proposition, range)
    | some keys =>
        if keys.isEmpty then continue
        let some root := invariantNs.expressions[declaration.condition.expression.index]?
          | throwError "namespace invariant expression is out of range"
        match root.kind with
        | .quantifier .forall #[binder] triggers condition body =>
            unless triggers.isEmpty do
              throwError "namespace invariant triggers are not supported in generated contracts"
            let some pattern := invariantNs.patterns[binder.pattern.index]?
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
              terms := terms.push (proposition, range)
        | _ =>
            let proposition ← guardedInvariantBody invariantContext
              declaration.condition.expression declaration.condition.expression none
            terms := terms.push (proposition, range)
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
    | .abortsWith =>
        let codes := #[condition.expression] ++ condition.auxiliary.map (·.2)
        groups := { groups with abortsWith := groups.abortsWith ++ codes.map (·, range) }
    | .letPre name => groups := { groups with
        lets := groups.lets.push { name, value := condition.expression, post := false } }
    | .letPost name => groups := { groups with
        lets := groups.lets.push { name, value := condition.expression, post := true } }
    | kind =>
        throwError "specification clause {repr kind} is not supported in           generated contracts"
  return groups

/-- A clause context with a contract's `let`s bound, in order: a
pre-state `let` is read at entry, a post-state one, where `post` admits it,
in the context itself. A contract's bindings are the function's last
locals, one per `let`, after the ones its body declares. -/
private def bindLets (context : Context) (lets : Array ContractLet) (post : Bool) :
    MetaM Context := do
  let mut context := context
  let firstBinding := context.localNames.size - lets.size
  for h : position in [:lets.size] do
    let binding := lets[position]
    if binding.post && !post then continue
    let reading := if binding.post then context
      else { context with locals := context.oldLocals, state := context.oldState }
    let index := firstBinding + position
    unless context.localNames[index]? == some binding.name do
      throwError "the contract binding `{binding.name}` has no local"
    let value ← translate reading binding.value
    let value ← match context.localTypes[index]?.map domainOf with
      | some .boolean => mkDecide value
      | _ => pure value
    context := { context with
      locals := context.locals.set! index (some value)
      oldLocals := context.oldLocals.set! index (some value) }
  return context


/-! ## Data invariants

Data invariants belong to values, rather than to the function which happens
to consume or produce them.  The runtime representation deliberately erases
those proofs, so the generated contract restores their verification meaning:
incoming values provide invariant assumptions, while outgoing values and
modified resources owe the invariant again. -/

/-- A type use under the type arguments of the declaration it occurs in:
the use, with the arguments its type parameters stand for. -/
private inductive Scoped where
  | mk (typeId : LeanerIR.TypeId) (arguments : Array Scoped)

/-- The type a use denotes, with the arguments its parameters stand for; none
for a parameter of the function, whose type is not known. -/
private def Scoped.resolve (unit : ValidatedUnit) (arguments : Array Scoped)
    (typeId : LeanerIR.TypeId) : Option (IrTy × Array Scoped) :=
  match unit.tables.types[typeId.index]? with
  | some (.typeParameter index) => do
      let .mk argument outer ← arguments[index]?
      let resolved ← unit.tables.types[argument.index]?
      if resolved matches .typeParameter _ then none else some (resolved, outer)
  | some ty => some (ty, arguments)
  | none => none

/-- The arguments a nominal type use gives its declaration's parameters. -/
private def Scoped.ofNominal (arguments : Array Scoped) (uses : Array LeanerIR.GenericArgument) :
    Array Scoped :=
  uses.filterMap fun
    | .typeArg argument => some (.mk argument.typeId arguments)
    | _ => none

/-- The rows of fields a nominal declaration holds: its fields, or each
variant's payload with the variant's name. -/
private def fieldRows (unit : ValidatedUnit) (declaration : LeanerIR.StructDecl) :
    Array (Option String × Array LeanerIR.FieldDecl) :=
  if declaration.variants.isEmpty then #[(none, declaration.fields)]
  else declaration.variants.map fun variant =>
    (some (((unit.tables.names[variant.name.index]?).map (·.name)).getD ""), variant.fields)

/-- The type of the values an intrinsic map in an entries layout holds, as a
use under the owner's arguments. -/
private def mapValueType? (unit : ValidatedUnit) (declaration : LeanerIR.StructDecl)
    (arguments : Array Scoped) : Option (IrTy × Array Scoped) := do
  let [(_, fields)] := (fieldRows unit declaration).toList | none
  let [field] := fields.toList | none
  let (.vector element _, scope) ← Scoped.resolve unit arguments field.type.typeId | none
  let (entryType, scope) ← Scoped.resolve unit scope element
  let .nominal _ uses := entryType | none
  let (_, _, entry) ← nominalDeclaration? unit entryType
  let valueField ← entry.fields[1]?
  Scoped.resolve unit (Scoped.ofNominal scope uses) valueField.type.typeId

/-- Whether values of a type carry a data invariant at any depth: one it
declares, the representation invariant of an intrinsic map, or one of a
field's, an element's, or a stored value's, as the Move Prover assumes them
(`data_invariant_instrumentation.rs`). -/
private partial def carriesInvariant (unit : ValidatedUnit) (ty : IrTy)
    (arguments : Array Scoped) (depth : Nat) : Bool :=
  if depth > 16 then false else
  match ty with
  | .vector element _ => match Scoped.resolve unit arguments element with
      | some (elementType, scope) => carriesInvariant unit elementType scope (depth + 1)
      | none => false
  | .tuple elements => elements.any fun element =>
      match Scoped.resolve unit arguments element with
      | some (elementType, scope) => carriesInvariant unit elementType scope (depth + 1)
      | none => false
  | .nominal _ uses => match nominalDeclaration? unit ty with
      | some (_, owner, declaration) =>
          let scope := Scoped.ofNominal arguments uses
          LeanerIR.Proofs.Denote.declarationHasClosureFields owner declaration ||
            declaration.contract.conditions.any (·.kind == .structInvariant) ||
            (mapModel? unit ty).isSome ||
            (fieldRows unit declaration).any fun (_, fields) => fields.any fun field =>
              match Scoped.resolve unit scope field.type.typeId with
              | some (fieldType, fieldScope) => carriesInvariant unit fieldType fieldScope (depth + 1)
              | none => false
      | none => false
  | _ => false

/-- Whether a physical type contributes a data-invariant clause. -/
def hasDataInvariant (unit : ValidatedUnit) (ty : IrTy) : Bool :=
  carriesInvariant unit ty #[] 0

/-- Whether a data invariant a physical type carries, at any depth, states a
behavioral predicate: its clause then reads the executable unit. -/
private partial def carriesMatchingInvariant (unit : ValidatedUnit) (needed : SpecReads → Bool)
    (ty : IrTy) (arguments : Array Scoped) (depth : Nat) : Bool :=
  if depth > 16 then false else
  let element (typeId : LeanerIR.TypeId) : Bool :=
    match Scoped.resolve unit arguments typeId with
    | some (elementType, scope) => carriesMatchingInvariant unit needed elementType scope (depth + 1)
    | none => false
  match ty with
  | .vector elementId _ => element elementId
  | .tuple elements => elements.any element
  | .nominal _ uses => match nominalDeclaration? unit ty with
      | some (handle, _, declaration) =>
          let reads := invariantReads unit handle.namespaceId declaration
          let scope := Scoped.ofNominal arguments uses
          needed reads ||
            (fieldRows unit declaration).any fun (_, fields) => fields.any fun field =>
              match Scoped.resolve unit scope field.type.typeId with
              | some (fieldType, fieldScope) =>
                  carriesMatchingInvariant unit needed fieldType fieldScope (depth + 1)
              | none => false
      | none => false
  | _ => false

/-- Whether a data invariant of a parameter or result of a function, at any
depth, states a behavioral predicate: the contract's clause for it then
reads the executable unit. -/
def signatureInvariantsReadUnit (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  let carries (typeId : LeanerIR.TypeId) : Bool :=
    match ns.tables.types[typeId.index]? with
    | some (.reference reference) =>
        (ns.tables.types[reference.referent.index]?).any
          (carriesMatchingInvariant unit (fun reads => reads.unit || reads.requires) · #[] 0)
    | some ty => carriesMatchingInvariant unit (fun reads => reads.unit || reads.requires) ty #[] 0
    | none => false
  declaration.signature.parameters.any (carries ·.typeUse.typeId) ||
    declaration.signature.results.any (carries ·.typeId)

/-- Specialize every invariant declared by `ty` to its erased runtime value.
Struct invariant locals are the fields in declaration order; enum invariants
receive the whole tagged value as local zero and bind payloads in their own
logical match. -/
private def declaredInvariantTerms (context : Context) (ty : IrTy)
    (value : Lean.Expr) : MetaM (Array (Lean.Expr × ObligationRange)) := do
  let some (owner, ns, declaration) := nominalDeclaration? context.unit ty
    | return #[]
  let namespaceId := owner.namespaceId
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
    -- The whole value, after the fields.
    if (declaration.locals[declaration.fields.size]?).any (·.name == "this") then
      locals := locals.set! declaration.fields.size (some value)
  else
    unless !declaration.locals.isEmpty do
      throwError "an enum invariant has no `this` declaration local"
    locals := locals.set! 0 (some value)
  let invariantContext : Context := {
    context with
    namespaceId, ns, locals, localTypes, oldLocals := locals
    results := #[], resultTypes := #[] }
  -- An invariant reading memory, through a behavioral predicate, holds in
  -- every memory: what a value satisfies does not change with the memory
  -- around it.
  let readsMemory := (invariantReads context.unit namespaceId declaration).state
  conditions.mapM fun condition => do
    unless condition.auxiliary.isEmpty do
      throwError "a data invariant cannot carry auxiliary expressions"
    let proposition ← if readsMemory then
        withLocalDeclD `memory (mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory)
            (← invariantContext.unitExpr)) fun memory => do
          mkForallFVars #[memory] (← translate
            { invariantContext with state := some memory, oldState := some memory }
            condition.expression) (usedOnly := true)
      else translate invariantContext condition.expression
    let range := conditionRange context.unit condition
    pure (proposition, range)


/-- A scoped type argument in the native row used by a closure's frame. -/
private partial def Scoped.toNative? (unit : ValidatedUnit) (owner : LeanerIR.NamespaceId)
    (argument : Scoped) : Option LeanerIR.Proofs.Denote.NTy := do
  let .mk id arguments := argument
  let ty ← LeanerIR.Proofs.Denote.ntyOf unit owner id
  if arguments.isEmpty then return ty
  let resolved ← arguments.toList.mapM (Scoped.toNative? unit owner)
  return ty.subst (LeanerIR.Proofs.Denote.NRow.ofList resolved)

/-- A frame binder may retain a parameter of the enclosing function. Unlike
invariant discovery, binding that value does not require a concrete type. -/
private partial def Scoped.bindingType? (unit : ValidatedUnit) : Scoped → Option IrTy
  | .mk id arguments => do
    let ty ← unit.tables.types[id.index]?
    if let .typeParameter index := ty then
      if let some argument := arguments[index]? then return ← argument.bindingType? unit
    return ty

/-- The memory slot a `modifies global<T>(k)` clause names: its resource
type at the contract's frame and its storage key, over the contract's
logical binders. -/
private def modifiedSlotTerm (context : Context) (id : ExprId) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "a modifies clause is out of range"
  match expression.kind with
  | .operation (.specification (.global _)) instantiations arguments _ =>
      let resource ← match instantiations.toList with
        | [.typeArg resource] => pure resource.typeId
        | _ => throwError "a modifies clause needs one resource type"
      let some key := arguments[0]?
        | throwError "a modifies clause expects one key"
      let some keyTy := typeOfExpr? context key
        | throwError "a modifies key has an unknown type"
      let encoded ← (domainOf keyTy).encode (← translate context key)
      let typeArguments ← if context.typeArgumentTypes.isEmpty then pure none else do
        let types ← context.typeArgumentTypes.toList.mapM fun type => do
          let some type := type | throwError "a frame's type argument has no native type"
          pure type
        pure (some (LeanerIR.Proofs.Denote.NRow.ofList types))
      let (resourceExpr, _) ← quoteResource (← context.frame) context.unit context.namespaceId
        resource typeArguments
      return (resourceExpr, ← mkAppM ``LeanerIR.RuntimeValue.storageKey #[encoded])
  | _ =>
      throwError "a modifies clause must name a resource at a key in \
        generated contracts"

/-- The frame of a change of global memory from `initial` to `final` within
the slots `slots` name: every listed resource type reads the same at every
key other than its listed ones, and, unless the frame is loose, every other
resource type reads the same everywhere. -/
private def slotFrame (initial final : Lean.Expr) (slots : Array (Lean.Expr × Lean.Expr))
    (loose : Bool) : MetaM Lean.Expr := do
  let mut resources : Array Lean.Expr := #[]
  for (resource, _) in slots do
    unless resources.contains resource do resources := resources.push resource
  let mut frames := #[]
  for resource in resources do
    let frame ← withLocalDeclD `key (mkConst ``LeanerIR.StorageKey) fun key => do
      let mut implication ← mkEq (mkApp2 final resource key) (mkApp2 initial resource key)
      for (written, writtenKey) in slots.reverse do
        if written == resource then
          implication ← mkArrow (← mkAppM ``Ne #[key, writtenKey]) implication
      mkForallFVars #[key] implication
    frames := frames.push frame
  unless loose do
    let frame ← withLocalDeclD `resource (mkConst ``LeanerIR.Proofs.Denote.ResourceType)
      fun other => do
        let mut implication ← mkEq (mkApp final other) (mkApp initial other)
        for resource in resources.reverse do
          implication ← mkArrow (← mkAppM ``Ne #[other, resource]) implication
        mkForallFVars #[other] implication
    frames := frames.push frame
  conjunction frames

/-- The element types of a quoted native row. -/
private partial def rowElementTypes (row : Lean.Expr) : Array Lean.Expr :=
  if row.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 then
    #[row.getArg! 0] ++ rowElementTypes (row.getArg! 1)
  else #[]

/-- The components of a native row value, as projections. -/
private def rowProjections (skolems row : Lean.Expr) (types : Array Lean.Expr) :
    MetaM (Array Lean.Expr) := do
    let carriers ← frameCarriers skolems
    let mut rest := row
    let mut values := #[]
    for index in [:types.size] do
      let tail := (types.extract (index + 1) types.size).foldr
        (fun ty row => mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NRow.cons) ty row)
        (mkConst ``LeanerIR.Proofs.Denote.NRow.nil)
      let carrier := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.carrier) carriers types[index]!
      let tailType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.HList) carriers tail
      values := values.push (mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) carrier tailType rest)
      rest := mkApp3 (mkConst ``Prod.snd [Level.zero, Level.zero]) carrier tailType rest
    return values

/-- The clause binder of a native value at a physical type: an integer's
value, a boolean or text as itself, and an aggregate's encoding. -/
private def nativeBinder (skolems : Lean.Expr) (physical : IrTy) (ty value : Lean.Expr) :
    MetaM Lean.Expr :=
  match domainOf physical with
  | .integer => mkAppM ``LeanerIR.SpecInt.val #[value]
  | .boolean | .text _ => pure value
  | .aggregate => do
      return mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) (← frameCarriers skolems) ty value

private def fieldFrameTerm (context : Context) (owner : LeanerIR.NamespaceId)
    (declaration : LeanerIR.StructDecl) (index : Nat) (field : LeanerIR.FieldDecl)
    (scope : Array Scoped) (whole value : Lean.Expr) : MetaM Lean.Expr := do
  let some nativeType := Scoped.toNative? context.unit owner (.mk field.type.typeId scope)
    | throwError "a function-valued field has no native argument row"
  let .function parameters _ _ := nativeType
    | throwError "a function-valued field's native type is not a function"
  let some executable := context.executable
    | throwError "a function-valued field's frame needs the executable unit"
  let skolems ← context.frame
  let unitExpr ← context.unitExpr
  let row ← quoteRow parameters
  let some frame := declaration.contract.parameterFrames.find? (·.parameter.index == index)
    | mkAppM' (mkApp3 (mkConst ``LeanerIR.Proofs.EncodedKeepsMemory)
        unitExpr executable skolems) #[row, value]
  if frame.modifiesAll then return mkConst ``True
  let some ns := context.unit.namespaces[owner.index]?
    | throwError "a field frame's namespace is out of range"
  let localTypes ← declaration.locals.mapM fun localDecl => do
    let some ty := (Scoped.mk localDecl.type.typeId scope).bindingType? context.unit
      | throwError "a field frame's local has no type"
    pure ty
  let mut locals := Array.replicate declaration.locals.size none
  for (_, fieldIndex) in declaration.fields.zipIdx do
    let selected ← mkAppM ``LeanerIR.RuntimeValue.field #[whole, toExpr fieldIndex]
    locals := locals.set! fieldIndex
      (some (← (domainOf localTypes[fieldIndex]!).binderOfRuntime selected))
  if (declaration.locals[declaration.fields.size]?).any (·.name == "this") then
    locals := locals.set! declaration.fields.size (some whole)
  let typeArgumentTypes := scope.map (Scoped.toNative? context.unit owner)
  let typeArguments ← scope.mapM fun argument => do
    let some ty := argument.bindingType? context.unit
      | throwError "a field frame's type argument is out of range"
    pure (ty, none)
  let invocationType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.HList)
    (← frameCarriers skolems) row
  let memoryType := mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) unitExpr
  let frameTerm ← withLocalDeclD `invocation invocationType fun invocation =>
    withLocalDeclD `pre memoryType fun pre =>
    withLocalDeclD `post memoryType fun post => do
      let componentTypes := rowElementTypes row
      let components ← rowProjections skolems invocation componentTypes
      let mut active : Context := { context with
        namespaceId := owner, ns, locals, localTypes, oldLocals := locals
        results := #[], resultTypes := #[], state := some pre, oldState := some pre
        typeArguments, typeArgumentTypes }
      for (formal, ty, component) in frame.formals.zip (componentTypes.zip components) do
        let some physical := localTypes[formal.index]?
          | throwError "a field frame's formal has no local type"
        let binder ← nativeBinder skolems physical ty component
        active := { active with
          locals := active.locals.set! formal.index (some binder)
          oldLocals := active.oldLocals.set! formal.index (some binder) }
      let slots ← frame.modifies.mapM (modifiedSlotTerm active)
      mkLambdaFVars #[invocation, pre, post] (← slotFrame pre post slots false)
  mkAppM' (mkApp3 (mkConst ``LeanerIR.Proofs.EncodedFramed)
    unitExpr executable skolems) #[row, frameTerm, value]

/-- The data invariants of a value of a type, at any depth, as the Move
Prover assumes them: the type's declared invariants; for an intrinsic map,
its representation invariant and the invariants of its values; the
invariants of each field, under the test of its variant; and those of each
element of a vector. -/
private partial def deepInvariantTerms (context : Context) (ty : IrTy) (arguments : Array Scoped)
    (value : Lean.Expr) (depth : Nat) : MetaM (Array (Lean.Expr × ObligationRange)) := do
  unless carriesInvariant context.unit ty arguments depth do return #[]
  match ty with
  | .vector element _ =>
      let some (elementType, scope) := Scoped.resolve context.unit arguments element | return #[]
      withLocalDeclD `element (mkConst ``RuntimeValue) fun elementValue => do
        let terms ← deepInvariantTerms context elementType scope elementValue (depth + 1)
        let some (_, range) := terms[0]? | return #[]
        let member ← mkAppM ``Membership.mem
          #[← mkAppM ``LeanerLang.Contract.elementsVector #[value], elementValue]
        let body ← mkArrow member (← conjunction (terms.map (·.1)))
        return #[(← mkForallFVars #[elementValue] body, range)]
  | .tuple elements =>
      -- The invariants of each component of a multi-value result.
      let mut terms := #[]
      for (element, index) in elements.zipIdx do
        let some (elementType, scope) := Scoped.resolve context.unit arguments element | continue
        let selected ← mkAppM ``LeanerIR.RuntimeValue.field #[value, toExpr index]
        terms := terms ++ (← deepInvariantTerms context elementType scope selected (depth + 1))
      return terms
  | .nominal _ uses =>
      let some (owner, ns, declaration) := nominalDeclaration? context.unit ty | return #[]
      if !declaration.variants.isEmpty && !declaration.contract.parameterFrames.isEmpty then
        throwError "modifies_of on enum fields is not carried yet"
      let scope := Scoped.ofNominal arguments uses
      let mut terms ← declaredInvariantTerms context ty value
      if let some model := mapModel? context.unit ty then
        let some intrinsic := ns.intrinsics.find? (·.owner == declaration.name) | return terms
        let range := locRange context.unit intrinsic.loc
        terms := terms.push (← mkAppM ``LeanerIR.Maps.Valid #[model.discipline, value], range)
        if let some (valueType, valueScope) := mapValueType? context.unit declaration scope then
          let stored ← withLocalDeclD `key (mkConst ``RuntimeValue) fun key => do
            let held ← mkAppM ``LeanerIR.Maps.valueAt #[value, key]
            let inner ← deepInvariantTerms context valueType valueScope held (depth + 1)
            let some (_, range) := inner[0]? | return none
            let present ← mkEq (← mkAppM ``LeanerIR.Maps.hasKey #[value, key]) (mkConst ``Bool.true)
            let body ← mkArrow present (← conjunction (inner.map (·.1)))
            return some (← mkForallFVars #[key] body, range)
          if let some term := stored then terms := terms.push term
        return terms
      for (variant, fields) in fieldRows context.unit declaration do
        for (field, index) in fields.zipIdx do
          let some (fieldType, fieldScope) := Scoped.resolve context.unit scope field.type.typeId
            | continue
          let selected ← mkAppM ``LeanerIR.RuntimeValue.field #[value, toExpr index]
          let mut inner ← deepInvariantTerms context fieldType fieldScope selected (depth + 1)
          if LeanerIR.Proofs.Denote.fieldKeepsMemory ns field then
            inner := inner.push (← fieldFrameTerm context owner.namespaceId declaration index
              field scope value selected,
              locRange context.unit field.loc)
          for (term, range) in inner do
            match variant with
            | none => terms := terms.push (term, range)
            | some name =>
                let test ← mkEq (← mkAppM ``LeanerLang.Contract.testVariants
                  #[value, toExpr owner, toExpr #[name]]) (mkConst ``Bool.true)
                terms := terms.push (← mkArrow test term, range)
      return terms
  | _ => return #[]

/-- The data invariants of a value of a physical type, at any depth. -/
private def dataInvariantTerms (context : Context) (ty : IrTy) (value : Lean.Expr) :
    MetaM (Array (Lean.Expr × ObligationRange)) :=
  deepInvariantTerms context ty #[] value 0

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
    MetaM (Array (Lean.Expr × ObligationRange)) := do
  let mut terms := #[]
  for (slot, binders) in slots.zip bound do
    unless hasDataInvariant context.unit slot.physical do continue
    let some binder := binderOf binders | continue
    let value ← slotValueRuntime context slot binder
    terms := terms ++ (← dataInvariantTerms context slot.physical value)
  pure terms

/-- The name a clause gives the opaque specification function `name` of the
namespace at `path` (`opaqueSpec`), when the unit declares it. -/
private def declaredOpaqueSpec? (unit : ValidatedUnit) (path : List String) (name : String) :
    Option String := do
  let index ← unit.tables.namespaces.toList.findIdx? (·.segments.toList == path)
  let ns ← unit.namespaces[index]?
  guard <| ns.specFunctions.any fun function =>
    function.body.isNone && (unit.tables.names[function.name.index]?.map (·.name)) == some name
  some ("::".intercalate (path ++ [name]))

/-- Whether a physical type is a byte vector, `Vector<u8>`. -/
private def isByteVector (unit : ValidatedUnit) (ty : IrTy) : Bool :=
  if let .vector element _ := ty then
    unit.tables.types[element.index]? == some (.integer (.bits 8) false)
  else false

/-- Whether the unit declares the native at a qualified name. -/
private def declaresNative (unit : ValidatedUnit) (qualified : String) : Bool := Id.run do
  let segments := qualified.splitOn "::"
  let some name := segments.getLast? | return false
  let path := segments.dropLast
  let some index := unit.tables.namespaces.toList.findIdx? (·.segments.toList == path)
    | return false
  let some ns := unit.namespaces[index]? | return false
  return ns.functions.any fun function =>
    function.body == .absent && (unit.tables.names[function.name.index]?.map (·.name)) == some name

/-- What a Move function assumes of the hashes it applies at any depth: no
two of its byte-vector parameters collide (`moveCollisionFreeHashes`), stated
over their encodings as a hash's value at them reads in a clause. -/
private def hashFacts (context : Context)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (slots : Array Slot) (bound : Array SlotBinders) : MetaM (Array Lean.Expr) := do
  unless context.ns.profile == some .move do return #[]
  let hashes := reachedFunctions context.unit context.namespaceId declaration
    (moveCollisionFreeHashes.filter (declaresNative context.unit))
  if hashes.isEmpty then return #[]
  let mut values := #[]
  for (slot, binders) in slots.zip bound do
    if isByteVector context.unit slot.physical then
      values := values.push (← slotValueRuntime context slot binders.entry)
  let mut facts := #[]
  for hash in hashes do
    let application := fun (value : Lean.Expr) => do
      mkAppOptM ``LeanerLang.Contract.opaqueSpec
        #[toExpr hash, ← mkListLit (mkConst ``LeanerLang.Contract.SpecTypeArgument) [],
          mkConst ``RuntimeValue, none, ← mkListLit (mkConst ``RuntimeValue) [value]]
    for i in [0:values.size] do
      for j in [i + 1:values.size] do
        let collide ← mkEq (← application values[i]!) (← application values[j]!)
        facts := facts.push (← mkArrow collide (← mkEq values[i]! values[j]!))
  return facts

/-- What the Move Prover assumes of every signer value: that it signs the
transaction, as `std::signer` states it (`is_txn_signer` of the signer,
`is_txn_signer_addr` of its address), where the unit declares those
predicates. A Move signer comes from a parameter or a native, so stating it
of the signers a function takes and returns carries it everywhere. -/
private def signerFacts (context : Context) (slots : Array Slot) (bound : Array SlotBinders)
    (binderOf : SlotBinders → Option Lean.Expr) : MetaM (Array Lean.Expr) := do
  unless context.ns.profile == some .move do return #[]
  let predicates := [("is_txn_signer", ``RuntimeValue.signer),
      ("is_txn_signer_addr", ``RuntimeValue.address)].filterMap fun (name, encoder) =>
    (declaredOpaqueSpec? context.unit ["0x1", "signer"] name).map (·, encoder)
  if predicates.isEmpty then return #[]
  let mut facts := #[]
  for (slot, binders) in slots.zip bound do
    unless slot.physical matches .signer do continue
    let some value := binderOf binders | continue
    for (name, encoder) in predicates do
      let call ← mkAppOptM ``LeanerLang.Contract.opaqueSpec
        #[toExpr name, ← mkListLit (mkConst ``LeanerLang.Contract.SpecTypeArgument) [],
          mkConst ``Bool, none,
          ← mkListLit (mkConst ``RuntimeValue) [← mkAppM encoder #[value]]]
      facts := facts.push (← mkEq call (mkConst ``Bool.true))
  return facts

/-- Whether a namespace invariant reads memory a function reaches
(`memoryReach`): those it assumes at entry and owes where it writes, as the
Move Prover evaluates the invariants of the memory a function uses. An axiom
reading no memory holds in every state and is assumed everywhere, as the Move
Prover states its axioms globally. -/
private def invariantReached (reach : Option (Array LeanerIR.StructHandle))
    (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (invariant : LeanerIR.NamespaceInvariant) : Bool :=
  let read := reachFrom unit [(namespaceId, invariant.condition.expression)]
  if invariant.condition.kind matches .axiom_ _ && read.all (·.isEmpty) then true else
  match reach, read with
  | some reached, some read => read.any reached.contains
  | _, _ => false

/-- Whether a namespace invariant is `[suspendable]`. -/
private def isSuspendable (invariant : LeanerIR.NamespaceInvariant) : Bool :=
  invariant.condition.properties.any fun
    | .assign "suspendable" (.constant (.bool true)) _ => true
    | .call "suspendable" arguments _ => arguments.isEmpty
    | _ => false

/-- The functions a function's body calls or makes closures of, as
namespace and function indices. -/
private def bodyCallees (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array (Nat × Nat) := Id.run do
  let .structured root := declaration.body | return #[]
  let some ns := unit.namespaces[namespaceId.index]? | return #[]
  let mut work : List ExprId := [root]
  let mut callees : Array (Nat × Nat) := #[]
  for _ in [0:ns.expressions.size + 1] do
    match work with
    | [] => break
    | id :: rest =>
        work := rest
        let some expression := ns.expressions[id.index]? | continue
        if let .operation operation _ _ _ := expression.kind then
          let reference? := match operation with
            | .call (.function reference) | .call (.closure reference _) => some reference
            | _ => none
          if let some reference := reference? then
            if let some functionId := unit.resolution.function? reference.name then
              let callee := (reference.namespaceId.index, functionId.index)
              unless callees.contains callee do callees := callees.push callee
        work := (LeanerIR.Validation.expressionChildren expression.kind).toList ++ work
  return callees

/-- Whether a function's callers carry the unit's `[suspendable]` invariants
for it, as the Move Prover delegates them: it declares
`delegate_invariants_to_caller`, or a function that declares it or
`disable_invariants_in_body` calls it, at any depth. A function with
callers outside the unit, public or entry, carries them itself. -/
def delegatesInvariants (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool := Id.run do
  unless unit.namespaces.any (·.invariants.any isSuspendable) do return false
  let .ok modifiers := declaredModifiers declaration | return false
  if modifiers.visibility == .public_ || modifiers.isEntry then return false
  let some ns := unit.namespaces[namespaceId.index]? | return false
  let some index := ns.functions.findIdx? (·.name == declaration.name) | return false
  let mut delegated : Std.HashSet (Nat × Nat) := {}
  let mut work : List (Nat × Nat) := []
  let mut count := 0
  for (ns, nsIndex) in unit.namespaces.toList.zipIdx do
    for (function, functionIndex) in ns.functions.toList.zipIdx do
      count := count + 1
      let delegating := pragmaEnabled function.contract "delegate_invariants_to_caller"
      if delegating then delegated := delegated.insert (nsIndex, functionIndex)
      if delegating || pragmaEnabled function.contract "disable_invariants_in_body" then
        work := (nsIndex, functionIndex) :: work
  -- Every function enters the work list at most twice.
  for _ in [0:2 * count + 1] do
    match work with
    | [] => break
    | (nsIndex, functionIndex) :: rest =>
        work := rest
        let some function := unit.namespaces[nsIndex]?.bind (·.functions[functionIndex]?)
          | continue
        for callee in bodyCallees unit ⟨nsIndex⟩ function do
          unless delegated.contains callee do
            delegated := delegated.insert callee
            work := callee :: work
  return delegated.contains (namespaceId.index, index)

/-- The namespace invariants a function assumes at entry and owes at exit:
those of the memory it reaches, but for the `[suspendable]` ones its
callers carry (`delegatesInvariants`). -/
private def invariantApplies (reach : Option (Array LeanerIR.StructHandle))
    (unit : ValidatedUnit) (delegated : Bool) (namespaceId : LeanerIR.NamespaceId)
    (invariant : LeanerIR.NamespaceInvariant) : Bool :=
  invariantReached reach unit namespaceId invariant && !(delegated && isSuspendable invariant)

/-- The namespace invariants a write of a resource owes in a function, as
the Move Prover checks them after the write: those the function carries
(`invariantApplies`) that read the resource written, update invariants
comparing with `oldState`, the memory before the write. `True` where the
function's body defers its invariants to its exit
(`disable_invariants_in_body`). -/
def memoryWriteInvariants (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (codecs types : Option Lean.Expr) (twins : Array SpecTypes.TwinInfo)
    (written : LeanerIR.StructHandle) (state oldState : Lean.Expr)
    (executable : Option Lean.Expr) : MetaM Lean.Expr := do
  if pragmaEnabled declaration.contract "disable_invariants_in_body" then
    return mkConst ``True
  let reach := memoryReach unit namespaceId declaration
  let delegated := delegatesInvariants unit namespaceId declaration
  let context : Context := {
    unit, namespaceId, ns, locals := #[], oldLocals := #[], localTypes := #[],
    localNames := #[], results := #[], codecs, types, state := some state,
    oldState := some oldState, twins, executable }
  let reads (invariantNamespace : LeanerIR.NamespaceId) (invariant : LeanerIR.NamespaceInvariant) :=
    (reachFrom unit [(invariantNamespace, invariant.condition.expression)]).any
      (·.contains written)
  let terms ← namespaceInvariantTerms context none .exit fun invariantNamespace invariant =>
    invariantApplies reach unit delegated invariantNamespace invariant &&
      reads invariantNamespace invariant
  conjunction (terms.map fun (clause, range) => markObligation range clause)

/-- The callees a function calls that leave their `[suspendable]`
invariants to their callers (`delegatesInvariants`), each with the resource
declarations it writes, at any depth: the function checks those invariants
after each call, as the Move Prover does, where it carries them itself and
its body does not defer them. -/
def delegatingCallees (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array (LeanerIR.FunctionHandle × Array LeanerIR.StructHandle) := Id.run do
  if pragmaEnabled declaration.contract "disable_invariants_in_body" ||
      delegatesInvariants unit namespaceId declaration then return #[]
  let mut found := #[]
  for (calleeNs, calleeIndex) in bodyCallees unit namespaceId declaration do
    let some callee := unit.namespaces[calleeNs]?.bind (·.functions[calleeIndex]?) | continue
    unless delegatesInvariants unit ⟨calleeNs⟩ callee do continue
    let some written := reachFrom unit
        ((functionRoots callee).toList.map (⟨calleeNs⟩, ·)) (writes := true) | continue
    found := found.push ({ namespaceId := ⟨calleeNs⟩, functionId := ⟨calleeIndex⟩ }, written)
  return found

/-- The `[suspendable]` invariants a function carries that read memory a
callee of `delegatingCallees` writes: owed after the call over the memory
after it, update invariants comparing with `before`. -/
def callWriteInvariants (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (codecs types : Option Lean.Expr) (twins : Array SpecTypes.TwinInfo)
    (written : Array LeanerIR.StructHandle) (before after : Lean.Expr)
    (executable : Option Lean.Expr) : MetaM Lean.Expr := do
  let reach := memoryReach unit namespaceId declaration
  let context : Context := {
    unit, namespaceId, ns, locals := #[], oldLocals := #[], localTypes := #[],
    localNames := #[], results := #[], codecs, types, state := some after,
    oldState := some before, twins, executable }
  let reads (invariantNamespace : LeanerIR.NamespaceId) (invariant : LeanerIR.NamespaceInvariant) :=
    (reachFrom unit [(invariantNamespace, invariant.condition.expression)]).any
      fun read => read.any written.contains
  let terms ← namespaceInvariantTerms context none .exit fun invariantNamespace invariant =>
    isSuspendable invariant && invariantApplies reach unit false invariantNamespace invariant &&
      reads invariantNamespace invariant
  conjunction (terms.map fun (clause, range) => markObligation range clause)

/-- Whether a function owes invariants of memory: it writes, at any depth,
memory a namespace invariant of the unit reads, or a resource that carries a
data invariant. Such a function is verified for them even without a
specification of its own. -/
def owesMemoryInvariants (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  match reachFrom unit
      ((functionRoots declaration).toList.map (namespaceId, ·)) (writes := true) with
  | none => false
  | some written =>
      let delegated := delegatesInvariants unit namespaceId declaration
      let invariantRead := (unit.namespaces.toList.zipIdx).any fun (ns, index) =>
        ns.invariants.any fun invariant =>
          invariantApplies (some written) unit delegated ⟨index⟩ invariant
      invariantRead || written.any fun handle =>
        (unit.namespaces[handle.namespaceId.index]?.bind (·.structs[handle.structId]?)).any
          fun resource => resource.abilities.contains .key &&
            hasDataInvariant unit (.nominal resource.name #[])

/-- A resource declaration by its module path and name, which identifies it
across units. -/
def qualifiedResource? (unit : ValidatedUnit) (handle : LeanerIR.StructHandle) :
    Option (Array String × String) := do
  let path ← unit.tables.namespaces[handle.namespaceId.index]?
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.structs[handle.structId]?
  let name ← unit.tables.names[declaration.name.index]?
  pure (path.segments, name.name)

/-- The resources, by qualified name, the functions of a namespace reach in
global memory, or with `writes` write. -/
def namespaceFunctionMemory (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (writes : Bool) : Array (Array String × String) := Id.run do
  let some ns := unit.namespaces[namespaceId.index]? | return #[]
  let mut found := #[]
  for declaration in ns.functions do
    let reached := (reachFrom unit ((functionRoots declaration).toList.map (namespaceId, ·))
      (writes := writes)).getD #[]
    for handle in reached do
      if let some resource := qualifiedResource? unit handle then
        unless found.contains resource do found := found.push resource
  return found

/-- The resources, by qualified name, the invariants of a namespace read. -/
def namespaceInvariantMemory (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId) :
    Array (Array String × String) := Id.run do
  let some ns := unit.namespaces[namespaceId.index]? | return #[]
  let mut found := #[]
  for invariant in ns.invariants do
    for handle in (reachFrom unit [(namespaceId, invariant.condition.expression)]).getD #[] do
      if let some resource := qualifiedResource? unit handle then
        unless found.contains resource do found := found.push resource
  return found

/-- Whether a function owes an invariant of the namespace `invariantNs`: it
writes, at any depth, memory such an invariant reads, and carries it. -/
def owesInvariantsOf (unit : ValidatedUnit) (invariantNs namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  match reachFrom unit
      ((functionRoots declaration).toList.map (namespaceId, ·)) (writes := true) with
  | none => false
  | some written =>
      let delegated := delegatesInvariants unit namespaceId declaration
      (unit.namespaces[invariantNs.index]?).any fun ns =>
        ns.invariants.any (invariantApplies (some written) unit delegated invariantNs ·)

/-- Whether a function can reach global memory (`memoryReach`). -/
def reachesMemory (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  (memoryReach unit namespaceId declaration).isSome

/-- The data invariants of a unit's stored resources: their predicate
(`storedInvariantTerm`), the declarations it states an invariant of, each
with its invariant over a value's encoding (after the executable unit,
where they read it), and those whose invariants it does not carry, each
with the reason, which a function reaching them reports. -/
structure StoredInvariants where
  predicate : Option Lean.Expr := none
  carriers : Array LeanerIR.StructHandle := #[]
  cases : Array Lean.Expr := #[]
  unsupported : Array (LeanerIR.StructHandle × String) := #[]
  collectionCases : Array (LeanerIR.StructHandle × Lean.Expr) := #[]
  collectionUnsupported : Array (LeanerIR.StructHandle × String) := #[]
  hasCollections : Bool := false

/-- A declaration's own invariant, parameterized by its resolved arguments.
Generic declarations are read in every compatible frame. This preserves type
identity (including phantom arguments) without choosing a default generic
instantiation or requiring an inhabitant of an arbitrary native type. -/
private def storedDeclarationTerm (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (namespaceId : LeanerIR.NamespaceId) (ns : ValidatedNamespace)
    (declaration : LeanerIR.StructDecl) : MetaM Lean.Expr := do
  let readsUnit := storedInvariantReadsUnit unit
  withLocalDeclD `unit (mkConst ``ValidatedUnit) fun memoryUnit =>
  withLocalDeclD `executable (executableType memoryUnit) fun executable =>
  withLocalDeclD `arguments (mkConst ``LeanerIR.Proofs.Denote.NRow) fun arguments =>
  withLocalDeclD `value (mkConst ``RuntimeValue) fun value => do
    let leading := if readsUnit then #[memoryUnit, executable] else #[memoryUnit]
    let atFrame (frame : Lean.Expr) : MetaM Lean.Expr := do
      let carriers ← frameCarriers frame
      let context : Context := {
        unit, namespaceId, ns, twins, locals := #[], results := #[]
        physicalInvariantLengths := true
        carrier := some (mkApp (mkConst ``LeanerIR.Proofs.Denote.Carriers.carrier) carriers)
        codecs := some (mkApp (mkConst ``LeanerIR.Proofs.Denote.Carriers.codec) carriers)
        types := some (← frameTypes frame)
        executable := if readsUnit then some executable else none }
      let ty : IrTy := .nominal declaration.name #[]
      let mut terms ← declaredInvariantTerms context ty value
      if let some model := mapModel? unit ty then
        terms := terms.push (← mkAppM ``LeanerIR.Maps.Valid #[model.discipline, value],
          locRange unit declaration.loc)
      unless declaration.contract.parameterFrames.isEmpty || declaration.variants.isEmpty do
        throwError "modifies_of on enum fields is not carried yet"
      for (field, index) in declaration.fields.zipIdx do
        if LeanerIR.Proofs.Denote.fieldKeepsMemory ns field then
          let selected ← mkAppM ``RuntimeValue.field #[value, toExpr index]
          terms := terms.push (← fieldFrameTerm context namespaceId declaration index field #[]
            value selected, locRange unit field.loc)
      conjunction (terms.map fun (term, range) => markObligation range term)
    let body ← if declaration.generics.isEmpty then
        atFrame (mkApp (mkConst ``LeanerIR.Proofs.Denote.Skolems.runtime) memoryUnit)
      else
        withLocalDeclD `frame (mkApp (mkConst ``LeanerIR.Proofs.Denote.Skolems) memoryUnit)
            fun frame => do
          let body ← atFrame frame
          unless body.containsFVar frame.fvarId! do return body
          let types ← frameTypes frame
          -- Most generic invariants use only the parameter identities (for
          -- example an opaque specification call at a phantom argument).
          -- Read those identities directly from the stored native row instead
          -- of asking proof search to invent a compatible Skolems witness.
          let storedTypes ← withLocalDeclD `index (mkConst ``Nat) fun index => do
            mkLambdaFVars #[index]
              (← mkAppM ``LeanerIR.Proofs.Denote.NRow.getD
                #[arguments, index, mkConst ``LeanerIR.Proofs.Denote.NTy.unit])
          let specialized := body.replace fun expression =>
            if expression == types then some storedTypes else none
          unless specialized.containsFVar frame.fvarId! do return specialized.headBeta
          let argumentsAtFrame ← mkAppM ``LeanerIR.Proofs.Denote.NRow.ofList
            #[← mkListLit (mkConst ``LeanerIR.Proofs.Denote.NTy)
              ((List.range declaration.generics.size).map fun index => mkApp types (toExpr index))]
          mkForallFVars #[frame] (← mkArrow (← mkEq argumentsAtFrame arguments) body)
    mkLambdaFVars (leading ++ #[arguments, value]) body

/-- The data invariants of the unit's stored resources: of each `key`
declaration that has one, at any depth, its invariant over the encoding of
a value of it, its type parameters read at the runtime frame of the unit
memory is typed at. An invariant reading memory holds in every memory, so
none reads memory. Each takes that unit first, then the executable unit
where one states a behavioral predicate (`storedInvariantAt`). -/
def storedInvariantTerm (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo) :
    MetaM StoredInvariants := do
  let readsUnit := storedInvariantReadsUnit unit
  let mut carriers : Array LeanerIR.StructHandle := #[]
  let mut cases : Array Lean.Expr := #[]
  let mut unsupported : Array (LeanerIR.StructHandle × String) := #[]
  let mut collectionCases : Array (LeanerIR.StructHandle × Lean.Expr) := #[]
  let mut collectionUnsupported : Array (LeanerIR.StructHandle × String) := #[]
  let collections := hasTableModel unit
  for h : namespaceIndex in [:unit.namespaces.size] do
    let ns := unit.namespaces[namespaceIndex]
    for h : structIndex in [:ns.structs.size] do
      let declaration := ns.structs[structIndex]
      let owner : LeanerIR.StructHandle := ⟨⟨namespaceIndex⟩, structIndex⟩
      if collections && (declaration.contract.conditions.any (·.kind == .structInvariant) ||
          LeanerIR.Proofs.Denote.declarationHasClosureFields ns declaration ||
          (mapModel? unit (.nominal declaration.name #[])).isSome) then
        try
          collectionCases := collectionCases.push
            (owner, ← storedDeclarationTerm unit twins ⟨namespaceIndex⟩ ns declaration)
        catch error =>
          collectionUnsupported := collectionUnsupported.push (owner, ← error.toMessageData.toString)
      unless declaration.abilities.contains .key do continue
      let ty : IrTy := .nominal declaration.name #[]
      unless hasDataInvariant unit ty do continue
      let case? : Option Lean.Expr ⊕ String ←
          withLocalDeclD `unit (mkConst ``ValidatedUnit) fun memoryUnit =>
          withLocalDeclD `executable (executableType memoryUnit) fun executable =>
          withLocalDeclD `value (mkConst ``RuntimeValue) fun value => do
        let leading := if readsUnit then #[memoryUnit, executable] else #[memoryUnit]
        let runtime := mkApp (mkConst ``LeanerIR.Proofs.Denote.Carriers.runtime) memoryUnit
        let context : Context := {
          unit, namespaceId := ⟨namespaceIndex⟩, ns, twins, locals := #[], results := #[]
          carrier := some (mkApp (mkConst ``LeanerIR.Proofs.Denote.Carriers.carrier) runtime)
          codecs := some (mkApp (mkConst ``LeanerIR.Proofs.Denote.Carriers.codec) runtime)
          types := some (← frameTypes
            (mkApp (mkConst ``LeanerIR.Proofs.Denote.Skolems.runtime) memoryUnit))
          executable := if readsUnit then some executable else none }
        let terms ← try dataInvariantTerms context ty value catch error =>
          return Sum.inr (← error.toMessageData.toString)
        if terms.isEmpty then return Sum.inl none
        let held ← conjunction (terms.map fun (term, range) => markObligation range term)
        return Sum.inl (some (← mkLambdaFVars (leading.push value) held))
      match case? with
      | .inr reason => unsupported := unsupported.push (owner, reason)
      | .inl none => pure ()
      | .inl (some case) =>
          carriers := carriers.push owner
          cases := cases.push case
  let hasCollections := !collectionCases.isEmpty
  return { carriers, cases, unsupported, collectionCases, collectionUnsupported, hasCollections }

/-- The stored invariants' predicate at a memory: over the unit the memory
is typed at, then the executable unit where they read it. -/
def storedInvariantAt (unit : ValidatedUnit) (predicate : Lean.Expr) (executable : Option Lean.Expr)
    (memory : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfR (← inferType memory)
  unless type.isAppOfArity ``LeanerIR.Proofs.Denote.Memory 1 do
    throwError "internal: the stored invariants are applied to {memory}, not a memory"
  let atUnit := mkApp predicate type.appArg!
  unless storedInvariantReadsUnit unit do return atUnit
  let some executable := executable
    | throwError "the stored invariants read the executable unit, which this contract does \
        not take"
  return mkApp atUnit executable

/-- The conjunction of a loop annotation's `invariant` clauses over the
given local binders: the clause translation the contracts use, with no
runtime row, frame, or route-specific representation. Each clause is marked
with its source range, so an invariant not established is reported there.
The data invariants of the `dataInvariantSlots` (locals of a type declaring
them, which the body modifies) are conjoined: a value of the type satisfies
them wherever it is not under construction, so the loop owes them as it
owes an authored clause, and each iteration assumes them. A state anchor in
a clause reads the loop's entry: `entryLocals` and `entryState`. The
clauses of another `kind`, such as in-body assertions, translate alike. -/
def translateLoopInvariants (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (block : LeanerIR.SpecBlock)
    (locals : Array (Option Lean.Expr)) (localTypes : Array IrTy) (codecs : Option Lean.Expr)
    (types : Option Lean.Expr)
    (localNames : Array String) (oldLocals : Array (Option Lean.Expr))
    (oldState : Lean.Expr) (twins : Array SpecTypes.TwinInfo)
    (dataInvariantSlots : Array Nat := #[])
    (entryLocals : Array (Option Lean.Expr) := #[]) (entryState : Option Lean.Expr := none)
    (state : Option Lean.Expr := none) (executable : Option Lean.Expr := none)
    (kind : LeanerIR.ConditionKind := .loopInvariant)
    (labeledAnchors : Array (Nat × Array (Option Lean.Expr) × Lean.Expr) := #[]) :
    MetaM Lean.Expr := do
  let context : Context := {
    unit, namespaceId, ns, locals, oldLocals, localTypes, localNames, results := #[],
    codecs, types, state, oldState := some oldState, twins,
    anchorLocals := entryLocals, anchorState := entryState, labeledAnchors, executable }
  let mut clauses ← block.conditions.filterMapM fun condition => do
    if condition.kind != kind then return none
    -- A split directs the cases on a Boolean; one on an enum is no step.
    if condition.kind == .split then
      let isBoolean := (ns.expressions[condition.expression.index]?).any fun expression =>
        unit.tables.types[expression.typeId.index]? == some .bool
      unless isBoolean do return none
      return some (markObligation (conditionRange unit condition)
        (← mkAppM ``LeanerIR.Proofs.CaseSplit #[← translate context condition.expression]))
    some <$> markObligation (conditionRange unit condition) <$>
      translate context condition.expression
  for slot in dataInvariantSlots do
    let some (some value) := locals[slot]? | throwError "a data invariant slot has no binder"
    let some ty := localTypes[slot]? | throwError "a data invariant slot has no type"
    for (invariant, range) in ← dataInvariantTerms context ty value do
      clauses := clauses.push (markObligation range invariant)
  conjunction clauses

/-- The data invariants of a value of a physical type, at any depth, as one
conjunction of clauses marked with their source ranges: what a construction
owes of the value it makes. The value is its runtime encoding. -/
def valueDataInvariants (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (codecs types : Option Lean.Expr)
    (twins : Array SpecTypes.TwinInfo) (ty : IrTy) (value state : Lean.Expr)
    (executable : Option Lean.Expr) : MetaM Lean.Expr := do
  let context : Context := {
    unit, namespaceId, ns, locals := #[], oldLocals := #[], localTypes := #[],
    localNames := #[], results := #[], codecs, types, state := some state,
    oldState := some state, twins, executable }
  conjunction ((← dataInvariantTerms context ty value).map fun (clause, range) =>
    markObligation range clause)

/-- Whether a quoted native row holds only scalars and type parameters, and
at least one type parameter: a row an unseen invocation is decided at only
where the frame resolves it to scalars (`ScalarAt`). -/
private partial def mentionsInvocableParameter (row : Lean.Expr) : Bool :=
  go row false
where
  go (row : Lean.Expr) (mentioned : Bool) : Bool :=
    if row.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 then
      let τ := row.getArg! 0
      if τ.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.param 1 then go (row.getArg! 1) true
      else if τ.isConstOf ``LeanerIR.Proofs.Denote.NTy.bool ||
          τ.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.int 2 ||
          τ.isConstOf ``LeanerIR.Proofs.Denote.NTy.address ||
          τ.isConstOf ``LeanerIR.Proofs.Denote.NTy.signer ||
          τ.isConstOf ``LeanerIR.Proofs.Denote.NTy.string ||
          τ.isConstOf ``LeanerIR.Proofs.Denote.NTy.bytes then
        go (row.getArg! 1) mentioned
      else false
    else mentioned

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

/-- Whether a quoted native type is a mutable reference. -/
private def isReferenceType (ty : Lean.Expr) : Bool :=
  ty.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.ref 1

/-- The current value and the prophecy of a native reference to `referent`. -/
private def referenceParts (skolems referent value : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  let carrier := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.carrier) (← frameCarriers skolems) referent
  return (mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) carrier carrier value,
    mkApp3 (mkConst ``Prod.snd [Level.zero, Level.zero]) carrier carrier value)

/-- The clause binders of the parameters, read off the native arguments.  A
mutable reference's entry is its current value and its exit its prophecy. -/
private def parameterBinders (skolems : Lean.Expr) (slots : Array Slot) (types : Array Lean.Expr)
    (arguments : Lean.Expr) : MetaM (Array SlotBinders) := do
  let values ← rowProjections skolems arguments types
  unless slots.size == types.size do
    throwError "internal: a parameter row differs from its declaration"
  (slots.zip (types.zip values)).mapM fun (slot, ty, value) => do
    match slot.kind with
    | .mutableRef =>
        let referent := ty.appArg!
        let (current, prophecy) ← referenceParts skolems referent value
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

/-- A function's declared precondition over runtime arguments: its
`requires` clauses, each parameter read from its argument value, as
`requires_of` reads a closure target's (`LeanerIR.Proofs.RequiresTable`). -/
def buildDeclaredRequires (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (twins : Array SpecTypes.TwinInfo := #[]) :
    MetaM Lean.Expr := do
  let groups ← groupConditions unit declaration.contract.conditions
  let localTypes ← declaration.locals.mapM fun localDecl => do
    let some ty := unit.tables.types[localDecl.type.typeId.index]?
      | throwError "function local has an unknown type"
    pure ty
  let values := mkApp (mkConst ``Array [Level.zero]) (mkConst ``LeanerIR.RuntimeValue)
  withLocalDeclD `unit (mkConst ``ValidatedUnit) fun unitExpr =>
  withLocalDeclD `arguments values fun arguments =>
    withLocalDeclD `state (mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) unitExpr) fun state => do
      let mut locals : Array (Option Lean.Expr) := Array.replicate declaration.locals.size none
      for parameter in declaration.signature.parameters, index in [0:declaration.signature.parameters.size] do
        let some ty := unit.tables.types[parameter.typeUse.typeId.index]?
          | throwError "a parameter has an unknown type"
        let value ← mkAppM ``Array.getD #[arguments, toExpr index, mkConst ``LeanerIR.RuntimeValue.unit]
        locals := locals.set! index (some (← (domainOf ty).binderOfRuntime value))
      let context : Context :=
        { unit, namespaceId, ns, twins, locals, localTypes
          localNames := declaration.locals.map (·.name)
          oldLocals := locals, results := #[]
          types := some (← frameTypes (mkApp (mkConst ``LeanerIR.Proofs.Denote.Skolems.runtime)
            unitExpr))
          state := some state, oldState := some state }
      let context ← bindLets context groups.lets false
      let clauses ← groups.requires.mapM (translate context)
      mkLambdaFVars #[unitExpr, arguments, state] (← conjunction clauses)

/-- Whether a function's body is verified: it has one, and neither its
specification nor its module sets `pragma verify = false`. -/
private def verifiedBody (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Bool :=
  declaration.body != .absent &&
    !(declaration.pragmas.any fun
      | .assign "verify" (.constant (.bool false)) _ => true
      | _ => false)

/-- The functions a declaration's body calls. -/
private def directCallees (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Array LeanerIR.FunctionHandle := Id.run do
  let .structured root := declaration.body | return #[]
  let mut pending := #[root]
  let mut visited : Array Nat := #[]
  let mut callees := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if let .operation (.call (.function reference)) .. := expression.kind then
      if let some handle := LeanerIR.SemanticOperations.resolveFunction? unit namespaceId reference then
        callees := callees.push handle
    pending := pending ++ LeanerIR.Validation.expressionChildren expression.kind
  return callees

/-- Behavioral predicates in the data invariants a body must use or establish,
including constructed local values and stored resources. They need the same
proof assumptions as predicates written in the function's own contract. -/
private def invariantNeeds (unit : ValidatedUnit)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (needed : SpecReads → Bool) : Bool := Id.run do
  let carries (id : LeanerIR.TypeId) : Bool :=
    match ns.tables.types[id.index]? with
    | some (.reference reference) =>
        (ns.tables.types[reference.referent.index]?).any
          (carriesMatchingInvariant unit needed · #[] 0)
    | some ty => carriesMatchingInvariant unit needed ty #[] 0
    | none => false
  if declaration.locals.any (carries ·.type.typeId) ||
      declaration.signature.parameters.any (carries ·.typeUse.typeId) ||
      declaration.signature.results.any (carries ·.typeId) then return true
  let .structured root := declaration.body | return false
  let mut pending := #[root]
  let mut visited : Array Nat := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if carries expression.typeId then return true
    pending := pending ++ LeanerIR.Validation.expressionChildren expression.kind
  return false

/-- Whether a function's theorem assumes the typing a run of a function
value it cannot see needs (`designs/static-typing.md`, Phase 5): a body it
verifies with a parameter of function type, or calling, through verified
bodies, one that has; or a verified body whose contract states `aborts_of`
or `result_of`. -/
def assumesTyping (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool := Id.run do
  let mut pending := #[(namespaceId, ns, declaration)]
  let mut visited : Array (Nat × Nat) := #[]
  while let some (namespaceId, ns, declaration) := pending.back? do
    pending := pending.pop
    if visited.contains (namespaceId.index, declaration.name.index) then continue
    visited := visited.push (namespaceId.index, declaration.name.index)
    unless verifiedBody declaration do continue
    if invariantNeeds unit ns declaration (·.determinism) then return true
    if declaration.contract.conditions.any (fun condition =>
        (readsFrom unit [(namespaceId, condition.expression)]).determinism) then
      return true
    if declaration.signature.parameters.any (fun parameter =>
        ns.tables.types[parameter.typeUse.typeId.index]? matches some (.function ..)) then
      return true
    for callee in directCallees unit namespaceId ns declaration do
      let some calleeNs := unit.namespaces[callee.namespaceId.index]? | continue
      let some calleeDeclaration := calleeNs.functions[callee.functionId.index]? | continue
      pending := pending.push (callee.namespaceId, calleeNs, calleeDeclaration)
  return false

/-- Whether a function's theorem assumes that the unit's runs end
(`Terminating`): its contract, or the contract of a function it calls at
any depth, states `result_of`, which a proof resolves at a known function
value through a run of it. -/
def assumesTermination (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool := Id.run do
  let mut pending := #[(namespaceId, ns, declaration)]
  let mut visited : Array (Nat × Nat) := #[]
  while let some (namespaceId, ns, declaration) := pending.back? do
    pending := pending.pop
    if visited.contains (namespaceId.index, declaration.name.index) then continue
    visited := visited.push (namespaceId.index, declaration.name.index)
    if invariantNeeds unit ns declaration (·.result) then return true
    if declaration.contract.conditions.any (fun condition =>
        (readsFrom unit [(namespaceId, condition.expression)]).result) then
      return true
    for callee in directCallees unit namespaceId ns declaration do
      let some calleeNs := unit.namespaces[callee.namespaceId.index]? | continue
      let some calleeDeclaration := calleeNs.functions[callee.functionId.index]? | continue
      pending := pending.push (callee.namespaceId, calleeNs, calleeDeclaration)
  return false

/-- Whether a condition states a behavioral predicate, itself or through
the specification functions it calls. -/
def conditionReadsBehavior (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (root : ExprId) : Bool :=
  let reads := readsFrom unit [(namespaceId, root)]
  reads.unit || reads.requires

/-- Whether a condition reads the table of declared preconditions
(`requires_of`), itself or through the specification functions it calls. -/
def conditionReadsRequires (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (root : ExprId) : Bool :=
  (readsFrom unit [(namespaceId, root)]).requires

/-- Whether a function's contract states a behavioral predicate or assumes
typing: then it takes the executable unit. -/
def contractReadsUnit (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  declaration.contract.conditions.any (conditionReadsBehavior unit namespaceId ·.expression) ||
    assumesTyping unit namespaceId ns declaration ||
    signatureInvariantsReadUnit unit ns declaration ||
    (storedInvariantReadsUnit unit && reachesMemory unit namespaceId declaration)

/-- Whether a function's precondition, its `requires` and the pre-state
`let`s they read, states a behavioral predicate. -/
def preconditionReadsUnit (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  declaration.contract.conditions.any fun condition =>
    (condition.kind matches .requires | .letPre _) &&
      conditionReadsBehavior unit namespaceId condition.expression

/-- What a function's contract reads, itself or through the specification
functions it calls. -/
private def contractReads (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : SpecReads :=
  readsFrom unit <| declaration.contract.conditions.toList.map
    fun condition => (namespaceId, condition.expression)

/-- Whether a function's contract binds a state label: then the closer keeps
program points as its witnesses. -/
def contractBindsStateLabel (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  (contractReads unit namespaceId declaration).labels

/-- Whether a function's contract states `requires_of`: then it also takes
the unit's table of declared preconditions. -/
def contractReadsRequires (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Bool :=
  (contractReads unit namespaceId declaration).requires

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
        let (current, prophecy) ← referenceParts signature.skolems referent result
        return (#[{ entry := ← nativeBinder signature.skolems slot.physical referent current }],
          #[← mkEq prophecy current],
          #[some (← nativeBinder signature.skolems slot.physical referent prophecy)])
      if signature.resultComponentTypes.any isReferenceType then
        let values ← rowProjections signature.skolems result signature.resultComponentTypes
        let mut viewed := #[]
        let mut premises := #[]
        let mut finals := #[]
        for (componentType, value) in signature.resultComponentTypes.zip values do
          if isReferenceType componentType then
            let referent := componentType.appArg!
            let (current, prophecy) ← referenceParts signature.skolems referent value
            viewed := viewed.push
              (mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) ((← frameCarriers signature.skolems)) referent current)
            premises := premises.push (← mkEq prophecy current)
            finals := finals.push (some
              (mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) ((← frameCarriers signature.skolems)) referent prophecy))
          else
            finals := finals.push none
            viewed := viewed.push
              (mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) ((← frameCarriers signature.skolems)) componentType
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
def ensureSpecFunctionDefinitions (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo) :
    MetaM Unit := do
  for reference in recursiveSpecFunctions unit do
    let some ns := unit.namespaces[reference.namespaceId.index]? | continue
    let context : Context := {
      unit, ns, twins
      namespaceId := reference.namespaceId
      locals := #[]
      results := #[] }
    let _ ← translate.ensureDefinition context reference

/-- Define a lemma's statement once (`ensureLemmaDefinitions`). -/
def ensureLemmaDefinitions (unit : ValidatedUnit) (twins : Array SpecTypes.TwinInfo)
    (reference : LeanerIR.QualifiedRef) : MetaM LemmaNames := do
  let some ns := unit.namespaces[reference.namespaceId.index]?
    | throwError "a lemma's namespace is out of range"
  let context : Context := {
    unit, ns, twins
    namespaceId := reference.namespaceId
    locals := #[]
    results := #[] }
  translate.ensureLemmaDefinitions context reference

/-- The Lean type of each parameter of a lemma's definitions. -/
def lemmaParameterTypes (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    MetaM (Array Lean.Expr) := do
  let some (_, declaration) := lemmaOf? unit reference
    | throwError "a lemma does not resolve"
  declaration.signature.parameters.mapM fun parameter =>
    (·.leanType) <$> signatureDomain unit parameter.typeUse.typeId "a lemma parameter"

/-- What a lemma's statement reads besides its parameters: the executable
unit, the table of declared preconditions, the family, and the state, each
a binder of its definitions in this order where read. -/
def lemmaReadsOf (unit : ValidatedUnit) (reference : LeanerIR.QualifiedRef) :
    Bool × Bool × Bool × Bool :=
  match lemmaOf? unit reference with
  | some (_, declaration) =>
      let reads := readsFrom unit <|
        (declaration.contract.conditions.toList ++ declaration.proof.toList).map
          fun condition => (reference.namespaceId, condition.expression)
      (reads.unit, reads.requires, lemmaTakesFrame unit reference reads.state, reads.state)
  | none => (false, false, false, false)

/-- The value of the uninterpreted function `name` at a native's type
parameters and argument values. -/
private def modelApplication (context : Context)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) (name : String)
    (parameterSlots : Array Slot) (values : Array Lean.Expr) (domain : Domain) :
    MetaM Lean.Expr := do
  let some types := context.types
    | throwError "the native model `{name}` is applied outside a family"
  let typeParameters := declaration.signature.generics.filter (·.kind == .typeArg) |>.size
  let typeArguments ← (List.range typeParameters).mapM fun index =>
    return mkApp (mkConst ``LeanerLang.Contract.SpecTypeArgument.native)
      (mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.substWith) types
        (mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.param) (toExpr index)))
  let encoded ← (parameterSlots.zip values).mapM fun (slot, value) =>
    (domainOf slot.physical).encode value
  mkAppOptM ``LeanerLang.Contract.opaqueSpec
    #[toExpr name, ← mkListLit (mkConst ``LeanerLang.Contract.SpecTypeArgument) typeArguments,
      domain.leanType, none, ← mkListLit (mkConst ``RuntimeValue) encoded.toList]

/-- A native's path-qualified name. -/
private def nativeName (context : Context)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : MetaM String := do
  let some path := context.unit.tables.namespaces[context.namespaceId.index]?
    | throwError "a native's namespace has no path"
  let some qualified := context.unit.tables.names[declaration.name.index]?
    | throwError "a native has no name"
  return "::".intercalate (path.segments.toList ++ [qualified.name])

/-- The result clauses of an uninterpreted native model: the result is the
model function's value at the native's type parameters and arguments, and a
vector result has the length the model states. None for a native without
exactly one result. -/
private def uninterpretedResult (context : Context)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (resultLength : Option Nat) (function : Option String)
    (parameterSlots : Array Slot) (parameterBound : Array SlotBinders)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders) : MetaM (Array Lean.Expr) := do
  let [resultSlot] := resultSlots.toList | return #[]
  let [resultBinders] := resultBound.toList | return #[]
  let name ← match function with
    | some name => pure name
    | none => nativeName context declaration
  let domain := domainOf resultSlot.physical
  let value ← modelApplication context declaration name parameterSlots
    (parameterBound.map (·.current)) domain
  -- The uninterpreted value on the left: a second call at the same arguments
  -- rewrites to the first's result, and a clause naming the native closes.
  let mut clauses := #[← mkEq value resultBinders.entry]
  if let some length := resultLength then
    if domain matches .aggregate then
      clauses := clauses.push (← mkEq
        (← mkAppM ``LeanerLang.Contract.lengthVector #[resultBinders.entry])
        (toExpr (Int.ofNat length)))
  return clauses

/-- The result clause of the structural-order model: the result is the
ordering variant at the order of the two operands. The result is on the
left, as a defined model states what the result is. -/
private def orderResult (context : Context)
    (parameterSlots : Array Slot) (parameterBound : Array SlotBinders)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders) : MetaM Lean.Expr := do
  let [(leftSlot, left), (rightSlot, right)] := (parameterSlots.zip parameterBound).toList
    | throwError "a structural order compares two operands"
  let [resultSlot] := resultSlots.toList | throwError "a structural order has one result"
  let [resultBinders] := resultBound.toList | throwError "a structural order has one result"
  let order ← structuralOrderTerm context
    (← (domainOf leftSlot.physical).encode left.current)
    (← (domainOf rightSlot.physical).encode right.current)
  mkEq resultBinders.entry (← orderingValue context resultSlot.physical order)

/-- The executable role a function plays for an intrinsic map of its
namespace, with the map's model. -/
def mapRoleOf? (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Option String := do
  let target : LeanerIR.QualifiedRef := { namespaceId, name := declaration.name }
  let intrinsic ← ns.intrinsics.find? fun intrinsic =>
    intrinsic.model == "map" && intrinsic.executableBindings.any (·.target == target)
  let binding ← intrinsic.executableBindings.find? (·.target == target)
  guard ((mapModel? unit (.nominal intrinsic.owner #[])).isSome ||
    ((tableModel? unit (.nominal intrinsic.owner #[])).isSome &&
      (binding.role == "map_borrow" || binding.role == "map_has_key")))
  pure binding.role

/-- Read-only Table roles observe external contents at the contract's memory.
They do not require an entries field in the physical owner. -/
private def tableReadRoleOf? (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : Option String := do
  let role ← mapRoleOf? unit namespaceId ns declaration
  let parameter ← declaration.signature.parameters[0]?
  let .reference reference ← unit.tables.types[parameter.typeUse.typeId.index]? | none
  let ty ← unit.tables.types[reference.referent.index]?
  let _ ← tableModel? unit ty
  some role

/-- The model of the map a role function's namespace owns. -/
private def mapRoleModel (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) : MetaM MapModel := do
  let target : LeanerIR.QualifiedRef := { namespaceId, name := declaration.name }
  let some intrinsic := ns.intrinsics.find? fun intrinsic =>
      intrinsic.model == "map" && intrinsic.executableBindings.any (·.target == target)
    | throwError "a map role function has no intrinsic declaration"
  let some model := mapModel? unit (.nominal intrinsic.owner #[])
    | throwError "a map role function's owner has no carried representation"
  return model

/-- The runtime value of a parameter's binder at entry, or at exit. -/
private def slotRuntime (slot : Slot) (binder : Lean.Expr) : MetaM Lean.Expr := do
  if (← whnfR (← inferType binder)).isConstOf ``RuntimeValue then return binder
  (domainOf slot.physical).encode binder

/-- Entry-state observations shared by Table membership and shared lookup.
The role remains an explicit contract hypothesis, just like entries-layout
intrinsics. This translation never executes a hypothetical specification map. -/
private def tableReadOperands (context : Context)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (slots : Array Slot) (bound : Array SlotBinders) : MetaM (Lean.Expr × Lean.Expr) := do
  let [(tableSlot, table), (keySlot, key)] := (slots.zip bound).toList
    | throwError "a Table read role requires a Table and a key"
  unless tableSlot.kind == .sharedRef && keySlot.kind == .plain do
    throwError "a Table read role requires a shared Table reference and a key value"
  let .nominal _ #[.typeArg keyType, .typeArg _] := tableSlot.physical
    | throwError "a Table read role requires an owner with key and value type arguments"
  unless context.unit.tables.types[keyType.typeId.index]? == some keySlot.physical do
    throwError "a Table read role's key type differs from its owner's key type"
  let some parameter := declaration.signature.parameters[0]?
    | throwError "a Table read role has no owner parameter"
  let some keyParameter := declaration.signature.parameters[1]?
    | throwError "a Table read role has no key parameter"
  let table ← context.observeInput parameter.typeUse.typeId
    (← slotRuntime tableSlot table.entry)
  unless ← isSnapshotValue table do
    throwError "a Table read role requires an observed contents snapshot"
  let key ← context.observeInput keyParameter.typeUse.typeId (← slotRuntime keySlot key.entry)
  return (table, ← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.identity
    #[← snapshotOperand key])

private def tableReadEnsures (context : Context)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) (role : String)
    (parameterSlots : Array Slot) (parameterBound : Array SlotBinders)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders) : MetaM Lean.Expr := do
  let [(resultSlot, result)] := (resultSlots.zip resultBound).toList
    | throwError "a Table read role requires one result"
  let (table, key) ← tableReadOperands context declaration parameterSlots parameterBound
  match role with
  | "map_has_key" =>
      unless resultSlot.kind == .plain && resultSlot.physical == .bool do
        throwError "Table membership requires a boolean result"
      mkEq (← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.hasKey #[table, key]) result.entry
  | "map_borrow" =>
      unless resultSlot.kind == .sharedRef do
        throwError "Table shared lookup requires a shared reference result"
      let some resultType := declaration.signature.results[0]?
        | throwError "Table shared lookup has no result type"
      let some tableSlot := parameterSlots[0]?
        | throwError "Table shared lookup has no owner parameter"
      let .nominal _ #[.typeArg _, .typeArg valueType] := tableSlot.physical
        | throwError "Table shared lookup requires an owner with key and value type arguments"
      unless context.unit.tables.types[valueType.typeId.index]? == some resultSlot.physical do
        throwError "Table shared lookup's result type differs from its owner's value type"
      let observed ← context.observeInput resultType.typeId (← slotRuntime resultSlot result.entry)
      mkEq (← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.getValue #[table, key])
        (← snapshotOperand observed)
  | _ => throwError "the Table read role `{role}` is not carried"

/-- The value of the `Option` result type at an optional runtime value: its
one vector field holds the value or nothing. -/
private def optionValue (context : Context) (ty : IrTy) (value : Option Lean.Expr) :
    MetaM Lean.Expr := do
  let some (owner, _, _) := nominalDeclaration? context.unit ty
    | throwError "an optional map result is not an `Option`"
  let elements ← mkArrayLit (mkConst ``RuntimeValue) value.toList
  let vector ← mkAppM ``RuntimeValue.vector #[elements]
  mkAppM ``RuntimeValue.nominal
    #[toExpr owner, toExpr (none : Option String), ← mkArrayLit (mkConst ``RuntimeValue) [vector]]

/-- The runtime values of a role's parameters at entry, and of its map at
exit, with the model terms the roles share. -/
private structure RoleOperands where
  role : String
  model : MapModel
  slots : Array Slot
  bound : Array SlotBinders

namespace RoleOperands

private def parameter (operands : RoleOperands) (index : Nat) : MetaM (Slot × SlotBinders) := do
  let (some slot, some binders) := (operands.slots[index]?, operands.bound[index]?)
    | throwError m!"the intrinsic map role `{operands.role}` lacks parameter {index}"
  pure (slot, binders)

/-- A parameter's runtime value at entry. -/
private def entry (operands : RoleOperands) (index : Nat) : MetaM Lean.Expr := do
  let (slot, binders) ← operands.parameter index
  slotRuntime slot binders.entry

/-- A parameter's clause value at entry, in its own domain. -/
private def clause (operands : RoleOperands) (index : Nat) : MetaM Lean.Expr := do
  pure (← operands.parameter index).2.entry

/-- A mutated parameter's runtime value at exit. -/
private def exit (operands : RoleOperands) (index : Nat) : MetaM Lean.Expr := do
  let (slot, binders) ← operands.parameter index
  let some exit := binders.exit
    | throwError m!"the intrinsic map role `{operands.role}` does not mutate parameter {index}"
  slotRuntime slot exit

private def size (map : Lean.Expr) : MetaM Lean.Expr :=
  mkAppM ``LeanerIR.Maps.size #[map]

private def hasKey (map key : Lean.Expr) : MetaM Lean.Expr :=
  mkAppM ``LeanerIR.Maps.hasKey #[map, key]

private def empty (map : Lean.Expr) : MetaM Lean.Expr := do
  mkEq (← size map) (mkIntLit 0)

/-- The key an enumerating role reads: the first, or the last. -/
private def endKey (operands : RoleOperands) (map : Lean.Expr) : MetaM Lean.Expr := do
  let front := operands.role == "map_borrow_front" || operands.role == "map_front_key" ||
    operands.role == "map_pop_front"
  let position ← if front then pure (mkIntLit 0)
    else mkAppM ``HSub.hSub #[← size map, mkIntLit 1]
  mkAppM ``LeanerIR.Maps.keyAt #[map, position]

end RoleOperands

/-- The abort condition of an intrinsic map role over the entry binders
(`designs/intrinsic-maps.md`, "Roles"), `none` when it never aborts. -/
private def mapRoleAborts (context : Context) (operands : RoleOperands) :
    MetaM (Option Lean.Expr) := do
  let map := operands.entry 0
  let absent : MetaM Lean.Expr := do
    mkEq (← RoleOperands.hasKey (← map) (← operands.entry 1)) (mkConst ``Bool.false)
  match operands.role with
  | "map_borrow" | "map_borrow_mut" | "map_del_must_exist" | "map_del_return_key" =>
      return some (← absent)
  | "map_add_no_override" =>
      return some (← mkEq (← RoleOperands.hasKey (← map) (← operands.entry 1)) (mkConst ``Bool.true))
  | "map_destroy_empty" => return some (← mkAppM ``Ne #[← RoleOperands.size (← map), mkIntLit 0])
  | "map_borrow_front" | "map_borrow_back" | "map_front_key" | "map_back_key" | "map_pop_front"
  | "map_pop_back" => return some (← RoleOperands.empty (← map))
  | "map_new_from" =>
      return some (← mkAppM ``LeanerIR.Maps.AbortsNewFrom #[← operands.entry 0, ← operands.entry 1])
  | "map_add_all" =>
      return some (← mkAppM ``LeanerIR.Maps.AbortsAddAll
        #[← map, ← operands.entry 1, ← operands.entry 2])
  | "map_upsert_all" =>
      return some (← mkAppM ``LeanerIR.Maps.AbortsUpsertAll #[← operands.entry 1, ← operands.entry 2])
  | "map_append_disjoint" =>
      return some (← mkAppM ``LeanerIR.Maps.AbortsAppendDisjoint #[← map, ← operands.entry 1])
  | "map_trim" => return some (← mkAppM ``LT.lt #[← RoleOperands.size (← map), ← operands.clause 1])
  | "map_replace_key_inplace" =>
      return some (← mkAppM ``LeanerIR.Maps.AbortsReplaceKey
        #[operands.model.layout, operands.model.discipline, ← map, ← operands.entry 1,
          ← operands.entry 2])
  | "map_iter_borrow_mut" =>
      let (iteratorSlot, _) ← operands.parameter 0
      let some (owner, variant) := iteratorPosition? context.unit iteratorSlot.physical
        | throwError "an iterator of an intrinsic map has no position variant"
      return some (← mkAppM ``Not
        #[← iteratorInRange owner variant (← operands.entry 0) (← operands.entry 1)])
  | _ => return none

/-- The roles returning a mutable reference into the map, whose ensures state
the map at the reference's final value. -/
private def mapRoleReadsFinal (role : String) : Bool :=
  role == "map_borrow_mut" || role == "map_borrow_mut_with_default" ||
    role == "map_iter_borrow_mut"

/-- The result and effect clauses of an intrinsic map role, over the entry
binders, the exit binders of a mutated map, and the result binders. A value
the role builds replaces its result; an observation of the map is on the
left, so that a clause naming it speaks about the result. -/
private def mapRoleEnsures (context : Context) (operands : RoleOperands)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders)
    (finals : Array (Option Lean.Expr)) : MetaM (Array Lean.Expr) := do
  let model := operands.model
  let role := operands.role
  let result : MetaM (Slot × Lean.Expr) := do
    let (some slot, some binders) := (resultSlots[0]?, resultBound[0]?)
      | throwError m!"the intrinsic map role `{role}` lacks its result"
    pure (slot, binders.entry)
  let observed (value : Lean.Expr) : MetaM Lean.Expr := do
    let (slot, binder) ← result
    mkEq (← (domainOf slot.physical).binderOfRuntime value) binder
  let built (value : Lean.Expr) : MetaM Lean.Expr := do
    let (_, binder) ← result
    mkEq binder value
  let tuple (values : List Lean.Expr) : MetaM Lean.Expr := do
    mkAppM ``RuntimeValue.tuple #[← mkArrayLit (mkConst ``RuntimeValue) values]
  let map := operands.entry 0
  let key := operands.entry 1
  let valueAt (map key : Lean.Expr) : MetaM Lean.Expr := mkAppM ``LeanerIR.Maps.valueAt #[map, key]
  let present : MetaM Lean.Expr := do
    mkEq (← RoleOperands.hasKey (← map) (← key)) (mkConst ``Bool.true)
  let update (map key value : Lean.Expr) : MetaM Lean.Expr := do
    mkAppM ``LeanerIR.Maps.update #[model.layout, model.discipline, map, key, value]
  let remove (map key : Lean.Expr) : MetaM Lean.Expr := do
    mkAppM ``LeanerIR.Maps.remove #[model.layout, model.discipline, map, key]
  let exitIs (value : Lean.Expr) : MetaM Lean.Expr := do mkEq (← operands.exit 0) value
  let optional (value : Option Lean.Expr) (condition : Option Lean.Expr) : MetaM Lean.Expr := do
    let (slot, binder) ← result
    let some_ ← optionValue context slot.physical value
    let none_ ← optionValue context slot.physical none
    match condition with
    | some condition => mkEq (← mkAppOptM ``ite #[none, condition, none, some_, none_]) binder
    | none => mkEq some_ binder
  let final : MetaM Lean.Expr := do
    let (slot, _) ← result
    let some (some final) := finals[0]?
      | throwError m!"the intrinsic map role `{role}` returns no mutable reference"
    slotRuntime slot final
  match role with
  | "map_new" => return #[← built (← mkAppM ``LeanerIR.Maps.empty #[model.layout])]
  | "map_new_from" =>
      return #[← built (← mkAppM ``LeanerIR.Maps.updateAll #[model.layout, model.discipline,
        ← mkAppM ``LeanerIR.Maps.empty #[model.layout], ← operands.entry 0, ← operands.entry 1])]
  | "map_len" =>
      let (_, binder) ← result
      return #[← mkEq (← RoleOperands.size (← map)) binder]
  | "map_is_empty" =>
      let (_, binder) ← result
      return #[← mkEq (← mkDecide (← RoleOperands.empty (← map))) binder]
  | "map_has_key" =>
      let (_, binder) ← result
      return #[← mkEq (← RoleOperands.hasKey (← map) (← key)) binder]
  | "map_borrow" => return #[← observed (← valueAt (← map) (← key))]
  | "map_borrow_with_default" =>
      return #[← observed (← mkAppOptM ``ite
        #[none, ← present, none, ← valueAt (← map) (← key), ← operands.entry 2])]
  | "map_get" => return #[← optional (some (← valueAt (← map) (← key))) (some (← present))]
  | "map_borrow_mut" | "map_borrow_mut_with_default" =>
      let current ← if role == "map_borrow_mut" then valueAt (← map) (← key)
        else mkAppOptM ``ite #[none, ← present, none, ← valueAt (← map) (← key), ← operands.entry 2]
      return #[← observed current, ← exitIs (← update (← map) (← key) (← final))]
  | "map_iter_borrow_mut" =>
      -- The iterator is parameter 0, the map parameter 1.
      let (iteratorSlot, _) ← operands.parameter 0
      let some (owner, variant) := iteratorPosition? context.unit iteratorSlot.physical
        | throwError "an iterator of an intrinsic map has no position variant"
      let iteratorKey ← iteratorKey owner variant (← operands.entry 0) (← operands.entry 1)
      return #[← observed (← valueAt (← operands.entry 1) iteratorKey),
        ← mkEq (← operands.exit 1) (← update (← operands.entry 1) iteratorKey (← final))]
  | "map_add_no_override" | "map_add_override_if_exists" =>
      return #[← exitIs (← update (← map) (← key) (← operands.entry 2))]
  | "map_upsert" =>
      return #[← optional (some (← valueAt (← map) (← key))) (some (← present)),
        ← exitIs (← update (← map) (← key) (← operands.entry 2))]
  | "map_del_must_exist" =>
      return #[← observed (← valueAt (← map) (← key)), ← exitIs (← remove (← map) (← key))]
  | "map_remove_or_none" =>
      return #[← optional (some (← valueAt (← map) (← key))) (some (← present)),
        ← exitIs (← remove (← map) (← key))]
  | "map_del_return_key" =>
      return #[← built (← tuple [← key, ← valueAt (← map) (← key)]),
        ← exitIs (← remove (← map) (← key))]
  | "map_destroy_empty" => return #[]
  | "map_add_all" | "map_upsert_all" =>
      return #[← exitIs (← mkAppM ``LeanerIR.Maps.updateAll #[model.layout, model.discipline, ← map,
        ← operands.entry 1, ← operands.entry 2])]
  | "map_append" | "map_append_disjoint" =>
      let other ← operands.entry 1
      return #[← exitIs (← mkAppM ``LeanerIR.Maps.updateAll #[model.layout, model.discipline, ← map,
        ← mkAppM ``LeanerIR.Maps.keysOf #[other], ← mkAppM ``LeanerIR.Maps.valuesOf #[other]])]
  | "map_trim" =>
      let count ← operands.clause 1
      return #[← built (← mkAppM ``LeanerIR.Maps.dropEntries #[model.layout, ← map, count]),
        ← exitIs (← mkAppM ``LeanerIR.Maps.takeEntries #[model.layout, ← map, count])]
  | "map_replace_key_inplace" =>
      return #[← exitIs (← mkAppM ``LeanerIR.Maps.replaceKey
        #[model.layout, ← map, ← operands.entry 1, ← operands.entry 2])]
  | "map_borrow_front" | "map_borrow_back" =>
      let key ← operands.endKey (← map)
      return #[← built (← tuple [key, ← valueAt (← map) key])]
  | "map_front_key" | "map_back_key" => return #[← observed (← operands.endKey (← map))]
  | "map_pop_front" | "map_pop_back" =>
      let key ← operands.endKey (← map)
      return #[← built (← tuple [key, ← valueAt (← map) key]), ← exitIs (← remove (← map) key)]
  | "map_prev_key" | "map_next_key" =>
      let some rank := model.rank?
        | throwError m!"the intrinsic map role `{role}` needs an ordered map"
      let neighbour ← mkAppM (if role == "map_prev_key" then ``LeanerIR.Maps.prevKey?
        else ``LeanerIR.Maps.nextKey?) #[rank, ← map, ← key]
      let (slot, binder) ← result
      let none_ ← optionValue context slot.physical none
      let some_ ← withLocalDeclD `neighbour (mkConst ``RuntimeValue) fun neighbour => do
        mkLambdaFVars #[neighbour] (← optionValue context slot.physical (some neighbour))
      return #[← mkEq (← mkAppOptM ``Option.elim #[none, none, neighbour, none_, some_]) binder]
  | "map_keys" => return #[← built (← mkAppM ``LeanerIR.Maps.keysOf #[← map])]
  | "map_values" => return #[← built (← mkAppM ``LeanerIR.Maps.valuesOf #[← map])]
  | "map_to_vec_pair" =>
      return #[← built (← tuple [← mkAppM ``LeanerIR.Maps.keysOf #[← map],
        ← mkAppM ``LeanerIR.Maps.valuesOf #[← map]])]
  | _ => throwError m!"the intrinsic map role `{role}` is not carried"

/-- Free state-label definitions, independent of clause order. Definitions
under a quantifier are local to that quantifier. -/
private def stateDefinitions (ns : ValidatedNamespace)
    (contract : LeanerIR.FunctionContract) : Array StateDefinition := Id.run do
  let mut pending := contract.conditions.map fun condition =>
    (condition.expression, (#[] : Array (LeanerIR.PatternId × ExprId)))
  let mut visited : Array ExprId := #[]
  let mut definitions := #[]
  while let some (id, bindings) := pending.back? do
    pending := pending.pop
    if visited.contains id then continue
    visited := visited.push id
    let some expression := ns.expressions[id.index]? | continue
    if expression.kind matches .quantifier .. then continue
    if let .operation (.specification operation) _ _ _ := expression.kind then
      let post := match operation with
        | .publish range | .remove range | .update range
        | .behavior .ensuresOf range | .behavior .resultOf range => range.post
        | _ => none
      if let some label := post then
        definitions := definitions.push { label, operation := id, bindings }
    if let .letDecl pattern (some initializer) body := expression.kind then
      pending := pending.push (initializer, bindings) |>.push
        (body, bindings.push (pattern, initializer))
    else
      pending := pending ++ (LeanerIR.Validation.expressionChildren expression.kind).map
        (·, bindings)
  return definitions

/-- The contract of a function over its native arguments and result. -/
def buildContract (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (signature : NativeSignature)
    (twins : Array SpecTypes.TwinInfo := #[])
    (carrier : Option Lean.Expr := none)
    (codecs : Option Lean.Expr := none)
    (types : Option Lean.Expr := none)
    (typeInstantiation : Option Lean.Expr := none)
    (nativeModel : Option NativeModel := none)
    (executable : Option Lean.Expr := none)
    (requiresTable : Option Lean.Expr := none)
    (stored : StoredInvariants := {}) :
    MetaM Lean.Expr := do
  -- The data invariants of stored resources hold of the memory a function
  -- reaches, at entry and at exit, as the Move Prover assumes them at entry
  -- and checks them where a value is written.
  let reach := memoryReach unit namespaceId declaration
  let reachesTables := !(tableReach unit namespaceId declaration).isEmpty
  let delegated := delegatesInvariants unit namespaceId declaration
  if let some resources := reach then
    for (handle, reason) in stored.unsupported do
      if resources.contains handle then
        let name := ((LeanerIR.SemanticOperations.structName? unit handle).map (·.name)).getD
          s!"{handle.structId}"
        throwError "the data invariant of `{name}`, which this function reaches, is not \
          carried: {reason}"
  -- A function reaching a resource that carries one assumes them at entry;
  -- every other function that reaches memory keeps them, wherever they hold.
  if reachesTables then
    if let some (handle, reason) := stored.collectionUnsupported[0]? then
      let name := ((LeanerIR.SemanticOperations.structName? unit handle).map (·.name)).getD
        s!"{handle.structId}"
      throwError "the stored collection invariant of `{name}` is not carried: {reason}"
  let assumesStored := (reachesTables && stored.hasCollections) ||
    reach.any fun resources => resources.any stored.carriers.contains
  let storedInvariant := if reach.isSome then stored.predicate else none
  let memoryInvariants (state : Lean.Expr) : MetaM Lean.Expr := do
    let some invariant := storedInvariant | throwError "internal: no stored invariants"
    mkAppM ``LeanerIR.Proofs.Denote.MemoryInvariants
      #[← storedInvariantAt unit invariant executable state, state]
  -- An intrinsic map role is read by the map model, not by its source
  -- specification (`designs/intrinsic-maps.md`, "Roles").
  let tableRole := tableReadRoleOf? unit namespaceId ns declaration
  let mapRole ← (if tableRole.isSome then none else mapRoleOf? unit namespaceId ns declaration).mapM fun role =>
    return (role, ← mapRoleModel unit namespaceId ns declaration)
  let groups ← if mapRole.isSome || tableRole.isSome then pure {}
    else groupConditions unit declaration.contract.conditions
  let invariantResources ← invariantModifiedResources ns declaration.contract
  -- A function's pragmas: its contract's, and those its module sets and its
  -- contract does not.
  let inEffect (name : String) : Bool := declaration.pragmas.any fun
    | .assign pragmaName (.constant (.bool true)) _ => pragmaName == name
    | _ => false
  let isPartial := pragmaEnabled declaration.contract "aborts_if_is_partial" ||
    inEffect "aborts_if_is_partial"
  -- A native without a specification is read by its Prover model
  -- (`NativeModel`): it aborts only as the model says, and its result is the
  -- value of the model's uninterpreted function at its arguments, so two
  -- calls at equal arguments agree and a clause naming the function speaks
  -- about the same value.
  let isStrict := pragmaEnabled declaration.contract "aborts_if_is_strict" ||
    inEffect "aborts_if_is_strict" || nativeModel.isSome ||
    mapRole.isSome || tableRole.isSome
  let parameterSlots ← declaration.signature.parameters.mapIdxM fun index parameter =>
    slotOf { unit := unit, namespaceId := namespaceId, ns := ns,
             locals := #[], results := #[], twins := twins,
             carrier := carrier, codecs := codecs, types := types,
             typeInstantiation := typeInstantiation }
      (Name.mkSimple (if parameter.name.isEmpty then s!"argument{index}" else parameter.name))
      parameter.typeUse.typeId (allowReference := true)
  let resultSlots ← declaration.signature.results.mapIdxM fun index result =>
    slotOf { unit := unit, namespaceId := namespaceId, ns := ns,
             locals := #[], results := #[], twins := twins,
             carrier := carrier, codecs := codecs, types := types,
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
  let argumentsType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.HList) ((← frameCarriers signature.skolems))
    signature.params
  let resultType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.ResultShape.carrier) ((← frameCarriers signature.skolems))
    signature.shape
  let runtimeState := mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) (← frameUnit signature.skolems)
  let failure := mkConst ``LeanerIR.Proofs.Failure
  /- In a one-state clause a parameter denotes its entry value; in
  `ensures` it denotes the current value — the prophecy of a mutable
  reference — and `spec.old` reaches the entry. -/
  let labelDefinitions := stateDefinitions ns declaration.contract
  let contextOf (state : Lean.Expr) (bound : Array SlotBinders) : Context :=
    { unit, namespaceId, ns, twins
      carrier := carrier, codecs := codecs, types := types,
      typeInstantiation := typeInstantiation
      locals := padLocals (bound.map fun binders => some binders.entry)
      localTypes := functionLocalTypes
      localNames := declaration.locals.map (·.name)
      oldLocals := padLocals (bound.map fun binders => some binders.entry)
      results := #[]
      state := some state, oldState := some state, executable, requiresTable
      labelDefinitions
      labelEntry := some { memory := state, locals := padLocals (bound.map (some ∘ SlotBinders.entry)) }
      labelExit := some { memory := state, locals := padLocals (bound.map (some ∘ SlotBinders.entry)) } }
  let translateAll (context : Context) (clauses : Array ExprId) :
      MetaM (Array Lean.Expr) :=
    clauses.mapM (translate context)
  let requiresTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state => do
      let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
      let context ← bindLets (contextOf state bound) groups.lets false
      let dataInvariants ← slotDataInvariantTerms context parameterSlots bound
        (some ·.entry)
      let clauses ← translateAll context groups.requires
      -- At entry the invariants over the memory the function reaches hold
      -- everywhere, as the Move Prover assumes them; only what a write
      -- owes is specialized to the keys it modifies.
      let invariants ← namespaceInvariantTerms context none .entry
        (invariantApplies reach unit delegated) (specInstantiations unit namespaceId declaration)
      let storedAssumed ← if assumesStored && storedInvariant.isSome then
          pure #[← memoryInvariants state] else pure #[]
      let signers ← signerFacts context parameterSlots bound (some ·.entry)
      let hashes ← hashFacts context declaration parameterSlots bound
      let body ← conjunction (clauses ++ dataInvariants.map (·.1) ++ invariants.map (·.1) ++
        storedAssumed ++ signers ++ hashes)
      mkLambdaFVars #[arguments, state] body
  let ensuresTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `result resultType fun result =>
        withLocalDeclD `final runtimeState fun final => do
          let parameterBound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
          let (resultBound, prophecies, finals) ← resultBinders resultSlots signature result
          let context : Context :=
            { unit, namespaceId, ns, twins
              carrier := carrier, codecs := codecs, types := types,
              typeInstantiation := typeInstantiation
              locals := padLocals (parameterBound.map fun binders => some binders.current)
              localTypes := functionLocalTypes
              localNames := declaration.locals.map (·.name)
              oldLocals := padLocals (parameterBound.map fun binders => some binders.entry)
              results := resultBound.map (·.entry)
              resultFinals := finals
              resultTypes := resultSlots.map (·.physical)
              state := some final, oldState := some state, executable, requiresTable
              labelDefinitions
              labelEntry := some {
                memory := state, locals := padLocals (parameterBound.map (some ∘ SlotBinders.entry)) }
              labelExit := some {
                memory := final, locals := padLocals (parameterBound.map (some ∘ SlotBinders.current)) } }
          let context ← bindLets context groups.lets true
          let mut clauses ← groups.ensures.mapM fun (clause, range) =>
            return markObligation range (← translate context clause)
          if let some (.uninterpreted resultLength function _) := nativeModel then
            clauses := clauses ++ (← uninterpretedResult context declaration resultLength
              function parameterSlots parameterBound resultSlots resultBound)
          if let some .structuralOrder := nativeModel then
            clauses := clauses.push (← orderResult context parameterSlots parameterBound
              resultSlots resultBound)
          if let some role := tableRole then
            clauses := clauses.push (← tableReadEnsures { context with state := some state }
              declaration role parameterSlots parameterBound resultSlots resultBound)
          if let some (role, model) := mapRole then
            let operands : RoleOperands := { role, slots := parameterSlots, bound := parameterBound
                                             model }
            clauses := clauses ++ (← mapRoleEnsures context operands resultSlots resultBound finals)
          let resultInvariants ← slotDataInvariantTerms context resultSlots resultBound
            (some ·.entry)
          let resultSigners ← signerFacts context resultSlots resultBound (some ·.entry)
          let exitInvariants ← slotDataInvariantTerms context parameterSlots parameterBound
            (·.exit)
          let dataInvariantObligations :=
            (resultInvariants ++ exitInvariants).map fun (clause, range) =>
              markObligation range clause
          let storedInvariants ← if storedInvariant.isNone then pure #[]
            else if assumesStored then pure #[← memoryInvariants final]
            else pure #[← mkArrow (← memoryInvariants state) (← memoryInvariants final)]
          let invariants ← namespaceInvariantTerms context invariantResources .exit
            (invariantApplies reach unit delegated)
          let invariantObligations := invariants.map fun (clause, range) =>
            markObligation range clause
          let body ← conjunction (clauses ++ dataInvariantObligations ++ storedInvariants ++
            invariantObligations ++ resultSigners)
          -- The value view: a returned reference is read at its current
          -- value, as if it died on return, unless the clauses read its
          -- final value.
          let readsFinal := groups.ensures.any (fun (clause, _) => mentionsFinal ns clause) ||
            mapRole.any (mapRoleReadsFinal ·.1)
          let body ← if prophecies.isEmpty || readsFinal then pure body
            else mkArrow (← conjunction prophecies) body
          mkLambdaFVars #[arguments, state, result, final] body
  /- An `aborts_if` condition forces failure and excuses the postcondition.
  A non-partial list also covers every failure. Independently, when any
  abort codes are declared, a failure must match a conditioned code (or an
  unqualified abort condition) or a standalone `aborts_with` code. Partial
  conditions do not relax this code check. -/
  let declaredAborts := !groups.abortsIf.isEmpty
  let hasCodes := !groups.abortsWith.isEmpty || groups.abortsIf.any (·.code.isSome)
  -- A native model's abort: exactly when its predicate is false.
  let modelAborts (context : Context) (bound : Array SlotBinders) : MetaM (Option Lean.Expr) := do
    if let some role := tableRole then
      if role == "map_borrow" then
        let (table, key) ← tableReadOperands context declaration parameterSlots bound
        return some (← mkEq
          (← mkAppM ``LeanerIR.Proofs.Denote.SnapshotValue.Value.hasKey #[table, key])
          (mkConst ``Bool.false))
      return none
    if let some (role, model) := mapRole then
      return ← mapRoleAborts context { role, slots := parameterSlots, bound, model }
    let some (.uninterpreted _ _ (some predicate)) := nativeModel | return none
    let holds ← modelApplication context declaration predicate parameterSlots
      (bound.map (·.entry)) .boolean
    return some (mkNot (← Domain.boolean.ofBinder holds))
  let abortConditionTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state => do
      let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
      let clauses ← translateAll (← bindLets (contextOf state bound) groups.lets false)
        (groups.abortsIf.map (·.condition))
      let body ← if declaredAborts then disjunction clauses
        else pure ((← modelAborts (contextOf state bound) bound).getD (mkConst ``False))
      mkLambdaFVars #[arguments, state] body
  let abortsTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `failure failure fun failureBinder => do
        if !declaredAborts && !hasCodes then
          let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes
            arguments
          let permitted := (← modelAborts (contextOf state bound) bound).getD
            (if isStrict then mkConst ``False else mkConst ``True)
          mkLambdaFVars #[arguments, state, failureBinder] permitted
        else
          let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
          let context ← bindLets (contextOf state bound) groups.lets false
          let mut matched : Array Lean.Expr := #[]
          let mut conditions : Array Lean.Expr := #[]
          for clause in groups.abortsIf do
            let condition ← translate context clause.condition
            conditions := conditions.push condition
            match clause.code with
            | some code =>
                let codeTerm ← translate context code
                let codeMatches ← mkAppM ``LeanerLang.Contract.abortCodeMatches #[failureBinder, codeTerm]
                matched := matched.push (markObligation clause.range (← mkAppM ``And
                  #[condition, codeMatches]))
            | none => matched := matched.push (markObligation clause.range condition)
          for (code, range) in groups.abortsWith do
            let codeTerm ← translate context code
            let codeMatches ← mkAppM ``LeanerLang.Contract.abortCodeMatches #[failureBinder, codeTerm]
            matched := matched.push (markObligation range codeMatches)
          let permitted ← if hasCodes then
              let codes ← disjunction matched
              if declaredAborts && !isPartial && !groups.abortsWith.isEmpty then
                mkAppM ``And #[← disjunction conditions, codes]
              else pure codes
            else if declaredAborts && !isPartial then disjunction matched
            else pure (mkConst ``True)
          mkLambdaFVars #[arguments, state, failureBinder] permitted
  /- The frame is stated over global memory: a successful execution leaves
  the slots it does not declare as modified alone.  With no `modifies`
  clause the whole memory is unchanged.  With clauses, every listed resource
  type reads the same at every key other than its listed ones, and every
  other resource type reads the same everywhere; a loose frame leaves the
  unlisted resource types open.  The memory's update laws discharge the
  frame and a caller frames with it. -/
  let frameTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `initial runtimeState fun initial =>
      withLocalDeclD `result resultType fun result =>
        withLocalDeclD `final runtimeState fun final => do
          let body ←
            if declaration.contract.modifiesAll then
              pure (mkConst ``True)
            else if declaration.contract.modifies.isEmpty then
              mkEq final initial
            else do
              let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
              -- A key may name a pre-state `let`.
              let context ← bindLets (contextOf initial bound) groups.lets false
              let slots ← declaration.contract.modifies.mapM (modifiedSlotTerm context)
              slotFrame initial final slots (hasLooseFrame declaration.contract)
          mkLambdaFVars #[arguments, initial, result, final] body
  -- What the theorem assumes beyond the precondition: of a function with
  -- function-typed parameters, typed global memory, the frame resolving
  -- each such parameter's rows over type parameters to scalars, and keeping
  -- memory, as a parameter without `modifies_of` does.
  let assumesTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `initial runtimeState fun initial => do
      unless assumesTyping unit namespaceId ns declaration do
        return ← mkLambdaFVars #[arguments, initial] (mkConst ``True)
      let some executable := executable
        | throwError "internal: a contract assuming typing does not take the unit"
      let unitExpr ← executableUnit executable
      let values ← rowProjections signature.skolems arguments signature.argumentTypes
      let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
      let mut conjuncts := #[]
      for (type, value, index) in signature.argumentTypes.zipWith (·, ·) values |>.zipIdx |>.map
          (fun ((type, value), index) => (type, value, index)) do
        let type ← whnfR type
        if type.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.function 3 then
          let row := type.getArg! 0
          let value ← mkAppM ``Subtype.val #[value]
          -- The value is typed by its carrier; an invocation's rows that
          -- mention the frame's type parameters are decided where the frame
          -- resolves them to scalars.
          for invoked in #[row, type.getArg! 2] do
            if mentionsInvocableParameter invoked then
              conjuncts := conjuncts.push
                (mkApp3 (mkConst ``LeanerIR.Proofs.ScalarAt) unitExpr signature.skolems invoked)
          match declaration.contract.parameterFrames.find? (·.parameter.index == index) with
          | none =>
              conjuncts := conjuncts.push (← mkAppM'
                  (mkApp3 (mkConst ``LeanerIR.Proofs.KeepsMemoryAt) unitExpr executable
                    signature.skolems)
                #[row, value])
          | some frame =>
              -- The closed frame but for the targets, the formals bound to
              -- an invocation's arguments; `modifies_of<f> *` frames nothing.
              unless frame.modifiesAll do
                let invocationType := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.HList)
                  ((← frameCarriers signature.skolems)) row
                let frameTerm ← withLocalDeclD `invocation invocationType fun invocation =>
                  withLocalDeclD `pre runtimeState fun pre =>
                    withLocalDeclD `post runtimeState fun post => do
                      let componentTypes := rowElementTypes row
                      let components ← rowProjections signature.skolems invocation componentTypes
                      let mut context := contextOf pre bound
                      for (formal, ty, component) in frame.formals.zip
                          (componentTypes.zip components) do
                        let some physical := functionLocalTypes[formal.index]?
                          | throwError "a frame's formal has no local type"
                        let binder ← nativeBinder signature.skolems physical ty component
                        context := { context with
                          locals := context.locals.set! formal.index (some binder)
                          oldLocals := context.oldLocals.set! formal.index (some binder) }
                      let slots ← frame.modifies.mapM (modifiedSlotTerm context)
                      mkLambdaFVars #[invocation, pre, post] (← slotFrame pre post slots false)
                conjuncts := conjuncts.push (← mkAppM'
                    (mkApp3 (mkConst ``LeanerIR.Proofs.FramedAt) unitExpr executable
                      signature.skolems)
                  #[row, frameTerm, value])
      mkLambdaFVars #[arguments, initial] (← conjunction conjuncts)
  mkAppOptM ``LeanerIR.Proofs.Contract.mk
    #[some runtimeState, some failure, some argumentsType, some resultType,
      some requiresTerm, some assumesTerm, some ensuresTerm, some abortsTerm,
      some abortConditionTerm, some abortConditionTerm, some frameTerm]

/-- Segments of a `leanerPath`, ignoring separators. -/
private partial def pathSegments (stx : Syntax) : Array String :=
  match stx with
  -- The identifier's name, as the elaborator registers it: a quoted segment
  -- such as a script's `«<SELF>_0»` without its quotes.
  | .ident _ _ name _ => #[name.toString (escape := false)]
  | .atom _ value => if value == "::" then #[] else #[value]
  | .node _ _ arguments => arguments.flatMap pathSegments
  | _ => #[]

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

/-- A function's own name, without its namespace path. -/
private def functionName (unit : ValidatedUnit) (handle : LeanerIR.FunctionHandle) : String :=
  let qualified? : Option LeanerIR.QualifiedName := do
    let ns ← unit.namespaces[handle.namespaceId.index]?
    let declaration ← ns.functions[handle.functionId.index]?
    unit.tables.names[declaration.name.index]?
  (qualified?.map (·.name)).getD ""

/-- A function's path-qualified name, in every namespace. -/
def qualifiedFunctionName (unit : ValidatedUnit) (handle : LeanerIR.FunctionHandle) : String :=
  let path := (unit.tables.namespaces[handle.namespaceId.index]?.map (·.segments)).getD #[]
  "::".intercalate (path.push (functionName unit handle)).toList

/-- A function's name relative to a unit: its own name in the unit's module,
namespace 0, and its path-qualified name in another namespace. -/
def functionKey (unit : ValidatedUnit) (handle : LeanerIR.FunctionHandle) : String :=
  if handle.namespaceId.index == 0 then functionName unit handle
  else qualifiedFunctionName unit handle

/-- The prelude model a native is read by, beside what its specification
states: the Move Prover's prelude defines a native whatever its
specification adds. -/
def nativeModelOf? (unit : ValidatedUnit) (handle : LeanerIR.FunctionHandle)
    (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Option NativeModel :=
  if declaration.body == .absent then nativeModel? ns.profile (qualifiedFunctionName unit handle)
  else none

/-- Locate one function of a registered unit by its key: its name in the
unit's module, or its path-qualified name in another namespace. -/
def findFunction? (unit : ValidatedUnit) (key : String) :
    Option (Nat × ValidatedNamespace × Nat ×
      LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) := do
  let parts := (key.splitOn "::").toArray
  let path := parts.pop
  let name := parts.back?.getD key
  for h : namespaceIndex in [0:unit.namespaces.size] do
    let ns := unit.namespaces[namespaceIndex]
    let owned := if path.isEmpty then namespaceIndex == 0 else
      unit.tables.namespaces[ns.identity.index]?.map (·.segments) == some path
    unless owned do continue
    for h : index in [0:ns.functions.size] do
      let declaration := ns.functions[index]
      if let some qualified := unit.tables.names[declaration.name.index]? then
        if qualified.name == name then
          return (namespaceIndex, ns, index, declaration)
  none

/-- Which clauses of a contract hold where: a `[concrete]` clause is proved
of the function's body, an `[abstract]` one is what its callers see of it,
and an unmarked clause is both. -/
inductive ContractView where
  | implementation
  | interface
  deriving BEq

private def markedAs (name : String) (condition : LeanerIR.Condition) : Bool :=
  condition.properties.any fun
    | .assign marker (.constant (.bool true)) _ => marker == name
    | _ => false

/-- Whether callers see a function differently than its body is proved:
some clause is `[abstract]` or `[concrete]`. -/
def hasInterfaceView (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    Bool :=
  declaration.contract.conditions.any fun condition =>
    markedAs "abstract" condition || markedAs "concrete" condition

/-- A declaration with only the clauses a view admits. -/
def ContractView.of (view : ContractView)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody) :
    LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody :=
  let excluded := match view with
    | .implementation => "abstract"
    | .interface => "concrete"
  { declaration with contract := { declaration.contract with
      conditions := declaration.contract.conditions.filter (!markedAs excluded ·) } }

/-- The precondition of a function where it starts, as its caller owes it
at the call: its `requires` clauses in the view its callers see,
over its parameters' values (`locals`, the other locals unbound) and the
memory there (`state`), after the pre-state `let`s they read. Each clause is
marked with its source range, so a precondition not established is
reported there. A clause applying `requires_of` reads the module's table of
declared preconditions, which the start does not take: it is left out, as
the caller's obligation it would be is not stated. -/
def startPrecondition (unit : ValidatedUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (locals : Array (Option Lean.Expr)) (localTypes : Array IrTy) (codecs types : Option Lean.Expr)
    (state : Lean.Expr) (twins : Array SpecTypes.TwinInfo) (executable : Option Lean.Expr) :
    MetaM Lean.Expr := do
  let conditions := (ContractView.interface.of declaration).contract.conditions
  let groups ← groupConditions unit conditions
  let context : Context := {
    unit, namespaceId, ns, locals, oldLocals := locals, localTypes,
    localNames := declaration.locals.map (·.name), results := #[], codecs, types,
    state := some state, oldState := some state, twins, executable }
  let context ← bindLets context groups.lets false
  conjunction (← (conditions.filter fun condition => condition.kind == .requires &&
      !conditionReadsRequires unit namespaceId condition.expression).mapM fun condition => do
    pure (markCondition unit condition (← translate context condition.expression)))

/-- Name of the generated contract definition. -/
def contractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "contract"

/-- Name of the contract over a function's native arguments and result. -/
def typedContractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedContract"

/-- Name of the contract a function's callers see of it, where that differs
from its implementation's. -/
def typedInterfaceContractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedInterfaceContract"

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
  -- A theorem holds of the unit as preparation admits it: a unit the static
  -- checker refuses would make every theorem about it vacuous.
  let failures := LeanerIR.Validation.StaticTyping.unitFailures unit
    (LeanerIR.Validation.targetPointerWidth? unit)
  unless failures.isEmpty do
    throwError m!"the unit is not statically typed, so preparation refuses it:\n\
      {"\n".intercalate failures.toList}"
  -- Nor may preparation refuse its loans.
  let loanErrors := (unit.borrowDiagnostics ++ LeanerIR.Validation.loanDeathDiagnostics unit).filter
    (·.severity == .error)
  unless loanErrors.isEmpty do
    throwError m!"the borrow analysis rejects the unit, so preparation refuses it:\n\
      {"\n".intercalate (loanErrors.toList.map fun d => s!"{d.code}: {d.message}")}"
  liftTermElabM do
    addDecl (.defnDecl {
      name, levelParams := []
      type := mkConst ``LeanerIR.Validation.ValidatedUnit
      value := toExpr unit
      hints := .abbrev, safety := .safe })
    enableRealizationsForConst name
  return name

/-- Prefix of a namespace's compilation artifacts: its compile views, key
maps, type tables, and frames. -/
def semanticsName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "semantics"

/-- Name of the equation evaluating the unit's target pointer width. -/
def pointerWidthEqName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "targetPointerWidth_eq"

/-- Quote the registered unit once per namespace, with the typed twins its
contracts read storage through and the kernel's evaluation of its target
pointer width. The semantics reads the validated unit itself, so there is
no prepared unit to certify. -/
def ensureUnitDefinitions (namespaceSegments : Array String)
    (unit : ValidatedUnit) : CommandElabM Name := do
  let unitDefinition ← ensureUnitDefinition namespaceSegments unit
  let widthEqName := pointerWidthEqName namespaceSegments
  if (← getEnv).contains widthEqName then return widthEqName
  -- An inductive declaration waits for every pending kernel check: the twin
  -- structures come first.
  discard <| SpecTypes.ensureSpecTypes namespaceSegments unit
  let width := LeanerIR.Validation.targetPointerWidth? unit
  liftTermElabM do
    addDecl (.thmDecl {
      name := widthEqName, levelParams := []
      type := ← mkEq (mkApp (mkConst ``LeanerIR.Validation.targetPointerWidth?)
        (mkConst unitDefinition)) (toExpr width)
      value := ← mkEqRefl (toExpr width) })
  return widthEqName

/-- Materialize a registered unit as a Lean definition. -/
syntax (name := leanerUnitCommand) "#leaner_unit" leanerPath : command

@[command_elab leanerUnitCommand]
def elabLeanerUnit : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let segments := pathSegments pathSyntax
  let some unit := LeanerLang.registeredUnit? (← getEnv) (pathName segments)
    | throwErrorAt stx s!"unknown Leaner namespace `{pathName segments}`"
  -- A hand-written proof needs the same quoted unit the generated script
  -- uses, along with the typed twins contracts read storage through.
  discard <| ensureUnitDefinitions segments unit

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
