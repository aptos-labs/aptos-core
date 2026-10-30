-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.FunctionValues
import VerexIR.Proofs.Denote.Authored

/-!
# Verification: targets, cycles, and lemmas

The verification of a target, of the members of a cycle of calls, and of
specification lemmas: theorem statements, the solver's tactics, the
artifact audit, and the transport to the big-step meaning.
-/

namespace VerexSL.Verify

open Lean Meta Elab Command
open VerexIR (RuntimeValue FunctionHandle NamespaceId FunctionId)
open VerexIR.Validation (ValidatedUnit ValidatedNamespace)
open VerexIR.Proofs.Denote
open VerexSL.Contract

/-- What a failed verification asks for: an authored proof did not close
its obligations; an automatic one asks for a proof, including when its
bounded attempt ran out. -/
private def failureMessage (ns : ValidatedNamespace) (function : String) (authored : Bool)
    (overBudget : Bool) (budget : Nat) (leaves : Array String) : MessageData :=
  if authored then
    m!"verex verification failed: the proof of `{function}` does not establish its \
      specification"
  else
    let attempt := if overBudget then
        m!"the automatic verification of `{function}` exceeded its budget of {budget} \
          maxHeartbeats"
      else m!"the automatic verification of `{function}` failed"
    m!"verex verification failed: {attempt}; {proofRequest ns function leaves}"

/-! ## Verification -/

/-- The declared field names of a constructor, or positions when unnamed. -/
private def fieldNames (unit : ValidatedUnit) (source : VerexIR.StructHandle)
    (variant : Option String) (count : Nat) : List String :=
  let declared : Option (List String) := do
    let (targetNs, fields) ← VerexIR.SemanticOperations.handleFields? unit source variant
    fields.toList.mapM fun (field : VerexIR.FieldDecl) =>
      (targetNs.tables.names[field.name.index]?).map (·.name)
  match declared with
  | some names => if names.length == count then names else (List.range count).map toString
  | none => (List.range count).map toString

/-- The names of a struct value's components, from `base`: its declared
fields, or, for an intrinsic map owner, its leading fields, its ghosts, and
the map itself, which takes the base name, as the Move source names the
whole value. -/
def componentNames (unit : ValidatedUnit) (source : VerexIR.StructHandle) (base : String)
    (count : Nat) : List String :=
  let owner : Option (List String) := do
    let layout ← VerexIR.Validation.ownerLayout? unit source.namespaceId source.structId
    let ns ← unit.namespaces[source.namespaceId.index]?
    let declaration ← ns.structs[source.structId]?
    let canonical := layout.canonical
    let leading := (if canonical.handle then [s!"{base}_handle"]
        else if canonical.dummy then [s!"{base}_dummy"] else []) ++
      (if canonical.length then [s!"{base}_length"] else [])
    let ghosts ← declaration.ghosts.toList.mapM fun ghost =>
      (ns.tables.names[ghost.name.index]?).map fun name => s!"{base}_{name.name}"
    let names := leading ++ ghosts ++ [base]
    guard (names.length == count)
    pure names
  owner.getD ((fieldNames unit source none count).map (s!"{base}_{·}"))

/-- The names of the payload components of a variant `variant` of an enum
value, from `base`: `base_variant_f` of a field `f`. -/
def variantComponentNames (unit : ValidatedUnit) (source : VerexIR.StructHandle) (base variant : String)
    (count : Nat) : List String :=
  (fieldNames unit source (some variant) count).map (s!"{base}_{variant}_{·}")

/-- The `rcases` pattern destructuring a native value of a type into its
scalar parts, named from `base`.  A variant value becomes one alternative
per variant; the empty tail of the variant sum closes its case. -/
private partial def carrierPattern (unit : ValidatedUnit) (base : String) : NTy → String
  | .tuple elements => rowPattern (elements.toList.zipIdx.map fun (σ, index) => (σ, s!"{base}_{index}"))
  | .struct source _ fields =>
      rowPattern (fields.toList.zip (componentNames unit source base fields.length))
  | .enum source _ names rows _ =>
      let alternatives := (names.zip rows.toList).map fun (name, fields) =>
        rowPattern (fields.toList.zip (variantComponentNames unit source base name fields.length))
      "(" ++ " | ".intercalate (alternatives ++ ["(⟨⟩ : Empty)"]) ++ ")"
  | .ref referent => s!"⟨{carrierPattern unit base referent}, «{base}_final»⟩"
  -- A binder is written escaped, as a source name may be a Lean keyword.
  | _ => s!"«{base}»"
where
  rowPattern (parts : List (NTy × String)) : String :=
    if parts.isEmpty then "_"
    else "⟨" ++ ", ".intercalate (parts.map fun (σ, name) => carrierPattern unit name σ) ++ ", _⟩"

/-- The reference type a type is, if it is one. -/
private def referenceOf? (unit : ValidatedUnit) (typeId : VerexIR.TypeId) :
    Option VerexIR.ReferenceType :=
  match unit.tables.types[typeId.index]? with
  | some (.reference reference) => some reference
  | _ => none

