-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.Steps

/-!
# Verification through the denotation

`verify f` compiles `f`'s validated body to its typed term
(`VerexIR.Proofs.Denote.compileFunction`), certifies that compilation by
`rfl`, proves the authored contract over the term's denotation by one
normalization with the `vir_denote` inventory and the leaf solver, and
transports the result to the big-step meaning through the agreement
theorem.  Per target it publishes:

- `f.compiled`, the term, and `f.compiled_eq`, the kernel-checked
  certificate that the compiler produced it;
- `f.typedVerified`, the contract over the denotation;
- `f.verified`, the same contract of the function's big-step meaning.

Nothing here is proved about agreement per target, and nothing selects a
route: a construct the compiler does not carry is a reported error.

This module holds the commands (`verify`, module items). The parts before it,
split for build time: `Verify.Contracts` (compiled functions, contracts,
loop invariants) and `Verify.Members` (targets, cycles, lemmas).
-/

namespace VerexSL.Verify

open Lean Meta Elab Command
open VerexIR (RuntimeValue FunctionHandle NamespaceId FunctionId)
open VerexIR.Validation (ValidatedUnit ValidatedNamespace)
open VerexIR.Proofs.Denote
open VerexSL.Contract

/-! ## Commands -/

private partial def pathSegments (stx : Syntax) : Array String :=
  match stx with
  -- The identifier's name, as the elaborator registers it: a quoted segment
  -- such as a script's `«<SELF>_0»` without its quotes.
  | .ident _ _ name _ => #[name.toString (escape := false)]
  | .atom _ value => if value == "::" then #[] else #[value]
  | .node _ _ arguments => arguments.flatMap pathSegments
  | _ => #[]

/-- The optional authored script of a `verify` form: its block lowered, a
proof in steps unless it is one `lean =>` step. -/
def scriptOfOptional (optional : Syntax) : CommandElabM (Option AuthoredScript) :=
  match optional.getArgs with
  | #[_, block] => do
      let steps := !(match blockSteps block with
        | #[step] => step.isOfKind ``verexLeanStep
        | _ => false)
      return some { tactic := ← lowerProofBlock block, steps }
  | _ => pure none

/-- Verify one function of a registered unit from outside its module. A
qualified path names the module; a bare name must be declared by exactly one
registered module. -/
syntax (name := verexVerifyCommand)
  ("verify!" <|> "verify") verexPath ("by" verexProofBlock)? : command

/-- Whether a `verify` item or command is marked `verify!`. -/
def demandedVerify (stx : Syntax) : Bool :=
  (stx[0].find? fun keyword => keyword.isAtom && keyword.getAtomVal == "verify!").isSome

/-- Where the `verify` items are skipped (`skipVerify`), one without `!` is
skipped with this message. -/
def skippedInEditor (reference : Syntax) (function : String) : CommandElabM Unit :=
  logInfoAt reference m!"`{function}` is not verified in the editor: `verify! {function}` runs \
    it here; `lake test` runs every item"

