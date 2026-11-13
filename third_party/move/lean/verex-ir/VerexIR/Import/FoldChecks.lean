-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Import.Raw

/-!
# Folding the checks rustc inserts

Optimized MIR spells each check of a Rust operation as an `assert` terminator
in front of an operation that is undefined or wraps where the check fails. The
source has one operation, so a raw graph is normalized before structurization:
a check folds into the checked primitive of the operation it guards, which
fails with `panic` exactly where the check does. A panic's payload is not
observable under `panic=abort`, and the order of a check against the pure
statements around it is not either.

A check folds only where its condition is the exact failure condition of the
operation over the same operands, with nothing but writes of single-assignment
temporaries between them. Any other check stays an assertion, which is sound:
it structurizes to a conditional.
-/

namespace VerexIR.Import.FoldChecks

/-- An operand or condition, spelled independently of the temporaries that
hold its parts. -/
private inductive Sym where
  | literal (value : ConstValue)
  | local (localId : LocalId)
  | deref (base : Sym)
  | field (base : Sym) (owner : QualifiedRef) (field : NameId)
  | element (base : Sym) (index : Sym)
  | borrowed (place : Sym)
  | unary (operation : PrimitiveOperation) (operand : Sym)
  | binary (operation : PrimitiveOperation) (left right : Sym)
  deriving BEq, Inhabited

inductive Access where
  | read
  | write
  deriving BEq

structure Occurrence where
  place : PlaceId
  node : ExprId
  access : Access

structure Context where
  places : Array Place
  types : Array Ty
  pointerWidth : Option Nat

structure State where
  expressions : Array Expr
  locals : Array LocalDecl
  blocks : Array RawBasicBlock

def exprKind? (state : State) (id : ExprId) : Option ExprKind :=
  (state.expressions[id.index]?).map (·.kind)

private def typeOf? (state : State) (id : ExprId) : Option TypeId :=
  (state.expressions[id.index]?).map (·.typeId)

def bareLocal? (context : Context) (place : PlaceId) : Option LocalId :=
  match context.places[place.index]? with
  | some (.localVar localId) => some localId
  | _ => none

partial def rootLocal? (context : Context) (place : PlaceId) : Option LocalId :=
  match context.places[place.index]? with
  | some (.localVar localId) => some localId
  | some (.deref base) | some (.field base _ _) | some (.ghostField base _ _)
  | some (.index base _) | some (.subslice base _ _ _) | some (.unsize base)
      | some (.downcast base _) =>
      rootLocal? context base
  | none => none

/-- The width and signedness of an integer type at the unit's target. -/
private def integerInfo? (context : Context) (typeId : TypeId) : Option (Nat × Bool) :=
  match context.types[typeId.index]? with
  | some (.integer (.bits width) signed) => some (width, signed)
  | some (.integer .pointer signed) => context.pointerWidth.map (·, signed)
  | _ => none

/-! ## What a graph reads and writes -/

mutual
  partial def placeOccurrences (context : Context) (state : State) (place : PlaceId)
      (node : ExprId) (access : Access) : Option (Array Occurrence) := do
    let inner ← match context.places[place.index]? with
      | some (.index _ index) => exprOccurrences context state index
      | _ => pure #[]
    let nested ← match context.places[place.index]? with
      | some (.deref base) | some (.field base _ _) | some (.ghostField base _ _)
      | some (.index base _) | some (.subslice base _ _ _) | some (.unsize base)
      | some (.downcast base _) =>
          -- A nested place is only read on the way to the projected slot.
          let parts ← placeOccurrences context state base node .read
          pure (parts.filter (fun occurrence => occurrence.place != base))
      | _ => pure #[]
    return #[{ place, node, access }] ++ inner ++ nested

  /-- The places an expression tree of a raw block reaches; none for a form
  no raw block holds that this pass would have to look inside of. -/
  partial def exprOccurrences (context : Context) (state : State) (id : ExprId) :
      Option (Array Occurrence) := do
    match ← exprKind? state id with
    | .value .. | .constant _ => pure #[]
    | .localVar localId =>
        let place ← context.places.findIdx? fun place => match place with
          | .localVar read => read == localId
          | _ => false
        pure #[{ place := ⟨place⟩, node := id, access := .read }]
    | .operation operation _ arguments _ =>
        let own ← match operation with
          | .move place | .copy place | .read place | .readDiscriminant place | .borrow _ place =>
              placeOccurrences context state place id .read
          | .write place | .drop place => placeOccurrences context state place id .write
          | _ => pure #[]
        let parts ← arguments.mapM (exprOccurrences context state)
        pure (own ++ parts.flatten)
    | .assign place value =>
        pure ((← placeOccurrences context state place id .write) ++
          (← exprOccurrences context state value))
    | _ => none
