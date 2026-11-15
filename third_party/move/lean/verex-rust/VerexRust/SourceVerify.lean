-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.Driver
import VerexSL.SourceVerify

/-!
# Verifying a Rust source

A Rust file carries no specifications of its own: they are VerexSL items —
`spec` blocks, specification functions, invariants — in a file beside it,
`<name>.spec.lean` for `<name>.rs` unless another is named. The file is
exported through rustc, its crate rendered as VerexSL with those items
appended and verified, and every message reported in the coordinates of the
Rust file or of the specification file.

A Cargo package's library crate is one namespace whose modules are scopes.
Each file of the crate's modules has its specification file beside it, whose
items reopen the module's scope; an inline module's specifications nest in
those of the file it is declared in.

The standard library's contracts come from `verex-rust/lib` (`VerexRust.Library`).
-/

namespace VerexIR.Rust.SourceVerify

open VerexSL.SourceVerify

/-- Checked contracts for exporter-generated receiver adapters. Borrowing
adapters preserve the receiver; consuming adapters expose the invocation and,
for mutable targets, existentially retain its final receiver state. These are
ordinary specifications proved from the body, never granted by the marker. -/
private def adapterSpecs (unit : VerexIR.Validation.ValidatedUnit)
    (source : System.FilePath) : Except String (Array Companion) := do
  let some ns := unit.namespaces[0]? | return #[]
  let mut specifications : Array String := #[]
  for function in ns.functions do
    let marker (key : String) := function.attributes.findSome? (fun
      | .assign name (.constant (.integer index)) _ => if name == key then some index else none
      | _ => none)
    let dictionaryIndex := marker "shared_method_adapter" <|> marker "consuming_method_adapter"
    let closureTarget := function.attributes.findSome? (fun
      | .assign "closure_method_adapter" (.constant (.string target)) _ => some target
      | _ => none)
    if dictionaryIndex.isNone && closureTarget.isNone then continue
    let parameters := function.signature.parameters
    let some receiver := parameters[0]? | throw "receiver adapter has no receiver"
    let some name := ns.nameOf function.name | throw "receiver adapter has no name"
    unless name.scope.isEmpty do throw "receiver adapter must be at crate scope"
    let (callable, parameterCount, targetReceiver) ← match dictionaryIndex, closureTarget with
      | some index, none => do
          unless parameters.size ≥ 2 && index == Int.ofNat (parameters.size - 1) do
            throw "receiver adapter dictionary must be its final parameter"
          let some dictionary := parameters.back? | throw "receiver adapter has no dictionary"
          let some (.function arguments _ _) := unit.tables.types[dictionary.typeUse.typeId.index]?
            | throw "adapter dictionary is not a function"
          let some targetReceiver := arguments[0]? | throw "adapter dictionary has no receiver"
          pure (s!"«{dictionary.name}»", parameters.size - 1, targetReceiver)
      | none, some qualified => do
          let some target := ns.functions.find? (fun candidate =>
              (ns.nameOf candidate.name).any fun name =>
                "::".intercalate (name.scope.push name.name).toList == qualified)
            | throw "closure adapter target is missing"
          let some targetReceiver := target.signature.parameters[0]? | throw "adapter target has no receiver"
          pure (← VerexSL.Print.declaredFunctionText unit ns target, parameters.size, targetReceiver.typeUse.typeId)
      | _, _ => throw "receiver adapter has conflicting targets"
    let some result := function.signature.results[0]? | throw "receiver adapter has no result"
    let outputs := match unit.tables.types[result.typeId.index]? with
      | some .unit => #[]
      | some (.tuple elements) => elements.mapIdx fun index _ => s!"spec.result[{index}]"
      | _ => #["result"]
    let consuming := !(unit.tables.types[receiver.typeUse.typeId.index]?.any (· matches .reference _))
    let mutableTarget := unit.tables.types[targetReceiver.index]?.any (fun
      | .reference reference => reference.kind == .mutable
      | _ => false)
    let mut finalReceiver := "__verex_final_receiver"
    while parameters.any (·.name == finalReceiver) || function.signature.generics.any (·.name == finalReceiver) do
      finalReceiver := finalReceiver ++ "_"
    let receiverType ← if consuming && mutableTarget then
        VerexSL.Print.specificationTypeText unit ns function.signature.generics receiver.typeUse.typeId
      else pure ""
    let receiver := s!"«{receiver.name}»"
    let mut inputs := #[s!"old({receiver})"]
    let mut mutated := if consuming && mutableTarget then #[s!"«{finalReceiver}»"] else #[]
    for parameter in parameters.extract 1 parameterCount do
      let name := s!"«{parameter.name}»"
      match unit.tables.types[parameter.typeUse.typeId.index]? with
        | some (.reference reference) =>
            if reference.kind == .mutable then
              inputs := inputs.push s!"old({name})"
              mutated := mutated.push name
            else inputs := inputs.push name
        | _ => inputs := inputs.push name
    let arguments := inputs ++ outputs ++ mutated
    let preservation := if consuming then "" else s!"  ensures {receiver} == old({receiver})\n"
    let quantified := if consuming && mutableTarget then
        s!"exists («{finalReceiver}» : {receiverType}), "
      else ""
    specifications := specifications.push s!"spec «{name.name}» where
  pragma opaque
{preservation}  ensures {quantified}ensures_of<{callable}>({", ".intercalate arguments.toList})
"
  if specifications.isEmpty then return #[]
  return #[{
    path := source
    generated := true
    namespaceId := ns.unitIdentity
    text := some ("\n".intercalate specifications.toList) }]

