-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Compiler.Normalize
import Move.SourceArtifact
import Move.Verify.Syntax
import LeanerMove
import Transpiler.LIR.Codec

/-!
# Leaner Move to raw LIR

This adapter consumes Leaner's existing named compiler IR immediately after
Lean elaboration.  It is deliberately mechanical: names and source spans are
retained, three-address instructions become expression/assignment nodes, and
the named CFG is handed to the shared LIR structurizer.  No backend reads the
Lean environment or `Move.Compiler.LIR`.

For whole namespaces, the adapter also consumes the parsed source artifact
retained by the module elaborator. Supported function contracts become
ordinary LIR conditions and frames. Every source-semantic item outside the
current bridge is named in import evidence, so NSIR's omissions cannot become
silent semantic loss.
-/

namespace Transpiler.LIR.Leaner

open Transpiler Xast

structure SourceInfo where
  fileName : String
  sourceSize : Nat := 0
  contentHash : String := ""
  deriving Inhabited

private structure BuildState where
  source : Move.Compiler.LIR.Module
  sourceTypes : Array (Move.Compiler.LIR.Ty × LeanerIR.TypeId) := #[]
  namespaces : Array ModuleRef := #[]
  locations : Array LeanerIR.Location := #[]
  lifetimes : Array LeanerIR.Lifetime := #[]
  types : Array LeanerIR.Ty := #[]
  names : Array LeanerIR.QualifiedName := #[]
  expressions : Array LeanerIR.Expr := #[]
  patterns : Array LeanerIR.Pattern := #[]
  referenceAliases : Array (String × LeanerIR.ExprId) := #[]
  importedContracts : Nat := 0
  importedConstants : Nat := 0
  importedSpecFunctions : Nat := 0
  importedInvariants : Nat := 0
  unsupportedSource : Array String := #[]

private abbrev BuildM := StateT BuildState (Except String)

private def ownModule (source : Move.Compiler.LIR.Module) : ModuleRef :=
  { address := source.address, addressAlias := none, name := source.name }

private def addModuleRef (module : ModuleRef) : BuildM LeanerIR.NamespaceId := do
  let state ← get
  if let some index := state.namespaces.findIdx? (· == module) then return ⟨index⟩
  let id : LeanerIR.NamespaceId := ⟨state.namespaces.size⟩
  set { state with namespaces := state.namespaces.push module }
  return id

private def addName (name : QualifiedName) : BuildM LeanerIR.NameId := do
  let namespaceId ← addModuleRef name.module
  let state ← get
  if let some index := state.names.findIdx? fun candidate =>
      candidate.namespaceId == namespaceId && candidate.name == name.name then
    return ⟨index⟩
  let id : LeanerIR.NameId := ⟨state.names.size⟩
  set { state with names := state.names.push { namespaceId, name := name.name } }
  return id

private def addQualifiedRef (name : QualifiedName) : BuildM LeanerIR.QualifiedRef := do
  return { namespaceId := ← addModuleRef name.module, name := ← addName name }

private def qualifiedStruct (name : Lean.Name) : BuildM QualifiedName := do
  let source := (← get).source
  if let some declaration := source.structs.find? (·.leanName == name) then
    return { module := ownModule source, name := declaration.moveName }
  if let some declaration := source.externalStructs.find? (·.leanName == name) then
    return {
      module := { address := declaration.address, addressAlias := none, name := declaration.moduleName }
      name := declaration.structName }
  throw s!"Leaner type `{name.toString}` has no Move module identity"

private def qualifiedFunction (name : Lean.Name) : BuildM QualifiedName := do
  let source := (← get).source
  if let some declaration := source.functions.find? (·.leanName == name) then
    return { module := ownModule source, name := declaration.moveName }
  if let some declaration := source.externalFuns.find? (·.leanName == name) then
    return {
      module := { address := declaration.address, addressAlias := none, name := declaration.moduleName }
      name := declaration.functionName }
  throw s!"Leaner function `{name.toString}` has no Move module identity"

private def width : MoveModel.IR.IntWidth → Nat
  | .w8 => 8 | .w16 => 16 | .w32 => 32 | .w64 => 64
  | .w128 => 128 | .w256 => 256

private def addInferredLifetime (loc : LeanerIR.LocId) : BuildM LeanerIR.LifetimeId := do
  let state ← get
  let id : LeanerIR.LifetimeId := ⟨state.lifetimes.size⟩
  set { state with lifetimes := state.lifetimes.push { kind := .inference, loc } }
  return id

private partial def addType (loc : LeanerIR.LocId)
    (sourceType : Move.Compiler.LIR.Ty) : BuildM LeanerIR.TypeId := do
  let state ← get
  unless sourceType matches .ref _ | .mutRef _ do
    if let some entry := state.sourceTypes.find? (·.1 == sourceType) then return entry.2
  let lirType ← match sourceType with
    | .bool => pure LeanerIR.Ty.bool
    | .int number => pure <| .integer (.bits (width number.width)) number.signed
    | .address => pure .address
    | .signer => pure .signer
    | .typeParam index => pure <| .typeParameter index
    | .struct name | .enum name =>
        pure <| .nominal (← addName (← qualifiedStruct name)) #[]
    | .structInst name arguments | .enumInst name arguments => do
        let arguments ← arguments.mapM fun argument => do
          let typeId ← addType loc argument
          pure <| LeanerIR.GenericArgument.typeArg { typeId, loc }
        pure <| .nominal (← addName (← qualifiedStruct name)) arguments
    | .vector element => do
        let element ← addType loc element
        pure <| .vector element
    | .ref element | .mutRef element => do
        let mutable := sourceType matches .mutRef _
        let element ← addType loc element
        let lifetime ← addInferredLifetime loc
        pure <| LeanerIR.Ty.reference {
          profile := .move
          kind := if mutable then .mutable else .shared
          referent := element
          lifetime }
  let state ← get
  let id : LeanerIR.TypeId := ⟨state.types.size⟩
  set { state with
    sourceTypes := state.sourceTypes.push (sourceType, id)
    types := state.types.push lirType }
  return id

private def addPackedType (loc : LeanerIR.LocId)
    (types : Array Move.Compiler.LIR.Ty) : BuildM LeanerIR.TypeId := do
  match types with
  | #[] =>
      let state ← get
      if let some index := state.types.findIdx? (· == .unit) then return ⟨index⟩
      let id : LeanerIR.TypeId := ⟨state.types.size⟩
      set { state with types := state.types.push .unit }
      return id
  | #[type] => addType loc type
  | types => do
      let elements ← types.mapM (addType loc)
      let state ← get
      if let some index := state.types.findIdx? (· == .tuple elements) then return ⟨index⟩
      let id : LeanerIR.TypeId := ⟨state.types.size⟩
      set { state with types := state.types.push (.tuple elements) }
      return id

private def typeUse (loc : LeanerIR.LocId) (type : Move.Compiler.LIR.Ty) : BuildM LeanerIR.TypeUse := do
  return { typeId := ← addType loc type, loc }

private def addLocation (span : Option MoveModel.IR.SourceSpan) : BuildM LeanerIR.LocId := do
  match span with
  | none => pure ⟨0⟩
  | some span =>
      let state ← get
      let id : LeanerIR.LocId := ⟨state.locations.size⟩
      set { state with locations := state.locations.push {
        primary := some { file := ⟨0⟩, startByte := span.start, endByte := span.end } } }
      return id

private def addSyntaxLocation (stx : Lean.Syntax) : BuildM LeanerIR.LocId := do
  match stx.getPos?, stx.getTailPos? with
  | some start, some stop =>
      addLocation (some { start := start.byteIdx, «end» := stop.byteIdx })
  | _, _ => pure ⟨0⟩

private def addDirectType (type : LeanerIR.Ty) : BuildM LeanerIR.TypeId := do
  let state ← get
  if let some index := state.types.findIdx? (· == type) then return ⟨index⟩
  let id : LeanerIR.TypeId := ⟨state.types.size⟩
  set { state with types := state.types.push type }
  return id

private def boolType : BuildM LeanerIR.TypeId :=
  addDirectType .bool

private def numberType : BuildM LeanerIR.TypeId :=
  addDirectType (.integer .unbounded true)

private def addExpr (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId)
    (kind : LeanerIR.ExprKind) : BuildM LeanerIR.ExprId := do
  let state ← get
  let id : LeanerIR.ExprId := ⟨state.expressions.size⟩
  set { state with expressions := state.expressions.push { loc, typeId, kind } }
  return id

private def addPattern (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId)
    (kind : LeanerIR.PatternKind) : BuildM LeanerIR.PatternId := do
  let state ← get
  let id : LeanerIR.PatternId := ⟨state.patterns.size⟩
  set { state with patterns := state.patterns.push { loc, typeId, kind } }
  return id

private structure LocalContext where
  byCompilerName : Array (String × LeanerIR.LocalId)
  declarations : Array LeanerIR.LocalDecl

private def localId (context : LocalContext) (name : String) : Except String LeanerIR.LocalId :=
  match context.byCompilerName.find? (·.1 == name) with
  | some entry => .ok entry.2
  | none => .error s!"unknown Leaner compiler local `{name}`"

private def localDecl (context : LocalContext) (name : String) : Except String LeanerIR.LocalDecl := do
  let id ← localId context name
  match context.declarations[id.index]? with
  | some declaration => return declaration
  | none => throw s!"invalid declaration for compiler local `{name}`"

private def localExpr (context : LocalContext) (loc : LeanerIR.LocId)
    (name : String) : BuildM LeanerIR.ExprId := do
  if let some (_, expression) :=
      (← get).referenceAliases.find? (fun entry => entry.1 == name) then
    return expression
  let declaration ← localDecl context name
  addExpr loc declaration.type.typeId (.localVar declaration.id)

