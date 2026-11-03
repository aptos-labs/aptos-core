-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model
import Import

/-!
# The model on the Aptos framework (N4)

N4 (plan §7): `WellFormed` is non-trivial, because the model accepts the
Aptos framework and the standard library. `scripts/check-framework.sh` exports
the modules of the framework's release bundle (`head.mrb`) with the Rust
verifier's decisions (`vm-oracle`), and runs this file on them:

```bash
lake env lean --run Tests/Framework.lean DOCUMENT.json
```

The check fails unless:

- the module ids are distinct;
- the model accepts every module, by local verification and by link checks
  against the other modules, and decides each as the Rust verifier does (A2);
- the framework's `0x1::table` keeps the table rules
  (`CompiledModule.followsTableRules`), under which its natives are modeled;
- at least `minCovered` modules are covered: the program of the module and
  everything it depends on is in the supported subset (`Program.Supported`,
  `Program.SupportedVersions`) and well formed under the model, so the
  theorems of `Spec/Properties/Theorems.lean` apply to it.

Coverage is a measurement, not a theorem. The clauses of
`CompiledModule.Supported` are transcribed here, not derived from the
definition. They quantify over all type arguments of a given length, and
`RuntimeType.ofTokenWith` fails only on a token's shape and a type
parameter's index, never on the types substituted for type parameters, so
checking one instance (all `bool`) decides them.

`lake test` does not run this file: the document is generated from the
release bundle, which changes with every change to the framework, so it is
not committed.
-/

namespace MoveBytecode.Tests.Framework

open MoveBytecode.Import MoveBytecode.Model

/-- The fewest covered modules the check accepts. Raise it when coverage
grows. -/
def minCovered : Nat := 121

def reps (n : Nat) : List RuntimeType := List.replicate n .bool

def signatureOk (m : CompiledModule) (s : SignatureIndex) (n : Nat) : Bool :=
  match m.signatures[s]? with
  | some sig => (RuntimeType.ofSignatureWith m (reps n) sig).isSome
  | none => false

def valueSignatureOk (m : CompiledModule) (s : SignatureIndex) (n : Nat) : Bool :=
  match m.signatures[s]? with
  | some sig => (RuntimeType.ofTokenListWith m (reps n) sig).isSome
  | none => false

def noFunctionTypes (m : CompiledModule) (s : SignatureIndex) : Bool :=
  match m.signatures[s]? with
  | some sig => sig.all (fun t => !t.mentionsFunction)
  | none => false

def hasReference (m : CompiledModule) (s : SignatureIndex) : Bool :=
  match m.signatures[s]? with
  | some sig => sig.any fun t => match t with
    | .reference _ | .mutableReference _ => true
    | _ => false
  | none => false

/-- `CompiledModule.SupportedOperands` at one instance. -/
def operandsOk (m : CompiledModule) (i : Bytecode) (n : Nat) : Bool :=
  match i with
  | .vecPack s _ | .vecUnpack s _ | .vecLen s | .vecImmBorrow s | .vecMutBorrow s
  | .vecPushBack s | .vecPopBack s | .vecSwap s => (m.vectorElemTypeWith (reps n) s).isSome
  | .callGeneric i | .packClosureGeneric i _ =>
    match m.functionInstantiations[i]? with
    | some x => valueSignatureOk m x.typeParameters n
    | none => false
  | .packGeneric i | .unpackGeneric i | .mutBorrowGlobalGeneric i | .immBorrowGlobalGeneric i
  | .existsGeneric i | .moveFromGeneric i | .moveToGeneric i =>
    match m.structDefInstantiations[i]? with
    | some x => valueSignatureOk m x.typeParameters n
    | none => false
  | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i =>
    match m.structVariantInstantiations[i]? with
    | some x => valueSignatureOk m x.typeParameters n
    | none => false
  | .mutBorrowFieldGeneric i | .immBorrowFieldGeneric i =>
    match m.fieldInstantiations[i]? with
    | some x => valueSignatureOk m x.typeParameters n
    | none => false
  | .mutBorrowVariantFieldGeneric i | .immBorrowVariantFieldGeneric i =>
    match m.variantFieldInstantiations[i]? with
    | some x => valueSignatureOk m x.typeParameters n
    | none => false
  | .callClosure s => valueSignatureOk m s n
  | _ => true

def address (a : Address) : String :=
  s!"0x{String.ofList (Nat.toDigits 16 a.toNat)}"