/-- The specification file of a Rust source: the one named, or
`<name>.spec.lean` beside `<name>.rs`, read when it exists. Its items are
spliced into the crate's namespace, the unit's first. -/
def specsFor (unit : VerexIR.Validation.ValidatedUnit) (source : System.FilePath)
    (named : Option System.FilePath) : IO (Array Companion) := do
  let path := named.getD (source.withExtension "spec.lean")
  let some ns := unit.namespaces[0]? | return #[]
  let text ← if ← path.pathExists then some <$> IO.FS.readFile path else pure none
  let adapters ← match adapterSpecs unit source with
    | .ok adapters => pure adapters
    | .error error => throw <| IO.userError error
  return adapters ++ #[{ path, namespaceId := ns.unitIdentity, text }]

/-- The files of a crate's modules: the root's and each module's declared in a
file of its own, with the module's scope. A scope's file holds its
declarations and those of its submodules; a module in its parent's file is
inline and has none of its own. -/
def moduleFiles (unit : VerexIR.Validation.ValidatedUnit)
    (ns : VerexIR.Validation.ValidatedNamespace) :
    Except String (Array (Array String × String)) := do
  let fileOf (loc : VerexIR.LocId) : Option String := do
    let range ← (← ns.location? loc).primary
    return (← ns.localTables.files[range.file.index]?).name
  let scopeOf (name : VerexIR.NameId) : Array String :=
    ((ns.nameOf name).map (·.scope)).getD #[]
  -- Generated adapters have specifications supplied by adapterSpecs. Their
  -- crate-scoped names must not make a module file appear to declare root items.
  let sourceFunctions := ns.functions.filter fun function =>
    !function.attributes.any (fun attr => attr matches
      .assign "shared_method_adapter" _ _ | .assign "consuming_method_adapter" _ _ |
      .assign "closure_method_adapter" _ _)
  let declarations := sourceFunctions.map (fun declaration => (scopeOf declaration.name, declaration.loc)) ++
    ns.structs.map (fun declaration => (scopeOf declaration.name, declaration.loc)) ++
    ns.constants.map (fun declaration => (scopeOf declaration.name, declaration.loc)) ++
    ns.scopes.map fun scope => (scope.path.pop, scope.loc)
  let text (scope : Array String) := if scope.isEmpty then "the crate root"
    else s!"module `{"::".intercalate scope.toList}`"
  -- Every scope's file, each after its parent's.
  let mut scopeFiles : Array (Array String × String) := #[]
  for scope in #[#[]] ++ (ns.scopes.filter (·.kind == .module)).map (·.path) do
    let found := declarations.foldl (init := #[]) fun found (owner, loc) =>
      match fileOf loc with
      | some file => if owner == scope && !found.contains file then found.push file else found
      | none => found
    match found with
    | #[file] => scopeFiles := scopeFiles.push (scope, file)
    | #[] => throw s!"{text scope} declares nothing with a source location"
    | _ => throw s!"{text scope} declares items in several files: {", ".intercalate found.toList}"
  return scopeFiles.filter fun (scope, file) =>
    scope.isEmpty || (scopeFiles.find? (·.1 == scope.pop)).map (·.2) != some file

/-- The specification files of a crate: `<name>.spec.lean` beside each file of
its modules, read when it exists, its items in the module's scope. -/
def crateSpecs (unit : VerexIR.Validation.ValidatedUnit) (root : System.FilePath) :
    IO (Array Companion) := do
  -- The crate is namespace 0; the namespaces after it are its dependencies'.
  let some ns := unit.namespaces[0]?
    | throw <| IO.userError "a Rust crate is exported with no namespace"
  let files ← match moduleFiles unit ns with
    | .ok files => pure files
    | .error message => throw <| IO.userError message
  let companions ← files.mapM fun (scope, file) => do
    let path := (root / file).withExtension "spec.lean"
    let text ← if ← path.pathExists then some <$> IO.FS.readFile path else pure none
    return { path, namespaceId := ns.unitIdentity, scope, text }
  let adapters ← match adapterSpecs unit root with
    | .ok adapters => pure adapters
    | .error error => throw <| IO.userError error
  return adapters ++ companions

/-- What a type names: its nominal types, each `crate::scope::name`, and its scalars. -/
private partial def namedBy (unit : VerexIR.Validation.ValidatedUnit) (type : VerexIR.TypeId)
    (fuel : Nat := unit.tables.types.size + 1) : Array String :=
  if fuel == 0 then #[] else
  let visit := (namedBy unit · (fuel - 1))
  match unit.tables.types[type.index]? with
  | some .bool => #["Bool"]
  | some (.integer .pointer signed) => #[if signed then "isize" else "usize"]
  | some (.integer (.bits width) signed) => #[s!"{if signed then "i" else "u"}{width}"]
  | some (.reference reference) => visit reference.referent
  | some (.tuple elements) => elements.flatMap visit
  | some (.vector element _) | some (.slice element) => visit element
  | some (.nominal name arguments) =>
      let arguments := arguments.flatMap fun
        | .typeArg argument => visit argument.typeId
        | _ => #[]
      match unit.tables.names[name.index]? with
      | some qualified =>
          let crate := (unit.tables.namespaces[qualified.namespaceId.index]?.map (·.segments)).getD #[]
          arguments.push ("::".intercalate (crate ++ qualified.scope ++ #[qualified.name]).toList)
      | none => arguments
  | _ => #[]

/-- The calls of a crate's bodies, each by the scope and name of its callee, in `ns`,
with what its generic arguments name. -/
private def callsInto (unit : VerexIR.Validation.ValidatedUnit)
    (ns : VerexIR.Validation.ValidatedNamespace) : Array (Array String × String × Array String) :=
  match unit.namespaces[0]? with
  | none => #[]
  | some crate => crate.expressions.filterMap fun expression => do
    let .operation (.call callee) instantiations _ _ := expression.kind | none
    let reference ← match callee with
      | .function reference | .closure reference _ => some reference
      | _ => none
    guard (crate.namespaceId reference.namespaceId == ns.unitIdentity)
    let name ← crate.nameOf reference.name
    let named := (crate.genericArguments instantiations).flatMap fun
      | .typeArg argument => namedBy unit argument.typeId
      | _ => #[]
    return (name.scope, name.name, named)

/-- The library's contracts of the functions a unit declares, each library file
a companion of its crate's namespace. A contract is a `native_f` hypothesis of
the theorems whose functions reach it. A contract at a shape comes only for a call
naming what the shape names, since every type it adds makes a call's frame dearer to
certify; a function with calls no shape matches keeps its shapes for the message
listing them. -/
def libraryCompanions (unit : VerexIR.Validation.ValidatedUnit)
    (clauses : Array String := #[]) : IO (Array Companion) := do
  let files ← VerexIR.Rust.Library.readFiles (← Driver.libraryRoot)
  -- The crate is namespace 0; the library speaks of its dependencies.
  let dependencies := unit.namespaces.extract 1 unit.namespaces.size
  let crateNamespace := (unit.namespaces[0]?.map (·.unitIdentity)).getD ⟨0⟩
  let spliced := files.filterMap fun file => do
    let ns ← dependencies.find? fun ns =>
      (unit.tables.namespaces[ns.unitIdentity.index]?.map (·.segments)) == some #[file.crate]
    let declares (scope : Array String) (target : String) := ns.functions.any fun function =>
      (ns.nameOf function.name).any fun name =>
        name.scope == scope && name.name == target
    let crateOf (other : VerexIR.Validation.ValidatedNamespace) :=
      unit.tables.namespaces[other.unitIdentity.index]?.map (·.segments)
    let declaresIn (ns : VerexIR.Validation.ValidatedNamespace) (scope : Array String)
        (target : String) := ns.structs.any fun declaration =>
      (ns.nameOf declaration.name).any fun name =>
        name.scope == scope && name.name == target
    -- A shape's path resolves in the namespace, or from another crate's name.
    let crateNamed (scope : Array String) := dependencies.find? fun other =>
      other.unitIdentity != ns.unitIdentity && crateOf other == some (scope.take 1)
    let declaresType (scope : Array String) (target : String) :=
      declaresIn ns scope target ||
        (crateNamed scope).any (declaresIn · (scope.extract 1 scope.size) target)
    let calls := callsInto unit ns
    let typeText (scope : Array String) (name : String) :=
      let qualified := if (crateNamed scope).isSome then scope else #[file.crate] ++ scope
      "::".intercalate (qualified.push name).toList
    let matched (contract : VerexIR.Rust.Library.ContractItem) :=
      calls.any fun (scope, target, named) =>
        scope == contract.scope && target == contract.target &&
          contract.shapeScalars.all named.contains &&
          contract.shapeTypes.all fun (typeScope, name) => named.contains (typeText typeScope name)
    let keep (contract : VerexIR.Rust.Library.ContractItem) :=
      declares contract.scope contract.target &&
        contract.shapeTypes.all (fun (scope, name) => declaresType scope name) &&
        (!contract.atShape || matched contract || !file.contracts.any fun other =>
          other.scope == contract.scope && other.target == contract.target && matched other)
    let text ← file.reachedItems keep clauses
    let uses := (file.contracts.filter keep).flatMap fun contract =>
      contract.shapeTypes.filterMap fun (scope, _) => (crateNamed scope).map (·.unitIdentity)
    -- A clause writes a function of the library by the path from its scope.
    let scopes := (file.specFunctions.filter fun item =>
      clauses.any (item.namedIn false)).foldl (init := #[]) fun scopes item =>
        if scopes.contains item.scope then scopes else scopes.push item.scope
    let imports := scopes.map fun scope =>
      let segments := scope.map (s!"«{·}»")
      s!"use {file.crate}::{"::".intercalate segments.toList}"
    let clauseUses : Array Companion := if imports.isEmpty then #[] else #[{
      path := file.path, namespaceId := crateNamespace
      text := some ("\n".intercalate imports.toList), uses := #[ns.unitIdentity] }]
    return #[{ path := file.path, namespaceId := ns.unitIdentity, text := some text, uses }] ++
      clauseUses
  return spliced.flatten

/-- Verify a Rust file against its specification file, writing the
VerexSL rendering to `output`. -/
def verifyFile (environment : Lean.Environment) (source : System.FilePath)
    (specFile : Option System.FilePath) (output : System.FilePath)
    (rustcArgs : Array String := #[]) (ignoreProofs : Bool := false) : IO (Array Report) := do
  let (unit, companions) ← VerexSL.Perf.withPhase .frontend do
    let unit ← IO.FS.withTempDir fun directory =>
      return (← Driver.importRustFile {
        source, output := directory / "unit.raw.json", rustcArgs }).unit
    let specs ← specsFor unit source specFile
    pure (unit, specs ++ (← libraryCompanions unit (specs.filterMap (·.text))))
  run environment { unit, companions, output, ignoreProofs }

/-- Verify the library crate of a Cargo package against the specification
files of its modules, writing the VerexSL rendering to `output`. -/
def verifyCrate (environment : Lean.Environment) (request : Driver.CargoImportRequest)
    (output : System.FilePath) (ignoreProofs : Bool := false) : IO (Array Report) := do
  let (unit, sourceRoot, companions) ← VerexSL.Perf.withPhase .frontend do
    let result ← IO.FS.withTempDir fun directory =>
      Driver.importCargoCrate { request with output := directory / "unit.raw.json" }
    let some root := result.sourceRoot
      | throw <| IO.userError "a Cargo import did not name its workspace root"
    let specs ← crateSpecs result.unit root
    pure (result.unit, root, specs ++ (← libraryCompanions result.unit (specs.filterMap (·.text))))
  run environment { unit, companions, output, ignoreProofs, sourceRoot }

end VerexIR.Rust.SourceVerify
