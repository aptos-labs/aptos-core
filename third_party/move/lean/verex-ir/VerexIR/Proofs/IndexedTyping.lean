-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.GlobalTyping

/-!
# The whole-unit typing checks, over indexed tables

`typesAgreeCheck` and `resourcesTypedCheck` read a unit's tables as arrays,
which the kernel evaluates as list walks (`doc/designs/performance.md`). The
checks here read one view of the tables instead: balanced trees built once per
unit, for a unit whose namespaces all hold its tables. Each reader below has a
theorem equating it to the array-reading original, and a passing indexed check
gives the original check (`typesAgreeCheck_of_indexed`,
`resourcesTypedCheck_of_indexed`), so the facts proved from them state what
they did.
-/

namespace VerexIR.Proofs

open VerexIR.Validation
open Denote
open SemanticOperations

/-- What the checks read of a declaration. -/
structure DeclarationView where
  declaration : StructDecl
  /-- The links of the declaring namespace. -/
  links : Links
  layout : Option OwnerLayout
  isTable : Bool

/-- The tables of a unit as the checks read them. -/
structure TypingView where
  namespaceCount : Nat
  tableNamespaces : Nat
  types : IndexedArena Ty
  names : IndexedArena QualifiedName
  canonical : IndexedArena NameId
  nominals : IndexedArena (Option TypeDeclId)
  declarations : IndexedArena (IndexedArena DeclarationView)
  /-- Whether a type parameter has a node, below the most binders a function has. -/
  parameterNodes : IndexedArena Bool
  parameterLimit : Nat
  pointerWidth : Option Nat
  typeCount : Nat
  fuel : Nat

/-- A unit's namespaces all hold its tables. -/
def TablesShared (unit : ValidatedUnit) : Prop :=
  unit.namespaces.toList.map (·.tables) =
    List.replicate unit.namespaces.toList.length unit.tables

/-- The first node of a type parameter in a table. -/
def parameterNodeAt (types : Array Ty) (index : Nat) : Bool :=
  (types.findIdx? fun type => match type with
    | .typeParameter other => other == index
    | _ => false).isSome

/-- The most generic binders a function of a unit declares. -/
def parameterLimitOf (unit : ValidatedUnit) : Nat :=
  unit.namespaces.toList.foldl (init := 0) fun limit ns =>
    ns.functions.toList.foldl (init := limit) fun limit declaration =>
      max limit declaration.signature.generics.size

def declarationViews (unit : ValidatedUnit) (index : Nat) (ns : ValidatedNamespace) :
    IndexedArena DeclarationView :=
  .ofArray (ns.structs.toList.zipIdx.map fun (declaration, structId) =>
    { declaration
      links := ns.links
      layout := abstractLayout? unit ⟨index⟩ structId
      isTable := (tableLayout? unit ⟨index⟩ structId).isSome }).toArray

/-- The view of a unit's tables, evaluated once. -/
def TypingView.of (unit : ValidatedUnit) : TypingView :=
  { namespaceCount := unit.namespaces.toList.length
    tableNamespaces := unit.tables.namespaces.size
    types := .ofArray unit.tables.types
    names := .ofArray unit.tables.names
    canonical := .ofArray unit.resolution.canonicalNames
    nominals := .ofArray unit.resolution.nominals
    declarations := .ofArray (unit.namespaces.toList.zipIdx.map fun (ns, index) =>
      declarationViews unit index ns).toArray
    parameterNodes := .ofArray
      ((List.range (parameterLimitOf unit)).map (parameterNodeAt unit.tables.types)).toArray
    parameterLimit := parameterLimitOf unit
    pointerWidth := targetPointerWidth? unit
    typeCount := unit.tables.types.size
    fuel := typeFuel unit }

namespace TypingView

variable (view : TypingView)

def declarationAt? (handle : StructHandle) : Option DeclarationView :=
  (view.declarations.get? handle.namespaceId.index).bind (·.get? handle.structId)

def lookup : TypeLookup := fun namespaceId typeId =>
  if namespaceId.index < view.namespaceCount then view.types.get? typeId.index else none

def nominal? (name : NameId) : Option TypeDeclId :=
  match view.nominals.get? ((view.canonical.get? name.index).getD name).index with
  | some (some value) => some value
  | _ => none

def resolveNominal? (name : NameId) : Option StructHandle := do
  let qualified ← view.names.get? name.index
  let typeId ← view.nominal? name
  let handle : StructHandle := { namespaceId := qualified.namespaceId, structId := typeId.index }
  let _ ← view.declarationAt? handle
  return handle

mutual
/-- `ntyOfFuel`, read through the view. -/
def ntyFuel : NRow → Nat → NamespaceId → TypeId → Option NTy
  | _, 0, _, _ => none
  | env, fuel + 1, namespaceId, typeId => do
      let ty ← view.lookup namespaceId typeId
      match ty with
      | .unit => some .unit
      | .bool => some .bool
      | .integer (.bits width) signed => if width == 0 then none else some (.int width signed)
      | .integer .pointer signed => (.int · signed) <$> view.pointerWidth
      | .integer .unbounded true => some .num
      | .never => some .unit
      | .address => some .address
      | .signer => some .signer
      | .string => some .string
      | .bytes => some .bytes
      | .character => some .char
      | .reference reference =>
          match reference.kind with
          | .shared => NTy.erase <$> ntyFuel env fuel namespaceId reference.referent
          | .mutable =>
              match view.lookup namespaceId reference.referent with
              | some (.slice element) => .sliceRef <$> ntyFuel env fuel namespaceId element
              | _ => .ref <$> ntyFuel env fuel namespaceId reference.referent
      | .tuple elements =>
          (.tuple ∘ NRow.ofList) <$> elements.toList.mapM (ntyFuel env fuel namespaceId)
      | .slice element => .slice <$> ntyFuel env fuel namespaceId element
      | .vector element none => .vector <$> ntyFuel env fuel namespaceId element
      | .vector element (some (.integer length)) =>
          if length < 0 ∨ 2 ^ 64 ≤ length then none
          else (.array · length.toNat) <$> ntyFuel env fuel namespaceId element
      | .nominal name arguments => do
          let handle ← view.resolveNominal? name
          if arguments.isEmpty then structFuel .nil fuel handle else
            let θ ← arguments.toList.mapM fun argument => match argument with
              | .typeArg value => ntyFuel env fuel namespaceId value.typeId
              | .lifetime _ | .const _ => some .unit
              | .evidence _ => none
            structFuel (NRow.ofList θ) fuel handle
      | .typeParameter index => some (env.argument index).unmark
      | .function arguments result _ => do
          let parameters ← arguments.toList.mapM (ntyFuel env fuel namespaceId)
          let isUnit : Bool := match ← view.lookup namespaceId result with
            | .unit => true
            | .typeParameter index => (env.argument index).unmark == .unit
            | _ => false
          let results ← if isUnit then some [] else (fun τ => [τ]) <$> ntyFuel env fuel namespaceId result
          let resultIds := if isUnit then [] else [result]
          some (.function (NRow.ofList parameters)
            { parameters := arguments.toList.map (sharedReference view.lookup namespaceId)
              results := resultIds.map (sharedReference view.lookup namespaceId) }
            (NRow.ofList results))
      | _ => none

