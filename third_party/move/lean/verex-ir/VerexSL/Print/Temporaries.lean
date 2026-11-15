-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Validated

/-!
# Substituting compiler temporaries

The VerexSL rendering writes a compiler temporary's value where the
temporary is read instead of declaring it, but only where that keeps what
the body does. The temporary is written once and every read follows the
write in the same region. A value that only reads may be written at each
read when nothing evaluated before its last read writes what it reads; any
other value is read once, on every path, outside any loop, and commutes with
everything evaluated before that read.

What evaluating code does is a sequence of events in evaluation order: reads
of the temporary, accesses summarized by a footprint, and the bounds of
alternatives and loops.
-/

namespace VerexSL.Print.Temporaries

open VerexIR VerexIR.Validation

/-- What evaluating an expression observes and changes. -/
structure Footprint where
  /-- Locals read, directly or as the reference a dereference goes through. -/
  reads : Array LocalId := #[]
  /-- Locals assigned, moved from, or mutably borrowed. -/
  writes : Array LocalId := #[]
  /-- Whether it reads through a reference. -/
  readsMemory : Bool := false
  /-- Whether it writes or mutably borrows through a reference, or calls. -/
  writesMemory : Bool := false
  /-- Whether it calls, borrows mutably, drops, or leaves the region. -/
  acts : Bool := false
  /-- Whether it can fail. -/
  fails : Bool := false
  deriving Inhabited

def Footprint.merge (left right : Footprint) : Footprint where
  reads := left.reads ++ right.reads
  writes := left.writes ++ right.writes
  readsMemory := left.readsMemory || right.readsMemory
  writesMemory := left.writesMemory || right.writesMemory
  acts := left.acts || right.acts
  fails := left.fails || right.fails

/-- Whether the value only reads: evaluating it again gives the same value
as long as what it reads is unchanged. -/
def Footprint.onlyReads (footprint : Footprint) : Bool :=
  footprint.writes.isEmpty && !footprint.writesMemory && !footprint.acts && !footprint.fails

/-- Whether evaluating `value` after `between` rather than before it gives
the same value and the same behavior. -/
def Footprint.commutes (value between : Footprint) : Bool :=
  let disjoint (left right : Array LocalId) := !left.any right.contains
  disjoint between.writes value.reads && disjoint value.writes between.reads &&
    disjoint between.writes value.writes &&
    !(value.readsMemory && between.writesMemory) &&
    !(value.writesMemory && between.readsMemory) &&
    (!(value.acts || value.fails) || !(between.acts || between.fails))

/-- One step of evaluation. -/
inductive Event where
  /-- A read of the temporary. -/
  | read
  | access (footprint : Footprint)
  /-- Alternatives begin: what follows up to `merge` runs on some paths. -/
  | branch
  | merge
  | loopStart
  | loopEnd
  deriving Inhabited

/-- Primitives that neither fail nor act: all others are taken to fail. -/
private def primitiveTotal : PrimitiveOperation → Bool
  | .tuple | .vector | .repeatVector | .pushVector | .concatVector | .containsVector |
      .indexOfVector | .length | .add | .subtract | .multiply | .overflowingAdd |
      .overflowingSubtract | .overflowingMultiply | .bitwiseOr | .bitwiseAnd | .bitwiseXor |
      .bitwiseNot | .logicalAnd | .logicalOr | .logicalNot | .equal | .notEqual | .less |
      .greater | .lessEqual | .greaterEqual | .compare | .signerAddress | .negate |
      .copyValue | .moveValue | .cast | .range | .implies | .equivalent | .identical => true
  | .destroyEmptyVector | .insertVector | .removeVector | .swapVector |
      .reverseSliceVector | .checkVectorIndex _ | .index | .slice | .checkedAdd _ |
      .checkedSubtract _ | .checkedMultiply _ | .modulo | .checkedModulo _ | .divide |
      .checkedDivide _ | .shiftLeft | .checkedShiftLeft _ | .shiftRight |
      .checkedShiftRight _ | .checkedNegate _ | .checkedCast _ => false

/-- How an operation accesses its place. -/
private inductive Access where
  | read
  | move
  | borrowMutable
  | write

private partial def patternLocals (ns : ValidatedNamespace) (pattern : PatternId) :
    Array LocalId :=
  match ns.patterns[pattern.index]? with
  | some { kind := .variable localId, .. } => #[localId]
  | some { kind := .tuple elements, .. } | some { kind := .constructor _ _ _ elements, .. } =>
      elements.flatMap (patternLocals ns)
  | _ => #[]

