-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Import.Raw
import VerexIR.Validation.Validated

/-!
# Mutable references held by locals

A mutable reference passes its current value back to its lender when it
dies, and the semantics reads that value from the local holding it
(`settleLoans`). A reference a call or an invocation returns is passed on
intact where it is an argument, returned, written through, or bound to a
local. Anywhere else no local holds it, and its current value is
lost when it dies: a discarded result, a pattern's wildcard, or the operand
of an operation reading through it (`*f(x)`, `f(x).field`). Each such
reference is bound here, before validation, to a local of its own, which
holds it until it dies; an operation reading through it reads the local, and
the operands evaluated before it are bound first, so the evaluation order is
kept. A borrow of a path through it, `&mut f(x).field`, then starts at the
local, and the value-borrow normalization makes it a reborrow from there.
-/

namespace VerexIR.Validation

open VerexIR.Import

private structure HeldState where
  types : Array Ty
  expressions : Array Expr
  patterns : Array Pattern
  locals : Array LocalDecl
  holders : Array (PatternId × Holding) := #[]

private abbrev HeldM := StateM HeldState

private def exprAt (id : ExprId) : HeldM Expr := do
  return (← get).expressions[id.index]?.getD default

private def pushExpr (loc : LocId) (typeId : TypeId) (kind : ExprKind) : HeldM ExprId :=
  modifyGet fun state =>
    (⟨state.expressions.size⟩, { state with expressions := state.expressions.push { loc, typeId, kind } })

private def pushPattern (loc : LocId) (typeId : TypeId) (kind : PatternKind) : HeldM PatternId :=
  modifyGet fun state =>
    (⟨state.patterns.size⟩, { state with patterns := state.patterns.push { loc, typeId, kind } })

private def setKind (id : ExprId) (kind : ExprKind) : HeldM Unit :=
  modify fun state => { state with expressions := state.expressions.modify id.index ({ · with kind }) }

private def unitType : HeldM TypeId := do
  match (← get).types.findIdx? (· == .unit) with
  | some index => pure ⟨index⟩
  | none => modifyGet fun state => (⟨state.types.size⟩, { state with types := state.types.push .unit })

/-- A fresh local, `$held` for one an operation reads, `$dropped` for a
discarded value or a wildcard, bound by `pattern`, which is recorded as its
holder. The `$` keeps the names out of the source's reach, and the printer
spells the holders back as the source does. -/
private def holdIn (pattern : PatternId) (typeId : TypeId) (loc : LocId) (holding : Holding)
    (mutable : Bool := false) : HeldM LocalId :=
  modifyGet fun state =>
    let localId : LocalId := ⟨state.locals.size⟩
    let name := s!"{if holding == .read then "$held" else "$dropped"}{localId.index}"
    let declaration : LocalDecl := { id := localId, name, type := { typeId, loc }, mutable, loc }
    (localId, { state with
      locals := state.locals.push declaration
      patterns := state.patterns.modify pattern.index ({ · with kind := .variable localId })
      holders := state.holders.push (pattern, holding) })

/-- A fresh pattern binding a fresh local (`holdIn`). -/
private def holderPattern (typeId : TypeId) (loc : LocId) (holding : Holding)
    (mutable : Bool := false) : HeldM (LocalId × PatternId) := do
  let pattern ← pushPattern loc typeId .wildcard
  return (← holdIn pattern typeId loc holding mutable, pattern)

private def isMutableReference (typeId : TypeId) : HeldM Bool := do
  return match (← get).types[typeId.index]? with
    | some (.reference reference) => reference.kind == .mutable
    | _ => false

private def isReference (typeId : TypeId) : HeldM Bool := do
  return (← get).types[typeId.index]? matches some (.reference _)

/-- Whether an expression's value is the result of a call or an invocation,
or passed on from one by a block, a branch, an arm, or a `let`. -/
private def produced (id : ExprId) : HeldM Bool := do
  let expressions := (← get).expressions
  -- At most one step per node, as the pass runs before the raw checks.
  let rec go : Nat → ExprId → Bool
    | 0, _ => false
    | fuel + 1, id =>
      let kind : Option ExprKind := (expressions[id.index]?).map (·.kind)
      match kind with
      | some (.operation (.call (.function _)) _ _ _) | some (.operation (.call .invoke) _ _ _) => true
      | some (.block _ (some result)) => go fuel result
      | some (.ifElse _ thenBranch (some elseBranch)) => go fuel thenBranch || go fuel elseBranch
      | some (.match_ _ arms) => arms.any (go fuel ·.body)
      | some (.letDecl _ _ body) => go fuel body
      | _ => false
  return go expressions.size id