end

def blockOccurrences (context : Context) (state : State) (block : RawBasicBlock) :
    Option (Array Occurrence) := do
  let mut result := #[]
  for statement in block.statements do
    if let .execute expression := statement then
      result := result ++ (← exprOccurrences context state expression)
  match block.terminator with
  | .branch condition _ _ | .switch condition _ _ | .assert condition _ _ _ _ =>
      result := result ++ (← exprOccurrences context state condition)
  | .call call destination _ =>
      result := result ++ (← exprOccurrences context state call)
      if let some destination := destination then
        result := result ++ (← placeOccurrences context state destination.place call .write)
  | .drop place _ _ =>
      result := result ++ (← placeOccurrences context state place ⟨0⟩ .write)
  | .return_ values | .throw_ _ values =>
      for value in values do
        result := result ++ (← exprOccurrences context state value)
  | .goto _ | .unreachable | .resume | .abort => pure ()
  return result

def graphOccurrences (context : Context) (state : State) : Option (Array Occurrence) :=
  state.blocks.foldlM (init := #[]) fun result block => do
    pure (result ++ (← blockOccurrences context state block))

private def writesOf (context : Context) (occurrences : Array Occurrence) (localId : LocalId) :
    Nat :=
  (occurrences.filter fun occurrence =>
    occurrence.access == .write && rootLocal? context occurrence.place == some localId).size

private def readsOf (context : Context) (occurrences : Array Occurrence) (localId : LocalId) :
    Array Occurrence :=
  occurrences.filter fun occurrence =>
    occurrence.access == .read && rootLocal? context occurrence.place == some localId

private def predecessorCount (blocks : Array RawBasicBlock) (target : Nat) : Nat :=
  blocks.foldl (init := 0) fun count block =>
    let successors : Array BlockId := match block.terminator with
      | .goto next | .drop _ next _ | .assert _ _ _ next _ => #[next]
      | .branch _ thenTarget elseTarget => #[thenTarget, elseTarget]
      | .switch _ cases defaultTarget => (cases.map (·.2)).push defaultTarget
      | .call _ destination _ => (destination.map (·.target)).toArray
      | .return_ _ | .throw_ _ _ | .unreachable | .resume | .abort => #[]
    count + (successors.filter (·.index == target)).size

/-! ## The value of an expression over a run of statements -/

/-- The temporaries a run of statements assigns, each with its value. -/
private abbrev Run := Array (LocalId × ExprId)

/-- Whether an expression only computes: it writes nothing and cannot fail. -/
private partial def pureValue (state : State) (id : ExprId) : Bool :=
  match exprKind? state id with
  | some (.value ..) => true
  | some (.operation operation _ arguments _) =>
      let own := match operation with
        | .move _ | .copy _ | .read _ | .borrow _ _ => true
        | .reference (.borrow _) | .reference .dereference => true
        | .primitive kind => match kind with
            | .equal | .notEqual | .less | .lessEqual | .greater | .greaterEqual | .length
            | .bitwiseAnd | .bitwiseOr | .bitwiseXor | .logicalNot | .cast => true
            | _ => false
        | _ => false
      own && arguments.all (pureValue state)
  | _ => false

/-- The temporary a statement assigns, where it is the only write of that
temporary in the graph and the value only computes. -/
private def pureAssignment? (context : Context) (state : State)
    (occurrences : Array Occurrence) (statement : RawStatement) :
    Option (LocalId × ExprId) := do
  let .execute expression := statement | none
  let .assign place value ← exprKind? state expression | none
  let localId ← bareLocal? context place
  guard (writesOf context occurrences localId == 1 && pureValue state value)
  pure (localId, value)

mutual
  private partial def placeSym? (context : Context) (state : State) (run : Run)
      (place : PlaceId) : Option Sym := do
    match ← context.places[place.index]? with
    | .localVar localId => pure (.local localId)
    | .deref base => pure (.deref (← placeSym? context state run base))
    | .field base owner field =>
        pure (.field (← placeSym? context state run base) owner field)
    | .index base index =>
        pure (.element (← placeSym? context state run base) (← resolve? context state run index))
    | _ => none

  /-- An expression's value in terms of what the run does not assign. -/
  private partial def resolve? (context : Context) (state : State) (run : Run) (id : ExprId) :
      Option Sym := do
    match ← exprKind? state id with
    | .value value _ => pure (.literal value)
    | .operation (.move place) _ _ _ | .operation (.copy place) _ _ _
    | .operation (.read place) _ _ _ =>
        if let some localId := bareLocal? context place then
          if let some (_, value) := run.find? (·.1 == localId) then
            return ← resolve? context state run value
        placeSym? context state run place
    | .operation (.borrow _ place) _ _ _ => pure (.borrowed (← placeSym? context state run place))
    | .operation (.reference (.borrow _)) _ #[operand] _ => resolve? context state run operand
    | .operation (.reference .dereference) _ #[operand] _ =>
        match ← resolve? context state run operand with
        | .borrowed place => pure place
        | reference => pure (.deref reference)
    | .operation (.primitive kind) _ #[operand] _ =>
        pure (.unary kind (← resolve? context state run operand))
    | .operation (.primitive kind) _ #[left, right] _ =>
        pure (.binary kind (← resolve? context state run left) (← resolve? context state run right))
    | _ => none
end

private def integerLiteral? : Sym → Option Int
  | .literal (.integer value) => some value
  | _ => none

/-! ## The operation a check guards -/

private structure Guard where
  block : Nat
  condition : ExprId
  expected : Bool
  kind : RawAssertKind

/-- The operations a check folds into, with the checked primitive of each. -/
private def checkedForm? : PrimitiveOperation → Option PrimitiveOperation
  | .divide => some (.checkedDivide .panic)
  | .modulo => some (.checkedModulo .panic)
  | .negate => some (.checkedNegate .panic)
  | .shiftLeft => some (.checkedShiftLeft .panic)
  | .shiftRight => some (.checkedShiftRight .panic)
  | _ => none

/-- The value and primitive of a statement assigning a checked form's
operation. -/
private def guardedOperation? (state : State) (statement : RawStatement) :
    Option (ExprId × PrimitiveOperation × Array ExprId) := do
  let .execute expression := statement | none
  let .assign _ value ← exprKind? state expression | none
  let .operation (.primitive kind) _ arguments _ ← exprKind? state value | none
  guard (checkedForm? kind).isSome
  pure (value, kind, arguments)

private def panicAssertion : RawAssertKind → Bool
  | .boundsCheck | .overflow | .divisionByZero | .remainderByZero => true
  | .misalignedPointerDereference | .profile _ => false

/-- The run of assignments a block ends in. -/
private def trailingRun (context : Context) (state : State) (occurrences : Array Occurrence)
    (block : RawBasicBlock) : Run :=
  Id.run do
    let mut run : Run := #[]
    for statement in block.statements.reverse do
      if let some assignment := pureAssignment? context state occurrences statement then
        run := run.push assignment
      else if statement matches .execute _ then break
    return run

/-- From a check, the following checks and the operation they lead to: every
block on the way is reached only from the one before and holds nothing but
assignments of temporaries. -/
private partial def guardChain? (context : Context) (state : State)
    (occurrences : Array Occurrence) (start : Nat) :
    Option (Array Guard × Run × ExprId × PrimitiveOperation × Array ExprId) := do
  let block ← state.blocks[start]?
  let run := trailingRun context state occurrences block
  go start #[] run (state.blocks.size + 1)
where
  go (current : Nat) (guards : Array Guard) (run : Run) (fuel : Nat) :
      Option (Array Guard × Run × ExprId × PrimitiveOperation × Array ExprId) := do
    guard (fuel != 0)
    let block ← state.blocks[current]?
    let .assert condition expected kind target .unreachable := block.terminator | none
    guard (panicAssertion kind)
    let guards := guards.push { block := current, condition, expected, kind : Guard }
    guard (predecessorCount state.blocks target.index == 1)
    let next ← state.blocks[target.index]?
    let mut run := run
    for statement in next.statements do
      if let some (value, operation, arguments) := guardedOperation? state statement then
        return (guards, run, value, operation, arguments)
      if let some assignment := pureAssignment? context state occurrences statement then
        run := run.push assignment
      else if statement matches .execute _ then none
    go target.index guards run (fuel - 1)

/-- Whether the checks are exactly those of the operation, over its operands. -/
private def checksMatch (context : Context) (state : State) (guards : Array Guard) (run : Run)
    (operation : PrimitiveOperation) (value : ExprId) (arguments : Array ExprId) : Bool :=
  (do
    let (width, signed) ← integerInfo? context (← typeOf? state value)
    let minimum : Int := - (2 : Int) ^ (width - 1)
    let conditions : Array (Guard × Sym) ← guards.mapM fun (check : Guard) => do
      pure (check, ← resolve? context state run check.condition)
    let isZeroCheck (divisor : Sym) (check : Guard) (condition : Sym) : Bool :=
      !check.expected &&
        (check.kind == RawAssertKind.divisionByZero || check.kind == RawAssertKind.remainderByZero) &&
        condition == Sym.binary .equal divisor (.literal (.integer 0))
    let isOverflowCheck (dividend divisor : Sym) (check : Guard) (condition : Sym) : Bool :=
      !check.expected && check.kind == RawAssertKind.overflow &&
        (condition == Sym.binary .bitwiseAnd
            (.binary .equal divisor (.literal (.integer (-1))))
            (.binary .equal dividend (.literal (.integer minimum))) ||
          (divisor == Sym.literal (.integer (-1)) &&
            condition == Sym.binary .equal dividend (.literal (.integer minimum))))
    match operation, arguments with
    | .divide, #[left, right] | .modulo, #[left, right] =>
        let left ← resolve? context state run left
        let right ← resolve? context state run right
        let needsZero := (integerLiteral? right).all (· == 0)
        let needsOverflow := signed &&
          (integerLiteral? right).all (· == -1) && (integerLiteral? left).all (· == minimum)
        let zero := conditions.filter fun (check, condition) => isZeroCheck right check condition
        let overflow := conditions.filter fun (check, condition) =>
          isOverflowCheck left right check condition
        pure (zero.size == (if needsZero then 1 else 0) &&
          overflow.size == (if needsOverflow then 1 else 0) &&
          zero.size + overflow.size == conditions.size)
    | .negate, #[operand] =>
        let operand ← resolve? context state run operand
        pure (signed && conditions.size == 1 && conditions.all fun (check, condition) =>
          !check.expected && check.kind == RawAssertKind.overflow &&
            condition == Sym.binary .equal operand (.literal (.integer minimum)))
    | .shiftLeft, #[_, distance] | .shiftRight, #[_, distance] =>
        let distance ← resolve? context state run distance
        -- A distance of a signed type is compared at its unsigned bits, under
        -- which a negative one is out of range as the checked form has it.
        pure (conditions.size == 1 && conditions.all fun (check, condition) =>
          check.expected && check.kind == RawAssertKind.overflow &&
            (condition == Sym.binary .less distance (.literal (.integer width)) ||
              condition == Sym.binary .less (.unary .cast distance) (.literal (.integer width))))
    | _, _ => pure false).getD false

/-! ## Rewriting -/

private def setKind (state : State) (id : ExprId) (kind : ExprKind) : State :=
  { state with expressions := state.expressions.modify id.index ({ · with kind }) }

private def setPrimitive (state : State) (id : ExprId) (primitive : PrimitiveOperation) : State :=
  match exprKind? state id with
  | some (.operation _ instantiations arguments surface) =>
      setKind state id (.operation (.primitive primitive) instantiations arguments surface)
  | _ => state

private def setTerminator (state : State) (block : Nat) (terminator : RawTerminator) : State :=
  { state with blocks := state.blocks.modify block ({ · with terminator }) }

private def foldGuards (state : State) (guards : Array Guard) : State :=
  guards.foldl (init := state) fun state check =>
    match (state.blocks[check.block]?.map (·.terminator) : Option RawTerminator) with
    | some (.assert _ _ _ target _) => setTerminator state check.block (.goto target)
    | _ => state

/-- Fold a check of the kind of a division, a negation or a shift. -/
private def foldOperationCheck (context : Context) (state : State) (start : Nat) : State :=
  (do
    let occurrences ← graphOccurrences context state
    let (guards, run, value, operation, arguments) ←
      guardChain? context state occurrences start
    guard (checksMatch context state guards run operation value arguments)
    let checked ← checkedForm? operation
    pure (setPrimitive (foldGuards state guards) value checked)).getD state

/-- An element read of a tuple at a literal position, and the local and the
position it reads. -/
private def tupleElement? (context : Context) (state : State) (place : PlaceId) :
    Option (PlaceId × LocalId × Int) := do
  let .index base index ← context.places[place.index]? | none
  let localId ← bareLocal? context base
  let .value (.integer position) _ ← exprKind? state index | none
  pure (base, localId, position)

/-- Fold an overflow check of an `overflowing*` operation: its tuple local
becomes the local of the value, where nothing else reads the tuple. -/
private def foldOverflowCheck (context : Context) (state : State) (index : Nat) : State :=
  (do
    let occurrences ← graphOccurrences context state
    let block ← state.blocks[index]?
    let .assert condition false .overflow target .unreachable := block.terminator | none
    let flagPlace ← match ← exprKind? state condition with
      | .operation (.move place) _ _ _ | .operation (.copy place) _ _ _ => some place
      | _ => none
    let (base, tupleLocal, 1) ← tupleElement? context state flagPlace | none
    let .execute assignment ← (block.statements.filter (fun (statement : RawStatement) => statement matches .execute _)).back? | none
    let .assign assigned value ← exprKind? state assignment | none
    guard (bareLocal? context assigned == some tupleLocal)
    guard (writesOf context occurrences tupleLocal == 1)
    let .operation (.primitive overflowing) instantiations arguments surface ←
      exprKind? state value | none
    let checked ← match overflowing with
      | .overflowingAdd => some (PrimitiveOperation.checkedAdd .panic)
      | .overflowingSubtract => some (.checkedSubtract .panic)
      | .overflowingMultiply => some (.checkedMultiply .panic)
      | _ => none
    let valueType ← typeOf? state (← arguments[0]?)
    -- Every read is the flag this check tests or the value.
    let reads := readsOf context occurrences tupleLocal
    let values := reads.filter fun (occurrence : Occurrence) => occurrence.node != condition
    let rewritten ← values.foldlM (init := state) fun state (occurrence : Occurrence) => do
      let (valueBase, _, 0) ← tupleElement? context state occurrence.place | none
      match ← exprKind? state occurrence.node with
      | .operation (.move _) instantiations arguments surface =>
          pure (setKind state occurrence.node
            (.operation (.move valueBase) instantiations arguments surface))
      | .operation (.copy _) instantiations arguments surface =>
          pure (setKind state occurrence.node
            (.operation (.copy valueBase) instantiations arguments surface))
      | _ => none
    let state := setKind rewritten assignment (.assign base value)
    let state := { state with
      expressions := state.expressions.modify value.index fun expression =>
        { expression with
          typeId := valueType
          kind := .operation (.primitive checked) instantiations arguments surface } }
    -- The tested flag is no one's any more; a value keeps the arena well typed.
    let state := setKind state condition (.value (.bool false))
    let state := { state with
      locals := state.locals.map fun (localDecl : LocalDecl) =>
        if localDecl.id == tupleLocal then
          { localDecl with type := { localDecl.type with typeId := valueType } }
        else localDecl }
    pure (setTerminator state index (.goto target))).getD state

/-- The levels a place indexes through: each base and its index. -/
private partial def indexLevels (context : Context) (place : PlaceId) : Array (PlaceId × ExprId) :=
  match context.places[place.index]? with
  | some (.index base index) => #[(base, index)] ++ indexLevels context base
  | some (.deref base) | some (.field base _ _) | some (.ghostField base _ _)
  | some (.subslice base _ _ _) | some (.unsize base) | some (.downcast base _) =>
      indexLevels context base
  | _ => #[]

/-- The expression observing a place's vector without copying its elements,
as the lowering of `v[i]` writes it, with its type. -/
private partial def observation? (context : Context) (state : State) (loc : LocId)
    (place : PlaceId) : Option (State × ExprId × TypeId) := do
  let push (state : State) (typeId : TypeId) (kind : ExprKind) : State × ExprId × TypeId :=
    ({ state with expressions := state.expressions.push { loc, typeId, kind } },
      ⟨state.expressions.size⟩, typeId)
  match ← context.places[place.index]? with
  | .localVar localId =>
      let localDecl ← state.locals.find? (·.id == localId)
      let typeId := localDecl.type.typeId
      match ← context.types[typeId.index]? with
      | .reference _ => pure (push state typeId (.localVar localId))
      | _ => pure (push state typeId (.operation (.read place) #[] #[]))
  | .deref base =>
      let (state, inner, innerType) ← observation? context state loc base
      let .reference reference ← context.types[innerType.index]? | none
      pure (push state reference.referent (.operation (.reference .dereference) #[] #[inner]))
  | _ => none

/-- Whether a length is the one of the collection at a place. An array's
check compares against the literal length of its type. -/
private def lengthOf (context : Context) (typeId : TypeId) (collection length : Sym) : Bool :=
  match length with
  | .unary .length observed => observed == collection
  | .literal (.integer count) =>
      match context.types[typeId.index]? with
      | some (.vector _ (some (.integer fixed))) => fixed == count
      | _ => false
  | _ => false

/-- Fold a bounds check into `checkVectorIndex` at the element access it
guards, so that the two stand together as the lowering of `v[i]` has them. -/
private def foldBoundsCheck (context : Context) (unitType : Option TypeId) (state : State)
    (index : Nat) : State :=
  (do
    let unitType ← unitType
    let occurrences ← graphOccurrences context state
    let block ← state.blocks[index]?
    let .assert condition true .boundsCheck target .unreachable := block.terminator | none
    guard (predecessorCount state.blocks target.index == 1)
    let next ← state.blocks[target.index]?
    let mut run := trailingRun context state occurrences block
    for (statement, position) in next.statements.zipIdx do
      let .execute expression := statement | continue
      let levels : Array (PlaceId × ExprId) :=
        (← exprOccurrences context state expression).flatMap fun (occurrence : Occurrence) =>
        indexLevels context occurrence.place
      let .binary .less tested length ← resolve? context state run condition | none
      let matching := levels.find? fun (base, element) =>
        resolve? context state run element == some tested &&
          (match placeSym? context state run base with
           | some collection =>
               match observation? context state block.loc base with
               | some (_, _, typeId) => lengthOf context typeId collection length
               | none => false
           | none => false)
      if let some (base, element) := matching then
        -- A literal index below an array's literal length needs no check.
        if let (.literal (.integer position), .literal (.integer count)) := (tested, length) then
          if 0 ≤ position && position < count then
            return setTerminator state index (.goto target)
        let (state, observed, _) ← observation? context state block.loc base
        let check : ExprId := ⟨state.expressions.size⟩
        let state := { state with
          expressions := state.expressions.push {
            loc := block.loc
            typeId := unitType
            kind := .operation (.primitive (.checkVectorIndex .panic)) #[] #[observed, element] } }
        let state := { state with
          blocks := state.blocks.modify target.index fun next =>
            { next with statements := next.statements.insertIdxIfInBounds position (.execute check) } }
        return setTerminator state index (.goto target)
      let some assignment := pureAssignment? context state occurrences statement | none
      run := run.push assignment
    none).getD state

/-- An operation rustc leaves unchecked because it cannot fail, a literal
divisor or distance in range, is the same checked operation as any other. -/
private def foldUnguarded (context : Context) (state : State) : State :=
  state.blocks.foldl (init := state) fun state block =>
    block.statements.foldl (init := state) fun state statement =>
      (do
        let (value, operation, arguments) ← guardedOperation? state statement
        let (width, signed) ← integerInfo? context (← typeOf? state value)
        let minimum : Int := - (2 : Int) ^ (width - 1)
        let noRun : Run := #[]
        let cannotFail ← match operation, arguments with
          | .divide, #[left, right] | .modulo, #[left, right] => do
              let divisor ← integerLiteral? (← resolve? context state noRun right)
              let dividend := integerLiteral? (← resolve? context state noRun left)
              pure (divisor != 0 && (!signed || divisor != -1 ||
                dividend.any (· != minimum)))
          | .shiftLeft, #[_, distance] | .shiftRight, #[_, distance] => do
              let distance ← integerLiteral? (← resolve? context state noRun distance)
              pure (0 ≤ distance && distance < width)
          | _, _ => pure false
        guard cannotFail
        pure (setPrimitive state value (← checkedForm? operation))).getD state

/-- Remove the assignments of temporaries that only the folded checks read. -/
private partial def removeDeadTemporaries (context : Context) (state : State)
    (candidates : Array LocalId) : State :=
  match graphOccurrences context state with
  | none => state
  | some occurrences =>
      let dead (statement : RawStatement) : Bool :=
        match pureAssignment? context state occurrences statement with
        | some (localId, _) =>
            candidates.contains localId && (readsOf context occurrences localId).isEmpty
        | none => false
      if state.blocks.any (·.statements.any dead) then
        removeDeadTemporaries context
          { state with blocks := state.blocks.map fun block =>
              { block with statements := block.statements.filter (!dead ·) } } candidates
      else state

/-- The locals the assert conditions read, through the assignments of the graph: what
folding their checks may leave unread. -/
private def conditionLocals (context : Context) (state : State) (conditions : Array ExprId) :
    Array LocalId := Id.run do
  let assignments := (state.blocks.flatMap (·.statements)).filterMap fun statement => do
    let .execute expression := statement | none
    let .assign place value ← exprKind? state expression | none
    return (← bareLocal? context place, value)
  let mut pending := conditions
  let mut reached : Array LocalId := #[]
  repeat
    let some id := pending.back? | break
    pending := pending.pop
    for occurrence in (exprOccurrences context state id).getD #[] do
      let some localId := rootLocal? context occurrence.place | continue
      unless reached.contains localId do
        reached := reached.push localId
        for (assigned, value) in assignments do
          if assigned == localId then pending := pending.push value
  return reached

/-- Fold the checks of a raw graph. A graph holding a form this pass does not
read is left as it is. -/
def foldChecks (places : Array Place) (types : Array Ty) (pointerWidth : Option Nat)
    (unitType : Option TypeId) (expressions : Array Expr) (locals : Array LocalDecl) (graph : RawCfg) :
    Array Expr × Array LocalDecl × RawCfg :=
  let context : Context := { places, types, pointerWidth }
  let initial : State := { expressions, locals, blocks := graph.blocks }
  let conditions := graph.blocks.filterMap fun block => match block.terminator with
    | .assert condition _ kind _ _ => if panicAssertion kind then some condition else none
    | _ => none
  let candidates := conditionLocals context initial conditions
  let indices := Array.range graph.blocks.size
  let overflow := indices.foldl (foldOverflowCheck context) initial
  let operations := indices.foldl (foldOperationCheck context) overflow
  let bounds := indices.foldl (foldBoundsCheck context unitType) operations
  let folded := foldUnguarded context bounds
  let state := removeDeadTemporaries context folded candidates
  (state.expressions, state.locals, { graph with blocks := state.blocks })

end VerexIR.Import.FoldChecks
