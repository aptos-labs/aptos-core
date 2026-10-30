-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base
import VerexSL.Elab
import VerexSL.Frame
import VerexSL.Modifiers
import VerexSL.Perf
import VerexSL.Options
import VerexSL.Quote
import VerexSL.Registry
import VerexSL.SpecTypes
import VerexSL.Syntax
import VerexIR.Proofs.Denote.Compile
import VerexIR.Proofs.Invocation
import VerexIR.Proofs.Meaning
import VerexIR.Proofs.Behavior
import VerexIR.Proofs.IntegerArithmetic
import VerexIR.Proofs.Denote.Types
import VerexIR.Proofs.Maps
import VerexIR.Proofs.Denote.NativeMaps

/-!
# Generated contracts: native types and values

The first part of the contract generator (`VerexSL.Contract`): quotation of
native types, typed memory in clauses, intrinsic maps, typed specification
values, and native map owners.
-/

namespace VerexSL.Contract

open Lean Meta Elab Command
open VerexIR (RuntimeValue ConstValue Operation PrimitiveOperation ExprId
  LocalId TypeId)
open VerexIR.Validation (ValidatedUnit ValidatedNamespace)
open VerexIR.Proofs (ObligationRange)
open VerexSL.Quote

/-- Whether a failure is one an `aborts_with` or `aborts_if … with` code
names: an abort with that code, or, for the code `-1` Move gives a runtime
error, an arithmetic or vector error. -/
def abortCodeMatches (failure : VerexIR.Proofs.Failure) (code : Int) : Prop :=
  match failure with
  | (.abort, payload) => payload = #[code]
  | (.profile value, _) =>
      (value = { profile := .move, tag := "runtime.arithmetic_error" } ∨
       value = { profile := .move, tag := "runtime.vector_error" }) ∧ code = -1
  | _ => False

/-- An abort names the code it carries alone. -/
@[simp, vir_denote_norm] theorem abortCodeMatches_abort (values : Array Int) (code : Int) :
    abortCodeMatches (VerexIR.Proofs.Failure.ofInts .abort values) code ↔ values = #[code] :=
  Iff.rfl

/-- A runtime error names the code `-1`. -/
@[simp, vir_denote_norm] theorem abortCodeMatches_profile (value : VerexIR.ProfileValue)
    (values : Array Int) (code : Int) :
    abortCodeMatches (VerexIR.Proofs.Failure.ofInts (.profile value) values) code ↔
      (value = { profile := .move, tag := "runtime.arithmetic_error" } ∨
       value = { profile := .move, tag := "runtime.vector_error" }) ∧ code = -1 := Iff.rfl

/-- A panic names no code. -/
@[simp, vir_denote_norm] theorem abortCodeMatches_panic (values : Array Int) (code : Int) :
    abortCodeMatches (VerexIR.Proofs.Failure.ofInts .panic values) code ↔ False := Iff.rfl

/-- What a state label denotes: a memory, and the value of each local at
that state — a mutable reference parameter's own, the others as they are. -/
structure LabelState where
  memory : Lean.Expr
  locals : Array (Option Lean.Expr)

/-- The shared IR type node; `VerexSL.Ty` is the surface type. -/
abbrev IrTy := VerexIR.Ty

/-- The logical domain a specification value of one VIR type inhabits:
integers as mathematical integers, a Boolean as its truth, a textual scalar
as a string, and every other value natively, as the carrier of its native
type at the clause's frame (`doc/designs/native-verification.md`). -/
inductive Domain where
  | integer
  | boolean
  /-- Addresses, signers, and strings: the strings they hold. -/
  | text
  | native
  deriving BEq, Inhabited

/-- A member of the group of recursive specification functions a
definition is built for, as a call to it reads it: the recursion hypothesis
over its bundles below the definition's argument, its measure, and its
result type. -/
structure RecursiveMember where
  reference : VerexIR.QualifiedRef
  recurse : Lean.Expr
  measure : Lean.Expr
  /-- The member's result type, for the value of a guarded call. -/
  resultType : Lean.Expr
  /-- The member's result domain, and its native type where its result is
  native (`DefinitionShape`). -/
  resultDomain : Domain := .native
  resultNative : Option Lean.Expr := none

/-- The definition of a recursive specification function under
construction: its bundled argument, its measure, and the members of its
group, itself among them. -/
structure RecursiveDefinition where
  reference : VerexIR.QualifiedRef
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
partial def quoteNTy : VerexIR.Proofs.Denote.NTy → MetaM Lean.Expr
  | .unit => return mkConst ``VerexIR.Proofs.Denote.NTy.unit
  | .bool => return mkConst ``VerexIR.Proofs.Denote.NTy.bool
  | .int width signed => return mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.int) #[toExpr width, toExpr signed]
  | .num => return mkConst ``VerexIR.Proofs.Denote.NTy.num
  | .address => return mkConst ``VerexIR.Proofs.Denote.NTy.address
  | .signer => return mkConst ``VerexIR.Proofs.Denote.NTy.signer
  | .string => return mkConst ``VerexIR.Proofs.Denote.NTy.string
  | .bytes => return mkConst ``VerexIR.Proofs.Denote.NTy.bytes
  | .tuple elements => return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.tuple) (← quoteRow elements)
  | .table key value =>
      return mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.table) (← quoteNTy key) (← quoteNTy value)
  | .struct source arguments fields =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.struct)
        #[toExpr source, ← quoteRow arguments, ← quoteRow fields]
  | .enum source arguments names rows _ => do
      let namesExpr := toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.enum)
        #[toExpr source, ← quoteRow arguments, namesExpr, ← quoteRows rows, distinct]
  | .vector element => return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.vector) (← quoteNTy element)
  | .ref referent => return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.ref) (← quoteNTy referent)
  | .ghost slot => return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.ghost) (← quoteNTy slot)
  | .param index => return mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.param) (toExpr index)
  | .function parameters shared results =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.function)
        #[← quoteRow parameters, toExpr shared, ← quoteRow results]

partial def quoteRow : VerexIR.Proofs.Denote.NRow → MetaM Lean.Expr
  | .nil => return mkConst ``VerexIR.Proofs.Denote.NRow.nil
  | .cons τ rest => return mkAppN (mkConst ``VerexIR.Proofs.Denote.NRow.cons) #[← quoteNTy τ, ← quoteRow rest]

partial def quoteRows : VerexIR.Proofs.Denote.NRows → MetaM Lean.Expr
  | .nil => return mkConst ``VerexIR.Proofs.Denote.NRows.nil
  | .cons fields rest => return mkAppN (mkConst ``VerexIR.Proofs.Denote.NRows.cons) #[← quoteRow fields, ← quoteRows rest]
end

/-- The literal of type arguments, with their inhabitability decided. -/
def quoteTypeArgs (θ : VerexIR.Proofs.Denote.TypeArgs) : MetaM Lean.Expr :=
  quoteTypeArgsRow θ.1
