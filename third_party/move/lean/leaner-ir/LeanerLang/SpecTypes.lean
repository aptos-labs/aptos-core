-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Registry
import LeanerIR.Proofs.Typed
import LeanerIR.Proofs.Denotation
import LeanerIR.Proofs.SimpAttrs
import LeanerIR.Proofs.Plain

/-!
# Generated typed twins of LIR struct declarations

Each supported struct declaration of a registered unit gets a Lean twin: a
structure whose fields carry the certified specification representation of
the declared field types (`SpecInt` for integers, plain Lean scalars for
the rest, nested twins for nominal fields), together with its
`erase`/`decode?` pair, their roundtrip, and — for a `key` struct whose
family appears in the type table — the keyed accessors a contract clause
reads storage through.

Generation is driven by the validated unit, not by surface syntax, so any
frontend that registers a unit gets the same twins.  A struct with content
the representation does not support yet (generics, variants, vectors,
closures, references) gets no twin; a clause or family that needs the
missing twin is a loud diagnostic downstream, never a silent degradation.
-/

namespace LeanerLang.SpecTypes

open Lean Elab Command
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
  /-- A nested twin, by its generated Lean name. -/
  | nominal (twin : Name)
  deriving Repr, BEq, Inhabited

/-- One generated twin: the struct it represents and its field row. -/
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
  fields : Array (String × FieldRep)
  deriving Repr, Inhabited

/-- One storable family: a `key` struct together with the type-table entry
that identifies it at global-operation sites. -/
structure FamilyInfo where
  info : TwinInfo
  /-- Index of the family's `.nominal` spelling in the shared type table. -/
  typeIndex : Nat
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
private def fieldRep? (unit : ValidatedUnit) (twinName : QualifiedName → Name)
    (supported : QualifiedName → Bool) (typeId : TypeId) : Option FieldRep := do
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
  | .nominal name arguments => do
      unless arguments.isEmpty do none
      let qualified ← nameOf? unit name
      unless supported qualified do none
      some (.nominal (twinName qualified))
  | _ => none

/-- Whether a struct declaration supports a twin, following nominal fields
through the acyclic declaration graph. -/
private partial def structSupported (unit : ValidatedUnit)
    (qualified : QualifiedName) : Bool :=
  match structOf? unit qualified with
  | none => false
  | some (_, _, declaration) =>
      declaration.generics.isEmpty && declaration.variants.isEmpty &&
        declaration.fields.all fun field =>
          (fieldRep? unit (fun _ => Name.anonymous) (structSupported unit)
            field.type.typeId).isSome

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
        let fields := declaration.fields.filterMap fun field => do
          let name ← nameOf? unit field.name
          let rep ← fieldRep? unit (twinName segments)
            (fun nested => emitted.contains nested ∨ nested == qualified)
            field.type.typeId
          some (name.name, rep)
        -- Nested twins must already be emitted; self-reference is impossible
        -- in an acyclic unit, so a missing dependency just waits a round.
        let ready := declaration.fields.all fun field =>
          match unit.tables.types[field.type.typeId.index]? with
          | some (.nominal nested _) =>
              (nameOf? unit nested).any emitted.contains
          | _ => true
        if ready && fields.size == declaration.fields.size then
          ordered := ordered.push {
            name := qualified.name, qualified
            twin := twinName segments qualified
            namespaceIndex, structIndex, fields }
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
    let spellings := unit.tables.types.zipIdx.filterMap fun (ty, index) =>
      match ty with
      | .nominal name arguments =>
          if arguments.isEmpty && nameOf? unit name == some info.qualified then
            some index
          else none
      | _ => none
    if let #[typeIndex] := spellings then
      families := families.push { info, typeIndex }
  return families

/-! ## Command generation -/

private def natLit (value : Nat) : Term :=
  Syntax.mkNatLit value

/-- Generated declarations are absolute: emission may run inside any user
namespace, and a relative name would be captured by it. -/
private def rootIdent (name : Name) : Ident :=
  mkIdent (rootNamespace ++ name)

