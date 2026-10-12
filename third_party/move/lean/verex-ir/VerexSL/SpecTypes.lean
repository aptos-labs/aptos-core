-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Registry
import VerexIR.Proofs.Typed
import VerexIR.Proofs.SimpAttrs
import VerexIR.Proofs.Denote.Attr

/-!
# Generated typed twins of VIR nominal declarations

Each supported nominal declaration of a registered unit gets a Lean twin: a
structure or inductive whose fields carry the certified specification
representation of the declared field types (`SpecInt` for integers, plain
Lean scalars for the rest, nested twins for nominal fields), together with its
`erase`/`decode?` pair and their roundtrip.

Generation is driven by the validated unit, not by surface syntax, so any
frontend that registers a unit gets the same twins.  A declaration with
content the representation does not support yet (closures or references)
gets no twin; a clause that needs the missing twin is a
loud diagnostic downstream, never a silent degradation.
-/

namespace VerexSL.SpecTypes

open Lean Meta Elab Command
open VerexIR (IntWidth QualifiedName TypeId NamespaceId)
open VerexIR.Validation (ValidatedUnit ValidatedNamespace)

/-- Specification representation of one twin field. -/
inductive FieldRep where
  | int (width : IntWidth) (signed : Bool)
  | bool
  | string
  | address
  | signer
  | bytes
  | unit
  /-- Move vectors carry their u64 bound; other profiles retain native arrays. -/
  | vector (element : FieldRep) (bounded : Bool)
  /-- A type parameter of the declaration currently being represented. -/
  | parameter (index : Nat)
  /-- A nested twin, by its generated Lean name. -/
  | nominal (twin : Name) (arguments : Array FieldRep)
  deriving Repr, BEq, Inhabited

/-- One represented enum variant and its payload row. -/
structure VariantInfo where
  name : String
  fields : Array (String × FieldRep)
  deriving Repr, Inhabited

/-- One generated twin: the nominal declaration it represents.  Plain
structures use `fields`; enums use `variants`. -/
structure TwinInfo where
  /-- Declared struct name. -/
  name : String
  qualified : QualifiedName
  /-- Generated Lean name of the twin structure. -/
  twin : Name
  namespaceIndex : Nat
  /-- Index of the declaration in its namespace's struct table, the runtime
  identity nominal values carry. -/
  structIndex : Nat
  /-- Number of type parameters accepted by the generated twin. -/
  typeParameterCount : Nat := 0
  fields : Array (String × FieldRep)
  variants : Array VariantInfo := #[]
  deriving Repr, Inhabited


/-- The interned spelling of a name id, if any. -/
private def nameOf? (unit : ValidatedUnit) (name : VerexIR.NameId) :
    Option QualifiedName :=
  unit.tables.names[name.index]?

/-- Locate a struct declaration by qualified name. -/
private def structOf? (unit : ValidatedUnit) (qualified : QualifiedName) :
    Option (Nat × Nat × VerexIR.StructDecl) := do
  let ns ← unit.namespaces[qualified.namespaceId.index]?
  ns.structs.zipIdx.findSome? fun (declaration, index) =>
    if nameOf? unit declaration.name == some qualified then
      some (qualified.namespaceId.index, index, declaration)
    else none

/-- The representation of one field type, if supported.  Nominal fields name
their twin; the caller is responsible for ensuring the nested twin exists. -/
private partial def fieldRep? (unit : ValidatedUnit) (twinName : QualifiedName → Name)
    (supported : QualifiedName → Bool) (typeId : TypeId)
    (profile : Option VerexIR.Profile) : Option FieldRep := do
  let ty ← unit.tables.types[typeId.index]?
  match ty with
  | .integer .pointer _ => none
  | .integer (.bits 0) _ => none
  | .integer width signed => some (.int width signed)
  | .bool => some .bool
  | .string => some .string
  | .address => some .address
  | .signer => some .signer
  | .bytes => some .bytes
  | .unit => some .unit
  | .vector element _ =>
      some (.vector (← fieldRep? unit twinName supported element profile) (profile == some .move))
  | .typeParameter index => some (.parameter index)
  | .nominal name arguments => do
      let qualified ← nameOf? unit name
      unless supported qualified do none
      let arguments ← arguments.mapM fun argument => match argument with
        | .typeArg value => fieldRep? unit twinName supported value.typeId profile
        | .const _ | .lifetime _ | .evidence _ => none
      some (.nominal (twinName qualified) arguments)
  | _ => none

/-- Whether a nominal declaration supports a twin, following nominal fields
through the declaration graph; a declaration on a cycle has none. -/
private partial def structSupported (unit : ValidatedUnit)
    (qualified : QualifiedName) (visiting : List QualifiedName := []) : Bool :=
  if visiting.contains qualified then false else
  match structOf? unit qualified with
  | none => false
  | some (namespaceIndex, _, declaration) =>
      let fields := declaration.fields ++
        declaration.variants.flatMap fun variant => variant.fields
      declaration.generics.all (fun binder => binder.kind == .typeArg) &&
        fields.all fun field =>
          (fieldRep? unit (fun _ => Name.anonymous)
            (structSupported unit · (qualified :: visiting))
            field.type.typeId (unit.namespaces[namespaceIndex]?.bind (·.profile))).isSome