/-- `structNTyFuel`, read through the view. -/
def structFuel : NRow → Nat → StructHandle → Option NTy
  | _, 0, _ => none
  | env, fuel + 1, handle => do
      let info ← view.declarationAt? handle
      let declaration := info.declaration
      let row := fun (fields : Array FieldDecl) =>
        NRow.ofList <$> fields.toList.mapM fun field =>
          ntyFuel env fuel handle.namespaceId (info.links.typeId field.type.typeId)
      let arguments := NRow.ofList
        ((List.range declaration.generics.size).map fun index => env.getD index (.param index))
      let ghosts ← NRow.ofList <$> declaration.ghosts.toList.mapM fun ghost =>
        NTy.ghost <$> ntyFuel env fuel handle.namespaceId (info.links.typeId ghost.type.typeId)
      let withGhosts := fun (fields : NRow) =>
        if declaration.ghosts.isEmpty then fields else fields ++ ghosts
      match info.layout with
      | some layout =>
          some (.struct handle arguments
            (withGhosts layout.leadingRow ++
              .cons (.table (env.getD 0 (.param 0)) (env.getD 1 (.param 1))) .nil))
      | none =>
      if declaration.variants.isEmpty then
        some (.struct handle arguments (withGhosts (← row declaration.fields)))
      else
        let names ← declaration.variants.toList.mapM fun variant =>
          (view.names.get? (info.links.nameId variant.name).index).map (·.name)
        let rows ← NRows.ofList <$> declaration.variants.toList.mapM fun variant =>
          withGhosts <$> row variant.fields
        if distinct : names.Nodup then some (.enum handle arguments names rows distinct)
        else none
end

def nty (namespaceId : NamespaceId) (typeId : TypeId) : Option NTy :=
  view.ntyFuel .nil view.fuel namespaceId typeId

/-- `SemTy.resolveFuel`, read through the view. -/
def resolveFuel (env : Array SemArg) : Nat → TypeId → Option SemTy
  | 0, _ => none
  | fuel + 1, typeId => do
      let type : Ty ← view.types.get? typeId.index
      match type with
      | .unit => some .unit
      | .never => some .never
      | .bool => some .bool
      | .character => some .character
      | .string => some .string
      | .bytes => some .bytes
      | .address => some .address
      | .signer => some .signer
      | .integer width signed => some (.integer width signed)
      | .tuple elements => .tuple <$> elements.toList.mapM (resolveFuel env fuel)
      | .slice element => .slice <$> resolveFuel env fuel element
      | .vector element bound => (.vector · bound) <$> resolveFuel env fuel element
      | .nominal name arguments => do
          let name ← view.names.get? name.index
          let arguments ← arguments.toList.mapM fun
            | .typeArg value => SemArg.type <$> resolveFuel env fuel value.typeId
            | .const value => some (.const value)
            | .lifetime _ | .evidence _ => some .erased
          some (.nominal name arguments)
      | .function arguments result _ => do
          let arguments ← arguments.toList.mapM (resolveFuel env fuel)
          let result ← resolveFuel env fuel result
          some (.function arguments result)
      | .reference borrowed =>
          (.reference borrowed.kind ·) <$> resolveFuel env fuel borrowed.referent
      | .profile tag => some (.profile tag)
      | .typeParameter index => match env[index]? with
          | some (.type type) => some type
          | _ => none
      | .range | .eventStore | .typeDomain _ | .resourceDomain _ _ | .stateDomain => none

def resolve (env : Array SemArg) (typeId : TypeId) : Option SemTy :=
  view.resolveFuel env (view.typeCount + 1) typeId

end TypingView

namespace TypingView

variable (view : TypingView)

/-- `SemanticOperations.structName?`, of a declaration read through the view. -/
def structName? (info : DeclarationView) : Option QualifiedName := do
  let name ← view.names.get? (info.links.nameId info.declaration.name).index
  if name.namespaceId.index < view.tableNamespaces then some name else none

def declaredTypes? (arguments : List SemArg) (typeIds : List TypeId) : Option (List SemTy) :=
  typeIds.mapM (view.resolve arguments.toArray)

def _root_.VerexIR.Proofs.DeclarationView.ghostIds (info : DeclarationView) : List TypeId :=
  info.declaration.ghosts.toList.map (info.links.typeId ·.type.typeId)

def _root_.VerexIR.Proofs.DeclarationView.contents (info : DeclarationView)
    (arguments : List SemArg) : List SemTy :=
  match info.isTable, arguments with
  | true, [.type key, .type value] => [.vector (.tuple [key, value]) none]
  | _, _ => []

def variantFields? (info : DeclarationView) (name : String) : Option (Array FieldDecl) :=
  (info.declaration.variants.toList.find? fun candidate =>
    (view.names.get? (info.links.nameId candidate.name).index).any (·.name == name)).map (·.fields)

def variantNames? (info : DeclarationView) : Option (List String) :=
  info.declaration.variants.toList.mapM fun variant =>
    (view.names.get? (info.links.nameId variant.name).index).map (·.name)

