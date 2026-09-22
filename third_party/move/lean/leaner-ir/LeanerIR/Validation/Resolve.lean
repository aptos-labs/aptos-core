-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Std.Data.HashMap
import LeanerIR.Import.Raw
import LeanerIR.Validation.Diagnostic

/-!
# Authoritative name resolution

Pass 1 of validation resolves every semantic name use to a typed declaration
identity. The `ResolutionIndex` maps each interned name to the declaration it
denotes per declaration kind, so downstream passes and semantic consumers look
targets up by identity instead of re-deriving resolution by string comparison.

A name owned by one of the unit's namespaces must resolve to a declaration of
the kind its use site requires; failure is a validation error. A name owned by
an external namespace resolves against that namespace's dependency interface:
an interface with an explicit export list rejects unlisted names, while an
interface without exports — and, transitionally, an absent interface — leaves
the reference an unchecked external target for the dependency-interface
enrichment to tighten.
-/

namespace LeanerIR.Validation

open LeanerIR.Import (RawNamespaceInterface)

/-- Checked resolution facts for one compilation unit. Every array except
`dependencyInterfaces` is keyed by `NameId` position; declaration entries are
populated at the canonical (first-interned) name of each declaration.
`dependencyInterfaces` is keyed by table-namespace position. -/
structure ResolutionIndex where
  private mk ::
  canonicalNames : Array NameId
  functions : Array (Option FunctionId)
  constants : Array (Option ConstantDeclId)
  nominals : Array (Option TypeDeclId)
  traits : Array (Option TraitDeclId)
  specFunctions : Array (Option SpecFunctionId)
  dependencyInterfaces : Array Bool
  externallyDeclared : Array Bool
  deriving Repr, BEq, Inhabited

namespace ResolutionIndex

/-- The first interned `NameId` with this name's owner and spelling. Duplicate
table entries are legal raw input; resolution treats them as one name. -/
def canonical (index : ResolutionIndex) (name : NameId) : NameId :=
  index.canonicalNames[name.index]?.getD name

private def entry? {α : Type} (index : ResolutionIndex)
    (entries : Array (Option α)) (name : NameId) : Option α := do
  let some value ← entries[(index.canonical name).index]? | none
  value

def function? (index : ResolutionIndex) (name : NameId) : Option FunctionId :=
  index.entry? index.functions name

def constant? (index : ResolutionIndex) (name : NameId) : Option ConstantDeclId :=
  index.entry? index.constants name

def nominal? (index : ResolutionIndex) (name : NameId) : Option TypeDeclId :=
  index.entry? index.nominals name

def trait? (index : ResolutionIndex) (name : NameId) : Option TraitDeclId :=
  index.entry? index.traits name

def specFunction? (index : ResolutionIndex) (name : NameId) : Option SpecFunctionId :=
  index.entry? index.specFunctions name

/-- Whether an external name is covered by its namespace's dependency
interface: listed in a nonempty export list, or any name of an interface
without exports. -/
def externallyResolvable (index : ResolutionIndex) (name : NameId) : Bool :=
  index.externallyDeclared[(index.canonical name).index]?.getD false

/-- Whether a table namespace has a declared dependency interface. -/
def hasDependencyInterface (index : ResolutionIndex) (namespaceId : NamespaceId) : Bool :=
  index.dependencyInterfaces[namespaceId.index]?.getD false

private def canonicalizeNames (tables : Tables) : Array NameId := Id.run do
  let mut canonical := Array.mkEmpty tables.names.size
  let mut seen : Std.HashMap (Nat × String) NameId := {}
  for (name, index) in tables.names.zipIdx do
    let key := (name.namespaceId.index, name.name)
    match seen[key]? with
    | some first => canonical := canonical.push first
    | none =>
        seen := seen.insert key ⟨index⟩
        canonical := canonical.push (⟨index⟩ : NameId)
  return canonical

private def declarationEntries {α β : Type} (canonical : Array NameId)
    (nameCount : Nat) (namespaces : Array (Namespace β))
    (declarations : Namespace β → Array α) (nameOf : α → NameId) :
    Array (Option Nat) := Id.run do
  let mut entries := Array.replicate nameCount none
  for ns in namespaces do
    for (declaration, index) in (declarations ns).zipIdx do
      let nameIndex := (nameOf declaration).index
      if let some canonicalName := canonical[nameIndex]? then
        entries := entries.set! canonicalName.index (some index)
  return entries

