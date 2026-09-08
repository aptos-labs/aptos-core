-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Elab
import LeanerLang.Perf
import LeanerLang.Denotation
import LeanerLang.RowScript
import LeanerLang.Quote
import LeanerLang.Registry
import LeanerLang.SpecTypes
import LeanerLang.Syntax
import LeanerLang.Typed
import LeanerIR.Proofs.Certify
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
  /-- Logical type of each local binder. Mutable references are represented
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

private def typeOfExpr? (context : Context) (id : ExprId) : Option IrTy := do
  let expression ← context.ns.expressions[id.index]?
  context.unit.tables.types[expression.typeId.index]?

private def describeType (ty : IrTy) : String :=
  toString (repr ty)

/-- The typed twin a specification expression's value inhabits: a storage
read is twin-typed, and the twin domain follows through `spec.old`,
dereference, and selection of a nominal field.  Every other expression stays
in its scalar or runtime-value domain. -/
private partial def twinOfExpr? (context : Context) (id : ExprId) :
    Option SpecTypes.TwinInfo := do
  let expression ← context.ns.expressions[id.index]?
  match expression.kind with
  | .operation (.specification (.global _)) instantiations _ _ =>
      match instantiations.toList with
      | [.typeArg resource] =>
          (context.families.find?
            (·.typeIndex == resource.typeId.index)).map (·.info)
      | _ => none
  | .operation (.specification .old) _ arguments _ => do
      twinOfExpr? context (← arguments[0]?)
  | .operation (.reference .dereference) _ arguments _ => do
      twinOfExpr? context (← arguments[0]?)
  | .operation (.data (.select _ field)) _ arguments _ => do
      let base ← twinOfExpr? context (← arguments[0]?)
      let (_, rep) ← base.fields.find? (·.1 == field)
      match rep with
      | .nominal twin => context.twins.find? (·.twin == twin)
      | _ => none
  | _ => none

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
  | .ifElse condition thenBranch (some elseBranch) =>
      -- A conditional in a clause is Lean's `ite` on the decided test.
      let test ← translate context condition
      let thenTerm ← translate context thenBranch
      let elseTerm ← translate context elseBranch
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
    | _, _ =>
        throwError "specification literal {repr literal} at type {describeType ty} \
          is not supported in generated contracts"
  /-- Translate an operand and re-encode it as a runtime value, for the
  positions — a storage key, a published resource — where a clause hands a
  value back to the runtime vocabulary. -/
  runtimeOperand (id : ExprId) : MetaM Lean.Expr := do
    let some ty := typeOfExpr? context id
      | throwError "a specification operand has an unknown type"
    let translated ← translate context id
    match domainOf ty with
    | .boolean => mkAppM ``RuntimeValue.bool #[← mkAppM ``Decidable.decide #[translated]]
    | domain => domain.encode translated
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
    | .specification .bitVectorToInt =>
        -- The logical domain already represents every integer unbounded, so
        -- reading a fixed-width value at `Int` is the identity.
        let some argument := arguments[0]?
          | throwError "spec.bitVectorToInt expects one argument"
        translate context argument
    | .specification (.global _) =>
        let some key := arguments[0]?
          | throwError "a storage read expects one key"
        let family ← storableFamily instantiations "read"
        let globals ← mkAppM ``LeanerIR.RuntimeState.globals #[← currentState]
        mkAppM (family.info.twin ++ `get) #[globals, ← runtimeOperand key]
    | .global .contains =>
        let some key := arguments[0]?
          | throwError "a storage existence test expects one key"
        let family ← storableFamily instantiations "existence test"
        let globals ← mkAppM ``LeanerIR.RuntimeState.globals #[← currentState]
        let test ← mkAppM (family.info.twin ++ `contains)
          #[globals, ← runtimeOperand key]
        mkEq test (mkConst ``Bool.true)
    | .data (.select reference field) =>
        let some base := arguments[0]?
          | throwError "a field selection expects one operand"
        match twinOfExpr? context base with
        | some info =>
            let some (fieldName, rep) := info.fields.find? (·.1 == field)
              | throwError "field `{field}` does not exist on the typed \
                  twin of `{info.name}`"
            let projected := mkApp
              (mkConst (info.twin ++ Name.mkSimple fieldName))
              (← translate context base)
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
              #[← translate context base, toExpr index]
            (domainOf ty).ofRuntime selected
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
    | .add => let (l, r) ← binary arguments; mkAppM ``HAdd.hAdd #[l, r]
    | .subtract => let (l, r) ← binary arguments; mkAppM ``HSub.hSub #[l, r]
    | .multiply => let (l, r) ← binary arguments; mkAppM ``HMul.hMul #[l, r]
    | .divide => let (l, r) ← binary arguments; mkAppM ``Int.tdiv #[l, r]
    | .modulo => let (l, r) ← binary arguments; mkAppM ``Int.tmod #[l, r]
    | .negate => let value ← unary arguments; mkAppM ``Neg.neg #[value]
    | .less => let (l, r) ← binary arguments; mkAppM ``LT.lt #[l, r]
    | .greater => let (l, r) ← binary arguments; mkAppM ``LT.lt #[r, l]
    | .lessEqual => let (l, r) ← binary arguments; mkAppM ``LE.le #[l, r]
    | .greaterEqual => let (l, r) ← binary arguments; mkAppM ``LE.le #[r, l]
    | .equal =>
        let (l, r) ← binary arguments
        let some operandTy := arguments[0]?.bind (typeOfExpr? context)
          | throwError "an equality operand has an unknown type"
        if operandTy == LeanerIR.Ty.bool then mkAppM ``Iff #[l, r] else mkAppM ``Eq #[l, r]
    | .notEqual =>
        let (l, r) ← binary arguments
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
  let (kind, physical) ← match declared with
    | .reference reference => do
        unless allowReference do
          throwError "a result of reference type is not supported in           generated contracts"
        let some referent := context.unit.tables.types[reference.referent.index]?
          | throwError "reference referent type {reference.referent.index} is           out of range"
        let kind := match reference.kind with
          | .shared => SlotKind.sharedRef
          | .mutable => SlotKind.mutableRef
        pure (kind, referent)
    | declared => pure (SlotKind.plain, declared)
  return { name, kind, physical, leanType := (domainOf physical).leanType, components }

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
  let some encoded ← encodeValue? slot.physical binders.entry
    | throwError "a value of type {describeType slot.physical} has no runtime \
        encoding in generated contracts"
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
      let step ← mkAppM ``LeanerIR.SemanticOperations.FocusStep.mk #[handle, before, after]
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
is retained only as a static proof tag by the native loop combinator; the
generated predicate below reads the current frame directly. -/
private partial def loopSpecifications (ns : ValidatedNamespace)
    (root : ExprId) (seen : Array Nat := #[]) :
    Array (ExprId × LeanerIR.SpecBlock) := Id.run do
  if seen.contains root.index then return #[]
  let some expression := ns.expressions[root.index]? | return #[]
  let seen := seen.push root.index
  let mut found := #[]
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
  return found

/-- Translate every authored loop invariant to a closed predicate over the
native runtime frame and state. Every local is existentially decoded and tied
to its actual slot so the predicate records the complete native frame shape;
no source path or expression evaluation is present in the resulting term. -/
private def buildLoopInvariants (unit : ValidatedUnit)
    (namespaceId : LeanerIR.NamespaceId) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (twins : Array SpecTypes.TwinInfo) : MetaM (Array (ExprId × Lean.Expr)) := do
  let .structured root := declaration.body | return #[]
  let families := SpecTypes.familyInfos unit twins
  let allSlots ← declaration.locals.mapM fun localDecl =>
    slotOf { unit, namespaceId, ns, locals := #[], results := #[] }
      (Name.mkSimple s!"loop_local_{localDecl.id.index}") localDecl.type.typeId
      (allowReference := true)
  let localTypes := allSlots.map (·.physical)
  let mut predicates := #[]
  for (site, block) in loopSpecifications ns root do
    let conditions := block.conditions.filter
      (fun condition => condition.kind == .loopInvariant)
    /- Close over every local, not only the ones named by the authored
    clause.  This gives the invariant a complete native frame shape, which
    is required both for scalar frame finalization and for retaining the
    prophecy identities of reference-shaped locals. -/
    let slots := allSlots
    let predicate ←
      withLocalDeclD `loopEntryFrame (mkConst ``LeanerIR.RuntimeFrame) fun entryFrame =>
      withLocalDeclD `loopEntryState (mkConst ``LeanerIR.RuntimeState) fun entryState =>
      withLocalDeclD `loopFrame (mkConst ``LeanerIR.RuntimeFrame) fun frame =>
      withLocalDeclD `loopState (mkConst ``LeanerIR.RuntimeState) fun state =>
        withSlotBinders slots (withExit := false) fun bound => do
          let mut locals : Array (Option Lean.Expr) :=
            Array.replicate declaration.locals.size none
          let mut encodedLocals := #[]
          let mut equations := #[]
          let mut entryEquations := #[]
          for ((localDecl, slot), binders) in
              (declaration.locals.zip slots).zip bound do
            let localId := localDecl.id
            locals := locals.set! localId.index (some binders.current)
            let encoded ← slotRuntimeValue slot binders
            encodedLocals := encodedLocals.push
              (← mkAppM ``Option.some #[encoded])
            let read ← mkAppM ``LeanerIR.SemanticOperations.readLocal?
              #[frame, toExpr localId]
            equations := equations.push
              (← mkAppM ``Eq #[read, ← mkAppM ``Option.some #[encoded]])
            unless localDecl.mutable do
              let entryRead ← mkAppM ``LeanerIR.SemanticOperations.readLocal?
                #[entryFrame, toExpr localId]
              entryEquations := entryEquations.push
                (← mkAppM ``Eq
                  #[entryRead, ← mkAppM ``Option.some #[encoded]])
          let optionRuntimeValue := Lean.mkApp
            (Lean.mkConst ``Option [Lean.Level.zero])
            (mkConst ``LeanerIR.RuntimeValue)
          let localsLiteral ← mkArrayLit optionRuntimeValue encodedLocals.toList
          let frameLocals ← mkAppM ``LeanerIR.RuntimeFrame.locals #[frame]
          let frameShape ← mkAppM ``Eq #[frameLocals, localsLiteral]
          let context : Context :=
            { unit, namespaceId, ns, twins, families, locals, localTypes
              oldLocals := locals, results := #[], state := some state,
              oldState := some state }
          let clauses ← conditions.mapM (translate context ·.expression)
          let ranges ← slotRanges slots (some ·.entry) bound
          let stateStable ← mkAppM ``Eq #[state, entryState]
          let body ← conjunction
            (#[stateStable, frameShape] ++ equations ++ entryEquations ++
              ranges ++ clauses)
          let closed ← existsOver (bound.flatMap SlotBinders.flat) body
          mkLambdaFVars #[entryFrame, entryState, frame, state] closed
    predicates := predicates.push (site, predicate)
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
      mkAppM (family.info.twin ++ `key) #[encoded]
  | _ =>
      throwError "a modifies clause must name a resource at a key in \
        generated contracts"

/-- Bind one typed-contents binder per storable family and continue. -/
private def withFamilyContents (families : Array SpecTypes.FamilyInfo)
    (k : Array Lean.Expr → MetaM α) : MetaM α :=
  let rec go (index : Nat) (bound : Array Lean.Expr) : MetaM α := do
    if h : index < families.size then
      let family := families[index]
      let contentsType ← mkArrow (mkConst ``LeanerIR.StorageKey)
        (← mkAppM ``Option #[mkConst family.info.twin])
      withLocalDeclD (Name.mkSimple (family.info.name ++ "_contents"))
        contentsType fun binder => go (index + 1) (bound.push binder)
    else k bound
  go 0 #[]

/-- The representation conjuncts tying each family binder to the runtime
map: this is the well-typed-store precondition, in the shape that rewrites. -/
private def familyConjuncts (families : Array SpecTypes.FamilyInfo)
    (bound : Array Lean.Expr) (state : Lean.Expr) : MetaM (Array Lean.Expr) := do
  let globals ← mkAppM ``LeanerIR.RuntimeState.globals #[state]
  (families.zip bound).mapM fun (family, binder) =>
    mkAppM ``LeanerIR.FamilyRepresentation
      #[mkConst (family.info.twin ++ `erase),
        toExpr (⟨family.info.namespaceIndex⟩ : LeanerIR.NamespaceId),
        toExpr (⟨family.typeIndex⟩ : LeanerIR.TypeId),
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
  let key ← mkAppM (lender.family.info.twin ++ `key) #[encodedKey]
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
    (families : Array SpecTypes.FamilyInfo := #[]) :
    MetaM Lean.Expr := do
  let families := familiesUsed ns declaration families
  let groups ← groupConditions unit declaration.contract.conditions
  let isPartial := pragmaEnabled declaration.contract "aborts_if_is_partial"
  let isStrict := pragmaEnabled declaration.contract "aborts_if_is_strict"
  let parameterSlots ← declaration.signature.parameters.mapIdxM fun index parameter =>
    slotOf { unit, namespaceId, ns, locals := #[], results := #[] }
      (Name.mkSimple (if parameter.name.isEmpty then s!"argument{index}" else parameter.name))
      parameter.typeUse.typeId (allowReference := true)
  let resultSlots ← declaration.signature.results.mapIdxM fun index result =>
    slotOf { unit, namespaceId, ns, locals := #[], results := #[] }
      (Name.mkSimple (if declaration.signature.results.size == 1 then "result"
        else s!"result{index}"))
      result.typeId (allowReference := true)
  let runtimeValues := mkApp (mkConst ``Array [.zero]) (mkConst ``LeanerIR.RuntimeValue)
  let runtimeState := mkConst ``LeanerIR.RuntimeState
  let failure := mkConst ``LeanerIR.Proofs.Failure
  /- Bind the argument row, state, and (for `ensures`) result row, then the
  logical slots under them. In a one-state clause a parameter denotes its
  entry value; in `ensures` it denotes the current value — the exit of a
  mutable reference — and `spec.old` reaches the entry. -/
  let contextOf (state : Lean.Expr) (bound : Array SlotBinders) : Context :=
    { unit, namespaceId, ns, twins, families
      locals := bound.map fun binders => some binders.entry
      localTypes := parameterSlots.map (·.physical)
      oldLocals := bound.map fun binders => some binders.entry
      results := #[]
      state := some state, oldState := some state }
  let translateAll (context : Context) (clauses : Array ExprId) :
      MetaM (Array Lean.Expr) :=
    clauses.mapM (translate context)
  let flatten (bound : Array SlotBinders) : Array Lean.Expr :=
    bound.flatMap SlotBinders.flat
  let requiresTerm ← withLocalDeclD `arguments runtimeValues fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withFamilyContents families fun familyBound =>
        withSlotBinders parameterSlots (withExit := false) fun bound => do
          let represented ← familyConjuncts families familyBound state
          let equation ← rowEquation arguments parameterSlots bound
          let ranges ← slotRanges parameterSlots (some ·.entry) bound
          let ownership ← mutableLoanFacts state parameterSlots bound
          let clauses ← translateAll (contextOf state bound) groups.requires
          let body ← conjunction
            (represented ++ #[equation] ++ ranges ++ ownership ++ clauses)
          let closed ← existsOver (familyBound ++ flatten bound) body
          mkLambdaFVars #[arguments, state] closed
  let ensuresTerm ← withLocalDeclD `arguments runtimeValues fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `results runtimeValues fun results =>
        withLocalDeclD `final runtimeState fun final =>
          withSlotBinders parameterSlots (withExit := true) fun parameterBound =>
            withSlotBinders resultSlots (withExit := false) fun resultBound =>
            withGlobalLender state final parameterSlots parameterBound resultSlots resultBound
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
                  locals := parameterBound.map fun binders => some binders.current
                  localTypes := parameterSlots.map (·.physical)
                  oldLocals := parameterBound.map fun binders => some binders.entry
                  results := resultBound.map (·.entry)
                  resultTypes := resultSlots.map (·.physical)
                  state := some globalBound.resolvedFinal, oldState := some state }
              let clauses ← groups.ensures.mapM fun (clause, range) =>
                return markObligation range (← translate context clause)
              let body ← conjunction
                (#[argumentEquation, resultEquation] ++ componentEquations ++ pending.toArray ++
                  globalBound.facts ++ ranges ++ componentRanges ++ exitRanges ++ clauses)
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
              let quantified ←
                withLocalDeclD `key (mkConst ``LeanerIR.GlobalKey) fun key => do
                  let mut implication ← mkAppM ``Eq
                    #[← mkAppM ``LeanerIR.GlobalMap.lookup #[finalGlobals, key],
                      ← mkAppM ``LeanerIR.GlobalMap.lookup #[initialGlobals, key]]
                  for modified in keys.reverse do
                    implication ←
                      mkArrow (← mkAppM ``Ne #[key, modified]) implication
                  mkForallFVars #[key] implication
              existsOver (flatten bound) (← conjunction #[equation, quantified])
        mkLambdaFVars #[arguments, initial, final]
          (← mkAppM ``And #[body, loanDiscipline])
  mkAppOptM ``LeanerIR.Proofs.Contract.mk
    #[some runtimeState, some failure, some runtimeValues, some runtimeValues,
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

/-- Quote the semantically prepared unit as a literal, once per namespace,
with the equations connecting it to `prepareExecution`.  The preparation
runs natively in the elaborator and the kernel certifies the quoted result
by one definitional-equality check, so symbolic execution references the
prepared unit by name: every verify in the namespace shares one
preparation, and goals carry a constant instead of the unit literal. -/
def ensureSemanticsDefinitions (namespaceSegments : Array String)
    (unit : ValidatedUnit) : CommandElabM (Name × Name) := do
  let unitDefinition ← ensureUnitDefinition namespaceSegments unit
  let name := semanticsName namespaceSegments
  let eqName := semanticsEqName namespaceSegments
  let widthEqName := pointerWidthEqName namespaceSegments
  if (← getEnv).contains eqName then return (eqName, widthEqName)
  let prepared := (LeanerIR.Validation.prepareSemantics unit).1
  liftTermElabM do
    addDecl (.defnDecl {
      name, levelParams := []
      type := mkConst ``LeanerIR.Validation.ValidatedUnit
      value := toExpr prepared
      hints := .abbrev, safety := .safe })
    enableRealizationsForConst name
    let preparedTerm ← mkAppM ``Prod.fst
      #[mkApp (mkConst ``LeanerIR.Validation.prepareSemantics)
        (mkConst unitDefinition)]
    addDecl (.thmDecl {
      name := eqName, levelParams := []
      type := ← mkEq preparedTerm (mkConst name)
      value := ← mkEqRefl (mkConst name) })
    let width := LeanerIR.Validation.targetPointerWidth? unit
    let widthTerm := mkApp (mkConst ``LeanerIR.Validation.targetPointerWidth?)
      (mkConst unitDefinition)
    addDecl (.thmDecl {
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
  let value ← liftTermElabM
    (buildContract unit ⟨namespaceIndex⟩ ns declaration twins families)
  let type := mkConst ``LeanerIR.Proofs.FunctionContract
  let signatureResult ← Typed.ensureSignatureDefinitions unit namespaceSegments
    function declaration twins
  match signatureResult with
  | .error reason =>
      logInfo m!"V3 native wrapper unavailable for `{function}`: {reason}; using runtime rows"
      liftTermElabM do
        addDecl (.defnDecl {
          name, levelParams := [], type, value
          hints := .abbrev, safety := .safe })
        enableRealizationsForConst name
  | .ok artifacts =>
      let rawName := rawContractName namespaceSegments function
      liftTermElabM do
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
            ($codecs:ident : ∀ index,
              LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue) :
            LeanerIR.Proofs.Contract LeanerIR.RuntimeState
              LeanerIR.Proofs.Failure $argumentsType $resultsType :=
          LeanerIR.Proofs.Contract.typed
            ($(rootIdent artifacts.argumentsCodec) $codecs)
            ($(rootIdent artifacts.resultsCodec) $codecs) $rawIdent))
        let runtimeCodecs ← `(term|
          fun (_ : Nat) => LeanerIR.Proofs.Codec.identity LeanerIR.RuntimeValue)
        elabCommand (← `(def $publicName:ident :
            LeanerIR.Proofs.FunctionContract :=
          LeanerIR.Proofs.Contract.runtime
            ($(rootIdent artifacts.argumentsCodec) $runtimeCodecs)
            ($(rootIdent artifacts.resultsCodec) $runtimeCodecs)
            ($typedName $runtimeCodecs)))
  return name

@[command_elab leanerContractCommand]
def elabLeanerContract : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let (namespaceSegments, function, unit, namespaceIndex, ns, _, declaration) ←
    resolvePath pathSyntax
  discard <| ensureContractDefinition namespaceSegments function unit namespaceIndex
    ns declaration

/-- Report every residual verification condition at its authored clause and
fail.  A goal that survives the automatic finish still carries the
`Obligation` markers of the clauses it obligates; each marker's byte range
locates the clause in the source file.  With no residual goals this is a
no-op, so the generated script always ends with it. -/
elab "leaner_report" : tactic => do
  let goals ← Tactic.getUnsolvedGoals
  if goals.isEmpty then return
  for goal in goals do
    LeanerIR.Proofs.Certify.reportObligation goal
    if leaner.rowDebug.get (← getOptions) then
      goal.withContext do
        logError m!"residual obligation:\n{(← Lean.Meta.ppGoal goal)}"
  throwError "leaner verification failed"

/-- Name of the V3 theorem over the generated native signature. -/
def typedVerifiedName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedVerified"

/-- Destructure a generated argument record and the representation wrappers
whose projections should disappear before arithmetic.  In particular, a
mutable integer becomes its loan id, mathematical value, and range proof;
`omega` then sees one atom instead of alternate projection spellings of the
same `SpecInt`. -/
elab "leaner_native_cases" hypothesis:ident : tactic => do
  let isNativeWrapper (type : Lean.Expr) : Bool :=
    type.getAppFn.constName? == some ``LeanerIR.SpecInt ||
      type.getAppFn.constName? == some ``LeanerIR.Proofs.MutableArgument
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
      (fuel : Nat) : Lean.MetaM Lean.MVarId := do
    match fuel with
    | 0 => return goal
    | fuel + 1 =>
        let some (field, base) := fields[0]? | return goal
        let rest := fields.extract 1 fields.size
        let type ← goal.withContext do
          Lean.instantiateMVars (← field.getType)
        if !isNativeWrapper type then
          destructWrappers goal rest fuel
        else
          let subnames : Array String :=
            if type.getAppFn.constName? == some ``LeanerIR.Proofs.MutableArgument then
              #[s!"{base}_loan", base]
            else
              #[base, s!"{base}_fits"]
          match ← goal.cases field with
          | #[subgoal] =>
              let (goal, named) ← nameFields subgoal.mvarId subgoal.fields subnames
              destructWrappers goal (rest ++ named) fuel
          | _ => return goal
  Lean.Elab.Tactic.liftMetaTactic1 fun goal => do
    let declaration ← Lean.Meta.getLocalDeclFromUserName hypothesis.getId
    let structName? ← goal.withContext do
      pure (← Lean.instantiateMVars declaration.type).getAppFn.constName?
    let env ← Lean.getEnv
    let fieldNames := match structName? with
      | some structName => (Lean.getStructureFields env structName).map (·.toString)
      | none => #[]
    match ← goal.cases declaration.fvarId with
    | #[subgoal] =>
        let (goal, named) ← nameFields subgoal.mvarId subgoal.fields fieldNames
        destructWrappers goal named 64
    | _ => return goal
  /- Reduce the projection redexes the substitution leaves in retained
  facts, so `omega` sees the destructured components as atoms.  The goal is
  left to the drive's own normalization order. -/
  let goal ← Lean.Elab.Tactic.getMainGoal
  let hypotheses ← goal.withContext do
    let mut found : Array (Lean.TSyntax `ident) := #[]
    for declaration in ← Lean.getLCtx do
      if declaration.isImplementationDetail then continue
      found := found.push ⟨(Lean.mkIdent declaration.userName).raw⟩
    pure found
  unless hypotheses.isEmpty do
    Lean.Elab.Tactic.evalTactic
      (← `(tactic| try simp only [] at $hypotheses*))
  Lean.Elab.Tactic.evalTactic (← `(tactic| try leaner_certify!))
  Lean.Elab.Tactic.evalTactic (← `(tactic| leaner_name_facts))

/-- The errors a message log holds, reported or not. -/
private def countErrors (log : MessageLog) : Nat :=
  log.reportedPlusUnreported.toList.filter (·.severity == .error) |>.length

/-- Prove the native V3 view directly over the shallow denotation.  Generic
functions quantify over the abstract carrier and its certified codec; the
proof is shared by every concrete instantiation. -/
private def ensureTypedVerificationTheorem (reference : Syntax)
    (namespaceSegments : Array String) (function : String)
    (namespaceIndex functionIndex : Nat)
    (unitDefinition semanticsEq widthEq : Name)
    (preparedUnit : ValidatedUnit) (preparedNs : ValidatedNamespace)
    (preparedDeclaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (generated : LeanerLang.Denotation.Generated)
    (artifacts : Typed.Artifacts)
    (twins : Array SpecTypes.TwinInfo)
    (script? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq)) : CommandElabM Name := do
  let theoremName := typedVerifiedName namespaceSegments function
  let qualifiedTheoremName := (← getCurrNamespace) ++ theoremName
  if (← getEnv).contains qualifiedTheoremName then return theoremName
  let typedContract := typedContractName namespaceSegments function
  let rawContract := rawContractName namespaceSegments function
  let loopInvariants ← liftTermElabM <|
    buildLoopInvariants preparedUnit ⟨namespaceIndex⟩ preparedNs
      preparedDeclaration twins
  for (site, predicate) in loopInvariants do
    let localName := Name.str (Name.str (pathName namespaceSegments) function)
      s!"loopInvariant_{site.index}"
    let qualifiedName := (← getCurrNamespace) ++ localName
    unless (← getEnv).contains qualifiedName do
      liftTermElabM do
        let predicate ← Lean.instantiateMVars predicate
        let type ← Lean.instantiateMVars (← Lean.Meta.inferType predicate)
        addDecl (.defnDecl {
          name := qualifiedName, levelParams := [], type, value := predicate
          hints := .abbrev, safety := .safe })
        enableRealizationsForConst qualifiedName
  /- The frame-free route, generated per body from its combinator tree
  when the tree is inside the modeled subset.  The frame script stays
  behind it: a generation gap costs one failed deterministic attempt,
  visible in the benchmark rather than silent. -/
  /- A recursive body is planned with its recursive calls resolved to the
  function's own relation: the plan reads the tree, and the script it
  emits consumes the recursive call's contract from the induction
  hypothesis (`RowScript.CallerNames.selfRelation`). -/
  let bodyValue? ← match generated.openBody with
    | none => pure ((← getEnv).find? generated.body |>.bind (·.value?))
    | some openBody =>
        match (← getEnv).find? openBody |>.bind (·.value?) with
        | none => pure none
        | some openValue => liftTermElabM do
            Lean.Meta.lambdaTelescope openValue fun binders tree => do
              let #[executable, self] := binders | return none
              let closed := tree.replaceFVar self
                (Lean.mkApp (Lean.mkConst generated.body) executable)
              return some (← Lean.Meta.mkLambdaFVars #[executable] closed)
  let rowAlternative? ← do
    match bodyValue? with
    | none => pure none
    | some bodyValue =>
        if artifacts.signature.typeParameterCount != 0 then
          /- A generic body's theorem is over abstract codecs; the one
          generic shape generated is the move-out. -/
          let shape := LeanerIR.SemanticOperations.FunctionShape.ofDeclaration
            preparedDeclaration
          let parameters := preparedDeclaration.signature.parameters.zipIdx.map
            fun (parameter, index) =>
              if parameter.name.isEmpty then s!"argument{index}" else parameter.name
          let genericCaller : RowScript.CallerNames :=
            { rawContract := ← `(term| $(rootIdent rawContract):term)
              resultsCodec := ← `(term| $(rootIdent artifacts.resultsCodec):term)
              argumentsCodec := ← `(term| $(rootIdent artifacts.argumentsCodec):term)
              shape := ← `(term| $(mkIdent generated.shape):term)
              parameters }
          match RowScript.genericCallPlan? bodyValue shape.parameterCount
              shape.localCount shape.resultCount (parameters.getD 0 "value") with
          | some genericCallPlan =>
              let script ← RowScript.genericCallScript genericCaller genericCallPlan
              pure (some script)
          | none =>
          match RowScript.genericPlan? bodyValue shape.parameterCount
              shape.localCount shape.resultCount (parameters.getD 0 "value") with
          | none => pure none
          | some genericPlan =>
              let script ← RowScript.genericScript
                { rawContract := ← `(term| $(rootIdent rawContract):term)
                  resultsCodec := ← `(term| $(rootIdent artifacts.resultsCodec):term)
                  argumentsCodec := ← `(term| $(rootIdent artifacts.argumentsCodec):term)
                  shape := ← `(term| $(mkIdent generated.shape):term)
                  parameters }
                genericPlan
              pure (some script)
        else
        let shape := LeanerIR.SemanticOperations.FunctionShape.ofDeclaration
          preparedDeclaration
        let families := SpecTypes.familyInfos preparedUnit twins
        let currentNamespace ← getCurrNamespace
        let caller : RowScript.CallerNames :=
          { rawContract := ← `(term| $(rootIdent rawContract):term)
            resultsCodec := ← `(term| $(rootIdent artifacts.resultsCodec):term)
            argumentsCodec := ← `(term| $(rootIdent artifacts.argumentsCodec):term)
            shape := ← `(term| $(mkIdent generated.shape):term)
            parameters := preparedDeclaration.signature.parameters.zipIdx.map
              fun (parameter, index) =>
                if parameter.name.isEmpty then s!"argument{index}" else parameter.name
            requiresCount := preparedDeclaration.contract.conditions.foldl
              (fun count condition =>
                match condition.kind with
                | .requires => count + 1
                | _ => count) 0
            loopInvariant := fun site => mkIdent (rootNamespace ++ currentNamespace ++ Name.str
              (Name.str (pathName namespaceSegments) function) s!"loopInvariant_{site}")
            selfRelation := if generated.openBody.isSome then some generated.relation
              else none }
        /- A plain parameter's kind: an integer (`true`) or a `Bool`.  A
        shared reference to one is that value at the boundary — its codec
        encodes the referent — so it has the referent's kind. -/
        let rec plainKind? (typeId : LeanerIR.TypeId) (fuel : Nat) : Option Bool :=
          match fuel with
          | 0 => none
          | fuel + 1 =>
            match preparedUnit.tables.types[typeId.index]? with
            | some (.integer _ _) => some true
            | some .bool => some false
            | some (.reference reference) =>
                if reference.kind == .shared then plainKind? reference.referent fuel
                else none
            | _ => none
        /- Two mutable scalar parameters: the second is a mutable reference
        to an integer, like the first. -/
        let secondBorrow := match preparedDeclaration.signature.parameters[1]? with
          | some parameter =>
              match preparedUnit.tables.types[parameter.typeUse.typeId.index]? with
              | some (.reference reference) =>
                  reference.kind == .mutable &&
                    match preparedUnit.tables.types[reference.referent.index]? with
                    | some (.integer _ _) => true
                    | _ => false
              | _ => false
          | none => false
        /- Every parameter's kind, for the routes reading the whole
        signature. -/
        let parameterKind (parameter : LeanerIR.Parameter) : RowScript.ParamKind :=
          match preparedUnit.tables.types[parameter.typeUse.typeId.index]? with
          | some (.integer _ _) => RowScript.ParamKind.integer
          | some .bool => RowScript.ParamKind.bool
          | some (.reference reference) =>
              if reference.kind == .mutable then
                match preparedUnit.tables.types[reference.referent.index]? with
                | some (.integer _ _) => RowScript.ParamKind.borrow
                | _ => RowScript.ParamKind.other
              else
                match plainKind? reference.referent 8 with
                | some true => RowScript.ParamKind.integer
                | some false => RowScript.ParamKind.bool
                | none => RowScript.ParamKind.other
          | _ => RowScript.ParamKind.other
        let parameterKinds : Array RowScript.ParamKind :=
          preparedDeclaration.signature.parameters.map parameterKind
        /- The value route reads an integer parameter's facts. -/
        let integerParameter := preparedDeclaration.signature.parameters.all fun parameter =>
          plainKind? parameter.typeUse.typeId 8 == some true
        /- The plain rows hold integers and `Bool`s, by kind. -/
        let plainKinds : Option (Array Bool) :=
          preparedDeclaration.signature.parameters.mapM fun parameter =>
            plainKind? parameter.typeUse.typeId 8
        /- The scalar rows hold one borrow in front of plain integers. -/
        let tailKinds : Option (Array Bool) :=
          (preparedDeclaration.signature.parameters.extract 1
            preparedDeclaration.signature.parameters.size).mapM fun parameter =>
            plainKind? parameter.typeUse.typeId 8
        /- A reference-returning callee is consumed only when its summary
        states the transfer of its lender's prophecy. -/
        let env ← getEnv
        let statesTransfer (rawContract : Name) : Bool :=
          match env.find? rawContract |>.bind (·.value?) with
          | some value => (value.find? (·.isConstOf ``LeanerIR.RuntimeValue.loanHole)).isSome
          | none => false
        let twinOf (namespaceIndex typeIndex : Nat) : Option (Name × Nat) := do
          let family ← families.find? (·.typeIndex == typeIndex)
          guard (family.info.namespaceIndex == namespaceIndex)
          pure (rootNamespace ++ family.info.twin, family.info.structIndex)
        /- A value callee is consumed at the unit carrier: its agreement
        quantifies over one. -/
        let genericCallee (agreement : Name) : CommandElabM Bool := do
          let rec quantifiesCarrier : Lean.Expr → Bool
            | .forallE name _ body _ =>
                name.eraseMacroScopes == `Carrier || quantifiesCarrier body
            | _ => false
          match ← liftCoreM (Lean.resolveGlobalName agreement) with
          | (constant, _) :: _ =>
              match env.find? constant with
              | some info => pure (quantifiesCarrier info.type)
              | none => pure false
          | [] => pure false
        /- A callee is consumed through its verified contract; one without
        a contract is run inline, by the call's own semantics, when its
        body is inside the scalar subset and takes one plain parameter. -/
        let inlineCallee? (relation : Name) (handle : Nat × Nat) :
            Option RowScript.InlineCallee := do
          let names ← RowScript.calleeNames? relation
          let (namespaceIndex, functionIndex) := handle
          let calleeNamespace ← preparedUnit.namespaces[namespaceIndex]?
          let calleeDeclaration ← calleeNamespace.functions[functionIndex]?
          guard calleeDeclaration.contract.conditions.isEmpty
          guard (calleeDeclaration.signature.parameters.size == 1)
          guard (calleeDeclaration.locals.size == 1)
          let parameter ← calleeDeclaration.signature.parameters[0]?
          let integerParameter ← match preparedUnit.tables.types[parameter.typeUse.typeId.index]? with
            | some (.integer _ _) => some true
            | some .bool => some false
            | _ => none
          let calleeBody ← env.find? names.body |>.bind (·.value?)
          let events ← RowScript.calleeEvents? calleeBody
          pure { names, events, integerParameter }
        /- A callee with a contract is consumed modularly at a spine call
        when its contract is in the destructured grammar — one integer or
        `Bool` parameter, one integer result, one `ensures` clause — and
        it is verified, or is the function itself under induction. -/
        let modularCallee? (relation : Name) (handle : Nat × Nat) :
            CommandElabM (Option RowScript.ModularCallee) := do
          let some names := RowScript.calleeNames? relation | return none
          let (namespaceIndex, functionIndex) := handle
          let some calleeNamespace := preparedUnit.namespaces[namespaceIndex]? | return none
          let some calleeDeclaration := calleeNamespace.functions[functionIndex]?
            | return none
          let conditions := calleeDeclaration.contract.conditions
          let ensuresCount := conditions.foldl (fun count condition =>
            match condition.kind with
            | .ensures => count + 1
            | _ => count) 0
          unless ensuresCount == 1 && conditions.all (fun condition =>
              match condition.kind with
              | .ensures | .abortsIf => true
              | _ => false) do
            return none
          unless calleeDeclaration.signature.parameters.size == 1 &&
              calleeDeclaration.signature.results.size == 1 do
            return none
          let some parameter := calleeDeclaration.signature.parameters[0]? | return none
          let some integerParameter :=
              (match preparedUnit.tables.types[parameter.typeUse.typeId.index]? with
                | some (.integer _ _) => some true
                | some .bool => some false
                | _ => none) | return none
          let some result := calleeDeclaration.signature.results[0]? | return none
          let some (.integer _ _) := preparedUnit.tables.types[result.typeId.index]?
            | return none
          if caller.selfRelation == some relation then
            return some { names, integerParameter }
          match ← liftCoreM (Lean.resolveGlobalName names.verified) with
          | [] => return none
          | _ => return some { names, integerParameter }
        let spinePlan? : CommandElabM (Option RowScript.SpinePlan) := do
          let some kinds := plainKinds | return none
          let some spine := RowScript.spine? bodyValue.bindingBody! | return none
          let calls := spine.calls
          if calls.isEmpty then return none
          let mut callees : Array (Name × RowScript.SpineCallee) := #[]
          for (relation, handle) in calls do
            match inlineCallee? relation handle with
            | some callee => callees := callees.push (relation, .inline callee)
            | none =>
                match ← modularCallee? relation handle with
                | some callee => callees := callees.push (relation, .modular callee)
                | none => return none
          return some { spine, parameterCount := shape.parameterCount,
                        localCount := shape.localCount, kinds, callees }
        let spinePlan? ← spinePlan?
        let valuePlan? ← match RowScript.valuePlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount plainKinds with
          | some valuePlan => do
              if caller.selfRelation == some valuePlan.callee.relation then
                /- The recursive call: its contract is the induction
                hypothesis, at the function's own codecs. -/
                pure (some { valuePlan with generic := false })
              else
              match ← liftCoreM (Lean.resolveGlobalName valuePlan.callee.verified) with
              | [] => pure none
              | _ =>
                  let generic ← genericCallee valuePlan.callee.agreement
                  pure (some { valuePlan with generic })
          | none => pure none
        match valuePlan? with
        | some valuePlan =>
            let script ← if valuePlan.generic then RowScript.valueScript caller valuePlan
              else RowScript.valueScriptMono caller valuePlan
            pure (some script)
        | none =>
        match spinePlan? with
        | some spinePlan =>
            let script ← RowScript.spineScript caller spinePlan
            pure (some script)
        | none =>
        /- The parameter's twin, when it borrows a struct of integers. -/
        let parameterTwin? : Option (Name × Nat × Nat × Nat) := do
          let parameter ← preparedDeclaration.signature.parameters[0]?
          let .reference reference ← preparedUnit.tables.types[parameter.typeUse.typeId.index]?
            | none
          guard (reference.kind == .mutable)
          let .nominal name _ ← preparedUnit.tables.types[reference.referent.index]? | none
          let qualified ← preparedUnit.tables.names[name.index]?
          let info ← twins.find? (·.qualified == qualified)
          guard (info.fields.all fun (_, rep) => match rep with
            | .int _ _ => true
            | _ => false)
          pure (rootNamespace ++ info.twin, info.namespaceIndex, info.structIndex,
            info.fields.size)
        /- The field a callee's returned reborrow projects from its first
        parameter, read off its carried loan. -/
        let calleeFocus? (handle : Nat × Nat) : Option Nat := do
          let (namespaceIndex, functionIndex) := handle
          let calleeNamespace ← preparedUnit.namespaces[namespaceIndex]?
          let calleeDeclaration ← calleeNamespace.functions[functionIndex]?
          let lenders := reborrowLenders preparedUnit ⟨namespaceIndex⟩ calleeNamespace twins
            calleeDeclaration
          let lender ← lenders.find? (·.parameter == 0)
          let focus ← lender.focus
          pure focus.field
        let twinInfoOf (namespaceIndex structIndex : Nat) :
            Option (Name × Array (String × SpecTypes.FieldRep)) := do
          let info ← twins.find? fun info =>
            info.namespaceIndex == namespaceIndex && info.structIndex == structIndex
          pure (rootNamespace ++ info.twin, info.fields)
        /- The family, resource struct, and field a callee's returned
        reborrow projects from storage, read off its global lender. -/
        let calleeGlobalLender? (handle : Nat × Nat) : Option (Nat × Nat × Nat × Nat) := do
          let (namespaceIndex, functionIndex) := handle
          let calleeNamespace ← preparedUnit.namespaces[namespaceIndex]?
          let calleeDeclaration ← calleeNamespace.functions[functionIndex]?
          let lender ← globalReborrowLender? preparedUnit ⟨namespaceIndex⟩ calleeNamespace twins
            families calleeDeclaration
          pure (lender.family.info.namespaceIndex, lender.family.typeIndex,
            lender.focus.twin.structIndex, lender.focus.field)
        match RowScript.globalSetPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount calleeGlobalLender? twinOf twinInfoOf with
        | some globalSetPlan =>
            if caller.requiresCount > 0 then
              pure (some (← RowScript.globalSetScript caller globalSetPlan))
            else pure none
        | none =>
        match RowScript.projectedSetPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount parameterTwin? calleeFocus? with
        | some projectedSetPlan =>
            pure (some (← RowScript.projectedSetScript caller projectedSetPlan))
        | none =>
        /- The whole-reborrow routes read an integer through the parameter. -/
        let integerBorrow := parameterKinds[0]? == some RowScript.ParamKind.borrow
        match if integerBorrow then RowScript.setPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount statesTransfer else none with
        | some setPlan =>
            let script ← RowScript.setScript
              caller
              setPlan
            pure (some script)
        | none =>
        match if integerBorrow then RowScript.forwardPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount statesTransfer else none with
        | some forwardPlan =>
            let script ← RowScript.forwardScript
              caller
              forwardPlan
            pure (some script)
        | none =>
        match RowScript.returnPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount with
        | some returnPlan =>
            let script ← RowScript.returnScript
              caller
              returnPlan
            pure (some script)
        | none =>
        let structOf (namespaceIndex structIndex : Nat) : Option (Name × Array String) := do
          let info ← twins.find? fun info =>
            info.namespaceIndex == namespaceIndex && info.structIndex == structIndex
          pure (rootNamespace ++ info.twin, info.fields.map (·.1))
        match RowScript.readPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount twinOf structOf with
        | some readPlan =>
            let script ← RowScript.readScript
              caller
              readPlan
            pure (some script)
        | none =>
        match RowScript.takePlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount twinOf twinInfoOf structOf with
        | some takePlan =>
            pure (some (← RowScript.takeScript caller takePlan))
        | none =>
        match RowScript.globalReturnPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount twinOf twinInfoOf with
        | some globalReturnPlan =>
            if caller.requiresCount > 0 then
              pure (some (← RowScript.globalReturnScript caller globalReturnPlan))
            else pure none
        | none =>
        match RowScript.projectPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount twinInfoOf with
        | some projectPlan =>
            pure (some (← RowScript.projectScript caller projectPlan))
        | none =>
        match RowScript.choosePlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount parameterKinds with
        | some choosePlan =>
            pure (some (← RowScript.chooseScript caller choosePlan))
        | none =>
        match RowScript.setPairPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount parameterKinds statesTransfer with
        | some setPairPlan =>
            pure (some (← RowScript.setPairScript caller setPairPlan))
        | none =>
        match RowScript.returnPairPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount parameterKinds with
        | some returnPairPlan =>
            pure (some (← RowScript.returnPairScript caller returnPairPlan))
        | none =>
        match RowScript.fieldCallPlan? bodyValue shape.parameterCount
            shape.localCount twinOf twinInfoOf with
        | some fieldCallPlan =>
            if caller.requiresCount > 0 then
              pure (some (← RowScript.fieldCallScript caller fieldCallPlan))
            else pure none
        | none =>
        match RowScript.storePlan? bodyValue shape.parameterCount
            shape.localCount twinOf twinInfoOf with
        | some storePlan =>
            let script ← RowScript.storeScript
              caller
              storePlan
            pure (some script)
        | none =>
        /- A callee's `ensures` clauses, each an obligation conjunct of its
        summary a call site destructures. -/
        let clauseCountOf (handle : Nat × Nat) : Nat :=
          let (namespaceIndex, functionIndex) := handle
          match preparedUnit.namespaces[namespaceIndex]?.bind
              (·.functions[functionIndex]?) with
          | some calleeDeclaration =>
              (calleeDeclaration.contract.conditions.filter (·.kind == .ensures)).size
          | none => 1
        match RowScript.callPlan? bodyValue shape.parameterCount
            shape.localCount tailKinds clauseCountOf secondBorrow with
        | some callPlan =>
            let script ← RowScript.callScript
              caller
              callPlan
            pure (some script)
        | none =>
        /- The parameter's twin, when it is a struct passed by value. -/
        let valueTwin? : Option Name := do
          let parameter ← preparedDeclaration.signature.parameters[0]?
          let .nominal name _ ← preparedUnit.tables.types[parameter.typeUse.typeId.index]?
            | none
          let qualified ← preparedUnit.tables.names[name.index]?
          let info ← twins.find? (·.qualified == qualified)
          pure (rootNamespace ++ info.twin)
        match RowScript.nominalReturnPlan? bodyValue shape.parameterCount
            shape.localCount shape.resultCount valueTwin? with
        | some nominalPlan =>
            let script ← RowScript.nominalReturnScript caller nominalPlan
            pure (some script)
        | none =>
        match RowScript.plan? bodyValue shape.parameterCount
            shape.localCount tailKinds plainKinds secondBorrow with
        | none => pure none
        | some plan =>
            let script ← RowScript.script caller plan
            pure (some script)
  /- The proof is the generated row script, or an authored script in its
  place.  A body no route recognizes is an error at the `verify`: there is
  no slower proof behind the generated one. -/
  let rowTactic ← match rowAlternative? with
    | some tactic =>
        if leaner.rowDebug.get (← getOptions) then
          logInfo m!"row script for {theoremName}:\n{tactic}"
        pure (some tactic)
    | none => pure none
  let tailTactic ← match script?, rowTactic with
    | some script, _ => `(tactic| ($script:tacticSeq))
    | none, some tactic => pure tactic
    | none, none =>
        throwErrorAt reference m!"no generated route for `{function}`: \
          the body is outside the generated subset"
  let genericTail := tailTactic
  let publicContract := contractName namespaceSegments function
  let command ← if artifacts.signature.typeParameterCount == 0 &&
      generated.openBody.isSome then
    /- A recursive function: the fixed point by induction — the body is
    proved under the hypothesis that its recursive calls satisfy the
    contract, which the row script consumes at those calls exactly as it
    consumes a verified callee. -/
    let budget := Syntax.mkNumLit (toString (leaner.verifyHeartbeats.get (← getOptions)))
    let some openBody := generated.openBody | unreachable!
    let some selfNames := RowScript.calleeNames? generated.relation
      | throwErrorAt reference m!"the relation of `{function}` has no callee names"
    `(command|
      set_option Elab.async false in
      set_option maxRecDepth 100000 in
      set_option maxHeartbeats $budget:num in
      theorem $(mkIdent theoremName)
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          ($(mkIdent `prepared):ident : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) :
          LeanerIR.Proofs.Satisfies
            ($(rootIdent artifacts.denotation) executable)
            $(rootIdent typedContract) := by
        have $(mkIdent `hu):ident :=
          (LeanerIR.Validation.prepareExecution_unit $(mkIdent `prepared):ident).trans
          $(mkIdent semanticsEq)
        have $(mkIdent `hw):ident :=
          (LeanerIR.Validation.prepareExecution_targetPointerWidth $(mkIdent `prepared):ident).trans
            $(mkIdent widthEq)
        have $(mkIdent `hn):ident : executable.unit.namespaces[$(Syntax.mkNatLit namespaceIndex)]? =
            some $(mkIdent generated.namespaceDef) := by
          rw [$(mkIdent `hu):ident]
          rfl
        simp only [$(rootIdent artifacts.denotation):term,
          $(mkIdent generated.denotation):term, $(mkIdent generated.body):term]
        apply LeanerIR.Proofs.Denotation.satisfies_typed_fixBody
        intro $(mkIdent `self):ident $(mkIdent `selfSatisfies):ident
        have $(RowScript.calleeSatIdent selfNames):ident :
            LeanerIR.Proofs.Satisfies
              (LeanerIR.Proofs.Denotation.nativeFunction executable
                $(mkIdent generated.shape) $(mkIdent `self):ident)
              $(rootIdent publicContract) := by
          simpa only [$(rootIdent publicContract):term] using
            (LeanerIR.Proofs.satisfies_runtime
              $(rootIdent artifacts.argumentsCodec)
              $(rootIdent artifacts.resultsCodec)
              (LeanerIR.Proofs.Denotation.nativeFunction executable
                $(mkIdent generated.shape) $(mkIdent `self):ident)
              $(rootIdent typedContract)
              $(mkIdent `selfSatisfies):ident)
        apply LeanerIR.Proofs.satisfies_of_wp
        intro arguments $(mkIdent `initial):ident permitted
        simp only [$(rootIdent typedContract):term,
          LeanerIR.Proofs.Contract.typed] at permitted ⊢
        simp [$(rootIdent rawContract):term,
          $(rootIdent artifacts.argumentsCodec):term, lir_data_norm] at permitted
        leaner_cases permitted
        all_goals
          (leaner_native_cases arguments
           rw [LeanerIR.Proofs.wp_typedFunction]
           simp only [$(mkIdent openBody):term]
           rw [LeanerIR.Proofs.Denotation.wp_nativeFunction]
           $tailTactic:tactic
           leaner_report))
  else if artifacts.signature.typeParameterCount == 0 then
    let budget := Syntax.mkNumLit (toString (leaner.verifyHeartbeats.get (← getOptions)))
    `(command|
      set_option Elab.async false in
      set_option maxRecDepth 100000 in
      set_option maxHeartbeats $budget:num in
      theorem $(mkIdent theoremName)
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          ($(mkIdent `prepared):ident : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) :
          LeanerIR.Proofs.Satisfies
            ($(rootIdent artifacts.denotation) executable)
            $(rootIdent typedContract) := by
        /- Raw names: the spliced row script consumes the callee facts these
        two establish, and a name written inside this quotation would carry
        a macro scope the splice cannot see. -/
        have $(mkIdent `hu):ident :=
          (LeanerIR.Validation.prepareExecution_unit $(mkIdent `prepared):ident).trans
          $(mkIdent semanticsEq)
        have $(mkIdent `hw):ident :=
          (LeanerIR.Validation.prepareExecution_targetPointerWidth $(mkIdent `prepared):ident).trans
            $(mkIdent widthEq)
        have $(mkIdent `hn):ident : executable.unit.namespaces[$(Syntax.mkNatLit namespaceIndex)]? =
            some $(mkIdent generated.namespaceDef) := by
          rw [$(mkIdent `hu):ident]
          rfl
        apply LeanerIR.Proofs.satisfies_of_wp
        intro arguments $(mkIdent `initial):ident permitted
        simp only [$(rootIdent typedContract):term,
          LeanerIR.Proofs.Contract.typed] at permitted ⊢
        simp [$(rootIdent rawContract):term,
          $(rootIdent artifacts.argumentsCodec):term, lir_data_norm] at permitted
        leaner_cases permitted
        /- A `Bool` argument's contract enumerates its values: one goal per
        alternative, each proved by the same script. -/
        all_goals
          (leaner_native_cases arguments
           simp only [$(rootIdent artifacts.denotation):term]
           rw [LeanerIR.Proofs.wp_typedFunction]
           simp only [$(mkIdent generated.denotation):term,
             $(mkIdent generated.body):term]
           rw [LeanerIR.Proofs.Denotation.wp_nativeFunction]
           $tailTactic:tactic
           leaner_report))
  else
    let carrier := mkIdent `Carrier
    let codecs := mkIdent `codecs
    let budget := Syntax.mkNumLit (toString (leaner.verifyHeartbeats.get (← getOptions)))
    `(command|
      set_option Elab.async false in
      set_option maxRecDepth 100000 in
      set_option maxHeartbeats $budget:num in
      theorem $(mkIdent theoremName)
          {$carrier:ident : Nat → Type}
          ($codecs:ident : ∀ index,
            LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue)
          {registry : LeanerIR.Validation.SemanticsRegistry}
          {executable : LeanerIR.Validation.ExecutableUnit}
          (prepared : LeanerIR.Validation.prepareExecution registry
            $(mkIdent unitDefinition) = .ok executable) :
          LeanerIR.Proofs.Satisfies
            ($(rootIdent artifacts.denotation) $codecs executable)
            ($(rootIdent typedContract) $codecs) := by
        have hu := (LeanerIR.Validation.prepareExecution_unit prepared).trans
          $(mkIdent semanticsEq)
        have $(mkIdent `hw):ident :=
          (LeanerIR.Validation.prepareExecution_targetPointerWidth prepared).trans
            $(mkIdent widthEq)
        have $(mkIdent `hn):ident : executable.unit.namespaces[$(Syntax.mkNatLit namespaceIndex)]? =
            some $(mkIdent generated.namespaceDef) := by
          rw [hu]
          rfl
        apply LeanerIR.Proofs.satisfies_of_wp
        intro arguments $(mkIdent `initial):ident permitted
        simp only [$(rootIdent typedContract):term,
          LeanerIR.Proofs.Contract.typed] at permitted ⊢
        simp [$(rootIdent rawContract):term,
          $(rootIdent artifacts.argumentsCodec):term, lir_data_norm] at permitted
        leaner_cases permitted
        leaner_native_cases arguments
        simp only [$(rootIdent artifacts.denotation):term]
        rw [LeanerIR.Proofs.wp_typedFunction]
        simp only [$(mkIdent generated.denotation):term,
          $(mkIdent generated.body):term]
        rw [LeanerIR.Proofs.Denotation.wp_nativeFunction]
        $genericTail:tactic
        leaner_report)
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
  let carrierIdent := Lean.mkIdent `Carrier
  let agreementApplication ← if generated.typeParameterCount == 0 &&
      generated.requiresUnitEquality then
    `(term| $(mkIdent generated.agreement) hu)
  else if generated.typeParameterCount == 0 then
    `(term| $(mkIdent generated.agreement) hn)
  else if generated.requiresUnitEquality then
    `(term| $(mkIdent generated.agreement)
      ($carrierIdent := fun _ => PUnit) hu)
  else
    `(term| $(mkIdent generated.agreement)
      ($carrierIdent := fun _ => PUnit) hn)
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
        let $runtimeCodecs:ident : ∀ _ : Nat,
            LeanerIR.Proofs.Codec LeanerIR.RuntimeValue LeanerIR.RuntimeValue :=
          fun _ => LeanerIR.Proofs.Codec.identity LeanerIR.RuntimeValue
        simpa only [$(rootIdent publicContract):term] using
          (LeanerIR.Proofs.satisfies_runtime
            ($(rootIdent artifacts.argumentsCodec) $runtimeCodecs)
            ($(rootIdent artifacts.resultsCodec) $runtimeCodecs)
            ($(mkIdent generated.denotation) executable)
            ($(rootIdent (typedContractName namespaceSegments function))
              $runtimeCodecs)
            ($(mkIdent theoremName)
              ($carrierIdent := fun _ => LeanerIR.RuntimeValue)
              $runtimeCodecs prepared)))
  try
    Perf.measure s!"{pathName namespaceSegments}::{function} transport"
      ((← getCurrNamespace) ++ verifiedName) (elabCommand runtimeCommand)
  catch error =>
    throwErrorAt reference m!"failed to transport V3 theorem for `{function}`:\n{error.toMessageData}"
  return theoremName

/-- Elaborate the verification theorem of one function of a registered
unit: materialize the unit, its prepared semantics, its native denotation
and the generated contract, then prove the typed theorem over the native
denotation and transport it to the ordinary theorem
`<path>.<function>.verified`.  Failure is an error at the elaborating
command: a body without a native denotation, a body no generated route
recognizes, or a clause the generated script cannot establish, reported at
its authored range.  An authored script replaces the generated row script
after the shared entry (`leaner_cases`, `leaner_native_cases`, the native
`wp`), and reaches the facts by their contract-derived names. -/
def verifyFunction (reference : Syntax) (namespaceSegments : Array String)
    (function : String)
    (script? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) := none) :
    CommandElabM Unit := do
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
  discard <| ensureTypedVerificationTheorem reference namespaceSegments function
    namespaceIndex functionIndex unitDefinition semanticsEq widthEq preparedUnit
    preparedNs preparedDeclaration generated artifacts twins script?

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

def elabLeanerRequireNative : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let (namespaceSegments, function, unit, namespaceIndex, _, functionIndex, _) ←
    resolvePath pathSyntax
  let currentNamespace ← getCurrNamespace
  let theoremName :=
    currentNamespace ++ typedVerifiedName namespaceSegments function
  if (← getEnv).contains theoremName then return
  let preparedUnit := (LeanerIR.Validation.prepareSemantics unit).1
  let diagnostic := match LeanerLang.Denotation.checkFunction preparedUnit {
      namespaceId := ⟨namespaceIndex⟩, functionId := ⟨functionIndex⟩ } with
    | .ok _ => "native denotation eligibility succeeded, but no typed proof theorem was emitted"
    | .error reason => s!"native denotation rejected the function: {reason}"
  throwErrorAt pathSyntax
    m!"`{pathName namespaceSegments}::{function}` was not verified through the native path; {diagnostic}"

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
