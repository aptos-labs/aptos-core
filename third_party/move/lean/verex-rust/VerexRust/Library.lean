-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR

/-!
# The standard-library specification library

VerexSL files under `verex-rust/lib/<crate>/*.spec.lean` give meaning to the
dependency functions and types of `<crate>`. A file holds scopes (`mod m where`,
`impl T where`, `trait impl «…» where`), contracts (`spec f where`, its clauses
deeper than it), specification functions (`spec fun f(…) : T := …`, definitions, kept
when a kept contract or a clause of the unit names them), and models (`@[model(vector T)] opaque struct Vec {T} {A} where`),
one to a line, nested by indentation. A model naming a binder (`@[model(T)] opaque struct
Box {T} {A} where`) says the type is that argument: a dereference of it is the place itself.

The import reads the models before validation: a type a library models is the
VIR type it names, in every signature and body, so that validation and verification
see that type alone. A library may instead state the fields of a type the unit sees
as opaque (`@[model] struct Iter {a : lifetime} {T} where`, its fields deeper than
it): the declaration then holds them, with no destructor (`applyModels`). The
verification splices the contracts of the
functions a unit declares into the namespace of their crate (`reachedItems`). The
two read a file alike, without VerexSL's parser, which the import does not load;
the contracts it keeps are elaborated with the rendering.
-/

namespace VerexIR.Import

deriving instance VerexIR.Remap for
  RawUnwindAction, RawCallDestination, RawAssertKind, RawStatement, RawTerminator,
  RawBasicBlock, RawCfg, RawBody, RawNamespaceInterface

end VerexIR.Import

namespace VerexIR.Rust.Library

open VerexIR VerexIR.Import