mutual
/-- `NTy.typedAsCore`, read through the view. -/
def typedAs : NTy → SemTy → Bool
  | .unit, type => type == .unit
  | .bool, type => type == .bool
  | .int width signed, type => type == .integer (.bits width) signed
  | .num, type => type == .integer .unbounded true
  | .address, type => type == .address
  | .signer, type => type == .signer
  | .string, type => type == .string
  | .bytes, type => type == .bytes
  | .char, type => type == .character
  | .tuple elements, .tuple types => rowTypedAs elements types
  | .slice element, .slice type => typedAs element type
  | .sliceRef element, .reference .mutable (.slice type) => typedAs element type
  | .vector element, .vector type none => typedAs element type
  | .array element length, .vector type (some (.integer actual)) =>
      actual == (length : Int) && typedAs element type
  | .table key value, .vector (.tuple [keyType, valueType]) none =>
      typedAs key keyType && typedAs value valueType
  | .struct source nativeArguments fields, .nominal name arguments =>
      match view.declarationAt? source with
      | none => false
      | some info =>
        view.structName? info == some name &&
          info.declaration.generics.size == arguments.length &&
          argumentsTypedAs nativeArguments arguments &&
          match info.layout with
          | some layout =>
            match arguments with
            | [.type key, .type value] =>
              match view.declaredTypes? arguments (info.ghostIds) with
              | some ghostTypes => rowTypedAs fields (layout.canonicalTypes ghostTypes key value)
              | none => false
            | _ => false
          | none =>
            info.declaration.variants.isEmpty &&
            match view.declaredTypes? arguments
                  (info.declaration.fields.toList.map (info.links.typeId ·.type.typeId)),
                view.declaredTypes? arguments (info.ghostIds) with
            | some fieldTypes, some ghostTypes =>
                rowTypedAs fields
                  (fieldTypes ++ (ghostTypes.map .ghost ++ info.contents arguments))
            | _, _ => false
  | .enum source nativeArguments names rows _, .nominal name arguments =>
      match view.declarationAt? source with
      | none => false
      | some info =>
        view.structName? info == some name &&
          info.declaration.generics.size == arguments.length &&
          argumentsTypedAs nativeArguments arguments && !names.isEmpty &&
          view.variantNames? info == some names &&
          variantsTypedAs info arguments names rows
  | .ref referent, .reference .mutable type => typedAs referent type
  | .ghost slot, .ghost type => typedAs slot type
  | .param index, .param other => index == other
  | .function parameters shared returned, .function parameterTypes result =>
      parametersTypedAs parameters shared.parameters parameterTypes &&
        SemTy.pack result.resultRow == result &&
        parametersTypedAs returned shared.results result.resultRow
  | _, _ => false

def rowTypedAs : NRow → List SemTy → Bool
  | .nil, [] => true
  | .cons τ rest, type :: types => typedAs τ type && rowTypedAs rest types
  | _, _ => false

def argumentsTypedAs : NRow → List SemArg → Bool
  | .nil, [] => true
  | .cons τ rest, .type type :: arguments => typedAs τ type && argumentsTypedAs rest arguments
  | _, _ => false

def parametersTypedAs : NRow → List Bool → List SemTy → Bool
  | .nil, [], [] => true
  | .cons τ rest, false :: shared, type :: types =>
      type.unshared == type && typedAs τ type && parametersTypedAs rest shared types
  | .cons τ rest, true :: shared, .reference .shared type :: types =>
      type.unshared == type && typedAs τ type && parametersTypedAs rest shared types
  | _, _, _ => false

def variantsTypedAs (info : DeclarationView) (arguments : List SemArg) :
    List String → NRows → Bool
  | [], .nil => true
  | name :: names, .cons fields rest =>
      (match view.variantFields? info name with
        | some declared =>
          match view.declaredTypes? arguments (declared.toList.map (info.links.typeId ·.type.typeId)),
              view.declaredTypes? arguments (info.ghostIds) with
          | some fieldTypes, some ghostTypes =>
              rowTypedAs fields (fieldTypes ++ ghostTypes.map .ghost)
          | _, _ => false
        | none => false) &&
        variantsTypedAs info arguments names rest
  | _, _ => false
end

end TypingView

theorem TablesShared.tables {unit : ValidatedUnit} (shared : TablesShared unit) {index : Nat}
    {ns : ValidatedNamespace} (found : unit.namespaces[index]? = some ns) :
    ns.tables = unit.tables := by
  have atIndex := congrArg (·[index]?) shared
  simp only [List.getElem?_map, Array.getElem?_toList, found, Option.map_some,
    List.getElem?_replicate] at atIndex
  split at atIndex
  · exact Option.some.inj atIndex
  · cases atIndex

section Reads

variable {unit : ValidatedUnit}

theorem TypingView.types_of (index : Nat) :
    (TypingView.of unit).types.get? index = unit.tables.types[index]? :=
  IndexedArena.get?_ofArray _ _

theorem TypingView.names_of (index : Nat) :
    (TypingView.of unit).names.get? index = unit.tables.names[index]? :=
  IndexedArena.get?_ofArray _ _

theorem TypingView.lookup_of (shared : TablesShared unit) :
    (TypingView.of unit).lookup = unitTypes unit := by
  funext namespaceId typeId
  simp only [TypingView.lookup, unitTypes, TypingView.types_of]
  cases found : unit.namespaces[namespaceId.index]? with
  | none =>
      have beyond : ¬ namespaceId.index < (TypingView.of unit).namespaceCount := by
        intro inside
        simp only [TypingView.of, Array.length_toList] at inside
        simp [Array.getElem?_eq_getElem inside] at found
      simp [beyond]
  | some ns =>
      have inside : namespaceId.index < (TypingView.of unit).namespaceCount := by
        simp only [TypingView.of, Array.length_toList]
        rcases Nat.lt_or_ge namespaceId.index unit.namespaces.size with inside | beyond
        · exact inside
        · simp [Array.getElem?_eq_none beyond] at found
      simp [inside, shared.tables found]

theorem TypingView.declarationAt?_of (handle : StructHandle) :
    (TypingView.of unit).declarationAt? handle =
      (unit.namespaces[handle.namespaceId.index]?).bind fun ns =>
        (ns.structs[handle.structId]?).map fun declaration =>
          { declaration
            links := ns.links
            layout := abstractLayout? unit handle.namespaceId handle.structId
            isTable := (tableLayout? unit handle.namespaceId handle.structId).isSome } := by
  simp only [TypingView.declarationAt?, TypingView.of, IndexedArena.get?_ofArray,
    List.getElem?_toArray, List.getElem?_map, List.getElem?_zipIdx, Array.getElem?_toList]
  cases unit.namespaces[handle.namespaceId.index]? with
  | none => rfl
  | some ns =>
      simp only [Option.map_map, Option.map_some, Option.bind_some, declarationViews,
        IndexedArena.get?_ofArray, List.getElem?_toArray, List.getElem?_map,
        List.getElem?_zipIdx, Array.getElem?_toList]
      cases ns.structs[handle.structId]? <;> simp

