-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.SemanticTypes

/-!
# Static typing

A total checker over validated LIR whose acceptance a proof can use
(`designs/static-typing.md`, Phase 1). Each node is checked by a total
`Bool` function over semantic types: the node's stored type and its
children's, resolved under the body's generic environment, where each type
binder is its rigid parameter. A function's body is walked from its root,
and acceptance states that every node it runs satisfies its check.
-/

namespace LeanerIR.Validation.StaticTyping

/-- A type of a namespace's table resolved under an environment, within as
many nested nodes as the table has. -/
def resolveIn (ns : ValidatedNamespace) (env : Array SemArg) (typeId : TypeId) : Option SemTy :=
  SemTy.resolveFuel ns.tables env (ns.tables.types.size + 1) typeId

/-- A generic body's environment: each type binder its rigid parameter. Any
other binder shapes no value. -/
def staticEnv (generics : Array GenericBinder) : Array SemArg :=
  generics.mapIdx fun index binder => match binder.kind with
    | .typeArg => .type (.param index)
    | .const | .lifetime | .evidence => .erased

/-- An instantiation's semantic generic arguments under an environment. -/
def resolveArguments (ns : ValidatedNamespace) (env : Array SemArg)
    (arguments : Array GenericArgument) : Option (Array SemArg) :=
  arguments.mapM fun
    | .typeArg value => SemArg.type <$> resolveIn ns env value.typeId
    | .const value => some (.const value)
    | .lifetime _ | .evidence _ => some .erased

/-- The static length of a subslice of a vector of a static length. -/
def subsliceLength (length : Option ConstValue) (start stop : Nat) (fromEnd : Bool) :
    Option ConstValue :=
  match length with
  | some (.integer count) =>
      let stop := if fromEnd then count.toNat - stop else stop
      some (.integer (Int.ofNat (stop - start)))
  | _ => none

/-- An integer width as the target runs it: a pointer-sized integer at the
target's pointer width, which a target without a supported width has none
of. -/
def targetWidth (pointerWidth : Option Nat) : IntWidth → Option IntWidth
  | .pointer => match pointerWidth with
      | some width => if supportedTargetPointerWidth width then some (.bits width) else none
      | none => none
  | width => some width

/-- Whether an integer type holds a value on the target. -/
def holdsAt (pointerWidth : Option Nat) (width : IntWidth) (signed : Bool) (value : Int) : Bool :=
  match targetWidth pointerWidth width with
  | some width => (Ty.integer width signed).integerValueFits? value == some true
  | none => false

/-- Whether a type is the one value `packResults` makes of these results:
none is the unit (or the empty tuple), one is itself, more a tuple. -/
def packs (results : List SemTy) (type : SemTy) : Bool :=
  match results with
  | [] => type == .unit || type == .tuple []
  | [result] => type == result
  | results => type == .tuple results

/-! ## Constants -/

mutual
/-- Whether a compile-time constant inhabits a semantic type. -/
def constTyped (pointerWidth : Option Nat) : ConstValue → SemTy → Bool
  | .unit, type => type == .unit || type == .tuple []
  | .bool _, type => type == .bool
  | .character value, type => type == .character && isUnicodeScalar value
  | .integer value, .integer width signed => holdsAt pointerWidth width signed value
  | .address _, type => type == .address
  | .string _, type => type == .string
  | .bytes _, type => type == .bytes
  | .vector elements, .vector element length =>
      constsTypedEach pointerWidth elements.toList element &&
        match length with
        | none => true
        | some (.integer count) => count == Int.ofNat elements.size
        | some _ => false
  | .tuple elements, .tuple types => constsTyped pointerWidth elements.toList types
  | _, _ => false

def constsTypedEach (pointerWidth : Option Nat) : List ConstValue → SemTy → Bool
  | [], _ => true
  | value :: values, type =>
      constTyped pointerWidth value type && constsTypedEach pointerWidth values type

def constsTyped (pointerWidth : Option Nat) : List ConstValue → List SemTy → Bool
  | [], [] => true
  | value :: values, type :: types =>
      constTyped pointerWidth value type && constsTyped pointerWidth values types
  | _, _ => false
end

/-! ## Divergence -/

/-- Whether a `break` in an expression leaves the loop `depth` loops out. -/
def breaksTo (ns : ValidatedNamespace) : Nat → Nat → ExprId → Bool
  | 0, _, _ => true
  | fuel + 1, depth, id => match ns.expressions[id.index]? with
    | none => true
    | some expression => match expression.kind with
      | .break_ nest value => nest == depth || value.any (breaksTo ns fuel depth)
      | .loop _ body => breaksTo ns fuel (depth + 1) body
      | .spec _ | .quantifier .. => false
      | kind => (expressionChildren kind).any (breaksTo ns fuel depth)

/-- Whether an expression never produces a value: it transfers control, a
part it evaluates before its value does, or it is a loop nothing breaks.
An expression whose stored type differs from where its value would go is
accepted when it diverges. -/
def diverts (ns : ValidatedNamespace) : Nat → ExprId → Bool
  | 0, _ => false
  | fuel + 1, id => match ns.expressions[id.index]? with
    | none => false
    | some expression => match expression.kind with
      | .return_ _ | .throw_ _ _ | .break_ _ _ | .continue_ _ => true
      | .block statements result =>
          statements.any (diverts ns fuel) || result.any (diverts ns fuel)
      | .letDecl _ value body => value.any (diverts ns fuel) || diverts ns fuel body
      | .ifElse condition thenBranch elseBranch =>
          diverts ns fuel condition ||
            (diverts ns fuel thenBranch && elseBranch.any (diverts ns fuel))
      | .match_ scrutinee arms =>
          diverts ns fuel scrutinee ||
            (!arms.isEmpty && arms.all fun arm => diverts ns fuel arm.body)
      | .operation _ _ arguments _ => arguments.any (diverts ns fuel)
      | .assign _ value | .assignPattern _ value => diverts ns fuel value
      | .loop _ body => !breaksTo ns fuel 0 body
      | _ => false

/-! ## Instantiation facts -/

