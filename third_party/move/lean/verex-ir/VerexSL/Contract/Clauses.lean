-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Contract.Values

/-!
# Generated contracts: the clause translator

The translation of specification expressions and clauses into Lean
propositions over native binders, one mutual block. Every value a clause
holds is native (`doc/designs/native-verification.md`): an integer is a
mathematical integer, a Boolean its truth, a textual scalar its string, any
other value the carrier of its native type at the clause's frame.
-/

namespace VerexSL.Contract

open Lean Meta Elab Command
open VerexIR (ConstValue Operation PrimitiveOperation ExprId LocalId TypeId)
open VerexIR.Validation (ValidatedUnit ValidatedNamespace)
open VerexIR.Proofs (ObligationRange)
open VerexSL.Quote

/-- A compact spelling of a native type, naming the definition of a generic
recursive specification function at an instantiation. -/
partial def ntySpelling : VerexIR.Proofs.Denote.NTy → String
  | .unit => "unit"
  | .bool => "bool"
  | .int width signed => (if signed then "i" else "u") ++ toString width
  | .num => "num"
  | .address => "address"
  | .signer => "signer"
  | .string => "string"
  | .bytes => "bytes"
  | .tuple elements => "(" ++ rowSpelling elements ++ ")"
  | .struct source arguments _ | .enum source arguments _ _ _ =>
      s!"s{source.namespaceId.index}_{source.structId}" ++
        (if arguments matches .nil then "" else "<" ++ rowSpelling arguments ++ ">")
  | .vector element => "vector<" ++ ntySpelling element ++ ">"
  | .table key value => "table<" ++ ntySpelling key ++ "," ++ ntySpelling value ++ ">"
  | .ghost slot => "ghost<" ++ ntySpelling slot ++ ">"
  | .ref referent => "&" ++ ntySpelling referent
  | .param index => s!"T{index}"
  | .function parameters shared results =>
      "|" ++ rowSpelling parameters ++ "|" ++ marks "&" shared.parameters ++
        marks "&r" shared.results ++ rowSpelling results
where
  marks (lead : String) (flags : List Bool) : String :=
    if flags.any id then lead ++ String.ofList (flags.map fun s => if s then '1' else '0') ++ ":"
    else ""
  rowSpelling : VerexIR.Proofs.Denote.NRow → String
    | .nil => ""
    | .cons τ .nil => ntySpelling τ
    | .cons τ rest => ntySpelling τ ++ "," ++ rowSpelling rest

/-- The spelling of type arguments in a name: `at_T0`, `at_u64,bool`. -/
def typeArgumentsSpelling (θ : VerexIR.Proofs.Denote.NRow) : String :=
  "at_" ++ ",".intercalate (θ.toList.map ntySpelling)

/-- The weave of a closure's mask over its target's parameters: bit `i` set
where parameter `i` is captured (`Weave.mask`). -/
partial def weaveOf : List VerexIR.Proofs.Denote.NTy → Nat → MetaM Lean.Expr
  | [], _ => pure (mkConst ``VerexIR.Proofs.Denote.Weave.nil)
  | τ :: rest, mask => do
    let inner ← weaveOf rest (mask / 2)
    let constructor := if mask % 2 == 1 then ``VerexIR.Proofs.Denote.Weave.captured
      else ``VerexIR.Proofs.Denote.Weave.supplied
    mkAppOptM constructor #[some (← quoteNTy τ), none, none, none, some inner]

/-- The native row of native values, a nested pair. -/
partial def nativeValueRow (carriers : Lean.Expr) :
    VerexIR.Proofs.Denote.NRow → List Lean.Expr → MetaM Lean.Expr
  | .nil, [] => return mkApp (mkConst ``VerexIR.Proofs.Denote.HList.nil) carriers
  | .cons τ rest, value :: values => do
      return mkAppN (mkConst ``VerexIR.Proofs.Denote.HList.cons)
        #[carriers, ← quoteNTy τ, ← quoteRow rest, value, ← nativeValueRow carriers rest values]
  | _, _ => throwError "internal: a row of native values differs from its row type"

/-- A native value read at a type differing from its own only in the widths
of the integers it holds, as a specification function's value at a call the
specification types otherwise: an integer by its value, cut to the width it
is read at; a row component by component. -/
partial def coerceNative (carriers : Lean.Expr) :
    VerexIR.Proofs.Denote.NTy → VerexIR.Proofs.Denote.NTy → Lean.Expr → MetaM Lean.Expr
  | σ, τ, value => do
    if σ == τ then return value
    match σ, τ with
    | .int _ _, .int _ _ | .int _ _, .num | .num, .int _ _ =>
        nativeOfClause carriers τ (← mkAppM ``VerexIR.SpecInt.val #[value])
    | .tuple σs, .tuple τs =>
        let components ← nativeComponents carriers (← quoteNTy σ) value
        unless components.size == σs.toList.length && σs.toList.length == τs.toList.length do
          throwError "internal: a tuple's components differ from its row"
        let coerced ← ((σs.toList.zip τs.toList).zip components.toList).mapM
          fun ((σ, τ), (_, component)) => coerceNative carriers σ τ component
        nativeValueRow carriers τs coerced
    | .vector σe, .vector τe =>
        let element ← withLocalDeclD `element (carrierType carriers (← quoteNTy σe)) fun e => do
          mkLambdaFVars #[e] (← coerceNative carriers σe τe e)
        mkAppM ``VerexIR.SpecVector.map #[element, value]
    | _, _ => throwError m!"a specification value of type {repr σ} is read at the type \
        {repr τ}, which generated contracts do not carry"

mutual
/-- A type with its integers unbounded: what values of types differing only
in the widths of their integers compare at, losslessly. -/
partial def widenedNTy : VerexIR.Proofs.Denote.NTy → VerexIR.Proofs.Denote.NTy
  | .int _ _ => .num
  | .tuple row => .tuple (widenedNRow row)
  | .vector element => .vector (widenedNTy element)
  | τ => τ

partial def widenedNRow : VerexIR.Proofs.Denote.NRow → VerexIR.Proofs.Denote.NRow
  | .nil => .nil
  | .cons τ rest => .cons (widenedNTy τ) (widenedNRow rest)
end

/-- The type an uninterpreted function takes an argument at: its
parameter's declared type with its integers unbounded (`widenedNTy`), as the
Prover's uninterpreted functions take them, at the call's type arguments,
whose own integers stay as the function's contract instantiates them. -/
def Context.uninterpretedParameterType (callee : Context) (typeId : TypeId) :
    MetaM VerexIR.Proofs.Denote.NTy := do
  let widened := widenedNTy (← { callee with typeArgumentTypes := #[] }.nativeType typeId)
  if callee.typeArgumentTypes.isEmpty then return widened
  let some row := callee.typeArgumentTypes.toList.mapM id
    | throwError "a type argument of an uninterpreted function has no native type"
  return widened.subst (VerexIR.Proofs.Denote.NRow.ofList row)

/-- An invocation of a function value a behavioral predicate and its
defined label share (`Proofs/Behavior.lean`): the invocation meaning, the
closure, the supplied row and result shape, each parameter's entry value
(a mutable reference's at the pre-state), and the pre-state. -/
structure Invocation where
  unit : Lean.Expr
  functions : Lean.Expr
  frame : Lean.Expr
  invoke : Lean.Expr
  function : Lean.Expr
  row : VerexIR.Proofs.Denote.NRow
  shared : VerexIR.Proofs.Denote.Sharing
  shape : VerexIR.Proofs.Denote.ResultShape
  /-- Each parameter's entry value, at its type (a mutable reference's at
  its referent's). -/
  entries : Array Lean.Expr
  pre : Lean.Expr

/-- The arguments of an invocation: each mutable reference as its entry
and final value, its final value `finals` gives or, settled, its entry. -/
def Invocation.arguments (invocation : Invocation) (finals : Array Lean.Expr := #[]) :
    MetaM Lean.Expr := do
  let carriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) invocation.unit
    invocation.functions invocation.frame
  let mut values : Array Lean.Expr := #[]
  let mut finalIndex := 0
  for (τ, entry) in invocation.row.toList.zip invocation.entries.toList do
    match τ with
    | .ref _ =>
        let final := finals[finalIndex]?.getD entry
        finalIndex := finalIndex + 1
        values := values.push (← mkAppM ``Prod.mk #[entry, final])
    | _ => values := values.push entry
  nativeValueRow carriers invocation.row values.toList

/-- A behavioral predicate of an invocation, applied to the invocation's
meaning, row, shared flags, shape, and function value, then `rest`. -/
def Invocation.predicate (invocation : Invocation) (predicate : Name) (rest : Array Lean.Expr) :
    MetaM Lean.Expr := do
  return mkAppN (mkConst predicate)
    (#[invocation.unit, invocation.functions, invocation.frame, invocation.invoke,
      ← quoteRow invocation.row,
      toExpr invocation.shared, ← quoteShape invocation.shape, invocation.function] ++ rest)

/-- A value of a specification binder at a type, as a bundle component: an
integer, a Boolean decided, a string, or a native value. -/
def bundleComponent (domain : Domain) (clause : Lean.Expr) : MetaM Lean.Expr :=
  match domain with
  | .boolean => decideProposition clause
  | _ => pure clause

mutual
/-- A let binding a variable to a variant field whose type has no value to
stand for it under another variant: the local, its carrier, and the
selection (`Choices.select?`) of the translated base. -/
private partial def Context.heldVariantField? (context : Context) (pattern : VerexIR.PatternId)
    (initializer : ExprId) : MetaM (Option (VerexIR.LocalId × Lean.Expr × Lean.Expr)) := do
  let some { kind := .variable localId, .. } := context.ns.patterns[pattern.index]? | return none
  let some { kind := .operation (.data (.selectVariants reference fields)) _ #[base] _, .. } :=
      context.ns.expressions[initializer.index]? | return none
  let handle ← structHandleOf context reference
  let some choices := VerexIR.SemanticOperations.variantFieldChoices? context.unit handle fields
    | return none
  let τ ← context.exprType base
  let carriers ← context.carriers
  let τExpr ← quoteNTy τ
  let value ← translate context base
  if (← typedVariantField? τ choices carriers τExpr value).isSome then return none
  let some (fieldType, selection) ← typedVariantSelection? τ choices carriers τExpr value
    | return none
  return some (localId, carrierType carriers fieldType, selection)

/-- Bind a specification let in its lexical scope, also used when a free
state-label definition is read from another clause. -/
private partial def Context.bindLet (context : Context) (pattern : VerexIR.PatternId)
    (initializer : ExprId) : MetaM Context := do
  let some patternNode := context.ns.patterns[pattern.index]?
    | throwError "specification binding pattern {pattern.index} is out of range"
  match patternNode.kind with
  | .wildcard => pure context
  | .variable localId =>
      let some patternType := context.valueTypeOf? patternNode.typeId
        | throwError "specification binding pattern has an unknown type"
      context.bindLocal localId patternType
        (← decidedAt patternType (← translate context initializer))
  | .tuple elements | .constructor _ _ _ elements =>
      let literal? : Option (Array ExprId) :=
        match patternNode.kind, context.ns.expressions[initializer.index]? with
        | .tuple _, some { kind := .operation (.primitive .tuple) _ components _, .. } =>
            some components
        | _, _ => none
      let some components := literal? | do
        -- Any other tuple or constructor value binds its components by
        -- position, its row's or its variant's.
        let translated ← translate context initializer
        let τ ← context.exprType initializer
        let carriers ← context.carriers
        let τExpr ← quoteNTy τ
        let variant? := match patternNode.kind with
          | .constructor _ _ variant _ => variant
          | _ => none
        let componentAt (index : Nat) : MetaM Lean.Expr := do
          match τ, variant? with
          | .enum .., some variant => do
              let (_, read) ← nativeVariantField τ #[(variant, index)] carriers τExpr translated
              pure read
          | _, _ => do
              let some (_, component) := (← nativeComponents carriers τExpr translated)[index]?
                | throwError "a destructuring pattern binds a component the value lacks"
              pure component
        let mut bound := context
        for (element, index) in elements.zipIdx do
          bound ← bound.bindComponent element "destructuring" fun elementType => do
            (domainOf elementType).binderOfNative (← componentAt index)
        pure bound
      -- A tuple bound from a tuple literal binds component-wise, every
      -- component taken in the scope before the binding, as an inlined
      -- function's parameters are bound from its arguments.
      unless components.size == elements.size do
        throwError "a tuple pattern binds a tuple of another arity"
      let mut bound := context
      for (element, component) in elements.zip components do
        bound ← bound.bindComponent element "tuple" fun elementType => do
          decidedAt elementType (← translate context component)
      pure bound
  | _ =>
      throwError "generated contracts require a variable or wildcard binding pattern"

/-- The state a label denotes in this clause. -/
private partial def Context.labelState (context : Context) (label : Nat) : MetaM LabelState := do
  if let some (_, state) := context.stateLabels.find? (·.1 == label) then return state
  let name := (context.unit.tables.names[label]?.map (·.name)).getD s!"{label}"
  if context.resolvingLabels.contains label then
    throwError "cyclic definition of state label `{name}`"
  let some definition := context.labelDefinitions.find? (·.label == label)
    | throwError "state label `{name}` has no definition in this contract"
  let some expression := context.ns.expressions[definition.operation.index]?
    | throwError "state label definition is out of range"
  let .operation (.specification operation) instantiations arguments _ := expression.kind
    | throwError "state label definition is not a two-state operation"
  let context := { context with resolvingLabels := label :: context.resolvingLabels }
  let context := match context.labelEntry, context.labelExit with
    | some entry, some exit => { context with
        oldState := some entry.memory
        oldLocals := entry.locals.zipWith (·.or ·) context.oldLocals
        state := some exit.memory
        locals := exit.locals.zipWith (·.or ·) context.locals }
    | _, _ => context
  let context ← definition.bindings.foldlM (init := context) fun context (pattern, initializer) =>
    context.bindLet pattern initializer
  let memory ← match operation with
    | .behavior .ensuresOf range | .behavior .resultOf range => do
        -- The post-state of the run `result_of` reads.
        let invocation ← translateInvocation context range arguments
        invocation.predicate ``VerexIR.Proofs.StateOf #[← invocation.arguments, invocation.pre]
    | _ => (·.1) <$> translateStateChange context operation instantiations arguments
  return { memory, locals := context.oldLocals }

/-- The clause's states under a memory range: a pre-state label replaces the
state `old` reads and the locals' entry values, a post-state label the current
state and the locals' current values. -/
private partial def Context.atRange (context : Context) (range : VerexIR.MemoryRange) : MetaM Context := do
  let (oldState, oldLocals) ← match range.pre with
    | some label => do
        let state ← context.labelState label
        pure (some state.memory, state.locals)
    | none => pure (context.oldState, context.oldLocals)
  let (state, locals) ← match range.post with
    | some label => do
        let state ← context.labelState label
        pure (some state.memory, state.locals)
    | none => pure (context.state, context.locals)
  return { context with oldState, oldLocals, state, locals }

/-- A state change computes a memory and a presence condition. Naming the
memory does not assume the condition; it remains an obligation of the predicate. -/
private partial def translateStateChange (context : Context)
    (operation : VerexIR.SpecOperation) (instantiations : Array VerexIR.GenericArgument)
    (arguments : Array ExprId) (post : Option Lean.Expr := none) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let range ← match operation with
    | .publish range | .remove range | .update range => pure range
    | _ => throwError "expected a state-change predicate"
  let pre ← match range.pre with
    | some label => (·.memory) <$> context.labelState label
    | none => match context.oldState with
      | some state => pure state
      | none => throwError "a state-change predicate has no pre-state"
  let [.typeArg resourceUse] := instantiations.toList
    | throwError "a state-change predicate needs one resource type"
  let some key := arguments[0]? | throwError "a state-change predicate needs an address"
  let keyValue := mkApp (mkConst ``VerexIR.StorageKey.address) (← translate context key)
  let (slot, resource, nativeType) ← context.slot pre resourceUse.typeId keyValue
  let present ← slotPresent slot
  let condition := if operation matches .publish _ then mkNot present else present
  let frame ← context.frame
  let leading ← frameLeading frame
  let carrier := mkAppN (mkConst ``VerexIR.Proofs.Denote.ResourceType.carrier) (leading.push resource)
  let mut valueCondition := mkConst ``True
  let value ← if operation matches .remove _ then
      pure (mkApp (mkConst ``Option.none [Level.zero]) carrier)
    else do
      let some value := arguments[1]? | throwError "a state-change predicate needs a value"
      let stored ← translate context value
      if let some post := post then
        let slot := memorySlot post resource keyValue
        -- Bind the slot's value explicitly. A caller can destruct this
        -- proposition and read its fields without a program point or a
        -- reconstruction of the native row (whose empty tail is opaque).
        valueCondition ← withLocalDeclD `stored carrier fun held => do
          let someStored ← mkAppM ``Option.some #[held]
          let read := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.ofRuntime)
            (leading ++ #[frame, nativeType, held])
          mkAppM ``Exists #[← mkLambdaFVars #[held]
            (← mkAppM ``And #[← mkEq slot someStored, ← mkEq read stored])]
        pure slot
      else
        let transport := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.toRuntime)
          (leading ++ #[frame, nativeType, stored])
        mkAppOptM ``Option.some #[carrier, transport]
  let memory ← mkAppM ``VerexIR.Proofs.Denote.Memory.set #[pre, resource, keyValue, value]
  let condition ← if operation matches .remove _ then pure condition
    else mkAppM ``And #[condition, ← mkAppM ``And #[← slotPresent value, valueCondition]]
  return (memory, condition)

/-- The context reading each mutable reference parameter at a state label,
or, without one, at its entry value (`entry`) or its current one: an
invocation reads the references it lends at its pre-state, as the Prover
reads `old` of them, and `ensures_of` their final values at its post-state. -/
private partial def Context.mutableAt (context : Context) (label : Option Nat) (entry : Bool) :
    MetaM Context := do
  let values ← match label with
    | some label => pure (← context.labelState label).locals
    | none => pure (if entry then context.oldLocals else context.locals)
  let locals := context.mutableParameters.foldl (init := context.locals) fun locals parameter =>
    match values[parameter]? with
    | some (some value) => locals.set! parameter (some value)
    | _ => locals
  return { context with locals }

/-- The invocation of a function value a behavioral predicate and its
defined label share: the invocation meaning (the executable's big-step run,
or a model's meaning), the closure, its native row and shape, each
parameter's entry value, and the pre-state. -/
private partial def translateInvocation (context : Context) (range : VerexIR.MemoryRange)
    (arguments : Array ExprId) : MetaM Invocation := do
  let some executable := context.executable
    | throwError "a behavioral predicate reads the executable unit, which this contract \
        does not take"
  let some callable := arguments[0]?
    | throwError "a behavioral predicate names a function value"
  let some callableExpression := context.ns.expressions[callable.index]?
    | throwError "a behavioral predicate's function value is out of range"
  let some (.function parameters _ _) := context.valueTypeOf? callableExpression.typeId
    | throwError "a behavioral predicate's operand is not a function value"
  let .function row shared results ← context.exprType callable
    | throwError "a behavioral predicate's function value has no native function type"
  let shape : VerexIR.Proofs.Denote.ResultShape ← match results with
    | .nil => pure .none
    | .cons τ .nil => pure (.one τ)
    | _ => throwError "a behavioral predicate's function value returns several results, which \
        an invocation does not carry"
  let frame ← context.frame
  let unit ← frameUnit frame
  let functions ← VerexIR.Proofs.Denote.functionCarriersOfFrame frame
  -- Over a model (`doc/designs/ghost-fields.md`), the model's invocation meaning
  -- at the frame; otherwise the executable's big-step run.
  let invoke ← match ← modelInvoke? executable frame with
    | some invoke => pure invoke
    | none => pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.closureMeaning)
        #[unit, functions, executable, frame])
  -- A literal, and the literal a struct literal's field holds, is the
  -- function value of its target, typed at the leaf's unit.
  let mut literal := callableExpression
  for _ in [0:context.ns.expressions.size] do
    let .operation (.data (.select reference field)) _ #[base] _ := literal.kind | break
    let some baseExpression := context.ns.expressions[base.index]? | break
    let .operation (.call (.constructor _ none)) _ operands _ := baseExpression.kind | break
    let some index := VerexIR.SemanticOperations.referencedFieldIndex?
        context.unit context.namespaceId reference none field | break
    let some operand := operands[index]? | break
    let some operandExpression := context.ns.expressions[operand.index]? | break
    literal := operandExpression
  let function ← match literal.kind with
    | .operation (.call (.closure reference mask)) instantiations captures _ =>
        functionLiteral context reference mask instantiations captures
    | .operation (.data (.selectVariants _ _)) _ _ _ =>
        throwError "a behavioral predicate of a variant's function field is not carried in \
          generated contracts: under another variant the field has no value, and a function \
          type has no default"
    | _ => translate context callable
  -- A mutable reference is lent its entry value, read at the pre-state.
  let mutable := parameters.map fun parameter =>
    match context.typeOf? parameter with
    | some (.reference reference) => reference.kind == .mutable
    | _ => false
  let entriesContext ← if mutable.contains true then context.mutableAt range.pre (entry := true)
    else pure context
  let inputs := arguments.extract 1 (parameters.size + 1)
  let carriers ← context.carriers
  let mut entries := #[]
  for (input, isMutable, τ) in inputs.toList.zip (mutable.toList.zip row.toList) do
    let referent := match τ with
      | .ref σ => σ
      | σ => σ
    let value ← translate (if isMutable then entriesContext else context) input
    entries := entries.push (← nativeOfClause carriers referent value)
  let pre ← match range.pre, context.oldState with
    | some label, _ => (·.memory) <$> context.labelState label
    | none, some state => pure state
    | none, none => do
        let some state := context.state | throwError "an invocation needs a pre-state"
        pure state
  return { unit, functions, frame, invoke, function, row, shared, shape, entries, pre }

/-- A literal function value of a target without type parameters, its
captures native, its mask the target's parameters woven, as the program
spells a closure's (`Weave.mask`), so that both meet one form. It is a value
of its function type typed by the unit's check (`SiteTyped.ofCheck`), which a
contract, generic in its unit, does not decide: the proposition reading it
binds the check (`bindCuts`), and a leaf at the verified unit reads it off
the literal's closure site. -/
private partial def functionLiteral (context : Context) (reference : VerexIR.QualifiedRef)
    (mask : Nat) (instantiations : Array VerexIR.GenericArgument) (arguments : Array ExprId) :
    MetaM Lean.Expr := do
  unless instantiations.isEmpty do
    throwError "a function value with type arguments is not carried in generated contracts"
  let some handle := VerexIR.SemanticOperations.resolveFunction? context.unit
      context.namespaceId reference
    | throwError "a function value's target does not resolve"
  let some (parameters, _) := VerexIR.Proofs.Denote.closureSignature? context.unit handle
    | throwError "a function value's target has no native signature"
  let captured := VerexIR.Proofs.Denote.NRow.ofList (VerexIR.ClosureMask.extract mask true parameters)
  unless captured.refFree do
    throwError "a function value captures a reference"
  let captures ← nativeRow (← context.carriers) captured (← arguments.toList.mapM (translate context))
  let some (_, supplied, results) := VerexIR.Proofs.Denote.closureRows? context.unit handle mask
    | throwError "a function value's target has no native rows"
  let shared := VerexIR.Proofs.Denote.closureShared context.unit handle mask
  let frame ← context.frame
  let unit ← frameUnit frame
  let functions ← VerexIR.Proofs.Denote.functionCarriersOfFrame frame
  let value := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.construct)
    ((← frameLeading frame) ++ #[frame, ← quoteRow supplied, toExpr shared, ← quoteRow results,
      toExpr handle, ← mkAppM ``VerexIR.Proofs.Denote.Weave.mask #[← weaveOf parameters mask],
      ← mkArrayLit (← mkAppM ``Prod #[mkConst ``TypeId, mkConst ``TypeId]) [],
      ← quoteRow captured, captures])
  let check ← mkEq (mkAppN (mkConst ``VerexIR.Proofs.Denote.closureTypingCheck)
      #[unit, toExpr handle, value.getArg! 7, ← quoteRow captured, ← quoteRow supplied,
        ← quoteRow results, toExpr shared])
    (mkConst ``Bool.true)
  let typed ← mkAppOptM ``VerexIR.Proofs.Denote.SiteTyped.ofCheck
    #[unit, functions, frame, none, none, none, none, none, none,
      ← mkFreshExprMVar check (kind := .syntheticOpaque)]
  return mkApp value typed