theorem TypingView.nominal?_of (name : NameId) :
    (TypingView.of unit).nominal? name = unit.resolution.nominal? name := by
  rw [ResolutionIndex.nominal?_eq]
  simp only [TypingView.nominal?, TypingView.of, IndexedArena.get?_ofArray,
    ResolutionIndex.canonical]
  rcases unit.resolution.nominals[(unit.resolution.canonicalNames[name.index]?.getD name).index]?
    with _ | _ | _ <;> rfl

theorem TypingView.resolveNominal?_of (shared : TablesShared unit) {index : Nat}
    {ns : ValidatedNamespace} (found : unit.namespaces[index]? = some ns) (name : NameId) :
    (TypingView.of unit).resolveNominal? name = SemanticOperations.resolveNominal? unit ns name := by
  simp only [TypingView.resolveNominal?, SemanticOperations.resolveNominal?, shared.tables found,
    TypingView.names_of, TypingView.nominal?_of, TypingView.declarationAt?_of]
  cases unit.tables.names[name.index]? with
  | none => rfl
  | some qualified =>
      simp only [Option.bind_eq_bind, Option.bind_some]
      cases unit.resolution.nominal? name with
      | none => rfl
      | some typeId =>
          simp only [Option.bind_some]
          cases unit.namespaces[qualified.namespaceId.index]? with
          | none => rfl
          | some target =>
              simp only [Option.bind_some]
              cases target.structs[typeId.index]? <;> rfl

theorem TypingView.resolveFuel_of (env : Array SemArg) (fuel : Nat) :
    (TypingView.of unit).resolveFuel env fuel = SemTy.resolveFuel unit.tables env fuel := by
  induction fuel with
  | zero => funext t; simp only [TypingView.resolveFuel, SemTy.resolveFuel]
  | succ fuel ih =>
      funext t
      simp only [TypingView.resolveFuel, SemTy.resolveFuel, TypingView.types_of,
        TypingView.names_of, ih]
      refine congrArg (Option.bind _) (funext fun type => ?_)
      cases type <;> rfl

theorem TypingView.resolve_of {ns : ValidatedNamespace} (tables : ns.tables = unit.tables)
    (env : Array SemArg) :
    (TypingView.of unit).resolve env = StaticTyping.resolveAt ns env := by
  funext t
  simp only [TypingView.resolve, StaticTyping.resolveAt, tables, TypingView.resolveFuel_of]
  rfl

theorem TypingView.ntyFuel_of (shared : TablesShared unit) (fuel : Nat) :
    (∀ env namespaceId typeId, (TypingView.of unit).ntyFuel env fuel namespaceId typeId =
      ntyOfFuel (unitTypes unit) unit env fuel namespaceId typeId) ∧
    (∀ env handle, (TypingView.of unit).structFuel env fuel handle =
      structNTyFuel (unitTypes unit) unit env fuel handle) := by
  induction fuel with
  | zero =>
      exact ⟨fun _ _ _ => by simp only [TypingView.ntyFuel, ntyOfFuel],
        fun _ _ => by simp only [TypingView.structFuel, structNTyFuel]⟩
  | succ fuel ih =>
      obtain ⟨nty, struct⟩ := ih
      have ntyAt : ∀ env namespaceId, (TypingView.of unit).ntyFuel env fuel namespaceId =
          ntyOfFuel (unitTypes unit) unit env fuel namespaceId := fun env namespaceId =>
        funext (nty env namespaceId)
      have width : (TypingView.of unit).pointerWidth = targetPointerWidth? unit := rfl
      refine ⟨fun env namespaceId typeId => ?_, fun env handle => ?_⟩
      · cases found : unit.namespaces[namespaceId.index]? with
        | none =>
            simp [TypingView.ntyFuel, ntyOfFuel, TypingView.lookup_of shared, unitTypes, found]
        | some ns =>
            simp only [TypingView.ntyFuel, ntyOfFuel, TypingView.lookup_of shared, found,
              TypingView.resolveNominal?_of shared found, Option.bind_eq_bind, Option.bind_some,
              nty, ntyAt, struct, width]
            refine congrArg (Option.bind _) (funext fun ty => ?_)
            cases ty <;> rfl
      · simp only [TypingView.structFuel, structNTyFuel, TypingView.declarationAt?_of]
        cases found : unit.namespaces[handle.namespaceId.index]? with
        | none => rfl
        | some ns =>
            simp only [Option.bind_eq_bind, Option.bind_some]
            cases ns.structs[handle.structId]? with
            | none => rfl
            | some declaration =>
                simp only [Option.map_some, Option.bind_some, nty, ValidatedNamespace.nameOf,
                  ValidatedNamespace.nameId, ValidatedNamespace.typeId, shared.tables found,
                  TypingView.names_of]
                refine congrArg (Option.bind _) (funext fun ghosts => ?_)
                cases abstractLayout? unit handle.namespaceId handle.structId <;> rfl

theorem TypingView.nty_of (shared : TablesShared unit) (namespaceId : NamespaceId)
    (typeId : TypeId) : (TypingView.of unit).nty namespaceId typeId = ntyOf unit namespaceId typeId :=
  (TypingView.ntyFuel_of shared _).1 .nil namespaceId typeId

theorem TypingView.resolveAt_of {unit : ValidatedUnit} {ns : ValidatedNamespace}
    (tables : ns.tables = unit.tables) (arguments : List SemArg) (typeIds : List TypeId) :
    (TypingView.of unit).declaredTypes? arguments typeIds =
      typeIds.mapM (StaticTyping.resolveAt ns arguments.toArray) := by
  simp only [TypingView.declaredTypes?, TypingView.resolve_of tables]

end Reads

section Declarations

variable {unit : ValidatedUnit}

theorem TypingView.declarationAt?_none {handle : StructHandle}
    (missing : (TypingView.of unit).declarationAt? handle = none) :
    SemanticOperations.structName? unit handle = none := by
  rw [TypingView.declarationAt?_of] at missing
  simp only [SemanticOperations.structName?, Option.bind_eq_bind]
  cases found : unit.namespaces[handle.namespaceId.index]? with
  | none => rfl
  | some ns =>
      simp only [found, Option.bind_some, Option.map_eq_none_iff] at missing
      simp only [Option.bind_some, missing, Option.bind_none]