where
  /-- The literal of a row of type arguments, its inhabitability decided. -/
  quoteTypeArgsRow (row : VerexIR.Proofs.Denote.NRow) : MetaM Lean.Expr := do
  let row ← quoteRow row
  let inhabitable (row : Lean.Expr) :=
    mkApp3 (mkConst ``Eq [1]) (mkConst ``Bool)
      (mkApp (mkConst ``VerexIR.Proofs.Denote.NRow.inhabitable) row) (mkConst ``Bool.true)
  let predicate := Lean.mkLambda `row .default (mkConst ``VerexIR.Proofs.Denote.NRow)
    (inhabitable (.bvar 0))
  return mkAppN (mkConst ``Subtype.mk [1])
    #[mkConst ``VerexIR.Proofs.Denote.NRow, predicate, row, ← mkDecideProof (inhabitable row)]

def quoteShape : VerexIR.Proofs.Denote.ResultShape → MetaM Lean.Expr
  | .none => return mkConst ``VerexIR.Proofs.Denote.ResultShape.none
  | .one τ => return mkApp (mkConst ``VerexIR.Proofs.Denote.ResultShape.one) (← quoteNTy τ)

/-- The literal of a position in a row. -/
def quoteVar : {Γ : VerexIR.Proofs.Denote.NRow} → {τ : VerexIR.Proofs.Denote.NTy} →
    VerexIR.Proofs.Denote.Var Γ τ → MetaM Lean.Expr
  | .cons τ Γ, _, .here =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.Var.here) #[← quoteRow Γ, ← quoteNTy τ]
  | .cons σ Γ, τ, .there rest =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.Var.there)
        #[← quoteRow Γ, ← quoteNTy σ, ← quoteNTy τ, ← quoteVar rest]

/-- The literal of a variant of an enum. -/
def quoteWhich : {names : List String} → {rows : VerexIR.Proofs.Denote.NRows} →
    {σs : VerexIR.Proofs.Denote.NRow} → VerexIR.Proofs.Denote.Which names rows σs →
    MetaM Lean.Expr
  | name :: names, .cons fields rest, _, .here =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.Which.here)
        #[toExpr name, toExpr names, ← quoteRow fields, ← quoteRows rest]
  | name :: names, .cons fields rest, σs, .there later =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.Which.there)
        #[toExpr name, toExpr names, ← quoteRow fields, ← quoteRow σs, ← quoteRows rest,
          ← quoteWhich later]

/-- The literal of the payload choices of a variant-field selection. -/
def quoteChoices {names : List String} {rows : VerexIR.Proofs.Denote.NRows}
    {τ : VerexIR.Proofs.Denote.NTy} :
    VerexIR.Proofs.Denote.Choices names rows τ → MetaM Lean.Expr
  | .nil =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.Choices.nil)
        #[toExpr names, ← quoteRows rows, ← quoteNTy τ]
  | @VerexIR.Proofs.Denote.Choices.cons _ _ _ σs choice x rest =>
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.Choices.cons)
        #[toExpr names, ← quoteRows rows, ← quoteNTy τ, ← quoteRow σs, ← quoteWhich choice,
          ← quoteVar x, ← quoteChoices rest]

/-! ## Typed memory in clauses

A clause reads global memory as the encoding of the typed value a slot holds
(`doc/designs/static-memory.md`): nothing decodes. -/

/-- The unit a frame belongs to. -/
def frameUnit (frame : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfR (← inferType frame)
  unless type.isAppOfArity ``VerexIR.Proofs.Denote.Skolems 2 do
    throwError "internal: {frame} is not a frame"
  return type.getArg! 0

/-- The unit and its function values a frame belongs to: the leading
arguments of a term at the frame. -/
def frameLeading (frame : Lean.Expr) : MetaM (Array Lean.Expr) :=
  return #[← frameUnit frame, ← VerexIR.Proofs.Denote.functionCarriersOfFrame frame]

/-- The validated unit an executable unit executes, or a model is of
(`ModelUnit`, `doc/designs/ghost-fields.md`). -/
def executableUnit (executable : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfR (← inferType executable)
  unless type.isAppOfArity ``VerexIR.Validation.ExecutableUnit 1 ||
      type.isAppOfArity ``VerexIR.Proofs.ModelUnit 2 do
    throwError "internal: {executable} is not an executable unit"
  return type.getArg! 0

/-- The runtime frame of the unit an executable unit or a model is of. -/
def executableRuntime (executable : Lean.Expr) : MetaM Lean.Expr := do
  let unit ← executableUnit executable
  return mkApp2 (mkConst ``VerexIR.Proofs.Denote.Skolems.runtime) unit
    (← VerexIR.Proofs.Denote.functionCarriersOf unit)

/-- The type of the executable units of a unit. -/
def executableType (unit : Lean.Expr) : Lean.Expr :=
  mkApp (mkConst ``VerexIR.Validation.ExecutableUnit) unit

/-- The type of what a contract reading the unit takes: an executable unit,
or, in a unit verified through its model, a model (`ModelUnit`) at the
unit's function values. -/
def unitReaderType (unit : ValidatedUnit) (unitExpr functions : Lean.Expr) : Lean.Expr :=
  if VerexIR.Proofs.Denote.declaresGhosts unit then
    mkApp2 (mkConst ``VerexIR.Proofs.ModelUnit) unitExpr functions
  else executableType unitExpr

/-- Run `k` over a unit variable and its function values, where `present`. -/
def withOptionalUnit {α : Type} (present : Bool)
    (k : Option Lean.Expr → Option Lean.Expr → MetaM α) : MetaM α :=
  if present then
    withLocalDeclD `unit (mkConst ``ValidatedUnit) fun unit =>
      withLocalDecl `functions .instImplicit
        (mkApp (mkConst ``VerexIR.Proofs.Denote.FunctionCarriers) unit) fun functions =>
          k (some unit) (some functions)
  else k none none

/-- Run `k` over a unit variable and its function values (`FunctionCarriers`,
a local instance). -/
def withUnitVariable {n : Type → Type} [MonadControlT MetaM n] [Monad n] {α : Type}
    (binderInfo : BinderInfo) (k : Lean.Expr → Lean.Expr → n α) : n α :=
  withLocalDecl `unit binderInfo (mkConst ``ValidatedUnit) fun unit =>
    withLocalDecl `functions .instImplicit
      (mkApp (mkConst ``VerexIR.Proofs.Denote.FunctionCarriers) unit) (k unit)

/-- The invocation meaning a model gives at a frame; none where a contract
takes an executable unit, whose invocations mean big-step runs. -/
def modelInvoke? (reader frame : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let type ← whnfR (← inferType reader)
  unless type.isAppOfArity ``VerexIR.Proofs.ModelUnit 2 do return none
  return some (mkApp4 (mkConst ``VerexIR.Proofs.ModelUnit.invoke) (type.getArg! 0) (type.getArg! 1)
    reader frame)

/-- The carriers of a frame. -/
def frameCarriers (frame : Lean.Expr) : MetaM Lean.Expr := do
  return mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) ((← frameLeading frame).push frame)

/-- What a frame's type parameters stand for. -/
def frameTypes (frame : Lean.Expr) : MetaM Lean.Expr := do
  return mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.type) ((← frameLeading frame).push frame)

/-- The resource type a type identifier of a namespace denotes, at a frame,
with its native type. -/
def quoteResource (skolems : Lean.Expr) (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (typeId : TypeId) (typeArguments : Option VerexIR.Proofs.Denote.NRow := none) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let some resource := VerexIR.Proofs.Denote.resourceOf unit namespaceId typeId
    | throwError "a storage clause names a type that is not a resource"
  -- An expanded generic specification function reads the resources of its
  -- type arguments.
  let resource := match typeArguments with
    | some row => resource.subst row
    | none => resource
  let nativeType ← quoteNTy resource.type
  return (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.resource)
    ((← frameLeading skolems) ++ #[skolems, nativeType, ← quoteRow resource.arguments]), nativeType)

/-- A memory's slot at a resource type and a storage key. -/
def memorySlot (memory resource key : Lean.Expr) : Lean.Expr :=
  mkApp2 memory resource key

/-- Whether a memory slot holds a value. -/
def slotPresent (slot : Lean.Expr) : MetaM Lean.Expr := do
  mkEq (← mkAppM ``Option.isSome #[slot]) (mkConst ``Bool.true)

open VerexIR.Proofs.Denote (NRow Weave) in
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
structure StateDefinition where
  label : Nat
  operation : ExprId
  bindings : Array (VerexIR.PatternId × ExprId)

/-- Translation context for one function's specification clauses. -/
structure Context where
  unit : ValidatedUnit
  /-- The namespace owning the specification, for reference resolution. -/
  namespaceId : VerexIR.NamespaceId
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
  /-- The components of a Move tuple result, each as its binder: what
  `spec.result[i]` reads. -/
  resultComponents : Array Lean.Expr := #[]
  /-- Logical referent type of each result binder.  The VIR type carried by
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
  /-- What the family's type parameters stand for (`Skolems.type`), which
  names the type arguments of an opaque specification function. -/
  types : Option Lean.Expr := none
  /-- Sparse declaration-type substitution selected for this invocation. -/
  typeInstantiation : Option Lean.Expr := none
  /-- Derived specification calls are expanded from their already-lowered
  bodies. Keep the active identities so an accidental recursive model is
  rejected instead of recursing in the command elaborator. -/
  specCallStack : Array VerexIR.QualifiedRef := #[]
  /-- The native types of the type arguments of an expanded generic
  specification function, a definition, or a declaration read at an
  instance, in the contract's frame: the values of a type parameter are
  native values of its argument's native type. -/
  typeArgumentTypes : Array (Option VerexIR.Proofs.Denote.NTy) := #[]
  /-- Inside the body of a recursive specification function's definition. -/
  definition : Option RecursiveDefinition := none
  /-- The conditions on the path to the current position, which prove a
  recursive call's measure smaller. -/
  facts : Array Lean.Expr := #[]
  /-- The executable unit, which a behavioral predicate reads a function
  value's meaning from; set where the contract takes it. -/
  executable : Option Lean.Expr := none
  /-- The requires meaning `requires_of` reads (`RequiresMeaning`); set where the
  contract takes it. -/
  requiresMeaning : Option Lean.Expr := none
  /-- Inside a quantifier: a lemma instance there is its implication. -/
  lemmaQuantified : Bool := false
  /-- A stored declaration may read a vector field's physical length without
  observing its elements. This does not license physical equality or content
  observations of Tables hidden in generic fields. -/
  physicalInvariantLengths : Bool := false

/-- The skolem instance the clauses are stated over: the one whose
`Skolems.type` the context's `types` is. -/
def Context.skolems? (context : Context) : Option Lean.Expr := do
  let types ← context.types
  if types.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.type 3 then some types.appArg! else none

/-- The frame a clause's native values are at: the contract's, the runtime
frame where a contract states none; none where a clause has neither, as a
specification function's definition has. -/
def Context.frame? (context : Context) : MetaM (Option Lean.Expr) := do
  if let some frame := context.skolems? then return some frame
  let some executable := context.executable | return none
  return some (← executableRuntime executable)

/-- The frame a clause's storage reads resolve resource types at: the
contract's, the runtime frame where a contract states none. -/
def Context.frame (context : Context) : MetaM Lean.Expr := do
  if let some frame := context.skolems? then return frame
  let some executable := context.executable
    | throwError "internal: a clause reads storage without a frame or a unit"
  executableRuntime executable

/-- The unit a clause's memory and frame are at. -/
def Context.unitExpr (context : Context) : MetaM Lean.Expr := do
  frameUnit (← context.frame)

/-- The unit a clause's memory and frame are at, and its function values. -/
def Context.leading (context : Context) : MetaM (Array Lean.Expr) := do
  frameLeading (← context.frame)

/-- A storage clause's memory slot at a resource type identifier and a
storage key, with the resource type and its native type. -/
def Context.slot (context : Context) (memory : Lean.Expr) (typeId : TypeId)
    (key : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr × Lean.Expr) := do
  let typeArguments ← if context.typeArgumentTypes.isEmpty then pure none else
    let some row := context.typeArgumentTypes.toList.mapM id
      | throwError "a type argument of a specification function reading storage has no \
          native type"
    pure (some (VerexIR.Proofs.Denote.NRow.ofList row))
  let (resource, nativeType) ← quoteResource (← context.frame) context.unit context.namespaceId typeId
    typeArguments
  return (memorySlot memory resource key, resource, nativeType)

/-- The type a specification type denotes, as declared: a type parameter's
values are native values at every instantiation (`Context.ntyOf?`), so that
a declaration's instance substitutes carriers, never domains. -/
def Context.typeOf? (context : Context) (typeId : TypeId) : Option IrTy :=
  context.unit.tables.types[typeId.index]?

/-- The type a specification reads a value of type `typeId` at:
specifications see through references, so a reference's values are its
referent's. -/
partial def specTypeId (unit : ValidatedUnit) (typeId : TypeId) : TypeId :=
  match unit.tables.types[typeId.index]? with
  | some (.reference reference) => specTypeId unit reference.referent
  | _ => typeId

/-- The type of a specification value, through references. -/
def Context.valueTypeOf? (context : Context) (typeId : TypeId) : Option IrTy :=
  context.typeOf? (specTypeId context.unit typeId)

/-- The native type of a specification type, with the parameters replaced
by the expansion's type arguments. -/
def Context.ntyOf? (context : Context) (typeId : TypeId) :
    Option VerexIR.Proofs.Denote.NTy := do
  let ty ← context.unit.tables.types[typeId.index]?
  match ty with
  | .typeParameter index =>
      -- The family's own parameter outside an expansion, the expansion's
      -- type argument inside one.
      if context.typeArgumentTypes.isEmpty then some (.param index)
      else (context.typeArgumentTypes[index]?).join
  | _ =>
      let nty ← VerexIR.Proofs.Denote.ntyOf context.unit context.namespaceId typeId
      if context.typeArgumentTypes.isEmpty then some nty else
        let row ← context.typeArgumentTypes.toList.mapM id
        some (nty.subst (VerexIR.Proofs.Denote.NRow.ofList row))

def domainOf (ty : IrTy) : Domain :=
  match ty with
  | .integer _ _ => .integer
  | .bool => .boolean
  | .string | .address | .signer => .text
  | _ => .native

/-- The domain of a type of a declaration's signature, through references;
`role` names the position in the error for an unknown type. -/
def signatureDomain (unit : ValidatedUnit) (typeId : TypeId) (role : String) :
    MetaM Domain := do
  let some ty := unit.tables.types[(specTypeId unit typeId).index]?
    | throwError "{role} has an unknown type"
  pure (domainOf ty)

/-- The decision of a proposition: its instance where one exists, otherwise
the classical one, as for a quantifier over an unbounded domain or an
equation of native values. -/
private def decisionOf (proposition : Lean.Expr) : MetaM Lean.Expr := do
  match ← trySynthInstance (mkApp (mkConst ``Decidable) proposition) with
  | .some decision => pure decision
  | _ => pure (mkApp (mkConst ``Classical.propDecidable) proposition)

/-- A proposition decided as a Boolean (`decisionOf`). -/
def decideProposition (proposition : Lean.Expr) : MetaM Lean.Expr := do
  return mkApp2 (mkConst ``Decidable.decide) proposition (← decisionOf proposition)

def conjunction (parts : Array Lean.Expr) : MetaM Lean.Expr := do
  match parts.toList with
  | [] => return mkConst ``True
  | head :: tail =>
      tail.foldlM (fun accumulated part => mkAppM ``And #[accumulated, part]) head

/-- A clause's conditional: `ite` on the decided test (`decisionOf`). -/
def conditional (test thenTerm elseTerm : Lean.Expr) (type? : Option Lean.Expr := none) :
    MetaM Lean.Expr := do
  mkAppOptM ``ite #[type?, test, some (← decisionOf test), thenTerm, elseTerm]

/-- The values of a bounded integer type, as a binder's membership. -/
def boundedMembership? : IrTy → Option (Lean.Expr → MetaM Lean.Expr)
  | .integer (.bits width) signed => some fun value => pure (mkApp3
      (mkConst ``VerexIR.IntegerValueFits)
      (mkApp (mkConst ``VerexIR.IntWidth.bits) (toExpr width)) (toExpr signed) value)
  | _ => none

/-- The Lean type of a binder of a scalar domain: integers `Int`, a Boolean
`Bool`, a textual scalar `String`; a native value's is its carrier
(`Context.binderType`). -/
def Domain.scalarType? : Domain → Option Lean.Expr
  | .integer => some (mkConst ``Int)
  | .boolean => some (mkConst ``Bool)
  | .text => some (mkConst ``String)
  | .native => none

/-- The clause-level term a binder denotes.  A clause is a proposition, so a
boolean reads as its truth; every other domain reads as itself. -/
def Domain.ofBinder (domain : Domain) (binder : Lean.Expr) :
    MetaM Lean.Expr := do
  match domain with
  | .boolean => mkEq binder (mkConst ``Bool.true)
  | _ => return binder

/-- Every carrier has decidable equality, so a clause can decide a
proposition about carrier values, as when a Boolean argument of a typed
contract is passed to an expanded specification function. -/
instance [VerexIR.Proofs.Denote.Carriers] (τ : VerexIR.Proofs.Denote.NTy) :
    DecidableEq τ.carrier :=
  τ.decEq

/-- A type argument of a specification function: a native type, or the
mathematical integers (`num`), which no native type is. -/
inductive SpecTypeArgument where
  | native (type : VerexIR.Proofs.Denote.NTy)
  | integer

/-- The meaning of a specification function without a body: a fixed
function about which nothing is known, named by the declaration's qualified
spelling and its type arguments, of its arguments' native values bundled as
right-nested pairs. -/
opaque opaqueSpec (name : String) (typeArguments : List SpecTypeArgument)
    (result : Type) [Inhabited result] {arguments : Type} (bundle : arguments) : result

/-- The value an order chooses among the less, equal, and greater values: a
three-variant ordering enum's native value at an order, as
`std::cmp::compare` returns it (`orderingVariant`). -/
def orderingChoice {α : Type} (less equal greater : α) : Ordering → α
  | .lt => less
  | .eq => equal
  | .gt => greater

@[simp, vir_denote_norm] theorem orderingChoice_lt {α : Type} (less equal greater : α) :
    orderingChoice less equal greater .lt = less := rfl
@[simp, vir_denote_norm] theorem orderingChoice_eq {α : Type} (less equal greater : α) :
    orderingChoice less equal greater .eq = equal := rfl
@[simp, vir_denote_norm] theorem orderingChoice_gt {α : Type} (less equal greater : α) :
    orderingChoice less equal greater .gt = greater := rfl

/-- A value an order chooses is a given value exactly at the orders choosing
it: stated before the order is read, as for `orderingVariant`. -/
@[vir_denote_norm ↓] theorem orderingChoice_eq_iff {α : Type} (less equal greater value : α)
    (order : Ordering) :
    orderingChoice less equal greater order = value ↔
      (order = .lt ∧ less = value) ∨ (order = .eq ∧ equal = value) ∨
        (order = .gt ∧ greater = value) := by
  cases order <;> simp [orderingChoice]

/-- The variant a chosen ordering value holds is the chosen variant's. -/
theorem variantName_orderingChoice {unit : ValidatedUnit} [VerexIR.Proofs.Denote.FunctionCarriers unit]
    [VerexIR.Proofs.Denote.Skolems unit] (names : List String) (rows : VerexIR.Proofs.Denote.NRows)
    (less equal greater : VerexIR.Proofs.Denote.variantCarrier names rows) (order : Ordering) :
    VerexIR.Proofs.Denote.variantName names rows (orderingChoice less equal greater order) =
      orderingChoice (VerexIR.Proofs.Denote.variantName names rows less)
        (VerexIR.Proofs.Denote.variantName names rows equal)
        (VerexIR.Proofs.Denote.variantName names rows greater) order := by
  cases order <;> rfl

/-- `variantName_orderingChoice`, instantiated at the term's own instance: the
decided inhabitation of an induced family's type arguments defeats simp's
matching of the lemma. -/
simproc [vir_denote_norm] variantNameOrderingChoice
    (@VerexIR.Proofs.Denote.variantName ?carriers _ _ (orderingChoice _ _ _ _)) := fun e => do
  let_expr VerexIR.Proofs.Denote.variantName carriers names rows value := e
    | return .continue
  let_expr VerexIR.Proofs.Denote.Skolems.toCarriers unit functions instance_ := carriers.consumeMData
    | return .continue
  let_expr VerexSL.Contract.orderingChoice _ less equal greater order := value | return .continue
  let proof := mkAppN (mkConst ``VerexSL.Contract.variantName_orderingChoice)
    #[unit, functions, instance_, names, rows, less, equal, greater, order]
  let some (_, _, chosen) := (← inferType proof).eq? | return .continue
  return .visit { expr := chosen, proof? := some (← mkExpectedTypeHint proof (← mkEq e chosen)) }

def typeOfExpr? (context : Context) (id : ExprId) : Option IrTy := do
  let expression ← context.ns.expressions[id.index]?
  context.valueTypeOf? expression.typeId

def describeType (ty : IrTy) : String :=
  toString (repr ty)

/-- The runtime identity of the nominal declaration an operation names. -/
def structHandleOf (context : Context) (reference : VerexIR.QualifiedRef) :
    MetaM VerexIR.StructHandle := do
  let some handle := VerexIR.SemanticOperations.resolveStruct?
      context.unit context.namespaceId reference
    | throwError "a nominal operation's declaration does not resolve"
  return handle

/-- The declaration of a nominal type, with its runtime identity and owning
namespace. -/
def nominalDeclaration? (unit : ValidatedUnit) (ty : IrTy) :
    Option (VerexIR.StructHandle × ValidatedNamespace × VerexIR.StructDecl) := do
  let .nominal name _ := ty | none
  let qualified ← unit.tables.names[name.index]?
  let ns ← unit.namespaces[qualified.namespaceId.index]?
  let structId ← ns.structs.findIdx? (·.name == name)
  let declaration ← ns.structs[structId]?
  some ({ namespaceId := qualified.namespaceId, structId }, ns, declaration)

/-! ## Native values

Every specification value a clause holds is native
(`doc/designs/native-verification.md`): an integer as a mathematical integer, a
Boolean as its truth, a textual scalar as a string, and every other value as
the carrier of its native type at the clause's frame. The native type of a
value is its specification type's (`Context.nativeType`). -/

/-- The carriers native values are at: the clause's frame's. -/
def Context.carriers (context : Context) : MetaM Lean.Expr := do
  frameCarriers (← context.frame)

/-- The native type of a specification type. -/
def Context.nativeType (context : Context) (typeId : TypeId) :
    MetaM VerexIR.Proofs.Denote.NTy := do
  let some nty := context.ntyOf? (specTypeId context.unit typeId)
    | throwError "a specification value of type {describeType ((context.valueTypeOf? typeId).getD .unit)} \
        has no native type"
  pure nty

/-- The native type of an expression's value. -/
def Context.exprType (context : Context) (id : ExprId) : MetaM VerexIR.Proofs.Denote.NTy := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "specification expression {id.index} is out of range"
  context.nativeType expression.typeId

/-- The Lean type of the native values of a quoted type at a family. -/
def carrierType (carriers τ : Lean.Expr) : Lean.Expr :=
  mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) carriers τ

/-- The Lean type of a binder of a specification type: an integer's `Int`, a
Boolean's `Bool`, a textual scalar's `String`, any other's carrier. -/
def Context.binderType (context : Context) (typeId : TypeId) : MetaM Lean.Expr := do
  let some ty := context.valueTypeOf? typeId
    | throwError "a specification binder has an unknown type"
  match (domainOf ty).scalarType? with
  | some type => pure type
  | none => return carrierType (← context.carriers) (← quoteNTy (← context.nativeType typeId))

/-- A native value as a clause reads it: an integer's value, a Boolean's
truth, any other value itself. -/
def Domain.ofNative (domain : Domain) (value : Lean.Expr) : MetaM Lean.Expr :=
  match domain with
  | .integer => mkAppM ``VerexIR.SpecInt.val #[value]
  -- At `Bool`, the clause-level type, not the carrier that unfolds to it.
  | .boolean => pure (mkApp3 (mkConst ``Eq [Level.one]) (mkConst ``Bool) value (mkConst ``Bool.true))
  | .text | .native => pure value

/-- A native value as a local of its domain holds it: an integer's value,
any other value itself (`Domain.ofBinder` reads it). -/
def Domain.binderOfNative (domain : Domain) (value : Lean.Expr) : MetaM Lean.Expr :=
  match domain with
  | .integer => mkAppM ``VerexIR.SpecInt.val #[value]
  | .boolean | .text | .native => pure value

/-- The inhabitant of a native type whose inhabitability evaluates: the
value a read outside a vector or of an absent resource stands for. -/
def typedInhabitant? (carriers τ : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let inhabitable := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) τ
  unless ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) do return none
  let proof ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq inhabitable (mkConst ``Bool.true))
  return some (mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitant) carriers τ proof)

/-- The inhabitant of a native type, where a construct needs one. -/
def typedInhabitant (carriers τ : Lean.Expr) : MetaM Lean.Expr := do
  let some inhabitant ← typedInhabitant? carriers τ
    | throwError "the native type {τ} has no value a specification can stand for"
  pure inhabitant

/-- The decided inhabitability of a quoted native type. -/
def inhabitableProof (τ : Lean.Expr) : MetaM Lean.Expr := do
  let inhabitable := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) τ
  unless ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) do
    throwError "the native type {τ} has no values"
  mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq inhabitable (mkConst ``Bool.true))

/-- The clause form of a native value of a type: an integer's value, a
Boolean's truth, any other value itself, as `nativeOfClause` reads it back. -/
def clauseOfNative (τ : VerexIR.Proofs.Denote.NTy) (value : Lean.Expr) :
    MetaM Lean.Expr :=
  match τ with
  | .int _ _ | .num => mkAppM ``VerexIR.SpecInt.val #[value]
  | .bool => mkEq value (mkConst ``Bool.true)
  | _ => pure value

/-- A clause value as a native value of a type: an integer's certified value
(the value a clause reads of one, a literal that fits, otherwise the integer
cut to the type, `SpecInt.ofInt`), a Boolean's decision, any other value
itself. An integer cut by `cut`, a clause's by `SpecInt.ofInt`. A native
value, as a clause reads a type parameter's at every instance, is itself. -/
def nativeOfClause (carriers : Lean.Expr) (τ : VerexIR.Proofs.Denote.NTy) (clause : Lean.Expr)
    (cut : Name := ``VerexIR.SpecInt.ofInt) : MetaM Lean.Expr := do
  let bare := clause.consumeMData
  if τ matches .int _ _ | .num | .bool then
    if ← isDefEq (← inferType clause) (carrierType carriers (← quoteNTy τ)) then return clause
  match τ with
  | .int width signed =>
      let carrier := carrierType carriers (← quoteNTy τ)
      if bare.isAppOfArity ``VerexIR.SpecInt.val 3 then
        let value := bare.getArg! 2
        if ← isDefEq (← inferType value) carrier then return value
      let widthExpr := toExpr (VerexIR.IntWidth.bits width)
      let certified (literal : Int) : MetaM (Option Lean.Expr) := do
        unless VerexIR.Ty.integerValueFits? (.integer (.bits width) signed) literal == some true do
          return none
        let fits ← VerexIR.Proofs.Denote.decidedLemma (← mkAppM ``VerexIR.IntegerValueFits
          #[widthExpr, toExpr signed, toExpr literal])
        return some (mkAppN (mkConst ``VerexIR.SpecInt.mk) #[widthExpr, toExpr signed, toExpr literal, fits])
      let certifiedFits (literal : Int) : MetaM (Option Lean.Expr) := do
        unless VerexIR.Ty.integerValueFits? (.integer (.bits width) signed) literal == some true do
          return none
        some <$> VerexIR.Proofs.Denote.decidedLemma (← mkAppM ``VerexIR.IntegerValueFits
          #[widthExpr, toExpr signed, toExpr literal])
      if let some literal := bare.int? then
        if let some value ← certified literal then return value
      -- A value the context certifies: a bounded quantifier's membership, or
      -- a range's whose ends fit.
      let fitsType ← mkAppM ``VerexIR.IntegerValueFits #[widthExpr, toExpr signed, clause]
      let certify (fits : Lean.Expr) : Lean.Expr :=
        mkAppN (mkConst ``VerexIR.SpecInt.mk) #[widthExpr, toExpr signed, clause, fits]
      if let some fits ← findLocalDeclWithType? fitsType then
        return certify (.fvar fits)
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let some (low, high) := (← instantiateMVars decl.type).app2? ``And | continue
        unless low.isAppOfArity ``LE.le 4 && high.isAppOfArity ``LT.lt 4 do continue
        unless (low.getArg! 0).isConstOf ``Int && low.appArg! == clause &&
            high.getArg! 2 == clause do continue
        let (some lower, some upper) := ((low.getArg! 2).int?, high.appArg!.int?) | continue
        let some lowerFits := ← certifiedFits lower | continue
        let some lastFits := ← certifiedFits (upper - 1) | continue
        return certify (← mkAppM ``VerexIR.IntegerValueFits.of_range
          #[decl.toExpr, lowerFits, lastFits])
      let default ← match ← certified 0 with
        | some zero => pure zero
        | none => typedInhabitant carriers (← quoteNTy τ)
      return mkAppN (mkConst cut) #[widthExpr, toExpr signed, default, clause]
  | .num =>
      if bare.isAppOfArity ``VerexIR.SpecInt.val 3 then
        let value := bare.getArg! 2
        if ← isDefEq (← inferType value) (carrierType carriers (← quoteNTy τ)) then return value
      let unbounded := mkConst ``VerexIR.IntWidth.unbounded
      let fits ← mkExpectedTypeHint (← mkEqRefl (← mkAppM ``Option.some #[mkConst ``Bool.true]))
        (← mkAppM ``VerexIR.IntegerValueFits #[unbounded, mkConst ``Bool.true, clause])
      return mkAppN (mkConst ``VerexIR.SpecInt.mk) #[unbounded, mkConst ``Bool.true, clause, fits]
  | .bool =>
      if bare.isAppOfArity ``Eq 3 && (bare.getArg! 0).isConstOf ``Bool &&
          (bare.getArg! 2).isConstOf ``Bool.true then
        return bare.getArg! 1
      decideProposition clause
  | _ => pure clause

/-- A proposition with each specification integer it cuts to a fixed width
(`SpecInt.ofInt`) bound to the certified integer of that value, innermost
first: `∃ x, x.val = e ∧ p x`. An integer out of the width's range is no
value of it, so the proposition does not hold of one, as the Move Prover
reads an unbounded value against fixed-width ones. A cut under a binder of
the proposition is left to the clause that binds it. The unit's check of a
literal function value it reads (a pending proof, `.call (.closure …)`,
`SiteTyped.ofCheck`) is bound alike: `∃ checked, p checked`. -/
partial def bindCuts (proposition : Lean.Expr) : MetaM Lean.Expr := do
  let isCut (e : Lean.Expr) : Bool := e.isAppOfArity ``VerexIR.SpecInt.ofInt 4 && !e.hasLooseBVars
  let some cut := proposition.find? fun e => isCut e && ((e.getArg! 3).find? isCut).isNone
    | bindTypings proposition
  let value := cut.getArg! 3
  withLocalDeclD `cut (← inferType cut) fun named => do
    let body ← bindCuts (proposition.replace fun e => if e == cut then some named else none)
    let definition ← mkEq (← mkAppM ``VerexIR.SpecInt.val #[named]) value
    mkAppM ``Exists #[← mkLambdaFVars #[named] (← mkAppM ``And #[definition, body])]
where
  /-- The proposition with each pending check of a literal function value
  bound. -/
  bindTypings (proposition : Lean.Expr) : MetaM Lean.Expr := do
    let typings ← (← getMVars proposition).filterM fun mvarId => do
      let type ← instantiateMVars (← mvarId.getType)
      return type.isAppOfArity ``Eq 3 &&
        (type.getArg! 1).isAppOf ``VerexIR.Proofs.Denote.closureTypingCheck
    let some typing := typings[0]? | return proposition
    withLocalDeclD `checked (← instantiateMVars (← typing.getType)) fun typed => do
      let body ← bindTypings (proposition.replace fun e =>
        if e.isMVar && e.mvarId! == typing then some typed else none)
      mkAppM ``Exists #[← mkLambdaFVars #[typed] body]

/-- The storage key a native key value keys memory by: an address or a
signer by its address, as the denotation keys it (`NTy.storageKey`). -/
def storageKeyOf (carriers : Lean.Expr) (τ : VerexIR.Proofs.Denote.NTy) (key : Lean.Expr) :
    MetaM Lean.Expr := do
  match τ with
  | .address | .signer => return mkApp (mkConst ``VerexIR.StorageKey.address) key
  | _ => return mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.storageKey) carriers (← quoteNTy τ) key

/-- The element types of a quoted row. -/
partial def quotedRowTypes (row : Lean.Expr) : Array Lean.Expr :=
  if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 then
    #[row.getArg! 0] ++ quotedRowTypes (row.getArg! 1)
  else #[]

/-- The components of a native row value at a family's carriers. -/
def quotedRowProjections (carriers value : Lean.Expr) (types : Array Lean.Expr) :
    Array Lean.Expr := Id.run do
  let mut rest := value
  let mut values := #[]
  for index in [:types.size] do
    let tail := (types.extract (index + 1) types.size).foldr
      (fun ty row => mkApp2 (mkConst ``VerexIR.Proofs.Denote.NRow.cons) ty row)
      (mkConst ``VerexIR.Proofs.Denote.NRow.nil)
    let carrier := carrierType carriers types[index]!
    let tailType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) carriers tail
    values := values.push
      (mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) carrier tailType rest)
    rest := mkApp3 (mkConst ``Prod.snd [Level.zero, Level.zero]) carrier tailType rest
  return values

/-- The row of a native struct or tuple type, quoted. -/
def nativeRowOf? (τ : Lean.Expr) : Option Lean.Expr :=
  let τ := τ.consumeMData
  if τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 then some (τ.getArg! 2)
  else if τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.tuple 1 then some (τ.getArg! 0)
  else none

/-- The components of a native struct or tuple value: per component, its
native type and value. -/
def nativeComponents (carriers τ value : Lean.Expr) : MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let some row := nativeRowOf? τ | throwError "internal: {τ} is not a row type"
  let types := quotedRowTypes row
  return types.zip (quotedRowProjections carriers value types)

/-- The unit, its function values, and the frame of a family's carriers
(`Skolems.toCarriers`). -/
def carriersFrame? (carriers : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr) :=
  let carriers := carriers.consumeMData
  if carriers.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.toCarriers 3 then
    some (carriers.getArg! 0, carriers.getArg! 1, carriers.getArg! 2)
  else none

/-- The unit, its function values, and the frame of a family's carriers,
which every clause's are. -/
def carriersFrame (carriers : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr × Lean.Expr) := do
  let some frame := carriersFrame? carriers
    | throwError "internal: the carriers {carriers} are not a frame's"
  pure frame

/-- Whether a native enum value holds one of the tested variants, as the
program tests it (`Term.isVariant`). -/
def typedVariantTest? (carriers τ value : Lean.Expr) (handle : VerexIR.StructHandle)
    (variants : Array String) : MetaM (Option Lean.Expr) := do
  unless τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 && τ.getArg! 0 == toExpr handle do
    return none
  let name := mkAppN (mkConst ``VerexIR.Proofs.Denote.variantName)
    #[carriers, τ.getArg! 2, τ.getArg! 3, value]
  let tests ← mkListLit (mkConst ``String) (variants.toList.map toExpr)
  return some (mkApp2 (mkConst ``VerexIR.Proofs.Denote.namedIn) name tests)

/-- That a native enum value holds one of the tested variants. -/
def nativeVariantTest (carriers τ value : Lean.Expr) (handle : VerexIR.StructHandle)
    (variants : Array String) : MetaM Lean.Expr := do
  let some test ← typedVariantTest? carriers τ value handle variants
    | throwError "internal: a variant test of a value whose type {τ} is not the tested enum's"
  mkEq test (mkConst ``Bool.true)

/-- A variant field of a native enum value: its variant's field, as the
program reads it (`Term.payload`), and the field type's inhabitant under a
variant without it. The field's native type and the read; none where the
selection does not resolve at the value's type. -/
def typedVariantField? (enum : VerexIR.Proofs.Denote.NTy) (choices : Array (String × Nat))
    (carriers τ value : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let .enum _ _ names rows _ := enum | return none
  unless ← isDefEq (← quoteNTy enum) τ do return none
  let some (variant, index) := choices[0]? | return none
  let some ⟨σs, _⟩ := VerexIR.Proofs.Denote.Which.ofName names rows variant | return none
  let some ⟨field, _⟩ := VerexIR.Proofs.Denote.Var.ofIndex σs index | return none
  let .ok selection := VerexIR.Proofs.Denote.Choices.ofList names rows field choices.toList
    | return none
  let some (unit, functions, frame) := carriersFrame? carriers | return none
  let fieldType ← quoteNTy field
  let some inhabitant ← typedInhabitant? carriers fieldType | return none
  let selected := mkAppN (mkConst ``VerexIR.Proofs.Denote.Choices.select?)
    #[unit, functions, frame, τ.getArg! 2, τ.getArg! 3, fieldType, ← quoteChoices selection, value]
  let carrier := carrierType carriers fieldType
  return some (fieldType,
    mkAppN (mkConst ``Option.getD [Level.zero]) #[carrier, selected, inhabitant])

/-- A variant field of a native enum value as the selection reads it: some
value under its variant, none under another (`Choices.select?`). The field's
native type and the selection; none where the selection does not resolve at
the value's type. -/
def typedVariantSelection? (enum : VerexIR.Proofs.Denote.NTy) (choices : Array (String × Nat))
    (carriers τ value : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let .enum _ _ names rows _ := enum | return none
  unless ← isDefEq (← quoteNTy enum) τ do return none
  let some (variant, index) := choices[0]? | return none
  let some ⟨σs, _⟩ := VerexIR.Proofs.Denote.Which.ofName names rows variant | return none
  let some ⟨field, _⟩ := VerexIR.Proofs.Denote.Var.ofIndex σs index | return none
  let .ok selection := VerexIR.Proofs.Denote.Choices.ofList names rows field choices.toList
    | return none
  let some (unit, functions, frame) := carriersFrame? carriers | return none
  let fieldType ← quoteNTy field
  return some (fieldType, mkAppN (mkConst ``VerexIR.Proofs.Denote.Choices.select?)
    #[unit, functions, frame, τ.getArg! 2, τ.getArg! 3, fieldType, ← quoteChoices selection, value])

/-- A variant field of a native enum value, where the selection resolves. -/
def nativeVariantField (enum : VerexIR.Proofs.Denote.NTy) (choices : Array (String × Nat))
    (carriers τ value : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  let some read ← typedVariantField? enum choices carriers τ value
    | throwError "a variant field of a type without a default value, such as a function \
        type, is read outside a `let` of a Boolean clause, which generated contracts do not \
        carry (at {τ})"
  pure read

/-- The native row of clause values, a nested pair. -/
partial def nativeRow (carriers : Lean.Expr) :
    VerexIR.Proofs.Denote.NRow → List Lean.Expr → MetaM Lean.Expr
  | .nil, [] => return mkApp (mkConst ``VerexIR.Proofs.Denote.HList.nil) carriers
  | .cons τ rest, operand :: operands => do
      let value ← nativeOfClause carriers τ operand
      let tail ← nativeRow carriers rest operands
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.HList.cons)
        #[carriers, ← quoteNTy τ, ← quoteRow rest, value, tail]
  | _, _ => throwError "internal: a row of values differs from its row type"

/-- The value of a ghost slot of a value a specification builds:
unspecified, as the Move Prover leaves a ghost field no `update` wrote
(`doc/designs/ghost-fields.md`). -/
opaque unspecifiedGhost (slot : VerexIR.Proofs.Denote.NTy) (carrier : Type) [Inhabited carrier] :
    carrier

/-- The unspecified value of each ghost slot that follows `count` declared
fields in a row. -/
def ghostValues (carriers : Lean.Expr) (row : VerexIR.Proofs.Denote.NRow) (count : Nat) :
    MetaM (List Lean.Expr) := do
  let slots := (row.toList.drop count)
  slots.mapM fun slot => do
    let .ghost contents := slot
      | throwError "internal: a built value's row has a field after its declared fields that is \
          not a ghost slot"
    let τ ← quoteNTy slot
    let carrier := carrierType carriers τ
    let inhabitant ← typedInhabitant carriers (← quoteNTy contents)
    return mkAppN (mkConst ``VerexSL.Contract.unspecifiedGhost)
      #[← quoteNTy contents, carrier, mkApp2 (mkConst ``Inhabited.mk [1]) carrier inhabitant]

/-- A value a specification builds of clause operands, as the program builds
it (`Term.pack`, `Term.variant`), its ghost slots unspecified. -/
def nativeConstructed (built : VerexIR.Proofs.Denote.NTy) (variant : Option String)
    (carriers : Lean.Expr) (operands : Array Lean.Expr) : MetaM Lean.Expr := do
  let (unit, functions, frame) ← carriersFrame carriers
  let builtType ← quoteNTy built
  match built, variant with
  | .struct _ _ fields, none =>
      let ghosts ← ghostValues carriers fields operands.size
      nativeRow carriers fields (operands.toList ++ ghosts)
  | .enum _ _ names rows _, some name =>
      let some ⟨σs, which⟩ := VerexIR.Proofs.Denote.Which.ofName names rows name
        | throwError "internal: the variant `{name}` is not the enum's"
      let ghosts ← ghostValues carriers σs operands.size
      let row ← nativeRow carriers σs (operands.toList ++ ghosts)
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.Which.inject)
        #[unit, functions, frame, builtType.getArg! 2, builtType.getArg! 3, ← quoteRow σs, ← quoteWhich which, row]
  | _, _ => throwError "internal: a constructor builds a value that is neither a struct nor an enum"

/-- The values of a call as the bundle an uninterpreted function takes:
right-nested pairs closed by `()`. -/
def bundleOf (values : Array Lean.Expr) : MetaM Lean.Expr :=
  values.foldrM (fun value rest => mkAppM ``Prod.mk #[value, rest]) (mkConst ``Unit.unit)

/-- An uninterpreted value of a Lean type: the specification function `name`
at type arguments and a bundle of native arguments (`opaqueSpec`), or the
meaning the prelude of `profile` gives the function
(`specificationModel?`). -/
def opaqueValue (name : String) (typeArguments : List Lean.Expr) (result inhabited : Lean.Expr)
    (arguments : Array Lean.Expr) (profile : Option VerexIR.Profile := none) : MetaM Lean.Expr := do
  let typeArguments ← mkListLit (mkConst ``VerexSL.Contract.SpecTypeArgument) typeArguments
  if let some .serialization := specificationModel? profile name then
    return ← mkAppOptM ``VerexIR.Proofs.serializedBytes #[none, typeArguments, none, ← bundleOf arguments]
  mkAppOptM ``VerexSL.Contract.opaqueSpec
    #[toExpr name, typeArguments, result, inhabited, none, ← bundleOf arguments]

/-- An unspecified value of a native type at a frame, as an aborting
specification branch denotes one (`arbitraryValue`): one fixed value of the
type's carrier. -/
def nativeArbitrary (carriers : Lean.Expr) (nty : VerexIR.Proofs.Denote.NTy) (name : String)
    (arguments : Array Lean.Expr) : MetaM Lean.Expr := do
  let τ ← quoteNTy nty
  let inhabitant ← typedInhabitant carriers τ
  let carrier := carrierType carriers τ
  opaqueValue name [] carrier (mkApp2 (mkConst ``Inhabited.mk [1]) carrier inhabitant) arguments

/-- The native value a memory slot holds, read at a frame; the type's
inhabitant where it holds nothing. -/
def nativeSlot (skolems nativeType resource slot : Lean.Expr) : MetaM Lean.Expr := do
  let carriers ← frameCarriers skolems
  let inhabitant ← typedInhabitant carriers nativeType
  let leading ← frameLeading skolems
  let carrier := mkAppN (mkConst ``VerexIR.Proofs.Denote.ResourceType.carrier) (leading.push resource)
  let frameCarrier := carrierType carriers nativeType
  let read ← withLocalDeclD `value carrier fun value =>
    mkLambdaFVars #[value] (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.ofRuntime)
      (leading ++ #[skolems, nativeType, value]))
  let mapped := mkAppN (mkConst ``Option.map [Level.zero, Level.zero])
    #[carrier, frameCarrier, read, slot]
  return mkAppN (mkConst ``Option.getD [Level.zero]) #[frameCarrier, mapped, inhabitant]

/-- The value of a three-variant ordering enum at an order, as
`std::cmp::compare` returns it (`orderingChoice`). -/
def nativeOrderingValue (carriers : Lean.Expr) (unit : ValidatedUnit) (owner : VerexIR.StructHandle)
    (variants : Array String) (order : Lean.Expr) : MetaM Lean.Expr := do
  let some enum@(.enum _ _ names rows _) := VerexIR.Proofs.Denote.structNTy unit owner
    | throwError "the result of a structural order is not a native enum"
  let (unitExpr, functionsExpr, frameExpr) ← carriersFrame carriers
  let τ ← quoteNTy enum
  let namesExpr := τ.getArg! 2
  let rowsExpr := τ.getArg! 3
  let values ← variants.mapM fun name => do
    let some ⟨.nil, which⟩ := VerexIR.Proofs.Denote.Which.ofName names rows name
      | throwError "the variant `{name}` of an ordering carries fields"
    return mkAppN (mkConst ``VerexIR.Proofs.Denote.Which.inject)
      #[unitExpr, functionsExpr, frameExpr, namesExpr, rowsExpr, mkConst ``VerexIR.Proofs.Denote.NRow.nil,
        ← quoteWhich which, mkApp (mkConst ``VerexIR.Proofs.Denote.HList.nil) carriers]
  let #[less, equal, greater] := values
    | throwError "the result of a structural order has three variants"
  return mkAppN (mkConst ``VerexSL.Contract.orderingChoice)
    #[carrierType carriers τ, less, equal, greater, order]

/-- The structural order of two native values of a type, as
`std::cmp::compare` and the `compare` primitive observe it (`NTy.order`). -/
def structuralOrderTerm (context : Context) (τ : VerexIR.Proofs.Denote.NTy) (left right : Lean.Expr) :
    MetaM Lean.Expr := do
  let rank := mkApp (mkConst ``VerexIR.SemanticOperations.valueRanks)
    (← quoteValueOrders context.ns.orders)
  return mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.order)
    #[← context.carriers, rank, ← quoteNTy τ, left, right]