/-- Whether a lifetime is a declaration's parameter. -/
def lifetimeParameter (ns : ValidatedNamespace) (lifetime : LifetimeId) : Bool :=
  (ns.tables.lifetimes[lifetime.index]?).any (·.kind matches .parameter _)

/-- Whether a type mentions a lifetime parameter. -/
def mentionsLifetimeParameter (ns : ValidatedNamespace) : Nat → TypeId → Bool
  | 0, _ => true
  | fuel + 1, id => match ns.tables.types[id.index]? with
    | none => false
    | some type => match type with
      | .tuple elements => elements.any (mentionsLifetimeParameter ns fuel)
      | .vector element _ | .typeDomain element => mentionsLifetimeParameter ns fuel element
      | .resourceDomain _ arguments =>
          arguments.any (·.any (mentionsLifetimeParameter ns fuel))
      | .nominal _ arguments => arguments.any fun
          | .typeArg value => mentionsLifetimeParameter ns fuel value.typeId
          | .lifetime value => lifetimeParameter ns value
          | .const _ | .evidence _ => false
      | .function arguments result _ =>
          arguments.any (mentionsLifetimeParameter ns fuel) ||
            mentionsLifetimeParameter ns fuel result
      | .reference borrowed => lifetimeParameter ns borrowed.lifetime ||
          mentionsLifetimeParameter ns fuel borrowed.referent
      | _ => false

/-- The type arguments of an instantiation. -/
def typeArguments (instantiations : Array GenericArgument) : Array TypeId :=
  instantiations.filterMap fun
    | .typeArg value => some value.typeId
    | _ => none

/-- A table of types per function position. -/
abbrev TypeTable := Array (Array (Array TypeId))

/-- The types at a function position. -/
def TypeTable.at (table : TypeTable) (position : Nat × Nat) : Array TypeId :=
  (table[position.1]?.bind (·[position.2]?)).getD #[]

/-- The position of the function a reference names: its namespace's and its
own index. -/
def functionIndex? (unit : ValidatedUnit) (ns : ValidatedNamespace) (reference : QualifiedRef) :
    Option (Nat × Nat) := do
  let qualified ← ns.tables.names[reference.name.index]?
  if qualified.namespaceId != reference.namespaceId then none else
    let functionId ← unit.resolution.function? reference.name
    let targetNs ← unit.namespaces[reference.namespaceId.index]?
    let _ ← targetNs.functions[functionId.index]?
    some (reference.namespaceId.index, functionId.index)

/-! ## Contexts -/

/-- Where a node is checked: its namespace, the locals and generic
environment of its body, the body's results (none in a constant's
initializer), the types of the loops around it, innermost first, the types
its frame reads its instantiation at (none in a constant's initializer,
which runs without one), and the types each function's frame requires. -/
structure Context where
  unit : ValidatedUnit
  pointerWidth : Option Nat
  ns : ValidatedNamespace
  locals : Array LocalDecl
  env : Array SemArg
  results : Option (List SemTy)
  loops : List SemTy
  required : Option (Array TypeId)
  table : TypeTable

namespace Context

def typeOf (context : Context) (typeId : TypeId) : Option SemTy :=
  resolveIn context.ns context.env typeId

def exprType (context : Context) (id : ExprId) : Option SemTy := do
  context.typeOf (← context.ns.expressions[id.index]?).typeId

def localType (context : Context) (id : LocalId) : Option SemTy := do
  context.typeOf (← context.locals[id.index]?).type.typeId

/-- Whether an expression never produces a value: it diverges, or its type
is uninhabited. -/
def stops (context : Context) (id : ExprId) : Bool :=
  context.exprType id == some .never || diverts context.ns (context.ns.expressions.size + 1) id

/-- Whether an expression's value goes where a value of `type` does: its
type is `type`, or it never produces a value. -/
def flows (context : Context) (id : ExprId) (type : SemTy) : Bool :=
  context.exprType id == some type || context.stops id

end Context

/-! ## Declarations

Targets are resolved as the runtime resolves them: through the unit's
resolution index, from a reference whose interned name agrees with its
namespace. -/

/-- The function a reference names, with its namespace. -/
def functionTarget? (unit : ValidatedUnit) (ns : ValidatedNamespace) (reference : QualifiedRef) :
    Option (ValidatedNamespace × FunctionDecl FunctionBody) := do
  let qualified ← ns.tables.names[reference.name.index]?
  if qualified.namespaceId != reference.namespaceId then none else
    let functionId ← unit.resolution.function? reference.name
    let targetNs ← unit.namespaces[reference.namespaceId.index]?
    let declaration ← targetNs.functions[functionId.index]?
    some (targetNs, declaration)

/-- The constant a reference names, with its namespace. -/
def constantTarget? (unit : ValidatedUnit) (ns : ValidatedNamespace) (reference : QualifiedRef) :
    Option (ValidatedNamespace × ConstantDecl) := do
  let qualified ← ns.tables.names[reference.name.index]?
  if qualified.namespaceId != reference.namespaceId then none else
    let constantId ← unit.resolution.constant? reference.name
    let targetNs ← unit.namespaces[reference.namespaceId.index]?
    let declaration ← targetNs.constants[constantId.index]?
    some (targetNs, declaration)

/-- The nominal declaration an interned name denotes, with its namespace
and spelled name. -/
def nominalTarget? (unit : ValidatedUnit) (ns : ValidatedNamespace) (name : NameId) :
    Option (ValidatedNamespace × StructDecl × QualifiedName) := do
  let qualified ← ns.tables.names[name.index]?
  let typeId ← unit.resolution.nominal? name
  let targetNs ← unit.namespaces[qualified.namespaceId.index]?
  let declaration ← targetNs.structs[typeId.index]?
  let spelled ← targetNs.tables.names[declaration.name.index]?
  let _ ← targetNs.tables.namespaces[spelled.namespaceId.index]?
  some (targetNs, declaration, spelled)

/-- Whether a declaration's variants have distinct names, so that a name
finds the variant a value of it holds. -/
def variantsDistinct (targetNs : ValidatedNamespace) (declaration : StructDecl) : Bool :=
  decide (declaration.variants.toList.map fun variant =>
    (targetNs.tables.names[variant.name.index]?).map (·.name)).Nodup

