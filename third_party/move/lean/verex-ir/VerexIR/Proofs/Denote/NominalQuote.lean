-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types

/-! # Canonical quotation of finite nominal layouts

A defined unit owns one parameterized descriptor per nominal declaration.
Quotation validates the incoming finite native layout, then names that
descriptor with its explicit type arguments. The descriptor's fields name
nested descriptors too. This is a quotation boundary for the existing finite
layout backend, not a recursive carrier interpretation.
-/

namespace VerexIR.Proofs.Denote

open Lean Meta

private def nominalUnitOption : Name := `_verexNominalUnit
private def syntheticNativeTypesOption : Name := `_verexSyntheticNativeTypes

/-- Literal nominal layouts are available only to explicitly scoped synthetic
metaprogram tests. An active source unit still validates every nominal layout. -/
def withSyntheticNativeTypes (action : MetaM α) : MetaM α :=
  withOptions (fun options => options.setBool syntheticNativeTypesOption true) action

/-- Scope quotation to the unit whose handles the native types use. This is
reader-local, including when elaboration exits by an exception. -/
def nominalUnitOptions (name : Name) (options : Options) : Options :=
  options.set nominalUnitOption name

def withNominalUnit [MonadWithOptions m] (name : Name) (action : m α) : m α :=
  withOptions (nominalUnitOptions name) action

/-- Scope command elaboration through its command scope. Meta quotation inside
it inherits the same unit, including commands that elaborate nested terms. -/
def withNominalUnitCommand (name : Name) (action : Elab.Command.CommandElabM α) :
    Elab.Command.CommandElabM α :=
  Elab.Command.withScope (fun scope =>
    { scope with opts := nominalUnitOptions name scope.opts }) action

private structure DescriptorCache where
  schema : NTy
  validated : Array NTy := #[]

private structure NominalCache where
  units : NameMap Validation.ValidatedUnit := {}
  descriptors : NameMap DescriptorCache := {}
  deriving Inhabited

private initialize nominalCache : EnvExtension NominalCache ←
  registerEnvExtension (pure {})

/-- Descriptor names must remain recognizable in importers without retaining
an additional persistent copy of their unit. -/
private initialize descriptorNames : SimplePersistentEnvExtension (Name × Nat) (NameMap Nat) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun names (name, arity) => names.insert name arity
    addImportedFn := fun modules => modules.foldl (init := {}) fun names entries =>
      entries.foldl (init := names) fun names (name, arity) => names.insert name arity }

/-- Register the authoritative value of a defined unit for subsequent scoped
quotation. Imported units are registered by the same definition boundary. -/
def registerNominalUnit (name : Name) (unit : Validation.ValidatedUnit) : MetaM Unit :=
  modifyEnv fun env => nominalCache.modifyState env fun state =>
    { state with units := state.units.insert name unit }