/-- The value of the ordering enum `ty` at an order. -/
def orderingTerm (context : Context) (ty : IrTy) (order : Lean.Expr) : MetaM Lean.Expr := do
  let some (owner, _, declaration) := nominalDeclaration? context.unit ty
    | throwError "a structural order returns an ordering enum"
  let names := declaration.variants.map fun variant =>
    ((context.unit.tables.names[variant.name.index]?).map (·.name)).getD ""
  nativeOrderingValue (← context.carriers) context.unit owner names order

/-- A native struct value with one field replaced, as the program assigns it
(`Var.update`). -/
def nativeFieldUpdate (carriers : Lean.Expr) (built : VerexIR.Proofs.Denote.NTy) (index : Nat)
    (base replacement : Lean.Expr) : MetaM Lean.Expr := do
  let .struct _ _ fields := built | throwError "internal: a field update of a value that is not a struct"
  let some ⟨σ, x⟩ := VerexIR.Proofs.Denote.Var.ofIndex fields index
    | throwError "internal: a field update's field is out of range"
  let (unit, functions, frame) ← carriersFrame carriers
  let replaced ← nativeOfClause carriers σ replacement
  return mkAppN (mkConst ``VerexIR.Proofs.Denote.Var.update)
    #[unit, functions, frame, ← quoteRow fields, ← quoteNTy σ, ← quoteVar x, replaced, base]

