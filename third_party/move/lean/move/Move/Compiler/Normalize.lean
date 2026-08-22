-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Attributes
import Move.Basic
import Move.Action
import Move.Compiler.LIR
import Move.Compiler.LCNF
import Move.Syntax

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

/-- Recognize a width-tag type name. -/
private def widthOfTagName? (name : Name) : Option MoveModel.IR.IntWidth :=
  if name == ``Move.W8 then some .w8
  else if name == ``Move.W16 then some .w16
  else if name == ``Move.W32 then some .w32
  else if name == ``Move.W64 then some .w64
  else if name == ``Move.W128 then some .w128
  else if name == ``Move.W256 then some .w256
  else none

/-- Recognize a width-tag type expression. -/
private def widthOfExpr? (type : Expr) : Option MoveModel.IR.IntWidth :=
  match type with
  | .const name _ => widthOfTagName? name
  | _ => none

/-- Recognize a sign-tag type expression: the second half of a `NumType`. -/
private def signOfExpr? (type : Expr) : Option Bool :=
  match type with
  | .const name _ =>
      if name == ``Move.Unsigned then some false
      else if name == ``Move.Signed then some true
      else none
  | _ => none

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
  if type.isConstOf ``U8 then return some (.uint .w8)
  if type.isConstOf ``U16 then return some (.uint .w16)
  if type.isConstOf ``U32 then return some (.uint .w32)
  if type.isConstOf ``U64 then return some (.uint .w64)
  if type.isConstOf ``U128 then return some (.uint .w128)
  if type.isConstOf ``U256 then return some (.uint .w256)
  if type.isConstOf ``I8 then return some (.sint .w8)
  if type.isConstOf ``I16 then return some (.sint .w16)
  if type.isConstOf ``I32 then return some (.sint .w32)
  if type.isConstOf ``I64 then return some (.sint .w64)
  if type.isConstOf ``I128 then return some (.sint .w128)
  if type.isConstOf ``I256 then return some (.sint .w256)
  -- The general form: `MoveInt S W`, carrying the sign tag then the width tag.
  if type.isAppOf ``Move.MoveInt then
    let args := type.getAppArgs
    let some signTag := args[0]?
      | throw s!"integer type `{type}` is missing its sign tag"
    let some signed := signOfExpr? signTag
      | throw s!"integer signedness of `{type}` is not statically known"
    let some widthTag := args[1]?
      | throw s!"integer type `{type}` is missing its width tag"
    let some w := widthOfExpr? widthTag
      | throw s!"integer width of `{type}` is not statically known"
    return some (.int ⟨w, signed⟩)
  if type.isAppOf ``Move.Width then return none
  if type.isAppOf ``Move.Sign then return none
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

/-- Whether a declaration's type ends in `Prop`. -/
private partial def resultIsProp : Expr → Bool
  | .forallE _ _ body _ => resultIsProp body
  | .sort level => level.isZero
  | _ => false

/-- Whether a structure field carries a data invariant rather than data: its
type is a proposition, so LCNF erases it and Move never sees it. -/
private partial def isDataInvariantField (env : Environment) (type : Expr) : Bool :=
  match type.cleanupAnnotations with
  | .forallE _ _ body _ => isDataInvariantField env body
  | result =>
      match result.getAppFn with
      | .const name _ =>
          match env.find? name with
          | some info => resultIsProp info.type
          | none => false
      | .sort level => level.isZero
      | _ => false

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
  | "U8" => some (.uint .w8)
  | "U16" => some (.uint .w16)
  | "U32" => some (.uint .w32)
  | "U64" => some (.uint .w64)
  | "U128" => some (.uint .w128)
  | "U256" => some (.uint .w256)
  | "I8" => some (.sint .w8)
  | "I16" => some (.sint .w16)
  | "I32" => some (.sint .w32)
  | "I64" => some (.sint .w64)
  | "I128" => some (.sint .w128)
  | "I256" => some (.sint .w256)
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
            for _ in [:index + 1 - typeParams.size] do
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

private partial def returnTypes (env : Environment) (ctx : TyContext)
    (type : Expr) : Except String (Array LIR.Ty) := do
  if type.isAppOfArity ``Prod 2 then
    let args := type.getAppArgs
    return (← returnTypes env ctx args[0]!) ++ (← returnTypes env ctx args[1]!)
  match translateTyWith env ctx type with
  | .ok (some ty) => return #[ty]
  | .ok none => return #[]
  | .error message => throw message

/-- Move multiple returns are represented by transient Lean products.  The
outer product of `StateM World` is the sequencing implementation of
`Action`, while products inside its value are flattened to the IR return
array just like a pure product-valued function. -/
private def functionShape (env : Environment) (ctx : TyContext)
    (type : Expr) : Except String (Bool × Array LIR.Ty) := do
  let type := resultType type
  if type.isAppOfArity ``Prod 2 then
    let args := type.getAppArgs
    if args[1]!.isConstOf ``World then
      return (true, ← returnTypes env ctx args[0]!)
  return (false, ← returnTypes env ctx type)

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

