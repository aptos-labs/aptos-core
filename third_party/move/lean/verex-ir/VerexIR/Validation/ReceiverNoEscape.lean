-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Validated
import VerexIR.Validation.PlaceIndex

/-!
A deliberately bounded, frontend-independent receiver refinement. A receipt is
conditional on the declaration's scoped method dictionary requirements. The
analysis tracks dependency on the receiver, not just the final reference's loan
site: reborrowing and putting a reference in an aggregate must not erase it.
Unknown operations and recursive calls with dependent arguments fail closed.
-/
namespace VerexIR.Validation

/-- Dictionary parameters whose explicit receiver may not escape an invocation. -/
def scopedMethodParameters {Body : Type} (function : FunctionDecl Body) : Array Nat :=
  function.attributes.filterMap fun
    | .assign "scoped_method_evidence" (.constant (.integer index)) _ =>
        if index >= 0 then some index.toNat else none
    | _ => none

namespace ReceiverNoEscape

private def target? (unit : ValidatedUnit) (reference : QualifiedRef) :
    Option (ValidatedNamespace × FunctionDecl FunctionBody) := do
  let ns ← unit.namespaces[reference.namespaceId.index]?
  let function ← ns.functions.find? (·.name == reference.name)
  return (ns, function)

private def referenceBearing (ns : ValidatedNamespace) : Nat → TypeId → Bool
  | 0, _ => true
  | fuel + 1, id => match ns.tables.types[id.index]? with
    | some .unit | some .never | some .bool | some .character | some .string |
        some .bytes | some .address | some .signer | some (.integer ..) => false
    | some (.tuple elements) => elements.any (referenceBearing ns fuel)
    | some (.vector element _) | some (.slice element) => referenceBearing ns fuel element
    | _ => true

/-- Root local and whether a write reaches storage outside the local owner. -/
private def placeRoot (ns : ValidatedNamespace) : Nat → PlaceId → Option (LocalId × Bool)
  | 0, _ => none
  | fuel + 1, id => match ns.places[id.index]? with
    | some (.localVar localId) => some (localId, false)
    | some (.deref base) => (placeRoot ns fuel base).map fun (localId, _) => (localId, true)
    | some (.field base ..) | some (.subslice base ..) |
        some (.unsize base) | some (.downcast base _) => placeRoot ns fuel base
    | some (.index base index) =>
        match placeIndexForm? ns index with
        | none => none
        | some (.fromEnd source _) =>
            if (placeRoot ns fuel source).isSome then placeRoot ns fuel base else none
        | some _ => placeRoot ns fuel base
    | some (.ghostField ..) | none => none

private def patternLocals (ns : ValidatedNamespace) : Nat → PatternId → Array LocalId
  | 0, _ => #[]
  | fuel + 1, id => match ns.patterns[id.index]? with
    | some { kind := .variable localId, .. } => #[localId]
    | some { kind := .tuple children, .. } | some { kind := .constructor _ _ _ children, .. } =>
        children.flatMap (patternLocals ns fuel)
    | _ => #[]

/-- Visit the expression arena once, even when several parents share a child. -/
private def bodyNodes (ns : ValidatedNamespace) (_fuel : Nat) (root : ExprId) : Array ExprId := Id.run do
  let mut pending := #[root]
  let mut visited : Std.HashSet Nat := {}
  let mut nodes := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.insert id.index
    let some expression := ns.expressions[id.index]? | continue
    match expression.kind with
    | .spec _ | .quantifier .. | .operation (.specification _) .. => pure ()
    | kind =>
        nodes := nodes.push id
        pending := pending ++ (expressionChildren kind).reverse
  return nodes

private def writtenLocal (ns : ValidatedNamespace) (node : ExprId) (localId : LocalId) : Bool :=
  match ns.expressions[node.index]? with
  | some { kind := .assign place _, .. } | some { kind := .operation (.write place) _ _ _, .. } =>
      (placeRoot ns (ns.places.size + 1) place).any fun (root, outside) =>
        root == localId && !outside
  | some { kind := .operation (.borrow _ place) _ _ _, .. } =>
      (placeRoot ns (ns.places.size + 1) place).any fun (root, _) => root == localId
  | some { kind := .assignPattern pattern _, .. } =>
      (patternLocals ns (ns.patterns.size + 1) pattern).contains localId
  | some { kind := .match_ _ arms, .. } =>
      arms.any fun arm => (patternLocals ns (ns.patterns.size + 1) arm.pattern).contains localId
  | some { kind := .letDecl pattern _ _, .. } =>
      match ns.patterns[pattern.index]? with
      | some { kind := .variable _, .. } => false -- checked as a unique initializer below
      | _ => (patternLocals ns (ns.patterns.size + 1) pattern).contains localId
  | _ => false