private def operation (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId)
    (op : Xast.Operation) (instantiations : Array Move.Compiler.LIR.Ty)
    (arguments : Array LeanerIR.ExprId) : BuildM LeanerIR.ExprId := do
  let operation ← match op with
    | .moveFunction name => pure <| LeanerIR.Operation.call (.function (← addQualifiedRef name))
    | .pack name variant => pure <| LeanerIR.Operation.call (.constructor (← addQualifiedRef name) variant)
    | .exists _ => pure <| LeanerIR.Operation.global .contains
    | .borrowGlobal .immutable => pure <| LeanerIR.Operation.global (.borrow .immutable)
    | .borrowGlobal .mutable => pure <| LeanerIR.Operation.global (.borrow .mutable)
    | .moveFrom => pure <| LeanerIR.Operation.global .take
    | .moveTo => pure <| LeanerIR.Operation.global .publish
    | .borrow .immutable => pure <| LeanerIR.Operation.reference (.borrow .immutable)
    | .borrow .mutable => pure <| LeanerIR.Operation.reference (.borrow .mutable)
    | .deref => pure <| LeanerIR.Operation.reference .dereference
    | .freeze explicit => pure <| LeanerIR.Operation.reference (.freeze explicit)
    | .select name field => pure <| .data (.select (← addQualifiedRef name) field)
    | .selectVariants name fields =>
        pure <| .data (.selectVariants (← addQualifiedRef name) fields.toArray)
    | .testVariants name variants =>
        pure <| .data (.testVariants (← addQualifiedRef name) variants.toArray)
    | .updateField name field => pure <| .data (.updateField (← addQualifiedRef name) field)
    | .specFunction name range =>
        pure <| .specification (.functionCall (← addQualifiedRef name) {
          pre := range.pre, post := range.post })
    | op => do
      match Codec.primitiveOperation? op with
      | some primitive => pure <| .primitive primitive
      | none =>
          match Codec.specOperation? op with
          | some specification => pure <| .specification specification
          | none =>
              let encoded := Codec.encodeOperation op
              let targets ← encoded.targets.mapM addQualifiedRef
              pure <| .profile {
                profile := .move, tag := encoded.tag, payload := encoded.payload } targets
  let instantiations ← instantiations.mapM fun ty =>
    return LeanerIR.GenericArgument.typeArg (← typeUse loc ty)
  addExpr loc typeId <| .operation operation instantiations arguments

private structure FunctionContext where
  locals : LocalContext
  sourceTypes : Array (String × Move.Compiler.LIR.Ty)
  aliasableReferences : Array String

private def sourceLocalType (context : FunctionContext) (name : String) : Except String Move.Compiler.LIR.Ty :=
  match context.sourceTypes.find? (·.1 == name) with
  | some entry => .ok entry.2
  | none => .error s!"unknown source type for compiler local `{name}`"

private def sourceLocalTypeM (context : FunctionContext) (name : String) :
    BuildM Move.Compiler.LIR.Ty :=
  match sourceLocalType context name with
  | .ok type => pure type
  | .error message => throw message

private def stripReference : Move.Compiler.LIR.Ty → Move.Compiler.LIR.Ty
  | .ref value | .mutRef value => value
  | value => value

private def referenceMutable : Move.Compiler.LIR.Ty → Option Bool
  | .ref _ => some false
  | .mutRef _ => some true
  | _ => none

private def ownerDecl (type : Move.Compiler.LIR.Ty) : BuildM Move.Compiler.LIR.StructDecl := do
  let name ← match stripReference type with
    | .struct name | .structInst name _ | .enum name | .enumInst name _ => pure name
    | type => throw s!"field operation owner has non-aggregate type `{repr type}`"
  let some declaration := (← get).source.structs.find? (·.leanName == name)
    | throw s!"field operation owner `{name}` is not declared in this module"
  return declaration

private def ownerInstantiation (type : Move.Compiler.LIR.Ty) : Array Move.Compiler.LIR.Ty :=
  match stripReference type with
  | .structInst _ arguments | .enumInst _ arguments => arguments
  | _ => #[]

private def fieldInfo (owner : Move.Compiler.LIR.Ty) (index : Nat) : BuildM (QualifiedName × String × Move.Compiler.LIR.Ty) := do
  let declaration ← ownerDecl owner
  let some field := declaration.fields[index]?
    | throw s!"invalid field {index} of `{declaration.moveName}`"
  return (← qualifiedStruct declaration.leanName, field.moveName, field.ty)

private def variantInfo (name : Lean.Name) (index : Nat) : BuildM (QualifiedName × String) := do
  let some declaration := (← get).source.structs.find? (·.leanName == name)
    | throw s!"unknown enum `{name.toString}`"
  let some variants := declaration.variants
    | throw s!"`{name.toString}` is not an enum"
  let some variant := variants[index]?
    | throw s!"invalid variant {index} of `{declaration.moveName}`"
  return (← qualifiedStruct name, variant.moveName)

private def vectorFunction (name : String) : QualifiedName := {
  module := { address := "0x1", addressAlias := some "std", name := "vector" }
  name }

private def assignment (context : FunctionContext) (loc : LeanerIR.LocId)
    (destinations : Array String) (value : LeanerIR.ExprId) : BuildM LeanerIR.ExprId := do
  let pattern ← match destinations with
    | #[destination] => do
        let declaration ← localDecl context.locals destination
        addPattern loc declaration.type.typeId (.variable declaration.id)
    | destinations => do
        let fields ← destinations.mapM fun destination => do
          let declaration ← localDecl context.locals destination
          addPattern loc declaration.type.typeId (.variable declaration.id)
        let sourceTypes ← destinations.mapM (sourceLocalTypeM context)
        let tupleType ← addPackedType loc sourceTypes
        addPattern loc tupleType (.tuple fields)
  let unit ← addDirectType .unit
  addExpr loc unit (.assignPattern pattern value)

private def constructorAssignment (context : FunctionContext) (loc : LeanerIR.LocId)
    (destinations : Array String) (name : Lean.Name) (variant : Option String)
    (typeArguments : Array Move.Compiler.LIR.Ty) (value : LeanerIR.ExprId) :
    BuildM LeanerIR.ExprId := do
  let fields ← destinations.mapM fun destination => do
    let declaration ← localDecl context.locals destination
    addPattern loc declaration.type.typeId (.variable declaration.id)
  let instantiations ← typeArguments.mapM fun ty =>
    return LeanerIR.GenericArgument.typeArg (← typeUse loc ty)
  let valueType := (← get).expressions[value.index]!.typeId
  let pattern ← addPattern loc valueType <|
    .constructor (← addName (← qualifiedStruct name)) instantiations variant fields
  let unit ← addDirectType .unit
  addExpr loc unit (.assignPattern pattern value)

private def resultType (context : FunctionContext) (destinations : Array String)
    (loc : LeanerIR.LocId) : BuildM LeanerIR.TypeId := do
  let sourceTypes ← destinations.mapM (sourceLocalTypeM context)
  addPackedType loc sourceTypes

/-- Bind the result of an NSIR instruction. Single-definition reference
temporaries are expression aliases: materializing them as ordinary Lean `let`s
loses the reference path tracked by the Leaner elaborator. Other destinations
remain explicit LIR assignments. -/
private def bindResult (context : FunctionContext) (loc : LeanerIR.LocId)
    (destinations : Array String) (value : LeanerIR.ExprId) :
    BuildM (Array LeanerIR.ExprId) := do
  match destinations with
  | #[] => return #[value]
  | #[destination] =>
      if context.aliasableReferences.contains destination then
        modify fun state => { state with
          referenceAliases :=
            (state.referenceAliases.filter fun entry => entry.1 != destination).push
              (destination, value) }
        return #[]
      return #[← assignment context loc destinations value]
  | destinations =>
      if destinations.any context.aliasableReferences.contains then
        throw "a reference alias cannot be bound by a multi-result NSIR instruction"
      return #[← assignment context loc destinations value]

private def ordinaryOperation (op : Move.Compiler.LIR.Oper) : Option Xast.Operation :=
  match op with
  | .add _ => some .add | .sub _ => some .sub | .mul _ => some .mul
  | .div _ => some .div | .mod _ => some .mod
  | .bitAnd _ => some .bitAnd | .bitOr _ => some .bitOr | .bitXor _ => some .xor
  | .shl _ => some .shl | .shr _ => some .shr | .cast _ => some .cast
  | .lt => some .lt | .le => some .le | .eq => some .eq
  | .vecPack => some .vector | .vecLen => some .len | .vecGet => some .index
  | .vecSet => some .updateVec | .vecContains => some .containsVec
  | .vecIndexOf => some .indexOfVec
  | _ => none

private def operationTypeArgs : Move.Compiler.LIR.Oper → Array Move.Compiler.LIR.Ty
  | .pack _ arguments | .unpack _ arguments | .packVariant _ _ arguments
  | .unpackVariant _ _ arguments | .testVariant _ _ arguments | .getField _ _ arguments
  | .borrowGlobal _ arguments | .borrowField _ arguments | .existsAt _ arguments
  | .borrowVariantField _ _ _ arguments | .testVariantRef _ _ arguments
  | .moveFrom _ arguments | .moveTo _ arguments | .function _ arguments => arguments
  | _ => #[]

