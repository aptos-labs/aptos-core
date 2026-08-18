-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Attributes
import Move.Basic
import Move.Action
import Move.Compiler.LIR
import Move.Compiler.LCNF

/-!
# LCNF normalization

This pass recognizes the deliberately small Leaner primitive vocabulary,
erases `Action`'s world token, and constructs a named, typed LIR.  Unknown
runtime calls are errors instead of being silently compiled with Lean
semantics.
-/

namespace Move.Compiler

open Lean
open Lean.Compiler.LCNF

private abbrev Local := String

private def localName (id : FVarId) : Local := id.name.toString

private def assocFind? [BEq α] (key : α) : List (α × β) → Option β
  | [] => none
  | (k, v) :: rest => if k == key then some v else assocFind? key rest

private def typeName? (type : Expr) : Option Name :=
  match type.getAppFn with
  | .const name _ => some name
  | _ => none

private structure TyContext where
  bvars : List (Nat × LIR.Ty) := []
  fvars : List (FVarId × LIR.Ty) := []

private partial def translateTyWith (env : Environment) (ctx : TyContext)
    (type : Expr) : Except String (Option LIR.Ty) := do
  if let .bvar index := type then
    let some ty := assocFind? index ctx.bvars
      | throw s!"unbound type variable `#{index}` in Move type (context {repr ctx.bvars})"
    return some ty
  if let .fvar id := type then
    let some ty := assocFind? id ctx.fvars
      | throw s!"unbound type variable `{id.name}` in Move type"
    return some ty
  if type.isSort then return none
  if type.isConstOf ``Bool then return some .bool
  if type.isConstOf ``U64 then return some .u64
  if type.isConstOf ``Address then return some .address
  if type.isConstOf ``Signer then return some .signer
  if type.isConstOf ``World || type.isConstOf ``PUnit || type.isConstOf ``Unit then
    return none
  -- Lean needs `Inhabited T` evidence to give compiler-marker operations such
  -- as checked vector borrows a total host implementation.  The evidence is
  -- not a Move value and is erased exactly like `Type` and `World` binders.
  if type.isAppOfArity ``Inhabited 1 then return none
  if type.isAppOfArity ``Ref 1 then
    let some elem ← translateTyWith env ctx type.appArg!
      | throw s!"reference to erased type `{type}`"
    return some (.ref elem)
  if type.isAppOfArity ``MutRef 1 then
    let some elem ← translateTyWith env ctx type.appArg!
      | throw s!"mutable reference to erased type `{type}`"
    return some (.mutRef elem)
  if type.isAppOfArity ``Move.Vector 1 then
    let some elem ← translateTyWith env ctx type.appArg!
      | throw s!"vector of erased type `{type}`"
    return some (.vector elem)
  if let some name := typeName? type then
    if moveStructAttr.hasTag env name || moveEnumAttr.hasTag env name then
      let args ← type.getAppArgs.toList.mapM fun arg => do
        let some ty ← translateTyWith env ctx arg
          | throw s!"erased type argument `{arg}` in `{type}`"
        pure ty
      if moveEnumAttr.hasTag env name then
        return some <| if args.isEmpty then .enum name else .enumInst name args.toArray
      return some <| if args.isEmpty then .struct name else .structInst name args.toArray
  throw s!"`{type}` is not a supported Move runtime type"

private def translateTy (env : Environment) (type : Expr) : Except String (Option LIR.Ty) :=
  translateTyWith env {} type

private def resultType (type : Expr) : Expr :=
  match type with
  | .forallE _ _ body _ => resultType body
  | other => other

private structure FunSignature where
  typeParams : Array LIR.TypeParamDecl
  params : Array LIR.Ty
  returns : Array LIR.Ty
  isEffectful : Bool

private abbrev FunSignatures := List (Name × FunSignature)

private def forallBinders : Expr → List (Name × Expr)
  | .forallE name type body _ => (name, type) :: forallBinders body
  | _ => []

private def syntheticType (name : Name) : Option LIR.Ty :=
  match name.getString! with
  | "Bool" => some .bool
  | "U64" => some .u64
  | "Address" => some .address
  | "Signer" => some .signer
  | _ => none

private def functionContext (env : Environment) (decl : Decl .pure) :
    Except String (TyContext × Array LIR.TypeParamDecl) := do
  let some original := env.find? decl.name
    | throw s!"missing source declaration `{decl.name}`"
  let sourceNames := (forallBinders original.type).filterMap fun (name, type) =>
    if type.isSort then some name else none
  let mut fvars := []
  let mut typeParams := #[]
  let mut positions := #[]
  for (param, position) in decl.params.zipIdx do
    if param.type.isSort then
      let ty ← match sourceNames.idxOf? param.binderName with
        | some index =>
            while typeParams.size ≤ index do
              let name := sourceNames[typeParams.size]!
              typeParams := typeParams.push {
                name := name.toString
                -- Lean's `Type` binder carries no Move ability information.
                -- The prototype's generated projections and local handling
                -- may copy and drop values, while stored fields require
                -- `store`, so infer the conservative supported bound.
                abilities := { copy := true, drop := true, store := true }
              }
            pure (.typeParam index)
        | none => match syntheticType param.binderName with
          | some ty => pure ty
          | none => throw s!"unsupported compiler type parameter `{param.binderName}`"
      fvars := (param.fvarId, ty) :: fvars
      positions := positions.push (position, ty)
  let binderCount := (forallBinders decl.type).length
  let bvars := positions.toList.map fun (position, ty) =>
    (binderCount - 1 - position, ty)
  pure ({ bvars := bvars, fvars := fvars }, typeParams)