/-- What a module's theorems see opened, as its authored proofs do. -/
def authoredOpens : List OpenDecl := [.simple `VerexIR []]

/-- The module a proof file's header selected: in a proof file, `verify`
items follow `verex proofs where`. -/
private initialize selectedExtension : EnvExtension (Option Name) ←
  registerEnvExtension (pure none)

/-- Select the module whose proofs the file being elaborated holds. -/
def selectProofModule (environment : Environment) (name : Name) : Environment :=
  selectedExtension.setState environment (some name)

def selectedProofModule? (environment : Environment) : Option Name :=
  selectedExtension.getState environment

/-- Elaborate `act` with `Elab.async`: declarations are checked by the
kernel in its chain, behind the proofs running, and the command waits for
none of them; a target's own proof tasks are what its conclusion waits for. -/
private def withAsyncElaboration (act : CommandElabM α) (async := true) : CommandElabM α :=
  withScope (fun scope => { scope with opts := Elab.async.set scope.opts async }) act

@[command_elab verexVerifyCommand]
def elabVerexVerify : CommandElab := fun stx => Perf.withPhase .certification do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  if (selectedProofModule? (← getEnv)).isSome then
    throwErrorAt stx "in a proof file, `verify` items follow `verex proofs where`"
  let segments := pathSegments pathSyntax
  -- A module path may lead with a Move address alias; a namespace
  -- registered at the path as written takes precedence.
  let canonical := VerexSL.canonicalMovePath (← getEnv) segments.pop
  let segments := if segments.size > 1 &&
      (VerexSL.registeredUnit? (← getEnv) (pathName segments.pop)).isNone &&
      (VerexSL.registeredUnit? (← getEnv) (pathName canonical)).isSome then
    canonical.push segments.back!
  else segments
  let script ← scriptOfOptional (stx[2]?.getD .missing)
  let function := segments.back!
  if skipVerify (← getOptions) && !demandedVerify stx then
    skippedInEditor pathSyntax function
    return
  if segments.size > 1 then
    return ← withAsyncElaboration do
      (← verifyFunction pathSyntax segments.pop function script).finish
  let owners := (VerexSL.registeredUnits (← getEnv)).filter fun (_, unit) =>
    (findFunction? unit function).isSome
  match owners with
  | [(owner, _)] =>
      let ownerSegments := owner.components.toArray.map (·.toString (escape := false))
      withAsyncElaboration do
        (← verifyFunction pathSyntax ownerSegments function script).finish
  | [] => throwErrorAt pathSyntax s!"no Verex module declares a function `{function}`"
  | _ =>
      let names := owners.map fun (owner, _) =>
        "::".intercalate (owner.components.map (·.toString (escape := false)))
      throwErrorAt pathSyntax
        s!"`{function}` is declared by several Verex modules ({", ".intercalate names}); \
          qualify it with its module path"

/-- Print the closure sites of a registered unit's link set, grouped by
function type into mutual blocks (`closureSites`). -/
syntax (name := verexClosureSitesCommand) "#verex_closure_sites" verexPath : command

@[command_elab verexClosureSitesCommand]
def elabVerexClosureSites : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let name := pathName (pathSegments pathSyntax)
  let some unit := VerexSL.registeredUnit? (← getEnv) name
    | throwErrorAt pathSyntax s!"unknown Verex namespace `{name}`"
  logInfoAt stx (displayClosureSites unit (groupClosureSites (closureSites unit)))

/-- Declare a registered unit's function values and print them, a
constructor per closure site (`ensureFunctionCarriers`). -/
syntax (name := verexFunctionValuesCommand) "#verex_function_values" verexPath : command

@[command_elab verexFunctionValuesCommand]
def elabVerexFunctionValues : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let name := pathName (pathSegments pathSyntax)
  let some unit := VerexSL.registeredUnit? (← getEnv) name
    | throwErrorAt pathSyntax s!"unknown Verex namespace `{name}`"
  discard <| ensureFunctionCarriers (pathSegments pathSyntax) unit
  let sites := closureSites unit
  logInfoAt stx (if sites.isEmpty then "no closure sites" else displayFunctionValues unit sites)

@[command_elab VerexSL.Contract.verexUnitCommand]
def elabVerexUnit : CommandElab := fun stx => do
  let some pathSyntax := stx[1]? | throwErrorAt stx "expected a path"
  let segments := pathSegments pathSyntax
  let some unit := VerexSL.registeredUnit? (← getEnv) (pathName segments)
    | throwErrorAt stx s!"unknown Verex namespace `{pathName segments}`"
  -- A hand-written proof needs the same quoted unit, and its function
  -- values, the generated script uses, along with the typed twins of its
  -- declarations.
  discard <| ensureUnitInstance segments unit
  discard <| ensureUnitDefinitions segments unit

/-- A function a module verifies: its name, the syntax errors are reported
at, and its authored proof, if any. -/
private structure VerificationTarget where
  function : String
  reference : Syntax
  script : Option AuthoredScript
  /-- A `verify` item, not an automatic target: its skip in the editor is
  reported. -/
  item : Bool
  /-- A `verify!` item: verified in the editor as well. -/
  demanded : Bool
  /-- The file of `reference` and `script`, where not the command's own. -/
  file : Option ParsedFile := none
  deriving Inhabited

private def VerificationTarget.authored? (target : VerificationTarget) : Option AuthoredScript :=
  target.script.map ({ · with file := target.file })

/-- The specified functions a function's proof uses through their verified
theorems: its callees with a specification, found through the unspecified
callees it inlines, and the specified targets of its closures. Natives and
callees returning a reference are not among them. -/
private def verifiedCallees (unit : ValidatedUnit)
    (function : FunctionHandle) : Array FunctionHandle := Id.run do
  let callees (handle : FunctionHandle) : Array FunctionHandle :=
    match compileFunction unit handle with
    | .ok compiled => compiled.body.callees #[]
    | .error _ => #[]
  let targetsOfClosures := closureTargets unit
  let specified (target : FunctionHandle) : Bool :=
    match unit.namespaces[target.namespaceId.index]? with
    | some targetNs => (targetNs.functions[target.functionId.index]?).any (·.contract.loc.isSome)
    | none => false
  let contractTargets (callee : FunctionHandle) : Array FunctionHandle :=
    match unit.namespaces[callee.namespaceId.index]? with
    | some calleeNs => match calleeNs.functions[callee.functionId.index]? with
        | some calleeDeclaration =>
            (contractClosureTargets unit callee.namespaceId calleeNs calleeDeclaration).filter
              specified
        | none => #[]
    | none => #[]
  let mut found := contractTargets function
  let mut visited := #[function]
  let mut worklist := callees function
  while let some callee := worklist.back? do
    worklist := worklist.pop
    if visited.contains callee then continue
    visited := visited.push callee
    let some ns := unit.namespaces[callee.namespaceId.index]? | continue
    let some declaration := ns.functions[callee.functionId.index]? | continue
    if declaration.body == .absent then
      found := found ++ (contractTargets callee).filter (!found.contains ·)
      continue
    if usedThroughContract unit callee ns declaration then
      -- A callee seen through its interface is assumed, not used by its theorem.
      unless hasInterfaceView declaration do found := found.push callee
      found := found ++ (contractTargets callee).filter (!found.contains ·)
    else
      -- What `ensures_of` and `aborts_of` of a closure entail is its
      -- target's theorem.
      if targetsOfClosures.contains callee && declaration.contract.loc.isSome then
        found := found.push callee
      worklist := worklist ++ callees callee
  return found

/-- A target in the order a module verifies them: after the module's targets
among the specified callees its proof uses, so a caller finds its callees'
theorems whatever the order of their declarations. -/
private structure ScheduledTarget where
  target : VerificationTarget
  /-- The targets, by function, ordered before this one whose theorems its
  proof may use: they conclude before it begins. -/
  dependencies : Array String
  /-- The targets this one was reached from, each a callee of the previous:
  a proof reaching one of them again would depend on its own verification. -/
  ancestors : Array String
  deriving Inhabited

/-- Order `target` after its callees, as the verification enters them: a
target already entered, on a cycle through it included, is not entered
again. A callee of another module used through its contract is verified in
this unit too: a theorem about its own unit does not carry over. -/
private partial def scheduleTarget (unit : ValidatedUnit) (targets : Array VerificationTarget)
    (visited : IO.Ref (Array String)) (order : IO.Ref (Array ScheduledTarget))
    (ancestors : Array String) (target : VerificationTarget) : CommandElabM Unit := do
  if (← visited.get).contains target.function then return
  visited.modify (·.push target.function)
  let mut dependencies : Array String := #[]
  if let some (namespaceIndex, _, functionIndex, _) := findFunction? unit target.function then
    let inner := ancestors.push target.function
    for callee in verifiedCallees unit ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩ do
      let key := functionKey unit callee
      if callee.namespaceId.index == namespaceIndex then
        if let some calleeTarget := targets.find? (·.function == key) then
          scheduleTarget unit targets visited order inner calleeTarget
      else
        let some (_, ns, _, declaration) := findFunction? unit key | continue
        unless automaticVerificationDisabled unit callee.namespaceId ns declaration do
          scheduleTarget unit targets visited order inner
            ⟨key, target.reference, none, false, false, target.file⟩
      -- A callee ordered before the target concludes before it; one still
      -- being entered, on a cycle through the target, does not.
      if (← order.get).any (·.target.function == key) then
        dependencies := dependencies.push key
  order.modify (·.push { target, dependencies, ancestors })

/-- Run `act`, taking the messages it logs out of the log: a target's are
reported in its turn. -/
private def capturingMessages (act : CommandElabM α) : CommandElabM (α × Array Message) := do
  let before := (← get).messages.unreported.size
  let result ← act
  let logged := (← get).messages.unreported.toArray
  modify fun state => { state with
    messages := { state.messages with unreported := (logged.extract 0 before).toPArray' } }
  return (result, logged.extract before logged.size)

/-- Whether an error among `messages` is a proof that ran out of its budget. -/
private def anyOverBudget (messages : Array Message) : Bool :=
  messages.any fun message =>
    message.severity == .error && message.data.hasTag (· == `runtime.maxHeartbeats)