/-- The generated Lean name of a struct's twin under the registered path:
its simple name, or, when the unit declares several structs of that name in
different modules, its module's path and name. -/
def twinName (segments : Array String) (unit : ValidatedUnit) (ambiguous : String → Bool)
    (qualified : QualifiedName) : Name :=
  let base := segments.foldl (fun name segment => Name.str name segment) .anonymous
  let owner := if ambiguous qualified.name then
      ((unit.tables.namespaces[qualified.namespaceId.index]?).map (·.segments)).getD #[]
    else #[]
  Name.str (owner.foldl (fun name segment => Name.str name segment) base) qualified.name

/-- The twin row of every supported struct of the unit, in an order where
nested twins precede their users. -/
def twinInfos (segments : Array String) (unit : ValidatedUnit) :
    Array TwinInfo := Id.run do
  let mut ordered : Array TwinInfo := #[]
  let mut emitted : Array QualifiedName := #[]
  let mut pending : Array QualifiedName := #[]
  for ns in unit.namespaces do
    for declaration in ns.structs do
      if let some qualified := nameOf? unit declaration.name then
        if structSupported unit qualified then
          pending := pending.push qualified
  let ambiguous := fun (name : String) => (pending.filter (·.name == name)).size > 1
  let twinName := twinName segments unit ambiguous
  -- Declarations are acyclic, so at most `pending.size` rounds settle all.
  for _ in [0:pending.size] do
    for qualified in pending do
      unless emitted.contains qualified do
        let some (namespaceIndex, structIndex, declaration) := structOf? unit qualified
          | continue
        let representFields := fun fields => fields.filterMap fun field => do
          let name ← nameOf? unit field.name
          let rep ← fieldRep? unit twinName
            (fun nested => emitted.contains nested ∨ nested == qualified)
            field.type.typeId (unit.namespaces[namespaceIndex]?.bind (·.profile))
          some (name.name, rep)
        let fields := representFields declaration.fields
        let variants := declaration.variants.filterMap fun variant => do
          let name ← nameOf? unit variant.name
          let fields := representFields variant.fields
          guard (fields.size == variant.fields.size)
          some { name := name.name, fields }
        let allFields := declaration.fields ++
          declaration.variants.flatMap fun variant => variant.fields
        -- Nested twins must already be emitted; self-reference is impossible
        -- in an acyclic unit, so a missing dependency just waits a round.
        let ready := allFields.all fun field =>
          match unit.tables.types[field.type.typeId.index]? with
          | some (.nominal nested _) =>
              (nameOf? unit nested).any emitted.contains
          | _ => true
        if ready && fields.size == declaration.fields.size &&
            variants.size == declaration.variants.size then
          ordered := ordered.push {
            name := qualified.name, qualified
            twin := twinName qualified
            namespaceIndex, structIndex
            typeParameterCount := declaration.generics.size
            fields, variants }
          emitted := emitted.push qualified
  return ordered

/-! ## Command generation -/

/-- Elaborate a generated theorem with its proof checked before the command
returns: a failure is an error at this site, never a `sorry` reported at the
module's end. -/
private def elabTheorem (command : TSyntax `command) : CommandElabM Unit := do
  elabCommand (← `(command| set_option Elab.async false in $command))

private def natLit (value : Nat) : Term :=
  Syntax.mkNatLit value

/-- Generated declarations are absolute: emission may run inside any user
namespace, and a relative name would be captured by it. -/
private def rootIdent (name : Name) : Ident :=
  mkIdent (rootNamespace ++ name)