/-- The fields a variant (or, without one, a structure) declares. -/
def fieldsOf (targetNs : ValidatedNamespace) (declaration : StructDecl) :
    Option String → Option (Array FieldDecl)
  | none => if declaration.variants.isEmpty then some declaration.fields else none
  | some variantName => do
      let declared ← declaration.variants.toList.find? fun candidate =>
        (targetNs.tables.names[candidate.name.index]?).any (·.name == variantName)
      some declared.fields

/-- The names of the variants of a declaration that declare a field of this
name. -/
def declaringVariants (tables : Tables) (declaration : StructDecl) (field : String) :
    Array String :=
  let nameOf (name : NameId) := (tables.names[name.index]?).map (·.name)
  declaration.variants.filterMap fun variant =>
    if variant.fields.any (nameOf ·.name == some field) then nameOf variant.name else none

/-- Whether a selection of a field lists exactly the variants declaring it:
it is then the field access, which reads the field of whichever variant
declares it. -/
def listsDeclaringVariants (declaring listed : Array String) : Bool :=
  declaring.all listed.contains && listed.all declaring.contains

/-- The type of a named field of a nominal type, its declaration's field
type under the type's arguments. -/
def fieldType? (targetNs : ValidatedNamespace) (declaration : StructDecl)
    (variant : Option String) (arguments : List SemArg) (fieldName : String) : Option SemTy := do
  let fields ← fieldsOf targetNs declaration variant
  let field ← fields.find? fun field =>
    (targetNs.tables.names[field.name.index]?).any (·.name == fieldName)
  resolveIn targetNs arguments.toArray field.type.typeId

/-- The types of all fields of a variant (or structure) under arguments. -/
def fieldTypes? (targetNs : ValidatedNamespace) (declaration : StructDecl)
    (variant : Option String) (arguments : List SemArg) : Option (List SemTy) := do
  let fields ← fieldsOf targetNs declaration variant
  fields.toList.mapM fun field => resolveIn targetNs arguments.toArray field.type.typeId

/-! ## Primitives

A scalar primitive computes its value at its result type, so its rule
constrains the result type alone; an aggregate one relates its operands'
types to its result's. Operations without a runtime value (`range` and the
logical connectives of specifications) produce none. -/

/-- An integer of a fixed width on the target. -/
def fixedInteger (pointerWidth : Option Nat) : SemTy → Bool
  | .integer width _ => (targetWidth pointerWidth width matches some (.bits _))
  | _ => false

/-- Whether an integer type holds a value on the target. -/
def holds (pointerWidth : Option Nat) (type : SemTy) (value : Int) : Bool :=
  match type with
  | .integer width signed => holdsAt pointerWidth width signed value
  | _ => false

/-- An integer type the target runs. -/
def integral (pointerWidth : Option Nat) : SemTy → Bool
  | .integer width _ => (targetWidth pointerWidth width).isSome
  | _ => false

/-- Whether a primitive over operands of these types produces values of the
result type. -/
def primitiveTyped (pointerWidth : Option Nat) (operation : PrimitiveOperation)
    (operands : List SemTy) (result : SemTy) : Bool :=
  let fixedInteger := fixedInteger pointerWidth
  let integral := integral pointerWidth
  let holds := holds pointerWidth
  match operation with
  | .tuple => result == .tuple operands
  | .vector => match result with
      | .vector element length => operands.all (· == element) &&
          match length with
          | none => true
          | some (.integer count) => count == Int.ofNat operands.length
          | some _ => false
      | _ => false
  | .repeatVector => match result, operands with
      | .vector element (some (.integer _)), [operand] => operand == element
      | _, _ => false
  | .pushVector => match operands with
      | [.vector element _, value] => value == element && result == .vector element none
      | _ => false
  | .insertVector => match operands with
      | [.vector element _, index, value] =>
          integral index && value == element && result == .vector element none
      | _ => false
  | .concatVector => match operands with
      | [.vector element _, .vector other _] =>
          other == element && result == .vector element none
      | _ => false
  | .removeVector => match operands with
      | [.vector element _, index] =>
          integral index && result == .tuple [element, .vector element none]
      | _ => false
  | .swapVector | .reverseSliceVector | .slice => match operands with
      | [.vector element _, first, second] =>
          integral first && integral second && result == .vector element none
      | _ => false
  | .containsVector => result == .bool
  | .indexOfVector => match result with
      | .tuple [.bool, index] => integral index
      | _ => false
  | .destroyEmptyVector | .checkVectorIndex _ => packs [] result
  | .length => integral result
  | .index => match operands with
      | [.vector element _, index] => integral index && result == element
      | _ => false
  | .compare => holds result (-1) && holds result 0 && holds result 1
  | .signerAddress => result == .address
  | .add | .subtract | .multiply | .divide | .modulo
  | .checkedAdd _ | .checkedSubtract _ | .checkedMultiply _ | .checkedDivide _
  | .checkedModulo _ | .negate | .checkedNegate _ | .bitwiseNot
  | .shiftLeft | .shiftRight | .checkedShiftLeft _ | .checkedShiftRight _ =>
      fixedInteger result
  -- Boolean operands make a Boolean; integers an integer of their type.
  | .bitwiseOr | .bitwiseAnd | .bitwiseXor =>
      (result == .bool && operands == [.bool, .bool]) ||
        (fixedInteger result && operands == [result, result])
  | .cast | .checkedCast _ => fixedInteger result || result == .character
  | .overflowingAdd | .overflowingSubtract | .overflowingMultiply => match result with
      | .tuple [value, .bool] => fixedInteger value
      | _ => false
  | .logicalAnd | .logicalOr | .logicalNot | .equal | .notEqual
  | .less | .greater | .lessEqual | .greaterEqual => result == .bool
  | .copyValue | .moveValue => operands == [result]
  | .range | .implies | .equivalent | .identical => true

