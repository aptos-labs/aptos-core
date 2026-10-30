-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Contract.Clauses

/-!
# Generated contracts from VIR specifications

A validated function's declared contract is translated into an ordinary Lean
`VerexIR.Proofs.FunctionContract` at command-elaboration time.  The
translation is deliberately shallow: a specification clause becomes a Lean
proposition over Lean-typed parameter and result binders, so a proof
obligation reads like the authored specification rather than like an
interpreter state.  The runtime value rows are tied to those binders by an
explicit shape equation.

The generator is untrusted glue: whatever it produces is a statement the
Lean kernel checks, and a clause it cannot translate is a reported error
rather than a silently weakened contract.

This module assembles contracts: slots, loop and data invariants, and the
contract of a function. The parts before it, split for build time:
`Contract.Values` (native types and values, maps and owners) and
`Contract.Clauses` (the clause translator).
-/

namespace VerexSL.Contract

open Lean Meta Elab Command
open VerexIR (ConstValue Operation PrimitiveOperation ExprId
  LocalId TypeId)
open VerexIR.Validation (ValidatedUnit ValidatedNamespace)
open VerexIR.Proofs (ObligationRange)
open VerexSL.Quote

/-- How one parameter reaches the function: by value, by shared borrow,
or by mutable borrow. -/
private inductive SlotKind where
  | plain
  | sharedRef
  | mutableRef
  deriving BEq

/-- One parameter or result: its type, a reference's referent (specifications
see through references, so the clause binders live at the borrowed type),
and the native type of the values it holds. -/
private structure Slot where
  name : Name
  kind : SlotKind := .plain
  physical : IrTy
  nty : VerexIR.Proofs.Denote.NTy

private def slotOf (context : Context) (name : Name) (typeId : TypeId)
    (allowReference : Bool := false) : MetaM Slot := do
  let some declared := context.unit.tables.types[typeId.index]?
    | throwError "slot type {typeId.index} is out of range"
  let (kind, physical, physicalId) ← match declared with
    | .reference reference => do
        unless allowReference do
          throwError "a result of reference type is not supported in generated contracts"
        let some referent := context.unit.tables.types[reference.referent.index]?
          | throwError "reference referent type {reference.referent.index} is out of range"
        let kind := match reference.kind with
          | .shared => SlotKind.sharedRef
          | .mutable => SlotKind.mutableRef
        pure (kind, referent, reference.referent)
    | declared => pure (SlotKind.plain, declared, typeId)
  return { name, kind, physical, nty := ← context.nativeType physicalId }