/-- A native enum value with one field replaced in every variant holding it,
as the program assigns it (`Choices.update?`); the value itself under a
variant without it, which no update the validator admits reaches. -/
def nativeVariantFieldUpdate (carriers : Lean.Expr) (built : VerexIR.Proofs.Denote.NTy)
    (choices : List (String × Nat)) (base replacement : Lean.Expr) : MetaM Lean.Expr := do
  let .enum _ _ names rows _ := built
    | throwError "internal: a variant field update of a value that is not an enum"
  let some (variant, index) := choices.head? | throwError "internal: a field update without choices"
  let some ⟨σs, _⟩ := VerexIR.Proofs.Denote.Which.ofName names rows variant
    | throwError "internal: a field update's variant is not the enum's"
  let some ⟨field, _⟩ := VerexIR.Proofs.Denote.Var.ofIndex σs index
    | throwError "internal: a field update's field is out of range"
  let .ok selection := VerexIR.Proofs.Denote.Choices.ofList names rows field choices
    | throwError "internal: a field update's fields differ in type across variants"
  let (unit, functions, frame) ← carriersFrame carriers
  let τ ← quoteNTy built
  let replaced ← nativeOfClause carriers field replacement
  let updated := mkAppN (mkConst ``VerexIR.Proofs.Denote.Choices.update?)
    #[unit, functions, frame, τ.getArg! 2, τ.getArg! 3, ← quoteNTy field, ← quoteChoices selection, replaced, base]
  mkAppOptM ``Option.getD #[carrierType carriers τ, updated, base]