/-- The variants' types of a field present in each of them: what a value of
the nominal type has at the field, whichever variant it is. -/
def fieldTypesAcross? (targetNs : ValidatedNamespace) (declaration : StructDecl)
    (arguments : List SemArg) (fieldName : String) : Option (List SemTy) :=
  if declaration.variants.isEmpty then
    (fieldType? targetNs declaration none arguments fieldName).map ([·])
  else
    declaration.variants.toList.filterMapM fun variant => do
      let variantName ← targetNs.tables.names[variant.name.index]?
      let fields ← fieldsOf targetNs declaration (some variantName.name)
      if fields.any fun field =>
          (targetNs.tables.names[field.name.index]?).any (·.name == fieldName) then
        some <$> fieldType? targetNs declaration (some variantName.name) arguments fieldName
      else some none

/-- The type of a field of a nominal type at a variant, or, without one,
the type every variant holding the field agrees on. -/
def fieldTypeAt? (targetNs : ValidatedNamespace) (declaration : StructDecl)
    (variant : Option String) (arguments : List SemArg) (fieldName : String) : Option SemTy :=
  match variant with
  | some _ => fieldType? targetNs declaration variant arguments fieldName
  | none => match fieldTypesAcross? targetNs declaration arguments fieldName with
      | some (type :: types) => if types.all (· == type) then some type else none
      | _ => none

namespace Context

/-! ## Places -/

/-- The type of a place and the variant it is downcast to. -/
def placeType (context : Context) : Nat → PlaceId → Option (SemTy × Option String)
  | 0, _ => none
  | fuel + 1, id => match context.ns.places[id.index]? with
    | none => none
    | some (.localVar localId) => (·, none) <$> context.localType localId
    | some (.deref base) => do
        let (type, _) ← context.placeType fuel base
        -- The runtime reads a dereference the borrow certificate records as
        -- shared as its base: the record must be the reference's kind.
        match type with
        | .reference kind referent =>
            if (kind == .shared) == sharedDereference context.unit context.ns.identity id then
              some (referent, none)
            else none
        | _ => none
    | some (.field base owner field) => do
        let (type, variant) ← context.placeType fuel base
        let .nominal name arguments := type | none
        let (targetNs, declaration, spelled) ← nominalTarget? context.unit context.ns owner.name
        if spelled != name then none else
          let fieldName ← context.ns.tables.names[field.index]?
          let fieldType ← fieldTypeAt? targetNs declaration variant arguments fieldName.name
          some (fieldType, none)
    | some (.index base index) => do
        let (type, _) ← context.placeType fuel base
        match type with
        | .vector element _ => match context.exprType index with
            | some indexType =>
                if integral context.pointerWidth indexType then some (element, none) else none
            | none => none
        -- A tuple is indexed by a literal position.
        | .tuple elements => match context.ns.expressions[index.index]? with
            | some { kind := .value (.integer position) _, .. } =>
                if position < 0 then none else (·, none) <$> elements[position.toNat]?
            | _ => none
        | _ => none
    | some (.subslice base start stop fromEnd) => do
        let (type, _) ← context.placeType fuel base
        let .vector element length := type | none
        some (.vector element (subsliceLength length start stop fromEnd), none)
    | some (.downcast base variant) => do
        let (type, _) ← context.placeType fuel base
        let .nominal _ _ := type | none
        let variantName ← context.ns.tables.names[variant.index]?
        some (type, some variantName.name)

/-- The type of a place's value. -/
def placeTypeOf (context : Context) (id : PlaceId) : Option SemTy :=
  (·.1) <$> context.placeType (context.ns.places.size + 1) id

/-! ## Patterns -/

/-- Whether a pattern matches values of `type`, binding each variable at
its local's type. -/
def patternTypedFuel (context : Context) : Nat → PatternId → SemTy → Bool
  | 0, _, _ => false
  | fuel + 1, id, type => match context.ns.patterns[id.index]? with
    | none => false
    | some pattern => match pattern.kind with
      | .wildcard => true
      | .variable localId => context.localType localId == some type
      | .tuple elements => match type with
          | .tuple types => elements.size == types.length &&
              (elements.toList.zip types).all fun (element, elementType) =>
                context.patternTypedFuel fuel element elementType
          | _ => false
      | .constructor name instantiations variant fields =>
          match nominalTarget? context.unit context.ns name,
              resolveArguments context.ns context.env instantiations with
          | some (targetNs, declaration, spelled), some arguments =>
              type == .nominal spelled arguments.toList &&
                match fieldTypes? targetNs declaration variant arguments.toList with
                | some types => fields.size == types.length &&
                    (fields.toList.zip types).all fun (field, fieldType) =>
                      context.patternTypedFuel fuel field fieldType
                | none => false
          | _, _ => false
      | .literal value => constTyped context.pointerWidth value type
      | .range _ _ _ => integral context.pointerWidth type || type == .character

def patternTyped (context : Context) (id : PatternId) (type : SemTy) : Bool :=
  context.patternTypedFuel (context.ns.patterns.size + 1) id type

end Context

/-- The nominal declaration a reference names, with its namespace, by the
runtime's lookups: where it finds none, the runtime resolves none. -/
def declarationTarget? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (reference : QualifiedRef) : Option (ValidatedNamespace × StructDecl) := do
  let qualified ← ns.tables.names[reference.name.index]?
  if qualified.namespaceId != reference.namespaceId then none else
    let typeId ← unit.resolution.nominal? reference.name
    let targetNs ← unit.namespaces[reference.namespaceId.index]?
    let declaration ← targetNs.structs[typeId.index]?
    some (targetNs, declaration)

/-- The nominal declaration a reference names, with its namespace and
spelled name. -/
def structTarget? (unit : ValidatedUnit) (ns : ValidatedNamespace) (reference : QualifiedRef) :
    Option (ValidatedNamespace × StructDecl × QualifiedName) := do
  let (targetNs, declaration) ← declarationTarget? unit ns reference
  let spelled ← targetNs.tables.names[declaration.name.index]?
  let _ ← targetNs.tables.namespaces[spelled.namespaceId.index]?
  some (targetNs, declaration, spelled)

/-- The reference kind a borrow makes. -/
def borrowedKind : BorrowKind → Option ReferenceKind
  | .immutable => some .shared
  | .mutable => some .mutable
  | .profile _ => none