/-- Only unmodified marked parameters and immutable aliases of them are trusted.
The caller checks literal closures separately; arbitrary function values do not
inherit the refinement from their structural function type. -/
private def markedDictionary (ns : ValidatedNamespace) (function : FunctionDecl FunctionBody)
    (nodes : Array ExprId) : Nat → ExprId → Bool
  | 0, _ => false
  | fuel + 1, id =>
      let localId? := match ns.expressions[id.index]? with
        | some { kind := .localVar localId, .. } => some localId
        | some { kind := .operation (.copy place) _ _ _, .. } |
          some { kind := .operation (.move place) _ _ _, .. } |
          some { kind := .operation (.read place) _ _ _, .. } =>
            match ns.places[place.index]? with
            | some (.localVar localId) => some localId
            | _ => none
        | _ => none
      match localId? with
      | none => false
      | some localId =>
          if nodes.any (writtenLocal ns · localId) then false else
          if (scopedMethodParameters function).contains localId.index then
            !nodes.any (fun node => match ns.expressions[node.index]? with
              | some { kind := .letDecl pattern _ _, .. } =>
                  (patternLocals ns (ns.patterns.size + 1) pattern).contains localId
              | _ => false)
          else
          let initializers := nodes.filterMap fun node => do
            let expression ← ns.expressions[node.index]?
            let .letDecl pattern (some value) _ := expression.kind | none
            let pat ← ns.patterns[pattern.index]?
            let .variable bound := pat.kind | none
            if bound == localId then some value else none
          match initializers.toList with
          | [initializer] => markedDictionary ns function nodes fuel initializer
          | _ => false

private structure Context where
  ns : ValidatedNamespace
  function : FunctionDecl FunctionBody
  nodes : Array ExprId
  tainted : Array LocalId
  /-- A direct callee's checked receiver slot; recursion consumes independent fuel. -/
  callSafe : QualifiedRef → Nat → Bool

private def Context.placeTainted (context : Context) (place : PlaceId) : Bool :=
  (placeRoot context.ns (context.ns.places.size + 1) place).elim true fun (localId, _) =>
    context.tainted.contains localId

private def Context.bearing (context : Context) (expression : Expr) : Bool :=
  referenceBearing context.ns (context.ns.tables.types.size + 1) expression.typeId

/-- Return dependency and rejection. Rejection includes writes into externally
owned storage, even if the call's result is unit or never used. -/
private abbrev FlowTable := Array (Option (Bool × Bool))

private def flowAt (flows : FlowTable) (id : ExprId) : Bool × Bool :=
  ((flows[id.index]?).getD none).getD (true, true)

