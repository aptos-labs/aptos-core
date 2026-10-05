-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Relocate

/-!
# Relocatable namespaces and linking

A validated namespace can leave the unit it was validated in, as a
`RelocatableNamespace`: its own tables, holding exactly the entries it uses,
with itself at namespace 0 and every other namespace named by path, and its
validation results. A unit links relocatable namespaces without validating
them again: what validation established about a namespace does not depend on
the units that include it. Linking checks only the boundary: that the
declarations a unit was validated against equal the ones it links, and that
every name a linked namespace uses resolves in the unit.

Both directions are one operation, relocation: moving a namespace from one
set of tables into another, interning the entries it uses and renumbering
its identifiers through the derived `Remap` traversal.
-/

namespace LeanerIR.Validation

/-- A validated namespace detached from any unit. -/
structure RelocatableNamespace where
  tables : Tables
  profiles : Array ProfileConfig
  /-- The namespace, its identity 0. -/
  body : Namespace FunctionBody
  witnesses : Array StructurizationWitness := #[]
  initializationCertificates : Array InitializationCertificate := #[]
  borrowCertificates : Array BorrowCertificate := #[]
  borrowRejections : Array OwnedDiagnostic := #[]
  deriving Repr, BEq, Inhabited

/-- Where a relocatable namespace is. -/
def RelocatableNamespace.path (object : RelocatableNamespace) : Array String :=
  (object.tables.namespaces[0]?.map (·.segments)).getD #[]

/-- The other namespaces a relocatable namespace refers to. -/
def RelocatableNamespace.references (object : RelocatableNamespace) : Array NamespaceRef :=
  object.tables.namespaces.extract 1 object.tables.namespaces.size

/-- A namespace and its validation results, numbered in some tables. -/
private structure Placed where
  tables : Tables
  body : Namespace FunctionBody
  witnesses : Array StructurizationWitness
  initializationCertificates : Array InitializationCertificate
  borrowCertificates : Array BorrowCertificate
  borrowRejections : Array OwnedDiagnostic

private def RelocatableNamespace.placed (object : RelocatableNamespace) : Placed :=
  { object with }

/-- The namespace at `identity` of a validated unit, with its results. -/
private def placedIn (unit : ValidatedUnit) (identity : NamespaceId) : Option Placed := do
  let ns ← unit.namespaces[identity.index]?
  pure {
    tables := unit.tables
    body := ns.toNamespace
    witnesses := unit.structurizationWitnesses.filter (·.namespaceId == identity)
    initializationCertificates :=
      unit.initializationCertificates.filter (·.namespaceId == identity)
    borrowCertificates := unit.borrowCertificates.filter (·.namespaceId == identity)
    borrowRejections := unit.borrowRejections.filter (·.namespaceId == identity) }

/-! ## Relocation -/

deriving instance Hashable for
  Profile, ProfileValue, IntWidth, ConstValue, Ability, ReferenceKind, ReferenceType,
  TypeUse, GenericArgument, Ty, QualifiedName

/-- Relocation into target tables: what each source identifier became, per
kind, for the one source being relocated. Types, names, and namespaces are
values and are interned; every other entry keeps its identity and is
appended once. -/
private structure RelocState where
  target : Tables
  /-- Where each interned type and name is in the target; a type by its
  erasure, as the type table is location-free (`Ty.eraseLocs`). -/
  typeIndex : Std.HashMap Ty Nat := {}
  nameIndex : Std.HashMap QualifiedName Nat := {}
  files : Std.HashMap Nat FileId := {}
  locs : Std.HashMap Nat LocId := {}
  origins : Std.HashMap Nat OriginId := {}
  alignments : Std.HashMap Nat AlignmentId := {}
  types : Std.HashMap Nat TypeId := {}
  namespaces : Std.HashMap Nat NamespaceId := {}
  names : Std.HashMap Nat NameId := {}
  lifetimes : Std.HashMap Nat LifetimeId := {}

private abbrev RelocM := StateM RelocState

private def pureFns : IdFns RelocM := {
  file := pure, loc := pure, origin := pure, alignment := pure, profile := pure, type := pure,
  namespaceId := pure, name := pure, lifetime := pure }