/-- The bounds a `(T : Store, Copy)` group declares, as an `AbilitySet`. -/
private def declaredBounds (declared : Array Name) : LIR.AbilitySet :=
  { copy := declared.contains `Copy
    drop := declared.contains `Drop
    store := declared.contains `Store || declared.contains `Key
    key := declared.contains `Key }

/-- A parameter's ability bounds.  Declared bounds win; otherwise they are
inferred from the container's own abilities, which is all Move can conclude
when the source does not say. -/
private def inferredTypeParams (names : Array Name) (abilities : LIR.AbilitySet)
    (fieldTypes : Array LIR.Ty)
    (bounds : Array (String × Array Name) := #[]) : Array LIR.TypeParamDecl :=
  names.mapIdx fun index name =>
    let phantom := !fieldTypes.any (tyUsesParam index)
    let declared? := bounds.find? (·.1 == name.toString) |>.map (·.2)
    let constraints := match declared? with
      | some declared => declaredBounds declared
      | none =>
        if phantom then {} else
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
    if isDataInvariantField env constInfo.type then
      continue
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
      ((Move.typeParamBounds? env name).getD #[])
    abilities := abilities
    fields := fields
    attributes := Move.userAttributes env name
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
      -- The proof of a data invariant rides in the constructor but is
      -- erased before Move sees the variant.
      if isDataInvariantField env fieldType then continue
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
      ((Move.typeParamBounds? env name).getD #[])
    abilities := abilities
    fields := #[]
    variants := some variants
    attributes := Move.userAttributes env name
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

/-- Runtime fields of any Move constructor: its declared fields minus the
proofs of data invariants, which LCNF erases. -/
private def runtimeCtorFields (env : Environment) (ctor : ConstructorVal) : Nat :=
  let binders := forallBinders ctor.type |>.drop ctor.numParams |>.take ctor.numFields
  binders.foldl (init := ctor.numFields) fun count (_, type) =>
    if isDataInvariantField env type then count - 1 else count

private def enumConstructor? (env : Environment) (name : Name) : Option (Name × Nat × Nat × Nat) :=
  match env.find? name with
  | some (.ctorInfo ctor) =>
      if moveEnumAttr.hasTag env ctor.induct then
        some (ctor.induct, ctor.cidx, ctor.numParams, runtimeCtorFields env ctor)
      else none
  | _ => none

/-- Runtime fields of a Move constructor: LCNF erases the proof carried by a
certified value, so a data-invariant field is not passed to `pack`. -/
private def runtimeFields (env : Environment) (induct : Name) (declared : Nat) :
    Nat :=
  match getStructureInfo? env induct with
  | none => declared
  | some info =>
      info.fieldNames.foldl (init := declared) fun count fieldName =>
        match env.find? (induct ++ fieldName) with
        | some constInfo =>
            if isDataInvariantField env constInfo.type then count - 1 else count
        | none => count

private def structConstructor? (env : Environment) (name : Name) : Option (Name × Nat × Nat) :=
  match env.find? name with
  | some (.ctorInfo ctor) =>
      if moveStructAttr.hasTag env ctor.induct then
        some (ctor.induct, ctor.numParams,
          runtimeFields env ctor.induct ctor.numFields)
      else none
  | _ => none

private partial def intConstant? (name : Name) :
    CoreM (Option (MoveModel.IR.NumType × Int)) := do
  let decl ← match (← Lean.Compiler.LCNF.getBaseDecl? name) with
    | some decl => pure decl
    | none =>
        Lean.compileDecls #[name]
        let some decl ← Lean.Compiler.LCNF.getBaseDecl? name | return none
        pure decl
  let .code code := decl.value | return none
  let rec scan (nats : List (FVarId × Nat)) (ints : List (FVarId × Int))
      (results : List (FVarId × (MoveModel.IR.NumType × Int))) :
      Code .pure → CoreM (Option (MoveModel.IR.NumType × Int))
    | .let letDecl next =>
        match letDecl.value with
        | .lit (.nat n) =>
            scan ((letDecl.fvarId, n) :: nats) ((letDecl.fvarId, n) :: ints) results next
        | .const fn _ args _ =>
            -- `Int.ofNat` carries a non-negative literal into `MoveInt.ofInt`.
            if fn == ``Int.ofNat || fn == ``NatCast.natCast || fn == ``Nat.cast then
              match (fvarArgs args).back? with
              | some natArg =>
                  match assocFind? natArg nats with
                  | some n => scan nats ((letDecl.fvarId, n) :: ints) results next
                  | none => scan nats ints results next
              | none => scan nats ints results next
            else if fn == ``UInt.ofNat then
              match (typeArgs args)[0]?, (fvarArgs args).back? with
              | some tag, some natArg =>
                  match widthOfExpr? tag, assocFind? natArg nats with
                  | some w, some n =>
                      let nt : MoveModel.IR.NumType := ⟨w, false⟩
                      scan nats ints ((letDecl.fvarId, (nt, n)) :: results) next
                  | _, _ => pure none
              | _, _ => pure none
            else if fn == ``MoveInt.ofInt then
              match (typeArgs args)[0]?, (typeArgs args)[1]?, (fvarArgs args).back? with
              | some signTag, some widthTag, some intArg =>
                  match signOfExpr? signTag, widthOfExpr? widthTag, assocFind? intArg ints with
                  | some signed, some width, some value => do
                      let nt : MoveModel.IR.NumType := ⟨width, signed⟩
                      unless nt.lo ≤ value ∧ value < nt.hi do
                        throwError "integer constant `{name}` contains an out-of-range literal `{value}`"
                      scan nats ints ((letDecl.fvarId, (nt, value)) :: results) next
                  | _, _, _ => pure none
              | _, _, _ => pure none
            else if fn == ``MoveInt.neg then
              match (fvarArgs args).back? >>= fun arg => assocFind? arg results with
              | some (nt, value) => do
                  let result := -value
                  unless nt.lo ≤ result ∧ result < nt.hi do
                    throwError "integer constant `{name}` overflows in negation"
                  scan nats ints ((letDecl.fvarId, (nt, result)) :: results) next
              | none => pure none
            else if fn == ``MoveInt.cast then
              match (typeArgs args)[2]?, (typeArgs args)[3]?, (fvarArgs args).back? with
              | some signTag, some widthTag, some source =>
                  match signOfExpr? signTag, widthOfExpr? widthTag, assocFind? source results with
                  | some signed, some width, some (_, value) => do
                      let nt : MoveModel.IR.NumType := ⟨width, signed⟩
                      unless nt.lo ≤ value ∧ value < nt.hi do
                        throwError "integer constant `{name}` contains a cast whose result is out of range"
                      scan nats ints ((letDecl.fvarId, (nt, value)) :: results) next
                  | _, _, _ => pure none
              | _, _, _ => pure none
            else if fn == ``MoveInt.add || fn == ``MoveInt.sub || fn == ``MoveInt.mul ||
                fn == ``MoveInt.div || fn == ``MoveInt.mod || fn == ``MoveInt.land ||
                fn == ``MoveInt.lor || fn == ``MoveInt.lxor || fn == ``MoveInt.shl ||
                fn == ``MoveInt.shr then do
              let runtimeArgs := fvarArgs args
              match runtimeArgs[runtimeArgs.size - 2]?, runtimeArgs.back? with
              | some leftId, some rightId =>
                  match assocFind? leftId results, assocFind? rightId results with
                  | some (nt, left), some (rightNt, right) =>
                      unless (fn == ``MoveInt.shl || fn == ``MoveInt.shr || nt == rightNt) do
                        throwError "integer constant `{name}` combines operands of different types"
                      let result ←
                        if fn == ``MoveInt.add then pure (left + right)
                        else if fn == ``MoveInt.sub then pure (left - right)
                        else if fn == ``MoveInt.mul then pure (left * right)
                        else if fn == ``MoveInt.div then
                          if right == 0 then throwError "integer constant `{name}` divides by zero"
                          else pure (left.tdiv right)
                        else if fn == ``MoveInt.mod then
                          if right == 0 then throwError "integer constant `{name}` takes a remainder by zero"
                          else pure (left.tmod right)
                        else if fn == ``MoveInt.land then
                          pure (nt.fromBits ((nt.toBits left).toNat &&& (nt.toBits right).toNat))
                        else if fn == ``MoveInt.lor then
                          pure (nt.fromBits ((nt.toBits left).toNat ||| (nt.toBits right).toNat))
                        else if fn == ``MoveInt.lxor then
                          pure (nt.fromBits ((nt.toBits left).toNat ^^^ (nt.toBits right).toNat))
                        else do
                          let amount := right.toNat
                          unless amount < nt.width.bits do
                            throwError "integer constant `{name}` shifts by at least its integer width"
                          if fn == ``MoveInt.shl then
                            pure (nt.fromBits (((nt.toBits left).toNat <<< amount) % nt.size))
                          else
                            pure (left.fdiv (2 ^ amount))
                      unless nt.lo ≤ result ∧ result < nt.hi do
                        throwError "integer constant `{name}` overflows"
                      scan nats ints ((letDecl.fvarId, (nt, result)) :: results) next
                  | _, _ => pure none
              | _, _ => pure none
            else
              scan nats ints results next
        | _ => scan nats ints results next
    | .return result => pure (assocFind? result results)
    | _ => pure none
  scan [] [] [] code

private structure PendingCall where
  op : LIR.Oper
  srcs : Array FVarId
  resultTy : Option LIR.Ty

private inductive Pending where
  | call (call : PendingCall)
  | multiCall (op : LIR.Oper) (srcs : Array FVarId)
      (resultTys : Array LIR.Ty) (effectful : Bool)
  | abort (code : FVarId)
  | vectorInsert (reference index value : FVarId) (elemTy : LIR.Ty)
  | vectorRemove (reference index : FVarId) (elemTy : LIR.Ty)
  | vectorPop (reference : FVarId) (elemTy : LIR.Ty)
  | vectorMut (reference : FVarId) (args : Array FVarId) (elemTy : LIR.Ty)
      (op : LIR.Oper) (resultTys : Array LIR.Ty)

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
  strLiterals : List (FVarId × String) := []
  returnAliases : List (FVarId × Option FVarId) := []
  returnValues : List (FVarId × Array FVarId) := []
  joins : List (FVarId × (String × Array FVarId)) := []
  loopHeaders : List (Nat × String) := []
  loopExits : List (Nat × String) := []
  builtLoopExits : List Nat := []
  loopStates : List (Nat × Array FVarId) := []
  loopTails : List (Nat × (Array FVarId × Code .pure)) := []
  usedLoopMarkers : List (Name × Nat × Nat) := []
  products : List (FVarId × Array FVarId) := []
  tupleLocals : List (FVarId × Array (String × LIR.Ty)) := []
  loopLiveResults : List FVarId := []
  sourceMarkerResults : List FVarId := []
  blocks : Array LIR.Block := #[]
  calls : Array Name := #[]
  acquires : Array Name := #[]
  nextBlock : Nat := 0
  nextTemp : Nat := 0
  currentSpan : Option MoveModel.IR.SourceSpan := none
  currentSourceName : Option String := none
  sourceLocalStart : Nat := 0

private abbrev BuildM := StateT BuildState CoreM

/-- Assign an authored binding name to the last runtime local produced by its
statement. Intermediate LCNF values remain unnamed, and a binding optimized
to an existing value does not incorrectly rename that older local. -/
private def finishSourceBinding : BuildM Unit := do
  let state ← get
  let some sourceName := state.currentSourceName | return
  unless state.sourceLocalStart < state.locals.size do return
  let index := state.locals.size - 1
  let some decl := state.locals[index]? | return
  modify fun s => { s with
    locals := s.locals.set! index { decl with sourceName := some sourceName } }

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

private def setLocalSourceName (id : FVarId) (sourceName : String) : BuildM Unit := do
  let name := localName id
  let state ← get
  let some index := state.locals.findIdx? (·.name == name) | return
  let some decl := state.locals[index]? | return
  modify fun s => { s with
    locals := s.locals.set! index { decl with sourceName := some sourceName } }

/-- `sourceNamedValue` is transparent for compiler-only values which flatten
to several Move locals (or no Move local). Preserve the normalizer's transient
representation under the marker result so subsequent tuple elimination and
effect sequencing see the original value. -/
private def aliasTransientValue (target source : FVarId) : BuildM Unit := do
  let state ← get
  if let some value := assocFind? source state.pending then
    modify fun s => { s with pending := (target, value) :: s.pending }
  if let some value := assocFind? source state.products then
    modify fun s => { s with products := (target, value) :: s.products }
  if let some value := assocFind? source state.tupleLocals then
    modify fun s => { s with tupleLocals := (target, value) :: s.tupleLocals }
  if let some value := assocFind? source state.returnAliases then
    modify fun s => { s with returnAliases := (target, value) :: s.returnAliases }
  if let some value := assocFind? source state.returnValues then
    modify fun s => { s with returnValues := (target, value) :: s.returnValues }

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

/-- Runtime leaves of a transient product value.  Products never become Move
locals; only their recursively flattened components do. -/
private partial def flattenProductValue (id : FVarId) : BuildM (Array FVarId) := do
  if let some fields := assocFind? id (← get).products then
    let mut result := #[]
    for field in fields do
      result := result ++ (← flattenProductValue field)
    return result
  if (← localTy? id).isSome then return #[id]
  return #[]

private def currentReturnTypes (env : Environment) (type : Expr) :
    BuildM (Array LIR.Ty) := do
  match returnTypes env (← get).tyContext type with
  | .ok types => return types
  | .error message => throwError message

/-- Bind the constructor parameters of one transient product case to a flat
array of Move locals.  A nested product parameter retains its slice for the
next `cases`; leaf parameters become ordinary LIR locals. -/
private def bindTupleParams (env : Environment)
    (params : Array (Param .pure)) (sources : Array (String × LIR.Ty))
    (instrs : Array LIR.Instr) : BuildM (Array LIR.Instr) := do
  let mut offset := 0
  let mut nextInstrs := instrs
  for param in params do
    let types ← currentReturnTypes env param.type
    unless offset + types.size ≤ sources.size do
      throwError "tuple pattern has more runtime fields than its value"
    let fields := sources.extract offset (offset + types.size)
    offset := offset + types.size
    if types.size == 1 then
      let some source := fields[0]? | unreachable!
      let some expected := types[0]? | unreachable!
      unless source.2 == expected do
        throwError "tuple pattern field type does not match its returned value"
      addLocal env param.fvarId param.type
      nextInstrs := nextInstrs.push (.assign (localName param.fvarId) source.1)
    else if types.size > 1 then
      modify fun s => { s with
        tupleLocals := (param.fvarId, fields) :: s.tupleLocals }
  unless offset == sources.size do
    throwError "tuple pattern binds {offset} runtime fields; value has {sources.size}"
  return nextInstrs

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

private def freshResultLocals (types : Array LIR.Ty) :
    BuildM (Array (String × LIR.Ty)) := do
  types.mapM fun type => return (← freshTemp type, type)

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
  | .multiCall op srcs resultTys _ =>
      unless dsts.size == resultTys.size do
        throwError "multiple-return call produced {dsts.size} destinations; expected {resultTys.size}"
      return instrs.push (.call dsts op (srcs.map localName))
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
  | .vectorPop reference elemTy =>
      unless dsts.size == 1 do
        throwError "vector pop_back must produce exactly one removed element"
      let vectorTy := LIR.Ty.vector elemTy
      let old ← freshTemp vectorTy
      let updated ← freshTemp vectorTy
      return instrs
        |>.push (.call #[old] .readRef #[localName reference])
        |>.push (.call #[updated, dsts[0]!] .vecPop #[old])
        |>.push (.call #[] .writeRef #[localName reference, updated])
  | .vectorMut reference args elemTy op resultTys =>
      unless dsts.size == resultTys.size do
        throwError "vector operation produced {dsts.size} values; expected {resultTys.size}"
      let vectorTy := LIR.Ty.vector elemTy
      let old ← freshTemp vectorTy
      let updated ← freshTemp vectorTy
      return instrs
        |>.push (.call #[old] .readRef #[localName reference])
        |>.push (.call (#[updated] ++ dsts) op (#[old] ++ args.map localName))
        |>.push (.call #[] .writeRef #[localName reference, updated])

private def emitBlock (name : String) (instrs : Array LIR.Instr)
    (term : LIR.Terminator) : BuildM Unit := do
  let span := (← get).currentSpan
  let instrs := instrs.map (·.withSpan span)
  modify fun s => { s with
    blocks := s.blocks.push { name, instrs, term, termSpan := span } }

private def locateCurrent (instrs : Array LIR.Instr) : BuildM (Array LIR.Instr) := do
  let span := (← get).currentSpan
  return instrs.map (·.withSpan span)

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
  | enter (label nonce : Nat) (state : Array FVarId)
  | continue_ (label nonce : Nat) (state : Array FVarId) (tail : Bool)
  | break_ (label nonce : Nat) (state : Array FVarId)

private def loopMarker? (self fn : Name) (vars : Array FVarId) :
    BuildM (Option LoopMarker) := do
  let isMarker := Move.isLoopEnterMarker fn || Move.isLoopContinueMarker fn ||
    Move.isLoopContinueTailMarker fn || Move.isLoopBreakMarker fn
  unless isMarker do return none
  let literals ← loopMarkerNats vars
  let some label := literals[0]?
    | throwError "loop marker is missing its static label"
  let some nonce := literals[1]?
    | throwError "loop marker is missing its provenance nonce"
  let some arity := literals[2]?
    | throwError "loop marker is missing its state arity"
  unless Move.isRegisteredLoopMarker (← getEnv) self fn label nonce do
    throwError "unregistered compiler loop marker in `{self}`"
  let state ←
    if arity == 0 then pure #[]
    else
      let some packed := vars.back?
        | throwError "loop marker is missing its state"
      flattenLoopState arity packed
  if Move.isLoopEnterMarker fn then
    return some (.enter label nonce state)
  if Move.isLoopContinueMarker fn then
    return some (.continue_ label nonce state false)
  if Move.isLoopContinueTailMarker fn then
    return some (.continue_ label nonce state true)
  if Move.isLoopBreakMarker fn then
    return some (.break_ label nonce state)
  return none

private def claimLoopMarker (fn : Name) : LoopMarker → BuildM Unit
  | .enter label nonce _ | .continue_ label nonce .. | .break_ label nonce _ => do
      if (← get).usedLoopMarkers.contains (fn, label, nonce) then
        throwError "reused compiler loop marker in `{fn}`"
      modify fun s => { s with
        usedLoopMarkers := (fn, label, nonce) :: s.usedLoopMarkers }

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
  | .lit (.str value) =>
      modify fun s => { s with strLiterals := (decl.fvarId, value) :: s.strLiterals }
      return instrs
  | .const fn _ args _ =>
      let vars := fvarArgs args
      let types := typeArgs args
      if Move.isSourceNamedValue fn then
        let mut literals := #[]
        for id in vars do
          if let some n := assocFind? id (← get).natLiterals then
            literals := literals.push n
        let some start := literals[0]?
          | throwError "named source value is missing its start offset"
        let some stop := literals[1]?
          | throwError "named source value is missing its end offset"
        let strLiterals := (← get).strLiterals
        let some sourceName := vars.findSome? fun id => assocFind? id strLiterals
          | throwError "named source value is missing its local name"
        let some source := vars.back?
          | throwError "named source value is missing its wrapped value"
        unless start ≤ stop do
          throwError "named source value has a reversed range"
        if (← localTy? source).isSome then
          addLocal (← getEnv) decl.fvarId decl.type
          setLocalSourceName decl.fvarId sourceName
          let span : MoveModel.IR.SourceSpan := { start, «end» := stop }
          let instr : LIR.Instr := .assign (srcName decl.fvarId) (srcName source)
          return instrs.push (instr.withSpan (some span))
        aliasTransientValue decl.fvarId source
        return instrs
      if Move.isSourceSpanMarker fn then
        let mut literals := #[]
        for id in vars do
          if let some n := assocFind? id (← get).natLiterals then
            literals := literals.push n
        let some start := literals[0]?
          | throwError "source span marker is missing its start offset"
        let some stop := literals[1]?
          | throwError "source span marker is missing its end offset"
        unless start ≤ stop do
          throwError "source span marker has a reversed range"
        let strLiterals := (← get).strLiterals
        let sourceName := vars.findSome? fun id => assocFind? id strLiterals
        finishSourceBinding
        let previous := (← get).currentSpan
        modify fun s => { s with
          currentSpan := some { start, «end» := stop }
          currentSourceName := sourceName.bind fun name =>
            if name.isEmpty then none else some name
          sourceLocalStart := s.locals.size
          sourceMarkerResults := decl.fvarId :: s.sourceMarkerResults }
        return instrs.map (·.withSpan previous)
      -- `Int.ofNat n` / `Nat.cast n` is the carrier of a non-negative integer
      -- literal; track its value like a natural literal so `MoveInt.ofInt` can
      -- resolve it.
      if fn == ``Int.ofNat || fn == ``NatCast.natCast || fn == ``Nat.cast then
        match vars.back? with
        | some natArg =>
          match assocFind? natArg (← get).natLiterals with
          | some n =>
            modify fun s => { s with natLiterals := (decl.fvarId, n) :: s.natLiterals }
          | none => pure ()
        | none => pure ()
        return instrs
      if fn == ``continueMarker then
        throwError "`continue` must mark a direct self-call in tail position"
      if Move.isLoopTokenLiveMarker fn then
        modify fun s => { s with loopLiveResults := decl.fvarId :: s.loopLiveResults }
        return instrs
      if Move.isLoopTokenJoinMarker fn then
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
      if fn == ``existsAt || fn == ``moveFrom then
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
        if fn == ``moveFrom then
          addAcquire owner
        let resultTy := if fn == ``existsAt then some LIR.Ty.bool else some ownerTy
        let op := if fn == ``existsAt then LIR.Oper.existsAt owner typeArgs
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
        modify fun s => { s with pending :=
          (decl.fvarId, .call {
            op := .moveTo owner typeArgs
            srcs := #[signer, value]
            resultTy := none
          }) :: s.pending }
        return instrs
      if fn == `Move.abort then
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
      if fn == ``Move.Vector.popBack || fn == ``Move.MutRef.popBack then
        let some reference := beforeWorld? vars
          | throwError "vector pop_back is missing its mutable reference"
        let some elemType := types[0]?
          | throwError "vector pop_back is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending :=
          (decl.fvarId, .vectorPop reference elemTy) :: s.pending }
        return instrs
      if fn == ``Move.Vector.swap || fn == ``Move.MutRef.swap then
        let some reference := vars[vars.size - 4]? | throwError "vector swap is missing its reference"
        let some i := vars[vars.size - 3]? | throwError "vector swap is missing its first index"
        let some j := beforeWorld? vars | throwError "vector swap is missing its second index"
        let some elemType := types[0]? | throwError "vector swap is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending := (decl.fvarId,
          .vectorMut reference #[i, j] elemTy .vecSwap #[]) :: s.pending }
        return instrs
      if fn == ``Move.Vector.swapRemove || fn == ``Move.MutRef.swapRemove then
        let some reference := vars[vars.size - 3]? | throwError "vector swap_remove is missing its reference"
        let some i := beforeWorld? vars | throwError "vector swap_remove is missing its index"
        let some elemType := types[0]? | throwError "vector swap_remove is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending := (decl.fvarId,
          .vectorMut reference #[i] elemTy .vecSwapRemove #[elemTy]) :: s.pending }
        return instrs
      if fn == ``Move.Vector.append || fn == ``Move.MutRef.append then
        let some reference := vars[vars.size - 3]? | throwError "vector append is missing its reference"
        let some other := beforeWorld? vars | throwError "vector append is missing its source vector"
        let some elemType := types[0]? | throwError "vector append is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending := (decl.fvarId,
          .vectorMut reference #[other] elemTy .vecAppend #[]) :: s.pending }
        return instrs
      if fn == ``Move.Vector.reverse || fn == ``Move.MutRef.reverse then
        let some reference := beforeWorld? vars | throwError "vector reverse is missing its reference"
        let some elemType := types[0]? | throwError "vector reverse is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending := (decl.fvarId,
          .vectorMut reference #[] elemTy .vecReverse #[]) :: s.pending }
        return instrs
      if fn == ``Move.Vector.reverseSlice || fn == ``Move.MutRef.reverseSlice then
        let some reference := vars[vars.size - 4]? | throwError "vector reverse_slice is missing its reference"
        let some left := vars[vars.size - 3]? | throwError "vector reverse_slice is missing its left bound"
        let some right := beforeWorld? vars | throwError "vector reverse_slice is missing its right bound"
        let some elemType := types[0]? | throwError "vector reverse_slice is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending := (decl.fvarId,
          .vectorMut reference #[left, right] elemTy .vecReverseSlice #[]) :: s.pending }
        return instrs
      if fn == ``Move.Vector.trim || fn == ``Move.MutRef.trim ||
          fn == ``Move.Vector.trimReverse || fn == ``Move.MutRef.trimReverse then
        let some reference := vars[vars.size - 3]? | throwError "vector trim is missing its reference"
        let some newLen := beforeWorld? vars | throwError "vector trim is missing its new length"
        let some elemType := types[0]? | throwError "vector trim is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        let op := if fn == ``Move.Vector.trim || fn == ``Move.MutRef.trim then .vecTrim else .vecTrimReverse
        modify fun s => { s with pending := (decl.fvarId,
          .vectorMut reference #[newLen] elemTy op #[.vector elemTy]) :: s.pending }
        return instrs
      if fn == ``Move.Vector.rotate || fn == ``Move.MutRef.rotate then
        let some reference := vars[vars.size - 3]? | throwError "vector rotate is missing its reference"
        let some rot := beforeWorld? vars | throwError "vector rotate is missing its rotation"
        let some elemType := types[0]? | throwError "vector rotate is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending := (decl.fvarId,
          .vectorMut reference #[rot] elemTy .vecRotate #[.u64]) :: s.pending }
        return instrs
      if fn == ``Move.Vector.rotateSlice || fn == ``Move.MutRef.rotateSlice then
        let some reference := vars[vars.size - 5]? | throwError "vector rotate_slice is missing its reference"
        let some left := vars[vars.size - 4]? | throwError "vector rotate_slice is missing its left bound"
        let some rot := vars[vars.size - 3]? | throwError "vector rotate_slice is missing its rotation"
        let some right := beforeWorld? vars | throwError "vector rotate_slice is missing its right bound"
        let some elemType := types[0]? | throwError "vector rotate_slice is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        modify fun s => { s with pending := (decl.fvarId,
          .vectorMut reference #[left, rot, right] elemTy .vecRotateSlice #[.u64]) :: s.pending }
        return instrs
      if fn == ``Move.Vector.destroyEmpty then
        let some vector := beforeWorld? vars | throwError "destroy_empty is missing its vector"
        modify fun s => { s with pending := (decl.fvarId, .call {
          op := .vecDestroyEmpty, srcs := #[vector], resultTy := none }) :: s.pending }
        return instrs
      if fn == ``Prod.mk then
        modify fun s => { s with products := (decl.fvarId, vars) :: s.products }
        let some tailType := types[1]? | throwError "product constructor is missing its second type"
        if tailType.isConstOf ``World then
          let some value := vars[0]?
            | throwError "effect result is missing its returned value"
          let values ← flattenProductValue value
          if values.isEmpty then
            modify fun s => { s with
              returnAliases := (decl.fvarId, none) :: s.returnAliases }
          else if values.size == 1 then
            modify fun s => { s with
              returnAliases := (decl.fvarId, values[0]?) :: s.returnAliases }
          else
            modify fun s => { s with
              returnValues := (decl.fvarId, values) :: s.returnValues }
        return instrs
      -- Integer operations carry their width as the leading width-tag type
      -- argument; the `Width` instance is erased and the value operands are
      -- the trailing locals. Only operations whose Move semantics depends on
      -- the width annotate the LIR operation with it.
      let widthOfType (index : Nat) : BuildM MoveModel.IR.IntWidth := do
        let some tag := types[index]?
          | throwError "integer operation is missing its width tag"
        let some w := widthOfExpr? tag
          | throwError "integer width is not statically known"
        pure w
      -- One recognizer for every integer operation: the marker name gives the
      -- operation and the type arguments give the `NumType` (sign then width),
      -- so signed and unsigned share one path.  Comparisons use the
      -- width-agnostic `lt`/`le`/`eq` (correct on the mathematical value).
      let numTypeOfArgs (signIndex widthIndex : Nat) :
          BuildM MoveModel.IR.NumType := do
        let some signTag := types[signIndex]?
          | throwError "integer operation is missing its sign tag"
        let some signed := signOfExpr? signTag
          | throwError "integer signedness is not statically known"
        let some widthTag := types[widthIndex]?
          | throwError "integer operation is missing its width tag"
        let some w := widthOfExpr? widthTag
          | throwError "integer width is not statically known"
        pure ⟨w, signed⟩
      let intBinary? : Option (MoveModel.IR.NumType → LIR.Oper) :=
        if fn == ``MoveInt.add then some (.add ·)
        else if fn == ``MoveInt.sub then some (.sub ·)
        else if fn == ``MoveInt.mul then some (.mul ·)
        else if fn == ``MoveInt.div then some (.div ·)
        else if fn == ``MoveInt.mod then some (.mod ·)
        else if fn == ``MoveInt.land then some (.bitAnd ·)
        else if fn == ``MoveInt.lor then some (.bitOr ·)
        else if fn == ``MoveInt.lxor then some (.bitXor ·)
        else if fn == ``MoveInt.shl then some (.shl ·)
        else if fn == ``MoveInt.shr then some (.shr ·)
        else if fn == ``MoveInt.less then some (fun _ => .lt)
        else if fn == ``MoveInt.lessEq then some (fun _ => .le)
        else if fn == ``MoveInt.equal then some (fun _ => .eq)
        else if fn == ``MoveInt.instDecidableLt then some (fun _ => .lt)
        else if fn == ``MoveInt.instDecidableLe then some (fun _ => .le)
        else none
      if let some mkOp := intBinary? then
        let some lhs := vars[vars.size - 2]?
          | throwError "binary operation is missing its left operand"
        let some rhs := vars[vars.size - 1]?
          | throwError "binary operation is missing its right operand"
        let nt ← numTypeOfArgs 0 1
        let op := mkOp nt
        let ty := if op matches .lt | .le | .eq then LIR.Ty.bool else .int nt
        addLocalTy decl.fvarId ty
        return instrs.push (.call #[localName decl.fvarId] op #[srcName lhs, srcName rhs])
      let generic? : Option LIR.Oper :=
        if fn == ``Move.Compare.less then some .lt
        else if fn == ``Move.Compare.equal then some .eq
        else if fn == ``Move.Compare.genericDecidableLT then some .lt
        else none
      if let some op := generic? then
        let some lhs := vars[0]? | throwError "binary operation is missing its left operand"
        let some rhs := vars[1]? | throwError "binary operation is missing its right operand"
        addLocalTy decl.fvarId .bool
        return instrs.push (.call #[localName decl.fvarId] op #[srcName lhs, srcName rhs])
      -- A `Decidable p` is represented by the Boolean the comparison produced
      -- (`a < b` in value position is `decide (a < b)`); `decide` is the
      -- identity on it.
      if fn == ``Decidable.decide then
        let some decidable := vars[vars.size - 1]?
          | throwError "`decide` is missing its instance"
        addLocalTy decl.fvarId .bool
        return instrs.push (.assign (localName decl.fvarId) (srcName decidable))
      if fn == ``MoveInt.cast then
        let some operand := vars[vars.size - 1]?
          | throwError "integer cast is missing its operand"
        let target ← numTypeOfArgs 2 3
        addLocalTy decl.fvarId (.int target)
        return instrs.push
          (.call #[localName decl.fvarId] (.cast target) #[srcName operand])
      -- Literals: `UInt.ofNat n` and `SInt.ofInt (Int.ofNat n)` (both
      -- non-negative; negation is a separate `SInt.neg`).  The `Int.ofNat`
      -- value is tracked as a natural literal by `recognizeLet`.
      if fn == ``UInt.ofNat || fn == ``MoveInt.ofInt then
        let some source := vars[vars.size - 1]?
          | throwError "integer literal is missing its value"
        let some n := assocFind? source (← get).natLiterals
          | throwError "integer literal is not statically known"
        let nt ← if fn == ``UInt.ofNat then
            pure (⟨← widthOfType 0, false⟩ : MoveModel.IR.NumType)
          else numTypeOfArgs 0 1
        addLocalTy decl.fvarId (.int nt)
        return instrs.push (.loadInt nt (localName decl.fvarId) (n : Int))
      -- Negation lowers to `0 - x` on the signed type.
      if fn == ``MoveInt.neg then
        let some operand := vars[vars.size - 1]?
          | throwError "negation is missing its operand"
        let nt ← numTypeOfArgs 0 1
        let zero ← freshTemp (.int nt)
        addLocalTy decl.fvarId (.int nt)
        return (instrs.push (.loadInt nt zero 0)).push
          (.call #[localName decl.fvarId] (.sub nt) #[zero, srcName operand])
      if fn == ``Bool.true || fn == ``Bool.false then
        addLocalTy decl.fvarId .bool
        return instrs.push (.loadBool (localName decl.fvarId) (fn == ``Bool.true))
      if fn == ``Move.Address.ofNat then
        let some source := vars.back?
          | throwError "address literal is missing its value"
        let some value := assocFind? source (← get).natLiterals
          | throwError "address literal is not statically known"
        addLocalTy decl.fvarId .address
        return instrs.push (.loadAddress (localName decl.fvarId) value)
      if let some alias := Move.addressAliasByDeclaration? (← getEnv) fn then
        addLocalTy decl.fvarId .address
        return instrs.push (.loadAddress (localName decl.fvarId) alias.value)
      if let some (nt, value) ← intConstant? fn then
        addLocalTy decl.fvarId (.int nt)
        return instrs.push (.loadInt nt (localName decl.fvarId) value)
      if fn == ``Move.Vector.empty then
        let some elemType := types[0]? | throwError "empty vector is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        addLocalTy decl.fvarId (.vector elemTy)
        return instrs.push (.call #[srcName decl.fvarId] .vecPack #[])
      if fn == ``Move.Vector.singleton then
        let some value := vars.back? | throwError "singleton vector is missing its element"
        let some elemType := types[0]? | throwError "singleton vector is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        addLocalTy decl.fvarId (.vector elemTy)
        return instrs.push (.call #[srcName decl.fvarId] .vecPack #[srcName value])
      if fn == ``Move.Vector.push then
        let some vector := vars[0]? | throwError "vector push is missing its vector"
        let some value := vars[1]? | throwError "vector push is missing its value"
        let some elemType := types[0]? | throwError "vector push is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector element has compiler-erased type"
        addLocalTy decl.fvarId (.vector elemTy)
        return instrs.push (.call #[srcName decl.fvarId] .vecPush #[srcName vector, srcName value])
      if fn == ``Move.Vector.length then
        let some vector := vars[0]? | throwError "vector length is missing its vector"
        addLocalTy decl.fvarId .u64
        return instrs.push (.call #[srcName decl.fvarId] .vecLen #[srcName vector])
      if fn == ``Move.Vector.isEmpty then
        let some vector := vars[0]? | throwError "vector emptiness test is missing its vector"
        let length ← freshTemp .u64
        let zero ← freshTemp .u64
        addLocalTy decl.fvarId .bool
        return instrs
          |>.push (.call #[length] .vecLen #[srcName vector])
          |>.push (.loadInt .u64 zero 0)
          |>.push (.call #[srcName decl.fvarId] .eq #[length, zero])
      if fn == ``Move.Vector.contains then
        let some reference := vars[0]? | throwError "vector contains is missing its vector reference"
        let some valueRef := vars[1]? | throwError "vector contains is missing its value reference"
        let some elemType := types[0]? | throwError "vector contains is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        let vector ← freshTemp (.vector elemTy)
        let value ← freshTemp elemTy
        addLocalTy decl.fvarId .bool
        return instrs
          |>.push (.call #[vector] .readRef #[srcName reference])
          |>.push (.call #[value] .readRef #[srcName valueRef])
          |>.push (.call #[srcName decl.fvarId] .vecContains #[vector, value])
      if fn == ``Move.Vector.indexOf then
        let some reference := vars[0]? | throwError "vector index_of is missing its vector reference"
        let some valueRef := vars[1]? | throwError "vector index_of is missing its value reference"
        let some elemType := types[0]? | throwError "vector index_of is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType | throwError "vector element has compiler-erased type"
        let vector ← freshTemp (.vector elemTy)
        let value ← freshTemp elemTy
        let results ← freshResultLocals #[.bool, .u64]
        modify fun s => { s with tupleLocals := (decl.fvarId, results) :: s.tupleLocals }
        return instrs
          |>.push (.call #[vector] .readRef #[srcName reference])
          |>.push (.call #[value] .readRef #[srcName valueRef])
          |>.push (.call (results.map (·.1)) .vecIndexOf #[vector, value])
      if fn == ``Move.Ref.length || fn == ``Move.MutRef.length then
        let some reference := vars[0]? | throwError "vector length is missing its reference"
        let some elemType := types[0]? | throwError "vector length is missing its element type"
        let some elemTy ← translateCurrentTy (← getEnv) elemType
          | throwError "vector length has a compiler-erased element type"
        let vector ← freshTemp (.vector elemTy)
        addLocalTy decl.fvarId .u64
        return instrs
          |>.push (.call #[vector] .readRef #[srcName reference])
          |>.push (.call #[srcName decl.fvarId] .vecLen #[vector])
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
            let tagged := Move.isMoveFunction env fn
            if !tagged then
              pure none
            else
              let some owner := Move.moduleForDeclaration? env fn
                | throwError "Move function `{fn}` has no enclosing `module` identity"
              if owner == (← get).module then
                pure none
              else
                unless movePublicAttr.hasTag env fn || moveEntryAttr.hasTag env fn do
                  throwError "function `{fn}` is not visible outside Move module `{owner.name}`"
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
          if returns.size ≤ 1 then
            modify fun s => { s with pending := (decl.fvarId, .call {
              op := .function fn callTypeArgs
              srcs := callVars
              resultTy := returns[0]?
            }) :: s.pending }
          else
            modify fun s => { s with pending := (decl.fvarId, .multiCall
              (.function fn callTypeArgs) callVars returns true) :: s.pending }
          return instrs
        if returns.size > 1 then
          modify fun s => { s with pending := (decl.fvarId, .multiCall
            (.function fn callTypeArgs) callVars returns false) :: s.pending }
          return instrs
        match returns[0]? with
        | some ty =>
            addLocalTy decl.fvarId ty
            return instrs.push (.call #[srcName decl.fvarId]
              (.function fn callTypeArgs) (callVars.map srcName))
        | none =>
            modify fun s => { s with returnAliases := (decl.fvarId, none) :: s.returnAliases }
            return instrs.push (.call #[] (.function fn callTypeArgs) (callVars.map srcName))
      if Move.isMoveFunction (← getEnv) fn then
        throwError "callee `{fn}` is not selected in this `module%`"
      -- Type-class dictionaries and proof evidence are compiler-erased.
      if fn.toString.contains "instInhabited" then return instrs
      if fn.toString.contains "instWidth" || fn.toString.contains "instSign"
          || fn.toString.contains "instNatCast" then
        return instrs
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

private partial def collectLoopStates (self : Name) : Code .pure → BuildM Unit
  | .let decl next => do
      if let .const fn _ args _ := decl.value then
        match ← loopMarker? self fn (fvarArgs args) with
        | some (.enter label _ state) =>
            modify fun s => { s with loopStates := (label, state) :: s.loopStates }
        | some (.continue_ label _ state true) =>
            modify fun s => { s with loopTails := (label, (state, next)) :: s.loopTails }
        | _ => pure ()
      collectLoopStates self next
  | .fun decl next _ | .jp decl next => do
      collectLoopStates self decl.value
      collectLoopStates self next
  | .cases cases =>
      for alt in cases.alts do collectLoopStates self alt.getCode
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
              match ← loopMarker? self fn (fvarArgs args) with
              | some marker@(.enter label _ state) =>
                  claimLoopMarker fn marker
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
              | some marker@(.continue_ label _ state tail) =>
                  claimLoopMarker fn marker
                  emitLoopContinue label state blockName instrs
                  if tail && !(← get).builtLoopExits.contains label then
                    modify fun s => { s with builtLoopExits := label :: s.builtLoopExits }
                    let some headerState := assocFind? label (← get).loopStates
                      | throwError "loop {label} has no header state"
                    let exitInstrs ← assignLoopState label state headerState #[]
                    walk env signatures self signature next (← loopExit label) exitInstrs
              | some marker@(.break_ label _ state) =>
                  claimLoopMarker fn marker
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
                  let nextInstrs ← recognizeLet signatures decl instrs
                  walk env signatures self signature next blockName
                    (← locateCurrent nextInstrs)
          | _ =>
              let nextInstrs ← recognizeLet signatures decl instrs
              walk env signatures self signature next blockName
                (← locateCurrent nextInstrs)
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
      if let some values := assocFind? result (← get).returnValues then
        emitBlock blockName instrs (.ret (values.map srcName))
      else if let some fields := assocFind? result (← get).tupleLocals then
        emitBlock blockName instrs (.ret (fields.map (·.1)))
      else match assocFind? result (← get).returnAliases with
      | some (some value) => emitBlock blockName instrs (.ret #[srcName value])
      | some none => emitBlock blockName instrs (.ret #[])
      | none =>
          match assocFind? result (← get).pending with
          | some (.abort code) => emitBlock blockName instrs (.abort (srcName code))
          | some (.multiCall op srcs resultTys _) =>
              let results ← freshResultLocals resultTys
              let names := results.map (·.1)
              emitBlock blockName
                (instrs.push (.call names op (srcs.map srcName))) (.ret names)
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
          | some pending@(.vectorPop ..) =>
              let .vectorPop _ elemTy := pending | unreachable!
              addLocalTy result elemTy
              emitBlock blockName
                (← materializePending pending #[srcName result] instrs)
                (.ret #[srcName result])
          | some pending@(.vectorMut _ _ _ _ resultTys) =>
              let results ← freshResultLocals resultTys
              let names := results.map (·.1)
              emitBlock blockName (← materializePending pending names instrs) (.ret names)
          | none =>
              let returns ← if let some _ := assocFind? result (← get).products then
                  pure ((← flattenProductValue result).map srcName)
                else
                  pure <| if (← get).localIds.any (· == result) then #[srcName result] else #[]
              emitBlock blockName instrs (.ret returns)
  | .cases cases =>
      if (← get).sourceMarkerResults.contains cases.discr then
        let some alt := cases.alts[0]?
          | throwError "source span marker result has no product case"
        match alt with
        | .alt ``Prod.mk _ body _ =>
            walk env signatures self signature body blockName instrs
        | .alt ``Unit.unit _ body _ | .alt ``PUnit.unit _ body _ =>
            walk env signatures self signature body blockName instrs
        | _ => throwError "source span marker did not normalize to a product case"
      else if (← get).loopLiveResults.contains cases.discr then
        let some body := trueAlternative? cases.alts
          | throwError "loop liveness test has no true branch"
        walk env signatures self signature body blockName instrs
      else match assocFind? cases.discr (← get).pending with
      | some (.abort code) => emitBlock blockName instrs (.abort (srcName code))
      | some (.multiCall op srcs resultTys effectful) =>
          let some alt := cases.alts[0]? | throwError "multiple-return call has no product case"
          match alt with
          | .alt ``Prod.mk params body _ =>
              let results ← freshResultLocals resultTys
              let nextInstrs := instrs.push
                (.call (results.map (·.1)) op (srcs.map srcName))
              let nextInstrs ← locateCurrent nextInstrs
              if effectful then
                let some valueParam := params[0]?
                  | throwError "effect result product has no value field"
                modify fun s => { s with
                  tupleLocals := (valueParam.fvarId, results) :: s.tupleLocals }
                walk env signatures self signature body blockName nextInstrs
              else
                let bound ← bindTupleParams env params results nextInstrs
                walk env signatures self signature body blockName
                  (← locateCurrent bound)
          | _ => throwError "multiple-return call did not normalize to a `Prod.mk` case"
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
              walk env signatures self signature body blockName
                (← locateCurrent nextInstrs)
          | _ => throwError "Move effect did not normalize to a `Prod.mk` case"
      | some pending@(.vectorInsert ..) | some pending@(.vectorRemove ..) |
        some pending@(.vectorPop ..) | some pending@(.vectorMut ..) =>
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
              walk env signatures self signature body blockName
                (← locateCurrent nextInstrs)
          | _ => throwError "Move effect did not normalize to a `Prod.mk` case"
      | none =>
          if let some sources := assocFind? cases.discr (← get).tupleLocals then
            let some alt := cases.alts[0]? | throwError "tuple value has no product case"
            match alt with
            | .alt ``Prod.mk params body _ =>
                let bound ← bindTupleParams env params sources instrs
                walk env signatures self signature body blockName
                  (← locateCurrent bound)
                return
            | _ => throwError "tuple value did not normalize to a `Prod.mk` case"
          if let some _ := assocFind? cases.discr (← get).products then
            let values ← flattenProductValue cases.discr
            let mut sources := #[]
            for value in values do
              let some type ← localTy? value
                | throwError "tuple component has no Move runtime type"
              sources := sources.push (srcName value, type)
            let some alt := cases.alts[0]? | throwError "tuple value has no product case"
            match alt with
            | .alt ``Prod.mk params body _ =>
                let bound ← bindTupleParams env params sources instrs
                walk env signatures self signature body blockName
                  (← locateCurrent bound)
                return
            | _ => throwError "tuple value did not normalize to a `Prod.mk` case"
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
                    if param.type.isConstOf ``lcErased then continue
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
                  if param.type.isConstOf ``lcErased then continue
                  match ← translateCurrentTy env param.type with
                  | none => pure ()
                  | some _ =>
                      addLocal env param.fvarId param.type
                      dsts := dsts.push (srcName param.fvarId)
                let nextInstrs := instrs.push
                  (.call dsts (.unpack structName structTypeArgs) #[srcName cases.discr])
                walk env signatures self signature body blockName
                  (← locateCurrent nextInstrs)
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
  else if moveFriendAttr.hasTag env name || movePackageAttr.hasTag env name then .friend_
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
  unless Move.isMoveFunction env name do
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
      let sourceName := param.binderName.toString
      params := params.push {
        name := localName param.fvarId
        ty := ty
        sourceName := if sourceName.isEmpty || sourceName == "_" then none
          else some sourceName
      }
      paramIds := param.fvarId :: paramIds
  if moveNativeAttr.hasTag env name then
    return {
      leanName := name
      moveName := name.getString!
      typeParams := signature.typeParams
      visibility := visibility env name
      params := params
      returns := signature.returns
      locals := #[]
      blocks := #[]
      calls := #[]
      acquires := #[]
      attributes := Move.userAttributes env name
      native := true
    }
  let initial : BuildState := {
    module, tyContext, localIds := paramIds, loopParams := params
  }
  let (_, initial) ← (collectLoopMarkerInputs code).run initial
  let (_, initial) ← (collectLoopStates name code).run initial
  let (_, initial) ← (collectJoinParams env code).run initial
  let (_, state) ← (do
    walk env signatures name signature code "entry" #[]
    finishSourceBinding).run initial
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
    attributes := Move.userAttributes env name
  }

private def addNames (names additions : Array Name) : Array Name :=
  additions.foldl (fun names name => if names.any (· == name) then names else names.push name) names

private def propagateAcquires (functions : Array LIR.FunDecl) : Array LIR.FunDecl :=
  let rec go : Nat → Array LIR.FunDecl → Array LIR.FunDecl
    | 0, current => current
    | fuel + 1, current =>
        let next := current.map fun decl =>
          let inherited := decl.calls.foldl (fun names callee =>
            match current.find? (·.leanName == callee) with
            | some calleeDecl => addNames names calleeDecl.acquires
            | none => names) decl.acquires
          { decl with acquires := inherited }
        go fuel next
  go functions.size functions

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
  | .bool | .uint _ | .sint _ | .address => required != .key
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
def compileModule (outputModule : Move.ModuleRef) (structNames funNames : Array Name) :
    CoreM LIR.Module := do
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
  let module := representative?.bind (Move.moduleForDeclaration? env) |>.getD outputModule
  let mut signatures : FunSignatures := []
  for name in funNames do
    unless Move.isMoveFunction env name do
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
          throwError "callee `{callee}` is not selected in this `module%`"
        if movePackageAttr.hasTag env callee && owner.address != module.address then
          throwError "package-visible Move function `{callee}` cannot be called across package addresses"
        if moveFriendAttr.hasTag env callee then
          let trusted := Move.moduleFriendsForDeclaration env callee
          unless trusted.any fun friend =>
              friend.address == module.address && friend.name == module.name do
            throwError "Move function `{callee}` is friend-visible, but module `{module.name}` is not in its friend list"
        externalFuns := externalFuns.push {
          leanName := callee
          address := owner.address
          moduleName := owner.name
          functionName := callee.getString!
        }
  return {
    address := module.address
    name := outputModule.name
    structs := structs
    functions := functions
    externalFuns := externalFuns
    friends := representative?.map (Move.moduleFriendsForDeclaration env) |>.getD []
      |>.toArray.map fun friend => {
        address := friend.address
        moduleName := friend.name
      }
  }

end Move.Compiler