/-- Cache one transfer result per expression for this fixed incoming taint set.
The pending entry rejects cycles; valid expression arenas are acyclic. -/
private def expressionFlow (context : Context) : Nat → ExprId → StateM FlowTable (Bool × Bool)
  | 0, _ => pure (true, true)
  | fuel + 1, id => do
      if let some found := ((← get)[id.index]?).getD none then return found
      let some expression := context.ns.expressions[id.index]? | return (true, true)
      modify (·.set! id.index (some (true, true)))
      let children ← match expression.kind with
        | .spec _ | .quantifier .. | .operation (.specification _) .. => pure #[]
        | kind => (expressionChildren kind).mapM (expressionFlow context fuel)
      let table ← get
      let flow := flowAt table
      let result := Id.run do
        let anyTaint := children.any (·.1)
        let anyBad := children.any (·.2)
        let keep := context.bearing expression && anyTaint
        match expression.kind with
        | .spec _ | .quantifier .. => return (false, false)
        | .value .. | .constant _ => return (false, false)
        | .localVar localId => return (context.tainted.contains localId, false)
        | .operation operation _ arguments _ =>
            let flows := children
            let taint := flows.any (·.1)
            let bad := flows.any (·.2)
            match operation with
            | .copy place | .move place | .read place =>
                return (context.bearing expression && context.placeTainted place,
                  bad || (placeRoot context.ns (context.ns.places.size + 1) place).isNone)
            | .readDiscriminant place => return (false, bad ||
                (placeRoot context.ns (context.ns.places.size + 1) place).isNone)
            | .borrow _ place => return (context.placeTainted place,
                bad || (placeRoot context.ns (context.ns.places.size + 1) place).isNone)
            | .write place =>
                let outside := (placeRoot context.ns (context.ns.places.size + 1) place).elim true (·.2)
                return (false, bad || (taint && outside) ||
                  (placeRoot context.ns (context.ns.places.size + 1) place).isNone)
            | .drop place => return (false, bad ||
                (placeRoot context.ns (context.ns.places.size + 1) place).isNone)
            | .reference .mutate => return (false, bad || (flows[1]?).any (·.1))
            | .reference _ | .primitive _ | .data _ => return (keep, bad)
            | .call (.constructor ..) | .call (.destructor ..) | .call (.closure ..) => return (keep, bad)
            | .call (.function reference) =>
                let escaped := (flows.zipIdx).any fun (value, index) =>
                  value.1 && !context.callSafe reference index
                return (false, bad || escaped)
            | .call .invoke =>
                let marked := (arguments[0]?).any
                  (markedDictionary context.ns context.function context.nodes
                    (context.ns.expressions.size + 1))
                let escaped := flows.zipIdx |>.any fun (value, index) =>
                  value.1 && !(marked && index == 1)
                return (false, bad || escaped)
            | .specification _ => return (false, false)
            | .assert => return (false, bad)
            | _ => return (keep, bad || taint)
        | .assign place value =>
            let outside := (placeRoot context.ns (context.ns.places.size + 1) place).elim true (·.2)
            return (false, anyBad || (outside && (flow value).1) ||
              (placeRoot context.ns (context.ns.places.size + 1) place).isNone)
        | .assignPattern _ _ => return (false, anyBad)
        | .return_ values => return (false, anyBad || values.any (fun value => (flow value).1))
        | .throw_ _ _ => return (false, anyBad || anyTaint)
        | .block _ result => return (result.any (fun value => (flow value).1), anyBad)
        | .letDecl _ _ body => return ((flow body).1, anyBad)
        | .ifElse _ yes no => return ((flow yes).1 || no.any (fun value => (flow value).1), anyBad)
        | .match_ _ arms => return (arms.any (fun arm => (flow arm.body).1), anyBad)
        | .loop _ _ => return (anyTaint, anyBad)
        | .break_ _ _ => return (anyTaint, anyBad || anyTaint)
        | .continue_ _ => return (false, false)
      modify (·.set! id.index (some result))
      return result

/-- A shared expression table for one fixed-point pass. Changing local taints
starts a fresh table; a result from an earlier pass is never reused. -/
private def expressionFlows (context : Context) : FlowTable :=
  let action : StateM FlowTable Unit := do
    for id in context.nodes do
      discard <| expressionFlow context (context.ns.expressions.size + 1) id
  action.run (Array.replicate context.ns.expressions.size none) |>.2

private def grow (context : Context) : Array LocalId :=
  let flows := expressionFlows context
  context.nodes.foldl (init := context.tainted) fun found id =>
    let add := fun (locals : Array LocalId) (value : ExprId) =>
      if (flowAt flows value).1 then
        locals.foldl (fun found localId => if found.contains localId then found else found.push localId) found
      else found
    match context.ns.expressions[id.index]? with
    | none => found
    | some expression => match expression.kind with
      | .letDecl pattern (some value) _ | .assignPattern pattern value =>
          add (patternLocals context.ns (context.ns.patterns.size + 1) pattern) value
      | .match_ value arms =>
          add (arms.flatMap fun arm => patternLocals context.ns (context.ns.patterns.size + 1) arm.pattern) value
      | .assign place value | .operation (.write place) _ #[value] _ =>
          match placeRoot context.ns (context.ns.places.size + 1) place with
          | some (localId, false) => add #[localId] value
          | _ => found
      | _ => found

private def settle : Nat → Context → Context
  | 0, context => context
  | fuel + 1, context =>
      let tainted := grow context
      if tainted == context.tainted then context else settle fuel { context with tainted }