/-- How a target began: with its proofs' tasks and the members it covers,
not at all, or not for an error its verification raised. -/
private inductive Begun where
  | proving (pending : PendingVerification) (covers : Array String)
  | skipped
  | failed

/-- Verify the module's targets, `verex.jobs` proving at once. Every target
is prepared first, in order: the definitions and certificates its proof
needs are elaborated and kernel-checked while no proof runs. Then a target
begins once the targets whose theorems its proof may use concluded, in the
order `scheduleTarget` gives; its messages are reported in that order, as
those of targets verified one after another. -/
private def verifyScheduled (unit : ValidatedUnit) (segments : Array String)
    (opens : Array (TSyntax ``Lean.Parser.Command.openDecl))
    (targets : Array VerificationTarget) : CommandElabM Unit := do
  let visited ← IO.mkRef (#[] : Array String)
  let ordered ← IO.mkRef (#[] : Array ScheduledTarget)
  for target in targets do scheduleTarget unit targets visited ordered #[] target
  let order ← ordered.get
  let jobs := match verex.jobs.get (← getOptions) with
    | 0 => max 1 ((System.Platform.Internal.getHardwareConcurrency ()).toNat / 2)
    | jobs => jobs
  let moduleNamespace := pathName segments
  -- An authored proof sees the module's theorems by their short names.
  let openModule (scope : Scope) := { scope with
    openDecls := .simple moduleNamespace [] :: scope.openDecls }
  let inModule {α : Type} (act : CommandElabM α) : CommandElabM α :=
    if targets.any (·.script.isSome) then withScope openModule act else act
  let scripts (name : String) := (targets.find? (·.function == name)).bind (·.authored?)
  -- The targets concluded, verified or not, with the members of the cycles
  -- verified with them; and the members a target still proving covers.
  let concluded ← IO.mkRef (#[] : Array String)
  let covered ← IO.mkRef (#[] : Array String)
  -- Begin a target: its certificates, and its proofs' tasks; with
  -- `prepare`, the certificates alone. A member of a cycle verified from
  -- another member is not entered again. The editor explains a skip at a
  -- `verify` item of its document; an automatic target has no item to mark
  -- `verify!`.
  let begin (scheduled : ScheduledTarget) (prepare : Bool) : CommandElabM Begun := do
    let target := scheduled.target
    if (← covered.get).contains target.function then return .skipped
    if skipVerify (← getOptions) && !target.demanded then
      if target.item && ParsedFile.isDocument target.file then
        inParsedFile target.file (skippedInEditor target.reference target.function)
      return .skipped
    let covers ← IO.mkRef (#[] : Array String)
    let covering (names : Array String) : CommandElabM Unit := do
      covered.modify (· ++ names)
      covers.modify (· ++ names)
    modifyEnv (inProgressTargets.setState · (scheduled.ancestors.push target.function))
    try
      let pending ← inParsedFile target.file <| inModule <|
        verifyFunction target.reference segments target.function target.authored? scripts
          covering opens prepare
      return .proving pending (← covers.get)
    catch error =>
      -- A failure without a position of its own, such as a translation's, is
      -- reported at the target and names it.
      inParsedFile target.file do
        match error with
        | .error ref message =>
            if ref.getPos?.isSome then logException error
            else logErrorAt target.reference m!"verex verification of `{target.function}` failed: \
              {message}"
        | _ => logException error
      return .failed
    finally
      modifyEnv (inProgressTargets.setState · #[])
  let conclude (scheduled : ScheduledTarget) (pending : PendingVerification) :
      CommandElabM Unit := do
    let target := scheduled.target
    inParsedFile target.file <| inModule do
      try pending.conclude
      catch error =>
        match error with
        | .error ref message =>
            if ref.getPos?.isSome then logException error
            else logErrorAt target.reference m!"verex verification of `{target.function}` failed: \
              {message}"
        | _ => logException error
  -- The targets are prepared as the command elaborates, their certificates
  -- checked at their sites; the proofs and what follows them elaborate
  -- asynchronously.
  let asyncBefore := Elab.async.get (← getOptions)
  withAsyncElaboration do
    let mut messages : Array (Array Message) := order.map fun _ => #[]
    let mut begun : Array Bool := order.map fun _ => false
    -- The targets verified: neither covered nor skipped, their outcome measured.
    let mut attempted : Array Bool := order.map fun _ => false
    -- Prepare every target, in order; one whose preparation raises an error
    -- is concluded with its messages, and not begun. An error a definition
    -- recovered from is reported with the target's, whose proof goes on as
    -- the definition stands.
    for index in [0:order.size] do
      let (outcome, logged) ← capturingMessages <|
        withScope (fun scope => { scope with opts := Elab.async.set scope.opts asyncBefore })
          (begin order[index]! true)
      messages := messages.modify index (· ++ logged)
      if outcome matches .failed then
        begun := begun.set! index true
        attempted := attempted.set! index true
        concluded.modify (·.push order[index]!.target.function)
    covered.set #[]
    let mut running : Array (Nat × PendingVerification × Array String) := #[]
    repeat
      for index in [0:order.size] do
        if begun[index]! || running.size ≥ jobs then continue
        let scheduled := order[index]!
        let function := scheduled.target.function
        let done := (← concluded.get)
        -- A member covered by a cycle still proving concludes with it.
        unless scheduled.dependencies.all done.contains &&
            (done.contains function || !(← covered.get).contains function) do
          continue
        begun := begun.set! index true
        let (outcome, logged) ← capturingMessages (begin scheduled false)
        messages := messages.modify index (· ++ logged)
        match outcome with
        | .proving pending covers =>
            attempted := attempted.set! index true
            running := running.push (index, pending, covers)
        | .failed =>
            attempted := attempted.set! index true
            concluded.modify (·.push function)
        | .skipped => concluded.modify (·.push function)
      if running.isEmpty then
        -- Every target begins once its dependencies concluded, which they do
        -- in their turn; one left is begun regardless.
        match begun.findIdx? (!·) with
        | some index =>
            begun := begun.set! index true
            let (outcome, logged) ← capturingMessages (begin order[index]! false)
            messages := messages.modify index (· ++ logged)
            match outcome with
            | .proving pending covers =>
                attempted := attempted.set! index true
                running := running.push (index, pending, covers)
            | .failed =>
                attempted := attempted.set! index true
                concluded.modify (·.push order[index]!.target.function)
            | .skipped => concluded.modify (·.push order[index]!.target.function)
            continue
        | none => break
      -- The first target whose proofs ended concludes.
      let ended ← running.toList.mapM fun (index, pending, _) =>
        BaseIO.mapTasks (fun _ => pure index) (pending.tasks.toList.map (·.task))
      let position ← match ended with
        | [] => pure 0
        | first :: rest => Perf.withPhase .verification do
            let index ← IO.waitAny (first :: rest)
            pure ((running.findIdx? (·.1 == index)).getD 0)
      let some (index, pending, covers) := running[position]? | break
      running := running.eraseIdx! position
      let ((), logged) ← capturingMessages (conclude order[index]! pending)
      messages := messages.modify index (· ++ logged)
      concluded.modify (· ++ covers.push order[index]!.target.function)
      stageLog s!"{order[index]!.target.function}: concluded"
    -- The targets' messages, in their order.
    let measuring ← Perf.measuring.get
    for scheduled in order, logged in messages, verified in attempted do
      let target := scheduled.target
      inParsedFile target.file do
        let since ← unreportedCount
        for message in logged do
          modify fun state => { state with messages := state.messages.add message }
        if measuring && verified then
          let errors := (logged.filter (·.severity == .error)).size
          Perf.outcomes.modify (·.push {
            target := s!"{moduleNamespace}::{target.function}"
            status := if errors == 0 then "verified" else if anyOverBudget logged then "timeout"
              else "rejected"
            errors })
        if ParsedFile.isDocument target.file then relocateForeignMessages target.reference since

/-- Elaborate `commands` in the module namespace `moduleNamespace`, taken
from the root: the namespace of the module's generated definitions. -/
private def withModuleNamespace (moduleNamespace : Name) (commands : CommandElabM Unit) :
    CommandElabM Unit := do
  modify fun state => { state with
    env := state.env.registerNamespace moduleNamespace
    scopes := { state.scopes.head! with header := "", currNamespace := moduleNamespace } ::
      state.scopes }
  pushScope
  activateScoped moduleNamespace
  try commands
  finally
    modify fun state => { state with scopes := state.scopes.drop 1 }
    popScope

/-- The function items of a namespace command, in source order. -/
private partial def functionItems (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if child.isOfKind ``verexFunctionItem then found.push child
    else if child.isOfKind ``verexTheoremItem then found
    else found ++ functionItems child) #[]

/-- The first identifier of a syntax tree. -/
private partial def firstIdentifier? (stx : Syntax) : Option Syntax :=
  if stx.isIdent then some stx else stx.getArgs.findSome? firstIdentifier?

/-- Whether a function's parameters, through a reference, its results, its
locals, or the values it constructs carry data invariants: what the Move
Prover checks of every function that handles such values. -/
private def handlesDataInvariants (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  let direct (typeId : VerexIR.TypeId) : Bool :=
    (ns.tables.types[typeId.index]?).any (Contract.hasDataInvariant unit)
  let carries (typeId : VerexIR.TypeId) : Bool :=
    match ns.tables.types[typeId.index]? with
    | some (.reference reference) => direct reference.referent
    | some _ => direct typeId
    | none => false
  let constructs : Bool := match declaration.body with
    | .structured root => (bodyExpressionIds ns root).any fun id =>
        match ns.expressions[id.index]? with
        | some { kind := .operation (.call (.constructor _ _)) _ _ _, typeId, .. } => direct typeId
        | _ => false
    | _ => false
  declaration.signature.parameters.any (carries ·.typeUse.typeId) ||
    declaration.signature.results.any (carries ·.typeId) ||
    declaration.locals.any (direct ·.type.typeId) || constructs

/-- Whether a function's body states a condition of a kind `accept` admits. -/
private def bodyStates (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (accept : VerexIR.ConditionKind → Bool) : Bool :=
  match declaration.body with
  | .structured root => !(bodySpecifications ns root accept).isEmpty
  | _ => false

/-- The registered modules whose invariants concern a module's unit, which
it links: those whose invariants read memory its functions reach, and those
registered in this file whose functions write memory its own invariants
read, whose functions it verifies against them. Iterated to the closure: a
linked writer's memory reaches further invariants. -/
private def invariantRelatedModules (environment : Environment) (unit : ValidatedUnit) :
    Array (Array String) := Id.run do
  let modulePath (module : ValidatedUnit) : Option (Array String) := do
    let ns ← module.namespaces[0]?
    module.tables.namespaces[ns.identity.index]? |>.map (·.segments)
  let some ownPath := modulePath unit | return #[]
  let modules := (registeredUnits environment).filterMap fun (name, module) => do
    let path ← modulePath module
    guard (pathName path == name && path != ownPath)
    pure (path, module)
  let inFile := (moduleUnits environment).map (·.1)
  let ownInvariants := Contract.namespaceInvariantMemory unit ⟨0⟩
  let mut reached := Contract.namespaceFunctionMemory unit ⟨0⟩ false
  let mut related : Array (Array String) := #[]
  for _ in [0:modules.length + 1] do
    let mut changed := false
    for (path, module) in modules do
      if related.contains path then continue
      let readsReached := (Contract.namespaceInvariantMemory module ⟨0⟩).any reached.contains
      let writesOwn := inFile.contains (pathName path) &&
        (Contract.namespaceFunctionMemory module ⟨0⟩ true).any ownInvariants.contains
      unless readsReached || writesOwn do continue
      related := related.push path
      changed := true
      if writesOwn then
        for resource in Contract.namespaceFunctionMemory module ⟨0⟩ false do
          unless reached.contains resource do reached := reached.push resource
    unless changed do break
  return related

/-- The modules whose verification a module command defers: none, all, or
those whose `proof_file` pragma names one of the files. -/
inductive Deferral where
  | none
  | all
  | proofFiles (files : Array String)
  deriving Inhabited

/-- A module registered with its verification deferred: its command, its
Move addresses canonical, the file the command was parsed from, and its
`proof_file` pragma, the file whose `verex proofs where` verifies it. -/
structure DeferredModule where
  name : Name
  command : Syntax
  file : ParsedFile
  proofFile : Option String
  /-- Whether a `verex proofs where` has run its verification. -/
  verified : Bool := false

private initialize deferralExtension : EnvExtension Deferral ←
  registerEnvExtension (pure .none)

private initialize deferredExtension : EnvExtension (NameMap DeferredModule) ←
  registerEnvExtension (pure {})

/-- Which module commands of what is elaborated next defer their
verification. -/
def setDeferral (environment : Environment) (deferral : Deferral) : Environment :=
  deferralExtension.setState environment deferral

def getDeferral (environment : Environment) : Deferral :=
  deferralExtension.getState environment

/-- The modules registered with their verification deferred, by namespace. -/
def deferredModules (environment : Environment) : NameMap DeferredModule :=
  deferredExtension.getState environment

/-- Record that a deferred module's verification has run. -/
def markVerified (environment : Environment) (name : Name) : Environment :=
  deferredExtension.modifyState environment fun modules =>
    match modules.find? name with
    | some module => modules.insert name { module with verified := true }
    | none => modules

/-- The lemma items of a proof file's items, not descending into its Lean
items. -/
private partial def proofLemmaItems (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if child.isOfKind ``verexLemmaProofItem then found.push child
    else if child.isOfKind ``verexTheoremItem then found
    else found ++ proofLemmaItems child) #[]

/-- The items of a module and of its proof file's `verex proofs where`
command, each with the file it was parsed from where not the command's own. -/
private structure ItemSource where
  stx : Syntax
  file : Option ParsedFile := none

/-- The lemmas of a module's intrinsic maps (`mapLemmas`) under names composed
from the module's specification functions: for an owner binding `spec_get`,
`spec_set`, `spec_remove`, `spec_len`, `spec_contains_key`, `spec_key_at`
and `spec_rank`, the theorems `lemma_get_set`, `lemma_len_remove`,
`lemma_contains_key_key_at`, `lemma_key_at_eq_iff`, …
(`doc/designs/proof-strategy.md`, P2), each the lemma of typed maps it names
(`VerexIR.SpecMap.valueAt_update`, …). -/
private def ensureMapLemmas (segments : Array String) (unit : ValidatedUnit) : CommandElabM Unit := do
  let some ns := unit.namespaces[0]? | return
  for (lemmaName, lemmaTheorem) in namespaceMapLemmaNames ns do
    -- The module's artifacts are absolute names.
    let name := Name.str (pathName segments) lemmaName
    if (← getEnv).contains name then continue
    let info ← getConstInfo lemmaTheorem
    let value := mkConst lemmaTheorem (info.levelParams.map mkLevelParam)
    let alias : TheoremVal :=
      { name := name, levelParams := info.levelParams, type := info.type, value := value }
    liftTermElabM <| addDecl (.thmDecl alias)

/-- Verify a registered module from the items of `sources`, its command's
then its proof file's: its theorems in its namespace, then, with that
namespace open, its lemmas, its `verify` items, and its specified functions
without one, in source order. -/
private def verifyModuleItems (pathSyntax : Syntax) (sources : Array ItemSource) :
    CommandElabM Unit := do
  let some module := sources[0]? | return
  let segments := pathSegments pathSyntax
  let moduleNamespace := pathName segments
  -- The module's theorems and authored proofs see `VerexIR` and the
  -- namespaces of its `open` items opened.
  let opens := sources.flatMap fun source =>
    (VerexSL.openItems source.stx).map fun item =>
      ((⟨item[1]!⟩ : TSyntax ``Lean.Parser.Command.openDecl), source.file)
  let elaborated ← opens.toList.flatMapM fun (decl, file) => inParsedFile file (elabOpenDecl decl)
  let theoremOpens := authoredOpens ++ elaborated
  let theorems := sources.flatMap fun source =>
    (VerexSL.theoremItems source.stx).map (·, source.file)
  unless theorems.isEmpty do
    withModuleNamespace moduleNamespace <| Perf.withPhase .verification do
      modifyScope fun scope => { scope with openDecls := theoremOpens ++ scope.openDecls }
      for (theoremCommand, file) in theorems do
        -- Authored proofs are native (`doc/designs/native-verification.md`),
        -- elaborated at once so that their names are seen.
        inParsedFile file <| VerexIR.Proofs.Denote.reportAuthoredRuntimeNames <|
          withAsyncElaboration (elabCommand theoremCommand) (async := false)
  let opens := opens.map (·.1)
  let some unit := VerexSL.registeredUnit? (← getEnv) moduleNamespace | return
  -- The module's lemmas, and those its functions apply, before its functions.
  let ownLemmas := (unit.namespaces.toList.zipIdx.filter fun (ns, _) =>
      (unit.tables.namespaces[ns.identity.index]?.map (·.segments)) == some segments).flatMap
    fun (ns, index) => ns.lemmas.toList.map fun lemma =>
      ({ namespaceId := ⟨index⟩, name := lemma.name } : VerexIR.QualifiedRef)
  let appliedByFunctions := unit.namespaces.toList.flatMap fun ns =>
    ns.functions.toList.flatMap fun declaration => (functionLemmas ns declaration).toList
  let roots := (ownLemmas ++ appliedByFunctions).foldl (init := #[]) fun roots lemma =>
    if roots.contains lemma then roots else roots.push lemma
  let lemmaItems := (lemmaItemsOf module.stx).filterMap fun item => do
    let identifier ← item.getArgs.find? (·.isIdent)
    pure (identifier.getId.toString (escape := false), identifier)
  inParsedFile module.file (verifyLemmas unit segments roots lemmaItems pathSyntax)
  let items := sources.flatMap fun source =>
    (VerexSL.verificationItems source.stx).map (·, source.file)
  let explicit := items.filterMap fun (item, _) =>
    if item.isOfKind ``verexVerifyItem then item[1]?.map (·.getId.toString (escape := false))
    else none
  -- The explicit `verify` items, and the specified functions with a body
  -- that no explicit item names and whose specification does not set
  -- `pragma verify = false`, in source order.
  let targets : Array VerificationTarget ← items.filterMapM fun (item, file) => do
    if item.isOfKind ``verexVerifyItem then
      let some identifier := item[1]? | return none
      return some ⟨identifier.getId.toString (escape := false), identifier,
        ← scriptOfOptional (item[2]?.getD .missing), true, demandedVerify item, file⟩
    return (do
      let identifier ← item.getArgs.find? (·.isIdent)
      let function := identifier.getId.toString (escape := false)
      if explicit.contains function then none
      let (namespaceIndex, ns, _, declaration) ← findFunction? unit function
      if declaration.body == .absent || statesNothing declaration.contract ||
          automaticVerificationDisabled unit ⟨namespaceIndex⟩ ns declaration then none
      some ⟨function, identifier, none, false, false, file⟩ : Option VerificationTarget)
  -- A function without a specification is verified against its loop
  -- invariants, its in-body assertions, the preconditions of the functions
  -- it calls, the data invariants of the values it takes, returns,
  -- constructs, and mutates, and the invariants of the memory it writes. An
  -- intrinsic map role stands for the map model, as a native does: its body
  -- is not verified.
  let named := targets.map (·.function)
  let targets := targets ++ (functionItems module.stx).filterMap fun item => do
    let identifier ← firstIdentifier? item[4]
    let function := identifier.getId.toString (escape := false)
    if named.contains function then none
    let (namespaceIndex, ns, functionIndex, declaration) ← findFunction? unit function
    if declaration.body == .absent ||
        (declaration.contract.loc.isSome && !statesNothing declaration.contract) ||
        automaticVerificationDisabled unit ⟨namespaceIndex⟩ ns declaration ||
        (Contract.mapRoleOf? unit ⟨namespaceIndex⟩ ns declaration).isSome ||
        !(handlesDataInvariants unit ns declaration ||
          bodyStates ns declaration (· matches .loopInvariant | .assertion | .apply) ||
          callsRequiring unit ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩ ||
          Contract.owesMemoryInvariants unit ⟨namespaceIndex⟩ declaration) then
      none
    some ⟨function, identifier, none, false, false, module.file⟩
  -- A function of a module this file registered before, which writes memory
  -- an invariant of this module reads, is verified against it here.
  let inFile := (moduleUnits (← getEnv)).map (·.1)
  let writers : Array VerificationTarget :=
    (unit.namespaces.toList.zipIdx.drop 1).toArray.flatMap fun (ns, index) =>
      let path := (unit.tables.namespaces[ns.identity.index]?.map (·.segments)).getD #[]
      if !inFile.contains (pathName path) then #[] else
      ns.functions.zipIdx.filterMap fun (declaration, functionIndex) =>
        if declaration.body == .absent ||
            automaticVerificationDisabled unit ⟨index⟩ ns declaration ||
            !Contract.owesInvariantsOf unit ⟨0⟩ ⟨index⟩ declaration then none
        else some ⟨functionKey unit ⟨⟨index⟩, ⟨functionIndex⟩⟩, pathSyntax, none, false, false,
          module.file⟩
  let targets := targets ++ writers
  -- The lemma items of the proof file; a module holds `spec lemma`s.
  let proofLemmas ← sources.toList.zipIdx.toArray.flatMapM fun (source, position) => do
    let found := proofLemmaItems source.stx
    if position == 0 then
      for item in found do
        logErrorAt item "a `lemma` item belongs to a proof file; a module states `spec lemma`"
      return #[]
    let fileName ← match source.file with
      | some file => pure file.name
      | none => getFileName
    return found.map (·, fileName)
  if targets.isEmpty && proofLemmas.isEmpty then return
  -- The expressions of the items' steps, lowered once for the module.
  let stepped ← items.filterMapM fun (item, file) => do
    unless item.isOfKind ``verexVerifyItem do return none
    let some identifier := item[1]? | return none
    let optional := item[2]?.getD .missing
    let #[_, block] := optional.getArgs | return none
    let fileName ← match file with
      | some file => pure file.name
      | none => getFileName
    return some (item, identifier.getId.toString (escape := false), block, fileName)
  lowerStepExpressions invariantRelatedModules unit module.stx module.file stepped proofLemmas
  if targets.isEmpty then return
  stageLog s!"module {moduleNamespace}: verification start"
  verifyScheduled unit segments opens targets
  stageLog s!"module {moduleNamespace}: verification done"

/-- The path of a module command. -/
private def modulePath (stx : Syntax) : CommandElabM Syntax := do
  let some pathSyntax := stx.getArgs.find? (·.isOfKind ``verexPathSyntax)
    | throwErrorAt stx "a Verex namespace requires a path"
  return pathSyntax

/-- Verify a deferred module from its command's items and those of `proofs`,
its proof file's `verex proofs where` command, elaborated in this command's
file. -/
def verifyDeferredModule (module : DeferredModule) (proofs : Syntax) : CommandElabM Unit :=
    Perf.withPhase .certification do
  let file : ParsedFile := module.file
  let document : ParsedFile := { name := ← getFileName, map := ← getFileMap, document := true
                                 infoTrees := (← getInfoState).enabled }
  let pathSyntax ← inParsedFile file (modulePath module.command)
  verifyModuleItems pathSyntax #[{ stx := module.command, file }, { stx := proofs, file := document }]
  modifyEnv (markVerified · module.name)

/-- Elaborate a Verex namespace command and register its module, then verify
it (`verifyModuleItems`), unless its verification is deferred to its proof
file. This registration shadows the plain namespace elaborator. -/
@[command_elab verexNamespaceCommand, command_elab verexMoveModuleCommand,
  command_elab verexRustNamespaceCommand]
def elaborateNamespaceWithVerification : CommandElab := fun stx =>
    Perf.withPhase .certification do
  VerexSL.elaborateNamespaceWith invariantRelatedModules stx
  stageLog "module elaborated"
  -- Its module is registered at its canonical path.
  let stx ← match VerexSL.canonicalMoveCommand (← getEnv) stx with
    | .ok (canonical, _) => pure canonical
    | .error (location, message) => throwErrorAt location message
  let pathSyntax ← modulePath stx
  -- A recursive specification function is a definition of the module.
  let segments := pathSegments pathSyntax
  let some unit := VerexSL.registeredUnit? (← getEnv) (pathName segments) | return
  ensureMapLemmas segments unit
  unless (recursiveSpecFunctions unit).isEmpty do
    liftTermElabM (ensureSpecFunctionDefinitions unit)
  let proofFile := unit.namespaces[0]? >>= proofFile?
  let deferred := match getDeferral (← getEnv), proofFile with
    | .all, _ => true
    | .proofFiles files, some file => files.contains file
    | _, _ => false
  if deferred then
    let name := pathName segments
    let module : DeferredModule :=
      { name, command := stx, file := { name := ← getFileName, map := ← getFileMap }, proofFile }
    modifyEnv (deferredExtension.modifyState · (·.insert name module))
    return
  verifyModuleItems pathSyntax #[{ stx }]


/-- The `verify` item or command at `cursor` within `stx`. -/
private partial def verifyAt (cursor : String.Pos.Raw) (stx : Syntax) : Option Syntax :=
  if (stx.isOfKind ``verexVerifyItem || stx.isOfKind ``verexVerifyCommand) &&
      (stx.getPos? true).any (· ≤ cursor) && (stx.getTailPos? true).any (cursor ≤ ·) then
    some stx
  else stx.getArgs.findSome? (verifyAt cursor)

open Server CodeAction RequestM in
/-- In the editor, "Verify `f` here" marks the `verify` item at the cursor
`verify!`, which runs where `verex.skip_verify` skips the others; "Skip `f`"
unmarks it. -/
@[command_code_action verexMoveModuleCommand verexNamespaceCommand verexRustNamespaceCommand
  verexVerifyCommand]
def verifyHereCodeAction : CommandCodeAction := fun params _ _ node => do
  let .node (.ofCommandInfo info) _ := node | return #[]
  let doc ← readDoc
  let cursor := doc.meta.text.lspPosToUtf8Pos params.range.start
  let some item := verifyAt cursor info.stx | return #[]
  let (some start, some tail) := (item[0].getPos? true, item[0].getTailPos? true) | return #[]
  let function := (item[1].reprint.getD "").trim
  let (title, start, newText) := if demandedVerify item then
      (s!"Skip `{function}` in the editor", start, "verify")
    else (s!"Verify `{function}` here", tail, "!")
  return #[{ eager := {
    title, kind? := "quickfix"
    edit? := some <| .ofTextEdit doc.versionedIdentifier
      { range := doc.meta.text.utf8RangeToLspRange ⟨start, tail⟩, newText } } }]

end VerexSL.Verify
