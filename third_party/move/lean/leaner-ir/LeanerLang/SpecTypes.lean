-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Registry
import LeanerIR.Proofs.Typed
import LeanerIR.Proofs.Denotation
import LeanerIR.Proofs.SimpAttrs
import LeanerIR.Proofs.Plain

/- Nested vector fields expose their element codecs only after the outer
vector's pointwise plainness rule fires. Keep these reductions in that phase. -/
attribute [leaner_plain] LeanerIR.Proofs.Codec.vector_encode
  LeanerIR.Proofs.Codec.boundedVector_encode
  LeanerIR.Proofs.Codec.specInt LeanerIR.Proofs.Codec.bool
  LeanerIR.Proofs.Codec.string LeanerIR.Proofs.Codec.address
  LeanerIR.Proofs.Codec.signer LeanerIR.Proofs.Codec.bytes
  LeanerIR.Proofs.Codec.unit

/-!
# Generated typed twins of LIR nominal declarations

Each supported nominal declaration of a registered unit gets a Lean twin: a
structure or inductive whose fields carry the certified specification
representation of the declared field types (`SpecInt` for integers, plain
Lean scalars for the rest, nested twins for nominal fields), together with its
`erase`/`decode?` pair, their roundtrip, and — for a `key` struct whose
family appears in the type table — the keyed accessors a contract clause
reads storage through.

Generation is driven by the validated unit, not by surface syntax, so any
frontend that registers a unit gets the same twins.  A declaration with
content the representation does not support yet (closures or references)
gets no twin; a clause or family that needs the missing twin is a
loud diagnostic downstream, never a silent degradation.
-/

namespace LeanerLang.SpecTypes

open Lean Meta Elab Command
open LeanerIR (IntWidth QualifiedName TypeId NamespaceId)
open LeanerIR.Validation (ValidatedUnit ValidatedNamespace)

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

/-- One storable family: a `key` struct together with the type-table entry
that identifies it at global-operation sites. -/
structure FamilyInfo where
  info : TwinInfo
  /-- Index of the family's `.nominal` spelling in the shared type table. -/
  typeIndex : Nat
  /-- Native arguments applied to the generic twin at this spelling. -/
  arguments : Array FieldRep := #[]
  /-- Root of this spelling's generated `key`/`read`/`get`/`contains`
  declarations. Generic heads can have several distinct type-table
  spellings, so the head name alone is not unique. -/
  accessor : Name := .anonymous
  deriving Repr, Inhabited

/-- The interned spelling of a name id, if any. -/
private def nameOf? (unit : ValidatedUnit) (name : LeanerIR.NameId) :
    Option QualifiedName :=
  unit.tables.names[name.index]?

/-- Locate a struct declaration by qualified name. -/
private def structOf? (unit : ValidatedUnit) (qualified : QualifiedName) :
    Option (Nat × Nat × LeanerIR.StructDecl) := do
  let ns ← unit.namespaces[qualified.namespaceId.index]?
  ns.structs.zipIdx.findSome? fun (declaration, index) =>
    if nameOf? unit declaration.name == some qualified then
      some (qualified.namespaceId.index, index, declaration)
    else none

/-- The representation of one field type, if supported.  Nominal fields name
their twin; the caller is responsible for ensuring the nested twin exists. -/
private partial def fieldRep? (unit : ValidatedUnit) (twinName : QualifiedName → Name)
    (supported : QualifiedName → Bool) (typeId : TypeId)
    (profile : Option LeanerIR.Profile) : Option FieldRep := do
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
through the acyclic declaration graph. -/
private partial def structSupported (unit : ValidatedUnit)
    (qualified : QualifiedName) : Bool :=
  match structOf? unit qualified with
  | none => false
  | some (namespaceIndex, _, declaration) =>
      let fields := declaration.fields ++
        declaration.variants.flatMap fun variant => variant.fields
      declaration.generics.all (fun binder => binder.kind == .typeArg) &&
        fields.all fun field =>
          (fieldRep? unit (fun _ => Name.anonymous) (structSupported unit)
            field.type.typeId (unit.namespaces[namespaceIndex]?.bind (·.profile))).isSome

/-- The generated Lean name of a struct's twin under the registered path. -/
def twinName (segments : Array String) (qualified : QualifiedName) : Name :=
  Name.str (segments.foldl (fun name segment => Name.str name segment)
    .anonymous) qualified.name

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
  -- Declarations are acyclic, so at most `pending.size` rounds settle all.
  for _ in [0:pending.size] do
    for qualified in pending do
      unless emitted.contains qualified do
        let some (namespaceIndex, structIndex, declaration) := structOf? unit qualified
          | continue
        let representFields := fun fields => fields.filterMap fun field => do
          let name ← nameOf? unit field.name
          let rep ← fieldRep? unit (twinName segments)
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
            twin := twinName segments qualified
            namespaceIndex, structIndex
            typeParameterCount := declaration.generics.size
            fields, variants }
          emitted := emitted.push qualified
  return ordered