/-- The names of a function's results in its proof's scope, read at its
`ensures`: `result`, or with several results `result_k` the `k`-th (`result.k`
and `spec.result[k]` in a step) and `result` the first, as a clause reads
them. None beside a local of the name, and none for a returned reference. -/
def resultScope (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (scope : Array (String × String × Bool)) : Except String (Array (String × String × Bool)) := do
  let results := declaration.signature.results
  if results.isEmpty || scope.any (·.1.startsWith "result") then return #[]
  if results.any (fun (result : VerexIR.TypeUse) => (referenceOf? unit result.typeId).isSome) then
    return #[]
  let generics := declaration.signature.generics
  -- One result of a tuple type has its components, as several results do.
  let single : Option VerexIR.TypeId := if results.size == 1 then some results[0]!.typeId else none
  let resultTypes : Array VerexIR.TypeId := results.map fun (result : VerexIR.TypeUse) => result.typeId
  let components : Array VerexIR.TypeId :=
    match single.bind fun typeId => unit.tables.types[typeId.index]? with
    | some (.tuple elements) => elements
    | _ => resultTypes
  let types ← components.mapM fun typeId =>
    Print.specificationTypeText unit ns generics typeId
  if types.size == 1 then return #[("result", types[0]!, false)]
  return #[("result", types[0]!, false)] ++
    types.mapIdx fun k type => (s!"result_{k}", type, false)

/-- Whether a proof's syntax reads the function's result (`result`,
`result.k`, `spec.result[k]`): only then is the exit a binding site of it,
which names the returned value in the leaves. A dotted name (`result.t`) is
one identifier. -/
def readsResult (stx : Syntax) : Bool :=
  (stx.find? fun stx => (stx.isOfKind ``verexLocalExpr && stx[0].getId.getRoot == `result) ||
    stx.isOfKind ``verexSpecificationResultExpr).isSome

/-- The names in scope at every step of a function's proof, with their
specification types: each local with a source name, as `x`, `x'` and
`old_x`, and each callee's result as the leaf names it: in evaluation
order, the last call's result `g_result`, the earlier ones `old_g_result`,
`old_g_result_1`, …, a tuple's components `…_k`; and whether the value is a
mutable reference, its loan and current value, which a specification reads
through. A name given twice is an error. -/
def functionScope (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (withResult := false) :
    Except String (Array (String × String × Bool)) := do
  let generics := declaration.signature.generics
  let mutableReference (typeId : VerexIR.TypeId) : Bool :=
    (referenceOf? unit typeId).any (·.kind == .mutable)
  let mut scope := #[]
  for localDecl in declaration.locals do
    let name := localDecl.name
    unless (name.front.isAlpha || name.front == '_') && name.all (fun c => c.isAlphanum || c == '_') do
      continue
    if scope.any (·.1 == name) then continue
    let type ← Print.specificationTypeText unit ns generics localDecl.type.typeId
    let isReference := mutableReference localDecl.type.typeId
    scope := scope ++ #[(name, type, isReference), (name ++ "'", type, isReference),
      ("old_" ++ name, type, isReference)]
  if withResult then scope := scope ++ (← resultScope unit ns declaration scope)
  let .structured root := declaration.body | return scope
  -- The calls in evaluation order: a call's arguments before it.
  let mut calls : Array (String × VerexIR.TypeId) := #[]
  let mut work : List (VerexIR.ExprId × Bool) := [(root, false)]
  for _ in [0:2 * ns.expressions.size + 2] do
    let (id, expanded) :: rest := work | break
    work := rest
    let some expression := ns.expressions[id.index]? | continue
    if expanded then
      let .operation (.call (.function reference)) _ _ _ := expression.kind | continue
      let some callee := unit.tables.names[reference.name.index]? | continue
      calls := calls.push (callee.name, expression.typeId)
    else
      work := ((VerexIR.Validation.expressionChildren expression.kind).toList.map (·, false)) ++
        (id, true) :: work
  let callees := calls.foldl (init := #[]) fun found (callee, _) =>
    if found.contains callee then found else found.push callee
  for callee in callees do
    let ofCallee := calls.filter (·.1 == callee)
    for (_, typeId) in ofCallee, position in [0:ofCallee.size] do
      let name := if position + 1 == ofCallee.size then s!"{callee}_result"
        else if position == 0 then s!"old_{callee}_result"
        else s!"old_{callee}_result_{position}"
      let results ← match unit.tables.types[typeId.index]? with
        | some (.tuple elements) => elements.zipIdx.mapM fun (element, index) => do
            pure (s!"{name}_{index}", ← Print.specificationTypeText unit ns generics element,
              mutableReference element)
        | some .unit => pure #[]
        | _ => pure #[(name, ← Print.specificationTypeText unit ns generics typeId, mutableReference typeId)]
      for (resultName, type, isReference) in results do
        if scope.any (·.1 == resultName) then
          throw s!"`{resultName}` names two values in scope: a result of `{callee}` and another"
        scope := scope.push (resultName, type, isReference)
        -- A returned reference's prophecy, the value its place holds when
        -- it ends.
        if isReference then scope := scope.push (resultName ++ "'", type, isReference)
  return scope

/-- The argument pattern naming a function's parameters. -/
private def argumentPattern (unit : ValidatedUnit)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (params : NRow) :
    CommandElabM (TSyntax `rcasesPat) := do
  let names := declaration.signature.parameters.mapIdx fun index parameter =>
    if parameter.name.isEmpty then s!"argument{index}" else parameter.name
  if names.isEmpty then `(rcasesPat| _)
  else
    let parts := (params.toList.zip names.toList).map fun (τ, name) => carrierPattern unit name τ
    let text := "⟨" ++ ", ".intercalate parts ++ ", _⟩"
    match Parser.runParserCategory (← getEnv) `rcasesPat text with
    | .ok pattern => pure ⟨pattern⟩
    | .error message => throwError m!"internal: argument pattern `{text}`: {message}"

/-- The introduction of a step proof's arguments (`verexArguments`): each
parameter a local of its name, the scope's names recorded for the target
(`verex_scope_names`) and the parameters' scope entries bound
(`verex_scope_bind`), then each parameter taken apart as the argument
pattern takes it, which rewrites its entry. -/
private def scopedArguments (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (params : NRow)
    (withResult : Bool) : CommandElabM (TSyntax `tactic) := do
  let names := declaration.signature.parameters.mapIdx fun index parameter =>
    if parameter.name.isEmpty then s!"argument{index}" else parameter.name
  let scope ← match functionScope unit ns declaration withResult with
    | .ok scope => pure scope
    | .error message => throwError message
  let quoted (names : Array String) := ", ".intercalate (names.toList.map fun name => s!"\"{name}\"")
  let mut lines := #[]
  unless names.isEmpty do
    lines := lines.push s!"rcases verexArguments with ⟨{", ".intercalate (names.toList.map (s!"«{·}»"))}, _⟩"
  lines := lines.push s!"verex_scope_names [{quoted (scope.map (·.1))}] \
    [{quoted (scope.filterMap fun (name, _, reference) => if reference then some name else none)}] \
    [{quoted (declaration.locals.map (·.name))}]"
  lines := lines.push "verex_scope_row"
  unless names.isEmpty do
    lines := lines.push s!"verex_scope_bind [{", ".intercalate (names.toList.map (s!"«{·}»"))}]"
  for (τ, name) in params.toList.zip names.toList do
    let pattern := carrierPattern unit name τ
    unless pattern == s!"«{name}»" do lines := lines.push s!"all_goals rcases «{name}» with {pattern}"
  let text := "(" ++ "\n ".intercalate lines.toList ++ ")"
  match Parser.runParserCategory (← getEnv) `tactic text with
  | .ok tactic => pure ⟨tactic⟩
  | .error message => throwError m!"internal: scoped arguments `{text}`: {message}"

/-- Compare certificate types without treating holes as proof obligations. -/
def certificateTypesMatch (actual expected : Lean.Expr) : MetaM Bool := do
  let actual ← instantiateMVars actual
  let expected ← instantiateMVars expected
  if actual.hasMVar || actual.hasFVar || actual.hasLooseBVars ||
      expected.hasMVar || expected.hasFVar || expected.hasLooseBVars then
    return false
  isDefEq actual expected

/-- The statement a caller assumes of a native, rebuilt from its declaration:
its meaning satisfies its typed contract at every family, and at every
instantiation for a generic native. A non-generic native's types mention no
type parameter, so the statement is the same at every family; stated at each,
it serves a generic caller as well as the runtime family. -/
private def nativeHypothesisType (unit : ValidatedUnit)
    (stored : Contract.StoredInvariants) (executable functions requiresMeaning : Lean.Expr)
    (handle : FunctionHandle) : MetaM Lean.Expr := do
  let some nativeNs := unit.namespaces[handle.namespaceId.index]?
    | throwError "a native's namespace is out of range"
  let some declaration := nativeNs.functions[handle.functionId.index]?
    | throwError "a native is out of range"
  let some (params, result) := nativeSignature? unit handle.namespaceId declaration
    | throwError "a native has no native signature"
  -- A caller assumes what callers see: the interface; of a native without
  -- a specification, its prelude model.
  let nativeModel := nativeModelOf? unit handle nativeNs declaration
  let typed ← typedContractOf unit handle.namespaceId.index nativeNs
    (ContractView.interface.of declaration) params result nativeModel stored
  let view := ContractView.interface.of declaration
  let readsUnit := Contract.contractReadsUnit unit handle.namespaceId nativeNs view
  let readsRequires := Contract.contractReadsRequires unit handle.namespaceId view
  let unitExpr ← Contract.executableUnit executable
  -- The contract at a frame of the executable's unit: over the executable
  -- where it reads it, and its table last.
  let typedAt (skolems : Lean.Expr) (arguments : Array Lean.Expr) : Lean.Expr :=
    let leading := if readsUnit then #[unitExpr, functions, executable, skolems]
      else #[unitExpr, functions, skolems]
    let applied := mkAppN typed (leading ++ arguments)
    (if readsUnit && readsRequires then mkApp applied requiresMeaning else applied).headBeta
  let row ← quoteRow params
  let shape ← quoteShape result
  let meaningAt (skolems instantiation : Lean.Expr) :=
    mkAppN (mkConst ``propheticMeaning)
      #[unitExpr, functions, executable, skolems, instantiation, toExpr handle, row, shape]
  withLocalDecl `Θ .instImplicit (mkApp2 (mkConst ``Skolems) unitExpr functions) fun skolems =>
    if isGeneric declaration then
      withLocalDeclD `typeInstantiation
          (toTypeExpr (Array (VerexIR.TypeId × VerexIR.TypeId))) fun instantiation => do
        let statement ← mkAppM ``VerexIR.Proofs.Satisfies
          #[meaningAt skolems instantiation, typedAt skolems #[instantiation]]
        mkForallFVars #[skolems, instantiation] statement
    else do
      let statement ← mkAppM ``VerexIR.Proofs.Satisfies
        #[meaningAt skolems (toExpr (#[] : Array (VerexIR.TypeId × VerexIR.TypeId))),
          typedAt skolems #[]]
      mkForallFVars #[skolems] statement

/-- The functions whose verification has begun and not ended, by key: a
callee among them has no theorem yet, and a caller assuming its contract
would rest its proof on its own. -/
initialize inProgressTargets : EnvExtension (Array String) ←
  registerEnvExtension (pure #[])

/-- The constants this file's constants use, through their types and
values, constructors included, kept across the audits of one file: the
unit literal and the contracts are walked once, not at every target. An
environment extension, so that a rolled-back verification forgets its
entries with its declarations. -/
private initialize usedConstantsExt : EnvExtension (NameMap (Array Name)) ←
  registerEnvExtension (pure {})

/-- The unit definitions whose value the audit compared with the registered
unit's quotation (`auditedUnit`). An environment extension, as
`usedConstantsExt`. -/
private initialize auditedUnitsExt : EnvExtension NameSet ← registerEnvExtension (pure {})

/-- The unit definition of a registered unit, its value compared once per
environment with the unit's quotation: the audit states a target's theorem
over the definition, as the theorem does, instead of quoting the unit and
unfolding the definition at every target. -/
private def auditedUnit (segments : Array String) (unit : ValidatedUnit) : TermElabM Lean.Expr := do
  let name := unitName segments
  if (auditedUnitsExt.getState (← getEnv)).contains name then return mkConst name
  let some (.defnInfo info) := (← getEnv).find? name
    | throwError m!"the unit `{name}` has no definition"
  unless info.value == (← VerexSL.Quote.quoteUnit unit) do
    throwError m!"the definition of the unit `{name}` is not the registered unit"
  modifyEnv fun env => auditedUnitsExt.modifyState env (·.insert name)
  return mkConst name

/-- The constants `name` uses (`usedConstantsExt`), and whether it is an
axiom. -/
private def usedConstantsOf (name : Name) : CommandElabM (Array Name × Bool) := do
  let env ← getEnv
  let some info := env.find? name | return (#[], false)
  let isAxiom := info matches .axiomInfo _
  if let some used := (usedConstantsExt.getState env).find? name then return (used, isAxiom)
  let constructors := match info with
    | .inductInfo declaration => declaration.ctors.toArray
    | _ => #[]
  let used := info.type.getUsedConstants ++
    ((info.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[] ++ constructors
  modifyEnv fun env => usedConstantsExt.modifyState env (·.insert name used)
  return (used, isAxiom)

/-- The axioms the constants `roots` depend on: one walk over this file's
constants, each once, through `Environment.find?`, which forces no kernel
check (the kernel's chain holds the proofs still running, and the audit of
a concluded target does not wait for them); an imported constant's axioms
are those its module recorded (`collectAxioms`). -/
private def axiomsOf (roots : Array Name) : CommandElabM NameSet := do
  let env ← getEnv
  let mut visited : NameSet := {}
  let mut axioms : NameSet := {}
  let mut pending := roots.toList
  while let name :: rest := pending do
    pending := rest
    if visited.contains name then continue
    visited := visited.insert name
    if (env.getModuleIdxFor? name).isSome then
      axioms := (← collectAxioms name).foldl (·.insert ·) axioms
      continue
    let (used, isAxiom) ← usedConstantsOf name
    if isAxiom then axioms := axioms.insert name
    for dependency in used do
      unless visited.contains dependency do pending := dependency :: pending
  return axioms

/-- Whether an axiom is a `bv_decide` certificate: that its checker of an
UNSAT proof, evaluated natively, returns `true`. The checker is evaluated
again, as `bv_decide` evaluated it: a name and a shape alone admit nothing. -/
private def isBitVectorCertificate (name : Name) : CommandElabM Bool := do
  unless name.components.contains `_native && name.components.contains `bv_decide do
    return false
  let some (.axiomInfo info) := (← getEnv).find? name | return false
  let_expr Eq _ checked expected := info.type | return false
  unless checked.isAppOf ``Std.Tactic.BVDecide.Reflect.verifyBVExpr &&
      expected.isConstOf ``Bool.true && info.levelParams.isEmpty do
    return false
  liftTermElabM <| withoutModifyingEnv do
    try
      match ← Lean.Meta.nativeEqTrue `verex_audit checked with
      | .success _ => pure true
      | .notTrue => pure false
    catch _ => pure false

/-- The no-fallback audit of one verified function, including the transitive
axiom closure. The agreement axiom is the sole project-specific exception,
explicitly deferred to D4 in `doc/designs/denotation.md`; a function selecting
`pragma bv` may also rest on `bv_decide`'s natively checked certificates. -/
def requireNativeArtifacts (base : Name) (bitVectors : Bool) (model : Bool := false) :
    CommandElabM Unit := do
  let function := base.getString!
  let artifacts := base.replacePrefix (← getCurrNamespace) .anonymous
  let env ← getEnv
  let some theoremInfo := env.find? (base ++ `typedVerified)
    | throwError m!"`{function}` is not verified"
  unless theoremInfo.type.getUsedConstants.contains ``Term.denote do
    throwError m!"`{function}` was verified by a retired route"
  -- A model's function has no runtime certificate nor runtime theorem
  -- (`doc/designs/ghost-fields.md`).
  unless model || env.contains (artifacts ++ `compiled_eq) do
    throwError m!"missing compilation certificate for `{function}`"
  let roots := if model then #[base ++ `typedVerified, artifacts ++ `compiled]
    else #[base ++ `typedVerified, artifacts ++ `compiled_eq, artifacts ++ `compiled]
  let mut certificates : NameSet := {}
  for root in roots ++ (if model then #[] else #[base ++ `verified]) do
    for axiomName in (← axiomsOf #[root]).toArray do
      if bitVectors && certificates.contains axiomName then continue
      if bitVectors && (← isBitVectorCertificate axiomName) then
        certificates := certificates.insert axiomName
        continue
      unless #[``propext, ``Classical.choice, ``Quot.sound,
          ``compileFunction_agrees, ``compileFunction_least_cycle].contains axiomName do
        throwError m!"artifact `{root}` depends on unapproved axiom `{axiomName}`"
  let mut pending := roots
  let mut visited : NameSet := {}
  while let some name := pending.back? do
    pending := pending.pop
    if visited.contains name then continue
    visited := visited.insert name
    let some declaration := env.find? name
      | throwError m!"missing artifact `{name}`"
    let constants := declaration.type.getUsedConstants ++
      ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
    for dependency in constants do
      -- Rule 1 of the design: a verified function reasons over values,
      -- never over frames or loan bookkeeping.
      if dependency == ``sorryAx || dependency == ``VerexIR.RuntimeFrame ||
          dependency == ``VerexIR.SemanticOperations.FreshGlobalLoanIds ||
          dependency == ``VerexIR.SemanticOperations.LoanDiscipline ||
          dependency == ``VerexIR.SemanticOperations.globalLoanKeyIn? ||
          dependency == ``VerexIR.SemanticOperations.removeGlobalLoan then
        throwError m!"artifact `{name}` retains forbidden dependency `{dependency}`"
      if base.isPrefixOf dependency || artifacts.isPrefixOf dependency then
        pending := pending.push dependency
  unless completedDenotations.isTagged env (base ++ `typedVerified) do
    throwError m!"`{function}` has no completed denotation verification"
  -- A model's verification is its typed theorem (`doc/designs/ghost-fields.md`).
  if model then return
  -- The completion tag is only bookkeeping: source metaprograms can mutate
  -- extensions. Require an actual theorem for this registered unit, function
  -- and authored contract, reconstructed without trusting cached definitions.
  let some publicInfo := env.find? (base ++ `verified)
    | throwError m!"`{function}` has no public verification certificate"
  let namespaceName := artifacts.getPrefix
  let some unit := VerexSL.registeredUnit? env namespaceName
    | throwError m!"unknown Verex namespace `{namespaceName}`"
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwError m!"unknown function `{function}` in `{namespaceName}`"
  let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
    | throwError m!"`{function}` has no native signature"
  let segments := namespaceName.components.toArray.map (·.getString!)
  let stored ← ensureStoredInvariant segments unit
  let unitDefinition ← ensureUnitInstance segments unit
  let valid ← liftTermElabM do
    let typed ← typedContractOf unit namespaceIndex ns (ContractView.implementation.of declaration)
      params result (stored := stored)
    let readsUnit := Contract.contractReadsUnit unit ⟨namespaceIndex⟩ ns
      (ContractView.implementation.of declaration)
    let readsRequires :=
      Contract.contractReadsRequires unit ⟨namespaceIndex⟩
        (ContractView.implementation.of declaration)
    let unitExpr ← auditedUnit segments unit
    -- The unit's function values: the instance declared with the unit.
    let functions ← functionCarriersOf (mkConst unitDefinition)
    let requiresMeaning := mkApp (mkConst (functionRequiresName segments))
      (mkApp2 (mkConst ``VerexIR.Proofs.requiresForeign) unitExpr functions)
    let publicContract ← publicContractOf params result (isGeneric declaration) typed readsUnit
      readsRequires
    let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
    let expected ← withLocalDecl `registry .implicit
        (mkConst ``VerexIR.Validation.SemanticsRegistry) fun registry =>
      withLocalDecl `executable .implicit (Contract.executableType unitExpr) fun executable => do
        let prepared ← mkAppM ``VerexIR.Validation.prepareExecution #[registry, unitExpr]
        let errorType := (← inferType prepared).getAppArgs[0]!
        let success ← mkAppOptM ``Except.ok #[some errorType,
          some (Contract.executableType unitExpr), some executable]
        let preparation ← mkEq prepared success
        let contractAt (arguments : Array Lean.Expr) :=
          (mkAppN publicContract (#[unitExpr, functions, executable] ++ arguments ++
            (if readsRequires then #[requiresMeaning] else #[]))).headBeta
        -- A generic function's theorem holds at every instantiation a call
        -- gives it, coherent with the frame its type arguments induce.
        let conclusion ← if isGeneric declaration then
            let instantiationType ← mkAppM ``Array
              #[← mkAppM ``Prod #[mkConst ``VerexIR.TypeId, mkConst ``VerexIR.TypeId]]
            let arity := (declaration.signature.generics.filter (·.kind == .typeArg)).size
            withLocalDeclD `θ (mkConst ``TypeArgs) fun θ => do
            withLocalDeclD `free (← mkEq (mkApp (mkConst ``NRow.refFree)
                (← mkAppM ``Subtype.val #[θ])) (mkConst ``Bool.true)) fun free =>
            withLocalDeclD `typeInstantiation instantiationType fun instantiation =>
            withLocalDeclD `frame (mkAppN (mkConst ``FrameOf)
                #[unitExpr, toExpr handle, toExpr arity, instantiation]) fun frame => do
            let induced := mkApp4 (mkConst ``Skolems.instantiate) unitExpr functions θ
              (mkApp2 (mkConst ``Skolems.runtime) unitExpr functions)
            withLocalDeclD `coherent (mkAppN (mkConst ``Coherent)
                #[unitExpr, functions, induced, toExpr handle, instantiation]) fun coherent => do
            mkForallFVars #[θ, free, instantiation, frame, coherent]
              (← mkAppM ``VerexIR.Proofs.SatisfiesFunctionAt
                #[executable, toExpr handle, instantiation, contractAt #[θ, instantiation]])
          else
            mkAppM ``VerexIR.Proofs.SatisfiesFunction #[executable, toExpr handle, contractAt #[]]
        -- The natives the theorems assume, in their binding order.
        let natives := ((nativeDependencies.getState env).find? base).getD #[]
        let hypotheses ← natives.mapM fun (nativeHandle, _) =>
          nativeHypothesisType unit stored executable functions requiresMeaning nativeHandle
        -- Then the assumed steps of the lemmas it applies.
        let trusted := ((lemmaTrustDependencies.getState env).find? base).getD #[]
        let hypotheses := hypotheses ++ trusted.map mkConst
        -- The public theorem takes that the unit's runs keep memory typed and
        -- fail with integers.
        let hypotheses := #[mkApp3 (mkConst ``VerexIR.Proofs.Denote.GlobalsPreserved) unitExpr
            functions executable,
          ← mkAppM ``VerexIR.Proofs.Denote.FailuresIntegral #[executable]] ++ hypotheses
        -- A theorem assuming typing takes what the natives preserve, last,
        -- and one resolving `result_of` that the unit's runs end.
        let assumesTermination := Contract.assumesTermination unit ⟨namespaceIndex⟩ ns declaration
        let hypotheses ← if Contract.assumesTyping unit ⟨namespaceIndex⟩ ns declaration ||
            assumesTermination then
            pure (hypotheses ++ #[← mkAppM ``VerexIR.NativesTyped #[executable],
              ← mkAppM ``VerexIR.NativesShift #[executable],
              ← mkAppM ``VerexIR.Proofs.RunsAgree #[executable],
              ← mkAppM ``VerexIR.Proofs.OwnersRepresentable #[unitExpr]])
          else pure hypotheses
        let hypotheses ← if assumesTermination then
            pure (hypotheses.push
              (mkApp3 (mkConst ``VerexIR.Proofs.Terminating) unitExpr functions executable))
          else pure hypotheses
        -- Last, that the in-body assumptions of the functions it relies on
        -- hold, of their readings under the closed meanings
        -- (`assumptionBinderOf`).
        let assumptions := ((assumptionDependencies.getState env).find? base).getD #[]
        let instantiationType ← mkAppM ``Array
          #[← mkAppM ``Prod #[mkConst ``VerexIR.TypeId, mkConst ``VerexIR.TypeId]]
        let hypotheses := hypotheses ++ (← assumptions.mapM fun (_, name) => do
          let artifacts := Name.str namespaceName name
          withLocalDecl `Θ .implicit (mkApp2 (mkConst ``Skolems) unitExpr functions) fun family =>
          withLocalDecl `typeInstantiation .implicit instantiationType fun instantiation => do
            let meanings ← mkAppOptM ``VerexIR.Proofs.Denote.closedMeanings
              #[unitExpr, functions, executable, family, instantiation]
            let plain := mkAppN (mkConst (artifacts ++ `plainMeaning)) #[executable, family, meanings]
            let assumed :=
              mkAppN (mkConst (artifacts ++ `assumedMeaning)) #[executable, family, meanings, executable]
            mkForallFVars #[family, instantiation] (← mkAppM ``VerexIR.Proofs.AssumptionsHold
              #[plain, assumed]))
        let rec assume (index : Nat) (bound : Array Lean.Expr) : MetaM Lean.Expr := do
          if h : index < hypotheses.size then
            withLocalDeclD `native hypotheses[index] fun hypothesis =>
              assume (index + 1) (bound.push hypothesis)
          else
            mkForallFVars #[registry, executable]
              (← mkArrow preparation (← mkForallFVars bound conclusion))
        assume 0 #[]
    certificateTypesMatch publicInfo.type expected
  unless valid do
    throwError m!"`{function}` has an invalid public verification certificate"

/-- Publish the compilation view of a namespace as a literal, with the kernel
certificate that it is the view the unit's namespace has: functions'
certificates then read its arenas instead of rebuilding them. -/
private def publishCompileView (segments : Array String) (unit : ValidatedUnit)
    (namespaceId : NamespaceId) : TermElabM Name := do
  let viewName := Name.str (semanticsName segments) s!"compileView_{namespaceId.index}"
  let viewEqName := viewName ++ `eq
  if (← getEnv).contains viewEqName then return viewEqName
  let some view := compileNamespaceAt? unit namespaceId
    | throwError "namespace {namespaceId.index} is out of range"
  let semantics := mkConst (unitName segments)
  let source ← mkAppM ``Option.get!
    #[← mkAppM ``getElem? #[mkApp (mkConst ``ValidatedUnit.namespaces) semantics,
      toExpr namespaceId.index]]
  let value := mkAppN (mkConst ``CompileNamespace.mk) #[source, toExpr view.namespaceId,
    toExpr view.expressions, toExpr view.places, toExpr view.patterns, toExpr view.types,
    toExpr view.deaths, toExpr view.shared, toExpr view.invariants, toExpr view.typeFuel,
    toExpr view.fuel, toExpr view.placeFuel]
  addDecl (.defnDecl {
    name := viewName, levelParams := []
    type := mkConst ``CompileNamespace
    value, hints := .abbrev, safety := .safe })
  let some' := mkApp (mkConst ``Option.some [Level.zero]) (mkConst ``CompileNamespace)
  addDecl (.thmDecl {
    name := viewEqName, levelParams := []
    type := ← mkEq (mkAppN (mkConst ``compileNamespaceAt?) #[semantics, toExpr namespaceId])
      (mkApp some' (mkConst viewName))
    value := ← mkEqRefl (mkApp some' (mkConst viewName)) })
  return viewEqName

/-- The kernel certificate that a unit declares no ghost field, which a
compiled function's certificate rests on. -/
private def publishNoGhosts (segments : Array String) (unit : ValidatedUnit) : TermElabM Name := do
  let name := Name.str (semanticsName segments) "noGhosts"
  if (← getEnv).contains name then return name
  if declaresGhosts unit then
    throwError "a unit declaring ghost fields is verified through its model, which is not \
      available yet (`doc/designs/ghost-fields.md`)"
  let semantics := mkConst (unitName segments)
  addDecl (.thmDecl {
    name, levelParams := []
    type := ← mkEq (mkApp (mkConst ``declaresGhosts) semantics) (mkConst ``Bool.false)
    value := ← mkEqRefl (mkConst ``Bool.false) })
  return name

/-- Publish the compiled function of a target: its rows, shape, body,
mutable parameters, the function itself, and the kernel certificate of its
compilation, read through its namespace's view.  Nothing is republished. -/
private def publishCompiled (segments : Array String) (unit : ValidatedUnit)
    (function : String) (handle : FunctionHandle) (compiled : Function unit) : TermElabM Unit := do
  let artifacts := Name.str (pathName segments) function
  -- Published once: a model's function has no certificate to mark it.
  if (← getEnv).contains (artifacts ++ (if declaresGhosts unit then `compiled else `compiled_eq)) then
    return
  let semantics := mkConst (unitName segments)
  addAbbrev (artifacts ++ `row) (← quoteRow (compiled.params ++ compiled.locals))
  -- The signature a caller assuming the function's interface published.
  unless (← getEnv).contains (artifacts ++ `params) do
    addAbbrev (artifacts ++ `params) (← quoteRow compiled.params)
  addAbbrev (artifacts ++ `locals) (← quoteRow compiled.locals)
  unless (← getEnv).contains (artifacts ++ `shape) do
    addAbbrev (artifacts ++ `shape) (← quoteShape compiled.result)
  addAbbrev (artifacts ++ `body) (← quoteTerm semantics compiled.body)
  addAbbrev (artifacts ++ `mutables) (← quoteMutables compiled.mutables)
  addAbbrev (artifacts ++ `compiled)
    (← quoteFunction semantics compiled (mkConst (artifacts ++ `body))
      (mkConst (artifacts ++ `mutables)))
  let viewEq ← publishCompileView segments unit handle.namespaceId
  -- A model's function has no runtime certificate (`doc/designs/ghost-fields.md`).
  if declaresGhosts unit then return
  let noGhosts ← publishNoGhosts segments unit
  let lhs := mkAppN (mkConst ``compileFunction) #[semantics, toExpr handle]
  let rhs := mkAppN (mkConst ``Except.ok [Level.zero, Level.zero])
    #[mkConst ``String, mkApp (mkConst ``Function) semantics, mkConst (artifacts ++ `compiled)]
  let view := (Name.str (semanticsName segments) s!"compileView_{handle.namespaceId.index}")
  let throughView ← mkEq
    (mkAppN (mkConst ``compileFunctionIn) #[semantics, mkConst view, toExpr handle]) rhs
  addDecl (.thmDecl {
    name := artifacts ++ `compiled_eq
    levelParams := []
    type := ← mkEq lhs rhs
    value := ← mkEqTrans
      (mkAppN (mkConst ``compileFunction_of_view)
        #[semantics, toExpr handle, mkConst view, mkConst noGhosts, mkConst viewEq])
      (← mkExpectedTypeHint (← mkEqRefl rhs) throughView) })

/-- The signature artifacts of a native: it has no compiled body, so its
parameter row and result shape come from its declaration. -/
private def publishNativeSignature (segments : Array String) (function : String)
    (params : NRow) (result : ResultShape) : TermElabM Unit := do
  let artifacts := Name.str (pathName segments) function
  unless (← getEnv).contains (artifacts ++ `params) do
    addAbbrev (artifacts ++ `params) (← quoteRow params)
  unless (← getEnv).contains (artifacts ++ `shape) do
    addAbbrev (artifacts ++ `shape) (← quoteShape result)

/-- A function handle as a literal term. -/
private def handleSyntax (handle : FunctionHandle) : CommandElabM Term :=
  `(term| (⟨⟨$(Syntax.mkNatLit handle.namespaceId.index)⟩,
    ⟨$(Syntax.mkNatLit handle.functionId.index)⟩⟩ : VerexIR.FunctionHandle))

/-- The functions a unit's closures target. -/
def closureTargets (unit : ValidatedUnit) : Array FunctionHandle := Id.run do
  let mut targets := #[]
  for ns in unit.namespaces, index in [0:unit.namespaces.size] do
    for expression in ns.expressions do
      if let .operation (.call (.closure reference _)) .. := expression.kind then
        if let some target := VerexIR.SemanticOperations.resolveFunction? unit ⟨index⟩ reference then
          unless targets.contains target do targets := targets.push target
  return targets

/-- The functions the closures a function's contract names target: what the
contract's behavioral predicates may read of their targets' theorems. -/
def contractClosureTargets (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Array FunctionHandle := Id.run do
  let mut targets := #[]
  let mut pending := declaration.contract.conditions.map (·.expression)
  let mut visited : Array Nat := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if let .operation (.call (.closure reference _)) .. := expression.kind then
      if let some target := VerexIR.SemanticOperations.resolveFunction? unit namespaceId reference then
        unless targets.contains target do targets := targets.push target
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  return targets

/-- Whether a contract definition takes the executable unit (its contract
states a behavioral predicate) and the table of declared preconditions
(it states `requires_of`), as its last parameters. -/
def contractTakes (env : Environment) (name : Name) : Bool × Bool :=
  match env.find? name with
  | some info => takes info.type
  | none => (false, false)
where
  takes : Lean.Expr → Bool × Bool
    | .forallE _ domain body _ =>
        let (unit, requires) := takes body
        (unit || domain.isAppOf ``VerexIR.Validation.ExecutableUnit ||
            domain.isAppOf ``VerexIR.Proofs.ModelUnit,
          requires || domain.isAppOf ``VerexIR.Proofs.RequiresMeaning)
    | _ => (false, false)

/-- Prove once per unit, by evaluation, that its resource types correspond
to their keys' types (`ResourcesTyped`): every memory's runtime encoding is
then typed. -/
def ensureResourcesTyped (segments : Array String) (unit : ValidatedUnit) : CommandElabM Name := do
  let name := resourcesTypedName segments
  if (← getEnv).contains name then return name
  let unitDefinition ← ensureUnitInstance segments unit
  elabCommand (← `(command|
    theorem $(rootIdent name) : VerexIR.Proofs.ResourcesTyped $(rootIdent unitDefinition) :=
      VerexIR.Proofs.ResourcesTyped.ofCheck (by decide +kernel)))
  return name

/-- Prove once per unit, by evaluation, that its native and semantic readings
agree (`TypesAgree`), where they do: a function value is then typed by its
carrier. -/
def ensureTypesAgree (segments : Array String) (unit : ValidatedUnit) :
    CommandElabM (Option Name) := do
  let name := typesAgreeName segments
  if (← getEnv).contains name then return some name
  unless VerexIR.Proofs.typesAgreeCheck unit do return none
  let unitDefinition ← ensureUnitInstance segments unit
  elabCommand (← `(command|
    theorem $(rootIdent name) : VerexIR.Proofs.TypesAgree $(rootIdent unitDefinition) :=
      VerexIR.Proofs.TypesAgree.ofCheck (by decide +kernel)))
  return some name

/-- The module's unit, the constant a theorem's frames and memory are at. -/
def unitTermOf (segments : Array String) (unit : ValidatedUnit) : CommandElabM Term :=
  return rootIdent (← ensureUnitInstance segments unit)

/-- The binder of a model's theorem holding the meaning of invoking a
function value at every family (`doc/designs/ghost-fields.md`). -/
def invokeIdent : Ident := mkIdent `verexInvoke

/-- What a contract reading the unit takes in a theorem: the executable
unit, or, in a unit verified through its model, the model of the executable
and the theorem's invocation meaning. -/
def unitReaderTerm (unit : ValidatedUnit) : CommandElabM Term :=
  if declaresGhosts unit then
    `(term| (VerexIR.Proofs.ModelUnit.mk executable $invokeIdent $(mkIdent `verexInvokeCoherent)))
  else `(term| executable)

/-- The requires meaning a theorem's contracts read (`RequiresMeaning`): the
unit's, reading a value no closure site encodes by the uninterpreted
`requiresForeign`. -/
def requiresMeaningTerm (segments : Array String) (unit : ValidatedUnit) : CommandElabM Term := do
  for site in ← closureSitesOf segments unit do
    if siteRequiresRejected unit site.target then
      throwError m!"`requires_of` cannot read `{functionKey unit site.target}`: its precondition \
        states a behavioral predicate"
  let unitTerm ← unitTermOf segments unit
  let requires := rootIdent (functionRequiresName segments)
  `(term| ($requires (VerexIR.Proofs.requiresForeign $unitTerm)))

/-- A contract term at the module's unit, the executable unit, and the
module's requires meaning, where the contract takes them. -/
def atUnit (segments : Array String) (unit : ValidatedUnit) (name : Name) (contract : Term)
    (frame? : Option Term := none) (args : Array Term := #[]) : CommandElabM Term := do
  let (readsUnit, readsRequires) := contractTakes (← getEnv) name
  let unitTerm ← unitTermOf segments unit
  let reader ← unitReaderTerm unit
  let requiresMeaning : Array Term ← if readsRequires then
      pure #[← requiresMeaningTerm segments unit]
    else pure #[]
  match frame? with
  | some frame =>
      let functions ← `(term| _)
      let leading : Array Term ← if readsUnit then pure #[unitTerm, functions, reader]
        else pure #[unitTerm, functions]
      `(term| (@$contract $leading* $frame $args* $requiresMeaning*))
  | none =>
      if readsUnit then `(term| ($contract $reader $args* $requiresMeaning*))
      else if args.isEmpty then pure contract
      else `(term| ($contract $args*))

/-- The hypothesis a caller's theorems take for a native, or for a function
seen through its interface: the statement a verified callee's theorem
makes, that its meaning satisfies its contract, at every family; a generic
one's at every instantiation as well. -/
private def nativeBinder (segments : Array String) (unit : ValidatedUnit)
    (entry : FunctionHandle × String) (generic : Bool) (interface : Bool) :
    CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) := do
  let (handle, name) := entry
  let unitTerm ← unitTermOf segments unit
  let artifacts := Name.str (pathName segments) name
  let handleTerm ← handleSyntax handle
  let params := rootIdent (artifacts ++ `params)
  let shape := rootIdent (artifacts ++ `shape)
  let contractName := if interface then typedInterfaceContractName segments name
    else typedContractName segments name
  let contract := rootIdent contractName
  let binder := mkIdent (Name.mkSimple s!"native_{name}")
  if generic then
    let atFamily ← atUnit segments unit contractName contract (some (← `(term| Θ)))
      #[← `(term| typeInstantiation)]
    `(bracketedBinder| ($binder : ∀ {Θ : VerexIR.Proofs.Denote.Skolems $unitTerm}
        {typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId)},
        VerexIR.Proofs.Satisfies
          (VerexIR.Proofs.Denote.propheticMeaning executable typeInstantiation $handleTerm
            $params $shape)
          $atFamily))
  else
    let atFamily ← atUnit segments unit contractName contract (some (← `(term| Θ)))
    `(bracketedBinder| ($binder : ∀ {Θ : VerexIR.Proofs.Denote.Skolems $unitTerm},
        VerexIR.Proofs.Satisfies
          (@VerexIR.Proofs.Denote.propheticMeaning _ _ executable Θ #[] $handleTerm $params $shape)
          $atFamily))

/-- The standard-library modules whose intrinsic functions the Move Prover's
prelude implements. -/
private def preludeIntrinsicModules : List String :=
  ["vector", "event", "aggregator", "aggregator_v2"]

/-- Whether a function is intrinsic, as the Move Prover reads the pragma: it
declares `pragma intrinsic` and has a meaning other than its body (it is
native, a map role, or in a module the prelude implements), or it is opaque,
so that callers rely on its contract by the author's choice. Elsewhere the
pragma leaves the body its meaning, verified as any other. -/
private def isIntrinsic (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  let declared := declaration.pragmas.any fun
    | .assign "intrinsic" (.constant (.bool false)) _ => false
    | .assign "intrinsic" _ _ => true
    | _ => false
  let prelude := match (unit.tables.namespaces[namespaceId.index]?).map (·.segments) with
    | some #["0x1", name] => preludeIntrinsicModules.contains name
    | _ => false
  declared && (declaration.body == .absent || prelude || isOpaque declaration ||
    (Contract.mapRoleOf? unit namespaceId ns declaration).isSome)

/-- Whether a function's specification or its module sets
`pragma verify = false`, or the function is intrinsic: the function's
pragmas merge both. -/
def automaticVerificationDisabled (unit : ValidatedUnit)
    (namespaceId : VerexIR.NamespaceId) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  isIntrinsic unit namespaceId ns declaration ||
  (Contract.representationRead? unit ns declaration).isSome || declaration.pragmas.any fun
    | .assign "verify" (.constant (.bool false)) _ => true
    | _ => false

/-- A member of a cycle of calls standing for the calls to it, and its
contract of them. -/
private def cycleSelf (name : String) : Ident := mkIdent (Name.mkSimple s!"self_{name}")
/-- The hypothesis that an inlined callee's meaning refines its body. -/
private def cycleSelfRefines (name : String) : Ident :=
  mkIdent (Name.mkSimple s!"self_{name}_refines")
private def cycleSelfVerified (name : String) : Ident :=
  mkIdent (Name.mkSimple s!"selfVerified_{name}")

/-- The position of the `index`th member of a cycle. -/
private def cycleIndexSyntax : Nat → CommandElabM Term
  | 0 => `(term| VerexIR.Proofs.Denote.CycleIndex.here)
  | index + 1 => do `(term| VerexIR.Proofs.Denote.CycleIndex.there $(← cycleIndexSyntax index))

/-- The meanings of a cycle's members at every slot. -/
private def cycleSelves : Ident := mkIdent `verexSelves

/-- The members of a cycle, each a handle with its compiled function. -/
private def cycleMembersSyntax (segments : Array String) (cycle : Array (FunctionHandle × String)) :
    CommandElabM Term := do
  let pairs ← cycle.mapM fun (member, name) => do
    `(term| ($(← handleSyntax member), $(rootIdent (compiledName segments name))))
  `(term| [$pairs,*])

/-- The declaration a handle names. -/
private def declarationOf? (unit : ValidatedUnit) (handle : FunctionHandle) :
    Option (VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  ns.functions[handle.functionId.index]?

/-- The hypothesis a native assumes: its contract at every family and
instantiation when it is generic, at the runtime family otherwise. -/
private def nativeBinderOf (unit : ValidatedUnit) (segments : Array String)
    (entry : FunctionHandle × String) :
    CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) :=
  let declaration? := declarationOf? unit entry.1
  nativeBinder segments unit entry (declaration?.any isGeneric)
    (declaration?.any fun declaration => declaration.body != .absent)

/-- The meanings a function with in-body assumptions is read under, outside
a cycle: the closed meanings at the instantiation, under a model with its
ghost tables and the theorem's invocation meaning at the frame
(`meaningsTerm`). -/
private def assumptionMeanings (segments : Array String) (unit : ValidatedUnit) (function : String) :
    CommandElabM Term := do
  if !declaresGhosts unit then
    return ← `(term| VerexIR.Proofs.Denote.closedMeanings executable typeInstantiation)
  let artifacts := Name.str (pathName segments) function
  let reader ← unitReaderTerm unit
  `(term| ({ VerexIR.Proofs.Denote.closedMeanings executable typeInstantiation with
      ghost := $(rootIdent (artifacts ++ `ghosts)) $reader,
      ghostInit := $(rootIdent (artifacts ++ `ghostInits)) $reader,
      invoke := $invokeIdent Θ } : VerexIR.Proofs.Denote.Meanings executable))

/-- The hypothesis that a function's in-body assumptions hold
(`AssumptionsHold`) of its readings under its meanings (`assumptionMeanings`),
at every family and instantiation: bound after a model's invocation
binders, which the meanings name. -/
private def assumptionBinderOf (segments : Array String) (unit : ValidatedUnit)
    (entry : FunctionHandle × String) : CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) := do
  let unitTerm ← unitTermOf segments unit
  let artifacts := Name.str (pathName segments) entry.2
  let binder := mkIdent (Name.mkSimple s!"assumptions_{entry.2}")
  let meanings ← assumptionMeanings segments unit entry.2
  let reader ← unitReaderTerm unit
  `(bracketedBinder| ($binder : ∀ {Θ : VerexIR.Proofs.Denote.Skolems $unitTerm}
      {typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId)},
      VerexIR.Proofs.AssumptionsHold
        (@$(rootIdent (artifacts ++ `plainMeaning)) executable Θ $meanings)
        (@$(rootIdent (artifacts ++ `assumedMeaning)) executable Θ $meanings $reader)))

/-- The arguments passing the hypotheses `assumptionBinderOf` binds. -/
private def assumptionArgs (entries : Array (FunctionHandle × String)) : Array Term :=
  entries.map fun entry => ⟨mkIdent (Name.mkSimple s!"assumptions_{entry.2}")⟩

/-- The binder of a trusted step's hypothesis. -/
def trustBinderName (name : Name) : Name :=
  Name.mkSimple s!"trusted_{name.toString (escape := false) |>.replace "." "_"}"

/-- The hypothesis that an assumed step of a lemma holds. -/
private def trustBinderOf (name : Name) :
    CommandElabM (TSyntax ``Lean.Parser.Term.bracketedBinder) := do
  `(bracketedBinder| ($(mkIdent (trustBinderName name)) : $(rootIdent name)))

/-- The arguments passing the hypotheses `trustBinderOf` binds. -/
private def trustArgs (names : Array Name) : Array Term :=
  names.map fun name => ⟨mkIdent (trustBinderName name)⟩

/-- A function reached again along calls that are inlined, if any: such a
cycle has no finite inlining. -/
private partial def inliningCycle? (edges : Array (FunctionHandle × Array FunctionHandle))
    (path : Array FunctionHandle) (node : FunctionHandle) : Option FunctionHandle :=
  if path.contains node then some node
  else
    let next := ((edges.find? (·.1 == node)).map (·.2)).getD #[]
    next.findSome? (inliningCycle? edges (path.push node))

/-- A kernel-decided theorem: the statement's `Decidable` instance evaluates
to `true`. -/
private def decidedByKernel (declName : Name) (statement : Term) : TermElabM Unit := do
  let statement ← Lean.Elab.Term.elabType statement
  Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
  let statement ← instantiateMVars statement
  let decision ← synthInstance (mkApp (mkConst ``Decidable) statement)
  addDecl (.thmDecl {
    name := declName, levelParams := []
    type := statement
    value := mkApp3 (mkConst ``of_decide_eq_true) statement decision
      (mkApp2 (mkConst ``Eq.refl [1]) (mkConst ``Bool) (mkConst ``Bool.true)) })

/-- A kernel-checked equation, proved by `Eq.refl` of its right side. -/
private def reflexiveTheorem (declName : Name) (statement : Term) : TermElabM Unit := do
  let statement ← Lean.Elab.Term.elabType statement
  Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
  let statement ← instantiateMVars statement
  addDecl (.thmDecl {
    name := declName, levelParams := []
    type := statement
    value := ← mkEqRefl statement.appArg! })

/-- Run a kernel decision whose cost the size of its subject sets, without
the elaborator's heartbeat limit. -/
private def unbudgeted (act : TermElabM α) : TermElabM α :=
  withTheReader Core.Context (fun context => { context with maxHeartbeats := 0 }) act

/-- A reducible definition of a literal value. -/
private def literalDefinition (declName : Name) (type value : Lean.Expr) : TermElabM Unit :=
  addDecl (.defnDecl {
    name := declName, levelParams := [], type, value, hints := .abbrev, safety := .safe })

/-! ## Lemmas

A lemma becomes a theorem over its statement's definitions
(`Contract.ensureLemmaDefinitions`): its parameters, then its premise, give
its conclusion. Its proof proves each step, asserting what the step owes and
assuming what it gives, then the conclusion. A recursion group is one
`mutual` block whose theorems recurse on the lemmas' measure.
-/

/-- The lemma instances an expression states. -/
private partial def lemmaInstancesOf (ns : ValidatedNamespace) (id : VerexIR.ExprId) :
    Array VerexIR.QualifiedRef := Id.run do
  let mut work := [id]
  let mut found := #[]
  for _ in [0:ns.expressions.size + 1] do
    match work with
    | [] => break
    | current :: rest =>
        work := rest
        let some expression := ns.expressions[current.index]? | continue
        if let .operation (.specification (.lemma reference _)) _ _ _ := expression.kind then
          unless found.contains reference do found := found.push reference
        work := work ++ (VerexIR.Validation.expressionChildren expression.kind).toList
  return found

/-- What the lemmas of a proof file apply in their step blocks, which their
declarations do not state. -/
abbrev StepApplications := Array (VerexIR.QualifiedRef × Array VerexIR.QualifiedRef)

/-- The lemmas a lemma's proof applies, its step block's among them. -/
private def appliedLemmas (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef)
    (steps : StepApplications := #[]) : Array VerexIR.QualifiedRef :=
  let stepped := ((steps.find? (·.1 == reference)).map (·.2)).getD #[]
  match Contract.lemmaOf? unit reference with
  | some (ns, declaration) => declaration.proof.foldl (init := stepped) fun found condition =>
      (lemmaInstancesOf ns condition.expression).foldl (init := found) fun found reference =>
        if found.contains reference then found else found.push reference
  | none => stepped

/-- The lemmas a function's body applies. -/
def functionLemmas (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Array VerexIR.QualifiedRef :=
  match declaration.body with
  | .structured root => (bodySpecifications ns root (· matches .apply)).foldl (init := #[])
      fun found (_, block) => block.conditions.foldl (init := found) fun found condition =>
        if condition.kind != .apply then found
        else (lemmaInstancesOf ns condition.expression).foldl (init := found) fun found reference =>
          if found.contains reference then found else found.push reference
  | .absent => #[]

/-- The lemmas a lemma reaches through the lemmas its proof applies. -/
private def reachableLemmas (unit : ValidatedUnit) (start : VerexIR.QualifiedRef)
    (steps : StepApplications := #[]) : Array VerexIR.QualifiedRef := Id.run do
  let mut visited : Array VerexIR.QualifiedRef := #[]
  let mut work := (appliedLemmas unit start steps).toList
  let bound := unit.namespaces.foldl (fun total ns => total + ns.lemmas.size) 1
  for _ in [0:bound * bound + 1] do
    match work with
    | [] => break
    | current :: rest =>
        work := rest
        if visited.contains current then continue
        visited := visited.push current
        work := work ++ (appliedLemmas unit current steps).toList
  return visited

/-- The recursion groups of the lemmas `roots` reach, each after the groups
it applies. -/
private def lemmaGroups (unit : ValidatedUnit) (roots : Array VerexIR.QualifiedRef)
    (steps : StepApplications := #[]) : Array (Array VerexIR.QualifiedRef) := Id.run do
  let mut lemmas := roots
  for root in roots do
    for reached in reachableLemmas unit root steps do
      unless lemmas.contains reached do lemmas := lemmas.push reached
  let reaches := lemmas.map fun lemma => (lemma, reachableLemmas unit lemma steps)
  let reached (source target : VerexIR.QualifiedRef) : Bool :=
    (reaches.find? (·.1 == source)).any (·.2.contains target)
  let mut groups : Array (Array VerexIR.QualifiedRef) := #[]
  let mut placed : Array VerexIR.QualifiedRef := #[]
  for _ in [0:lemmas.size + 1] do
    for lemma in lemmas do
      if placed.contains lemma then continue
      let group := lemmas.filter fun other =>
        other == lemma || (reached lemma other && reached other lemma)
      -- A group comes after every group it applies.
      let ready := (reachableLemmas unit lemma steps).all fun applied =>
        group.contains applied || placed.contains applied
      if ready then
        groups := groups.push group
        placed := placed ++ group
  return groups

/-- The lemma items of a module, not descending into its Lean items. -/
partial def lemmaItemsOf (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if child.isOfKind ``verexLemmaItem then found.push child
    else if child.isOfKind ``verexTheoremItem then found
    else found ++ lemmaItemsOf child) #[]

/-- The trusted steps a lemma's theorem takes as hypotheses: its own and
those of every lemma it reaches. -/
def lemmaTrustClosure (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    MetaM (Array Name) := do
  let mut found : Array Name := #[]
  for lemma in #[reference] ++ reachableLemmas unit reference do
    for (_, name) in (← Contract.lemmaNames unit lemma).trusted do
      unless found.contains name do found := found.push name
  return found

/-- The syntax of a definition's parameter type. -/
private def lemmaTypeSyntax (type : Lean.Expr) : CommandElabM Term := do
  match type with
  | .sort .zero => `(term| Prop)
  | .const name _ => pure (rootIdent name)
  | _ => throwError m!"a lemma parameter has no spelled type: {type}"

/-- The steps of a proof file's lemma item over its premise (internal: its
theorem's proof): each parameter's binder under the parameter's name, then
the block (`VerexSL.Verify.Steps`). -/
syntax (name := verexLemmaSteps) "verex_lemma_steps" (str ident)* " => " verexProofBlock : tactic

/-- A lemma's theorem, in its recursion group: over what its statement
reads, its parameters, and its premise; with its measure when the group
recurses. A lemma item of a proof file is proved by its step block. -/
private def lemmaTheoremCommand (unit : ValidatedUnit)
    (reference : VerexIR.QualifiedRef) (recursive : Bool) (block? : Option Syntax := none) :
    CommandElabM (TSyntax `command) := do
  let names ← liftTermElabM (Contract.ensureLemmaDefinitions unit reference)
  let some (_, declaration) := Contract.lemmaOf? unit reference
    | throwError "a lemma does not resolve"
  let (readsUnit, readsRequires, readsFrame, readsState) := Contract.lemmaReadsOf unit reference
  let mut readBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
  let mut readArguments : Array Term := #[]
  if readsRequires || readsFrame || readsState then
    readBinders := readBinders.push
      (← `(bracketedBinder| (unit : VerexIR.Validation.ValidatedUnit)))
    readBinders := readBinders.push
      (← `(bracketedBinder| [VerexIR.Proofs.Denote.FunctionCarriers unit]))
    readArguments := readArguments.push (← `(term| unit))
  if readsUnit then
    readBinders := readBinders.push
      (← `(bracketedBinder| (executable : VerexIR.Validation.ExecutableUnit)))
    readArguments := readArguments.push (← `(term| executable))
  if readsRequires then
    readBinders := readBinders.push
      (← `(bracketedBinder| (requiresMeaning : VerexIR.Proofs.RequiresMeaning unit)))
    readArguments := readArguments.push (← `(term| requiresMeaning))
  if readsFrame then
    readBinders := readBinders.push (← `(bracketedBinder| (frame : VerexIR.Proofs.Denote.Skolems unit)))
    readArguments := readArguments.push (← `(term| frame))
  if readsState then
    readBinders := readBinders.push (← `(bracketedBinder| (state : VerexIR.Proofs.Denote.Memory unit)))
    readArguments := readArguments.push (← `(term| state))
  -- What the lemma and those it applies assume, as hypotheses.
  let trust ← liftTermElabM (lemmaTrustClosure unit reference)
  let trustBinders ← trust.mapM fun name =>
    `(bracketedBinder| ($(mkIdent (trustBinderName name)) : $(rootIdent name)))
  let parameterTypes ← liftTermElabM (Contract.lemmaParameterTypes unit reference)
  let parameters := (declaration.signature.parameters.zip parameterTypes).mapIdx
    fun index (parameter, type) => (mkIdent (Name.mkSimple s!"{parameter.name}_{index}"), type,
      index)
  let parameterBinders ← parameters.mapM fun (name, type, index) => do
    match type with
    | .inl scalar => `(bracketedBinder| ($name : $(← lemmaTypeSyntax scalar)))
    | .inr nty =>
        -- A native parameter's carrier at the lemma's frame, behind a
        -- reducible constant the binder spells.
        let typeName := Name.str names.requires.getPrefix s!"lemmaParameter_{index}"
        unless (← getEnv).contains typeName do
          liftTermElabM do
            let value ← Contract.withUnitVariable .default fun unitExpr functions =>
              withLocalDeclD `frame
                  (mkApp2 (mkConst ``VerexIR.Proofs.Denote.Skolems) unitExpr functions)
                fun frame => do
                  mkLambdaFVars #[unitExpr, functions, frame]
                    (Contract.carrierType (← Contract.frameCarriers frame) (← quoteNTy nty))
            let type ← inferType value
            addDecl <| .defnDecl
              { name := typeName, levelParams := [], type, value, hints := .abbrev, safety := .safe }
            setReducibleAttribute typeName
        `(bracketedBinder| ($name : $(rootIdent typeName) unit frame))
  let bundle ← parameters.foldrM (init := ← `(term| ())) fun (name, _, _) rest =>
    `(term| ($name, $rest))
  let premise := mkIdent `premise
  let stated := mkIdent `statedPremise
  let requires := rootIdent names.requires
  let ensures := rootIdent names.ensures
  -- Each step proved, then what it gives assumed; a split continues once per
  -- case, so each later tactic runs on every case.
  let mut steps : Array (TSyntax `tactic) := #[]
  for (condition, stepName) in declaration.proof.zip names.steps do
    let step := mkIdent (Name.mkSimple s!"step_{steps.size}")
    match condition.kind with
    | .assumption =>
        -- An assumed step holds by its hypothesis, of the premise and the
        -- assumptions before it.
        let some (_, trusted) := names.trusted.find? (·.1 == steps.size)
          | throwError "an assumed step has no hypothesis"
        let earlier := (names.trusted.filter (·.1 < steps.size)).map fun (index, _) =>
          (mkIdent (Name.mkSimple s!"assumed_{index}") : Term)
        let assumed := mkIdent (Name.mkSimple s!"assumed_{steps.size}")
        steps := steps.push (← `(tactic| all_goals
          (have $assumed : $(rootIdent stepName) $readArguments* $bundle :=
             $(mkIdent (trustBinderName trusted)) $readArguments* $bundle $stated $earlier*
           have $step := $assumed
           simp only [$(rootIdent stepName):ident] at $step:ident
           try verex_normalize at $step:ident
           verex_split_hypotheses)))
    | _ =>
        steps := steps.push (← `(tactic| all_goals
          (have $step : $(rootIdent stepName) $readArguments* $bundle := by
             simp only [$(rootIdent stepName):ident]
             verex_lemma_owed
           simp only [$(rootIdent stepName):ident] at $step:ident
           verex_lemma_holds $step)))
  let conclusion ← match block? with
    | none => `(tactic| all_goals
        (simp only [$ensures:ident]
         try verex_normalize
         all_goals verex_solve_goals))
    | some block =>
        let spelled := parameters.map fun (_, _, index) =>
          Syntax.mkStrLit (declaration.signature.parameters[index]!.name)
        let binders := parameters.map (·.1)
        `(tactic| all_goals
          (simp only [$ensures:ident, VerexIR.Proofs.Obligation_eq]
           verex_lemma_steps $[$spelled $binders]* => $(⟨block⟩)))
  let tactics : Array (TSyntax `tactic) :=
    -- The premise as stated, which an assumed step's hypothesis takes.
    #[← `(tactic| have $stated := $premise),
      ← `(tactic| simp only [$requires:ident, VerexIR.Proofs.Obligation_eq] at $premise:ident),
      ← `(tactic| try verex_normalize at $premise:ident),
      ← `(tactic| verex_split_hypotheses)] ++ steps ++ #[conclusion]
  let script ← `(tactic| ($tactics;*))
  let theoremName := rootIdent names.theorem_
  unless recursive do
    return ← `(command|
      theorem $theoremName $trustBinders* $readBinders* $parameterBinders*
          ($premise : $requires $readArguments* $bundle) : $ensures $readArguments* $bundle := by
        $script:tactic)
  -- The measure: the declared components, or the integer parameters.
  let measures ← if names.measures.isEmpty then
      (parameters.filter (·.2.1 == .inl (mkConst ``Int))).mapM fun (name, _) =>
        `(term| VerexIR.Proofs.lemmaMeasure $name)
    else names.measures.mapM fun measure =>
      `(term| VerexIR.Proofs.lemmaMeasure ($(rootIdent measure) $readArguments* $bundle))
  let some last := measures.back?
    | throwError m!"a recursive lemma needs a `decreases` measure: it has no integer parameter"
  let measure ← measures.pop.foldrM (init := last) fun component rest =>
    `(term| ($component, $rest))
  let measureDefinitions : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
    names.measures.mapM fun measure =>
      `(Lean.Parser.Tactic.simpLemma| $(rootIdent measure):ident)
  `(command|
    theorem $theoremName $trustBinders* $readBinders* $parameterBinders*
        ($premise : $requires $readArguments* $bundle) : $ensures $readArguments* $bundle := by
      $script:tactic
    termination_by $measure
    decreasing_by
      all_goals
        ((try simp only [$measureDefinitions,*])
         verex_lemma_decreasing))

/-- Verify the lemmas `roots` reach, a recursion group at a time, each after
the groups it applies; a group whose theorems exist is skipped. -/
def verifyLemmas (unit : ValidatedUnit) (segments : Array String)
    (roots : Array VerexIR.QualifiedRef) (items : Array (String × Syntax)) (reference : Syntax)
    (blocks : Array (VerexIR.QualifiedRef × Syntax) := #[]) (steps : StepApplications := #[]) :
    CommandElabM Unit := do
  if roots.isEmpty then return
  let budget := verex.verifyHeartbeats.get (← getOptions)
  for group in lemmaGroups unit roots steps do
    let names ← liftTermElabM <| group.mapM fun lemma => Contract.lemmaNames unit lemma
    let environment ← getEnv
    if names.all fun names => environment.contains names.theorem_ then continue
    let recursive := group.size > 1 ||
      group.any fun lemma => (appliedLemmas unit lemma steps).contains lemma
    -- A group is reported at its first lemma this module declares.
    let reference := (group.findSome? fun lemma => do
      let qualified ← unit.tables.names[lemma.name.index]?
      (items.find? (·.1 == qualified.name)).map (·.2)).getD reference
    let spelled := ", ".intercalate (group.filterMap fun lemma =>
      (unit.tables.names[lemma.name.index]?).map (s!"`{·.name}`")).toList
    let errorsBefore := countErrors (← get).messages
    -- A group that fails leaves the others, and the functions, to be tried.
    try
      -- Built at the lemma, so that what fails in it is reported there.
      let theorems ← withRef reference <|
        group.mapM fun lemma =>
          lemmaTheoremCommand unit lemma recursive ((blocks.find? (·.1 == lemma)).map (·.2))
      let command ← if group.size == 1 then pure theorems[0]!
        else `(command| mutual $[$theorems]* end)
      let budgetSyntax := Syntax.mkNumLit (toString budget)
      stageLog s!"lemmas {names.map (·.theorem_)}: theorem start"
      withRef reference do
        Perf.withPhase .verification <|
          elabCommand (← `(command| set_option Elab.async false in
            set_option maxHeartbeats $budgetSyntax in $command:command))
      stageLog s!"lemmas {names.map (·.theorem_)}: theorem done"
      if countErrors (← get).messages > errorsBefore then
        logErrorAt reference m!"verex verification failed: the lemma {spelled} is not established"
      else
        for lemmaNames in names do
          modifyEnv (VerexIR.Proofs.Denote.establishedLemmas.tag · lemmaNames.theorem_)
    catch error =>
      logErrorAt reference m!"verex verification failed: the lemma {spelled} is not \
        established: {error.toMessageData}"

/-- The file syntax was parsed from, where a module's items and its proof
file's are verified together. -/
structure ParsedFile where
  name : String
  map : FileMap
  /-- The file being elaborated, the document, rather than another. -/
  document : Bool := false
  /-- Whether info trees are kept of the file's syntax: the document's, as
  where its elaboration began; never another file's, whose positions the
  document does not have. -/
  infoTrees : Bool := false
  deriving Inhabited

/-- Whether syntax of `file?`, `none` for the command's own file, is the
document's. -/
def ParsedFile.isDocument : Option ParsedFile → Bool
  | some file => file.document
  | none => true

/-- Run `act` with the positions of `file`, or of the command's own file:
a message at syntax lands in the file the syntax was parsed from. -/
def inParsedFile (file? : Option ParsedFile) (act : CommandElabM α) : CommandElabM α :=
  match file? with
  | some file =>
      withReader (fun context => { context with fileName := file.name, fileMap := file.map })
        (withEnableInfoTree file.infoTrees act)
  | none => act

/-- Report every message about another file logged since the log held
`since` messages at `ref` instead, opened by its own position, unless
`verex.relocateMessages` is off: a document shows messages at its own
positions only. -/
def relocateForeignMessages (ref : Syntax) (since : Nat := 0) : CommandElabM Unit := do
  unless verex.relocateMessages.get (← getOptions) do return
  let fileName ← getFileName
  let fileMap ← getFileMap
  let some start := ref.getPos? | return
  let pos := fileMap.toPosition start
  let endPos := ref.getTailPos?.map fileMap.toPosition
  modify fun state =>
    let messages := state.messages.unreported.toArray
    let since := if since ≤ messages.size then since else 0
    let relocated := messages.extract since |>.map fun message =>
      if message.fileName == fileName then message
      else { message with
        fileName, pos, endPos
        data := m!"{message.fileName}:{message.pos.line}:{message.pos.column + 1}: " ++
          message.data }
    let unreported := (messages.extract 0 since ++ relocated).toPArray'
    { state with messages := { state.messages with unreported } }

/-- How many messages the log holds that are not reported yet. -/
def unreportedCount : CommandElabM Nat := do
  return (← get).messages.unreported.size

/-! ## Proof tasks

A target's typed theorem is elaborated with `Elab.async`: its statement by
the command, its proof in a task of its own, so that the targets of a
module are proved at once. The task reports what the command cannot see:
the proof's cost on its thread, and the leaves the solver could not decide,
whose state is the theorem's and released when its elaboration ends. The
command concludes the target once the task ended: its messages reported in
the target's turn, its failure raised, its artifacts carried to the runtime
and audited. -/

/-- What a proof's task reports to the command concluding its target. -/
structure ProofReport where
  heartbeats : Nat := 0
  elapsedMs : Nat := 0
  /-- The leaves the automatic proof did not decide, in report order. -/
  failedLeaves : Array String := #[]
  /-- What the proof printed, under the solver's debugging and profiling
  options: a task's streams are its own, so the command prints it in the
  target's turn. -/
  output : String := ""
  deriving Inhabited

/-- The reports of the proofs that ended, by theorem, until their targets
conclude. -/
initialize proofReports : IO.Ref (NameMap ProofReport) ← IO.mkRef {}

/-- Take the report of `theorem_`'s proof; a proof whose task ended before
its tactics ran, at its statement, reports nothing. -/
def takeProofReport (theorem_ : Name) : IO ProofReport :=
  proofReports.modifyGet fun reports =>
    ((reports.find? theorem_).getD default, reports.erase theorem_)

/-- `verex_proof t => tactics` proves `t`'s goal by `tactics`, and reports
the proof's heartbeats on its thread since its elaboration began, its wall
time, the leaves the solver left undecided, and what it printed, however
the tactics end; the heartbeats are charged to the verification phase. -/
elab "verex_proof " theorem_:ident " => " proof:Lean.Parser.Tactic.tacticSeq : tactic => do
  let started ← IO.monoNanosNow
  -- The solver's state of this proof, allocated before any tactic can
  -- backtrack past it.
  let state ← VerexIR.Proofs.Denote.targetState
  state.proofName.set ((← Term.getDeclName?).getD theorem_.getId)
  state.selectorUses.set #[]
  state.scopeNames.set none
  state.scopeLocals.set #[]
  state.routedKeys.set none
  let main ← Lean.Elab.Tactic.getMainGoal
  -- The streams are the thread's: what the proof prints is kept for the
  -- report, as the tactics leave it. An exception passes with the state
  -- the failing tactic left, as it does without the wrapper.
  let printed ← IO.mkRef ({} : IO.FS.Stream.Buffer)
  try
    IO.withStdout (.ofBuffer printed) <| IO.withStderr (.ofBuffer printed) <|
      Lean.Elab.Tactic.evalTactic proof
    VerexIR.Proofs.Denote.reportUnusedSelectors
    VerexIR.Proofs.Denote.declareHiddenTheorems (← instantiateMVars (mkMVar main))
  finally
    state.proofName.set .anonymous
    -- A runtime value an authored step's goal stated, reported once.
    VerexIR.Proofs.Denote.reportRuntimeMention
    let heartbeats := (← IO.getNumHeartbeats) - (← readThe Core.Context).initHeartbeats
    let elapsedMs := ((← IO.monoNanosNow) - started) / 1000000
    let failedLeaves ← VerexIR.Proofs.Denote.failedLeafNames
    let output := String.fromUTF8! (← printed.get).data
    Perf.chargeHeartbeats .verification heartbeats
    proofReports.modify
      (·.insert theorem_.getId { heartbeats, elapsedMs, failedLeaves, output })

/-- A target whose proofs elaborate in tasks, and how to conclude it once
they ended. -/
structure PendingVerification where
  /-- The elaboration tasks of its theorems, taken from the command's: their
  messages are reported in the target's turn, as the command's own are. -/
  tasks : Array (Language.SnapshotTask Language.SnapshotTree) := #[]
  /-- Conclude the target once its tasks ended: report their messages, raise
  its failure, carry and audit its artifacts. -/
  conclude : CommandElabM Unit := pure ()

/-- The last component of the next proof attempt of `base`'s typed theorem:
a proof elaborating in a task is committed whether or not it succeeded, so
each attempt is a theorem of its own, `base.typedVerified.attempt_<n>`, and
`base.typedVerified` is declared once an attempt proved it
(`declareTypedTheorem`). A failed attempt leaves nothing a later attempt
collides with. -/
private partial def attemptSuffix (env : Environment) (base : Name) (index : Nat := 1) : Name :=
  let suffix := Name.mkSimple s!"attempt_{index}"
  if env.contains (base ++ `typedVerified ++ suffix) then attemptSuffix env base (index + 1)
  else suffix

/-- Declare `base.typedVerified` as the attempt `attempt` that proved it. -/
private def declareTypedTheorem (base attempt : Name) : CommandElabM Unit := do
  let some info := (← getEnv).find? attempt | throwError m!"missing proof attempt `{attempt}`"
  liftCoreM <| addDecl (.thmDecl {
    name := base ++ `typedVerified
    levelParams := info.levelParams
    type := info.type
    value := mkConst attempt (info.levelParams.map mkLevelParam) })

/-- Wait for the target's tasks and conclude it. -/
def PendingVerification.finish (pending : PendingVerification) : CommandElabM Unit := do
  for task in pending.tasks do discard <| IO.wait task.task
  pending.conclude

/-- Run `act`, taking the proof tasks of the theorems it elaborates out of
the command's tasks: those carrying a cancellation token, as a theorem's
proof task does (`Lean.Elab.Command.elabMutualDef`); the task showing the
kernel's progress, which carries none, stays with the command, as the
kernel's checks chain behind every proof still running. -/
def proofTasks (act : CommandElabM α) :
    CommandElabM (α × Array (Language.SnapshotTask Language.SnapshotTree)) := do
  let before := (← get).snapshotTasks.size
  let result ← act
  let pushed := (← get).snapshotTasks.extract before (← get).snapshotTasks.size
  let (proofs, kept) := pushed.partition (·.cancelTk?.isSome)
  modify fun state => { state with
    snapshotTasks := state.snapshotTasks.extract 0 before ++ kept }
  return (result, proofs)

/-- Report the messages of an ended snapshot tree, in its order, after the
command's own: whether an error is among them and whether a proof ran out
of its budget. A child still running, the kernel's check of what the proof
declared, is left to the command, which reports it when it ends. -/
private partial def reportFinished (tree : Language.SnapshotTree) (failed overBudget : Bool) :
    CommandElabM (Bool × Bool) := do
  let mut failed := failed
  let mut overBudget := overBudget
  for message in tree.element.diagnostics.msgLog.toList do
    -- Added as the task logged it, in its own naming context.
    modify fun state => { state with messages := state.messages.add message }
    if message.severity == .error then failed := true
    if message.data.hasTag (· == `runtime.maxHeartbeats) then overBudget := true
  for child in tree.children do
    if ← IO.hasFinished child.task then
      (failed, overBudget) ← reportFinished child.task.get failed overBudget
    else
      modify fun state => { state with snapshotTasks := state.snapshotTasks.push child }
  return (failed, overBudget)

/-- Wait for a theorem's proof tasks and report their messages
(`reportFinished`): whether the proof failed, counting an error of the
statement, and whether it ran out of its budget. -/
def reportProofMessages (tasks : Array (Language.SnapshotTask Language.SnapshotTree))
    (statementFailed : Bool) : CommandElabM (Bool × Bool) := do
  let mut failed := statementFailed
  let mut overBudget := false
  for task in tasks do
    (failed, overBudget) ← reportFinished (← IO.wait task.task) failed overBudget
  return (failed, overBudget)

/-- The natives, in-body assumptions, and lemma steps a member's theorems
take. -/
abbrev MemberDependencies :=
  Array (FunctionHandle × String) × Array (FunctionHandle × String) × Array Name

/-- An authored proof, and the file it was written in where that is not the
command's own. -/
structure AuthoredScript where
  tactic : TSyntax ``Lean.Parser.Tactic.tacticSeq
  file : Option ParsedFile := none
  /-- A proof in steps, whose leaves read the function's scope through
  scope entries (`ScopeEntry`). -/
  steps : Bool := false

/-- Whether a model's proof routes a generic function: a callee it reaches,
inlined, through its contract, or native, that is generic. Such a proof is over every
skolem family and type instantiation, as a cycle's through a generic member
is (`doc/designs/ghost-fields.md`). -/
private def modelRoutesGeneric (unit : ValidatedUnit) (compiled : Function unit) : Bool := Id.run do
  let mut worklist := compiled.body.callees #[]
  let mut visited : Array FunctionHandle := #[]
  while let some callee := worklist.back? do
    worklist := worklist.pop
    if visited.contains callee then continue
    visited := visited.push callee
    let some calleeNs := unit.namespaces[callee.namespaceId.index]? | continue
    let some calleeDeclaration := calleeNs.functions[callee.functionId.index]? | continue
    if isGeneric calleeDeclaration then return true
    if calleeDeclaration.body == .absent then continue
    if usedThroughContract unit callee calleeNs calleeDeclaration then continue
    if let .ok calleeCompiled := compileModelFunction unit callee then
      worklist := calleeCompiled.body.callees worklist
  return false

/-- The binders of the meanings a proof's routed members stand for, each
with its hypothesis: a cycle's member, or a model's callee used through its
contract, satisfies its contract; a model's inlined callee refines its body
under the same meanings. At the runtime family, or at every skolem family
and type instantiation. -/
private def routedSelfBinders (segments : Array String) (unit : ValidatedUnit) (unitTerm : Term)
    (namespaceName : Name) (routed : Array (FunctionHandle × String)) (family : Bool)
    (inlinedMember : String → Bool) (meanings : Term) (atRuntime : Term → CommandElabM Term) :
    CommandElabM (Array (TSyntax ``Lean.Parser.Term.bracketedBinder)) := do
  -- Every member's meaning is bound before any hypothesis: a hypothesis's
  -- meanings name them all, and an unbound name would be bound implicitly
  -- in front, as a meaning of its own.
  let pairs ← routed.mapM fun (member, name) => do
    let memberArtifacts := Name.str namespaceName name
    let memberParams := rootIdent (memberArtifacts ++ `params)
    let memberShape := rootIdent (memberArtifacts ++ `shape)
    let memberContract := rootIdent (typedContractName segments name)
    if family && inlinedMember name then
      -- At every family and instantiation: the body under the meanings
      -- there, which name the members at that family.
      let memberCompiled := rootIdent (compiledName segments name)
      pure (← `(bracketedBinder| ($(cycleSelf name) :
            VerexIR.Proofs.Denote.SelfFamily $unitTerm $memberParams $memberShape)),
        ← `(bracketedBinder| ($(cycleSelfRefines name) :
            ∀ (Θ : VerexIR.Proofs.Denote.Skolems $unitTerm)
              (typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId)) args,
            VerexIR.Proofs.Spec.Refines ($(cycleSelf name) Θ typeInstantiation args)
              (VerexIR.Proofs.Denote.Function.denoteWith $meanings $memberCompiled args))))
    else if family then
      let atFamily ← atUnit segments unit (typedContractName segments name) memberContract
        (some (← `(term| family)))
        (← if (declarationOf? unit member).any isGeneric then
          pure #[← `(term| instantiation)] else pure #[])
      pure (← `(bracketedBinder| ($(cycleSelf name) :
            VerexIR.Proofs.Denote.SelfFamily $unitTerm $memberParams $memberShape)),
        ← `(bracketedBinder| ($(cycleSelfVerified name) :
            ∀ (family : VerexIR.Proofs.Denote.Skolems $unitTerm)
              (instantiation : Array (VerexIR.TypeId × VerexIR.TypeId)),
            VerexIR.Proofs.Satisfies ($(cycleSelf name) family instantiation) $atFamily)))
    else if inlinedMember name then
      let memberCompiled := rootIdent (compiledName segments name)
      let refinesBody ← `(term|
        ∀ args, VerexIR.Proofs.Spec.Refines ($(cycleSelf name) args)
          (VerexIR.Proofs.Denote.Function.denoteWith $meanings $memberCompiled args))
      let refines ← atRuntime refinesBody
      pure (← `(bracketedBinder| ($(cycleSelf name) :
            @VerexIR.Proofs.Denote.HList
              (VerexIR.Proofs.Denote.Carriers.runtime $unitTerm) $memberParams →
            VerexIR.Proofs.Denote.Comp $unitTerm
              (@VerexIR.Proofs.Denote.ResultShape.carrier
                (VerexIR.Proofs.Denote.Carriers.runtime $unitTerm) $memberShape))),
        ← `(bracketedBinder| ($(cycleSelfRefines name) : $refines)))
    else
      -- At the runtime frame, spelled out: binders precede the statement's.
      pure (← `(bracketedBinder| ($(cycleSelf name) :
            @VerexIR.Proofs.Denote.HList
              (VerexIR.Proofs.Denote.Carriers.runtime $unitTerm) $memberParams →
            VerexIR.Proofs.Denote.Comp $unitTerm
              (@VerexIR.Proofs.Denote.ResultShape.carrier
                (VerexIR.Proofs.Denote.Carriers.runtime $unitTerm) $memberShape))),
        ← `(bracketedBinder| ($(cycleSelfVerified name) :
            VerexIR.Proofs.Satisfies $(cycleSelf name)
              $(← atUnit segments unit (typedContractName segments name) memberContract
                (some (← `(term| ($(mkCIdent ``VerexIR.Proofs.Denote.Skolems.runtime) $unitTerm))))))))
  return pairs.map (·.1) ++ pairs.map (·.2)

/-- The binders of a model's invocation meaning (`doc/designs/ghost-fields.md`):
the meaning at every family; that an invocation of a function value at the
rows of its type is defined where its target's declared precondition holds,
and an invocation of a value no closure site encodes always (`RequiresOf`
of the definedness meaning, `functionRequires (fun … => True)`); that the meaning is coherent across the families type
arguments induce (`InvokeMeaning.Coherent`); and, for each routed member
without type parameters, that invoking its closure means the member's
meaning on the woven arguments: at the base frame its closure sites'
constructors, over every family its constructions.
The routing hypotheses are returned as the lemmas the normalizer rewrites
with. -/
private def modelInvokeBinders (segments : Array String) (unit : ValidatedUnit) (unitTerm : Term)
    (namespaceName : Name) (routed : Array (FunctionHandle × String)) (family : Bool) :
    CommandElabM (Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ×
      Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ×
      Array (TSyntax ``Lean.Parser.Tactic.simpLemma)) := do
  -- Invoking a value no closure site encodes is defined without a
  -- precondition, as the Move Prover calls function values without
  -- asserting `requires_of`; a site's value where its target's precondition holds.
  let requiresMeaning ← `(term| ($(rootIdent (functionRequiresName segments))
    (fun _ _ _ _ _ _ => True)))
  let runtimeFrame ← `(term| ($(mkCIdent ``VerexIR.Proofs.Denote.Skolems.runtime) $unitTerm))
  let meaning ← `(bracketedBinder| ($invokeIdent : (Θ : VerexIR.Proofs.Denote.Skolems $unitTerm) →
    @VerexIR.Proofs.Denote.InvokeMeaning $unitTerm _ Θ))
  let defined ← `(bracketedBinder| ($(mkIdent `verexInvokeDefined) :
    ∀ (Θ : VerexIR.Proofs.Denote.Skolems $unitTerm) (σs : VerexIR.Proofs.Denote.NRow)
      (shared : VerexIR.Proofs.Denote.Sharing) (shape : VerexIR.Proofs.Denote.ResultShape)
      (function : @VerexIR.Proofs.Denote.NTy.carrier
        (@VerexIR.Proofs.Denote.Skolems.toCarriers $unitTerm _ Θ)
        (VerexIR.Proofs.Denote.NTy.function σs shared (VerexIR.Proofs.Denote.ResultShape.row shape)))
      (args : @VerexIR.Proofs.Denote.HList (@VerexIR.Proofs.Denote.Skolems.toCarriers $unitTerm _ Θ) σs)
      (pre : VerexIR.Proofs.Denote.Memory $unitTerm),
      @VerexIR.Proofs.RequiresOf $unitTerm _ Θ $requiresMeaning σs shared
        (VerexIR.Proofs.Denote.ResultShape.row shape) function args pre →
      ¬($invokeIdent Θ σs shared shape function args).undefined pre))
  let coherent ← `(bracketedBinder| ($(mkIdent `verexInvokeCoherent) :
    @VerexIR.Proofs.Denote.InvokeMeaning.Coherent $unitTerm _ $invokeIdent))
  let mut routing : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
  let mut lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) := #[]
  -- Over every family, a construction of a routed member's closure at the
  -- family is routed to its meaning there on the woven arguments: no
  -- construction reduces at an abstract frame.
  if family then
    for (member, name) in routed do
      if (declarationOf? unit member).any isGeneric then continue
      let memberArtifacts := Name.str namespaceName name
      let params := rootIdent (memberArtifacts ++ `params)
      let shape := rootIdent (memberArtifacts ++ `shape)
      let handleTerm ← handleSyntax member
      let binderName := mkIdent (Name.mkSimple s!"invokeRoutes_{name}")
      routing := routing.push (← `(bracketedBinder| ($binderName :
        ∀ (Θ : VerexIR.Proofs.Denote.Skolems $unitTerm) {captured supplied : VerexIR.Proofs.Denote.NRow}
          (weave : VerexIR.Proofs.Denote.Weave $params captured supplied)
          (typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId))
          (captures : @VerexIR.Proofs.Denote.HList
            (@VerexIR.Proofs.Denote.Skolems.toCarriers $unitTerm _ Θ) captured)
          (shared : VerexIR.Proofs.Denote.Sharing)
          (typed : @VerexIR.Proofs.Denote.SiteTyped $unitTerm _ Θ supplied shared
            (VerexIR.Proofs.Denote.ResultShape.row $shape) $handleTerm
            (VerexIR.Proofs.Denote.Weave.mask weave) typeInstantiation captured)
          (args : @VerexIR.Proofs.Denote.HList
            (@VerexIR.Proofs.Denote.Skolems.toCarriers $unitTerm _ Θ) supplied),
          $invokeIdent Θ supplied shared $shape
              (@VerexIR.Proofs.Denote.Skolems.construct $unitTerm _ Θ supplied shared
                (VerexIR.Proofs.Denote.ResultShape.row $shape) $handleTerm
                (VerexIR.Proofs.Denote.Weave.mask weave) typeInstantiation captured captures typed)
              args =
            $(cycleSelf name) Θ typeInstantiation
              (@VerexIR.Proofs.Denote.Weave.compose $unitTerm _ Θ _ _ _ weave captures args))))
      lemmas := lemmas.push (← `(Lean.Parser.Tactic.simpLemma| $binderName:ident))
    return (#[meaning, defined, coherent], routing, lemmas)
  -- At the base frame, a closure site's constructor of a routed member.
  let environment ← getEnv
  let sites := siteSpecs (← closureSitesOf segments unit)
  for site in sites, index in [0:sites.size] do
    let some entry := siteEquationsAt? environment
        (functionValuesName segments ++ Name.mkSimple s!"site{index}") | continue
    let some (_, name) := routed.find? (·.1 == site.key.function) | continue
    if (declarationOf? unit site.key.function).any isGeneric then continue
    let memberArtifacts := Name.str namespaceName name
    let shape := rootIdent (memberArtifacts ++ `shape)
    let ctor := rootIdent entry.constructor
    let weave := rootIdent (functionValuesName segments ++ Name.mkSimple s!"weave{index}")
    let binderName := mkIdent (Name.mkSimple s!"invokeRoutes_{name}_{index}")
    routing := routing.push (← `(bracketedBinder| ($binderName : ∀ captures args,
      $invokeIdent $runtimeFrame _ _ $shape ($ctor captures) args =
        $(cycleSelf name)
          (@VerexIR.Proofs.Denote.Weave.compose $unitTerm _ $runtimeFrame _ _ _ $weave captures args))))
    lemmas := lemmas.push (← `(Lean.Parser.Tactic.simpLemma| $binderName:ident))
  return (#[meaning, defined, coherent], routing, lemmas)

/-- What the steps of `verifyMember` read of the function they verify:
its unit, declaration and denotation, how it is proved, and the names of
its artifacts. -/
private structure Member where
  reference : Syntax
  segments : Array String
  function : String
  script? : Option AuthoredScript
  cycle : Array (FunctionHandle × String)
  unit : ValidatedUnit
  namespaceName : Name
  namespaceIndex : Nat
  ns : ValidatedNamespace
  declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody
  handle : FunctionHandle
  compiled : Function unit
  /-- The unit declares ghost fields and is verified through its model. -/
  model : Bool
  /-- Proved over every skolem family and type instantiation. -/
  family : Bool
  generic : Bool
  /-- Proved for the frames it runs in (`FrameOf`). -/
  framed : Bool
  arity : Nat
  ownAssumptions : Bool
  artifacts : Name
  base : Name
  unitDefinition : Name
  unitTerm : Term
  handleTerm : Term

/-- The theorem of a verified callee, and the dependencies its theorems
take: the natives they assume, the functions whose in-body assumptions they
assume, and the lemma steps they trust. -/
private def addTheoremDependencies (theoremName : Name)
    (natives assumptionDeps : Array (FunctionHandle × String)) (trustDeps : Array Name) :
    CommandElabM (Array (FunctionHandle × String) × Array (FunctionHandle × String) × Array Name) := do
  let mut natives := natives
  let mut assumptionDeps := assumptionDeps
  let mut trustDeps := trustDeps
  for entry in ((nativeDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
    unless natives.any (·.1 == entry.1) do natives := natives.push entry
  for entry in ((assumptionDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
    unless assumptionDeps.any (·.1 == entry.1) do assumptionDeps := assumptionDeps.push entry
  for name in ((lemmaTrustDependencies.getState (← getEnv)).find? theoremName.getPrefix).getD #[] do
    unless trustDeps.contains name do trustDeps := trustDeps.push name
  return (natives, assumptionDeps, trustDeps)

/-- A callee standing by its contract as a hypothesis, unapplied, so a
generic native's family and instantiation are found at each call, not fixed
at the first. -/
private def nativePair (handleTerm : Term) (calleeKey : String) : CommandElabM Term :=
  `(term| PProd.mk $handleTerm (PProd.mk $(Syntax.mkStrLit calleeKey)
    @$(mkIdent (Name.mkSimple s!"native_{calleeKey}"))))

/-- A callee known by its theorem. -/
private def theoremPair (handleTerm : Term) (calleeKey : String) (theoremName : Name) :
    CommandElabM Term :=
  `(term| PProd.mk $handleTerm
    (PProd.mk $(Syntax.mkStrLit calleeKey) (@$(mkIdent theoremName) _ _ prepared)))

/-- The generic calls of the bodies proved at the empty instantiation: the
target's, when it has no type parameters, and every inlined callee's. -/
private def recordGenericCall (callee : FunctionHandle) (typeArgs : Array VerexIR.TypeUse)
    (_ : Option TypeArgs) (found : Array (FunctionHandle × Array VerexIR.TypeUse)) :=
  if typeArgs.isEmpty || found.contains (callee, typeArgs) then found
  else found.push (callee, typeArgs)

/-- The callees a proof meets, collected through every inlined body. -/
private structure Callees (unit : ValidatedUnit) where
  calleePairs : Array Term := #[]
  /-- Under a model: the callees standing as meanings, each with whether it is
  inlined (`doc/designs/ghost-fields.md`). -/
  modelMembers : Array (FunctionHandle × String × Bool) := #[]
  /-- The inlined callees, whose loops the proof meets with their invariants. -/
  inlined : Array (FunctionHandle × ValidatedNamespace ×
    VerexIR.FunctionDecl VerexIR.Validation.FunctionBody × Function unit) := #[]
  /-- The natives the theorems assume: those called here or by an inlined
  callee, and those a verified callee assumes. -/
  natives : Array (FunctionHandle × String) := #[]
  /-- The functions whose in-body assumptions the theorems assume to hold:
  its own, and those a verified callee assumes. -/
  assumptionDeps : Array (FunctionHandle × String) := #[]
  /-- The assumed steps of the lemmas its own proof applies, and those a
  verified callee's theorems take. -/
  trustDeps : Array Name := #[]
  edges : Array (FunctionHandle × Array FunctionHandle) := #[]
  instantiated : Array (FunctionHandle × Array VerexIR.TypeUse) := #[]
  behaviorPairs : Array Term := #[]
  behaviorTargets : Array FunctionHandle := #[]
  /-- The targets of the closures the contracts the proof meets name: its own
  and those of the callees it uses through their contracts. -/
  contractTargets : Array FunctionHandle := #[]
  /-- The contract each callee used through one is met by, which its generic
  calls meet specialized. -/
  contracts : Array (FunctionHandle × Name) := #[]

/-- Whether a callee's runtime theorem `theoremName` exists and its
target's verification was audited. -/
private def auditedTheorem (env : Environment) (theoremName : Name) : Bool :=
  env.contains theoremName &&
    auditedDenotations.isTagged env (theoremName.getPrefix ++ `typedVerified)

/-- Collects the callees of `m`'s body and of every inlined callee's, from
`worklist`, into `found`: each by its contract as a hypothesis, by its
theorem, or inlined; a closure's target with a theorem by it as well. -/
private def collectCallees (m : Member) (members : Array FunctionHandle)
    (worklist : Array FunctionHandle) (found : Callees m.unit) : CommandElabM (Callees m.unit) := do
  let { reference, segments, function, namespaceName, handle, model, generic, .. } := m
  let unit := m.unit
  let targetsOfClosures := closureTargets unit
  let mut calleePairs := found.calleePairs
  let mut modelMembers := found.modelMembers
  let mut inlined := found.inlined
  let mut natives := found.natives
  let mut assumptionDeps := found.assumptionDeps
  let mut trustDeps := found.trustDeps
  let mut edges := found.edges
  let mut instantiated := found.instantiated
  let mut behaviorPairs := found.behaviorPairs
  let mut behaviorTargets := found.behaviorTargets
  let mut contractTargets := found.contractTargets
  let mut contracts := found.contracts
  let mut worklist := worklist
  let mut visited : Array FunctionHandle := #[handle] ++ members
  while let some callee := worklist.back? do
    worklist := worklist.pop
    if visited.contains callee then continue
    visited := visited.push callee
    let some calleeNs := unit.namespaces[callee.namespaceId.index]?
      | throwErrorAt reference "a callee's namespace is out of range"
    let some calleeDeclaration := calleeNs.functions[callee.functionId.index]?
      | throwErrorAt reference "a callee is out of range"
    let calleeKey := functionKey unit callee
    let theoremName := (← getCurrNamespace) ++ typedSemanticsVerifiedName segments calleeKey
    let handleTerm ← handleSyntax callee
    let usesContract := contractStandsFor unit callee.namespaceId calleeNs calleeDeclaration
    let throughContract := usedThroughContract unit callee calleeNs calleeDeclaration
    if throughContract || calleeDeclaration.body == .absent then
      for target in contractClosureTargets unit callee.namespaceId calleeNs calleeDeclaration do
        unless contractTargets.contains target do contractTargets := contractTargets.push target
    -- A callee reading an intrinsic map's representation has no denotation
    -- (`doc/designs/intrinsic-maps.md`, "Abstract representation"): it is used
    -- through its specification, as the Move Prover uses an opaque
    -- function. A role's body is the model's, whatever it reads.
    let representation? :=
      if (Contract.mapRoleOf? unit callee.namespaceId calleeNs calleeDeclaration).isSome then none
      else Contract.representationRead? unit calleeNs calleeDeclaration
    if let some owner := representation? then
      let ownerName := ((unit.tables.names[owner.name.index]?).map (·.name)).getD "?"
      unless calleeDeclaration.contract.loc.isSome do
        throwErrorAt reference m!"`{function}` calls `{calleeKey}`, which reads the \
          representation of the intrinsic map `{ownerName}`; such a function is used through \
          its specification: specify `{calleeKey}`"
    if calleeDeclaration.body == .absent then
      -- A native has no body: its contract is assumed, as a hypothesis. One
      -- without a specification is assumed by the model the Move Prover's
      -- prelude gives it (`NativeModel`), or by the intrinsic map role it
      -- plays (`doc/designs/intrinsic-maps.md`, "Roles"), and rejected without
      -- either, as the Prover rejects it.
      let nativeModel := nativeModelOf? unit callee calleeNs calleeDeclaration
      let playsRole := (Contract.mapRoleOf? unit callee.namespaceId calleeNs calleeDeclaration).isSome
      if readsNativeModel calleeDeclaration && nativeModel.isNone && !playsRole then
        throwErrorAt reference m!"`{function}` calls the native `{calleeKey}`, which has \
          neither a specification nor a prelude model; specify it"
      unless usesContract do
        throwErrorAt reference m!"`{function}` calls the native `{calleeKey}`, which returns \
          a mutable reference without stating its `final` value: a caller reasons over such a \
          callee through its body, and a native has none"
      let some (nativeParams, nativeResult) :=
          nativeSignature? unit callee.namespaceId calleeDeclaration
        | throwErrorAt reference m!"the native `{calleeKey}` has no native signature"
      ensureContracts segments calleeKey unit callee.namespaceId.index calleeNs
        calleeDeclaration nativeParams nativeResult nativeModel
      liftTermElabM (publishNativeSignature segments calleeKey nativeParams nativeResult)
      -- Under a model a native is a meaning assumed to satisfy its contract,
      -- as the Prover assumes a native's specification: its runtime run holds
      -- no ghost contents (`doc/designs/ghost-fields.md`).
      if model then
        unless modelMembers.any (·.1 == callee) do
          modelMembers := modelMembers.push (callee, calleeKey, false)
        continue
      unless natives.any (·.1 == callee) do natives := natives.push (callee, calleeKey)
      calleePairs := calleePairs.push (← nativePair handleTerm calleeKey)
      contracts := contracts.push (callee, typedContractName segments calleeKey)
    else if model then
      -- Under a model a callee stands as a meaning: one used through its
      -- contract is assumed to satisfy it, as the Move Prover assumes an
      -- opaque callee's specification; any other is inlined, its body under
      -- the same meanings.
      let some (calleeParams, calleeResult) :=
          nativeSignature? unit callee.namespaceId calleeDeclaration
        | throwErrorAt reference m!"`{calleeKey}` has no native signature"
      if throughContract then
        ensureContracts segments calleeKey unit callee.namespaceId.index calleeNs
          calleeDeclaration calleeParams calleeResult
        liftTermElabM (publishNativeSignature segments calleeKey calleeParams calleeResult)
        modelMembers := modelMembers.push (callee, calleeKey, false)
      else
        let calleeCompiled ← match compileModelFunction unit callee with
          | .ok compiled => pure compiled
          | .error reason =>
              throwErrorAt reference m!"no denotation for the callee `{calleeKey}`: {reason}"
        liftTermElabM (publishCompiled segments unit calleeKey callee calleeCompiled)
        inlined := inlined.push (callee, calleeNs, calleeDeclaration, calleeCompiled)
        worklist := calleeCompiled.body.callees worklist
        unless generic do
          instantiated := calleeCompiled.body.foldCalls recordGenericCall (fun _ _ found => found) instantiated
        edges := edges.push (callee, calleeCompiled.body.callees #[])
        modelMembers := modelMembers.push (callee, calleeKey, true)
    else if throughContract &&
        (hasInterfaceView calleeDeclaration || automaticVerificationDisabled unit callee.namespaceId calleeNs calleeDeclaration ||
          (Contract.mapRoleOf? unit callee.namespaceId calleeNs calleeDeclaration).isSome ||
          !auditedTheorem (← getEnv) theoremName &&
            !(inProgressTargets.getState (← getEnv)).contains calleeKey) then
      -- Callers see such a callee through the interface its body is not
      -- proved against, through the contract of a function whose
      -- verification is disabled, or through the contract of one without an
      -- audited theorem, whose verification failed or was not run: they assume it,
      -- as they assume a native's contract, and as the Move Prover verifies
      -- a caller against its callees' specifications. The callee's own
      -- failure is reported where it is verified.
      let some (nativeParams, nativeResult) :=
          nativeSignature? unit callee.namespaceId calleeDeclaration
        | throwErrorAt reference m!"`{calleeKey}` has no native signature"
      ensureInterfaceContract segments calleeKey unit callee.namespaceId.index calleeNs
        calleeDeclaration nativeParams nativeResult
      liftTermElabM (publishNativeSignature segments calleeKey nativeParams nativeResult)
      unless natives.any (·.1 == callee) do natives := natives.push (callee, calleeKey)
      calleePairs := calleePairs.push (← nativePair handleTerm calleeKey)
      contracts := contracts.push (callee, typedInterfaceContractName segments calleeKey)
    else if auditedTheorem (← getEnv) theoremName && throughContract then
      let (natives', assumptionDeps', trustDeps') ←
        addTheoremDependencies theoremName natives assumptionDeps trustDeps
      natives := natives'; assumptionDeps := assumptionDeps'; trustDeps := trustDeps'
      calleePairs := calleePairs.push (← theoremPair handleTerm calleeKey theoremName)
      contracts := contracts.push (callee, typedContractName segments calleeKey)
    else if throughContract then
      throwErrorAt reference m!"`{function}` calls `{calleeKey}`, whose verification depends \
        on `{function}`'s own: `{calleeKey}` reaches `{function}` through the function values \
        or callees its proof uses"
    else
      -- Any other callee is inlined: its compiled body stands for its
      -- meaning through the agreement theorem.
      let calleeCompiled ← match compileFunction unit callee with
        | .ok compiled => pure compiled
        | .error reason =>
            throwErrorAt reference m!"no denotation for the callee `{calleeKey}`: {reason}"
      liftTermElabM (publishCompiled segments unit calleeKey callee calleeCompiled)
      inlined := inlined.push (callee, calleeNs, calleeDeclaration, calleeCompiled)
      worklist := calleeCompiled.body.callees worklist
      -- As the target's own: a generic target's calls are proved over every
      -- family and instantiation, with no empty-instantiation frames.
      unless generic do
        instantiated := calleeCompiled.body.foldCalls recordGenericCall (fun _ _ found => found) instantiated
      edges := edges.push (callee, calleeCompiled.body.callees #[])
      let certificate := Name.str (Name.str namespaceName calleeKey) "compiledSemantics"
      unless (← getEnv).contains certificate do
        elabCommand (← `(command|
          set_option Elab.async false in
          theorem $(rootIdent certificate) :
              VerexIR.Proofs.Denote.compileFunction $(m.unitTerm) $handleTerm =
                .ok $(rootIdent (compiledName segments calleeKey)) :=
            $(rootIdent (compiledEqName segments calleeKey))))
      calleePairs := calleePairs.push
        (← `(term| PProd.mk $handleTerm
          (PProd.mk $(Syntax.mkStrLit calleeKey) $(rootIdent certificate))))
      -- A closure's target with a theorem is known by it as well, after the
      -- body a call takes: what `ensures_of` and `aborts_of` of a closure
      -- over it entail.
      if targetsOfClosures.contains callee && (← getEnv).contains theoremName then
        let (natives', assumptionDeps', trustDeps') ←
          addTheoremDependencies theoremName natives assumptionDeps trustDeps
        natives := natives'; assumptionDeps := assumptionDeps'; trustDeps := trustDeps'
        behaviorPairs := behaviorPairs.push (← theoremPair handleTerm calleeKey theoremName)
        behaviorTargets := behaviorTargets.push callee
  for target in contractTargets do
    if behaviorTargets.contains target then continue
    let targetKey := functionKey unit target
    let theoremName := (← getCurrNamespace) ++ typedSemanticsVerifiedName segments targetKey
    unless (← getEnv).contains theoremName do continue
    let (natives', assumptionDeps', trustDeps') ←
      addTheoremDependencies theoremName natives assumptionDeps trustDeps
    natives := natives'; assumptionDeps := assumptionDeps'; trustDeps := trustDeps'
    behaviorPairs := behaviorPairs.push (← theoremPair (← handleSyntax target) targetKey theoremName)
    behaviorTargets := behaviorTargets.push target
  return {
    calleePairs := calleePairs
    modelMembers := modelMembers
    inlined := inlined
    natives := natives
    assumptionDeps := assumptionDeps
    trustDeps := trustDeps
    edges := edges
    instantiated := instantiated
    behaviorPairs := behaviorPairs
    behaviorTargets := behaviorTargets
    contractTargets := contractTargets
    contracts := contracts }

/-- The type uses `typeArgs` as a term. -/
private def typeUsesTerm (typeArgs : Array VerexIR.TypeUse) : CommandElabM Term := do
  let typeUses ← typeArgs.mapM fun (typeUse : VerexIR.TypeUse) => do
    `(term| (⟨⟨$(Syntax.mkNatLit typeUse.typeId.index)⟩, ⟨$(Syntax.mkNatLit typeUse.loc.index)⟩⟩ :
      VerexIR.TypeUse))
  `(term| (#[$typeUses,*] : Array VerexIR.TypeUse))

/-- The certificate of a generic call at the empty instantiation, the
`index`th of `m`'s: it instantiates its callee's frame as the runtime
computes it, a kernel-checked literal the call's meaning is stated at. -/
private def frameCertificate (m : Member) (callee : FunctionHandle)
    (typeArgs : Array VerexIR.TypeUse) (index : Nat) : CommandElabM Term := do
  let { reference, segments, function, artifacts, unitTerm, .. } := m
  let unit := m.unit
  let name := artifacts ++ Name.mkSimple s!"frameInstantiation_{index}"
  let certificate := rootIdent name
  unless (← getEnv).contains name do
    let value := frameInstantiation unit callee #[] typeArgs
    let calleeTerm ← handleSyntax callee
    let typeIdTerm (typeId : VerexIR.TypeId) : CommandElabM Term :=
      `(term| (⟨$(Syntax.mkNatLit typeId.index)⟩ : VerexIR.TypeId))
    let typeArgsTerm ← typeUsesTerm typeArgs
    -- The frame reads which types the arguments name, not where they
    -- occur (`frameInstantiation_congr`): it is certified once for those
    -- types, at location 0, and shared by every call naming them.
    let frameArgs := typeArgs.map fun typeUse => { typeUse with loc := ⟨0⟩ }
    let frameArgsTerm ← typeUsesTerm frameArgs
    let entries ← value.mapM fun (symbolic, concrete) => do
      `(term| ($(← typeIdTerm symbolic), $(← typeIdTerm concrete)))
    let valueTerm ← `(term| (#[$entries,*] : Array (VerexIR.TypeId × VerexIR.TypeId)))
    -- The runtime's computation over the unit is certified by
    -- witness: the callee namespace's key map, decided by the kernel once,
    -- and this instantiation's witnesses, which the kernel checks in one
    -- pass over the table with logarithmic reads — never by replaying its
    -- searches.
    -- Every proof is a kernel decision (`Eq.refl`), stated from syntax and
    -- built as a term, so the elaborator evaluates nothing.
    let namespaceIndex := callee.namespaceId.index
    let some calleeNs := unit.namespaces[namespaceIndex]?
      | throwErrorAt reference m!"the callee's namespace is out of range"
    let viewEq ← liftTermElabM (publishCompileView segments unit callee.namespaceId)
    let view := rootIdent (Name.str (semanticsName segments) s!"compileView_{namespaceIndex}")
    let mapName := Name.str (semanticsName segments) s!"keyMap_{namespaceIndex}"
    let mapCorrect := mapName ++ `correct
    let map := rootIdent mapName
    let instantiations :=
      VerexIR.SemanticOperations.instantiateGenericArguments #[] (frameArgs.map .typeArg)
    let witnessValue := VerexIR.Validation.computeWitnesses calleeNs instantiations
    -- Once per unit for a namespace and type arguments: every call at them
    -- shares the instantiation.
    let frameKey := "_".intercalate (frameArgs.toList.map fun (typeUse : VerexIR.TypeUse) =>
      s!"{typeUse.typeId.index}")
    let frameName := Name.str (semanticsName segments) s!"frame_{namespaceIndex}_{frameKey}"
    let witnessesName := frameName ++ `witnesses
    let witnesses := rootIdent witnessesName
    let treeName := frameName ++ `witnessTree
    let tree := rootIdent treeName
    let tableName := Name.str (semanticsName segments) s!"typesTable_{namespaceIndex}"
    let tableEq := tableName ++ `eq
    let table := rootIdent tableName
    let kindsName := Name.str (semanticsName segments) s!"lifetimeKinds_{namespaceIndex}"
    let kindsEq := kindsName ++ `eq
    let kinds := rootIdent kindsName
    -- A frame is declared whole or not at all: a failure leaves no part
    -- of it for a later call to find.
    let saved ← getEnv
    stageLog s!"{function}: frame {frameName} over {calleeNs.tables.types.size} types"
    -- Certifying a frame is a kernel decision over the callee namespace's
    -- types, whose cost grows with the namespace: no search a heartbeat
    -- budget bounds.
    if !(← getEnv).contains frameName then try liftTermElabM <| unbudgeted do
      unless (← getEnv).contains mapName do
        -- Once per namespace: the types as a literal, so the kernel reads
        -- them without deriving them from the unit (`Array.mk` of the list
        -- literal: its `toList` is a projection), and the key map over
        -- their fingerprints, answering every search by one lookup.
        literalDefinition tableName (mkApp (mkConst ``Array [0]) (mkConst ``VerexIR.Ty))
          (mkApp2 (mkConst ``Array.mk [0]) (mkConst ``VerexIR.Ty)
            (toExpr calleeNs.tables.types.toList))
        reflexiveTheorem tableEq (← `(term| ($view).source.tables.types = $table))
        literalDefinition mapName (mkConst ``VerexIR.Validation.KeyMap)
          (toExpr (VerexIR.Validation.computeKeyMap calleeNs.tables.types))
        decidedByKernel mapCorrect (← `(term| VerexIR.Validation.Correct ($view).types
          ($table).size $map))
        -- The lifetimes' kinds, indexed: a reference type's instantiation
        -- reads its lifetime's.
        literalDefinition kindsName
          (mkApp (mkConst ``VerexIR.Validation.IndexedArena) (mkConst ``VerexIR.LifetimeKind))
          (toExpr (VerexIR.Validation.IndexedArena.ofArray
            (VerexIR.Validation.lifetimeKinds calleeNs)))
        reflexiveTheorem kindsEq (← `(term| VerexIR.Validation.IndexedArena.ofArray
          (VerexIR.Validation.lifetimeKinds ($view).source) = $kinds))
      -- The witnesses, as the array the theorem folds and as the index the
      -- checker reads them through.
      literalDefinition witnessesName
        (mkApp (mkConst ``Array [0]) (mkConst ``VerexIR.Validation.Witness))
        (toExpr witnessValue)
      literalDefinition treeName
        (mkApp (mkConst ``VerexIR.Validation.IndexedArena) (mkConst ``VerexIR.Validation.Witness))
        (toExpr (VerexIR.Validation.IndexedArena.ofArray witnessValue))
      reflexiveTheorem (frameName ++ `treeEq)
        (← `(term| VerexIR.Validation.IndexedArena.ofArray $witnesses = $tree))
      reflexiveTheorem (frameName ++ `checked) (← `(term|
        VerexIR.Validation.checkAll $kinds ($view).types $map
        (VerexIR.SemanticOperations.instantiateGenericArguments #[]
          (($frameArgsTerm).map VerexIR.GenericArgument.typeArg))
        $tree ($table).size = true))
      reflexiveTheorem (frameName ++ `sizes) (← `(term| ($witnesses).size = ($table).size))
      reflexiveTheorem (frameName ++ `depths) (← `(term| ($witnesses).toList.all
        (fun witness => decide (witness.depth ≤ ($table).size)) = true))
      reflexiveTheorem (frameName ++ `value)
        (← `(term| VerexIR.Validation.foldWitnesses $tree ($table).size = $valueTerm))
      let statement ← Lean.Elab.Term.elabType (← `(term|
        VerexIR.Proofs.Denote.frameInstantiationIn $view #[] $frameArgsTerm = $valueTerm))
      let proof ← Lean.Elab.Term.elabTermEnsuringType (← `(term|
        (VerexIR.Proofs.Denote.frameInstantiationIn_eq_of_check $frameArgsTerm rfl
          (VerexIR.Proofs.Denote.types_of_view $(rootIdent viewEq)) $(rootIdent tableEq)
          $(rootIdent kindsEq) $(rootIdent mapCorrect) $(rootIdent (frameName ++ `treeEq))
          $(rootIdent (frameName ++ `checked)) $(rootIdent (frameName ++ `sizes))
          $(rootIdent (frameName ++ `depths))).trans $(rootIdent (frameName ++ `value))))
        statement
      Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
      let proof ← instantiateMVars proof
      addDecl (.thmDecl
        { name := frameName, levelParams := [], type := ← instantiateMVars statement, value := proof })
    catch error =>
      setEnv saved
      throw error
    elabCommand (← `(command|
      set_option Elab.async false in
      theorem $certificate :
          VerexIR.Proofs.Denote.frameInstantiation $unitTerm $calleeTerm #[] $typeArgsTerm =
            $valueTerm := by
        rw [VerexIR.Proofs.Denote.frameInstantiation_congr (left := $typeArgsTerm)
            (right := $frameArgsTerm) rfl,
          VerexIR.Proofs.Denote.frameInstantiation_of_view $(rootIdent viewEq)]
        exact $(rootIdent frameName)))
    stageLog s!"{function}: frame certificate {name}"
  `(term| $certificate)

/-- The certificate of the `index`th call passing `m`'s own type parameters,
which runs in its frame: at the empty frame as the kernel computes the
call's, otherwise by `frameInstantiation_own`. -/
private def ownFrameCertificate (m : Member) (instantiated : Array (FunctionHandle × Array VerexIR.TypeUse))
    (callee : FunctionHandle) (typeArgs : Array VerexIR.TypeUse) (index : Nat) :
    CommandElabM Term := do
  let { reference, function, artifacts, unitTerm, handleTerm, unitDefinition, arity, .. } := m
  let unit := m.unit
  let name := artifacts ++ Name.mkSimple s!"ownFrame_{index}"
  unless (← getEnv).contains name do
    unless (frameInstantiation unit callee #[] typeArgs).isEmpty do
      throwErrorAt reference m!"`{function}` calls a function of its namespace with its own \
        type parameters, whose frame outside any call is not the empty instantiation"
    let some emptyIndex := instantiated.idxOf? (callee, typeArgs)
      | throwErrorAt reference "an own call has no frame certificate"
    let empty := rootIdent (artifacts ++ Name.mkSimple s!"frameInstantiation_{emptyIndex}")
    let calleeTerm ← handleSyntax callee
    let typeArgsTerm ← typeUsesTerm typeArgs
    let parametersName := name ++ `parameters
    -- Decided by the kernel on the unit.
    liftTermElabM <| reflexiveTheorem parametersName (← `(term|
      VerexIR.SemanticOperations.ownParametersIn $(mkIdent unitDefinition) $handleTerm
        $typeArgsTerm = true))
    elabCommand (← `(command|
      set_option Elab.async false in
      theorem $(rootIdent name) {typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId)}
          (frame : VerexIR.Proofs.Denote.FrameOf $unitTerm $handleTerm
            $(Syntax.mkNatLit arity) typeInstantiation) :
          VerexIR.Proofs.Denote.frameInstantiation $unitTerm $calleeTerm typeInstantiation
            $typeArgsTerm = typeInstantiation :=
        VerexIR.Proofs.Denote.frameInstantiation_own rfl rfl $(rootIdent parametersName) $empty
          frame))
  `(term| $(rootIdent name) frame)

/-- The functions whose bodies the proof meets, with their denotations: the
target's own and its inlined callees'. -/
private abbrev Owners (unit : ValidatedUnit) := Array (FunctionHandle × ValidatedNamespace ×
  VerexIR.FunctionDecl VerexIR.Validation.FunctionBody × Function unit)

/-- The conditions the proof meets, each as an abbreviation applied at the
unit and frame its marker names: the loop invariants, assertions, data and
memory invariants of `owners`, paired by site and function, and the
preconditions of the inlined callees, paired by function. -/
private def memberConditions (m : Member) (owners : Owners m.unit)
    (stored : Contract.StoredInvariants) :
    CommandElabM (Array Term × Array Term) := do
  let { handle, artifacts, unitTerm, .. } := m
  let unit := m.unit
  -- The callees a body the proof meets calls: a closure's target, reached
  -- through a function value, owes no precondition of its own there (its
  -- invocation's is the value's, `requires_of`).
  let called := owners.foldl (fun found (owner, _, _, _) =>
    (bodyCallees unit owner).foldl (fun found callee =>
      if found.contains callee then found else found.push callee) found) #[]
  let invariants ← liftTermElabM <| withExecutableSkolems unit fun executable skolems => do
    owners.flatMapM fun (owner, ownerNs, ownerDeclaration, ownerCompiled) => do
      let types ← Contract.frameTypes skolems
      let row := ownerCompiled.params ++ ownerCompiled.locals
      let invariants ← loopInvariants unit executable owner.namespaceId ownerNs ownerDeclaration
        ownerCompiled.params row types stored.predicate
      let assertions ← assertionConditions unit executable owner.namespaceId ownerNs ownerDeclaration
        ownerCompiled.params row types (ownProof := owner == handle)
      -- The data invariants owed where mutations end and values are built,
      -- read off the namespace's compilation view where the unit has any.
      let view? :=
        if unitCarriesInvariants unit then compileNamespaceAt? unit owner.namespaceId else none
      let mutations ← view?.mapM fun view => mutationConditions unit executable owner view ownerNs
        ownerDeclaration ownerCompiled.params row types
      let constructions ← view?.mapM fun view => constructionConditions unit executable owner view
        ownerNs ownerDeclaration ownerCompiled.params types
      let mutations := mutations.getD #[]
      let constructions := constructions.getD #[]
      -- The invariants owed where the function's own writes of memory end
      -- and its own calls return; an inlined callee's owe none in its
      -- caller.
      let writes ← if owner == handle then
          memoryConditions unit executable owner ownerNs ownerDeclaration ownerCompiled.params row types
        else pure #[]
      let calls ← if owner == handle then
          afterCallConditions unit executable owner ownerNs ownerDeclaration ownerCompiled.params types
        else pure #[]
      -- The precondition an inlined callee's caller owes where it starts.
      let precondition ← if owner == handle || !called.contains owner then pure none
        else
          preconditionCondition unit executable owner.namespaceId ownerNs ownerDeclaration
            ownerCompiled.params types
      let invariants := invariants.map (fun (site, invariant, readsUnit) =>
        (site, invariant, readsUnit, "loopInvariant")) ++
        assertions.map (fun (site, condition, readsUnit) =>
          (site, condition, readsUnit, "assertion")) ++
        mutations.map (fun (site, condition, readsUnit) =>
          (site, condition, readsUnit, "mutationEnd")) ++
        constructions.map (fun (site, condition, readsUnit) =>
          (site, condition, readsUnit, "construction")) ++
        writes.map (fun (site, condition, readsUnit) =>
          (site, condition, readsUnit, "memoryWritten")) ++
        calls.map (fun (site, condition, readsUnit) =>
          (site, condition, readsUnit, "afterCall")) ++
        (precondition.map fun (condition, readsUnit) =>
          (0, condition, readsUnit, "precondition")).toArray
      invariants.mapM fun (site, invariant, readsUnit, kind) =>
        return (site, owner, ← mkLambdaFVars (← executableBinders executable skolems) invariant,
          readsUnit, kind)
  let mut loopPairs : Array Term := #[]
  let mut preconditionPairs : Array Term := #[]
  for (site, owner, invariant, _, kind) in invariants do
    let name := if kind == "precondition" then
        artifacts ++ Name.mkSimple s!"precondition_{functionKey unit owner}"
      else artifacts ++ Name.mkSimple s!"{kind}_{site}"
    -- Declared when the target is prepared; the proof finds it.
    unless (← getEnv).contains name do liftTermElabM (addAbbrev name invariant)
    -- The invariant's skolem instance is the loop's own: an inlined generic
    -- callee runs under its frame's, so the solver applies it, at the unit
    -- and frame its marker names, not the ambient instance an elaborated
    -- constant would take. The executable unit is the theorem's.
    let applied ← `(term| fun (_ : VerexIR.Validation.ValidatedUnit)
        (Θ : VerexIR.Proofs.Denote.Skolems $unitTerm) =>
          @$(rootIdent name) $unitTerm _ $(← unitReaderTerm unit) Θ)
    if kind == "precondition" then
      preconditionPairs := preconditionPairs.push
        (← `(term| ($(← handleSyntax owner), $applied)))
    else
      loopPairs := loopPairs.push
        (← `(term| ($(Syntax.mkNumLit (toString site)), $(← handleSyntax owner), $applied)))
  return (loopPairs, preconditionPairs)

/-- A model's ghost tables, as abbreviations the normalizer reads a write's
value off: the tables of its bodies, and their initial values. -/
private def ghostDefinitions (m : Member) (owners : Owners m.unit)
    : CommandElabM Unit := do
  let { artifacts, .. } := m
  let unit := m.unit
  if (← getEnv).contains (artifacts ++ `ghosts) then return
  let (ghostTable, initTable) ← liftTermElabM <| withExecutableSkolems unit fun executable skolems => do
    let types ← Contract.frameTypes skolems
    let (ghosts, inits) ← ghostTables unit executable owners types
    pure (← mkLambdaFVars (← executableBinders executable skolems) ghosts,
      ← mkLambdaFVars (← executableBinders executable skolems) inits)
  liftTermElabM do
    addAbbrev (artifacts ++ `ghosts) ghostTable
    addAbbrev (artifacts ++ `ghostInits) initTable
    -- The normalizer reads a write's value off the tables.
    let attr ← `(attr| vir_denote_norm)
    for name in [artifacts ++ `ghosts, artifacts ++ `ghostInits] do
      Lean.Elab.Term.applyAttributes name #[{ name := `vir_denote_norm, stx := attr, kind := .global }]

/-- The meanings the function's body is read under: closed, or routing each
member of its cycle (a model's meanings) to the meaning standing for it,
spelled out as `cycleMeanings` unfolds at the member's slot, so that the
normalizer meets the routes directly. At the runtime family, calls with
type arguments stay closed. -/
private def meaningsTerm (m : Member) (routed : Array (FunctionHandle × String))
    (instantiation ghostTerm initTerm invokeTerm : Term) : CommandElabM Term := do
  let { model, family, namespaceName, unitTerm, .. } := m
  if routed.isEmpty then
    if model then
      `(term| ({ VerexIR.Proofs.Denote.closedMeanings executable $instantiation with
        ghost := $ghostTerm, ghostInit := $initTerm, invoke := $invokeTerm } :
        VerexIR.Proofs.Denote.Meanings executable))
    else `(term| VerexIR.Proofs.Denote.closedMeanings executable $instantiation)
  else do
    let routes ← routed.foldrM (init := ← `(term|
        VerexIR.Proofs.Denote.propheticMeaning executable #[]))
      fun (member, name) rest => do
        let memberArtifacts := Name.str namespaceName name
        let self ← if family then `(term| ($(cycleSelf name) Θ typeInstantiation))
          else `(term| $(cycleSelf name))
        `(term| VerexIR.Proofs.Denote.routeMeaning $(← handleSyntax member)
          $(rootIdent (memberArtifacts ++ `params)) $(rootIdent (memberArtifacts ++ `shape))
          $self $rest)
    let closed ← `(term| VerexIR.Proofs.Denote.closedGeneric executable $instantiation)
    let generics ← if !family then pure closed
      else routed.foldrM (init := closed) fun (member, name) rest => do
        let memberArtifacts := Name.str namespaceName name
        `(term| VerexIR.Proofs.Denote.routeGeneric $(← handleSyntax member)
          $(rootIdent (memberArtifacts ++ `params)) $(rootIdent (memberArtifacts ++ `shape))
          $(cycleSelf name)
          (fun callee typeArgs => VerexIR.Proofs.Denote.frameInstantiation $unitTerm
            callee $instantiation typeArgs)
          $rest)
    `(term| (⟨$routes, $generics, $invokeTerm, $instantiation, fun _ => none, $ghostTerm,
        $initTerm⟩ :
      VerexIR.Proofs.Denote.Meanings executable))

/-- The readings of a function with in-body assumptions, with and without
them, which the hypothesis that they hold relates: its assumption table and
the two meanings. -/
private def assumptionDefinitions (m : Member)
    (bodyIdent row paramsTerm localsTerm shapeTerm mutablesTerm assumedIdent
      tableIdent : Ident) : CommandElabM Unit := do
  let { ns, declaration, handle, artifacts, unitTerm, .. } := m
  let unit := m.unit
  let compiled := m.compiled
  if (← getEnv).contains (artifacts ++ `assumptions) then return
  let table ← liftTermElabM <| withExecutableSkolems unit fun executable skolems => do
    let types ← Contract.frameTypes skolems
    mkLambdaFVars (← executableBinders executable skolems) (← assumptionTable unit executable
      handle.namespaceId ns declaration compiled.params (compiled.params ++ compiled.locals)
      types)
  liftTermElabM (addAbbrev (artifacts ++ `assumptions) table)
  -- The readings under the meanings the theorem reads the body under, and
  -- the unit its assumption table reads.
  let readerType ← if declaresGhosts unit then `(term| VerexIR.Proofs.ModelUnit $unitTerm)
    else `(term| VerexIR.Validation.ExecutableUnit $unitTerm)
  elabCommand (← `(command|
    noncomputable def $(rootIdent (artifacts ++ `plainMeaning))
        (executable : VerexIR.Validation.ExecutableUnit $unitTerm)
        [VerexIR.Proofs.Denote.Skolems $unitTerm]
        (meanings : VerexIR.Proofs.Denote.Meanings executable) :=
      fun (args : VerexIR.Proofs.Denote.HList $paramsTerm) => VerexIR.Proofs.Spec.bind
        (VerexIR.Proofs.Denote.Term.denote meanings
          (ρ := $shapeTerm) (Γ := $row) $bodyIdent
          (VerexIR.Proofs.Denote.initialEnv $paramsTerm $localsTerm args))
        (VerexIR.Proofs.Denote.ResultShape.finish (Γ := $row) $mutablesTerm $shapeTerm)))
  elabCommand (← `(command|
    noncomputable def $assumedIdent (executable : VerexIR.Validation.ExecutableUnit $unitTerm)
        [VerexIR.Proofs.Denote.Skolems $unitTerm]
        (meanings : VerexIR.Proofs.Denote.Meanings executable) (reader : $readerType) :=
      fun (args : VerexIR.Proofs.Denote.HList $paramsTerm) => VerexIR.Proofs.Spec.bind
        VerexIR.Proofs.Spec.get fun start => VerexIR.Proofs.Spec.bind
          (VerexIR.Proofs.Denote.Term.denote
            ({ meanings with assumption := $tableIdent reader args start } :
              VerexIR.Proofs.Denote.Meanings executable)
            (ρ := $shapeTerm) (Γ := $row) $bodyIdent
            (VerexIR.Proofs.Denote.initialEnv $paramsTerm $localsTerm args))
          (VerexIR.Proofs.Denote.ResultShape.finish (Γ := $row) $mutablesTerm $shapeTerm)))

/-- The hypotheses the theorems take, as binders and as the arguments
instantiating them: the natives assumed, the functions whose in-body
assumptions hold, the lemma steps trusted; for a theorem assuming typing,
what the natives preserve (typing, and agreement up to the loans they mint)
and what the unit's runs preserve over its abstract maps (agreement and
representability, `doc/designs/intrinsic-maps.md`, "Abstract representation");
for one resolving `result_of` of a known function value, that the unit's
runs end, with the typing its runs rest on. Also whether the theorem assumes
typing. -/
private def theoremHypotheses (m : Member) (natives assumptionDeps : Array (FunctionHandle × String))
    (trustDeps : Array Name) :
    CommandElabM (Array (TSyntax ``Lean.Parser.Term.bracketedBinder) × Array Term ×
      Array (TSyntax ``Lean.Parser.Term.bracketedBinder) × Array Term × Bool) := do
  let { segments, namespaceIndex, ns, declaration, unitTerm, .. } := m
  let unit := m.unit
  let nativeBinders ← natives.mapM (nativeBinderOf unit segments)
  let nativeArgs : Array Term := natives.map fun entry =>
    ⟨mkIdent (Name.mkSimple s!"native_{entry.2}")⟩
  let nativeBinders := nativeBinders ++ (← trustDeps.mapM trustBinderOf)
  let nativeArgs := nativeArgs ++ trustArgs trustDeps
  -- That the in-body assumptions hold, bound last (`assumptionBinderOf`).
  let assumptionBinders ← assumptionDeps.mapM (assumptionBinderOf segments unit)
  let assumesTermination := Contract.assumesTermination unit ⟨namespaceIndex⟩ ns declaration
  let assumesTyping := Contract.assumesTyping unit ⟨namespaceIndex⟩ ns declaration ||
    assumesTermination
  let (nativeBinders, nativeArgs) ← if assumesTyping then do
      let typing := mkIdent `nativesTyped
      let shifts := mkIdent `nativesShift
      let agrees := mkIdent `runsAgree
      let representable := mkIdent `ownersRepresentable
      pure (nativeBinders ++
          #[← `(bracketedBinder| ($typing : VerexIR.NativesTyped executable)),
            ← `(bracketedBinder| ($shifts : VerexIR.NativesShift executable)),
            ← `(bracketedBinder| ($agrees : VerexIR.Proofs.RunsAgree executable)),
            ← `(bracketedBinder| ($representable : VerexIR.Proofs.OwnersRepresentable $unitTerm))],
        nativeArgs ++ #[(⟨typing⟩ : Term), ⟨shifts⟩, ⟨agrees⟩, ⟨representable⟩])
    else pure (nativeBinders, nativeArgs)
  let (nativeBinders, nativeArgs) ← if assumesTermination then do
      let terminating := mkIdent `verexTerminating
      pure (nativeBinders.push
          (← `(bracketedBinder| ($terminating : VerexIR.Proofs.Terminating executable))),
        nativeArgs.push ⟨terminating⟩)
    else pure (nativeBinders, nativeArgs)
  return (nativeBinders, nativeArgs, assumptionBinders, assumptionArgs assumptionDeps, assumesTyping)

/-- The equivalences of the specialized contracts the generic calls of `m`'s
body and of its inlined callees meet (`doc/designs/native-verification.md`,
"Specialized contracts"): one per callee used through a contract, the
members of its cycle and, under a model, the meanings standing for a
contract among them, and type arguments. -/
private def specializationCertificates (m : Member) (callees : Callees m.unit)
    (routed : Array (FunctionHandle × String)) : CommandElabM (Array Term) := do
  let unit := m.unit
  -- Under a model an inlined member stands for its body, not a contract.
  let contracted := routed.filter fun (member, _) =>
    !callees.modelMembers.any fun (other, _, inlined) => inlined && other == member
  let contracts := callees.contracts ++ contracted.map fun (member, name) =>
    (member, typedContractName m.segments name)
  let mut calls : Array (FunctionHandle × TypeArgs) := m.compiled.body.foldCalls collect (fun _ _ found => found) #[]
  for (_, _, _, inlined) in callees.inlined do
    calls := inlined.body.foldCalls collect (fun _ _ found => found) calls
  let mut seen : Array (FunctionHandle × NRow) := #[]
  let mut certificates : Array Term := #[]
  for (callee, θ) in calls do
    if seen.contains (callee, θ.1) then continue
    seen := seen.push (callee, θ.1)
    let some (_, generic) := contracts.find? (·.1 == callee) | continue
    let name ← liftTermElabM (ensureSpecialization unit generic θ)
    certificates := certificates.push (← `(term| @$(rootIdent name)))
  return certificates
where
  collect (callee : FunctionHandle) (_ : Array VerexIR.TypeUse) (θ? : Option TypeArgs)
      (found : Array (FunctionHandle × TypeArgs)) : Array (FunctionHandle × TypeArgs) :=
    match θ? with
    | some θ => found.push (callee, θ)
    | none => found

/-- The solver's tactics: the automatic verification, and the
`verex_obligations` of an authored proof, each leaf named by the clause it
owes. An authored script receives every obligation without automatic leaf
solving; a
contract binding a state label has the solver keep its program points; an
obligation no clause locates is reported at the function. -/
private def solverTactics (m : Member) (owners : Owners m.unit) (loopPairs calleePairs
    instantiationCertificates preconditionPairs specializations : Array Term) :
    CommandElabM (TSyntax `tactic × TSyntax `tactic) := do
  let { reference, namespaceIndex, ns, declaration, model, .. } := m
  let unit := m.unit
  let mut flags : Array (TSyntax `solveFlag) := #[]
  if Contract.contractBindsStateLabel unit ⟨namespaceIndex⟩ declaration then
    flags := flags.push (← `(solveFlag| labeled))
  let unrollBound? := declaration.pragmas.findSome? fun
    | .assign "unroll" (.constant (.integer bound)) _ =>
        if 0 ≤ bound then some bound.toNat else none
    | _ => none
  let unrollPairs ← match unrollBound?, declaration.body with
    | some bound, .structured root => (loopSpecifications ns root).mapM fun (site, _) =>
        `(term| ($(quote (loopSite namespaceIndex site.index)), $(quote bound)))
    | _, _ => pure #[]
  let writes ← (if model then ghostWriteSites unit owners else #[]).mapM fun (site, range) =>
    `(ghostWriteSite| ($(Syntax.mkNumLit (toString site)), $(Syntax.mkStrLit range.file),
      $(Syntax.mkNumLit (toString range.startByte)), $(Syntax.mkNumLit (toString range.endByte))))
  -- A callee's clause, assumed at a call, is named after the callee.
  let calleeNames : Array (VerexIR.Proofs.ObligationRange × String) :=
    (unit.namespaces.mapIdx fun index calleeNs =>
      calleeNs.functions.flatMap fun function =>
        if index == namespaceIndex && function.name == declaration.name then #[]
        else
          let callee := ((calleeNs.tables.names[function.name.index]?).map (·.name)).getD "callee"
          (clauseNames unit calleeNs function).map fun (range, name) =>
            (range, s!"{callee}_{name}")).flatten
  let names ← (clauseNames unit ns declaration ++ calleeNames).mapM fun (range, name) =>
    `(obligationName| ($(Syntax.mkStrLit name), $(Syntax.mkStrLit range.file),
      $(Syntax.mkNumLit (toString range.startByte)), $(Syntax.mkNumLit (toString range.endByte))))
  -- Every function's preconditions are numbered, its own for its recursive
  -- calls.
  let numbered := (unit.namespaces.map fun calleeNs =>
    calleeNs.functions.flatMap fun function => preconditionNumbers unit function).flatten
  let numbers ← numbered.mapM fun (range, number) =>
    `(preconditionNumber| ($(Syntax.mkNumLit (toString number)), $(Syntax.mkStrLit range.file),
      $(Syntax.mkNumLit (toString range.startByte)), $(Syntax.mkNumLit (toString range.endByte))))
  let sites : Array (Nat × String × Nat × Nat) := match declaration.body with
    | .structured root => callSites unit ns namespaceIndex root
    | _ => #[]
  let calls ← sites.mapM fun (site, callee, number, overall) =>
    `(callSite| ($(Syntax.mkNumLit (toString site)), $(Syntax.mkStrLit callee),
      $(Syntax.mkNumLit (toString number)), $(Syntax.mkNumLit (toString overall))))
  -- Every invariant clause keyed by what it is.
  let invariantClauses := (unit.namespaces.map fun invariantNs =>
    (invariantNs.structs.flatMap fun declaration =>
      -- A data invariant numbered by its position in its type's spec block.
      (declaration.contract.conditions.filter (·.kind == .structInvariant)).zipIdx.map
        fun (condition, position) =>
          (s!"data invariant_{position + 1}", Contract.locRange unit condition.loc)) ++
    (invariantNs.invariants.map fun invariant =>
      ("module invariant", Contract.locRange unit invariant.condition.loc))).flatten
  let keyed ← invariantClauses.mapM fun (kind, range) =>
    `(invariantClause| ($(Syntax.mkStrLit kind), $(Syntax.mkStrLit range.file),
      $(Syntax.mkNumLit (toString range.startByte)), $(Syntax.mkNumLit (toString range.endByte))))
  let kinds := m.compiled.body.foldCalls (fun _ _ _ found => found)
    (fun kind site found => found.insert site kind) ({} : Std.HashMap Nat String)
  let sourceSites : Array (Nat × String × Nat × String) := match declaration.body with
    | .structured root => bodySites unit ns namespaceIndex root kinds
    | _ => #[]
  let sited ← sourceSites.mapM fun (site, kind, number, label) =>
    `(sourceSite| ($(Syntax.mkNumLit (toString site)), $(Syntax.mkStrLit kind),
      $(Syntax.mkNumLit (toString number)), $(Syntax.mkStrLit label)))
  let solveTactic ← withRef reference `(tactic| verex_solve_goals $flags* [$loopPairs,*]
    with [$calleePairs,*] using [$instantiationCertificates,*]
    specializing [$specializations,*] unrolling [$unrollPairs,*]
    requiring [$preconditionPairs,*] writing [$writes,*] naming [$names,*]
    calling [$calls,*] numbering [$numbers,*] siting [$sited,*] keying [$keyed,*])
  let obligationsTactic ← withRef reference `(tactic| verex_obligations $flags* [$loopPairs,*]
    with [$calleePairs,*] using [$instantiationCertificates,*]
    specializing [$specializations,*] unrolling [$unrollPairs,*]
    requiring [$preconditionPairs,*] writing [$writes,*] naming [$names,*]
    calling [$calls,*] numbering [$numbers,*] siting [$sited,*] keying [$keyed,*])
  return (solveTactic, obligationsTactic)

/-- The introduction of the typed theorem's goal: a loop invariant reading
`old` needs the function's start, recorded as a hypothesis the solver finds
(so does a loop keeping the stored resources' invariants). The hypotheses it
names are `assumed` and `permitted`, the spelling the proof shares. -/
private def introductionTactic (m : Member) (stored : Contract.StoredInvariants)
    (pattern : TSyntax `rcasesPat)
    (assumed permitted : Ident) : CommandElabM (TSyntax `tactic) := do
  let { ns, declaration, handle, handleTerm, .. } := m
  let unit := m.unit
  let compiled := m.compiled
  let readsStart : Bool := match declaration.body with
    | .structured root => (loopSpecifications ns root ++
          bodySpecifications ns root (· matches .assertion | .apply)).any fun (_, block) =>
        block.conditions.any fun condition =>
          (condition.kind matches .loopInvariant | .assertion | .apply | .split) &&
            (!(oldReferencedLocals ns condition.expression).isEmpty ||
              readsEntryState ns condition.expression)
    | _ => false
  let readsStart := readsStart || (stored.predicate.isSome &&
    loopsKeepStoredInvariants unit handle.namespaceId ns declaration compiled.params)
  -- A proof in steps binds the parameters' scope entries before taking
  -- them apart.
  let steps := m.script?.any (·.steps)
  let arguments := mkIdent `verexArguments
  let withResult := m.script?.any fun script => readsResult script.tactic.raw
  if readsStart then
    let destructure ← if steps then scopedArguments unit ns declaration compiled.params withResult
      else `(tactic| rcases $arguments:ident with $pattern:rcasesPat)
    let introduce ← `(tactic|
      (intro $arguments:ident initialState $assumed:ident $permitted:ident
       have := VerexIR.Proofs.Denote.FunctionStart.intro $handleTerm $arguments
         initialState
       $destructure:tactic))
    if steps then `(tactic| ($introduce:tactic; all_goals verex_scope_state initialState))
    else pure introduce
  else if steps then
    let introduce ← `(tactic|
      (intro $arguments:ident
       $(← scopedArguments unit ns declaration compiled.params withResult):tactic
       all_goals intro initialState $assumed:ident $permitted:ident))
    `(tactic| ($introduce:tactic; all_goals verex_scope_state initialState))
  else `(tactic| rintro $pattern:rcasesPat initialState $assumed:ident $permitted:ident)

/-- What a theorem assuming typing knows of its unit: global memory typed by
its resource types, which the leaves keep, and, where the unit's readings
agree, a function value typed by its carrier. -/
private def unitFactTactic (m : Member) (assumesTyping : Bool) : CommandElabM (TSyntax `tactic) := do
  let { segments, unitTerm, .. } := m
  let unit := m.unit
  unless assumesTyping do return ← `(tactic| skip)
  let resources ← ensureResourcesTyped segments unit
  let resourcesFact ← `(tactic|
    have $(mkIdent `verexResources) : VerexIR.Proofs.ResourcesTyped $unitTerm :=
      $(rootIdent resources))
  match ← ensureTypesAgree segments unit with
  | some agreement =>
      `(tactic| (have $(mkIdent `verexAgree) : VerexIR.Proofs.TypesAgree $unitTerm :=
                   $(rootIdent agreement)
                 $resourcesFact:tactic))
  | none => pure resourcesFact

/-- The automatic verification, or the authored proof of the function's
goal, which sees `VerexIR` and the module's `open` items opened: the two
tactics the theorem runs, one of them `skip`. -/
private def proofTactics (script? : Option AuthoredScript)
    (opens : Array (TSyntax ``Lean.Parser.Command.openDecl))
    (solveTactic obligationsTactic : TSyntax `tactic) :
    CommandElabM (TSyntax `tactic × TSyntax `tactic) := do
  match script? with
  | some script =>
      let authored := Id.run <| script.tactic.raw.replaceM fun stx =>
        pure <| if (stx.isOfKind ``VerexIR.Proofs.Denote.verexObligations ||
            stx.isOfKind ``VerexIR.Proofs.Denote.verexProof) &&
            stx[1].getNumArgs == 0 then
          some (stx.setArg 1 obligationsTactic.raw[1])
        else none
      -- Authored proofs are native (`doc/designs/native-verification.md`).
      let mut tactic ← `(tactic| verex_authored $(⟨authored⟩):tacticSeq)
      for decl in opens.reverse do
        tactic ← `(tactic| open $decl:openDecl in $tactic:tactic)
      tactic ← `(tactic| open $(mkIdent `VerexIR):ident in $tactic:tactic)
      pure (← `(tactic| skip), tactic)
  | none => pure (solveTactic, ← `(tactic| skip))

/-- The theorems carrying the typed theorem to the runtime: the function's
semantics satisfies its contract, and so does its run, at every
instantiation coherent with the frame its type arguments induce for a
generic function. Their proofs elaborate in tasks (`proofTasks`), which the
target's conclusion waits for. -/
private def transportCommands (m : Member) (runtimeFrame instantiation contract : Term)
    (familyBinders frameBinders nativeBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder))
    (frameArgs nativeArgs : Array Term) (atRuntime : Term → CommandElabM Term)
    (compiledIdent compiledEqIdent publicContract paramsTerm shapeTerm : Term)
    (typedVerified : Name) : CommandElabM (TSyntax `command × TSyntax `command) := do
  let { segments, function, declaration, generic, unitDefinition, unitTerm, handleTerm, .. } := m
  let unit := m.unit
  -- The proof names the frame its statement is at.
  let frame ← if generic then `(term| Θ) else pure runtimeFrame
  let semanticsStatement ← atRuntime (← `(term|
        VerexIR.Proofs.Satisfies
          (VerexIR.Proofs.Denote.propheticMeaning executable $instantiation $handleTerm $paramsTerm
            $shapeTerm)
          $contract))
  let semanticsCommand ← `(command|
    theorem $(mkIdent (typedSemanticsVerifiedName segments function))
        {registry : VerexIR.Validation.SemanticsRegistry}
        {executable : VerexIR.Validation.ExecutableUnit $unitTerm}
        (prepared : VerexIR.Validation.prepareExecution registry
          $(mkIdent unitDefinition) = .ok executable) $familyBinders* $nativeBinders* :
        $semanticsStatement := by
      have compiledAt : VerexIR.Proofs.Denote.compileFunction $unitTerm $handleTerm =
          .ok $compiledIdent := $compiledEqIdent
      exact @VerexIR.Proofs.Denote.satisfies_propheticMeaning _ _ executable $frame
        $instantiation $handleTerm $compiledIdent compiledAt _
        ($(mkIdent typedVerified) prepared $frameArgs* $nativeArgs*))
  let verifiedCommand ← if isGeneric declaration then
      -- A generic function's runs are at the instantiations calls give it:
      -- its theorem holds at every one coherent with the frame its type
      -- arguments induce.
      let publicContract ← atUnit segments unit (contractName segments function) publicContract
        none #[← `(term| θ), ← `(term| typeInstantiation)]
      let induced ← `(term| (VerexIR.Proofs.Denote.Skolems.instantiate θ $runtimeFrame))
      `(command|
        theorem $(mkIdent (verifiedName segments function))
            {registry : VerexIR.Validation.SemanticsRegistry}
            {executable : VerexIR.Validation.ExecutableUnit $unitTerm}
            (prepared : VerexIR.Validation.prepareExecution registry
              $(mkIdent unitDefinition) = .ok executable)
            (globalsPreserved : VerexIR.Proofs.Denote.GlobalsPreserved executable)
            (failuresIntegral : VerexIR.Proofs.Denote.FailuresIntegral executable)
            $nativeBinders* (θ : VerexIR.Proofs.Denote.TypeArgs) (free : θ.1.refFree = true)
            (typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId)) $frameBinders*
            (coherent : @VerexIR.Proofs.Denote.Coherent $unitTerm _ $induced $handleTerm
              typeInstantiation) :
            VerexIR.Proofs.SatisfiesFunctionAt executable $handleTerm typeInstantiation
              $publicContract :=
          VerexIR.Proofs.Denote.satisfies_generic_at executable globalsPreserved failuresIntegral $handleTerm θ
            free typeInstantiation $paramsTerm $shapeTerm _ coherent
            (@$(mkIdent (typedSemanticsVerifiedName segments function)) _ _ prepared $induced
              typeInstantiation $frameArgs* $nativeArgs*))
    else
      let publicContract ← atUnit segments unit (contractName segments function) publicContract
      `(command|
        theorem $(mkIdent (verifiedName segments function))
            {registry : VerexIR.Validation.SemanticsRegistry}
            {executable : VerexIR.Validation.ExecutableUnit $unitTerm}
            (prepared : VerexIR.Validation.prepareExecution registry
              $(mkIdent unitDefinition) = .ok executable)
            (globalsPreserved : VerexIR.Proofs.Denote.GlobalsPreserved executable)
            (failuresIntegral : VerexIR.Proofs.Denote.FailuresIntegral executable)
            $nativeBinders* :
            VerexIR.Proofs.SatisfiesFunction executable $handleTerm $publicContract :=
          VerexIR.Proofs.Denote.satisfies_prophetic executable globalsPreserved failuresIntegral $handleTerm
            $paramsTerm $shapeTerm
            _
            ($(mkIdent (typedSemanticsVerifiedName segments function)) prepared $nativeArgs*))
  return (semanticsCommand, verifiedCommand)

/-- Records a verified function's artifacts: its completion, and the
dependencies its theorems take. -/
private def recordArtifacts (base : Name) (natives assumptionDeps : Array (FunctionHandle × String))
    (trustDeps : Array Name) : CommandElabM Unit := do
  modifyEnv fun env => completedDenotations.tag env (base ++ `typedVerified)
  modifyEnv fun env => nativeDependencies.addEntry env (base, natives)
  modifyEnv fun env => assumptionDependencies.addEntry env (base, assumptionDeps)
  modifyEnv fun env => lemmaTrustDependencies.addEntry env (base, trustDeps)

/-- Proves `m`'s typed theorem, over the callees collected and the frame
certificates, and carries it to the runtime unless `m` is a member of a
cycle or verified through a model; returns the dependencies its theorems
take, for `verifyCycle` to conclude. -/
private def proveMember (m : Member) (opens : Array (TSyntax ``Lean.Parser.Command.openDecl))
    (callees : Callees m.unit) (routed : Array (FunctionHandle × String))
    (instantiationCertificates : Array Term) (errorsBefore : Nat) (prepare : Bool) :
    CommandElabM (MemberDependencies × PendingVerification) := do
  let { reference, segments, function, script?, cycle, namespaceName, ns, declaration, handle, model, family, generic, framed, arity, ownAssumptions, artifacts, base, unitDefinition, unitTerm, handleTerm, .. } := m
  let unit := m.unit
  let compiled := m.compiled
  let { calleePairs, natives, assumptionDeps, trustDeps, inlined, modelMembers, .. } := callees
  let inlinedMember := fun (name : String) =>
    modelMembers.any fun (_, key, inlined) => inlined && key == name
  let typedVerified := typedVerifiedName segments function
  let compiledIdent := rootIdent (compiledName segments function)
  let bodyIdent := rootIdent (Name.str (Name.str namespaceName function) "body")
  let row := rootIdent (Name.str (Name.str namespaceName function) "row")
  let paramsTerm := rootIdent (Name.str (Name.str namespaceName function) "params")
  let localsTerm := rootIdent (Name.str (Name.str namespaceName function) "locals")
  let shapeTerm := rootIdent (Name.str (Name.str namespaceName function) "shape")
  let mutablesTerm := rootIdent (Name.str (Name.str namespaceName function) "mutables")
  let compiledEqIdent := rootIdent (compiledEqName segments function)
  let typedContract := rootIdent (typedContractName segments function)
  let publicContract := rootIdent (contractName segments function)
  let stored ← ensureStoredInvariant segments unit
  -- The invariants of the loops and the conditions of the assertions the
  -- proof meets: the function's own and those of its inlined callees, each
  -- keyed by its site and function.
  let owners : Owners unit := #[(handle, ns, declaration, compiled)] ++ inlined
  let (loopPairs, preconditionPairs) ← memberConditions m owners stored
  liftTermElabM (publishCompiled segments unit function handle compiled)
  let pattern ← argumentPattern unit declaration compiled.params
  let budgetValue := (heartbeatBudget? declaration).getD (verex.verifyHeartbeats.get (← getOptions))
  let budget := Syntax.mkNumLit (toString budgetValue)
  let leafBudget := Syntax.mkNumLit (toString
    ((leafHeartbeatBudget? declaration).getD (verex.leafHeartbeats.get (← getOptions))))
  let familyBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ← if generic then
      pure #[← `(bracketedBinder| {Θ : VerexIR.Proofs.Denote.Skolems $unitTerm}),
        ← `(bracketedBinder| {typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId)})]
    else pure #[]
  let frameBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ← if framed then
      pure #[← `(bracketedBinder| (frame : VerexIR.Proofs.Denote.FrameOf $unitTerm
        $handleTerm $(Syntax.mkNatLit arity) typeInstantiation))]
    else pure #[]
  let familyBinders := familyBinders ++ frameBinders
  let frameArgs : Array Term ← if framed then pure #[← `(term| frame)] else pure #[]
  let instantiation ← if generic then `(term| typeInstantiation) else `(term| #[])
  let contract ← atUnit segments unit (typedContractName segments function) typedContract none
    (if isGeneric declaration then #[instantiation] else #[])
  -- A function without type parameters is stated and proved at the runtime
  -- frame of the executable's unit.
  let runtimeFrame ← `(term| ($(mkCIdent ``VerexIR.Proofs.Denote.Skolems.runtime) $unitTerm))
  let atRuntime (statement : Term) : CommandElabM Term :=
    if generic then pure statement
    else `(term| letI := $runtimeFrame
        $statement)
  -- A model reads its ghost writes from the tables of its bodies.
  let ghostsIdent := rootIdent (artifacts ++ `ghosts)
  let initsIdent := rootIdent (artifacts ++ `ghostInits)
  if model then ghostDefinitions m owners
  let reader ← unitReaderTerm unit
  let ghostTerm ← if model then `(term| $ghostsIdent $reader) else `(term| fun _ => none)
  let initTerm ← if model then `(term| $initsIdent $reader) else `(term| fun _ => none)
  -- Under a model an invocation means what the theorem's invocation meaning
  -- says at the frame (`doc/designs/ghost-fields.md`); otherwise the target's
  -- big-step run.
  let invokeTerm ← if !model then `(term| VerexIR.Proofs.Denote.closureMeaning executable)
    else if family then `(term| ($invokeIdent Θ))
    else `(term| ($invokeIdent ($(mkCIdent ``VerexIR.Proofs.Denote.Skolems.runtime) $unitTerm)))
  let meanings ← meaningsTerm m routed instantiation ghostTerm initTerm invokeTerm
  -- A member of a cycle is verified over a meaning per member satisfying
  -- its contract: at the runtime family, or at every family and
  -- instantiation. Under a model an inlined callee's meaning refines its
  -- body under the same meanings.
  let selfBinders ← routedSelfBinders segments unit unitTerm namespaceName routed family
    inlinedMember meanings atRuntime
  let (invokeBinders, routingBinders, routingLemmas) ← if !model then pure (#[], #[], #[]) else
    modelInvokeBinders segments unit unitTerm namespaceName routed family
  -- The readings of the function with and without its in-body assumptions,
  -- which the hypothesis that they hold relates.
  let assumptionsIdent := mkIdent (Name.mkSimple s!"assumptions_{function}")
  let assumedIdent := rootIdent (artifacts ++ `assumedMeaning)
  let tableIdent := rootIdent (artifacts ++ `assumptions)
  if ownAssumptions then
    assumptionDefinitions m bodyIdent row paramsTerm localsTerm shapeTerm mutablesTerm
      assumedIdent tableIdent
  let (nativeBinders, nativeArgs, assumptionBinders, assumedArgs, assumesTyping) ←
    theoremHypotheses m natives assumptionDeps trustDeps
  let specializations ← specializationCertificates m callees routed
  let (solveTactic, obligationsTactic) ←
    solverTactics m owners loopPairs calleePairs instantiationCertificates preconditionPairs
      specializations
  -- The hypotheses the introduction names and the proof below uses share
  -- one spelling across the two quotations.
  let permitted := mkIdent `permitted
  let assumed := mkIdent `assumed
  let introduction ← introductionTactic m stored pattern assumed permitted
  let unitFact ← unitFactTactic m assumesTyping
  let (automaticTactic, scriptTactic) ← proofTactics script? opens solveTactic obligationsTactic
  -- Under its in-body assumptions: the reading that assumes them is
  -- verified, and the hypothesis that they hold carries the contract over.
  let assumptionsTactic ← if ownAssumptions then
      `(tactic| refine VerexIR.Proofs.satisfies_of_assumptions $assumptionsIdent ?_)
    else `(tactic| skip)
  let assumptionLemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← if ownAssumptions then
      pure #[← `(Lean.Parser.Tactic.simpLemma| $assumedIdent:ident),
        ← `(Lean.Parser.Tactic.simpLemma| $tableIdent:ident),
        ← `(Lean.Parser.Tactic.simpLemma| VerexIR.Proofs.wp_bind_get)]
    else pure #[]
  -- The signatures of the cycle's members, which route their calls.
  let memberSignatures : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
    routed.flatMapM fun (_, name) => do
      let memberArtifacts := Name.str namespaceName name
      pure #[← `(Lean.Parser.Tactic.simpLemma| $(rootIdent (memberArtifacts ++ `params)):ident),
        ← `(Lean.Parser.Tactic.simpLemma| $(rootIdent (memberArtifacts ++ `shape)):ident)]
  let ghostLemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← if model then
      pure #[← `(Lean.Parser.Tactic.simpLemma| $ghostsIdent:ident),
        ← `(Lean.Parser.Tactic.simpLemma| $initsIdent:ident)]
    else pure #[]
  let memberSignatures := memberSignatures ++ assumptionLemmas ++ ghostLemmas ++ routingLemmas
  -- A function without type parameters is proved at the runtime family,
  -- a closed term the normalizer's caches keep.
  -- A scripted theorem's messages land on the script: an unsolved
  -- obligation is reported at the authored proof.
  -- Prepared: every definition and certificate the proof needs is in the
  -- environment, checked before any proof task runs.
  if prepare then return ((natives, assumptionDeps, trustDeps), {})
  -- The proof elaborates in a task (`verex_proof`) as an attempt of its
  -- own; the command elaborates the statement and goes on to the next
  -- target. Declared by its name in the module, as the theorems are, under
  -- the command's namespace.
  let suffix := attemptSuffix (← getEnv) base
  let attempt := typedVerified ++ suffix
  let attemptFull := base ++ `typedVerified ++ suffix
  let typedStatement ← atRuntime (← `(term|
        VerexIR.Proofs.Satisfies
          (fun args => VerexIR.Proofs.Spec.bind
            (VerexIR.Proofs.Denote.Term.denote $meanings (ρ := $shapeTerm) (Γ := $row)
              $bodyIdent (VerexIR.Proofs.Denote.initialEnv $paramsTerm $localsTerm args))
            (VerexIR.Proofs.Denote.ResultShape.finish (Γ := $row) $mutablesTerm $shapeTerm))
          $contract))
  let typedCommand ← withRef (script?.map (·.tactic.raw) |>.getD reference) do `(command|
    set_option Elab.async true in
    set_option maxHeartbeats $budget:num in
    set_option verex.leafHeartbeats $leafBudget:num in
    theorem $(mkIdent attempt)
        {registry : VerexIR.Validation.SemanticsRegistry}
        {executable : VerexIR.Validation.ExecutableUnit $unitTerm}
        (prepared : VerexIR.Validation.prepareExecution registry
          $(mkIdent unitDefinition) = .ok executable) $familyBinders* $nativeBinders*
          $invokeBinders* $selfBinders* $routingBinders* $assumptionBinders* : $typedStatement := by
      verex_proof $(mkIdent attempt) =>
        $unitFact:tactic
        $assumptionsTactic:tactic
        apply VerexIR.Proofs.satisfies_of_wp
        $introduction:tactic
        all_goals
        (simp only [$bodyIdent:ident, $row:ident, $paramsTerm:ident, $localsTerm:ident,
          $shapeTerm:ident, $mutablesTerm:ident, $typedContract:ident, vir_denote_norm,
          reduceCtorEq, Nat.reduceEqDiff, String.reduceBEq, String.reduceEq, String.reduceBNe,
          String.reduceNe, $memberSignatures,*] at $assumed:ident $permitted:ident ⊢
         -- The function assumes its preconditions; their markers locate
         -- them for its callers only.
         all_goals try simp only [VerexIR.Proofs.Obligation_eq] at $permitted:ident
         all_goals verex_normalize at $permitted:ident ⊢
         all_goals $automaticTactic:tactic)
        $scriptTactic:tactic)
  let typedCommand ← if selectsBitVectors declaration then
      `(command| set_option verex.bitVectors true in $typedCommand)
    else pure typedCommand
  stageLog s!"{function}: theorem start"
  -- The proof's task is taken from the command's: the target's turn
  -- reports its messages (`reportProofMessages`).
  let ((), tasks) ← proofTasks <| Perf.withPhase .verification <|
    inParsedFile (script?.bind (·.file)) (elabCommand typedCommand)
  let statementFailed := countErrors (← get).messages > errorsBefore
  let conclude : CommandElabM Unit := do
    let (failed, overBudget) ← reportProofMessages tasks statementFailed
    let report ← takeProofReport attempt
    IO.print report.output
    let outcome := if !failed then "verified" else if overBudget then "budget" else "failed"
    stageLog s!"{function}: theorem done, {report.heartbeats} heartbeats, {outcome}, \
      {if script?.isSome then "authored" else "automatic"}"
    Perf.recordArtifacts s!"{namespaceName}::{function} typed" #[attemptFull]
      report.heartbeats report.elapsedMs
    if failed then
      throwErrorAt reference
        (failureMessage ns function script?.isSome overBudget budgetValue report.failedLeaves)
    declareTypedTheorem base attemptFull
    stageLog s!"{function}: typed theorem declared"
    -- A member of a cycle concludes with the whole cycle.
    unless cycle.isEmpty do return
    -- A model's theorem is the function's verification; its runtime reading
    -- follows from the unit's model (`doc/designs/ghost-fields.md`).
    if model then
      recordArtifacts base natives assumptionDeps trustDeps
      withRef reference <| requireNativeArtifacts base (selectsBitVectors declaration) model
      stageLog s!"{function}: audited"
      modifyEnv fun env => auditedDenotations.tag env (base ++ `typedVerified)
      return
    -- Elaborated once the typed proof ended, their errors reported before
    -- the artifacts are accepted; their kernel checks follow its.
    let (semanticsCommand, verifiedCommand) ← transportCommands m runtimeFrame instantiation
      contract familyBinders frameBinders (nativeBinders ++ assumptionBinders) frameArgs
      (nativeArgs ++ assumedArgs) atRuntime
      compiledIdent compiledEqIdent publicContract paramsTerm shapeTerm typedVerified
    let errorsBefore := countErrors (← get).messages
    let ((), transports) ← proofTasks <| Perf.withPhase .verification <|
      Perf.measure s!"{namespaceName}::{function} transport" (base ++ `verified) do
        elabCommand semanticsCommand
        elabCommand verifiedCommand
    stageLog s!"{function}: transports elaborated"
    let (transportFailed, _) ←
      reportProofMessages transports (countErrors (← get).messages > errorsBefore)
    stageLog s!"{function}: transports proved"
    if transportFailed then throwErrorAt reference "verex verification failed"
    recordArtifacts base natives assumptionDeps trustDeps
    -- Audit fresh proofs as well as cached ones.
    withRef reference <| requireNativeArtifacts base (selectsBitVectors declaration)
    stageLog s!"{function}: audited"
    modifyEnv fun env => auditedDenotations.tag env (base ++ `typedVerified)
  return ((natives, assumptionDeps, trustDeps), { tasks, conclude })

/-- Verify one function through its denotation.  A member of a cycle of
calls is proved over a meaning for each member satisfying its contract,
standing for the calls to them: at the runtime family, or, when `family`
holds, at every skolem family and type instantiation.  Its typed theorem
alone is elaborated, and the natives it assumes returned, for `verifyCycle`
to conclude. -/
private def verifyMember (reference : Syntax) (segments : Array String) (function : String)
    (script? : Option AuthoredScript) (opens : Array (TSyntax ``Lean.Parser.Command.openDecl))
    (cycle : Array (FunctionHandle × String)) (family : Bool := false) (prepare : Bool := false) :
    CommandElabM (MemberDependencies × PendingVerification) := do
  let ignoring := verex.ignoreProofs.get (← getOptions)
  let script? := if ignoring then none else script?
  let namespaceName := pathName segments
  let some unit := VerexSL.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Verex namespace `{namespaceName}`"
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwErrorAt reference s!"unknown function `{function}` in `{namespaceName}`"
  let unitDefinition ← ensureUnitInstance segments unit
  let unitTerm ← unitTermOf segments unit
  discard <| ensureUnitDefinitions segments unit
  stageLog s!"{function}: semantics"
  let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  -- A unit declaring ghost fields is verified through its model
  -- (`doc/designs/ghost-fields.md`): every callee used through its contract is a
  -- meaning assumed to satisfy it, an inlined one refines its body under the
  -- same meanings, and no theorem is carried to the runtime here.
  let model := declaresGhosts unit
  let compiled ← match (if model then compileModelFunction else compileFunction) unit handle with
    | .ok compiled => pure compiled
    | .error reason => throwErrorAt reference m!"no denotation for `{function}`: {reason}"
  let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
    | throwErrorAt reference m!"no denotation for `{function}`: its signature is not native"
  ensureContracts segments function unit namespaceIndex ns declaration params result
  stageLog s!"{function}: compiled and contracts"
  -- A model routing a generic function, the target among them, is proved
  -- over every family, as a cycle through a generic member is.
  let family := family || (model && (isGeneric declaration || modelRoutesGeneric unit compiled))
  let artifacts := Name.str namespaceName function
  let base := (← getCurrNamespace) ++ artifacts
  if (← getEnv).contains (base ++ `typedVerified) then
    -- A colliding name alone is not evidence of a denotation proof.
    withRef reference <| requireNativeArtifacts base (selectsBitVectors declaration) model
    return ((#[], #[], #[]), {})
  if script?.isNone && requiresAuthoredProof declaration && !ignoring then
    throwErrorAt reference m!"verex verification failed: `{function}` sets \
      `pragma verify = manual`; {proofRequest ns function #[]}"
  let handleTerm ← handleSyntax ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  let ownAssumptions := statesAssumptions ns declaration
  if ownAssumptions && !cycle.isEmpty then
    throwErrorAt reference m!"`{function}` states an in-body assumption and is verified by \
      induction with the members of its cycle; the hypothesis that its assumptions hold is \
      stated of a function verified on its own"
  -- A call to any member of its cycle, itself included, is assumed at its
  -- contract, by fixed-point induction.
  let members := cycle.map (·.1)
  -- A generic function, and every member of a cycle through one, is proved
  -- over every skolem family and type instantiation; any other at the
  -- runtime family, a closed term the normalizer's caches keep, and the
  -- empty instantiation.
  let generic := isGeneric declaration || family ||
    -- A generic invariant's ghost type parameter ranges over every type.
    Contract.hasGhostInvariantInstances unit handle.namespaceId declaration
  -- A generic function outside a cycle is proved for the frames it runs in
  -- (`FrameOf`); a call passing its own type parameters runs in its frame.
  let framed := isGeneric declaration && cycle.isEmpty && !family
  let arity := (declaration.signature.generics.filter (·.kind == .typeArg)).size
  let m : Member := {
    reference := reference
    segments := segments
    function := function
    script? := script?
    cycle := cycle
    unit := unit
    namespaceName := namespaceName
    namespaceIndex := namespaceIndex
    ns := ns
    declaration := declaration
    handle := handle
    compiled := compiled
    model := model
    family := family
    generic := generic
    framed := framed
    arity := arity
    ownAssumptions := ownAssumptions
    artifacts := artifacts
    base := base
    unitDefinition := unitDefinition
    unitTerm := unitTerm
    handleTerm := handleTerm }
  let ownCalls : Array (FunctionHandle × Array VerexIR.TypeUse) := if !framed then #[] else
    compiled.body.foldCalls (fun callee typeArgs _ found =>
      if !typeArgs.isEmpty && typeArgs.size == arity &&
          callee.namespaceId == handle.namespaceId &&
          VerexIR.SemanticOperations.ownParametersIn unit handle typeArgs &&
          !found.contains (callee, typeArgs)
        then found.push (callee, typeArgs) else found) (fun _ _ found => found) #[]
  -- The assumed steps of the lemmas its own proof applies.
  let trustDeps : Array Name ← liftTermElabM do
    (functionLemmas ns declaration).foldlM (init := #[]) fun found lemma => do
      return (← lemmaTrustClosure unit lemma).foldl (init := found) fun found name =>
        if found.contains name then found else found.push name
  -- Every certificate from here on is elaborated synchronously, so a
  -- failure is counted at the target; the proof's task reports its own.
  let errorsBefore := countErrors (← get).messages
  -- An inlined callee's body brings its own callees into the proof, so the
  -- callees are collected through every inlined body.
  let worklist := (compiled.body.callees #[]).filter fun callee =>
    callee != handle && !members.contains callee
  let mut callees ← collectCallees m members worklist {
    assumptionDeps := if ownAssumptions then #[(handle, function)] else #[]
    trustDeps := trustDeps
    edges := #[(handle, worklist)]
    instantiated := if generic then ownCalls else compiled.body.foldCalls recordGenericCall (fun _ _ found => found) #[]
    contractTargets := contractClosureTargets unit handle.namespaceId ns declaration }
  callees := { callees with calleePairs := callees.calleePairs ++ callees.behaviorPairs }
  -- Each generic call at the empty instantiation instantiates its callee's
  -- frame as the runtime computes it: a kernel-checked literal the call's
  -- meaning is stated at.
  let mut instantiationCertificates : Array Term := #[]
  for (callee, typeArgs) in callees.instantiated, index in [0:callees.instantiated.size] do
    instantiationCertificates := instantiationCertificates.push
      (← frameCertificate m callee typeArgs index)
  -- A call passing the target's own type parameters runs in its frame: at
  -- the empty frame as the kernel computes the call's, otherwise by
  -- `frameInstantiation_own`.
  for (callee, typeArgs) in ownCalls, index in [0:ownCalls.size] do
    instantiationCertificates := instantiationCertificates.push
      (← ownFrameCertificate m callees.instantiated callee typeArgs index)
  stageLog s!"{function}: callees and certificates"
  if let some cycle := inliningCycle? callees.edges #[] handle then
    let name := (do
      let calleeNs ← unit.namespaces[cycle.namespaceId.index]?
      let declaration ← calleeNs.functions[cycle.functionId.index]?
      let name ← calleeNs.tables.names[declaration.name.index]?
      pure name.name).getD "a callee"
    throwErrorAt reference m!"`{function}` reaches `{name}` again through callees that are \
      inlined; only a function calling itself directly is verified by induction, so specify \
      and verify the callees on the cycle"
  -- Under a model the callees standing as meanings are routed as a cycle's
  -- members are, the function itself too where it calls itself.
  if model then
    if (compiled.body.callees #[]).contains handle then
      callees := { callees with modelMembers := callees.modelMembers.push (handle, function, false) }
  let routed : Array (FunctionHandle × String) :=
    if model then callees.modelMembers.map fun (member, name, _) => (member, name) else cycle
  let inlinedMember := fun (name : String) =>
    callees.modelMembers.any fun (_, key, inlined) => inlined && key == name
  for (_, name) in routed do
    let pair ← `(term| PProd.mk $(cycleSelf name) (PProd.mk $(Syntax.mkStrLit name)
      $(if inlinedMember name then cycleSelfRefines name else cycleSelfVerified name)))
    callees := { callees with calleePairs := callees.calleePairs.push pair }
  -- A failure before the proof's task began leaves no artifacts behind;
  -- the target concludes outside this state.
  let saved ← get
  try
    proveMember m opens callees routed instantiationCertificates errorsBefore prepare
  catch failure =>
    modify fun state => { saved with messages := state.messages }
    throw failure

/-- The functions of the cycle of calls through `handle`, `handle` first:
those it calls, directly or not, that call it again.  Empty when no call of
`handle` reaches it. -/
private def callCycle (unit : ValidatedUnit) (handle : FunctionHandle) :
    Array FunctionHandle := Id.run do
  let callees (caller : FunctionHandle) : Array FunctionHandle :=
    match compileFunction unit caller with
    | .ok compiled => compiled.body.callees #[]
    | .error _ => #[]
  let mut reached : Array FunctionHandle := #[]
  let mut worklist := callees handle
  while let some next := worklist.back? do
    worklist := worklist.pop
    if reached.contains next then continue
    reached := reached.push next
    worklist := worklist ++ callees next
  unless reached.contains handle do return #[]
  let mut members := #[handle]
  let mut grown := true
  while grown do
    grown := false
    for node in reached do
      if !members.contains node && (callees node).any members.contains then
        members := members.push node
        grown := true
  return members

/-- Verify the members of a cycle of calls together, a function calling
itself being a cycle of one: each over the members' meanings at every slot,
then all of them by fixed-point induction over the slots.  Without a generic
member every member is proved at the runtime family, the other slots
standing vacuous; with one, every member at every skolem family and type
instantiation, the runtime slots standing vacuous. -/
private def verifyCycle (reference : Syntax) (segments : Array String)
    (members : Array (FunctionHandle × String))
    (scripts : String → Option AuthoredScript) (opens : Array (TSyntax ``Lean.Parser.Command.openDecl))
    (prepare : Bool) : CommandElabM PendingVerification := do
  let namespaceName := pathName segments
  let some unit := VerexSL.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Verex namespace `{namespaceName}`"
  let unitDefinition ← ensureUnitInstance segments unit
  let unitTerm ← unitTermOf segments unit
  discard <| ensureUnitDefinitions segments unit
  let isGenericMember (member : FunctionHandle) := (declarationOf? unit member).any isGeneric
  let memberArity (member : FunctionHandle) := ((declarationOf? unit member).map fun declaration =>
    (declaration.signature.generics.filter (·.kind == .typeArg)).size).getD 0
  let family := members.any (isGenericMember ·.1)
  let saved ← get
  try
    -- Each member's typed theorem speaks of every member's signature and
    -- contract.
    for (member, name) in members do
      let some (namespaceIndex, ns, _, declaration) := findFunction? unit name
        | throwErrorAt reference s!"unknown function `{name}` in `{namespaceName}`"
      let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
        | throwErrorAt reference m!"no denotation for `{name}`: its signature is not native"
      ensureContracts segments name unit namespaceIndex ns declaration params result
      let compiled ← match compileFunction unit member with
        | .ok compiled => pure compiled
        | .error reason => throwErrorAt reference m!"no denotation for `{name}`: {reason}"
      liftTermElabM (publishCompiled segments unit name member compiled)
    let mut assumed : Array MemberDependencies := #[]
    let mut pendings : Array PendingVerification := #[]
    for (_, name) in members do
      let (dependencies, pending) ←
        verifyMember reference segments name (scripts name) opens members family prepare
      assumed := assumed.push dependencies
      pendings := pendings.push pending
    if prepare then return {}
    -- The members' proofs elaborate in their tasks; the cycle concludes
    -- once every one ended, carrying each member's theorem to the runtime.
    let conclude : CommandElabM Unit := do
      for pending in pendings do pending.conclude
      let mut natives : Array (FunctionHandle × String) := #[]
      let mut assumptionDeps : Array (FunctionHandle × String) := #[]
      let mut trustDeps : Array Name := #[]
      for (memberNatives, memberAssumptions, memberTrust) in assumed do
        for entry in memberNatives do
          unless natives.any (·.1 == entry.1) do natives := natives.push entry
        for entry in memberAssumptions do
          unless assumptionDeps.any (·.1 == entry.1) do assumptionDeps := assumptionDeps.push entry
        for name in memberTrust do
          unless trustDeps.contains name do trustDeps := trustDeps.push name
      let nativeBinders ← natives.mapM (nativeBinderOf unit segments)
      let nativeBinders := nativeBinders ++ (← assumptionDeps.mapM (assumptionBinderOf segments unit)) ++
        (← trustDeps.mapM trustBinderOf)
      let nativeArgs (entries : Array (FunctionHandle × String) × Array (FunctionHandle × String) ×
          Array Name) : Array Term :=
        entries.1.map (fun entry => ⟨mkIdent (Name.mkSimple s!"native_{entry.2}")⟩) ++
          assumptionArgs entries.2.1 ++ trustArgs entries.2.2
      -- A member's typed theorem binds its assumption hypotheses after its
      -- selves (`theoremHypotheses`).
      let memberArgs (entries : Array (FunctionHandle × String) × Array (FunctionHandle × String) ×
          Array Name) (selves : Array Term) : Array Term :=
        entries.1.map (fun entry => ⟨mkIdent (Name.mkSimple s!"native_{entry.2}")⟩) ++
          trustArgs entries.2.2 ++ selves ++ assumptionArgs entries.2.1
      let artifactsOf (name : String) := Name.str namespaceName name
      let compiledAt (name : String) := mkIdent (Name.mkSimple s!"compiledAt_{name}")
      let membersTerm ← cycleMembersSyntax segments members
      let indices ← (List.range members.size).toArray.mapM cycleIndexSyntax
      let compiledHaves ← members.mapM fun (member, name) => do
        `(tactic| have $(compiledAt name) : VerexIR.Proofs.Denote.compileFunction $unitTerm
              $(← handleSyntax member) = .ok $(rootIdent (compiledName segments name)) :=
            $(rootIdent (compiledEqName segments name)))
      let compiledAlternatives ← (members.zip indices).mapM fun ((_, name), index) =>
        `(Lean.Parser.Term.matchAltExpr| | $index => $(compiledAt name))
      let contractAlternatives ← (members.zip indices).mapM fun ((member, name), index) => do
        let typedContract := rootIdent (typedContractName segments name)
        let contract : Term ← if !family then
            atUnit segments unit (typedContractName segments name) typedContract
              (some (← `(term| ($(mkCIdent ``VerexIR.Proofs.Denote.Skolems.runtime) $unitTerm))))
          else if isGenericMember member then do
            let atFamily ← atUnit segments unit (typedContractName segments name) typedContract
              (some (← `(term| family))) #[← `(term| instantiation)]
            `(term| fun family instantiation => $atFamily)
          else do
            let atFamily ← atUnit segments unit (typedContractName segments name) typedContract
              (some (← `(term| family)))
            `(term| fun family _ => $atFamily)
        `(Lean.Parser.Term.matchAltExpr| | $index => $contract)
      let selvesVerified := mkIdent `selvesVerified
      -- Every member's meaning, then every member's hypothesis, as
      -- `routedSelfBinders` binds them.
      let mut selfArguments : Array Term := #[]
      let mut verifiedArguments : Array Term := #[]
      for index in indices do
        if family then
          selfArguments := selfArguments.push
            (← `(term| (fun family instantiation => $cycleSelves ⟨$index, some (family, instantiation)⟩)))
        else
          selfArguments := selfArguments.push (← `(term| ($cycleSelves ⟨$index, none⟩)))
        verifiedArguments := verifiedArguments.push (← `(term| ($selvesVerified $index)))
      selfArguments := selfArguments ++ verifiedArguments
      let verifiedAlternatives ← ((members.zip indices).zip assumed).mapM
        fun (((_, name), index), memberNatives) => do
          let typedVerified := mkIdent (typedVerifiedName segments name)
          let proof ← if family then
              `(term| fun family instantiation => @$typedVerified _ _ prepared family instantiation
                $(memberArgs memberNatives selfArguments)*)
            else `(term| $typedVerified prepared $(memberArgs memberNatives selfArguments)*)
          `(Lean.Parser.Term.matchAltExpr| | $index => $proof)
      for ((member, name), index) in members.zip indices do
        let handleTerm ← handleSyntax member
        let params := rootIdent (artifactsOf name ++ `params)
        let shape := rootIdent (artifactsOf name ++ `shape)
        let typedContract := rootIdent (typedContractName segments name)
        let generic := isGenericMember member
        -- A generic member's theorem holds at every family and instantiation;
        -- any other at the runtime family.
        let familyBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ← if generic then
            pure #[← `(bracketedBinder| {Θ : VerexIR.Proofs.Denote.Skolems $unitTerm}),
              ← `(bracketedBinder| {typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId)})]
          else pure #[]
        let instantiation ← if generic then `(term| typeInstantiation) else `(term| #[])
        let runtimeFrame ← `(term| ($(mkCIdent ``VerexIR.Proofs.Denote.Skolems.runtime) $unitTerm))
        let contract ← atUnit segments unit (typedContractName segments name) typedContract none
          (if generic then #[instantiation] else #[])
        let cycleProof ← if !family then
            `(term| @VerexIR.Proofs.Denote.satisfies_cycle_runtime _ _ executable $runtimeFrame #[]
              $membersTerm
              (fun $compiledAlternatives:matchAlt*)
              (fun $contractAlternatives:matchAlt*)
              (fun $cycleSelves $selvesVerified index => match index with
                $verifiedAlternatives:matchAlt*)
              $index)
          else
            `(term| @VerexIR.Proofs.Denote.satisfies_cycle_family
              _ _ executable $runtimeFrame #[] $membersTerm
              (fun $compiledAlternatives:matchAlt*)
              (fun $contractAlternatives:matchAlt*)
              (fun $cycleSelves $selvesVerified index => match index with
                $verifiedAlternatives:matchAlt*)
              $index $(← if generic then `(term| Θ) else pure runtimeFrame)
              $(← if generic then `(term| typeInstantiation) else `(term| #[])))
        -- A member without type parameters is stated and proved at the runtime
        -- frame of the executable's unit.
        let statement ← `(term|
              VerexIR.Proofs.Satisfies
                (VerexIR.Proofs.Denote.propheticMeaning executable $instantiation $handleTerm $params
                  $shape)
                $contract)
        let statement ← if generic then pure statement
          else `(term| letI := $runtimeFrame
              $statement)
        let semanticsTheorem ← `(command|
          theorem $(mkIdent (typedSemanticsVerifiedName segments name))
              {registry : VerexIR.Validation.SemanticsRegistry}
              {executable : VerexIR.Validation.ExecutableUnit $unitTerm}
              (prepared : VerexIR.Validation.prepareExecution registry
                $(mkIdent unitDefinition) = .ok executable) $familyBinders* $nativeBinders* :
              $statement := by
            $[$compiledHaves]*
            exact $cycleProof)
        let semanticsCommand := semanticsTheorem
        let memberNativeArgs := nativeArgs (natives, assumptionDeps, trustDeps)
        let verifiedCommand ← if isGenericMember member then
            let publicContract ← atUnit segments unit (contractName segments name)
              (rootIdent (contractName segments name)) none
              #[← `(term| θ), ← `(term| typeInstantiation)]
            let induced ← `(term| (VerexIR.Proofs.Denote.Skolems.instantiate θ $runtimeFrame))
            `(command|
              theorem $(mkIdent (verifiedName segments name))
                  {registry : VerexIR.Validation.SemanticsRegistry}
                  {executable : VerexIR.Validation.ExecutableUnit $unitTerm}
                  (prepared : VerexIR.Validation.prepareExecution registry
                    $(mkIdent unitDefinition) = .ok executable)
                  (globalsPreserved : VerexIR.Proofs.Denote.GlobalsPreserved executable)
                  (failuresIntegral : VerexIR.Proofs.Denote.FailuresIntegral executable)
                  $nativeBinders* (θ : VerexIR.Proofs.Denote.TypeArgs) (free : θ.1.refFree = true)
                  (typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId))
                  (_frame : VerexIR.Proofs.Denote.FrameOf $unitTerm $handleTerm
                    $(Syntax.mkNatLit (memberArity member)) typeInstantiation)
                  (coherent : @VerexIR.Proofs.Denote.Coherent $unitTerm _ $induced $handleTerm
                    typeInstantiation) :
                  VerexIR.Proofs.SatisfiesFunctionAt executable $handleTerm typeInstantiation
                    $publicContract :=
                VerexIR.Proofs.Denote.satisfies_generic_at executable globalsPreserved failuresIntegral $handleTerm θ
                  free typeInstantiation $params $shape _ coherent
                  (@$(mkIdent (typedSemanticsVerifiedName segments name)) _ _ prepared $induced
                    typeInstantiation $memberNativeArgs*))
          else
            `(command|
              theorem $(mkIdent (verifiedName segments name))
                  {registry : VerexIR.Validation.SemanticsRegistry}
                  {executable : VerexIR.Validation.ExecutableUnit $unitTerm}
                  (prepared : VerexIR.Validation.prepareExecution registry
                    $(mkIdent unitDefinition) = .ok executable)
                  (globalsPreserved : VerexIR.Proofs.Denote.GlobalsPreserved executable)
                  (failuresIntegral : VerexIR.Proofs.Denote.FailuresIntegral executable)
                  $nativeBinders* :
                  VerexIR.Proofs.SatisfiesFunction executable $handleTerm
                    $(← atUnit segments unit (contractName segments name)
                      (rootIdent (contractName segments name))) :=
                VerexIR.Proofs.Denote.satisfies_prophetic executable globalsPreserved failuresIntegral $handleTerm
                  $params $shape _
                  ($(mkIdent (typedSemanticsVerifiedName segments name)) prepared
                    $memberNativeArgs*))
        let base := (← getCurrNamespace) ++ artifactsOf name
        let errorsBefore := countErrors (← get).messages
        let ((), transports) ← proofTasks <| Perf.withPhase .verification <|
          Perf.measure s!"{namespaceName}::{name} transport" (base ++ `verified) do
            elabCommand semanticsCommand
            elabCommand verifiedCommand
        let (transportFailed, _) ←
          reportProofMessages transports (countErrors (← get).messages > errorsBefore)
        if transportFailed then throwErrorAt reference "verex verification failed"
      for (_, name) in members do
        let base := (← getCurrNamespace) ++ artifactsOf name
        modifyEnv fun env => completedDenotations.tag env (base ++ `typedVerified)
        modifyEnv fun env => nativeDependencies.addEntry env (base, natives)
        modifyEnv fun env => assumptionDependencies.addEntry env (base, assumptionDeps)
        modifyEnv fun env => lemmaTrustDependencies.addEntry env (base, trustDeps)
      for (member, name) in members do
        withRef reference <| requireNativeArtifacts ((← getCurrNamespace) ++ artifactsOf name)
          ((declarationOf? unit member).any selectsBitVectors)
        let base := (← getCurrNamespace) ++ artifactsOf name
        modifyEnv fun env => auditedDenotations.tag env (base ++ `typedVerified)
    return { tasks := pendings.flatMap (·.tasks), conclude }
  catch failure =>
    -- A failure before the proofs' tasks began leaves no artifacts behind.
    modify fun state => { saved with messages := state.messages }
    throw failure

/-- Verify one function through its denotation, with the other members of
its cycle of calls, if any, whose authored proofs `scripts` gives.  The
members are reported to `covering` before they are verified, succeeding or
failing together. The target's proofs elaborate in tasks: the result
concludes it once they ended. With `prepare`, every definition and
certificate the proofs need is elaborated and checked, and no proof begins:
a module prepares its targets before any proof task runs, so that no kernel
check of the command waits behind the proofs. -/
def verifyFunction (reference : Syntax) (segments : Array String) (function : String)
    (script? : Option AuthoredScript := none)
    (scripts : String → Option AuthoredScript := fun _ => none)
    (covering : Array String → CommandElabM Unit := fun _ => pure ())
    (opens : Array (TSyntax ``Lean.Parser.Command.openDecl) := #[]) (prepare : Bool := false) :
    CommandElabM PendingVerification := do
  let namespaceName := pathName segments
  let some unit := VerexSL.registeredUnit? (← getEnv) namespaceName
    | throwErrorAt reference s!"unknown Verex namespace `{namespaceName}`"
  let some (namespaceIndex, ns, functionIndex, declaration) := findFunction? unit function
    | throwErrorAt reference s!"unknown function `{function}` in `{namespaceName}`"
  -- A body reading an intrinsic map's representation has no denotation: the
  -- function is used through its specification, and its body is not verified
  -- (`doc/designs/intrinsic-maps.md`, "Abstract representation").
  let representation? :=
    if (Contract.mapRoleOf? unit ⟨namespaceIndex⟩ ns declaration).isSome then none
    else Contract.representationRead? unit ns declaration
  if let some owner := representation? then
    let ownerName := ((unit.tables.names[owner.name.index]?).map (·.name)).getD "?"
    throwErrorAt reference m!"`{function}` reads the representation of the intrinsic map \
      `{ownerName}`; a function of an intrinsic map's module reading its representation is \
      used through its specification, and its body is not verified"
  let handle : FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  let base := (← getCurrNamespace) ++ Name.str namespaceName function
  let cycle := if (← getEnv).contains (base ++ `typedVerified) then #[]
    else callCycle unit handle
  if cycle.isEmpty then
    return (← verifyMember reference segments function script? opens #[] (prepare := prepare)).2
  let mut members : Array (FunctionHandle × String) := #[]
  for member in cycle do
    let some memberNs := unit.namespaces[member.namespaceId.index]?
      | throwErrorAt reference "a callee's namespace is out of range"
    let some memberDeclaration := memberNs.functions[member.functionId.index]?
      | throwErrorAt reference "a callee is out of range"
    let some name := memberNs.tables.names[memberDeclaration.name.index]?
      | throwErrorAt reference "a callee has no name"
    if member != handle then
      unless member.namespaceId == handle.namespaceId do
        throwErrorAt reference m!"`{function}` reaches itself through `{name.name}` of another \
          module"
      unless memberDeclaration.contract.loc.isSome do
        throwErrorAt reference m!"`{function}` reaches itself through `{name.name}`, which is \
          unspecified; a function on a cycle of calls is used through its contract, so specify \
          `{name.name}`"
      if automaticVerificationDisabled unit member.namespaceId memberNs memberDeclaration then
        throwErrorAt reference m!"`{function}` reaches itself through `{name.name}`, which sets \
          `pragma verify = false`; the members of a cycle of calls are verified together"
    members := members.push (member, name.name)
  covering (members.map (·.2))
  verifyCycle reference segments members (fun name =>
    if name == function then script? else scripts name) opens prepare

end VerexSL.Verify