private def disjunction (parts : Array Lean.Expr) : MetaM Lean.Expr := do
  match parts.toList with
  | [] => return mkConst ``False
  | head :: tail =>
      tail.foldlM (fun accumulated part => mkAppM ``Or #[accumulated, part]) head

/-- The clause binders of one slot: its value, and for a mutable reference
its exit value, the prophecy, each with its native value.  The bare
parameter name denotes the exit in `ensures`, and `spec.old` reaches the
entry. -/
private structure SlotBinders where
  entry : Lean.Expr
  entryNative : Lean.Expr
  exit : Option Lean.Expr := none
  exitNative : Option Lean.Expr := none

/-- The binder a specification local denotes in the current state. -/
private def SlotBinders.current (binders : SlotBinders) : Lean.Expr :=
  binders.exit.getD binders.entry

/-- The native value of the current state's binder. -/
private def SlotBinders.currentNative (binders : SlotBinders) : Lean.Expr :=
  binders.exitNative.getD binders.entryNative

/-- Existentially close a body over the logical slots. -/
private def existsOver (binders : Array Lean.Expr) (body : Lean.Expr) :
    MetaM Lean.Expr := do
  binders.foldrM (fun binder accumulated => do
    mkAppM ``Exists #[← mkLambdaFVars #[binder] accumulated]) body

/-! ## Native loop invariants -/

/-- Find source-normalized `spec invariant; loop` pairs.  The expression id
identifies the generated predicate over the loop's typed header product. -/
partial def loopSpecifications (ns : ValidatedNamespace)
    (root : ExprId) (seen : Array Nat := #[]) :
    Array (ExprId × VerexIR.SpecBlock) := Id.run do
  if seen.contains root.index then return #[]
  let some expression := ns.expressions[root.index]? | return #[]
  let seen := seen.push root.index
  let mut found := #[]
  -- A while annotation is carried by its condition, not a preceding
  -- statement. Discover that representation without rewriting the loop.
  if let .loop _ body := expression.kind then
    if let some { kind := .ifElse condition _ _, .. } := ns.expressions[body.index]? then
      if let some { kind := .block statements _, .. } := ns.expressions[condition.index]? then
        for statement in statements do
          if let some { kind := .spec block, .. } := ns.expressions[statement.index]? then
            if block.conditions.any (·.kind == .loopInvariant) then
              found := found.push (root, block)
    -- So is an annotation opening the loop's body, at the head of every
    -- iteration.
    if let some { kind := .block statements _, .. } := ns.expressions[body.index]? then
      if let some (first : ExprId) := statements[0]? then
        if let some { kind := .spec block, .. } := ns.expressions[first.index]? then
          if block.conditions.any (·.kind == .loopInvariant) then
            found := found.push (root, block)
  for child in VerexIR.Validation.expressionChildren expression.kind do
    found := found ++ loopSpecifications ns child seen
  -- Unannotated loops still need a typed header invariant. Authored clauses
  -- strengthen that invariant, but are not required for type/range facts.
  if let .loop .. := expression.kind then
    unless found.any (fun (site, _) => site == root) do
      found := found.push (root, { loc := expression.loc })
  -- Several annotations can name one site: the blocks its heading's
  -- condition holds and the one its body begins with. The site's
  -- specification is all of their conditions, in order; the default,
  -- discovered after them, adds none.
  found := found.foldl (fun acc (site, block) =>
    match acc.findIdx? (·.1 == site) with
    | some index => acc.modify index fun (site, previous) =>
        (site, { previous with
          pragmas := previous.pragmas ++ block.pragmas
          conditions := previous.conditions ++ block.conditions
          frame := previous.frame <|> block.frame })
    | none => acc.push (site, block)) #[]
  return found

/-- A condition's clause marked with the condition's range, so that a
failure inside it is reported at the condition. -/
def markCondition (unit : ValidatedUnit) (condition : VerexIR.Condition)
    (clause : Lean.Expr) : Lean.Expr :=
  markObligation (conditionRange unit condition) clause

/-- Locals lexically available at a loop header. Body-local slots exist in
the runtime row but are not initialized on the first visit. -/
partial def loopHeaderLocals (ns : ValidatedNamespace) (root target : ExprId)
    (available : Array LocalId) : Option (Array LocalId) := do
  if root == target then return available
  let expression ← ns.expressions[root.index]?
  if let .letDecl pattern value body := expression.kind then
    if let some value := value then
      if let some found := loopHeaderLocals ns value target available then return found
    let rec boundLocals (pattern : VerexIR.PatternId) : Array LocalId :=
      match ns.patterns[pattern.index]? with
      | some { kind := .variable localId, .. } => #[localId]
      | some { kind := .tuple children, .. }
      | some { kind := .constructor _ _ _ children, .. } => children.flatMap boundLocals
      | _ => #[]
    loopHeaderLocals ns body target (available ++ boundLocals pattern)
  else
    (VerexIR.Validation.expressionChildren expression.kind).findSome?
      fun child => loopHeaderLocals ns child target available

/-- Every erased in-body specification must be accounted for. Until inline
assertions/assumptions have native obligations, only linked loop annotations
may accompany a generated loop computation. -/
private partial def checkLoopAnnotations (ns : ValidatedNamespace) (root : ExprId)
    (specifications : Array (ExprId × VerexIR.SpecBlock)) : MetaM Unit := do
  let some expression := ns.expressions[root.index]? | return
  if let .spec block := expression.kind then
    unless block.frame.isNone && block.pragmas.isEmpty &&
        block.conditions.all (·.kind == .loopInvariant) do
      throwError "native loops do not yet translate inline specification obligations"
    unless block.conditions.isEmpty || specifications.any (·.2 == block) do
      throwError "native loop invariant has no consuming loop"
  for child in VerexIR.Validation.expressionChildren expression.kind do
    checkLoopAnnotations ns child specifications

private structure AbortClause where
  condition : ExprId
  code : Option ExprId := none
  range : ObligationRange := {}

/-- A contract's `let`: the local it binds, its value, and whether that is
read in the post-state. -/
private structure ContractLet where
  name : String
  value : ExprId
  post : Bool

private structure ClauseGroups where
  requires : Array (ExprId × ObligationRange) := #[]
  ensures : Array (ExprId × ObligationRange) := #[]
  abortsIf : Array AbortClause := #[]
  abortsWith : Array (ExprId × ObligationRange) := #[]
  lets : Array ContractLet := #[]

/-- One total resource read in a module invariant.  `old` selects whether its
implicit existence guard observes the entry or current state. -/
private structure InvariantValueRead where
  typeIndex : Nat
  key : ExprId
  atEntry : Bool := false
  deriving BEq

private structure ModifiedResource where
  typeIndex : Nat
  key : ExprId

/-- Resource reads whose totalized values occur in an invariant.  Existence
tests are deliberately absent: unlike a value read, they do not need an
implicit guard. -/
private partial def invariantValueReads (ns : ValidatedNamespace) (root : ExprId)
    (inOld : Bool := false) : Array InvariantValueRead :=
  match ns.expressions[root.index]? with
  | none => #[]
  | some expression =>
      match expression.kind with
      | .operation (.specification .old) _ arguments _ =>
          arguments.flatMap fun child => invariantValueReads ns child true
      | .operation (.specification (.global _)) instantiations arguments _ =>
          let read := match instantiations.toList, arguments[0]? with
            | [.typeArg resource], some key =>
                #[{ typeIndex := resource.typeId.index, key, atEntry := inOld }]
            | _, _ => #[]
          read ++ arguments.flatMap fun child => invariantValueReads ns child inOld
      | kind =>
          (VerexIR.Validation.expressionChildren kind).flatMap fun child =>
            invariantValueReads ns child inOld

/-- Whether the lowered contract sets a boolean pragma.  Lowering already
merged the namespace's pragmas into the contract, contract-first, so the
declared attribute list is authoritative. -/
private def pragmaEnabled (contract : VerexIR.FunctionContract) (name : String) : Bool :=
  contract.pragmas.any fun pragma =>
    match pragma with
    | .assign pragmaName (.constant (.bool true)) _ => pragmaName == name
    | _ => false

/-- The expressions a function's verification reads: the conditions of its
contract and its body. -/
private def functionRoots (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Array ExprId :=
  let roots := declaration.contract.conditions.flatMap fun condition =>
    #[condition.expression] ++ condition.auxiliary.map (·.2)
  match declaration.body with
  | .structured root => roots.push root
  | _ => roots

/-- Fold `visit` over the operations the expressions `roots` reach: in
them, in the specification functions they expand, and in the functions they
call, at any depth. `false` with the fold when the walk exceeds its bound. -/
private def reachFold {α : Type} (unit : ValidatedUnit) (roots : List (VerexIR.NamespaceId × ExprId))
    (init : α) (visit : α → VerexIR.Operation → Array VerexIR.GenericArgument → α) :
    α × Bool := Id.run do
  let mut work : List (VerexIR.NamespaceId × ExprId) := roots
  let mut visited : Std.HashSet (Nat × Nat) := {}
  let mut folded := init
  let bound := unit.namespaces.foldl (fun total ns => total + ns.expressions.size) 1
  for _ in [0:bound] do
    match work with
    | [] => break
    | (owner, id) :: rest =>
        work := rest
        if visited.contains (owner.index, id.index) then continue
        visited := visited.insert (owner.index, id.index)
        let some ownerNs := unit.namespaces[owner.index]? | continue
        let some expression := ownerNs.expressions[id.index]? | continue
        if let .operation operation instantiations _ _ := expression.kind then
          folded := visit folded operation instantiations
          match operation with
          | .specification (.functionCall reference _) =>
              let body? := do
                let targetNs ← unit.namespaces[reference.namespaceId.index]?
                let functionId ← unit.resolution.specFunction? reference.name
                let callee ← targetNs.specFunctions[functionId.index]?
                callee.body
              if let some body := body? then work := (reference.namespaceId, body) :: work
          | .call (.function reference) =>
              if let some calleeNs := unit.namespaces[reference.namespaceId.index]? then
                if let some functionId := unit.resolution.function? reference.name then
                  if let some callee := calleeNs.functions[functionId.index]? then
                    work := (functionRoots callee).toList.map (reference.namespaceId, ·) ++ work
          | _ => pure ()
        work := (VerexIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·) ++ work
  return (folded, work.isEmpty)

/-- Whether a type mentions no type parameter. -/
private def closedType (unit : ValidatedUnit) (typeId : TypeId) : Bool :=
  go typeId 32
where
  go (typeId : TypeId) : Nat → Bool
    | 0 => false
    | fuel + 1 => match unit.tables.types[typeId.index]? with
      | some (.typeParameter _) | none => false
      | some (.vector element _) | some (.typeDomain element) => go element fuel
      | some (.reference reference) => go reference.referent fuel
      | some (.tuple elements) => elements.all (go · fuel)
      | some (.function arguments result _) => arguments.all (go · fuel) && go result fuel
      | some (.nominal _ arguments) => arguments.all fun
          | .typeArg typeUse => go typeUse.typeId fuel
          | _ => true
      | some _ => true

/-- What a function's verification uses in its frame: the specification
functions it applies, each at its type arguments, the resource types of the
memory it reads or writes, and of the memory it writes. -/
structure FrameUses where
  applied : Array (VerexIR.QualifiedRef × Array TypeId)
  memory : Array TypeId
  written : Array TypeId

/-- The specification functions a function's verification applies, each at
its type arguments in the function's frame, and the resource types of the
memory it reads or writes, and of the memory it writes: in the function's
body and contract, in the contracts of the functions it calls, and in the
specification functions these expand. A callee's or an expansion's type
parameter is the argument its application gives it; an application at a
type that mentions another parameter is left out. -/
def frameUses (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : FrameUses := Id.run do
  -- Each expression with the arguments of its owner's type parameters, none
  -- in the function itself, whose parameters are its own.
  let mut work : List (VerexIR.NamespaceId × ExprId × Option (Array (Option TypeId))) :=
    (functionRoots declaration).toList.map (namespaceId, ·, none)
  let mut visited : Std.HashSet (Nat × Nat × List (Option Nat)) := {}
  let mut applied : Array (VerexIR.QualifiedRef × Array TypeId) := #[]
  let mut memory : Array TypeId := #[]
  let mut written : Array TypeId := #[]
  let bound := unit.namespaces.foldl (fun total ns => total + ns.expressions.size) 1
  for _ in [0:bound] do
    match work with
    | [] => break
    | (owner, id, arguments?) :: rest =>
        work := rest
        let key := (owner.index, id.index,
          (arguments?.getD #[]).toList.map (·.map (·.index)))
        if visited.contains key then continue
        visited := visited.insert key
        let some ownerNs := unit.namespaces[owner.index]? | continue
        let some expression := ownerNs.expressions[id.index]? | continue
        let inFrame (typeId : TypeId) : Option TypeId := match arguments? with
          | none => some typeId
          | some arguments => match unit.tables.types[typeId.index]? with
            | some (.typeParameter index) => arguments[index]?.join
            | _ => if closedType unit typeId then some typeId else none
        if let .operation operation instantiations _ _ := expression.kind then
          let inner := instantiations.map fun
            | .typeArg typeUse => inFrame typeUse.typeId
            | _ => none
          match operation with
          | .global _ | .specification (.global _) | .specification (.publish _) |
              .specification (.remove _) | .specification (.update _) =>
              if let some (some resource) := inner[0]? then
                unless memory.contains resource do memory := memory.push resource
                if operation matches .global (.borrow .mutable) | .global .take |
                    .global .publish | .specification (.publish _) |
                    .specification (.remove _) | .specification (.update _) then
                  unless written.contains resource do written := written.push resource
          | .specification (.functionCall reference _) =>
              if let some typeArguments := inner.mapM (fun argument => argument) then
                unless applied.contains (reference, typeArguments) do
                  applied := applied.push (reference, typeArguments)
              let body? := do
                let targetNs ← unit.namespaces[reference.namespaceId.index]?
                let functionId ← unit.resolution.specFunction? reference.name
                (← targetNs.specFunctions[functionId.index]?).body
              if let some body := body? then
                work := (reference.namespaceId, body, some inner) :: work
          | .call (.function reference) =>
              -- A callee's contract, not its body, which its own verification reads.
              if arguments?.isNone then
                if let some calleeNs := unit.namespaces[reference.namespaceId.index]? then
                  if let some functionId := unit.resolution.function? reference.name then
                    if let some callee := calleeNs.functions[functionId.index]? then
                      let conditions := callee.contract.conditions.flatMap fun condition =>
                        #[condition.expression] ++ condition.auxiliary.map (·.2)
                      work := (conditions ++ callee.contract.modifies).toList.map
                        (reference.namespaceId, ·, some inner) ++ work
                      -- The memory a callee's frame names is written.
                      for target in callee.contract.modifies do
                        let some { kind := .operation _ targetInstantiations _ _, .. } :=
                            calleeNs.expressions[target.index]? | continue
                        let some (VerexIR.GenericArgument.typeArg resource) :=
                            targetInstantiations[0]? | continue
                        let resource? := match unit.tables.types[resource.typeId.index]? with
                          | some (.typeParameter index) => inner[index]?.join
                          | _ => if closedType unit resource.typeId then some resource.typeId
                              else none
                        if let some resource := resource? then
                          unless written.contains resource do written := written.push resource
          | _ => pure ()
        work := (VerexIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·, arguments?) ++ work
  return { applied, memory, written }

/-- The instantiations a generic axiom is assumed at: those that apply a
specification function the axiom applies at a type argument list
`applied` holds, as the Move Prover instantiates an axiom at the
instantiations a verification uses. -/
private def axiomInstances (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.NamespaceInvariant) (parameters : Nat)
    (applied : Array (VerexIR.QualifiedRef × Array TypeId)) : Array (Array TypeId) := Id.run do
  let mut own : Array (VerexIR.QualifiedRef × Array TypeId) := #[]
  let mut work := [declaration.condition.expression]
  for _ in [0:ns.expressions.size + 1] do
    match work with
    | [] => break
    | id :: rest =>
        work := rest
        let some expression := ns.expressions[id.index]? | continue
        if let .operation (.specification (.functionCall reference _)) instantiations _ _ :=
            expression.kind then
          if let some arguments := instantiations.mapM (fun
              | .typeArg typeUse => some typeUse.typeId
              | _ => none) then
            own := own.push (reference, arguments)
        work := (VerexIR.Validation.expressionChildren expression.kind).toList ++ work
  let mut instances : Array (Array TypeId) := #[]
  for (reference, pattern) in own do
    for (target, arguments) in applied do
      unless target == reference && arguments.size == pattern.size do continue
      let mut binding : Array (Option TypeId) := Array.replicate parameters none
      let mut matched := true
      for (formal, actual) in pattern.zip arguments do
        match unit.tables.types[formal.index]? with
        | some (.typeParameter index) =>
            match binding[index]? with
            | some none => binding := binding.set! index (some actual)
            | some (some bound) => if bound != actual then matched := false
            | none => matched := false
        | _ => if formal != actual then matched := false
      if matched then
        if let some found := binding.mapM (fun argument => argument) then
          unless instances.contains found do instances := instances.push found
  return instances

/-- A type read inside a generic invariant: of the invariant itself (`none`),
or of a specification function it applies, whose type parameters are the
types of the application (`some`), in the scope of the application. -/
private inductive ScopedType where
  | mk (typeId : TypeId) (scope : Option (Array ScopedType))
  deriving Inhabited, BEq

/-- Unify a type of a generic invariant, whose type parameters `binding`
binds, with a type in a function's frame: an invariant's parameter binds to
the type it meets, and a type parameter of the function may be any type, so
it meets every type. `none` where the two cannot be one type. -/
private partial def unifyInstance (unit : ValidatedUnit) (pattern : ScopedType) (actual : TypeId)
    (binding : Array (Option TypeId)) (fuel : Nat := 32) : Option (Array (Option TypeId)) :=
  match fuel with
  | 0 => none
  | fuel + 1 =>
    let .mk patternId scope := pattern
    let within (typeId : TypeId) : ScopedType := .mk typeId scope
    match unit.tables.types[patternId.index]?, unit.tables.types[actual.index]? with
    | some (.typeParameter index), _ => match scope with
        -- A specification function's parameter is its application's type.
        | some arguments => match arguments[index]? with
            | some argument => unifyInstance unit argument actual binding fuel
            | none => none
        | none => match binding[index]? with
            | some none => some (binding.set! index (some actual))
            | some (some bound) => if bound == actual then some binding else none
            | none => none
    | _, some (.typeParameter _) => some binding
    | some (.nominal handle arguments), some (.nominal handle' arguments') =>
        if handle != handle' || arguments.size != arguments'.size then none else
        (arguments.zip arguments').foldlM (init := binding) fun binding pair => match pair with
          | (.typeArg left, .typeArg right) =>
              unifyInstance unit (within left.typeId) right.typeId binding fuel
          | _ => some binding
    | some (.vector element _), some (.vector element' _) =>
        unifyInstance unit (within element) element' binding fuel
    | _, _ => if patternId == actual && scope.isNone then some binding else none

/-- The instantiations a generic invariant holds at in a function: those
that make a resource type it reads, itself or through the specification
functions it applies, one of the resource types of the memory the function
uses (`memory`), as the Move Prover instantiates its global invariants by
unifying their memory with a function's. A type parameter the memory leaves
undetermined is `none`. -/
private def invariantInstances (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.NamespaceInvariant) (parameters : Nat)
    (memory : Array TypeId) : Array (Array (Option TypeId)) := Id.run do
  let mut patterns : Array ScopedType := #[]
  -- Each expression with its namespace and the scope of its types.
  let mut work : List (VerexIR.NamespaceId × ExprId × Option (Array ScopedType)) :=
    [(⟨0⟩, declaration.condition.expression, none)]
  let ownNamespace := ns
  -- The applications walked, by function and type arguments.
  let mut applied : Array (VerexIR.QualifiedRef × Array TypeId) := #[]
  let bound := unit.namespaces.foldl (fun total ns => total + ns.expressions.size) 1
  for _ in [0:bound] do
    match work with
    | [] => break
    | (owner, id, scope) :: rest =>
        work := rest
        let ownerNs := if scope.isNone then ownNamespace
          else (unit.namespaces[owner.index]?).getD ownNamespace
        let some expression := ownerNs.expressions[id.index]? | continue
        if let .operation operation instantiations _ _ := expression.kind then
          if operation matches .global _ | .specification (.global _) |
              .specification (.publish _) | .specification (.remove _) |
              .specification (.update _) then
            if let some (VerexIR.GenericArgument.typeArg resource) := instantiations[0]? then
              let pattern := ScopedType.mk resource.typeId scope
              unless patterns.contains pattern do patterns := patterns.push pattern
          -- A specification function's body reads at its application's types.
          if let .specification (.functionCall reference _) := operation then
            let body? := do
              let targetNs ← unit.namespaces[reference.namespaceId.index]?
              let functionId ← unit.resolution.specFunction? reference.name
              (← targetNs.specFunctions[functionId.index]?).body
            let key := (reference, instantiations.filterMap fun
              | .typeArg typeUse => some typeUse.typeId
              | _ => none)
            if let some body := body? then
              -- An application walked before, a recursive one among them,
              -- adds no memory.
              unless applied.contains key do
                applied := applied.push key
                let arguments := instantiations.filterMap fun
                  | .typeArg typeUse => some (ScopedType.mk typeUse.typeId scope)
                  | _ => none
                work := (reference.namespaceId, body, some arguments) :: work
        work := (VerexIR.Validation.expressionChildren expression.kind).toList.map
          (owner, ·, scope) ++ work
  let mut instances : Array (Array (Option TypeId)) := #[]
  for pattern in patterns do
    for actual in memory do
      let some binding := unifyInstance unit pattern actual (Array.replicate parameters none)
        | continue
      unless instances.contains binding do instances := instances.push binding
  return instances

/-- A type argument of an invariant's instance: a type of the function's
frame, or a ghost type parameter of the function, which the Move Prover adds
for a parameter the memory leaves undetermined. -/
private inductive InstanceArgument where
  | type (typeId : TypeId)
  | ghost (index : Nat)
  deriving BEq

/-- The type parameters of a function, a generic invariant's ghost
parameters following them. -/
def typeArity (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Nat :=
  (declaration.signature.generics.filter (·.kind == .typeArg)).size

/-- The ghost type parameter of the generic invariant at `position` among
the unit's, for its parameter `index`, past the function's own (`base`). -/
private def ghostIndex (base position index : Nat) : Nat :=
  base + position * 16 + index

/-- Whether a function's verification meets a generic invariant at an
instance its memory leaves partly undetermined: it is proved at a ghost type
parameter, over every frame, so that the ghost ranges over every type. -/
def hasGhostInvariantInstances (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  let memory := (frameUses unit namespaceId declaration).written
  !memory.isEmpty && unit.namespaces.any fun ns => ns.invariants.any fun invariant =>
    match invariant.condition.kind with
    | .globalInvariant parameters | .globalInvariantUpdate parameters =>
        !parameters.isEmpty &&
          (invariantInstances unit ns invariant parameters.size memory).any (·.any Option.isNone)
    | _ => false

/-- The resource declarations what the expressions `roots` reach reaches in
global memory, or with `writes` writes, `none` when it reaches none. Past the
walk's bound, memory is conservatively reachable. -/
private def reachFrom (unit : ValidatedUnit) (roots : List (VerexIR.NamespaceId × ExprId))
    (writes : Bool := false) : Option (Array VerexIR.StructHandle) :=
  let ((reached, resources), complete) := reachFold unit roots (false, #[])
    fun (reached, resources) operation instantiations =>
      let counts := match operation with
        | .global (.borrow .mutable) | .global .take | .global .publish => true
        | .global _ | .specification (.global _) | .specification (.publish _) |
            .specification (.remove _) | .specification (.update _) => !writes
        | _ => false
      if !counts then (reached, resources) else
        (true, instantiations.foldl (init := resources) fun resources instantiation =>
          match instantiation with
          | .typeArg typeUse =>
              match unit.tables.types[typeUse.typeId.index]?.bind (nominalDeclaration? unit) with
              | some (handle, _, _) =>
                  if resources.contains handle then resources else resources.push handle
              | none => resources
          | _ => resources)
  if reached || !complete then some resources else none

/-- The resource declarations a function can reach in global memory:
through a global operation or a storage clause in its body or contract, in a
specification function one expands, or in a function it calls, at any
depth. `none` when it reaches no global memory. -/
def memoryReach (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Option (Array VerexIR.StructHandle) :=
  reachFrom unit ((functionRoots declaration).toList.map (namespaceId, ·))

/-- The qualified name of a function or specification function. -/
private def qualifiedName? (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    Option String := do
  let path ← unit.tables.namespaces[reference.namespaceId.index]?
  let qualified ← unit.tables.names[reference.name.index]?
  some ("::".intercalate (path.segments.toList ++ [qualified.name]))

/-- The functions of `names` a function applies, in code or in a
specification, at any depth (`reachFold`); past the walk's bound, all of
them. -/
private def reachedFunctions (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (names : List String) : List String :=
  if names.isEmpty then [] else
  let (reached, complete) :=
    reachFold unit ((functionRoots declaration).toList.map (namespaceId, ·)) []
      fun reached operation _ => match operation with
        | .call (.function reference) | .specification (.functionCall reference _) =>
            match qualifiedName? unit reference with
            | some name => if names.contains name && !reached.contains name then name :: reached
                else reached
            | none => reached
        | _ => reached
  if complete then names.filter reached.contains else names

/-- The implicit existence premise for one totalized invariant read. -/
private def invariantReadGuard (context : Context) (read : InvariantValueRead) :
    MetaM Lean.Expr := do
  let readContext := if read.atEntry then
      { context with locals := context.oldLocals, state := context.oldState }
    else context
  let some state := readContext.state
    | throwError "a namespace invariant resource read has no state"
  let key := mkApp (mkConst ``VerexIR.StorageKey.address) (← translate readContext read.key)
  let (slot, _, _) ← context.slot state ⟨read.typeIndex⟩ key
  slotPresent slot

/-- Translate the body at one fixed quantifier assignment and add Move's
implicit guard: a value-based invariant constrains an address only while all
resources it reads exist in the selected pre/post states. -/
private def guardedInvariantBody (context : Context) (root body : ExprId)
    (condition : Option ExprId) : MetaM Lean.Expr := do
  let proposition ← translate context body
  let mut guards := #[]
  if let some condition := condition then
    guards := guards.push (← translate context condition)
  let reads := (invariantValueReads context.ns root).foldl
    (fun unique read => if unique.contains read then unique else unique.push read) #[]
  for read in reads do
    guards := guards.push (← invariantReadGuard context read)
  if guards.isEmpty then return proposition
  mkArrow (← conjunction guards) proposition

private inductive InvariantPhase where
  | entry
  | exit
  deriving BEq

/-- Translate this namespace's invariants in the state selected by `context`.
Invariant locals are declaration-owned, so their quantifier binders start
unbound even when the surrounding function has locals with the same IDs.
Update invariants are obligations only at exit; regular invariants are also
assumptions at entry; axioms are assumptions at entry and never obligations,
a generic one at each instantiation of `applied` it applies
(`axiomInstances`). -/
private def namespaceInvariantTerms (context : Context)
    (modifiedResources : Option (Array ModifiedResource)) (phase : InvariantPhase)
    (relevant : VerexIR.NamespaceId → VerexIR.NamespaceInvariant → Bool := fun _ _ => true)
    (applied : Array (VerexIR.QualifiedRef × Array TypeId) := #[])
    (memory : Array TypeId := #[]) (ghostBase : Option Nat := none)
    (ghostMemory : Option (Array TypeId) := none) :
    MetaM (Array (Lean.Expr × ObligationRange)) := do
  let mut terms := #[]
  -- The invariants of every namespace of the unit, each read in its own.
  let invariants := (context.unit.namespaces.toList.zipIdx).flatMap fun (ns, index) =>
    ns.invariants.toList.map fun invariant => ((⟨index⟩ : VerexIR.NamespaceId), ns, invariant)
  let invariants := invariants.zipIdx.flatMap fun ((namespaceId, ns, invariant), position) =>
    match invariant.condition.kind with
    | .axiom_ typeParameters =>
        if typeParameters.isEmpty then [(namespaceId, ns, invariant, #[])] else
          (axiomInstances context.unit ns invariant typeParameters.size applied).toList.map
            fun arguments => (namespaceId, ns, invariant, arguments.map InstanceArgument.type)
    | .globalInvariant typeParameters | .globalInvariantUpdate typeParameters =>
        if typeParameters.isEmpty then [(namespaceId, ns, invariant, #[])] else
          -- An instance the memory leaves undetermined is the written
          -- memory's alone, as the Move Prover ignores one of memory a
          -- function only reads.
          let determined := (invariantInstances context.unit ns invariant typeParameters.size
            memory).filter (·.all Option.isSome)
          let undetermined := (invariantInstances context.unit ns invariant typeParameters.size
            (ghostMemory.getD memory)).filter (·.any Option.isNone)
          (determined ++ undetermined).toList.filterMap
            fun binding =>
              -- A parameter the memory leaves undetermined is a ghost type
              -- parameter of the function, where it has room for one.
              let arguments := binding.mapIdx fun index argument => match argument, ghostBase with
                | some typeId, _ => some (InstanceArgument.type typeId)
                | none, some base => some (.ghost (ghostIndex base position index))
                | none, none => none
              (arguments.mapM id).map fun arguments => (namespaceId, ns, invariant, arguments)
    | _ => [(namespaceId, ns, invariant, #[])]
  for (invariantNamespaceId, invariantNs, declaration, axiomArguments) in invariants do
    unless relevant invariantNamespaceId declaration do continue
    let (isUpdate, isAxiom) ← match declaration.condition.kind with
      | .globalInvariant _ => pure (false, false)
      | .globalInvariantUpdate _ => pure (true, false)
      | .axiom_ _ => pure (false, true)
      | kind => throwError "namespace condition {repr kind} is not a global invariant"
    if phase == .entry && isUpdate then continue
    if phase == .exit && isAxiom then continue
    unless declaration.condition.auxiliary.isEmpty do
      throwError "namespace invariants cannot carry auxiliary expressions"
    let localTypes ← declaration.locals.mapM fun localDecl => do
      let some ty := context.unit.tables.types[localDecl.type.typeId.index]?
        | throwError "namespace invariant local type {localDecl.type.typeId.index} is out of range"
      pure ty
    let emptyLocals := Array.replicate declaration.locals.size none
    -- A generic declaration's type parameters read as its instance's native
    -- types, in the function's frame.
    let invariantContext := { context with
      namespaceId := invariantNamespaceId
      ns := invariantNs
      locals := emptyLocals
      localTypes
      oldLocals := emptyLocals
      results := #[]
      resultTypes := #[]
      typeArgumentTypes := if axiomArguments.isEmpty then context.typeArgumentTypes
        else axiomArguments.map fun
          | .type typeId => context.ntyOf? typeId
          | .ghost index => some (.param index) }
    let range := conditionRange context.unit declaration.condition
    -- The declarations the invariant reads, against those a write modifies.
    let handleOf (typeIndex : Nat) : Option VerexIR.StructHandle := do
      let ty ← context.unit.tables.types[typeIndex]?
      let (handle, _, _) ← nominalDeclaration? context.unit ty
      some handle
    let invariantFamilies := (reachFrom context.unit
      [(invariantNamespaceId, declaration.condition.expression)]).getD #[]
    -- An axiom is not re-established at the modified keys: it is assumed
    -- as stated.
    let modifiedKeys := if isAxiom then none else modifiedResources.map fun resources =>
      resources.filterMap fun resource =>
        if (handleOf resource.typeIndex).any invariantFamilies.contains then some resource.key
        else none
    match modifiedKeys with
    | none =>
        let some root := invariantNs.expressions[declaration.condition.expression.index]?
          | throwError "namespace invariant expression is out of range"
        -- An axiom quantifies as any specification does, over its binders'
        -- domains at its instance's types.
        if isAxiom then
          let proposition ← guardedInvariantBody invariantContext
            declaration.condition.expression declaration.condition.expression none
          terms := terms.push (proposition, range)
          continue
        match root.kind with
        | .quantifier .forall #[binder] triggers condition body =>
            unless triggers.isEmpty do
              throwError "namespace invariant triggers are not supported in generated contracts"
            let some pattern := invariantNs.patterns[binder.pattern.index]?
              | throwError "namespace invariant quantifier pattern is out of range"
            let .variable localId := pattern.kind
              | throwError "namespace invariants require a variable binder"
            let some patternType := context.unit.tables.types[pattern.typeId.index]?
              | throwError "namespace invariant binder type is out of range"
            let proposition ← withLocalDeclD (Name.mkSimple s!"invariant_{localId.index}")
                (← invariantContext.binderType pattern.typeId) fun value => do
              let active := { invariantContext with
                locals := invariantContext.locals.set! localId.index (some value)
                oldLocals := invariantContext.oldLocals.set! localId.index (some value) }
              let bodyProposition ← guardedInvariantBody active
                declaration.condition.expression body condition
              mkForallFVars #[value] bodyProposition
            terms := terms.push (proposition, range)
        | _ =>
            let proposition ← guardedInvariantBody invariantContext
              declaration.condition.expression declaration.condition.expression none
            terms := terms.push (proposition, range)
    | some keys =>
        if keys.isEmpty then continue
        let some root := invariantNs.expressions[declaration.condition.expression.index]?
          | throwError "namespace invariant expression is out of range"
        match root.kind with
        | .quantifier .forall #[binder] triggers condition body =>
            unless triggers.isEmpty do
              throwError "namespace invariant triggers are not supported in generated contracts"
            let some pattern := invariantNs.patterns[binder.pattern.index]?
              | throwError "namespace invariant quantifier pattern is out of range"
            let .variable localId := pattern.kind
              | throwError "modified-key invariant specialization requires a variable binder"
            let mut translatedKeys := #[]
            for key in keys do
              let key ← translate context key
              if translatedKeys.any (· == key) then continue
              translatedKeys := translatedKeys.push key
              let active := { invariantContext with
                locals := invariantContext.locals.set! localId.index (some key)
                oldLocals := invariantContext.oldLocals.set! localId.index (some key) }
              let proposition ← guardedInvariantBody active
                declaration.condition.expression body condition
              terms := terms.push (proposition, range)
        | _ =>
            let proposition ← guardedInvariantBody invariantContext
              declaration.condition.expression declaration.condition.expression none
            terms := terms.push (proposition, range)
  pure terms

/-- Source-key expressions named by the function's frame. Specializing a
universal invariant at precisely these keys is sufficient together with the
generated frame, and avoids duplicating a quantified final-state expression. -/
private def invariantModifiedResources (ns : ValidatedNamespace)
    (contract : VerexIR.FunctionContract) : MetaM (Option (Array ModifiedResource)) := do
  if contract.modifiesAll || hasLooseFrame contract then return none
  let mut resources := #[]
  for modified in contract.modifies do
    let some expression := ns.expressions[modified.index]?
      | throwError "a modifies expression is out of range"
    let .operation (.specification (.global _)) instantiations arguments _ := expression.kind
      | throwError "a modifies clause must name a resource at a key"
    let [.typeArg resource] := instantiations.toList
      | throwError "a modifies clause must name one resource type"
    let some key := arguments[0]?
      | throwError "a modifies clause expects one key"
    resources := resources.push { typeIndex := resource.typeId.index, key }
  pure (some resources)

private def groupConditions (unit : ValidatedUnit)
    (conditions : Array VerexIR.Condition) : MetaM ClauseGroups := do
  let mut groups : ClauseGroups := {}
  for condition in conditions do
    let range := conditionRange unit condition
    match condition.kind with
    | .requires => groups := { groups with
        requires := groups.requires.push (condition.expression, range) }
    | .ensures => groups := { groups with
        ensures := groups.ensures.push (condition.expression, range) }
    | .abortsIf =>
        let code := condition.auxiliary.findSome? fun (key, value) =>
          if key == "abortCode" then some value else none
        groups := { groups with
          abortsIf := groups.abortsIf.push
            { condition := condition.expression, code, range } }
    | .abortsWith =>
        let codes := #[condition.expression] ++ condition.auxiliary.map (·.2)
        groups := { groups with abortsWith := groups.abortsWith ++ codes.map (·, range) }
    | .letPre name => groups := { groups with
        lets := groups.lets.push { name, value := condition.expression, post := false } }
    | .letPost name => groups := { groups with
        lets := groups.lets.push { name, value := condition.expression, post := true } }
    | kind =>
        throwError "specification clause {repr kind} is not supported in           generated contracts"
  return groups

/-- A clause context with a contract's `let`s bound, in order: a
pre-state `let` is read at entry, a post-state one, where `post` admits it,
in the context itself. A contract's bindings are the function's last
locals, one per `let`, after the ones its body declares. -/
private def bindLets (context : Context) (lets : Array ContractLet) (post : Bool) :
    MetaM Context := do
  let mut context := context
  let firstBinding := context.localNames.size - lets.size
  for h : position in [:lets.size] do
    let binding := lets[position]
    if binding.post && !post then continue
    let reading := if binding.post then context
      else { context with locals := context.oldLocals, state := context.oldState }
    let index := firstBinding + position
    unless context.localNames[index]? == some binding.name do
      throwError "the contract binding `{binding.name}` has no local"
    let value ← translate reading binding.value
    let value ← match context.localTypes[index]?.map domainOf with
      | some .boolean => decideProposition value
      | _ => pure value
    context := { context with
      locals := context.locals.set! index (some value)
      oldLocals := context.oldLocals.set! index (some value) }
  return context


/-! ## Data invariants

Data invariants belong to values, rather than to the function which happens
to consume or produce them.  The runtime representation deliberately erases
those proofs, so the generated contract restores their verification meaning:
incoming values provide invariant assumptions, while outgoing values and
modified resources owe the invariant again. -/

/-- A type use under the type arguments of the declaration it occurs in:
the use, with the arguments its type parameters stand for. -/
private inductive Scoped where
  | mk (typeId : VerexIR.TypeId) (arguments : Array Scoped)

/-- The type a use denotes, with the arguments its parameters stand for; none
for a parameter of the function, whose type is not known. -/
private def Scoped.resolve (unit : ValidatedUnit) (arguments : Array Scoped)
    (typeId : VerexIR.TypeId) : Option (IrTy × Array Scoped) :=
  match unit.tables.types[typeId.index]? with
  | some (.typeParameter index) => do
      let .mk argument outer ← arguments[index]?
      let resolved ← unit.tables.types[argument.index]?
      if resolved matches .typeParameter _ then none else some (resolved, outer)
  | some ty => some (ty, arguments)
  | none => none

/-- The arguments a nominal type use gives its declaration's parameters. -/
private def Scoped.ofNominal (arguments : Array Scoped) (uses : Array VerexIR.GenericArgument) :
    Array Scoped :=
  uses.filterMap fun
    | .typeArg argument => some (.mk argument.typeId arguments)
    | _ => none

/-- The rows of fields a nominal declaration holds: its fields, or each
variant's payload with the variant's name. -/
private def fieldRows (unit : ValidatedUnit) (declaration : VerexIR.StructDecl) :
    Array (Option String × Array VerexIR.FieldDecl) :=
  if declaration.variants.isEmpty then #[(none, declaration.fields)]
  else declaration.variants.map fun variant =>
    (some (((unit.tables.names[variant.name.index]?).map (·.name)).getD ""), variant.fields)

/-- A scoped type argument in the native row used by a closure's frame. -/
private partial def Scoped.toNative? (unit : ValidatedUnit) (owner : VerexIR.NamespaceId)
    (argument : Scoped) : Option VerexIR.Proofs.Denote.NTy := do
  let .mk id arguments := argument
  let ty ← VerexIR.Proofs.Denote.ntyOf unit owner id
  if arguments.isEmpty then return ty
  let resolved ← arguments.toList.mapM (Scoped.toNative? unit owner)
  return ty.subst (VerexIR.Proofs.Denote.NRow.ofList resolved)

/-- The type of the values an intrinsic map holds, as a use under the owner's
arguments: its second type argument. -/
private def mapValueType? (unit : ValidatedUnit) (declaration : VerexIR.StructDecl)
    (arguments : Array Scoped) : Option (IrTy × Array Scoped) := do
  guard (mapModel? unit (.nominal declaration.name #[])).isSome
  let valueArgument ← arguments[1]?
  let .mk id scope := valueArgument
  Scoped.resolve unit scope id

/-- Whether values of a type carry a data invariant at any depth: one it
declares, the representation invariant of an intrinsic map, or one of a
field's, an element's, or a stored value's, as the Move Prover assumes them
(`data_invariant_instrumentation.rs`). -/
private partial def carriesInvariant (unit : ValidatedUnit) (ty : IrTy)
    (arguments : Array Scoped) (depth : Nat) : Bool :=
  if depth > 16 then false else
  match ty with
  | .vector element _ => match Scoped.resolve unit arguments element with
      | some (elementType, scope) => carriesInvariant unit elementType scope (depth + 1)
      | none => false
  | .tuple elements => elements.any fun element =>
      match Scoped.resolve unit arguments element with
      | some (elementType, scope) => carriesInvariant unit elementType scope (depth + 1)
      | none => false
  | .nominal _ uses => match nominalDeclaration? unit ty with
      | some (_, owner, declaration) =>
          let scope := Scoped.ofNominal arguments uses
          VerexIR.Proofs.Denote.declarationHasClosureFields owner declaration ||
            declaration.contract.conditions.any (·.kind == .structInvariant) ||
            (mapModel? unit ty).isSome ||
            (fieldRows unit declaration).any fun (_, fields) => fields.any fun field =>
              match Scoped.resolve unit scope field.type.typeId with
              | some (fieldType, fieldScope) => carriesInvariant unit fieldType fieldScope (depth + 1)
              | none => false
      | none => false
  | _ => false

/-- Whether a physical type contributes a data-invariant clause. -/
def hasDataInvariant (unit : ValidatedUnit) (ty : IrTy) : Bool :=
  carriesInvariant unit ty #[] 0

/-- Whether a data invariant a physical type carries, at any depth, states a
behavioral predicate: its clause then reads the executable unit. -/
private partial def carriesMatchingInvariant (unit : ValidatedUnit) (needed : SpecReads → Bool)
    (ty : IrTy) (arguments : Array Scoped) (depth : Nat) : Bool :=
  if depth > 16 then false else
  let element (typeId : VerexIR.TypeId) : Bool :=
    match Scoped.resolve unit arguments typeId with
    | some (elementType, scope) => carriesMatchingInvariant unit needed elementType scope (depth + 1)
    | none => false
  match ty with
  | .vector elementId _ => element elementId
  | .tuple elements => elements.any element
  | .nominal _ uses => match nominalDeclaration? unit ty with
      | some (handle, _, declaration) =>
          let reads := invariantReads unit handle.namespaceId declaration
          let scope := Scoped.ofNominal arguments uses
          needed reads ||
            (fieldRows unit declaration).any fun (_, fields) => fields.any fun field =>
              match Scoped.resolve unit scope field.type.typeId with
              | some (fieldType, fieldScope) =>
                  carriesMatchingInvariant unit needed fieldType fieldScope (depth + 1)
              | none => false
      | none => false
  | _ => false

/-- Whether a data invariant of a parameter or result of a function, at any
depth, states a behavioral predicate: the contract's clause for it then
reads the executable unit. -/
def signatureInvariantsReadUnit (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  let carries (typeId : VerexIR.TypeId) : Bool :=
    match ns.tables.types[typeId.index]? with
    | some (.reference reference) =>
        (ns.tables.types[reference.referent.index]?).any
          (carriesMatchingInvariant unit (fun reads => reads.unit || reads.requires) · #[] 0)
    | some ty => carriesMatchingInvariant unit (fun reads => reads.unit || reads.requires) ty #[] 0
    | none => false
  declaration.signature.parameters.any (carries ·.typeUse.typeId) ||
    declaration.signature.results.any (carries ·.typeId)

/-- The field, `this`, and ghost binders shared by a nominal declaration's
invariants and ghost initializers, of a native value of the declaration's
instance type `τ`: each field's component, the whole value after the
fields, and each ghost field's slot. An enum has no payload field locals. -/
private def declarationLocalValues (unit : ValidatedUnit) (owner : VerexIR.StructHandle)
    (ns : ValidatedNamespace) (declaration : VerexIR.StructDecl)
    (localTypes : Array IrTy) (carriers : Lean.Expr) (τ : VerexIR.Proofs.Denote.NTy)
    (value : Lean.Expr) : MetaM (Array (Option Lean.Expr)) := do
  let mut locals := Array.replicate declaration.locals.size none
  let quoted ← quoteNTy τ
  if declaration.variants.isEmpty then
    unless declaration.fields.size <= declaration.locals.size do
      throwError "a struct invariant has fewer declaration locals than fields"
    let components ← nativeComponents carriers quoted value
    for (_, index) in declaration.fields.zipIdx do
      let some localType := localTypes[index]?
        | throwError "a struct invariant field local is out of range"
      let some (_, component) := components[index]?
        | throwError "a struct invariant field is out of its value's row"
      locals := locals.set! index (some (← (domainOf localType).binderOfNative component))
    -- The whole value, after the fields.
    if (declaration.locals[declaration.fields.size]?).any (·.name == "this") then
      locals := locals.set! declaration.fields.size (some value)
    -- A ghost field's local, its slot.
    for ghost in declaration.ghosts do
      let some ghostName := ns.tables.names[ghost.name.index]? | continue
      let some position := declaration.locals.findIdx? (·.name == ghostName.name) | continue
      let some slot := VerexIR.SemanticOperations.handleGhostIndex? unit owner none
          ghostName.name
        | throwError "ghost field `{ghostName.name}` does not resolve in a data invariant"
      let some localType := localTypes[position]?
        | throwError "a data invariant's ghost local is out of range"
      let some (_, contents) := components[slot]?
        | throwError "ghost field `{ghostName.name}` is out of its value's row"
      locals := locals.set! position (some (← (domainOf localType).binderOfNative contents))
  else
    unless !declaration.locals.isEmpty do
      throwError "an enum invariant has no `this` declaration local"
    locals := locals.set! 0 (some value)
    -- A ghost field's local, its slot: each variant's slot follows its own
    -- fields.
    for ghost in declaration.ghosts do
      let some ghostName := ns.tables.names[ghost.name.index]? | continue
      let some position := declaration.locals.findIdx? (·.name == ghostName.name) | continue
      let some choices := declaration.variants.toList.mapM fun (variant : VerexIR.VariantDecl) => do
          let named : VerexIR.QualifiedName ← ns.tables.names[variant.name.index]?
          let index ← VerexIR.SemanticOperations.handleGhostIndex? unit owner
            (some named.name) ghostName.name
          some (named.name, index)
        | throwError "ghost field `{ghostName.name}` does not resolve in a data invariant"
      let some localType := localTypes[position]?
        | throwError "a data invariant's ghost local is out of range"
      let (_, contents) ← nativeVariantField τ choices.toArray carriers quoted value
      locals := locals.set! position (some (← (domainOf localType).binderOfNative contents))
  return locals

/-- Specialize every invariant declared by `ty` to a native value of its
instance type `τ`. Struct invariant locals are the fields in declaration
order; enum invariants receive the whole value as local zero and bind
payloads in their own logical match. -/
private def declaredInvariantTerms (context : Context) (ty : IrTy) (scope : Array Scoped)
    (τ : VerexIR.Proofs.Denote.NTy) (value : Lean.Expr) :
    MetaM (Array (Lean.Expr × ObligationRange)) := do
  let some (owner, ns, declaration) := nominalDeclaration? context.unit ty
    | return #[]
  let namespaceId := owner.namespaceId
  let conditions := declaration.contract.conditions.filter
    (fun condition => condition.kind == .structInvariant)
  if conditions.isEmpty then return #[]
  -- The declaration's type parameters stand for the arguments of the
  -- instance the value has, in a field, an element, or a map's entry: their
  -- values are native values of the arguments' native types.
  let localTypes ← declaration.locals.mapM fun localDecl => do
    let some localType := context.unit.tables.types[localDecl.type.typeId.index]?
      | throwError "data invariant local type {localDecl.type.typeId.index} is out of range"
    pure localType
  let typeArgumentTypes := scope.map (Scoped.toNative? context.unit namespaceId)
  let locals ← declarationLocalValues context.unit owner ns declaration localTypes
    (← context.carriers) τ value
  let invariantContext : Context := {
    context with
    namespaceId, ns, locals, localTypes, oldLocals := locals
    results := #[], resultTypes := #[], typeArgumentTypes }
  -- An invariant reading memory, through a behavioral predicate, holds in
  -- every memory: what a value satisfies does not change with the memory
  -- around it.
  let readsMemory := (invariantReads context.unit namespaceId declaration).state
  conditions.mapM fun condition => do
    unless condition.auxiliary.isEmpty do
      throwError "a data invariant cannot carry auxiliary expressions"
    let proposition ← if readsMemory then
        withLocalDeclD `memory (mkAppN (mkConst ``VerexIR.Proofs.Denote.Memory)
            (← invariantContext.leading)) fun memory => do
          mkForallFVars #[memory] (← translate
            { invariantContext with state := some memory, oldState := some memory }
            condition.expression) (usedOnly := true)
      else translate invariantContext condition.expression
    let range := conditionRange context.unit condition
    pure (proposition, range)


/-- The memory slot a `modifies global<T>(k)` clause names: its resource
type at the contract's frame and its storage key, over the contract's
logical binders. -/
private def modifiedSlotTerm (context : Context) (id : ExprId) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let some expression := context.ns.expressions[id.index]?
    | throwError "a modifies clause is out of range"
  match expression.kind with
  | .operation (.specification (.global _)) instantiations arguments _ =>
      let resource ← match instantiations.toList with
        | [.typeArg resource] => pure resource.typeId
        | _ => throwError "a modifies clause needs one resource type"
      let some key := arguments[0]?
        | throwError "a modifies clause expects one key"
      let typeArguments ← if context.typeArgumentTypes.isEmpty then pure none else do
        let types ← context.typeArgumentTypes.toList.mapM fun type => do
          let some type := type | throwError "a frame's type argument has no native type"
          pure type
        pure (some (VerexIR.Proofs.Denote.NRow.ofList types))
      let (resourceExpr, _) ← quoteResource (← context.frame) context.unit context.namespaceId
        resource typeArguments
      return (resourceExpr, mkApp (mkConst ``VerexIR.StorageKey.address) (← translate context key))
  | _ =>
      throwError "a modifies clause must name a resource at a key in \
        generated contracts"

/-- The frame of a change of global memory from `initial` to `final` within
the slots `slots` name: every listed resource type reads the same at every
key other than its listed ones, and, unless the frame is loose, every other
resource type reads the same everywhere. -/
private def slotFrame (initial final : Lean.Expr) (slots : Array (Lean.Expr × Lean.Expr))
    (loose : Bool) : MetaM Lean.Expr := do
  let mut resources : Array Lean.Expr := #[]
  for (resource, _) in slots do
    unless resources.contains resource do resources := resources.push resource
  let mut frames := #[]
  for resource in resources do
    let frame ← withLocalDeclD `key (mkConst ``VerexIR.StorageKey) fun key => do
      let mut implication ← mkEq (mkApp2 final resource key) (mkApp2 initial resource key)
      for (written, writtenKey) in slots.reverse do
        if written == resource then
          implication ← mkArrow (← mkAppM ``Ne #[key, writtenKey]) implication
      mkForallFVars #[key] implication
    frames := frames.push frame
  unless loose do
    let frame ← withLocalDeclD `resource (mkConst ``VerexIR.Proofs.Denote.ResourceType)
      fun other => do
        let mut implication ← mkEq (mkApp final other) (mkApp initial other)
        for resource in resources.reverse do
          implication ← mkArrow (← mkAppM ``Ne #[other, resource]) implication
        mkForallFVars #[other] implication
    frames := frames.push frame
  conjunction frames

/-- The element types of a quoted native row. -/
private partial def rowElementTypes (row : Lean.Expr) : Array Lean.Expr :=
  if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 then
    #[row.getArg! 0] ++ rowElementTypes (row.getArg! 1)
  else #[]

/-- The components of a native row value, as projections. -/
private def rowProjections (skolems row : Lean.Expr) (types : Array Lean.Expr) :
    MetaM (Array Lean.Expr) := do
    let carriers ← frameCarriers skolems
    let mut rest := row
    let mut values := #[]
    for index in [:types.size] do
      let tail := (types.extract (index + 1) types.size).foldr
        (fun ty row => mkApp2 (mkConst ``VerexIR.Proofs.Denote.NRow.cons) ty row)
        (mkConst ``VerexIR.Proofs.Denote.NRow.nil)
      let carrier := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) carriers types[index]!
      let tailType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) carriers tail
      values := values.push (mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) carrier tailType rest)
      rest := mkApp3 (mkConst ``Prod.snd [Level.zero, Level.zero]) carrier tailType rest
    return values

/-- The result shape of a function type's result row: an invocation returns
no value or one. -/
private def functionShape (results : VerexIR.Proofs.Denote.NRow) :
    MetaM VerexIR.Proofs.Denote.ResultShape :=
  match results with
  | .nil => pure .none
  | .cons τ .nil => pure (.one τ)
  | _ => throwError "a function value returning several results is not carried"

/-- The frame a function-valued field's runs keep, of the field's function
value `value` of native type `fieldType` in the whole native value `whole`
of type `τ`: its `modifies_of` frame over the invocation's arguments, or,
with none, keeping memory. -/
private def fieldFrameTerm (context : Context) (owner : VerexIR.NamespaceId)
    (declaration : VerexIR.StructDecl) (index : Nat) (scope : Array Scoped)
    (τ : VerexIR.Proofs.Denote.NTy) (whole : Lean.Expr) (fieldType : VerexIR.Proofs.Denote.NTy)
    (value : Lean.Expr) : MetaM Lean.Expr := do
  let .function parameters shared results := fieldType
    | throwError "a function-valued field's native type is not a function"
  let some executable := context.executable
    | throwError "a function-valued field's frame needs the executable unit"
  let skolems ← context.frame
  let unitExpr ← context.unitExpr
  let functions ← VerexIR.Proofs.Denote.functionCarriersOfFrame skolems
  let carriers ← context.carriers
  let row ← quoteRow parameters
  let sharing := toExpr shared
  let shape ← quoteShape (← functionShape results)
  -- Over a model, the frame is of the model's invocations.
  let invoke? ← modelInvoke? executable skolems
  let some frame := declaration.contract.parameterFrames.find? (·.parameter.index == index)
    | match invoke? with
      | some invoke =>
          mkAppM' (mkApp4 (mkConst ``VerexIR.Proofs.KeepsMemoryAtOf) unitExpr functions skolems
            invoke) #[row, sharing, shape, value]
      | none =>
          mkAppM' (mkApp4 (mkConst ``VerexIR.Proofs.KeepsMemoryAt)
            unitExpr functions executable skolems) #[row, sharing, shape, value]
  if frame.modifiesAll then return mkConst ``True
  let some ns := context.unit.namespaces[owner.index]?
    | throwError "a field frame's namespace is out of range"
  let localTypes ← declaration.locals.mapM fun localDecl => do
    let some ty := context.unit.tables.types[localDecl.type.typeId.index]?
      | throwError "a field frame's local has no type"
    pure ty
  let mut locals := Array.replicate declaration.locals.size none
  let components ← nativeComponents carriers (← quoteNTy τ) whole
  for (_, fieldIndex) in declaration.fields.zipIdx do
    let (some localType, some (_, component)) := (localTypes[fieldIndex]?, components[fieldIndex]?)
      | throwError "a field frame's field is out of its value's row"
    locals := locals.set! fieldIndex (some (← (domainOf localType).binderOfNative component))
  if (declaration.locals[declaration.fields.size]?).any (·.name == "this") then
    locals := locals.set! declaration.fields.size (some whole)
  let typeArgumentTypes := scope.map (Scoped.toNative? context.unit owner)
  let invocationType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) carriers row
  let memoryType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.Memory) unitExpr functions
  let frameTerm ← withLocalDeclD `invocation invocationType fun invocation =>
    withLocalDeclD `pre memoryType fun pre =>
    withLocalDeclD `post memoryType fun post => do
      let componentTypes := rowElementTypes row
      let components ← rowProjections skolems invocation componentTypes
      let mut active : Context := { context with
        namespaceId := owner, ns, locals, localTypes, oldLocals := locals
        results := #[], resultTypes := #[], state := some pre, oldState := some pre
        typeArgumentTypes }
      for (formal, component) in frame.formals.zip components do
        let some physical := localTypes[formal.index]?
          | throwError "a field frame's formal has no local type"
        let binder ← (domainOf physical).binderOfNative component
        active := { active with
          locals := active.locals.set! formal.index (some binder)
          oldLocals := active.oldLocals.set! formal.index (some binder) }
      let slots ← frame.modifies.mapM (modifiedSlotTerm active)
      mkLambdaFVars #[invocation, pre, post] (← slotFrame pre post slots false)
  match invoke? with
  | some invoke =>
      mkAppM' (mkApp4 (mkConst ``VerexIR.Proofs.FramedAtOf) unitExpr functions skolems invoke)
        #[row, sharing, shape, frameTerm, value]
  | none =>
      mkAppM' (mkApp4 (mkConst ``VerexIR.Proofs.FramedAt)
        unitExpr functions executable skolems) #[row, sharing, shape, frameTerm, value]

/-- The native type of a variant's field of an enum type. -/
private def variantFieldType? (τ : VerexIR.Proofs.Denote.NTy) (variant : String) (index : Nat) :
    Option VerexIR.Proofs.Denote.NTy := do
  let .enum _ _ names rows _ := τ | none
  let row ← VerexIR.Proofs.Denote.NRows.variant? names rows variant
  row.toList[index]?

/-- The data invariants of a native value of a type, at any depth, as the
Move Prover assumes them: the type's declared invariants; for an intrinsic
map, its discipline's validity and the invariants of its values; the
invariants of each field, under the test of its variant; and those of each
element of a vector. `τ` is the value's native type, the type `ty` denotes
under `arguments`. -/
private partial def deepInvariantTerms (context : Context) (ty : IrTy) (arguments : Array Scoped)
    (τ : VerexIR.Proofs.Denote.NTy) (value : Lean.Expr) (depth : Nat) :
    MetaM (Array (Lean.Expr × ObligationRange)) := do
  unless carriesInvariant context.unit ty arguments depth do return #[]
  let carriers ← context.carriers
  match ty, τ with
  | .vector element _, .vector elementType =>
      let some (elementIr, scope) := Scoped.resolve context.unit arguments element | return #[]
      let carrier := carrierType carriers (← quoteNTy elementType)
      withLocalDeclD `element carrier fun elementValue => do
        let terms ← deepInvariantTerms context elementIr scope elementType elementValue (depth + 1)
        let some (_, range) := terms[0]? | return #[]
        let member ← mkAppM ``Membership.mem
          #[← mkAppM ``VerexIR.SpecVector.values #[value], elementValue]
        let body ← mkArrow member (← conjunction (terms.map (·.1)))
        return #[(← mkForallFVars #[elementValue] body, range)]
  | .tuple elements, .tuple row =>
      -- The invariants of each component of a multi-value result.
      let components ← nativeComponents carriers (← quoteNTy τ) value
      let mut terms := #[]
      for (element, index) in elements.zipIdx do
        let some (elementIr, scope) := Scoped.resolve context.unit arguments element | continue
        let (some componentType, some (_, component)) := (row.toList[index]?, components[index]?)
          | throwError "internal: a tuple's component is out of its native row"
        terms := terms ++
          (← deepInvariantTerms context elementIr scope componentType component (depth + 1))
      return terms
  | .nominal _ uses, _ =>
      let some (owner, ns, declaration) := nominalDeclaration? context.unit ty | return #[]
      if !declaration.variants.isEmpty && !declaration.contract.parameterFrames.isEmpty then
        throwError "modifies_of on enum fields is not carried yet"
      let scope := Scoped.ofNominal arguments uses
      let mut terms ← declaredInvariantTerms context ty scope τ value
      if let some model := mapModel? context.unit ty then
        let some intrinsic := ns.intrinsics.find? (·.owner == declaration.name) | return terms
        let range := locRange context.unit intrinsic.loc
        -- An owner's entries are valid under its discipline, and each value
        -- held satisfies its invariants.
        let owned ← nativeOwner carriers τ value
        let table := owned.typed
        terms := terms.push (← mkAppOptM ``VerexIR.SpecMap.Valid
          #[table.keys.carrier, table.values.carrier, ← table.discipline model, table.map], range)
        if let some (valueIr, valueScope) := mapValueType? context.unit declaration scope then
          let some (.table _ valueType) := owned.fields.toList.getLast?
            | throwError "internal: a map owner's last field is not its table"
          let stored ← withLocalDeclD `key table.keys.carrier fun key => do
            let held := TypedTable.nativeOf table.values (← table.valueAt key)
            let inner ← deepInvariantTerms context valueIr valueScope valueType held (depth + 1)
            let some (_, range) := inner[0]? | return none
            let present ← mkEq (← table.hasKey key) (mkConst ``Bool.true)
            let body ← mkArrow present (← conjunction (inner.map (·.1)))
            return some (← mkForallFVars #[key] body, range)
          if let some term := stored then terms := terms.push term
        return terms
      let quoted ← quoteNTy τ
      let components ← if declaration.variants.isEmpty then nativeComponents carriers quoted value
        else pure #[]
      let fieldRow := match τ with
        | .struct _ _ fields => fields.toList
        | _ => []
      for (variant, fields) in fieldRows context.unit declaration do
        for (field, index) in fields.zipIdx do
          let some (fieldIr, fieldScope) := Scoped.resolve context.unit scope field.type.typeId
            | continue
          let fieldTerms (fieldType : VerexIR.Proofs.Denote.NTy) (selected : Lean.Expr) :
              MetaM (Array (Lean.Expr × ObligationRange)) := do
            let mut inner ← deepInvariantTerms context fieldIr fieldScope fieldType selected
              (depth + 1)
            if VerexIR.Proofs.Denote.fieldKeepsMemory ns field then
              inner := inner.push (← fieldFrameTerm context owner.namespaceId declaration index
                scope τ value fieldType selected, locRange context.unit field.loc)
            return inner
          match variant with
          | none =>
              let (some fieldType, some (_, component)) := (fieldRow[index]?, components[index]?)
                | throwError "internal: a struct's field is out of its native row"
              terms := terms ++ (← fieldTerms fieldType component)
          | some name =>
              let some fieldType := variantFieldType? τ name index
                | throwError "internal: a variant's field is out of its native row"
              if let some (_, read) ← typedVariantField? τ #[(name, index)] carriers quoted value then
                let test ← nativeVariantTest carriers quoted value owner #[name]
                for (term, range) in ← fieldTerms fieldType read do
                  terms := terms.push (← mkArrow test term, range)
              else
                -- A field of a type without a value to stand for it under
                -- another variant, such as a function value: the terms of
                -- what the selection holds.
                let some (fieldTypeExpr, selection) ← typedVariantSelection? τ #[(name, index)]
                    carriers quoted value
                  | throwError "internal: a variant field selection does not resolve"
                let held ← withLocalDeclD `field (carrierType carriers fieldTypeExpr) fun payload => do
                  let inner ← fieldTerms fieldType payload
                  let held ← mkEq selection (← mkAppM ``Option.some #[payload])
                  inner.mapM fun (term, range) => do
                    pure (← mkForallFVars #[payload] (← mkArrow held term), range)
                terms := terms ++ held
      return terms
  | _, _ => return #[]

/-- The data invariants of a native value of a physical type, at any depth. -/
private def dataInvariantTerms (context : Context) (ty : IrTy) (τ : VerexIR.Proofs.Denote.NTy)
    (value : Lean.Expr) : MetaM (Array (Lean.Expr × ObligationRange)) :=
  deepInvariantTerms context ty #[] τ value 0

/-- Data invariants carried by selected native values of a parameter/result
row. Data invariants constrain a reference's referent. -/
private def slotDataInvariantTerms (context : Context) (slots : Array Slot)
    (bound : Array SlotBinders)
    (nativeOf : SlotBinders → Option Lean.Expr) :
    MetaM (Array (Lean.Expr × ObligationRange)) := do
  let mut terms := #[]
  for (slot, binders) in slots.zip bound do
    unless hasDataInvariant context.unit slot.physical do continue
    let some value := nativeOf binders | continue
    terms := terms ++ (← dataInvariantTerms context slot.physical slot.nty value)
  pure terms

/-- The name a clause gives the opaque specification function `name` of the
namespace at `path` (`opaqueSpec`), when the unit declares it. -/
private def declaredOpaqueSpec? (unit : ValidatedUnit) (path : List String) (name : String) :
    Option String := do
  let index ← unit.tables.namespaces.toList.findIdx? (·.segments.toList == path)
  let ns ← unit.namespaces[index]?
  guard <| ns.specFunctions.any fun function =>
    function.body.isNone && (unit.tables.names[function.name.index]?.map (·.name)) == some name
  some ("::".intercalate (path ++ [name]))

/-- Whether a physical type is a byte vector, `Vector<u8>`. -/
private def isByteVector (unit : ValidatedUnit) (ty : IrTy) : Bool :=
  if let .vector element _ := ty then
    unit.tables.types[element.index]? == some (.integer (.bits 8) false)
  else false

/-- Whether the unit declares the native at a qualified name. -/
private def declaresNative (unit : ValidatedUnit) (qualified : String) : Bool := Id.run do
  let segments := qualified.splitOn "::"
  let some name := segments.getLast? | return false
  let path := segments.dropLast
  let some index := unit.tables.namespaces.toList.findIdx? (·.segments.toList == path)
    | return false
  let some ns := unit.namespaces[index]? | return false
  return ns.functions.any fun function =>
    function.body == .absent && (unit.tables.names[function.name.index]?.map (·.name)) == some name

/-- The value of an uninterpreted function at type arguments and clause
values, as a clause applying it denotes it (`opaqueSpec`): of a native
result type its native value, of a scalar one its clause value. -/
private def opaqueApplication (context : Context) (name : String) (typeArguments : List Lean.Expr)
    (domain : Domain) (resultType : Option VerexIR.Proofs.Denote.NTy)
    (values : Array (Domain × Lean.Expr)) : MetaM Lean.Expr := do
  let bundle ← values.mapM fun (domain, value) => do
    bundleComponent domain (← domain.ofBinder value)
  match domain.scalarType? with
  | some type =>
      let inhabited ← synthInstance (← mkAppM ``Inhabited #[type])
      opaqueValue name typeArguments type inhabited bundle context.ns.profile
  | none =>
      let some nty := resultType
        | throwError "internal: an uninterpreted value of a native type without the type"
      let carriers ← context.carriers
      let τ ← quoteNTy nty
      let carrier := carrierType carriers τ
      let inhabited := mkApp2 (mkConst ``Inhabited.mk [1]) carrier (← typedInhabitant carriers τ)
      opaqueValue name typeArguments carrier inhabited bundle context.ns.profile

/-- The native type of a byte vector. -/
private def bytesType : VerexIR.Proofs.Denote.NTy := .vector (.int 8 false)

/-- The length of a native vector, as a clause reads it. -/
private def nativeLength (vector : Lean.Expr) : MetaM Lean.Expr := do
  mkAppOptM ``Nat.cast #[mkConst ``Int, none,
    ← mkAppM ``Array.size #[← mkAppM ``VerexIR.SpecVector.values #[vector]]]

/-- What a Move function assumes of the hashes it applies at any depth: no
two of its byte-vector parameters collide (`moveCollisionFreeHashes`), as a
hash's value at them reads in a clause. -/
private def hashFacts (context : Context)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (slots : Array Slot) (bound : Array SlotBinders) : MetaM (Array Lean.Expr) := do
  unless context.ns.profile == some .move do return #[]
  let hashes := reachedFunctions context.unit context.namespaceId declaration
    (moveCollisionFreeHashes.filter (declaresNative context.unit))
  if hashes.isEmpty then return #[]
  let mut values := #[]
  for (slot, binders) in slots.zip bound do
    if isByteVector context.unit slot.physical then
      values := values.push binders.entry
  let mut facts := #[]
  let carriers ← context.carriers
  for hash in hashes do
    -- At the type the hash takes its input at, its integers unbounded
    -- (`Context.uninterpretedParameterType`).
    let application (value : Lean.Expr) : MetaM Lean.Expr := do
      let input ← coerceNative carriers bytesType (widenedNTy bytesType) value
      opaqueApplication context hash [] .native (some bytesType) #[(.native, input)]
    for i in [0:values.size] do
      for j in [i + 1:values.size] do
        let collide ← mkEq (← application values[i]!) (← application values[j]!)
        facts := facts.push (← mkArrow collide (← mkEq values[i]! values[j]!))
  return facts

/-- What a Move function assumes of `bcs::serialize` at the values it
takes, where it applies the function at any depth (`bcs::to_bytes` reads it):
the Move Prover's prelude (`native.bpl`) states of every value that the
serialization is injective and that addresses serialize to one length, which
no function into bounded byte vectors is of every value. They are stated of
the function's parameters, as the hashes' collision freedom is
(`hashFacts`); that no serialization is empty holds of the function itself
(`serializedBytes_nonempty`). -/
private def serializeFacts (context : Context)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (slots : Array Slot) (bound : Array SlotBinders) : MetaM (Array Lean.Expr) := do
  unless context.ns.profile == some .move do return #[]
  let serialize := "0x1::bcs::serialize"
  if (reachedFunctions context.unit context.namespaceId declaration
      [serialize, "0x1::bcs::to_bytes"]).isEmpty then return #[]
  let some types := context.types | return #[]
  -- Each parameter's value, with its type (a reference's referent).
  let mut values : Array (TypeId × IrTy × Lean.Expr × Lean.Expr) := #[]
  for ((slot, binders), parameter) in (slots.zip bound).zip declaration.signature.parameters do
    let typeId := match context.unit.tables.types[parameter.typeUse.typeId.index]? with
      | some (.reference reference) => reference.referent
      | _ => parameter.typeUse.typeId
    let some nty := context.ntyOf? typeId | continue
    let typeArgument := mkApp (mkConst ``VerexSL.Contract.SpecTypeArgument.native)
      (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.substWith) types (← quoteNTy nty))
    let serialized ← opaqueApplication context serialize [typeArgument] .native (some bytesType)
      #[(domainOf slot.physical, binders.entry)]
    values := values.push (typeId, slot.physical, binders.entry, serialized)
  let mut facts := #[]
  for i in [0:values.size] do
    for j in [i + 1:values.size] do
      let (left, leftType, leftValue, leftSerialized) := values[i]!
      let (right, _, rightValue, rightSerialized) := values[j]!
      unless left == right do continue
      facts := facts.push (← mkArrow (← mkEq leftSerialized rightSerialized)
        (← mkEq leftValue rightValue))
      if leftType matches .address then
        facts := facts.push (← mkEq (← nativeLength leftSerialized)
          (← nativeLength rightSerialized))
  return facts

/-- What the Move Prover assumes of every signer value: that it signs the
transaction, as `std::signer` states it (`is_txn_signer` of the signer,
`is_txn_signer_addr` of its address), where the unit declares those
predicates. A Move signer comes from a parameter or a native, so stating it
of the signers a function takes and returns carries it everywhere. A signer
and its address are the same native value. -/
private def signerFacts (context : Context) (slots : Array Slot) (bound : Array SlotBinders)
    (binderOf : SlotBinders → Option Lean.Expr) : MetaM (Array Lean.Expr) := do
  unless context.ns.profile == some .move do return #[]
  let predicates := ["is_txn_signer", "is_txn_signer_addr"].filterMap fun name =>
    declaredOpaqueSpec? context.unit ["0x1", "signer"] name
  if predicates.isEmpty then return #[]
  let mut facts := #[]
  for (slot, binders) in slots.zip bound do
    unless slot.physical matches .signer do continue
    let some value := binderOf binders | continue
    for name in predicates do
      let call ← opaqueApplication context name [] .boolean none #[(.text, value)]
      facts := facts.push (← Domain.boolean.ofBinder call)
  return facts

/-- Whether a namespace invariant reads memory a function reaches
(`memoryReach`): those it assumes at entry and owes where it writes, as the
Move Prover evaluates the invariants of the memory a function uses. An axiom
reading no memory holds in every state and is assumed everywhere, as the Move
Prover states its axioms globally. -/
private def invariantReached (reach : Option (Array VerexIR.StructHandle))
    (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (invariant : VerexIR.NamespaceInvariant) : Bool :=
  let read := reachFrom unit [(namespaceId, invariant.condition.expression)]
  if invariant.condition.kind matches .axiom_ _ && read.all (·.isEmpty) then true else
  match reach, read with
  | some reached, some read => read.any reached.contains
  | _, _ => false

/-- Whether a namespace invariant is `[suspendable]`. -/
private def isSuspendable (invariant : VerexIR.NamespaceInvariant) : Bool :=
  invariant.condition.properties.any fun
    | .assign "suspendable" (.constant (.bool true)) _ => true
    | .call "suspendable" arguments _ => arguments.isEmpty
    | _ => false

/-- The functions a function's body calls or makes closures of, as
namespace and function indices. -/
private def bodyCallees (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Array (Nat × Nat) := Id.run do
  let .structured root := declaration.body | return #[]
  let some ns := unit.namespaces[namespaceId.index]? | return #[]
  let mut work : List ExprId := [root]
  let mut callees : Array (Nat × Nat) := #[]
  for _ in [0:ns.expressions.size + 1] do
    match work with
    | [] => break
    | id :: rest =>
        work := rest
        let some expression := ns.expressions[id.index]? | continue
        if let .operation operation _ _ _ := expression.kind then
          let reference? := match operation with
            | .call (.function reference) | .call (.closure reference _) => some reference
            | _ => none
          if let some reference := reference? then
            if let some functionId := unit.resolution.function? reference.name then
              let callee := (reference.namespaceId.index, functionId.index)
              unless callees.contains callee do callees := callees.push callee
        work := (VerexIR.Validation.expressionChildren expression.kind).toList ++ work
  return callees

/-- Whether a function's callers carry the unit's `[suspendable]` invariants
for it, as the Move Prover delegates them: it declares
`delegate_invariants_to_caller`, or a function that declares it or
`disable_invariants_in_body` calls it, at any depth. A function with
callers outside the unit, public or entry, carries them itself. -/
def delegatesInvariants (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool := Id.run do
  unless unit.namespaces.any (·.invariants.any isSuspendable) do return false
  let .ok modifiers := declaredModifiers declaration | return false
  if modifiers.visibility == .public_ || modifiers.isEntry then return false
  let some ns := unit.namespaces[namespaceId.index]? | return false
  let some index := ns.functions.findIdx? (·.name == declaration.name) | return false
  let mut delegated : Std.HashSet (Nat × Nat) := {}
  let mut work : List (Nat × Nat) := []
  let mut count := 0
  for (ns, nsIndex) in unit.namespaces.toList.zipIdx do
    for (function, functionIndex) in ns.functions.toList.zipIdx do
      count := count + 1
      let delegating := pragmaEnabled function.contract "delegate_invariants_to_caller"
      if delegating then delegated := delegated.insert (nsIndex, functionIndex)
      if delegating || pragmaEnabled function.contract "disable_invariants_in_body" then
        work := (nsIndex, functionIndex) :: work
  -- Every function enters the work list at most twice.
  for _ in [0:2 * count + 1] do
    match work with
    | [] => break
    | (nsIndex, functionIndex) :: rest =>
        work := rest
        let some function := unit.namespaces[nsIndex]?.bind (·.functions[functionIndex]?)
          | continue
        for callee in bodyCallees unit ⟨nsIndex⟩ function do
          unless delegated.contains callee do
            delegated := delegated.insert callee
            work := callee :: work
  return delegated.contains (namespaceId.index, index)

/-- The namespace invariants a function assumes at entry and owes at exit:
those of the memory it reaches, but for the `[suspendable]` ones its
callers carry (`delegatesInvariants`). -/
private def invariantApplies (reach : Option (Array VerexIR.StructHandle))
    (unit : ValidatedUnit) (delegated : Bool) (namespaceId : VerexIR.NamespaceId)
    (invariant : VerexIR.NamespaceInvariant) : Bool :=
  invariantReached reach unit namespaceId invariant && !(delegated && isSuspendable invariant)

/-- The namespace invariants a write of a resource owes in a function, as
the Move Prover checks them after the write: those the function carries
(`invariantApplies`) that read the resource written, update invariants
comparing with `oldState`, the memory before the write. `True` where the
function's body defers its invariants to its exit
(`disable_invariants_in_body`). -/
def memoryWriteInvariants (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace) (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (types : Option Lean.Expr)
    (written : VerexIR.StructHandle) (state oldState : Lean.Expr)
    (executable : Option Lean.Expr) (writtenType : Option TypeId := none) : MetaM Lean.Expr := do
  if pragmaEnabled declaration.contract "disable_invariants_in_body" then
    return mkConst ``True
  let reach := memoryReach unit namespaceId declaration
  let delegated := delegatesInvariants unit namespaceId declaration
  let context : Context := {
    unit, namespaceId, ns, locals := #[], oldLocals := #[], localTypes := #[],
    localNames := #[], results := #[], types, state := some state,
    oldState := some oldState, executable }
  let reads (invariantNamespace : VerexIR.NamespaceId) (invariant : VerexIR.NamespaceInvariant) :=
    (reachFrom unit [(invariantNamespace, invariant.condition.expression)]).any
      (·.contains written)
  -- A generic invariant is owed at the instances of the type written.
  let terms ← namespaceInvariantTerms context none .exit (fun invariantNamespace invariant =>
    invariantApplies reach unit delegated invariantNamespace invariant &&
      reads invariantNamespace invariant)
    (memory := writtenType.toArray) (ghostBase := some (typeArity declaration))
  conjunction (terms.map fun (clause, range) => markObligation range clause)

/-- The callees a function calls that leave their `[suspendable]`
invariants to their callers (`delegatesInvariants`), each with the resource
declarations it writes, at any depth: the function checks those invariants
after each call, as the Move Prover does, where it carries them itself and
its body does not defer them. -/
def delegatingCallees (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Array (VerexIR.FunctionHandle × Array VerexIR.StructHandle) := Id.run do
  if pragmaEnabled declaration.contract "disable_invariants_in_body" ||
      delegatesInvariants unit namespaceId declaration then return #[]
  let mut found := #[]
  for (calleeNs, calleeIndex) in bodyCallees unit namespaceId declaration do
    let some callee := unit.namespaces[calleeNs]?.bind (·.functions[calleeIndex]?) | continue
    unless delegatesInvariants unit ⟨calleeNs⟩ callee do continue
    let some written := reachFrom unit
        ((functionRoots callee).toList.map (⟨calleeNs⟩, ·)) (writes := true) | continue
    found := found.push ({ namespaceId := ⟨calleeNs⟩, functionId := ⟨calleeIndex⟩ }, written)
  return found

/-- The `[suspendable]` invariants a function carries that read memory a
callee of `delegatingCallees` writes: owed after the call over the memory
after it, update invariants comparing with `before`. -/
def callWriteInvariants (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace) (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (types : Option Lean.Expr)
    (written : Array VerexIR.StructHandle) (before after : Lean.Expr)
    (executable : Option Lean.Expr) : MetaM Lean.Expr := do
  let reach := memoryReach unit namespaceId declaration
  let context : Context := {
    unit, namespaceId, ns, locals := #[], oldLocals := #[], localTypes := #[],
    localNames := #[], results := #[], types, state := some after,
    oldState := some before, executable }
  let reads (invariantNamespace : VerexIR.NamespaceId) (invariant : VerexIR.NamespaceInvariant) :=
    (reachFrom unit [(invariantNamespace, invariant.condition.expression)]).any
      fun read => read.any written.contains
  let uses := frameUses unit namespaceId declaration
  let terms ← namespaceInvariantTerms context none .exit (fun invariantNamespace invariant =>
    isSuspendable invariant && invariantApplies reach unit false invariantNamespace invariant &&
      reads invariantNamespace invariant)
    (memory := uses.memory) (ghostBase := some (typeArity declaration))
    (ghostMemory := some uses.written)
  conjunction (terms.map fun (clause, range) => markObligation range clause)

/-- Whether a function owes invariants of memory: it writes, at any depth,
memory a namespace invariant of the unit reads, or a resource that carries a
data invariant. Such a function is verified for them even without a
specification of its own. -/
def owesMemoryInvariants (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  match reachFrom unit
      ((functionRoots declaration).toList.map (namespaceId, ·)) (writes := true) with
  | none => false
  | some written =>
      let delegated := delegatesInvariants unit namespaceId declaration
      let invariantRead := (unit.namespaces.toList.zipIdx).any fun (ns, index) =>
        ns.invariants.any fun invariant =>
          invariantApplies (some written) unit delegated ⟨index⟩ invariant
      invariantRead || written.any fun handle =>
        (unit.namespaces[handle.namespaceId.index]?.bind (·.structs[handle.structId]?)).any
          fun resource => resource.abilities.contains .key &&
            hasDataInvariant unit (.nominal resource.name #[])

/-- A resource declaration by its module path and name, which identifies it
across units. -/
def qualifiedResource? (unit : ValidatedUnit) (handle : VerexIR.StructHandle) :
    Option (Array String × String) := do
  let path ← unit.tables.namespaces[handle.namespaceId.index]?
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.structs[handle.structId]?
  let name ← unit.tables.names[declaration.name.index]?
  pure (path.segments, name.name)

/-- The resources, by qualified name, the functions of a namespace reach in
global memory, or with `writes` write. -/
def namespaceFunctionMemory (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (writes : Bool) : Array (Array String × String) := Id.run do
  let some ns := unit.namespaces[namespaceId.index]? | return #[]
  let mut found := #[]
  for declaration in ns.functions do
    let reached := (reachFrom unit ((functionRoots declaration).toList.map (namespaceId, ·))
      (writes := writes)).getD #[]
    for handle in reached do
      if let some resource := qualifiedResource? unit handle then
        unless found.contains resource do found := found.push resource
  return found

/-- The resources, by qualified name, the invariants of a namespace read. -/
def namespaceInvariantMemory (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId) :
    Array (Array String × String) := Id.run do
  let some ns := unit.namespaces[namespaceId.index]? | return #[]
  let mut found := #[]
  for invariant in ns.invariants do
    for handle in (reachFrom unit [(namespaceId, invariant.condition.expression)]).getD #[] do
      if let some resource := qualifiedResource? unit handle then
        unless found.contains resource do found := found.push resource
  return found

/-- Whether a function owes an invariant of the namespace `invariantNs`: it
writes, at any depth, memory such an invariant reads, and carries it. -/
def owesInvariantsOf (unit : ValidatedUnit) (invariantNs namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  match reachFrom unit
      ((functionRoots declaration).toList.map (namespaceId, ·)) (writes := true) with
  | none => false
  | some written =>
      let delegated := delegatesInvariants unit namespaceId declaration
      (unit.namespaces[invariantNs.index]?).any fun ns =>
        ns.invariants.any (invariantApplies (some written) unit delegated invariantNs ·)

/-- Whether a function can reach global memory (`memoryReach`). -/
def reachesMemory (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  (memoryReach unit namespaceId declaration).isSome

/-- The data invariants of a unit's stored resources: their predicate
(`storedInvariantTerm`), the declarations it states an invariant of, each
with its native type and its invariant (`StoredCase`) over an instance's
type arguments and a value of it at the frame they induce (after the
executable unit, where they read it), and those whose invariants it does
not carry, each with the reason, which a function reaching them reports. -/
structure StoredInvariants where
  predicate : Option Lean.Expr := none
  carriers : Array VerexIR.StructHandle := #[]
  types : Array VerexIR.Proofs.Denote.NTy := #[]
  cases : Array Lean.Expr := #[]
  unsupported : Array (VerexIR.StructHandle × String) := #[]

/-- The data invariants of the unit's stored resources: of each `key`
declaration that has one, at any depth, its invariant over the type
arguments `θ` of an instance and a native value of it at the frame `θ`
induces over the runtime frame of the unit memory is typed at. An
invariant reading memory holds in every memory, so none reads memory. Each
takes that unit first, then the executable unit where one states a
behavioral predicate (`storedInvariantAt`). -/
def storedInvariantTerm (unit : ValidatedUnit) :
    MetaM StoredInvariants := do
  let readsUnit := storedInvariantReadsUnit unit
  let mut carriers : Array VerexIR.StructHandle := #[]
  let mut types : Array VerexIR.Proofs.Denote.NTy := #[]
  let mut cases : Array Lean.Expr := #[]
  let mut unsupported : Array (VerexIR.StructHandle × String) := #[]
  for h : namespaceIndex in [:unit.namespaces.size] do
    let ns := unit.namespaces[namespaceIndex]
    for h : structIndex in [:ns.structs.size] do
      let declaration := ns.structs[structIndex]
      let owner : VerexIR.StructHandle := ⟨⟨namespaceIndex⟩, structIndex⟩
      unless declaration.abilities.contains .key do continue
      let ty : IrTy := .nominal declaration.name #[]
      unless hasDataInvariant unit ty do continue
      let some σ := VerexIR.Proofs.Denote.structNTy unit owner
        | throwError "internal: a stored declaration has no native type"
      -- A declaration with type parameters states its invariant at the frame
      -- its resource's type arguments induce, one without at the runtime
      -- frame its values are at.
      let generic := !(σ.nominalArguments? matches some .nil)
      let case? : Option Lean.Expr ⊕ String ←
          withUnitVariable .default fun memoryUnit functions =>
          withLocalDeclD `executable (unitReaderType unit memoryUnit functions) fun executable => do
        let leading :=
          if readsUnit then #[memoryUnit, functions, executable] else #[memoryUnit, functions]
        let atFrame (binder frame : Lean.Expr) : MetaM (Option Lean.Expr ⊕ String) := do
          let carrier := carrierType (← frameCarriers frame) (← quoteNTy σ)
          withLocalDeclD `value carrier fun value => do
            let context : Context := {
              unit, namespaceId := ⟨namespaceIndex⟩, ns, locals := #[], results := #[]
              types := some (← frameTypes frame)
              executable := if readsUnit then some executable else none }
            let terms ← try dataInvariantTerms context ty σ value catch error =>
              return Sum.inr (← error.toMessageData.toString)
            if terms.isEmpty then return Sum.inl none
            let held ← conjunction (terms.map fun (term, range) => markObligation range term)
            return Sum.inl (some (← mkLambdaFVars (leading ++ #[binder, value]) held))
        withLocalDeclD `θ (mkConst ``VerexIR.Proofs.Denote.TypeArgs) fun θ =>
          let runtime := mkApp2 (mkConst ``VerexIR.Proofs.Denote.Skolems.runtime) memoryUnit functions
          atFrame θ (if generic then
              mkApp4 (mkConst ``VerexIR.Proofs.Denote.Skolems.instantiate) memoryUnit functions θ
                runtime
            else runtime)
      match case? with
      | .inr reason => unsupported := unsupported.push (owner, reason)
      | .inl none => pure ()
      | .inl (some case) =>
          carriers := carriers.push owner
          types := types.push σ
          cases := cases.push case
  return { carriers, types, cases, unsupported }

/-- The stored invariants' predicate at a memory: over the unit the memory
is typed at, then the executable unit where they read it. -/
def storedInvariantAt (unit : ValidatedUnit) (predicate : Lean.Expr) (executable : Option Lean.Expr)
    (memory : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfR (← inferType memory)
  unless type.isAppOfArity ``VerexIR.Proofs.Denote.Memory 2 do
    throwError "internal: the stored invariants are applied to {memory}, not a memory"
  let atUnit := mkApp2 predicate (type.getArg! 0) (type.getArg! 1)
  unless storedInvariantReadsUnit unit do return atUnit
  let some executable := executable
    | throwError "the stored invariants read the executable unit, which this contract does \
        not take"
  return mkApp atUnit executable

/-- The conjunction of a loop annotation's `invariant` clauses over the
given local binders: the clause translation the contracts use, with no
runtime row, frame, or route-specific representation. Each clause is marked
with its source range, so an invariant not established is reported there.
The data invariants of the `dataInvariantSlots` (locals of a type declaring
them, which the body modifies) are conjoined: a value of the type satisfies
them wherever it is not under construction, so the loop owes them as it
owes an authored clause, and each iteration assumes them. A state anchor in
a clause reads the loop's entry: `entryLocals` and `entryState`. The
clauses of another `kind`, such as in-body assertions, translate alike. -/
def translateLoopInvariants (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace) (block : VerexIR.SpecBlock)
    (locals : Array (Option Lean.Expr)) (localTypes : Array IrTy)
    (types : Option Lean.Expr)
    (localNames : Array String) (oldLocals : Array (Option Lean.Expr))
    (oldState : Lean.Expr)
    (dataInvariantSlots : Array (Nat × VerexIR.Proofs.Denote.NTy) := #[])
    (entryLocals : Array (Option Lean.Expr) := #[]) (entryState : Option Lean.Expr := none)
    (state : Option Lean.Expr := none) (executable : Option Lean.Expr := none)
    (kind : VerexIR.ConditionKind := .loopInvariant)
    (labeledAnchors : Array (Nat × Array (Option Lean.Expr) × Lean.Expr) := #[]) :
    MetaM Lean.Expr := do
  let context : Context := {
    unit, namespaceId, ns, locals, oldLocals, localTypes, localNames, results := #[],
    types, state, oldState := some oldState,
    anchorLocals := entryLocals, anchorState := entryState, labeledAnchors, executable }
  let mut clauses ← block.conditions.filterMapM fun condition => do
    if condition.kind != kind then return none
    -- A split directs the cases on a Boolean; one on an enum is no step.
    if condition.kind == .split then
      let isBoolean := (ns.expressions[condition.expression.index]?).any fun expression =>
        unit.tables.types[expression.typeId.index]? == some .bool
      unless isBoolean do return none
      return some (markObligation (conditionRange unit condition)
        (← mkAppM ``VerexIR.Proofs.CaseSplit #[← translate context condition.expression]))
    some <$> markObligation (conditionRange unit condition) <$>
      translate context condition.expression
  -- A slot of a type carrying a data invariant binds its native value.
  for (slot, τ) in dataInvariantSlots do
    let some (some value) := locals[slot]? | throwError "a data invariant slot has no binder"
    let some ty := localTypes[slot]? | throwError "a data invariant slot has no type"
    for (invariant, range) in ← dataInvariantTerms context ty τ value do
      clauses := clauses.push (markObligation range invariant)
  conjunction clauses

/-- The data invariants of a native value of a physical type and its
native type, at any depth, as one conjunction of clauses marked with their
source ranges: what a construction owes of the value it makes. -/
def valueDataInvariants (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace) (types : Option Lean.Expr)
    (ty : IrTy) (τ : VerexIR.Proofs.Denote.NTy)
    (value state : Lean.Expr) (executable : Option Lean.Expr) : MetaM Lean.Expr := do
  let context : Context := {
    unit, namespaceId, ns, locals := #[], oldLocals := #[], localTypes := #[],
    localNames := #[], results := #[], types, state := some state,
    oldState := some state, executable }
  conjunction ((← dataInvariantTerms context ty τ value).map fun (clause, range) =>
    markObligation range clause)

/-- The native value of type `τ` a specification expression denotes over a
function's locals and state: what an `update` writes
(`doc/designs/ghost-fields.md`). A read under `old` has no binder here. -/
def translateUpdateValue (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace) (expression : ExprId)
    (locals : Array (Option Lean.Expr)) (localTypes : Array IrTy) (types : Option Lean.Expr)
    (localNames : Array String) (state : Lean.Expr)
    (executable : Option Lean.Expr) (τ : VerexIR.Proofs.Denote.NTy) : MetaM Lean.Expr := do
  let context : Context := {
    unit, namespaceId, ns, locals, oldLocals := locals.map fun _ => none, localTypes,
    localNames, results := #[], types, state := some state, executable }
  nativeOfClause (← context.carriers) τ (← translate context expression)

/-- The native value of type `σ` a ghost field's initializer gives a value
packed at a nominal type, over the packed native value of type `τ` and the
state (`doc/designs/ghost-fields.md`). -/
def translateGhostInitializer (unit : ValidatedUnit) (types : Option Lean.Expr)
    (ty : IrTy) (_variant : Option String)
    (initializer : ExprId) (τ σ : VerexIR.Proofs.Denote.NTy) (value state : Lean.Expr)
    (executable : Option Lean.Expr) : MetaM Lean.Expr := do
  let some (owner, ns, declaration) := nominalDeclaration? unit ty
    | throwError "a ghost initializer's type is not a declared nominal"
  let .nominal _ uses := ty | throwError "a ghost initializer's type is not a declared nominal"
  let scope := Scoped.ofNominal #[] uses
  let localTypes ← declaration.locals.mapM fun localDecl => do
    let some localType := unit.tables.types[localDecl.type.typeId.index]?
      | throwError "a ghost initializer's local type is out of range"
    pure localType
  let typeArgumentTypes := scope.map (Scoped.toNative? unit owner.namespaceId)
  let context : Context := {
    unit, namespaceId := owner.namespaceId, ns, locals := #[], localTypes, results := #[],
    types, state := some state, executable, typeArgumentTypes }
  let locals ← declarationLocalValues unit owner ns declaration localTypes (← context.carriers) τ
    value
  let context := { context with locals, oldLocals := locals.map fun _ => none }
  nativeOfClause (← context.carriers) σ (← translate context initializer)

/-- Whether a quoted native row holds only scalars and type parameters, and
at least one type parameter: a row an unseen invocation is decided at only
where the frame resolves it to scalars (`ScalarAt`). -/
private partial def mentionsInvocableParameter (row : Lean.Expr) : Bool :=
  go row false
where
  go (row : Lean.Expr) (mentioned : Bool) : Bool :=
    if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 then
      let τ := row.getArg! 0
      if τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.param 1 then go (row.getArg! 1) true
      else if τ.isConstOf ``VerexIR.Proofs.Denote.NTy.bool ||
          τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.int 2 ||
          τ.isConstOf ``VerexIR.Proofs.Denote.NTy.address ||
          τ.isConstOf ``VerexIR.Proofs.Denote.NTy.signer ||
          τ.isConstOf ``VerexIR.Proofs.Denote.NTy.string ||
          τ.isConstOf ``VerexIR.Proofs.Denote.NTy.bytes then
        go (row.getArg! 1) mentioned
      else false
    else mentioned

/-- The quoted native signature a contract is stated over. -/
structure NativeSignature where
  /-- The skolem family the carriers are taken at. -/
  skolems : Lean.Expr
  /-- The parameter row. -/
  params : Lean.Expr
  /-- Each parameter's native type, in order. -/
  argumentTypes : Array Lean.Expr
  /-- The result shape. -/
  shape : Lean.Expr
  /-- The result's native type, when the function has a result. -/
  resultType : Option Lean.Expr := none
  /-- The component types of a tuple result. -/
  resultComponentTypes : Array Lean.Expr := #[]

/-- Whether a quoted native type is a mutable reference. -/
private def isReferenceType (ty : Lean.Expr) : Bool :=
  ty.isAppOfArity ``VerexIR.Proofs.Denote.NTy.ref 1

/-- The current value and the prophecy of a native reference to `referent`. -/
private def referenceParts (skolems referent value : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  let carrier := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) (← frameCarriers skolems) referent
  return (mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) carrier carrier value,
    mkApp3 (mkConst ``Prod.snd [Level.zero, Level.zero]) carrier carrier value)

/-- The clause binders of the parameters, read off the native arguments.  A
mutable reference's entry is its current value and its exit its prophecy. -/
private def parameterBinders (skolems : Lean.Expr) (slots : Array Slot) (types : Array Lean.Expr)
    (arguments : Lean.Expr) : MetaM (Array SlotBinders) := do
  let values ← rowProjections skolems arguments types
  unless slots.size == types.size do
    throwError "internal: a parameter row differs from its declaration"
  (slots.zip (types.zip values)).mapM fun (slot, ty, value) => do
    let domain := domainOf slot.physical
    match slot.kind with
    | .mutableRef =>
        let (current, prophecy) ← referenceParts skolems ty.appArg! value
        return { entry := ← domain.binderOfNative current, entryNative := current
                 exit := some (← domain.binderOfNative prophecy), exitNative := some prophecy }
    | .plain | .sharedRef =>
        return { entry := ← domain.binderOfNative value, entryNative := value }

/-- Whether the expression tree at `root` reads a returned reference's final
value. -/
def mentionsFinal (ns : ValidatedNamespace) (root : ExprId) : Bool := Id.run do
  let mut pending := #[root]
  let mut visited : Array Nat := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if let .operation (.specification .final) .. := expression.kind then return true
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  return false

/-- A function's declared precondition over native values of its
arguments, as `requires_of` reads a closure site's target: its `requires`
clauses at a row of its arguments at the frame `frameOf` gives (the base
frame, or the frame a generic target's instantiation induces), each
parameter read from its argument, a reference as the value it passes. A
function of the unit, its function values, the arguments and the state. -/
def buildDeclaredRequires (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (frameOf : Lean.Expr → Lean.Expr → MetaM Lean.Expr) :
    MetaM Lean.Expr := do
  let groups ← groupConditions unit declaration.contract.conditions
  let localTypes ← declaration.locals.mapM fun localDecl => do
    let some ty := unit.tables.types[(specTypeId unit localDecl.type.typeId).index]?
      | throwError "function local has an unknown type"
    pure ty
  let parameterTypes ← declaration.signature.parameters.mapM fun parameter => do
    let some nty := VerexIR.Proofs.Denote.ntyOf unit namespaceId parameter.typeUse.typeId
      | throwError "a parameter has no native type"
    pure nty
  let row ← quoteRow (VerexIR.Proofs.Denote.NRow.ofList parameterTypes.toList)
  withUnitVariable .default fun unitExpr functions => do
    let frame ← frameOf unitExpr functions
    let carriers ← frameCarriers frame
    let argumentsType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) carriers row
    withLocalDeclD `args argumentsType fun args =>
    withLocalDeclD `state (mkApp2 (mkConst ``VerexIR.Proofs.Denote.Memory) unitExpr functions)
      fun state => do
      let natives ← rowProjections frame args (rowElementTypes row)
      let mut locals : Array (Option Lean.Expr) := Array.replicate declaration.locals.size none
      for (parameter, nty, nativeValue, index) in
          ((declaration.signature.parameters.zip parameterTypes).zip natives).zipIdx.map
            fun (((parameter, nty), nativeValue), index) => (parameter, nty, nativeValue, index) do
        let some ty := unit.tables.types[(specTypeId unit parameter.typeUse.typeId).index]?
          | throwError "a parameter has an unknown type"
        let current ← match nty with
          | .ref referent => do
              let carrier := carrierType carriers (← quoteNTy referent)
              pure (mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) carrier carrier nativeValue)
          | _ => pure nativeValue
        locals := locals.set! index (some (← (domainOf ty).binderOfNative current))
      let context : Context :=
        { unit, namespaceId, ns, locals, localTypes
          localNames := declaration.locals.map (·.name)
          oldLocals := locals, results := #[]
          types := some (← frameTypes frame)
          state := some state, oldState := some state }
      let context ← bindLets context groups.lets false
      let clauses ← groups.requires.mapM (translate context ·.1)
      mkLambdaFVars #[unitExpr, functions, args, state] (← conjunction clauses)

/-- Whether a function's body is verified: it has one, and neither its
specification nor its module sets `pragma verify = false`. -/
private def verifiedBody (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Bool :=
  declaration.body != .absent &&
    !(declaration.pragmas.any fun
      | .assign "verify" (.constant (.bool false)) _ => true
      | _ => false)

/-- The functions a declaration's body calls. -/
private def directCallees (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Array VerexIR.FunctionHandle := Id.run do
  let .structured root := declaration.body | return #[]
  let mut pending := #[root]
  let mut visited : Array Nat := #[]
  let mut callees := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if let .operation (.call (.function reference)) .. := expression.kind then
      if let some handle := VerexIR.SemanticOperations.resolveFunction? unit namespaceId reference then
        callees := callees.push handle
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  return callees

/-- Behavioral predicates in the data invariants a body must use or establish,
including constructed local values and stored resources. They need the same
proof assumptions as predicates written in the function's own contract. -/
private def invariantNeeds (unit : ValidatedUnit)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (needed : SpecReads → Bool) : Bool := Id.run do
  let carries (id : VerexIR.TypeId) : Bool :=
    match ns.tables.types[id.index]? with
    | some (.reference reference) =>
        (ns.tables.types[reference.referent.index]?).any
          (carriesMatchingInvariant unit needed · #[] 0)
    | some ty => carriesMatchingInvariant unit needed ty #[] 0
    | none => false
  if declaration.locals.any (carries ·.type.typeId) ||
      declaration.signature.parameters.any (carries ·.typeUse.typeId) ||
      declaration.signature.results.any (carries ·.typeId) then return true
  let .structured root := declaration.body | return false
  let mut pending := #[root]
  let mut visited : Array Nat := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if carries expression.typeId then return true
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  return false

/-- What the specifications inside a verified body read: its loop
invariants, assertions, and assumptions. -/
private def bodyReads (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : SpecReads :=
  match declaration.body with
  | .structured root => if verifiedBody declaration then readsFrom unit [(namespaceId, root)] else {}
  | .absent => {}

/-- Whether a function's theorem assumes the typing a run of a function
value it cannot see needs (`doc/historical/static-typing.md`, Phase 5): a body it
verifies with a parameter of function type, or calling, through verified
bodies, one that has; or a verified body whose contract or inner
specifications state `aborts_of` or `result_of`. -/
def assumesTyping (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool := Id.run do
  let mut pending := #[(namespaceId, ns, declaration)]
  let mut visited : Array (Nat × Nat) := #[]
  while let some (namespaceId, ns, declaration) := pending.back? do
    pending := pending.pop
    if visited.contains (namespaceId.index, declaration.name.index) then continue
    visited := visited.push (namespaceId.index, declaration.name.index)
    unless verifiedBody declaration do continue
    if invariantNeeds unit ns declaration (·.determinism) then return true
    if declaration.contract.conditions.any (fun condition =>
        (readsFrom unit [(namespaceId, condition.expression)]).determinism) then
      return true
    if (bodyReads unit namespaceId declaration).determinism then return true
    if declaration.signature.parameters.any (fun parameter =>
        ns.tables.types[parameter.typeUse.typeId.index]? matches some (.function ..)) then
      return true
    for callee in directCallees unit namespaceId ns declaration do
      let some calleeNs := unit.namespaces[callee.namespaceId.index]? | continue
      let some calleeDeclaration := calleeNs.functions[callee.functionId.index]? | continue
      pending := pending.push (callee.namespaceId, calleeNs, calleeDeclaration)
  return false

/-- Whether a function's theorem assumes that the unit's runs end
(`Terminating`): its contract or a verified body's inner specifications, or
those of a function it calls at any depth, state `result_of`, which a proof
resolves at a known function value through a run of it. -/
def assumesTermination (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool := Id.run do
  let mut pending := #[(namespaceId, ns, declaration)]
  let mut visited : Array (Nat × Nat) := #[]
  while let some (namespaceId, ns, declaration) := pending.back? do
    pending := pending.pop
    if visited.contains (namespaceId.index, declaration.name.index) then continue
    visited := visited.push (namespaceId.index, declaration.name.index)
    if invariantNeeds unit ns declaration (·.result) then return true
    if declaration.contract.conditions.any (fun condition =>
        (readsFrom unit [(namespaceId, condition.expression)]).result) then
      return true
    if (bodyReads unit namespaceId declaration).result then return true
    for callee in directCallees unit namespaceId ns declaration do
      let some calleeNs := unit.namespaces[callee.namespaceId.index]? | continue
      let some calleeDeclaration := calleeNs.functions[callee.functionId.index]? | continue
      pending := pending.push (callee.namespaceId, calleeNs, calleeDeclaration)
  return false

/-- Whether a condition states a behavioral predicate, itself or through
the specification functions it calls. -/
def conditionReadsBehavior (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (root : ExprId) : Bool :=
  let reads := readsFrom unit [(namespaceId, root)]
  reads.unit || reads.requires

/-- Whether a condition reads the table of declared preconditions
(`requires_of`), itself or through the specification functions it calls. -/
def conditionReadsRequires (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (root : ExprId) : Bool :=
  (readsFrom unit [(namespaceId, root)]).requires

/-- Whether a function's contract states a behavioral predicate or assumes
typing: then it takes the executable unit. -/
def contractReadsUnit (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  declaration.contract.conditions.any (conditionReadsBehavior unit namespaceId ·.expression) ||
    assumesTyping unit namespaceId ns declaration ||
    signatureInvariantsReadUnit unit ns declaration ||
    (storedInvariantReadsUnit unit && reachesMemory unit namespaceId declaration)

/-- Whether a function's precondition, its `requires` and the pre-state
`let`s they read, states a behavioral predicate. -/
def preconditionReadsUnit (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  declaration.contract.conditions.any fun condition =>
    (condition.kind matches .requires | .letPre _) &&
      conditionReadsBehavior unit namespaceId condition.expression

/-- What a function's contract reads, itself or through the specification
functions it calls. -/
private def contractReads (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : SpecReads :=
  readsFrom unit <| declaration.contract.conditions.toList.map
    fun condition => (namespaceId, condition.expression)

/-- Whether a function's contract binds a state label: then the solver keeps
program points as its witnesses. -/
def contractBindsStateLabel (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  (contractReads unit namespaceId declaration).labels

/-- Whether a function's contract states `requires_of`: then it also takes
the unit's table of declared preconditions. -/
def contractReadsRequires (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  (contractReads unit namespaceId declaration).requires

/-- Whether a function's contract relates a returned mutable reference's final
value to its lenders: then callers use the contract, not the body. -/
def hasFinalContract (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  declaration.contract.conditions.any fun condition =>
    condition.kind == .ensures && mentionsFinal ns condition.expression

/-- The clause binders of the result, read off the native result, with the
value view of every reference it returns: its prophecy is its current
value.  A clause sees a returned reference as its current value, and reads
its prophecy with `final`. Besides the binders: the premises of the value
view, each result's final value, and the components of a Move tuple result,
which `spec.result[i]` reads. -/
private def resultBinders (unit : ValidatedUnit) (slots : Array Slot)
    (signature : NativeSignature) (result : Lean.Expr) :
    MetaM (Array SlotBinders × Array Lean.Expr × Array (Option Lean.Expr) × Array Lean.Expr) := do
  match slots.toList, signature.resultType with
  | [], _ => return (#[], #[], #[], #[])
  | [slot], some ty =>
      let domain := domainOf slot.physical
      if slot.kind == .mutableRef then
        let (current, prophecy) ← referenceParts signature.skolems ty.appArg! result
        let final ← domain.binderOfNative prophecy
        return (#[{ entry := ← domain.binderOfNative current, entryNative := current
                    exit := some final, exitNative := some prophecy }],
          #[← mkEq prophecy current], #[some final], #[])
      let .tuple elements := slot.physical
        | return (#[{ entry := ← domain.binderOfNative result, entryNative := result }], #[],
            #[none], #[])
      -- A Move tuple: each component's binder, a returned reference's its
      -- current value, and its final value its prophecy.
      let values ← rowProjections signature.skolems result signature.resultComponentTypes
      unless values.size == elements.size do
        throwError "internal: a tuple result's components differ from its declaration"
      let mut components := #[]
      let mut premises := #[]
      let mut finals := #[]
      for ((componentType, value), element) in
          (signature.resultComponentTypes.zip values).zip elements do
        let componentDomain ← signatureDomain unit element "a result component"
        if isReferenceType componentType then
          let (current, prophecy) ← referenceParts signature.skolems componentType.appArg! value
          components := components.push (← componentDomain.binderOfNative current)
          premises := premises.push (← mkEq prophecy current)
          finals := finals.push (some (← componentDomain.binderOfNative prophecy))
        else
          components := components.push (← componentDomain.binderOfNative value)
          finals := finals.push none
      return (#[{ entry := result, entryNative := result }], premises, finals, components)
  | _, _ => throwError "internal: a result row differs from its declaration"

/-- The recursive specification functions of a unit that name a measure:
those with a definition. Without a measure there is no definition; a
contract using the function reports the missing measure. -/
def recursiveSpecFunctions (unit : ValidatedUnit) : Array VerexIR.QualifiedRef := Id.run do
  let mut found := #[]
  for ns in unit.namespaces, namespaceIndex in [0:unit.namespaces.size] do
    for declaration in ns.specFunctions do
      let some qualified := unit.tables.names[declaration.name.index]? | continue
      let reference : VerexIR.QualifiedRef := ⟨⟨namespaceIndex⟩, declaration.name⟩
      unless qualified.namespaceId == reference.namespaceId && declaration.body.isSome do continue
      unless declaration.contract.conditions.any (·.kind == .decreases) do continue
      unless specRecursive unit reference do continue
      found := found.push reference
  return found

/-- Define the recursive specification functions of a unit, so that lemmas
about a definition can precede the verification using it. A generic one is
defined at its own type parameters (`f.at_T0.spec`), the definition a
generic caller at its own parameters applies and every instance is
transported to across a call's frame (`ensureDefinitionTransport`); it is
defined at other type arguments where a call applies it. -/
def ensureSpecFunctionDefinitions (unit : ValidatedUnit) :
    MetaM Unit := do
  for reference in recursiveSpecFunctions unit do
    let some (ns, declaration) := specFunctionOf? unit reference | continue
    let parameters := (declaration.signature.generics.filter (·.kind == .typeArg)).size
    let context : Context := {
      unit, ns
      namespaceId := reference.namespaceId
      locals := #[]
      results := #[] }
    let _ ← translateExpression.ensureDefinitionAt context reference
      ((Array.range parameters).map fun index => some (VerexIR.Proofs.Denote.NTy.param index))

/-- Define a lemma's statement once (`ensureLemmaDefinitions`). -/
def ensureLemmaDefinitions (unit : ValidatedUnit)
    (reference : VerexIR.QualifiedRef) : MetaM LemmaNames := do
  let some ns := unit.namespaces[reference.namespaceId.index]?
    | throwError "a lemma's namespace is out of range"
  let context : Context := {
    unit, ns
    namespaceId := reference.namespaceId
    locals := #[]
    results := #[] }
  translateExpression.ensureLemmaDefinitions context reference

/-- The type of each parameter of a lemma's definitions: a scalar's clause
type, or the native type of one its definitions take at a family. -/
def lemmaParameterTypes (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    MetaM (Array (Lean.Expr ⊕ VerexIR.Proofs.Denote.NTy)) := do
  let some (_, declaration) := lemmaOf? unit reference
    | throwError "a lemma does not resolve"
  let natives := lemmaNativeParameters unit reference
  declaration.signature.parameters.mapIdxM fun index parameter => do
    if let some (_, nty) := natives.find? (·.1 == index) then return .inr nty
    let domain ← signatureDomain unit parameter.typeUse.typeId "a lemma parameter"
    let some type := domain.scalarType? | throwError "a lemma parameter has no native type"
    return .inl type

/-- What a lemma's statement reads besides its parameters: the executable
unit, the table of declared preconditions, the family, and the state, each
a binder of its definitions in this order where read. -/
def lemmaReadsOf (unit : ValidatedUnit) (reference : VerexIR.QualifiedRef) :
    Bool × Bool × Bool × Bool :=
  match lemmaOf? unit reference with
  | some (_, declaration) =>
      let reads := readsFrom unit <|
        (declaration.contract.conditions.toList ++ declaration.proof.toList).map
          fun condition => (reference.namespaceId, condition.expression)
      (reads.unit, reads.requires, lemmaTakesFrame unit reference reads.state, reads.state)
  | none => (false, false, false, false)

/-- The value of the uninterpreted function `name` at a native's type
parameters and argument binders, as a clause applying it denotes it. -/
private def modelApplication (context : Context)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (name : String)
    (parameterSlots : Array Slot) (values : Array Lean.Expr) (domain : Domain)
    (resultType : Option VerexIR.Proofs.Denote.NTy) : MetaM Lean.Expr := do
  let some types := context.types
    | throwError "the native model `{name}` is applied outside a family"
  let typeParameters := declaration.signature.generics.filter (·.kind == .typeArg) |>.size
  let typeArguments ← (List.range typeParameters).mapM fun index =>
    return mkApp (mkConst ``VerexSL.Contract.SpecTypeArgument.native)
      (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.substWith) types
        (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.param) (toExpr index)))
  -- A native argument at the type the function takes it at, with its
  -- integers unbounded, as a caller's clause applies it
  -- (`Context.uninterpretedParameterType`).
  let carriers ← context.carriers
  let arguments ← (parameterSlots.zip values).mapM fun (slot, value) => do
    let domain := domainOf slot.physical
    unless domain == .native do return (domain, value)
    let widened := widenedNTy slot.nty
    if widened == slot.nty then return (domain, value)
    return (domain, ← coerceNative carriers slot.nty widened value)
  opaqueApplication context name typeArguments domain resultType arguments

/-- A native's path-qualified name. -/
private def nativeName (context : Context)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : MetaM String := do
  let some path := context.unit.tables.namespaces[context.namespaceId.index]?
    | throwError "a native's namespace has no path"
  let some qualified := context.unit.tables.names[declaration.name.index]?
    | throwError "a native has no name"
  return "::".intercalate (path.segments.toList ++ [qualified.name])

/-- The result clauses of an uninterpreted native model: the result is the
model function's value at the native's type parameters and arguments, and a
vector result has the length the model states. None for a native without
exactly one result. -/
private def uninterpretedResult (context : Context)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (resultLength : Option Nat) (function : Option String)
    (parameterSlots : Array Slot) (parameterBound : Array SlotBinders)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders) : MetaM (Array Lean.Expr) := do
  let [resultSlot] := resultSlots.toList | return #[]
  let [resultBinders] := resultBound.toList | return #[]
  let name ← match function with
    | some name => pure name
    | none => nativeName context declaration
  let domain := domainOf resultSlot.physical
  let value ← modelApplication context declaration name parameterSlots
    (parameterBound.map (·.current)) domain (some resultSlot.nty)
  -- The uninterpreted value on the left: a second call at the same arguments
  -- rewrites to the first's result, and a clause naming the native closes.
  let mut clauses := #[← mkEq value resultBinders.entry]
  if let some length := resultLength then
    if resultSlot.nty matches .vector _ then
      clauses := clauses.push (← mkEq (← nativeLength resultBinders.entryNative)
        (toExpr (Int.ofNat length)))
  return clauses

/-- The result clause of the structural-order model: the result is the
ordering variant at the order of the two operands. The result is on the
left, as a defined model states what the result is. -/
private def orderResult (context : Context)
    (parameterSlots : Array Slot) (parameterBound : Array SlotBinders)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders) : MetaM Lean.Expr := do
  let [(leftSlot, left), (_, right)] := (parameterSlots.zip parameterBound).toList
    | throwError "a structural order compares two operands"
  let [resultSlot] := resultSlots.toList | throwError "a structural order has one result"
  let [resultBinders] := resultBound.toList | throwError "a structural order has one result"
  let order ← structuralOrderTerm context leftSlot.nty left.currentNative right.currentNative
  mkEq resultBinders.entryNative (← orderingTerm context resultSlot.physical order)

/-- The executable role a function plays for an intrinsic map of its
namespace with a carried model, by the name its binding declares. -/
def mapRoleName? (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Option String := do
  let target : VerexIR.QualifiedRef := { namespaceId, name := declaration.name }
  let intrinsic ← ns.intrinsics.find? fun intrinsic =>
    intrinsic.model == "map" && intrinsic.executableBindings.any (·.target == target)
  let binding ← intrinsic.executableBindings.find? (·.target == target)
  guard (mapModel? unit (.nominal intrinsic.owner #[])).isSome
  pure binding.role

/-- The executable role a function plays for an intrinsic map of its
namespace with a carried model. -/
def mapRoleOf? (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Option VerexIR.MapOperation :=
  (mapRoleName? unit namespaceId ns declaration).bind VerexIR.MapOperation.ofName?

/-- The intrinsic map owner whose representation a function's body reads:
the owner it constructs or destructures, whose declared fields or variants
it selects, tests, or updates, or through whose field a place of the body
projects; a table's declared fields are its abstract value's leading fields,
so a table is no such owner. In verification an owner is its abstract map
(`doc/designs/intrinsic-maps.md`, "Abstract representation"): such a body has
no denotation, so the function is used through its specification. Only the
owner's module can read its representation, by Move's visibility, which the
frontend enforces. -/
def representationRead? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Option VerexIR.QualifiedRef := Id.run do
  let .structured root := declaration.body | return none
  -- A table's declared fields are the leading fields of its abstract value.
  let owner? (reference : VerexIR.QualifiedRef) : Option VerexIR.QualifiedRef := do
    let ownerNs ← unit.namespaces[reference.namespaceId.index]?
    let structId ← ownerNs.structs.findIdx? (·.name == reference.name)
    let layout ← VerexIR.Validation.ownerLayout? unit reference.namespaceId structId
    guard (!(layout.shape matches .table _))
    some reference
  let rec placeOwner? (placeId : VerexIR.PlaceId) (fuel : Nat) : Option VerexIR.QualifiedRef :=
    match fuel, ns.places[placeId.index]? with
    | 0, _ | _, none | _ + 1, some (.localVar _) => none
    | fuel + 1, some (.field base reference _) | fuel + 1, some (.ghostField base reference _) =>
        owner? reference <|> placeOwner? base fuel
    | fuel + 1, some (.deref base) | fuel + 1, some (.index base _)
    | fuel + 1, some (.subslice base ..) | fuel + 1, some (.downcast base _) =>
        placeOwner? base fuel
  let mut pending := #[root]
  let mut visited : Array Nat := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.push id.index
    let some expression := ns.expressions[id.index]? | continue
    if let .operation operation .. := expression.kind then
      let read := match operation with
        | .call (.constructor reference _) | .call (.destructor reference _) => owner? reference
        | .data (.select reference _) | .data (.selectGhost reference _)
        | .data (.selectVariants reference _) | .data (.testVariants reference _)
        | .data (.discriminant reference) | .data (.updateField reference _) => owner? reference
        | .move place | .copy place | .borrow _ place | .read place | .write place
        | .drop place => placeOwner? place ns.places.size
        | _ => none
      if let some owner := read then return some owner
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  return none

/-- The model of the map a role function's namespace owns. -/
private def mapRoleModel (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : MetaM MapModel := do
  let target : VerexIR.QualifiedRef := { namespaceId, name := declaration.name }
  let some intrinsic := ns.intrinsics.find? fun intrinsic =>
      intrinsic.model == "map" && intrinsic.executableBindings.any (·.target == target)
    | throwError "a map role function has no intrinsic declaration"
  let some model := mapModel? unit (.nominal intrinsic.owner #[])
    | throwError "a map role function's owner has no carried representation"
  return model

/-- Whether a function allocates a table: it is the `new` role of a table
owner with a handle, or calls one at any depth. The allocation history is
then in its frame (`doc/designs/intrinsic-maps.md`, "Owned Table values"). -/
def allocatesTables (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
  let allocator (namespaceId : VerexIR.NamespaceId) (ns : ValidatedNamespace)
      (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) : Bool :=
    mapRoleOf? unit namespaceId ns declaration == some .new_ &&
      (Id.run do
        let target : VerexIR.QualifiedRef := { namespaceId, name := declaration.name }
        let some intrinsic := ns.intrinsics.find? fun intrinsic =>
            intrinsic.model == "map" && intrinsic.executableBindings.any (·.target == target)
          | return false
        return (mapModel? unit (.nominal intrinsic.owner #[])).any (·.handle))
  allocator namespaceId ns declaration ||
    let (found, complete) := reachFold unit
      ((functionRoots declaration).toList.map (namespaceId, ·)) false fun found operation _ =>
        found || match operation with
          | .call (.function reference) =>
              (do
                let calleeNs ← unit.namespaces[reference.namespaceId.index]?
                let functionId ← unit.resolution.function? reference.name
                let callee ← calleeNs.functions[functionId.index]?
                pure (allocator reference.namespaceId calleeNs callee)).getD false
          | _ => false
    found || !complete

/-- The native value of an optional result at its native type: present with
a native value, or empty, as its declared layout holds it, a vector wrapper
or an enum with empty and single-field variants. -/
private def optionalValue (context : Context) (slot : Slot) (value : Option Lean.Expr) :
    MetaM Lean.Expr := do
  let some (_, ns, declaration) := nominalDeclaration? context.unit slot.physical
    | throwError "an optional map result is not an `Option`"
  let some layout := VerexIR.SemanticOperations.optionalLayout? ns declaration
    | throwError "an optional map result must be a vector wrapper or an enum with empty and \
        single-field variants"
  let carriers ← context.carriers
  match layout, slot.nty with
  | .vector, .struct _ _ (.cons vectorType@(.vector element) _) =>
      let elements ← value.toArray.mapM (clauseOfNative element)
      nativeConstructed slot.nty none carriers #[← nativeVectorLiteral carriers vectorType elements]
  | .variants empty present, .enum _ _ names rows _ =>
      match value with
      | none => nativeConstructed slot.nty (some empty) carriers #[]
      | some held =>
          let some row := VerexIR.Proofs.Denote.NRows.variant? names rows present
            | throwError "internal: an optional's present variant is not its type's"
          let some payload := row.toList.head?
            | throwError "internal: an optional's present variant carries no value"
          nativeConstructed slot.nty (some present) carriers #[← clauseOfNative payload held]
  | _, _ => throwError "internal: an optional map result's native type is not its layout's"

/-- A role's parameters, with their binders at entry and exit, the model of
its map, and the carriers its values are at. -/
private structure RoleOperands where
  role : VerexIR.MapOperation
  model : MapModel
  carriers : Lean.Expr
  slots : Array Slot
  bound : Array SlotBinders

namespace RoleOperands

private def parameter (operands : RoleOperands) (index : Nat) : MetaM (Slot × SlotBinders) := do
  let (some slot, some binders) := (operands.slots[index]?, operands.bound[index]?)
    | throwError m!"the intrinsic map role `{operands.role.name}` lacks parameter {index}"
  pure (slot, binders)

/-- A parameter's clause value at entry, in its own domain. -/
private def clause (operands : RoleOperands) (index : Nat) : MetaM Lean.Expr := do
  pure (← operands.parameter index).2.entry

/-- The owner a parameter holds at entry. -/
private def owner (operands : RoleOperands) (index : Nat) : MetaM NativeOwner := do
  let (slot, binders) ← operands.parameter index
  nativeOwner operands.carriers slot.nty binders.entryNative

/-- The owner a mutated parameter holds at exit. -/
private def exitOwner (operands : RoleOperands) (index : Nat) : MetaM NativeOwner := do
  let (slot, binders) ← operands.parameter index
  let some exit := binders.exitNative
    | throwError m!"the intrinsic map role `{operands.role.name}` does not mutate parameter {index}"
  nativeOwner operands.carriers slot.nty exit

/-- A parameter's entry value as a view's typed value. -/
private def typed (operands : RoleOperands) (view : VerexIR.Proofs.Denote.NativeMaps.TypedView)
    (index : Nat) : MetaM Lean.Expr := do
  TypedTable.typedOfNative view (← operands.parameter index).2.entryNative

/-- A vector parameter's elements as a typed list. -/
private def list (operands : RoleOperands) (index : Nat) : MetaM Lean.Expr := do
  let (slot, binders) ← operands.parameter index
  return (← typedListOf operands.carriers (← quoteNTy slot.nty) binders.entryNative).2

end RoleOperands

/-- The abort condition of an intrinsic map role over the entry binders
(`doc/designs/intrinsic-maps.md`, "Roles"), `none` when it never aborts. -/
private def mapRoleAborts (context : Context) (operands : RoleOperands) :
    MetaM (Option Lean.Expr) := do
  match operands.role with
  | .newFrom =>
      return some (← mkAppM ``VerexIR.SpecMap.AbortsNewFrom
        #[← operands.list 0, ← operands.list 1])
  | .newWithConfig =>
      -- A nonzero degree outside its range.
      let degree (index : Nat) (least : Int) : MetaM Lean.Expr := do
        let value ← operands.clause index
        mkAppM ``And #[← mkAppM ``Ne #[value, mkIntLit 0], ← mkAppM ``Or
          #[← mkAppM ``LT.lt #[value, mkIntLit least], ← mkAppM ``LT.lt #[mkIntLit 4096, value]]]
      return some (← mkAppM ``Or #[← degree 0 4, ← degree 1 3])
  | .iterBorrowMut =>
      -- The iterator is parameter 0, the map parameter 1.
      let (iteratorSlot, iterator) ← operands.parameter 0
      let some model := iteratorModel? context.unit iteratorSlot.physical
        | throwError "an iterator of an intrinsic map has no key or position variant"
      let owner ← operands.owner 1
      return some (mkNot (← iteratorInRange model iteratorSlot.nty operands.carriers
        iterator.entryNative owner.typed))
  | .new_ | .len | .isEmpty | .hasKey | .borrowWithDefault | .get
  | .borrowMutWithDefault | .addOverrideIfExists | .upsert
  | .removeOrNone | .destroyUnchecked | .append | .prevKey
  | .nextKey | .keys | .values | .toVecPair | .toOrderedMap => return none
  | .borrow | .borrowMut | .delMustExist | .delReturnKey | .addNoOverride | .destroyEmpty
  | .borrowFront | .borrowBack | .frontKey | .backKey | .popFront | .popBack | .trim
  | .addAll | .upsertAll | .appendDisjoint | .replaceKeyInplace => pure ()
  let table := (← operands.owner 0).typed
  let contains (present : Bool) : MetaM (Option Lean.Expr) := do
    let key ← operands.typed table.keys 1
    return some (← mkEq (← table.hasKey key) (toExpr present))
  match operands.role with
  | .borrow | .borrowMut | .delMustExist | .delReturnKey => contains false
  | .addNoOverride => contains true
  | .destroyEmpty => return some (← mkAppM ``Ne #[← table.size, mkIntLit 0])
  | .borrowFront | .borrowBack | .frontKey | .backKey | .popFront | .popBack =>
      return some (← mkEq (← table.size) (mkIntLit 0))
  | .trim => return some (← mkAppM ``LT.lt #[← table.size, ← operands.clause 1])
  | .addAll =>
      return some (← mkAppOptM ``VerexIR.SpecMap.AbortsAddAll
        #[table.keys.carrier, table.values.carrier, table.keys.decEq, table.map,
          ← operands.list 1, ← operands.list 2])
  | .upsertAll =>
      return some (← mkAppM ``VerexIR.SpecMap.AbortsUpsertAll
        #[← operands.list 1, ← operands.list 2])
  | .appendDisjoint =>
      let other := (← operands.owner 1).typed
      return some (← mkAppOptM ``VerexIR.SpecMap.AbortsAppendDisjoint
        #[table.keys.carrier, table.values.carrier, table.keys.decEq, table.map, other.map])
  | .replaceKeyInplace =>
      return some (← mkAppOptM ``VerexIR.SpecMap.AbortsReplaceKey
        #[table.keys.carrier, table.values.carrier, table.keys.decEq,
          ← table.discipline operands.model, table.map, ← operands.typed table.keys 1,
          ← operands.typed table.keys 2])
  | _ => return none

/-- The roles returning a mutable reference into the map, whose ensures state
the map at the reference's final value. -/
private def mapRoleReadsFinal (role : VerexIR.MapOperation) : Bool :=
  role == .borrowMut || role == .borrowMutWithDefault ||
    role == .iterBorrowMut

/-- The allocation of a table at `handle` recorded: the handle is not in the
allocation history at entry, and the history at exit appends it. -/
private def allocationRecorded (context : Context) (handle : Lean.Expr) : MetaM Lean.Expr := do
  let some final := context.state
    | throwError "the allocation of a table has no exit memory"
  let some entry := context.oldState
    | throwError "the allocation of a table has no entry memory"
  let allocation := mkConst ``VerexIR.Proofs.Denote.TableMemory.allocationResource
  let historyKey := mkConst ``VerexIR.StorageKey.unit
  let history (memory : Lean.Expr) : Lean.Expr := mkApp2 memory allocation historyKey
  withLocalDeclD `history (← mkAppM ``Array #[mkConst ``String]) fun allocated => do
    let held ← mkEq (history entry) (← mkAppOptM ``Option.some #[none, allocated])
    let fresh ← mkAppM ``Not #[← mkAppM ``Membership.mem #[allocated, handle]]
    let appended ← mkEq (history final)
      (← mkAppOptM ``Option.some #[none, ← mkAppM ``Array.push #[allocated, handle]])
    mkForallFVars #[allocated] (← mkArrow held (← mkAppM ``And #[fresh, appended]))

/-- The empty entries of a native owner's table. -/
private def emptyEntries (owner : NativeOwner) : MetaM Lean.Expr := do
  emptyTableOf (← inferType owner.table)

/-- The result and effect clauses of an intrinsic map role, over the entry
binders, the exit binders of a mutated map, and the result binders. A value
the role builds replaces its result; an observation of the map is on the
left, so that a clause naming it speaks about the result. A role building
or structurally mutating a map leaves its ghost slots open, fresh; a value
write keeps them. -/
private def mapRoleEnsures (context : Context) (operands : RoleOperands)
    (resultSlots : Array Slot) (resultBound : Array SlotBinders) : MetaM (Array Lean.Expr) := do
  let role := operands.role
  let model := operands.model
  let carriers := operands.carriers
  let result : MetaM (Slot × SlotBinders) := do
    let (some slot, some binders) := (resultSlots[0]?, resultBound[0]?)
      | throwError m!"the intrinsic map role `{role.name}` lacks its result"
    pure (slot, binders)
  -- The result's value as a view's typed value, on the right.
  let observed (view : VerexIR.Proofs.Denote.NativeMaps.TypedView) (value : Lean.Expr) :
      MetaM Lean.Expr := do
    mkEq value (← TypedTable.typedOfNative view (← result).2.entryNative)
  let built (value : Lean.Expr) : MetaM Lean.Expr := do
    mkEq (← result).2.entryNative value
  -- The final value of a returned mutable reference as a view's typed value.
  let final (view : VerexIR.Proofs.Denote.NativeMaps.TypedView) : MetaM Lean.Expr := do
    let some final := (← result).2.exitNative
      | throwError m!"the intrinsic map role `{role.name}` returns no mutable reference"
    TypedTable.typedOfNative view final
  -- An optional result, present with a typed value where `condition` holds.
  let optional (view : VerexIR.Proofs.Denote.NativeMaps.TypedView) (value condition : Lean.Expr) :
      MetaM Lean.Expr := do
    let (slot, binders) ← result
    let present ← optionalValue context slot (some (TypedTable.nativeOf view value))
    let empty ← optionalValue context slot none
    mkEq (← mkAppOptM ``ite #[none, condition, none, present, empty]) binders.entryNative
  -- A pair result of a typed key and value.
  let pair (table : TypedTable) (key value : Lean.Expr) : MetaM Lean.Expr := do
    let (slot, _) ← result
    let .tuple row := slot.nty
      | throwError m!"the result of `{role.name}` is not a pair"
    built (← nativeValueRow carriers row
      [TypedTable.nativeOf table.keys key, TypedTable.nativeOf table.values value])
  -- A vector result holding typed values of a view, in order.
  let listed (view : VerexIR.Proofs.Denote.NativeMaps.TypedView) (values vector : Lean.Expr) :
      MetaM Lean.Expr := do
    let unread ← mkAppM ``List.map #[view.unread, values]
    mkEq unread (← mkAppM ``Array.toList #[← mkAppM ``VerexIR.SpecVector.values #[vector]])
  -- What a value write keeps: the slots of the map at entry.
  let sameGhosts (exit entry : NativeOwner) : MetaM Lean.Expr := do
    conjunction (← (exit.ghosts.zip entry.ghosts).mapM fun (left, right) => mkEq left right)
  -- What every mutation keeps of the leading fields: the handle, and the
  -- cached length as the map's size.
  let leadingKept (exit entry : NativeOwner) : MetaM (Array Lean.Expr) := do
    let mut kept := #[]
    if let (some left, some right) := (exit.handle, entry.handle) then
      kept := kept.push (← mkEq left right)
    if let some (_, length) := exit.length then
      kept := kept.push (← mkEq (← mkAppM ``VerexIR.SpecInt.val #[length]) (← exit.typed.size))
    return kept
  -- A role allocating a table: its result holds no entries, at a fresh
  -- handle the allocation records; its slots are fresh.
  if role == .new_ || role == .newWithConfig then
    let (slot, binders) ← result
    let built ← nativeOwner carriers slot.nty binders.entryNative
    let mut facts := #[← mkEq built.table (← emptyEntries built)]
    if let some (_, length) := built.length then
      facts := facts.push (← mkEq (← mkAppM ``VerexIR.SpecInt.val #[length]) (mkIntLit 0))
    -- A placeholder `dummy_field` is `false`, as an empty struct's
    -- (`doc/designs/intrinsic-maps.md`, "Table layout").
    if let some placeholder := built.handle then
      if (← whnfR (← inferType placeholder)).isConstOf ``Bool then
        facts := facts.push (← mkEq placeholder (mkConst ``Bool.false))
    let (some handle, true) := (built.handle, model.handle)
      | return #[← conjunction facts]
    let clause ← withLocalDeclD `handle (mkConst ``String) fun fresh => do
      mkLambdaFVars #[fresh] (← mkAppM ``And
        #[← conjunction (#[← mkEq handle fresh] ++ facts), ← allocationRecorded context fresh])
    return #[← mkAppM ``Exists #[clause]]
  -- A role building a map: its result's map the typed lists set into the
  -- empty map, its slots fresh.
  if role == .newFrom then
    let (slot, binders) ← result
    let built ← nativeOwner carriers slot.nty binders.entryNative
    let table := built.typed
    let map ← table.updateAll (← table.discipline model) (← table.empty) (← operands.list 0)
      (← operands.list 1)
    let mut facts := #[← mkEq table.map map]
    if let some (_, length) := built.length then
      facts := facts.push (← mkEq (← mkAppM ``VerexIR.SpecInt.val #[length]) (← table.size))
    return facts
  -- The owner: parameter 0, or parameter 1 after an iterator.
  let ownerIndex := if role == .iterBorrowMut then 1 else 0
  let entry ← operands.owner ownerIndex
  let table := entry.typed
  let discipline ← table.discipline model
  let exit : MetaM NativeOwner := operands.exitOwner ownerIndex
  let key : MetaM Lean.Expr := operands.typed table.keys 1
  let value : MetaM Lean.Expr := operands.typed table.values 2
  let present (key : Lean.Expr) : MetaM Lean.Expr := do
    mkEq (← table.hasKey key) (mkConst ``Bool.true)
  let endKey : MetaM Lean.Expr := do
    let front := role == .borrowFront || role == .frontKey || role == .popFront
    let position ← if front then pure (mkIntLit 0)
      else mkAppM ``HSub.hSub #[← table.size, mkIntLit 1]
    table.keyAt position
  match role with
  | .len =>
      let (_, binders) ← result
      return #[← mkEq (← table.size) binders.entry]
  | .isEmpty =>
      let (_, binders) ← result
      return #[← mkEq (← mkDecide (← mkEq (← table.size) (mkIntLit 0))) binders.entryNative]
  | .hasKey =>
      let (_, binders) ← result
      return #[← mkEq (← table.hasKey (← key)) binders.entryNative]
  | .borrow => return #[← observed table.values (← table.valueAt (← key))]
  | .borrowWithDefault =>
      let key ← key
      return #[← observed table.values
        (← mkAppOptM ``ite #[none, ← present key, none, ← table.valueAt key, ← value])]
  | .get =>
      let key ← key
      return #[← optional table.values (← table.valueAt key) (← present key)]
  | .upsert | .addOverrideIfExists | .addNoOverride =>
      let (exit, key, value) := (← exit, ← key, ← value)
      let written ← mkEq exit.typed.map (← table.update discipline key value)
      let kept ← leadingKept exit entry
      if role == .addNoOverride then return #[written] ++ kept
      let condition ← present key
      let same ← mkArrow condition (← sameGhosts exit entry)
      if role == .upsert then
        return #[← optional table.values (← table.valueAt key) condition, written, same] ++ kept
      return #[written, same] ++ kept
  | .delMustExist | .removeOrNone | .delReturnKey =>
      let (exit, key) := (← exit, ← key)
      let removed ← mkEq exit.typed.map (← table.remove discipline key)
      let read ← match role with
        | .delMustExist => observed table.values (← table.valueAt key)
        | .removeOrNone => do optional table.values (← table.valueAt key) (← present key)
        | _ => do pair table key (← table.valueAt key)
      return #[read, removed] ++ (← leadingKept exit entry)
  | .destroyEmpty | .destroyUnchecked => return #[]
  | .borrowFront | .borrowBack =>
      let key ← endKey
      return #[← pair table key (← table.valueAt key)]
  | .frontKey | .backKey => return #[← observed table.keys (← endKey)]
  | .popFront | .popBack =>
      let (exit, key) := (← exit, ← endKey)
      return #[← pair table key (← table.valueAt key),
        ← mkEq exit.typed.map (← table.remove discipline key)] ++ (← leadingKept exit entry)
  | .borrowMut =>
      let (exit, key) := (← exit, ← key)
      return #[← observed table.values (← table.valueAt key),
        ← mkEq exit.typed.map (← table.update discipline key (← final table.values)),
        ← sameGhosts exit entry] ++ (← leadingKept exit entry)
  | .borrowMutWithDefault =>
      let (exit, key) := (← exit, ← key)
      let condition ← present key
      return #[← observed table.values
          (← mkAppOptM ``ite #[none, condition, none, ← table.valueAt key, ← value]),
        ← mkEq exit.typed.map (← table.update discipline key (← final table.values)),
        ← mkArrow condition (← sameGhosts exit entry)] ++ (← leadingKept exit entry)
  | .iterBorrowMut =>
      -- The iterator is parameter 0: its key read by the keys' view, or the
      -- key at its position.
      let exit ← exit
      let (iteratorSlot, iterator) ← operands.parameter 0
      let some iteratorModel := iteratorModel? context.unit iteratorSlot.physical
        | throwError "an iterator of an intrinsic map has no key or position variant"
      let key ← iteratorKey iteratorModel iteratorSlot.nty carriers iterator.entryNative table
      return #[← observed table.values (← table.valueAt key),
        ← mkEq exit.typed.map (← table.update discipline key (← final table.values)),
        ← sameGhosts exit entry] ++ (← leadingKept exit entry)
  | .addAll | .upsertAll =>
      let exit ← exit
      return #[← mkEq exit.typed.map (← table.updateAll discipline table.map
        (← operands.list 1) (← operands.list 2))] ++ (← leadingKept exit entry)
  | .append | .appendDisjoint =>
      let exit ← exit
      let other := (← operands.owner 1).typed
      return #[← mkEq exit.typed.map
          (← table.updateAll discipline table.map (← other.keyList) (← other.valueList))] ++
        (← leadingKept exit entry)
  | .trim =>
      let exit ← exit
      let (slot, binders) ← result
      let rest ← nativeOwner carriers slot.nty binders.entryNative
      let count ← operands.clause 1
      let mut facts := #[← mkEq rest.typed.map (← table.dropEntries count),
        ← mkEq exit.typed.map (← table.takeEntries count)]
      if let some (_, length) := rest.length then
        facts := facts.push (← mkEq (← mkAppM ``VerexIR.SpecInt.val #[length]) (← rest.typed.size))
      return facts ++ (← leadingKept exit entry)
  | .replaceKeyInplace =>
      let exit ← exit
      return #[← mkEq exit.typed.map
        (← table.replaceKey (← key) (← operands.typed table.keys 2))] ++ (← leadingKept exit entry)
  | .prevKey | .nextKey =>
      let neighbour ← table.neighbour model (role == .nextKey) (← key)
      let (slot, binders) ← result
      let empty ← optionalValue context slot none
      let present ← withLocalDeclD `neighbour table.keys.carrier fun neighbour => do
        mkLambdaFVars #[neighbour]
          (← optionalValue context slot (some (TypedTable.nativeOf table.keys neighbour)))
      return #[← mkEq (← mkAppOptM ``Option.elim #[none, none, neighbour, empty, present])
        binders.entryNative]
  | .keys => return #[← listed table.keys (← table.keyList) (← result).2.entryNative]
  | .values => return #[← listed table.values (← table.valueList) (← result).2.entryNative]
  | .toVecPair =>
      let (slot, binders) ← result
      let components ← nativeComponents carriers (← quoteNTy slot.nty) binders.entryNative
      let #[(_, keys), (_, values)] := components
        | throwError m!"the result of `{role.name}` is not a pair of vectors"
      return #[← listed table.keys (← table.keyList) keys,
        ← listed table.values (← table.valueList) values]
  | .toOrderedMap =>
      -- The same entries, in the result's representation; a result with a
      -- handle would be an allocation, which no conversion is.
      let (slot, binders) ← result
      let some resultModel := mapModel? context.unit slot.physical
        | throwError m!"the result of `{role.name}` has no carried representation"
      if resultModel.handle then
        throwError m!"the result of `{role.name}` is a table, which no conversion allocates"
      let converted := (← nativeOwner carriers slot.nty binders.entryNative).typed
      return #[← mkEq converted.map (← converted.updateAll (← converted.discipline resultModel)
        (← converted.empty) (← table.keyList) (← table.valueList))]
  | .new_ | .newWithConfig | .newFrom => return #[]

/-- Free state-label definitions, independent of clause order. Definitions
under a quantifier are local to that quantifier. -/
private def stateDefinitions (ns : ValidatedNamespace)
    (contract : VerexIR.FunctionContract) : Array StateDefinition := Id.run do
  let mut pending := contract.conditions.map fun condition =>
    (condition.expression, (#[] : Array (VerexIR.PatternId × ExprId)))
  let mut visited : Array ExprId := #[]
  let mut definitions := #[]
  while let some (id, bindings) := pending.back? do
    pending := pending.pop
    if visited.contains id then continue
    visited := visited.push id
    let some expression := ns.expressions[id.index]? | continue
    if expression.kind matches .quantifier .. then continue
    if let .operation (.specification operation) _ _ _ := expression.kind then
      let post := match operation with
        | .publish range | .remove range | .update range
        | .behavior .ensuresOf range | .behavior .resultOf range => range.post
        | _ => none
      if let some label := post then
        definitions := definitions.push { label, operation := id, bindings }
    if let .letDecl pattern (some initializer) body := expression.kind then
      pending := pending.push (initializer, bindings) |>.push
        (body, bindings.push (pattern, initializer))
    else
      pending := pending ++ (VerexIR.Validation.expressionChildren expression.kind).map
        (·, bindings)
  return definitions

/-- The contract of a function over its native arguments and result. -/
def buildContract (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (signature : NativeSignature)
    (types : Option Lean.Expr := none)
    (typeInstantiation : Option Lean.Expr := none)
    (nativeModel : Option NativeModel := none)
    (executable : Option Lean.Expr := none)
    (requiresMeaning : Option Lean.Expr := none)
    (stored : StoredInvariants := {}) :
    MetaM Lean.Expr := do
  let uses := frameUses unit namespaceId declaration
  -- The data invariants of stored resources hold of the memory a function
  -- reaches, at entry and at exit, as the Move Prover assumes them at entry
  -- and checks them where a value is written.
  let reach := memoryReach unit namespaceId declaration
  let delegated := delegatesInvariants unit namespaceId declaration
  if let some resources := reach then
    for (handle, reason) in stored.unsupported do
      if resources.contains handle then
        let name := ((VerexIR.SemanticOperations.structName? unit handle).map (·.name)).getD
          s!"{handle.structId}"
        throwError "the data invariant of `{name}`, which this function reaches, is not \
          carried: {reason}"
  -- A function reaching a resource that carries one assumes them at entry;
  -- every other function that reaches memory keeps them, wherever they hold.
  let assumesStored := reach.any fun resources => resources.any stored.carriers.contains
  let storedInvariant := if reach.isSome then stored.predicate else none
  let memoryInvariants (state : Lean.Expr) : MetaM Lean.Expr := do
    let some invariant := storedInvariant | throwError "internal: no stored invariants"
    mkAppM ``VerexIR.Proofs.Denote.MemoryInvariants
      #[← storedInvariantAt unit invariant executable state, state]
  -- An intrinsic map role is read by the map model, not by its source
  -- specification (`doc/designs/intrinsic-maps.md`, "Roles"), but for the
  -- preconditions it declares, which its callers owe as the Move Prover
  -- asserts them.
  let mapRole ← (mapRoleName? unit namespaceId ns declaration).mapM fun name => do
    let some role := VerexIR.MapOperation.ofName? name
      | throwError m!"the intrinsic map role `{name}` is not carried"
    return (role, ← mapRoleModel unit namespaceId ns declaration)
  let declared ← groupConditions unit declaration.contract.conditions
  let groups := if mapRole.isSome then { requires := declared.requires, lets := declared.lets }
    else declared
  let invariantResources ← invariantModifiedResources ns declaration.contract
  -- A function's pragmas: its contract's, and those its module sets and its
  -- contract does not.
  let inEffect (name : String) : Bool := declaration.pragmas.any fun
    | .assign pragmaName (.constant (.bool true)) _ => pragmaName == name
    | _ => false
  let isPartial := pragmaEnabled declaration.contract "aborts_if_is_partial" ||
    inEffect "aborts_if_is_partial"
  -- A native without a specification is read by its Prover model
  -- (`NativeModel`): it aborts only as the model says, and its result is the
  -- value of the model's uninterpreted function at its arguments, so two
  -- calls at equal arguments agree and a clause naming the function speaks
  -- about the same value.
  let isStrict := pragmaEnabled declaration.contract "aborts_if_is_strict" ||
    inEffect "aborts_if_is_strict" || nativeModel.isSome || mapRole.isSome
  let parameterSlots ← declaration.signature.parameters.mapIdxM fun index parameter =>
    slotOf { unit := unit, namespaceId := namespaceId, ns := ns,
             locals := #[], results := #[], types := types,
             typeInstantiation := typeInstantiation }
      (Name.mkSimple (if parameter.name.isEmpty then s!"argument{index}" else parameter.name))
      parameter.typeUse.typeId (allowReference := true)
  let resultSlots ← declaration.signature.results.mapIdxM fun index result =>
    slotOf { unit := unit, namespaceId := namespaceId, ns := ns,
             locals := #[], results := #[], types := types,
             typeInstantiation := typeInstantiation }
      (Name.mkSimple (if declaration.signature.results.size == 1 then "result"
        else s!"result{index}"))
      result.typeId (allowReference := true)
  let functionLocalTypes ← declaration.locals.mapM fun localDecl => do
    let some ty := unit.tables.types[localDecl.type.typeId.index]?
      | throwError "function local has an unknown type"
    match ty with
    | .reference reference =>
        let some referent := unit.tables.types[reference.referent.index]?
          | throwError "function reference local has an unknown referent type"
        pure referent
    | _ => pure ty
  let padLocals (values : Array (Option Lean.Expr)) : Array (Option Lean.Expr) :=
    values ++ Array.replicate (declaration.locals.size - values.size) none
  let argumentsType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) ((← frameCarriers signature.skolems))
    signature.params
  let resultType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.ResultShape.carrier) ((← frameCarriers signature.skolems))
    signature.shape
  let runtimeState := mkAppN (mkConst ``VerexIR.Proofs.Denote.Memory)
    (← frameLeading signature.skolems)
  let failure := mkConst ``VerexIR.Proofs.Failure
  /- In a one-state clause a parameter denotes its entry value; in
  `ensures` it denotes the current value — the prophecy of a mutable
  reference — and `spec.old` reaches the entry. -/
  let labelDefinitions := stateDefinitions ns declaration.contract
  let contextOf (state : Lean.Expr) (bound : Array SlotBinders) : Context :=
    { unit, namespaceId, ns
      types := types, typeInstantiation := typeInstantiation
      locals := padLocals (bound.map fun binders => some binders.entry)
      localTypes := functionLocalTypes
      localNames := declaration.locals.map (·.name)
      oldLocals := padLocals (bound.map fun binders => some binders.entry)
      results := #[]
      state := some state, oldState := some state, executable, requiresMeaning
      labelDefinitions
      labelEntry := some { memory := state, locals := padLocals (bound.map (some ∘ SlotBinders.entry)) }
      labelExit := some { memory := state, locals := padLocals (bound.map (some ∘ SlotBinders.entry)) } }
  let requiresTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state => do
      let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
      let context ← bindLets (contextOf state bound) groups.lets false
      let dataInvariants ← slotDataInvariantTerms context parameterSlots bound
        (some ·.entryNative)
      -- Each clause marked, so that a caller's leaf owing it is keyed by it.
      let clauses ← groups.requires.mapM fun (clause, range) =>
        return markObligation range (← translate context clause)
      -- At entry the invariants over the memory the function reaches hold
      -- everywhere, as the Move Prover assumes them; only what a write
      -- owes is specialized to the keys it modifies.
      let invariants ← namespaceInvariantTerms context none .entry
        (invariantApplies reach unit delegated) uses.applied uses.memory
        (some (typeArity declaration)) (some uses.written)
      let storedAssumed ← if assumesStored && storedInvariant.isSome then
          pure #[← memoryInvariants state] else pure #[]
      let signers ← signerFacts context parameterSlots bound (some ·.entry)
      -- What a body applying hashes and serialization assumes: a role's
      -- meaning is the map model, which assumes none of it.
      let (hashes, serialized) ← if mapRole.isSome then pure (#[], #[]) else
        pure (← hashFacts context declaration parameterSlots bound,
          ← serializeFacts context declaration parameterSlots bound)
      let body ← conjunction (clauses ++ dataInvariants.map (·.1) ++ invariants.map (·.1) ++
        storedAssumed ++ signers ++ hashes ++ serialized)
      mkLambdaFVars #[arguments, state] body
  let ensuresTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `result resultType fun result =>
        withLocalDeclD `final runtimeState fun final => do
          let parameterBound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
          let (resultBound, prophecies, finals, components) ←
            resultBinders unit resultSlots signature result
          let context : Context :=
            { unit, namespaceId, ns
              types := types, typeInstantiation := typeInstantiation
              locals := padLocals (parameterBound.map fun binders => some binders.current)
              localTypes := functionLocalTypes
              localNames := declaration.locals.map (·.name)
              oldLocals := padLocals (parameterBound.map fun binders => some binders.entry)
              results := resultBound.map (·.entry)
              resultFinals := finals
              resultComponents := components
              resultTypes := resultSlots.map (·.physical)
              state := some final, oldState := some state, executable, requiresMeaning
              labelDefinitions
              labelEntry := some {
                memory := state, locals := padLocals (parameterBound.map (some ∘ SlotBinders.entry)) }
              labelExit := some {
                memory := final, locals := padLocals (parameterBound.map (some ∘ SlotBinders.current)) } }
          let context ← bindLets context groups.lets true
          let mut clauses ← groups.ensures.mapM fun (clause, range) =>
            return markObligation range (← translate context clause)
          if let some (.uninterpreted resultLength function _) := nativeModel then
            clauses := clauses ++ (← uninterpretedResult context declaration resultLength
              function parameterSlots parameterBound resultSlots resultBound)
          if let some .structuralOrder := nativeModel then
            clauses := clauses.push (← orderResult context parameterSlots parameterBound
              resultSlots resultBound)
          if let some (role, model) := mapRole then
            let operands : RoleOperands := { role, slots := parameterSlots, bound := parameterBound
                                             model, carriers := ← frameCarriers signature.skolems }
            clauses := clauses ++ (← mapRoleEnsures { context with oldState := some state }
              operands resultSlots resultBound)
          let resultInvariants ← slotDataInvariantTerms context resultSlots resultBound
            (some ·.entryNative)
          let resultSigners ← signerFacts context resultSlots resultBound (some ·.entry)
          let exitInvariants ← slotDataInvariantTerms context parameterSlots parameterBound
            (·.exitNative)
          let dataInvariantObligations :=
            (resultInvariants ++ exitInvariants).map fun (clause, range) =>
              markObligation range clause
          let storedInvariants ← if storedInvariant.isNone then pure #[]
            else if assumesStored then pure #[← memoryInvariants final]
            else pure #[← mkArrow (← memoryInvariants state) (← memoryInvariants final)]
          let invariants ← namespaceInvariantTerms context invariantResources .exit
            (invariantApplies reach unit delegated) (memory := uses.memory)
            (ghostBase := some (typeArity declaration)) (ghostMemory := some uses.written)
          let invariantObligations := invariants.map fun (clause, range) =>
            markObligation range clause
          let body ← conjunction (clauses ++ dataInvariantObligations ++ storedInvariants ++
            invariantObligations ++ resultSigners)
          -- The value view: a returned reference is read at its current
          -- value, as if it died on return, unless the clauses read its
          -- final value.
          let readsFinal := groups.ensures.any (fun (clause, _) => mentionsFinal ns clause) ||
            mapRole.any (mapRoleReadsFinal ·.1)
          let body ← if prophecies.isEmpty || readsFinal then pure body
            else mkArrow (← conjunction prophecies) body
          mkLambdaFVars #[arguments, state, result, final] body
  /- An `aborts_if` condition forces failure and excuses the postcondition.
  A non-partial list also covers every failure. Independently, when any
  abort codes are declared, a failure must match a conditioned code (or an
  unqualified abort condition) or a standalone `aborts_with` code. Partial
  conditions do not relax this code check. -/
  let declaredAborts := !groups.abortsIf.isEmpty
  let hasCodes := !groups.abortsWith.isEmpty || groups.abortsIf.any (·.code.isSome)
  -- A native model's abort: exactly when its predicate is false.
  let modelAborts (context : Context) (bound : Array SlotBinders) : MetaM (Option Lean.Expr) := do
    if let some (role, model) := mapRole then
      return ← mapRoleAborts context
        { role, slots := parameterSlots, bound, model, carriers := ← frameCarriers signature.skolems }
    let some (.uninterpreted _ _ (some predicate)) := nativeModel | return none
    let holds ← modelApplication context declaration predicate parameterSlots
      (bound.map (·.entry)) .boolean none
    return some (mkNot (← Domain.boolean.ofBinder holds))
  let abortConditionTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state => do
      let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
      let context ← bindLets (contextOf state bound) groups.lets false
      let clauses ← groups.abortsIf.mapM (translate context ·.condition)
      let body ← if declaredAborts then disjunction clauses
        else pure ((← modelAborts (contextOf state bound) bound).getD (mkConst ``False))
      mkLambdaFVars #[arguments, state] body
  let abortsTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `state runtimeState fun state =>
      withLocalDeclD `failure failure fun failureBinder => do
        if !declaredAborts && !hasCodes then
          let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes
            arguments
          let permitted := (← modelAborts (contextOf state bound) bound).getD
            (if isStrict then mkConst ``False else mkConst ``True)
          mkLambdaFVars #[arguments, state, failureBinder] permitted
        else
          let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
          let context ← bindLets (contextOf state bound) groups.lets false
          let mut matched : Array Lean.Expr := #[]
          let mut conditions : Array Lean.Expr := #[]
          for clause in groups.abortsIf do
            let condition ← translate context clause.condition
            conditions := conditions.push condition
            match clause.code with
            | some code =>
                let codeTerm ← translate context code
                let codeMatches ← mkAppM ``VerexSL.Contract.abortCodeMatches #[failureBinder, codeTerm]
                matched := matched.push (markObligation clause.range (← mkAppM ``And
                  #[condition, codeMatches]))
            | none => matched := matched.push (markObligation clause.range condition)
          for (code, range) in groups.abortsWith do
            let codeTerm ← translate context code
            let codeMatches ← mkAppM ``VerexSL.Contract.abortCodeMatches #[failureBinder, codeTerm]
            matched := matched.push (markObligation range codeMatches)
          let permitted ← if hasCodes then
              let codes ← disjunction matched
              if declaredAborts && !isPartial && !groups.abortsWith.isEmpty then
                mkAppM ``And #[← disjunction conditions, codes]
              else pure codes
            else if declaredAborts && !isPartial then disjunction matched
            else pure (mkConst ``True)
          mkLambdaFVars #[arguments, state, failureBinder] permitted
  /- The frame is stated over global memory: a successful execution leaves
  the slots it does not declare as modified alone.  With no `modifies`
  clause the whole memory is unchanged.  With clauses, every listed resource
  type reads the same at every key other than its listed ones, and every
  other resource type reads the same everywhere; a loose frame leaves the
  unlisted resource types open.  The memory's update lemmas discharge the
  frame and a caller frames with it.  A function with a body that declares
  no contract and plays no map or table role claims no frame: it is
  verified for the invariants it writes, and its callers inline it. -/
  let claimsNoFrame := declaration.contract.loc.isNone && declaration.body != .absent &&
    mapRole.isNone
  -- Allocating a table records its handle in the allocation history, a
  -- slot no Move specification names: the frame of a function allocating
  -- one, itself or through a callee, leaves it open
  -- (`doc/designs/intrinsic-maps.md`, "Owned Table values").
  let allocates := allocatesTables unit namespaceId ns declaration
  let allocationSlot : Array (Lean.Expr × Lean.Expr) := if allocates then
      #[(mkConst ``VerexIR.Proofs.Denote.TableMemory.allocationResource,
        mkConst ``VerexIR.StorageKey.unit)]
    else #[]
  let frameTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `initial runtimeState fun initial =>
      withLocalDeclD `result resultType fun result =>
        withLocalDeclD `final runtimeState fun final => do
          let body ←
            if declaration.contract.modifiesAll || claimsNoFrame then
              pure (mkConst ``True)
            else if declaration.contract.modifies.isEmpty then
              if allocates then slotFrame initial final allocationSlot false
              else mkEq final initial
            else do
              let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
              -- A key may name a pre-state `let`.
              let context ← bindLets (contextOf initial bound) groups.lets false
              let slots ← declaration.contract.modifies.mapM (modifiedSlotTerm context)
              slotFrame initial final (slots ++ allocationSlot) (hasLooseFrame declaration.contract)
          mkLambdaFVars #[arguments, initial, result, final] body
  -- What the theorem assumes beyond the precondition: of a function with
  -- function-typed parameters, typed global memory, the frame resolving
  -- each such parameter's rows over type parameters to scalars, and keeping
  -- memory, as a parameter without `modifies_of` does.
  let assumesTerm ← withLocalDeclD `arguments argumentsType fun arguments =>
    withLocalDeclD `initial runtimeState fun initial => do
      unless assumesTyping unit namespaceId ns declaration do
        return ← mkLambdaFVars #[arguments, initial] (mkConst ``True)
      let some executable := executable
        | throwError "internal: a contract assuming typing does not take the unit"
      let unitExpr ← executableUnit executable
      let values ← rowProjections signature.skolems arguments signature.argumentTypes
      let bound ← parameterBinders signature.skolems parameterSlots signature.argumentTypes arguments
      let mut conjuncts := #[]
      for (type, value, index) in signature.argumentTypes.zipWith (·, ·) values |>.zipIdx |>.map
          (fun ((type, value), index) => (type, value, index)) do
        let type ← whnfR type
        if type.isAppOfArity ``VerexIR.Proofs.Denote.NTy.function 3 then
          let row := type.getArg! 0
          let sharing := type.getArg! 1
          let results := (type.getArg! 2).consumeMData
          let shape ← if results.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then
              pure (mkConst ``VerexIR.Proofs.Denote.ResultShape.none)
            else if results.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 &&
                (results.getArg! 1).consumeMData.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then
              pure (mkApp (mkConst ``VerexIR.Proofs.Denote.ResultShape.one) (results.getArg! 0))
            else throwError "a function-typed parameter returning several results is not carried"
          -- The value is typed by its carrier; an invocation's rows that
          -- mention the frame's type parameters are decided where the frame
          -- resolves them to scalars.
          for invoked in #[row, type.getArg! 2] do
            if mentionsInvocableParameter invoked then
              conjuncts := conjuncts.push
                (mkAppN (mkConst ``VerexIR.Proofs.ScalarAt)
                  ((← frameLeading signature.skolems) ++ #[signature.skolems, invoked]))
          -- Over a model, the frame is of the model's invocations.
          let invoke? ← modelInvoke? executable signature.skolems
          match declaration.contract.parameterFrames.find? (·.parameter.index == index) with
          | none =>
              conjuncts := conjuncts.push (← match invoke? with
                | some invoke =>
                    mkAppM' (mkAppN (mkConst ``VerexIR.Proofs.KeepsMemoryAtOf)
                      ((← frameLeading signature.skolems) ++ #[signature.skolems, invoke]))
                      #[row, sharing, shape, value]
                | none =>
                    mkAppM' (mkAppN (mkConst ``VerexIR.Proofs.KeepsMemoryAt)
                      ((← frameLeading signature.skolems) ++ #[executable, signature.skolems]))
                      #[row, sharing, shape, value])
          | some frame =>
              -- The closed frame but for the targets, the formals bound to
              -- an invocation's arguments; `modifies_of<f> *` frames nothing.
              unless frame.modifiesAll do
                let invocationType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList)
                  ((← frameCarriers signature.skolems)) row
                let frameTerm ← withLocalDeclD `invocation invocationType fun invocation =>
                  withLocalDeclD `pre runtimeState fun pre =>
                    withLocalDeclD `post runtimeState fun post => do
                      let componentTypes := rowElementTypes row
                      let components ← rowProjections signature.skolems invocation componentTypes
                      let mut context := contextOf pre bound
                      for (formal, component) in frame.formals.zip components do
                        let some physical := functionLocalTypes[formal.index]?
                          | throwError "a frame's formal has no local type"
                        let binder ← (domainOf physical).binderOfNative component
                        context := { context with
                          locals := context.locals.set! formal.index (some binder)
                          oldLocals := context.oldLocals.set! formal.index (some binder) }
                      let slots ← frame.modifies.mapM (modifiedSlotTerm context)
                      mkLambdaFVars #[invocation, pre, post] (← slotFrame pre post slots false)
                conjuncts := conjuncts.push (← match invoke? with
                  | some invoke =>
                      mkAppM' (mkAppN (mkConst ``VerexIR.Proofs.FramedAtOf)
                        ((← frameLeading signature.skolems) ++ #[signature.skolems, invoke]))
                        #[row, sharing, shape, frameTerm, value]
                  | none =>
                      mkAppM' (mkAppN (mkConst ``VerexIR.Proofs.FramedAt)
                        ((← frameLeading signature.skolems) ++ #[executable, signature.skolems]))
                        #[row, sharing, shape, frameTerm, value])
      mkLambdaFVars #[arguments, initial] (← conjunction conjuncts)
  let contract ← mkAppOptM ``VerexIR.Proofs.Contract.mk
    #[some runtimeState, some failure, some argumentsType, some resultType,
      some requiresTerm, some assumesTerm, some ensuresTerm, some abortsTerm,
      some abortConditionTerm, some abortConditionTerm, some frameTerm]
  -- Verification is native (`doc/designs/native-verification.md`).
  let name := ((ns.tables.names[declaration.name.index]?).map (·.name)).getD "a function"
  discard <| VerexIR.Proofs.Denote.reportRuntimeValues m!"the contract of `{name}`" contract
  return contract

/-- Segments of a `verexPath`, ignoring separators. -/
private partial def pathSegments (stx : Syntax) : Array String :=
  match stx with
  -- The identifier's name, as the elaborator registers it: a quoted segment
  -- such as a script's `«<SELF>_0»` without its quotes.
  | .ident _ _ name _ => #[name.toString (escape := false)]
  | .atom _ value => if value == "::" then #[] else #[value]
  | .node _ _ arguments => arguments.flatMap pathSegments
  | _ => #[]

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

/-- A function's own name, without its namespace path. -/
private def functionName (unit : ValidatedUnit) (handle : VerexIR.FunctionHandle) : String :=
  let qualified? : Option VerexIR.QualifiedName := do
    let ns ← unit.namespaces[handle.namespaceId.index]?
    let declaration ← ns.functions[handle.functionId.index]?
    unit.tables.names[declaration.name.index]?
  (qualified?.map (·.name)).getD ""

/-- A function's path-qualified name, in every namespace. -/
def qualifiedFunctionName (unit : ValidatedUnit) (handle : VerexIR.FunctionHandle) : String :=
  let path := (unit.tables.namespaces[handle.namespaceId.index]?.map (·.segments)).getD #[]
  "::".intercalate (path.push (functionName unit handle)).toList

/-- A function's name relative to a unit: its own name in the unit's module,
namespace 0, and its path-qualified name in another namespace. -/
def functionKey (unit : ValidatedUnit) (handle : VerexIR.FunctionHandle) : String :=
  if handle.namespaceId.index == 0 then functionName unit handle
  else qualifiedFunctionName unit handle

/-- The prelude model a native is read by, beside what its specification
states: the Move Prover's prelude defines a native whatever its
specification adds. -/
def nativeModelOf? (unit : ValidatedUnit) (handle : VerexIR.FunctionHandle)
    (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Option NativeModel :=
  if declaration.body == .absent then nativeModel? ns.profile (qualifiedFunctionName unit handle)
  else none

/-- Locate one function of a registered unit by its key: its name in the
unit's module, or its path-qualified name in another namespace. -/
def findFunction? (unit : ValidatedUnit) (key : String) :
    Option (Nat × ValidatedNamespace × Nat ×
      VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) := do
  let parts := (key.splitOn "::").toArray
  let path := parts.pop
  let name := parts.back?.getD key
  for h : namespaceIndex in [0:unit.namespaces.size] do
    let ns := unit.namespaces[namespaceIndex]
    let owned := if path.isEmpty then namespaceIndex == 0 else
      unit.tables.namespaces[ns.identity.index]?.map (·.segments) == some path
    unless owned do continue
    for h : index in [0:ns.functions.size] do
      let declaration := ns.functions[index]
      if let some qualified := unit.tables.names[declaration.name.index]? then
        if qualified.name == name then
          return (namespaceIndex, ns, index, declaration)
  none

/-- Which clauses of a contract hold where: a `[concrete]` clause is proved
of the function's body, an `[abstract]` one is what its callers see of it,
and an unmarked clause is both. -/
inductive ContractView where
  | implementation
  | interface
  deriving BEq

private def markedAs (name : String) (condition : VerexIR.Condition) : Bool :=
  condition.properties.any fun
    | .assign marker (.constant (.bool true)) _ => marker == name
    | _ => false

/-- Whether callers see a function differently than its body is proved:
some clause is `[abstract]` or `[concrete]`. -/
def hasInterfaceView (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    Bool :=
  declaration.contract.conditions.any fun condition =>
    markedAs "abstract" condition || markedAs "concrete" condition

/-- A declaration with only the clauses a view admits. -/
def ContractView.of (view : ContractView)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) :
    VerexIR.FunctionDecl VerexIR.Validation.FunctionBody :=
  let excluded := match view with
    | .implementation => "abstract"
    | .interface => "concrete"
  { declaration with contract := { declaration.contract with
      conditions := declaration.contract.conditions.filter (!markedAs excluded ·) } }

/-- The precondition of a function where it starts, as its caller owes it
at the call: its `requires` clauses in the view its callers see,
over its parameters' values (`locals`, the other locals unbound) and the
memory there (`state`), after the pre-state `let`s they read. Each clause is
marked with its source range, so a precondition not established is
reported there. A clause applying `requires_of` reads the module's table of
declared preconditions, which the start does not take: it is left out, as
the caller's obligation it would be is not stated. -/
def startPrecondition (unit : ValidatedUnit) (namespaceId : VerexIR.NamespaceId)
    (ns : ValidatedNamespace) (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (locals : Array (Option Lean.Expr)) (localTypes : Array IrTy) (types : Option Lean.Expr)
    (state : Lean.Expr) (executable : Option Lean.Expr) :
    MetaM Lean.Expr := do
  let conditions := (ContractView.interface.of declaration).contract.conditions
  let groups ← groupConditions unit conditions
  let context : Context := {
    unit, namespaceId, ns, locals, oldLocals := locals, localTypes,
    localNames := declaration.locals.map (·.name), results := #[], types,
    state := some state, oldState := some state, executable }
  let context ← bindLets context groups.lets false
  conjunction (← (conditions.filter fun condition => condition.kind == .requires &&
      !conditionReadsRequires unit namespaceId condition.expression).mapM fun condition => do
    pure (markCondition unit condition (← translate context condition.expression)))

/-- Name of the generated contract definition. -/
def contractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "contract"

/-- Name of the contract over a function's native arguments and result. -/
def typedContractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedContract"

/-- Name of a contract specialized at type arguments `θ` of a caller's frame
(`specializedContractOf`): `f.at_θ.typedContract` of `f.typedContract`. -/
def specializedName (generic : Name) (θ : VerexIR.Proofs.Denote.NRow) : Name :=
  match generic with
  | .str prefix_ last => Name.str (Name.str prefix_ (typeArgumentsSpelling θ)) last
  | _ => Name.str generic (typeArgumentsSpelling θ)

/-- Name of the contract a function's callers see of it, where that differs
from its implementation's. -/
def typedInterfaceContractName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedInterfaceContract"

private def rootIdent (name : Name) : Ident :=
  mkIdent (rootNamespace ++ name)

/-- Name of the generated validated-unit definition. -/
def unitName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "unit"

/-- Name of the unit's function values instance (`FunctionCarriers`). -/
def functionCarriersName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "functionCarriers"

/-- Define the registered unit as a Lean literal, once per namespace. -/
def ensureUnitDefinition (namespaceSegments : Array String)
    (unit : ValidatedUnit) : CommandElabM Name := do
  let name := unitName namespaceSegments
  if (← getEnv).contains name then return name
  -- A theorem holds of the unit as preparation admits it: a unit the static
  -- checker refuses would make every theorem about it vacuous.
  let failures := VerexIR.Validation.StaticTyping.unitFailures unit
    (VerexIR.Validation.targetPointerWidth? unit)
  unless failures.isEmpty do
    throwError m!"the unit is not statically typed, so preparation refuses it:\n\
      {"\n".intercalate failures.toList}"
  -- Nor may preparation refuse its loans.
  let loanErrors := (unit.borrowDiagnostics ++ VerexIR.Validation.loanDeathDiagnostics unit).filter
    (·.severity == .error)
  unless loanErrors.isEmpty do
    throwError m!"the borrow analysis rejects the unit, so preparation refuses it:\n\
      {"\n".intercalate (loanErrors.toList.map fun d => s!"{d.code}: {d.message}")}"
  -- Nor may execution preparation refuse it under every registry.
  let refusals := VerexIR.Validation.executionRefusals unit
  unless refusals.isEmpty do
    throwError m!"execution preparation refuses the unit under every registry, so its \
      theorems would be vacuous:\n\
      {"\n".intercalate (refusals.toList.map fun d => s!"{d.code}: {d.message}")}"
  let value ← VerexSL.Quote.quoteUnit unit
  liftTermElabM do
    addDecl (.defnDecl {
      name, levelParams := []
      type := mkConst ``VerexIR.Validation.ValidatedUnit
      value
      hints := .abbrev, safety := .safe })
    enableRealizationsForConst name
  return name

/-- Prefix of a namespace's compilation artifacts: its compile views, key
maps, type tables, and frames. -/
def semanticsName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "semantics"

/-- Name of the equation evaluating the unit's target pointer width. -/
def pointerWidthEqName (namespaceSegments : Array String) : Name :=
  Name.str (pathName namespaceSegments) "targetPointerWidth_eq"

/-- Quote the registered unit once per namespace, with the typed twins of
its declarations and the kernel's evaluation of its target
pointer width. The semantics reads the validated unit itself, so there is
no prepared unit to certify. -/
def ensureUnitDefinitions (namespaceSegments : Array String)
    (unit : ValidatedUnit) : CommandElabM Name := do
  let unitDefinition ← ensureUnitDefinition namespaceSegments unit
  let widthEqName := pointerWidthEqName namespaceSegments
  if (← getEnv).contains widthEqName then return widthEqName
  -- An inductive declaration waits for every pending kernel check: the twin
  -- structures come first.
  discard <| SpecTypes.ensureSpecTypes namespaceSegments unit
  let width := VerexIR.Validation.targetPointerWidth? unit
  liftTermElabM do
    addDecl (.thmDecl {
      name := widthEqName, levelParams := []
      type := ← mkEq (mkApp (mkConst ``VerexIR.Validation.targetPointerWidth?)
        (mkConst unitDefinition)) (toExpr width)
      value := ← mkEqRefl (toExpr width) })
  return widthEqName

/-- Materialize a registered unit as a Lean definition. -/
syntax (name := verexUnitCommand) "#verex_unit" verexPath : command


/-- Name of the typed verification theorem over a function's native
signature. -/
def typedVerifiedName (namespaceSegments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName namespaceSegments) function) "typedVerified"

/-- Turn verification-cost measurement on for the rest of this file.  Only
the benchmark file uses it, so an ordinary `verify` never pays for it. -/
elab "#verex_measure" : command => do
  Perf.recorded.set #[]
  Perf.measuring.set true

/-- Compare the measured costs of this file with a baseline, or rewrite the
baseline when `UB=1` is set.  The comparison gates on the two reproducible
numbers and reports wall time beside them. -/
elab "#verex_perf " path:str : command => do
  Perf.measuring.set false
  let samples ← Perf.recorded.get
  if samples.isEmpty then
    logError "no verification target was measured; \
      `#verex_measure` must precede the modules"
    return
  let path := System.FilePath.mk path.getString
  let update := (← IO.getEnv "UB").isSome
  if update then
    IO.FS.writeFile path (Perf.baselineText samples)
    logInfo m!"verification-cost baseline updated:\n{Perf.baselineText samples}"
    return
  unless ← path.pathExists do
    logError m!"the verification-cost baseline {path} does not exist; \
      write it with `UB=1`"
    return
  let baseline := Perf.parseBaseline (← IO.FS.readFile path)
  let (text, ok) := Perf.report samples baseline 10
  if ok then logInfo m!"verification cost:\n{text}"
  else throwError m!"verification cost regressed:\n{text}"

end VerexSL.Contract