private def translateCall (context : FunctionContext) (loc : LeanerIR.LocId)
    (destinations : Array String) (op : Move.Compiler.LIR.Oper) (sources : Array String) :
    BuildM (Array LeanerIR.ExprId) := do
  let argumentExprs ← sources.mapM (localExpr context.locals loc)
  let typeId ← resultType context destinations loc
  let assign := bindResult context loc destinations
  if let some xastOp := ordinaryOperation op then
    return ← assign (← operation loc typeId xastOp (operationTypeArgs op) argumentExprs)
  match op with
  | .borrowLoc =>
      let some destination := destinations[0]? | throw "borrowLoc has no destination"
      let some mutable := referenceMutable (← sourceLocalType context destination)
        | throw "borrowLoc destination is not a reference"
      assign (← operation loc typeId (.borrow (if mutable then .mutable else .immutable)) #[] argumentExprs)
  | .borrowGlobal resource typeArguments =>
      let some destination := destinations[0]? | throw "borrowGlobal has no destination"
      let some mutable := referenceMutable (← sourceLocalType context destination)
        | throw "borrowGlobal destination is not a reference"
      let ownerType := if typeArguments.isEmpty then .struct resource else .structInst resource typeArguments
      assign (← operation loc typeId (.borrowGlobal (if mutable then .mutable else .immutable))
        #[ownerType] argumentExprs)
  | .borrowField index _ =>
      let some source := sources[0]? | throw "borrowField has no owner"
      let owner ← sourceLocalType context source
      let (name, field, fieldType) ← fieldInfo owner index
      -- The core value `select` consumes a value operand; NSIR's borrow-field
      -- owner is a reference, so dereference it explicitly. Value-borrow
      -- normalization recovers the same borrowed place from either spelling.
      let ownerValue := stripReference owner
      let operands ← if ownerValue == owner then pure argumentExprs else do
        let ownerTypeId ← addType loc ownerValue
        pure #[← operation loc ownerTypeId .deref #[] argumentExprs]
      let selectedType ← addType loc fieldType
      let selected ← operation loc selectedType (.select name field)
        #[ownerValue] operands
      let some destination := destinations[0]? | throw "borrowField has no destination"
      let some mutable := referenceMutable (← sourceLocalType context destination)
        | throw "borrowField destination is not a reference"
      assign (← operation loc typeId (.borrow (if mutable then .mutable else .immutable)) #[] #[selected])
  | .borrowVecElem =>
      let some destination := destinations[0]? | throw "borrowVecElem has no destination"
      let some mutable := referenceMutable (← sourceLocalType context destination)
        | throw "borrowVecElem destination is not a reference"
      let elementType := stripReference (← sourceLocalType context destination)
      let selectedType ← addType loc elementType
      let selected ← operation loc selectedType .index #[] argumentExprs
      assign (← operation loc typeId (.borrow (if mutable then .mutable else .immutable)) #[] #[selected])
  | .readRef => assign (← operation loc typeId .deref #[] argumentExprs)
  | .writeRef =>
      assign (← addExpr loc typeId <| .operation (.reference .mutate) #[] argumentExprs)
  | .freezeRef => assign (← operation loc typeId (.freeze true) #[] argumentExprs)
  | .existsAt resource typeArguments =>
      let ownerType := if typeArguments.isEmpty then .struct resource else .structInst resource typeArguments
      assign (← operation loc typeId (.exists none) #[ownerType] argumentExprs)
  | .moveFrom resource typeArguments =>
      let ownerType := if typeArguments.isEmpty then .struct resource else .structInst resource typeArguments
      assign (← operation loc typeId .moveFrom #[ownerType] argumentExprs)
  | .moveTo resource typeArguments =>
      let ownerType := if typeArguments.isEmpty then .struct resource else .structInst resource typeArguments
      assign (← operation loc typeId .moveTo #[ownerType] argumentExprs)
  | .pack name typeArguments =>
      assign (← operation loc typeId (.pack (← qualifiedStruct name) none) typeArguments argumentExprs)
  | .packVariant name variant typeArguments =>
      let (name, variant) ← variantInfo name variant
      assign (← operation loc typeId (.pack name (some variant)) typeArguments argumentExprs)
  | .unpack name typeArguments =>
      let some value := argumentExprs[0]? | throw "unpack has no aggregate operand"
      return #[← constructorAssignment context loc destinations name none typeArguments value]
  | .unpackVariant name variant typeArguments =>
      let (_, variantName) ← variantInfo name variant
      let some value := argumentExprs[0]? | throw "unpackVariant has no aggregate operand"
      return #[← constructorAssignment context loc destinations name (some variantName)
        typeArguments value]
  | .getField name field typeArguments =>
      let declaration ← ownerDecl (.struct name)
      let some fieldDecl := declaration.fields[field]?
        | throw s!"invalid field {field} of `{declaration.moveName}`"
      assign (← operation loc typeId (.select (← qualifiedStruct name) fieldDecl.moveName)
        (if typeArguments.isEmpty then #[.struct name] else #[.structInst name typeArguments]) argumentExprs)
  | .testVariant name variant typeArguments =>
      let (name, variant) ← variantInfo name variant
      assign (← operation loc typeId (.testVariants name [variant]) typeArguments argumentExprs)
  | .function name typeArguments =>
      assign (← operation loc typeId (.moveFunction (← qualifiedFunction name)) typeArguments argumentExprs)
  | .vecPush => do
      let some vector := argumentExprs[0]? | throw "vecPush has no vector operand"
      let vectorType := ← sourceLocalType context sources[0]!
      let element ← match vectorType with
        | .vector element => pure element
        | type => throw s!"vecPush owner has type `{repr type}`"
      let refType ← addType loc (.mutRef vectorType)
      let borrowed ← operation loc refType (.borrow .mutable) #[] #[vector]
      let call ← operation loc (← addPackedType loc #[]) (.moveFunction (vectorFunction "push_back"))
        #[element] #[borrowed, argumentExprs[1]!]
      let mut statements := #[call]
      if let some destination := destinations[0]? then
        statements := statements.push (← assignment context loc #[destination] vector)
      return statements
  | .vecPop =>
      throw "Leaner IR frontend does not yet normalize vecPop"
  | .vecInsert | .vecRemove | .vecSwap | .vecSwapRemove | .vecAppend
  | .vecReverse | .vecReverseSlice | .vecTrim | .vecTrimReverse | .vecRotate
  | .vecRotateSlice | .vecDestroyEmpty
  | .borrowVariantField .. | .testVariantRef .. =>
      throw s!"Leaner IR frontend does not yet normalize `{repr op}`"
  | _ => throw s!"unsupported Leaner named operation `{repr op}`"

private def translateInstruction (context : FunctionContext)
    (instruction : Move.Compiler.LIR.Instr) : BuildM (Array LeanerIR.ExprId) := do
  let loc ← addLocation instruction.span
  match instruction.kind with
  | .loadBool destination value =>
      let declaration ← localDecl context.locals destination
      let value ← addExpr loc declaration.type.typeId (.value (.bool value))
      return #[← assignment context loc #[destination] value]
  | .loadInt _ destination value =>
      let declaration ← localDecl context.locals destination
      let value ← addExpr loc declaration.type.typeId (.value (.integer value))
      return #[← assignment context loc #[destination] value]
  | .loadAddress destination value =>
      let declaration ← localDecl context.locals destination
      let constant := LeanerIR.ConstValue.address (Move.encodeAddress value)
      let value ← addExpr loc declaration.type.typeId (.value constant)
      return #[← assignment context loc #[destination] value]
  | .assign destination source =>
      bindResult context loc #[destination] (← localExpr context.locals loc source)
  | .call destinations op sources => translateCall context loc destinations op sources

private def vectorMutationName : Move.Compiler.LIR.Oper → Option String
  | .vecPop => some "pop_back"
  | .vecInsert => some "insert"
  | .vecRemove => some "remove"
  | .vecSwap => some "swap"
  | .vecSwapRemove => some "swap_remove"
  | .vecAppend => some "append"
  | .vecReverse => some "reverse"
  | .vecReverseSlice => some "reverse_slice"
  | .vecTrim => some "trim"
  | .vecTrimReverse => some "trim_reverse"
  | .vecRotate => some "rotate"
  | .vecRotateSlice => some "rotate_slice"
  | _ => none

/-- Recognize the named compiler IR's canonical
`read_ref; vector-value-op; write_ref` expansion and recover the source-level
mutable vector operation. This is administrative normalization, not a second
semantic analysis: all names and types still come directly from NSIR. -/
private def translateVectorMutation (context : FunctionContext)
    (readInstruction operationInstruction writeInstruction : Move.Compiler.LIR.Instr) :
    BuildM (Option (Array LeanerIR.ExprId)) := do
  let (.call readDestinations .readRef readSources) := readInstruction.kind
    | return none
  let (.call operationDestinations operationKind operationSources) := operationInstruction.kind
    | return none
  let (.call writeDestinations .writeRef writeSources) := writeInstruction.kind
    | return none
  let some operationName := vectorMutationName operationKind | return none
  let some oldVector := readDestinations[0]? | return none
  let some reference := readSources[0]? | return none
  let some updatedVector := operationDestinations[0]? | return none
  unless readDestinations.size == 1 && operationSources[0]? == some oldVector &&
      writeDestinations.isEmpty && writeSources == #[reference, updatedVector] do
    return none
  let elementType ← match ← sourceLocalTypeM context reference with
    | .mutRef (.vector element) => pure element
    | type => throw s!"vector mutation reference has type `{repr type}`"
  let destinations := operationDestinations.extract 1 operationDestinations.size
  let sources := #[reference] ++ operationSources.extract 1 operationSources.size
  let loc ← addLocation operationInstruction.span
  let argumentExprs ← sources.mapM (localExpr context.locals loc)
  let resultType ← resultType context destinations loc
  let call ← operation loc resultType (.moveFunction (vectorFunction operationName))
    #[elementType] argumentExprs
  if destinations.isEmpty then return some #[call]
  return some #[← assignment context loc destinations call]

private def translateInstructions (context : FunctionContext)
    (instructions : Array Move.Compiler.LIR.Instr) : BuildM (Array LeanerIR.ExprId) :=
  go 0 #[] (instructions.size + 1)
where
  go (cursor : Nat) (statements : Array LeanerIR.ExprId) : Nat → BuildM (Array LeanerIR.ExprId)
    | 0 => throw "instruction normalization exhausted its traversal bound"
    | fuel + 1 => do
        let some instruction := instructions[cursor]? | return statements
        match instructions[cursor + 1]?, instructions[cursor + 2]? with
        | some next, some afterNext =>
            match ← translateVectorMutation context instruction next afterNext with
            | some translated => go (cursor + 3) (statements ++ translated) fuel
            | none =>
                go (cursor + 1) (statements ++ (← translateInstruction context instruction)) fuel
        | _, _ =>
            go (cursor + 1) (statements ++ (← translateInstruction context instruction)) fuel

private def uniqueNames (parameters locals : Array Move.Compiler.LIR.LocalDecl) :
    Array (String × String × Move.Compiler.LIR.Ty) := Id.run do
  let mut used : Array String := #[]
  let mut result := #[]
  for declaration in parameters ++ locals do
    let base := declaration.sourceName.getD declaration.name
    let rec fresh (candidate : String) (suffix fuel : Nat) : String :=
      match fuel with
      | 0 => candidate
      | fuel + 1 =>
          if used.contains candidate then fresh s!"{base}_{suffix}" (suffix + 1) fuel else candidate
    let name := fresh base 1 (parameters.size + locals.size + 1)
    used := used.push name
    result := result.push (declaration.name, name, declaration.ty)
  return result

private def instructionDestinations (instruction : Move.Compiler.LIR.Instr) : Array String :=
  match instruction.kind with
  | .loadBool destination _ | .loadInt _ destination _ | .loadAddress destination _
  | .assign destination _ => #[destination]
  | .call destinations _ _ => destinations

/-- Reference locals introduced by normalization are safe to inline when they
have one definition and that definition is not part of tuple destructuring.
This recovers the authored borrow path while leaving parameters and merged or
reassigned references represented as real locals. -/
private def aliasableReferenceNames (declaration : Move.Compiler.LIR.FunDecl) : Array String :=
  Id.run do
    let mut result := #[]
    for localDeclaration in declaration.locals do
      if (referenceMutable localDeclaration.ty).isSome then
        let mut definitions := 0
        let mut singleton := true
        for block in declaration.blocks do
          for instruction in block.instrs do
            let destinations := instructionDestinations instruction
            if destinations.contains localDeclaration.name then
              definitions := definitions + 1
              if destinations != #[localDeclaration.name] then singleton := false
        if definitions == 1 && singleton then
          result := result.push localDeclaration.name
    return result

private def lirAbilities (abilities : Move.Compiler.LIR.AbilitySet) : Array LeanerIR.Ability :=
  #[
    (.copy, abilities.copy), (.drop, abilities.drop),
    (.store, abilities.store), (.key, abilities.key)
  ].filterMap fun (ability, present) => if present then some ability else none

private def function (declaration : Move.Compiler.LIR.FunDecl) : BuildM
    (LeanerIR.FunctionDecl LeanerIR.Import.RawBody) := do
  let loc : LeanerIR.LocId := ⟨0⟩
  let namedLocals := uniqueNames declaration.params declaration.locals
  let aliasableReferences := aliasableReferenceNames declaration
  modify fun state => { state with referenceAliases := #[] }
  let mut localDeclarations := #[]
  let mut localMap := #[]
  for (compilerName, sourceName, sourceType) in namedLocals do
    unless aliasableReferences.contains compilerName do
      let id : LeanerIR.LocalId := ⟨localDeclarations.size⟩
      let type ← typeUse loc sourceType
      localDeclarations := localDeclarations.push { id, name := sourceName, type, loc }
      localMap := localMap.push (compilerName, id)
  let localContext := { byCompilerName := localMap, declarations := localDeclarations }
  let context := {
    locals := localContext
    sourceTypes := namedLocals.map fun (compilerName, _, type) => (compilerName, type)
    aliasableReferences }
  let parameters ← declaration.params.mapM fun parameter => do
    let localDeclaration ← localDecl localContext parameter.name
    return {
      name := localDeclaration.name
      typeUse := localDeclaration.type }
  let resultType ← addPackedType loc declaration.returns
  let mut blocks := #[]
  for block in declaration.blocks do
    let blockLoc ← addLocation block.termSpan
    let statements ← translateInstructions context block.instrs
    let terminator ← match block.term with
      | .jump target => pure <| LeanerIR.Import.RawTerminator.goto ⟨declaration.blocks.findIdx (·.name == target)⟩
      | .branch condition thenTarget elseTarget =>
          pure <| .branch (← localExpr localContext blockLoc condition)
            ⟨declaration.blocks.findIdx (·.name == thenTarget)⟩
            ⟨declaration.blocks.findIdx (·.name == elseTarget)⟩
      | .ret sources => do
          let values ← sources.mapM (localExpr localContext blockLoc)
          let result ← match values with
            | #[] => addExpr blockLoc resultType (.value (.tuple #[]))
            | #[value] => pure value
            | values => operation blockLoc resultType .tuple #[] values
          pure <| .return_ #[result]
      | .abort code => pure <| .throw_ .abort #[← localExpr localContext blockLoc code]
    blocks := blocks.push {
      loc := blockLoc, statements := statements.map .execute, terminator }
  let visibilityTag := match declaration.visibility with
    | .private_ => "visibility.private" | .public_ | .entry => "visibility.public"
    | .friend_ => "visibility.friend"
  let mut profileData := #[LeanerIR.Move.propertyValue visibilityTag,
    LeanerIR.Move.propertyValue (if declaration.native then "function.native" else "function.regular")]
  if declaration.visibility == .entry then
    profileData := profileData.push (LeanerIR.Move.propertyValue "function.entry")
  return {
    loc
    name := ← addName { module := ownModule (← get).source, name := declaration.moveName }
    profile := .move
    signature := {
      generics := declaration.typeParams.map fun parameter => {
        name := parameter.name
        kind := .typeArg
        abilities := lirAbilities parameter.abilities
        loc }
      parameters
      results := #[{ typeId := resultType, loc }] }
    body := if declaration.native then .absent else .cfg { entry := ⟨0⟩, blocks }
    origin := ⟨0⟩
    alignment := ⟨0⟩
    locals := localDeclarations
    profileData }

private def structDecl (declaration : Move.Compiler.LIR.StructDecl) : BuildM LeanerIR.StructDecl := do
  let loc : LeanerIR.LocId := ⟨0⟩
  let fields ← declaration.fields.mapM fun field =>
    return {
      loc
      name := ← addName { module := ownModule (← get).source, name := field.moveName }
      type := ← typeUse loc field.ty }
  let variants ← declaration.variants.mapM fun variants => variants.mapM fun variant => do
    let fields ← variant.fields.mapM fun field =>
      return {
        loc
        name := ← addName { module := ownModule (← get).source, name := field.moveName }
        type := ← typeUse loc field.ty }
    return {
      loc
      name := ← addName { module := ownModule (← get).source, name := variant.moveName }
      fields }
  return {
    loc
    name := ← addName { module := ownModule (← get).source, name := declaration.moveName }
    generics := declaration.typeParams.map fun parameter => {
      name := parameter.name
      kind := .typeArg
      abilities := lirAbilities parameter.abilities
      predicates := if parameter.phantom then
        #[.profile (LeanerIR.Move.propertyValue "typeParameter.phantom")] else #[]
      loc }
    fields
    variants := variants.getD #[]
    abilities := lirAbilities declaration.abilities
    properties := if declaration.variants.isSome then
      #[LeanerIR.Move.propertyValue "struct.variants"] else #[] }

/-! ## Retained specification bridge -/

private structure SpecValue where
  expression : LeanerIR.ExprId
  typeId : LeanerIR.TypeId
  sourceType : Option Move.Compiler.LIR.Ty := none

private structure SpecContext where
  locals : Array (String × LeanerIR.LocalDecl × Option Move.Compiler.LIR.Ty)
  resultType : LeanerIR.TypeId
  resultSourceType : Option Move.Compiler.LIR.Ty
  sourceArtifact : Move.SourceModuleArtifact

private def instantiateFieldType (owner : Move.Compiler.LIR.Ty) :
    Move.Compiler.LIR.Ty → Move.Compiler.LIR.Ty
  | .typeParam index => match stripReference owner with
      | .structInst _ arguments | .enumInst _ arguments =>
          arguments[index]?.getD (.typeParam index)
      | _ => .typeParam index
  | .vector element => .vector (instantiateFieldType owner element)
  | .ref element => .ref (instantiateFieldType owner element)
  | .mutRef element => .mutRef (instantiateFieldType owner element)
  | .structInst name arguments => .structInst name (arguments.map (instantiateFieldType owner))
  | .enumInst name arguments => .enumInst name (arguments.map (instantiateFieldType owner))
  | type => type

private def sourceStructBySpelling (name : Lean.Name) : BuildM Move.Compiler.LIR.StructDecl := do
  let spelling := name.getString!
  let some declaration := (← get).source.structs.find? fun declaration =>
      declaration.moveName == spelling || declaration.leanName == name ||
        declaration.leanName.getString! == spelling
    | throw s!"unknown resource or aggregate type `{name}` in retained specification"
  return declaration

private def syntaxNameParts (name : Lean.Name) : List String :=
  name.components.map (·.getString!)

private def specLocal (context : SpecContext) (name : String) :
    Except String (LeanerIR.LocalDecl × Option Move.Compiler.LIR.Ty) :=
  match context.locals.find? (·.1 == name) with
  | some (_, declaration, sourceType) => .ok (declaration, sourceType)
  | none => .error s!"unknown retained specification local `{name}`"

private def specOperation (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId)
    (operationName : Xast.Operation) (instantiations : Array Move.Compiler.LIR.Ty)
    (arguments : Array SpecValue) : BuildM SpecValue := do
  let expression ← operation loc typeId operationName instantiations (arguments.map (·.expression))
  return { expression, typeId }

private def sourceConstant? (artifact : Move.SourceModuleArtifact) (name : String) :
    Option Lean.Syntax := do
  let declaration ← artifact.items.find? fun item =>
    item.isOfKind ``Lean.Parser.Command.declaration && item.getNumArgs > 1 &&
      item[1].isOfKind ``Lean.Parser.Command.definition && item[1].getNumArgs > 3 &&
      item[1][1].getNumArgs > 0 && item[1][1][0].isIdent &&
      item[1][1][0].getId.getString! == name
  let value := declaration[1][3]
  guard <| value.isOfKind ``Lean.Parser.Command.declValSimple && value.getNumArgs > 1
  return value[1]

private structure ParsedSpecType where
  typeId : LeanerIR.TypeId
  /-- The compiler type, when this is also a Move value type. `Int` is a
  specification-only type and deliberately has no NSIR spelling. -/
  sourceType : Option Move.Compiler.LIR.Ty := none

private partial def specType (generics : Array String) (stx : Lean.Syntax)
    (fuel : Nat := 48) : BuildM ParsedSpecType := do
  if fuel == 0 then throw "retained specification type exhausted its traversal bound"
  let loc ← addSyntaxLocation stx
  if stx.isOfKind `choice && stx.getNumArgs > 0 then
    return ← specType generics stx[0] (fuel - 1)
  if stx.isOfKind ``Lean.Parser.Term.paren && stx.getNumArgs > 1 then
    return ← specType generics stx[1] (fuel - 1)
  if stx.isOfKind ``Lean.Parser.Term.prop then
    return { typeId := ← boolType, sourceType := some .bool }
  if (stx.isOfKind ``Move.borrowTerm || stx.isOfKind ``Move.borrowMutTerm) &&
      stx.getNumArgs > 1 then
    let referent ← specType generics stx[1] (fuel - 1)
    let some sourceReferent := referent.sourceType
      | throw "reference to a specification-only type"
    let sourceType := if stx.isOfKind ``Move.borrowMutTerm then
        Move.Compiler.LIR.Ty.mutRef sourceReferent
      else Move.Compiler.LIR.Ty.ref sourceReferent
    return { typeId := ← addType loc sourceType, sourceType := some sourceType }
  if stx.isIdent then
    let spelling := stx.getId.getString!
    if let some index := generics.findIdx? (· == spelling) then
      let sourceType := Move.Compiler.LIR.Ty.typeParam index
      return { typeId := ← addType loc sourceType, sourceType := some sourceType }
    let sourceType? : Option Move.Compiler.LIR.Ty := match spelling with
      | "Bool" | "Prop" => some .bool
      | "U8" => some (.int { width := .w8, signed := false })
      | "U16" => some (.int { width := .w16, signed := false })
      | "U32" => some (.int { width := .w32, signed := false })
      | "U64" => some (.int { width := .w64, signed := false })
      | "U128" => some (.int { width := .w128, signed := false })
      | "U256" => some (.int { width := .w256, signed := false })
      | "I8" => some (.int { width := .w8, signed := true })
      | "I16" => some (.int { width := .w16, signed := true })
      | "I32" => some (.int { width := .w32, signed := true })
      | "I64" => some (.int { width := .w64, signed := true })
      | "I128" => some (.int { width := .w128, signed := true })
      | "I256" => some (.int { width := .w256, signed := true })
      | "Address" => some .address
      | "Signer" => some .signer
      | _ => none
    if spelling == "Int" then return { typeId := ← numberType }
    if let some sourceType := sourceType? then
      return { typeId := ← addType loc sourceType, sourceType := some sourceType }
    let declaration ← sourceStructBySpelling stx.getId
    let sourceType := if declaration.variants.isSome then
        Move.Compiler.LIR.Ty.enum declaration.leanName
      else Move.Compiler.LIR.Ty.struct declaration.leanName
    return { typeId := ← addType loc sourceType, sourceType := some sourceType }
  if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs > 1 && stx[0].isIdent then
    let arguments := stx[1].getArgs
    if stx[0].getId.getString! == "Vector" && arguments.size == 1 then
      let element ← specType generics arguments[0]! (fuel - 1)
      let typeId ← addDirectType <| .vector element.typeId
      return {
        typeId
        sourceType := element.sourceType.map .vector }
    let declaration ← sourceStructBySpelling stx[0].getId
    let parsedArguments ← arguments.mapM (specType generics · (fuel - 1))
    let some sourceArguments := parsedArguments.mapM (·.sourceType)
      | throw s!"aggregate `{declaration.moveName}` has a specification-only type argument"
    let sourceType := if declaration.variants.isSome then
        Move.Compiler.LIR.Ty.enumInst declaration.leanName sourceArguments
      else Move.Compiler.LIR.Ty.structInst declaration.leanName sourceArguments
    return { typeId := ← addType loc sourceType, sourceType := some sourceType }
  throw s!"unsupported retained specification type `{stx.getKind}`"

private def sourceConstantParts (item : Lean.Syntax) :
    Except String (String × Lean.Syntax × Lean.Syntax) := do
  unless item.isOfKind ``Lean.Parser.Command.declaration && item.getNumArgs > 1 &&
      item[1].isOfKind ``Lean.Parser.Command.definition && item[1].getNumArgs > 3 do
    throw "retained declaration is not a constant definition"
  let definition := item[1]
  unless definition[1].getNumArgs > 0 && definition[1][0].isIdent do
    throw "retained constant has no identifier"
  let name := definition[1][0].getId.getString!
  unless definition[2].getNumArgs > 1 && definition[2][1].getNumArgs > 0 &&
      definition[2][1][0].getNumArgs > 1 do
    throw s!"constant `{name}` has no explicit type"
  unless definition[3].isOfKind ``Lean.Parser.Command.declValSimple &&
      definition[3].getNumArgs > 1 do
    throw s!"constant `{name}` has no simple value"
  return (name, definition[2][1][0][1], definition[3][1])

private def hexDigit? (character : Char) : Option Nat :=
  if character >= '0' && character <= '9' then some (character.toNat - '0'.toNat)
  else if character >= 'a' && character <= 'f' then some (character.toNat - 'a'.toNat + 10)
  else if character >= 'A' && character <= 'F' then some (character.toNat - 'A'.toNat + 10)
  else none

private def hexBytes (value : String) : Except String (Array LeanerIR.ConstValue) := do
  let characters := value.toList
  unless characters.length % 2 == 0 do throw "hex byte string has odd length"
  let rec go : List Char → Except String (Array LeanerIR.ConstValue)
    | [] => pure #[]
    | high :: low :: rest => do
        let some high := hexDigit? high | throw "hex byte string contains a non-hexadecimal digit"
        let some low := hexDigit? low | throw "hex byte string contains a non-hexadecimal digit"
        return #[.integer (Int.ofNat (high * 16 + low))] ++ (← go rest)
    | _ => throw "hex byte string has odd length"
  go characters

private partial def sourceConstantValue (artifact : Move.SourceModuleArtifact)
    (stx : Lean.Syntax) (fuel : Nat := 32) : Except String LeanerIR.ConstValue := do
  if fuel == 0 then throw "retained constant exhausted its traversal bound"
  if stx.isOfKind `choice && stx.getNumArgs > 0 then
    return ← sourceConstantValue artifact stx[0] (fuel - 1)
  if stx.isOfKind ``Lean.Parser.Term.paren && stx.getNumArgs > 1 then
    return ← sourceConstantValue artifact stx[1] (fuel - 1)
  if let some value := stx.isNatLit? then return .integer (Int.ofNat value)
  if stx.isIdent then
    let spelling := stx.getId.getString!
    if spelling == "true" || spelling == "True" then return .bool true
    if spelling == "false" || spelling == "False" then return .bool false
    if let some value := sourceConstant? artifact spelling then
      return ← sourceConstantValue artifact value (fuel - 1)
    throw s!"unknown identifier `{spelling}` in retained constant"
  if stx.isOfKind ``Move.addressLiteral && stx.getNumArgs > 1 then
    let some value := stx[1].isNatLit? | throw "address constant is not numeric"
    return .address (Move.encodeAddress value)
  if stx.isOfKind ``Move.byteStringLiteral && stx.getNumArgs > 1 && stx[0].isIdent then
    let leader := stx[0].getId.getString!
    let value := (⟨stx[1]⟩ : Lean.TSyntax `str).getString
    if leader == "b" then
      return .vector <| value.toUTF8.data.map fun byte => .integer (Int.ofNat byte.toNat)
    if leader == "x" then return .vector (← hexBytes value)
    throw s!"unknown byte-string leader `{leader}`"
  throw s!"unsupported retained constant value `{stx.getKind}`"

private def sourceConstantDecl (artifact : Move.SourceModuleArtifact)
    (item : Lean.Syntax) : BuildM LeanerIR.ConstantDecl := do
  let (name, typeSyntax, valueSyntax) ← sourceConstantParts item
  let loc ← addSyntaxLocation item
  let parsedType ← specType #[] typeSyntax
  let some _ := parsedType.sourceType
    | throw s!"constant `{name}` has a specification-only type"
  let value ← sourceConstantValue artifact valueSyntax
  return {
    loc
    name := ← addName { module := ownModule (← get).source, name }
    type := { typeId := parsedType.typeId, loc := ← addSyntaxLocation typeSyntax }
    value := ← addExpr (← addSyntaxLocation valueSyntax) parsedType.typeId (.value value) }

private def sourceConstants (artifact : Move.SourceModuleArtifact) :
    BuildM (Array LeanerIR.ConstantDecl) := do
  let mut result := #[]
  for item in artifact.items do
    if item.isOfKind ``Lean.Parser.Command.declaration && item.getNumArgs > 1 &&
        item[1].isOfKind ``Lean.Parser.Command.definition then
      let name := (sourceConstantParts item).toOption.map (·.1) |>.getD "<unknown>"
      try
        result := result.push (← sourceConstantDecl artifact item)
        modify fun state => { state with importedConstants := state.importedConstants + 1 }
      catch message =>
        let issue := s!"constant `{name}`: {message}"
        modify fun state => { state with
          unsupportedSource := state.unsupportedSource.push issue }
  return result

private def sourceSpecFunction? (artifact : Move.SourceModuleArtifact) (name : String) :
    Option Lean.Syntax :=
  artifact.items.find? fun item =>
    (item.isOfKind ``Move.Spec.specFunctionDecl ||
      item.isOfKind ``Move.Spec.opaqueSpecFunctionDecl) &&
      item.getNumArgs > 3 && item[3].isIdent && item[3].getId.getString! == name

private def sourceSpecGenericNames (item : Lean.Syntax) : Except String (Array String) := do
  let mut result := #[]
  for binder in item[4].getArgs do
    let some opening := binder.getArgs[0]? | throw "empty retained specification binder"
    if opening.isAtom && opening.getAtomVal == "{" then
      let some name := binder.getArgs[1]? | throw "unnamed retained specification type binder"
      unless name.isIdent do throw "retained specification type binder is not an identifier"
      result := result.push name.getId.getString!
    else if opening.isAtom && opening.getAtomVal == "[" then
      throw "instance binders are not representable in the Move printer profile"
  return result

private def sourceSpecResultType (artifact : Move.SourceModuleArtifact) (name : String) :
    BuildM ParsedSpecType := do
  if let some item := sourceSpecFunction? artifact name then
    let generics ← sourceSpecGenericNames item
    let resultSyntax ← if item.isOfKind ``Move.Spec.opaqueSpecFunctionDecl then
        pure item[6]
      else if item[5].getNumArgs > 1 then pure item[5][1]
      else throw s!"specification function `{name}` has an inferred result type"
    return ← specType generics resultSyntax
  if let some function := (← get).source.functions.find? (·.moveName == name) then
    let some result := function.returns[0]?
      | throw s!"Move function `{name}` has no specification result"
    return { typeId := ← addType ⟨0⟩ result, sourceType := some result }
  throw s!"unknown specification function `{name}`"

private partial def selectSpecFields (context : SpecContext) (loc : LeanerIR.LocId)
    (base : SpecValue) (fields : List String) : BuildM SpecValue := do
  match fields with
  | [] => pure base
  | field :: rest =>
      let owner ← match base.sourceType with
        | some owner => pure owner
        | none => throw s!"cannot select field `{field}` from an untyped retained specification expression"
      let selected ← match stripReference owner with
        | .vector _ =>
            unless field == "length" do
              throw s!"unknown vector specification field `{field}`"
            let typeId ← addType loc (.int { width := .w64, signed := false })
            let value ← specOperation loc typeId .len #[] #[base]
            pure { value with sourceType := some (.int { width := .w64, signed := false }) }
        | .signer =>
            unless field == "address" do
              throw s!"unknown signer specification field `{field}`"
            let typeId ← addType loc .address
            let target : QualifiedName := {
              module := ownModule (← get).source
              name := "Signer" }
            let value ← specOperation loc typeId (.select target field) #[] #[base]
            pure { value with sourceType := some .address }
        | .struct _ | .structInst _ _ | .enum _ | .enumInst _ _ => do
            let declaration ← ownerDecl owner
            let some fieldDeclaration := declaration.fields.find? (·.moveName == field)
              | throw s!"unknown field `{field}` of `{declaration.moveName}` in retained specification"
            let fieldType := instantiateFieldType owner fieldDeclaration.ty
            let typeId ← addType loc fieldType
            let value ← specOperation loc typeId
              (.select (← qualifiedStruct declaration.leanName) field)
              #[stripReference owner] #[base]
            pure { value with sourceType := some fieldType }
        | type => throw s!"cannot select field `{field}` from `{repr type}`"
      selectSpecFields context loc selected rest

private def sourceTypeOrNumber (value : SpecValue) : BuildM LeanerIR.TypeId :=
  match value.sourceType with
  | some type => addType ⟨0⟩ type
  | none => pure value.typeId

private partial def specExpression (context : SpecContext) (stx : Lean.Syntax)
    (expected : Option Move.Compiler.LIR.Ty := none) (fuel : Nat := 96) : BuildM SpecValue := do
  if fuel == 0 then throw "retained specification expression exhausted its traversal bound"
  let loc ← addSyntaxLocation stx
  if stx.isOfKind `choice && stx.getNumArgs > 0 then
    return ← specExpression context stx[0] expected (fuel - 1)
  if stx.isOfKind ``Lean.Parser.Term.paren && stx.getNumArgs > 1 then
    return ← specExpression context stx[1] expected (fuel - 1)
  if let some value := stx.isNatLit? then
    let typeId ← match expected with
      | some type => addType loc type
      | none => numberType
    return {
      expression := ← addExpr loc typeId (.value (.integer value))
      typeId
      sourceType := expected }
  if stx.isIdent then
    let parts := syntaxNameParts stx.getId
    let some root := parts.head? | throw "empty identifier in retained specification"
    if root == "True" || root == "False" then
      let typeId ← boolType
      return {
        expression := ← addExpr loc typeId (.value (.bool (root == "True")))
        typeId }
    if root == "result" then
      let value ← specOperation loc context.resultType (.result 0) #[] #[]
      return ← selectSpecFields context loc
        { value with sourceType := context.resultSourceType } parts.tail
    if let .ok (declaration, sourceType) := specLocal context root then
      let value : SpecValue := {
        expression := ← addExpr loc declaration.type.typeId (.localVar declaration.id)
        typeId := declaration.type.typeId
        sourceType }
      return ← selectSpecFields context loc value parts.tail
    if let some constant := sourceConstant? context.sourceArtifact root then
      return ← specExpression context constant expected (fuel - 1)
    throw s!"unsupported identifier `{stx.getId}` in retained specification"
  if stx.isOfKind ``Move.Spec.resourceExistsTerm && stx.getNumArgs > 3 && stx[1].isIdent then
    let declaration ← sourceStructBySpelling stx[1].getId
    let address ← specExpression context stx[3] none (fuel - 1)
    let typeId ← boolType
    let value ← specOperation loc typeId (.exists none) #[.struct declaration.leanName] #[address]
    return { value with sourceType := some .bool }
  if stx.isOfKind ``Move.Spec.oldResourceTerm && stx.getNumArgs > 1 then
    let value ← specExpression context stx[1] expected (fuel - 1)
    let old ← specOperation loc value.typeId .old #[] #[value]
    return { old with sourceType := value.sourceType }
  if stx.getKind == `«term__[_]» && stx.getNumArgs == 4 && stx[0].isIdent then
    let declaration ← sourceStructBySpelling stx[0].getId
    let address ← specExpression context stx[2] none (fuel - 1)
    let resourceType := Move.Compiler.LIR.Ty.struct declaration.leanName
    let typeId ← addType loc resourceType
    let value ← specOperation loc typeId (.global none) #[resourceType] #[address]
    return { value with sourceType := some resourceType }
  if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs > 1 && stx[0].isIdent then
    let name := stx[0].getId.getString!
    let resultType ← sourceSpecResultType context.sourceArtifact name
    let arguments ← stx[1].getArgs.mapM fun argument =>
      specExpression context argument none (fuel - 1)
    let target : QualifiedName := { module := ownModule (← get).source, name }
    let value ← specOperation loc resultType.typeId
      (.specFunction target { pre := none, post := none }) #[] arguments
    return { value with sourceType := resultType.sourceType }
  if stx.isOfKind ``Lean.Parser.Term.proj && stx.getNumArgs > 2 && stx[2].isIdent then
    let base ← specExpression context stx[0] none (fuel - 1)
    return ← selectSpecFields context loc base (syntaxNameParts stx[2].getId)
  if stx.getNumArgs == 2 && stx[0].isAtom && stx[0].getAtomVal == "¬" then
    let argument ← specExpression context stx[1] (some .bool) (fuel - 1)
    let typeId ← boolType
    let value ← specOperation loc typeId .not #[] #[argument]
    return { value with sourceType := some .bool }
  if stx.getNumArgs == 3 && stx[1].isAtom then
    let symbol := stx[1].getAtomVal
    let operation? : Option Xast.Operation := match symbol with
      | "+" => some .add | "-" => some .sub | "*" => some .mul
      | "/" => some .div | "%" => some .mod
      | "=" | "==" => some .eq | "≠" | "!=" => some .neq
      | "<" => some .lt | ">" => some .gt | "≤" | "<=" => some .le
      | "≥" | ">=" => some .ge | "∧" | "&&" => some .and
      | "∨" | "||" => some .or | "→" => some .implies
      | _ => none
    if let some operationName := operation? then
      let left ← specExpression context stx[0] expected (fuel - 1)
      let right ← specExpression context stx[2] left.sourceType (fuel - 1)
      let logical := ["=", "==", "≠", "!=", "<", ">", "≤", "<=", "≥", ">=", "∧", "&&", "∨", "||", "→"].contains symbol
      let typeId ← if logical then boolType else sourceTypeOrNumber left
      let value ← specOperation loc typeId operationName #[] #[left, right]
      return { value with sourceType := if logical then some .bool else left.sourceType }
  throw s!"unsupported retained specification expression `{stx.getKind}`"

private def sourcePragmas (node : Lean.Syntax) : BuildM (Array LeanerIR.Attribute) := do
  let mut pragmas := #[]
  for pragma in node.getArgs do
    if pragma.getNumArgs > 1 && pragma[1].isIdent then
      let name := pragma[1].getId.getString!
      pragmas := pragmas.push <| .assign name (.constant (.bool true))
        (some (← addSyntaxLocation pragma))
  return pragmas

private def sourceCondition (context : SpecContext) (kind : String)
    (expression : Lean.Syntax) (code : Option Lean.Syntax := none) : BuildM LeanerIR.Condition := do
  let loc ← addSyntaxLocation expression
  let expression ← specExpression context expression (some .bool)
  let mut auxiliary := #[]
  if let some code := code then
    auxiliary := auxiliary.push ("abortCode", (← specExpression context code).expression)
  let kind ← match kind with
    | "ensures" => pure LeanerIR.ConditionKind.ensures
    | "requires" => pure .requires
    | "abortsIf" => pure .abortsIf
    | value => throw s!"unsupported retained condition kind `{value}`"
  return {
    loc
    kind
    expression := expression.expression
    auxiliary }

private def sourceFrame (context : SpecContext) (node : Lean.Syntax) :
    BuildM (Array LeanerIR.ExprId × Bool) := do
  let some clause := node.getArgs[0]? | return (#[], false)
  if clause.getNumArgs <= 1 then return (#[], false)
  let mut targets := #[]
  let mut hasWildcard := false
  for target in clause[1].getSepArgs do
    if target.isOfKind `Move.Spec.modifiesAny then
      hasWildcard := true
    else if target.isOfKind `Move.Spec.modifiesAddress && target.getNumArgs > 2 && target[0].isIdent then
      let declaration ← sourceStructBySpelling target[0].getId
      let address ← specExpression context target[2]
      let loc ← addSyntaxLocation target
      let resourceType := Move.Compiler.LIR.Ty.struct declaration.leanName
      let typeId ← addType loc resourceType
      let value ← specOperation loc typeId (.global none) #[resourceType] #[address]
      targets := targets.push value.expression
    else
      throw s!"unsupported retained modifies target `{target.getKind}`"
  return (targets, hasWildcard && targets.isEmpty)

private structure SourceSpecSignature where
  generics : Array LeanerIR.GenericBinder := #[]
  parameters : Array LeanerIR.Parameter := #[]
  locals : Array LeanerIR.LocalDecl := #[]
  contextLocals : Array (String × LeanerIR.LocalDecl × Option Move.Compiler.LIR.Ty) := #[]

private def sourceSpecSignature (item : Lean.Syntax) (loc : LeanerIR.LocId) :
    BuildM SourceSpecSignature := do
  let genericNames ← sourceSpecGenericNames item
  let mut generics := #[]
  for name in genericNames do
    generics := generics.push { name, kind := .typeArg, loc }
  let mut parameters := #[]
  let mut locals := #[]
  let mut contextLocals := #[]
  for binder in item[4].getArgs do
    let some opening := binder.getArgs[0]? | throw "empty retained specification binder"
    if opening.isAtom && opening.getAtomVal == "(" then
      unless binder.getNumArgs > 3 && binder[1].isIdent do
        throw "malformed retained specification value binder"
      let name := binder[1].getId.getString!
      let parsedType ← specType genericNames binder[3]
      let binderLoc ← addSyntaxLocation binder
      let localDeclaration : LeanerIR.LocalDecl := {
        id := ⟨locals.size⟩
        name
        type := { typeId := parsedType.typeId, loc := binderLoc }
        loc := binderLoc }
      locals := locals.push localDeclaration
      parameters := parameters.push {
        name
        typeUse := localDeclaration.type }
      contextLocals := contextLocals.push (name, localDeclaration, parsedType.sourceType)
    else if opening.isAtom && opening.getAtomVal == "[" then
      throw "instance binders are not representable in the Move printer profile"
  return { generics, parameters, locals, contextLocals }

private def sourceSpecFunction (artifact : Move.SourceModuleArtifact)
    (item : Lean.Syntax) (forceOpaque : Bool := false) :
    BuildM LeanerIR.SpecFunctionDecl := do
  unless item.getNumArgs > 6 && item[3].isIdent &&
      (item.isOfKind ``Move.Spec.opaqueSpecFunctionDecl || item.getNumArgs > 7) do
    throw "malformed retained specification function"
  let loc ← addSyntaxLocation item
  let name := item[3].getId.getString!
  let parsedSignature ← sourceSpecSignature item loc
  let resultSyntax ← if item.isOfKind ``Move.Spec.opaqueSpecFunctionDecl then
      pure item[6]
    else if item[5].getNumArgs > 1 then pure item[5][1]
    else throw s!"specification function `{name}` has an inferred result type"
  let genericNames ← sourceSpecGenericNames item
  let resultType ← specType genericNames resultSyntax
  let isOpaque := forceOpaque || item.isOfKind ``Move.Spec.opaqueSpecFunctionDecl
  let body : Option LeanerIR.ExprId ← if isOpaque then pure none else do
    let context : SpecContext := {
      locals := parsedSignature.contextLocals
      resultType := resultType.typeId
      resultSourceType := resultType.sourceType
      sourceArtifact := artifact }
    let value ← specExpression context item[7] resultType.sourceType
    pure (some value.expression)
  let mut profileData := #[]
  if isOpaque then
    profileData := profileData.push <| LeanerIR.Move.propertyValue "specFunction.uninterpreted"
  if (← get).source.functions.any (·.moveName == name) then
    profileData := profileData.push <| LeanerIR.Move.propertyValue "specFunction.moveFunction"
  return {
    loc
    name := ← addName { module := ownModule (← get).source, name }
    profile := .move
    signature := {
      generics := parsedSignature.generics
      parameters := parsedSignature.parameters
      results := #[{ typeId := resultType.typeId, loc := ← addSyntaxLocation resultSyntax }] }
    body
    origin := ⟨0⟩
    locals := parsedSignature.locals
    profileData }

private def sourceSpecFunctions (artifact : Move.SourceModuleArtifact) :
    BuildM (Array LeanerIR.SpecFunctionDecl) := do
  let mut result := #[]
  for item in artifact.items do
    if item.isOfKind ``Move.Spec.specFunctionDecl ||
        item.isOfKind ``Move.Spec.opaqueSpecFunctionDecl then
      let name := if item.getNumArgs > 3 && item[3].isIdent then
          item[3].getId.getString!
        else "<unknown>"
      try
        result := result.push (← sourceSpecFunction artifact item)
        modify fun state => { state with
          importedSpecFunctions := state.importedSpecFunctions + 1 }
      catch message =>
        let issue := s!"specification function `{name}`: {message}"
        modify fun state => { state with
          unsupportedSource := state.unsupportedSource.push issue }
        -- Keep a typed uninterpreted declaration when only the defining term
        -- is outside the supported expression slice. Evidence above makes the
        -- loss explicit while downstream modules can still reference it.
        try
          result := result.push (← sourceSpecFunction artifact item true)
        catch _ => pure ()
  return result

private def sourceAxiomParts (item : Lean.Syntax) :
    Except String (Array String × Lean.Syntax) := do
  unless item.isOfKind ``Lean.Parser.Command.declaration && item.getNumArgs > 1 &&
      item[1].isOfKind ``Lean.Parser.Command.axiom && item[1].getNumArgs > 2 do
    throw "retained declaration is not an axiom"
  let signature := item[1][2]
  unless signature.getNumArgs > 1 && signature[1].getNumArgs > 1 do
    throw "retained axiom has no proposition"
  let mut genericNames := #[]
  for binder in signature[0].getArgs do
    if binder.isOfKind ``Lean.Parser.Term.implicitBinder && binder.getNumArgs > 1 then
      for name in binder[1].getArgs do
        unless name.isIdent do throw "retained axiom type binder is not an identifier"
        genericNames := genericNames.push name.getId.getString!
    else if binder.isOfKind ``Lean.Parser.Term.instBinder then
      -- Leaner emits the matching `Inhabited` evidence automatically for
      -- every Move type parameter carried by the invariant payload.
      pure ()
    else
      throw s!"unsupported retained axiom binder `{binder.getKind}`"
  return (genericNames, signature[1][1])

private def specTypeDomain (loc : LeanerIR.LocId) (type : ParsedSpecType) : BuildM LeanerIR.ExprId := do
  let domainType ← addDirectType <| .typeDomain type.typeId
  addExpr loc domainType <| .operation
    (.specification .typeDomain)
    #[.typeArg { typeId := type.typeId, loc }] #[]

private def sourceAxiom (artifact : Move.SourceModuleArtifact)
    (item : Lean.Syntax) : BuildM LeanerIR.NamespaceInvariant := do
  let (genericNames, proposition) ← sourceAxiomParts item
  let loc ← addSyntaxLocation item
  let bool ← boolType
  let mut locals := #[]
  let mut contextLocals := #[]
  let expression ← if proposition.isOfKind ``Lean.Parser.Term.forall &&
      proposition.getNumArgs > 4 then do
    unless proposition[2].getNumArgs > 0 && proposition[2][0].getNumArgs > 1 do
      throw "retained axiom quantifier has no explicit domain"
    let parsedType ← specType genericNames proposition[2][0][1]
    let mut binders := #[]
    for nameSyntax in proposition[1].getArgs do
      unless nameSyntax.isIdent do throw "retained axiom quantifier binder is not an identifier"
      let name := nameSyntax.getId.getString!
      let binderLoc ← addSyntaxLocation nameSyntax
      let localDeclaration : LeanerIR.LocalDecl := {
        id := ⟨locals.size⟩
        name
        type := { typeId := parsedType.typeId, loc := binderLoc }
        loc := binderLoc }
      locals := locals.push localDeclaration
      contextLocals := contextLocals.push (name, localDeclaration, parsedType.sourceType)
      let pattern ← addPattern binderLoc parsedType.typeId (.variable localDeclaration.id)
      binders := binders.push {
        pattern
        domain := ← specTypeDomain binderLoc parsedType }
    let context : SpecContext := {
      locals := contextLocals
      resultType := bool
      resultSourceType := some .bool
      sourceArtifact := artifact }
    let body ← specExpression context proposition[4] (some .bool)
    addExpr loc bool (.quantifier .forall binders #[] none body.expression)
  else do
    let context : SpecContext := {
      locals := #[]
      resultType := bool
      resultSourceType := some .bool
      sourceArtifact := artifact }
    let value ← specExpression context proposition (some .bool)
    pure value.expression
  return {
    loc
    condition := {
      loc
      kind := .axiom_ genericNames
      expression }
    locals }

private def sourceAxioms (artifact : Move.SourceModuleArtifact) :
    BuildM (Array LeanerIR.NamespaceInvariant) := do
  let mut result := #[]
  for item in artifact.items do
    if item.isOfKind ``Lean.Parser.Command.declaration && item.getNumArgs > 1 &&
        item[1].isOfKind ``Lean.Parser.Command.axiom then
      let name := if item[1].getNumArgs > 1 && item[1][1].getNumArgs > 0 &&
          item[1][1][0].isIdent then item[1][1][0].getId.getString! else "<unknown>"
      try
        result := result.push (← sourceAxiom artifact item)
        modify fun state => { state with importedInvariants := state.importedInvariants + 1 }
      catch message =>
        let issue := s!"axiom `{name}`: {message}"
        modify fun state => { state with
          unsupportedSource := state.unsupportedSource.push issue }
  return result

private def sourceContract (artifact : Move.SourceModuleArtifact)
    (sourceFunction : Move.Compiler.LIR.FunDecl)
    (lirFunction : LeanerIR.FunctionDecl LeanerIR.Import.RawBody) : BuildM
      (LeanerIR.FunctionDecl LeanerIR.Import.RawBody) := do
  let some item := artifact.items.find? fun item =>
      (item.isOfKind ``Move.Spec.abortsIfSourceSpec ||
        item.isOfKind ``Move.Spec.ensuresOnlySpec ||
        item.getKind.toString == "Move.Spec.commandSpec__Where__Ensures_") &&
        item.getNumArgs > 1 && item[1].isIdent &&
        item[1].getId.getString! == sourceFunction.moveName
    | return lirFunction
  let namedLocals := uniqueNames sourceFunction.params sourceFunction.locals
  let locals := namedLocals.filterMap fun (_, sourceName, sourceType) =>
    (lirFunction.locals.find? fun declaration => declaration.name == sourceName).map fun declaration =>
      (sourceName, declaration, some sourceType)
  let resultSourceType := sourceFunction.returns[0]?
  let context := {
    locals
    resultType := lirFunction.signature.results[0]!.typeId
    resultSourceType
    sourceArtifact := artifact }
  let mut conditions := #[]
  let mut modifies := #[]
  let mut modifiesAll := false
  let ensuresOnly := item.isOfKind ``Move.Spec.ensuresOnlySpec
  let pragmas ← if ensuresOnly then pure #[] else sourcePragmas item[4]
  if ensuresOnly then
    conditions := conditions.push (← sourceCondition context "ensures" item[5])
  else if item.isOfKind ``Move.Spec.abortsIfSourceSpec then
    if item[5].getNumArgs > 1 then
      conditions := conditions.push (← sourceCondition context "requires" item[5][1])
    let frame ← sourceFrame context item[6]
    modifies := frame.1
    modifiesAll := frame.2
    conditions := conditions.push (← sourceCondition context "ensures" item[8])
    let code := if item[12].getNumArgs > 1 then some item[12][1] else none
    conditions := conditions.push (← sourceCondition context "abortsIf" item[11] code)
    for additional in item[13].getArgs do
      let code := if additional.getNumArgs > 3 && additional[3].getNumArgs > 1 then
          some additional[3][1]
        else none
      conditions := conditions.push (← sourceCondition context "abortsIf" additional[2] code)
  else
    let frame ← sourceFrame context item[5]
    modifies := frame.1
    modifiesAll := frame.2
    conditions := conditions.push (← sourceCondition context "ensures" item[7])
  modify fun state => { state with importedContracts := state.importedContracts + 1 }
  return { lirFunction with contract := {
    loc := some (← addSyntaxLocation item)
    conditions
    modifies
    hasFrame := if ensuresOnly then false else
      !item[if item.isOfKind ``Move.Spec.abortsIfSourceSpec then 6 else 5].getArgs.isEmpty
    modifiesAll
    pragmas } }

private def attachSourceContracts (artifact : Move.SourceModuleArtifact)
    (sourceFunctions : Array Move.Compiler.LIR.FunDecl)
    (functions : Array (LeanerIR.FunctionDecl LeanerIR.Import.RawBody)) : BuildM
      (Array (LeanerIR.FunctionDecl LeanerIR.Import.RawBody)) := do
  let mut result := #[]
  for (sourceFunction, lirFunction) in sourceFunctions.zip functions do
    try
      result := result.push (← sourceContract artifact sourceFunction lirFunction)
    catch message =>
      let issue := s!"contract `{sourceFunction.moveName}`: {message}"
      modify fun state =>
        let issues := state.unsupportedSource.push issue
        { state with unsupportedSource := issues }
      result := result.push lirFunction
  for item in artifact.items do
    if item.isOfKind ``Move.Spec.dataInvariantSpec ||
        item.isOfKind ``Move.Spec.globalInvariantSpec then
      let issue := s!"invariant declaration at byte {item.getPos?.map (·.byteIdx) |>.getD 0}"
      modify fun state =>
        let issues := state.unsupportedSource.push issue
        { state with unsupportedSource := issues }
  return result

private def rawUnitWithSource (source : Move.Compiler.LIR.Module) (sourceInfo : SourceInfo)
    (sourceArtifact : Option Move.SourceModuleArtifact) : Except String LeanerIR.Import.RawUnit := do
  let rootLocation : LeanerIR.Location := {
    primary := some { file := ⟨0⟩, startByte := 0, endByte := sourceInfo.sourceSize } }
  let initial : BuildState := {
    source
    namespaces := #[ownModule source]
    locations := #[rootLocation] }
  let (lirNamespace, finalState) ← (do
    let structDecls ← source.structs.mapM structDecl
    let mut functionDecls ← source.functions.mapM function
    let mut constantDecls := #[]
    let mut specFunctionDecls := #[]
    let mut invariantDecls := #[]
    if let some sourceArtifact := sourceArtifact then
      functionDecls ← attachSourceContracts sourceArtifact source.functions functionDecls
      constantDecls ← sourceConstants sourceArtifact
      specFunctionDecls ← sourceSpecFunctions sourceArtifact
      invariantDecls ← sourceAxioms sourceArtifact
    -- CFG structurization introduces fallthrough blocks, loop bodies, branch
    -- joins, and assignment statements whose result type is necessarily Unit.
    -- Keep Unit available even when every source function returns a value.
    let _ ← addDirectType .unit
    let current ← get
    pure ({
      loc := ⟨0⟩
      identity := ⟨0⟩
      profile := some .move
      expressions := current.expressions
      patterns := current.patterns
      profileMetadata := source.friends.map fun (friend : Move.Compiler.LIR.ExternalModuleRef) =>
        LeanerIR.Move.propertyValue "metadata.friend" <| Codec.encodeModuleRef {
          address := friend.address, addressAlias := none, name := friend.moduleName }
      constants := constantDecls
      structs := structDecls
      functions := functionDecls
      specFunctions := specFunctionDecls
      invariants := invariantDecls } : LeanerIR.Import.RawNamespace)).run initial
  let sourceEvidence := finalState.unsupportedSource.map fun unsupported => ({
    producer := "leaner-source-bridge"
    description := "unsupported source semantic item: " ++ unsupported
    trusted := false } : LeanerIR.Import.ImportEvidence)
  return {
    tables := {
      files := #[{ name := sourceInfo.fileName, contentHash := sourceInfo.contentHash }]
      locations := finalState.locations
      origins := #[{
        kind := LeanerIR.OriginKind.leanerSource
        location := ⟨0⟩
        sourceIdentity := some sourceInfo.fileName }]
      alignments := #[{
        source := ⟨0⟩
        trust := .checked
        description := "Lean elaboration and named compiler IR" }]
      lifetimes := finalState.lifetimes
      types := finalState.types
      namespaces := finalState.namespaces.map fun (module : ModuleRef) => {
        segments := #[module.address, module.addressAlias.getD "", module.name] }
      names := finalState.names }
    profiles := #[LeanerIR.Move.config]
    namespaces := #[lirNamespace]
    evidence := #[({
      producer := "leaner-named-ir"
      description := s!"Lean elaboration plus named compiler IR; imported {finalState.importedConstants} constant(s), {finalState.importedContracts} source contract(s), {finalState.importedSpecFunctions} specification function(s), and {finalState.importedInvariants} axiom/invariant(s)"
      trusted := finalState.unsupportedSource.isEmpty && sourceArtifact.isSome } : LeanerIR.Import.ImportEvidence)] ++
      sourceEvidence }

/-- Convert an already elaborated Leaner named module to the only public LIR
frontend boundary. The caller explicitly selects its semantic language. This
adapter currently implements Move-profile Leaner; other selections fail
instead of reinterpreting the source. The returned unit is still raw: all
graph checking and structurization is performed by `LeanerIR.Move.validate`.
A standalone NSIR value has no retained source artifact, so this entry point
imports executable semantics only; `compileNamespace` supplies the source side
as well. -/
def rawUnit (profile : LeanerIR.Profile) (source : Move.Compiler.LIR.Module)
    (sourceInfo : SourceInfo) : Except String LeanerIR.Import.RawUnit :=
  match profile with
  | .move => rawUnitWithSource source sourceInfo none
  | .rust => .error "the Rust-profile Leaner frontend is not implemented"
  | .extension _ => .error "no Leaner frontend is registered for the selected extension profile"

/-- Compile selected Leaner declarations with the existing elaboration path,
then mechanically construct raw LIR. -/
def compile (profile : LeanerIR.Profile) (module : Move.ModuleRef)
    (structNames functionNames : Array Lean.Name)
    (sourceInfo : SourceInfo) : Lean.CoreM (Except String LeanerIR.Import.RawUnit) := do
  match profile with
  | .move =>
      let named ← Move.Compiler.compileModule module structNames functionNames
      return rawUnitWithSource named sourceInfo none
  | .rust => return .error "the Rust-profile Leaner frontend is not implemented"
  | .extension _ =>
      return .error "no Leaner frontend is registered for the selected extension profile"

/-- Compile every deployable declaration in a registered Leaner Move module.
The module identity and declaration set come from persistent elaboration
metadata, so an LIR client does not need to duplicate the compiler's selection
rules or manually enumerate a namespace. -/
def compileNamespace (profile : LeanerIR.Profile) (namespaceName : Lean.Name)
    (sourceInfo : SourceInfo) :
    Lean.CoreM (Except String LeanerIR.Import.RawUnit) := do
  match profile with
  | .rust => return .error "the Rust-profile Leaner frontend is not implemented"
  | .extension _ =>
      return .error "no Leaner frontend is registered for the selected extension profile"
  | .move => pure ()
  let env ← Lean.getEnv
  let some module := Move.moduleForNamespace? env namespaceName
    | return .error s!"`{namespaceName}` is not a registered Move module namespace"
  let (structNames, functionNames) :=
    Move.Compiler.declarationsInNamespace env namespaceName
  let named ← Move.Compiler.compileModule module structNames functionNames
  let artifact := Move.sourceModuleArtifact? env namespaceName
  let result := rawUnitWithSource named sourceInfo artifact
  return result.map fun raw =>
    let sourceStatus := if (Move.sourceModuleArtifact? env namespaceName).isSome then
        "parsed Leaner source retained"
      else
        "parsed Leaner source unavailable"
    { raw with evidence := raw.evidence.map fun evidence =>
        { evidence with description := evidence.description ++ "; " ++ sourceStatus } }

end Transpiler.LIR.Leaner
