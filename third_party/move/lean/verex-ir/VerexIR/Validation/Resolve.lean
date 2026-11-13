-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Std.Data.HashMap
import VerexIR.Import.Raw
import VerexIR.Validation.Diagnostic

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

namespace VerexIR.Validation

open VerexIR.Import (RawNamespaceInterface)

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
  /-- Lemmas, by their position in their namespace. -/
  lemmas : Array (Option Nat)
  dependencyInterfaces : Array Bool
  externallyDeclared : Array Bool
  deriving Repr, BEq, Inhabited

namespace ResolutionIndex

/-- The first interned `NameId` with this name's owner, scope, and spelling. Duplicate
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

theorem nominal?_eq (index : ResolutionIndex) (name : NameId) :
    index.nominal? name =
      match index.nominals[(index.canonical name).index]? with
      | some (some value) => some value
      | _ => none := by
  simp only [nominal?, entry?, Option.bind_eq_bind]
  cases index.nominals[(index.canonical name).index]? with
  | none => rfl
  | some entry => cases entry <;> rfl

def trait? (index : ResolutionIndex) (name : NameId) : Option TraitDeclId :=
  index.entry? index.traits name

def specFunction? (index : ResolutionIndex) (name : NameId) : Option SpecFunctionId :=
  index.entry? index.specFunctions name

def lemma? (index : ResolutionIndex) (name : NameId) : Option Nat :=
  index.entry? index.lemmas name

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
  let mut seen : Std.HashMap (Nat × Array String × String) NameId := {}
  for (name, index) in tables.names.zipIdx do
    let key := (name.namespaceId.index, name.scope, name.name)
    match seen[key]? with
    | some first => canonical := canonical.push first
    | none =>
        seen := seen.insert key ⟨index⟩
        canonical := canonical.push (⟨index⟩ : NameId)
  return canonical

private def declarationEntries {α β : Type} (canonical : Array NameId)
    (nameCount : Nat) (namespaces : Array (Namespace β)) (nameId : Nat → NameId → NameId)
    (declarations : Namespace β → Array α) (nameOf : α → NameId) :
    Array (Option Nat) := Id.run do
  let mut entries := Array.replicate nameCount none
  for (ns, namespaceIndex) in namespaces.zipIdx do
    for (declaration, index) in (declarations ns).zipIdx do
      let nameIndex := (nameId namespaceIndex (nameOf declaration)).index
      if let some canonicalName := canonical[nameIndex]? then
        entries := entries.set! canonicalName.index (some index)
  return entries

/-- Build the resolution index for a unit's owned namespaces and declared
dependency interfaces. Declarations and tables are unchanged by
structurization, so the index is valid for both stages. `nameId` gives the
unit identifier of a name of the namespace at an index. -/
def build {β : Type} (tables : Tables) (namespaces : Array (Namespace β))
    (dependencies : Array RawNamespaceInterface)
    (nameId : Nat → NameId → NameId := fun _ id => id) : ResolutionIndex := Id.run do
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
    functions := (declarationEntries canonical nameCount namespaces nameId
      (fun ns => ns.functions) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    constants := (declarationEntries canonical nameCount namespaces nameId
      (fun ns => ns.constants) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    nominals := (declarationEntries canonical nameCount namespaces nameId
      (fun ns => ns.structs) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    traits := (declarationEntries canonical nameCount namespaces nameId
      (fun ns => ns.traits) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    specFunctions := (declarationEntries canonical nameCount namespaces nameId
      (fun ns => ns.specFunctions) (fun declaration => declaration.name)).map (·.map (⟨·⟩))
    lemmas := declarationEntries canonical nameCount namespaces nameId
      (fun ns => ns.lemmas) (fun declaration => declaration.name)
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
  | lemma
  | callable

private def TargetKind.description : TargetKind → String
  | .function => "function"
  | .constant => "constant"
  | .nominal => "struct or enum"
  | .trait => "trait"
  | .specFunction => "specification function"
  | .lemma => "lemma"
  | .callable => "function or specification function"

private def TargetKind.resolves (kind : TargetKind) (index : ResolutionIndex)
    (name : NameId) : Bool :=
  match kind with
  | .function => (index.function? name).isSome
  | .constant => (index.constant? name).isSome
  | .nominal => (index.nominal? name).isSome
  | .trait => (index.trait? name).isSome
  | .specFunction => (index.specFunction? name).isSome
  | .lemma => (index.lemma? name).isSome
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
          #[{ code := "VIR-SEMANTIC-TARGET"
              message := s!"{site} `{qualifiedName.name}` does not resolve to a declared {kind.description}"
              primary := loc }]
      else if !index.hasDependencyInterface qualifiedName.namespaceId then #[]
      else if index.externallyResolvable name then #[]
      else
        #[{ code := "VIR-SEMANTIC-TARGET"
            message := s!"{site} `{qualifiedName.name}` is not exported by the dependency interface of namespace {qualifiedName.namespaceId.index}"
            primary := loc }]