instance : Inhabited (IdFns RelocM) := ⟨pureFns⟩

/-- The index of an entry equal to `value`, appending it when there is none. -/
private def intern [BEq α] (entries : Array α) (value : α) : Array α × Nat :=
  match entries.findIdx? (· == value) with
  | some index => (entries, index)
  | none => (entries.push value, entries.size)

mutual

/-- The relocation functions over source tables. -/
private partial def relocation (source : Tables) : IdFns RelocM := {
  file := relocateFile source
  loc := relocateLoc source
  origin := relocateOrigin source
  alignment := relocateAlignment source
  -- Linked units share their profile configuration.
  profile := pure
  type := relocateType source
  namespaceId := relocateNamespace source
  name := relocateName source
  lifetime := relocateLifetime source }

/-- Files keep their identity: a unit may hold one file twice, and what
refers to either stays apart. -/
private partial def relocateFile (source : Tables) (id : FileId) : RelocM FileId := do
  if let some done := (← get).files[id.index]? then return done
  let some file := source.files[id.index]? | return id
  let state ← get
  let result : FileId := ⟨state.target.files.size⟩
  set { state with
    target := { state.target with files := state.target.files.push file }
    files := state.files.insert id.index result }
  return result

private partial def relocateLoc (source : Tables) (id : LocId) : RelocM LocId := do
  if let some done := (← get).locs[id.index]? then return done
  let some location := source.locations[id.index]? | return id
  let location ← remap location (relocation source)
  let state ← get
  let result : LocId := ⟨state.target.locations.size⟩
  set { state with
    target := { state.target with locations := state.target.locations.push location }
    locs := state.locs.insert id.index result }
  return result

private partial def relocateOrigin (source : Tables) (id : OriginId) : RelocM OriginId := do
  if let some done := (← get).origins[id.index]? then return done
  let some origin := source.origins[id.index]? | return id
  let origin ← remap origin (relocation source)
  let state ← get
  let result : OriginId := ⟨state.target.origins.size⟩
  set { state with
    target := { state.target with origins := state.target.origins.push origin }
    origins := state.origins.insert id.index result }
  return result

private partial def relocateAlignment (source : Tables) (id : AlignmentId) :
    RelocM AlignmentId := do
  if let some done := (← get).alignments[id.index]? then return done
  let some alignment := source.alignments[id.index]? | return id
  let alignment ← remap alignment (relocation source)
  let state ← get
  let result : AlignmentId := ⟨state.target.alignments.size⟩
  set { state with
    target := { state.target with alignments := state.target.alignments.push alignment }
    alignments := state.alignments.insert id.index result }
  return result

private partial def relocateType (source : Tables) (id : TypeId) : RelocM TypeId := do
  if let some done := (← get).types[id.index]? then return done
  let some ty := source.types[id.index]? | return id
  let ty ← remap ty (relocation source)
  let state ← get
  let (state, index) := match state.typeIndex[ty.eraseLocs]? with
    | some index => (state, index)
    | none => let index := state.target.types.size
        ({ state with target := { state.target with types := state.target.types.push ty }
                      typeIndex := state.typeIndex.insert ty.eraseLocs index }, index)
  let result : TypeId := ⟨index⟩
  set { state with types := state.types.insert id.index result }
  return result

/-- A namespace is its path; the alias an earlier entry was spelled with
stays, and an entry spelled with none takes this one's. -/
private partial def relocateNamespace (source : Tables) (id : NamespaceId) :
    RelocM NamespaceId := do
  if let some done := (← get).namespaces[id.index]? then return done
  let some ref := source.namespaces[id.index]? | return id
  let state ← get
  let (namespaces, index) := match state.target.namespaces.findIdx? (· == ref) with
    | some index =>
        let existing := state.target.namespaces[index]!
        (if existing.alias.isNone && ref.alias.isSome then
          state.target.namespaces.set! index ref else state.target.namespaces, index)
    | none => (state.target.namespaces.push ref, state.target.namespaces.size)
  let result : NamespaceId := ⟨index⟩
  set { state with
    target := { state.target with namespaces }
    namespaces := state.namespaces.insert id.index result }
  return result