/-- Term syntax of a field's representation type. `parameters` names the
type arguments of the declaration whose field is being emitted. -/
partial def FieldRep.typeSyntax (parameters : Array Ident := #[]) :
    FieldRep → CommandElabM Term
  | .int (.bits width) signed =>
      ``(VerexIR.SpecInt (IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(VerexIR.SpecInt IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width twin field"
  | .bool => ``(Bool)
  | .string | .address | .signer => ``(String)
  | .bytes => ``(Array UInt8)
  | .unit => ``(Unit)
  | .vector element bounded => do
      if bounded then ``(VerexIR.SpecVector $(← element.typeSyntax parameters))
      else ``(Array $(← element.typeSyntax parameters))
  | .parameter index => do
      let some parameterType := parameters[index]?
        | throwError "a generic twin field has no type parameter {index}"
      return parameterType
  | .nominal twin arguments => do
      let arguments ← arguments.mapM (FieldRep.typeSyntax parameters)
      if arguments.isEmpty then return rootIdent twin
      ``($(rootIdent twin) $arguments*)

/-- Whether this representation depends on the surrounding declaration's
type parameters. -/
partial def FieldRep.mentionsParameter : FieldRep → Bool
  | .parameter _ => true
  | .vector element _ => element.mentionsParameter
  | .nominal _ arguments => arguments.any mentionsParameter
  | _ => false

/-- Number of surrounding carrier slots needed to interpret a
representation. -/
partial def FieldRep.parameterCount : FieldRep → Nat
  | .parameter index => index + 1
  | .vector element _ => element.parameterCount
  | .nominal _ arguments =>
      arguments.foldl (fun count argument => max count argument.parameterCount) 0
  | _ => 0

/-- Substitute the represented arguments of a nominal use into one of the
head declaration's fields. -/
partial def FieldRep.instantiate (arguments : Array FieldRep) : FieldRep → FieldRep
  | .parameter index => arguments[index]?.getD (.parameter index)
  | .vector element bounded => .vector (element.instantiate arguments) bounded
  | .nominal twin nested => .nominal twin (nested.map (instantiate arguments))
  | rep => rep

/-- Meta-level native type of a represented field. -/
private def intWidthExpr : IntWidth → Lean.Expr
  | .bits width => mkApp (mkConst ``VerexIR.IntWidth.bits) (toExpr width)
  | .pointer => mkConst ``VerexIR.IntWidth.pointer
  | .unbounded => mkConst ``VerexIR.IntWidth.unbounded

partial def FieldRep.leanType (rep : FieldRep) (carrier? : Option Lean.Expr) :
    MetaM Lean.Expr := do
  match rep with
  | .int width signed => mkAppM ``VerexIR.SpecInt #[intWidthExpr width, toExpr signed]
  | .bool => return mkConst ``Bool
  | .string | .address | .signer => return mkConst ``String
  | .bytes => mkAppM ``Array #[mkConst ``UInt8]
  | .unit => return mkConst ``Unit
  | .vector element bounded =>
      mkAppM (if bounded then ``VerexIR.SpecVector else ``Array) #[← element.leanType carrier?]
  | .parameter index =>
      let some carrier := carrier?
        | throwError "a generic field representation has no carrier"
      return mkApp carrier (toExpr index)
  | .nominal twin arguments =>
      arguments.foldlM (init := mkConst twin) fun type argument =>
        return mkApp type (← argument.leanType carrier?)

/-- Meta-level codec of a represented field. -/
partial def FieldRep.codec (rep : FieldRep) (codecs? : Option Lean.Expr) :
    MetaM Lean.Expr := do
  match rep with
  | .int width signed =>
      mkAppM ``VerexIR.Proofs.Codec.specInt #[intWidthExpr width, toExpr signed]
  | .bool => return mkConst ``VerexIR.Proofs.Codec.bool
  | .string => return mkConst ``VerexIR.Proofs.Codec.string
  | .address => return mkConst ``VerexIR.Proofs.Codec.address
  | .signer => return mkConst ``VerexIR.Proofs.Codec.signer
  | .bytes => return mkConst ``VerexIR.Proofs.Codec.bytes
  | .unit => return mkConst ``VerexIR.Proofs.Codec.unit
  | .vector element bounded =>
      mkAppM (if bounded then ``VerexIR.Proofs.Codec.boundedVector else
        ``VerexIR.Proofs.Codec.vector) #[← element.codec codecs?]
  | .parameter index =>
      let some codecs := codecs?
        | throwError "a generic field representation has no codec family"
      return mkApp codecs (toExpr index)
  | .nominal twin arguments => do
      let argumentCodecs ← arguments.mapM (·.codec codecs?)
      mkAppM (twin ++ `codec) argumentCodecs

/-- Codec syntax of a represented value. -/
partial def FieldRep.codecSyntax (codecs : Array Term := #[]) :
    FieldRep → CommandElabM Term
  | .int (.bits width) signed =>
      ``(VerexIR.Proofs.Codec.specInt
        (IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(VerexIR.Proofs.Codec.specInt IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width twin codec"
  | .bool => ``(VerexIR.Proofs.Codec.bool)
  | .string => ``(VerexIR.Proofs.Codec.string)
  | .address => ``(VerexIR.Proofs.Codec.address)
  | .signer => ``(VerexIR.Proofs.Codec.signer)
  | .bytes => ``(VerexIR.Proofs.Codec.bytes)
  | .unit => ``(VerexIR.Proofs.Codec.unit)
  | .vector element bounded => do
      if bounded then ``(VerexIR.Proofs.Codec.boundedVector $(← element.codecSyntax codecs))
      else ``(VerexIR.Proofs.Codec.vector $(← element.codecSyntax codecs))
  | .parameter index => do
      let some codec := codecs[index]?
        | throwError "a generic twin value has no codec {index}"
      return codec
  | .nominal twin arguments => do
      let arguments ← arguments.mapM (FieldRep.codecSyntax codecs)
      ``($(rootIdent (twin ++ `codec)) $arguments*)

/-- Term syntax of one field's erasure, from its projection. -/
def FieldRep.eraseSyntax (rep : FieldRep) (projected : Term)
    (codecs : Array Term := #[]) :
    CommandElabM Term :=
  match rep with
  | .int _ _ => ``(VerexIR.RuntimeValue.integer (VerexIR.SpecInt.val $projected))
  | .bool => ``(VerexIR.RuntimeValue.bool $projected)
  | .string => ``(VerexIR.RuntimeValue.string $projected)
  | .address => ``(VerexIR.RuntimeValue.address $projected)
  | .signer => ``(VerexIR.RuntimeValue.signer $projected)
  | .bytes => ``(VerexIR.RuntimeValue.bytes $projected)
  | .unit => ``(VerexIR.RuntimeValue.unit)
  | .vector _ _ => do
      let codec ← rep.codecSyntax codecs
      ``(VerexIR.Proofs.Codec.encode $codec $projected)
  | .parameter index => do
      let some codec := codecs[index]?
        | throwError "a generic twin field has no codec {index}"
      ``(VerexIR.Proofs.Codec.encode $codec $projected)
  | .nominal twin arguments => do
      let arguments ← arguments.mapM (FieldRep.codecSyntax codecs)
      ``($(rootIdent (twin ++ `erase)) $arguments* $projected)

/-- Term syntax of one field's decoder. -/
def FieldRep.decodeSyntax (codecs : Array Term := #[]) :
    FieldRep → CommandElabM Term
  | .int (.bits width) signed =>
      ``(VerexIR.decodeInt? (IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(VerexIR.decodeInt? IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width twin field"
  | .bool => ``(VerexIR.decodeBool?)
  | .string => ``(VerexIR.decodeString?)
  | .address => ``(VerexIR.decodeAddress?)
  | .signer => ``(VerexIR.decodeSigner?)
  | .bytes => ``(VerexIR.decodeBytes?)
  | .unit => ``(VerexIR.decodeUnit?)
  | .vector element bounded => do
      let codec ← (FieldRep.vector element bounded).codecSyntax codecs
      ``(VerexIR.Proofs.Codec.decode? $codec)
  | .parameter index => do
      let some codec := codecs[index]?
        | throwError "a generic twin field has no decoder codec {index}"
      ``(VerexIR.Proofs.Codec.decode? $codec)
  | .nominal twin arguments => do
      let arguments ← arguments.mapM (FieldRep.codecSyntax codecs)
      ``($(rootIdent (twin ++ `decode?)) $arguments*)

/-- Term syntax of one field's default value. -/
private def FieldRep.defaultSyntax (parameters : Array Ident := #[]) :
    FieldRep → CommandElabM Term
  | .int _ _ => ``(⟨0, by decide⟩)
  | .bool => ``(Bool.false)
  | .string | .address | .signer => ``("")
  | .bytes => ``(#[])
  | .unit => ``(())
  | .vector _ bounded => if bounded then ``(default) else ``(#[])
  | .parameter index => do
      let some parameterType := parameters[index]?
        | throwError "a generic twin default has no type parameter {index}"
      ``((default : $parameterType))
  | .nominal twin arguments => do
      let type ← FieldRep.typeSyntax parameters (.nominal twin arguments)
      ``((default : $type))

/-- Emit a type-parameterized structure twin.  Its fields stay in their
native Lean representations; only `erase`/`decode?` mention `RuntimeValue`,
through the codecs supplied for the declaration's type arguments. -/
private def emitGenericStructTwin (info : TwinInfo) : CommandElabM Unit := do
  if (← getEnv).contains info.twin then return
  let twin := rootIdent info.twin
  let typeParameters := (Array.range info.typeParameterCount).map fun index =>
    mkIdent (Name.mkSimple s!"T{index}")
  let typeBinders ← typeParameters.mapM fun parameterType =>
    `(bracketedBinder| ($parameterType:ident : Type))
  let implicitTypeBinders ← typeParameters.mapM fun parameterType =>
    `(bracketedBinder| {$parameterType:ident : Type})
  let codecIds := (Array.range info.typeParameterCount).map fun index =>
    mkIdent (Name.mkSimple s!"codec{index}")
  let codecBinders ← (typeParameters.zip codecIds).mapM fun (parameterType, codec) =>
    `(bracketedBinder| ($codec:ident :
      VerexIR.Proofs.Codec $parameterType VerexIR.RuntimeValue))
  let codecTerms : Array Term := codecIds.map fun codec => ⟨codec.raw⟩
  let twinType ← `(term| $twin:ident $typeParameters*)
  let fieldIds := info.fields.map fun (name, _) => mkIdent (Name.mkSimple name)
  let fieldTypes ← info.fields.mapM fun (_, rep) => rep.typeSyntax typeParameters
  elabCommand (← `(@[verex_twin] structure $twin:ident $typeBinders* where
    $[( $fieldIds:ident : $fieldTypes:term)]*))

  let value := mkIdent `value
  let eraseName := rootIdent (info.twin ++ `erase)
  let erasures ← info.fields.mapM fun (name, rep) => do
    rep.eraseSyntax
      (← ``($(rootIdent (info.twin ++ Name.mkSimple name)) $value)) codecTerms
  elabCommand (← `(def $eraseName:ident $implicitTypeBinders* $codecBinders*
      ($value:ident : $twinType) : VerexIR.RuntimeValue :=
    VerexIR.RuntimeValue.nominal
      ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
      #[$erasures,*]))

  let decodeName := rootIdent (info.twin ++ `decode?)
  let operands := info.fields.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"operand{index}")
  let decodedIds := info.fields.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"decoded{index}")
  let mut chain ← ``(some ⟨$decodedIds,*⟩)
  for index in (List.range info.fields.size).reverse do
    let decoder ← info.fields[index]!.2.decodeSyntax codecTerms
    chain ← ``(Option.bind ($decoder $(operands[index]!))
      fun $(decodedIds[index]!):ident => $chain)
  elabCommand (← `(def $decodeName:ident $implicitTypeBinders* $codecBinders* :
      VerexIR.RuntimeValue → Option $twinType
    | VerexIR.RuntimeValue.nominal
        ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
        none fields =>
        match fields.toList with
        | [$operands,*] => $chain
        | _ => none
    | _ => none))

  let roundtripName := rootIdent (info.twin ++ `decode?_erase)
  elabTheorem (← `(command| @[simp] theorem $roundtripName:ident
      $implicitTypeBinders* $codecBinders* ($value:ident : $twinType) :
      $decodeName $codecIds* ($eraseName $codecIds* $value) = some $value := by
    cases $value:ident
    simp [$eraseName:ident, $decodeName:ident]))
  /- Normalize a correctly tagged runtime literal without unfolding the
  whole decoder at every storage read.  Quantifying the raw operands makes
  this applicable both before and after a concrete field codec has reduced
  its `encode` projection. -/
  let operandBinders ← operands.mapM fun operand =>
    `(bracketedBinder| ($operand:ident : VerexIR.RuntimeValue))
  let literal ← ``(VerexIR.RuntimeValue.nominal
    ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
    #[$operands,*])
  let literalRoundtripName := rootIdent (info.twin ++ `decode?_literal)
  elabTheorem (← `(command| @[simp] theorem $literalRoundtripName:ident
      $implicitTypeBinders* $codecBinders* $operandBinders* :
      $decodeName $codecIds* $literal = $chain := by
    simp [$decodeName:ident]))

  let fieldVars := info.fields.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"field{index}")
  let fieldBinders ← info.fields.mapIdxM fun index (_, rep) => do
    `(bracketedBinder| ($(fieldVars[index]!):ident :
      $(← rep.typeSyntax typeParameters)))
  let literalErasures ← info.fields.mapIdxM fun index (_, rep) =>
    rep.eraseSyntax fieldVars[index]! codecTerms

  let codecName := rootIdent (info.twin ++ `codec)
  elabCommand (← `(def $codecName:ident $implicitTypeBinders* $codecBinders* :
      VerexIR.Proofs.Codec $twinType VerexIR.RuntimeValue where
    encode := $eraseName $codecIds*
    decode? := $decodeName $codecIds*
    decode_encode := $roundtripName $codecIds*))

  let eraseEqName := rootIdent (info.twin ++ `erase_eq_erase)
  elabTheorem (← `(command| @[simp] theorem $eraseEqName:ident
      $implicitTypeBinders* $codecBinders* (left right : $twinType) :
      $eraseName $codecIds* left = $eraseName $codecIds* right ↔ left = right :=
    VerexIR.Proofs.Codec.encode_eq_encode ($codecName $codecIds*) left right))

  let eraseLiteralName := rootIdent (info.twin ++ `erase_mk)
  elabTheorem (← `(command| theorem $eraseLiteralName:ident
      $implicitTypeBinders* $codecBinders* $fieldBinders* :
      $eraseName $codecIds* (⟨$fieldVars,*⟩ : $twinType) =
        VerexIR.RuntimeValue.nominal
          ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
          #[$literalErasures,*] := rfl))

  let inhabitedBinders ← typeParameters.mapM fun parameterType =>
    `(bracketedBinder| [Inhabited $parameterType])
  let defaults ← info.fields.mapM fun (_, rep) =>
    rep.defaultSyntax typeParameters
  let inhabitedName := rootIdent (info.twin ++ `instInhabited)
  elabCommand (← `(instance $inhabitedName:ident
      $implicitTypeBinders* $inhabitedBinders* :
      Inhabited $twinType := ⟨⟨$defaults,*⟩⟩))
  elabCommand (← `(attribute [irreducible] $decodeName:ident))

/-- Emit one plain struct twin's declarations: the tagged structure, `erase`, `decode?`,
their roundtrips, and `Inhabited`. -/
private def emitStructTwin (info : TwinInfo) : CommandElabM Unit := do
  if info.typeParameterCount != 0 then
    return ← emitGenericStructTwin info
  if (← getEnv).contains info.twin then return
  let twin := rootIdent info.twin
  let fieldIds := info.fields.map fun (name, _) => mkIdent (Name.mkSimple name)
  let fieldTypes ← info.fields.mapM fun (_, rep) => rep.typeSyntax
  elabCommand (← `(@[verex_twin] structure $twin:ident where
    $[($fieldIds:ident : $fieldTypes:term)]*))
  -- erase
  let eraseName := rootIdent (info.twin ++ `erase)
  let value := mkIdent `value
  let erasures ← info.fields.mapM fun (name, rep) => do
    rep.eraseSyntax (← ``($(rootIdent (info.twin ++ Name.mkSimple name)) $value))
  elabCommand (← `(def $eraseName:ident ($value:ident : $twin) :
      VerexIR.RuntimeValue :=
    VerexIR.RuntimeValue.nominal
      ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
      #[$erasures,*]))
  -- decode?
  let decodeName := rootIdent (info.twin ++ `decode?)
  let operands := info.fields.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"operand{index}")
  let mut chain ← ``(some ⟨$fieldIds,*⟩)
  for index in (List.range info.fields.size).reverse do
    let decoder ← info.fields[index]!.2.decodeSyntax
    chain ← ``(Option.bind ($decoder $(operands[index]!))
      fun $(fieldIds[index]!):ident => $chain)
  elabCommand (← `(def $decodeName:ident :
      VerexIR.RuntimeValue → Option $twin
    | VerexIR.RuntimeValue.nominal
        ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
        none fields =>
        match fields.toList with
        | [$operands,*] => $chain
        | _ => none
    | _ => none))
  -- roundtrips
  let roundtripName := rootIdent (info.twin ++ `decode?_erase)
  elabTheorem (← `(command| @[simp] theorem $roundtripName:ident
      ($value:ident : $twin) :
      $decodeName ($eraseName $value) = some $value := by
    cases $value:ident
    simp [$eraseName:ident, $decodeName:ident]))
  /- Denotation normalization may already have unfolded `erase` before a
  result reaches its decoder.  Keep the constructor-level spelling as a
  simp theorem too, so decoding uses the certificate rather than reopening
  every certified integer's range test. -/
  let literal ← ``(VerexIR.RuntimeValue.nominal
    ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
    #[$erasures,*])
  let literalRoundtripName :=
    rootIdent (info.twin ++ `decode?_literal)
  elabTheorem (← `(command| @[simp] theorem $literalRoundtripName:ident
      ($value:ident : $twin) :
      $decodeName $literal = some $value :=
    $roundtripName $value))
  elabCommand (← `(attribute [vir_denote_norm] $roundtripName:ident
    $literalRoundtripName:ident))
  /- The decoder belongs to the simp phase, not to reduction.  Symbolic
  execution that unfolds it splits the range test of every certified
  integer it reaches, and the branch that assumes the test failed cannot be
  refuted later: a `SpecInt` nested inside a twin never becomes a range
  hypothesis.  Sealed, the decoder stays folded until the closing executes
  it against the range facts the contract carries. -/
  elabCommand (← `(attribute [irreducible] $decodeName:ident))
  let codecName := rootIdent (info.twin ++ `codec)
  elabCommand (← `(def $codecName:ident :
      VerexIR.Proofs.Codec $twin VerexIR.RuntimeValue where
    encode := $eraseName
    decode? := $decodeName
    decode_encode := $roundtripName))
  /- The erasure of a literal twin is its runtime image; the erasure of a
  variable is an atom.  Only the literal equation joins the inventory: an
  unfolded variable erasure would bury the roundtrips under the nominal
  spelling, and with nested twins no literal roundtrip meets it again. -/
  let fieldVars := info.fields.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"field{index}")
  let fieldBinders ← info.fields.mapIdxM fun index (_, rep) => do
    `(bracketedBinder| ($(fieldVars[index]!):ident : $(← rep.typeSyntax)))
  let literalErasures ← info.fields.mapIdxM fun index (_, rep) =>
    rep.eraseSyntax fieldVars[index]!
  let eraseLiteralName := rootIdent (info.twin ++ `erase_mk)
  elabTheorem (← `(command| theorem $eraseLiteralName:ident $fieldBinders* :
      $eraseName (⟨$fieldVars,*⟩ : $twin) =
        VerexIR.RuntimeValue.nominal
          ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
          #[$literalErasures,*] := rfl))
  -- Inhabited
  let defaults ← info.fields.mapM fun (_, rep) => rep.defaultSyntax
  elabCommand (← `(instance : Inhabited $twin := ⟨⟨$defaults,*⟩⟩))

/-- A constructor applied to the supplied terms, usable both as a pattern
and as an expression. -/
private def constructorTerm (constructor : Ident) (arguments : Array Term) :
    CommandElabM Term := do
  if arguments.isEmpty then return constructor
  `($constructor:ident $arguments*)

/-- Emit an enum twin as a genuine Lean inductive.  Consequently every
native value is a well-formed source variant, while its codec still erases
to the exact nominal runtime layout consumed by the interpreter. -/
private def emitEnumTwin (info : TwinInfo) : CommandElabM Unit := do
  if (← getEnv).contains info.twin then return
  let twin := rootIdent info.twin
  let typeParameters := (Array.range info.typeParameterCount).map fun index =>
    mkIdent (Name.mkSimple s!"T{index}")
  let typeBinders ← typeParameters.mapM fun parameterType =>
    `(bracketedBinder| ($parameterType:ident : Type))
  let implicitTypeBinders ← typeParameters.mapM fun parameterType =>
    `(bracketedBinder| {$parameterType:ident : Type})
  let codecIds := (Array.range info.typeParameterCount).map fun index =>
    mkIdent (Name.mkSimple s!"codec{index}")
  let codecBinders ← (typeParameters.zip codecIds).mapM fun (parameterType, codec) =>
    `(bracketedBinder| ($codec:ident :
      VerexIR.Proofs.Codec $parameterType VerexIR.RuntimeValue))
  let codecTerms : Array Term := codecIds.map fun codec => ⟨codec.raw⟩
  let twinType ← `(term| $twin:ident $typeParameters*)
  let mut constructors : Array (TSyntax ``Lean.Parser.Command.ctor) := #[]
  for variant in info.variants do
    let constructor := mkIdent (Name.mkSimple variant.name)
    let binders ← variant.fields.mapM fun (field, rep) => do
      `(bracketedBinder| ($(mkIdent (Name.mkSimple field)):ident : $(← rep.typeSyntax typeParameters)))
    constructors := constructors.push
      (← `(Lean.Parser.Command.ctor| | $constructor:ident $[$binders]*))
  elabCommand (← `(@[verex_twin] inductive $twin:ident $typeBinders* where
    $[$constructors:ctor]*))

  let value := mkIdent `value
  let eraseName := rootIdent (info.twin ++ `erase)
  let mut eraseArms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  for variant in info.variants do
    let constructor := rootIdent (info.twin ++ Name.mkSimple variant.name)
    let variables := variant.fields.mapIdx fun index _ =>
      mkIdent (Name.mkSimple s!"field{index}")
    let pattern ← constructorTerm constructor variables
    let erasures ← variant.fields.mapIdxM fun index (_, rep) =>
      rep.eraseSyntax variables[index]! codecTerms
    let runtime ← `(VerexIR.RuntimeValue.nominal
      ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
      (some $(quote variant.name)) #[$erasures,*])
    eraseArms := eraseArms.push
      (← `(Lean.Parser.Term.matchAltExpr| | $pattern:term => $runtime))
  elabCommand (← `(def $eraseName:ident $implicitTypeBinders* $codecBinders*
      ($value:ident : $twinType) :
      VerexIR.RuntimeValue :=
    match $value:ident with $eraseArms:matchAlt*))
  -- The erasure of each constructor is its literal, so a decoded variant's
  -- erasure meets a clause's literal without unfolding the match.
  for variant in info.variants do
    let constructor := rootIdent (info.twin ++ Name.mkSimple variant.name)
    let variables := variant.fields.mapIdx fun index _ =>
      mkIdent (Name.mkSimple s!"field{index}")
    let binders ← variant.fields.mapIdxM fun index (_, rep) => do
      `(bracketedBinder| ($(variables[index]!):ident : $(← rep.typeSyntax typeParameters)))
    let constructed ← constructorTerm constructor variables
    let erasures ← variant.fields.mapIdxM fun index (_, rep) =>
      rep.eraseSyntax variables[index]! codecTerms
    let literalName := rootIdent (info.twin ++ Name.mkSimple s!"erase_{variant.name}")
    elabTheorem (← `(command| @[simp] theorem $literalName:ident $implicitTypeBinders* $codecBinders*
        $binders* :
        $eraseName $codecIds* $constructed = VerexIR.RuntimeValue.nominal
          ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
          (some $(quote variant.name)) #[$erasures,*] := rfl))

  let decodeName := rootIdent (info.twin ++ `decode?)
  let variantName := mkIdent `variant
  let fieldsName := mkIdent `fields
  let mut variantChain ← ``(none)
  for variant in info.variants.reverse do
    let constructor := rootIdent (info.twin ++ Name.mkSimple variant.name)
    let operands := variant.fields.mapIdx fun index _ =>
      mkIdent (Name.mkSimple s!"operand{index}")
    let fieldIds := variant.fields.mapIdx fun index _ =>
      mkIdent (Name.mkSimple s!"field{index}")
    let constructed ← constructorTerm constructor fieldIds
    let mut decoded ← ``(some $constructed)
    for index in (List.range variant.fields.size).reverse do
      let decoder ← variant.fields[index]!.2.decodeSyntax codecTerms
      decoded ← ``(Option.bind ($decoder $(operands[index]!))
        fun $(fieldIds[index]!):ident => $decoded)
    let row ← `(match ($fieldsName:ident).toList with
      | [$operands,*] => $decoded
      | _ => none)
    variantChain ← ``(if $variantName:ident == $(quote variant.name) then
      $row else $variantChain)
  elabCommand (← `(def $decodeName:ident $implicitTypeBinders* $codecBinders* :
      VerexIR.RuntimeValue → Option $twinType
    | VerexIR.RuntimeValue.nominal
        ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
        (some $variantName:ident) $fieldsName:ident => $variantChain
    | _ => none))

  let roundtripName := rootIdent (info.twin ++ `decode?_erase)
  elabTheorem (← `(command| @[simp] theorem $roundtripName:ident
      $implicitTypeBinders* $codecBinders* ($value:ident : $twinType) :
      $decodeName $codecIds* ($eraseName $codecIds* $value) = some $value := by
    cases $value:ident <;> simp [$eraseName:ident, $decodeName:ident]))

  for variant in info.variants do
    -- A raw constructor literal must select its variant before field
    -- decoding. Native field projections need not unify with an erased
    -- SpecInt metavariable, so also expose a theorem over raw operands.
    let operands := variant.fields.mapIdx fun index _ =>
      mkIdent (Name.mkSimple s!"operand{index}")
    let operandBinders ← operands.mapM fun operand =>
      `(bracketedBinder| ($operand:ident : VerexIR.RuntimeValue))
    let constructor := rootIdent (info.twin ++ Name.mkSimple variant.name)
    let variables := variant.fields.mapIdx fun index _ =>
      mkIdent (Name.mkSimple s!"field{index}")
    let binders ← variant.fields.mapIdxM fun index (_, rep) => do
      `(bracketedBinder| ($(variables[index]!):ident : $(← rep.typeSyntax typeParameters)))
    let constructed ← constructorTerm constructor variables
    let mut chain ← ``(some $constructed)
    for index in (List.range variant.fields.size).reverse do
      let decoder ← variant.fields[index]!.2.decodeSyntax codecTerms
      chain ← ``(Option.bind ($decoder $(operands[index]!))
        fun $(variables[index]!):ident => $chain)
    let rawLiteralName := rootIdent
      (info.twin ++ Name.mkSimple s!"decode?_{variant.name}_literal")
    elabTheorem (← `(command| @[simp] theorem $rawLiteralName:ident
        $implicitTypeBinders* $codecBinders* $operandBinders* :
        $decodeName $codecIds* (VerexIR.RuntimeValue.nominal
          ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
          (some $(quote variant.name)) #[$operands,*]) = $chain := by
      simp [$decodeName:ident]))
    let erasures ← variant.fields.mapIdxM fun index (_, rep) =>
      rep.eraseSyntax variables[index]! codecTerms
    let runtime ← `(VerexIR.RuntimeValue.nominal
      ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
      (some $(quote variant.name)) #[$erasures,*])
    let eraseCtorName := rootIdent
      (info.twin ++ Name.mkSimple s!"erase_${variant.name}")
    elabTheorem (← `(command| theorem $eraseCtorName:ident $implicitTypeBinders* $codecBinders* $binders* :
      $eraseName $codecIds* $constructed = $runtime := rfl))
    let literalName := rootIdent
      (info.twin ++ Name.mkSimple s!"decode?_${variant.name}")
    elabTheorem (← `(command| @[simp] theorem $literalName:ident
      $implicitTypeBinders* $codecBinders* $binders* :
      $decodeName $codecIds* $runtime = some $constructed := by
        simp [$decodeName:ident]))

  elabCommand (← `(attribute [irreducible] $decodeName:ident))
  let codecName := rootIdent (info.twin ++ `codec)
  elabCommand (← `(def $codecName:ident $implicitTypeBinders* $codecBinders* :
      VerexIR.Proofs.Codec $twinType VerexIR.RuntimeValue where
    encode := $eraseName $codecIds*
    decode? := $decodeName $codecIds*
    decode_encode := $roundtripName $codecIds*))

  let eraseEqName := rootIdent (info.twin ++ `erase_eq_erase)
  elabTheorem (← `(command| @[simp] theorem $eraseEqName:ident
      $implicitTypeBinders* $codecBinders* (left right : $twinType) :
      $eraseName $codecIds* left = $eraseName $codecIds* right ↔ left = right :=
    VerexIR.Proofs.Codec.encode_eq_encode ($codecName $codecIds*) left right))

  let first := info.variants[0]!
  let constructor := rootIdent (info.twin ++ Name.mkSimple first.name)
  let defaults ← first.fields.mapM fun (_, rep) => rep.defaultSyntax typeParameters
  let default ← constructorTerm constructor defaults
  let inhabitedBinders ← typeParameters.mapM fun parameterType =>
    `(bracketedBinder| [Inhabited $parameterType])
  let inhabitedName := rootIdent (info.twin ++ `instInhabited)
  elabCommand (← `(instance $inhabitedName:ident $implicitTypeBinders* $inhabitedBinders* :
    Inhabited $twinType := ⟨$default⟩))

/-- Emit one twin's declarations. -/
private def emitTwin (info : TwinInfo) : CommandElabM Unit := do
  if info.variants.isEmpty then emitStructTwin info else emitEnumTwin info

/-- Ensure every twin of a registered unit exists, and return them. -/
def ensureSpecTypes (segments : Array String) (unit : ValidatedUnit) :
    CommandElabM (Array TwinInfo) := do
  let twins := twinInfos segments unit
  for info in twins do
    emitTwin info
  return twins

end VerexSL.SpecTypes
