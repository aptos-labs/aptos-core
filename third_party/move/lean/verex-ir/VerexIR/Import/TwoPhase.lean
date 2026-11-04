-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Import.FoldChecks
import VerexIR.Validation.Diagnostic

/-!
# Minting two-phase borrows

`v.push(v.len())` reserves `&mut v`, reads `v` for the argument, and activates the
borrow at the call. The runtime has one kind of mutable borrow, which takes the value
when it runs, so a raw graph is normalized before structurization: a two-phase borrow
held in a temporary is minted at the temporary's one use. Reads between see the lender's
old value, because the prophecy enters its slot only at activation, and the folded unit
is checked by the borrow analysis as any other.

Optimized MIR spells some of these borrows as a copy of the reference (`add(h.r, *h.r)`
is `_7 = copy h.r` used once at the call), which the exporter marks as a reference copy.
A copy that is not an operand's one use is the reborrow it reads as, where it stands.

Minting moves the place to the use, so nothing between may change what it names or lend
it: a written, moved or mutably borrowed place overlapping it, or an index local of it.
Rustc admits assigning a deref's base between (`p = q` while `*p` is reserved), where
the reservation names the old referent. That is rejected at the borrow, as is a two-phase
borrow that cannot be minted at all; the runtime gives its kind no meaning.
-/

namespace VerexIR.Import.TwoPhase

open VerexIR.Validation
open VerexIR.Import.FoldChecks

/-- One step of a place, as far as telling two places apart needs. -/
private inductive Step where
  | field (name : NameId)
  | downcast (variant : NameId)
  | other
  deriving BEq

/-- A place's root local, its steps from the root, and the index expressions on them. -/
private partial def pathOf (places : Array Place) (place : PlaceId) :
    Option (LocalId × List Step × List ExprId) :=
  match places[place.index]? with
  | some (.localVar root) => some (root, [], [])
  | some (.field base _ name) | some (.ghostField base _ name) => extend base (.field name) []
  | some (.downcast base variant) => extend base (.downcast variant) []
  | some (.index base index) => extend base .other [index]
  | some (.deref base) | some (.subslice base ..) | some (.unsize base) => extend base .other []
  | none => none
where
  extend (base : PlaceId) (step : Step) (indexes : List ExprId) :=
    (pathOf places base).map fun (root, steps, inner) =>
      (root, steps ++ [step], inner ++ indexes)

/-- Whether one path is a prefix of the other, step by step: distinct fields and
variants are disjoint, and anything else may be the same. -/
private def overlap : List Step → List Step → Bool
  | [], _ | _, [] => true
  | .field left :: lefts, .field right :: rights
  | .downcast left :: lefts, .downcast right :: rights =>
      left == right && overlap lefts rights
  | _ :: lefts, _ :: rights => overlap lefts rights

/-- What one position of a block reaches: a statement's, or the terminator's. -/
private def statementOccurrences (context : Context) (state : State) :
    RawStatement → Option (Array Occurrence)
  | .execute expression => exprOccurrences context state expression
  | .deinit place | .setDiscriminant place _ =>
      placeOccurrences context state place ⟨0⟩ .write
  | .storageLive _ | .storageDead _ | .retag _ | .placeMention _ | .ascribeUserType .. | .profile _ =>
      some #[]