private partial def relocateName (source : Tables) (id : NameId) : RelocM NameId := do
  if let some done := (← get).names[id.index]? then return done
  let some name := source.names[id.index]? | return id
  let name ← remap name (relocation source)
  let state ← get
  let (state, index) := match state.nameIndex[name]? with
    | some index => (state, index)
    | none => let index := state.target.names.size
        ({ state with target := { state.target with names := state.target.names.push name }
                      nameIndex := state.nameIndex.insert name index }, index)
  let result : NameId := ⟨index⟩
  set { state with names := state.names.insert id.index result }
  return result

private partial def relocateLifetime (source : Tables) (id : LifetimeId) : RelocM LifetimeId := do
  if let some done := (← get).lifetimes[id.index]? then return done
  let some lifetime := source.lifetimes[id.index]? | return id
  let lifetime ← remap lifetime (relocation source)
  let state ← get
  let result : LifetimeId := ⟨state.target.lifetimes.size⟩
  set { state with
    target := { state.target with lifetimes := state.target.lifetimes.push lifetime }
    lifetimes := state.lifetimes.insert id.index result }
  return result

end

/-- The appended entries a value uses, by kind, and the interned ones already
walked. -/
private structure Used where
  files : Std.HashSet Nat := {}
  locs : Std.HashSet Nat := {}
  origins : Std.HashSet Nat := {}
  alignments : Std.HashSet Nat := {}
  lifetimes : Std.HashSet Nat := {}
  types : Std.HashSet Nat := {}
  names : Std.HashSet Nat := {}

private abbrev CollectM := StateM Used

instance : Inhabited (IdFns CollectM) := ⟨{
  file := pure, loc := pure, origin := pure, alignment := pure, profile := pure, type := pure,
  namespaceId := pure, name := pure, lifetime := pure }⟩

/-- Collect the entries a value uses, following each into its table. -/
private partial def collecting (source : Tables) : IdFns CollectM := {
  file := fun id => do modify fun used => { used with files := used.files.insert id.index }; pure id
  loc := fun id => do
    if (← get).locs.contains id.index then return id
    modify fun used => { used with locs := used.locs.insert id.index }
    if let some location := source.locations[id.index]? then
      discard <| remap location (collecting source)
    pure id
  origin := fun id => do
    if (← get).origins.contains id.index then return id
    modify fun used => { used with origins := used.origins.insert id.index }
    if let some origin := source.origins[id.index]? then discard <| remap origin (collecting source)
    pure id
  alignment := fun id => do
    if (← get).alignments.contains id.index then return id
    modify fun used => { used with alignments := used.alignments.insert id.index }
    if let some alignment := source.alignments[id.index]? then
      discard <| remap alignment (collecting source)
    pure id
  lifetime := fun id => do
    if (← get).lifetimes.contains id.index then return id
    modify fun used => { used with lifetimes := used.lifetimes.insert id.index }
    if let some lifetime := source.lifetimes[id.index]? then
      discard <| remap lifetime (collecting source)
    pure id
  type := fun id => do
    if (← get).types.contains id.index then return id
    modify fun used => { used with types := used.types.insert id.index }
    if let some ty := source.types[id.index]? then discard <| remap ty (collecting source)
    pure id
  name := fun id => do
    if (← get).names.contains id.index then return id
    modify fun used => { used with names := used.names.insert id.index }
    if let some name := source.names[id.index]? then discard <| remap name (collecting source)
    pure id
  namespaceId := pure
  profile := pure }