/-- The type of a field a library states, over its declaration's binders. -/
inductive FieldType where
  | closed (type : Ty)
  | parameter (position : Nat)
  | vector (element : FieldType)
  | slice (element : FieldType)
  /-- A shared reference at the lifetime binder at this position. -/
  | shared (lifetime : Nat) (referent : FieldType)
  /-- A mutable reference at that binder: the lender a model holds, whose
  prophecy a contract reads with `final` (`IterMut`'s slice). -/
  | mutable (lifetime : Nat) (referent : FieldType)
  deriving Repr, BEq, Inhabited

/-- What a library declares a dependency's type to be. -/
inductive Model where
  /-- The vector of the type parameter at this position. -/
  | vector (element : Nat)
  | string
  /-- The type parameter at this position itself. -/
  | identity (argument : Nat)
  /-- The nominal type itself, holding these fields. -/
  | fields (binders : Array BinderKind) (fields : Array (String × FieldType))
  /-- A function, a call of which is the drop of its one argument. -/
  | dropsArgument
  deriving Repr, BEq, Inhabited

/-- A library's declaration of the VIR type of a dependency's type. -/
structure ModelDecl where
  crate : String
  scope : Array String
  name : String
  /-- The number of its generic parameters. -/
  generics : Nat
  model : Model
  deriving Repr, BEq, Inhabited

/-- A contract of a library file, by the function it specifies. -/
structure ContractItem where
  scope : Array String
  target : String
  /-- The types its shape names by a path, each with its scope. -/
  shapeTypes : Array (Array String × String) := #[]
  /-- The scalars its shape names. -/
  shapeScalars : Array String := #[]
  /-- Whether it is a contract at a shape. -/
  atShape : Bool := false
  /-- Its lines, the first the `spec` line, counted from zero. -/
  first : Nat
  last : Nat
  deriving Repr, Inhabited

/-- A specification function of a library file: a definition, so nothing trusts it. -/
structure SpecFunctionItem where
  scope : Array String
  name : String
  /-- Its lines, the first the `spec fun` line, counted from zero. -/
  first : Nat
  last : Nat
  deriving Repr, Inhabited

/-- A scope of a library file, with the lines it spans. -/
structure ScopeItem where
  line : Nat
  /-- The line after its last item. -/
  stop : Nat
  deriving Repr, Inhabited

/-- A library file, read. -/
structure File where
  path : System.FilePath
  crate : String
  lines : Array String
  models : Array ModelDecl := #[]
  contracts : Array ContractItem := #[]
  specFunctions : Array SpecFunctionItem := #[]
  scopes : Array ScopeItem := #[]
  deriving Inhabited

/-- A line's words, a `«…»` name one word without its guillemets. -/
private def words (line : String) : List String := Id.run do
  let mut found : Array String := #[]
  let mut current := ""
  let mut quoted := false
  for c in line.toList do
    if c == '«' then quoted := true
    else if c == '»' then quoted := false
    else if !quoted && c.isWhitespace then
      unless current.isEmpty do
        found := found.push current
        current := ""
    else current := current.push c
  unless current.isEmpty do found := found.push current
  return found.toList

private def indentOf (line : String) : Nat :=
  (line.toList.takeWhile (· == ' ')).length

/-- A line that holds no item. -/
private def isBlank (line : String) : Bool :=
  let text := line.trimAscii.toString
  text.isEmpty || text.startsWith "--"

/-- A declaration's binders, each a type's `{T}` or a lifetime's `{a : lifetime}`. -/
private def bindersOf : List String → Option (List (String × BinderKind))
  | [] => some []
  | name :: ":" :: "lifetime}" :: rest => do
      guard (name.startsWith "{")
      return ((name.drop 1).toString, .lifetime) :: (← bindersOf rest)
  | binder :: rest => do
      guard (binder.startsWith "{" && binder.endsWith "}")
      return (((binder.drop 1).dropEnd 1).toString, .typeArg) :: (← bindersOf rest)

/-- The model a declaration's attribute names, over its binders. -/
private def modelOf (named : List String) (binders : List String) : Option Model :=
  match named with
  | ["string"] => some .string
  | ["vector", element] => (binders.findIdx? (· == element)).map .vector
  | [argument] => (binders.findIdx? (· == argument)).map .identity
  | _ => none

/-- `@[model(vector T)] opaque struct Vec {T} {A} where`. -/
private def modelLine? (crate : String) (scope : Array String) (line : List String) :
    Option ModelDecl := do
  let text := " ".intercalate line
  guard (text.startsWith "@[model(")
  let [named, declaration] := text.splitOn ")] " | none
  let named := words (named.drop "@[model(".length).toString
  let "opaque" :: "struct" :: name :: rest := words declaration | none
  guard (rest.getLast? == some "where")
  let binders := (← bindersOf rest.dropLast).map (·.1)
  return { crate, scope, name, generics := binders.length, model := ← modelOf named binders }

/-- `@[model(drop)] fun drop {T}`: a call of it is the drop of its argument. -/
private def functionModelLine? (crate : String) (scope : Array String) :
    List String → Option ModelDecl
  | "@[model(drop)]" :: "fun" :: name :: rest => do
      let binders ← bindersOf rest
      return { crate, scope, name, generics := binders.length, model := .dropsArgument }
  | _ => none

/-- `@[model] struct Iter {a : lifetime} {T} where`: a declaration with no fields yet,
and the binders its fields' types read. -/
private def fieldsModelLine? (crate : String) (scope : Array String) :
    List String → Option (ModelDecl × List (String × BinderKind))
  | "@[model]" :: "struct" :: name :: rest => do
      guard (rest.getLast? == some "where")
      let binders ← bindersOf rest.dropLast
      return ({ crate, scope, name, generics := binders.length,
                model := .fields (binders.map (·.2)).toArray #[] }, binders)
  | _ => none

/-- A scalar type of a field, as VerexSL spells it. -/
private def scalarOf? (text : String) : Option Ty :=
  let widths := [8, 16, 32, 64, 128]
  match text with
  | "Bool" => some .bool
  | "usize" => some (.integer .pointer false)
  | "isize" => some (.integer .pointer true)
  | _ =>
      widths.findSome? fun width =>
        if text == s!"u{width}" then some (.integer (.bits width) false)
        else if text == s!"i{width}" then some (.integer (.bits width) true)
        else none

/-- The type of a field, as VerexSL spells it: a scalar, a binder, `Vector<T>`,
`Slice<T>`, `&[a] T`, `&[a] mut T`. -/
private partial def fieldTypeOf? (binders : List (String × BinderKind)) (text : String) :
    Option FieldType := do
  let text := text.trimAscii.toString
  if text.startsWith "&[" then
    let [lifetime, referent] := (text.drop 2).toString.splitOn "] " | none
    let position ← binders.findIdx? (· == (lifetime, BinderKind.lifetime))
    if referent.startsWith "mut " then
      return .mutable position (← fieldTypeOf? binders (referent.drop 4).toString)
    return .shared position (← fieldTypeOf? binders referent)
  if text.startsWith "Slice<" && text.endsWith ">" then
    return .slice (← fieldTypeOf? binders (((text.drop 6).dropEnd 1).toString))
  if text.startsWith "Vector<" && text.endsWith ">" then
    return .vector (← fieldTypeOf? binders (((text.drop 7).dropEnd 1).toString))
  match binders.findIdx? (· == (text, BinderKind.typeArg)) with
  | some position => return .parameter position
  | none => return .closed (← scalarOf? text)

/-- A field line of a model: `slice : &[a] Slice<T>`. -/
private def fieldLine? (binders : List (String × BinderKind)) :
    List String → Option (String × FieldType)
  | name :: ":" :: type => return (name, ← fieldTypeOf? binders (" ".intercalate type))
  | _ => none

private def scopeName? : List String → Option String
  | ["mod", name, "where"] | ["impl", name, "where"] | ["trait", name, "where"]
  | ["trait", "impl", name, "where"] => some name
  | _ => none

/-- `spec f where` and `spec f at {…} where`. -/
private def contractTarget? : List String → Option String
  | "spec" :: name :: rest =>
      if rest.getLast? == some "where" && name != "fun" && name != "module" && name != "lemma"
      then some name else none
  | _ => none

/-- `spec fun f(x : Int) : Int := …` and `spec fun f {T}(…)`: its name. -/
private def specFunctionName? : List String → Option String
  | "spec" :: "fun" :: name :: _ => some ((name.splitOn "(").headD name)
  | _ => none

/-- The types a `spec` line's shape names by a path (`ops::range::Range<u8>`), and its
scalars: a shape matches only a call of the function naming them. -/
private def shapeNamed (line : String) : Array (Array String × String) × Array String := Id.run do
  let [_, shape] := line.splitOn " at " | return (#[], #[])
  let delimiter (c : Char) := "{}<>(),&; ".contains c
  let tokens := (shape.toList.splitBy (fun a b => delimiter a == delimiter b)).filterMap fun part =>
    if part.any delimiter then none else some (String.ofList part)
  let mut types := #[]
  let mut scalars := #[]
  for token in tokens do
    let segments : List String := (token.splitOn "::").map fun segment =>
      (segment.replace "«" "").replace "»" ""
    if segments.length > 1 then
      types := types.push (segments.dropLast.toArray, segments.getLast!)
    else if (scalarOf? token).isSome then
      scalars := scalars.push token
  return (types, scalars)

/-- Read a library file by its layout. -/
def File.read (path : System.FilePath) (crate text : String) : Except String File := do
  let lines := (text.splitOn "\n").toArray
  let mut file : File := { path, crate, lines }
  -- The open scopes, each with its indentation, name and index in `file.scopes`.
  let mut stack : Array (Nat × String × Nat) := #[]
  let mut index := 0
  while index < lines.size do
    let line := lines[index]!
    if isBlank line then
      index := index + 1
      continue
    let indent := indentOf line
    while stack.back?.any (·.1 >= indent) do
      let (_, _, position) := stack.back!
      file := { file with scopes := file.scopes.modify position ({ · with stop := index }) }
      stack := stack.pop
    let scope := stack.map (·.2.1)
    let parts := words line
    if let some name := scopeName? parts then
      file := { file with scopes := file.scopes.push { line := index, stop := lines.size } }
      stack := stack.push (indent, name, file.scopes.size - 1)
      index := index + 1
    else if let some name := specFunctionName? parts then
      let first := index
      index := index + 1
      let mut last := first
      while index < lines.size &&
          (isBlank lines[index]! || indentOf lines[index]! > indent) do
        unless isBlank lines[index]! do last := index
        index := index + 1
      file := { file with specFunctions := file.specFunctions.push { scope, name, first, last } }
      index := last + 1
    else if let some target := contractTarget? parts then
      let first := index
      index := index + 1
      let mut last := first
      while index < lines.size &&
          (isBlank lines[index]! || indentOf lines[index]! > indent) do
        unless isBlank lines[index]! do last := index
        index := index + 1
      let (shapeTypes, shapeScalars) := shapeNamed lines[first]!
      let atShape := parts.contains "at"
      let contract := { scope, target, shapeTypes, shapeScalars, atShape, first, last }
      file := { file with contracts := file.contracts.push contract }
      index := last + 1
    else if let some model := (modelLine? crate scope parts).orElse
        fun _ => functionModelLine? crate scope parts then
      file := { file with models := file.models.push model }
      index := index + 1
    else if let some (model, binders) := fieldsModelLine? crate scope parts then
      index := index + 1
      let mut fields := #[]
      while index < lines.size &&
          (isBlank lines[index]! || indentOf lines[index]! > indent) do
        unless isBlank lines[index]! do
          let some field := fieldLine? binders (words lines[index]!)
            | throw s!"{path}:{index + 1}: a model's field is `name : type`, its type a scalar, \
                a binder of the declaration, `Vector<T>`, `Slice<T>`, `&[a] T` or \
                `&[a] mut T`: \
                `{lines[index]!.trimAscii.toString}`"
          fields := fields.push field
        index := index + 1
      let .fields kinds _ := model.model
        | throw s!"{path}: a model with fields lost its binders"
      let held := { model with model := .fields kinds fields }
      file := { file with models := file.models.push held }
    else
      throw s!"{path}:{index + 1}: a library file holds scopes, contracts, specification \
        functions and models, one to a line; this is none: `{line.trimAscii.toString}`"
  return file

private def isIdentifierChar (c : Char) : Bool :=
  c.isAlphanum || c == '_' || c == '\''

/-- Whether a text calls a specification function: at a path ending in its scope's
name, or, where `bare`, as the name alone. -/
def SpecFunctionItem.namedIn (item : SpecFunctionItem) (bare : Bool) (text : String) : Bool :=
  let pieces := text.splitOn item.name
  let qualifier := item.scope.back?.map (· ++ "::")
  Id.run do
    let mut before := ""
    for (piece, position) in pieces.zipIdx do
      if position > 0 then
        let called := piece.startsWith "(" || piece.startsWith "::<"
        let leadingChar := before.back
        let separate := before.isEmpty || !(isIdentifierChar leadingChar || leadingChar == ':')
        let qualified := qualifier.any (before.endsWith ·)
        if called && ((bare && separate) || qualified) then return true
      before := before ++ (if position > 0 then item.name else "") ++ piece
    return false

/-- The lines of an item, as one text. -/
private def File.linesOf (file : File) (first last : Nat) : String :=
  "\n".intercalate (file.lines.extract first (last + 1)).toList

/-- The specification functions kept: those a kept contract or `clauses`, the unit's
specification files, name, and those a kept function names. -/
def File.keptSpecFunctions (file : File) (keep : ContractItem → Bool) (clauses : Array String) :
    Array SpecFunctionItem := Id.run do
  let contracts := file.contracts.filter keep
  let named (item : SpecFunctionItem) (scope : Array String) (text : String) :=
    item.namedIn (scope == item.scope) text
  let mut kept := file.specFunctions.filter fun item =>
    clauses.any (item.namedIn false) || contracts.any fun contract =>
      named item contract.scope (file.linesOf contract.first contract.last)
  let mut grew := true
  while grew do
    let more := file.specFunctions.filter fun item =>
      !kept.any (fun other => other.first == item.first) && kept.any fun other =>
        named item other.scope (file.linesOf other.first other.last)
    grew := !more.isEmpty
    kept := kept ++ more
  return kept

/-- The items of a file that `keep` and `clauses` choose, its other lines blanked so that
a message keeps its line, or none when they choose no item. -/
def File.reachedItems (file : File) (keep : ContractItem → Bool)
    (clauses : Array String := #[]) : Option String :=
  let spans := (file.contracts.filter keep).map (fun item => (item.first, item.last)) ++
    (file.keptSpecFunctions keep clauses).map (fun item => (item.first, item.last))
  if spans.isEmpty then none else
  let keptLine (line : Nat) := spans.any fun (first, last) => first <= line && line <= last
  let keptScope (line : Nat) := file.scopes.any fun scope =>
    scope.line == line && spans.any fun (first, _) => scope.line < first && first < scope.stop
  some <| "\n".intercalate <| (file.lines.mapIdx fun line text =>
    if keptLine line || keptScope line then text else "").toList

/-- The library files under `root`, by crate, in a stable order. -/
def readFiles (root : System.FilePath) : IO (Array File) := do
  unless ← root.pathExists do
    throw <| IO.userError s!"the Rust specification library {root} does not exist"
  let mut files := #[]
  let crates := (← root.readDir).qsort fun a b => a.fileName < b.fileName
  for crate in crates do
    unless ← crate.path.isDir do continue
    let entries := (← crate.path.readDir).qsort fun a b => a.fileName < b.fileName
    for entry in entries do
      unless entry.fileName.endsWith ".spec.lean" do continue
      match File.read entry.path crate.fileName (← IO.FS.readFile entry.path) with
      | .ok file => files := files.push file
      | .error message => throw <| IO.userError message
  return files

/-! ## Models -/

private def mappingFns {m : Type → Type} [Monad m] (type : TypeId → m TypeId)
    (name : NameId → m NameId) (namespaceId : NamespaceId → m NamespaceId) : IdFns m where
  file := pure
  loc := pure
  origin := pure
  alignment := pure
  profile := pure
  type := type
  namespaceId := namespaceId
  name := name
  lifetime := pure

/-- The first entry of each type, up to the entries it names, and the entry
every identifier maps to. -/
private partial def internTypes (types : Array Ty) (mapping : Array TypeId) :
    Array Ty × Array TypeId := Id.run do
  let renamed : Array Ty := types.map fun type =>
    Id.run (remap type (mappingFns (fun id => pure (mapping[id.index]?.getD id)) pure pure))
  let mut firsts : Array Nat := #[]
  let mut next := mapping
  for index in [:renamed.size] do
    -- An entry already mapped away stays mapped.
    if (mapping[index]?.map (·.index)) != some index then continue
    match firsts.find? (fun first => renamed[first]! == renamed[index]!) with
    | some first => next := next.set! index ⟨first⟩
    | none => firsts := firsts.push index
  if next == mapping then return (renamed, mapping)
  return internTypes types (next.map fun id => next[id.index]?.getD id)

/-- The table entries a value names, through the entries they name. -/
private structure Named where
  types : Array Bool
  names : Array Bool
  namespaces : Array Bool

private instance : Nonempty (IdFns (StateM Named)) := ⟨mappingFns pure pure pure⟩

private partial def namedFns (tables : Tables) : IdFns (StateM Named) :=
  mappingFns
    (fun id => do
      if (← get).types[id.index]?.getD true then return id
      modify fun named => { named with types := named.types.set! id.index true }
      if let some type := tables.types[id.index]? then
        discard <| remap type (namedFns tables)
      return id)
    (fun id => do
      if (← get).names[id.index]?.getD true then return id
      modify fun named => { named with names := named.names.set! id.index true }
      if let some name := tables.names[id.index]? then
        discard <| remap name (namedFns tables)
      return id)
    (fun id => do
      modify fun named =>
        if id.index < named.namespaces.size then
          { named with namespaces := named.namespaces.set! id.index true }
        else named
      return id)

/-- What the namespaces and interfaces of a unit name in its tables. -/
private def namedBy (tables : Tables) (namespaces : Array RawNamespace)
    (dependencies : Array RawNamespaceInterface) : Named :=
  let initial : Named := {
    types := Array.replicate tables.types.size false
    names := Array.replicate tables.names.size false
    namespaces := Array.replicate tables.namespaces.size false }
  let visit : StateM Named Unit := do
    for ns in namespaces do discard <| remap ns (namedFns tables)
    for dependency in dependencies do discard <| remap dependency (namedFns tables)
  (visit.run initial).2

/-- Also the types naming no declaration, such as `Unit`, which validation may
look up whether or not anything names them. -/
private def keepNameless (tables : Tables) (named : Named) : Named := Id.run do
  let mut types := named.types
  let nothing : Named := {
    types := Array.replicate tables.types.size false
    names := Array.replicate tables.names.size false
    namespaces := #[] }
  for type in tables.types, index in [:tables.types.size] do
    if types[index]! then continue
    let reached := ((remap type (namedFns tables)).run nothing).2
    if !reached.names.any id then
      types := (types.zip reached.types).map fun (left, right) => left || right
      types := types.set! index true
  return { named with types }

/-- The positions of the marked entries, as a map from an old position. -/
private def positions (marks : Array Bool) : Array Nat := Id.run do
  let mut found := Array.mkEmpty marks.size
  let mut next := 0
  for mark in marks do
    found := found.push next
    if mark then next := next + 1
  return found

private def keepMarked (marks : Array Bool) (values : Array α) : Array α :=
  (values.zip marks).filterMap fun (value, mark) => if mark then some value else none

/-- Remove what no declaration of a rewritten unit names any more: a dependency's
opaque type reached only as an argument of a modeled type, and a dependency's
namespace left without declarations. -/
private def prune (unit : RawUnit) : RawUnit := Id.run do
  let tables := unit.tables
  let local? := unit.namespaces[0]?.map (·.identity)
  let isOpaque (ns : RawNamespace) (declaration : StructDecl) :=
    some ns.identity != local? && declaration.properties.any (·.tag == rustOpaqueTag)
  -- An opaque type declares nothing else, so it is kept when anything else names it.
  let others := unit.namespaces.map fun ns =>
    { ns with structs := ns.structs.filter (!isOpaque ns ·) }
  let named := namedBy tables others unit.dependencies
  let kept := unit.namespaces.map fun ns =>
    { ns with structs := ns.structs.filter fun declaration =>
        !isOpaque ns declaration || named.names[declaration.name.index]?.getD true }
  let empty (ns : RawNamespace) :=
    some ns.identity != local? && ns.structs.isEmpty && ns.functions.isEmpty &&
      ns.constants.isEmpty && ns.specFunctions.isEmpty
  let dropped := (kept.filter empty).map (·.identity)
  let kept := (kept.filter (!empty ·)).map fun ns =>
    { ns with imports := ns.imports.filter (!dropped.contains ·) }
  let dependencies := unit.dependencies.filter (!dropped.contains ·.namespaceId)
  let named := keepNameless tables (namedBy tables kept dependencies)
  let typePositions := positions named.types
  let namePositions := positions named.names
  let namespacePositions := positions named.namespaces
  let fns : IdFns Id := mappingFns
    (fun id => pure ⟨typePositions[id.index]?.getD id.index⟩)
    (fun id => pure ⟨namePositions[id.index]?.getD id.index⟩)
    (fun id => pure ⟨namespacePositions[id.index]?.getD id.index⟩)
  return { unit with
    tables := { tables with
      types := (keepMarked named.types tables.types).map fun type => Id.run (remap type fns)
      names := (keepMarked named.names tables.names).map fun name => Id.run (remap name fns)
      namespaces := keepMarked named.namespaces tables.namespaces }
    namespaces := kept.map fun ns => Id.run (remap ns fns)
    dependencies := dependencies.map fun dependency => Id.run (remap dependency fns) }

/-- The expressions a raw function body starts from. -/
private def bodyRoots : RawBody → Array ExprId
  | .absent => #[]
  | .structured root => #[root]
  | .cfg graph => graph.blocks.flatMap fun block =>
      let statements := block.statements.filterMap fun
        | .execute expression => some expression
        | _ => none
      statements ++ match block.terminator with
        | .branch condition _ _ | .switch condition _ _ | .call condition _ _
        | .assert condition _ _ _ _ => #[condition]
        | .return_ values | .throw_ _ values => values
        | .goto _ | .drop _ _ _ | .unreachable | .resume | .abort => #[]

/-- The blocks of a graph its entry reaches. -/
private def reachableBlocks (graph : RawCfg) : Array RawBasicBlock := Id.run do
  let mut seen : Std.HashSet Nat := {}
  let mut pending := #[graph.entry]
  let mut found := #[]
  while let some block := pending.back? do
    pending := pending.pop
    if seen.contains block.index then continue
    seen := seen.insert block.index
    let some next := graph.blocks[block.index]? | continue
    found := found.push next
    pending := pending ++ next.terminator.successors
  return found

/-- What a node the unit cannot state says of itself, and where it is: an opaque
constant, or a call of an opaque function. -/
private partial def opaqueNode? (tables : Tables) (ns : RawNamespace)
    (opaqueNames : Std.HashSet Nat) (root : ExprId) (seen : Std.HashSet Nat) :
    Std.HashSet Nat × Option (String × LocId) :=
  if seen.contains root.index then (seen, none) else
  let seen := seen.insert root.index
  match ns.expressions[root.index]? with
  | none => (seen, none)
  | some expression =>
      match expression.kind with
      | .value (.opaque description) _ => (seen, some (description, expression.loc))
      | .operation (.call (.function callee)) _ _ _ =>
          if opaqueNames.contains callee.name.index then
            let name := (tables.names[callee.name.index]?.map (·.name)).getD "?"
            (seen, some (s!"it calls `{name}`, which is unsafe or has a signature the unit \
              cannot state", expression.loc))
          else children seen expression
      | _ => children seen expression
where
  children (seen : Std.HashSet Nat) (expression : Expr) :
      Std.HashSet Nat × Option (String × LocId) :=
    (Validation.expressionChildren expression.kind).foldl (init := (seen, none))
      fun (seen, found) child =>
        if found.isSome then (seen, found) else opaqueNode? tables ns opaqueNames child seen

/-- The opaque node a body reaches first, of what survives the collapse of its
diverging regions. -/
private def survivingNode? (tables : Tables) (ns : RawNamespace) (opaqueNames : Std.HashSet Nat)
    (graph : RawCfg) : Option (String × LocId) :=
  let roots := (reachableBlocks graph).flatMap fun block => bodyRoots (.cfg { graph with
    blocks := #[block] })
  (roots.foldl (init := (({} : Std.HashSet Nat), none)) fun (seen, found) root =>
    if found.isSome then (seen, found) else opaqueNode? tables ns opaqueNames root seen).2

/-- A function whose body holds a node the unit cannot state, after the collapse of
diverging regions, is refused: declared by its signature as a dependency's function,
stating why, with the node's range related to its location. An opaque function of the
crate, whose signature the unit cannot state, is refused the same way, so it is reported
and not pruned; its callers are refused here. Every other function of the crate stays
what it is. -/
def refuseFunctions (unit : RawUnit) : RawUnit := Id.run do
  let opaqueNames := unit.namespaces.foldl (init := ({} : Std.HashSet Nat)) fun names ns =>
    ns.functions.foldl (init := names) fun names function =>
      if function.profileData.any (·.tag == rustOpaqueFunctionTag) then
        names.insert function.name.index else names
  let mut locations := unit.tables.locations
  let mut namespaces := #[]
  for ns in unit.namespaces do
    let mut functions := #[]
    for function in ns.functions do
      let .cfg graph := function.body | do functions := functions.push function; continue
      let some (reason, loc) := survivingNode? unit.tables ns opaqueNames graph
        | do functions := functions.push function; continue
      let node := locations[loc.index]?
      let range := node.bind fun location => location.primary <|> location.expansion[0]?
      let declared := (locations[function.loc.index]?).getD {}
      let related := match range with
        | some range => declared.related.push range
        | none => declared.related
      let refusedLoc : LocId := ⟨locations.size⟩
      locations := locations.push { declared with related }
      functions := functions.push { function with
        body := .absent
        loc := refusedLoc
        locals := function.locals.filter (·.id.index < function.signature.parameters.size)
        profileData := function.profileData.filter (·.tag != rustOpaqueFunctionTag) ++ #[
          { profile := .rust, tag := rustDependencyTag },
          { profile := .rust, tag := rustRefusedTag, payload := reason }] }
    namespaces := namespaces.push { ns with functions }
  return { unit with namespaces, tables := { unit.tables with locations } }

/-- The functions a namespace's bodies call, by name. -/
private partial def calledFunctions (ns : RawNamespace) : Std.HashSet Nat :=
  let (_, called) := (ns.functions.flatMap (bodyRoots ·.body)).foldl
    (fun state root => visit root state) (({} : Std.HashSet Nat), ({} : Std.HashSet Nat))
  called
where
  visit (id : ExprId) : Std.HashSet Nat × Std.HashSet Nat → Std.HashSet Nat × Std.HashSet Nat
    | (seen, called) =>
      if seen.contains id.index then (seen, called) else
      match ns.expressions[id.index]? with
      | none => (seen, called)
      | some expression =>
          let called := match expression.kind with
            | .operation (.call (.function callee)) _ _ _ => called.insert callee.name.index
            | _ => called
          (Validation.expressionChildren expression.kind).foldl (fun state child => visit child state)
            (seen.insert id.index, called)

/-- Drop the opaque functions no body calls any more, which a collapsed region
leaves: a false interface a user could specify and a crate could link against.
What named them only in a collapsed region is out of every body. -/
def pruneOpaqueFunctions (unit : RawUnit) : RawUnit :=
  let called := unit.namespaces.foldl (init := ({} : Std.HashSet Nat)) fun called ns =>
    called.insertMany (calledFunctions ns)
  let isOpaque (function : FunctionDecl RawBody) :=
    function.profileData.any (·.tag == rustOpaqueFunctionTag)
  let pruned := unit.namespaces.flatMap fun ns =>
    (ns.functions.filter fun function =>
      isOpaque function && !called.contains function.name.index).map (·.name)
  if pruned.isEmpty then unit else
  prune { unit with namespaces := unit.namespaces.map fun ns =>
    { ns with
      functions := ns.functions.filter (!pruned.contains ·.name)
      expressions := ns.expressions.map fun expression =>
        match expression.kind with
        | .operation (.call (.function callee)) _ _ _ =>
            if pruned.contains callee.name then { expression with kind := .value .unit }
            else expression
        | _ => expression } }

/-- A field's type in the tables, its entries added where they are missing. -/
private partial def internFieldType (loc : LocId) (type : FieldType) : StateM Tables TypeId := do
  let entry ← match type with
    | .closed scalar => pure scalar
    | .parameter position => pure (.typeParameter position)
    | .vector element => pure (.vector (← internFieldType loc element) none)
    | .slice element => pure (.slice (← internFieldType loc element))
    | .shared position referent | .mutable position referent => do
        let kind := if type matches .mutable .. then ReferenceKind.mutable else .shared
        let referent ← internFieldType loc referent
        let tables ← get
        let lifetime : LifetimeId := ⟨tables.lifetimes.size⟩
        set { tables with
          lifetimes := tables.lifetimes.push { kind := .parameter position, loc } }
        pure (.reference { profile := .rust, kind, referent, lifetime })
  let tables ← get
  match tables.types.findIdx? (· == entry) with
  | some index => return ⟨index⟩
  | none =>
      set { tables with types := tables.types.push entry }
      return ⟨tables.types.size⟩

/-- A declaration holding the fields a library states of it. It stops being opaque,
and stating no destructor is the library's word that `core` declares none: what it
holds of its arguments is read from the fields. -/
private def holdingFields (name : String) (kinds : Array BinderKind)
    (fields : Array (String × FieldType)) (declaration : StructDecl) :
    ExceptT String (StateM Tables) StructDecl := do
  unless declaration.generics.map (·.kind) == kinds do
    throw s!"the library models `{name}` with other binders than the unit's"
  let tables ← get
  let some qualified := tables.names[declaration.name.index]?
    | throw s!"the name of `{name}` is out of range"
  let held ← fields.mapM fun (field, type) => do
    let typeId ← internFieldType declaration.loc type
    let tables ← get
    let fieldName : NameId := ⟨tables.names.size⟩
    set { tables with names := tables.names.push { qualified with name := field } }
    return ({ loc := declaration.loc, name := fieldName,
              type := { typeId, loc := declaration.loc } } : FieldDecl)
  return { declaration with
    fields := held
    properties := declaration.properties.filter fun property =>
      property.tag != rustOpaqueTag && property.tag != Validation.destructorTag }

/-- A type table entry under the type arguments of the nominals it was reached through. -/
private inductive PlaceType where
  | mk (type : TypeId) (arguments : Array PlaceType)
  deriving Inhabited

/-- A type parameter's entry read at the arguments it is under, as far as they say. -/
private partial def PlaceType.resolved (tables : Tables) : PlaceType → PlaceType
  | .mk type arguments =>
      match tables.types[type.index]? with
      | some (.typeParameter position) =>
          match arguments[position]? with
          | some argument => argument.resolved tables
          | none => .mk type arguments
      | _ => .mk type arguments

/-- The arguments of a nominal, each under the arguments its entry was reached under. A
lifetime's or a constant's position holds a placeholder no type reads. -/
private def PlaceType.nominalArguments (outer : Array PlaceType)
    (arguments : Array GenericArgument) : Array PlaceType :=
  arguments.map fun
    | .typeArg use => .mk use.typeId outer
    | _ => .mk ⟨0⟩ #[]

/-- What the type of a place is read from, at a raw unit. -/
private structure PlaceTyping where
  tables : Tables
  places : Array Place
  expressions : Array Expr
  declarations : Array StructDecl
  modelOfName : NameId → Option ModelDecl

/-- The content a dereference reaches in a value of this type: a reference's referent or
an identity model's argument. -/
private def PlaceTyping.dereferenced (typing : PlaceTyping) (type : PlaceType) : Option PlaceType :=
  let .mk id arguments := type.resolved typing.tables
  match typing.tables.types[id.index]? with
  | some (.reference reference) => some (.mk reference.referent arguments)
  | some (.nominal name held) =>
      match typing.modelOfName name with
      | some { model := .identity position, .. } =>
          (PlaceType.nominalArguments arguments held)[position]?
      | _ => none
  | _ => none

/-- Whether a type is an identity model's. -/
private def PlaceTyping.isIdentity (typing : PlaceTyping) (type : PlaceType) : Bool :=
  let .mk id _ := type.resolved typing.tables
  match typing.tables.types[id.index]? with
  | some (.nominal name _) =>
      match typing.modelOfName name with
      | some { model := .identity _, .. } => true
      | _ => false
  | _ => false

/-- An element of a value of this type: a vector's or a vector model's, or a tuple's at the
literal position `index` holds. -/
private def PlaceTyping.element (typing : PlaceTyping) (type : PlaceType) (index : ExprId) :
    Option PlaceType :=
  let .mk id arguments := type.resolved typing.tables
  match typing.tables.types[id.index]? with
  | some (.vector element _) | some (.slice element) => some (.mk element arguments)
  | some (.tuple elements) =>
      match typing.expressions[index.index]? with
      | some { kind := .value (.integer position) _, .. } =>
          (elements[position.toNat]?).map (.mk · arguments)
      | _ => none
  | some (.nominal name held) =>
      match typing.modelOfName name with
      | some { model := .vector position, .. } =>
          (PlaceType.nominalArguments arguments held)[position]?
      | _ => none
  | _ => none

/-- The declared type of a field of a value of this type. -/
private def PlaceTyping.field (typing : PlaceTyping) (type : PlaceType) (owner : QualifiedRef)
    (field : NameId) : Option PlaceType := do
  let .mk id arguments := type.resolved typing.tables
  let some (.nominal _ held) := typing.tables.types[id.index]? | none
  let declaration ← typing.declarations.find? (·.name == owner.name)
  let fields := declaration.variants.foldl (· ++ ·.fields) declaration.fields
  let found ← fields.find? (·.name == field)
  return .mk found.type.typeId (PlaceType.nominalArguments arguments held)

/-- The type of a place of a function, none where it cannot be read. -/
private partial def PlaceTyping.typeOf (typing : PlaceTyping) (locals : Array LocalDecl)
    (place : PlaceId) : Option PlaceType :=
  match typing.places[place.index]? with
  | some (.localVar localId) =>
      (locals.find? (·.id == localId)).map fun declared => .mk declared.type.typeId #[]
  | some (.deref base) => (typing.typeOf locals base).bind typing.dereferenced
  | some (.field base owner field) =>
      (typing.typeOf locals base).bind (typing.field · owner field)
  | some (.index base index) => (typing.typeOf locals base).bind (typing.element · index)
  | some (.subslice base ..) | some (.unsize base) => do
      let type ← typing.typeOf locals base
      let .mk id arguments := type.resolved typing.tables
      let element ← match typing.tables.types[id.index]? with
        | some (.vector element _) | some (.slice element) => some element
        | _ => none
      let index ← typing.tables.types.findIdx? (· == .slice element)
      some (.mk ⟨index⟩ arguments)
  | some (.downcast base _) => typing.typeOf locals base
  | some (.ghostField ..) | none => none

/-- The dereferences of identity models on a place's path, and the index expressions on it. -/
private partial def PlaceTyping.identityDerefs (typing : PlaceTyping) (locals : Array LocalDecl)
    (place : PlaceId) : Array PlaceId × Array ExprId :=
  match typing.places[place.index]? with
  | some (.deref base) =>
      let (derefs, indexes) := typing.identityDerefs locals base
      let identity := (typing.typeOf locals base).any typing.isIdentity
      (if identity then derefs.push place else derefs, indexes)
  | some (.index base index) =>
      let (derefs, indexes) := typing.identityDerefs locals base
      (derefs, indexes.push index)
  | some (.field base ..) | some (.ghostField base ..) | some (.subslice base ..)
  | some (.unsize base) | some (.downcast base _) => typing.identityDerefs locals base
  | some (.localVar _) | none => (#[], #[])

/-- The places a raw function body names directly, and the expressions it starts from. -/
private def bodyOccurrences : RawBody → Array PlaceId × Array ExprId
  | .absent => (#[], #[])
  | .structured root => (#[], #[root])
  | body@(.cfg graph) =>
      let places := graph.blocks.flatMap fun block =>
        let statements := block.statements.filterMap fun
          | .deinit place | .setDiscriminant place _ | .retag place | .placeMention place
          | .ascribeUserType place _ => some place
          | _ => none
        match block.terminator with
        | .drop place _ _ => statements.push place
        | .call _ (some destination) _ => statements.push destination.place
        | _ => statements
      (places, bodyRoots body)

/-- The place an expression names directly. -/
private def expressionPlace? : ExprKind → Option PlaceId
  | .operation (.move place) .. | .operation (.copy place) .. | .operation (.read place) ..
  | .operation (.borrow _ place) .. | .operation (.write place) ..
  | .operation (.drop place) .. | .assign place _ => some place
  | _ => none

/-- The dereferences of identity models in a function's body. -/
private partial def functionIdentityDerefs (typing : PlaceTyping) (ns : RawNamespace)
    (function : FunctionDecl RawBody) : Array PlaceId := Id.run do
  let (direct, roots) := bodyOccurrences function.body
  let mut found : Array PlaceId := #[]
  let mut seen : Std.HashSet Nat := {}
  let mut pending := roots
  for place in direct do
    let (derefs, indexes) := typing.identityDerefs function.locals place
    found := found ++ derefs
    pending := pending ++ indexes
  while let some id := pending.back? do
    pending := pending.pop
    if seen.contains id.index then continue
    seen := seen.insert id.index
    let some expression := ns.expressions[id.index]? | continue
    if let some place := expressionPlace? expression.kind then
      let (derefs, indexes) := typing.identityDerefs function.locals place
      found := found ++ derefs
      pending := pending ++ indexes
    pending := pending ++ Validation.expressionChildren expression.kind
  return found

/-- A namespace with each dereference of an identity model replaced by its base: the
place of the box's content is the place of the box. -/
private def eraseIdentityDerefs (typing : PlaceTyping) (ns : RawNamespace) :
    Except String RawNamespace := do
  let typing := { typing with places := ns.places, expressions := ns.expressions }
  let erased := ns.functions.flatMap (functionIdentityDerefs typing ns)
  if erased.isEmpty then return ns
  let marked := erased.foldl (fun marked (place : PlaceId) => marked.insert place.index)
    (({} : Std.HashSet Nat))
  let mut places := ns.places
  -- A base precedes what it is the base of, so it is already replaced.
  for index in [:places.size] do
    unless marked.contains index do continue
    let .deref base := places[index]! | continue
    unless base.index < index do
      throw "a place's base does not precede it"
    places := places.set! index places[base.index]!
  return { ns with places }

/-- The entry an identity model's type aliases: its argument's, through further ones. -/
private partial def aliasedEntry (types : Array Ty) (modelOfName : NameId → Option ModelDecl)
    (id : TypeId) : Except String Ty := do
  let some entry := types[id.index]?
    | throw s!"a type identifier is out of range: {id.index}"
  let .nominal name arguments := entry | return entry
  let some { name := modeled, model := .identity position, .. } := modelOfName name | return entry
  let some (.typeArg use) := arguments[position]?
    | throw s!"the argument of the library's model of `{modeled}` is not a type"
  aliasedEntry types modelOfName use.typeId

/-- A namespace with each call of a function that drops its argument replaced by what
the callee does: its parameter takes the argument, and dies. -/
private def expandDropCalls (unitType : TypeId) (dropsArgument : NameId → Bool)
    (ns : RawNamespace) : RawNamespace := Id.run do
  let mut ns := ns
  for position in [:ns.functions.size] do
    let function := ns.functions[position]!
    let .cfg graph := function.body | continue
    let mut locals := function.locals
    let mut blocks := graph.blocks
    for index in [:blocks.size] do
      let block := blocks[index]!
      let .call call (some destination) unwind := block.terminator | continue
      let some { kind := .operation (.call (.function callee)) _ #[argument] _, loc, .. } :=
        ns.expressions[call.index]? | continue
      unless dropsArgument callee.name do continue
      let some { typeId, .. } := ns.expressions[argument.index]? | continue
      let parameter : LocalId := ⟨locals.size⟩
      locals := locals.push
        { id := parameter, name := s!"_x{locals.size}", type := { typeId, loc }, loc }
      let place : PlaceId := ⟨ns.places.size⟩
      let first : ExprId := ⟨ns.expressions.size⟩
      let finished : ExprId := ⟨first.index + 2⟩
      ns := { ns with
        places := ns.places.push (.localVar parameter)
        expressions := ns.expressions ++ #[
          { loc, typeId := unitType, kind := .assign place argument },
          { loc, typeId := unitType, kind := .value .unit },
          { loc, typeId := unitType, kind := .assign destination.place ⟨first.index + 1⟩ }] }
      blocks := blocks.set! index { block with
        statements := block.statements ++ #[.execute first, .execute finished]
        terminator := .drop place destination.target unwind }
    let graph := { graph with blocks }
    let function := { function with locals := locals, body := .cfg graph }
    ns := { ns with functions := ns.functions.set! position function }
  return ns

/-- Rewrite a raw unit to what the library's models name: every use of a type
modeled as a VIR type is that type and its declaration is dropped, its scope staying
a name and a generic the model does not read an unused parameter of each dependency
function; a type modeled with fields holds them; a dereference of a type modeled as its
argument is the place itself, read from the place's type (`eraseIdentityDerefs`). A unit
naming no modeled type is unchanged. A call of a function modeled as dropping its
argument becomes that drop. -/
def applyModels (allModels : Array ModelDecl) (unit : RawUnit) : Except String RawUnit := do
  let local? := unit.namespaces[0]?.map (·.identity)
  let modelIn (models : Array ModelDecl) (name : NameId) : Option ModelDecl := do
    let qualified ← unit.tables.names[name.index]?
    if some qualified.namespaceId == local? then none else
    let crate ← unit.tables.namespaces[qualified.namespaceId.index]?
    models.find? fun model =>
      crate.segments == #[model.crate] && qualified.scope == model.scope &&
        qualified.name == model.name
  let (functionModels, models) := allModels.partition (·.model == .dropsArgument)
  let dropsArgument (name : NameId) := (modelIn functionModels name).isSome
  let unit := match unit.tables.types.findIdx? (· == Ty.unit) with
    | some unitType =>
        let namespaces := unit.namespaces.map (expandDropCalls ⟨unitType⟩ dropsArgument)
        { unit with namespaces }
    | none => unit
  let modelOfName := modelIn models
  let modeled (declaration : StructDecl) := (modelOfName declaration.name).isSome
  let dropped (declaration : StructDecl) := match modelOfName declaration.name with
    | some { model := .fields .., .. } | none => false
    | some _ => true
  for ns in unit.namespaces do
    for declaration in ns.structs do
      if modeled declaration && !declaration.properties.any (·.tag == rustOpaqueTag) then
        let name := (unit.tables.names[declaration.name.index]?.map (·.name)).getD "?"
        throw s!"the library models `{name}`, whose shape the unit observes"
  unless unit.namespaces.any (·.structs.any modeled) do return unit
  let typing : PlaceTyping := {
    tables := unit.tables
    places := #[]
    expressions := #[]
    declarations := unit.namespaces.flatMap (·.structs) ++ unit.dependencies.flatMap (·.structs)
    modelOfName }
  let places ← unit.namespaces.mapM (eraseIdentityDerefs typing)
  let mut types := unit.tables.types
  for index in [:types.size] do
    let .nominal name arguments := types[index]! | continue
    let some model := modelOfName name | continue
    unless arguments.size == model.generics do
      throw s!"the library models `{model.name}` with {model.generics} generic parameters, \
        and the unit names it with {arguments.size}"
    if let .identity _ := model.model then
      types := types.set! index (← aliasedEntry unit.tables.types modelOfName ⟨index⟩)
  for index in [:types.size] do
    let .nominal name arguments := types[index]! | continue
    let some model := modelOfName name | continue
    match model.model with
    | .fields .. | .identity _ | .dropsArgument => pure ()
    | .string => types := types.set! index Ty.string
    | .vector element =>
        let some (.typeArg use) := arguments[element]?
          | throw s!"the element of the library's model of `{model.name}` is not a type"
        types := types.set! index (Ty.vector use.typeId none)
  let hold (declaration : StructDecl) : ExceptT String (StateM Tables) StructDecl :=
    match modelOfName declaration.name with
    | some { name, model := .fields kinds fields, .. } =>
        holdingFields name kinds fields declaration
    | _ => pure declaration
  let holdAll : ExceptT String (StateM Tables) (Array RawNamespace × Array RawNamespaceInterface) := do
    let namespaces ← places.mapM fun ns => do
      return { ns with structs := ← (ns.structs.filter (!dropped ·)).mapM hold }
    let dependencies ← unit.dependencies.mapM fun dependency => do
      return { dependency with structs := ← (dependency.structs.filter (!dropped ·)).mapM hold }
    return (namespaces, dependencies)
  let (held, tables) := holdAll.run.run { unit.tables with types }
  let (namespaces, dependencies) ← held
  let (interned, mapping) :=
    internTypes tables.types (Array.ofFn (n := tables.types.size) fun index => ⟨index.val⟩)
  let fns : IdFns Id := mappingFns (fun id => pure (mapping[id.index]?.getD id)) pure pure
  return prune { unit with
    tables := { tables with types := interned }
    namespaces := namespaces.map fun ns => Id.run (remap ns fns)
    dependencies := dependencies.map fun dependency => Id.run (remap dependency fns) }

end VerexIR.Rust.Library
