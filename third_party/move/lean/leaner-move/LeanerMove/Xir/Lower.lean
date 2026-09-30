-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR
import LeanerLang.Modifiers
import LeanerMove.Xir.Syntax

/-!
# Lowering validated LIR to XIR

One Move-profile namespace of a validated unit becomes one deployable XIR
module. Structured bodies become a control-flow graph of three-address
instructions: a value is the locals holding it (none for a unit, several for
a tuple), a place is borrowed step by step down to its root, and a construct
with no Move bytecode form is a located error.
-/

namespace LeanerIR.Move.Xir

open LeanerIR
open LeanerIR.Validation
open LeanerIR.SemanticOperations

/-- A lowering failure at an LIR location, when one is known. -/
structure Failure where
  loc : Option LocId
  message : String
  deriving Repr, Inhabited

structure Context where
  unit : ValidatedUnit
  namespaceId : NamespaceId
  ns : ValidatedNamespace

structure ModuleState where
  externalFunctions : Array External := #[]
  externalStructs : Array External := #[]

abbrev M := ReaderT Context (StateT ModuleState (Except Failure))

def fail (loc : Option LocId) (message : String) : M α := throw { loc, message }

private def tables : M Tables := return (← read).ns.tables

/-! ## Names and declarations -/

private def nameOf (name : NameId) : M String := do
  let some qualified := (← tables).names[name.index]?
    | fail none s!"name {name.index} is out of range"
  pure qualified.name

/-- The address and module name of a namespace: its first and last path
segments. -/
private def moduleOf (loc : Option LocId) (namespaceId : NamespaceId) : M (String × String) := do
  let some reference := (← tables).namespaces[namespaceId.index]?
    | fail loc s!"namespace {namespaceId.index} is out of range"
  match reference.segments[0]?, reference.segments.back? with
  | some address, some module =>
      if reference.segments.size < 2 then
        fail loc s!"namespace `{"::".intercalate reference.segments.toList}` is not a Move module"
      else pure (address, module)
  | _, _ => fail loc "a namespace without a path is not a Move module"

private def external (entries : Array External) (entry : External) : Array External × Nat :=
  match entries.findIdx? (· == entry) with
  | some index => (entries, index)
  | none => (entries.push entry, entries.size)

/-- The declaration a nominal name denotes, when the unit or one of its
dependency interfaces declares it. -/
private def nominalDecl? (name : NameId) : M (Option StructDecl) := do
  let context ← read
  if let some handle := resolveNominal? context.unit context.ns name then
    return (context.unit.namespaces[handle.namespaceId.index]?).bind (·.structs[handle.structId]?)
  let some qualified := context.ns.tables.names[name.index]? | return none
  return context.unit.dependencies.findSome? fun dependency =>
    if dependency.namespaceId == qualified.namespaceId then
      dependency.structs.find? (·.name == name)
    else none

/-- The XIR struct id of a nominal name, and whether it is an enum. -/
private def structIdOf (loc : Option LocId) (name : NameId) : M (Nat × Bool) := do
  let context ← read
  let some qualified := context.ns.tables.names[name.index]?
    | fail loc s!"name {name.index} is out of range"
  let isEnum := ((← nominalDecl? name).map (!·.variants.isEmpty)).getD false
  if qualified.namespaceId == context.namespaceId then
    let some typeId := context.unit.resolution.nominal? name
      | fail loc s!"`{qualified.name}` is not a declared type"
    return (typeId.index, isEnum)
  let (address, module) ← moduleOf loc qualified.namespaceId
  let state ← get
  let (entries, index) := external state.externalStructs { address, module, name := qualified.name }
  set { state with externalStructs := entries }
  return (context.ns.structs.size + index, isEnum)

/-- The XIR function id a call's target denotes. -/
private def functionIdOf (loc : Option LocId) (reference : QualifiedRef) : M Nat := do
  let context ← read
  let name ← nameOf reference.name
  if reference.namespaceId == context.namespaceId then
    let some functionId := context.unit.resolution.function? reference.name
      | fail loc s!"`{name}` is not a declared function"
    return functionId.index
  let (address, module) ← moduleOf loc reference.namespaceId
  let state ← get
  let (entries, index) := external state.externalFunctions { address, module, name }
  set { state with externalFunctions := entries }
  return context.ns.functions.size + index

/-- A function of the Move standard library. -/
private def stdFunction (module name : String) : M Nat := do
  let context ← read
  let state ← get
  let (entries, index) := external state.externalFunctions { address := "0x1", module, name }
  set { state with externalFunctions := entries }
  return context.ns.functions.size + index

/-! ## Types -/

private def abilityName : Ability → String
  | .copy => "copy" | .drop => "drop" | .store => "store" | .key => "key"

private def intType? : IntWidth → Bool → Option IntType
  | .bits 8, false => some .u8 | .bits 16, false => some .u16 | .bits 32, false => some .u32
  | .bits 64, false => some .u64 | .bits 128, false => some .u128
  | .bits 256, false => some .u256
  | .bits 8, true => some .i8 | .bits 16, true => some .i16 | .bits 32, true => some .i32
  | .bits 64, true => some .i64 | .bits 128, true => some .i128
  | .bits 256, true => some .i256
  | _, _ => none

private def typeOf (loc : Option LocId) (typeId : TypeId) : M LeanerIR.Ty := do
  let some type := (← tables).types[typeId.index]?
    | fail loc s!"type {typeId.index} is out of range"
  pure type

mutual
  /-- The XIR types a value of an LIR type occupies: none for a unit, one
  per element for a tuple. -/
  partial def xirTypes (loc : Option LocId) (typeId : TypeId) : M (Array Xir.Ty) := do
    match ← typeOf loc typeId with
    | .unit | .never => pure #[]
    | .tuple elements => elements.flatMapM (xirTypes loc)
    | _ => pure #[← xirType loc typeId]

  /-- The XIR type of a single-valued LIR type. -/
  partial def xirType (loc : Option LocId) (typeId : TypeId) : M Xir.Ty := do
    match ← typeOf loc typeId with
    | .bool => pure .bool
    | .address => pure .address
    | .signer => pure .signer
    | .integer width signed =>
        match intType? width signed with
        | some type => pure (.int type)
        | none => fail loc "Move bytecode has no unbounded or pointer-width integers"
    | .vector element none => pure (.vector (← xirType loc element))
    | .typeParameter index => pure (.typeParameter index)
    | .reference value =>
        let referent ← xirType loc value.referent
        pure <| if value.kind == .mutable then .mutRef referent else .ref referent
    | .nominal name arguments =>
        let arguments ← arguments.mapM fun
          | .typeArg use => xirType loc use.typeId
          | _ => fail loc "Move type arguments are types"
        let (id, isEnum) ← structIdOf loc name
        pure <| if isEnum then .enum id arguments else .struct id arguments
    | .function parameters result abilities =>
        pure (.function (← parameters.mapM (xirType loc)) (← xirTypes loc result)
          (abilities.map abilityName))
    | .unit | .never | .tuple _ => fail loc "a unit or tuple type is not a single Move value"
    | type => fail loc s!"the type `{(repr type).pretty}` has no Move bytecode form"