/-- A function's parameter and result types under arguments. -/
def signatureTypes? (targetNs : ValidatedNamespace) (declaration : FunctionDecl FunctionBody)
    (arguments : Array SemArg) : Option (List SemTy × List SemTy) := do
  let parameters ← declaration.signature.parameters.toList.mapM fun parameter =>
    resolveIn targetNs arguments parameter.typeUse.typeId
  let results ← declaration.signature.results.toList.mapM fun result =>
    resolveIn targetNs arguments result.typeId
  some (parameters, results)

namespace Context

/-- Whether each generic argument has its binder's kind: a type argument
exactly at a type binder. -/
def kindsAgree (generics : Array GenericBinder) (instantiations : Array GenericArgument) : Bool :=
  (generics.toList.zip instantiations.toList).all fun (binder, argument) =>
    (binder.kind matches .typeArg) == (argument matches .typeArg _)

/-- Whether the frame reads its instantiation at a type. -/
def consults (context : Context) (typeId : TypeId) : Bool :=
  match context.required with
  | some required => required.contains typeId
  | none => true

/-- Whether a call creates a faithful frame: each type the target requires
is interned at the call's arguments, read by the calling frame, and free of
lifetime parameters (`designs/static-typing.md`, "Instantiation"). -/
def edgeClosed (context : Context) (reference : QualifiedRef)
    (instantiations : Array GenericArgument) : Bool :=
  (typeArguments instantiations).all context.consults &&
    match functionIndex? context.unit context.ns reference with
    | some target => match context.unit.namespaces[target.1]? with
        | some targetNs => (context.table.at target).all fun typeId =>
            match instantiatePlaceFieldType? targetNs instantiations typeId with
            | some instantiated => context.consults instantiated &&
                !mentionsLifetimeParameter context.ns (context.ns.tables.types.size + 1) instantiated
            | none => false
        | none => false
    | none => false

/-- Whether a node's own type is a shared reference in the arena: what the
runtime reads to tell a shared operand from a mutable one
(`sharedOperandAt`). -/
def sharedNode (ns : ValidatedNamespace) (id : ExprId) : Bool :=
  match ns.expressions[id.index]? with
  | some expression => match ns.tables.types[expression.typeId.index]? with
      | some (.reference reference) => reference.kind == .shared
      | _ => false
  | none => false

/-- Whether a call of this kind, at these instantiations and operands,
produces values of `result`. A target the unit does not hold has no
evaluation: the runtime resolves it as these lookups do, so such a call
produces no value. -/
def checkCall (context : Context) (result : SemTy) : CallKind → Array GenericArgument →
    Array ExprId → Bool
  | .function reference, instantiations, operands =>
      match functionTarget? context.unit context.ns reference,
          resolveArguments context.ns context.env instantiations with
      | none, _ => true
      | some (targetNs, declaration), some arguments =>
          arguments.size == declaration.signature.generics.size &&
            kindsAgree declaration.signature.generics instantiations &&
            context.edgeClosed reference instantiations &&
            match signatureTypes? targetNs declaration arguments with
            | some (parameters, results) => operands.size == parameters.length &&
                (operands.toList.zip parameters).all (fun (operand, parameter) =>
                  context.flows operand parameter) && packs results result
            | none => false
      | _, _ => false
  | .constructor reference variant, instantiations, operands =>
      match structTarget? context.unit context.ns reference,
          resolveArguments context.ns context.env instantiations with
      | none, _ => (declarationTarget? context.unit context.ns reference).isNone
      | some (targetNs, declaration, spelled), some arguments =>
          match fieldTypes? targetNs declaration variant arguments.toList with
          | some fields => operands.size == fields.length &&
              (operands.toList.zip fields).all (fun (operand, field) =>
                context.flows operand field) && result == .nominal spelled arguments.toList
          | none => false
      | _, _ => false
  | .destructor reference variant, instantiations, operands =>
      match structTarget? context.unit context.ns reference,
          resolveArguments context.ns context.env instantiations, operands.toList with
      | none, _, _ => (declarationTarget? context.unit context.ns reference).isNone
      | some (targetNs, declaration, spelled), some arguments, [operand] =>
          context.exprType operand == some (.nominal spelled arguments.toList) &&
            match fieldTypes? targetNs declaration variant arguments.toList with
            | some fields => packs fields result
            | none => false
      | _, _, _ => false
  | .closure reference mask, instantiations, captures =>
      match functionTarget? context.unit context.ns reference,
          resolveArguments context.ns context.env instantiations, result with
      | none, _, _ => true
      | some (targetNs, declaration), some arguments, .function openTypes resultType =>
          arguments.size == declaration.signature.generics.size &&
            kindsAgree declaration.signature.generics instantiations &&
            context.edgeClosed reference instantiations &&
            mask < 2 ^ declaration.signature.parameters.size &&
            match signatureTypes? targetNs declaration arguments with
            | some (parameters, results) =>
                let captured := ClosureMask.extract mask true parameters
                captures.size == captured.length &&
                  (captures.toList.zip captured).all (fun (capture, parameter) =>
                    context.flows capture parameter) &&
                  openTypes == ClosureMask.extract mask false parameters &&
                  packs results resultType
            | none => false
      | _, _, _ => false
  | .invoke, _, operands => match operands.toList with
      | callee :: supplied => match context.exprType callee with
          | some (.function parameters resultType) => supplied.length == parameters.length &&
              (supplied.zip parameters).all (fun (operand, parameter) =>
                context.flows operand parameter) && result == resultType
          | _ => false
      | [] => false
  | .extension _ _, _, _ => false

/-- A selected value's type: the field's own, or, through a reference, a
reference of its kind to it (a shared reference is the value it observes). -/
def selected (operand result : SemTy) (field : SemTy → Bool) : Bool :=
  field result || match operand, result with
    | .reference kind _, .reference resultKind referent => kind == resultKind && field referent
    | _, _ => false

/-- The nominal type a data operation's operand holds, directly or through
a reference. -/
def nominalOperand : SemTy → Option (QualifiedName × List SemArg)
  | .nominal name arguments | .reference _ (.nominal name arguments) => some (name, arguments)
  | _ => none