theorem TypingView.declarationAt?_some {handle : StructHandle} {info : DeclarationView}
    (found : (TypingView.of unit).declarationAt? handle = some info) :
    ∃ ns, unit.namespaces[handle.namespaceId.index]? = some ns ∧
      ns.structs[handle.structId]? = some info.declaration ∧
      info.layout = abstractLayout? unit handle.namespaceId handle.structId ∧
      info.isTable = (tableLayout? unit handle.namespaceId handle.structId).isSome ∧
      info.links = ns.links := by
  rw [TypingView.declarationAt?_of] at found
  cases namespace_eq : unit.namespaces[handle.namespaceId.index]? with
  | none => simp [namespace_eq] at found
  | some ns =>
      simp only [namespace_eq, Option.bind_some, Option.map_eq_some_iff] at found
      obtain ⟨declaration, declaration_eq, rfl⟩ := found
      exact ⟨ns, rfl, declaration_eq, rfl, rfl, rfl⟩

theorem TypingView.structName?_of (shared : TablesShared unit) {handle : StructHandle}
    {ns : ValidatedNamespace} {info : DeclarationView}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some ns)
    (declaration_eq : ns.structs[handle.structId]? = some info.declaration)
    (links_eq : info.links = ns.links) :
    SemanticOperations.structName? unit handle = (TypingView.of unit).structName? info := by
  simp only [SemanticOperations.structName?, TypingView.structName?, namespace_eq, declaration_eq,
    Option.bind_eq_bind, Option.bind_some, declaredName?, shared.tables namespace_eq,
    TypingView.names_of, links_eq, ValidatedNamespace.nameOf, ValidatedNamespace.nameId]
  cases unit.tables.names[(ns.links.nameId info.declaration.name).index]? with
  | none => rfl
  | some qualified =>
      simp only [Option.bind_some]
      by_cases inside : qualified.namespaceId.index < unit.tables.namespaces.size
      · simp [Array.getElem?_eq_getElem inside, TypingView.of, inside]
      · have beyond := Nat.le_of_not_lt inside
        simp [Array.getElem?_eq_none beyond, TypingView.of, inside]

end Declarations

section Typed

variable {unit : ValidatedUnit}

/-- The indexed reading of a type is the array-reading one. -/
theorem TypingView.typedAs_of (shared : TablesShared unit) :
    (∀ τ type, (TypingView.of unit).typedAs τ type = NTy.typedAsCore unit τ type) ∧
    (∀ row flags types, (TypingView.of unit).parametersTypedAs row flags types =
      NRow.parametersCheck unit row flags types) ∧
    (∀ source arguments names rows, ∀ info, (TypingView.of unit).declarationAt? source = some info →
      (TypingView.of unit).variantsTypedAs info arguments names rows =
        NRows.variantsCheck unit source arguments names rows) ∧
    (∀ row types, (TypingView.of unit).rowTypedAs row types = NRow.typedAsCheck unit row types) ∧
    (∀ row arguments, (TypingView.of unit).argumentsTypedAs row arguments =
      NRow.argumentsCheck unit row arguments) := by
  apply NTy.typedAsCore.mutual_induct unit
    (fun τ type => (TypingView.of unit).typedAs τ type = NTy.typedAsCore unit τ type)
    (fun row flags types => (TypingView.of unit).parametersTypedAs row flags types =
      NRow.parametersCheck unit row flags types)
    (fun source arguments names rows => ∀ info, (TypingView.of unit).declarationAt? source = some info →
      (TypingView.of unit).variantsTypedAs info arguments names rows =
        NRows.variantsCheck unit source arguments names rows)
    (fun row types => (TypingView.of unit).rowTypedAs row types = NRow.typedAsCheck unit row types)
    (fun row arguments => (TypingView.of unit).argumentsTypedAs row arguments =
      NRow.argumentsCheck unit row arguments)
  case case16 =>
    intro source nativeArguments fields name arguments argumentsEq layoutRows fieldRows
    simp only [TypingView.typedAs, NTy.typedAsCore, argumentsEq]
    cases found : (TypingView.of unit).declarationAt? source with
    | none =>
        simp [TypingView.declarationAt?_none found]
    | some info =>
        obtain ⟨ns, namespace_eq, declaration_eq, layout_eq, table_eq, links_eq⟩ :=
          TypingView.declarationAt?_some found
        have tables := shared.tables namespace_eq
        have ghostIds : ghostTypeIds unit source = info.ghostIds := by
          simp [ghostTypeIds, nominalGhosts, namespace_eq, declaration_eq, DeclarationView.ghostIds,
            links_eq, ValidatedNamespace.typeId]
        have contents : contentsTypes unit source none arguments = info.contents arguments := by
          simp only [contentsTypes, DeclarationView.contents, table_eq]
          cases tableLayout? unit source.namespaceId source.structId with
          | none => simp only [Option.isSome_none]
          | some _ =>
              simp only [Option.isSome_some]
              rcases arguments with _ | ⟨first, _ | ⟨second, rest⟩⟩
              · rfl
              · cases first <;> rfl
              · cases first <;> cases second <;> cases rest <;> rfl
        simp only [TypingView.structName?_of shared namespace_eq declaration_eq links_eq,
          declarationArity?, namespace_eq, declaration_eq, Option.bind_eq_bind, Option.bind_some,
          ← layout_eq, ghostIds, SemanticOperations.handleFields?, Denote.declaredTypes?,
          Denote.ghostDeclaredTypes?, contents, TypingView.declaredTypes?,
          Option.some_beq_some, links_eq, ValidatedNamespace.typeId] at fieldRows ⊢
        simp only [TypingView.resolve_of tables]
        cases layoutEq : info.layout with
        | some layout =>
            rcases arguments with _ | ⟨first, _ | ⟨second, rest⟩⟩
            · rfl
            · cases first <;> rfl
            · cases first <;> cases second <;> cases rest <;> try rfl
              rename_i key value
              have rows : ∀ read : Option (List SemTy),
                  (match read with
                    | some ghostTypes =>
                        (TypingView.of unit).rowTypedAs fields
                          (layout.canonicalTypes ghostTypes key value)
                    | none => false) =
                  (match read with
                    | some ghostTypes =>
                        NRow.typedAsCheck unit fields (layout.canonicalTypes ghostTypes key value)
                    | none => false) := by
                intro read; cases read <;> simp only [layoutRows]
              exact congrArg _ (rows _)
        | none =>
            by_cases empty : info.declaration.variants.isEmpty = true
            · simp only [empty, if_true, Option.bind_some, Bool.true_and]
              generalize List.mapM (m := Option) _ info.ghostIds = ghostsRead
              generalize List.mapM (m := Option) _ (List.map _ info.declaration.fields.toList) = fieldsRead
              cases ghostsRead <;> cases fieldsRead <;> simp only [fieldRows]
            · have nonempty : info.declaration.variants.isEmpty = false := by simpa using empty
              simp [nonempty]
  case case17 =>
    intro source nativeArguments names rows distinct name arguments argumentsEq variants
    simp only [TypingView.typedAs, NTy.typedAsCore, argumentsEq]
    cases found : (TypingView.of unit).declarationAt? source with
    | none => simp [TypingView.declarationAt?_none found]
    | some info =>
        obtain ⟨ns, namespace_eq, declaration_eq, _, _, links_eq⟩ := TypingView.declarationAt?_some found
        have tables := shared.tables namespace_eq
        simp only [TypingView.structName?_of shared namespace_eq declaration_eq links_eq,
          declarationArity?, declarationVariants?, namespace_eq, declaration_eq,
          Option.bind_eq_bind, Option.bind_some, tables, TypingView.variantNames?,
          TypingView.names_of, variants info found, Option.some_beq_some, links_eq,
          ValidatedNamespace.nameOf, ValidatedNamespace.nameId]
  case case34 =>
    intro source arguments name names fields rest fieldRows later info found
    obtain ⟨ns, namespace_eq, declaration_eq, _, _, links_eq⟩ := TypingView.declarationAt?_some found
    have tables := shared.tables namespace_eq
    have ghostIds : ghostTypeIds unit source = info.ghostIds := by
      simp [ghostTypeIds, nominalGhosts, namespace_eq, declaration_eq, DeclarationView.ghostIds,
        links_eq, ValidatedNamespace.typeId]
    simp only [TypingView.variantsTypedAs, NRows.variantsCheck, later info found,
      SemanticOperations.handleFields?, namespace_eq, declaration_eq, Option.bind_eq_bind,
      Option.bind_some, TypingView.variantFields?, tables, TypingView.names_of,
      Denote.declaredTypes?, Denote.ghostDeclaredTypes?, ghostIds, TypingView.declaredTypes?,
      TypingView.resolve_of tables, links_eq, ValidatedNamespace.typeId, ValidatedNamespace.nameOf,
      ValidatedNamespace.nameId]
    cases info.declaration.variants.toList.find? _ with
    | none => rfl
    | some declared =>
        simp only [Option.map_some, Option.bind_some]
        generalize List.mapM (m := Option) _ info.ghostIds = ghostsRead
        generalize List.mapM (m := Option) _ (List.map _ declared.fields.toList) = fieldsRead
        cases ghostsRead <;> cases fieldsRead <;> simp only [fieldRows]
  all_goals first
    | (intros; simp_all [TypingView.typedAs, NTy.typedAsCore, TypingView.parametersTypedAs,
        NRow.parametersCheck, TypingView.variantsTypedAs, NRows.variantsCheck,
        TypingView.rowTypedAs, NRow.typedAsCheck, TypingView.argumentsTypedAs, NRow.argumentsCheck];
        done)
    | skip