end

/-- Substitute a declaration's type parameters. -/
partial def Ty.instantiate (arguments : Array Xir.Ty) : Xir.Ty → Xir.Ty
  | .typeParameter index => arguments[index]?.getD (.typeParameter index)
  | .struct id inner => .struct id (inner.map (Ty.instantiate arguments))
  | .enum id inner => .enum id (inner.map (Ty.instantiate arguments))
  | .vector element => .vector (element.instantiate arguments)
  | .ref referent => .ref (referent.instantiate arguments)
  | .mutRef referent => .mutRef (referent.instantiate arguments)
  | .function parameters results abilities =>
      .function (parameters.map (Ty.instantiate arguments))
        (results.map (Ty.instantiate arguments)) abilities
  | type => type

private def typeParameters (loc : Option LocId) (generics : Array GenericBinder) :
    M (Array TypeParameter) :=
  generics.mapM fun binder => do
    unless binder.kind == .typeArg do
      fail loc s!"the generic `{binder.name}` is not a type parameter"
    let phantom := binder.predicates.any fun
      | .profile { profile := .move, tag := "typeParameter.phantom", .. } => true
      | _ => false
    pure { name := binder.name, abilities := binder.abilities.map abilityName, phantom }

/-- A nominal type's declaration and type arguments. -/
private def nominalOf (loc : Option LocId) : Xir.Ty → M (Nat × Array Xir.Ty)
  | .struct id arguments | .enum id arguments => pure (id, arguments)
  | _ => fail loc "expected a struct or enum type"

/-- The fields of a nominal declaration, or of one of its variants, with
their types under `arguments`. -/
private def fieldsOf (loc : Option LocId) (owner : QualifiedRef) (variant : Option String)
    (arguments : Array Xir.Ty) : M (Array (NameId × Xir.Ty)) := do
  let some declaration ← nominalDecl? owner.name
    | fail loc s!"`{← nameOf owner.name}` has no declaration available"
  let fields ← match variant with
    | none => pure declaration.fields
    | some name => do
        let candidates ← declaration.variants.filterM fun candidate =>
          return (← nameOf candidate.name) == name
        let some selected := candidates[0]?
          | fail loc s!"`{← nameOf owner.name}` has no variant `{name}`"
        pure selected.fields
  fields.mapM fun field => return (field.name, (← xirType loc field.type.typeId).instantiate arguments)

/-- The reference to the nominal declaration a name denotes. -/
private def ownerOf (name : NameId) : M QualifiedRef := do
  let some qualified := (← tables).names[name.index]?
    | fail none s!"name {name.index} is out of range"
  pure { namespaceId := qualified.namespaceId, name }

/-- Whether a declaration is a structure without fields, which Move bytecode
represents with the one Boolean `dummy_field` Move's compilers add. -/
private def fieldless (owner : QualifiedRef) : M Bool := do
  let some declaration ← nominalDecl? owner.name | return false
  return declaration.fields.isEmpty && declaration.variants.isEmpty

/-- The offset of a named field. -/
private def fieldOffset (loc : Option LocId) (fields : Array (NameId × Xir.Ty)) (field : String) :
    M (Nat × Xir.Ty) := do
  for ((name, type), offset) in fields.zipIdx do
    if (← nameOf name) == field then return (offset, type)
  fail loc s!"no field `{field}`"

private def variantIndexOf (loc : Option LocId) (owner : QualifiedRef) (variant : String) :
    M Nat := do
  let some declaration ← nominalDecl? owner.name
    | fail loc s!"`{← nameOf owner.name}` has no declaration available"
  let names ← declaration.variants.mapM (nameOf ·.name)
  let some index := names.findIdx? (· == variant)
    | fail loc s!"`{← nameOf owner.name}` has no variant `{variant}`"
  pure index

/-! ## Function bodies -/

structure LoopFrame where
  header : Nat
  exit : Nat
  results : Array Nat

structure Builder where
  locals : Array Xir.Ty := #[]
  names : Array (Option String) := #[]
  /-- The XIR locals of each LIR slot, none for a unit. -/
  localMap : Array (Option Nat) := #[]
  /-- Blocks by id, filled when terminated. -/
  blocks : Array (Option Xir.Block) := #[]
  /-- Whether a jump or branch targets a block. -/
  targeted : Array Bool := #[]
  /-- The open block and its instructions; none where code is unreachable. -/
  current : Option (Nat × Array Xir.Instr × Array (Option Xir.Span)) := none
  loops : List LoopFrame := []
  span : Option Xir.Span := none
  /-- Where the current expression is, for failures. -/
  loc : Option LocId := none

abbrev BodyM := StateT Builder M

private def failHere (message : String) : BodyM α := do fail (← get).loc message

private def fresh (type : Xir.Ty) : BodyM Nat := do
  let builder ← get
  set { builder with locals := builder.locals.push type, names := builder.names.push none }
  pure builder.locals.size

private def localType (slot : Nat) : BodyM Xir.Ty := do
  let some type := (← get).locals[slot]? | failHere s!"local {slot} is out of range"
  pure type

private def emit (instr : Xir.Instr) : BodyM Unit :=
  modify fun builder => match builder.current with
    | none => builder
    | some (id, instrs, spans) =>
        { builder with current := some (id, instrs.push instr, spans.push builder.span) }

private def newBlock : BodyM Nat := do
  let builder ← get
  set { builder with blocks := builder.blocks.push none, targeted := builder.targeted.push false }
  pure builder.blocks.size

private def target (block : Nat) : BodyM Unit :=
  modify fun builder => { builder with targeted := builder.targeted.set! block true }

private def terminate (term : Xir.Term) : BodyM Unit := do
  let builder ← get
  let some (id, instrs, spans) := builder.current | return
  match term with
  | .jump block => target block
  | .branch _ thenBlock elseBlock => do target thenBlock; target elseBlock
  | _ => pure ()
  modify fun builder => { builder with
    blocks := builder.blocks.set! id (some {
      instrs := instrs, term := term, instrSpans := spans, termSpan := builder.span })
    current := none }