/-- Whether a data operation over these operands produces values of
`result`. -/
def checkData (context : Context) (result : SemTy) : DataOperation → Array ExprId → Bool
  | .select reference field, operands => match structTarget? context.unit context.ns reference,
        operands.toList with
      | some (targetNs, declaration, spelled), [operand] =>
          match context.exprType operand with
          | some operandType => match nominalOperand operandType with
              | some (name, arguments) => name == spelled &&
                  match fieldTypesAcross? targetNs declaration arguments field with
                  | some types => !types.isEmpty &&
                      selected operandType result fun type => types.all (· == type)
                  | none => false
              | none => false
          | none => false
      | _, _ => false
  | .selectVariants reference fields, operands =>
      match structTarget? context.unit context.ns reference, operands.toList with
      | some (targetNs, declaration, spelled), [operand] =>
          match context.exprType operand with
          | some operandType => match nominalOperand operandType with
              | some (name, arguments) => name == spelled &&
                  selected operandType result fun type => fields.all fun selected =>
                    fieldType? targetNs declaration (some selected.1) arguments selected.2 ==
                      some type
              | none => false
          | none => false
      | _, _ => false
  | .testVariants _ _, _ => result == .bool
  -- A declared discriminant is the value: each must fit the result type.
  | .discriminant reference, _ => integral context.pointerWidth result &&
      match declarationTarget? context.unit context.ns reference with
      | some (_, declaration) => declaration.variants.all fun variant =>
          variant.discriminant.all (holds context.pointerWidth result ·)
      | none => true
  | .updateField reference field, operands =>
      match structTarget? context.unit context.ns reference, operands.toList with
      | some (targetNs, declaration, spelled), [operand, replacement] =>
          match context.exprType operand with
          | some (.nominal name arguments) => name == spelled &&
              result == .nominal name arguments &&
              match fieldTypesAcross? targetNs declaration arguments field with
              | some types => types.all (context.flows replacement ·)
              | none => false
          | _ => false
      | _, _ => false

/-- Whether an operation at these instantiations and operands produces
values of `result`. -/
def checkOperation (context : Context) (result : SemTy) (operation : Operation)
    (instantiations : Array GenericArgument) (operands : Array ExprId) : Bool :=
  match operation with
  | .copy place | .read place | .move place =>
      operands.isEmpty && context.placeTypeOf place == some result
  | .borrow kind place => operands.isEmpty &&
      match borrowedKind kind, context.placeTypeOf place with
      | some referenceKind, some referent => result == .reference referenceKind referent
      | _, _ => false
  | .write place => match operands.toList, context.placeTypeOf place with
      | [value], some type => context.flows value type && packs [] result
      | _, _ => false
  | .drop place => operands.isEmpty && (context.placeTypeOf place).isSome && packs [] result
  | .call kind => context.checkCall result kind instantiations operands
  | .global kind => match instantiations.toList with
      | [.typeArg resource] => context.consults resource.typeId &&
          match context.typeOf resource.typeId with
          | some resourceType => match kind, operands.toList with
              | .contains, [_] => result == .bool
              | .borrow borrowKind, [_] => match borrowedKind borrowKind with
                  | some referenceKind => result == .reference referenceKind resourceType
                  | none => false
              | .take, [_] => result == resourceType
              | .publish, [_, value] => context.flows value resourceType && packs [] result
              | _, _ => false
          | none => false
      | _ => false
  | .primitive operation => match operands.toList.mapM context.exprType with
      | some types => primitiveTyped context.pointerWidth operation types result
      | none => false
  | .reference kind => match kind, operands.toList with
      | .dereference, [reference] => match context.exprType reference with
          | some (.reference kind referent) => result == referent &&
              (kind == .shared) == sharedNode context.ns reference
          | _ => false
      | .freeze _, [reference] => match context.exprType reference with
          | some (.reference kind referent) => result == .reference .shared referent &&
              (kind == .shared) == sharedNode context.ns reference
          | _ => false
      | .mutate, [reference, value] => match context.exprType reference with
          | some (.reference .mutable referent) => context.flows value referent && packs [] result
          | _ => false
      -- A value borrow has no runtime evaluation.
      | .borrow _, _ => true
      | _, _ => false
  | .data operation => context.checkData result operation operands
  | .assert => packs [] result
  | .specification _ | .profile _ _ => false

/-- Whether a node's value is of its type, its parts typed as it uses them. -/
def checkNode (context : Context) (id : ExprId) : Bool :=
  match context.ns.expressions[id.index]? with
  | none => false
  | some expression => match context.typeOf expression.typeId with
    | none => false
    | some type => match expression.kind with
      | .value literal _ => constTyped context.pointerWidth literal type
      | .constant reference => match constantTarget? context.unit context.ns reference with
          | some (targetNs, declaration) => resolveIn targetNs #[] declaration.type.typeId == some type
          | none => true
      | .localVar localId => context.localType localId == some type
      | .operation operation instantiations operands _ =>
          context.checkOperation type operation instantiations operands
      | .block statements result => match result with
          | some value => context.flows value type
          | none => packs [] type || statements.any context.stops
      | .letDecl pattern value body =>
          (match value with
            | some value => context.stops value || match context.exprType value with
                | some valueType => context.patternTyped pattern valueType
                | none => false
            | none => true) && context.flows body type
      | .ifElse condition thenBranch elseBranch =>
          context.flows condition .bool && context.flows thenBranch type &&
            match elseBranch with
            | some elseBranch => context.flows elseBranch type
            | none => packs [] type
      | .match_ scrutinee arms => context.stops scrutinee || match context.exprType scrutinee with
          | some scrutineeType => arms.all fun arm =>
              context.patternTyped arm.pattern scrutineeType &&
                arm.guard.all (context.flows · .bool) && context.flows arm.body type
          | none => false
      | .loop _ _ => true
      | .break_ nest value => match context.loops[nest]? with
          | some loopType => match value with
              | some value => context.flows value loopType
              | none => packs [] loopType
          | none => false
      | .continue_ nest => nest < context.loops.length
      | .return_ values => match context.results with
          | some results => values.size == results.length &&
              (values.toList.zip results).all fun (value, result) => context.flows value result
          | none => false
      | .throw_ _ _ => true
      | .assign place value => (match context.placeTypeOf place with
            | some placeType => context.flows value placeType
            | none => false) && packs [] type
      | .assignPattern pattern value => (context.stops value || match context.exprType value with
            | some valueType => context.patternTyped pattern valueType
            | none => false) && packs [] type
      | .quantifier .. => false
      | .spec _ => packs [] type