/-- Term syntax of a field's representation type. -/
private def FieldRep.typeSyntax : FieldRep → CommandElabM Term
  | .int (.bits width) signed =>
      ``(LeanerIR.SpecInt (IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(LeanerIR.SpecInt IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width twin field"
  | .bool => ``(Bool)
  | .string | .address | .signer => ``(String)
  | .bytes => ``(Array UInt8)
  | .unit => ``(Unit)
  | .nominal twin => return rootIdent twin

/-- Term syntax of one field's erasure, from its projection. -/
def FieldRep.eraseSyntax (rep : FieldRep) (projected : Term) :
    CommandElabM Term :=
  match rep with
  | .int _ _ => ``(LeanerIR.RuntimeValue.integer (LeanerIR.SpecInt.val $projected))
  | .bool => ``(LeanerIR.RuntimeValue.bool $projected)
  | .string => ``(LeanerIR.RuntimeValue.string $projected)
  | .address => ``(LeanerIR.RuntimeValue.address $projected)
  | .signer => ``(LeanerIR.RuntimeValue.signer $projected)
  | .bytes => ``(LeanerIR.RuntimeValue.bytes $projected)
  | .unit => ``(LeanerIR.RuntimeValue.unit)
  | .nominal twin => ``($(rootIdent (twin ++ `erase)) $projected)

/-- Term syntax of one field's decoder. -/
private def FieldRep.decodeSyntax : FieldRep → CommandElabM Term
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
  | .nominal twin => return rootIdent (twin ++ `decode?)

/-- Term syntax of one field's default value. -/
private def FieldRep.defaultSyntax : FieldRep → CommandElabM Term
  | .int _ _ => ``(⟨0, by decide⟩)
  | .bool => ``(Bool.false)
  | .string | .address | .signer => ``("")
  | .bytes => ``(#[])
  | .unit => ``(())
  | .nominal _ => ``(default)

/-- Emit one twin's declarations: the tagged structure, `erase`, `decode?`,
their roundtrips, and `Inhabited`. -/
private def emitTwin (info : TwinInfo) : CommandElabM Unit := do
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
  -- Twin fields never contain references.  Pin that representation fact at
  -- the boundary so function finalization does not inspect an erased native
  -- argument as an arbitrary `RuntimeValue` looking for nested loans.
  let noBorrowsName := rootIdent (info.twin ++ `outermostBorrows_erase)
  elabCommand (← `(@[simp] theorem $noBorrowsName:ident
      ($value:ident : $twin) :
      LeanerIR.SemanticOperations.outermostBorrows ($eraseName $value) = #[] := by
    cases $value:ident
    simp [$eraseName:ident, LeanerIR.SemanticOperations.outermostBorrows,
      LeanerIR.SemanticOperations.borrowEntry?,
      LeanerIR.SemanticOperations.collectPruned,
      LeanerIR.SemanticOperations.collectPrunedList]
    all_goals (repeat' apply And.intro) <;>
      (intro entry membership
       exact
         (LeanerIR.SemanticOperations.collectPruned_borrows_all_none_iff
           _).mpr (by simp) entry membership)))
  -- A twin's erasure is loan-free: what lets a bracket walk a resource
  -- through its focus alone.
  let plainName := rootIdent (info.twin ++ `plain_erase)
  elabCommand (← `(@[leaner_plain] theorem $plainName:ident ($value:ident : $twin) :
      LeanerIR.SemanticOperations.Plain ($eraseName $value) := by
    simp only [$eraseName:ident]
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

/-- Emit one family's keyed accessors: the key, the typed read, the junk-
totalized read a clause uses, and the existence test. -/
private def emitFamily (family : FamilyInfo) : CommandElabM Unit := do
  if (← getEnv).contains (family.info.twin ++ `key) then return
  let keyName := rootIdent (family.info.twin ++ `key)
  let twin := rootIdent family.info.twin
  elabCommand (← `(def $keyName:ident (key : LeanerIR.RuntimeValue) :
      LeanerIR.GlobalKey :=
    ⟨⟨$(natLit family.info.namespaceIndex)⟩, ⟨$(natLit family.typeIndex)⟩,
      LeanerIR.RuntimeValue.storageKey key⟩))
  let readName := rootIdent (family.info.twin ++ `read)
  elabCommand (← `(def $readName:ident (globals : LeanerIR.GlobalMap)
      (key : LeanerIR.RuntimeValue) : Option $twin :=
    Option.bind (globals.lookup ($keyName key))
      $(rootIdent (family.info.twin ++ `decode?))))
  let getName := rootIdent (family.info.twin ++ `get)
  elabCommand (← `(def $getName:ident (globals : LeanerIR.GlobalMap)
      (key : LeanerIR.RuntimeValue) : $twin :=
    ($readName globals key).getD default))
  let containsName := rootIdent (family.info.twin ++ `contains)
  elabCommand (← `(def $containsName:ident (globals : LeanerIR.GlobalMap)
      (key : LeanerIR.RuntimeValue) : Bool :=
    (globals.lookup ($keyName key)).isSome))
  -- A clause's read and the program's read of one key meet in the closing
  -- normalization only if both unfold to the same keyed lookup.
  elabCommand (← `(attribute [lir_data_norm] $keyName:ident $readName:ident
    $getName:ident $containsName:ident))

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