/-- An operand evaluating without effects, which binding it first would not
reorder. -/
private def effectFree (expression : Expr) : Bool :=
  match expression.kind with
  | .value _ _ | .constant _ | .localVar _ => true
  | _ => false

/-- Bind a value to a fresh local around a body: `let local := value; body`. -/
private def bindAround (value : ExprId) (body : ExprId) (mutable : Bool := false) :
    HeldM (LocalId × ExprId) := do
  let node ← exprAt value
  let (localId, pattern) ← holderPattern node.typeId node.loc .read mutable
  let bodyNode ← exprAt body
  let bound ← pushExpr node.loc bodyNode.typeId (.letDecl pattern (some value) body)
  return (localId, bound)

/-- The components of a tuple type that are mutable references, where one is. -/
private def mutableComponents (typeId : TypeId) : HeldM (Option (Array (TypeId × Bool))) := do
  match (← get).types[typeId.index]? with
  | some (.tuple elements) =>
      let components ← elements.mapM fun element => return (element, ← isMutableReference element)
      return if components.any (·.2) then some components else none
  | _ => return none

/-- The base of a projection: a field selection whose value is a reference
into its operand's referent, which a write or a borrow takes. -/
private def projectionBase? (id : ExprId) : HeldM (Option ExprId) := do
  let node ← exprAt id
  unless ← isReference node.typeId do return none
  return match node.kind with
    | .operation (.data (.select ..)) _ #[base] _ | .operation (.data (.selectVariants ..)) _ #[base] _ =>
        some base
    | _ => none

/-- The mutable reference a value holds that no local holds: the value
itself where a call produced it, or the call's result the value's
projections select from, with the projection whose base it is. -/
private def unheldStart? (id : ExprId) : HeldM (Option (ExprId × Option ExprId)) := do
  let node ← exprAt id
  if (← produced id) && (← isMutableReference node.typeId) then return some (id, none)
  -- At most one step per node, as the pass runs before the raw checks.
  let mut projection := id
  for _ in [0:(← get).expressions.size] do
    let some base ← projectionBase? projection | return none
    if (← produced base) && (← isMutableReference (← exprAt base).typeId) then
      return some (base, some projection)
    projection := base
  return none

/-- A projection's base replaced by a read of a local. -/
private def readBase (projection : ExprId) (localId : LocalId) : HeldM Unit := do
  let .operation operation instantiations arguments surface := (← exprAt projection).kind | return
  let some base := arguments[0]? | return
  let baseNode ← exprAt base
  let read ← pushExpr baseNode.loc baseNode.typeId (.localVar localId)
  setKind projection (.operation operation instantiations (arguments.set! 0 read) surface)

/-- A value whose projections select from a call's mutable reference, with
that reference bound to a fresh local around it, the projections starting
at the local: `let _held := f(x); _held.field`. -/
private def holdProjected (id : ExprId) : HeldM ExprId := do
  let some (start, some projection) ← unheldStart? id | return id
  let (localId, bound) ← bindAround start id
  readBase projection localId
  return bound