/-! ## Bodies -/

/-- The nodes a node runs, each with the context it runs in: a loop's body
inside the loop; a specification block runs nothing. -/
def children (context : Context) (id : ExprId) : List (Context × ExprId) :=
  match context.ns.expressions[id.index]? with
  | none => []
  | some expression => match expression.kind with
    | .spec _ | .quantifier .. => []
    | .loop _ body => match context.typeOf expression.typeId with
        | some type => [({ context with loops := type :: context.loops }, body)]
        | none => []
    | kind => (expressionChildren kind).toList.map (context, ·)

/-- Whether every node a node runs, within `fuel` nested nodes, satisfies
its check. -/
def checkTree (context : Context) : Nat → ExprId → Bool
  | 0, _ => false
  | fuel + 1, id => context.checkNode id &&
      (context.children id).all fun (child, childId) => child.checkTree fuel childId

/-- The nodes a body runs from its root, with their contexts. -/
inductive Reaches (root : Context) (rootId : ExprId) : Context → ExprId → Prop
  | refl : Reaches root rootId root rootId
  | child {context : Context} {id : ExprId} {child : Context} {childId : ExprId} :
      Reaches root rootId context id → (child, childId) ∈ context.children id →
        Reaches root rootId child childId

theorem checkTree_reaches {root : Context} {rootId : ExprId} {fuel : Nat}
    (checked : root.checkTree fuel rootId = true) {context : Context} {id : ExprId}
    (reaches : Reaches root rootId context id) : ∃ fuel, context.checkTree fuel id = true := by
  induction reaches with
  | refl => exact ⟨fuel, checked⟩
  | child _ member ih =>
      obtain ⟨fuel, checked⟩ := ih
      cases fuel with
      | zero => simp [checkTree] at checked
      | succ fuel =>
          simp only [checkTree, Bool.and_eq_true, List.all_eq_true] at checked
          exact ⟨fuel, checked.2 _ member⟩

/-- Every node a checked body runs satisfies its check. -/
theorem checkNode_of_reaches {root : Context} {rootId : ExprId} {fuel : Nat}
    (checked : root.checkTree fuel rootId = true) {context : Context} {id : ExprId}
    (reaches : Reaches root rootId context id) : context.checkNode id = true := by
  obtain ⟨fuel, checked⟩ := checkTree_reaches checked reaches
  cases fuel with
  | zero => simp [checkTree] at checked
  | succ fuel =>
      simp only [checkTree, Bool.and_eq_true] at checked
      exact checked.1

end Context

/-- The context a function's body is checked in. -/
def functionContext (unit : ValidatedUnit) (pointerWidth : Option Nat) (ns : ValidatedNamespace)
    (table : TypeTable) (position : Nat × Nat) (declaration : FunctionDecl FunctionBody)
    (results : List SemTy) : Context :=
  { unit, pointerWidth, ns, locals := declaration.locals,
    env := staticEnv declaration.signature.generics, results := some results, loops := [],
    required := some (table.at position), table }

/-- Whether a function's body is typed: its parameters are its leading
locals, its fall-through value is its packed results, and every node it runs
satisfies its check. -/
def checkFunction (unit : ValidatedUnit) (pointerWidth : Option Nat) (ns : ValidatedNamespace)
    (table : TypeTable) (position : Nat × Nat) (declaration : FunctionDecl FunctionBody) : Bool :=
  match declaration.body with
  | .absent => true
  | .structured root =>
      match signatureTypes? ns declaration (staticEnv declaration.signature.generics) with
      | none => false
      | some (parameters, results) =>
          let context := functionContext unit pointerWidth ns table position declaration results
          (parameters.zipIdx.all fun (parameter, index) =>
              context.localType ⟨index⟩ == some parameter) &&
            (match context.exprType root with
              | some rootType => packs results rootType || rootType == .never ||
                  diverts ns (ns.expressions.size + 1) root
              | none => false) &&
            context.checkTree (ns.expressions.size + 1) root

/-- The context a constant's initializer is checked in: no locals, no
generics, no results. -/
def constantContext (unit : ValidatedUnit) (pointerWidth : Option Nat) (ns : ValidatedNamespace)
    (table : TypeTable) : Context :=
  { unit, pointerWidth, ns, locals := #[], env := #[], results := none, loops := [],
    required := none, table }

/-- Whether a constant's initializer is of the constant's type. -/
def checkConstant (unit : ValidatedUnit) (pointerWidth : Option Nat) (ns : ValidatedNamespace)
    (table : TypeTable) (declaration : ConstantDecl) : Bool :=
  match resolveIn ns #[] declaration.type.typeId with
  | some type => (constantContext unit pointerWidth ns table).flows declaration.value type &&
      (constantContext unit pointerWidth ns table).checkTree (ns.expressions.size + 1)
        declaration.value
  | none => false

/-! ## Instantiation closure

A frame's runtime reads its instantiation (`instantiatedTypeId`) at the
types it *consults*: the resource types of its global operations and the
type arguments of its generic calls and closures. The runtime finds an
instantiated type by searching the arena for its node, so a frame is
faithful to its arguments where every type it requires is interned. A
callee's required types, instantiated at a call's arguments, are required
of the caller in turn: the callee's search then repeats the caller's
(`designs/static-typing.md`, "Instantiation"). The table is computed by
iteration here and checked at each node (`Context.consults`,
`Context.edgeClosed`). -/

/-- What a body's frame consults and the generic frames it creates. -/
structure BodyFacts where
  consulted : Array TypeId := #[]
  /-- Each call or closure: its target's position and type arguments. -/
  edges : Array ((Nat × Nat) × Array GenericArgument) := #[]