/-- The storable families of the unit: `key` structs with a twin whose
non-generic nominal spelling appears in the shared type table.  A `key`
struct without a twin, or with an ambiguous spelling, yields no family; the
missing representation surfaces as a loud diagnostic wherever a contract
needs it. -/
def familyInfos (unit : ValidatedUnit) (twins : Array TwinInfo) :
    Array FamilyInfo := Id.run do
  let mut families : Array FamilyInfo := #[]
  for info in twins do
    let some (_, _, declaration) := structOf? unit info.qualified | continue
    unless declaration.abilities.contains .key do continue
    let spellings := unit.tables.types.zipIdx.filterMap fun (ty, index) => do
      match ty with
      | .nominal name arguments =>
          guard (nameOf? unit name == some info.qualified)
          guard (arguments.size == info.typeParameterCount)
          let arguments ← arguments.mapM fun argument => match argument with
            | .typeArg value => fieldRep? unit
                (fun qualified => (twins.find? (·.qualified == qualified)).map
                  (·.twin) |>.getD .anonymous)
                (structSupported unit) value.typeId
                (unit.namespaces[info.namespaceIndex]?.bind (·.profile))
            | .const _ | .lifetime _ | .evidence _ => none
          some (index, arguments)
      | _ => none
    for (typeIndex, arguments) in spellings do
      let accessor := if info.typeParameterCount == 0 then info.twin else
        info.twin ++ Name.mkSimple s!"family{typeIndex}"
      families := families.push { info, typeIndex, arguments, accessor }
  return families