private def reachable : BodyM Bool := return (← get).current.isSome

/-- Continue at a block, falling through from open code. A block nothing
targets stays unreachable. -/
private def startBlock (block : Nat) : BodyM Unit := do
  terminate (.jump block)
  if (← get).targeted[block]?.getD false then
    modify fun builder => { builder with current := some (block, #[], #[]) }

private def assignAll (targets sources : Array Nat) : BodyM Unit := do
  for (destination, source) in targets.zip sources do
    unless destination == source do emit (.assign destination source)

private def freshFor (typeId : TypeId) : BodyM (Array Nat) := do
  (← xirTypes (← get).loc typeId).mapM fresh

private def localOf (localId : LocalId) : BodyM (Option Nat) := do
  let some mapped := (← get).localMap[localId.index]?
    | failHere s!"local {localId.index} is out of range"
  pure mapped

private def withLoc (loc : LocId) (action : BodyM α) : BodyM α := do
  let saved := (← get)
  let context ← read
  let span := do
    let location ← context.ns.tables.locations[loc.index]?
    let range ← location.primary
    pure { start := range.startByte, stop := range.endByte : Xir.Span }
  set { saved with loc := some loc, span := span.orElse fun _ => saved.span }
  let result ← action
  modify fun builder => { builder with loc := saved.loc, span := saved.span }
  pure result

private def exprAt (id : ExprId) : BodyM Expr := do
  let some expression := (← read).ns.expressions[id.index]?
    | failHere s!"expression {id.index} is out of range"
  pure expression

private def place (id : PlaceId) : BodyM Place := do
  let some place := (← read).ns.places[id.index]?
    | failHere s!"place {id.index} is out of range"
  pure place

private def pattern (id : PatternId) : BodyM Pattern := do
  let some pattern := (← read).ns.patterns[id.index]?
    | failHere s!"pattern {id.index} is out of range"
  pure pattern

/-- The element at an index a construct's arity guarantees. -/
private def nth (values : Array α) (index : Nat) (what : String) : BodyM α := do
  let some value := values[index]? | failHere s!"{what} has no operand {index}"
  pure value

private def single (values : Array Nat) : BodyM Nat := do
  let some value := values[0]? | failHere "expected a value"
  unless values.size == 1 do failHere "expected a single value"
  pure value

private def intTypeOf (slot : Nat) : BodyM IntType := do
  match ← localType slot with
  | .int type => pure type
  | _ => failHere "expected an integer operand"

private def referent : Xir.Ty → BodyM Xir.Ty
  | .ref referent | .mutRef referent => pure referent
  | _ => failHere "expected a reference"

/-- Whether a place stores into a local itself: its path reaches the local
without passing through a reference. -/
private partial def storesInto (target : LocalId) (id : PlaceId) : BodyM Bool := do
  match ← place id with
  | .localVar localId => pure (localId == target)
  | .deref _ => pure false
  | .field base .. | .index base _ | .subslice base .. | .downcast base _ => storesInto target base

private partial def patternBinds (target : LocalId) (id : PatternId) : BodyM Bool := do
  match (← pattern id).kind with
  | .variable localId => pure (localId == target)
  | .tuple elements | .constructor _ _ _ elements => elements.anyM (patternBinds target)
  | _ => pure false

/-- Whether evaluating an expression can change a local: it assigns it, or
borrows it mutably. -/
private partial def mayWrite (target : LocalId) (id : ExprId) : BodyM Bool := do
  let expression ← exprAt id
  let here ← match expression.kind with
    | .assign written _ => storesInto target written
    | .assignPattern binding _ => patternBinds target binding
    | .operation (.borrow .mutable borrowed) .. | .operation (.write borrowed) ..
    | .operation (.move borrowed) .. => storesInto target borrowed
    | .operation (.reference (.borrow .mutable)) _ #[operand] _ =>
        return (← exprAt operand).kind == .localVar target
    | _ => pure false
  if here then return true
  (expressionChildren expression.kind).anyM (mayWrite target)

private def valueConst (loc : Option LocId) : ConstValue → M Xir.Value
  | .bool value => pure (.bool value)
  | .integer value =>
      if value < 0 then fail loc "Move bytecode has no negative integers"
      else pure (.num value.toNat)
  | .address value => pure (.address value)
  | .bytes value => pure (.vector (value.map fun byte => .num byte.toNat))
  | .vector elements => .vector <$> elements.mapM (valueConst loc)
  | _ => fail loc "the constant has no Move bytecode form"

mutual
  /-- Lower an expression, returning the locals holding its value. -/
  partial def lowerExpr (id : ExprId) : BodyM (Array Nat) := do
    let expression ← exprAt id
    withLoc expression.loc do
    match expression.kind with
    | .value .unit _ => pure #[]
    | .value (.tuple elements) _ => do
        let types ← xirTypes (some expression.loc) expression.typeId
        let mut values := #[]
        for (element, type) in elements.zip types do
          let destination ← fresh type
          emit (.load destination (← valueConst (some expression.loc) element))
          values := values.push destination
        pure values
    | .value value _ => do
        let destination ← fresh (← xirType (some expression.loc) expression.typeId)
        emit (.load destination (← valueConst (some expression.loc) value))
        pure #[destination]
    | .constant reference => do
        let context ← read
        let some handle := resolveConstant? context.unit context.namespaceId reference
          | failHere s!"`{← nameOf reference.name}` is not a declared constant"
        unless handle.namespaceId == context.namespaceId do
          failHere "a constant of another module has no Move bytecode form"
        let some declaration := context.ns.constants[handle.constantId]?
          | failHere "a constant is out of range"
        lowerExpr declaration.value
    | .localVar localId => pure ((← localOf localId).toArray)
    | .operation operation instantiations arguments _ =>
        lowerOperation expression operation instantiations arguments
    | .block statements result => do
        for statement in statements do discard <| lowerExpr statement
        match result with
        | some result => lowerExpr result
        | none => pure #[]
    | .letDecl binding value body => do
        if let some value := value then
          bindPattern binding (← lowerExpr value)
        lowerExpr body
    | .ifElse condition thenBranch elseBranch => do
        let results ← freshFor expression.typeId
        let condition ← single (← lowerExpr condition)
        let thenBlock ← newBlock
        let elseBlock ← newBlock
        let join ← newBlock
        terminate (.branch condition thenBlock elseBlock)
        startBlock thenBlock
        assignAll results (← lowerExpr thenBranch)
        terminate (.jump join)
        startBlock elseBlock
        if let some elseBranch := elseBranch then
          assignAll results (← lowerExpr elseBranch)
        terminate (.jump join)
        startBlock join
        pure results
    | .match_ scrutinee arms => lowerMatch expression scrutinee arms
    | .loop _ body => do
        let results ← freshFor expression.typeId
        let header ← newBlock
        let exit ← newBlock
        terminate (.jump header)
        startBlock header
        modify fun builder => { builder with loops := { header, exit, results } :: builder.loops }
        discard <| lowerExpr body
        terminate (.jump header)
        modify fun builder => { builder with loops := builder.loops.drop 1 }
        startBlock exit
        pure results
    | .break_ nest value => do
        let some frame := (← get).loops[nest]? | failHere "a break outside a loop"
        if let some value := value then
          assignAll frame.results (← lowerExpr value)
        terminate (.jump frame.exit)
        pure #[]
    | .continue_ nest => do
        let some frame := (← get).loops[nest]? | failHere "a continue outside a loop"
        terminate (.jump frame.header)
        pure #[]
    | .return_ values => do
        let values ← lowerOperands values
        terminate (.ret values.flatten)
        pure #[]
    | .throw_ .abort #[code] => do
        let code ← single (← lowerExpr code)
        terminate (.abort code)
        pure #[]
    | .throw_ .. => failHere "only an abort with a code has a Move bytecode form"
    | .assign target value => do
        let value ← lowerExpr value
        assignPlace target value
        pure #[]
    | .assignPattern binding value => do
        bindPattern binding (← lowerExpr value)
        pure #[]
    | .spec _ => pure #[]
    | .quantifier .. => failHere "a quantifier is not executable"

  /-- Lower operands left to right. An operand naming a slot is read into
  a temporary when a later operand could change that slot. -/
  partial def lowerOperands (arguments : Array ExprId) : BodyM (Array (Array Nat)) := do
    let mut values := #[]
    for (argument, index) in arguments.zipIdx do
      let value ← lowerExpr argument
      let later := arguments.extract (index + 1) arguments.size
      let named ← match (← exprAt argument).kind with
        | .localVar localId => pure (some localId)
        | .operation (.read read) .. | .operation (.copy read) .. | .operation (.move read) .. =>
            match ← place read with
            | .localVar localId => pure (some localId)
            | _ => pure none
        | _ => pure none
      let changed ← match named with
        | some localId => later.anyM (mayWrite localId)
        | none => pure false
      if changed then
        let copied ← value.mapM fun slot => do
          let temporary ← fresh (← localType slot)
          emit (.assign temporary slot)
          pure temporary
        values := values.push copied
      else values := values.push value
    pure values

  /-- A reference to the value an expression denotes: the place it reads
  when it reads one, else a borrow of its value. -/
  partial def lowerRef (id : ExprId) (mutable : Bool) : BodyM Nat := do
    let expression ← exprAt id
    withLoc expression.loc do
    match expression.kind with
    | .localVar localId =>
        let some slot ← localOf localId | failHere "a unit value has no reference"
        borrowLocal slot mutable
    | .operation (.read target) _ _ _ | .operation (.copy target) _ _ _ =>
        borrowPlace target mutable
    | .operation (.reference .dereference) _ #[reference] _ =>
        coerceRef (← single (← lowerExpr reference)) mutable
    | _ =>
        let value ← single (← lowerExpr id)
        borrowLocal value mutable

  /-- Whether an expression is a mutable reference. -/
  partial def isMutRef (operand : Expr) : BodyM Bool := do
    match ← typeOf (← get).loc operand.typeId with
    | .reference value => pure (value.kind == .mutable)
    | _ => pure false

  partial def borrowLocal (slot : Nat) (mutable : Bool) : BodyM Nat := do
    let type ← localType slot
    let reference ← fresh (if mutable then .mutRef type else .ref type)
    emit (.call #[reference] .borrowLoc #[slot])
    pure reference

  /-- A reference of the requested mutability, freezing a mutable one. -/
  partial def coerceRef (reference : Nat) (mutable : Bool) : BodyM Nat := do
    match ← localType reference with
    | .mutRef referent =>
        if mutable then pure reference
        else
          let frozen ← fresh (.ref referent)
          emit (.call #[frozen] .freezeRef #[reference])
          pure frozen
    | .ref _ =>
        if mutable then failHere "a shared reference cannot be borrowed mutably"
        else pure reference
    | _ => failHere "expected a reference"

  /-- The type of the value a place denotes. -/
  partial def placeType (id : PlaceId) : BodyM Xir.Ty := do
    match ← place id with
    | .localVar localId =>
        let some slot ← localOf localId | failHere "a unit local has no place"
        localType slot
    | .deref base => referent (← placeType base)
    | .field base owner field => do
        let (_, arguments) ← nominalOf (← get).loc (← placeType base)
        let variant ← match ← place base with
          | .downcast _ variant => some <$> nameOf variant
          | _ => pure none
        let fields ← fieldsOf (← get).loc owner variant arguments
        let some (_, type) := fields.find? (·.1 == field)
          | failHere s!"`{← nameOf owner.name}` has no field `{← nameOf field}`"
        pure type
    | .index base _ =>
        match ← placeType base with
        | .vector element => pure element
        | _ => failHere "an indexed place is not a vector"
    | .downcast base _ => placeType base
    | .subslice .. => failHere "a subslice has no Move bytecode form"

  /-- Borrow a place, returning the reference slot. -/
  partial def borrowPlace (id : PlaceId) (mutable : Bool) : BodyM Nat := do
    let wrap (type : Xir.Ty) := if mutable then Xir.Ty.mutRef type else .ref type
    match ← place id with
    | .localVar localId =>
        let some slot ← localOf localId | failHere "a unit local has no place"
        borrowLocal slot mutable
    | .deref base =>
        match ← place base with
        | .localVar localId =>
            let some slot ← localOf localId | failHere "a unit local has no place"
            coerceRef slot mutable
        | _ => coerceRef (← readPlace base) mutable
    | .field base owner field => do
        let fieldType ← placeType id
        let (_, arguments) ← nominalOf (← get).loc (← placeType base)
        let reference ← fresh (wrap fieldType)
        match ← place base with
        | .downcast enumBase variant =>
            let variantName ← nameOf variant
            let fields ← fieldsOf (← get).loc owner (some variantName) arguments
            let some offset := fields.findIdx? (·.1 == field)
              | failHere s!"variant `{variantName}` has no field `{← nameOf field}`"
            let base ← borrowPlace enumBase mutable
            let variantIndex ← variantIndexOf (← get).loc owner variantName
            emit (.call #[reference] (.borrowVariantField #[variantIndex] offset arguments) #[base])
        | _ =>
            let fields ← fieldsOf (← get).loc owner none arguments
            let some offset := fields.findIdx? (·.1 == field)
              | failHere s!"`{← nameOf owner.name}` has no field `{← nameOf field}`"
            let base ← borrowPlace base mutable
            emit (.call #[reference] (.borrowField offset arguments) #[base])
        pure reference
    | .index base index => do
        let element ← placeType id
        let vector ← borrowPlace base mutable
        let index ← single (← lowerExpr index)
        let reference ← fresh (wrap element)
        emit (.call #[reference] .borrowVecElem #[vector, index])
        pure reference
    | .downcast .. => failHere "a whole variant cannot be borrowed in Move bytecode"
    | .subslice .. => failHere "a subslice has no Move bytecode form"

  /-- The value of a place: the slot itself, or a read through a borrow. -/
  partial def readPlace (id : PlaceId) : BodyM Nat := do
    match ← place id with
    | .localVar localId =>
        let some slot ← localOf localId | failHere "a unit local has no place"
        pure slot
    | _ =>
        let reference ← borrowPlace id false
        let value ← fresh (← placeType id)
        emit (.call #[value] .readRef #[reference])
        pure value

  partial def assignPlace (id : PlaceId) (value : Array Nat) : BodyM Unit := do
    match ← place id with
    | .localVar localId =>
        if let some slot ← localOf localId then
          assignAll #[slot] value
    | _ =>
        let reference ← borrowPlace id true
        emit (.call #[] .writeRef #[reference, ← single value])

  /-- Bind a pattern to the locals holding a value. -/
  partial def bindPattern (id : PatternId) (value : Array Nat) : BodyM Unit := do
    let binding ← pattern id
    match binding.kind with
    | .wildcard => pure ()
    | .variable localId =>
        if let some slot ← localOf localId then
          assignAll #[slot] value
    | .tuple elements => do
        let mut offset := 0
        for element in elements do
          let width := (← xirTypes (← get).loc (← pattern element).typeId).size
          bindPattern element (value.extract offset (offset + width))
          offset := offset + width
    | .constructor name _ variant fields => do
        let owner ← ownerOf name
        let type ← xirType (← get).loc binding.typeId
        let (_, arguments) ← nominalOf (← get).loc type
        let declared ← fieldsOf (← get).loc owner variant arguments
        let parts ← declared.mapM fun (_, type) => fresh type
        let unpacked ← if ← fieldless owner then pure #[← fresh .bool] else pure parts
        let source ← single value
        match variant with
        | none => emit (.call unpacked (.unpack arguments) #[source])
        | some variant =>
            let index ← variantIndexOf (← get).loc owner variant
            emit (.call unpacked (.unpackVariant index arguments) #[source])
        for (field, part) in fields.zip parts do bindPattern field #[part]
    | .literal _ | .range .. => failHere "a refutable pattern cannot bind a value"

  /-- Whether a pattern matches every value of its type. -/
  partial def irrefutable (id : PatternId) : BodyM Bool := do
    match (← pattern id).kind with
    | .wildcard | .variable _ => pure true
    | .tuple elements | .constructor _ _ none elements => elements.allM irrefutable
    | _ => pure false

  /-- Whether the arms of a match cover every value of its scrutinee: an
  irrefutable arm does, and so do arms naming every variant of the enum, or
  both Booleans. -/
  partial def covers (arms : Array MatchArm) : BodyM Bool := do
    let mut owner : Option NameId := none
    let mut variants : Array String := #[]
    let mut booleans : Array Bool := #[]
    for arm in arms do
      if arm.guard.isSome then continue
      match (← pattern arm.pattern).kind with
      | .constructor name _ (some variant) fields =>
          if ← fields.allM irrefutable then
            owner := some name
            variants := variants.push variant
      | .literal (.bool value) => booleans := booleans.push value
      | _ => if ← irrefutable arm.pattern then return true
    if booleans.contains true && booleans.contains false then return true
    let some name := owner | return false
    let some declaration ← nominalDecl? name | return false
    declaration.variants.allM fun variant => return variants.contains (← nameOf variant.name)

  /-- A match selects its arm by the scrutinee's variant or literal. Move
  requires the arms to cover the scrutinee, which validation does not
  establish; once they do, the last arm is taken unconditionally. -/
  partial def lowerMatch (expression : Expr) (scrutinee : ExprId) (arms : Array MatchArm) :
      BodyM (Array Nat) := do
    unless ← covers arms do failHere "a match must cover every value of its scrutinee"
    let results ← freshFor expression.typeId
    let value ← single (← lowerExpr scrutinee)
    let join ← newBlock
    for (arm, index) in arms.zipIdx do
      if arm.guard.isSome then failHere "a match guard has no Move bytecode form yet"
      let last := index + 1 == arms.size
      let next ← newBlock
      let test : Option (BodyM Nat) ← match (← pattern arm.pattern).kind with
        | .constructor name _ (some variant) fields => do
            unless ← fields.allM irrefutable do
              failHere "a nested refutable pattern has no Move bytecode form yet"
            let (_, arguments) ← nominalOf (← get).loc (← localType value)
            let index ← variantIndexOf (← get).loc (← ownerOf name) variant
            pure <| some do
              let test ← fresh .bool
              emit (.call #[test] (.testVariant index arguments) #[value])
              pure test
        | .literal constant => pure <| some do
            let expected ← fresh (← localType value)
            emit (.load expected (← valueConst (← get).loc constant))
            let test ← fresh .bool
            emit (.call #[test] .eq #[value, expected])
            pure test
        | .range .. => failHere "a range pattern has no Move bytecode form"
        | _ =>
            unless ← irrefutable arm.pattern do
              failHere "a nested refutable pattern has no Move bytecode form yet"
            pure none
      if let some test := test then
        unless last do
          let test ← test
          let matched ← newBlock
          terminate (.branch test matched next)
          startBlock matched
      bindPattern arm.pattern #[value]
      assignAll results (← lowerExpr arm.body)
      terminate (.jump join)
      startBlock next
    startBlock join
    pure results

  partial def lowerOperation (expression : Expr) (operation : Operation)
      (instantiations : Array GenericArgument) (arguments : Array ExprId) :
      BodyM (Array Nat) := do
    let loc := some expression.loc
    let typeArguments := do
      instantiations.mapM fun
        | .typeArg use => xirType loc use.typeId
        | _ => fail loc "Move type arguments are types"
    let resultTypes := xirTypes loc expression.typeId
    -- The operand of a field or variant access as a reference, and whether
    -- the access yields a reference (its operand is one) or a value.
    let accessBase : BodyM (Nat × Bool) := do
      let operand ← exprAt (← nth arguments 0 "an access")
      if (← typeOf loc operand.typeId) matches .reference _ then
        return (← coerceRef (← single (← lowerExpr (← nth arguments 0 "the operation"))) (← isMutRef operand), true)
      return (← lowerRef (← nth arguments 0 "the operation") false, false)
    let call (oper : Xir.Oper) (sources : Array Nat) : BodyM (Array Nat) := do
      let destinations ← (← resultTypes).mapM fresh
      emit (.call destinations oper sources)
      pure destinations
    -- The field reference an access yields, and the value when it reads.
    let access (fieldType : Xir.Ty) (byRef : Bool) (oper : Xir.Oper) (base : Nat) :
        BodyM (Array Nat) := do
      let reference ← fresh (← if byRef then xirType loc expression.typeId
        else pure (.ref fieldType))
      emit (.call #[reference] oper #[base])
      if byRef then pure #[reference] else call .readRef #[reference]
    match operation with
    | .call (.function reference) => do
        let function ← functionIdOf loc reference
        let arguments ← lowerOperands arguments
        call (.function function (← typeArguments)) arguments.flatten
    | .call (.constructor owner variant) => do
        let type ← xirType loc expression.typeId
        let (_, typeArgs) ← nominalOf loc type
        let arguments ← lowerOperands arguments
        match variant with
        | none =>
            if ← fieldless owner then
              let dummy ← fresh .bool
              emit (.load dummy (.bool false))
              call (.pack typeArgs) #[dummy]
            else call (.pack typeArgs) arguments.flatten
        | some variant =>
            call (.packVariant (← variantIndexOf loc owner variant) typeArgs) arguments.flatten
    | .call (.destructor owner variant) => do
        let value ← single (← lowerExpr (← nth arguments 0 "the operation"))
        let (_, typeArgs) ← nominalOf loc (← localType value)
        match variant with
        | none =>
            if ← fieldless owner then
              emit (.call #[← fresh .bool] (.unpack typeArgs) #[value])
              pure #[]
            else call (.unpack typeArgs) #[value]
        | some variant => call (.unpackVariant (← variantIndexOf loc owner variant) typeArgs) #[value]
    | .call (.closure reference mask) => do
        let function ← functionIdOf loc reference
        let captures ← lowerOperands arguments
        call (.closure function mask (← typeArguments)) captures.flatten
    | .call .invoke => do
        -- LIR passes the function value first, XIR last.
        let operands ← lowerOperands arguments
        let some callable := operands[0]?
          | failHere "an invocation has no function value"
        call .invoke (operands.extract 1 |>.flatten |>.append callable)
    | .call (.extension ..) => failHere "a profile call has no Move bytecode form"
    | .borrow kind target => pure #[← borrowPlace target (kind == .mutable)]
    | .read target | .copy target | .move target => pure #[← readPlace target]
    | .write target => do
        let value ← lowerExpr (← nth arguments 0 "the operation")
        assignPlace target value
        pure #[]
    | .drop _ => pure #[]
    | .global kind => do
        let some (GenericArgument.typeArg resource) := instantiations[0]?
          | failHere "a global operation names its resource type"
        let (struct, typeArgs) ← nominalOf loc (← xirType loc resource.typeId)
        match kind with
        | .contains => call (.exists struct typeArgs) (← lowerOperands arguments).flatten
        | .borrow _ => call (.borrowGlobal struct typeArgs) (← lowerOperands arguments).flatten
        | .take => call (.moveFrom struct typeArgs) (← lowerOperands arguments).flatten
        | .publish =>
            let operands ← lowerOperands arguments
            let signer ← single (← nth operands 0 "the operation")
            unless (← localType signer) matches .ref .signer do
              failHere "Move bytecode publishes a resource under a `&signer`"
            call (.moveTo struct typeArgs) operands.flatten
    | .reference (.borrow kind) => pure #[← lowerRef (← nth arguments 0 "the operation") (kind == .mutable)]
    | .reference .dereference => do
        let reference ← single (← lowerExpr (← nth arguments 0 "the operation"))
        call .readRef #[reference]
    | .reference (.freeze _) => do
        let reference ← single (← lowerExpr (← nth arguments 0 "the operation"))
        call .freezeRef #[reference]
    | .reference .mutate => do
        let operands ← lowerOperands arguments
        emit (.call #[] .writeRef operands.flatten)
        pure #[]
    | .data (.select owner field) => do
        let (base, byRef) ← accessBase
        let (_, typeArgs) ← nominalOf loc (← referent (← localType base))
        let (offset, fieldType) ← fieldOffset loc (← fieldsOf loc owner none typeArgs) field
        access fieldType byRef (.borrowField offset typeArgs) base
    | .data (.selectVariants owner selected) => do
        let (base, byRef) ← accessBase
        let (_, typeArgs) ← nominalOf loc (← referent (← localType base))
        let some declaration ← nominalDecl? owner.name
          | failHere s!"`{← nameOf owner.name}` has no declaration available"
        -- Each listed pair resolves to its variant's index and the field's
        -- offset and type.
        let chosen : Array (Nat × Nat × Xir.Ty) ← selected.mapM
            fun (variantName, fieldName) => do
          let some (_, index) ← declaration.variants.zipIdx.findM? fun (variant, _) =>
              return (← nameOf variant.name) == variantName
            | failHere s!"variant `{variantName}` is not declared"
          let fields ← fieldsOf loc owner (some variantName) typeArgs
          let some (field, offset) ← fields.zipIdx.findM? fun (field, _) =>
              return (← nameOf field.1) == fieldName
            | failHere s!"variant `{variantName}` declares no field `{fieldName}`"
          pure (index, offset, field.2)
        let some (_, offset, selectedType) := chosen[0]?
          | failHere "a variant field selection names no variant"
        unless chosen.all (·.2.1 == offset) do
          failHere "Move bytecode selects a variant field at one offset in every variant"
        access selectedType byRef (.borrowVariantField (chosen.map (·.1)) offset typeArgs) base
    | .data (.testVariants owner variants) => do
        let (base, _) ← accessBase
        let base ← coerceRef base false
        let (_, typeArgs) ← nominalOf loc (← referent (← localType base))
        let mut result := none
        for variant in variants do
          let test ← fresh .bool
          emit (.call #[test] (.testVariantRef (← variantIndexOf loc owner variant) typeArgs) #[base])
          result ← match result with
            | none => pure (some test)
            | some previous => do
                let combined ← fresh .bool
                emit (.call #[combined] .or #[previous, test])
                pure (some combined)
        let some combined := result | failHere "a variant test names no variant"
        pure #[combined]
    | .data (.updateField owner field) => do
        let operands ← lowerOperands arguments
        let value ← single (← nth operands 0 "the operation")
        let updated ← fresh (← localType value)
        emit (.assign updated value)
        let (_, typeArgs) ← nominalOf loc (← localType updated)
        let (offset, fieldType) ← fieldOffset loc (← fieldsOf loc owner none typeArgs) field
        let base ← borrowLocal updated true
        let reference ← fresh (.mutRef fieldType)
        emit (.call #[reference] (.borrowField offset typeArgs) #[base])
        emit (.call #[] .writeRef #[reference, ← single (← nth operands 1 "the operation")])
        pure #[updated]
    | .data (.discriminant _) => failHere "a variant discriminant has no Move bytecode form"
    | .primitive primitive => lowerPrimitive expression primitive arguments call
    | .assert => failHere "an assertion without an abort code has no Move bytecode form"
    | .specification _ => failHere "a specification operation is not executable"
    | .profile .. => failHere "the profile operation has no Move bytecode form"

  partial def lowerPrimitive (expression : Expr) (primitive : PrimitiveOperation)
      (arguments : Array ExprId) (call : Xir.Oper → Array Nat → BodyM (Array Nat)) :
      BodyM (Array Nat) := do
    let operands := do return (← lowerOperands arguments).flatten
    let arithmetic (oper : IntType → Xir.Oper) := do
      let operands ← operands
      call (oper (← intTypeOf (← nth operands 0 "the operation"))) operands
    let swapped (oper : Xir.Oper) := do
      let operands ← operands
      call oper #[← nth operands 1 "the comparison", ← nth operands 0 "the comparison"]
    match primitive with
    | .checkedAdd .abort => arithmetic .add
    | .checkedSubtract .abort => arithmetic .sub
    | .checkedMultiply .abort => arithmetic .mul
    | .checkedDivide .abort => arithmetic .div
    | .checkedModulo .abort => arithmetic .mod
    | .checkedShiftLeft .abort => arithmetic .shl
    | .checkedShiftRight .abort => arithmetic .shr
    | .bitwiseAnd => arithmetic .bitAnd
    | .bitwiseOr => arithmetic .bitOr
    | .bitwiseXor => arithmetic .bitXor
    | .checkedCast .abort => do
        let target ← xirType (some expression.loc) expression.typeId
        let .int type := target | failHere "a cast targets an integer type"
        call (.cast type) (← operands)
    | .less => call .lt (← operands)
    | .lessEqual => call .le (← operands)
    | .greater => swapped .lt
    | .greaterEqual => swapped .le
    | .equal => call .eq (← operands)
    | .notEqual => do
        let equal ← single (← call .eq (← operands))
        call .not #[equal]
    | .logicalAnd => call .and (← operands)
    | .logicalOr => call .or (← operands)
    | .logicalNot => call .not (← operands)
    | .copyValue | .moveValue => do
        let value ← single (← operands)
        let copied ← fresh (← localType value)
        emit (.assign copied value)
        pure #[copied]
    | .tuple => operands
    | .vector => call .vecPack (← operands)
    | .length => do
        let vector ← lowerRef (← nth arguments 0 "the operation") false
        call .vecLen #[vector]
    | .index => do
        let vector ← lowerRef (← nth arguments 0 "the operation") false
        let index ← single (← lowerExpr (← nth arguments 1 "the operation"))
        call .vecGet #[vector, index]
    | .checkVectorIndex _ => do
        -- The element borrow fails exactly when the index is out of range.
        let vector ← lowerRef (← nth arguments 0 "the operation") false
        let index ← single (← lowerExpr (← nth arguments 1 "the operation"))
        let element ← referent (← localType vector)
        let .vector element := element | failHere "an index check needs a vector"
        let reference ← fresh (.ref element)
        emit (.call #[reference] .borrowVecElem #[vector, index])
        pure #[]
    | .pushVector => call .vecPush (← operands)
    | .insertVector => call .vecInsert (← operands)
    | .swapVector => call .vecSwap (← operands)
    | .removeVector => do
        -- XIR yields `(vector, element)`, LIR `(element, vector)`.
        let operands ← operands
        let vectorType ← localType (← nth operands 0 "the removal")
        let .vector elementType := vectorType | failHere "a removal needs a vector"
        let remaining ← fresh vectorType
        let element ← fresh elementType
        emit (.call #[remaining, element] .vecRemove operands)
        pure #[element, remaining]
    | .destroyEmptyVector => do
        let function ← stdFunction "vector" "destroy_empty"
        let operands ← operands
        let element ← match ← localType (← nth operands 0 "the operation") with
          | .vector element => pure element
          | _ => failHere "an empty vector to destroy"
        call (.function function #[element]) operands
    | .signerAddress => do
        let function ← stdFunction "signer" "address_of"
        call (.function function #[]) (← operands)
    | _ => failHere s!"the primitive `{(repr primitive).pretty}` has no Move bytecode form"
end

/-! ## Declarations -/

/-- The modifiers XIR carries as fields, or that only Leaner reads. -/
private def modifierAttribute : LeanerIR.Attribute → Bool
  | .assign "visibility" .. | .call "entry" .. | .call "native" .. | .call "opaque" .. => true
  | _ => false

private def attributeValue (loc : Option LocId) (name : String) :
    LeanerIR.AttributeValue → M Xir.AttributeArg
  | .constant (.bool value) => pure (.bool value)
  | .constant (.integer value) =>
      if 0 ≤ value then pure (.num value.toNat)
      else fail loc s!"the attribute `{name}` has a negative value, which Move bytecode cannot carry"
  | .constant _ => fail loc s!"the attribute `{name}` has a value Move bytecode cannot carry"
  | .name none value | .qualifiedName value => pure (.name value #[])
  | .name (some namespaceId) value => do
      let some ns := (← tables).namespaces[namespaceId.index]?
        | fail loc s!"the attribute `{name}` names an unknown namespace"
      pure (.name ("::".intercalate (ns.segments.push value).toList) #[])

private partial def attributeArg (loc : Option LocId) : LeanerIR.Attribute → M Xir.AttributeArg
  | .call name args _ => do pure (.name name (← args.mapM (attributeArg loc)))
  | .assign name value _ => do pure (.name name #[← attributeValue loc name value])

/-- The source attributes a declaration passes to its bytecode, such as
`module_lock` or `persistent`. -/
private def lowerAttributes (loc : Option LocId) (attributes : Array LeanerIR.Attribute) :
    M (Array Xir.Attribute) :=
  (attributes.filter (!modifierAttribute ·)).mapM fun
    | .call name args _ => do pure { name, args := ← args.mapM (attributeArg loc) }
    | .assign name value _ => do pure { name, args := #[← attributeValue loc name value] }

private def lowerStruct (declaration : StructDecl) : M Xir.Struct := do
  let loc := some declaration.loc
  let name ← nameOf declaration.name
  let typeParameters ← typeParameters loc declaration.generics
  let field (field : FieldDecl) : M Xir.Field := do
    pure { name := ← nameOf field.name, ty := ← xirType (some field.loc) field.type.typeId }
  let variants ← if declaration.variants.isEmpty then pure none
    else some <$> declaration.variants.mapM fun variant => do
      pure { name := ← nameOf variant.name, fields := ← variant.fields.mapM field }
  let fields ← if declaration.fields.isEmpty && declaration.variants.isEmpty then
      pure #[{ name := "dummy_field", ty := .bool : Xir.Field }]
    else declaration.fields.mapM field
  pure {
    name := name, abilities := declaration.abilities.map abilityName,
    typeParameters := typeParameters, fields := fields, variants := variants,
    attributes := ← lowerAttributes loc declaration.attributes }

private def lowerFunction (declaration : FunctionDecl FunctionBody) : M Xir.Function := do
  let loc := some declaration.loc
  let name ← nameOf declaration.name
  let modifiers ← match LeanerLang.declaredModifiers declaration with
    | .ok modifiers => pure modifiers
    | .error message => fail loc message
  let visibility ← match modifiers.visibility with
    | .private_ => pure Visibility.private_
    | .public_ => pure .public_
    | .friend => pure .friend
    | .package => fail loc "Move bytecode has no package visibility; declare it `friend`"
  let typeParameters ← typeParameters loc declaration.signature.generics
  let returns ← declaration.signature.results.flatMapM fun result =>
    xirTypes loc result.typeId
  let locations := (← tables).locations
  let span : Option Xir.Span := do
    let location ← locations[declaration.loc.index]?
    let range ← location.primary
    pure { start := range.startByte, stop := range.endByte }
  let base : Xir.Function := {
    name := name, typeParameters := typeParameters,
    visibility := visibility, isEntry := modifiers.isEntry, params := 0, locals := #[],
    returns := returns, blocks := #[], span := span,
    attributes := ← lowerAttributes loc declaration.attributes }
  match declaration.body with
  | .absent =>
      unless modifiers.isNative do
        fail loc s!"`{name}` has no body to compile; only a native function may omit it"
      let params ← declaration.signature.parameters.flatMapM fun parameter =>
        xirTypes loc parameter.typeUse.typeId
      pure { base with isNative := true, params := params.size, locals := params }
  | .structured root =>
      let body : BodyM Unit := do
        for (decl, index) in declaration.locals.zipIdx do
          let types ← xirTypes (some decl.loc) decl.type.typeId
          if index < declaration.signature.parameters.size && types.isEmpty then
            failHere s!"the parameter `{decl.name}` has no Move value"
          match types with
          | #[] => modify fun builder => { builder with localMap := builder.localMap.push none }
          | #[type] => do
              let slot ← fresh type
              modify fun builder => { builder with
                localMap := builder.localMap.push (some slot)
                names := builder.names.set! slot (some decl.name) }
          | _ => failHere s!"the local `{decl.name}` holds a tuple, which Move keeps in no local"
        let entry ← newBlock
        target entry
        startBlock entry
        let result ← lowerExpr root
        terminate (.ret result)
      let (_, builder) ← body.run { loc }
      let params := (builder.localMap.extract 0 declaration.signature.parameters.size).filterMap id
      unless params == (List.range params.size).toArray do
        fail loc "the parameters are not the leading locals"
      -- Unreachable blocks are dropped and the rest renumbered.
      let live := builder.blocks.zipIdx.filterMap fun (block, index) => block.map (·, index)
      let renumber (block : Nat) : Nat := (live.findIdx? (·.2 == block)).getD 0
      let blocks := live.map fun (block, _) => { block with term := match block.term with
        | .jump target => .jump (renumber target)
        | .branch condition thenBlock elseBlock =>
            .branch condition (renumber thenBlock) (renumber elseBlock)
        | term => term }
      pure { base with
        params := params.size, locals := builder.locals, localNames := builder.names,
        blocks := blocks }

/-- The resources each function acquires: the module's own it borrows or
moves from, directly or through a call within the module. -/
private def withAcquires (structs : Nat) (functions : Array Xir.Function) : Array Xir.Function :=
  Id.run do
    let direct := functions.map fun function => function.blocks.foldl (init := #[])
      fun found block => block.instrs.foldl (init := found) fun found => fun
        | .call _ (.borrowGlobal struct _) _ | .call _ (.moveFrom struct _) _ =>
            if struct < structs && !found.contains struct then found.push struct else found
        | _ => found
    let callees := functions.map fun function => function.blocks.foldl (init := #[])
      fun found block => block.instrs.foldl (init := found) fun found => fun
        | .call _ (.function callee _) _ =>
            if callee < functions.size && !found.contains callee then found.push callee else found
        | _ => found
    let mut acquires := direct
    let mut changed := true
    while changed do
      changed := false
      for index in [0:functions.size] do
        for callee in callees[index]?.getD #[] do
          for struct in acquires[callee]?.getD #[] do
            unless (acquires[index]?.getD #[]).contains struct do
              acquires := acquires.modify index (·.push struct)
              changed := true
    return functions.zipIdx.map fun (function, index) =>
      { function with acquires := (acquires[index]?.getD #[]).qsort (· < ·) }

/-- Lower one namespace of a validated unit to a deployable XIR module. -/
def lowerModule (unit : ValidatedUnit) (namespaceId : NamespaceId) : Except Failure Xir.Module := do
  let some ns := unit.namespaces[namespaceId.index]?
    | throw { loc := none, message := s!"namespace {namespaceId.index} is out of range" }
  let run : M Xir.Module := do
    unless ns.profile == some .move do
      fail (some ns.loc) "only a Move namespace compiles to Move bytecode"
    let (address, name) ← moduleOf (some ns.loc) ns.identity
    let structs ← ns.structs.mapM lowerStruct
    let functions ← ns.functions.mapM lowerFunction
    let state ← get
    pure {
      address := address, name := name, structs := structs,
      functions := withAcquires structs.size functions,
      externalFunctions := state.externalFunctions, externalStructs := state.externalStructs }
  let (module, _) ← (run.run { unit, namespaceId, ns }).run {}
  pure module

end LeanerIR.Move.Xir
