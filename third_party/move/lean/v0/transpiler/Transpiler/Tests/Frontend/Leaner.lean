-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.LIR.Leaner
import Transpiler.LIR.Backend
import Transpiler.LIR.Effects
import Transpiler.Print
import Transpiler.Tests.Programs.Account
import Transpiler.Tests.Programs.AptosFramework.Counter
import Transpiler.Tests.Programs.BasicCoin
import Transpiler.Tests.Programs.Constants
import Transpiler.Tests.Programs.Enums
import Transpiler.Tests.Programs.Generics
import Transpiler.Tests.Programs.MoveStdlib.Std.Signer
import Transpiler.Tests.Programs.MoveStdlib.Std.Mem
import Transpiler.Tests.Programs.MoveStdlib.Std.Error
import Transpiler.Tests.Programs.OrderedMap
import Transpiler.Tests.Programs.Vectors

/-!
# Leaner frontend end-to-end integration

This test starts with elaborated Leaner declarations, runs the existing named
compiler IR through the raw LIR boundary and its shared structurizer, projects
the checked unit through the Leaner backend, and finally exercises the
canonical printer.  In particular, the two selected functions contain natural
loops, so a successful run checks more than a one-block transport.
-/

namespace Transpiler.Tests.Frontend.Leaner

open Lean

private def validateRaw (label : String) (raw : Except String LeanerIR.Import.RawUnit) :
    Except String Unit := do
  let raw ← raw
  match LeanerIR.Move.validate raw with
  | .ok checked =>
      let effects ← Transpiler.LIR.Effects.computePrinterTable checked
      let package ← Transpiler.LIR.Backend.toPrinterPackage .move checked
      for sourceModule in package.modules do
        let _ ← Transpiler.Print.printModule package sourceModule effects
  | .error diagnostics => throw <| label ++ ":\n" ++
      Transpiler.LIR.Backend.renderDiagnostics diagnostics

private def requireContractCount (label : String) (expected : Nat)
    (raw : Except String LeanerIR.Import.RawUnit) : Except String LeanerIR.Import.RawUnit := do
  let raw ← raw
  let some sourceNamespace := raw.namespaces[0]?
    | throw <| label ++ " produced no namespace"
  let actual := sourceNamespace.functions.countP (!·.contract.conditions.isEmpty)
  unless actual == expected do
    throw s!"{label} attached {actual} source contracts; expected {expected}"
  return raw

private def u64 : Move.Compiler.LIR.Ty :=
  .int { width := .w64, signed := false }