private def terminatorOccurrences (context : Context) (state : State) (block : RawBasicBlock) :
    Option (Array Occurrence) :=
  blockOccurrences context state { block with statements := #[] }

/-- The marked borrow a statement assigns: the assignment's place, the borrow's node, its
kind and its place. -/
private def markedAssignment? (state : State) : RawStatement →
    Option (PlaceId × ExprId × BorrowKind × PlaceId)
  | .execute expression => do
      let .assign target value ← exprKind? state expression | none
      let .operation (.borrow kind place) _ _ _ ← exprKind? state value | none
      guard (kind == rustTwoPhase || kind == rustReferenceCopy)
      return (target, value, kind, place)
  | _ => none

/-- Whether the node reads the bare local as an operand: a move or copy of it, or the
reborrow a reference copy reads as. -/
private def operandOf (context : Context) (state : State) (localId : LocalId) (node : ExprId) :
    Bool :=
  match exprKind? state node with
  | some (.operation (.move place) _ #[] _) | some (.operation (.copy place) _ #[] _) =>
      bareLocal? context place == some localId
  | some (.operation (.borrow kind place) _ #[] _) =>
      (kind == .mutable || kind == rustTwoPhase || kind == rustReferenceCopy) &&
        match context.places[place.index]? with
        | some (.deref base) => bareLocal? context base == some localId
        | _ => false
  | _ => false

/-- Whether an occurrence changes or lends `borrowed`. -/
private def conflicts (context : Context) (state : State) (borrowed : PlaceId)
    (occurrence : Occurrence) : Bool :=
  let changes := occurrence.access == .write || match exprKind? state occurrence.node with
    | some (.operation (.move place) ..) => place == occurrence.place
    -- A reference copy in between reads: rustc admitted no write through it.
    | some (.operation (.borrow kind place) ..) =>
        place == occurrence.place && kind != .immutable && kind != rustReferenceCopy
    | _ => false
  changes && match pathOf context.places borrowed, pathOf context.places occurrence.place with
    | some (root, steps, indexes), some (other, others, _) =>
        (root == other && overlap steps others) ||
          indexes.any fun index =>
            ((exprOccurrences context state index).getD #[]).any fun read =>
              rootLocal? context read.place == some other
    | _, _ => true

/-- Whether a path from just after a position to the use may change or lend the place.
A path ends at the use, or where the borrow runs again. -/
private def changedBetween (context : Context) (state : State)
    (borrowed : PlaceId) (own use : ExprId) (block position : Nat) : Bool := Id.run do
  let mut pending := #[(block, position)]
  let mut visited : Array Nat := #[]
  while true do
    let some (current, start) := pending.back? | return false
    pending := pending.pop
    let some raw := state.blocks[current]? | continue
    let mut reachedUse := false
    for statement in raw.statements.extract start raw.statements.size do
      let occurrences := (statementOccurrences context state statement).getD #[]
      if occurrences.any (fun occurrence => occurrence.node == use || occurrence.node == own) then
        reachedUse := true
        break
      if occurrences.any (conflicts context state borrowed) then return true
    if reachedUse then continue
    let occurrences := (terminatorOccurrences context state raw).getD #[]
    if occurrences.any (·.node == use) then continue
    if occurrences.any (conflicts context state borrowed) then return true
    for next in raw.terminator.successors do
      unless visited.contains next.index do
        visited := visited.push next.index
        pending := pending.push (next.index, 0)
  return false

private def setKind (state : State) (node : ExprId) (kind : ExprKind) : State :=
  { state with expressions := state.expressions.modify node.index fun expression =>
      { expression with kind } }

private def rejection (loc : LocId) (reason : String) : Diagnostic :=
  .at "VIR-RUST-TWO-PHASE" s!"a two-phase borrow {reason}" loc

/-- Settle the marked borrow at one position: mint it at its use, or reject it. A
reference copy that is not an operand's one use stays, for the end. -/
private def settle (context : Context) (state : State) (block index : Nat)
    (target : PlaceId) (value : ExprId) (kind : BorrowKind) (borrowed : PlaceId) :
    Except Diagnostic State := do
  let some valueNode := state.expressions[value.index]?
    | throw <| .error "VIR-RUST-TWO-PHASE" "a two-phase borrow is out of range"
  let unminted (reason : String) : Except Diagnostic State :=
    if kind == rustReferenceCopy then pure state
    else throw (rejection valueNode.loc reason)
  let some temporary := bareLocal? context target
    | unminted "is not held in a temporary"
  let some occurrences := graphOccurrences context state
    | unminted "stands in a body this pass cannot read"
  let ofTemporary := occurrences.filter fun occurrence =>
    rootLocal? context occurrence.place == some temporary
  let writes := ofTemporary.filter (·.access == .write)
  let reads := ofTemporary.filter (·.access == .read)
  -- A reservation nothing activates, its use being in a collapsed panic region, has
  -- no effect.
  if reads.isEmpty && kind == rustTwoPhase then
    return { state with blocks := state.blocks.modify block fun raw =>
      { raw with statements := raw.statements.eraseIdx! index } }
  let #[use] := reads | unminted "has no single use"
  unless writes.size == 1 && operandOf context state temporary use.node do
    return ← unminted "is not an operand's one use"
  if changedBetween context state borrowed value use.node block (index + 1) then
    throw (rejection valueNode.loc
      "names a place that is written, moved or mutably borrowed before its use")
  -- A use that assigns another temporary is settled there in its turn.
  let assigned := state.blocks.any fun block => block.statements.any fun statement =>
    match statement with
    | .execute expression => match exprKind? state expression with
        | some (.assign _ assignedValue) => assignedValue == use.node
        | _ => false
    | _ => false
  let minted : BorrowKind := if assigned then kind else .mutable
  let state := setKind state use.node (.operation (.borrow minted borrowed) #[] #[] none)
  pure { state with blocks := state.blocks.modify block fun raw =>
    { raw with statements := raw.statements.eraseIdx! index } }

/-- Whether a place goes through a dereference of a local shared reference. -/
private partial def throughSharedLocal (types : Array Ty) (locals : Array LocalDecl)
    (places : Array Place) (place : PlaceId) : Bool :=
  match places[place.index]? with
  | some (.deref base) =>
      let isShared (localId : LocalId) : Bool :=
        (locals[localId.index]?).any fun (declaration : LocalDecl) =>
          match types[declaration.type.typeId.index]? with
          | some (.reference reference) => reference.kind == .shared
          | _ => false
      let viaLocal := match places[base.index]? with
        | some (.localVar localId) => isShared localId
        | _ => false
      viaLocal || throughSharedLocal types locals places base
  | some (.field base _ _) | some (.ghostField base _ _) | some (.index base _)
  | some (.subslice base ..) | some (.unsize base) | some (.downcast base _) =>
      throughSharedLocal types locals places base
  | some (.localVar _) | none => false

/-- A place through a temporary, the temporary's dereference replaced by the place the
temporary's reborrow names. -/
private partial def rebased (temporary : LocalId) (borrowed : PlaceId) (places : Array Place)
    (place : PlaceId) : Array Place × PlaceId :=
  let step (base : PlaceId) (node : PlaceId → Place) :=
    let (places, base) := rebased temporary borrowed places base
    (places.push (node base), ⟨places.size⟩)
  match places[place.index]? with
  | some (.deref base) =>
      if places[base.index]? == some (.localVar temporary) then (places, borrowed)
      else step base .deref
  | some (.field base owner name) => step base (.field · owner name)
  | some (.ghostField base owner name) => step base (.ghostField · owner name)
  | some (.index base index) => step base (.index · index)
  | some (.subslice base start stop fromEnd) => step base (.subslice · start stop fromEnd)
  | some (.unsize base) => step base .unsize
  | some (.downcast base variant) => step base (.downcast · variant)
  | some (.localVar _) | none => (places, place)

/-- What a place operation reads, with its place replaced. -/
private def withPlace (kind : ExprKind) (place : PlaceId) : Option ExprKind :=
  match kind with
  | .operation (.move _) instantiations arguments access =>
      some (.operation (.move place) instantiations arguments access)
  | .operation (.copy _) instantiations arguments access =>
      some (.operation (.copy place) instantiations arguments access)
  | .operation (.read _) instantiations arguments access =>
      some (.operation (.read place) instantiations arguments access)
  | .operation (.borrow borrowKind _) instantiations arguments access =>
      some (.operation (.borrow borrowKind place) instantiations arguments access)
  | _ => none

/-- A copy of a mutable reference through a shared view holds what the view does, no
loan: where the temporary is only dereferenced, each use reads the copied place itself.
A shared view holds nothing a reborrow could lend. -/
private def foldSharedCopy (types : Array Ty) (places : Array Place) (state : State) (block index : Nat)
    (temporaryPlace borrowed : PlaceId) : Option (Array Place × State) := do
  let context : Context := { places, types, pointerWidth := none }
  let temporary ← bareLocal? context temporaryPlace
  let occurrences ← graphOccurrences context state
  let ofTemporary := occurrences.filter fun occurrence =>
    rootLocal? context occurrence.place == some temporary
  let writes := ofTemporary.filter (·.access == .write)
  let reads := ofTemporary.filter (·.access == .read)
  guard (writes.size == 1 && reads.all fun read => bareLocal? context read.place == none)
  let (places, state) ← reads.foldlM (init := (places, state)) fun (places, state) read => do
    let kind ← exprKind? state read.node
    let (places, rebasedPlace) := rebased temporary borrowed places read.place
    let kind ← withPlace kind rebasedPlace
    pure (places, setKind state read.node kind)
  pure (places, { state with blocks := state.blocks.modify block fun raw =>
    { raw with statements := raw.statements.eraseIdx! index } })

/-- Mint the marked borrows of a raw graph, in turn. Those that stay are reference copies,
each the plain reborrow it reads as: they are read as such only now, since a borrow
minted across one must see that it is a copy. -/
def mintBorrows (places : Array Place) (types : Array Ty) (expressions : Array Expr)
    (locals : Array LocalDecl) (graph : RawCfg) :
    Except Diagnostic (Array Place × Array Expr × RawCfg) := do
  let mut places := places
  let mut state : State := { expressions, locals, blocks := graph.blocks }
  let mut stays : Array ExprId := #[]
  let marked (state : State) := state.blocks.foldl (fun count block =>
    count + (block.statements.filter fun statement =>
      (markedAssignment? state statement).isSome).size) 0
  for _ in [0:(marked state) + 1] do
    let found := state.blocks.zipIdx.findSome? fun (block, index) =>
      block.statements.zipIdx.findSome? fun (statement, position) => do
        let assignment ← markedAssignment? state statement
        if stays.contains assignment.2.1 then none else some (index, position, assignment)
    let some (block, position, target, value, kind, borrowed) := found
      | break
    if kind == rustReferenceCopy && throughSharedLocal types locals places borrowed then
      if let some (folded, foldedState) :=
          foldSharedCopy types places state block position target borrowed then
        places := folded
        state := foldedState
        continue
    let context : Context := { places, types, pointerWidth := none }
    let settled ← settle context state block position target value kind borrowed
    -- A statement left in place stays.
    if settled.blocks[block]?.map (·.statements.size) ==
        state.blocks[block]?.map (·.statements.size) then
      stays := stays.push value
    state := settled
  for value in stays do
    if let some (.operation (.borrow _ borrowed) _ _ _) := exprKind? state value then
      state := setKind state value (.operation (.borrow .mutable borrowed) #[] #[] none)
  pure (places, state.expressions, { graph with blocks := state.blocks })

end VerexIR.Import.TwoPhase