/-- A vector of clause elements at a vector type. -/
def nativeVectorLiteral (carriers : Lean.Expr) (built : VerexIR.Proofs.Denote.NTy)
    (elements : Array Lean.Expr) : MetaM Lean.Expr := do
  let .vector element := built | throwError "internal: a vector literal of a type that is not a vector"
  let carrier := carrierType carriers (← quoteNTy element)
  let values ← elements.mapM (nativeOfClause carriers element)
  let array ← mkArrayLit carrier values.toList
  let bounded ← VerexIR.Proofs.Denote.decidedLemma (← mkAppM ``LT.lt
    #[← mkAppM ``Array.size #[array], mkNatLit (2 ^ 64)])
  return mkApp3 (mkConst ``VerexIR.SpecVector.mk) carrier array bounded

/-- The elements of a native vector literal. -/
def vectorLiteralElements? (vector : Lean.Expr) : Option (Lean.Expr × List Lean.Expr) := do
  let vector := vector.consumeMData
  guard (vector.isAppOfArity ``VerexIR.SpecVector.mk 3)
  let (_, elements) ← (vector.getArg! 1).arrayLit?
  return (vector.getArg! 0, elements)

/-- The concatenation of two native vectors: the literal of their elements
where both are literals, else their concatenation (`SpecVector.append`). -/
def nativeVectorConcat (left right : Lean.Expr) : MetaM Lean.Expr := do
  if let (some (carrier, leftElements), some (_, rightElements)) :=
      (vectorLiteralElements? left, vectorLiteralElements? right) then
    let array ← mkArrayLit carrier (leftElements ++ rightElements)
    let bounded ← VerexIR.Proofs.Denote.decidedLemma (← mkAppM ``LT.lt
      #[← mkAppM ``Array.size #[array], mkNatLit (2 ^ 64)])
    return mkApp3 (mkConst ``VerexIR.SpecVector.mk) carrier array bounded
  mkAppM ``VerexIR.SpecVector.append #[left, right]

/-- The equation of two native values of a type, as a specification's
equality reads them: as values, or, at a type holding a part equality reads
through its identity, by `NTy.equal` with handles (a table owner by its
handle, no ghost slot read). -/
def nativeEquation (carriers τ left right : Lean.Expr) (identity : Bool) : MetaM Lean.Expr := do
  unless identity do return ← mkEq left right
  mkEq (mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.equal) #[carriers, mkConst ``Bool.true, τ, left, right])
    (mkConst ``Bool.true)

/-! ## Intrinsic maps

An owner of an intrinsic map is a native row: a handle or placeholder, a
cached length, its ghost slots, and its table's entries
(`doc/designs/intrinsic-maps.md`, "Abstract representation"). A clause reads the
map through the typed map of the entries (`SpecMap.ofEntries`), keys and
values read by the typed views of their types (`NativeMaps.typedView?`),
under the owner's discipline: in key order when the owner binds a role that
enumerates it so, in insertion order otherwise. -/

/-- A map owner's model: its discipline, and whether its table is behind a
handle, whose allocation `new` records. -/
structure MapModel where
  /-- The value orders of the owner's namespace, where the owner keeps its
  entries in key order; none in insertion order. -/
  orders : Option VerexIR.Validation.ValueOrders
  /-- An owner holding table storage, whose allocation `new` records. -/
  handle : Bool := false
  /-- The owner's ghost slots: the validity slot the Move model synthesizes
  for a map binding a validity role (`doc/designs/ghost-fields.md`). -/
  ghosts : Nat := 0

/-- The model of an intrinsic map owner. -/
def mapModel? (unit : ValidatedUnit) (ty : IrTy) : Option MapModel := do
  let (owner, ns, _) ← nominalDeclaration? unit ty
  let layout ← VerexIR.Validation.ownerLayout? unit owner.namespaceId owner.structId
  let canonical := layout.canonical
  some {
    orders := if layout.ordered then some ns.orders else none
    handle := canonical.handle
    ghosts := canonical.ghosts }

/-- The rank an ordered owner compares keys by: that of its namespace's
value orders. -/
def MapModel.rank? (model : MapModel) : MetaM (Option Lean.Expr) :=
  model.orders.mapM fun orders => do
    return mkApp (mkConst ``VerexIR.SemanticOperations.valueRanks) (← quoteValueOrders orders)

/-- The typed map of a native owner's table: its entries read by the typed
views of its key and value types. -/
structure TypedTable where
  keys : VerexIR.Proofs.Denote.NativeMaps.TypedView
  values : VerexIR.Proofs.Denote.NativeMaps.TypedView
  /-- `SpecMap.ofEntries keys.read values.read table`, or the entries
  themselves where both views read values as they are. -/
  map : Lean.Expr

/-- The typed view of a native type's values, where one evaluates. -/
def typedViewOf (carriers τ : Lean.Expr) : MetaM VerexIR.Proofs.Denote.NativeMaps.TypedView := do
  let some view ← VerexIR.Proofs.Denote.NativeMaps.typedView? carriers τ
    | throwError "the map's keys or values of type {τ} have no typed view: the type has no \
        value a specification can stand for"
  pure view

/-- A clause value as the typed value a view holds it as: an integer as
itself, a value read as it is as its native value, any other value read
(`NTy.read`). -/
def TypedTable.typedOf (view : VerexIR.Proofs.Denote.NativeMaps.TypedView)
    (τ : VerexIR.Proofs.Denote.NTy) (clause : Lean.Expr) : MetaM Lean.Expr := do
  if view.integer then return clause
  let nativeValue ← nativeOfClause view.carriers τ clause
  if view.identity then return nativeValue
  return (mkApp view.read nativeValue).headBeta

/-- A typed value of a view as a clause of a domain reads it: an integer as
itself, any other value as its native value (`NTy.unread`) in the domain. A
native clause of an integer view, a type parameter's value at an integer
type `τ`, is the integer's value at `τ` (`nativeOfClause`). -/
def TypedTable.clauseOf (view : VerexIR.Proofs.Denote.NativeMaps.TypedView) (domain : Domain)
    (typed : Lean.Expr) (τ? : Option VerexIR.Proofs.Denote.NTy := none) : MetaM Lean.Expr := do
  if view.integer then
    if domain != .native then return typed
    let some τ := τ? | throwError "internal: a native clause of an integer map view has no type"
    return ← nativeOfClause view.carriers τ typed
  domain.ofNative (mkApp view.unread typed).headBeta

/-- The native value of a typed value of a view. -/
def TypedTable.nativeOf (view : VerexIR.Proofs.Denote.NativeMaps.TypedView) (typed : Lean.Expr) :
    Lean.Expr :=
  (mkApp view.unread typed).headBeta

def TypedTable.size (table : TypedTable) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.size #[table.keys.carrier, table.values.carrier, table.map]

def TypedTable.sizeOf (table : TypedTable) (map : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.size #[table.keys.carrier, table.values.carrier, map]

def TypedTable.hasKey (table : TypedTable) (key : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.hasKey
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, table.map, key]

def TypedTable.valueAt (table : TypedTable) (key : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.valueAt
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, table.values.inhabited, table.map,
      key]

def TypedTable.keyAt (table : TypedTable) (index : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.keyAt
    #[table.keys.carrier, table.values.carrier, table.keys.inhabited, table.map, index]

def TypedTable.rank (table : TypedTable) (key : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.rank
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, table.map, key]

/-- The order an ordered owner's typed keys compare by: integers by their
values, values read as they are by the structural order (`NTy.keyOrder`),
a type parameter's typed values by the family's (`NTy.readOrder`). -/
def TypedTable.keyOrder (table : TypedTable) (rank : Lean.Expr) : MetaM Lean.Expr := do
  if table.keys.integer then return mkConst ``VerexIR.SpecMap.intOrder
  if table.keys.identity then
    return mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.keyOrder) #[table.keys.carriers, rank,
      table.keys.type]
  return mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.readOrder)
    #[table.keys.carriers, rank, table.keys.type]

/-- The typed discipline of an owner: the sequence, or the order of its keys. -/
def TypedTable.discipline (table : TypedTable) (model : MapModel) : MetaM Lean.Expr := do
  match ← model.rank? with
  | none => mkAppOptM ``VerexIR.SpecMap.Discipline.sequence #[table.keys.carrier]
  | some rank =>
      mkAppOptM ``VerexIR.SpecMap.Discipline.ordered #[table.keys.carrier, ← table.keyOrder rank]

def TypedTable.update (table : TypedTable) (discipline key value : Lean.Expr) :
    MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.update
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, discipline, table.map, key, value]