end Typed

namespace TypingView

variable (view : TypingView)

/-- Whether a function's type parameter has a node in the tables: read from
the view, below the most binders a function declares. -/
def parameterNode (unit : ValidatedUnit) (namespaceId : NamespaceId) (index : Nat) : Bool :=
  if index < view.parameterLimit ∧ namespaceId.index < view.namespaceCount then
    (view.parameterNodes.get? index).getD false
  else (paramNodeIn? unit namespaceId index).isSome

def signatureTypes? (links : Links) (declaration : FunctionDecl FunctionBody)
    (env : Array SemArg) : Option (List SemTy × List SemTy) := do
  let parameters ← declaration.signature.parameters.toList.mapM fun parameter =>
    view.resolve env (links.typeId parameter.typeUse.typeId)
  let results ← declaration.signature.results.toList.mapM fun result =>
    view.resolve env (links.typeId result.typeId)
  some (parameters, results)

def signatureSharingAtTop (links : Links) (declaration : FunctionDecl FunctionBody) : Bool :=
  match view.signatureTypes? links declaration
      (StaticTyping.staticEnv declaration.signature.generics) with
  | some (parameterTypes, resultTypes) => (parameterTypes ++ resultTypes).all SemTy.sharingAtTop
  | none => true

/-- `closureSignature?` of a function whose declaration the caller holds. -/
def closureSignature? (unit : ValidatedUnit) (namespaceId : NamespaceId) (links : Links)
    (declaration : FunctionDecl FunctionBody) : Option (List NTy × List NTy) := do
  let parameters ← declaration.signature.parameters.toList.mapM fun parameter =>
    view.nty namespaceId (links.typeId parameter.typeUse.typeId)
  let results ← declaration.signature.results.toList.mapM fun result =>
    view.nty namespaceId (links.typeId result.typeId)
  guard ((parameters ++ results).all fun τ => τ.paramsAll fun index =>
    (declaration.signature.generics[index]?.any fun binder => binder.kind matches .typeArg) &&
      view.parameterNode unit namespaceId index)
  guard (view.signatureSharingAtTop links declaration)
  some (parameters, results)

/-- `signatureAgrees` of a function whose declaration the caller holds. -/
def signatureAgrees (unit : ValidatedUnit) (namespaceId : NamespaceId) (links : Links)
    (declaration : FunctionDecl FunctionBody) : Bool :=
  match view.closureSignature? unit namespaceId links declaration with
  | none => true
  | some (parameters, results) =>
      match view.signatureTypes? links declaration
          (StaticTyping.staticEnv declaration.signature.generics) with
      | some (parameterTypes, resultTypes) =>
          view.parametersTypedAs (NRow.ofList parameters)
              (declaration.signature.parameters.toList.map fun parameter =>
                sharedReference view.lookup namespaceId (links.typeId parameter.typeUse.typeId))
                parameterTypes &&
            view.parametersTypedAs (NRow.ofList results)
              (declaration.signature.results.toList.map fun result =>
                sharedReference view.lookup namespaceId (links.typeId result.typeId)) resultTypes
      | none => false

def typeAgrees (type : Nat) : Bool :=
  match view.nty ⟨0⟩ ⟨type⟩, view.resolve #[] ⟨type⟩ with
  | some τ, some type => !type.referenceFree || view.typedAs τ type
  | _, _ => true

/-- `typesAgreeCheck` of a unit whose namespaces hold its tables: the first
namespace's types once, and each function where the walk holds it. -/
def typesAgree (unit : ValidatedUnit) : Bool :=
  (List.range view.typeCount).all view.typeAgrees &&
    unit.namespaces.toList.zipIdx.all fun (ns, index) =>
      ns.functions.toList.all (view.signatureAgrees unit ⟨index⟩ ns.links)