/-! ## Command generation -/

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
      ``(LeanerIR.SpecInt (IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(LeanerIR.SpecInt IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width twin field"
  | .bool => ``(Bool)
  | .string | .address | .signer => ``(String)
  | .bytes => ``(Array UInt8)
  | .unit => ``(Unit)
  | .vector element bounded => do
      if bounded then ``(LeanerIR.SpecVector $(← element.typeSyntax parameters))
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
  | .bits width => mkApp (mkConst ``LeanerIR.IntWidth.bits) (toExpr width)
  | .pointer => mkConst ``LeanerIR.IntWidth.pointer
  | .unbounded => mkConst ``LeanerIR.IntWidth.unbounded

partial def FieldRep.leanType (rep : FieldRep) (carrier? : Option Lean.Expr) :
    MetaM Lean.Expr := do
  match rep with
  | .int width signed => mkAppM ``LeanerIR.SpecInt #[intWidthExpr width, toExpr signed]
  | .bool => return mkConst ``Bool
  | .string | .address | .signer => return mkConst ``String
  | .bytes => mkAppM ``Array #[mkConst ``UInt8]
  | .unit => return mkConst ``Unit
  | .vector element bounded =>
      mkAppM (if bounded then ``LeanerIR.SpecVector else ``Array) #[← element.leanType carrier?]
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
      mkAppM ``LeanerIR.Proofs.Codec.specInt #[intWidthExpr width, toExpr signed]
  | .bool => return mkConst ``LeanerIR.Proofs.Codec.bool
  | .string => return mkConst ``LeanerIR.Proofs.Codec.string
  | .address => return mkConst ``LeanerIR.Proofs.Codec.address
  | .signer => return mkConst ``LeanerIR.Proofs.Codec.signer
  | .bytes => return mkConst ``LeanerIR.Proofs.Codec.bytes
  | .unit => return mkConst ``LeanerIR.Proofs.Codec.unit
  | .vector element bounded =>
      mkAppM (if bounded then ``LeanerIR.Proofs.Codec.boundedVector else
        ``LeanerIR.Proofs.Codec.vector) #[← element.codec codecs?]
  | .parameter index =>
      let some codecs := codecs?
        | throwError "a generic field representation has no codec family"
      return mkApp codecs (toExpr index)
  | .nominal twin arguments => do
      let argumentCodecs ← arguments.mapM (·.codec codecs?)
      mkAppM (twin ++ `codec) argumentCodecs

/-- Applied native type represented by this resource-family spelling. -/
def FamilyInfo.leanType (family : FamilyInfo) (carrier? : Option Lean.Expr) :
    MetaM Lean.Expr :=
  (FieldRep.nominal family.info.twin family.arguments).leanType carrier?

/-- Applied erasure represented by this resource-family spelling. -/
def FamilyInfo.erase (family : FamilyInfo) (codecs? : Option Lean.Expr) :
    MetaM Lean.Expr := do
  let argumentCodecs ← family.arguments.mapM (·.codec codecs?)
  mkAppM (family.info.twin ++ `erase) argumentCodecs

/-- Codec arguments expected by a parameterized family accessor, in outer
carrier-index order. -/
def FamilyInfo.outerCodecs (family : FamilyInfo) (codecs? : Option Lean.Expr) :
    MetaM (Array Lean.Expr) := do
  let count := family.arguments.foldl
    (fun count argument => max count argument.parameterCount) 0
  if count == 0 then return #[]
  let some codecs := codecs?
    | throwError "a parameterized resource family has no codec family"
  return (Array.range count).map fun index => mkApp codecs (toExpr index)

def FamilyInfo.accessorName (family : FamilyInfo) (suffix : Name) : Name :=
  family.accessor ++ suffix

/-- Codec syntax of a represented value. -/
partial def FieldRep.codecSyntax (codecs : Array Term := #[]) :
    FieldRep → CommandElabM Term
  | .int (.bits width) signed =>
      ``(LeanerIR.Proofs.Codec.specInt
        (IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(LeanerIR.Proofs.Codec.specInt IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width twin codec"
  | .bool => ``(LeanerIR.Proofs.Codec.bool)
  | .string => ``(LeanerIR.Proofs.Codec.string)
  | .address => ``(LeanerIR.Proofs.Codec.address)
  | .signer => ``(LeanerIR.Proofs.Codec.signer)
  | .bytes => ``(LeanerIR.Proofs.Codec.bytes)
  | .unit => ``(LeanerIR.Proofs.Codec.unit)
  | .vector element bounded => do
      if bounded then ``(LeanerIR.Proofs.Codec.boundedVector $(← element.codecSyntax codecs))
      else ``(LeanerIR.Proofs.Codec.vector $(← element.codecSyntax codecs))
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
  | .int _ _ => ``(LeanerIR.RuntimeValue.integer (LeanerIR.SpecInt.val $projected))
  | .bool => ``(LeanerIR.RuntimeValue.bool $projected)
  | .string => ``(LeanerIR.RuntimeValue.string $projected)
  | .address => ``(LeanerIR.RuntimeValue.address $projected)
  | .signer => ``(LeanerIR.RuntimeValue.signer $projected)
  | .bytes => ``(LeanerIR.RuntimeValue.bytes $projected)
  | .unit => ``(LeanerIR.RuntimeValue.unit)
  | .vector _ _ => do
      let codec ← rep.codecSyntax codecs
      ``(LeanerIR.Proofs.Codec.encode $codec $projected)
  | .parameter index => do
      let some codec := codecs[index]?
        | throwError "a generic twin field has no codec {index}"
      ``(LeanerIR.Proofs.Codec.encode $codec $projected)
  | .nominal twin arguments => do
      let arguments ← arguments.mapM (FieldRep.codecSyntax codecs)
      ``($(rootIdent (twin ++ `erase)) $arguments* $projected)

/-- Term syntax of one field's decoder. -/
def FieldRep.decodeSyntax (codecs : Array Term := #[]) :
    FieldRep → CommandElabM Term
  | .int (.bits width) signed =>
      ``(LeanerIR.decodeInt? (IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(LeanerIR.decodeInt? IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width twin field"
  | .bool => ``(LeanerIR.decodeBool?)
  | .string => ``(LeanerIR.decodeString?)
  | .address => ``(LeanerIR.decodeAddress?)
  | .signer => ``(LeanerIR.decodeSigner?)
  | .bytes => ``(LeanerIR.decodeBytes?)
  | .unit => ``(LeanerIR.decodeUnit?)
  | .vector element bounded => do
      let codec ← (FieldRep.vector element bounded).codecSyntax codecs
      ``(LeanerIR.Proofs.Codec.decode? $codec)
  | .parameter index => do
      let some codec := codecs[index]?
        | throwError "a generic twin field has no decoder codec {index}"
      ``(LeanerIR.Proofs.Codec.decode? $codec)
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
      LeanerIR.Proofs.Codec $parameterType LeanerIR.RuntimeValue))
  let codecTerms : Array Term := codecIds.map fun codec => ⟨codec.raw⟩
  let twinType ← `(term| $twin:ident $typeParameters*)
  let fieldIds := info.fields.map fun (name, _) => mkIdent (Name.mkSimple name)
  let fieldTypes ← info.fields.mapM fun (_, rep) => rep.typeSyntax typeParameters
  elabCommand (← `(@[leaner_twin] structure $twin:ident $typeBinders* where
    $[( $fieldIds:ident : $fieldTypes:term)]*))

  let value := mkIdent `value
  let eraseName := rootIdent (info.twin ++ `erase)
  let erasures ← info.fields.mapM fun (name, rep) => do
    rep.eraseSyntax
      (← ``($(rootIdent (info.twin ++ Name.mkSimple name)) $value)) codecTerms
  elabCommand (← `(def $eraseName:ident $implicitTypeBinders* $codecBinders*
      ($value:ident : $twinType) : LeanerIR.RuntimeValue :=
    LeanerIR.RuntimeValue.nominal
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
      LeanerIR.RuntimeValue → Option $twinType
    | LeanerIR.RuntimeValue.nominal
        ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
        none fields =>
        match fields.toList with
        | [$operands,*] => $chain
        | _ => none
    | _ => none))

  let roundtripName := rootIdent (info.twin ++ `decode?_erase)
  elabCommand (← `(@[simp] theorem $roundtripName:ident
      $implicitTypeBinders* $codecBinders* ($value:ident : $twinType) :
      $decodeName $codecIds* ($eraseName $codecIds* $value) = some $value := by
    cases $value:ident
    simp [$eraseName:ident, $decodeName:ident]))
  /- Normalize a correctly tagged runtime literal without unfolding the
  whole decoder at every storage read.  Quantifying the raw operands makes
  this applicable both before and after a concrete field codec has reduced
  its `encode` projection. -/
  let operandBinders ← operands.mapM fun operand =>
    `(bracketedBinder| ($operand:ident : LeanerIR.RuntimeValue))
  let literal ← ``(LeanerIR.RuntimeValue.nominal
    ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
    #[$operands,*])
  let literalRoundtripName := rootIdent (info.twin ++ `decode?_literal)
  elabCommand (← `(@[simp] theorem $literalRoundtripName:ident
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
  let mapName := rootIdent (info.twin ++ `decode?_map_erase)
  elabCommand (← `(@[simp] theorem $mapName:ident
      $implicitTypeBinders* $codecBinders* (contents : Option $twinType) :
      Option.bind (Option.map ($eraseName $codecIds*) contents)
        ($decodeName $codecIds*) = contents :=
    LeanerIR.map_erase_bind_decode ($roundtripName $codecIds*) contents))

  let codecName := rootIdent (info.twin ++ `codec)
  elabCommand (← `(def $codecName:ident $implicitTypeBinders* $codecBinders* :
      LeanerIR.Proofs.Codec $twinType LeanerIR.RuntimeValue where
    encode := $eraseName $codecIds*
    decode? := $decodeName $codecIds*
    decode_encode := $roundtripName $codecIds*))

  let eraseEqName := rootIdent (info.twin ++ `erase_eq_erase)
  elabCommand (← `(@[simp, lir_data_norm high] theorem $eraseEqName:ident
      $implicitTypeBinders* $codecBinders* (left right : $twinType) :
      $eraseName $codecIds* left = $eraseName $codecIds* right ↔ left = right :=
    LeanerIR.Proofs.Codec.encode_eq_encode ($codecName $codecIds*) left right))

  for index in List.range info.fields.size do
    let (fieldName, rep) := info.fields[index]!
    let selectName := rootIdent (info.twin ++ Name.mkSimple s!"select_{fieldName}")
    let projected ← ``($(rootIdent (info.twin ++ Name.mkSimple fieldName)) $value)
    let selected ← rep.eraseSyntax projected codecTerms
    elabCommand (← `(theorem $selectName:ident $implicitTypeBinders* $codecBinders*
        ($value:ident : $twinType) (frame : LeanerIR.RuntimeFrame)
        (state : LeanerIR.RuntimeState) :
        (LeanerIR.Proofs.Denotation.NominalFieldLocation.mk
            ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
            $(natLit index)).evaluateSelect?
          #[$eraseName $codecIds* $value] frame state =
        some (LeanerIR.SemanticOperations.GlobalOperationResult.value frame state
          $selected) := by
      simp [$eraseName:ident,
        LeanerIR.Proofs.Denotation.NominalFieldLocation.evaluateSelect?,
        LeanerIR.Proofs.Denotation.liftConstructorEvaluator,
        LeanerIR.SemanticOperations.selectNominalFieldAt?]))

  let eraseLiteralName := rootIdent (info.twin ++ `erase_mk)
  elabCommand (← `(theorem $eraseLiteralName:ident
      $implicitTypeBinders* $codecBinders* $fieldBinders* :
      $eraseName $codecIds* (⟨$fieldVars,*⟩ : $twinType) =
        LeanerIR.RuntimeValue.nominal
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
  elabCommand (← `(attribute [lir_data_norm high]
    $roundtripName:ident $mapName:ident))
  elabCommand (← `(attribute [irreducible] $decodeName:ident))
  elabCommand (← `(attribute [lir_data_norm low]
    $eraseLiteralName:ident $codecName:ident))

/-- Emit one plain struct twin's declarations: the tagged structure, `erase`, `decode?`,
their roundtrips, and `Inhabited`. -/
private def emitStructTwin (info : TwinInfo) : CommandElabM Unit := do
  if info.typeParameterCount != 0 then
    return ← emitGenericStructTwin info
  if (← getEnv).contains info.twin then return
  let twin := rootIdent info.twin
  let fieldIds := info.fields.map fun (name, _) => mkIdent (Name.mkSimple name)
  let fieldTypes ← info.fields.mapM fun (_, rep) => rep.typeSyntax
  elabCommand (← `(@[leaner_twin] structure $twin:ident where
    $[($fieldIds:ident : $fieldTypes:term)]*))
  -- erase
  let eraseName := rootIdent (info.twin ++ `erase)
  let value := mkIdent `value
  let erasures ← info.fields.mapM fun (name, rep) => do
    rep.eraseSyntax (← ``($(rootIdent (info.twin ++ Name.mkSimple name)) $value))
  elabCommand (← `(def $eraseName:ident ($value:ident : $twin) :
      LeanerIR.RuntimeValue :=
    LeanerIR.RuntimeValue.nominal
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
      LeanerIR.RuntimeValue → Option $twin
    | LeanerIR.RuntimeValue.nominal
        ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
        none fields =>
        match fields.toList with
        | [$operands,*] => $chain
        | _ => none
    | _ => none))
  -- roundtrips
  let roundtripName := rootIdent (info.twin ++ `decode?_erase)
  elabCommand (← `(@[simp] theorem $roundtripName:ident
      ($value:ident : $twin) :
      $decodeName ($eraseName $value) = some $value := by
    cases $value:ident
    simp [$eraseName:ident, $decodeName:ident]))
  /- Denotation normalization may already have unfolded `erase` before a
  result reaches its decoder.  Keep the constructor-level spelling as a
  simp theorem too, so decoding uses the certificate rather than reopening
  every certified integer's range test. -/
  let literal ← ``(LeanerIR.RuntimeValue.nominal
    ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
    #[$erasures,*])
  let literalRoundtripName :=
    rootIdent (info.twin ++ `decode?_literal)
  elabCommand (← `(@[simp] theorem $literalRoundtripName:ident
      ($value:ident : $twin) :
      $decodeName $literal = some $value :=
    $roundtripName $value))
  let mapName := rootIdent (info.twin ++ `decode?_map_erase)
  elabCommand (← `(@[simp] theorem $mapName:ident
      (contents : Option $twin) :
      Option.bind (Option.map $eraseName contents) $decodeName = contents :=
    LeanerIR.map_erase_bind_decode $roundtripName contents))
  /- Field selections through the erasure: a proof that reads a field of
  a stored value keeps the twin folded, so the roundtrips still apply to
  the whole once the closing meets it. -/
  for index in List.range info.fields.size do
    let (name, rep) := info.fields[index]!
    let selectName := rootIdent (info.twin ++ Name.mkSimple s!"select_{name}")
    let projected ← ``($(rootIdent (info.twin ++ Name.mkSimple name)) $value)
    let selected ← rep.eraseSyntax projected
    elabCommand (← `(theorem $selectName:ident ($value:ident : $twin)
        (frame : LeanerIR.RuntimeFrame) (state : LeanerIR.RuntimeState) :
        (LeanerIR.Proofs.Denotation.NominalFieldLocation.mk
            ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
            $(natLit index)).evaluateSelect?
          #[$eraseName $value] frame state =
        some (LeanerIR.SemanticOperations.GlobalOperationResult.value frame state
          $selected) := by
      simp [$eraseName:ident,
        LeanerIR.Proofs.Denotation.NominalFieldLocation.evaluateSelect?,
        LeanerIR.Proofs.Denotation.liftConstructorEvaluator,
        LeanerIR.SemanticOperations.selectNominalFieldAt?]))
  -- The roundtrips outrank the decoder's own unfolding wherever both
  -- match, so an erase-image folds instead of reopening its range tests.
  elabCommand (← `(attribute [lir_data_norm high] $roundtripName:ident
    $literalRoundtripName:ident $mapName:ident))
  /- The decoder belongs to the simp phase, not to reduction.  Symbolic
  execution that unfolds it splits the range test of every certified
  integer it reaches, and the branch that assumes the test failed cannot be
  refuted later: a `SpecInt` nested inside a twin never becomes a range
  hypothesis.  Sealed, the decoder stays folded until the closing executes
  it against the range facts the contract carries. -/
  elabCommand (← `(attribute [irreducible] $decodeName:ident))
  let codecName := rootIdent (info.twin ++ `codec)
  elabCommand (← `(def $codecName:ident :
      LeanerIR.Proofs.Codec $twin LeanerIR.RuntimeValue where
    encode := $eraseName
    decode? := $decodeName
    decode_encode := $roundtripName))
  elabCommand (← `(attribute [leaner_plain] $codecName:ident))
  -- Twin fields never contain references.  Pin that representation fact at
  -- the boundary so function finalization does not inspect an erased native
  -- argument as an arbitrary `RuntimeValue` looking for nested loans.
  let noBorrowsName := rootIdent (info.twin ++ `outermostBorrows_erase)
  elabCommand (← `(@[simp] theorem $noBorrowsName:ident
      ($value:ident : $twin) :
      LeanerIR.SemanticOperations.outermostBorrows ($eraseName $value) = #[] := by
    apply LeanerIR.SemanticOperations.Plain.outermostBorrows_eq_empty
    simp only [$eraseName:ident, LeanerIR.Proofs.Codec.vector_encode,
      LeanerIR.Proofs.Codec.boundedVector_encode,
      LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
      LeanerIR.Proofs.Codec.string, LeanerIR.Proofs.Codec.address,
      LeanerIR.Proofs.Codec.signer, LeanerIR.Proofs.Codec.bytes,
      LeanerIR.Proofs.Codec.unit]
    leaner_plain))
  -- A twin's erasure is loan-free: what lets a bracket walk a resource
  -- through its focus alone.
  let plainName := rootIdent (info.twin ++ `plain_erase)
  elabCommand (← `(@[leaner_plain] theorem $plainName:ident ($value:ident : $twin) :
      LeanerIR.SemanticOperations.Plain ($eraseName $value) := by
    simp only [$eraseName:ident, LeanerIR.Proofs.Codec.vector_encode,
      LeanerIR.Proofs.Codec.boundedVector_encode,
      LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
      LeanerIR.Proofs.Codec.string, LeanerIR.Proofs.Codec.address,
      LeanerIR.Proofs.Codec.signer, LeanerIR.Proofs.Codec.bytes,
      LeanerIR.Proofs.Codec.unit]
    leaner_plain))
  let noParameterLoanName :=
    rootIdent (info.twin ++ `parameterLoanLocations_erase)
  elabCommand (← `(@[simp] theorem $noParameterLoanName:ident
      ($value:ident : $twin) :
      LeanerIR.SemanticOperations.parameterLoanLocations
        #[$eraseName $value] = #[] := by
    cases $value:ident
    simp [$eraseName:ident,
      LeanerIR.SemanticOperations.parameterLoanLocations]))
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
  elabCommand (← `(theorem $eraseLiteralName:ident $fieldBinders* :
      $eraseName (⟨$fieldVars,*⟩ : $twin) =
        LeanerIR.RuntimeValue.nominal
          ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩ none
          #[$literalErasures,*] := rfl))
  -- A value the program built decodes by computation, and an erasure the
  -- representation put in a hypothesis has to meet the nominal a split
  -- produced.  Low priority keeps the roundtrips first, so a decode facing
  -- a still-folded erasure collapses through them instead.
  elabCommand (← `(attribute [lir_data_norm] $literalRoundtripName:ident))
  elabCommand (← `(attribute [lir_data_norm low]
    $eraseLiteralName:ident $codecName:ident $noBorrowsName:ident
    $noParameterLoanName:ident))
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
      LeanerIR.Proofs.Codec $parameterType LeanerIR.RuntimeValue))
  let codecTerms : Array Term := codecIds.map fun codec => ⟨codec.raw⟩
  let twinType ← `(term| $twin:ident $typeParameters*)
  let mut constructors : Array (TSyntax ``Lean.Parser.Command.ctor) := #[]
  for variant in info.variants do
    let constructor := mkIdent (Name.mkSimple variant.name)
    let binders ← variant.fields.mapM fun (field, rep) => do
      `(bracketedBinder| ($(mkIdent (Name.mkSimple field)):ident : $(← rep.typeSyntax typeParameters)))
    constructors := constructors.push
      (← `(Lean.Parser.Command.ctor| | $constructor:ident $[$binders]*))
  elabCommand (← `(@[leaner_twin] inductive $twin:ident $typeBinders* where
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
    let runtime ← `(LeanerIR.RuntimeValue.nominal
      ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
      (some $(quote variant.name)) #[$erasures,*])
    eraseArms := eraseArms.push
      (← `(Lean.Parser.Term.matchAltExpr| | $pattern:term => $runtime))
  elabCommand (← `(def $eraseName:ident $implicitTypeBinders* $codecBinders*
      ($value:ident : $twinType) :
      LeanerIR.RuntimeValue :=
    match $value:ident with $eraseArms:matchAlt*))

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
      LeanerIR.RuntimeValue → Option $twinType
    | LeanerIR.RuntimeValue.nominal
        ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
        (some $variantName:ident) $fieldsName:ident => $variantChain
    | _ => none))

  let roundtripName := rootIdent (info.twin ++ `decode?_erase)
  elabCommand (← `(@[simp] theorem $roundtripName:ident
      $implicitTypeBinders* $codecBinders* ($value:ident : $twinType) :
      $decodeName $codecIds* ($eraseName $codecIds* $value) = some $value := by
    cases $value:ident <;> simp [$eraseName:ident, $decodeName:ident]))
  let mapName := rootIdent (info.twin ++ `decode?_map_erase)
  elabCommand (← `(@[simp] theorem $mapName:ident
      $implicitTypeBinders* $codecBinders* (contents : Option $twinType) :
      Option.bind (Option.map ($eraseName $codecIds*) contents)
        ($decodeName $codecIds*) = contents :=
    LeanerIR.map_erase_bind_decode ($roundtripName $codecIds*) contents))

  let mut normalizationNames : Array Ident := #[roundtripName, mapName]
  for variant in info.variants do
    -- A raw constructor literal must select its variant before field
    -- decoding. Native field projections need not unify with an erased
    -- SpecInt metavariable, so also expose a theorem over raw operands.
    let operands := variant.fields.mapIdx fun index _ =>
      mkIdent (Name.mkSimple s!"operand{index}")
    let operandBinders ← operands.mapM fun operand =>
      `(bracketedBinder| ($operand:ident : LeanerIR.RuntimeValue))
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
    elabCommand (← `(@[simp] theorem $rawLiteralName:ident
        $implicitTypeBinders* $codecBinders* $operandBinders* :
        $decodeName $codecIds* (LeanerIR.RuntimeValue.nominal
          ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
          (some $(quote variant.name)) #[$operands,*]) = $chain := by
      simp [$decodeName:ident]))
    normalizationNames := normalizationNames.push rawLiteralName
    let erasures ← variant.fields.mapIdxM fun index (_, rep) =>
      rep.eraseSyntax variables[index]! codecTerms
    let runtime ← `(LeanerIR.RuntimeValue.nominal
      ⟨⟨$(natLit info.namespaceIndex)⟩, $(natLit info.structIndex)⟩
      (some $(quote variant.name)) #[$erasures,*])
    let eraseCtorName := rootIdent
      (info.twin ++ Name.mkSimple s!"erase_${variant.name}")
    elabCommand (← `(theorem $eraseCtorName:ident $implicitTypeBinders* $codecBinders* $binders* :
      $eraseName $codecIds* $constructed = $runtime := rfl))
    let literalName := rootIdent
      (info.twin ++ Name.mkSimple s!"decode?_${variant.name}")
    elabCommand (← `(@[simp] theorem $literalName:ident
      $implicitTypeBinders* $codecBinders* $binders* :
      $decodeName $codecIds* $runtime = some $constructed := by
        simp [$decodeName:ident]))
    normalizationNames := normalizationNames.push literalName
    elabCommand (← `(attribute [lir_data_norm low] $eraseCtorName:ident))

  elabCommand (← `(attribute [lir_data_norm high]
    $normalizationNames:ident*))
  elabCommand (← `(attribute [irreducible] $decodeName:ident))
  let codecName := rootIdent (info.twin ++ `codec)
  elabCommand (← `(def $codecName:ident $implicitTypeBinders* $codecBinders* :
      LeanerIR.Proofs.Codec $twinType LeanerIR.RuntimeValue where
    encode := $eraseName $codecIds*
    decode? := $decodeName $codecIds*
    decode_encode := $roundtripName $codecIds*))

  let eraseEqName := rootIdent (info.twin ++ `erase_eq_erase)
  elabCommand (← `(@[simp, lir_data_norm high] theorem $eraseEqName:ident
      $implicitTypeBinders* $codecBinders* (left right : $twinType) :
      $eraseName $codecIds* left = $eraseName $codecIds* right ↔ left = right :=
    LeanerIR.Proofs.Codec.encode_eq_encode ($codecName $codecIds*) left right))

  let first := info.variants[0]!
  let constructor := rootIdent (info.twin ++ Name.mkSimple first.name)
  let defaults ← first.fields.mapM fun (_, rep) => rep.defaultSyntax typeParameters
  let default ← constructorTerm constructor defaults
  let inhabitedBinders ← typeParameters.mapM fun parameterType =>
    `(bracketedBinder| [Inhabited $parameterType])
  let inhabitedName := rootIdent (info.twin ++ `instInhabited)
  elabCommand (← `(instance $inhabitedName:ident $implicitTypeBinders* $inhabitedBinders* :
    Inhabited $twinType := ⟨$default⟩))
  elabCommand (← `(attribute [lir_data_norm low] $codecName:ident))
  -- Arbitrary parameter codecs need not erase to loan-free values. As for
  -- generic structures, do not assert unconditional plainness for them.
  if info.typeParameterCount != 0 then return

  let plainName := rootIdent (info.twin ++ `plain_erase)
  elabCommand (← `(@[leaner_plain] theorem $plainName:ident
      ($value:ident : $twin) :
      LeanerIR.SemanticOperations.Plain ($eraseName $value) := by
    cases $value:ident <;> simp only [$eraseName:ident] <;> leaner_plain))
  let noBorrowsName := rootIdent (info.twin ++ `outermostBorrows_erase)
  elabCommand (← `(@[simp] theorem $noBorrowsName:ident
      ($value:ident : $twin) :
      LeanerIR.SemanticOperations.outermostBorrows ($eraseName $value) = #[] :=
    LeanerIR.SemanticOperations.Plain.outermostBorrows_eq_empty ($plainName $value)))
  let noParameterLoanName :=
    rootIdent (info.twin ++ `parameterLoanLocations_erase)
  elabCommand (← `(@[simp] theorem $noParameterLoanName:ident
      ($value:ident : $twin) :
      LeanerIR.SemanticOperations.parameterLoanLocations
        #[$eraseName $value] = #[] := by
    cases $value:ident <;>
      simp [$eraseName:ident,
        LeanerIR.SemanticOperations.parameterLoanLocations]))
  elabCommand (← `(attribute [lir_data_norm low]
    $codecName:ident $noBorrowsName:ident $noParameterLoanName:ident))

/-- Emit one twin's declarations. -/
private def emitTwin (info : TwinInfo) : CommandElabM Unit := do
  if info.variants.isEmpty then emitStructTwin info else emitEnumTwin info

/-- Emit one family's keyed accessors: the key, the typed read, the junk-
totalized read a clause uses, and the existence test. -/
private def emitFamily (family : FamilyInfo) : CommandElabM Unit := do
  if (← getEnv).contains (family.accessor ++ `key) then return
  let keyName := rootIdent (family.accessor ++ `key)
  let keyAtName := rootIdent (family.accessor ++ `keyAt)
  elabCommand (← `(def $keyAtName:ident
      (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId))
      (key : LeanerIR.RuntimeValue) :
      LeanerIR.GlobalKey :=
    ⟨⟨$(natLit family.info.namespaceIndex)⟩,
      LeanerIR.SemanticOperations.instantiatedTypeId typeInstantiation
        ⟨$(natLit family.typeIndex)⟩,
      LeanerIR.RuntimeValue.storageKey key⟩))
  elabCommand (← `(def $keyName:ident (key : LeanerIR.RuntimeValue) :
      LeanerIR.GlobalKey := $keyAtName #[] key))
  let parameterCount := family.arguments.foldl
    (fun count argument => max count argument.parameterCount) 0
  let parameters := (Array.range parameterCount).map fun index =>
    mkIdent (Name.mkSimple s!"Outer{index}")
  let typeBinders ← parameters.mapM fun parameterType =>
    `(bracketedBinder| {$parameterType:ident : Type})
  let codecIds := (Array.range parameterCount).map fun index =>
    mkIdent (Name.mkSimple s!"outerCodec{index}")
  let codecBinders ← (parameters.zip codecIds).mapM fun (parameterType, codec) =>
    `(bracketedBinder| ($codec:ident :
      LeanerIR.Proofs.Codec $parameterType LeanerIR.RuntimeValue))
  let codecTerms : Array Term := codecIds.map fun codec => ⟨codec.raw⟩
  let typeArguments ← family.arguments.mapM (FieldRep.typeSyntax parameters)
  let twin := rootIdent family.info.twin
  let valueType ← if typeArguments.isEmpty then
    pure (⟨twin.raw⟩ : Term)
  else `(term| $twin:ident $typeArguments*)
  let argumentCodecs ← family.arguments.mapM (FieldRep.codecSyntax codecTerms)
  let decode := rootIdent (family.info.twin ++ `decode?)
  let readName := rootIdent (family.accessor ++ `read)
  let readAtName := rootIdent (family.accessor ++ `readAt)
  elabCommand (← `(def $readAtName:ident $typeBinders* $codecBinders*
      (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId))
      (globals : LeanerIR.GlobalMap) (key : LeanerIR.RuntimeValue) :
      Option $valueType :=
    Option.bind (globals.lookup ($keyAtName typeInstantiation key))
      ($decode $argumentCodecs*)))
  elabCommand (← `(def $readName:ident $typeBinders* $codecBinders*
      (globals : LeanerIR.GlobalMap) (key : LeanerIR.RuntimeValue) :
      Option $valueType :=
    $readAtName $codecIds* #[] globals key))
  let getName := rootIdent (family.accessor ++ `get)
  let getAtName := rootIdent (family.accessor ++ `getAt)
  let inhabitedBinders ← parameters.mapM fun parameterType =>
    `(bracketedBinder| [Inhabited $parameterType])
  let defaults ← family.info.fields.mapM fun (_, rep) =>
    (rep.instantiate family.arguments).defaultSyntax parameters
  let defaultValue ← `(term| (⟨$defaults,*⟩ : $valueType))
  elabCommand (← `(def $getAtName:ident $typeBinders* $inhabitedBinders*
      $codecBinders* (globals : LeanerIR.GlobalMap)
      (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId))
      (key : LeanerIR.RuntimeValue) : $valueType :=
    ($readAtName $codecIds* typeInstantiation globals key).getD $defaultValue))
  elabCommand (← `(def $getName:ident $typeBinders* $inhabitedBinders*
      $codecBinders* (globals : LeanerIR.GlobalMap)
      (key : LeanerIR.RuntimeValue) : $valueType :=
    $getAtName $codecIds* globals #[] key))
  let containsName := rootIdent (family.accessor ++ `contains)
  let containsAtName := rootIdent (family.accessor ++ `containsAt)
  elabCommand (← `(def $containsAtName:ident
      (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId))
      (globals : LeanerIR.GlobalMap) (key : LeanerIR.RuntimeValue) : Bool :=
    (globals.lookup ($keyAtName typeInstantiation key)).isSome))
  elabCommand (← `(def $containsName:ident (globals : LeanerIR.GlobalMap)
      (key : LeanerIR.RuntimeValue) : Bool :=
    $containsAtName #[] globals key))
  -- A clause's read and the program's read of one key meet in the closing
  -- normalization only if both unfold to the same keyed lookup.
  elabCommand (← `(attribute [lir_data_norm]
    $keyAtName:ident $keyName:ident $readAtName:ident $readName:ident
    $getAtName:ident $getName:ident $containsAtName:ident $containsName:ident))

/-- Ensure every twin and family accessor of a registered unit exists, and
return the family row a contract builds against. -/
def ensureSpecTypes (segments : Array String) (unit : ValidatedUnit) :
    CommandElabM (Array TwinInfo × Array FamilyInfo) := do
  let twins := twinInfos segments unit
  for info in twins do
    emitTwin info
  let families := familyInfos unit twins
  for family in families do
    emitFamily family
  -- A storable family without a representation cannot be reasoned about;
  -- say so once here rather than silently weakening every contract.
  for ns in unit.namespaces do
    for declaration in ns.structs do
      if declaration.abilities.contains .key then
        if let some qualified := nameOf? unit declaration.name then
          unless families.any (·.info.qualified == qualified) do
            logWarning m!"resource `{qualified.name}` has no typed \
              specification twin; contracts over its storage will not verify"
  return (twins, families)

end LeanerLang.SpecTypes