/-- The canonical name depends on the defined unit and the relocated handle,
never on the encountered instantiation or the contents of its fields. -/
def nominalDescriptorName (unit : Name) (source : StructHandle) : Name :=
  unit ++ `nominal ++ Name.num `namespace source.namespaceId.index ++
    Name.num `declaration source.structId

/-- The arity of a descriptor produced by this quotation boundary. -/
def nominalDescriptorArity? (env : Environment) (name : Name) : Option Nat :=
  (descriptorNames.getState env).find? name

/-- Whether a term is a fully applied canonical descriptor. -/
def isNominalType (env : Environment) (type : Lean.Expr) : Bool :=
  let type := type.consumeMData
  match type.getAppFn with
  | .const name _ => nominalDescriptorArity? env name == some type.getAppNumArgs
  | _ => false

/-- Open exactly one registered descriptor application for a syntactic
consumer. Its fields and arguments retain their own canonical descriptors. -/
def nativeTypeView (type : Lean.Expr) : MetaM Lean.Expr := do
  let type := type.consumeMData
  let .const name levels := type.getAppFn | return type
  unless isNominalType (← getEnv) type do return type
  let info ← getConstInfoDefn name
  return (info.value.instantiateLevelParams info.levelParams levels).beta type.getAppArgs

private def activeNominalUnit : MetaM (Option (Name × Validation.ValidatedUnit)) := do
  let some name := (← getOptions).get? (α := Name) nominalUnitOption | return none
  let some unit := (nominalCache.getState (← getEnv)).units.find? name
    | throwError "internal: native quotation has no registered unit {name}"
  return some (name, unit)

private def descriptorSchema (unitName : Name) (unit : Validation.ValidatedUnit)
    (source : StructHandle) : MetaM NTy := do
  let name := nominalDescriptorName unitName source
  if let some cached := (nominalCache.getState (← getEnv)).descriptors.find? name then
    return cached.schema
  let some schema := structNTy unit source
    | throwError "the nominal declaration {repr source} has no finite native layout"
  modifyEnv fun env => nominalCache.modifyState env fun state =>
    { state with descriptors := state.descriptors.insert name { schema } }
  return schema

private def quoteSource (source : StructHandle) : Lean.Expr :=
  mkApp2 (mkConst ``StructHandle.mk)
    (mkApp (mkConst ``NamespaceId.mk) (toExpr source.namespaceId.index)) (toExpr source.structId)

private def quoteSharing (shared : Sharing) : Lean.Expr :=
  mkApp2 (mkConst ``Sharing.mk) (toExpr shared.parameters) (toExpr shared.results)

private def nominalArguments : NTy → NRow
  | .struct _ arguments _ | .enum _ arguments _ _ _ => arguments
  | _ => .nil

/-- Whether a nominal type is its declaration's layout at its arguments, which the
declaration's descriptor names. Two other layouts of the same declaration are
written out: a shared view of a declaration holding a mutable reference
(`NTy.erase`), and a declaration holding a shared view of a type argument that
holds one, whose field erases after the substitution (`structNTyFuel` at the
arguments), which substituting the descriptor's layout does not. -/
private def validateNominal (unitName : Name) (unit : Validation.ValidatedUnit)
    (source : StructHandle) (type : NTy) : MetaM Bool := do
  let name := nominalDescriptorName unitName source
  let schema ← descriptorSchema unitName unit source
  let some cached := (nominalCache.getState (← getEnv)).descriptors.find? name
    | throwError "internal: missing descriptor schema {name}"
  if cached.validated.any (· == type) then return true
  let arguments := nominalArguments type
  unless arguments.toList.length == (nominalArguments schema).toList.length do
    throwError "internal: native layout does not match declaration {repr source} in {unitName}"
  let declared := schema.subst arguments
  if declared == type then
    let cached := { cached with validated := cached.validated.push type }
    modifyEnv fun env => nominalCache.modifyState env fun state =>
      { state with descriptors := state.descriptors.insert name cached }
    return true
  if declared.erase == type then return false
  let atArguments := structNTyFuel (unitTypes unit) unit arguments (typeFuel unit) source
  unless atArguments == some type || atArguments.map (·.erase) == some type do
    throwError "internal: native layout does not match declaration {repr source} in {unitName}"
  return false

mutual
private partial def quoteNativeCore (parameter : Nat → MetaM Lean.Expr)
    (expandRoot : Bool) (ancestors : List Name) (type : NTy) : MetaM Lean.Expr := do
  match type with
  | .unit => return mkConst ``NTy.unit
  | .bool => return mkConst ``NTy.bool
  | .int width signed => return mkApp2 (mkConst ``NTy.int) (toExpr width) (toExpr signed)
  | .num => return mkConst ``NTy.num
  | .address => return mkConst ``NTy.address
  | .signer => return mkConst ``NTy.signer
  | .string => return mkConst ``NTy.string
  | .bytes => return mkConst ``NTy.bytes
  | .char => return mkConst ``NTy.char
  | .param index => parameter index
  | .struct source arguments fields =>
      if !expandRoot then
        if let some (unitName, unit) ← activeNominalUnit then
          if ← validateNominal unitName unit source type then
            let name ← ensureNominalDescriptor unitName unit source ancestors
            return mkAppN (mkConst name)
              (← arguments.toList.toArray.mapM (quoteNativeCore parameter false ancestors))
        else
          unless (← getOptions).getBool syntheticNativeTypesOption do
            throwError "native nominal quotation requires a defined unit scope"
      return mkApp3 (mkConst ``NTy.struct) (quoteSource source)
        (← quoteNativeRowCore parameter ancestors arguments)
        (← quoteNativeRowCore parameter ancestors fields)
  | .enum source arguments names rows _ =>
      if !expandRoot then
        if let some (unitName, unit) ← activeNominalUnit then
          if ← validateNominal unitName unit source type then
            let name ← ensureNominalDescriptor unitName unit source ancestors
            return mkAppN (mkConst name)
              (← arguments.toList.toArray.mapM (quoteNativeCore parameter false ancestors))
        else
          unless (← getOptions).getBool syntheticNativeTypesOption do
            throwError "native nominal quotation requires a defined unit scope"
      let namesExpr := toExpr names
      let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[namesExpr])
      return mkAppN (mkConst ``NTy.enum) #[quoteSource source,
        ← quoteNativeRowCore parameter ancestors arguments, namesExpr,
        ← quoteNativeRowsCore parameter ancestors rows, distinct]
  | .tuple elements =>
      return mkApp (mkConst ``NTy.tuple) (← quoteNativeRowCore parameter ancestors elements)
  | .table key value =>
      return mkApp2 (mkConst ``NTy.table)
        (← quoteNativeCore parameter false ancestors key) (← quoteNativeCore parameter false ancestors value)
  | .vector element => return mkApp (mkConst ``NTy.vector) (← quoteNativeCore parameter false ancestors element)
  | .slice element => return mkApp (mkConst ``NTy.slice) (← quoteNativeCore parameter false ancestors element)
  | .array element length =>
      return mkApp2 (mkConst ``NTy.array) (← quoteNativeCore parameter false ancestors element) (toExpr length)
  | .ref element => return mkApp (mkConst ``NTy.ref) (← quoteNativeCore parameter false ancestors element)
  | .sliceRef element =>
      return mkApp (mkConst ``NTy.sliceRef) (← quoteNativeCore parameter false ancestors element)
  | .ghost element => return mkApp (mkConst ``NTy.ghost) (← quoteNativeCore parameter false ancestors element)
  | .observed referent =>
      return mkApp (mkConst ``NTy.observed) (← quoteNativeCore parameter false ancestors referent)
  | .nominal source arguments productive =>
      return mkApp3 (mkConst ``NTy.nominal) (quoteSource source)
        (← quoteNativeRowCore parameter ancestors arguments) (toExpr productive)
  | .function parameters shared results =>
      return mkApp3 (mkConst ``NTy.function)
        (← quoteNativeRowCore parameter ancestors parameters) (quoteSharing shared)
        (← quoteNativeRowCore parameter ancestors results)

private partial def quoteNativeRowCore (parameter : Nat → MetaM Lean.Expr)
    (ancestors : List Name) : NRow → MetaM Lean.Expr
  | .nil => return mkConst ``NRow.nil
  | .cons head tail =>
      return mkApp2 (mkConst ``NRow.cons)
        (← quoteNativeCore parameter false ancestors head) (← quoteNativeRowCore parameter ancestors tail)

private partial def quoteNativeRowsCore (parameter : Nat → MetaM Lean.Expr)
    (ancestors : List Name) : NRows → MetaM Lean.Expr
  | .nil => return mkConst ``NRows.nil
  | .cons head tail =>
      return mkApp2 (mkConst ``NRows.cons)
        (← quoteNativeRowCore parameter ancestors head) (← quoteNativeRowsCore parameter ancestors tail)

private partial def ensureNominalDescriptor (unitName : Name) (unit : Validation.ValidatedUnit)
    (source : StructHandle) (ancestors : List Name) : MetaM Name := do
  let name := nominalDescriptorName unitName source
  if (descriptorNames.getState (← getEnv)).contains name then return name
  if ancestors.contains name then
    throwError "the nominal declaration {repr source} has a recursive native layout"
  let schema ← descriptorSchema unitName unit source
  let arity := (nominalArguments schema).toList.length
  let rec bindArguments (remaining : Nat) (arguments : Array Lean.Expr) : MetaM Unit := do
    match remaining with
    | 0 =>
        let value ← quoteNativeCore (fun index =>
          match arguments[index]? with
          | some argument => pure argument
          | none => throwError "internal: declaration {repr source} refers to type parameter {index}")
          true (name :: ancestors) schema
        let value ← mkLambdaFVars arguments value
        let type ← mkForallFVars arguments (mkConst ``NTy)
        addDecl (.defnDecl {
          name, levelParams := [], type := type, value := value
          hints := .regular 0, safety := .safe })
        enableRealizationsForConst name
        modifyEnv fun env => descriptorNames.addEntry env (name, arity)
    | remaining + 1 =>
        withLocalDeclD (Name.num `type arguments.size) (mkConst ``NTy) fun argument =>
          bindArguments remaining (arguments.push argument)
  bindArguments arity #[]
  return name
end

/-- Canonical quotation in a registered unit. Nominal types require its scope;
only explicitly scoped synthetic tests can quote a literal nominal layout. -/
def quoteNativeType (type : NTy) : MetaM Lean.Expr :=
  quoteNativeCore (fun index => pure (mkApp (mkConst ``NTy.param) (toExpr index))) false [] type

def quoteNativeRow (row : NRow) : MetaM Lean.Expr :=
  quoteNativeRowCore (fun index => pure (mkApp (mkConst ``NTy.param) (toExpr index))) [] row

def quoteNativeRows (rows : NRows) : MetaM Lean.Expr :=
  quoteNativeRowsCore (fun index => pure (mkApp (mkConst ``NTy.param) (toExpr index))) [] rows

/-- Resolve parameter leaves while building the canonical application. In
particular, no parameter replacement needs to inspect hidden descriptor bodies. -/
def quoteResolvedNativeType (types : Lean.Expr) (type : NTy) : MetaM Lean.Expr :=
  quoteNativeCore (fun index => pure ((mkApp types (toExpr index)).headBeta)) false [] type

end VerexIR.Proofs.Denote