def resourceOf? (typeId : TypeId) : Option ResourceType := do
  let ty ← view.lookup ⟨0⟩ typeId
  match ty with
  | .nominal name arguments =>
      let handle ← view.resolveNominal? name
      let info ← view.declarationAt? handle
      guard (info.declaration.abilities.contains .key)
      let type ← view.nty ⟨0⟩ typeId
      let arguments ← arguments.toList.mapM fun argument => match argument with
        | .typeArg value => view.nty ⟨0⟩ value.typeId
        | _ => none
      some ⟨type, NRow.ofList arguments, .value⟩
  | _ => none

def resourceTyped (agree : Bool) (type : Nat) : Bool :=
  match (view.resourceOf? ⟨type⟩).bind fun resource =>
      if resource.type.paramFree && resource.type.refFree then some resource else none with
  | none => true
  | some resource =>
      resource.type.paramFree && resource.type.refFree && (resource.type.closureFree || agree) &&
        match view.resolve #[] ⟨type⟩ with
        | some type => view.typedAs resource.type type
        | none => false

def resourcesTyped (agree : Bool) : Bool :=
  (List.range view.typeCount).all (view.resourceTyped agree)

end TypingView

section Checks

variable {unit : ValidatedUnit}

theorem sameTypes_refl : (types : List Ty) → sameTypes types types = true
  | [] => rfl
  | type :: types => by
      simp only [sameTypes, Bool.and_eq_true, (Ty.beq_iff type type).mpr rfl, sameTypes_refl types,
        and_self]

theorem sameNames_refl : (names : List QualifiedName) → sameNames names names = true
  | [] => rfl
  | name :: names => by simp [sameNames, sameNames_refl names]

/-- A unit whose namespaces hold its tables passes a check of every type of
every namespace by a check of the first's. -/
theorem everyNamespaceType_of_first (shared : TablesShared unit)
    {check : Nat → ValidatedNamespace → Nat → Bool}
    (first : ∀ ns, unit.namespaces[0]? = some ns → ∀ type, type < unit.tables.types.size →
      check 0 ns type = true) :
    everyNamespaceType unit check = true := by
  simp only [everyNamespaceType, List.all_eq_true, List.mem_range]
  intro index inside
  cases found : unit.namespaces[index]? with
  | none => rfl
  | some ns =>
      simp only
      by_cases isFirst : index = 0
      · subst isFirst
        simp only [bne_self_eq_false, Bool.false_and, Bool.false_or, List.all_eq_true,
          List.mem_range]
        intro type inTable
        rw [shared.tables found] at inTable
        exact first ns found type inTable
      · have firstInside : 0 < unit.namespaces.size := by omega
        have tables := shared.tables found
        have firstFound : unit.namespaces[0]? = some unit.namespaces[0] :=
          Array.getElem?_eq_getElem firstInside
        have firstTables := shared.tables firstFound
        simp [isFirst, firstFound, sameTables, tables, firstTables, sameTypes_refl, sameNames_refl]

theorem TypingView.typeCount_of : (TypingView.of unit).typeCount = unit.tables.types.size := rfl

theorem TypingView.parameterNode_of (shared : TablesShared unit) {namespaceId : NamespaceId}
    {ns : ValidatedNamespace} (found : unit.namespaces[namespaceId.index]? = some ns)
    (index : Nat) :
    (TypingView.of unit).parameterNode unit namespaceId index =
      (paramNodeIn? unit namespaceId index).isSome := by
  simp only [TypingView.parameterNode]
  split
  · next inside =>
      have tables := shared.tables found
      have limit : index < parameterLimitOf unit := inside.1
      simp only [TypingView.of, IndexedArena.get?_ofArray, List.getElem?_toArray,
        List.getElem?_map, List.getElem?_range limit, Option.map_some, Option.getD_some,
        parameterNodeAt, paramNodeIn?, found, Option.bind_eq_bind, Option.bind_some, tables]
      cases unit.tables.types.findIdx? _ <;> rfl
  · rfl

theorem TypingView.signatureTypes?_of {ns : ValidatedNamespace} (tables : ns.tables = unit.tables)
    (declaration : FunctionDecl FunctionBody) (env : Array SemArg) :
    (TypingView.of unit).signatureTypes? ns.links declaration env =
      StaticTyping.signatureTypes? ns declaration env := by
  simp only [TypingView.signatureTypes?, StaticTyping.signatureTypes?, TypingView.resolve_of tables,
    StaticTyping.resolveIn, ValidatedNamespace.typeId]

theorem TypingView.closureSignature?_of (shared : TablesShared unit) {index functionIndex : Nat}
    {ns : ValidatedNamespace} {declaration : FunctionDecl FunctionBody}
    (found : unit.namespaces[index]? = some ns) (declared : ns.functions[functionIndex]? = some declaration) :
    (TypingView.of unit).closureSignature? unit ⟨index⟩ ns.links declaration =
      Denote.closureSignature? unit ⟨⟨index⟩, ⟨functionIndex⟩⟩ := by
  have ntyAt := TypingView.nty_of shared
  simp only [TypingView.closureSignature?, Denote.closureSignature?, found, declared,
    Option.bind_eq_bind, Option.bind_some, ntyAt, TypingView.parameterNode_of shared found,
    TypingView.signatureSharingAtTop, signatureSharingAtTop,
    TypingView.signatureTypes?_of (shared.tables found), ValidatedNamespace.typeId]
  rfl

theorem TypingView.signatureAgrees_of (shared : TablesShared unit) {index functionIndex : Nat}
    {ns : ValidatedNamespace} {declaration : FunctionDecl FunctionBody}
    (found : unit.namespaces[index]? = some ns) (declared : ns.functions[functionIndex]? = some declaration) :
    (TypingView.of unit).signatureAgrees unit ⟨index⟩ ns.links declaration =
      VerexIR.Proofs.signatureAgrees unit ⟨⟨index⟩, ⟨functionIndex⟩⟩ := by
  obtain ⟨_, parameters, _, _, _⟩ := TypingView.typedAs_of (unit := unit) shared
  simp only [TypingView.signatureAgrees, VerexIR.Proofs.signatureAgrees, found, declared,
    TypingView.closureSignature?_of shared found declared,
    TypingView.signatureTypes?_of (shared.tables found), TypingView.lookup_of shared, parameters,
    parameterSharing, resultSharing, ValidatedNamespace.typeId]
  rcases Denote.closureSignature? unit ⟨⟨index⟩, ⟨functionIndex⟩⟩ with _ | ⟨_, _⟩
  · rfl
  · cases StaticTyping.signatureTypes? ns declaration _ with
    | none => rfl
    | some _ => rfl