def TypedTable.updateOf (table : TypedTable) (discipline map key value : Lean.Expr) :
    MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.update
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, discipline, map, key, value]

def TypedTable.remove (table : TypedTable) (discipline key : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.remove
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, discipline, table.map, key]

def TypedTable.removeOf (table : TypedTable) (discipline map key : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.remove
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, discipline, map, key]

/-- The keys of one list set to the values of another into `base`, a typed map
of the table's types. -/
def TypedTable.updateAll (table : TypedTable) (discipline base keys values : Lean.Expr) :
    MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.updateAll
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, discipline, base, keys, values]

def TypedTable.takeEntries (table : TypedTable) (count : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.takeEntries
    #[table.keys.carrier, table.values.carrier, table.map, count]

def TypedTable.dropEntries (table : TypedTable) (count : Lean.Expr) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.dropEntries
    #[table.keys.carrier, table.values.carrier, table.map, count]

def TypedTable.replaceKey (table : TypedTable) (previous replacement : Lean.Expr) :
    MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.replaceKey
    #[table.keys.carrier, table.values.carrier, table.keys.decEq, table.map, previous, replacement]

/-- The keys of the typed map, in order. -/
def TypedTable.keyList (table : TypedTable) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.keys #[table.keys.carrier, table.values.carrier, table.map]

/-- The values of the typed map, in order. -/
def TypedTable.valueList (table : TypedTable) : MetaM Lean.Expr :=
  mkAppOptM ``VerexIR.SpecMap.values #[table.keys.carrier, table.values.carrier, table.map]

/-- The empty typed map. -/
def TypedTable.empty (table : TypedTable) : MetaM Lean.Expr := do
  mkAppOptM ``List.nil #[← mkAppM ``Prod #[table.keys.carrier, table.values.carrier]]

/-- The neighbour of a key in an ordered owner's typed map. -/
def TypedTable.neighbour (table : TypedTable) (model : MapModel) (next : Bool) (key : Lean.Expr) :
    MetaM Lean.Expr := do
  let some rank ← model.rank?
    | throwError "the neighbour of a key needs an ordered map"
  mkAppOptM (if next then ``VerexIR.SpecMap.nextKey? else ``VerexIR.SpecMap.prevKey?)
    #[table.keys.carrier, table.values.carrier, ← table.keyOrder rank, table.map, key]

/-- The typed elements of a native vector, as a list read by the view of
their type. -/
def typedListOf (carriers τ vector : Lean.Expr) :
    MetaM (VerexIR.Proofs.Denote.NativeMaps.TypedView × Lean.Expr) := do
  unless τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 do
    throwError "internal: a list of keys or values that is not a vector"
  let view ← typedViewOf carriers (τ.getArg! 0)
  let elements ← mkAppM ``Array.toList #[← mkAppM ``VerexIR.SpecVector.values #[vector]]
  return (view, ← mkAppM ``List.map #[view.read, elements])

/-- A native owner value's leading fields, its ghost slots, and its table,
with the typed map of the table (`Maps.Layout`): a table layout keeps its
handle or placeholder across every mutation, and a cached length that every
mutation sets to the entries'. -/
structure NativeOwner where
  /-- The owner's native type, its field row, and its value. -/
  type : Lean.Expr
  fields : VerexIR.Proofs.Denote.NRow
  value : Lean.Expr
  handle : Option Lean.Expr := none
  /-- The position of the cached length, and its value. -/
  length : Option (Nat × Lean.Expr) := none
  ghosts : Array Lean.Expr
  /-- The table, as the owner's last component, and its position. -/
  table : Lean.Expr
  tablePosition : Nat
  typed : TypedTable

/-- The leading fields, ghost slots, and table of a native owner value; none
where the type is no owner's. -/
def nativeOwner? (carriers : Lean.Expr) (owner : VerexIR.Proofs.Denote.NTy) (value : Lean.Expr) :
    MetaM (Option NativeOwner) := do
  let .struct _ _ fieldRow := owner | return none
  let ownerType ← quoteNTy owner
  let fields := quotedRowTypes (ownerType.getArg! 2)
  let some table := fields.back? | return none
  unless table.isAppOfArity ``VerexIR.Proofs.Denote.NTy.table 2 do return none
  let projections := quotedRowProjections carriers value fields
  -- The fields before the ghost slots: a handle or placeholder, then a
  -- cached length, as a table layout orders them.
  let isGhost (type : Lean.Expr) := type.isAppOfArity ``VerexIR.Proofs.Denote.NTy.ghost 1
  let leadingCount := (fields.pop.findIdx? isGhost).getD fields.pop.size
  unless (fields.pop.extract leadingCount fields.pop.size).all isGhost do return none
  let mut handle := none
  let mut length := none
  for index in [:leadingCount] do
    let type := fields[index]!
    if type.isConstOf ``VerexIR.Proofs.Denote.NTy.address ||
        type.isConstOf ``VerexIR.Proofs.Denote.NTy.bool then
      if handle.isSome || length.isSome then return none
      handle := some projections[index]!
    else if type.isAppOfArity ``VerexIR.Proofs.Denote.NTy.int 2 then
      if length.isSome then return none
      length := some (index, projections[index]!)
    else return none
  let keys ← typedViewOf carriers (table.getArg! 0)
  let values ← typedViewOf carriers (table.getArg! 1)
  -- One spelling of a table's map, its reads identities or not.
  let map ← mkAppM ``VerexIR.SpecMap.ofEntries #[keys.read, values.read, projections.back!]
  return some { type := ownerType, fields := fieldRow, value, handle, length
                ghosts := projections.extract leadingCount (projections.size - 1)
                table := projections.back!, tablePosition := projections.size - 1
                typed := { keys, values, map } }

/-- The native owner a value of an owner type is. -/
def nativeOwner (carriers : Lean.Expr) (owner : VerexIR.Proofs.Denote.NTy) (value : Lean.Expr) :
    MetaM NativeOwner := do
  let some nativeValue ← nativeOwner? carriers owner value
    | throwError "a map's owner of type {repr owner} is not a native owner"
  pure nativeValue

/-- The table a typed map is written back as (`SpecMap.writeBack`): its
entries, each key and value its native value (`NTy.unread`). -/
def NativeOwner.tableOf (owner : NativeOwner) (map : Lean.Expr) : MetaM Lean.Expr := do
  let unread ← VerexIR.Proofs.Denote.NativeMaps.TypedView.entryUnread owner.typed.keys
    owner.typed.values
  mkAppM ``VerexIR.SpecMap.writeBack #[unread, map]

/-- The owner value with its table holding a typed map, its cached length
the map's size: what a specification's map update builds. -/
def NativeOwner.withMap (owner : NativeOwner) (carriers : Lean.Expr) (map : Lean.Expr) :
    MetaM Lean.Expr := do
  let (unit, functions, frame) ← carriersFrame carriers
  let row ← quoteRow owner.fields
  let replace (value : Lean.Expr) (position : Nat) (replacement : Lean.Expr) : MetaM Lean.Expr := do
    let some ⟨σ, x⟩ := VerexIR.Proofs.Denote.Var.ofIndex owner.fields position
      | throwError "internal: an owner's field is out of range"
    return mkAppN (mkConst ``VerexIR.Proofs.Denote.Var.update)
      #[unit, functions, frame, row, ← quoteNTy σ, ← quoteVar x, replacement, value]
  let mut value ← replace owner.value owner.tablePosition (← owner.tableOf map)
  if let some (position, _) := owner.length then
    let size ← owner.typed.sizeOf map
    let some (.int width signed) := owner.fields.toList[position]?
      | throwError "internal: an owner's cached length is not an integer"
    value ← replace value position
      (← nativeOfClause carriers (.int width signed) size (cut := ``VerexIR.SpecInt.ofSize))
  pure value

/-- The empty table of a table carrier's type (`SpecEntries` of its entries):
no entries. -/
def emptyTableOf (table : Lean.Expr) : MetaM Lean.Expr := do
  let_expr VerexIR.SpecEntries element := ← whnfR table
    | throwError "internal: {table} is not a table's carrier"
  let nil ← mkAppOptM ``List.nil #[element]
  let bound ← mkDecideProof (← mkAppM ``LT.lt
    #[← mkAppM ``List.length #[nil], ← mkAppM ``HPow.hPow #[mkNatLit 2, mkNatLit 64]])
  mkAppM ``VerexIR.SpecEntries.mk #[nil, bound]

/-- The empty owner of a type without a handle: no entries, a cached length
of zero, its ghost slots unspecified, as a specification's `new` builds it.
The components are clause values, which `nativeRow` reads natively. -/
def emptyOwner (carriers : Lean.Expr) (owner : VerexIR.Proofs.Denote.NTy) : MetaM Lean.Expr := do
  let .struct _ _ row := owner | throwError "internal: a map's owner is not a struct"
  let components ← row.toList.mapM fun τ => do
    match τ with
    | .int _ _ => pure (mkIntLit 0)
    | .ghost contents =>
        let carrier := carrierType carriers (← quoteNTy τ)
        let inhabitant ← typedInhabitant carriers (← quoteNTy contents)
        return mkAppN (mkConst ``VerexSL.Contract.unspecifiedGhost)
          #[← quoteNTy contents, carrier, mkApp2 (mkConst ``Inhabited.mk [1]) carrier inhabitant]
    | .table _ _ => emptyTableOf (carrierType carriers (← quoteNTy τ))
    -- The placeholder `dummy_field` of an empty struct, as Move's compilers
    -- build it (`doc/designs/intrinsic-maps.md`, "Table layout").
    | .bool => pure (mkConst ``False)
    | _ => throwError "a map without a handle is built with a field of type {repr τ}, \
        which no specification's `new` gives a value"
  nativeRow carriers row components

/-- An enum value at a native type, with its family and rows. -/
structure NativeVariant where
  unit : Lean.Expr
  functions : Lean.Expr
  frame : Lean.Expr
  names : Lean.Expr
  rows : Lean.Expr
  value : Lean.Expr

/-- The enum value of a native enum type. -/
def nativeVariant (carriers τ value : Lean.Expr) : MetaM NativeVariant := do
  let (unit, functions, frame) ← carriersFrame carriers
  let τ := τ.consumeMData
  unless τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 do
    throwError "internal: {τ} is not an enum"
  return { unit, functions, frame, names := τ.getArg! 2, rows := τ.getArg! 3, value }

/-- The ghost field the Move model synthesizes as the validity slot of a map
binding a validity role, and of its iterators (`$validity`,
`doc/designs/ghost-fields.md`), as a VerexSL rendering spells a hidden name. -/
def validitySlotName : String := "_validity"

/-- The position among an owner's or iterator's ghost slots of its validity
slot, none without one. -/
private def validitySlot? (unit : ValidatedUnit) (handle : VerexIR.StructHandle) : Option Nat := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  (VerexIR.SemanticOperations.nominalGhosts unit handle).findIdx? fun ghost =>
    (ns.tables.names[ghost.name.index]?).any (·.name == validitySlotName)

/-- The validity slot of an owner's native value: its ghost slot. -/
def ownerValiditySlot (context : Context) (ty : IrTy) (owner : NativeOwner) :
    MetaM Lean.Expr := do
  let some (handle, _, _) := nominalDeclaration? context.unit ty
    | throwError "a map's owner is not a declared nominal"
  let some position := validitySlot? context.unit handle
    | throwError "the map's owner has no validity slot"
  let some slot := owner.ghosts[position]?
    | throwError "the map's owner value has no ghost slot {position}"
  pure slot

/-- The validity slot of an iterator's native value: the ghost slot of
whichever variant it holds, each variant's following its own fields. -/
def iteratorValiditySlot (context : Context) (ty : IrTy) (enum : VerexIR.Proofs.Denote.NTy)
    (carriers value : Lean.Expr) : MetaM Lean.Expr := do
  let some (handle, ns, declaration) := nominalDeclaration? context.unit ty
    | throwError "an iterator is not a declared nominal"
  let some ghost := (VerexIR.SemanticOperations.nominalGhosts context.unit handle).find? fun ghost =>
      (ns.tables.names[ghost.name.index]?).any (·.name == validitySlotName)
    | throwError "the iterator has no validity slot"
  let some ghostName := (ns.tables.names[ghost.name.index]?).map (·.name)
    | throwError "the iterator's validity slot has no name"
  let some choices := declaration.variants.toList.mapM fun (variant : VerexIR.VariantDecl) => do
      let named : VerexIR.QualifiedName ← ns.tables.names[variant.name.index]?
      let index ← VerexIR.SemanticOperations.handleGhostIndex? context.unit handle (some named.name)
        ghostName
      some (named.name, index)
    | throwError "the iterator's validity slot does not resolve in each variant"
  let (_, read) ← nativeVariantField enum choices.toArray carriers (← quoteNTy enum) value
  pure read

/-- A native value as the typed value a view holds it as: an integer's
value, a value read as it is itself, any other value read (`NTy.read`). -/
def TypedTable.typedOfNative (view : VerexIR.Proofs.Denote.NativeMaps.TypedView) (nativeValue : Lean.Expr) :
    MetaM Lean.Expr := do
  if view.integer then return ← mkAppM ``VerexIR.SpecInt.val #[nativeValue]
  if view.identity then return nativeValue
  return (mkApp view.read nativeValue).headBeta

/-- The specification role a specification function plays for an intrinsic
map of its namespace, with the map's owner. -/
def mapSpecRole? (ns : ValidatedNamespace) (reference : VerexIR.QualifiedRef) :
    Option (VerexIR.MapSpecRole × VerexIR.NameId) :=
  ns.intrinsics.findSome? fun intrinsic =>
    if intrinsic.model == "map" then
      -- A leaf offset is uninterpreted (`doc/designs/intrinsic-maps.md`, "Roles").
      intrinsic.specBindings.findSome? fun binding => do
        guard (binding.target == reference)
        let role ← VerexIR.MapSpecRole.ofName? binding.role
        guard (role != .leafOffset)
        some (role, intrinsic.owner)
    else none


/-- How an iterator of an intrinsic map names its entry, as the Move Prover
classifies its payload variant (`iter_key_field`): by the key it carries, a
field of the map's key type parameter, or by a position, one integer field
the map's enumeration turns into a key. -/
structure IteratorModel where
  owner : VerexIR.StructHandle
  variant : String
  /-- The field's position in its variant. -/
  field : Nat
  keyed : Bool

/-- The iterator model of an enum: its one field of the key type parameter,
which wins, else its one `u64` or `num` field, in whichever variant. -/
def iteratorModel? (unit : ValidatedUnit) (ty : IrTy) : Option IteratorModel := do
  let (owner, _, declaration) ← nominalDeclaration? unit ty
  let fields := declaration.variants.toList.flatMap fun variant =>
    variant.fields.toList.zipIdx.map fun (field, index) =>
      (variant, index, unit.tables.types[field.type.typeId.index]?)
  let keys := fields.filter fun (_, _, type) => type == some (.typeParameter 0)
  let positions := fields.filter fun (_, _, type) =>
    type == some (.integer (.bits 64) false) || type == some (.integer .unbounded true)
  let (variant, index, keyed) ← match keys, positions with
    | [(variant, index, _)], _ => some (variant, index, true)
    | [], [(variant, index, _)] => some (variant, index, false)
    | _, _ => none
  let name ← (unit.tables.names[variant.name.index]?).map (·.name)
  pure { owner, variant := name, field := index, keyed }

/-- The payload an iterator holds: its key, or its position, its payload
variant's field the model names. -/
def iteratorPayload (model : IteratorModel) (enum : VerexIR.Proofs.Denote.NTy)
    (carriers iterator : Lean.Expr) : MetaM Lean.Expr := do
  let (_, payload) ← nativeVariantField enum #[(model.variant, model.field)] carriers
    (← quoteNTy enum) iterator
  pure payload

/-- Whether an iterator names an entry of a map: its payload variant, with a
key the map holds, or a position below the map's size. -/
def iteratorInRange (model : IteratorModel) (enum : VerexIR.Proofs.Denote.NTy)
    (carriers iterator : Lean.Expr) (table : TypedTable) : MetaM Lean.Expr := do
  let test ← nativeVariantTest carriers (← quoteNTy enum) iterator model.owner #[model.variant]
  let payload ← iteratorPayload model enum carriers iterator
  if model.keyed then
    mkAppM ``And #[test,
      ← mkEq (← table.hasKey (← TypedTable.typedOfNative table.keys payload)) (mkConst ``Bool.true)]
  else
    mkAppM ``And #[test, ← mkAppM ``LT.lt #[← mkAppM ``VerexIR.SpecInt.val #[payload], ← table.size]]

/-- The typed key an iterator names: its payload, or the key at its position. -/
def iteratorKey (model : IteratorModel) (enum : VerexIR.Proofs.Denote.NTy)
    (carriers iterator : Lean.Expr) (table : TypedTable) : MetaM Lean.Expr := do
  let payload ← iteratorPayload model enum carriers iterator
  if model.keyed then TypedTable.typedOfNative table.keys payload
  else table.keyAt (← mkAppM ``VerexIR.SpecInt.val #[payload])

mutual
/-- Whether a type holds a part a specification's equality reads through its
identity (`NTy.identityRuntime`): an owner identified by its handle, a ghost
slot, or a function value, at some depth; a function value's captures may
hold ghost slots, and in a unit declaring ghosts (`ghosts`) so may a type
parameter's instance. An owner without a handle compares by its contents,
so its entries are read as values are. -/
def holdsContents (ghosts : Bool) : VerexIR.Proofs.Denote.NTy → Bool
  | .ghost _ | .function _ _ _ => true
  | .param _ => ghosts
  | .table key value => holdsContents ghosts key || holdsContents ghosts value
  | .tuple elements => rowHoldsContents ghosts elements
  | .struct _ _ elements => elements.ownsHandle || rowHoldsContents ghosts elements
  | .enum _ _ _ rows _ => rowsHoldContents ghosts rows
  | .vector element | .ref element => holdsContents ghosts element
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes => false

private def rowHoldsContents (ghosts : Bool) : VerexIR.Proofs.Denote.NRow → Bool
  | .nil => false
  | .cons τ rest => holdsContents ghosts τ || rowHoldsContents ghosts rest

private def rowsHoldContents (ghosts : Bool) : VerexIR.Proofs.Denote.NRows → Bool
  | .nil => false
  | .cons fields rest => rowHoldsContents ghosts fields || rowsHoldContents ghosts rest
end

/-- What a quantifier over the elements of a native vector binds: an element
at its native type, its membership in the vector's values, and the
pattern's local of the domain. -/
def nativeVectorBinder (carriers : Lean.Expr) (domain : Domain) (vectorType : VerexIR.Proofs.Denote.NTy)
    (vector : Lean.Expr) (binderName : Name) :
    MetaM (Name × Lean.Expr × Option (Lean.Expr → MetaM Lean.Expr) ×
      (Lean.Expr → MetaM Lean.Expr)) := do
  let .vector element := vectorType
    | throwError "internal: a quantifier over the elements of a value that is not a vector"
  let values ← mkAppM ``VerexIR.SpecVector.values #[vector]
  return (binderName, carrierType carriers (← quoteNTy element),
    some fun bound => mkAppM ``Membership.mem #[values, bound],
    fun bound => domain.binderOfNative bound)

/-- The specification function a call names, with its namespace. -/
def specFunctionOf? (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    Option (ValidatedNamespace × VerexIR.SpecFunctionDecl) := do
  let targetNs ← unit.namespaces[reference.namespaceId.index]?
  let functionId ← unit.resolution.specFunction? reference.name
  let declaration ← targetNs.specFunctions[functionId.index]?
  pure (targetNs, declaration)

/-- The authored byte range of a location, when the unit records one. -/
def locRange (unit : ValidatedUnit) (loc : VerexIR.LocId) : ObligationRange :=
  match unit.tables.locations[loc.index]?.bind (·.primary) with
  | some range => {
      file := (unit.tables.files[range.file.index]?.map (·.name)).getD ""
      startByte := range.startByte, endByte := range.endByte }
  | none => {}

/-- The authored byte range of a condition. -/
def conditionRange (unit : ValidatedUnit) (condition : VerexIR.Condition) :
    ObligationRange :=
  locRange unit condition.loc

/-- Wrap a translated clause with its source range so a residual obligation
can be reported at the authored clause. -/
def markObligation (range : ObligationRange) (clause : Lean.Expr) : Lean.Expr :=
  mkApp4 (mkConst ``VerexIR.Proofs.Obligation) (mkStrLit range.file)
    (mkRawNatLit range.startByte) (mkRawNatLit range.endByte) clause

/-- The lemma a reference names, with its namespace. -/
def lemmaOf? (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    Option (ValidatedNamespace × VerexIR.LemmaDecl) := do
  let targetNs ← unit.namespaces[reference.namespaceId.index]?
  let index ← unit.resolution.lemma? reference.name
  let declaration ← targetNs.lemmas[index]?
  pure (targetNs, declaration)

/-- The parameters of a lemma that range over the values of a native type,
which its definitions take at a family, by position. -/
def lemmaNativeParameters (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    Array (Nat × VerexIR.Proofs.Denote.NTy) := Id.run do
  let some (_, declaration) := lemmaOf? unit reference | return #[]
  let mut natives := #[]
  for parameter in declaration.signature.parameters, index in [0:declaration.signature.parameters.size] do
    let typeId := specTypeId unit parameter.typeUse.typeId
    let some parameterType := unit.tables.types[typeId.index]? | continue
    unless domainOf parameterType matches .native do continue
    let some nty := VerexIR.Proofs.Denote.ntyOf unit reference.namespaceId typeId | continue
    natives := natives.push (index, nty)
  return natives

/-- Whether a lemma's definitions take a family: to read storage at, or to
type its parameters at. -/
def lemmaTakesFrame (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef)
    (readsState : Bool) : Bool :=
  readsState || !(lemmaNativeParameters unit reference).isEmpty

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
def lemmaNames (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) : MetaM LemmaNames := do
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
private partial def specReaches (unit : ValidatedUnit) (source target : VerexIR.QualifiedRef) :
    Bool := Id.run do
  let mut work : List (VerexIR.NamespaceId × ExprId) := match specFunctionOf? unit source with
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
        work := work ++ (VerexIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·)
  return false

/-- What a specification function's body reads besides its arguments,
through the specification functions it calls: the executable unit (a
behavioral predicate other than `requires_of`), the table of declared
preconditions (`requires_of`), and the state (either, or storage). -/
structure SpecReads where
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

/-- Whether a specification function is the unspecified value VerexSL
declares for an aborting specification branch. It does not depend on its
instantiation, which may be a mathematical integer no native type stands for. -/
def arbitraryValue (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) : Bool :=
  (unit.tables.names[reference.name.index]?).any (·.name.startsWith "__verex_arbitrary_")

/-- What the expressions `start` reach read, through the specification
functions they call. -/
def readsFrom (unit : ValidatedUnit) (start : List (VerexIR.NamespaceId × ExprId)) :
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
        work := work ++ (VerexIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·)
  return reads

/-- What a declaration's data invariants read. -/
def invariantReads (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.StructDecl) : SpecReads :=
  let reads := readsFrom unit <|
    (declaration.contract.conditions.filter (·.kind == .structInvariant)).toList.map
      fun condition => (namespaceId, condition.expression)
  if (unit.namespaces[namespaceId.index]?).any (VerexIR.Proofs.Denote.declarationHasClosureFields · declaration) then
    { reads with unit := true, determinism := true }
  else reads

/-- Whether a data invariant of the unit states a behavioral predicate: the
predicate of stored invariants then takes the executable unit. -/
def storedInvariantReadsUnit (unit : ValidatedUnit) : Bool :=
  (unit.namespaces.toList.zipIdx).any fun (ns, index) =>
    ns.structs.any fun declaration =>
      let reads := invariantReads unit ⟨index⟩ declaration
      reads.unit || reads.requires

def specReads (unit : ValidatedUnit) (source : VerexIR.QualifiedRef) : SpecReads :=
  match specFunctionOf? unit source with
  | some (_, { body := some root, .. }) => readsFrom unit [(source.namespaceId, root)]
  | _ => {}

/-- Whether a recursive specification function's definition takes the types
its type parameters stand for: it reads an uninterpreted value at type
arguments, and takes no frame, whose family would supply them. -/
def definitionTakesTypes (reads : SpecReads) (takesFrame : Bool) : Bool :=
  reads.types && !takesFrame

/-- Whether a specification function reaches itself through the
specification functions its body calls. -/
def specRecursive (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) : Bool :=
  specReaches unit reference reference

/-- The spelling of a specification function, for a diagnostic. -/
def specFunctionSpelling (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    String :=
  ((unit.tables.names[reference.name.index]?).map (·.name)).getD (toString (repr reference))

/-- The Lean name of a recursive specification function's definition. -/
def specDefinitionName (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    MetaM Name := do
  let some path := unit.tables.namespaces[reference.namespaceId.index]?
    | throwError "a specification function's namespace has no path"
  let some qualified := unit.tables.names[reference.name.index]?
    | throwError "a specification function has no name"
  let base := path.segments.foldl (fun name segment => Name.str name segment) .anonymous
  return Name.str (Name.str base qualified.name) "spec"

/-- The recursive specification functions defined together with one: those
it reaches that reach it back, itself among them, in declaration order. -/
def specGroup (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    Array VerexIR.QualifiedRef := Id.run do
  let mut group := #[]
  for ns in unit.namespaces, namespaceIndex in [0:unit.namespaces.size] do
    for declaration in ns.specFunctions do
      let other : VerexIR.QualifiedRef := ⟨⟨namespaceIndex⟩, declaration.name⟩
      if other == reference ||
          (specReaches unit reference other && specReaches unit other reference) then
        group := group.push other
  return group

/-- A member of a group of recursive specification functions, as its
definition is built. -/
structure SpecMember where
  reference : VerexIR.QualifiedRef
  name : Name
  targetNs : ValidatedNamespace
  declaration : VerexIR.SpecFunctionDecl
  root : ExprId
  domains : Array Domain
  resultDomain : Domain
  bundleType : Lean.Expr
  resultType : Lean.Expr
  localTypes : Array IrTy
  /-- The native type of each parameter the definition takes natively, and
  of its result where native (`DefinitionShape`). -/
  natives : Array (Option Lean.Expr) := #[]
  resultNative : Option Lean.Expr := none
  deriving Inhabited

/-- The shape of a recursive specification function's definition at an
instantiation: how its parameters and result are taken, and what it reads.
An aggregate parameter or result is a value of its type's carrier at the
frame the definition takes, and a generic function is defined per
instantiation, at the call's type arguments. -/
structure DefinitionShape where
  reference : VerexIR.QualifiedRef
  name : Name
  reads : SpecReads
  /-- The native types of the type arguments the definition is at. -/
  typeArgumentTypes : Array (Option VerexIR.Proofs.Denote.NTy) := #[]
  domains : Array Domain
  resultDomain : Domain
  localTypes : Array IrTy
  /-- The native type of each parameter taken natively. -/
  natives : Array (Option VerexIR.Proofs.Denote.NTy)
  /-- The result's native type where the result is native: its type is
  native and inhabited, and the body is a native value; a body that is not
  one has an encoded result, which the definition's type then states. -/
  resultNative : Option VerexIR.Proofs.Denote.NTy
  /-- Whether the definition takes a frame: it reads storage, or a parameter
  or its result is nativeValue. -/
  takesFrame : Bool
  takesTypes : Bool

/-- What a recursive specification function's definition defines: the
function, by its namespace's path and its name, and the native types of the
type arguments it is at, which name the definition at another frame's
instantiation (`ensureDefinitionTransport`). -/
structure DefinitionKey where
  path : Array String
  function : String
  typeArgumentTypes : Array (Option VerexIR.Proofs.Denote.NTy)
  deriving Inhabited

/-- The definitions of recursive specification functions, by name. -/
initialize definitionKeys :
    SimplePersistentEnvExtension (Name × DefinitionKey) (NameMap DefinitionKey) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, key) => map.insert name key
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (name, key) => map.insert name key) {} entries }

/-- The function a definition's key names, in a unit. -/
def DefinitionKey.reference? (key : DefinitionKey) (unit : ValidatedUnit) :
    Option VerexIR.QualifiedRef := do
  let namespaceIndex ← unit.tables.namespaces.findIdx? (·.segments == key.path)
  let nameIndex ← unit.tables.names.findIdx? fun name =>
    name.name == key.function && name.namespaceId.index == namespaceIndex
  pure { namespaceId := ⟨namespaceIndex⟩, name := ⟨nameIndex⟩ }

/-- The sum of types from position `k` on: `PSum`, nested to the right. -/
partial def sumType (types : Array Lean.Expr) (k : Nat) : MetaM Lean.Expr := do
  if k + 1 ≥ types.size then return types[k]!
  mkAppM ``PSum #[types[k]!, ← sumType types (k + 1)]

/-- A value of the type at position `index` in the sum from position `k`. -/
partial def sumInject (types : Array Lean.Expr) (k index : Nat) (value : Lean.Expr) :
    MetaM Lean.Expr := do
  if k + 1 ≥ types.size then return value
  let rest ← sumType types (k + 1)
  if index == k then mkAppOptM ``PSum.inl #[types[k]!, rest, value]
  else mkAppOptM ``PSum.inr #[types[k]!, rest, ← sumInject types (k + 1) index value]

/-- `x` of the sum from position `k`, by its summand: `branch index value`
at the type at `index`, of the type `motive` gives the injection of
`value` into the sum from position 0. -/
partial def sumCases (types : Array Lean.Expr) (k : Nat) (motive x : Lean.Expr)
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
def mkConstMotive (domain value : Lean.Expr) : MetaM Lean.Expr :=
  withLocalDeclD `x domain fun x => mkLambdaFVars #[x] value

/-- The parameters that are mutable references: their entry binder is apart
from their exit binder. -/
def Context.mutableParameters (context : Context) : Array Nat :=
  (Array.range context.locals.size).filter fun index =>
    context.locals[index]? != context.oldLocals[index]?

/-- A translated value as a local's: a boolean decided. -/
def decidedAt (type : IrTy) (value : Lean.Expr) : MetaM Lean.Expr :=
  match domainOf type with
  | .boolean => decideProposition value
  | _ => pure value

/-- The context with a local bound to a value at a type. -/
def Context.bindLocal (context : Context) (localId : VerexIR.LocalId) (type : IrTy)
    (value : Lean.Expr) : MetaM Context := do
  unless localId.index < context.locals.size do
    throwError "specification local {localId.index} has no binder slot"
  pure { context with
    locals := context.locals.set! localId.index (some value)
    localTypes := if localId.index < context.localTypes.size
      then context.localTypes.set! localId.index type else context.localTypes
    oldLocals := context.oldLocals.setIfInBounds localId.index (some value) }

/-- The context with a component of a destructuring pattern bound: a
variable to the value `valueOf` gives at its type, a wildcard to nothing. -/
def Context.bindComponent (context : Context) (element : VerexIR.PatternId)
    (pattern : String) (valueOf : IrTy → MetaM Lean.Expr) : MetaM Context := do
  let some elementNode := context.ns.patterns[element.index]?
    | throwError "specification binding pattern {element.index} is out of range"
  match elementNode.kind with
  | .wildcard => pure context
  | .variable localId =>
      let some elementType := context.valueTypeOf? elementNode.typeId
        | throwError "specification binding pattern has an unknown type"
      context.bindLocal localId elementType (← valueOf elementType)
  | _ => throwError "generated contracts require a variable or wildcard component \
      in a {pattern} binding pattern"

end VerexSL.Contract