/-- The diagnostics of one name use of a namespace, its name translated to
the unit. -/
private abbrev Target := Option LocId → String → TargetKind → NameId → Array Diagnostic

private def operationTargetDiagnostics (target : Target) (loc : LocId) :
    Operation → Array Diagnostic
  | .call (.function callee) =>
      target (some loc) "call target" .function callee.name
  | .call (.closure callee _) =>
      target (some loc) "closure target" .function callee.name
  | .call (.constructor callee _) =>
      target (some loc) "constructor target" .nominal callee.name
  | .call (.destructor callee _) =>
      target (some loc) "destructor target" .nominal callee.name
  | .call (.extension _ targets) =>
      targets.foldl (fun ds reference => ds ++
        target (some loc) "operation target" .callable reference.name) #[]
  | .profile _ targets =>
      targets.foldl (fun ds reference => ds ++
        target (some loc) "operation target" .callable reference.name) #[]
  -- Data-operation targets are deliberately not resolved here: they are
  -- type-directed (the operand's nominal type selects the declaration, and
  -- builtin carriers such as `signer` use a courtesy label the nominal tables
  -- cannot resolve), so they are checked by the typing pass with the operand
  -- type in hand, like `Place.field` and constructor patterns.
  | .specification (.functionCall reference _) =>
      -- A specification call may target a specification function or the
      -- logical view of an executable function.
      target (some loc) "specification call target"
        .callable reference.name
  | .specification (.lemma reference _) =>
      target (some loc) "lemma application" .lemma reference.name
  | _ => #[]

private def traitRefDiagnostics (target : Target) (loc : LocId) (trait : TraitRef) :
    Array Diagnostic :=
  target (some loc) "trait reference" .trait trait.trait.name

private def predicateDiagnostics (target : Target) (loc : LocId) :
    GenericPredicate → Array Diagnostic
  | .implements _ trait | .associatedTypeEq trait _ _ | .associatedConstEq trait _ _ =>
      traitRefDiagnostics target loc trait
  | _ => #[]