private def functionShape (env : Environment) (ctx : TyContext)
    (type : Expr) : Except String (Bool × Array LIR.Ty) := do
  let type := resultType type
  if type.isAppOfArity ``Prod 2 then
    let args := type.getAppArgs
    unless args[1]!.isConstOf ``World do
      throw s!"product-valued Move functions are not supported: `{type}`"
    match translateTyWith env ctx args[0]! with
    | .ok (some ty) => return (true, #[ty])
    | .ok none => return (true, #[])
    | .error message => throw message
  match translateTyWith env ctx type with
  | .ok (some ty) => return (false, #[ty])
  | .ok none => return (false, #[])
  | .error message => throw message

private def signatureOf (env : Environment) (decl : Decl .pure) : Except String FunSignature := do
  let (ctx, typeParams) ← functionContext env decl
  let (isEffectful, returns) ← functionShape env ctx decl.type
  let mut params := #[]
  for param in decl.params do
    match translateTyWith env ctx param.type with
    | .error message => throw message
    | .ok none => pure ()
    | .ok (some ty) => params := params.push ty
  return { typeParams, params, returns, isEffectful }

private def typeParamBinders (type : Expr) (count : Nat) : Except String (Array Name) := do
  let binders := (forallBinders type).take count
  unless binders.length == count do throw "missing generic type parameter binders"
  let mut names := #[]
  for (name, type) in binders do
    unless type.isSort do throw s!"Move type parameter `{name}` must have type `Type`"
    names := names.push name
  pure names

private def bvarContext (binderCount paramCount : Nat) : TyContext :=
  { bvars := (List.range paramCount).map fun param =>
      (binderCount - 1 - param, .typeParam param) }

private partial def tyUsesParam (index : Nat) : LIR.Ty → Bool
  | .typeParam actual => actual == index
  | .structInst _ args | .enumInst _ args => args.any (tyUsesParam index)
  | .vector elem | .ref elem | .mutRef elem => tyUsesParam index elem
  | _ => false

private def inferredTypeParams (names : Array Name) (abilities : LIR.AbilitySet)
    (fieldTypes : Array LIR.Ty) : Array LIR.TypeParamDecl :=
  names.mapIdx fun index name =>
    let phantom := !fieldTypes.any (tyUsesParam index)
    let constraints := if phantom then {} else
      { copy := abilities.copy
        drop := abilities.drop
        store := abilities.store || abilities.key }
    { name := name.toString, abilities := constraints, phantom := phantom }

private def declaredAbilities (env : Environment) (name : Name) : LIR.AbilitySet :=
  { copy := moveCopyAttr.hasTag env name
    drop := moveDropAttr.hasTag env name
    store := moveStoreAttr.hasTag env name
    key := moveKeyAttr.hasTag env name }

private def compileStruct (env : Environment) (name : Name) : Except String LIR.StructDecl := do
  unless moveStructAttr.hasTag env name do
    throw s!"`{name}` is not annotated with `move_struct`"
  let some info := getStructureInfo? env name
    | throw s!"`{name}` is not a Lean structure"
  let some (.inductInfo induct) := env.find? name
    | throw s!"missing inductive metadata for structure `{name}`"
  unless induct.numIndices = 0 do throw s!"indexed Move structure `{name}` is not supported"
  let paramNames ← typeParamBinders induct.type induct.numParams
  let abilities := declaredAbilities env name
  let mut fields := #[]
  for fieldName in info.fieldNames do
    let projectionName := name ++ fieldName
    let some constInfo := env.find? projectionName
      | throw s!"missing projection `{projectionName}`"
    let binders := forallBinders constInfo.type
    let ctx := bvarContext binders.length induct.numParams
    let some ty ← translateTyWith env ctx (resultType constInfo.type)
      | throw s!"field `{projectionName}` has an erased type"
    fields := fields.push {
      leanName := projectionName
      moveName := fieldName.toString
      ty := ty
    }
  return {
    leanName := name
    moveName := name.getString!
    typeParams := inferredTypeParams paramNames abilities (fields.map (·.ty))
    abilities := abilities
    fields := fields
  }

private def compileEnum (env : Environment) (name : Name) : Except String LIR.StructDecl := do
  unless moveEnumAttr.hasTag env name do
    throw s!"`{name}` is not annotated with `move_enum`"
  let some (.inductInfo info) := env.find? name
    | throw s!"`{name}` is not a Lean inductive type"
  unless info.numIndices = 0 do
    throw s!"indexed Move enum `{name}` is not supported by the prototype"
  if info.isRec then throw s!"recursive Move enum `{name}` is not supported by the prototype"
  if info.ctors.isEmpty then throw s!"Move enum `{name}` must declare at least one variant"
  let paramNames ← typeParamBinders info.type info.numParams
  let mut variants := #[]
  for ctorName in info.ctors do
    let some (.ctorInfo ctor) := env.find? ctorName
      | throw s!"missing enum constructor `{ctorName}`"
    let allBinders := forallBinders ctor.type
    let binders := allBinders.drop ctor.numParams |>.take ctor.numFields
    let mut fields := #[]
    for ((fieldName, fieldType), index) in binders.zipIdx do
      let ctx := bvarContext (ctor.numParams + index) ctor.numParams
      let some ty ← translateTyWith env ctx fieldType
        | throw s!"field {index} of enum constructor `{ctorName}` has an erased type"
      let generatedName := fieldName.isAnonymous || fieldName.isInaccessibleUserName ||
        fieldName.hasMacroScopes || fieldName.isInternal
      let moveName := if generatedName then s!"field{index}" else fieldName.toString
      fields := fields.push {
        leanName := ctorName ++ Name.mkSimple moveName
        moveName := moveName
        ty := ty
      }
    variants := variants.push {
      leanName := ctorName
      moveName := ctorName.getString!
      fields := fields
    }
  let fieldTypes := variants.foldl (init := #[]) fun types variant =>
    types ++ variant.fields.map (·.ty)
  let abilities := declaredAbilities env name
  return {
    leanName := name
    moveName := name.getString!
    typeParams := inferredTypeParams paramNames abilities fieldTypes
    abilities := abilities
    fields := #[]
    variants := some variants
  }

private def fvarArgs (args : Array (Arg .pure)) : Array FVarId :=
  args.filterMap fun
    | .fvar id => some id
    | _ => none

private def typeArgs (args : Array (Arg .pure)) : Array Expr :=
  args.filterMap fun
    | .type type _ => some type
    | _ => none

private def beforeWorld? (vars : Array FVarId) : Option FVarId :=
  if vars.size < 2 then none else vars[vars.size - 2]?

private def projectionIndex? : Code .pure → Option Nat
  | .let decl (.return result) =>
      match decl.value with
      | .proj _ index _ _ => if decl.fvarId == result then some index else none
      | _ => none
  | _ => none

private def enumConstructor? (env : Environment) (name : Name) : Option (Name × Nat × Nat × Nat) :=
  match env.find? name with
  | some (.ctorInfo ctor) =>
      if moveEnumAttr.hasTag env ctor.induct then
        some (ctor.induct, ctor.cidx, ctor.numParams, ctor.numFields)
      else none
  | _ => none

private def structConstructor? (env : Environment) (name : Name) : Option (Name × Nat × Nat) :=
  match env.find? name with
  | some (.ctorInfo ctor) =>
      if moveStructAttr.hasTag env ctor.induct then
        some (ctor.induct, ctor.numParams, ctor.numFields)
      else none
  | _ => none

private partial def u64Constant? (name : Name) : CoreM (Option Nat) := do
  let decl ← match (← Lean.Compiler.LCNF.getBaseDecl? name) with
    | some decl => pure decl
    | none =>
        Lean.compileDecls #[name]
        let some decl ← Lean.Compiler.LCNF.getBaseDecl? name | return none
        pure decl
  let .code code := decl.value | return none
  let rec scan (values : List (FVarId × Nat)) : Code .pure → Option Nat
    | .let letDecl next =>
        match letDecl.value with
        | .lit (.nat n) => scan ((letDecl.fvarId, n) :: values) next
        | .const fn _ args _ =>
            if fn == ``U64.ofNat then
              match (fvarArgs args)[0]? with
              | some arg =>
                  match assocFind? arg values with
                  | some n => scan ((letDecl.fvarId, n) :: values) next
                  | none => none
              | none => none
            else
              scan values next
        | _ => scan values next
    | .return result => assocFind? result values
    | _ => none
  return scan [] code

private structure PendingCall where
  op : LIR.Oper
  srcs : Array FVarId
  resultTy : Option LIR.Ty

private inductive Pending where
  | call (call : PendingCall)
  | abort (code : FVarId)
  | vectorInsert (reference index value : FVarId) (elemTy : LIR.Ty)
  | vectorRemove (reference index : FVarId) (elemTy : LIR.Ty)

private structure BuildState where
  module : Move.ModuleRef
  tyContext : TyContext := {}
  locals : Array LIR.LocalDecl := #[]
  localIds : List FVarId := []
  loopParams : Array LIR.LocalDecl := #[]
  pending : List (FVarId × Pending) := []
  projections : List (FVarId × Nat) := []
  descriptors : List (FVarId × Nat) := []
  natLiterals : List (FVarId × Nat) := []
  returnAliases : List (FVarId × Option FVarId) := []
  joins : List (FVarId × (String × Array FVarId)) := []
  loopHeaders : List (Nat × String) := []
  loopExits : List (Nat × String) := []
  builtLoopExits : List Nat := []
  loopStates : List (Nat × Array FVarId) := []
  loopTails : List (Nat × (Array FVarId × Code .pure)) := []
  products : List (FVarId × Array FVarId) := []
  loopLiveResults : List FVarId := []
  blocks : Array LIR.Block := #[]
  calls : Array Name := #[]
  acquires : Array Name := #[]
  nextBlock : Nat := 0
  nextTemp : Nat := 0

private abbrev BuildM := StateT BuildState CoreM

private def addLocal (env : Environment) (id : FVarId) (type : Expr) : BuildM Unit := do
  let state ← get
  if state.localIds.any (· == id) then return
  -- Structured-loop tokens are compiler-only `Nat`s threaded through LCNF
  -- join points solely to keep marker calls live.
  if type.isConstOf ``Nat && id.name.toString.contains "loopTok" then return
  match translateTyWith env state.tyContext type with
  | .error message => throwError message
  | .ok none => return
  | .ok (some ty) =>
      modify fun s => { s with
        locals := s.locals.push { name := localName id, ty := ty }
        localIds := id :: s.localIds }

private def addLocalTy (id : FVarId) (ty : LIR.Ty) : BuildM Unit := do
  let state ← get
  if state.localIds.any (· == id) then return
  modify fun s => { s with
    locals := s.locals.push { name := localName id, ty := ty }
    localIds := id :: s.localIds }

private def translateTypeArgs (env : Environment) (types : Array Expr)
    (count : Nat) : BuildM (Array LIR.Ty) := do
  unless count ≤ types.size do
    throwError "generic application has {types.size} type arguments; expected at least {count}"
  let ctx := (← get).tyContext
  let mut result := #[]
  for type in types.extract 0 count do
    match translateTyWith env ctx type with
    | .ok (some ty) => result := result.push ty
    | .ok none => throwError "generic application uses an erased type argument"
    | .error message => throwError message
  pure result

private def translateCurrentTy (env : Environment) (type : Expr) : BuildM (Option LIR.Ty) := do
  match translateTyWith env (← get).tyContext type with
  | .ok ty => pure ty
  | .error message => throwError message

private def localTy? (id : FVarId) : BuildM (Option LIR.Ty) := do
  let state ← get
  let name := localName id
  pure <| (state.loopParams ++ state.locals).find? (·.name == name) |>.map (·.ty)

private def addAcquire (name : Name) : BuildM Unit :=
  modify fun s => if s.acquires.any (· == name) then s else { s with acquires := s.acquires.push name }

private def addCall (name : Name) : BuildM Unit :=
  modify fun s => if s.calls.any (· == name) then s else { s with calls := s.calls.push name }

private def freshBlock (stem : String) : BuildM String := do
  let state ← get
  modify fun s => { s with nextBlock := s.nextBlock + 1 }
  return s!"{stem}.{state.nextBlock}"

private def freshTemp (ty : LIR.Ty) : BuildM String := do
  let state ← get
  let name := s!"$tail.{state.nextTemp}"
  modify fun s => { s with
    locals := s.locals.push { name := name, ty := ty }
    nextTemp := s.nextTemp + 1 }
  return name

/-- Materialize a pending effect once LCNF exposes the `StateM` result.
Vector insertion/removal retain Move's native reference API at source level,
but are normalized to value operations bracketed by `read_ref`/`write_ref`.
That keeps LIR and the downstream reference-elimination proof vocabulary
small while preserving the removed element as an ordinary destination. The
compiler-v2 XIR reader recognizes this canonical triple and emits the update
directly against the original mutable vector reference. -/
private def materializePending (pending : Pending) (dsts : Array String)
    (instrs : Array LIR.Instr) : BuildM (Array LIR.Instr) := do
  match pending with
  | .call call =>
      return instrs.push (.call dsts call.op (call.srcs.map localName))
  | .abort _ => throwError "internal: attempted to materialize an abort as a call"
  | .vectorInsert reference index value elemTy =>
      unless dsts.isEmpty do
        throwError "vector insert unexpectedly produced a Move value"
      let vectorTy := LIR.Ty.vector elemTy
      let old ← freshTemp vectorTy
      let updated ← freshTemp vectorTy
      return instrs
        |>.push (.call #[old] .readRef #[localName reference])
        |>.push (.call #[updated] .vecInsert
          #[old, localName index, localName value])
        |>.push (.call #[] .writeRef #[localName reference, updated])
  | .vectorRemove reference index elemTy =>
      unless dsts.size == 1 do
        throwError "vector remove must produce exactly one removed element"
      let vectorTy := LIR.Ty.vector elemTy
      let old ← freshTemp vectorTy
      let updated ← freshTemp vectorTy
      return instrs
        |>.push (.call #[old] .readRef #[localName reference])
        |>.push (.call #[updated, dsts[0]!] .vecRemove
          #[old, localName index])
        |>.push (.call #[] .writeRef #[localName reference, updated])

private def emitBlock (name : String) (instrs : Array LIR.Instr)
    (term : LIR.Terminator) : BuildM Unit :=
  modify fun s => { s with blocks := s.blocks.push { name, instrs, term } }

private def srcName (id : FVarId) : Local := localName id

private def natLiteral? (id : FVarId) : BuildM (Option Nat) := do
  return assocFind? id (← get).natLiterals

private def loopMarkerNats (vars : Array FVarId) : BuildM (Array Nat) := do
  let mut result := #[]
  for id in vars do
    if let some n ← natLiteral? id then
      result := result.push n
  return result

private partial def flattenLoopState (arity : Nat) (id : FVarId) :
    BuildM (Array FVarId) := do
  if arity == 0 then return #[]
  if arity == 1 then return #[id]
  let some fields := assocFind? id (← get).products
    | throwError "loop state tuple with {arity} fields was not retained by LCNF"
  let some first := fields[0]?
    | throwError "loop state tuple has no first field"
  let some rest := fields[1]?
    | throwError "loop state tuple has no tail"
  return #[first] ++ (← flattenLoopState (arity - 1) rest)

private inductive LoopMarker where
  | enter (label : Nat) (state : Array FVarId)
  | continue_ (label : Nat) (state : Array FVarId) (tail : Bool)
  | break_ (label : Nat) (state : Array FVarId)

private def loopMarker? (fn : Name) (vars : Array FVarId) : BuildM (Option LoopMarker) := do
  let isMarker := fn == ``Move.loopEnter || fn == ``loopEnter ||
    fn == ``Move.loopContinue || fn == ``loopContinue ||
    fn == ``Move.loopContinueTail || fn == ``loopContinueTail ||
    fn == ``Move.loopBreak || fn == ``loopBreak
  unless isMarker do return none
  let literals ← loopMarkerNats vars
  let some label := literals[0]?
    | throwError "loop marker is missing its static label"
  let some arity := literals[1]?
    | throwError "loop marker is missing its state arity"
  let state ←
    if arity == 0 then pure #[]
    else
      let some packed := vars.back?
        | throwError "loop marker is missing its state"
      flattenLoopState arity packed
  if fn == ``Move.loopEnter || fn == ``loopEnter then
    return some (.enter label state)
  if fn == ``Move.loopContinue || fn == ``loopContinue then
    return some (.continue_ label state false)
  if fn == ``Move.loopContinueTail || fn == ``loopContinueTail then
    return some (.continue_ label state true)
  if fn == ``Move.loopBreak || fn == ``loopBreak then
    return some (.break_ label state)
  return none

private def loopHeader (label : Nat) : BuildM String := do
  if let some name := assocFind? label (← get).loopHeaders then
    return name
  let name ← freshBlock s!"loop.{label}"
  modify fun s => { s with loopHeaders := (label, name) :: s.loopHeaders }
  return name

private def loopExit (label : Nat) : BuildM String := do
  if let some name := assocFind? label (← get).loopExits then
    return name
  let name ← freshBlock s!"loop.exit.{label}"
  modify fun s => { s with loopExits := (label, name) :: s.loopExits }
  return name

private def assignLoopState (label : Nat) (targets sources : Array FVarId)
    (instrs : Array LIR.Instr) : BuildM (Array LIR.Instr) := do
  unless targets.size == sources.size do
    throwError "loop {label} edge has {sources.size} state values; expected {targets.size}"
  let mut nextInstrs := instrs
  let mut temps : Array String := #[]
  for (target, source) in targets.zip sources do
    let ty ← match ← localTy? target with
      | some ty => pure ty
      | none =>
          let some ty ← localTy? source
            | throwError "loop {label} state `{localName target}` has no Move type"
          addLocalTy target ty
          pure ty
    let temp ← freshTemp ty
    temps := temps.push temp
    nextInstrs := nextInstrs.push (.assign temp (srcName source))
  for (target, temp) in targets.zip temps do
    nextInstrs := nextInstrs.push (.assign (srcName target) temp)
  return nextInstrs

private def emitLoopContinue (label : Nat) (state : Array FVarId)
    (blockName : String) (instrs : Array LIR.Instr) : BuildM Unit := do
  let some headerState := assocFind? label (← get).loopStates
    | throwError "continue targets loop {label} before its header state was recorded"
  let nextInstrs ← assignLoopState label headerState state instrs
  emitBlock blockName nextInstrs (.jump (← loopHeader label))

private def emitLoopBreak (label : Nat) (state : Array FVarId)
    (blockName : String) (instrs : Array LIR.Instr) : BuildM Unit := do
  let some headerState := assocFind? label (← get).loopStates
    | throwError "break targets loop {label} before its header state was recorded"
  let nextInstrs ← assignLoopState label headerState state instrs
  emitBlock blockName nextInstrs (.jump (← loopExit label))

private def continuedCall? (decl : LetDecl .pure)
    (next : Code .pure) : Option (Name × Array FVarId) :=
  match decl.value, next with
  | .const fn _ args _, .let markerDecl (.return result) =>
      match markerDecl.value with
      | .const marker _ markerArgs _ =>
          let markerVars := fvarArgs markerArgs
          if marker == ``continueMarker && markerVars[0]? == some decl.fvarId &&
              result == markerDecl.fvarId then
            some (fn, fvarArgs args)
          else
            none
      | _ => none
  | _, _ => none

private def emitTailCall (_signature : FunSignature) (blockName : String)
    (instrs : Array LIR.Instr) (vars : Array FVarId) : BuildM Unit := do
  let params := (← get).loopParams
  -- Generic `partial` functions carry compiler-inserted `Inhabited`
  -- dictionaries, and effectful functions additionally carry the erased
  -- world token.  Neither is a Move runtime argument.
  let mut callVars := #[]
  for var in vars do
    if (← localTy? var).isSome then
      callVars := callVars.push var
  unless callVars.size == params.size do
    throwError "tail call has {callVars.size} runtime arguments; expected {params.size}"
  let mut nextInstrs := instrs
  let mut temps : Array String := #[]
  for (param, arg) in params.zip callVars do
    let temp ← freshTemp param.ty
    temps := temps.push temp
    nextInstrs := nextInstrs.push (.assign temp (srcName arg))
  for (param, temp) in params.zip temps do
    nextInstrs := nextInstrs.push (.assign param.name temp)
  emitBlock blockName nextInstrs (.jump "entry")

private def emitJoinJump (blockName : String) (instrs : Array LIR.Instr)
    (destination : String) (params : Array FVarId)
    (args : Array (Arg .pure)) : BuildM Unit := do
  unless params.size == args.size do
    throwError "LCNF join jump has {args.size} arguments; expected {params.size}"
  let mut nextInstrs := instrs
  let mut assignments : Array (FVarId × String) := #[]
  for (param, arg) in params.zip args do
    if let some ty ← localTy? param then
      let .fvar source := arg
        | throwError "Move-valued join argument is not a local"
      let temp ← freshTemp ty
      nextInstrs := nextInstrs.push (.assign temp (srcName source))
      assignments := assignments.push (param, temp)
  for (param, temp) in assignments do
    nextInstrs := nextInstrs.push (.assign (srcName param) temp)
  emitBlock blockName nextInstrs (.jump destination)

private def recognizeLet (signatures : FunSignatures) (decl : LetDecl .pure)
    (instrs : Array LIR.Instr) : BuildM (Array LIR.Instr) := do
  match decl.value with
  | .lit (.nat n) =>
      modify fun s => { s with natLiterals := (decl.fvarId, n) :: s.natLiterals }
      return instrs
  | .const fn _ args _ =>
      let vars := fvarArgs args
      let types := typeArgs args
      if fn == ``continueMarker then
        throwError "`continue` must mark a direct self-call in tail position"
      if fn == ``Move.loopTokenLive || fn == ``loopTokenLive then
        modify fun s => { s with loopLiveResults := decl.fvarId :: s.loopLiveResults }
        return instrs
      if fn == ``Move.loopTokenJoin || fn == ``loopTokenJoin then
        return instrs
      if let some _ ← loopMarker? fn vars then
        return instrs
      if let some (structName, numParams, numFields) := structConstructor? (← getEnv) fn then
        unless vars.size == numFields do
          throwError "structure constructor `{fn}` has {vars.size} runtime fields; expected {numFields}"
        let typeArgs ← translateTypeArgs (← getEnv) types numParams
        let resultTy := if typeArgs.isEmpty then .struct structName else .structInst structName typeArgs
        addLocalTy decl.fvarId resultTy
        return instrs.push (.call #[srcName decl.fvarId]
          (.pack structName typeArgs) (vars.map srcName))
      if let some (enumName, variant, numParams, numFields) := enumConstructor? (← getEnv) fn then
        unless vars.size == numFields do
          throwError "enum constructor `{fn}` has {vars.size} runtime fields; expected {numFields}"
        let typeArgs ← translateTypeArgs (← getEnv) types numParams
        addLocalTy decl.fvarId <|
          if typeArgs.isEmpty then .enum enumName else .enumInst enumName typeArgs
        return instrs.push (.call #[srcName decl.fvarId]
          (.packVariant enumName variant typeArgs) (vars.map srcName))
      if fn == ``fieldOfProjection then
        let projections := (← get).projections
        let some projection := vars.findSome? fun id => assocFind? id projections
          | throwError "field descriptor does not contain a structure projection"
        modify fun s => { s with descriptors := (decl.fvarId, projection) :: s.descriptors }
        return instrs
      if fn == ``borrowLocal || fn == ``borrowLocalMut then
        let some value := vars[0]? | throwError "local borrow is missing its value"
        let some valueType := types[0]? | throwError "local borrow is missing its value type"
        let some ty ← translateCurrentTy (← getEnv) valueType
          | throwError "cannot borrow a compiler-erased local"
        let resultTy := some <| if fn == ``borrowLocalMut then .mutRef ty else .ref ty
        modify fun s => { s with pending :=
          (decl.fvarId, .call { op := .borrowLoc, srcs := #[value], resultTy }) :: s.pending }
        return instrs
      if fn == ``borrowGlobal || fn == ``borrowGlobalMut then
        let some ownerExpr := types[0]?
          | throwError "global borrow is missing its resource type"
        let some owner := typeName? ownerExpr
          | throwError "global borrow is missing its resource type"
        let some address := beforeWorld? vars
          | throwError "global borrow is missing its address"
        addAcquire owner
        let some ownerTy ← translateCurrentTy (← getEnv) ownerExpr
          | throwError "global borrow has an erased resource type"
        let typeArgs := match ownerTy with
          | .structInst _ args => args
          | _ => #[]
        let resultTy := some <| if fn == ``borrowGlobalMut then .mutRef ownerTy else .ref ownerTy
        modify fun s => { s with pending :=
          (decl.fvarId, .call { op := .borrowGlobal owner typeArgs, srcs := #[address], resultTy }) :: s.pending }
        return instrs
      if fn == ``borrowField || fn == ``borrowFieldMut then
        let some owner := vars[0]? | throwError "field borrow is missing its reference"
        let some descriptor := vars[1]? | throwError "field borrow is missing its descriptor"
        let some field := assocFind? descriptor (← get).descriptors
          | throwError "field borrow uses an unknown descriptor"
        let some fieldType := types[1]?
          | throwError "field borrow is missing its field type"
        let some ty ← translateCurrentTy (← getEnv) fieldType
          | throwError "cannot borrow a compiler-erased field"
        let resultTy := some <| if fn == ``borrowFieldMut then .mutRef ty else .ref ty
        let typeArgs := match ← localTy? owner with
          | some (.ref (.structInst _ args)) | some (.mutRef (.structInst _ args)) => args
          | _ => #[]
        modify fun s => { s with pending :=
          (decl.fvarId, .call { op := .borrowField field typeArgs, srcs := #[owner], resultTy }) :: s.pending }
        return instrs
      if fn == ``freezeRef then
        let some ref := vars[0]? | throwError "implicit freeze is missing its mutable reference"
        let some valueType := types[0]? | throwError "implicit freeze is missing its referent type"
        let some ty ← translateCurrentTy (← getEnv) valueType
          | throwError "cannot freeze a reference to a compiler-erased type"
        addLocalTy decl.fvarId (.ref ty)
        return instrs.push (.call #[srcName decl.fvarId] .freezeRef #[srcName ref])
      if fn == ``borrowElem || fn == ``borrowElemMut then
        let some owner := vars[vars.size - 3]? | throwError "vector borrow is missing its reference"
        let some index := beforeWorld? vars | throwError "vector borrow is missing its index"
        let some elemType := types[0]? | throwError "vector borrow is missing its element type"
        let some ty ← translateCurrentTy (← getEnv) elemType
          | throwError "cannot borrow an element with compiler-erased type"
        let resultTy := some <| if fn == ``borrowElemMut then .mutRef ty else .ref ty
        modify fun s => { s with pending :=
          (decl.fvarId, .call { op := .borrowVecElem, srcs := #[owner, index], resultTy }) :: s.pending }
        return instrs
      if fn == ``freeze then
        let some ref := vars[0]? | throwError "freeze is missing its mutable reference"
        let some valueType := types[0]? | throwError "freeze is missing its referent type"
        let some ty ← translateCurrentTy (← getEnv) valueType
          | throwError "cannot freeze a reference to a compiler-erased type"
        let resultTy := some (.ref ty)
        modify fun s => { s with pending :=
          (decl.fvarId, .call { op := .freezeRef, srcs := #[ref], resultTy }) :: s.pending }
        return instrs
      if fn == ``read || fn == ``readImm then
        let some ref := vars[0]? | throwError "read is missing its reference"
        let some valueType := types[0]? | throwError "read is missing its result type"
        let resultTy ← translateCurrentTy (← getEnv) valueType
        modify fun s => { s with pending :=
          (decl.fvarId, .call { op := .readRef, srcs := #[ref], resultTy }) :: s.pending }
        return instrs
      if fn == ``write then
        let some ref := vars[0]? | throwError "write is missing its reference"
        let some value := vars[1]? | throwError "write is missing its value"
        modify fun s => { s with pending :=
          (decl.fvarId, .call { op := .writeRef, srcs := #[ref, value], resultTy := none }) :: s.pending }
        return instrs
      if fn == ``exists_ || fn == ``moveFrom then
        let some ownerExpr := types[0]?
          | throwError "global operation is missing its resource type"
        let some owner := typeName? ownerExpr
          | throwError "global operation is missing its resource type"
        let some address := beforeWorld? vars
          | throwError "global operation is missing its address"
        let some ownerTy ← translateCurrentTy (← getEnv) ownerExpr
          | throwError "global operation has an erased resource type"
        let typeArgs := match ownerTy with
          | .structInst _ args => args
          | _ => #[]
        addAcquire owner
        let resultTy := if fn == ``exists_ then some LIR.Ty.bool else some ownerTy
        let op := if fn == ``exists_ then LIR.Oper.exists_ owner typeArgs
          else .moveFrom owner typeArgs
        modify fun s => { s with pending :=
          (decl.fvarId, .call { op, srcs := #[address], resultTy }) :: s.pending }
        return instrs
      if fn == ``moveTo then
        let some ownerExpr := types[0]?
          | throwError "move_to is missing its resource type"
        let some owner := typeName? ownerExpr
          | throwError "move_to is missing its resource type"
        let some signer := vars[0]? | throwError "move_to is missing its signer"
        let some value := vars[1]? | throwError "move_to is missing its resource value"
        let some ownerTy ← translateCurrentTy (← getEnv) ownerExpr
          | throwError "move_to has an erased resource type"
        let typeArgs := match ownerTy with
          | .structInst _ args => args
          | _ => #[]
        addAcquire owner
        modify fun s => { s with pending :=
          (decl.fvarId, .call {
            op := .moveTo owner typeArgs
            srcs := #[signer, value]
            resultTy := none
          }) :: s.pending }
        return instrs
      if fn == ``abort then
        let some code := beforeWorld? vars | throwError "abort is missing its code"
        modify fun s => { s with pending := (decl.fvarId, .abort code) :: s.pending }
        return instrs
      if fn == ``Move.Vector.insert || fn == ``Move.MutRef.insert then
        let some reference := vars[vars.size - 4]?
          | throwError "vector insert is missing its mutable reference"
        let some index := vars[vars.size - 3]?
          | throwError "vector insert is missing its index"
        let some value := beforeWorld? vars
          | throwError "vector insert is missing its value"
        let some elemType := types[0]?
          | throwError "vector insert is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending :=
          (decl.fvarId, .vectorInsert reference index value elemTy) :: s.pending }
        return instrs
      if fn == ``Move.Vector.remove || fn == ``Move.MutRef.remove then
        let some reference := vars[vars.size - 3]?
          | throwError "vector remove is missing its mutable reference"
        let some index := beforeWorld? vars
          | throwError "vector remove is missing its index"
        let some elemType := types[0]?
          | throwError "vector remove is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending :=
          (decl.fvarId, .vectorRemove reference index elemTy) :: s.pending }
        return instrs
      if fn == ``Prod.mk then
        modify fun s => { s with products := (decl.fvarId, vars) :: s.products }
        let some returnedType := types[0]? | throwError "product constructor is missing its first type"
        match ← translateCurrentTy (← getEnv) returnedType with
        | none =>
            modify fun s => { s with returnAliases := (decl.fvarId, none) :: s.returnAliases }
        | some _ =>
            let some value := beforeWorld? vars
              | throwError "effect result is missing its returned value"
            modify fun s => { s with
              returnAliases := (decl.fvarId, some value) :: s.returnAliases }
        return instrs
      let arithmetic? : Option LIR.Oper :=
        if fn == ``U64.add then some .add else if fn == ``U64.sub then some .sub
        else if fn == ``U64.mul then some .mul else if fn == ``U64.div then some .div
        else if fn == ``U64.mod then some .mod else if fn == ``U64.less then some .lt
        else if fn == ``U64.lessEq then some .le else if fn == ``U64.equal then some .eq
        else if fn == ``Move.Compare.less then some .lt
        else if fn == ``Move.Compare.equal then some .eq
        else if fn == ``Move.Compare.genericDecidableLT then some .lt
        else if fn == ``U64.instDecidableLt then some .lt else none
      if let some op := arithmetic? then
        let some lhs := vars[0]? | throwError "binary operation is missing its left operand"
        let some rhs := vars[1]? | throwError "binary operation is missing its right operand"
        let ty := if op == .lt || op == .le || op == .eq then LIR.Ty.bool else .u64
        addLocalTy decl.fvarId ty
        return instrs.push (.call #[localName decl.fvarId] op #[srcName lhs, srcName rhs])
      if fn == ``U64.ofNat then
        let some source := vars[0]? | throwError "u64 literal is missing its natural value"
        let some n := assocFind? source (← get).natLiterals
          | throwError "u64 literal is not statically known"
        addLocalTy decl.fvarId .u64
        return instrs.push (.loadU64 (localName decl.fvarId) n)
      if fn == ``Bool.true || fn == ``Bool.false then
        addLocalTy decl.fvarId .bool
        return instrs.push (.loadBool (localName decl.fvarId) (fn == ``Bool.true))
      if let some n ← u64Constant? fn then
        addLocalTy decl.fvarId .u64
        return instrs.push (.loadU64 (localName decl.fvarId) n)
      if fn == ``Move.Vector.empty then
        let some elemType := types[0]? | throwError "empty vector is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        addLocalTy decl.fvarId (.vector elemTy)
        return instrs.push (.call #[srcName decl.fvarId] .vecPack #[])
      if fn == ``Move.Vector.push then
        let some vector := vars[0]? | throwError "vector push is missing its vector"
        let some value := vars[1]? | throwError "vector push is missing its value"
        let some elemType := types[0]? | throwError "vector push is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        addLocalTy decl.fvarId (.vector elemTy)
        return instrs.push (.call #[srcName decl.fvarId] .vecPush #[srcName vector, srcName value])
      if fn == ``Move.Vector.length || fn == ``Move.Ref.length ||
          fn == ``Move.MutRef.length then
        let some vector := vars[0]? | throwError "vector length is missing its vector"
        addLocalTy decl.fvarId .u64
        return instrs.push (.call #[srcName decl.fvarId] .vecLen #[srcName vector])
      if fn == ``Move.Vector.get then
        let runtimeVars := vars.extract (vars.size - 2) vars.size
        let some vector := runtimeVars[0]? | throwError "vector get is missing its vector"
        let some index := runtimeVars[1]? | throwError "vector get is missing its index"
        let some elemType := types[0]? | throwError "vector get is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        addLocalTy decl.fvarId elemTy
        return instrs.push (.call #[srcName decl.fvarId] .vecGet #[srcName vector, srcName index])
      if fn == ``Move.Vector.set then
        let some vector := vars[0]? | throwError "vector set is missing its vector"
        let some index := vars[1]? | throwError "vector set is missing its index"
        let some value := vars[2]? | throwError "vector set is missing its value"
        let some elemType := types[0]? | throwError "vector set is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        addLocalTy decl.fvarId (.vector elemTy)
        return instrs.push (.call #[srcName decl.fvarId] .vecSet
          #[srcName vector, srcName index, srcName value])
      let signature? ← match assocFind? fn signatures with
        | some signature => pure (some signature)
        | none => do
            let env ← getEnv
            let tagged := moveFunAttr.hasTag env fn || movePublicAttr.hasTag env fn ||
              moveEntryAttr.hasTag env fn
            if !tagged then
              pure none
            else
              let some owner := Move.moduleForDeclaration? env fn
                | throwError "Move function `{fn}` has no enclosing `move_module` identity"
              if owner == (← get).module then
                pure none
              else
                unless movePublicAttr.hasTag env fn || moveEntryAttr.hasTag env fn do
                  throwError "function `{fn}` is private to Move module `{owner.name}`"
                let externalDecl ← getBaseDecl fn
                match signatureOf env externalDecl with
                | .ok signature => pure (some signature)
                | .error message =>
                    throwError "while compiling the imported signature of `{fn}`: {message}"
      if let some signature := signature? then
        let candidates := if signature.isEffectful then
          vars.extract 0 (vars.size - 1)
        else
          vars
        -- LCNF applications still mention erased proof/type-class arguments.
        -- Keep only locals which acquired a Move runtime type.
        let mut callVars := #[]
        for candidate in candidates do
          if (← localTy? candidate).isSome then
            callVars := callVars.push candidate
        unless callVars.size == signature.params.size do
          throwError "call to `{fn}` has {callVars.size} runtime arguments; expected {signature.params.size}"
        let callTypeArgs ← translateTypeArgs (← getEnv) types signature.typeParams.size
        let returns := signature.returns.map (·.instantiate callTypeArgs)
        addCall fn
        if signature.isEffectful then
          modify fun s => { s with pending := (decl.fvarId, .call {
            op := .function fn callTypeArgs
            srcs := callVars
            resultTy := returns[0]?
          }) :: s.pending }
          return instrs
        match returns[0]? with
        | some ty =>
            addLocalTy decl.fvarId ty
            return instrs.push (.call #[srcName decl.fvarId]
              (.function fn callTypeArgs) (callVars.map srcName))
        | none =>
            modify fun s => { s with returnAliases := (decl.fvarId, none) :: s.returnAliases }
            return instrs.push (.call #[] (.function fn callTypeArgs) (callVars.map srcName))
      if moveFunAttr.hasTag (← getEnv) fn || movePublicAttr.hasTag (← getEnv) fn ||
          moveEntryAttr.hasTag (← getEnv) fn then
        throwError "callee `{fn}` is not selected in this `move_module%`"
      -- Type-class dictionaries and proof evidence are compiler-erased.
      if fn.toString.contains "instInhabited" then return instrs
      if fn == ``PUnit.unit || fn == ``Unit.unit then return instrs
      throwError "unsupported call `{fn}` while compiling Move function"
  | .proj owner field source =>
      let state ← get
      let resultTy ← match translateTyWith (← getEnv) state.tyContext decl.type with
        | .ok (some ty) => pure ty
        | .ok none => throwError "structure projection has an erased result type"
        | .error message => throwError message
      let typeArgs := match ← localTy? source with
        | some (.structInst _ args) => args
        | _ => #[]
      addLocalTy decl.fvarId resultTy
      return instrs.push (.call #[srcName decl.fvarId]
        (.getField owner field typeArgs) #[srcName source])
  | .erased => return instrs
  | _ => throwError "unsupported LCNF let binding in Move function"

private def trueAlternative? (alts : Array (Alt .pure)) : Option (Code .pure) :=
  alts.findSome? fun
    | .alt ctor _ body _ =>
        if ctor == ``Bool.true || ctor == ``Decidable.isTrue then some body else none
    | .default _ | .ctorAlt .. => none

private partial def collectLoopMarkerInputs : Code .pure → BuildM Unit
  | .let decl next => do
      match decl.value with
      | .lit (.nat n) =>
          modify fun s => { s with natLiterals := (decl.fvarId, n) :: s.natLiterals }
      | .const ``Prod.mk _ args _ =>
          modify fun s => { s with products := (decl.fvarId, fvarArgs args) :: s.products }
      | _ => pure ()
      collectLoopMarkerInputs next
  | .fun decl next _ | .jp decl next => do
      collectLoopMarkerInputs decl.value
      collectLoopMarkerInputs next
  | .cases cases =>
      for alt in cases.alts do collectLoopMarkerInputs alt.getCode
  | .return _ | .jmp .. | .unreach _ => pure ()

private partial def collectLoopStates : Code .pure → BuildM Unit
  | .let decl next => do
      if let .const fn _ args _ := decl.value then
        match ← loopMarker? fn (fvarArgs args) with
        | some (.enter label state) =>
            modify fun s => { s with loopStates := (label, state) :: s.loopStates }
        | some (.continue_ label state true) =>
            modify fun s => { s with loopTails := (label, (state, next)) :: s.loopTails }
        | _ => pure ()
      collectLoopStates next
  | .fun decl next _ | .jp decl next => do
      collectLoopStates decl.value
      collectLoopStates next
  | .cases cases =>
      for alt in cases.alts do collectLoopStates alt.getCode
  | .return _ | .jmp .. | .unreach _ => pure ()

private partial def collectJoinParams (env : Environment) : Code .pure → BuildM Unit
  | .let _ next => collectJoinParams env next
  | .fun decl next _ => do
      collectJoinParams env decl.value
      collectJoinParams env next
  | .jp decl next => do
      for param in decl.params do
        unless param.type.isConstOf ``Nat &&
            param.binderName.toString.contains "loopTok" do
          addLocal env param.fvarId param.type
      collectJoinParams env decl.value
      collectJoinParams env next
  | .cases cases =>
      for alt in cases.alts do collectJoinParams env alt.getCode
  | .return _ | .jmp .. | .unreach _ => pure ()

private partial def walk (env : Environment) (signatures : FunSignatures) (self : Name)
    (signature : FunSignature) (code : Code .pure) (blockName : String)
    (instrs : Array LIR.Instr) : BuildM Unit := do
  match code with
  | .let decl next =>
      match continuedCall? decl next with
      | some (callee, vars) =>
          unless callee == self do
            throwError "`continue` in `{self}` calls `{callee}`; only a direct self-call can become a loop"
          emitTailCall signature blockName instrs vars
      | none =>
          match decl.value with
          | .const fn _ args _ =>
              match ← loopMarker? fn (fvarArgs args) with
              | some (.enter label state) =>
                  modify fun s => { s with loopStates := (label, state) :: s.loopStates }
                  discard <| loopExit label
                  if let some header := assocFind? label (← get).loopHeaders then
                    if instrs.isEmpty && blockName == header then
                      walk env signatures self signature next blockName instrs
                    else
                      emitBlock blockName instrs (.jump header)
                      walk env signatures self signature next header #[]
                  else if instrs.isEmpty then
                    modify fun s =>
                      { s with loopHeaders := (label, blockName) :: s.loopHeaders }
                    walk env signatures self signature next blockName instrs
                  else
                    let header ← loopHeader label
                    emitBlock blockName instrs (.jump header)
                    walk env signatures self signature next header #[]
              | some (.continue_ label state tail) =>
                  emitLoopContinue label state blockName instrs
                  if tail && !(← get).builtLoopExits.contains label then
                    modify fun s => { s with builtLoopExits := label :: s.builtLoopExits }
                    let some headerState := assocFind? label (← get).loopStates
                      | throwError "loop {label} has no header state"
                    let exitInstrs ← assignLoopState label state headerState #[]
                    walk env signatures self signature next (← loopExit label) exitInstrs
              | some (.break_ label state) =>
                  emitLoopBreak label state blockName instrs
                  if !(← get).builtLoopExits.contains label then
                    let some (tailState, tailNext) := assocFind? label (← get).loopTails
                      | throwError "loop {label} has no retained tail continuation"
                    modify fun s => { s with builtLoopExits := label :: s.builtLoopExits }
                    let some headerState := assocFind? label (← get).loopStates
                      | throwError "loop {label} has no header state"
                    let exitInstrs ← assignLoopState label tailState headerState #[]
                    walk env signatures self signature tailNext
                      (← loopExit label) exitInstrs
              | none =>
                  walk env signatures self signature next blockName
                    (← recognizeLet signatures decl instrs)
          | _ =>
              walk env signatures self signature next blockName
                (← recognizeLet signatures decl instrs)
  | .fun decl next _ =>
      if let some field := projectionIndex? decl.value then
        modify fun s => { s with projections := (decl.fvarId, field) :: s.projections }
        walk env signatures self signature next blockName instrs
      else
        throwError "captured local function `{decl.binderName}` is outside the Move subset"
  | .jp decl next =>
      let joinName ← freshBlock "join"
      let joinParams := decl.params.map (·.fvarId)
      modify fun s => { s with joins := (decl.fvarId, (joinName, joinParams)) :: s.joins }
      for param in decl.params do
        unless param.type.isConstOf ``Nat &&
            param.binderName.toString.contains "loopTok" do
          addLocal env param.fvarId param.type
      walk env signatures self signature decl.value joinName #[]
      walk env signatures self signature next blockName instrs
  | .jmp target args =>
      let some (destination, params) := assocFind? target (← get).joins
        | throwError "jump to unknown LCNF join point"
      emitJoinJump blockName instrs destination params args
  | .return result =>
      match assocFind? result (← get).returnAliases with
      | some (some value) => emitBlock blockName instrs (.ret #[srcName value])
      | some none => emitBlock blockName instrs (.ret #[])
      | none =>
          match assocFind? result (← get).pending with
          | some (.abort code) => emitBlock blockName instrs (.abort (srcName code))
          | some (.call pending) =>
              match pending.resultTy with
              | some ty =>
                  addLocalTy result ty
                  emitBlock blockName
                    (instrs.push (.call #[srcName result] pending.op (pending.srcs.map srcName)))
                    (.ret #[srcName result])
              | none =>
                  emitBlock blockName
                    (instrs.push (.call #[] pending.op (pending.srcs.map srcName))) (.ret #[])
          | some pending@(.vectorInsert ..) =>
              emitBlock blockName
                (← materializePending pending #[] instrs) (.ret #[])
          | some pending@(.vectorRemove ..) =>
              let .vectorRemove _ _ elemTy := pending | unreachable!
              addLocalTy result elemTy
              emitBlock blockName
                (← materializePending pending #[srcName result] instrs)
                (.ret #[srcName result])
          | none =>
              let returns := if (← get).localIds.any (· == result) then #[srcName result] else #[]
              emitBlock blockName instrs (.ret returns)
  | .cases cases =>
      if (← get).loopLiveResults.contains cases.discr then
        let some body := trueAlternative? cases.alts
          | throwError "loop liveness test has no true branch"
        walk env signatures self signature body blockName instrs
      else match assocFind? cases.discr (← get).pending with
      | some (.abort code) => emitBlock blockName instrs (.abort (srcName code))
      | some (.call pending) =>
          let some alt := cases.alts[0]? | throwError "effect result has no product case"
          match alt with
          | .alt ``Prod.mk params body _ =>
              let mut nextInstrs := instrs
              let mut dsts : Array String := #[]
              if let some valueParam := params[0]? then
                match ← translateCurrentTy env valueParam.type with
                | some _ =>
                    addLocal env valueParam.fvarId valueParam.type
                    dsts := #[srcName valueParam.fvarId]
                | none => pure ()
              nextInstrs := nextInstrs.push (.call dsts pending.op (pending.srcs.map srcName))
              walk env signatures self signature body blockName nextInstrs
          | _ => throwError "Move effect did not normalize to a `Prod.mk` case"
      | some pending@(.vectorInsert ..) | some pending@(.vectorRemove ..) =>
          let some alt := cases.alts[0]? | throwError "effect result has no product case"
          match alt with
          | .alt ``Prod.mk params body _ =>
              let mut dsts : Array String := #[]
              if let some valueParam := params[0]? then
                match ← translateCurrentTy env valueParam.type with
                | some _ =>
                    addLocal env valueParam.fvarId valueParam.type
                    dsts := #[srcName valueParam.fvarId]
                | none => pure ()
              let nextInstrs ← materializePending pending dsts instrs
              walk env signatures self signature body blockName nextInstrs
          | _ => throwError "Move effect did not normalize to a `Prod.mk` case"
      | none =>
          if cases.typeName == ``Unit || cases.typeName == ``PUnit then
            let some alt := cases.alts[0]?
              | throwError "unit case has no alternative"
            walk env signatures self signature alt.getCode blockName instrs
            return
          if moveEnumAttr.hasTag env cases.typeName then
            let enumTypeArgs := match ← localTy? cases.discr with
              | some (.enumInst _ args) => args
              | _ => #[]
            let compileAlt (alt : Alt .pure) (name : String) : BuildM Unit := do
              match alt with
              | .alt ctor params body _ =>
                  let some (enumName, variant, _, _) := enumConstructor? env ctor
                    | throwError "case constructor `{ctor}` is not a selected Move enum variant"
                  unless enumName == cases.typeName do
                    throwError "case constructor `{ctor}` does not belong to enum `{cases.typeName}`"
                  let mut dsts := #[]
                  for param in params do
                    match ← translateCurrentTy env param.type with
                    | none => pure ()
                    | some _ =>
                        addLocal env param.fvarId param.type
                        dsts := dsts.push (srcName param.fvarId)
                  walk env signatures self signature body name
                    #[.call dsts (.unpackVariant enumName variant enumTypeArgs) #[srcName cases.discr]]
              | .default body => walk env signatures self signature body name #[]
              | .ctorAlt _ _ _ => throwError "unexpected impure constructor alternative"
            let rec dispatch (alts : List (Alt .pure)) (name : String)
                (dispatchInstrs : Array LIR.Instr) : BuildM Unit := do
              match alts with
              | [] => throwError "Move enum match has no alternatives"
              | [alt] =>
                  let bodyName ← freshBlock "variant"
                  emitBlock name dispatchInstrs (.jump bodyName)
                  compileAlt alt bodyName
              | alt :: rest =>
                  let (ctor, variant) ← match alt with
                    | .alt ctor _ _ _ =>
                        let some (enumName, variant, _, _) := enumConstructor? env ctor
                          | throwError "case constructor `{ctor}` is not a Move enum variant"
                        unless enumName == cases.typeName do
                          throwError "case constructor `{ctor}` does not belong to enum `{cases.typeName}`"
                        pure (ctor, variant)
                    | .default _ => throwError "default enum alternative must be last"
                    | .ctorAlt _ _ _ => throwError "unexpected impure constructor alternative"
                  let bodyName ← freshBlock s!"variant.{ctor.getString!}"
                  let nextName ← freshBlock "variant.test"
                  let test ← freshTemp .bool
                  emitBlock name
                    (dispatchInstrs.push (.call #[test]
                      (.testVariant cases.typeName variant enumTypeArgs) #[srcName cases.discr]))
                    (.branch test bodyName nextName)
                  compileAlt alt bodyName
                  dispatch rest nextName #[]
            dispatch cases.alts.toList blockName instrs
            return
          if moveStructAttr.hasTag env cases.typeName then
            let structTypeArgs := match ← localTy? cases.discr with
              | some (.structInst _ args) => args
              | _ => #[]
            unless cases.alts.size == 1 do
              throwError "Move structure destructuring expects exactly one constructor"
            let alt := cases.alts[0]!
            match alt with
            | .alt ctor params body _ =>
                let some (structName, _, _) := structConstructor? env ctor
                  | throwError "case constructor `{ctor}` is not a selected Move structure"
                unless structName == cases.typeName do
                  throwError "case constructor `{ctor}` does not belong to structure `{cases.typeName}`"
                let mut dsts := #[]
                for param in params do
                  match ← translateCurrentTy env param.type with
                  | none => pure ()
                  | some _ =>
                      addLocal env param.fvarId param.type
                      dsts := dsts.push (srcName param.fvarId)
                walk env signatures self signature body blockName
                  (instrs.push (.call dsts (.unpack structName structTypeArgs)
                    #[srcName cases.discr]))
                return
            | _ => throwError "unsupported Move structure destructuring alternative"
          let thenName ← freshBlock "then"
          let elseName ← freshBlock "else"
          emitBlock blockName instrs (.branch (srcName cases.discr) thenName elseName)
          for alt in cases.alts do
            match alt with
            | .alt ctor _ body _ =>
                if ctor == ``Decidable.isTrue || ctor == ``Bool.true then
                  walk env signatures self signature body thenName #[]
                else if ctor == ``Decidable.isFalse || ctor == ``Bool.false then
                  walk env signatures self signature body elseName #[]
                else
                  throwError "only Boolean cases are supported in Move functions"
            | .default body => walk env signatures self signature body elseName #[]
            | _ => throwError "unsupported case alternative in Move function"
  | .unreach _ => emitBlock blockName instrs (.ret #[])

private def visibility (env : Environment) (name : Name) : LIR.Visibility :=
  if moveEntryAttr.hasTag env name then .entry
  else if movePublicAttr.hasTag env name then .public_
  else .private_

private def lirSuccessors : LIR.Terminator → Array String
  | .jump block => #[block]
  | .branch _ thenBlock elseBlock => #[thenBlock, elseBlock]
  | .ret _ | .abort _ => #[]

private partial def reversePostorder (blocks : Array LIR.Block)
    (entry : String) : Array LIR.Block :=
  let (_, postorder) := visit entry [] #[]
  let reachable := postorder.toList.map (·.name)
  postorder.reverse ++ blocks.filter fun block => !reachable.contains block.name
where
  visit (name : String) (visited : List String) (postorder : Array LIR.Block) :
      List String × Array LIR.Block :=
    if visited.contains name then
      (visited, postorder)
    else
      match blocks.find? (·.name == name) with
      | none => (name :: visited, postorder)
      | some block =>
          let (visited, postorder) :=
            (lirSuccessors block.term).foldl (init := (name :: visited, postorder))
              fun (visited, postorder) successor =>
                visit successor visited postorder
          (visited, postorder.push block)

private def compileFun (env : Environment) (signatures : FunSignatures)
    (module : Move.ModuleRef) (name : Name) : CoreM LIR.FunDecl := do
  unless moveFunAttr.hasTag env name || movePublicAttr.hasTag env name || moveEntryAttr.hasTag env name do
    throwError "`{name}` is not annotated as a Move function"
  let decl ← getBaseDecl name
  let .code code := decl.value | throwError "Move function `{name}` has no body"
  let signature ← match signatureOf env decl with
    | .ok signature => pure signature
    | .error message => throwError message
  let mut params := #[]
  let mut paramIds := []
  let (tyContext, _) ← match functionContext env decl with
    | .ok context => pure context
    | .error message => throwError message
  for param in decl.params do
    match translateTyWith env tyContext param.type with
    | .error message => throwError message
    | .ok none => pure ()
    | .ok (some ty) =>
        params := params.push { name := localName param.fvarId, ty := ty }
        paramIds := param.fvarId :: paramIds
  let initial : BuildState := {
    module, tyContext, localIds := paramIds, loopParams := params
  }
  let (_, initial) ← (collectLoopMarkerInputs code).run initial
  let (_, initial) ← (collectLoopStates code).run initial
  let (_, initial) ← (collectJoinParams env code).run initial
  let (_, state) ← (walk env signatures name signature code "entry" #[]).run initial
  let blocks := reversePostorder state.blocks "entry"
  return {
    leanName := name
    moveName := name.getString!
    typeParams := signature.typeParams
    visibility := visibility env name
    params := params
    returns := signature.returns
    locals := state.locals
    blocks := blocks
    calls := state.calls
    acquires := state.acquires
  }

private def addNames (names additions : Array Name) : Array Name :=
  additions.foldl (fun names name => if names.any (· == name) then names else names.push name) names

private def propagateAcquires (functions : Array LIR.FunDecl) : Array LIR.FunDecl :=
  let rec loop : Nat → Array LIR.FunDecl → Array LIR.FunDecl
    | 0, current => current
    | fuel + 1, current =>
        let next := current.map fun decl =>
          let inherited := decl.calls.foldl (fun names callee =>
            match current.find? (·.leanName == callee) with
            | some calleeDecl => addNames names calleeDecl.acquires
            | none => names) decl.acquires
          { decl with acquires := inherited }
        loop fuel next
  loop functions.size functions

private partial def typeDependencies : LIR.Ty → Array Name
  | .struct name => #[name]
  | .enum name => #[name]
  | .structInst name args | .enumInst name args =>
      args.foldl (fun names arg => addNames names (typeDependencies arg)) #[name]
  | .vector elem | .ref elem | .mutRef elem => typeDependencies elem
  | _ => #[]

private def recursiveStruct? (structs : Array LIR.StructDecl) : Option Name :=
  let direct := structs.map fun decl =>
    let dependencies := decl.fields.foldl
      (fun names field => addNames names (typeDependencies field.ty)) #[]
    let dependencies := decl.variants.getD #[] |>.foldl (fun names variant =>
      variant.fields.foldl
        (fun names field => addNames names (typeDependencies field.ty)) names) dependencies
    (decl.leanName, dependencies)
  let rec close : Nat → Array (Name × Array Name) → Array (Name × Array Name)
    | 0, current => current
    | fuel + 1, current =>
        let next := current.map fun (name, dependencies) =>
          let reachable := dependencies.foldl (fun reachable dependency =>
            match current.find? (·.1 == dependency) with
            | some (_, dependencyDependencies) => addNames reachable dependencyDependencies
            | none => reachable) dependencies
          (name, reachable)
        close fuel next
  (close structs.size direct).findSome? fun (name, dependencies) =>
    if dependencies.any (· == name) then some name else none

private inductive RequiredAbility where
  | copy | drop | store | key
  deriving BEq

private def RequiredAbility.display : RequiredAbility → String
  | .copy => "Copy"
  | .drop => "Drop"
  | .store => "Store"
  | .key => "Key"

private def abilitySetHas : LIR.AbilitySet → RequiredAbility → Bool
  | abilities, .copy => abilities.copy
  | abilities, .drop => abilities.drop
  | abilities, .store => abilities.store
  | abilities, .key => abilities.key

private partial def typeHasAbility (structs : Array LIR.StructDecl)
    (typeParams : Array LIR.TypeParamDecl) (fuel : Nat)
    (required : RequiredAbility) : LIR.Ty → Bool
  | .bool | .u64 | .address => required != .key
  | .signer => required == .drop
  | .typeParam index =>
      typeParams[index]?.any fun param => abilitySetHas param.abilities required
  | .vector elem =>
      required != .key && typeHasAbility structs typeParams fuel required elem
  | .ref _ => required == .copy || required == .drop
  | .mutRef _ => required == .drop
  | .struct name | .enum name =>
      structs.find? (·.leanName == name) |>.any fun decl =>
        decl.typeParams.isEmpty && abilitySetHas decl.abilities required
  | .structInst name args | .enumInst name args =>
      if fuel == 0 then false else
      structs.find? (·.leanName == name) |>.any fun decl =>
        abilitySetHas decl.abilities required && decl.typeParams.size == args.size &&
          (decl.typeParams.zip args).all fun (param, arg) =>
            (!param.abilities.copy ||
              typeHasAbility structs typeParams (fuel - 1) .copy arg) &&
            (!param.abilities.drop ||
              typeHasAbility structs typeParams (fuel - 1) .drop arg) &&
            (!param.abilities.store ||
              typeHasAbility structs typeParams (fuel - 1) .store arg) &&
            (!param.abilities.key ||
              typeHasAbility structs typeParams (fuel - 1) .key arg)

private def validateFieldAbility (structs : Array LIR.StructDecl)
    (decl : LIR.StructDecl) (declared required : RequiredAbility)
    (field : LIR.FieldDecl) : Except String Unit := do
  unless typeHasAbility structs decl.typeParams (structs.size + 1) required field.ty do
    throw s!"Move type `{decl.moveName}` derives `{declared.display}`, but field \
      `{field.moveName}` does not have the required `{required.display}` ability"

private def validateAbilities (structs : Array LIR.StructDecl) : Except String Unit := do
  for decl in structs do
    let fields := decl.variants.getD #[] |>.foldl
      (fun fields variant => fields ++ variant.fields) decl.fields
    for field in fields do
      if decl.abilities.copy then
        validateFieldAbility structs decl .copy .copy field
      if decl.abilities.drop then
        validateFieldAbility structs decl .drop .drop field
      if decl.abilities.store then
        validateFieldAbility structs decl .store .store field
      if decl.abilities.key then
        validateFieldAbility structs decl .key .store field

/-- Compile selected attributed Lean declarations to Leaner's named LIR. -/
def compileModule (moduleName : String) (structNames funNames : Array Name) : CoreM LIR.Module := do
  -- Compile the selected declarations as one request.  This also avoids
  -- duplicating Lean's impure code-generation index bookkeeping when several
  -- bodies are extracted while elaborating one module.
  let mut missing := #[]
  for name in funNames do
    if (← Lean.Compiler.LCNF.getBaseDecl? name).isNone then
      missing := missing.push name
  unless missing.isEmpty do
    Lean.compileDecls missing
  let env ← getEnv
  let representative? := funNames[0]? <|> structNames[0]?
  let module := representative?.bind (Move.moduleForDeclaration? env) |>.getD {
    name := moduleName
  }
  let mut signatures : FunSignatures := []
  for name in funNames do
    unless moveFunAttr.hasTag env name || movePublicAttr.hasTag env name || moveEntryAttr.hasTag env name do
      throwError "`{name}` is not annotated as a Move function"
    let decl ← getBaseDecl name
    let signature ← match signatureOf env decl with
      | .ok signature => pure signature
      | .error message => throwError "while compiling the signature of `{name}`: {message}"
    signatures := (name, signature) :: signatures
  let mut structs := #[]
  for name in structNames do
    let compiled := if moveEnumAttr.hasTag env name then compileEnum env name else compileStruct env name
    match compiled with
    | .ok decl => structs := structs.push decl
    | .error message => throwError message
  if let some name := recursiveStruct? structs then
    throwError "recursive Move type `{name}` is not supported by the prototype"
  match validateAbilities structs with
  | .ok () => pure ()
  | .error message => throwError message
  let mut functions := #[]
  for name in funNames do
    try
      functions := functions.push (← compileFun env signatures module name)
    catch error =>
      throwError "while compiling Move function `{name}`:\n{error.toMessageData}"
  functions := propagateAcquires functions
  let mut externalFuns : Array LIR.ExternalFunRef := #[]
  for function in functions do
    for callee in function.calls do
      unless funNames.any (· == callee) || externalFuns.any (·.leanName == callee) do
        let some owner := Move.moduleForDeclaration? env callee
          | throwError "called Move function `{callee}` has no enclosing module identity"
        if owner == module then
          throwError "callee `{callee}` is not selected in this `move_module%`"
        externalFuns := externalFuns.push {
          leanName := callee
          address := owner.address
          moduleName := owner.name
          functionName := callee.getString!
        }
  return {
    address := module.address
    name := moduleName
    structs := structs
    functions := functions
    externalFuns := externalFuns
  }

end Move.Compiler