/-- A discarded value holding mutable references no local holds, bound to
fresh locals: `let _held := value;`, or `let (_held, _, …) := value;` for a
tuple. -/
private def discard (id : ExprId) : HeldM ExprId := do
  let node ← exprAt id
  unless ← produced id do return ← holdProjected id
  let pattern ← if ← isMutableReference node.typeId then
      pure (← holderPattern node.typeId node.loc .dropped).2
    else
      let some components ← mutableComponents node.typeId | return id
      let elements ← components.mapM fun (typeId, mutable) => do
        if mutable then pure (← holderPattern typeId node.loc .dropped).2
        else pushPattern node.loc typeId .wildcard
      pushPattern node.loc node.typeId (.tuple elements)
  let unit ← unitType
  let rest ← pushExpr node.loc unit (.block #[] none)
  pushExpr node.loc unit (.letDecl pattern (some id) rest)

/-- A pattern's wildcards that would discard the mutable references of a
value no local holds, as fresh locals. -/
private def holdWildcards (pattern : PatternId) (value : ExprId) : HeldM Unit := do
  unless ← produced value do return
  let hold (pattern : PatternId) : HeldM Unit := do
    let some node := (← get).patterns[pattern.index]? | return
    if node.kind matches .wildcard then
      if ← isMutableReference node.typeId then
        let _ ← holdIn pattern node.typeId node.loc .dropped
  let some node := (← get).patterns[pattern.index]? | return
  match node.kind with
  | .tuple elements => for element in elements do hold element
  | _ => hold pattern

private def readsThrough (operation : Operation) (result : TypeId) : HeldM Bool :=
  return readsThroughReference (← get).types operation result

/-- The root of the path a value-level borrow selects through, with the
node of the path whose base it is. -/
private def pathRoot? (id : ExprId) : HeldM (Option (ExprId × ExprId)) := do
  let base? (id : ExprId) : HeldM (Option ExprId) := do
    return match (← exprAt id).kind with
      | .operation (.data (.select ..)) _ #[base] _ | .operation (.data (.selectVariants ..)) _ #[base] _
      | .operation (.reference .dereference) _ #[base] _ | .operation (.primitive .index) _ #[base, _] _ =>
          some base
      | _ => none
  let some first ← base? id | return none
  -- At most one step per node, as the pass runs before the raw checks.
  let mut found := (first, id)
  for _ in [0:(← get).expressions.size] do
    let some next ← base? found.1 | break
    found := (next, found.1)
  return some found

/-- A value-level borrow of a path through a mutable reference no local
holds, `&mut f(x).field`: the reference is bound to a fresh local the path
starts at, so the borrow reborrows from it. A mutable borrow of a path
through a computed value, `&mut (x + 1)`, borrows a fresh mutable local
holding the value, as Move compiles a borrow of a temporary. -/
private def borrowThroughHeld (id : ExprId) : HeldM ExprId := do
  let node ← exprAt id
  let .operation (.reference (.borrow kind)) _ #[path] _ := node.kind | return id
  let (root, parent) := (← pathRoot? path).getD (path, id)
  let rootNode ← exprAt root
  let throughReference := (← produced root) && (← isMutableReference rootNode.typeId)
  let temporary := kind == .mutable && !(rootNode.kind matches .localVar _) &&
    !(← isReference rootNode.typeId)
  unless throughReference || temporary do return id
  -- A computed index of the path is bound after the root, in evaluation
  -- order, as a place index reads a local.
  let mut indexed : Array (ExprId × ExprId) := #[]
  let mut current := path
  for _ in [0:(← get).expressions.size] do
    if current == root then break
    match (← exprAt current).kind with
    | .operation (.primitive .index) _ #[base, index] _ =>
        unless (← exprAt index).kind matches .value .. | .localVar _ do
          indexed := indexed.push (current, index)
        current := base
    | .operation _ _ arguments _ => match arguments[0]? with
        | some base => current := base
        | none => break
    | _ => break
  let mut body := id
  for (node, index) in indexed do
    let (indexLocal, boundIndex) ← bindAround index body
    let indexNode ← exprAt index
    let read ← pushExpr indexNode.loc indexNode.typeId (.localVar indexLocal)
    let .operation operation instantiations arguments surface := (← exprAt node).kind | continue
    setKind node (.operation operation instantiations (arguments.set! 1 read) surface)
    body := boundIndex
  let (localId, bound) ← bindAround root body (mutable := temporary)
  let read ← pushExpr rootNode.loc rootNode.typeId (.localVar localId)
  let .operation operation instantiations arguments surface := (← exprAt parent).kind | return id
  setKind parent (.operation operation instantiations (arguments.set! 0 read) surface)
  return bound

/-- An operation reading through mutable references no local holds. Each is
bound to a fresh local, the operation reading the local or its projections
starting at it, and the operands before the last of them are bound first,
in evaluation order. -/
private def readThroughHeld (id : ExprId) : HeldM ExprId := do
  let node ← exprAt id
  let .operation operation instantiations arguments surface := node.kind | return id
  if operation matches .reference (.borrow _) then return ← borrowThroughHeld id
  unless ← readsThrough operation node.typeId do return id
  let starts ← arguments.mapM unheldStart?
  let some last := (starts.zipIdx.filter (·.1.isSome)).back?.map (·.2) | return id
  -- The first operand's binding encloses the others.
  let mut arguments := arguments
  let mut body := id
  for index in (List.range (last + 1)).reverse do
    match starts[index]! with
    | some (start, some projection) =>
        let (localId, bound) ← bindAround start body
        readBase projection localId
        body := bound
    | _ =>
        let operand := arguments[index]!
        let operandNode ← exprAt operand
        unless starts[index]!.isSome || !effectFree operandNode do continue
        let (localId, bound) ← bindAround operand body
        arguments := arguments.set! index
          (← pushExpr operandNode.loc operandNode.typeId (.localVar localId))
        body := bound
  setKind id (.operation operation instantiations arguments surface)
  return body

/-- A node with the references its children produce held where they would
otherwise be lost. Specification blocks and quantifiers hold none. -/
private def holdChildren (id : ExprId) : HeldM Unit := do
  let node ← exprAt id
  match node.kind with
  | .block statements result =>
      let statements ← statements.mapM fun statement => do discard (← readThroughHeld statement)
      setKind id (.block statements (← result.mapM readThroughHeld))
  | .letDecl pattern value body =>
      let value ← value.mapM fun value => do
        holdWildcards pattern value
        if ((← get).patterns[pattern.index]?).any (·.kind matches .wildcard) then
          holdProjected (← readThroughHeld value)
        else readThroughHeld value
      setKind id (.letDecl pattern value (← readThroughHeld body))
  | .operation operation instantiations arguments surface =>
      setKind id (.operation operation instantiations (← arguments.mapM readThroughHeld) surface)
  | .ifElse condition thenBranch elseBranch =>
      setKind id (.ifElse (← readThroughHeld condition) (← readThroughHeld thenBranch)
        (← elseBranch.mapM readThroughHeld))
  | .match_ scrutinee arms =>
      let arms ← arms.mapM fun arm => do
        return { arm with guard := ← arm.guard.mapM readThroughHeld, body := ← readThroughHeld arm.body }
      setKind id (.match_ (← readThroughHeld scrutinee) arms)
  | .loop label body => setKind id (.loop label (← readThroughHeld body))
  | .break_ nest value => setKind id (.break_ nest (← value.mapM readThroughHeld))
  | .return_ values => setKind id (.return_ (← values.mapM readThroughHeld))
  | .throw_ kind arguments => setKind id (.throw_ kind (← arguments.mapM readThroughHeld))
  | .assign place value => setKind id (.assign place (← readThroughHeld value))
  | .assignPattern pattern value =>
      holdWildcards pattern value
      setKind id (.assignPattern pattern (← readThroughHeld value))
  | .value .. | .constant _ | .localVar _ | .continue_ _ | .quantifier .. | .spec _ => pure ()

/-- The nodes of a body outside its specifications, each once. The pass runs
before the raw checks, so a node already seen, as in a cyclic arena, is not
walked again. -/
private partial def runtimeNodes (expressions : Array Expr) (root : ExprId)
    (walked : Array ExprId × Array Bool) : Array ExprId × Array Bool :=
  let (found, seen) := walked
  match expressions[root.index]?, seen[root.index]? with
  | some { kind := .spec _, .. }, _ | some { kind := .quantifier .., .. }, _ => walked
  | some expression, some false =>
      (expressionChildren expression.kind).foldl
        (fun walked child => runtimeNodes expressions child walked)
        (found.push root, seen.set! root.index true)
  | _, _ => walked

/-- Bind the mutable references no local would hold in a unit's structured
function bodies to locals of their own; and each namespace's holders. -/
def holdMutableReferences (unit : RawUnit) : RawUnit × Array (Array (PatternId × Holding)) :=
    Id.run do
  let mut types := unit.tables.types
  let mut namespaces := #[]
  let mut holders := #[]
  for ns in unit.namespaces do
    let mut state : HeldState :=
      { types, expressions := ns.expressions, patterns := ns.patterns, locals := #[] }
    let mut functions := #[]
    for function in ns.functions do
      match function.body with
      | .structured root =>
          let (nodes, _) := runtimeNodes state.expressions root
            (#[], Array.replicate state.expressions.size false)
          state := { state with locals := function.locals }
          -- Parents first: a path a borrow selects through is bound at the
          -- borrow, before its nodes are visited.
          let (root, next) := (readThroughHeld root).run state
          let ((), next) := (nodes.forM holdChildren).run next
          state := next
          let held : FunctionDecl RawBody :=
            { function with locals := state.locals, body := .structured root }
          functions := functions.push held
      | _ => functions := functions.push function
    types := state.types
    namespaces := namespaces.push { ns with
      expressions := state.expressions, patterns := state.patterns, functions }
    holders := holders.push state.holders
  return ({ unit with tables := { unit.tables with types }, namespaces }, holders)

end VerexIR.Validation