/-- What the nodes a body runs below `id` consult and create. -/
def bodyFacts (unit : ValidatedUnit) (ns : ValidatedNamespace) :
    Nat → ExprId → BodyFacts → BodyFacts
  | 0, _, facts => facts
  | fuel + 1, id, facts => match ns.expressions[id.index]? with
    | none => facts
    | some expression =>
        let facts := match expression.kind with
          | .operation (.global _) instantiations _ _ =>
              { facts with consulted := facts.consulted ++ typeArguments instantiations }
          | .operation (.call (.function reference)) instantiations _ _
          | .operation (.call (.closure reference _)) instantiations _ _ =>
              let facts := { facts with consulted := facts.consulted ++ typeArguments instantiations }
              match functionIndex? unit ns reference with
              | some target => { facts with edges := facts.edges.push (target, instantiations) }
              | none => facts
          | _ => facts
        match expression.kind with
        | .spec _ | .quantifier .. => facts
        | kind => (expressionChildren kind).foldl (init := facts) fun facts child =>
            bodyFacts unit ns fuel child facts

/-- Each function's body facts, by namespace and function position. -/
def unitFacts (unit : ValidatedUnit) : Array (Array BodyFacts) :=
  unit.namespaces.map fun ns => ns.functions.map fun declaration =>
    match declaration.body with
    | .structured root => bodyFacts unit ns (ns.expressions.size + 1) root {}
    | .absent => {}

/-- The types required of each frame, iterated to the closure, or `none`
where a required type is not interned at a call's arguments. -/
def requiredTypes (unit : ValidatedUnit) : Option TypeTable := Id.run do
  let facts := unitFacts unit
  let mut required : TypeTable := facts.map (·.map (·.consulted))
  let bound := (facts.foldl (· + ·.size) 0) * (unit.tables.types.size + 1) + 1
  for _ in [0:bound] do
    let mut changed := false
    for namespaceFacts in facts, nsIndex in [0:facts.size] do
      for functionFacts in namespaceFacts, functionIndex in [0:namespaceFacts.size] do
        for (target, instantiations) in functionFacts.edges do
          let some targetNs := unit.namespaces[target.1]? | return none
          for typeId in required.at target do
            let some instantiated := instantiatePlaceFieldType? targetNs instantiations typeId
              | return none
            let own := required.at (nsIndex, functionIndex)
            unless own.contains instantiated do
              required := required.modify nsIndex (·.modify functionIndex (·.push instantiated))
              changed := true
    if !changed then return some required
  return none

/-- Whether a unit is statically typed: the types its frames require
close (`requiredTypes`), every namespace is at its index and reads the
unit's types and names, its declarations' variants are distinct, and its
functions and constants are typed. -/
def checkUnit (unit : ValidatedUnit) (pointerWidth : Option Nat) : Bool :=
  match requiredTypes unit with
  | none => false
  | some table => unit.namespaces.zipIdx.all fun (ns, index) =>
      ns.identity.index == index && decide (ns.tables.types = unit.tables.types) &&
        decide (ns.tables.names = unit.tables.names) &&
        ns.structs.all (variantsDistinct ns) &&
        ns.functions.zipIdx.all (fun (declaration, functionIndex) =>
          checkFunction unit pointerWidth ns table (index, functionIndex) declaration) &&
        ns.constants.all (checkConstant unit pointerWidth ns table)

/-! ## Diagnostics -/

/-- The nodes a body runs that fail their check, within `fuel` nested
nodes. -/
def Context.failures (context : Context) : Nat → ExprId → Array ExprId
  | 0, id => #[id]
  | fuel + 1, id =>
      let own := if context.checkNode id then #[] else #[id]
      (context.children id).foldl (init := own) fun found (child, childId) =>
        found ++ child.failures fuel childId

/-- The nodes of a unit that fail the static checks, by namespace and
function or constant, or the unit-level check that fails. -/
def unitFailures (unit : ValidatedUnit) (pointerWidth : Option Nat) : Array String := Id.run do
  let mut found := #[]
  let table := (requiredTypes unit).getD #[]
  unless (requiredTypes unit).isSome do
    found := found.push "a type a generic frame requires is not interned at a call's arguments"
  for ns in unit.namespaces, index in [0:unit.namespaces.size] do
    if ns.identity.index != index then
      found := found.push s!"namespace {index} has identity {ns.identity.index}"
    if !decide (ns.tables.types = unit.tables.types) || !decide (ns.tables.names = unit.tables.names) then
      found := found.push s!"namespace {index} does not read the unit's tables"
    for declaration in ns.structs, structIndex in [0:ns.structs.size] do
      if !variantsDistinct ns declaration then
        found := found.push s!"declaration #{structIndex} of namespace {index} repeats a variant name"
    for declaration in ns.functions, functionIndex in [0:ns.functions.size] do
      if checkFunction unit pointerWidth ns table (index, functionIndex) declaration then continue
      let name := (ns.tables.names[declaration.name.index]?).map (·.name) |>.getD s!"#{functionIndex}"
      match declaration.body, signatureTypes? ns declaration (staticEnv declaration.signature.generics) with
      | .structured root, some (_, results) =>
          let context := functionContext unit pointerWidth ns table (index, functionIndex)
            declaration results
          let nodes := context.failures (ns.expressions.size + 1) root
          let described := nodes.map fun id =>
            match ns.expressions[id.index]? with
            | some e =>
                let kind := ((repr e.kind).pretty 200).take 160
                let children := (expressionChildren e.kind).toList.map fun child =>
                  (repr (context.exprType child)).pretty 120
                s!"{id.index}: {kind} : {(repr (context.typeOf e.typeId)).pretty 120} ← {children}"
            | none => s!"{id.index}: ?"
          found := found.push s!"function {name}: {if nodes.isEmpty then "signature or fall-through" else "\n  ".intercalate described.toList}"
      | _, _ => found := found.push s!"function {name}: signature does not resolve"
    for declaration in ns.constants, constantIndex in [0:ns.constants.size] do
      if !checkConstant unit pointerWidth ns table declaration then
        found := found.push s!"constant #{constantIndex} of namespace {index}"
  return found

end LeanerIR.Validation.StaticTyping