theorem TypingView.typeAgrees_of (shared : TablesShared unit) {ns : ValidatedNamespace}
    (found : unit.namespaces[0]? = some ns) (type : Nat) :
    (TypingView.of unit).typeAgrees type = VerexIR.Proofs.typeAgrees unit 0 ns type := by
  obtain ⟨typed, _, _, _, _⟩ := TypingView.typedAs_of (unit := unit) shared
  simp only [TypingView.typeAgrees, VerexIR.Proofs.typeAgrees, TypingView.nty_of shared,
    TypingView.resolve_of (shared.tables found), typed, NTy.typedAsCheck]
  cases ntyOf unit ⟨0⟩ ⟨type⟩ <;> cases StaticTyping.resolveAt ns #[] ⟨type⟩ <;> rfl

/-- A passing indexed check gives the array-reading one. -/
theorem typesAgreeCheck_of_indexed (shared : TablesShared unit)
    (check : (TypingView.of unit).typesAgree unit = true) : typesAgreeCheck unit = true := by
  obtain ⟨types, functions⟩ := (Bool.and_eq_true _ _).mp check
  simp only [List.all_eq_true, List.mem_range, TypingView.typeCount_of] at types
  simp only [List.all_eq_true] at functions
  refine (Bool.and_eq_true _ _).mpr ⟨everyNamespaceType_of_first shared ?_, ?_⟩
  · intro ns found type inTable
    rw [← TypingView.typeAgrees_of shared found]
    exact types type inTable
  · simp only [List.all_eq_true, List.mem_range]
    intro index inside
    cases found : unit.namespaces[index]? with
    | none => rfl
    | some ns =>
        simp only [List.all_eq_true, List.mem_range]
        intro functionIndex inFunctions
        have member : (ns, index) ∈ unit.namespaces.toList.zipIdx :=
          List.mem_zipIdx_iff_getElem?.mpr (by simpa using found)
        have declared := Array.getElem?_eq_getElem inFunctions
        rw [← TypingView.signatureAgrees_of shared found declared]
        exact functions _ member _ (Array.mem_toList_iff.mpr (Array.getElem_mem inFunctions))

theorem TypingView.resourceOf?_of (shared : TablesShared unit) {ns : ValidatedNamespace}
    (found : unit.namespaces[0]? = some ns) (typeId : TypeId) :
    (TypingView.of unit).resourceOf? typeId = Denote.resourceOf unit ⟨0⟩ typeId := by
  simp only [TypingView.resourceOf?, Denote.resourceOf, TypingView.lookup_of shared,
    Option.bind_eq_bind, TypingView.nty_of shared]
  refine congrArg (Option.bind _) (funext fun ty => ?_)
  cases ty <;> try rfl
  simp only [show unit.namespaces[(⟨0⟩ : NamespaceId).index]? = some ns from found, Option.bind_some,
    TypingView.resolveNominal?_of shared found]
  refine congrArg (Option.bind _) (funext fun handle => ?_)
  simp only [TypingView.declarationAt?_of]
  cases unit.namespaces[handle.namespaceId.index]? with
  | none => rfl
  | some target =>
      simp only [Option.bind_some]
      cases target.structs[handle.structId]? <;> rfl

theorem TypingView.resourceTyped_of (shared : TablesShared unit) {ns : ValidatedNamespace}
    (found : unit.namespaces[0]? = some ns) (agree : Bool) (type : Nat) :
    (TypingView.of unit).resourceTyped agree type = resourceTypedCheck unit agree ⟨0⟩ ns ⟨type⟩ := by
  obtain ⟨typed, _, _, _, _⟩ := TypingView.typedAs_of (unit := unit) shared
  simp only [TypingView.resourceTyped, resourceTypedCheck, Denote.runtimeResourceOf,
    TypingView.resourceOf?_of shared found, TypingView.resolve_of (shared.tables found), typed,
    NTy.typedAsCheck, Option.bind_eq_bind]
  cases Denote.resourceOf unit ⟨0⟩ ⟨type⟩ with
  | none => rfl
  | some resource =>
      simp only [Option.bind_some, guard]
      by_cases closed : (resource.type.paramFree && resource.type.refFree) = true <;>
        simp [closed] <;> rfl

/-- A passing indexed check gives the array-reading one. -/
theorem resourcesTypedCheck_of_indexed (shared : TablesShared unit) {agree : Bool}
    (check : (TypingView.of unit).resourcesTyped agree = true) :
    resourcesTypedCheck unit agree = true := by
  simp only [TypingView.resourcesTyped, List.all_eq_true, List.mem_range,
    TypingView.typeCount_of] at check
  refine everyNamespaceType_of_first shared fun ns found type inTable => ?_
  rw [← TypingView.resourceTyped_of shared found]
  exact check type inTable

/-- `TypesAgree` from the indexed check over a published view of the unit. -/
theorem TypesAgree.ofIndexed {view : TypingView} (shared : TablesShared unit)
    (viewEq : TypingView.of unit = view) (check : view.typesAgree unit = true) :
    TypesAgree unit :=
  TypesAgree.ofCheck (typesAgreeCheck_of_indexed shared (viewEq ▸ check))

/-- `ResourcesTyped` from the indexed check over a published view of the unit. -/
theorem ResourcesTyped.ofIndexed {view : TypingView} {agree : Bool} (shared : TablesShared unit)
    (viewEq : TypingView.of unit = view) (agreed : agree = true → TypesAgree unit)
    (check : view.resourcesTyped agree = true) : ResourcesTyped unit :=
  ResourcesTyped.ofCheck agreed (resourcesTypedCheck_of_indexed shared (viewEq ▸ check))

/-- `ResourcesTyped` where the unit's readings agree. -/
theorem ResourcesTyped.ofIndexedAgreeing {view : TypingView} (shared : TablesShared unit)
    (viewEq : TypingView.of unit = view) (agreement : TypesAgree unit)
    (check : view.resourcesTyped true = true) : ResourcesTyped unit :=
  ResourcesTyped.ofIndexed shared viewEq (fun _ => agreement) check

/-- `ResourcesTyped` where the unit's readings do not agree. -/
theorem ResourcesTyped.ofIndexedDisagreeing {view : TypingView} (shared : TablesShared unit)
    (viewEq : TypingView.of unit = view) (check : view.resourcesTyped false = true) :
    ResourcesTyped unit :=
  ResourcesTyped.ofIndexed shared viewEq (fun absurd => by cases absurd) check

end Checks

end VerexIR.Proofs