private def binderDiagnostics (target : Target) (binders : Array GenericBinder) :
    Array Diagnostic :=
  binders.foldl (init := #[]) fun ds binder =>
    binder.predicates.foldl (fun ds predicate =>
      ds ++ predicateDiagnostics target binder.loc predicate) ds

private def signatureDiagnostics (target : Target) (loc : LocId) (signature : Signature) :
    Array Diagnostic :=
  binderDiagnostics target signature.generics ++
    signature.predicates.foldl (fun ds predicate =>
      ds ++ predicateDiagnostics target loc predicate) #[]

/-- Resolve every authoritative use site of a structurized unit. Type-table
entries are checked once per interned type; expression, declaration, trait,
and implementation sites are checked in source order per namespace. -/
def resolveUseSites {β : Type} (tables : Tables) (index : ResolutionIndex)
    (ownedCount : Nat) (namespaces : Array (Namespace β))
    (nameId : Nat → NameId → NameId := fun _ id => id) : Array Diagnostic :=
  let typeErrors := tables.types.foldl (init := #[]) fun ds ty =>
    match ty with
    | .nominal name _ =>
        ds ++ targetDiagnostics tables index ownedCount none "nominal type" .nominal name
    | .resourceDomain resource _ =>
        ds ++ targetDiagnostics tables index ownedCount none "resource domain" .nominal resource
    | _ => ds
  namespaces.zipIdx.foldl (init := typeErrors) fun ds (ns, namespaceIndex) =>
    let target : Target := fun loc site kind name =>
      targetDiagnostics tables index ownedCount loc site kind (nameId namespaceIndex name)
    let exprErrors := ns.expressions.foldl (init := #[]) fun ds expr =>
      match expr.kind with
      | .constant reference =>
          ds ++ target (some expr.loc)
            "constant reference" .constant reference.name
      | .operation op _ _ _ =>
          ds ++ operationTargetDiagnostics target expr.loc op
      | _ => ds
    let functionErrors := ns.functions.foldl (init := #[]) fun ds function =>
      function.shapes.foldl (init :=
          ds ++ signatureDiagnostics target function.loc function.signature)
        fun ds shape => ds ++ signatureDiagnostics target shape.loc shape.signature
    let specFunctionErrors := ns.specFunctions.foldl (init := #[]) fun ds function =>
      ds ++ signatureDiagnostics target function.loc function.signature
    let specFunctionErrors := ns.lemmas.foldl (init := specFunctionErrors) fun ds lemma =>
      ds ++ signatureDiagnostics target lemma.loc lemma.signature
    let structErrors := ns.structs.foldl (init := #[]) fun ds struct =>
      ds ++ binderDiagnostics target struct.generics
    let specVarErrors := ns.specVars.foldl (init := #[]) fun ds specVar =>
      ds ++ binderDiagnostics target specVar.generics
    let traitErrors := ns.traits.foldl (init := #[]) fun ds trait =>
      let ds := ds ++ binderDiagnostics target trait.generics
      let ds := trait.superTraits.foldl (fun ds parent =>
        ds ++ traitRefDiagnostics target trait.loc parent) ds
      trait.predicates.foldl (fun ds predicate =>
        ds ++ predicateDiagnostics target trait.loc predicate) ds
    let associatedItemErrors := ns.associatedItems.foldl (init := #[]) fun ds item =>
      match item.kind with
      | .type bounds _ => bounds.foldl (fun ds predicate =>
          ds ++ predicateDiagnostics target item.loc predicate) ds
      | .constant _ _ => ds
      | .method signature defaultImplementation =>
          let ds := ds ++ signatureDiagnostics target item.loc signature
          match defaultImplementation with
          | some implementation => ds ++ target (some item.loc)
              "default method implementation" .function implementation.name
          | none => ds
    let implementationErrors := ns.implementations.foldl (init := #[]) fun ds implementation =>
      let ds := ds ++ binderDiagnostics target
        implementation.generics
      let ds := ds ++ traitRefDiagnostics target implementation.loc
        implementation.trait
      let ds := implementation.predicates.foldl (fun ds predicate =>
        ds ++ predicateDiagnostics target implementation.loc predicate) ds
      implementation.bindings.foldl (init := ds) fun ds binding =>
        match binding.value with
        | .method implementation => ds ++ target (some binding.loc)
            "bound method implementation" .function implementation.name
        | _ => ds
    ds ++ exprErrors ++ functionErrors ++ specFunctionErrors ++ structErrors ++
      specVarErrors ++ traitErrors ++ associatedItemErrors ++ implementationErrors

end VerexIR.Validation