/-- Relocate a placed namespace into the target tables at `identity`, whose
path the target already holds. The appended entries it uses keep their
relative order, which source order reads. -/
private def relocatePlaced (placed : Placed) (identity : NamespaceId) : RelocM Placed := do
  let source := placed.tables
  let used := (do
    let collect := collecting source
    discard <| remap placed.body collect
    discard <| remap placed.witnesses collect
    discard <| remap placed.initializationCertificates collect
    discard <| remap placed.borrowCertificates collect
    discard <| remap placed.borrowRejections collect : CollectM Unit).run {} |>.2
  let ordered (entries : Std.HashSet Nat) := entries.toArray.qsort (· < ·)
  let assign (entries : Std.HashSet Nat) (base : Nat) : Std.HashMap Nat Nat :=
    (ordered entries).zipIdx.foldl (init := {}) fun memo (index, rank) =>
      memo.insert index (base + rank)
  let current ← get
  let target := current.target
  -- Identifiers of one source are memoized apart from another's; the
  -- appended entries are numbered in their source order before any is made.
  let wrap {κ : Type} (make : Nat → κ) (memo : Std.HashMap Nat Nat) : Std.HashMap Nat κ :=
    memo.fold (init := {}) fun result key value => result.insert key (make value)
  set ({
    target, typeIndex := current.typeIndex, nameIndex := current.nameIndex
    namespaces := ({} : Std.HashMap Nat NamespaceId).insert placed.body.identity.index identity
    files := wrap FileId.mk (assign used.files target.files.size)
    locs := wrap LocId.mk (assign used.locs target.locations.size)
    origins := wrap OriginId.mk (assign used.origins target.origins.size)
    alignments := wrap AlignmentId.mk (assign used.alignments target.alignments.size)
    lifetimes := wrap LifetimeId.mk (assign used.lifetimes target.lifetimes.size) } : RelocState)
  let functions := relocation source
  for index in ordered used.files do
    if let some file := source.files[index]? then
      modify fun state => { state with target := { state.target with
        files := state.target.files.push file } }
  for index in ordered used.locs do
    if let some location := source.locations[index]? then
      let location ← remap location functions
      modify fun state => { state with target := { state.target with
        locations := state.target.locations.push location } }
  for index in ordered used.origins do
    if let some origin := source.origins[index]? then
      let origin ← remap origin functions
      modify fun state => { state with target := { state.target with
        origins := state.target.origins.push origin } }
  for index in ordered used.alignments do
    if let some alignment := source.alignments[index]? then
      let alignment ← remap alignment functions
      modify fun state => { state with target := { state.target with
        alignments := state.target.alignments.push alignment } }
  for index in ordered used.lifetimes do
    if let some lifetime := source.lifetimes[index]? then
      let lifetime ← remap lifetime functions
      modify fun state => { state with target := { state.target with
        lifetimes := state.target.lifetimes.push lifetime } }
  return {
    tables := target
    body := ← remap placed.body functions
    witnesses := ← remap placed.witnesses functions
    initializationCertificates := ← remap placed.initializationCertificates functions
    borrowCertificates := ← remap placed.borrowCertificates functions
    borrowRejections := ← remap placed.borrowRejections functions }