/-- An operand as a native value of its type. -/
partial def nativeOperand (context : Context) (id : ExprId) :
    MetaM (Lean.Expr × VerexIR.Proofs.Denote.NTy) := do
  let τ ← context.exprType id
  return (← nativeOfClause (← context.carriers) τ (← translate context id), τ)

/-- Logical ranges are either explicit bounds or the valid indices of a
vector. Observe the vector in the active context, including old/labeled
memory, just as a direct length expression does. -/
private partial def translateRangeBounds (context : Context) (id : ExprId) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "a specification range is out of range"
  match expression.kind with
  | .operation (.primitive .range) _ #[lower, upper] _ =>
      return (← translate context lower, ← translate context upper)
  | .operation (.specification .vectorRange) _ #[vector] _ =>
      let vector ← translate context vector
      let values ← mkAppM ``VerexIR.SpecVector.values #[vector]
      return (toExpr (0 : Int), ← mkAppOptM ``Nat.cast #[mkConst ``Int, none, ← mkAppM ``Array.size #[values]])
  | _ => throwError "a specification range must supply bounds or a vector's indices"

/-- Translate one specification expression into a Lean term: a proposition
for a Boolean, an integer for an integer, a string for a textual scalar,
and the native value for any other type. A Boolean binds the integers its
own terms store in fixed-width values (`bindCuts`): one out of the width's
range builds no value. -/
partial def translate (context : Context) (id : ExprId) : MetaM Lean.Expr := do
  let term ← translateExpression context id
  let some expression := context.ns.expressions[id.index]? | return term
  let some ty := context.valueTypeOf? expression.typeId | return term
  if domainOf ty == .boolean then bindCuts term else return term

/-- `translate` but for binding the cuts. -/
partial def translateExpression (context : Context) (id : ExprId) : MetaM Lean.Expr := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "specification expression {id.index} is out of range"
  let some ty := context.valueTypeOf? expression.typeId
    | throwError "specification expression {id.index} has an unknown type"
  match expression.kind with
  | .value literal _ => translateLiteral literal ty expression.typeId
  | .constant reference =>
      -- A named constant denotes its defining literal, translated in the
      -- declaring namespace.  The definition is executable-typed, so the
      -- integer arm of the literal translation accepts fixed widths.
      let some handle := VerexIR.SemanticOperations.resolveConstant?
          context.unit context.namespaceId reference
        | throwError "specification constant does not resolve in generated contracts"
      let some targetNs := context.unit.namespaces[handle.namespaceId.index]?
        | throwError "specification constant namespace is out of range"
      let some declaration := targetNs.constants[handle.constantId]?
        | throwError "specification constant is out of range"
      translate { context with
        ns := targetNs, namespaceId := handle.namespaceId
        locals := #[], localTypes := #[], oldLocals := #[], results := #[] }
        declaration.value
  | .localVar localId =>
      let some (some binder) := context.locals[localId.index]?
        | throwError "specification local {localId.index} has no binder"
      let logicalType := context.localTypes[localId.index]?.getD ty
      (domainOf logicalType).ofBinder binder
  -- A specification function read at state labels: its arguments read the
  -- labels' states as the old and current ones.
  | .operation (.specification (.functionCall _ range)) _ _ _ =>
      if (range.pre.isSome || range.post.isSome) && context.rangeApplied != some id then
        translate { (← context.atRange range) with rangeApplied := some id } id
      else
        let .operation operation instantiations arguments _ := expression.kind | unreachable!
        translateOperation operation instantiations arguments ty expression.typeId
  | .operation operation instantiations arguments _ =>
      translateOperation operation instantiations arguments ty expression.typeId
  | .quantifier kind binders triggers condition body =>
      unless triggers.isEmpty do
        throwError "quantifier triggers are not supported in generated contracts"
      let rec bind (index : Nat) (active : Context) : MetaM Lean.Expr := do
        if h : index < binders.size then
          let binder := binders[index]
          let some pattern := active.ns.patterns[binder.pattern.index]?
            | throwError "quantifier pattern {binder.pattern.index} is out of range"
          let .variable localId := pattern.kind
            | throwError "generated contracts currently require a variable quantifier pattern"
          let some domainExpression := active.ns.expressions[binder.domain.index]?
            | throwError "quantifier domain {binder.domain.index} is out of range"
          let some patternType := active.valueTypeOf? pattern.typeId
            | throwError "quantifier pattern type {pattern.typeId.index} is out of range"
          let some domainType := active.valueTypeOf? domainExpression.typeId
            | throwError "quantifier domain type {domainExpression.typeId.index} is out of range"
          let domain := domainOf patternType
          let binderName := match active.localNames[localId.index]? with
            | some name => if name.isEmpty then Name.mkSimple s!"quantified_{localId.index}"
                else Name.mkSimple name
            | none => Name.mkSimple s!"quantified_{localId.index}"
          let range (lower upper : Lean.Expr) (value : Lean.Expr) : MetaM Lean.Expr := do
            mkAppM ``And #[← mkAppM ``LE.le #[lower, value], ← mkAppM ``LT.lt #[value, upper]]
          -- What the quantifier binds, the condition a bound value satisfies
          -- to be in the domain when not every value is, and the value the
          -- pattern's local takes.
          let (binderName, binderType, membership?, element) ← match domainType with
            -- A type's domain: a bounded integer type ranges over its values,
            -- bound at the widened logical type, any other type over its
            -- native values.
            | .typeDomain declaredId =>
                let some declared := active.typeOf? declaredId
                  | throwError "quantifier domain type {declaredId.index} is out of range"
                match domain with
                | .native =>
                    unless patternType == declared do
                      throwError "quantifier pattern and type domain differ"
                    let nty ← active.nativeType declaredId
                    -- A nominal's native carrier is its field row. For
                    -- function-valued fields that is not yet MVP's value
                    -- domain: MVP also ties the field to this nominal
                    -- instantiation. Quantifying over the unrestricted row
                    -- would let an invariant of G<u64> constrain a closure
                    -- taken from G<bool>. Reject until that domain is carried.
                    if declared matches .nominal .. then
                      unless nty.closureFree do
                        throwError "quantification over nominal values containing function fields requires a field-validity domain, which is not carried yet"
                    pure (binderName, carrierType (← active.carriers) (← quoteNTy nty), none,
                      fun (value : Lean.Expr) => (pure value : MetaM Lean.Expr))
                | _ =>
                -- A bounded integer type an instantiation gives the pattern
                -- ranges over its values.
                let membership? ← if patternType == declared then pure (boundedMembership? declared)
                  else
                  match patternType, declared with
                  | .integer .unbounded _, .integer (.bits _) _ => pure (boundedMembership? declared)
                  | _, _ => throwError "quantifier pattern and type domain differ"
                let some type := domain.scalarType? | unreachable!
                pure (binderName, type, membership?,
                  fun (value : Lean.Expr) => (pure value : MetaM Lean.Expr))
            -- A range ranges over the integers from its lower bound, below
            -- its upper bound.
            | .range =>
                let (lower, upper) ← translateRangeBounds active binder.domain
                pure (binderName, mkConst ``Int, some (range lower upper), fun value => pure value)
            -- A vector ranges over its elements: the pattern binds an element
            -- of the vector, the form the `contains` and `index_of`
            -- primitives denote to.
            | .vector _ _ =>
                let vector ← translate active binder.domain
                nativeVectorBinder (← active.carriers) domain (← active.exprType binder.domain) vector
                  binderName
            -- A binder over the state domain binds a memory, the state its
            -- label denotes.
            | .stateDomain =>
                let some skolems := active.skolems?
                  | throwError "a state label needs the contract's skolem instance"
                let memory := mkAppN (mkConst ``VerexIR.Proofs.Denote.Memory) (← frameLeading skolems)
                pure (binderName, memory, none, fun value => pure value)
            | _ => throwError "a quantifier domain must be a type, a range, or a vector"
          -- A state label also binds the value of each mutable reference
          -- parameter at its state, as the Prover keeps a copy per label.
          let mutable := if domainType matches .stateDomain then active.mutableParameters else #[]
          let copies ← mutable.mapM fun parameter => do
            let some (some entry) := active.oldLocals[parameter]?
              | throwError "a mutable reference parameter has no entry binder"
            let localName := (active.localNames[parameter]?).getD s!"local_{parameter}"
            pure (Name.mkSimple s!"{binderName}_{localName}",
              fun (_ : Array Lean.Expr) => inferType entry)
          withLocalDeclD binderName binderType fun value =>
          withLocalDeclsD copies fun copies => do
            let element ← element value
            let labelLocals := (mutable.zip copies).foldl
              (fun locals (parameter, copy) => locals.set! parameter (some copy)) active.locals
            let stateLabels := match binder.label with
              | some label => (label, { memory := value, locals := labelLocals }) :: active.stateLabels
              | none => active.stateLabels
            let membership ← membership?.mapM (· value)
            -- The membership is a local of the body: a bounded integer's
            -- certifies the integer where the body reads it natively
            -- (`nativeOfClause`).
            let withMembership (k : Option Lean.Expr → MetaM Lean.Expr) : MetaM Lean.Expr :=
              match membership with
              | some member => withLocalDeclD `fits member fun fits => k (some fits)
              | none => k none
            withMembership fun fits? => do
            let inner ← bind (index + 1) { active with
              locals := active.locals.set! localId.index (some element)
              oldLocals := active.oldLocals.set! localId.index (some element)
              stateLabels
              lemmaQuantified := true }
            let bound := #[value] ++ copies
            let existsOver (body : Lean.Expr) : MetaM Lean.Expr :=
              bound.foldrM (init := body) fun binder body => do
                mkAppM ``Exists #[← mkLambdaFVars #[binder] body]
            match kind, membership, fits? with
            | .forall, none, _ => mkForallFVars bound inner
            | .forall, some _, some fits => mkForallFVars (bound.push fits) inner
            | .exists, none, _ => existsOver inner
            | .exists, some member, some fits =>
                if inner.containsFVar fits.fvarId! then
                  existsOver (← mkAppM ``Exists #[← mkLambdaFVars #[fits] inner])
                else existsOver (← mkAppM ``And #[member, inner])
            | _, _, _ => throwError "choice quantifiers are not supported in generated contracts"
        else
          let proposition ← translate active body
          match condition with
          | none => pure proposition
          | some guard =>
              let guard ← translate active guard
              match kind with
              | .forall => pure (← mkArrow guard proposition)
              | .exists => mkAppM ``And #[guard, proposition]
              | _ => throwError "choice quantifiers are not supported in generated contracts"
      bind 0 context
  | .match_ scrutinee arms =>
      if arms.isEmpty then
        throwError "a specification match must contain an arm"
      -- The term below deliberately makes the final arm unconditional, as
      -- source matching does after an exhaustive decision tree. Establish
      -- that invariant here instead of giving a partial match a total but
      -- unintended logical meaning.
      let mut owner? : Option VerexIR.StructHandle := none
      let mut declaredVariants : Array String := #[]
      let mut coveredVariants : Array String := #[]
      let mut hasWildcard := false
      for (arm, index) in arms.zipIdx do
        let some pattern := context.ns.patterns[arm.pattern.index]?
          | throwError "specification match pattern {arm.pattern.index} is out of range"
        match pattern.kind with
        | .wildcard =>
            if index + 1 < arms.size then
              throwError "a wildcard arm in a specification match must be last"
            hasWildcard := true
        | .constructor owner _ (some variant) _ =>
            let some qualified := context.unit.tables.names[owner.index]?
              | throwError "specification match constructor has an unknown owner"
            let reference : VerexIR.QualifiedRef := {
              namespaceId := qualified.namespaceId, name := owner }
            let some handle := VerexIR.SemanticOperations.resolveStruct?
                context.unit context.namespaceId reference
              | throwError "specification match constructor does not resolve"
            if let some previous := owner? then
              unless previous == handle do
                throwError "specification match arms name different enum owners"
            else
              owner? := some handle
              let some targetNs := context.unit.namespaces[handle.namespaceId.index]?
                | throwError "specification match constructor namespace is out of range"
              let some declaration := targetNs.structs[handle.structId]?
                | throwError "specification match constructor declaration is out of range"
              declaredVariants := declaration.variants.filterMap fun declaration =>
                (context.unit.tables.names[declaration.name.index]?).map (·.name)
            unless declaredVariants.contains variant do
              throwError "specification match names unknown variant `{variant}`"
            coveredVariants := coveredVariants.push variant
        | _ =>
            throwError "generated contracts require enum-constructor or wildcard match patterns"
      unless hasWildcard || declaredVariants.all coveredVariants.contains do
        let missing := declaredVariants.filter (!coveredVariants.contains ·)
        throwError "a specification match must cover every variant (missing: \
          {String.intercalate ", " missing.toList})"
      let scrutineeValue ← translate context scrutinee
      let enum ← context.exprType scrutinee
      let carriers ← context.carriers
      let enumType ← quoteNTy enum
      let mut lowered : Array (Option Lean.Expr × Lean.Expr) := #[]
      for arm in arms do
        if arm.guard.isSome then
          throwError "guards in specification matches are not supported in generated contracts"
        let some pattern := context.ns.patterns[arm.pattern.index]?
          | throwError "specification match pattern {arm.pattern.index} is out of range"
        let (condition?, armContext, universal) ← match pattern.kind with
          | .wildcard => pure (none, context, #[])
          | .constructor owner _ (some variant) fields => do
              let some qualified := context.unit.tables.names[owner.index]?
                | throwError "specification match constructor has an unknown owner"
              let reference : VerexIR.QualifiedRef := {
                namespaceId := qualified.namespaceId, name := owner }
              let some handle := VerexIR.SemanticOperations.resolveStruct?
                  context.unit context.namespaceId reference
                | throwError "specification match constructor does not resolve"
              let condition ← nativeVariantTest carriers enumType scrutineeValue handle #[variant]
              let mut armLocals := context.locals
              let mut armOldLocals := context.oldLocals
              let mut universal := #[]
              for (childId, index) in fields.zipIdx do
                let some child := context.ns.patterns[childId.index]?
                  | throwError "specification match child pattern {childId.index} is out of range"
                match child.kind with
                | .wildcard => pure ()
                | .variable localId =>
                    let some childType := context.valueTypeOf? child.typeId
                      | throwError "specification match child has an unknown type"
                    if let some (_, selected) ← typedVariantField? enum #[(variant, index)] carriers
                        enumType scrutineeValue then
                      let value ← (domainOf childType).binderOfNative selected
                      armLocals := armLocals.set! localId.index (some value)
                      armOldLocals := armOldLocals.set! localId.index (some value)
                    else
                      -- A field of a type without a value to stand for it
                      -- elsewhere, such as a function value: bound to what
                      -- the selection holds, in a Boolean arm.
                      let some (fieldType, selection) ← typedVariantSelection? enum
                          #[(variant, index)] carriers enumType scrutineeValue
                        | throwError "internal: a variant field selection does not resolve"
                      unless domainOf ty == .boolean do
                        throwError "a specification match binds a field of a type without a \
                          default value outside a Boolean clause, which generated contracts \
                          do not carry"
                      universal := universal.push (localId, carrierType carriers fieldType, selection)
                | _ =>
                    throwError "generated contracts require variable or wildcard enum payload patterns"
              pure (some condition, { context with
                locals := armLocals, oldLocals := armOldLocals }, universal)
          | _ =>
              throwError "generated contracts require enum-constructor or wildcard match patterns"
        -- Each universally bound field: for what the selection holds.
        let rec bindFields (armContext : Context) (index : Nat) : MetaM Lean.Expr := do
          let some (localId, carrier, selection) := universal[index]?
            | translate armContext arm.body
          withLocalDeclD `field carrier fun field => do
            let held ← mkEq selection (← mkAppM ``Option.some #[field])
            let body ← bindFields { armContext with
              locals := armContext.locals.set! localId.index (some field)
              oldLocals := armContext.oldLocals.set! localId.index (some field) } (index + 1)
            mkForallFVars #[field] (← mkArrow held body)
        let body ← bindFields armContext 0
        lowered := lowered.push (condition?, body)
      let mut result? : Option Lean.Expr := none
      for (condition?, body) in lowered.reverse do
        result? ← match result?, condition? with
          | none, _ => pure (some body)
          | some _, none => pure (some body)
          | some fallback, some condition =>
              some <$> conditional condition body fallback
      let some result := result? | unreachable!
      pure result
  | .ifElse condition thenBranch (some elseBranch) =>
      if context.definition.isSome then
        -- Inside a recursive definition the branches know their condition,
        -- which a recursive call's termination may need.
        let test ← translate context condition
        let branch (fact : Lean.Expr) (body : ExprId) : MetaM Lean.Expr :=
          withLocalDeclD `fact fact fun hypothesis => do
            let term ← translate { context with facts := context.facts.push hypothesis } body
            mkLambdaFVars #[hypothesis] term
        let thenTerm ← branch test thenBranch
        let elseTerm ← branch (mkNot test) elseBranch
        let resultType := (← inferType thenTerm).bindingBody!
        return mkAppN (mkConst ``dite [← getLevel resultType])
          #[resultType, test, mkApp (mkConst ``Classical.propDecidable) test, thenTerm, elseTerm]
      -- A conditional in a clause is Lean's `ite` on the decided test.
      let test ← translate context condition
      let thenTerm ← translate context thenBranch
      let elseTerm ← translate context elseBranch
      conditional test thenTerm elseTerm
  | .letDecl pattern (some initializer) body =>
      -- A variant field of a type without a value to stand for it under
      -- another variant, such as a function value: bound to what the
      -- selection holds, in a Boolean clause.
      if let some (localId, carrier, selection) ← context.heldVariantField? pattern initializer then
        unless domainOf ty == .boolean do
          throwError "a specification let binds a variant field of a type without a default \
            value outside a Boolean clause, which generated contracts do not carry"
        return ← withLocalDeclD `field carrier fun field => do
          let held ← mkEq selection (← mkAppM ``Option.some #[field])
          let body ← translate { context with
            locals := context.locals.set! localId.index (some field)
            oldLocals := context.oldLocals.set! localId.index (some field) } body
          mkForallFVars #[field] (← mkArrow held body)
      translate (← context.bindLet pattern initializer) body
  | .block statements (some result) =>
      -- A block denotes its result after its statements: an assignment to a
      -- local rebinds it for the rest, every other statement must be
      -- without logical effect.
      let mut active := context
      for statement in statements do
        let some node := active.ns.expressions[statement.index]?
          | throwError "specification statement {statement.index} is out of range"
        match node.kind with
        | .assign place value =>
            let some (.localVar localId) := active.ns.places[place.index]?
              | throwError "a specification reading assigns only locals, not a place \
                  through a reference, field, or element"
            unless localId.index < active.locals.size do
              throwError "specification local {localId.index} has no binder slot"
            let some valueType := typeOfExpr? active value
              | throwError "an assigned value has an unknown type"
            let translated ← translate active value
            let translated ← match domainOf valueType with
              | .boolean => decideProposition translated
              | _ => pure translated
            active := { active with
              locals := active.locals.set! localId.index (some translated)
              oldLocals := active.oldLocals.setIfInBounds localId.index (some translated) }
        | kind =>
            unless withoutLogicalEffect statement do
              throwError "specification statement {repr kind} is not supported in \
                generated contracts"
      translate active result
  | kind =>
      throwError "specification node {repr kind} is not supported in generated contracts"
where
  /-- A statement a specification reading may drop: its value is discarded,
  so only a local mutation or a control transfer could reach the rest of the
  block. A failure imposes no condition. -/
  withoutLogicalEffect (id : ExprId) : Bool :=
    match context.ns.expressions[id.index]? with
    | none => false
    | some expression =>
        match expression.kind with
        | .value .. | .constant _ | .localVar _ | .quantifier .. | .spec _ => true
        | .operation (.write _) _ _ _ => false
        | .operation _ _ arguments _ => arguments.all withoutLogicalEffect
        | .throw_ _ arguments => arguments.all withoutLogicalEffect
        | .block statements result =>
            statements.all withoutLogicalEffect && result.all withoutLogicalEffect
        | .letDecl _ initializer body =>
            initializer.all withoutLogicalEffect && withoutLogicalEffect body
        | .ifElse condition thenBranch elseBranch =>
            withoutLogicalEffect condition && withoutLogicalEffect thenBranch &&
              elseBranch.all withoutLogicalEffect
        | .match_ scrutinee arms =>
            withoutLogicalEffect scrutinee && arms.all fun arm =>
              arm.guard.all withoutLogicalEffect && withoutLogicalEffect arm.body
        | .loop .. | .break_ .. | .continue_ _ | .return_ _ | .assign .. | .assignPattern .. =>
            false
  /-- A literal at its type: an integer, a proposition, a string, or a
  native value. -/
  translateLiteral (literal : ConstValue) (ty : IrTy) (typeId : TypeId) : MetaM Lean.Expr := do
    match literal, ty with
    | .integer value, .integer _ _ => return toExpr value
    | .bool true, .bool => return mkConst ``True
    | .bool false, .bool => return mkConst ``False
    | .string value, .string => return toExpr value
    | .address value, .address => return toExpr value
    | _, _ => nativeLiteral literal (← context.nativeType typeId)
  /-- A literal as a native value of a type. -/
  nativeLiteral (literal : ConstValue) (τ : VerexIR.Proofs.Denote.NTy) : MetaM Lean.Expr := do
    let carriers ← context.carriers
    match literal, τ with
    | .unit, .unit => return mkConst ``Unit.unit
    | .bool value, .bool => return toExpr value
    | .integer value, τ => nativeOfClause carriers τ (toExpr value)
    | .address value, .address | .address value, .signer | .string value, .string =>
        return toExpr value
    | .bytes value, .bytes => return toExpr value
    | .vector elements, .vector element =>
        let carrier := carrierType carriers (← quoteNTy element)
        let values ← elements.mapM (nativeLiteral · element)
        let array ← mkArrayLit carrier values.toList
        let bounded ← VerexIR.Proofs.Denote.decidedLemma (← mkAppM ``LT.lt
          #[← mkAppM ``Array.size #[array], mkNatLit (2 ^ 64)])
        return mkApp3 (mkConst ``VerexIR.SpecVector.mk) carrier array bounded
    | .tuple elements, .tuple row =>
        unless elements.size == row.length do
          throwError "a tuple literal differs in arity from its type"
        let values ← (elements.toList.zip row.toList).mapM fun (element, σ) => nativeLiteral element σ
        nativeValueRow carriers row values
    | literal, τ =>
        throwError "specification literal {repr literal} at the native type {repr τ} is not \
          supported in generated contracts"
  /-- The native type of an operand's value. -/
  operandNty? (id : ExprId) : Option VerexIR.Proofs.Denote.NTy := do
    let expression ← context.ns.expressions[id.index]?
    context.ntyOf? (specTypeId context.unit expression.typeId)
  /-- Whether equality reads an operand's value through its identity
  (`holdsContents`). -/
  holdsIdentity (id : ExprId) : Bool :=
    (operandNty? id).any (holdsContents (VerexIR.Proofs.Denote.declaresGhosts context.unit))
  /-- An equation of a native value with a value a specification builds,
  component by component, each in its component's domain
  (`componentEquation?`): with a constructor's operands, under its variant's
  test; with a field update's replacement at its field and its base's other
  fields. Equality ignores the ghost slots a built value leaves unspecified.
  None where the operand builds nothing. -/
  constructionEquation? (τ : VerexIR.Proofs.Denote.NTy) (value : Lean.Expr) (operand : ExprId) :
      MetaM (Option Lean.Expr) := do
    let some expression := context.ns.expressions[operand.index]? | return none
    let carriers ← context.carriers
    let τExpr ← quoteNTy τ
    -- A variant built of operands the carriers hold exactly, without a cut
    -- or a pending typing, and without a part equality reads through its
    -- identity, is a native value: its constructor states it, where its
    -- components are read through a variant test. A product's components
    -- are its projections, and their equations are the product's.
    if τ matches .enum .. && buildsVariant expression.kind &&
        !holdsContents (VerexIR.Proofs.Denote.declaresGhosts context.unit) τ then
      let built ← translate context operand
      let isCut (e : Lean.Expr) := e.isAppOfArity ``VerexIR.SpecInt.ofInt 4
      if !built.hasMVar && (built.find? isCut).isNone then
        if ← isDefEq (← inferType built) (← inferType value) then
          return some (← mkEq value built)
    match expression.kind, τ with
    | .operation (.call (.constructor _ none)) _ operands _, .struct _ _ fields =>
        let components ← nativeComponents carriers τExpr value
        let mut equations := #[]
        for (operand, index) in operands.zipIdx do
          let (some σ, some (_, component)) := (fields.toList[index]?, components[index]?)
            | return none
          let some equation ← componentEquation? σ component operand | return none
          equations := equations.push equation
        return some (← conjunction equations)
    | .operation (.call (.constructor reference (some name))) _ operands _,
        .enum _ _ names rows _ =>
        let handle ← structHandleOf context reference
        let some ⟨σs, _⟩ := VerexIR.Proofs.Denote.Which.ofName names rows name | return none
        let mut equations := #[← nativeVariantTest carriers τExpr value handle #[name]]
        for (operand, index) in operands.zipIdx do
          let some σ := σs.toList[index]? | return none
          let (_, read) ← nativeVariantField τ #[(name, index)] carriers τExpr value
          let some equation ← componentEquation? σ read operand | return none
          equations := equations.push equation
        return some (← conjunction equations)
    | .operation (.data (.updateField reference field)) _ operands _, .struct _ _ fields =>
        let #[base, replacement] := operands | return none
        let handle ← structHandleOf context reference
        let some declaration := context.unit.namespaces[handle.namespaceId.index]?.bind
            (·.structs[handle.structId]?) | return none
        let some index := VerexIR.SemanticOperations.handleFieldIndex? context.unit handle none field
          | return none
        let components ← nativeComponents carriers τExpr value
        let baseComponents ← nativeComponents carriers τExpr (← translate context base)
        let mut equations := #[]
        for position in [:declaration.fields.size] do
          let (some σ, some (_, component)) := (fields.toList[position]?, components[position]?)
            | return none
          if position == index then
            let some equation ← componentEquation? σ component replacement | return none
            equations := equations.push equation
          else
            let some (_, kept) := baseComponents[position]? | return none
            let some equation ← componentsEquation? σ component kept | return none
            equations := equations.push equation
        return some (← conjunction equations)
    -- A tuple the specification builds, component by component.
    | .operation (.primitive .tuple) _ operands _, .tuple row =>
        let components ← nativeComponents carriers τExpr value
        let mut equations := #[]
        for (operand, index) in operands.zipIdx do
          let (some σ, some (_, component)) := (row.toList[index]?, components[index]?)
            | return none
          let some equation ← componentEquation? σ component operand | return none
          equations := equations.push equation
        return some (← conjunction equations)
    | _, _ => return none
  /-- Whether an expression builds a variant. -/
  buildsVariant : VerexIR.ExprKind → Bool
    | .operation (.call (.constructor _ (some _))) _ _ _ => true
    | _ => false
  /-- The equation of a native component of a type with an operand, in the
  operand's domain: an integer's value, a Boolean's truth, a textual scalar,
  a native value, or built component by component. -/
  componentEquation? (σ : VerexIR.Proofs.Denote.NTy) (component : Lean.Expr) (operand : ExprId) :
      MetaM (Option Lean.Expr) := do
    let some operandTy := typeOfExpr? context operand | return none
    let domain := domainOf operandTy
    if domain == .native then
      if let some equation ← constructionEquation? σ component operand then return some equation
    let translated ← translate context operand
    match domain, σ with
    | .integer, .int _ _ | .integer, .num =>
        some <$> mkEq (← mkAppM ``VerexIR.SpecInt.val #[component]) translated
    | .boolean, .bool =>
        some <$> mkAppM ``Iff #[← mkEq component (mkConst ``Bool.true), translated]
    | .text, .string | .text, .address | .text, .signer => some <$> mkEq component translated
    | .native, _ =>
        let identity := holdsContents (VerexIR.Proofs.Denote.declaresGhosts context.unit) σ
        some <$> nativeEquation (← context.carriers) (← quoteNTy σ) component translated identity
    | _, _ => return none
  /-- The equation of two native components of one type, as equality reads
  them: integers by their values, a type holding a part equality reads
  through its identity by `NTy.equal`. -/
  componentsEquation? (σ : VerexIR.Proofs.Denote.NTy) (left right : Lean.Expr) :
      MetaM (Option Lean.Expr) := do
    match σ with
    | .int _ _ | .num =>
        some <$> mkEq (← mkAppM ``VerexIR.SpecInt.val #[left]) (← mkAppM ``VerexIR.SpecInt.val #[right])
    | _ =>
        let identity := holdsContents (VerexIR.Proofs.Denote.declaresGhosts context.unit) σ
        some <$> nativeEquation (← context.carriers) (← quoteNTy σ) left right identity
  /-- The resource type a global operation is instantiated at. -/
  resourceType (instantiations : Array VerexIR.GenericArgument) : MetaM TypeId := do
    match instantiations.toList with
    | [.typeArg resource] => return resource.typeId
    | _ => throwError "a global specification operation needs one resource type"
  /-- The state a storage read observes: `spec.old` reads the entry state,
  every other position the clause's own state. -/
  currentState : MetaM Lean.Expr := do
    let some state := context.state
      | throwError "a storage read has no state in this specification position"
    return state
  /-- The storage key an address operand keys memory by. -/
  addressKey (id : ExprId) : MetaM Lean.Expr := do
    return mkApp (mkConst ``VerexIR.StorageKey.address) (← translate context id)
  /-- The shape of a recursive specification function's definition at the
  type arguments of a call (`DefinitionShape`): a generic function is defined
  at the call's type arguments, as an expansion is. -/
  definitionShape (reference : VerexIR.QualifiedRef)
      (instantiations : Array VerexIR.GenericArgument) : MetaM DefinitionShape := do
    definitionShapeAt reference (← definitionTypeArguments instantiations)
  /-- The native types of a call's type arguments in the caller's frame. -/
  definitionTypeArguments (instantiations : Array VerexIR.GenericArgument) :
      MetaM (Array (Option VerexIR.Proofs.Denote.NTy)) :=
    instantiations.mapM fun
      | .typeArg argument => pure (context.ntyOf? argument.typeId)
      | _ => throwError "a specification function call takes only type arguments in \
          generated contracts"
  /-- The shape of a definition at resolved type arguments: a native
  parameter or result is a value of its type's carrier at the frame the
  definition takes. -/
  definitionShapeAt (reference : VerexIR.QualifiedRef)
      (typeArgumentTypes : Array (Option VerexIR.Proofs.Denote.NTy)) :
      MetaM DefinitionShape := do
    let some (targetNs, declaration) := specFunctionOf? context.unit reference
      | throwError "specification function `{repr reference}` does not resolve"
    let reads := specReads context.unit reference
    let probe := { context with
      namespaceId := reference.namespaceId, ns := targetNs, typeArgumentTypes }
    let domainAt (typeId : TypeId) (role : String) : MetaM Domain := do
      let some ty := probe.valueTypeOf? typeId | throwError "{role} has an unknown type"
      pure (domainOf ty)
    -- A native position: a value of a native type; a result also has an
    -- inhabitant, its value outside the definition's domain.
    let nativeAt (domain : Domain) (typeId : TypeId) (inhabited : Bool) :
        MetaM (Option VerexIR.Proofs.Denote.NTy) := do
      unless domain == .native do return none
      let some nty := probe.ntyOf? (specTypeId context.unit typeId)
        | throwError m!"a native position of the specification function \
            `{specFunctionSpelling context.unit reference}` has no native type"
      if inhabited then
        let inhabitable := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable)
          (← quoteNTy nty)
        unless ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) do
          throwError m!"the result type of the recursive specification function \
            `{specFunctionSpelling context.unit reference}` has no value it can stand for"
      return some nty
    let mut domains := #[]
    let mut natives := #[]
    for parameter in declaration.signature.parameters do
      let domain ← domainAt parameter.typeUse.typeId "a specification function parameter"
      domains := domains.push domain
      natives := natives.push (← nativeAt domain parameter.typeUse.typeId false)
    let localTypes ← declaration.locals.mapM fun localDecl => do
      let some localType := probe.typeOf? localDecl.type.typeId
        | throwError "a specification function local has an unknown type"
      pure localType
    let [result] := declaration.signature.results.toList
      | throwError "a recursive specification function returns one value"
    let resultDomain ← domainAt result.typeId "a specification function result"
    let resultNative ← nativeAt resultDomain result.typeId true
    let mut name ← specDefinitionName context.unit reference
    unless typeArgumentTypes.isEmpty do
      let spelled ← typeArgumentTypes.mapM fun
        | some nty => pure (ntySpelling nty)
        | none => throwError m!"a type argument of the recursive specification function \
            `{specFunctionSpelling context.unit reference}` has no native type"
      let .str base "spec" := name
        | throwError "internal: a definition's name does not end in `spec`"
      name := Name.str (Name.str base ("at_" ++ ",".intercalate spelled.toList)) "spec"
    let takesFrame := reads.state || natives.any (·.isSome) || resultNative.isSome
    return {
      reference := reference, name := name, reads := reads
      typeArgumentTypes := typeArgumentTypes, domains := domains, resultDomain := resultDomain
      localTypes := localTypes, natives := natives, resultNative := resultNative
      takesFrame := takesFrame, takesTypes := definitionTakesTypes reads takesFrame }
  /-- A member of a definition's group as its shape states it, its native
  positions at the carriers of the frame the definition takes. -/
  memberOfShape (shape : DefinitionShape) (reference : VerexIR.QualifiedRef)
      (carriers? : Option Lean.Expr) : MetaM SpecMember := do
    let some (targetNs, declaration) := specFunctionOf? context.unit reference
      | throwError "specification function `{repr reference}` does not resolve"
    let some root := declaration.body
      | throwError "a specification function's definition must have a body"
    let carrierOf (nty : VerexIR.Proofs.Denote.NTy) : MetaM Lean.Expr := do
      let some carriers := carriers? | throwError "internal: a native position without a frame"
      return carrierType carriers (← quoteNTy nty)
    let positionTypes ← (shape.domains.zip shape.natives).mapM
      fun (domain, nativeType) => match nativeType, domain.scalarType? with
        | some nty, _ => carrierOf nty
        | none, some type => pure type
        | none, none => throwError "internal: a native position without a native type"
    let bundleType ← positionTypes.foldrM
      (fun positionType rest => mkAppM ``Prod #[positionType, rest]) (mkConst ``Unit)
    let resultType ← match shape.resultNative with
      | some nty => carrierOf nty
      | none => definitionType shape.resultDomain
    return {
      reference := reference, name := shape.name, targetNs := targetNs
      declaration := declaration, root := root, domains := shape.domains
      resultDomain := shape.resultDomain, bundleType := bundleType
      resultType := resultType, localTypes := shape.localTypes
      natives := ← shape.natives.mapM (·.mapM quoteNTy)
      resultNative := ← shape.resultNative.mapM quoteNTy }
  /-- The Lean type of a scalar domain as a definition's result: a Boolean
  result is a proposition. -/
  definitionType (domain : Domain) : MetaM Lean.Expr :=
    match domain, domain.scalarType? with
    | .boolean, _ => pure (mkSort .zero)
    | _, some type => pure type
    | _, none => throwError "internal: a native result without a native type"
  /-- The arguments of a call as the bundle a recursive definition takes:
  right-nested pairs closed by `()`. -/
  bundleArguments (shape : DefinitionShape) (arguments : Array ExprId) :
      MetaM Lean.Expr := do
    unless arguments.size == shape.domains.size do
      throwError "specification function call argument count differs from its declaration"
    let mut values : Array Lean.Expr := #[]
    for argument in arguments, domain in shape.domains, nativeType in shape.natives do
      let value ← match nativeType with
        | some nty => nativeOfClause (← context.carriers) nty (← translate context argument)
        | none => bundleComponent domain (← translate context argument)
      values := values.push value
    bundleOf values
  /-- A specification function's value outside its domain, where an
  argument does not fit the fixed-width type of its parameter: an
  uninterpreted function (`opaqueSpec`) of the function, its type arguments
  as the frame resolves them, what the definition takes besides its frame and
  types, and the bundled arguments; the same value at every frame resolving
  them alike, as the transport across a call's frame reads it
  (`ensureDefinitionTransport`). -/
  outsideValue (member : SpecMember) (parameters : Array Lean.Expr) (frame? types? : Option Lean.Expr)
      (typeArgumentTypes : Array (Option VerexIR.Proofs.Denote.NTy)) (bundle : Lean.Expr) :
      MetaM Lean.Expr := do
    let default ← match member.resultNative, frame? with
      | some τ, some frame => typedInhabitant (← frameCarriers frame) τ
      | _, _ => mkAppOptM ``Inhabited.default #[member.resultType, none]
    let inhabited := mkApp2 (mkConst ``Inhabited.mk [1]) member.resultType default
    let typeArguments ← typeArgumentTypes.mapM fun nty? => do
      let some nty := nty?
        | throwError m!"a type argument of the specification function \
            `{specFunctionSpelling context.unit member.reference}` has no native type"
      let quoted ← quoteNTy nty
      let resolved ← match frame?, types? with
        | some frame, _ =>
            mkAppOptM ``VerexIR.Proofs.Denote.Skolems.resolve #[none, none, some frame, some quoted]
        | none, some types => pure (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.substWith) types quoted)
        | none, none => pure quoted
      pure (mkApp (mkConst ``VerexSL.Contract.SpecTypeArgument.native) resolved)
    let name := toString (← specDefinitionName context.unit member.reference) ++ ".outside"
    -- The unit's function values are the unit's: the value does not take them.
    let taken ← parameters.filterM fun parameter => do
      return some parameter != frame? && some parameter != types? &&
        !(← inferType parameter).isAppOf ``VerexIR.Proofs.Denote.FunctionCarriers
    opaqueValue name typeArguments.toList member.resultType inhabited (taken.push bundle)
  /-- The types a recursive definition's type parameters stand for at a
  call: the call's type arguments, under the caller's family. -/
  definitionTypes (instantiations : Array VerexIR.GenericArgument) : MetaM Lean.Expr := do
    let arguments ← instantiations.mapM fun instantiation => do
      let .typeArg argument := instantiation
        | throwError "a specification function call takes only type arguments in \
            generated contracts"
      let some nty := context.ntyOf? argument.typeId
        | throwError m!"a type argument of a recursive specification function has no \
            native type ({repr instantiation})"
      let quoted ← quoteNTy nty
      pure <| match context.types with
        | some types => mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.substWith) types quoted
        | none => quoted
    let list ← mkListLit (mkConst ``VerexIR.Proofs.Denote.NTy) arguments.toList
    withLocalDeclD `index (mkConst ``Nat) fun index => do
      mkLambdaFVars #[index] (← mkAppOptM ``List.getD
        #[none, list, index, mkConst ``VerexIR.Proofs.Denote.NTy.unit])
  /-- A specification role of an intrinsic map, as the typed map of the
  owner's table reads it. -/
  mapSpecCall (role : VerexIR.MapSpecRole) (owner : VerexIR.NameId) (ty : IrTy)
      (resultTypeId : TypeId) (arguments : Array ExprId) : MetaM Lean.Expr := do
    let operand (index : Nat) : MetaM ExprId := do
      let some id := arguments[index]?
        | throwError m!"the intrinsic map role `{role.name}` lacks operand {index}"
      pure id
    let some model := mapModel? context.unit (.nominal owner #[])
      | throwError m!"the intrinsic map role `{role.name}` belongs to a map whose representation \
          is not carried"
    -- A table's identity is its allocated handle, which no pure constructor
    -- has (`doc/designs/intrinsic-maps.md`, "Equality and order").
    if model.handle && role == .new_ then
      throwError "a pure Table constructor's logical identity is not carried"
    let carriers ← context.carriers
    -- The owner a role reads: its operand's native value.
    let ownerAt (index : Nat) : MetaM NativeOwner := do
      let id ← operand index
      nativeOwner carriers (← context.exprType id) (← translate context id)
    -- A key or value operand as the typed value the map holds it as. A key
    -- read off a typed map (`spec_key_at`) is the map's typed key already.
    let typedAt (view : VerexIR.Proofs.Denote.NativeMaps.TypedView) (index : Nat) :
        MetaM Lean.Expr := do
      let id ← operand index
      let clause ← translate context id
      match_expr clause.consumeMData with
      | VerexIR.Proofs.Denote.NTy.unread _ τ _ typed =>
          if ← isDefEq τ view.type then return typed
      | _ => pure ()
      -- A type parameter's value is native, whatever the representation its
      -- type has in the map.
      if (typeOfExpr? context id).map domainOf == some .native then
        return ← TypedTable.typedOfNative view clause
      TypedTable.typedOf view (← context.exprType id) clause
    let listAt (index : Nat) : MetaM Lean.Expr := do
      let id ← operand index
      let τ ← context.exprType id
      let (_, list) ← typedListOf carriers (← quoteNTy τ) (← translate context id)
      pure list
    match role with
    | .new_ => emptyOwner carriers (← context.nativeType resultTypeId)
    | .len => (← ownerAt 0).typed.size
    | .isEmpty | .abortsEmpty => mkEq (← (← ownerAt 0).typed.size) (mkIntLit 0)
    | .hasKey | .abortsAdd =>
        let table := (← ownerAt 0).typed
        mkEq (← table.hasKey (← typedAt table.keys 1)) (mkConst ``Bool.true)
    | .abortsDel | .abortsBorrow =>
        let table := (← ownerAt 0).typed
        mkEq (← table.hasKey (← typedAt table.keys 1)) (mkConst ``Bool.false)
    | .abortsDestroyEmpty => mkAppM ``Ne #[← (← ownerAt 0).typed.size, mkIntLit 0]
    | .get =>
        let table := (← ownerAt 0).typed
        TypedTable.clauseOf table.values (domainOf ty) (← table.valueAt (← typedAt table.keys 1))
          (← context.nativeType resultTypeId)
    | .keyAt | .insertionKeyAt =>
        let table := (← ownerAt 0).typed
        TypedTable.clauseOf table.keys (domainOf ty)
          (← table.keyAt (← translate context (← operand 1))) (← context.nativeType resultTypeId)
    | .rank | .insertionRank =>
        let table := (← ownerAt 0).typed
        table.rank (← typedAt table.keys 1)
    | .set =>
        let owned ← ownerAt 0
        let table := owned.typed
        owned.withMap carriers (← table.update (← table.discipline model) (← typedAt table.keys 1)
          (← typedAt table.values 2))
    | .del =>
        let owned ← ownerAt 0
        let table := owned.typed
        owned.withMap carriers (← table.remove (← table.discipline model) (← typedAt table.keys 1))
    | .abortsAddAll =>
        let table := (← ownerAt 0).typed
        mkAppOptM ``VerexIR.SpecMap.AbortsAddAll
          #[table.keys.carrier, table.values.carrier, table.keys.decEq, table.map, ← listAt 1,
            ← listAt 2]
    | .abortsNewFrom => mkAppM ``VerexIR.SpecMap.AbortsNewFrom #[← listAt 0, ← listAt 1]
    | .abortsUpsertAll => mkAppM ``VerexIR.SpecMap.AbortsUpsertAll #[← listAt 1, ← listAt 2]
    | .abortsAppendDisjoint =>
        let table := (← ownerAt 0).typed
        mkAppOptM ``VerexIR.SpecMap.AbortsAppendDisjoint
          #[table.keys.carrier, table.values.carrier, table.keys.decEq, table.map,
            (← ownerAt 1).typed.map]
    | .abortsTrim => mkAppM ``LT.lt #[← (← ownerAt 0).typed.size, ← translate context (← operand 1)]
    | .abortsReplaceKeyInplace =>
        let table := (← ownerAt 0).typed
        mkAppOptM ``VerexIR.SpecMap.AbortsReplaceKey
          #[table.keys.carrier, table.values.carrier, table.keys.decEq, ← table.discipline model,
            table.map, ← typedAt table.keys 1, ← typedAt table.keys 2]
    | .abortsIterBorrowMut =>
        let iteratorId ← operand 0
        let some iteratorType := typeOfExpr? context iteratorId
          | throwError "an iterator operand has an unknown type"
        let some iterator := iteratorModel? context.unit iteratorType
          | throwError "an iterator of an intrinsic map has no key or position variant"
        let enum ← context.exprType iteratorId
        mkAppM ``Not #[← iteratorInRange iterator enum carriers (← translate context iteratorId)
          (← ownerAt 1).typed]
    -- Iterator validity (`doc/designs/ghost-fields.md`, "Validity slots"): an
    -- iterator is valid at a map whose validity slot it carries; two map
    -- states preserve iterators where their slots agree.
    | .iterPreserved =>
        let some ownerType := arguments[0]?.bind (typeOfExpr? context)
          | throwError "a map operand has an unknown type"
        mkEq (← ownerValiditySlot context ownerType (← ownerAt 0))
          (← ownerValiditySlot context ownerType (← ownerAt 1))
    | .iterValid | .leafIterValid =>
        let iteratorId ← operand 0
        let some iteratorType := typeOfExpr? context iteratorId
          | throwError "an iterator operand has an unknown type"
        let some ownerType := arguments[1]?.bind (typeOfExpr? context)
          | throwError "a map operand has an unknown type"
        mkEq (← iteratorValiditySlot context iteratorType (← context.exprType iteratorId) carriers
            (← translate context iteratorId))
          (← ownerValiditySlot context ownerType (← ownerAt 1))
    | .abortsNewWithConfig | .leafOffset =>
        throwError m!"the intrinsic map role `{role.name}` is not carried"
  /-- The bounds the integer parameters of a bundle carry by their Move
  types, in the form `SpecInt.unsigned_bounds`/`signed_bounds` state them;
  `none` when no parameter is bounded. -/
  parameterBounds (bundle : Lean.Expr) (domains : Array Domain) (localTypes : Array IrTy) :
      MetaM (Option Lean.Expr) := do
    let mut values : Array Lean.Expr := #[]
    let mut rest := bundle
    for _ in [:domains.size] do
      values := values.push (← mkAppM ``Prod.fst #[rest])
      rest ← mkAppM ``Prod.snd #[rest]
    valueBounds values domains localTypes
  /-- Whether `value` fits the integer type of `width` and `signed` by its
  own form: a literal in range, the value of a certified integer of a type
  that fits, or the length of a vector at a width of at least 64 bits. -/
  fitsByType (value : Lean.Expr) (width : Nat) (signed : Bool) : MetaM Bool := do
    let (low, high) : Int × Int := if signed
      then (-(2 ^ (width - 1)), 2 ^ (width - 1) - 1) else (0, 2 ^ width - 1)
    if let some literal := value.int? then return low ≤ literal && literal ≤ high
    if value.isAppOfArity ``VerexIR.SpecInt.val 3 then
      let type ← whnfR (← inferType (value.getArg! 2))
      unless type.isAppOfArity ``VerexIR.SpecInt 2 do return false
      let bits ← whnfR (type.getArg! 0)
      unless bits.isAppOfArity ``VerexIR.IntWidth.bits 1 do return false
      let some width' := (bits.getArg! 0).rawNatLit? <|> (bits.getArg! 0).nat? | return false
      let signed' := (type.getArg! 1).isConstOf ``Bool.true
      return (signed' == signed && width' ≤ width) || (!signed' && signed && width' < width)
    if value.isAppOfArity ``Nat.cast 3 then
      let size := value.getArg! 2
      return !signed && 64 ≤ width && size.isAppOfArity ``Array.size 2 &&
        (size.getArg! 1).isAppOfArity ``VerexIR.SpecVector.values 2
    return false
  /-- The bounds `values` carry by the fixed-width integer types of the
  parameters they are bound to; `none` when no parameter is bounded. -/
  valueBounds (values : Array Lean.Expr) (domains : Array Domain) (localTypes : Array IrTy) :
      MetaM (Option Lean.Expr) := do
    let mut conjuncts : Array Lean.Expr := #[]
    for value in values, index in [:values.size] do
      match domains[index]?, localTypes[index]? with
      | some .integer, some (VerexIR.Ty.integer (.bits width) signed) =>
          if width == 0 then continue
          -- A value whose type already bounds it needs no guard: the
          -- conjunct would be a theorem.
          if ← fitsByType value width signed then continue
          let power (exponent : Nat) : MetaM Lean.Expr :=
            mkAppM ``HPow.hPow #[mkIntLit 2, mkNatLit exponent]
          let (low, high) ← if signed then
              pure (← mkAppM ``Neg.neg #[← power (width - 1)],
                ← mkAppM ``HSub.hSub #[← power (width - 1), mkIntLit 1])
            else pure (mkIntLit 0, ← mkAppM ``HSub.hSub #[← power width, mkIntLit 1])
          conjuncts := conjuncts.push (← mkAppM ``And
            #[← mkAppM ``LE.le #[low, value], ← mkAppM ``LE.le #[value, high]])
      | _, _ => pure ()
    let some first := conjuncts[0]? | return none
    let mut bounds := first
    for conjunct in conjuncts[1:] do
      bounds ← mkAppM ``And #[bounds, conjunct]
    return some bounds
  /-- Define a recursive specification function once, with the others of
  its group (`specGroup`): over its bundled arguments by well-founded
  recursion on its measure, with the unfolding theorem `f.unfold`. A
  definition first takes what its body reads besides its arguments
  (`specReads`), in that order; the members of a group read alike. -/
  ensureDefinition (reference : VerexIR.QualifiedRef)
      (instantiations : Array VerexIR.GenericArgument) : MetaM DefinitionShape := do
    ensureDefinitionAt reference (← definitionTypeArguments instantiations)
  /-- Define a recursive specification function once at native type
  arguments, with the others of its group. -/
  ensureDefinitionAt (reference : VerexIR.QualifiedRef)
      (typeArgumentTypes : Array (Option VerexIR.Proofs.Denote.NTy)) : MetaM DefinitionShape := do
    let shape ← definitionShapeAt reference typeArgumentTypes
    if (← getEnv).contains shape.name then return shape
    buildDefinition shape
  /-- Build the definitions of a group at a shape. -/
  buildDefinition (shape : DefinitionShape) : MetaM DefinitionShape := do
    let reference := shape.reference
    let name := shape.name
    let group := specGroup context.unit reference
    let reads := shape.reads
    let typesType ← mkArrow (mkConst ``Nat) (mkConst ``VerexIR.Proofs.Denote.NTy)
    let optionalLocal {α : Type} (present : Bool) (binder : Name) (type : Lean.Expr)
        (k : Option Lean.Expr → MetaM α) : MetaM α :=
      if present then withLocalDeclD binder type fun x => k (some x) else k none
    -- A definition reading the executable unit, the table of declared
    -- preconditions, or storage, or taking a frame, takes the unit they are
    -- at first.
    withOptionalUnit (reads.unit || reads.requires || shape.takesFrame) fun unit? functions? =>
    let atUnit (constant : Name) :=
      mkAppN (mkConst constant) (#[unit?, functions?].filterMap fun parameter => parameter)
    optionalLocal reads.unit `executable
      (mkApp (mkConst ``VerexIR.Validation.ExecutableUnit) (unit?.getD (mkConst ``Unit)))
      fun executable =>
    optionalLocal reads.requires `requiresMeaning (atUnit ``VerexIR.Proofs.RequiresMeaning)
      fun requiresMeaning =>
    -- A definition reading storage, or taking a native parameter or result,
    -- takes the family its reads resolve resource types at and its natives
    -- inhabit, before the memory.
    optionalLocal shape.takesFrame `frame (atUnit ``VerexIR.Proofs.Denote.Skolems) fun frame =>
    optionalLocal reads.state `state (atUnit ``VerexIR.Proofs.Denote.Memory) fun state =>
    -- One taking no frame, but an uninterpreted value at its type
    -- parameters, takes the types they stand for (`definitionTypes`).
    optionalLocal shape.takesTypes `types typesType fun instantiation => do
    let parameters :=
      #[unit?, functions?, executable, requiresMeaning, frame, state, instantiation].filterMap
        fun parameter => parameter
    let types ← match instantiation with
      | some instantiation => pure (some instantiation)
      | none => frame.mapM frameTypes
    let carriers? ← frame.mapM frameCarriers
    let some (_, entry) := specFunctionOf? context.unit reference
      | throwError "specification function `{repr reference}` does not resolve"
    let members ← group.mapM fun member => do
      -- The members of a group are shaped alike, at the entry's type arguments.
      let memberShape ← if member == reference then pure shape else do
        let some (_, declaration) := specFunctionOf? context.unit member
          | throwError "specification function `{repr member}` does not resolve"
        unless declaration.signature.generics.size == entry.signature.generics.size do
          throwError m!"the mutually recursive specification functions \
            `{specFunctionSpelling context.unit reference}` and \
            `{specFunctionSpelling context.unit member}` differ in their type parameters, \
            which generated contracts do not carry"
        definitionShapeAt member shape.typeArgumentTypes
      memberOfShape memberShape member carriers?
    -- The parameters are the projections of the bundle.
    let bodyContext (member : SpecMember) (bundle : Lean.Expr)
        (definition : Option RecursiveDefinition) : MetaM Context := do
      let mut locals : Array (Option Lean.Expr) :=
        Array.replicate member.declaration.locals.size none
      let mut rest := bundle
      for index in [:member.domains.size] do
        let projection ← mkAppM ``Prod.fst #[rest]
        locals := locals.set! index (some projection)
        rest ← mkAppM ``Prod.snd #[rest]
      pure { context with
        namespaceId := member.reference.namespaceId, ns := member.targetNs
        locals, localTypes := member.localTypes
        localNames := member.declaration.locals.map (·.name), oldLocals := locals
        results := #[], resultTypes := #[]
        specCallStack := #[]
        typeArgumentTypes := shape.typeArgumentTypes
        definition, facts := #[]
        executable, requiresMeaning, state, oldState := none, types
        anchorLocals := #[], anchorState := none }
    -- A member's measures: its `decreases` clause, or, without one, each
    -- integer parameter in turn. A Move specification function states no
    -- measure; the Move Prover takes its recursion as given.
    let candidateMeasures ← members.mapM fun member => do
      let candidates : Array (Lean.Expr → MetaM Lean.Expr) ←
        match member.declaration.contract.conditions.find? (·.kind == .decreases) with
        | some measureSource => pure #[fun bundle => do
            translate (← bodyContext member bundle none) measureSource.expression]
        | none => pure <| (member.domains.zipIdx).filterMap fun (domain, index) =>
            if domain matches .integer then some (fun bundle => do
              let mut rest := bundle
              for _ in [:index] do rest ← mkAppM ``Prod.snd #[rest]
              mkAppM ``Prod.fst #[rest])
            else none
      if candidates.isEmpty then
        throwError m!"the recursive specification function \
          `{specFunctionSpelling context.unit member.reference}` needs a `decreases` measure"
      candidates.mapM fun candidate =>
        withLocalDeclD `bundle member.bundleType fun bundle => do
          mkLambdaFVars #[bundle] (← mkAppM ``Int.toNat #[← candidate bundle])
    -- The step function of a member at the members' measures: over its
    -- bundle and a recursion hypothesis per member, over the bundles below
    -- its own. Guarded, a recursive call the path conditions do not prove
    -- descending is guarded by its descent. Assuming non-negativity, the
    -- integer parameters are taken as non-negative — the Move types the
    -- specification's `num` projection widened — only to select a
    -- measure; the step built so is discarded.
    let buildStep (index : Nat) (measures : Array Lean.Expr) (guarded assumeNonNegative : Bool) :
        MetaM Lean.Expr := do
      let member := members[index]!
      withLocalDeclD `bundle member.bundleType fun bundle => do
        let recurseTypes ← members.zipIdx.mapM fun (other, otherIndex) =>
          withLocalDeclD `smaller other.bundleType fun smaller => do
            let decreasing ← mkAppM ``LT.lt
              #[mkApp measures[otherIndex]! smaller, mkApp measures[index]! bundle]
            pure (`recurse, ← mkForallFVars #[smaller] (← mkArrow decreasing other.resultType))
        withLocalDeclsDND recurseTypes fun recurses => do
          let recursiveMembers := members.zipIdx.map fun (other, otherIndex) =>
            ({
              reference := other.reference, recurse := recurses[otherIndex]!
              measure := measures[otherIndex]!, resultType := other.resultType
              resultDomain := other.resultDomain, resultNative := other.resultNative } :
              RecursiveMember)
          let definition : RecursiveDefinition := {
            reference := member.reference, argument := bundle, measure := measures[index]!
            members := recursiveMembers, guarded := guarded }
          let translateBody : MetaM Lean.Expr := do
            translate (← bodyContext member bundle (some definition)) member.root
          let translateBody : MetaM Lean.Expr := if !assumeNonNegative then translateBody else do
            let mut nonNegative : Array Lean.Expr := #[]
            let mut rest := bundle
            for domain in member.domains do
              let value ← mkAppM ``Prod.fst #[rest]
              rest ← mkAppM ``Prod.snd #[rest]
              if domain matches .integer then
                nonNegative := nonNegative.push (← mkAppM ``LE.le #[mkIntLit 0, value])
            let rec assume (index : Nat) : MetaM Lean.Expr := do
              if h : index < nonNegative.size then
                withLocalDeclD `nonNegative nonNegative[index] fun _ => assume (index + 1)
              else translateBody
            assume 0
          -- The definition unfolds at well-typed arguments only: the
          -- parameters' fixed-width types are the function's domain, which
          -- the recursion's descent may rely on, and its value outside them
          -- is unspecified (`outsideValue`).
          let body ← match ← parameterBounds bundle member.domains member.localTypes with
            | none => translateBody
            | some bounds =>
                let decision ← synthInstance (← mkAppM ``Decidable #[bounds])
                let inside ← withLocalDeclD `bounds bounds fun boundsHypothesis => do
                  mkLambdaFVars #[boundsHypothesis] (← translateBody)
                let outside ← withLocalDeclD `outside (mkNot bounds) fun outsideHypothesis => do
                  mkLambdaFVars #[outsideHypothesis]
                    (← outsideValue member parameters frame instantiation
                      shape.typeArgumentTypes bundle)
                pure (mkApp5 (mkConst ``dite [1]) member.resultType bounds decision inside outside)
          mkLambdaFVars (#[bundle] ++ recurses) body
    -- Measures every recursive call provably descends on, a choice per
    -- member; failing that, ones they descend on for non-negative
    -- parameters, with the calls guarded.
    let combinations := candidateMeasures.foldl (init := #[#[]]) fun combinations options =>
      combinations.flatMap fun combination => options.map combination.push
    let buildSteps (measures : Array Lean.Expr) (guarded assumeNonNegative : Bool) :=
      (Array.range members.size).mapM fun index =>
        buildStep index measures guarded assumeNonNegative
    -- A body that does not translate fails at every measure: that failure
    -- is the reason.
    let mut attempts : Array MessageData := #[]
    let mut untranslatable : Option Exception := none
    let mut found : Option (Array Lean.Expr × Array Lean.Expr) := none
    for measures in combinations do
      if found.isSome || untranslatable.isSome then break
      try found := some (measures, ← buildSteps measures false false)
      catch failure =>
        if failure.toMessageData.hasTag (· == `verex.measure) then
          attempts := attempts.push failure.toMessageData
        else untranslatable := some failure
    for measures in combinations do
      if found.isSome || untranslatable.isSome then break
      try
        discard <| buildSteps measures false true
        found := some (measures, ← buildSteps measures true false)
      catch failure =>
        unless failure.toMessageData.hasTag (· == `verex.measure) do
          untranslatable := some failure
    if let some failure := untranslatable then throw failure
    let some (measures, steps) := found
      | let spelled := ", ".intercalate (members.map fun member =>
            s!"`{specFunctionSpelling context.unit member.reference}`").toList
        throwError m!"{if members.size == 1 then
            s!"no measure of the recursive specification function {spelled} decreases"
          else s!"no measures of the mutually recursive specification functions {spelled} \
            decrease"} at every recursive call: {MessageData.joinSep attempts.toList "\n"}"
    let bundleTypes := members.map (·.bundleType)
    -- A member's fixpoint, and its applications as a recursive call takes them.
    let (fixpoint, motive, relation, wellFounded, step, injections) ←
      if members.size == 1 then do
        let relation ← mkAppM ``measure #[measures[0]!]
        let motive ← withLocalDeclD `bundle bundleTypes[0]! fun bundle =>
          mkLambdaFVars #[bundle] members[0]!.resultType
        pure (none, motive, relation, ← mkAppOptM ``WellFoundedRelation.wf #[bundleTypes[0]!, relation],
          steps[0]!, #[fun (bundle : Lean.Expr) => pure bundle])
      else do
        -- The sum of the bundles: each member's summand, its measure there,
        -- and the result type there.
        let sum ← sumType bundleTypes 0
        let resultMotive ← withLocalDeclD `x sum fun x => do
          mkLambdaFVars #[x] (← sumCases bundleTypes 0 (← mkConstMotive sum (mkSort (.succ .zero)))
            x fun index _ => pure members[index]!.resultType)
        let measureSum ← withLocalDeclD `x sum fun x => do
          mkLambdaFVars #[x] (← sumCases bundleTypes 0 (← mkConstMotive sum (mkConst ``Nat)) x
            fun index bundle => pure (mkApp measures[index]! bundle))
        let relation ← mkAppM ``measure #[measureSum]
        let wellFounded ← mkAppOptM ``WellFoundedRelation.wf #[sum, relation]
        let relationOf ← mkAppOptM ``WellFoundedRelation.rel #[sum, relation]
        -- The step over the sum: each summand's member's step, its
        -- recursion hypotheses the sum's at the members' summands.
        let hypothesisType (x : Lean.Expr) : MetaM Lean.Expr :=
          withLocalDeclD `y sum fun y => do
            mkForallFVars #[y] (← mkArrow (mkApp2 relationOf y x) (mkApp resultMotive y))
        let stepMotive ← withLocalDeclD `x sum fun x => do
          mkLambdaFVars #[x] (← mkArrow (← hypothesisType x) (mkApp resultMotive x))
        let step ← withLocalDeclD `x sum fun x => do
          mkLambdaFVars #[x] (← sumCases bundleTypes 0 stepMotive x fun index bundle => do
            let injected ← sumInject bundleTypes 0 index bundle
            withLocalDeclD `recurse (← hypothesisType injected) fun recurse => do
              let recursions ← members.zipIdx.mapM fun (other, otherIndex) =>
                withLocalDeclD `smaller other.bundleType fun smaller => do
                  let decreasing ← mkAppM ``LT.lt
                    #[mkApp measures[otherIndex]! smaller, mkApp measures[index]! bundle]
                  withLocalDeclD `less decreasing fun less => do
                    mkLambdaFVars #[smaller, less]
                      (mkApp2 recurse (← sumInject bundleTypes 0 otherIndex smaller) less)
              mkLambdaFVars #[recurse] (mkAppN steps[index]! (#[bundle] ++ recursions)))
        let fixpoint ← mkAppOptM ``WellFounded.fix #[sum, resultMotive, relationOf, wellFounded, step]
        let groupName := Name.str members[0]!.name "group"
        let groupType ← mkForallFVars parameters (← withLocalDeclD `x sum fun x => do
          mkForallFVars #[x] (mkApp resultMotive x))
        let groupValue ← mkLambdaFVars parameters fixpoint
        addDecl (.defnDecl {
          name := groupName, levelParams := [], type := groupType
          value := groupValue, hints := .opaque, safety := .safe })
        setIrreducibleAttribute groupName
        pure (some (mkAppN (mkConst groupName) parameters), resultMotive, relation, wellFounded, step,
          (Array.range members.size).map fun index (bundle : Lean.Expr) =>
            sumInject bundleTypes 0 index bundle)
    let applied (index : Nat) := mkAppN (mkConst members[index]!.name) parameters
    for member in members, index in [0:members.size] do
      let type ← mkForallFVars parameters (← mkArrow member.bundleType member.resultType)
      let value ← match fixpoint with
        | none => mkLambdaFVars parameters (← mkAppOptM ``WellFounded.fix
            #[member.bundleType, motive, ← mkAppOptM ``WellFoundedRelation.rel
              #[member.bundleType, relation], wellFounded, step])
        | some group => withLocalDeclD `bundle member.bundleType fun bundle => do
            mkLambdaFVars (parameters.push bundle) (mkApp group (← injections[index]! bundle))
      addDecl (.defnDecl {
        name := member.name, levelParams := [], type := type, value := value
        hints := .opaque, safety := .safe })
      setIrreducibleAttribute member.name
      let some path := context.unit.tables.namespaces[member.reference.namespaceId.index]?
        | throwError "a specification function's namespace has no path"
      let some qualified := context.unit.tables.names[member.reference.name.index]?
        | throwError "a specification function has no name"
      modifyEnv (definitionKeys.addEntry · (member.name,
        { path := path.segments, function := qualified.name
          typeArgumentTypes := shape.typeArgumentTypes }))
    -- `f a = F a (fun b _ => g b) …`: a member unfolds once, its recursive
    -- calls to the members' definitions.
    for member in members, index in [0:members.size] do
      let unfoldType ← withLocalDeclD `bundle member.bundleType fun bundle => do
        let recursions ← members.zipIdx.mapM fun (other, otherIndex) =>
          withLocalDeclD `smaller other.bundleType fun smaller => do
            let decreasing ← mkAppM ``LT.lt
              #[mkApp measures[otherIndex]! smaller, mkApp measures[index]! bundle]
            withLocalDeclD `less decreasing fun less =>
              mkLambdaFVars #[smaller, less] (mkApp (applied otherIndex) smaller)
        mkForallFVars (parameters.push bundle) (← mkEq (mkApp (applied index) bundle)
          (← Core.betaReduce (mkAppN steps[index]! (#[bundle] ++ recursions))))
      let unfoldValue ← withLocalDeclD `bundle member.bundleType fun bundle => do
        let (domain, argument) ← match fixpoint with
          | none => pure (member.bundleType, bundle)
          | some _ => pure (← sumType bundleTypes 0, ← injections[index]! bundle)
        let equation ← mkAppOptM ``WellFounded.fix_eq
          #[domain, motive, ← mkAppOptM ``WellFoundedRelation.rel #[domain, relation], wellFounded,
            step, argument]
        mkLambdaFVars (parameters.push bundle) equation
      addDecl (.thmDecl {
        name := Name.str member.name "unfold", levelParams := []
        type := unfoldType, value := unfoldValue })
    return shape
  /-- What a lemma's statement reads besides its parameters. -/
  lemmaReads (reference : VerexIR.QualifiedRef) : SpecReads :=
    match lemmaOf? context.unit reference with
    | some (_, declaration) => readsFrom context.unit <|
        (declaration.contract.conditions.toList ++ declaration.proof.toList).map
          fun condition => (reference.namespaceId, condition.expression)
    | none => {}
  /-- The parameter domains of a lemma, as its definitions take them. -/
  lemmaDomains (reference : VerexIR.QualifiedRef) : MetaM (Array Domain) := do
    let some (_, declaration) := lemmaOf? context.unit reference
      | throwError "lemma `{repr reference}` does not resolve"
    declaration.signature.parameters.mapM fun parameter =>
      signatureDomain context.unit parameter.typeUse.typeId "a lemma parameter"
  /-- The arguments of a lemma instance as the bundle its definitions take:
  a native parameter's native value, any other's clause value. -/
  lemmaBundle (reference : VerexIR.QualifiedRef) (arguments : Array ExprId)
      (typeArguments? : Option VerexIR.Proofs.Denote.NRow := none) : MetaM Lean.Expr := do
    let domains ← lemmaDomains reference
    unless arguments.size == domains.size do
      throwError "a lemma instance's argument count differs from the lemma's"
    let natives := lemmaNativeParameters context.unit reference
    let mut values : Array Lean.Expr := #[]
    for (argument, domain, index) in (arguments.zip domains).zipIdx.map (fun ((a, d), i) => (a, d, i)) do
      let value ← match natives.find? (·.1 == index) with
        | some (_, nty) =>
            -- A native parameter at the type the lemma's instance gives it.
            let nty := match typeArguments? with
              | some row => nty.subst row
              | none => nty
            nativeOfClause (← context.carriers) nty (← translate context argument)
        | none => bundleComponent domain (← translate context argument)
      values := values.push value
    bundleOf values
  /-- A lemma's definition applied to what the position reads. -/
  lemmaApplied (reference : VerexIR.QualifiedRef) (name : Name)
      (frame? : Option Lean.Expr := none) : MetaM Lean.Expr := do
    let reads := lemmaReads reference
    let mut applied := mkConst name
    if reads.unit || reads.requires || lemmaTakesFrame context.unit reference reads.state ||
        reads.state then
      applied := mkAppN applied (← context.leading)
    if reads.unit then
      let some executable := context.executable
        | throwError "a lemma reads the executable unit, which this position does not take"
      applied := mkApp applied executable
    if reads.requires then
      let some table := context.requiresMeaning
        | throwError "a lemma reads the requires meaning, which this \
            position does not take"
      applied := mkApp applied table
    if lemmaTakesFrame context.unit reference reads.state then
      applied := mkApp applied (← match frame? with
        | some frame => pure frame
        | none => context.frame)
    if reads.state then applied := mkApp applied (← currentState)
    return applied
  /-- Define a lemma's statement once, each part over the bundle of its
  parameters, after what it reads besides them: the premise (the bounds the
  integer parameters carry by their types, then the `requires`), the
  conclusion, each step's proposition, and each measure component. -/
  ensureLemmaDefinitions (reference : VerexIR.QualifiedRef) : MetaM LemmaNames := do
    let names ← lemmaNames context.unit reference
    if (← getEnv).contains names.requires then return names
    let some (targetNs, declaration) := lemmaOf? context.unit reference
      | throwError "lemma `{repr reference}` does not resolve"
    let reads := lemmaReads reference
    let optionalLocal {α : Type} (present : Bool) (binder : Name) (type : Lean.Expr)
        (k : Option Lean.Expr → MetaM α) : MetaM α :=
      if present then withLocalDeclD binder type fun x => k (some x) else k none
    let takesFrame := lemmaTakesFrame context.unit reference reads.state
    -- What it reads besides its parameters is at a unit, taken first.
    withOptionalUnit (reads.unit || reads.requires || takesFrame || reads.state)
      fun unit? functions? =>
    let atUnit (constant : Name) :=
      mkAppN (mkConst constant) (#[unit?, functions?].filterMap fun parameter => parameter)
    optionalLocal reads.unit `executable
      (mkApp (mkConst ``VerexIR.Validation.ExecutableUnit) (unit?.getD (mkConst ``Unit)))
      fun executable =>
    optionalLocal reads.requires `requiresMeaning (atUnit ``VerexIR.Proofs.RequiresMeaning)
      fun requiresMeaning =>
    -- The family its reads resolve resource types at and its parameters
    -- are typed at, before the memory.
    optionalLocal takesFrame `frame (atUnit ``VerexIR.Proofs.Denote.Skolems) fun frame =>
    optionalLocal reads.state `state (atUnit ``VerexIR.Proofs.Denote.Memory) fun state => do
    let parameters := #[unit?, functions?, executable, requiresMeaning, frame, state].filterMap
      fun parameter => parameter
    let types ← frame.mapM frameTypes
    let domains ← lemmaDomains reference
    let natives := lemmaNativeParameters context.unit reference
    let positionTypes ← domains.zipIdx.mapM fun (domain, index) => do
      match natives.find? (·.1 == index), domain.scalarType? with
      | some (_, nty), _ =>
          let some frame := frame | throwError "a lemma typing its parameters takes no family"
          return carrierType (← frameCarriers frame) (← quoteNTy nty)
      | none, some type => pure type
      | none, none => throwError "a lemma parameter has no native type"
    let bundleType ← positionTypes.foldrM (fun type rest => mkAppM ``Prod #[type, rest])
      (mkConst ``Unit)
    let mut localTypes : Array IrTy := #[]
    for localDecl in declaration.locals do
      let some localType := context.unit.tables.types[localDecl.type.typeId.index]?
        | throwError "a lemma local has an unknown type"
      localTypes := localTypes.push localType
    let bodyContext (bundle : Lean.Expr) : MetaM Context := do
      let mut locals : Array (Option Lean.Expr) := Array.replicate declaration.locals.size none
      let mut rest := bundle
      for index in [:domains.size] do
        locals := locals.set! index (some (← mkAppM ``Prod.fst #[rest]))
        rest ← mkAppM ``Prod.snd #[rest]
      pure { context with
        namespaceId := reference.namespaceId, ns := targetNs
        locals, localTypes, localNames := declaration.locals.map (·.name), oldLocals := locals
        results := #[], resultTypes := #[]
        specCallStack := #[], typeArgumentTypes := #[]
        definition := none, facts := #[]
        executable, requiresMeaning, state, oldState := state, types
        anchorLocals := #[], anchorState := none, lemmaQuantified := false }
    let conjunction (clauses : Array Lean.Expr) : MetaM Lean.Expr :=
      match clauses.back? with
      | none => pure (mkConst ``True)
      | some last => clauses.pop.foldrM (fun clause rest => mkAppM ``And #[clause, rest]) last
    let define (name : Name) (resultType : Lean.Expr)
        (body : Lean.Expr → MetaM Lean.Expr) : MetaM Unit := do
      let value ← withLocalDeclD `bundle bundleType fun bundle => do
        mkLambdaFVars (parameters.push bundle) (← body bundle)
      let type ← mkForallFVars parameters (← mkArrow bundleType resultType)
      addDecl (.defnDecl
        { name, levelParams := [], type, value, hints := .abbrev, safety := .safe })
      enableRealizationsForConst name
    -- Each clause is marked with its range, which a failure is reported at.
    let clauses (kind : VerexIR.ConditionKind) (bundle : Lean.Expr) : MetaM (Array Lean.Expr) := do
      let active ← bodyContext bundle
      (declaration.contract.conditions.filter (·.kind == kind)).mapM fun condition =>
        markObligation (conditionRange context.unit condition) <$>
          translate active condition.expression
    define names.requires (mkSort .zero) fun bundle => do
      let bounds ← parameterBounds bundle domains localTypes
      conjunction (bounds.toArray ++ (← clauses .requires bundle))
    define names.ensures (mkSort .zero) fun bundle => do
      conjunction (← clauses .ensures bundle)
    for (condition, name) in declaration.proof.zip names.steps do
      define name (mkSort .zero) fun bundle => markObligation (conditionRange context.unit condition) <$> do
        let active ← bodyContext bundle
        match condition.kind with
        | .split =>
            -- A split on a Boolean directs the cases; one on an enum is no step.
            let some expression := targetNs.expressions[condition.expression.index]?
              | throwError "a split's expression is out of range"
            if context.unit.tables.types[expression.typeId.index]? == some .bool then
              mkAppM ``VerexIR.Proofs.CaseSplit #[← translate active condition.expression]
            else pure (mkConst ``True)
        | _ => translate active condition.expression
    let measures := declaration.contract.conditions.filter (·.kind == .decreases)
    for (condition, name) in measures.zip names.measures do
      define name (mkConst ``Int) fun bundle => do
        translate (← bodyContext bundle) condition.expression
    -- What the proof assumes at a step: of every bundle meeting the premise
    -- and the assumptions before it, closed over what it reads.
    for (index, name) in names.trusted do
      let type ← withLocalDeclD `bundle bundleType fun bundle => do
        let applied (definition : Name) := mkApp (mkAppN (mkConst definition) parameters) bundle
        let earlier := names.trusted.filter (·.1 < index) |>.map fun (earlier, _) =>
          applied names.steps[earlier]!
        let premises := #[applied names.requires] ++ earlier
        let body ← premises.foldrM (fun premise rest => mkArrow premise rest)
          (applied names.steps[index]!)
        mkForallFVars (parameters.push bundle) body
      addDecl (.defnDecl
        { name, levelParams := [], type := mkSort .zero, value := type, hints := .abbrev,
          safety := .safe })
      enableRealizationsForConst name
    return names
  translateOperation (operation : Operation)
      (instantiations : Array VerexIR.GenericArgument) (arguments : Array ExprId)
      (ty : IrTy) (resultTypeId : TypeId) : MetaM Lean.Expr := do
    match operation with
    | .specification (.lemma reference _) =>
        let names ← ensureLemmaDefinitions reference
        -- A generic lemma's instance is at the frame its type arguments
        -- induce over the position's; at the position's own type parameters
        -- in order, that frame is the position's.
        let row? ← if instantiations.isEmpty then pure none else do
          let natives ← definitionTypeArguments instantiations
          let some row := natives.toList.mapM (fun type? => type?)
            | throwError "a lemma instance's type argument has no native type"
          let identity := row.zipIdx.all fun (type, index) => match type with
            | .param position => position == index
            | _ => false
          if identity then pure none
          else pure (some (row.foldr VerexIR.Proofs.Denote.NRow.cons .nil))
        let frame? ← row?.mapM fun row => do
          pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.instantiate)
            ((← context.leading) ++ #[← quoteTypeArgs.quoteTypeArgsRow row, ← context.frame]))
        let bundle ← lemmaBundle reference arguments row?
        let owes := mkApp (← lemmaApplied reference names.requires frame?) bundle
        let gives := mkApp (← lemmaApplied reference names.ensures frame?) bundle
        mkAppM (if context.lemmaQuantified then ``VerexIR.Proofs.LemmaInstance
          else ``VerexIR.Proofs.LemmaApplication) #[owes, gives]
    | .specification (.functionCall reference _) =>
        if let some definition := context.definition then
          if let some member := definition.members.find? (·.reference == reference) then
            -- A recursive call: the bundled arguments, at a smaller measure.
            let shape ← definitionShape reference instantiations
            let bundle ← bundleArguments shape arguments
            let goal ← mkAppM ``LT.lt
              #[mkApp member.measure bundle, mkApp definition.measure definition.argument]
            -- The measure at the bundled arguments, projected out: `omega`
            -- reads the arithmetic, not the pairing.
            let (goal, _) ← dsimp goal (← Simp.mkContext)
            let proof ← mkFreshExprMVar goal
            let decreases ← try
                -- As the tactic does: refute the negated goal from the local
                -- hypotheses, which include the path conditions.
                if let some refutation ← proof.mvarId!.falseOrByContra then
                  let hypotheses ← refutation.withContext getLocalHyps
                  Lean.Elab.Tactic.Omega.omega hypotheses.toList refutation
                pure true
              catch failure =>
                if definition.guarded then pure false else
                  let facts ← context.facts.mapM fun fact => do pure m!"{← inferType fact}"
                  throwError (MessageData.tagged `verex.measure m!"the recursive call of \
                    `{specFunctionSpelling context.unit reference}` does not decrease its \
                    measure under the conditions on its path: \
                    {failure.toMessageData}\ngoal: {goal}\nconditions: {facts.toList}")
            if decreases then
              return mkApp2 member.recurse bundle (← instantiateMVars proof)
            -- The Move Prover's axiom for the function holds unguarded; a
            -- definition holds it wherever the measure descends and takes
            -- an arbitrary value elsewhere.
            let decision ← synthInstance (← mkAppM ``Decidable #[goal])
            let inside ← withLocalDeclD `descends goal fun descends =>
              mkLambdaFVars #[descends] (mkApp2 member.recurse bundle descends)
            let outside ← withLocalDeclD `stuck (mkNot goal) fun stuck => do
              mkLambdaFVars #[stuck] (← mkAppOptM ``Inhabited.default #[member.resultType, none])
            return mkApp5 (mkConst ``dite [1]) member.resultType goal decision inside outside
        if specRecursive context.unit reference then
          -- A recursive function is a Lean definition, not an expansion,
          -- taking what its body reads from the caller's position.
          let shape ← ensureDefinition reference instantiations
          let bundle ← bundleArguments shape arguments
          let reads := shape.reads
          let mut applied := mkConst shape.name
          if reads.unit || reads.requires || shape.takesFrame then
            applied := mkAppN applied (← context.leading)
          if reads.unit then
            let some executable := context.executable
              | throwError "a behavioral predicate reads the executable unit, which this \
                  contract does not take"
            applied := mkApp applied executable
          if reads.requires then
            let some table := context.requiresMeaning
              | throwError "`requires_of` reads the requires meaning, which \
                  this contract does not take"
            applied := mkApp applied table
          if shape.takesFrame then applied := mkApp applied (← context.frame)
          if reads.state then applied := mkApp applied (← currentState)
          if shape.takesTypes then
            applied := mkApp applied (← definitionTypes instantiations)
          -- A native result read at the call's domain, a type parameter's
          -- value at an instance whose type a clause reads otherwise.
          match shape.resultNative with
          | some τ =>
              if domainOf ty == .native then return mkApp applied bundle
              return ← clauseOfNative τ (mkApp applied bundle)
          | none => return mkApp applied bundle
        if context.specCallStack.contains reference then
          throwError m!"the recursive specification function \
            `{specFunctionSpelling context.unit reference}` needs a `decreases` measure"
        -- A generic function is expanded at its arguments' native types.
        let typeArgumentTypes ← instantiations.mapM fun
          | .typeArg argument => pure (context.ntyOf? argument.typeId)
          | _ => throwError "a specification function call takes only type arguments in \
              generated contracts"
        let some qualified := context.unit.tables.names[reference.name.index]?
          | throwError "specification function call has an unknown name"
        unless qualified.namespaceId == reference.namespaceId do
          throwError "specification function call name belongs to a different namespace"
        let some targetNs := context.unit.namespaces[reference.namespaceId.index]?
          | throwError "specification function call namespace is out of range"
        if let some (role, owner) := mapSpecRole? targetNs reference then
          return ← mapSpecCall role owner ty resultTypeId arguments
        -- An uninterpreted value of the call: an opaque specification
        -- function's, or a native's, which has no body to derive a
        -- specification version from, as the Move Prover reads it.
        let opaqueCall : MetaM Lean.Expr := do
          let some path := context.unit.tables.namespaces[reference.namespaceId.index]?
            | throwError "specification function namespace has no path"
          let own := "::".intercalate (path.segments.toList ++ [qualified.name])
          -- A native read by its Prover model denotes the model's value.
          let model : Option NativeModel := do
            let functionId ← context.unit.resolution.function? reference.name
            let declaration ← targetNs.functions[functionId.index]?
            guard (declaration.body == VerexIR.Validation.FunctionBody.absent)
            nativeModel? targetNs.profile own
          if let some .structuralOrder := model then
            let [left, right] := arguments.toList
              | throwError "a structural order compares two operands"
            let (leftValue, τ) ← nativeOperand context left
            let (rightValue, _) ← nativeOperand context right
            return ← orderingTerm context ty (← structuralOrderTerm context τ leftValue rightValue)
          let name : String := match model with
            | some (.uninterpreted _ (some function) _) => function
            | _ => own
          -- The type arguments as native types under the contract's family,
          -- which an instantiation of the contract resolves to the caller's,
          -- or `num`.
          let typeArgumentList := if arbitraryValue context.unit reference then #[]
            else instantiations.zip typeArgumentTypes
          let typeArgumentTerms ← typeArgumentList.mapM
            fun (instantiation, nty?) => do
              let some nty := nty?
                | match instantiation with
                  | .typeArg argument =>
                      match context.typeOf? argument.typeId with
                      | some (.integer .unbounded _) =>
                          return mkConst ``VerexSL.Contract.SpecTypeArgument.integer
                      | _ => throwError m!"a type argument of the specification function \
                          `{name}` has no native type ({repr instantiation})"
                  | _ => throwError m!"a type argument of the specification function \
                      `{name}` has no native type ({repr instantiation})"
              let some types := context.types
                | throwError "the specification function `{name}` is applied outside a family"
              return mkApp (mkConst ``VerexSL.Contract.SpecTypeArgument.native)
                (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.substWith) types (← quoteNTy nty))
          -- A native argument at the type the function takes it at
          -- (`uninterpretedParameterType`), where every argument reads
          -- losslessly, a caller's as the function's own contract's.
          let (parameterTypeIds, resultTypeIds) : Array TypeId × Array TypeId :=
            match context.unit.resolution.specFunction? reference.name with
            | some id => ((targetNs.specFunctions[id.index]?).map fun declaration =>
                (declaration.signature.parameters.map (·.typeUse.typeId),
                  declaration.signature.results.map (·.typeId))).getD (#[], #[])
            | none => match context.unit.resolution.function? reference.name with
              | some id => ((targetNs.functions[id.index]?).map fun declaration =>
                  (declaration.signature.parameters.map (·.typeUse.typeId),
                    declaration.signature.results.map (·.typeId))).getD (#[], #[])
              | none => (#[], #[])
          let callee : Context := { context with
            namespaceId := reference.namespaceId, ns := targetNs, typeArgumentTypes }
          -- A position the function declares native, a type parameter's
          -- among them, takes a native value whatever the domain the caller
          -- reads its argument at.
          let declaredNative (typeId : TypeId) : Bool :=
            (callee.valueTypeOf? typeId).map domainOf == some .native
          let values ← arguments.mapIdxM fun index argument => do
            let some argumentType := typeOfExpr? context argument
              | throwError "a specification function argument has an unknown type"
            let domain := domainOf argumentType
            let value ← translate context argument
            match parameterTypeIds[index]? with
            | some parameterTypeId =>
                if declaredNative parameterTypeId then
                  let τ ← callee.uninterpretedParameterType parameterTypeId
                  if domain == .native then
                    let σ ← context.exprType argument
                    if σ == τ then return value
                    return ← coerceNative (← context.carriers) σ τ value
                  return ← nativeOfClause (← context.carriers) τ value
                bundleComponent domain value
            | none => bundleComponent domain value
          let domain := domainOf ty
          -- A native result read at the call's domain.
          if let some resultTypeId := resultTypeIds[0]? then
            if declaredNative resultTypeId && domain != .native then
              let nty ← callee.nativeType resultTypeId
              let carriers ← context.carriers
              let τ ← quoteNTy nty
              let carrier := carrierType carriers τ
              let inhabited := mkApp2 (mkConst ``Inhabited.mk [1]) carrier
                (← typedInhabitant carriers τ)
              return ← clauseOfNative nty
                (← opaqueValue name typeArgumentTerms.toList carrier inhabited values targetNs.profile)
          match domain, domain.scalarType? with
          | .native, _ =>
              -- An unspecified value of a native type is a native value.
              let nty ← context.nativeType resultTypeId
              let carriers ← context.carriers
              let τ ← quoteNTy nty
              let carrier := carrierType carriers τ
              let inhabited := mkApp2 (mkConst ``Inhabited.mk [1]) carrier
                (← typedInhabitant carriers τ)
              opaqueValue name typeArgumentTerms.toList carrier inhabited values targetNs.profile
          | _, some type =>
              let inhabited ← synthInstance (← mkAppM ``Inhabited #[type])
              domain.ofBinder
                (← opaqueValue name typeArgumentTerms.toList type inhabited values targetNs.profile)
          | _, none => unreachable!
        let some functionId := context.unit.resolution.specFunction? reference.name
          | if (context.unit.resolution.function? reference.name).isSome then opaqueCall
            else throwError "specification function call target does not resolve"
        let some declaration := targetNs.specFunctions[functionId.index]?
          | throwError "specification function declaration is out of range"
        unless arguments.size == declaration.signature.parameters.size do
          throwError "specification function call argument count differs from its declaration"
        let some body := declaration.body
          | opaqueCall
        unless declaration.signature.results.size == 1 do
          throwError "generated contracts currently expand specification functions with one result"
        let callee : Context := {
          context with
          namespaceId := reference.namespaceId
          ns := targetNs
          results := #[]
          resultTypes := #[]
          specCallStack := context.specCallStack.push reference
          typeArgumentTypes }
        let mut targetLocals : Array (Option Lean.Expr) :=
          Array.replicate declaration.locals.size none
        let mut targetTypes : Array IrTy := #[]
        for localDecl in declaration.locals do
          let some localType := callee.valueTypeOf? localDecl.type.typeId
            | throwError "specification function local has an unknown type"
          targetTypes := targetTypes.push localType
        let mut values : Array Lean.Expr := #[]
        let mut parameterTypes : Array IrTy := #[]
        for (argument, index) in arguments.zipIdx do
          let some parameter := declaration.signature.parameters[index]?
            | throwError "specification function parameter is out of range"
          let some parameterType := callee.valueTypeOf? parameter.typeUse.typeId
            | throwError "specification function parameter has an unknown type"
          let value ← translate context argument
          let value ← match domainOf parameterType with
            | .boolean => decideProposition value
            | .native =>
                -- A type parameter's value is native, whatever the domain the
                -- caller reads its argument at.
                match typeOfExpr? context argument with
                | some argumentType =>
                    if domainOf argumentType == .native then pure value
                    else do
                      let τ ← callee.nativeType parameter.typeUse.typeId
                      nativeOfClause (← context.carriers) τ value
                | none => pure value
            | _ => pure value
          targetLocals := targetLocals.set! index (some value)
          values := values.push value
          parameterTypes := parameterTypes.push parameterType
        let expanded ← translate { callee with
          locals := targetLocals, localTypes := targetTypes, oldLocals := targetLocals
          localNames := declaration.locals.map (·.name) } body
        -- The value at the call's type, where the specification types the
        -- call otherwise than the function's result (`coerceNative`).
        let atCall (value : Lean.Expr) : MetaM Lean.Expr := do
          let some result := declaration.signature.results[0]? | return value
          unless domainOf ty == .native do
            -- A type parameter's value is native, read at the call's domain.
            if (callee.valueTypeOf? result.typeId).map domainOf == some .native then
              return ← clauseOfNative (← callee.nativeType result.typeId) value
            return value
          let σ ← callee.nativeType result.typeId
          let τ ← context.nativeType resultTypeId
          if σ == τ then return value
          coerceNative (← context.carriers) σ τ value
        -- The function is defined where its arguments fit the fixed-width
        -- types of its parameters, and unspecified elsewhere.
        let some bounds ← valueBounds values (parameterTypes.map domainOf) parameterTypes
          | atCall expanded
        -- The unspecified value is an uninterpreted function of the
        -- arguments, at the frame where one is native.
        let shape ← definitionShapeAt reference typeArgumentTypes
        let bundle ← bundleArguments shape arguments
        let outside ← if shape.natives.any (·.isSome) || shape.resultNative.isSome then do
            let frame ← context.frame
            let member ← memberOfShape shape reference (some (← frameCarriers frame))
            outsideValue member ((← frameLeading frame).push frame) (some frame) none
              shape.typeArgumentTypes bundle
          else outsideValue (← memberOfShape shape reference none) #[] none none #[] bundle
        atCall (← mkAppM ``ite #[bounds, expanded, outside])
    | .specification (.result index) =>
        match context.ns.profile, context.resultTypes[0]?, context.results[0]? with
        | some .move, some (.tuple _), some _ =>
            -- Move presents multiple returns as one tuple-typed physical
            -- result. `spec.result[i]` nevertheless denotes component `i`
            -- of the native tuple, without inventing extra result slots.
            let some component := context.resultComponents[index]?
              | throwError "specification result {index} is out of range"
            let some logicalType := typeOfResultComponent? context index
              | throwError "specification result {index} has no logical domain"
            (domainOf logicalType).ofBinder component
        | _, _, _ =>
            let some binder := context.results[index]?
              | throwError "specification result {index} has no binder"
            let some logicalType := context.resultTypes[index]?
              | throwError "specification result {index} has no logical domain"
            (domainOf logicalType).ofBinder binder
    | .specification .final =>
        let some argument := arguments[0]?
          | throwError "`final` expects one argument"
        let index ← match context.ns.expressions[argument.index]?.map (·.kind) with
          | some (VerexIR.ExprKind.operation (.specification (.result index)) _ _ _) =>
              pure index
          | _ => throwError "`final` reads a returned mutable reference (`result`)"
        let some (some final) := context.resultFinals[index]?
          | throwError "`final` has no final value for result {index} in this clause"
        (domainOf ty).ofBinder final
    | .specification (.behavior kind range) => translateBehavior kind range arguments ty
    | .specification .old =>
        let some argument := arguments[0]?
          | throwError "spec.old expects one argument"
        translate { context with
          locals := context.oldLocals, state := context.oldState } argument
    | .specification (.withStateAnchor label) =>
        -- `old` inside the anchor reads the state the anchor saved.
        let some argument := arguments[0]?
          | throwError "spec.withStateAnchor expects one argument"
        let (anchorLocals, anchorState) ← match context.labeledAnchors.find? (·.1 == label) with
          | some (_, locals, state) => pure (locals, state)
          | none =>
              let some anchorState := context.anchorState
                | throwError "a state anchor is read where no anchor is recorded"
              pure (context.anchorLocals, anchorState)
        translate { context with
          oldLocals := anchorLocals, oldState := some anchorState } argument
    | .specification .bitVectorToInt | .specification .intToBitVector =>
        -- The logical domain already represents every integer unbounded, so
        -- reading a fixed-width value at `Int` is the identity.
        let some argument := arguments[0]?
          | throwError "spec.bitVectorToInt expects one argument"
        translate context argument
    | .specification .indexVector =>
        let some vector := arguments[0]?
          | throwError "spec.indexVector expects a vector operand"
        let some index := arguments[1]?
          | throwError "spec.indexVector expects an index operand"
        let carriers ← context.carriers
        let .vector element ← context.exprType vector
          | throwError "spec.indexVector reads a value that is not a vector"
        let position ← mkAppM ``Int.toNat #[← translate context index]
        -- An element is its value at the position, the element type's
        -- inhabitant outside the vector.
        let inhabitant ← typedInhabitant carriers (← quoteNTy element)
        let values ← mkAppM ``VerexIR.SpecVector.values #[← translate context vector]
        let read ← mkAppM ``GetElem?.getElem? #[values, position]
        (domainOf ty).ofNative (← mkAppM ``Option.getD #[read, inhabitant])
    | .specification .lengthVector =>
        let some vector := arguments[0]?
          | throwError "spec.lengthVector expects a vector operand"
        let values ← mkAppM ``VerexIR.SpecVector.values #[← translate context vector]
        mkAppOptM ``Nat.cast #[mkConst ``Int, none, ← mkAppM ``Array.size #[values]]
    | .specification .containsVector =>
        let some vector := arguments[0]?
          | throwError "spec.containsVector expects a vector operand"
        let some element := arguments[1]?
          | throwError "spec.containsVector expects an element operand"
        let (elementValue, ε) ← nativeOperand context element
        containment (← context.exprType vector) (← translate context vector) ε elementValue
    | .specification .emptyVector =>
        nativeVectorLiteral (← context.carriers) (← context.nativeType resultTypeId) #[]
    | .specification .concatVector =>
        let (some left, some right) := (arguments[0]?, arguments[1]?)
          | throwError "spec.concatVector expects two vector operands"
        nativeVectorConcat (← translate context left) (← translate context right)
    | .specification .sliceVector =>
        let (some vector, some range) := (arguments[0]?, arguments[1]?)
          | throwError "spec.sliceVector expects a vector and a range"
        let (lower, upper) ← translateRangeBounds context range
        mkAppM ``VerexIR.SpecVector.extract #[← translate context vector,
          ← mkAppM ``Int.toNat #[lower], ← mkAppM ``Int.toNat #[upper]]
    | .specification .singletonVector =>
        let some element := arguments[0]?
          | throwError "a singleton vector expects one element"
        nativeVectorLiteral (← context.carriers) (← context.nativeType resultTypeId)
          #[← translate context element]
    | .specification .updateVector =>
        let some vector := arguments[0]?
          | throwError "spec.updateVector expects a vector operand"
        let some index := arguments[1]?
          | throwError "spec.updateVector expects an index operand"
        let some replacement := arguments[2]?
          | throwError "spec.updateVector expects a replacement operand"
        let (replaced, _) ← nativeOperand context replacement
        mkAppM ``VerexIR.SpecVector.set #[← translate context vector,
          ← mkAppM ``Int.toNat #[← translate context index], replaced]
    | .specification (.publish range) | .specification (.remove range) |
        .specification (.update range) =>
        let .specification operation := operation | unreachable!
        let post ← match range.post with
          | some label => (·.memory) <$> context.labelState label
          | none => currentState
        let (memory, condition) ← translateStateChange context operation instantiations arguments
          (some post)
        mkAppM ``And #[← mkEq post memory, condition]
    | .specification (.global label) =>
        let some key := arguments[0]?
          | throwError "a storage read expects one key"
        let state ← match label with
          | some label => (·.memory) <$> context.labelState label
          | none => currentState
        let (slot, resource, nativeType) ← context.slot state
          (← resourceType instantiations) (← addressKey key)
        (domainOf ty).ofNative (← nativeSlot (← context.frame) nativeType resource slot)
    | .specification (.exists label) =>
        let some key := arguments[0]?
          | throwError "a storage existence test expects one key"
        let state ← match label with
          | some label => (·.memory) <$> context.labelState label
          | none => currentState
        let (slot, _, _) ← context.slot state (← resourceType instantiations) (← addressKey key)
        slotPresent slot
    | .global .contains =>
        let some key := arguments[0]?
          | throwError "a storage existence test expects one key"
        let (slot, _, _) ← context.slot (← currentState) (← resourceType instantiations)
          (← addressKey key)
        slotPresent slot
    | .data (.select reference field) =>
        let some base := arguments[0]?
          | throwError "a field selection expects one operand"
        let some index := VerexIR.SemanticOperations.referencedFieldIndex?
            context.unit context.namespaceId reference none field
          | throwError "specification field `{field}` does not resolve in \
              generated contracts"
        -- A field of a native struct is its component of the struct's row.
        let τ ← context.exprType base
        let some (_, component) := (← nativeComponents (← context.carriers) (← quoteNTy τ)
            (← translate context base))[index]?
          | throwError "specification field `{field}` is out of the value's row"
        (domainOf ty).ofNative component
    -- A ghost field's slot follows the declared fields; it reads as its
    -- contents (`doc/designs/ghost-fields.md`).
    | .data (.selectGhost reference field) =>
        let some base := arguments[0]?
          | throwError "a ghost field selection expects one operand"
        let handle ← structHandleOf context reference
        let some owner := context.unit.namespaces[handle.namespaceId.index]?
          | throwError "a ghost field selection has no owner namespace"
        let some declaration := owner.structs[handle.structId]?
          | throwError "a ghost field selection has no owner declaration"
        let carriers ← context.carriers
        let τ ← context.exprType base
        let τExpr ← quoteNTy τ
        let value ← translate context base
        let selected ← if declaration.variants.isEmpty then do
            let some index :=
                VerexIR.SemanticOperations.handleGhostIndex? context.unit handle none field
              | throwError "ghost field `{field}` does not resolve in generated contracts"
            let some (_, component) := (← nativeComponents carriers τExpr value)[index]?
              | throwError "ghost field `{field}` is out of the value's row"
            pure component
          else do
            -- Each variant's slot follows its own fields.
            let some choices := declaration.variants.toList.mapM fun (variant : VerexIR.VariantDecl) => do
                let named : VerexIR.QualifiedName ← owner.tables.names[variant.name.index]?
                let name := named.name
                let index ← VerexIR.SemanticOperations.handleGhostIndex? context.unit handle
                  (some name) field
                some (name, index)
              | throwError "ghost field `{field}` does not resolve in generated contracts"
            let (_, read) ← nativeVariantField τ choices.toArray carriers τExpr value
            pure read
        (domainOf ty).ofNative selected
    | .data (.selectVariants reference fields) =>
        let some base := arguments[0]?
          | throwError "a variant field selection expects one operand"
        let handle ← structHandleOf context reference
        let some choices := VerexIR.SemanticOperations.variantFieldChoices?
            context.unit handle fields
          | throwError "a variant field selection has no choices"
        let τ ← context.exprType base
        let (_, read) ← nativeVariantField τ choices (← context.carriers) (← quoteNTy τ)
          (← translate context base)
        (domainOf ty).ofNative read
    | .data (.updateField reference field) =>
        let #[base, replacement] := arguments
          | throwError "a field update expects two operands"
        let handle ← structHandleOf context reference
        let some owner := context.unit.namespaces[handle.namespaceId.index]?
          | throwError "a field update has no owner namespace"
        let some declaration := owner.structs[handle.structId]?
          | throwError "a field update has no owner declaration"
        if owner.intrinsics.any (fun intrinsic =>
            intrinsic.model == "map" && intrinsic.owner == declaration.name) then
          throwError "cannot update a field of an intrinsic map type in a specification"
        let built ← context.exprType base
        let carriers ← context.carriers
        let baseValue ← translate context base
        let replacementValue ← translate context replacement
        if declaration.variants.isEmpty then
          let some index := VerexIR.SemanticOperations.handleFieldIndex? context.unit handle none field
            | throwError "specification field `{field}` does not resolve in generated contracts"
          return ← nativeFieldUpdate carriers built index baseValue replacementValue
        let variants : Array String := declaration.variants.filterMap fun variant =>
          (owner.tables.names[variant.name.index]?).map (·.name)
        let choices := variants.toList.filterMap fun variant =>
          (VerexIR.SemanticOperations.handleFieldIndex? context.unit handle (some variant) field).map
            (variant, ·)
        unless choices.length == variants.size do
          throwError "a field update on an enum whose variants do not all carry the field is not supported yet"
        nativeVariantFieldUpdate carriers built choices baseValue replacementValue
    | .data (.testVariants reference variants) =>
        let some base := arguments[0]?
          | throwError "a variant test expects one operand"
        let handle ← structHandleOf context reference
        let τ ← context.exprType base
        nativeVariantTest (← context.carriers) (← quoteNTy τ) (← translate context base) handle variants
    | .call (.constructor _ variant) =>
        -- A value a specification builds carries its type's ghost slots,
        -- their contents unspecified (`doc/designs/ghost-fields.md`).
        let operands ← arguments.mapM (translate context)
        nativeConstructed (← context.nativeType resultTypeId) variant (← context.carriers) operands
    | .reference .dereference =>
        -- Specifications see through references: a dereference is the
        -- operand itself.
        let some argument := arguments[0]?
          | throwError "a dereference expects one operand"
        translate context argument
    | .primitive primitive => translatePrimitive primitive arguments ty resultTypeId
    | .call (.closure reference mask) =>
        functionLiteral context reference mask instantiations arguments
    | _ =>
        throwError "specification operation {repr operation} is not supported \
          in generated contracts"
  /-- The logical type of a component of a Move tuple result. -/
  typeOfResultComponent? (context : Context) (index : Nat) : Option IrTy := do
    let .tuple elements ← context.resultTypes[0]? | none
    let element ← elements[index]?
    context.valueTypeOf? element
  /-- A behavioral predicate over the invocation of a function value
  (`Proofs/Behavior.lean`): from the function's entry state, and in
  `ensures` to its exit state, as the Move Book reads a predicate without
  state labels. -/
  translateBehavior (kind : VerexIR.BehaviorKind) (range : VerexIR.MemoryRange)
      (arguments : Array ExprId) (ty : IrTy) : MetaM Lean.Expr := do
    let invocation ← translateInvocation context range arguments
    let post ← match range.post with
      | some label => (·.memory) <$> context.labelState label
      | none => currentState
    let carriers ← context.carriers
    match kind with
    | .abortsOf =>
        invocation.predicate ``VerexIR.Proofs.AbortsOf #[← invocation.arguments, invocation.pre]
    | .ensuresOf =>
        let some callableExpression := context.ns.expressions[arguments[0]!.index]?
          | throwError "a behavioral predicate's function value is out of range"
        let some (.function parameters _ _) := context.valueTypeOf? callableExpression.typeId
          | throwError "a behavioral predicate's operand is not a function value"
        let outputs := arguments.extract (parameters.size + 1) arguments.size
        let count := (invocation.row.toList.filter (· matches .ref _)).length
        -- The declared result, then each mutable reference's final value,
        -- read at the post-state.
        unless count ≤ outputs.size do
          throwError "`ensures_of` names a final value for each mutable reference argument"
        let declared := outputs.extract 0 (outputs.size - count)
        let finals := outputs.extract (outputs.size - count) outputs.size
        let finalContext ← context.mutableAt range.post (entry := false)
        let mut finalValues := #[]
        for (final, τ) in finals.toList.zip
            (invocation.row.toList.filterMap fun | .ref σ => some σ | _ => none) do
          finalValues := finalValues.push (← nativeOfClause carriers τ (← translate finalContext final))
        let result ← match invocation.shape, declared.toList with
          | .none, [] => pure (mkConst ``Unit.unit)
          | .one (.ref σ), [output] =>
              -- A returned mutable reference at its value at return.
              let value ← nativeOfClause carriers σ (← translate context output)
              mkAppM ``Prod.mk #[value, value]
          | .one τ, [output] => nativeOfClause carriers τ (← translate context output)
          | _, _ => throwError "`ensures_of` names one value for the function's result"
        invocation.predicate ``VerexIR.Proofs.EnsuresOf
          #[← invocation.arguments finalValues, result, invocation.pre, post]
    | .resultOf =>
        let default ← match invocation.shape with
          | .none => pure (mkConst ``Unit.unit)
          | .one τ => typedInhabitant carriers (← quoteNTy τ)
        let value ← invocation.predicate ``VerexIR.Proofs.ResultOf
          #[← invocation.arguments, invocation.pre, default]
        match invocation.shape with
        | .none => pure value
        | .one (.ref _) => (domainOf ty).ofNative (← mkAppM ``Prod.fst #[value])
        | .one _ => (domainOf ty).ofNative value
    | .requiresOf =>
        let some table := context.requiresMeaning
          | throwError "`requires_of` reads the requires meaning, which this \
              contract does not take"
        return mkAppN (mkConst ``VerexIR.Proofs.RequiresOf)
          #[invocation.unit, invocation.functions, invocation.frame, table,
            ← quoteRow invocation.row,
            toExpr invocation.shared, ← quoteRow invocation.shape.row, invocation.function,
            ← invocation.arguments, invocation.pre]
    | .unchangedOf | .foldsOf | .writeOf _ =>
        throwError "behavioral predicate {repr kind} is not carried in generated contracts"
  binary (arguments : Array ExprId) : MetaM (Lean.Expr × Lean.Expr) := do
    let some left := arguments[0]? | throwError "a binary specification operation needs two operands"
    let some right := arguments[1]? | throwError "a binary specification operation needs two operands"
    return (← translate context left, ← translate context right)
  unary (arguments : Array ExprId) : MetaM Lean.Expr := do
    let some only := arguments[0]? | throwError "a unary specification operation needs one operand"
    translate context only
  /-- An equation of two operands, as a specification's equality reads them:
  Booleans as an equivalence, integers and strings as themselves, native
  values as values, a value a specification builds component by component,
  and a type holding a part equality reads through its identity by
  `NTy.equal`. -/
  equation (left right : ExprId) : MetaM Lean.Expr := do
    let some operandTy := typeOfExpr? context left
      | throwError "an equality operand has an unknown type"
    let l ← translate context left
    let r ← translate context right
    match domainOf operandTy with
    | .boolean => mkAppM ``Iff #[l, r]
    | .integer | .text => mkEq l r
    | .native =>
        let τ ← context.exprType left
        let σ ← context.exprType right
        if let some equation ← constructionEquation? τ l right then return equation
        if let some equation ← constructionEquation? σ r left then return equation
        if τ == σ then
          return ← nativeEquation (← context.carriers) (← quoteNTy τ) l r (holdsIdentity left)
        -- Values of types differing in the widths of their integers, at the
        -- type of both with their integers unbounded.
        let ω := widenedNTy τ
        unless widenedNTy σ == ω do
          throwError m!"values of the types {repr τ} and {repr σ} are compared, which \
            generated contracts do not carry"
        let carriers ← context.carriers
        nativeEquation carriers (← quoteNTy ω) (← coerceNative carriers τ ω l)
          (← coerceNative carriers σ ω r) (holdsIdentity left)
  /-- Whether a vector holds an element, at the vector's element type, or at
  both widened where they differ in the widths of their integers. -/
  containment (vectorType : VerexIR.Proofs.Denote.NTy) (vector : Lean.Expr)
      (elementType : VerexIR.Proofs.Denote.NTy) (element : Lean.Expr) : MetaM Lean.Expr := do
    let .vector held := vectorType | throwError "internal: membership in a value that is no vector"
    if held == elementType then
      return ← mkAppM ``Membership.mem #[← mkAppM ``VerexIR.SpecVector.values #[vector], element]
    let ω := widenedNTy held
    unless widenedNTy elementType == ω do
      throwError m!"an element of type {repr elementType} is sought in a vector of \
        {repr held}, which generated contracts do not carry"
    let carriers ← context.carriers
    mkAppM ``Membership.mem
      #[← mkAppM ``VerexIR.SpecVector.values #[← coerceNative carriers vectorType (.vector ω) vector],
        ← coerceNative carriers elementType ω element]
  translatePrimitive (primitive : PrimitiveOperation) (arguments : Array ExprId)
      (ty : IrTy) (resultTypeId : TypeId) : MetaM Lean.Expr := do
    match primitive with
    -- A signer's carrier is the address it holds.
    | .signerAddress => unary arguments
    | .vector =>
        nativeVectorLiteral (← context.carriers) (← context.nativeType resultTypeId)
          (← arguments.mapM (translate context))
    | .tuple =>
        let .tuple row ← context.nativeType resultTypeId
          | throwError "a tuple's type is not a native tuple"
        nativeRow (← context.carriers) row (← arguments.toList.mapM (translate context))
    | .pushVector =>
        let some vector := arguments[0]?
          | throwError "pushVector expects a vector operand"
        let some element := arguments[1]?
          | throwError "pushVector expects an element operand"
        let (elementValue, _) ← nativeOperand context element
        mkAppM ``VerexIR.SpecVector.push #[← translate context vector, elementValue]
    | .concatVector =>
        let some left := arguments[0]?
          | throwError "concatVector expects two vector operands"
        let some right := arguments[1]?
          | throwError "concatVector expects two vector operands"
        nativeVectorConcat (← translate context left) (← translate context right)
    | .compare =>
        let (left, right) ← match arguments.toList with
          | [left, right] => pure (left, right)
          | _ => throwError "compare expects two operands"
        let some operandTy := typeOfExpr? context left
          | throwError "a compared operand has an unknown type"
        -- Integers compare as integers (`NTy.order` at an integer type).
        let ordered ← if domainOf operandTy == .integer then
            mkAppM ``compare #[← translate context left, ← translate context right]
          else do
            let (leftValue, τ) ← nativeOperand context left
            let (rightValue, _) ← nativeOperand context right
            structuralOrderTerm context τ leftValue rightValue
        mkAppM ``VerexIR.orderValue #[ordered]
    | .containsVector =>
        let some vector := arguments[0]?
          | throwError "containsVector expects a vector operand"
        let some element := arguments[1]?
          | throwError "containsVector expects an element operand"
        let (elementValue, ε) ← nativeOperand context element
        containment (← context.exprType vector) (← translate context vector) ε elementValue
    | .add => let (l, r) ← binary arguments; mkAppM ``HAdd.hAdd #[l, r]
    | .subtract => let (l, r) ← binary arguments; mkAppM ``HSub.hSub #[l, r]
    | .multiply => let (l, r) ← binary arguments; mkAppM ``HMul.hMul #[l, r]
    | .divide => let (l, r) ← binary arguments; mkAppM ``Int.tdiv #[l, r]
    | .modulo => let (l, r) ← binary arguments; mkAppM ``Int.tmod #[l, r]
    | .negate => let value ← unary arguments; mkAppM ``Neg.neg #[value]
    | .bitwiseAnd =>
        let (l, r) ← binary arguments
        if ty == VerexIR.Ty.bool then mkAppM ``And #[l, r]
        else mkAppM ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd #[l, r]
    | .bitwiseOr =>
        let (l, r) ← binary arguments
        if ty == VerexIR.Ty.bool then mkAppM ``Or #[l, r]
        else mkAppM ``VerexIR.Proofs.IntegerArithmetic.bitwiseOr #[l, r]
    | .bitwiseXor =>
        let (l, r) ← binary arguments
        if ty == VerexIR.Ty.bool then mkAppM ``Not #[← mkAppM ``Iff #[l, r]]
        else mkAppM ``VerexIR.Proofs.IntegerArithmetic.bitwiseXor #[l, r]
    | .bitwiseNot => let value ← unary arguments; mkAppM ``Int.not #[value]
    | .shiftLeft =>
        let (l, r) ← binary arguments
        mkAppM ``Int.shiftLeft #[l, ← mkAppM ``Int.toNat #[r]]
    | .shiftRight =>
        let (l, r) ← binary arguments
        mkAppM ``Int.shiftRight #[l, ← mkAppM ``Int.toNat #[r]]
    | .less => let (l, r) ← binary arguments; mkAppM ``LT.lt #[l, r]
    | .greater => let (l, r) ← binary arguments; mkAppM ``LT.lt #[r, l]
    | .lessEqual => let (l, r) ← binary arguments; mkAppM ``LE.le #[l, r]
    | .greaterEqual => let (l, r) ← binary arguments; mkAppM ``LE.le #[r, l]
    | .equal =>
        let (some left, some right) := (arguments[0]?, arguments[1]?)
          | throwError "an equality needs two operands"
        equation left right
    | .notEqual =>
        let (some left, some right) := (arguments[0]?, arguments[1]?)
          | throwError "an inequality needs two operands"
        return mkNot (← equation left right)
    | .logicalAnd => let (l, r) ← binary arguments; mkAppM ``And #[l, r]
    | .logicalOr => let (l, r) ← binary arguments; mkAppM ``Or #[l, r]
    | .logicalNot => let value ← unary arguments; mkAppM ``Not #[value]
    | .implies => let (l, r) ← binary arguments; return ← mkArrow l r
    | .equivalent => let (l, r) ← binary arguments; mkAppM ``Iff #[l, r]
    | _ =>
        throwError "specification primitive {repr primitive} at type \
          {describeType ty} is not supported in generated contracts"

end

/-- The definitions of recursive specification functions a term applies,
through the abbreviations it unfolds to. -/
partial def definitionsIn (value : Lean.Expr) (found : Array Name := #[])
    (visited : NameSet := {}) : MetaM (Array Name × NameSet) := do
  let keys := definitionKeys.getState (← getEnv)
  let mut found := found
  let mut visited := visited
  for name in value.getUsedConstants do
    if visited.contains name then continue
    visited := visited.insert name
    if keys.contains name then
      unless found.contains name do found := found.push name
    else if let some (.defnInfo info) := (← getEnv).find? name then
      if info.hints.isAbbrev then
        (found, visited) ← definitionsIn info.value found visited
  return (found, visited)

/-- The name of a definition's transport across the frame type arguments
`θ` induce. -/
def transportName (definition : Name) (θ : VerexIR.Proofs.Denote.NRow) : Name :=
  Name.str definition ("transport_" ++ typeArgumentsSpelling θ)

/-- That two well-founded fixpoints agree at every argument: by induction on
the first's relation, a step of each agreeing with the other's under the
normal form, the recursion's calls the second's (`fix_eq`). `cases`
alternatives split an argument of a group's sum first. -/
private def fixpointsAgree (source target : Lean.Expr) (cases : Nat) (transports : Array Name) :
    Lean.Elab.Term.TermElabM Lean.Expr := do
  let_expr WellFounded.fix α motive relation wf step := source
    | throwError m!"internal: a definition is not a well-founded fixpoint: {source}"
  let_expr WellFounded.fix _ _ _ wfTarget stepTarget := target
    | throwError m!"internal: a definition is not a well-founded fixpoint: {target}"
  let goalMotive ← withLocalDeclD `x α fun x => do
    mkLambdaFVars #[x] (← mkEq (mkApp source x) (mkApp target x))
  let lemmas ← transports.mapM fun name =>
    `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)
  let alternatives := (List.replicate cases (← `(rcasesPat| x))).toArray
  let split ← if cases ≤ 1 then `(tactic| skip)
    else `(tactic| rcases x with $[$alternatives]|*)
  let stepProof ← withLocalDeclD `x α fun x => do
    let ihType ← withLocalDeclD `y α fun y => do
      mkForallFVars #[y] (← mkArrow (mkApp2 relation y x) (mkApp goalMotive y).headBeta)
    withLocalDeclD `ih ihType fun ih => do
      let unfolded ← mkAppM ``WellFounded.fix_eq #[wf, step, x]
      let unfoldedTarget ← mkAppM ``WellFounded.fix_eq #[wfTarget, stepTarget, x]
      let some (_, _, recursed) := (← inferType unfolded).eq? | throwError "internal: fix_eq"
      let some (_, _, recursedTarget) := (← inferType unfoldedTarget).eq?
        | throwError "internal: fix_eq"
      -- The recursion's calls are the target's, below `x` by the hypothesis.
      let calls ← withLocalDeclD `y α fun y => withLocalDeclD `h (mkApp2 relation y x) fun h => do
        mkLambdaFVars #[y, h] (mkApp target y)
      let below ← withLocalDeclD `y α fun y => do
        mkLambdaFVars #[y] (← mkFunExt (← withLocalDeclD `h (mkApp2 relation y x) fun h => do
          mkLambdaFVars #[h] (mkApp2 ih y h)))
      let recursion ← mkCongrArg (mkApp step x) (← mkFunExt below)
      let bodies ← mkFreshExprMVar (← mkEq (mkApp2 step x calls) recursedTarget)
      Lean.Elab.Term.runTactic bodies.mvarId! (← `(tactic| (
        $split:tactic
        all_goals (try simp only [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd,
          $lemmas,*])
        all_goals rfl))) (kind := .term)
      let chain ← mkEqTrans unfolded (← mkEqTrans recursion
        (← mkEqTrans (← instantiateMVars bodies) (← mkEqSymm unfoldedTarget)))
      mkLambdaFVars #[x, ih] chain
  withLocalDeclD `x α fun x => do
    mkLambdaFVars #[x] (← mkAppOptM ``WellFounded.induction
      #[α, relation, wf, goalMotive, x, stepProof])

/-- The value a definition's constant applies at parameters, its own
fixpoint or its group's: `fun params => body` at them. -/
private def valueAt (name : Name) (parameters : Array Lean.Expr) : MetaM Lean.Expr := do
  let some info := (← getEnv).find? name | throwError m!"internal: `{name}` is not defined"
  let some value := info.value? | throwError m!"internal: `{name}` has no value"
  return value.beta parameters

/-- The transport of a recursive specification function's definition
across the frame type arguments `θ` induce (`doc/designs/native-verification.md`,
"Specialized contracts"): its value at the induced frame is the definition
at its type arguments' native types with `θ` substituted, at the caller's
frame, at every bundle, for each member of its group; the definitions its
bodies apply are transported first. The entry's transport, none for a
constant that is not a definition. -/
partial def ensureDefinitionTransport (unit : ValidatedUnit) (source : Name)
    (θ : VerexIR.Proofs.Denote.TypeArgs) : Lean.Elab.Term.TermElabM (Option Name) := do
  let some key := (definitionKeys.getState (← getEnv)).find? source | return none
  if (← getEnv).contains (transportName source θ.1) then return some (transportName source θ.1)
  let some reference := key.reference? unit
    | throwError m!"the specification function `{source}` is defined for is not in this unit"
  let some σ := key.typeArgumentTypes.mapM id
    | throwError m!"a type argument of `{source}` has no native type"
  let σθ := σ.map (·.subst θ.1)
  let some (targetNs, _) := specFunctionOf? unit reference
    | throwError m!"the specification function `{source}` is defined for does not resolve"
  let context : Context := {
    unit, namespaceId := reference.namespaceId, ns := targetNs, locals := #[], results := #[] }
  discard <| translateExpression.ensureDefinitionAt context reference (σθ.map some)
  let members ← (specGroup unit reference).mapM fun member => do
    let atSource ← translateExpression.definitionShapeAt context member (σ.map some)
    let atTarget ← translateExpression.definitionShapeAt context member (σθ.map some)
    pure (atSource.name, atTarget.name)
  let grouped := members.size > 1
  let groupOf (name : Name) := if grouped then Name.str members[0]!.1 "group" else name
  -- The definitions the group's bodies apply besides its own, transported first.
  let mut transports := #[]
  let bodies := if grouped then #[groupOf members[0]!.1] else members.map (·.1)
  for body in bodies do
    let some value := ((← getEnv).find? body).bind (·.value?) | continue
    for dependency in (← definitionsIn value).1 do
      if members.any (·.1 == dependency) then continue
      if let some transport ← ensureDefinitionTransport unit dependency θ then
        unless transports.contains transport do transports := transports.push transport
  let θExpr ← quoteTypeArgs θ
  let some targetInfo := (← getEnv).find? members[0]!.2
    | throwError m!"internal: `{members[0]!.2}` is not defined"
  forallTelescope targetInfo.type fun binders _ => do
    let parameters := binders.pop
    let some frameIndex ← parameters.findIdxM? fun parameter => do
        return (← inferType parameter).isAppOfArity ``VerexIR.Proofs.Denote.Skolems 2
      | throwError m!"the definition `{source}` takes no frame, which its transport crosses"
    let frame := parameters[frameIndex]!
    let induced := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.instantiate)
      ((← frameLeading frame) ++ #[θExpr, frame])
    let sourceParameters := parameters.set! frameIndex induced
    let agree ← fixpointsAgree (← valueAt (groupOf members[0]!.1) sourceParameters)
      (← valueAt (groupOf members[0]!.2) parameters) members.size transports
    for (sourceMember, targetMember) in members do
      let sourceAt := mkAppN (mkConst sourceMember) sourceParameters
      let bundleType := (← whnf (← inferType sourceAt)).bindingDomain!
      let statement ← withLocalDeclD `bundle bundleType fun bundle => do
        mkForallFVars (parameters.push bundle)
          (← mkEq (mkApp sourceAt bundle) (mkApp (mkAppN (mkConst targetMember) parameters) bundle))
      let proof ← withLocalDeclD `bundle bundleType fun bundle => do
        -- A group's member is its group at the member's summand.
        let argument ← if grouped then
            pure (← valueAt sourceMember (sourceParameters.push bundle)).appArg!
          else pure bundle
        mkLambdaFVars (parameters.push bundle) (mkApp agree argument)
      let value ← instantiateMVars proof
      addDecl (.thmDecl
        { name := transportName sourceMember θ.1, levelParams := [], type := statement, value })
  return some (transportName source θ.1)

end VerexSL.Contract