def moduleName (m : CompiledModule) : String :=
  match m.selfId with
  | some id => s!"{address id.address}::{id.name}"
  | none => "?"

def handleName (m : CompiledModule) (fh : FunctionHandle) : String :=
  let module := match m.moduleIdAt fh.module with
    | some id => s!"{address id.address}::{id.name}"
    | none => "?"
  s!"{module}::{(m.identifiers[fh.name]?).getD "?"}"

def opName (i : Bytecode) : String :=
  ((toString (repr i)).splitOn " ").head!

/-- A reason a module is outside the supported subset, and what it is about. -/
abbrev Issue := String × String

/-- Every way module `m` falls outside `CompiledModule.Supported` and its part
of `Program.SupportedVersions`, clause by clause. -/
def moduleIssues (m : CompiledModule) : Array Issue := Id.run do
  let mut out : Array Issue := #[]
  unless 6 ≤ m.version && m.version ≤ 10 do
    out := out.push ("version", s!"{moduleName m}: {m.version}")
  for fh in m.functionHandles do
    let n := fh.typeParameters.length
    unless signatureOk m fh.parameters n && signatureOk m fh.returns n do
      out := out.push ("handle signature", handleName m fh)
    if m.version < 10 && fh.attributes.any (·.isStructApi) then
      out := out.push ("struct-API attribute below version 10", handleName m fh)
  for d in m.functionDefs do
    let some fh := m.functionHandles[d.function]? | out := out.push ("function handle", moduleName m)
    let n := fh.typeParameters.length
    let name := handleName m fh
    match d.code with
    | none =>
      let functionTypes := !(noFunctionTypes m fh.parameters && noFunctionTypes m fh.returns)
      unless (m.modeledNative? d).isSome ||
          (valueSignatureOk m fh.parameters n && valueSignatureOk m fh.returns n && !functionTypes) do
        if functionTypes then out := out.push ("native with function types", name)
        else if hasReference m fh.parameters || hasReference m fh.returns then
          out := out.push ("native with references", name)
        else out := out.push ("native signature", name)
    | some code =>
      unless signatureOk m code.locals n do out := out.push ("locals", name)
      for i in code.code do
        unless i.inSubset do out := out.push (s!"instruction {opName i}", name)
        unless operandsOk m i n do out := out.push (s!"operand of {opName i}", name)
  for d in m.structDefs do
    let name := s!"{moduleName m}::{(m.structDefName d).getD "?"}"
    if d.fieldInformation == .native then out := out.push ("native struct", name)
    else match m.structDefHandle d with
      | none => out := out.push ("struct handle", name)
      | some sh =>
        for f in CompiledModule.StructFieldInformation.allFields d.fieldInformation do
          unless (RuntimeType.ofTokenWith m (reps sh.typeParameters.length) f.signature).isSome do
            out := out.push ("struct field", name)
  for c in m.constantPool do
    unless (RuntimeType.ofToken m c.type).isSome do out := out.push ("constant", moduleName m)
  return out

/-- `Program.ClosureTargetsHaveCode` for the module at index `mi`: its closures
over natives. -/
def closureIssues (P : Program) (mi : Nat) (m : CompiledModule) : Array Issue := Id.run do
  let mut out := #[]
  for d in m.functionDefs do
    let some code := d.code | continue
    for i in code.code do
      let some h := i.closureHandle? m | continue
      let some f := P.resolve mi h | continue
      let some info := P.function? f | continue
      unless info.definition.code.isSome do
        out := out.push ("closure over a native", s!"{moduleName m}: {handleName m info.handle}")
  return out

/-- The program indices of the modules `m` names other than itself, or `none`
if one is not in the program. -/
def dependencies (P : Program) (m : CompiledModule) : Option (List Nat) :=
  m.moduleHandles.toList.filterMapM fun mh => do
    let id ← m.moduleIdOf mh
    if some id == m.selfId then pure none else some <$> P.findModule id

/-- The modules the module at index `i` depends on, transitively, itself
included, or `none` if one is not in the program. -/
partial def closure (P : Program) (todo : List Nat) (seen : Std.HashSet Nat) :
    Option (Std.HashSet Nat) :=
  match todo with
  | [] => some seen
  | i :: rest =>
    if seen.contains i then closure P rest seen
    else do
      let ds ← dependencies P (← P.modules[i]?)
      closure P (ds ++ rest) (seen.insert i)