/-- Build the resolution index for a unit's owned namespaces and declared
dependency interfaces. Declarations and tables are unchanged by
structurization, so the index is valid for both stages. -/
def build {β : Type} (tables : Tables) (namespaces : Array (Namespace β))
    (dependencies : Array RawNamespaceInterface) : ResolutionIndex := Id.run do
  let canonical := canonicalizeNames tables
  let nameCount := tables.names.size
  let mut dependencyInterfaces := Array.replicate tables.namespaces.size false
  for dependency in dependencies do
    if dependency.namespaceId.index < dependencyInterfaces.size then
      dependencyInterfaces := dependencyInterfaces.set! dependency.namespaceId.index true
  let mut externallyDeclared := Array.replicate nameCount false
  for dependency in dependencies do
    if dependency.exportedNames.isEmpty then
      for (name, index) in tables.names.zipIdx do
        if name.namespaceId == dependency.namespaceId then
          if let some canonicalName := canonical[index]? then
            externallyDeclared := externallyDeclared.set! canonicalName.index true
    else
      for exported in dependency.exportedNames do
        if let some canonicalName := canonical[exported.index]? then
          externallyDeclared := externallyDeclared.set! canonicalName.index true
  return {
    canonicalNames := canonical
    functions := (declarationEntries canonical nameCount namespaces
      (fun ns => ns.functions) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    constants := (declarationEntries canonical nameCount namespaces
      (fun ns => ns.constants) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    nominals := (declarationEntries canonical nameCount namespaces
      (fun ns => ns.structs) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    traits := (declarationEntries canonical nameCount namespaces
      (fun ns => ns.traits) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    specFunctions := (declarationEntries canonical nameCount namespaces
      (fun ns => ns.specFunctions) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    dependencyInterfaces
    externallyDeclared }

end ResolutionIndex

/-- Declaration kind an authoritative use site requires. `callable` admits
executable and specification functions, matching profile-operation targets. -/
private inductive TargetKind where
  | function
  | constant
  | nominal
  | trait
  | specFunction
  | callable

private def TargetKind.description : TargetKind → String
  | .function => "function"
  | .constant => "constant"
  | .nominal => "struct or enum"
  | .trait => "trait"
  | .specFunction => "specification function"
  | .callable => "function or specification function"

private def TargetKind.resolves (kind : TargetKind) (index : ResolutionIndex)
    (name : NameId) : Bool :=
  match kind with
  | .function => (index.function? name).isSome
  | .constant => (index.constant? name).isSome
  | .nominal => (index.nominal? name).isSome
  | .trait => (index.trait? name).isSome
  | .specFunction => (index.specFunction? name).isSome
  | .callable => (index.function? name).isSome || (index.specFunction? name).isSome

/-- Resolve one semantic name use. Owned names must resolve to the required
declaration kind. External names are checked against a declared dependency
interface's export list; without an interface the reference remains an
unchecked external target until dependency interfaces are enriched. -/
private def targetDiagnostics (tables : Tables) (index : ResolutionIndex)
    (ownedCount : Nat) (loc : Option LocId) (site : String) (kind : TargetKind)
    (name : NameId) : Array Diagnostic :=
  match tables.names[name.index]? with
  | none => #[]
  | some qualifiedName =>
      if qualifiedName.namespaceId.index < ownedCount then
        if kind.resolves index name then #[] else
          #[{ code := "LIR-SEMANTIC-TARGET"
              message := s!"{site} `{qualifiedName.name}` does not resolve to a declared {kind.description}"
              primary := loc }]
      else if !index.hasDependencyInterface qualifiedName.namespaceId then #[]
      else if index.externallyResolvable name then #[]
      else
        #[{ code := "LIR-SEMANTIC-TARGET"
            message := s!"{site} `{qualifiedName.name}` is not exported by the dependency interface of namespace {qualifiedName.namespaceId.index}"
            primary := loc }]

private def operationTargetDiagnostics (tables : Tables) (index : ResolutionIndex)
    (ownedCount : Nat) (loc : LocId) : Operation → Array Diagnostic
  | .call (.function callee) =>
      targetDiagnostics tables index ownedCount (some loc) "call target" .function callee.name
  | .call (.closure callee) =>
      targetDiagnostics tables index ownedCount (some loc) "closure target" .function callee.name
  | .call (.constructor callee _) =>
      targetDiagnostics tables index ownedCount (some loc) "constructor target" .nominal callee.name
  | .call (.destructor callee _) =>
      targetDiagnostics tables index ownedCount (some loc) "destructor target" .nominal callee.name
  | .call (.extension _ targets) =>
      targets.foldl (fun ds target => ds ++
        targetDiagnostics tables index ownedCount (some loc) "operation target" .callable target.name) #[]
  | .profile _ targets =>
      targets.foldl (fun ds target => ds ++
        targetDiagnostics tables index ownedCount (some loc) "operation target" .callable target.name) #[]
  -- Data-operation targets are deliberately not resolved here: they are
  -- type-directed (the operand's nominal type selects the declaration, and
  -- builtin carriers such as `signer` use a courtesy label the nominal tables
  -- cannot resolve), so they are checked by the typing pass with the operand
  -- type in hand, like `Place.field` and constructor patterns.
  | .specification (.functionCall reference _) =>
      -- A specification call may target a specification function or the
      -- logical view of an executable function.
      targetDiagnostics tables index ownedCount (some loc) "specification call target"
        .callable reference.name
  | _ => #[]

private def traitRefDiagnostics (tables : Tables) (index : ResolutionIndex)
    (ownedCount : Nat) (loc : LocId) (trait : TraitRef) : Array Diagnostic :=
  targetDiagnostics tables index ownedCount (some loc) "trait reference" .trait trait.trait.name

private def predicateDiagnostics (tables : Tables) (index : ResolutionIndex)
    (ownedCount : Nat) (loc : LocId) : GenericPredicate → Array Diagnostic
  | .implements _ trait | .associatedTypeEq trait _ _ | .associatedConstEq trait _ _ =>
      traitRefDiagnostics tables index ownedCount loc trait
  | _ => #[]

private def binderDiagnostics (tables : Tables) (index : ResolutionIndex)
    (ownedCount : Nat) (binders : Array GenericBinder) : Array Diagnostic :=
  binders.foldl (init := #[]) fun ds binder =>
    binder.predicates.foldl (fun ds predicate =>
      ds ++ predicateDiagnostics tables index ownedCount binder.loc predicate) ds

private def signatureDiagnostics (tables : Tables) (index : ResolutionIndex)
    (ownedCount : Nat) (loc : LocId) (signature : Signature) : Array Diagnostic :=
  binderDiagnostics tables index ownedCount signature.generics ++
    signature.predicates.foldl (fun ds predicate =>
      ds ++ predicateDiagnostics tables index ownedCount loc predicate) #[]

/-- Resolve every authoritative use site of a structurized unit. Type-table
entries are checked once per interned type; expression, declaration, trait,
and implementation sites are checked in source order per namespace. -/
def resolveUseSites {β : Type} (tables : Tables) (index : ResolutionIndex)
    (ownedCount : Nat) (namespaces : Array (Namespace β)) : Array Diagnostic :=
  let typeErrors := tables.types.foldl (init := #[]) fun ds ty =>
    match ty with
    | .nominal name _ =>
        ds ++ targetDiagnostics tables index ownedCount none "nominal type" .nominal name
    | .resourceDomain resource _ =>
        ds ++ targetDiagnostics tables index ownedCount none "resource domain" .nominal resource
    | _ => ds
  namespaces.foldl (init := typeErrors) fun ds ns =>
    let exprErrors := ns.expressions.foldl (init := #[]) fun ds expr =>
      match expr.kind with
      | .constant reference =>
          ds ++ targetDiagnostics tables index ownedCount (some expr.loc)
            "constant reference" .constant reference.name
      | .operation op _ _ _ =>
          ds ++ operationTargetDiagnostics tables index ownedCount expr.loc op
      | _ => ds
    let functionErrors := ns.functions.foldl (init := #[]) fun ds function =>
      ds ++ signatureDiagnostics tables index ownedCount function.loc function.signature
    let specFunctionErrors := ns.specFunctions.foldl (init := #[]) fun ds function =>
      ds ++ signatureDiagnostics tables index ownedCount function.loc function.signature
    let structErrors := ns.structs.foldl (init := #[]) fun ds struct =>
      ds ++ binderDiagnostics tables index ownedCount struct.generics
    let specVarErrors := ns.specVars.foldl (init := #[]) fun ds specVar =>
      ds ++ binderDiagnostics tables index ownedCount specVar.generics
    let traitErrors := ns.traits.foldl (init := #[]) fun ds trait =>
      let ds := ds ++ binderDiagnostics tables index ownedCount trait.generics
      let ds := trait.superTraits.foldl (fun ds parent =>
        ds ++ traitRefDiagnostics tables index ownedCount trait.loc parent) ds
      trait.predicates.foldl (fun ds predicate =>
        ds ++ predicateDiagnostics tables index ownedCount trait.loc predicate) ds
    let associatedItemErrors := ns.associatedItems.foldl (init := #[]) fun ds item =>
      match item.kind with
      | .type bounds _ => bounds.foldl (fun ds predicate =>
          ds ++ predicateDiagnostics tables index ownedCount item.loc predicate) ds
      | .constant _ _ => ds
      | .method signature defaultImplementation =>
          let ds := ds ++ signatureDiagnostics tables index ownedCount item.loc signature
          match defaultImplementation with
          | some target => ds ++ targetDiagnostics tables index ownedCount (some item.loc)
              "default method implementation" .function target.name
          | none => ds
    let implementationErrors := ns.implementations.foldl (init := #[]) fun ds implementation =>
      let ds := ds ++ binderDiagnostics tables index ownedCount
        implementation.generics
      let ds := ds ++ traitRefDiagnostics tables index ownedCount implementation.loc
        implementation.trait
      let ds := implementation.predicates.foldl (fun ds predicate =>
        ds ++ predicateDiagnostics tables index ownedCount implementation.loc predicate) ds
      implementation.bindings.foldl (init := ds) fun ds binding =>
        match binding.value with
        | .method target => ds ++ targetDiagnostics tables index ownedCount (some binding.loc)
            "bound method implementation" .function target.name
        | _ => ds
    ds ++ exprErrors ++ functionErrors ++ specFunctionErrors ++ structErrors ++
      specVarErrors ++ traitErrors ++ associatedItemErrors ++ implementationErrors

end LeanerIR.Validation