/-- Detach one namespace of a validated unit, with the validation results
about it. -/
def extract (unit : ValidatedUnit) (identity : NamespaceId) :
    Except String RelocatableNamespace := do
  let some placed := placedIn unit identity
    | throw s!"a unit has no namespace {identity.index}"
  let some self := unit.tables.namespaces[identity.index]?
    | throw s!"a unit has no namespace reference {identity.index}"
  let (relocated, state) := (relocatePlaced placed ⟨0⟩).run
    { target := { namespaces := #[self] } }
  return {
    tables := state.target
    profiles := unit.profiles
    body := relocated.body
    witnesses := relocated.witnesses
    initializationCertificates := relocated.initializationCertificates
    borrowCertificates := relocated.borrowCertificates
    borrowRejections := relocated.borrowRejections }

/-! ## Linking -/

/-- The shape of declarations: types and names interned by content into a
scratch table, locations erased, lifetimes numbered by first occurrence
within one declaration. Two declarations have the same shape when they
declare the same thing, wherever and in whatever unit they were written. -/
private structure ShapeState where
  types : Array Ty := #[]
  names : Array QualifiedName := #[]
  namespaces : Array (Array String) := #[]
  typeMemo : Std.HashMap Nat TypeId := {}
  lifetimeMemo : Std.HashMap Nat LifetimeId := {}

private abbrev ShapeM := StateM ShapeState

private def shapePure : IdFns ShapeM := {
  file := pure, loc := pure, origin := pure, alignment := pure, profile := pure, type := pure,
  namespaceId := pure, name := pure, lifetime := pure }

instance : Inhabited (IdFns ShapeM) := ⟨shapePure⟩

mutual

private partial def shaping (source : Tables) : IdFns ShapeM := {
  file := fun _ => pure ⟨0⟩
  loc := fun _ => pure ⟨0⟩
  origin := fun _ => pure ⟨0⟩
  alignment := fun _ => pure ⟨0⟩
  profile := pure
  type := shapeType source
  namespaceId := shapeNamespace source
  name := shapeName source
  lifetime := fun id => do
    let state ← get
    if let some done := state.lifetimeMemo[id.index]? then return done
    let result : LifetimeId := ⟨state.lifetimeMemo.size⟩
    set { state with lifetimeMemo := state.lifetimeMemo.insert id.index result }
    return result }

private partial def shapeType (source : Tables) (id : TypeId) : ShapeM TypeId := do
  if let some done := (← get).typeMemo[id.index]? then return done
  let some ty := source.types[id.index]? | return id
  let ty ← remap ty (shaping source)
  let state ← get
  let (types, index) := intern state.types ty
  let result : TypeId := ⟨index⟩
  set { state with types, typeMemo := state.typeMemo.insert id.index result }
  return result

private partial def shapeNamespace (source : Tables) (id : NamespaceId) : ShapeM NamespaceId := do
  let path := (source.namespaces[id.index]?.map (·.segments)).getD #[]
  let state ← get
  let (namespaces, index) := intern state.namespaces path
  set { state with namespaces }
  return ⟨index⟩

private partial def shapeName (source : Tables) (id : NameId) : ShapeM NameId := do
  let some name := source.names[id.index]? | return id
  let name ← remap name (shaping source)
  let state ← get
  let (names, index) := intern state.names name
  set { state with names }
  return ⟨index⟩

end

/-- Whether two declarations of one table have the same shape. -/
private def sameShape [Remap α] [BEq α] (tables : Tables) (left right : α) : Bool :=
  let (left, state) := (remap left (shaping tables)).run {}
  -- The right side shares the interned types and names, and numbers its
  -- lifetimes afresh.
  let (right, _) := (remap right (shaping tables)).run
    { state with typeMemo := {}, lifetimeMemo := {} }
  left == right

private def nameOf (tables : Tables) (id : NameId) : String :=
  (tables.names[id.index]?.map (·.name)).getD ""

/-- The boundary: every declaration of the interface a unit was validated
against has the same shape in the namespace that replaces it. -/
private def checkBoundary (tables : Tables) (path : Array String)
    (interface actual : Namespace FunctionBody) : Except String Unit := do
  let located (kind name : String) :=
    s!"`{"::".intercalate path.toList}::{name}`: the linked {kind} differs from the one it \
      was checked against"
  for declared in interface.functions do
    let name := nameOf tables declared.name
    let some linked := actual.functions.find? (nameOf tables ·.name == name)
      | throw s!"`{"::".intercalate path.toList}` does not declare the function `{name}`"
    unless sameShape tables declared.signature linked.signature do
      throw (located "function" name)
  for declared in interface.specFunctions do
    let name := nameOf tables declared.name
    let some linked := actual.specFunctions.find? (nameOf tables ·.name == name)
      | throw s!"`{"::".intercalate path.toList}` does not declare the specification function \
          `{name}`"
    unless sameShape tables declared.signature linked.signature do
      throw (located "specification function" name)
  for declared in interface.structs do
    let name := nameOf tables declared.name
    let some linked := actual.structs.find? (nameOf tables ·.name == name)
      | throw s!"`{"::".intercalate path.toList}` does not declare the type `{name}`"
    unless sameShape tables (declared.generics, declared.fields, declared.variants)
          (linked.generics, linked.fields, linked.variants) &&
        declared.abilities == linked.abilities do
      throw (located "type" name)

/-- Link relocatable namespaces into a validated unit. A namespace at a path
the unit declares replaces that namespace, the bodiless interface the unit
was validated against, after the boundary check; any other is added. The
namespaces a linked one refers to must be in the unit or linked with it. -/
def link (base : ValidatedUnit) (objects : Array RelocatableNamespace) :
    Except String ValidatedUnit := do
  for object in objects do
    unless object.profiles == base.profiles do
      throw s!"`{"::".intercalate object.path.toList}` was validated under another profile"
  -- The unit's namespaces, in order: the base's, each replaced by the
  -- namespace linked at its path, then the ones the base lacks.
  let pathOf (id : NamespaceId) := (base.tables.namespaces[id.index]?.map (·.segments)).getD #[]
  let baseEntries := base.namespaces.map fun ns =>
    (ns.identity, objects.find? (·.path == pathOf ns.identity))
  let added := objects.filter fun object =>
    !base.namespaces.any (pathOf ·.identity == object.path)
  let refs : Array NamespaceRef :=
    baseEntries.map (fun (id, _) => base.tables.namespaces[id.index]!) ++
      added.map fun object => object.tables.namespaces[0]!
  -- Every namespace moves into fresh tables whose first entries are the
  -- unit's namespaces, so each sits at its index.
  let mut state : RelocState := { target := { namespaces := refs } }
  let mut placed : Array Placed := #[]
  for ((identity, object?), index) in baseEntries.zipIdx do
    let some own := placedIn base identity | throw "a base namespace is missing"
    match object? with
    | none =>
        let (relocated, next) := (relocatePlaced own ⟨index⟩).run state
        state := next
        placed := placed.push relocated
    | some object =>
        let (linked, next) := (relocatePlaced object.placed ⟨index⟩).run state
        state := next
        placed := placed.push linked
        -- The interface the unit was checked against is relocated beside the
        -- namespace for the boundary check only, not into the unit.
        let (interface, checking) := (relocatePlaced own ⟨index⟩).run state
        checkBoundary checking.target ((refs[index]?.map (·.segments)).getD #[]) interface.body
          linked.body
  for (object, offset) in added.zipIdx do
    let (linked, next) := (relocatePlaced object.placed ⟨baseEntries.size + offset⟩).run state
    state := next
    placed := placed.push linked
  -- The base's dependency interfaces move along.
  let dependencies ← do
    let functions := relocation base.tables
    let (dependencies, next) := (base.dependencies.mapM (remap · functions)).run
      { target := state.target, typeIndex := state.typeIndex, nameIndex := state.nameIndex }
    state := next
    pure dependencies
  let tables := state.target
  let namespaces := placed.map (·.body)
  let orders := valueOrdersOf tables namespaces
  let validated := namespaces.map fun ns =>
    ({ toNamespace := ns, tables, orders } : ValidatedNamespace)
  let rawDependencies : Array Import.RawNamespaceInterface := dependencies.map fun dep => {
    namespaceId := dep.namespaceId, profile := dep.profile, exportedNames := dep.exportedNames
    structs := dep.structs
    functions := dep.functions.map (·.withBody .absent)
    specFunctions := dep.specFunctions }
  -- Every name a linked namespace uses must resolve in the unit, as it did
  -- in the unit that validated it.
  let resolution := ResolutionIndex.build tables namespaces rawDependencies
  let unresolved := (resolveUseSites tables resolution namespaces.size namespaces).filter
    (·.severity == .error)
  unless unresolved.isEmpty do
    throw ("\n".intercalate (unresolved.map (·.message)).toList)
  return Internal.mkValidatedUnit tables base.profiles validated dependencies base.evidence
    { namespaceCount := validated.size, functionCounts := validated.map (·.functions.size) }
    (placed.flatMap (·.witnesses))
    resolution
    (placed.flatMap (·.initializationCertificates))
    (placed.flatMap (·.borrowCertificates))
    (placed.flatMap (·.borrowRejections))

/-- A unit of relocatable namespaces alone, linked into an empty unit. -/
def assemble (profiles : Array ProfileConfig) (objects : Array RelocatableNamespace) :
    Except String ValidatedUnit :=
  link (Internal.mkValidatedUnit {} profiles #[] #[] #[] { namespaceCount := 0, functionCounts := #[] }
    #[] (ResolutionIndex.build ({} : Tables) (#[] : Array (Namespace FunctionBody)) #[])) objects

end LeanerIR.Validation