/-- The local a place is rooted at, the index expressions on the way, and
whether the place goes through a reference. -/
private partial def placeRoot (ns : ValidatedNamespace) (place : PlaceId) :
    Option (LocalId × Array ExprId × Bool) :=
  match ns.places[place.index]? with
  | some (.localVar root) => some (root, #[], false)
  | some (.deref base) => (placeRoot ns base).map fun (root, indexes, _) => (root, indexes, true)
  | some (.index base index) =>
      (placeRoot ns base).map fun (root, indexes, through) => (root, indexes.push index, through)
  | some (.field base ..) | some (.ghostField base ..) | some (.subslice base ..) |
      some (.unsize base) | some (.downcast base _) => placeRoot ns base
  | none => none

private def unknown : Footprint :=
  { acts := true, fails := true, readsMemory := true, writesMemory := true }

mutual
  private partial def placeEvents (ns : ValidatedNamespace) (target : Option LocalId)
      (place : PlaceId) (access : Access) : Array Event :=
    match placeRoot ns place with
    | none => #[.access unknown]
    | some (root, indexes, throughReference) =>
        indexes.flatMap (events ns target) |>.push <|
          if target == some root then .read else .access <|
            match access, throughReference with
            | .read, false => { reads := #[root] }
            | .read, true => { reads := #[root], readsMemory := true }
            | .move, _ => { reads := #[root], writes := #[root] }
            | .borrowMutable, false => { reads := #[root], writes := #[root], acts := true }
            | .borrowMutable, true =>
                { reads := #[root], readsMemory := true, writesMemory := true, acts := true }
            | .write, false => { writes := #[root] }
            | .write, true => { reads := #[root], writesMemory := true, acts := true }

  /-- The events of evaluating an expression; reads of `target` are `read`. -/
  partial def events (ns : ValidatedNamespace) (target : Option LocalId) (id : ExprId) :
      Array Event :=
    match ns.expressions[id.index]? with
    | none => #[.access unknown]
    | some expression => match expression.kind with
      | .value .. | .constant _ | .spec _ | .quantifier .. => #[]
      | .localVar localId =>
          #[if target == some localId then .read else .access { reads := #[localId] }]
      | .operation operation _ arguments _ =>
          -- In Rust, moving or dropping a value that owns its contents ends the
          -- loans of it, which a read through a reference must not outlive.
          let endsLoans :=
            ns.profile == some .rust && match ns.tables.types[expression.typeId.index]? with
              | some (.vector ..) | some (.slice ..) | some (.nominal ..) | some .string => true
              | _ => false
          arguments.flatMap (events ns target) ++ match operation with
            | .copy place | .read place | .readDiscriminant place => placeEvents ns target place .read
            | .move place =>
                (placeEvents ns target place .move).push (.access { writesMemory := endsLoans })
            | .borrow .mutable place => placeEvents ns target place .borrowMutable
            | .borrow _ place => placeEvents ns target place .read
            | .write place => placeEvents ns target place .write
            | .drop place =>
                (placeEvents ns target place .move).push
                  (.access { acts := true, writesMemory := ns.profile == some .rust })
            | .primitive primitive => #[.access { fails := !primitiveTotal primitive }]
            | .reference .dereference => #[.access { readsMemory := true }]
            | .reference (.freeze _) | .reference (.borrow .immutable) => #[]
            | .data (.select ..) | .data (.selectGhost ..) | .data (.testVariants ..) |
                .data (.discriminant _) | .data (.updateField ..) => #[]
            | .data (.selectVariants ..) => #[.access { fails := true }]
            | _ => #[.access unknown]
      | .block statements result =>
          (statements ++ result.toArray).flatMap (events ns target)
      | .letDecl pattern value body =>
          value.toArray.flatMap (events ns target) ++
            #[.access { writes := patternLocals ns pattern }] ++ events ns target body
      | .ifElse condition thenBranch elseBranch =>
          events ns target condition ++ #[.branch] ++ events ns target thenBranch ++
            elseBranch.toArray.flatMap (events ns target) ++ #[.merge]
      | .match_ scrutinee arms =>
          events ns target scrutinee ++ #[.access { fails := true }, .branch] ++
            arms.flatMap (fun arm =>
              #[.access { writes := patternLocals ns arm.pattern }] ++
                arm.guard.toArray.flatMap (events ns target) ++ events ns target arm.body) ++
            #[.merge]
      | .loop _ body => #[.loopStart] ++ events ns target body ++ #[.loopEnd]
      | .break_ _ value =>
          (value.toArray.flatMap (events ns target)).push (.access { acts := true })
      | .continue_ _ => #[.access { acts := true }]
      | .return_ values => (values.flatMap (events ns target)).push (.access { acts := true })
      | .throw_ _ arguments =>
          (arguments.flatMap (events ns target)).push (.access { acts := true, fails := true })
      | .assign place value => events ns target value ++ placeEvents ns target place .write
      | .assignPattern pattern value =>
          (events ns target value).push (.access { writes := patternLocals ns pattern })
end

/-- What all the events access. -/
def footprint (trace : Array Event) : Footprint :=
  trace.foldl (init := {}) fun footprint event => match event with
    | .access other => footprint.merge other
    | _ => footprint

/-- The first read: what is accessed before it, whether it is on only some
paths, and whether a loop repeats it. -/
def firstRead? (trace : Array Event) : Option (Footprint × Bool × Bool) := Id.run do
  let mut before : Footprint := {}
  let mut alternatives := 0
  let mut loops := 0
  for event in trace do
    match event with
    | .read => return some (before, alternatives > 0, loops > 0)
    | .access footprint => before := before.merge footprint
    | .branch => alternatives := alternatives + 1
    | .merge => alternatives := alternatives - 1
    | .loopStart => loops := loops + 1
    | .loopEnd => loops := loops - 1
  return none

/-- What is accessed before the last read; a loop that reads counts whole,
since its later iterations run before its last read. -/
def beforeLastRead (trace : Array Event) : Footprint := Id.run do
  let mut accessed : Footprint := {}
  let mut before : Footprint := {}
  let mut loops := 0
  let mut readInLoop := false
  for event in trace do
    match event with
    | .read =>
        before := accessed
        if loops > 0 then readInLoop := true
    | .access footprint => accessed := accessed.merge footprint
    | .loopStart => loops := loops + 1
    | .loopEnd =>
        loops := loops - 1
        if loops == 0 && readInLoop then
          before := accessed
          readInLoop := false
    | .branch | .merge => pure ()
  return before

def reads (trace : Array Event) : Nat :=
  trace.foldl (init := 0) fun count event => if event matches .read then count + 1 else count

/-- How often each local of a function is read and written, whether it is the
base of a projected place or a tag read, and whether it changes in place: a write or mutable
borrow of it or of a part of it, not through a reference. A local projected by
an assignment's target counts as read. -/
structure Uses where
  reads : Array Nat
  writes : Array Nat
  /-- The value a local is last assigned. -/
  values : Array (Option ExprId) := #[]
  bases : Array Bool := #[]
  changedInPlace : Array Bool := #[]

/-- Count the uses of the locals in the region rooted at `root`. -/
partial def Uses.of (ns : ValidatedNamespace) (localCount : Nat) (root : ExprId) : Uses :=
  let bump (counts : Array Nat) (localId : LocalId) : Array Nat :=
    if localId.index < counts.size then counts.modify localId.index (· + 1) else counts
  let rec visit (uses : Uses) (id : ExprId) : Uses :=
    match ns.expressions[id.index]? with
    | none => uses
    | some expression =>
      let read (uses : Uses) (place : PlaceId) (requiresPlace : Bool := false) : Uses :=
        match placeRoot ns place with
        | some (localId, indexes, _) =>
            let uses := indexes.foldl visit uses
            let projected := requiresPlace || !(ns.places[place.index]? matches some (.localVar _))
            { uses with
              reads := bump uses.reads localId
              bases := if projected && localId.index < uses.bases.size then
                uses.bases.set! localId.index true else uses.bases }
        | none => uses
      let changeInPlace (uses : Uses) (place : PlaceId) : Uses :=
        match placeRoot ns place with
        | some (localId, _, false) =>
            if localId.index < uses.changedInPlace.size then
              { uses with changedInPlace := uses.changedInPlace.set! localId.index true }
            else uses
        | _ => uses
      let bind (uses : Uses) (pattern : PatternId) : Uses :=
        { uses with writes := (patternLocals ns pattern).foldl bump uses.writes }
      match expression.kind with
      | .value .. | .constant _ | .continue_ _ => uses
      | .localVar localId => { uses with reads := bump uses.reads localId }
      | .operation operation _ arguments _ =>
          let uses := arguments.foldl visit uses
          match operation with
          | .borrow .mutable place | .write place => read (changeInPlace uses place) place
          | .readDiscriminant place => read uses place true
          | .move place | .copy place | .read place | .borrow _ place | .drop place =>
              read uses place
          | _ => uses
      | .block statements result => (statements ++ result.toArray).foldl visit uses
      | .letDecl pattern value body =>
          let uses := match value with
            | some value => bind (visit uses value) pattern
            | none => uses
          visit uses body
      | .ifElse condition thenBranch elseBranch =>
          (#[condition, thenBranch] ++ elseBranch.toArray).foldl visit uses
      | .match_ scrutinee arms =>
          arms.foldl (init := visit uses scrutinee) fun uses arm =>
            (arm.guard.toArray.push arm.body).foldl visit (bind uses arm.pattern)
      | .loop _ body => visit uses body
      | .break_ _ value => value.toArray.foldl visit uses
      | .return_ values | .throw_ _ values => values.foldl visit uses
      | .assign place value =>
          let uses := visit uses value
          match ns.places[place.index]? with
          | some (.localVar localId) =>
              { uses with writes := bump uses.writes localId
                          values := uses.values.setIfInBounds localId.index (some value) }
          | _ => read (changeInPlace uses place) place
      | .assignPattern pattern value => bind (visit uses value) pattern
      | .quantifier _ binders triggers condition body =>
          (binders.map (·.domain) ++ triggers.flatten ++ condition.toArray.push body).foldl
            visit uses
      | .spec block =>
          (block.conditions.flatMap fun condition =>
            #[condition.expression] ++ condition.auxiliary.map (·.2)).foldl visit
            ((block.frame.map (·.modifies)).getD #[] |>.foldl visit uses)
  let empty : Uses := {
    reads := Array.replicate localCount 0
    writes := Array.replicate localCount 0
    values := Array.replicate localCount none
    bases := Array.replicate localCount false
    changedInPlace := Array.replicate localCount false }
  visit empty root

/-- Whether `target` must remain a place: it is projected from or has its tag
read. Substitution could produce invalid `*copy(value.field)` or a tag read
whose argument is an arbitrary expression rather than an assignable place. -/
def Uses.projected (uses : Uses) (target : LocalId) : Bool :=
  uses.bases[target.index]?.getD false

/-- Whether the function never reads `target`. -/
def Uses.unread (uses : Uses) (target : LocalId) : Bool :=
  uses.reads[target.index]? == some 0

/-- Whether code in the regions `following` writes `target` or changes it in
place, so that a binding they follow declares it `mut`. -/
def writtenIn (ns : ValidatedNamespace) (localCount : Nat) (following : Array ExprId)
    (target : LocalId) : Bool :=
  following.any fun region =>
    let uses := Uses.of ns localCount region
    uses.writes[target.index]?.any (· > 0) || uses.changedInPlace[target.index]?.getD false

/-- What evaluating `value` does once the temporaries it reads are written where they are
read: its own footprint and those of the `temporary` locals it reads whose values may be
written at their reads, which move along with it. Those are the temporaries `substitutable`
can approve: one read once, and one read more often whose value only reads. -/
private def carriedFootprint (ns : ValidatedNamespace) (uses : Uses) (temporary : LocalId → Bool) :
    Nat → ExprId → Footprint
  | 0, value => footprint (events ns none value)
  | fuel + 1, value =>
      let own := footprint (events ns none value)
      (own.reads.filter temporary).foldl (init := own) fun found localId =>
        match uses.values[localId.index]?, uses.reads[localId.index]?,
            uses.writes[localId.index]? with
        | some (some assigned), some reads, some 1 =>
            let carried := carriedFootprint ns uses temporary fuel assigned
            if reads == 1 || carried.onlyReads then found.merge carried else found
        | _, _, _ => found

/-- Whether the assignment of `value` to the temporary `target` at
`statements[index]` of a region may be left out and `value` written where
`target` is read; see the module documentation. -/
def substitutable (ns : ValidatedNamespace) (uses : Uses) (temporary : LocalId → Bool)
    (statements : Array ExprId) (result : Option ExprId) (index : Nat) (target : LocalId)
    (value : ExprId) : Bool :=
  let following := (statements.extract (index + 1) statements.size) ++ result.toArray
  let trace := following.flatMap (events ns (some target))
  let value := carriedFootprint ns uses temporary uses.values.size value
  !uses.projected target && uses.writes[target.index]? == some 1 &&
    uses.reads[target.index]? == some (reads trace) &&
    if value.onlyReads then value.commutes (beforeLastRead trace)
    else reads trace == 1 && match firstRead? trace with
      | some (before, conditional, repeated) =>
          !repeated && value.commutes before && (!conditional || !(value.acts || value.fails))
      | none => false

end VerexSL.Print.Temporaries