def decision : VResult Unit → String
  | .ok () => "ok"
  | .error (.status code) => code
  | .error (.panic what) => s!"panic: {what}"
  | .error (.fuel what) => s!"fuel: {what}"

/-- The program of the modules at `indices` is well formed under the model:
every module passes local verification and link checks against the others. -/
def wellFormed (P : Program) (local_ : Array Bool) (indices : List Nat) : Bool :=
  let Q : Program := ⟨(indices.filterMap (P.modules[·]?)).toArray⟩
  indices.all (local_.getD · false) &&
    (List.range Q.modules.size).all fun j => match Q.modules[j]? with
      | some m => decision (verifyLinking VerifierConfig.production m (Q.others j)) == "ok"
      | none => false

def counts (xs : Array String) : List (String × Nat) :=
  let m := xs.foldl (fun (acc : Std.HashMap String Nat) x => acc.insert x (acc.getD x 0 + 1)) {}
  (m.toList.toArray.qsort (fun a b => a.2 > b.2 || (a.2 == b.2 && a.1 < b.1))).toList

end MoveBytecode.Tests.Framework

open MoveBytecode MoveBytecode.Import MoveBytecode.Model MoveBytecode.Tests.Framework in
def main (args : List String) : IO UInt32 := do
  let some path := args.head? | IO.eprintln "usage: Tests/Framework.lean DOCUMENT.json"; return 2
  let (modules, rust) ← IO.ofExcept (verifierDocument (← IO.FS.readFile path))
  let P : Program := ⟨modules⟩
  let mut failures : Array String := #[]
  if modules.size != rust.size || modules.isEmpty then
    failures := failures.push s!"{modules.size} modules but {rust.size} Rust decisions"
  let distinct := decide (modules.toList.map (·.selfId)).Nodup
  unless distinct do failures := failures.push "module ids are not distinct"
  IO.println s!"modules: {modules.size}, distinct ids: {distinct}"
  -- N4 and A2: the model accepts every module, as the Rust verifier does.
  let mut local_ : Array Bool := #[]
  let mut accepted := 0
  for (m, i) in modules.toList.zipIdx do
    let loc := decision (verifyModule VerifierConfig.production m)
    let lnk := decision (verifyLinking VerifierConfig.production m (P.others i))
    local_ := local_.push (loc == "ok")
    if loc == "ok" && lnk == "ok" then accepted := accepted + 1
    else failures := failures.push s!"the model rejects {moduleName m}: {loc}, linking {lnk}"
    if let some r := rust[i]? then
      if loc != r.local_ || lnk != r.linking then
        failures := failures.push
          s!"{moduleName m}: model {loc}, linking {lnk}; Rust {r.local_}, linking {r.linking}"
  IO.println s!"accepted by the model: {accepted} of {modules.size}"
  -- The table rules (`CompiledModule.followsTableRules`) on the framework's table module.
  let tableRules := (modules.find? (·.selfId == some tableModuleId)).map (·.followsTableRules)
  IO.println s!"0x1::table keeps the table rules: {tableRules}"
  unless tableRules == some true do failures := failures.push "0x1::table breaks the table rules"
  -- Coverage.
  let mut issues : Array Issue := #[]
  let mut inSubset : Std.HashSet Nat := {}
  for (m, i) in modules.toList.zipIdx do
    let own := moduleIssues m ++ closureIssues P i m
    issues := issues ++ own
    if own.isEmpty then inSubset := inSubset.insert i
  IO.println s!"in the supported subset: {inSubset.size} of {modules.size}"
  let mut covered : Array String := #[]
  for (m, i) in modules.toList.zipIdx do
    let some c := closure P [i] {} | continue
    if c.toList.all inSubset.contains && wellFormed P local_ c.toList then
      covered := covered.push (moduleName m)
  IO.println s!"covered with all their dependencies: {covered.size} (at least {minCovered} expected)"
  if covered.size < minCovered then
    failures := failures.push s!"only {covered.size} modules covered, fewer than {minCovered}"
  IO.println "outside the subset, by reason:"
  for (k, n) in counts (issues.map (·.1)) do IO.println s!"  {n}\t{k}"
  IO.println "outside the subset, by function or type:"
  let distinctIssues := (counts (issues.map fun x => s!"{x.1}: {x.2}")).map (·.1)
  for x in distinctIssues do IO.println s!"  {x}"
  IO.println s!"covered: {covered.toList}"
  if failures.isEmpty then
    IO.println "ok"
    return 0
  for x in failures do IO.eprintln s!"FAIL {x}"
  return 1