/-- Analyze one receiver with already checked direct-callee summaries. -/
private def checksWith (ns : ValidatedNamespace) (function : FunctionDecl FunctionBody)
    (callSafe : QualifiedRef → Nat → Bool) (parameter : Nat) : Bool := Id.run do
  -- An executable intrinsic can replace a structured body with native
  -- semantics. A receipt for the source body says nothing about that override.
  if ns.intrinsics.any (fun intrinsic => intrinsic.executableBindings.any fun binding =>
      binding.target.namespaceId == ns.identity && binding.target.name == function.name) then
    return false
  let some formal := function.signature.parameters[parameter]? | return false
  let some (.reference _) := ns.tables.types[formal.typeUse.typeId.index]? | return false
  let .structured root := function.body | return false
  let nodes := bodyNodes ns (ns.expressions.size + 1) root
  let context : Context := { ns, function, nodes, tainted := #[⟨parameter⟩], callSafe }
  let context := settle (function.locals.size + 1) context
  let result := flowAt (expressionFlows context) root
  return !result.1 && !result.2

/-- Conditional receipt checker, exposed for focused analysis tests. Production
uses the memoized unit summary below instead of recursively repeating calls. -/
def checks (unit : ValidatedUnit) : Nat → ValidatedNamespace → FunctionDecl FunctionBody → Nat → Bool
  | 0, _, _, _ => false
  | fuel + 1, ns, function, parameter =>
      checksWith ns function (fun reference index =>
        (target? unit reference).any fun (targetNs, target) =>
          checks unit fuel targetNs target index) parameter

/-- One conditional receiver summary. Pending recursive dependencies have no
receipts; consequently recursion can never manufacture its own guarantee. -/
structure Summary where
  namespaceId : NamespaceId
  name : NameId
  parameters : Array Nat := #[]
  deriving Repr, BEq, Inhabited

abbrev Summaries := Array Summary

def summaryParameters (summaries : Summaries) (namespaceId : NamespaceId)
    (name : NameId) : Array Nat :=
  ((summaries.find? fun summary =>
    summary.namespaceId == namespaceId && summary.name == name).map (·.parameters)).getD #[]

private def summarize (unit : ValidatedUnit) : Nat → QualifiedRef → StateM Summaries Unit
  | 0, _ => pure ()
  | fuel + 1, reference => do
      if (← get).any (fun summary => summary.namespaceId == reference.namespaceId &&
          summary.name == reference.name) then return
      let some (ns, function) := target? unit reference | return
      let position := (← get).size
      modify (·.push { namespaceId := reference.namespaceId, name := reference.name })
      unless function.profile == .rust do return
      let .structured root := function.body | return
      unless function.signature.parameters.any (fun parameter =>
          match ns.tables.types[parameter.typeUse.typeId.index]? with
          | some (.reference _) => true
          | _ => false) do return
      let nodes := bodyNodes ns (ns.expressions.size + 1) root
      for id in nodes do
        match ns.expressions[id.index]? with
        | some { kind := .operation (.call (.function target)) _ _ _, .. } =>
            summarize unit fuel target
        | _ => pure ()
      let summaries ← get
      let parameters := (Array.range function.signature.parameters.size).filter
        (checksWith ns function fun target parameter =>
          (summaryParameters summaries target.namespaceId target.name).contains parameter)
      modify fun summaries => summaries.set! position {
        namespaceId := reference.namespaceId, name := reference.name, parameters }

/-- Compute conditional receipts once per function. A pending cycle is empty,
so tainted recursive edges are conservatively refused. Shared callees of a
call-graph diamond are analyzed once rather than once per path or literal. -/
def summaries (unit : ValidatedUnit) : Summaries := Id.run do
  let fuel := unit.namespaces.foldl (fun count ns => count + ns.functions.size) 1
  let action : StateM Summaries Unit := do
    for ns in unit.namespaces do
      for function in ns.functions do
        summarize unit fuel { namespaceId := ns.identity, name := function.name }
  return action.run #[] |>.2

/-- Focused accessor; validation computes `summaries` once for the whole unit. -/
def checkedParameters (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Array Nat :=
  summaryParameters (summaries unit) ns.identity function.name

/-- Whether an invocation's dictionary is an unmodified scoped parameter. -/
def scopedInvocation (ns : ValidatedNamespace) (function : FunctionDecl FunctionBody)
    (callee : ExprId) : Bool :=
  match function.body with
  | .absent => false
  | .structured root => markedDictionary ns function
      (bodyNodes ns (ns.expressions.size + 1) root) (ns.expressions.size + 1) callee

/-- An arena node can be shared: every owning body must justify the refinement. -/
def scopedInvocationAt (ns : ValidatedNamespace) (invocation callee : ExprId) : Bool :=
  let owners := ns.functions.filter fun function => match function.body with
    | .absent => false
    | .structured root => (bodyNodes ns (ns.expressions.size + 1) root).contains invocation
  !owners.isEmpty && owners.all (scopedInvocation ns · callee)

/-- Check a supplied literal or an explicitly scoped, unmodified dictionary.
Every conditional requirement of a literal target must be captured and supplied
by another checked dictionary. Unknown values and cyclic aliases fail closed. -/
def supplied (unit : ValidatedUnit) (summaries : Summaries) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Nat → ExprId → Bool
  | 0, _ => false
  | fuel + 1, id => Id.run do
      if scopedInvocation ns function id then return true
      let some expression := ns.expressions[id.index]? | return false
      match expression.kind with
      | .operation (.call (.closure reference mask)) _ captures _ =>
          let some (targetNs, target) := target? unit reference | return false
          let openParameters := (Array.range target.signature.parameters.size).filter
            fun index => !mask.testBit index
          let some receiver := openParameters[0]? | return false
          if !(summaryParameters summaries targetNs.identity target.name).contains receiver then return false
          let capturedParameters := (Array.range target.signature.parameters.size).filter
            fun index => mask.testBit index
          return (scopedMethodParameters target).all fun parameter =>
            match capturedParameters.findIdx? (· == parameter) with
            | none => false
            | some position => (captures[position]?).any (supplied unit summaries ns function fuel)
      | .localVar localId =>
          let .structured root := function.body | return false
          let nodes := bodyNodes ns (ns.expressions.size + 1) root
          if nodes.any (writtenLocal ns · localId) then return false
          let initializers := nodes.filterMap fun node => do
            let item ← ns.expressions[node.index]?
            let .letDecl pattern (some value) _ := item.kind | none
            let pat ← ns.patterns[pattern.index]?
            let .variable bound := pat.kind | none
            if bound == localId then some value else none
          match initializers.toList with
          | [initializer] => return supplied unit summaries ns function fuel initializer
          | _ => return false
      | .operation (.copy place) _ _ _ | .operation (.move place) _ _ _ |
          .operation (.read place) _ _ _ =>
          -- Resolve only a simple local through its unique immutable binding.
          let some (.localVar localId) := ns.places[place.index]? | return false
          let .structured root := function.body | return false
          let nodes := bodyNodes ns (ns.expressions.size + 1) root
          if nodes.any (writtenLocal ns · localId) then return false
          let initializers := nodes.filterMap fun node => do
            let item ← ns.expressions[node.index]?
            let .letDecl pattern (some value) _ := item.kind | none
            let pat ← ns.patterns[pattern.index]?
            let .variable bound := pat.kind | none
            if bound == localId then some value else none
          match initializers.toList with
          | [initializer] => return supplied unit summaries ns function fuel initializer
          | _ => return false
      | _ => return false

/-- Validate every construction/application supplying a scoped dictionary,
including captures. This runs for all bodies, not only annotated declarations. -/
def supplyDiagnostics (unit : ValidatedUnit) (summaries : Summaries) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Array Diagnostic :=
  match function.body with
  | .absent => #[]
  | .structured root =>
      (bodyNodes ns (ns.expressions.size + 1) root).filterMap fun id => do
        let expression ← ns.expressions[id.index]?
        let .operation (.call call) _ arguments _ := expression.kind | none
        let (reference, mask) ← match call with
          | .function reference => some (reference, none)
          | .closure reference mask => some (reference, some mask)
          | _ => none
        let (_, target) ← target? unit reference
        let positions := (Array.range target.signature.parameters.size).filter fun index =>
          mask.elim true (·.testBit index)
        let valid := (scopedMethodParameters target).all fun parameter =>
          match positions.findIdx? (· == parameter) with
          | none => false
          | some position => (arguments[position]?).any
              (supplied unit summaries ns function (ns.expressions.size + 1))
        if valid then none else some (.at "VIR-SCOPED-METHOD-EVIDENCE"
          "a scoped method dictionary requires a checked literal or an unmodified scoped dictionary parameter; scoped requirements of a closure target must be captured"
          expression.loc)

end ReceiverNoEscape
end VerexIR.Validation