/-- The exact NSIR expansion used for mutating vector operations. Keeping it
synthetic avoids making this adapter test depend on Lean code-generation cost
for the much larger ordered-map fixture. -/
private def vectorMutationModule : Move.Compiler.LIR.Module := {
  address := "0x42"
  name := "vector_mutation"
  «structs» := #[]
  «functions» := #[{
    leanName := `vector_mutation.remove
    moveName := "remove"
    visibility := .private_
    params := #[
      { name := "v", ty := .mutRef (.vector u64), sourceName := some "v" },
      { name := "i", ty := u64, sourceName := some "i" }]
    returns := #[u64]
    locals := #[
      { name := "old", ty := .vector u64 },
      { name := "updated", ty := .vector u64 },
      { name := "removed", ty := u64 }]
    blocks := #[{
      name := "entry"
      instrs := #[
        .call #["old"] .readRef #["v"],
        .call #["updated", "removed"] .vecRemove #["old", "i"],
        .call #[] .writeRef #["v", "updated"]]
      term := .ret #["removed"] }]
    calls := #[]
    acquires := #[] }] }

private def checkAccount : Lean.CoreM (Except String Unit) := do
  let env ← Lean.getEnv
  let accountSource := Move.sourceModuleArtifact? env `account
  let accountResult ← Transpiler.LIR.Leaner.compileNamespace .move `account
    { fileName := "Transpiler/Tests/Programs/Account.lean" }
  return do
    let some accountSource := accountSource
      | throw "the Account module did not retain its parsed source artifact"
    unless accountSource.items.any (·.isOfKind ``Move.Spec.abortsIfSourceSpec) do
      throw "the retained Account artifact lost its authored specification clauses"
    let accountRaw ← accountResult
    let some accountNamespace := accountRaw.namespaces[0]?
      | throw "the Account frontend produced no namespace"
    unless accountNamespace.functions.countP (!·.contract.conditions.isEmpty) == 3 do
      throw "the Account frontend did not attach all three authored contracts"
    unless accountRaw.evidence.size == 1 && accountRaw.evidence[0]!.trusted do
      throw "the fully supported Account source bridge was not marked checked"
    validateRaw "account frontend failed" (.ok accountRaw)

private def checkDeclarationsAndContracts : Lean.CoreM (Except String Unit) := do
  let basicCoinResult ← Transpiler.LIR.Leaner.compileNamespace .move `basic_coin
    { fileName := "Transpiler/Tests/Programs/BasicCoin.lean" }
  let counterResult ← Transpiler.LIR.Leaner.compileNamespace .move `AptosFramework.counter
    { fileName := "Transpiler/Tests/Programs/AptosFramework/Counter.lean" }
  let constantsResult ← Transpiler.LIR.Leaner.compileNamespace .move `constants
    { fileName := "Transpiler/Tests/Programs/Constants.lean" }
  let genericsResult ← Transpiler.LIR.Leaner.compileNamespace .move `generics
    { fileName := "Transpiler/Tests/Programs/Generics.lean" }
  let enumsResult ← Transpiler.LIR.Leaner.compileNamespace .move `enums
    { fileName := "Transpiler/Tests/Programs/Enums.lean" }
  let orderedMapResult ← Transpiler.LIR.Leaner.compileNamespace .move `ordered_map
    { fileName := "Transpiler/Tests/Programs/OrderedMap.lean" }
  return do
    let basicCoinRaw ← requireContractCount "basic coin" 2 basicCoinResult
    let some basicCoinNamespace := basicCoinRaw.namespaces[0]?
      | throw "the BasicCoin frontend produced no namespace"
    unless basicCoinNamespace.structs.size == 1 && basicCoinNamespace.constants.size == 1 &&
        basicCoinNamespace.functions.size == 2 && basicCoinNamespace.specFunctions.size == 1 do
      throw "the BasicCoin frontend lost a declaration"
    let constantsRaw ← requireContractCount "constants" 1 constantsResult
    let counterRaw ← requireContractCount "counter" 1 counterResult
    let some counterNamespace := counterRaw.namespaces[0]?
      | throw "the Counter frontend produced no namespace"
    unless counterNamespace.functions.size == 4 && counterRaw.evidence.size == 1 &&
        counterRaw.evidence[0]!.trusted do
      throw "the Counter module was not completely and trustfully imported"
    let some constantsNamespace := constantsRaw.namespaces[0]?
      | throw "the Constants frontend produced no namespace"
    unless constantsNamespace.constants.size == 5 do
      throw "the Constants frontend did not preserve all named constants"
    let constantValues := constantsNamespace.constants.filterMap fun declaration =>
      constantsNamespace.expressions[declaration.value.index]?
    unless constantValues.any (fun expression => expression.kind matches .value (.vector _) _) &&
        constantValues.any (fun expression => expression.kind matches .value (.address _) _) do
      throw "the Constants frontend lost vector or address constant values"
    unless constantsNamespace.specFunctions.size == 3 &&
        constantsNamespace.specFunctions.countP (·.body.isSome) == 2 do
      throw "the Constants frontend did not distinguish translated and opaque specification functions"
    unless constantsRaw.evidence.any fun evidence =>
        evidence.description.startsWith
          "unsupported source semantic item: specification function `int2bv_and_u64`" do
      throw "the unsupported bitvector specification function was not reported in import evidence"
    let genericsRaw ← requireContractCount "generics" 1 genericsResult
    let orderedMapRaw ← requireContractCount "ordered map" 1 orderedMapResult
    let enumsRaw ← requireContractCount "enums" 0 enumsResult
    unless enumsRaw.evidence.any (·.description.startsWith
        "unsupported source semantic item: contract `total`") do
      throw "the unsupported enum match contract was not reported in import evidence"
    validateRaw "basic-coin frontend failed" (.ok basicCoinRaw)
    validateRaw "constants frontend failed" (.ok constantsRaw)
    validateRaw "counter frontend failed" (.ok counterRaw)
    validateRaw "generics frontend failed" (.ok genericsRaw)
    validateRaw "enums frontend failed" (.ok enumsRaw)
    validateRaw "ordered-map frontend failed" (.ok orderedMapRaw)

private def checkReferencesAndCollections : Lean.CoreM (Except String Unit) := do
  let signerResult ← Transpiler.LIR.Leaner.compileNamespace .move `Std.signer
    { fileName := "Transpiler/Tests/Programs/MoveStdlib/Std/Signer.lean" }
  let memResult ← Transpiler.LIR.Leaner.compileNamespace .move `Std.mem
    { fileName := "Transpiler/Tests/Programs/MoveStdlib/Std/Mem.lean" }
  let errorResult ← Transpiler.LIR.Leaner.compileNamespace .move `Std.error
    { fileName := "Transpiler/Tests/Programs/MoveStdlib/Std/Error.lean" }
  let vectorsResult ← Transpiler.LIR.Leaner.compileNamespace .move `vectors
    { fileName := "Transpiler/Tests/Programs/Vectors.lean" }
  let vectorMutationResult := Transpiler.LIR.Leaner.rawUnit .move vectorMutationModule
    { fileName := "vector-mutation.fixture" }
  return do
    let signerRaw ← requireContractCount "signer" 1 signerResult
    let some signerNamespace := signerRaw.namespaces[0]?
      | throw "the Signer frontend produced no namespace"
    unless signerNamespace.functions.size == 2 && signerNamespace.specFunctions.size == 3 &&
        signerRaw.evidence.size == 1 && signerRaw.evidence[0]!.trusted do
      throw "the Signer module's references and specification surface were not completely imported"
    unless signerRaw.tables.lifetimes.any (·.kind == .inference) &&
        signerRaw.tables.types.any (fun ty => ty matches .reference _) do
      throw "the Signer frontend did not encode references and inferred lifetimes in core LIR"
    let memRaw ← requireContractCount "mem" 2 memResult
    let some memNamespace := memRaw.namespaces[0]?
      | throw "the Mem frontend produced no namespace"
    unless memNamespace.functions.size == 2 && memRaw.evidence.size == 1 &&
        memRaw.evidence[0]!.trusted do
      throw "the Mem module's generic mutable-reference contracts were not completely imported"
    let errorRaw ← requireContractCount "error" 0 errorResult
    let some errorNamespace := errorRaw.namespaces[0]?
      | throw "the Error frontend produced no namespace"
    unless errorNamespace.functions.size == 13 && errorNamespace.constants.size == 13 do
      throw "the Error module did not preserve its executable functions and constants"
    unless errorRaw.evidence.any (·.description.startsWith
        "unsupported source semantic item: contract `canonical`") do
      throw "the unsupported let-bound Error contract was not reported in import evidence"
    let vectorsRaw ← vectorsResult
    let some vectorsNamespace := vectorsRaw.namespaces[0]?
      | throw "the Vectors frontend produced no namespace"
    unless vectorsNamespace.functions.size == 6 &&
        vectorsRaw.tables.types.any (fun ty => ty matches .vector _ none) do
      throw "the Vectors frontend did not encode vector structure in core LIR"
    validateRaw "signer frontend failed" (.ok signerRaw)
    validateRaw "mem frontend failed" (.ok memRaw)
    validateRaw "error frontend failed" (.ok errorRaw)
    validateRaw "vectors frontend failed" (.ok vectorsRaw)
    validateRaw "vector-mutation normalization failed" vectorMutationResult

private def checkLeanerFrontendCorpus : Lean.CoreM (Except String Unit) := do
  let account ← checkAccount
  let declarationsAndContracts ← checkDeclarationsAndContracts
  let referencesAndCollections ← checkReferencesAndCollections
  return do
    account
    declarationsAndContracts
    referencesAndCollections

syntax (name := checkLeanerFrontendCorpusCmd) "#check_leaner_frontend_corpus" : command

open Lean Elab Command in
@[command_elab checkLeanerFrontendCorpusCmd]
private def elabCheckLeanerFrontendCorpus : CommandElab := fun _ => do
  match ← liftCoreM checkLeanerFrontendCorpus with
  | .ok () => pure ()
  | .error message => throwError message

#check_leaner_frontend_corpus

end Transpiler.Tests.Frontend.Leaner
