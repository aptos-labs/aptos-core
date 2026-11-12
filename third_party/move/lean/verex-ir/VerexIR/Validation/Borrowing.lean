-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Diagnostic
import VerexIR.Validation.PlaceIndex
import VerexIR.Validation.Validated
import VerexIR.Validation.ReceiverNoEscape

/-!
# Conservative structured-loan analysis

This checked slice gives direct and statically projected local borrows a
frontend-independent meaning. Loans bound to local reference holders,
including direct local aliases and reborrows, die after every known holder's
last use in a sequential structured block; `if` branches and match arms
shorten holders independently, and nested evaluation preserves loans needed by
later operands without extending them past their final operand. Loop fixed points
retain only loans needed by the body or continuation. Unequal nonnegative
literal indexes, disjoint forward subslices, and literal indexes outside a
forward subslice do not overlap; dynamic and from-end projections remain
conservative. Tuple and constructor patterns select each child binding's
carrier path even when the aggregate producer is a conditional or match.
Whole tuple, vector, and same-unit nominal holders retain component
carrier paths so projected uses select only overlapping loans, and consuming a
projection releases only its selected carriers. Conditional and match result
joins preserve those paths across their alternative producers. Overwriting an
owned direct or projected holder ends its old loans before an independent
replacement expression is evaluated and attaches the replacement at the same
carrier path. Nested evaluation protects later uses by loan identity rather
than preserving every sibling loan at the same aggregate root. Direct
call-result selection follows structural, instantiated
type-parameter, and namespace-resolved nominal fields, cutting recursive back-edges
with the same fully instantiated arguments while retaining lifetimes found in
their nonrecursive siblings; argument-changing recursion stays conservative.
Same-unit calls retain packed result carriers through structural tuple/vector/reference and
acyclic namespace-resolved struct/enum carrier paths, including instantiated generic
fields, when declared lifetimes identify the returned component. Recursive
nominal back-edges conservatively retain their subtree root while nonrecursive
sibling carriers remain precise.
A result position holds the loans passed at parameters whose declared regions outlive
its own, by a declared or implied bound; a region nothing is declared of relates to
all. A body is checked against the same rule, since rustc does not guard VerexSL.
Evaluating a mutable reference other than as the operand of a dereference or
freeze may write its referent, so it conflicts with live loans reborrowed
through it. Function call and result boundaries recursively
retain structural tuple, vector, reference, and function-pointer lifetime
constraints without guessing nominal variance. Explicit reference freezes
record that the mutable source lifetime outlives the shared result lifetime.
-/

namespace VerexIR.Validation

/-- Root of a loaned place: a checked function local, or one keyed global
resource family. -/
private inductive LoanRoot where
  | local (localId : LocalId)
  | global (resource : TypeId)
  /-- Storage whose ownership is supplied by a checked callee rather than a
  caller-local place (for example a mutable global reference result). -/
  | external
  deriving Repr, BEq, Inhabited

private def LoanRoot.local? : LoanRoot → Option LocalId
  | .local localId => some localId
  | .global _ | .external => none

private structure LoanPlace where
  root : LoanRoot
  projections : Array LoanProjection := #[]
  deriving Repr, BEq, Inhabited

/-- An incoming loan has no borrow expression in this frame and must never
produce a runtime loan-death event. Its identity still follows the same
holder and aggregate-carrier flow as loans created by the body. -/
private inductive LoanSource where
  /-- A reference a parameter holds at a path, empty for a reference parameter. -/
  | parameter (localId : LocalId) (path : Array LoanProjection := #[])
  | expression (exprId : ExprId)
  deriving Repr, BEq, Inhabited

private def LoanSource.expression? : LoanSource → Option ExprId
  | .parameter _ _ => none
  | .expression exprId => some exprId

private structure LoanFact where
  source : LoanSource
  kind : ReferenceKind
  place : LoanPlace
  lifetime : LifetimeId
  loc : LocId
  holders : Array LocalId := #[]
  holderPlaces : Array LoanPlace := #[]
  /-- The places the loan was moved out of, until written. -/
  movedFrom : Array LoanPlace := #[]
  /-- In a death: the loan goes on, these holders alone are dead. -/
  alone : Bool := false
  /-- Established when the loan is created, before local holders can die.
  Joining alternative origins requires every origin to allow escape. -/
  canEscape : Bool := false
  /-- Stored through caller-owned reference storage; a non-result boundary carrier. -/
  callerHeld : Bool := false
  deriving Repr, BEq, Inhabited

private structure BorrowState where
  active : Array LoanFact := #[]
  /-- The value of the `return` this state leaves by, and the loans it carries. -/
  returned : Option (ExprId × Array LoanFact) := none
  deriving BEq, Inhabited

/-- A loan as the state has it at a node's entry (`before`) or exit. -/
private structure LoanPoint where
  anchor : ExprId
  before : Bool
  loan : LoanFact
  deriving Inhabited

/-- The recorded deaths, and what the check of those deaths reads: the merged
state at each anchor. Combined together, so no flow keeps one without the other. -/
private structure DeathLog where
  deaths : Array (ExprId × LoanDeath) := #[]
  carrierDeaths : Array LoanDeath := #[]
  points : Array LoanPoint := #[]
  deriving Inhabited

private structure BorrowFlow where
  normal : Option BorrowState := none
  breaks : Array (Nat × BorrowState) := #[]
  breakValueLoans : Array (Nat × Array LoanFact) := #[]
  /-- Each `continue`: its loop nesting, the state it reaches, and the
  expression itself, before which loans scoped to the body die. -/
  continues : Array (Nat × BorrowState × ExprId) := #[]
  returns : Array BorrowState := #[]
  valueLoans : Array LoanFact := #[]
  observed : Array LoanFact := #[]
  /-- Death points of mutable loans, keyed by the loan's borrow expression.
  Recorded where the flow releases a loan; the semantics ends the loan
  there. -/
  deaths : DeathLog := {}
  diagnostics : Array Diagnostic := #[]
  deriving Inhabited

private def appendUniqueLocals (left right : Array LocalId) : Array LocalId :=
  right.foldl (init := left) fun locals localId =>
    if locals.contains localId then locals else locals.push localId

private def appendUniqueLoanPlaces (left right : Array LoanPlace) : Array LoanPlace :=
  right.foldl (init := left) fun places place =>
    if places.contains place then places else places.push place

private def appendUniqueLoans (left right : Array LoanFact) : Array LoanFact :=
  right.foldl (init := left) fun loans loan =>
    if loans.any (·.source == loan.source) then
      loans.map fun current =>
        if current.source == loan.source then
          -- Merge `place` conservatively: `.external` only when both sides are.
          -- `forgetMovedOut` re-roots a loan to `.external` when its carrier
          -- reference is overwritten; the other branch may still hold the
          -- concrete place, so take whichever side is concrete.
          let place := match current.place.root, loan.place.root with
            | .external, _ => loan.place
            | _, _ => current.place
          { current with
            place
            holders := appendUniqueLocals current.holders loan.holders
            holderPlaces := appendUniqueLoanPlaces current.holderPlaces loan.holderPlaces
            movedFrom := appendUniqueLoanPlaces current.movedFrom loan.movedFrom
            canEscape := current.canEscape && loan.canEscape
            callerHeld := current.callerHeld || loan.callerHeld }
        else current
    else loans.push loan

/-- Mutable loans of `before` no longer active in `after`, and of those still active
the places no longer holding them: a death states what such a place held. -/
private def diffLoans (before after : BorrowState) : Array LoanFact :=
  before.active.filterMap fun loan =>
    if loan.kind != .mutable || loan.source.expression?.isNone then none else
    match after.active.find? (·.source == loan.source) with
    | none => some loan
    | some kept =>
        -- A place this step moved the reference out of gave it on: no death.
        let moved := kept.movedFrom.filter (!loan.movedFrom.contains ·)
        let holders := loan.holders.filter fun holder =>
          !kept.holders.contains holder && !moved.any (·.root == .local holder)
        let holderPlaces := loan.holderPlaces.filter fun place =>
          !kept.holderPlaces.contains place && !moved.contains place
        if holders.isEmpty && holderPlaces.isEmpty then none
        else some { loan with
          alone := true
          holders := holders ++ (holderPlaces.filterMap (·.root.local?)).filter
            (!holders.contains ·)
          holderPlaces }

private def appendUniqueDeaths (left right : DeathLog) : DeathLog :=
  { deaths := right.deaths.foldl (init := left.deaths) fun deaths death =>
      if deaths.contains death then deaths else deaths.push death
    carrierDeaths := left.carrierDeaths ++ right.carrierDeaths
    points := left.points ++ right.points }

/-- Note the mutable loans a state holds at a node's entry or exit. -/
private def DeathLog.atPoint (log : DeathLog) (anchor : ExprId) (before : Bool)
    (state : BorrowState) : DeathLog :=
  { log with points := state.active.foldl (init := log.points) fun points loan =>
      if loan.kind == .mutable && loan.source.expression?.isSome && !loan.holders.isEmpty then
        points.push { anchor, before, loan }
      else points }

private def pushDeaths (deaths : DeathLog)
    (died : Array LoanFact) (death : LoanDeath) : DeathLog :=
  died.foldl (init := deaths) fun deaths loan =>
    match loan.source with
    | .parameter _ _ => deaths
    | .expression exprId =>
        let holderPaths := loan.holderPlaces.filterMap fun place =>
          place.root.local?.map (·, place.projections)
        appendUniqueDeaths deaths
          { deaths := #[(exprId,
              { death with holders := loan.holders, holderPaths, alone := loan.alone })] }

private def mergeState (left right : BorrowState) : BorrowState :=
  { active := appendUniqueLoans left.active right.active }

private def mergeNormal : Option BorrowState → Option BorrowState → Option BorrowState
  | none, state | state, none => state
  | some left, some right => some (mergeState left right)

private def mergeFlows (left right : BorrowFlow) : BorrowFlow := {
  normal := mergeNormal left.normal right.normal
  breaks := left.breaks ++ right.breaks
  breakValueLoans := left.breakValueLoans ++ right.breakValueLoans
  continues := left.continues ++ right.continues
  returns := left.returns ++ right.returns
  valueLoans := appendUniqueLoans left.valueLoans right.valueLoans
  observed := appendUniqueLoans left.observed right.observed
  deaths := appendUniqueDeaths left.deaths right.deaths
  diagnostics := left.diagnostics ++ right.diagnostics }

private def literalLoanIndex? (ns : ValidatedNamespace) (exprId : ExprId) : Option Nat := do
  let expression ← ns.expressions[exprId.index]?
  let .value (.integer value) _ := expression.kind | none
  if value < 0 then none else some value.toNat

private partial def loanPlace? (ns : ValidatedNamespace) (placeId : PlaceId)
    (fuel : Nat := 0) : Option LoanPlace :=
  let fuel := if fuel == 0 then ns.places.size + 1 else fuel
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none => none
  | _ + 1, some (.localVar localId) => some { root := .local localId }
  | fuel + 1, some (.deref base) => do
      let place ← loanPlace? ns base fuel
      some { place with projections := place.projections.push .dereference }
  | fuel + 1, some (.field base _ field) | fuel + 1, some (.ghostField base _ field) => do
      let place ← loanPlace? ns base fuel
      some { place with projections := place.projections.push (.field field) }
  | fuel + 1, some (.index base index) => do
      let place ← loanPlace? ns base fuel
      some { place with
        projections := place.projections.push (.index (literalLoanIndex? ns index)) }
  | fuel + 1, some (.subslice base start stop fromEnd) => do
      let place ← loanPlace? ns base fuel
      some { place with
        projections := place.projections.push (.subslice start stop fromEnd) }
  | fuel + 1, some (.unsize base) => do
      let place ← loanPlace? ns base fuel
      some { place with projections := place.projections.push (.subslice 0 0 true) }
  | fuel + 1, some (.downcast base variant) => do
      let place ← loanPlace? ns base fuel
      some { place with projections := place.projections.push (.downcast variant) }

private def projectionsOverlap : List LoanProjection → List LoanProjection → Bool
  | [], _ | _, [] => true
  | left :: leftTail, right :: rightTail =>
      match left, right with
      | .field left, .field right => left == right && projectionsOverlap leftTail rightTail
      | .downcast left, .downcast right =>
          left == right && projectionsOverlap leftTail rightTail
      -- A downcast names the variant at the same location: the steps after
      -- it are its fields.
      | .downcast _, _ => projectionsOverlap leftTail (right :: rightTail)
      | _, .downcast _ => projectionsOverlap (left :: leftTail) rightTail
      | .index (some left), .index (some right) =>
          left == right && projectionsOverlap leftTail rightTail
      | .index _, .index _ => true
      | .index (some index), .subslice start stop false |
          .subslice start stop false, .index (some index) =>
          start <= index && index < stop
      | .subslice leftStart leftStop false, .subslice rightStart rightStop false =>
          !(leftStop <= rightStart || rightStop <= leftStart)
      | .dereference, .dereference => projectionsOverlap leftTail rightTail
      | _, _ => true
termination_by left right => left.length + right.length

private def placesOverlap (left right : LoanPlace) : Bool :=
  left.root == right.root && projectionsOverlap left.projections.toList right.projections.toList

/-- Whether writing `written` destroys a value carried at `carrier`.  The
write reaches the carrier only at or above it: a longer written path
continues through the reference the carrier holds (the projection beyond a
reference-typed location is a dereference), so writing through a reference
leaves the reference's own carrier intact.  Ambiguous index/slice
combinations fall back to the symmetric overlap reading. -/
private def projectionsCover : List LoanProjection → List LoanProjection → Bool
  | [], _ => true
  | _ :: _, [] => false
  | left :: leftTail, right :: rightTail =>
      match left, right with
      | .field written, .field carrier =>
          written == carrier && projectionsCover leftTail rightTail
      | .downcast written, .downcast carrier =>
          written == carrier && projectionsCover leftTail rightTail
      | .index (some written), .index (some carrier) =>
          written == carrier && projectionsCover leftTail rightTail
      | .dereference, .dereference => projectionsCover leftTail rightTail
      | .field _, .downcast _ | .downcast _, .field _ =>
          projectionsCover leftTail rightTail
      | written, carrier => projectionsOverlap (written :: leftTail) (carrier :: rightTail)

private def placeCovers (written carrier : LoanPlace) : Bool :=
  written.root == carrier.root &&
    projectionsCover written.projections.toList carrier.projections.toList

private def borrowIsMutable (kind : ReferenceKind) : Bool := kind == .mutable

private def conflictDiagnostic (loc : LocId) (loan : LoanFact) (action : String) : Diagnostic :=
  let kindName := match loan.kind with
    | .shared => "shared"
    | .mutable => "mutable"
  { code := "VIR-SEMANTIC-BORROW-CONFLICT"
    message := s!"{action} conflicts with an active {kindName} loan"
    primary := some loc
    related := #[{ loc := loan.loc, message := "active loan originates here" }] }

/-- A written place is a new place: what moved out of it says nothing about what it
holds, and a loan reborrowed through the reference it held no longer goes through it.
The loan goes on through its holders. -/
private def forgetMovedOut (state : BorrowState) (place : LoanPlace) : BorrowState :=
  { active := state.active.map fun loan =>
      let through := placeCovers place loan.place &&
        (loan.place.projections.toList.drop place.projections.size).any
          (· matches .dereference)
      { loan with
        movedFrom := loan.movedFrom.filter (!placeCovers place ·)
        place := if through then { root := .external } else loan.place } }

private def forgetMovedOutOf (ns : ValidatedNamespace) (state : BorrowState) (placeId : PlaceId) :
    BorrowState :=
  (loanPlace? ns placeId).elim state (forgetMovedOut state)

/-- What a write of a place conflicts with: a loan reborrowed through a reference the
place holds goes on when the reference is overwritten. -/
private def shallowWriteState (ns : ValidatedNamespace) (state : BorrowState) (placeId : PlaceId) :
    BorrowState :=
  match loanPlace? ns placeId with
  | none => state
  | some written =>
      { active := state.active.filter fun loan =>
          !(placeCovers written loan.place &&
            (loan.place.projections.toList.drop written.projections.size).any
              (· matches .dereference)) }

private def accessDiagnostics (state : BorrowState) (loc : LocId) (place : LoanPlace)
    (mutable : Bool) (action : String) : Array Diagnostic :=
  state.active.foldl (init := #[]) fun diagnostics loan =>
    if placesOverlap place loan.place && (mutable || borrowIsMutable loan.kind) then
      diagnostics.push (conflictDiagnostic loc loan action)
    else diagnostics

private def referenceType? (ns : ValidatedNamespace) (typeId : TypeId) : Option ReferenceType :=
  match ns.tables.types[typeId.index]? with
  | some (.reference reference) => some reference
  | _ => none

/-- The path of a place up to its first dereference: where the reference it
goes through rests. None without a dereference. -/
private def referenceCarrier? (place : LoanPlace) : Option LoanPlace :=
  (place.projections.findIdx? (· matches .dereference)).map fun first =>
    { place with projections := place.projections.extract 0 first }

/-- The loans a holder carries at a place. -/
private def loansCarriedAt (state : BorrowState) (place : LoanPlace) : Array LoanFact :=
  state.active.filter fun loan =>
    let carriers := loan.holderPlaces.filter (·.root == place.root)
    if carriers.isEmpty then place.root.local?.any loan.holders.contains
    else carriers.any (placesOverlap · place)

/-- A reborrow can escape only when every current origin of its reference
is known to outlive this frame. Missing origins provide no such evidence. -/
private def reborrowCanEscape (state : BorrowState) (place : LoanPlace) : Bool :=
  (referenceCarrier? place).any fun carrier =>
    let parents := loansCarriedAt state carrier
    !parents.isEmpty && parents.all (·.canEscape)

/-- The code of a write to an element index under a live mutable loan; read
out of the diagnostics into the certificate by `borrowOutcome`, so it refuses
the one function rather than the unit. -/
def indexWriteUnderLoanCode : String := "VIR-SEMANTIC-INDEX-WRITE-UNDER-LOAN"

/-- Whether a place selects an element at a local's value, at any depth: the
index itself, or beneath a field, an element, a slice, a downcast or a
dereference. -/
private partial def placeIndexesLocal (ns : ValidatedNamespace) (localId : LocalId)
    (placeId : PlaceId) (fuel : Nat) : Bool :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none | _, some (.localVar _) => false
  | fuel + 1, some (.index base index) =>
      (match placeIndexForm? ns index with
        | some (.local other) | some (.copyLocal other) => other == localId
        | _ => false) || placeIndexesLocal ns localId base fuel
  | fuel + 1, some (.deref base) | fuel + 1, some (.field base _ _) |
      fuel + 1, some (.ghostField base _ _) | fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.downcast base _) | fuel + 1, some (.unsize base) =>
        placeIndexesLocal ns localId base fuel

/-- Whether a loan is a mutable borrow of a place that selects an element at
a local's value.  The loan's place keeps only literal indices, so the index
operand is read from the borrow expression the loan comes from. -/
private def loanIndexesLocal (ns : ValidatedNamespace) (loan : LoanFact) (localId : LocalId) :
    Bool :=
  loan.kind == .mutable &&
    match loan.source.expression?.bind (ns.expressions[·.index]?) with
    | some { kind := .operation (.borrow .mutable place) _ _ _, .. } =>
        placeIndexesLocal ns localId place (ns.places.size + 1)
    | _ => false

/-- A write, move, `let` or match-arm binding, or mutable borrow of a local
while a mutable loan whose place selects an element at that local is active.  The loan is resolved at
its death against the local's value there, while its reference denotes the
element chosen at the borrow, so such a write would resolve it at another
element.  Liveness is this analysis's own active set at the access, so a
loan that died before the write — the next iteration's increment of a loop
that borrows anew each iteration — is not reported. -/
private def indexWriteDiagnostics (ns : ValidatedNamespace) (state : BorrowState) (loc : LocId)
    (localId : LocalId) : Array Diagnostic :=
  state.active.foldl (init := #[]) fun diagnostics loan =>
    if loanIndexesLocal ns loan localId then
      diagnostics.push {
        code := indexWriteUnderLoanCode
        message := "a local a live mutable borrow uses as an element index is written"
        primary := some loc
        related := #[{ loc := loan.loc, message := "the mutable borrow indexing by it" }] }
    else diagnostics

private def addBorrow (ns : ValidatedNamespace) (exprId : ExprId) (loc : LocId)
    (resultType : TypeId) (kind : BorrowKind) (placeId : PlaceId)
    (state : BorrowState) (discarded : Bool := false) :
    BorrowState × Array LoanFact × Array Diagnostic :=
  match kind, loanPlace? ns placeId, referenceType? ns resultType with
  | .immutable, some place, some reference =>
      let kind := ReferenceKind.shared
      let loan : LoanFact := {
        source := .expression exprId, kind, place
        lifetime := reference.lifetime, loc, canEscape := reborrowCanEscape state place }
      let prior := { state with active := state.active.filter (·.source != .expression exprId) }
      let diagnostics := accessDiagnostics prior loc place false "borrow"
      let active := appendUniqueLoans state.active #[loan]
      ({ active }, #[loan], diagnostics)
  | .mutable, some place, some reference =>
      let kind := ReferenceKind.mutable
      let loan : LoanFact := {
        source := .expression exprId, kind, place
        lifetime := reference.lifetime, loc, canEscape := reborrowCanEscape state place }
      let prior := { state with active := state.active.filter (·.source != .expression exprId) }
      -- Move's local-handle creation does not itself read or write the
      -- referent. An unobserved direct-local result can therefore take an
      -- empty live range even when another handle exists. The let analysis
      -- below proves non-use and retires this loan before entering its body.
      -- Projected/global resolution and Rust keep their existing checks.
      let diagnostics := if discarded && ns.profile == some .move &&
          place.projections.isEmpty then #[]
        else accessDiagnostics prior loc place true "borrow"
      -- A mutable borrow of a whole local can write it, through the reference
      -- or in a callee, so it counts as a write of the indices it supplies.
      let indexDiagnostics := match place.root, place.projections.isEmpty with
        | .local localId, true => indexWriteDiagnostics ns prior loc localId
        | _, _ => #[]
      let active := appendUniqueLoans state.active #[loan]
      ({ active }, #[loan], diagnostics ++ indexDiagnostics)
  | .profile _, _, _ =>
      (state, #[], #[.at "VIR-SEMANTIC-BORROW-COVERAGE"
        "profile-specific borrow modes are outside the initial loan certificate" loc])
  | _, none, _ =>
      (state, #[], #[.at "VIR-SEMANTIC-BORROW-COVERAGE"
        "borrow place is not rooted in a checked function local" loc])
  | _, _, none => (state, #[], #[]) -- Capability typing owns this diagnostic.

/-- Track one keyed global borrow: the loan's place is the resource family,
so overlapping global borrows of one family conflict and the loan's death
reunites the global slot with its final value. -/
private def addGlobalBorrow (ns : ValidatedNamespace) (exprId : ExprId) (loc : LocId)
    (resultType : TypeId) (kind : BorrowKind)
    (instantiations : Array GenericArgument)
    (state : BorrowState) : BorrowState × Array LoanFact × Array Diagnostic :=
  match (instantiations[0]? : Option GenericArgument), referenceType? ns resultType with
  | some (.typeArg resource), some reference =>
      let kind : ReferenceKind := match kind with
        | .mutable => .mutable
        | _ => .shared
      let place : LoanPlace := { root := .global resource.typeId }
      let loan : LoanFact := {
        source := .expression exprId, kind, place, lifetime := reference.lifetime, loc
        canEscape := ns.profile != some .move }
      let prior := { state with active := state.active.filter (·.source != .expression exprId) }
      let diagnostics := accessDiagnostics prior loc place (kind == .mutable) "global borrow"
      let active := if state.active.any (·.source == .expression exprId) then state.active
        else state.active.push loan
      ({ active }, #[loan], diagnostics)
  | _, _ => (state, #[], #[])

private partial def placeIndexAccessDiagnostics (ns : ValidatedNamespace)
    (state : BorrowState) (loc : LocId) (placeId : PlaceId) (fuel : Nat) : Array Diagnostic :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none | _, some (.localVar _) => #[]
  | fuel + 1, some (.deref base) | fuel + 1, some (.field base _ _) |
      fuel + 1, some (.ghostField base _ _) | fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.unsize base) | fuel + 1, some (.downcast base _) =>
      placeIndexAccessDiagnostics ns state loc base fuel
  | fuel + 1, some (.index base index) =>
      let baseDiagnostics := placeIndexAccessDiagnostics ns state loc base fuel
      match placeIndexForm? ns index with
      | some (.local localId) | some (.copyLocal localId) =>
          baseDiagnostics ++ accessDiagnostics state loc { root := .local localId } false "place index read"
      | some (.fromEnd sourcePlace _) =>
          let sourceDiagnostics := match loanPlace? ns sourcePlace with
            | some place => accessDiagnostics state loc place false "place index read"
            | none => #[]
          baseDiagnostics ++ sourceDiagnostics ++
            placeIndexAccessDiagnostics ns state loc sourcePlace fuel
      | _ => baseDiagnostics

private def checkPlaceAccess (ns : ValidatedNamespace) (state : BorrowState) (loc : LocId)
    (placeId : PlaceId) (mutable : Bool) (action : String) : Array Diagnostic :=
  let placeDiagnostics := match loanPlace? ns placeId with
    | some place => accessDiagnostics state loc place mutable action
    | none => #[]
  -- A write or a move of a local is also a write of every element index it
  -- supplies to a live mutable loan's place.
  let indexDiagnostics := match mutable, ns.places[placeId.index]? with
    | true, some (.localVar localId) => indexWriteDiagnostics ns state loc localId
    | _, _ => #[]
  placeDiagnostics ++ indexDiagnostics ++
    placeIndexAccessDiagnostics ns state loc placeId (ns.places.size + 1)

private partial def placeContainsDereference (ns : ValidatedNamespace) (placeId : PlaceId)
    (fuel : Nat) : Bool :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none | _, some (.localVar _) => false
  | _, some (.deref _) => true
  | fuel + 1, some (.field base _ _) | fuel + 1, some (.ghostField base _ _) |
      fuel + 1, some (.index base _) | fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.unsize base) | fuel + 1, some (.downcast base _) =>
      placeContainsDereference ns base fuel

private def dereferenceConsumptionDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (placeId : PlaceId) : Array Diagnostic :=
  if placeContainsDereference ns placeId (ns.places.size + 1) then
    #[.at "VIR-SEMANTIC-BORROW-DEREF-CONSUME"
      "moving or dropping through a reference is not supported by the safe reference model" loc]
  else #[]

private partial def placeUsesLocal (ns : ValidatedNamespace) (placeId : PlaceId)
    (localId : LocalId) (fuel : Nat) : Bool :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none => true
  | _ + 1, some (.localVar candidate) => candidate == localId
  | fuel + 1, some (.deref base) | fuel + 1, some (.field base _ _) |
      fuel + 1, some (.ghostField base _ _) | fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.unsize base) | fuel + 1, some (.downcast base _) => placeUsesLocal ns base localId fuel
  | fuel + 1, some (.index base index) =>
      placeUsesLocal ns base localId fuel || match ns.expressions[index.index]? with
        | some { kind := .localVar candidate, .. } => candidate == localId
        | some { kind := .value .., .. } => false
        -- Executable places currently admit only literal and direct-local
        -- indexes; retain every holder for any future richer index spelling.
        | _ => true

private partial def expressionUsesLocal (ns : ValidatedNamespace) (exprId : ExprId)
    (localId : LocalId) (fuel : Nat) : Bool :=
  match fuel, ns.expressions[exprId.index]? with
  | 0, _ | _, none => true
  | _ + 1, some { kind := .localVar candidate, .. } => candidate == localId
  | fuel + 1, some { kind := .operation operation _ arguments _, .. } =>
      let argumentsUse := arguments.any fun argument =>
        expressionUsesLocal ns argument localId fuel
      let placeUse := match operation with
        | .move place | .copy place | .borrow _ place | .read place | .readDiscriminant place |
            .write place | .drop place =>
            placeUsesLocal ns place localId (ns.places.size + 1)
        | _ => false
      argumentsUse || placeUse
  | fuel + 1, some { kind := .block statements result, .. } =>
      statements.any (expressionUsesLocal ns · localId fuel) ||
        result.any (expressionUsesLocal ns · localId fuel)
  | fuel + 1, some { kind := .letDecl _ initializer body, .. } =>
      initializer.any (expressionUsesLocal ns · localId fuel) ||
        expressionUsesLocal ns body localId fuel
  | fuel + 1, some { kind := .ifElse condition thenBranch elseBranch, .. } =>
      expressionUsesLocal ns condition localId fuel ||
        expressionUsesLocal ns thenBranch localId fuel ||
        elseBranch.any (expressionUsesLocal ns · localId fuel)
  | fuel + 1, some { kind := .match_ scrutinee arms, .. } =>
      expressionUsesLocal ns scrutinee localId fuel || arms.any fun arm =>
        arm.guard.any (expressionUsesLocal ns · localId fuel) ||
          expressionUsesLocal ns arm.body localId fuel
  | fuel + 1, some { kind := .loop _ body, .. } =>
      expressionUsesLocal ns body localId fuel
  | fuel + 1, some { kind := .break_ _ value, .. } =>
      value.any (expressionUsesLocal ns · localId fuel)
  | fuel + 1, some { kind := .return_ values, .. } |
      fuel + 1, some { kind := .throw_ _ values, .. } =>
      values.any (expressionUsesLocal ns · localId fuel)
  | fuel + 1, some { kind := .assign place value, .. } =>
      placeUsesLocal ns place localId (ns.places.size + 1) ||
        expressionUsesLocal ns value localId fuel
  | fuel + 1, some { kind := .assignPattern _ value, .. } =>
      expressionUsesLocal ns value localId fuel
  | fuel + 1, some { kind := .spec block, .. } =>
      (expressionChildren (.spec block)).any (expressionUsesLocal ns · localId fuel)
  -- A quantifier uses what its domains, triggers, condition, and body use.
  | fuel + 1, some { kind := kind@(.quantifier ..), .. } =>
      (expressionChildren kind).any (expressionUsesLocal ns · localId fuel)
  | _, some { kind := .value .., .. } | _, some { kind := .constant _, .. } |
      _, some { kind := .continue_ _, .. } => false

private def placeUsesCarrier (ns : ValidatedNamespace) (placeId : PlaceId)
    (carrier : LoanPlace) : Bool :=
  match loanPlace? ns placeId with
  | some accessed => placesOverlap carrier accessed ||
      (accessed.root != carrier.root &&
        (carrier.root.local?.any fun localId =>
          placeUsesLocal ns placeId localId (ns.places.size + 1)))
  | none => carrier.root.local?.any fun localId =>
      placeUsesLocal ns placeId localId (ns.places.size + 1)

private partial def expressionUsesCarrier (ns : ValidatedNamespace) (exprId : ExprId)
    (carrier : LoanPlace) (fuel : Nat) : Bool :=
  match fuel, ns.expressions[exprId.index]? with
  | 0, _ | _, none => true
  | _ + 1, some { kind := .localVar candidate, .. } => carrier.root == .local candidate
  | fuel + 1, some { kind := .operation operation _ arguments _, .. } =>
      arguments.any (expressionUsesCarrier ns · carrier fuel) || match operation with
        | .move place | .copy place | .borrow _ place | .read place | .readDiscriminant place |
            .write place | .drop place => placeUsesCarrier ns place carrier
        | _ => false
  | fuel + 1, some { kind := .block statements result, .. } =>
      statements.any (expressionUsesCarrier ns · carrier fuel) ||
        result.any (expressionUsesCarrier ns · carrier fuel)
  | fuel + 1, some { kind := .letDecl _ initializer body, .. } =>
      initializer.any (expressionUsesCarrier ns · carrier fuel) ||
        expressionUsesCarrier ns body carrier fuel
  | fuel + 1, some { kind := .ifElse condition thenBranch elseBranch, .. } =>
      expressionUsesCarrier ns condition carrier fuel ||
        expressionUsesCarrier ns thenBranch carrier fuel ||
        elseBranch.any (expressionUsesCarrier ns · carrier fuel)
  | fuel + 1, some { kind := .match_ scrutinee arms, .. } =>
      expressionUsesCarrier ns scrutinee carrier fuel || arms.any fun arm =>
        arm.guard.any (expressionUsesCarrier ns · carrier fuel) ||
          expressionUsesCarrier ns arm.body carrier fuel
  | fuel + 1, some { kind := .loop _ body, .. } =>
      expressionUsesCarrier ns body carrier fuel
  | fuel + 1, some { kind := .break_ _ value, .. } =>
      value.any (expressionUsesCarrier ns · carrier fuel)
  | fuel + 1, some { kind := .return_ values, .. } |
      fuel + 1, some { kind := .throw_ _ values, .. } =>
      values.any (expressionUsesCarrier ns · carrier fuel)
  | fuel + 1, some { kind := .assign place value, .. } =>
      placeUsesCarrier ns place carrier || expressionUsesCarrier ns value carrier fuel
  | fuel + 1, some { kind := .assignPattern _ value, .. } =>
      expressionUsesCarrier ns value carrier fuel
  | fuel + 1, some { kind := .spec block, .. } =>
      (expressionChildren (.spec block)).any (expressionUsesCarrier ns · carrier fuel)
  | fuel + 1, some { kind := kind@(.quantifier ..), .. } =>
      (expressionChildren kind).any (expressionUsesCarrier ns · carrier fuel)
  | _, some { kind := .value .., .. } | _, some { kind := .constant _, .. } |
      _, some { kind := .continue_ _, .. } => false

private partial def patternVariables (ns : ValidatedNamespace) (patternId : PatternId)
    (fuel : Nat := 0) : Array LocalId :=
  let fuel := if fuel == 0 then ns.patterns.size + 1 else fuel
  match fuel, ns.patterns[patternId.index]? with
  | 0, _ | _, none => #[]
  | _ + 1, some { kind := .variable localId, .. } => #[localId]
  | fuel + 1, some { kind := .tuple elements, .. } |
      fuel + 1, some { kind := .constructor _ _ _ elements, .. } =>
      elements.foldl (fun locals element =>
        appendUniqueLocals locals (patternVariables ns element fuel)) #[]
  | _, some { kind := .wildcard, .. } | _, some { kind := .literal _, .. } |
      _, some { kind := .range .., .. } => #[]

/-- Whether evaluating a node may leave it by a `break` or `continue`. -/
private partial def mayJumpOut (ns : ValidatedNamespace) (exprId : ExprId) (depth : Nat := 0) : Bool :=
  match ns.expressions[exprId.index]? with
  | none => false
  | some expression => match expression.kind with
    | .continue_ nest => nest >= depth
    | .break_ nest value => nest >= depth || value.any (mayJumpOut ns · depth)
    | .loop _ body => mayJumpOut ns body (depth + 1)
    | .spec _ | .quantifier .. => false
    | kind => (expressionChildren kind).any (mayJumpOut ns · depth)

/-- Whether an expression, evaluated in order, reads a holder (`some true`)
or first overwrites it whole (`some false`): an assignment covering the carrier
or a binding of the local, after which what follows reads the new value. A
conditional decides only where its branches agree or one reads; a loop's body may
not run, so only its reading decides. -/
private partial def holderFate (ns : ValidatedNamespace) (holder : LocalId)
    (overwrites placeUses : PlaceId → Bool) (uses : ExprId → Bool) (exprId : ExprId) : Option Bool :=
  let fate := holderFate ns holder overwrites placeUses uses
  -- An operand evaluated first may jump past an overwrite.
  let after (operands : List ExprId) (result : Option Bool) : Option Bool :=
    if result == some false && operands.any (mayJumpOut ns ·) then none else result
  let choice (fates : List (Option Bool)) : Option Bool :=
    if fates.any (· == some true) then some true
    else if fates.all (· == some false) then some false
    else none
  match ns.expressions[exprId.index]? with
  | some { kind := .assign place value, .. } =>
      match fate value with
      | some decided => some decided
      | none =>
          if overwrites place then after [value] (some false)
          else if placeUses place then some true else none
  | some { kind := .assignPattern pattern value, .. } =>
      if uses value then some true
      else if (patternVariables ns pattern).contains holder then after [value] (some false)
      else none
  | some { kind := .letDecl pattern initializer body, .. } =>
      match initializer.bind fate with
      | some decided => some decided
      | none =>
          if (patternVariables ns pattern).contains holder then
            after initializer.toList (some false)
          else after initializer.toList (fate body)
  | some { kind := .block statements result, .. } =>
      holderFate.sequenceFate ns holder overwrites placeUses uses (statements.toList ++ result.toList)
  | some { kind := .ifElse condition thenBranch elseBranch, .. } =>
      match fate condition with
      | some decided => some decided
      | none =>
        match elseBranch with
        | some branch => after [condition] (choice [fate thenBranch, fate branch])
        | none => if fate thenBranch == some true then some true else none
  | some { kind := .match_ scrutinee arms, .. } =>
      match fate scrutinee with
      | some decided => some decided
      | none =>
      if arms.any (·.guard.any uses) then some true
      else if arms.isEmpty then none
      else after (scrutinee :: arms.toList.flatMap (·.guard.toList))
        (choice (arms.toList.map (fate ·.body)))
  | some { kind := .loop _ body, .. } =>
      if fate body == some true then some true else none
  -- Operands are looked into: one may bind the holder afresh before reading it.
  | some { kind := .operation operation _ arguments _, .. } =>
      match holderFate.sequenceFate ns holder overwrites placeUses uses arguments.toList with
      | some decided => some decided
      | none => match operation with
        | .write place =>
            if overwrites place then after arguments.toList (some false)
            else if placeUses place then some true else none
        | .move place | .copy place | .borrow _ place | .read place | .readDiscriminant place | .drop place =>
            if placeUses place then some true else none
        | _ => none
  | _ => if uses exprId then some true else none
where
  /-- Past an element that may jump, an overwrite no longer decides: the jump
  skips it, so what follows is still scanned for reads. -/
  sequenceFate (ns : ValidatedNamespace) (holder : LocalId) (overwrites placeUses : PlaceId → Bool)
      (uses : ExprId → Bool) (expressions : List ExprId) (jumped : Bool := false) : Option Bool :=
    match expressions with
    | [] => none
    | expression :: rest =>
        match holderFate ns holder overwrites placeUses uses expression with
        | some true => some true
        | some false =>
            if jumped then sequenceFate ns holder overwrites placeUses uses rest true else some false
        | none =>
            sequenceFate ns holder overwrites placeUses uses rest (jumped || mayJumpOut ns expression)

/-- The fate of a carrier over a sequence of expressions. -/
private def carrierFate (ns : ValidatedNamespace) (carrier : LoanPlace) (holder : LocalId)
    (expressions : List ExprId) : Option Bool :=
  holderFate.sequenceFate ns holder
    (fun place => (loanPlace? ns place).any (placeCovers · carrier))
    (placeUsesCarrier ns · carrier)
    (fun expression => expressionUsesCarrier ns expression carrier (ns.expressions.size + 1))
    expressions

/-- The fate of a local over a sequence of expressions. -/
private def localFate (ns : ValidatedNamespace) (holder : LocalId)
    (expressions : List ExprId) : Option Bool :=
  holderFate.sequenceFate ns holder
    (fun place => ns.places[place.index]? == some (.localVar holder))
    (placeUsesLocal ns · holder (ns.places.size + 1))
    (fun expression => expressionUsesLocal ns expression holder (ns.expressions.size + 1))
    expressions

/-- Whether expressions read a carrier before overwriting it. -/
private def carrierUsedBy (ns : ValidatedNamespace) (carrier : LoanPlace) (holder : LocalId)
    (expressions : List ExprId) : Bool :=
  (carrierFate ns carrier holder expressions).getD false

/-- Whether expressions read a local before overwriting it. -/
private def localUsedBy (ns : ValidatedNamespace) (holder : LocalId)
    (expressions : List ExprId) : Bool :=
  (localFate ns holder expressions).getD false

private def loanUsedBy (ns : ValidatedNamespace) (expressions : List ExprId)
    (loan : LoanFact) : Bool :=
  loan.holders.any fun holder =>
    let carriers := loan.holderPlaces.filter (·.root == .local holder)
    if carriers.isEmpty then localUsedBy ns holder expressions
    else carriers.any fun carrier => carrierUsedBy ns carrier holder expressions

private def loansUsedBy (ns : ValidatedNamespace) (expressions : List ExprId)
    (state : BorrowState) (initial : Array LoanSource := #[]) : Array LoanSource :=
  state.active.foldl (init := initial) fun preserved loan =>
    if preserved.contains loan.source || !loanUsedBy ns expressions loan then preserved
    else preserved.push loan.source

private def loanUsedByPlace (ns : ValidatedNamespace) (placeId : PlaceId)
    (loan : LoanFact) : Bool :=
  loan.holders.any fun holder =>
    let carriers := loan.holderPlaces.filter (·.root == .local holder)
    if carriers.isEmpty then placeUsesLocal ns placeId holder (ns.places.size + 1)
    else carriers.any (placeUsesCarrier ns placeId)

private def loansUsedByPlace (ns : ValidatedNamespace) (placeId : PlaceId)
    (state : BorrowState) (initial : Array LoanSource := #[]) : Array LoanSource :=
  state.active.foldl (init := initial) fun preserved loan =>
    if preserved.contains loan.source || !loanUsedByPlace ns placeId loan then preserved
    else preserved.push loan.source

private def operationPlace? : Operation → Option PlaceId
  | .move place | .copy place | .borrow _ place | .read place | .readDiscriminant place |
      .write place | .drop place => some place
  | _ => none

/-- Whether a loan is a mutable one the caller lends. Outside Move it stays
active while a place holds it, so a write that destroys it is seen. -/
private def isLentMutable (ns : ValidatedNamespace) (loan : LoanFact) : Bool :=
  ns.profile != some .move && loan.kind == .mutable && loan.source.expression?.isNone

/-- Whether a tree writes or binds a local, not through a reference it holds. -/
private partial def writesLocal (ns : ValidatedNamespace) (holder : LocalId) (exprId : ExprId) :
    Bool :=
  let writes := fun (place : PlaceId) =>
    (loanPlace? ns place).any fun written =>
      written.root == .local holder && !written.projections.any (· matches .dereference)
  match ns.expressions[exprId.index]? with
  | none => false
  | some expression =>
      (match expression.kind with
        | .assign place _ | .operation (.write place) _ _ _ => writes place
        | .letDecl pattern _ _ | .assignPattern pattern _ =>
            (patternVariables ns pattern).contains holder
        | _ => false) ||
      (expressionChildren expression.kind).any (writesLocal ns holder)

private def retainLoansUsedBy (ns : ValidatedNamespace) (expressions : List ExprId)
    (state : BorrowState) (preserved : Array LoanSource := #[]) : BorrowState :=
  let retained := state.active.filter fun loan =>
    loan.callerHeld || loan.holders.isEmpty || preserved.contains loan.source ||
      loanUsedBy ns expressions loan || isLentMutable ns loan
  -- A loan going on through another holder: a place nothing reads again and something
  -- writes again gives its reference up, at its death. One never written again rests
  -- until the loan dies.
  { active := retained.map fun loan =>
      if loan.kind != .mutable || loan.source.expression?.isNone ||
          preserved.contains loan.source then loan else
      let gives := fun (holder : LocalId) =>
        !loanUsedBy ns expressions { loan with holders := #[holder] } &&
          expressions.any (writesLocal ns holder)
      let holders := loan.holders.filter (!gives ·)
      if holders.isEmpty then loan else
      { loan with
        holders
        holderPlaces := loan.holderPlaces.filter fun place =>
          place.root.local?.all holders.contains } }

private def isOwnedCarrierPlace (place : LoanPlace) : Bool :=
  !place.projections.any (· matches .dereference)

/-- The loans of this frame a write of a place destroys a carrier of. -/
private def overwrittenLoans (ns : ValidatedNamespace) (state : BorrowState) (placeId : PlaceId) :
    Array LoanFact :=
  match loanPlace? ns placeId with
  | none => #[]
  | some place =>
      if !isOwnedCarrierPlace place then #[] else
      state.active.filter fun loan =>
        loan.kind == .mutable && loan.source.expression?.isSome &&
          let carriers := loan.holderPlaces.filter (·.root == place.root)
          if carriers.isEmpty then
            place.projections.isEmpty && place.root.local?.any loan.holders.contains
          else carriers.any (placeCovers place ·)

/-- A write ends the loans it destroys a carrier of that nothing else uses: where
it still holds them, since its death would read what the place holds next. Move
holds no reference in a field, and a block already ends a local's loans before
the write overwriting it. -/
private def retireOverwritten (ns : ValidatedNamespace) (state : BorrowState)
    (overwritten : Array LoanFact) (expressions : List ExprId) : BorrowState :=
  if ns.profile == some .move then state else
  { active := state.active.filter fun loan =>
      loan.callerHeld || !overwritten.any (·.source == loan.source) || loanUsedBy ns expressions loan }

/-- A write destroying a carrier of a loan that lives on through another holder:
the loan's death would resolve what the place holds then. -/
private def overwrittenLiveDiagnostics (state : BorrowState) (overwritten carried : Array LoanFact)
    (loc : LocId) : Array Diagnostic :=
  if overwritten.any fun loan =>
      !carried.any (·.source == loan.source) && state.active.any (·.source == loan.source) then
    #[.at "VIR-SEMANTIC-BORROW-OVERWRITE"
      "a mutable reference that lives on through another holder cannot be overwritten here" loc]
  else #[]

/-- A write destroying a mutable reference the caller lends: its loan has no
death in this frame, and the slot could not export it at the exit. -/
private def overwrittenLoanDiagnostics (ns : ValidatedNamespace) (before after : BorrowState)
    (loc : LocId) : Array Diagnostic :=
  if before.active.any fun loan =>
      isLentMutable ns loan && !after.active.any (·.source == loan.source) then
    #[.at "VIR-SEMANTIC-BORROW-OVERWRITE"
      "a mutable reference the function's caller lends cannot be overwritten" loc]
  else #[]

private def bindObservedLoans (state : BorrowState) (observed : Array LoanFact)
    (holder : LocalId) : BorrowState :=
  let holderPlace : LoanPlace := { root := .local holder }
  { active := state.active.map fun loan =>
      if observed.any (·.source == loan.source) then
        { loan with
          holders := if loan.holders.contains holder then loan.holders else loan.holders.push holder
          holderPlaces := if loan.holderPlaces.contains holderPlace then loan.holderPlaces
            else loan.holderPlaces.push holderPlace }
      else loan }

private def releaseHolderLoans (state : BorrowState) (carried : Array LoanFact)
    (holder : LocalId) : BorrowState :=
  { active := state.active.filterMap fun loan =>
      if !loan.holders.contains holder then some loan else
        let holders := loan.holders.filter (· != holder)
        let holderPlaces := loan.holderPlaces.filter (·.root != .local holder)
        if holders.isEmpty && !loan.callerHeld && !carried.any (·.source == loan.source) then none
        else some { loan with holders, holderPlaces } }

private def releasePlaceLoans (ns : ValidatedNamespace) (state : BorrowState)
    (carried : Array LoanFact) (placeId : PlaceId) : BorrowState :=
  match loanPlace? ns placeId with
  | none => state
  | some place =>
    -- A loan the consumed value carries on was moved out of the place.
    let appendMoved (loan : LoanFact) : Array LoanPlace :=
      if carried.any (·.source == loan.source) then
        appendUniqueLoanPlaces loan.movedFrom #[place]
      else loan.movedFrom
    let active := state.active.filterMap fun loan =>
      let carriers := loan.holderPlaces.filter (·.root == place.root)
      if carriers.isEmpty then
        match place.root.local? with
        | some rootLocal =>
            if place.projections.isEmpty && loan.holders.contains rootLocal then
              let holders := loan.holders.filter (· != rootLocal)
              if holders.isEmpty && !loan.callerHeld && !carried.any (·.source == loan.source) then none
              else some ({ loan with holders, movedFrom := appendMoved loan } : LoanFact)
            else some loan
        | none => some loan
      else
        let holderPlaces := loan.holderPlaces.filter fun carrier =>
          carrier.root != place.root || !placeCovers place carrier
        if holderPlaces.size == loan.holderPlaces.size then some loan else
          let holders := if holderPlaces.any (·.root == place.root) then loan.holders
            else match place.root.local? with
              | some rootLocal => loan.holders.filter (· != rootLocal)
              | none => loan.holders
          if holders.isEmpty && !loan.callerHeld && !carried.any (·.source == loan.source) then none
          else some ({ loan with holders, holderPlaces, movedFrom := appendMoved loan } : LoanFact)
    { active }

private def releasePlaceLoansUnusedBy (ns : ValidatedNamespace) (state : BorrowState)
    (placeId : PlaceId) (expressions : List ExprId) : BorrowState :=
  match loanPlace? ns placeId with
  | none => state
  | some place =>
    if !isOwnedCarrierPlace place then state else
    let active := state.active.filterMap fun loan =>
      let carriers := loan.holderPlaces.filter (·.root == place.root)
      if carriers.isEmpty then
        match place.root.local? with
        | some rootLocal =>
            if place.projections.isEmpty && loan.holders.contains rootLocal &&
                !expressions.any (expressionUsesLocal ns · rootLocal (ns.expressions.size + 1)) then
              let holders := loan.holders.filter (· != rootLocal)
              if holders.isEmpty && !loan.callerHeld then none else some ({ loan with holders } : LoanFact)
            else some loan
        | none => some loan
      else
        let holderPlaces := loan.holderPlaces.filter fun carrier =>
          carrier.root != place.root || !placeCovers place carrier ||
            expressions.any (expressionUsesCarrier ns · carrier (ns.expressions.size + 1))
        if holderPlaces.size == loan.holderPlaces.size then some loan else
          let holders := if holderPlaces.any (·.root == place.root) then loan.holders
            else match place.root.local? with
              | some rootLocal => loan.holders.filter (· != rootLocal)
              | none => loan.holders
          if holders.isEmpty && !loan.callerHeld then none
          else some ({ loan with holders, holderPlaces } : LoanFact)
    { active }

private def replaceHolderLoans (state : BorrowState) (observed : Array LoanFact)
    (holder : LocalId) : BorrowState :=
  bindObservedLoans (releaseHolderLoans state observed holder) observed holder

private def recordBoundLoans (state : BorrowState) (observed : Array LoanFact) : Array LoanFact :=
  appendUniqueLoans observed <| state.active.filter fun loan =>
    observed.any (·.source == loan.source)

private def loansHeldAtPlace (ns : ValidatedNamespace) (state : BorrowState)
    (placeId : PlaceId) : Array LoanFact :=
  match loanPlace? ns placeId with
  | some place => loansCarriedAt state place
  | none => #[]

/-- A carrier path the checked post-RHS retirement can read exactly. -/
def retirableCarrierPlace (ns : ValidatedNamespace) : Nat → PlaceId → Bool
  | 0, _ => false
  | fuel + 1, place => match ns.places[place.index]? with
    | some (.localVar _) => true
    | some (.deref base) | some (.field base ..) | some (.downcast base _) =>
        retirableCarrierPlace ns fuel base
    | some (.index base index) =>
        match ns.expressions[index.index]? with
        | some { kind := .value (.integer value) _, .. } =>
            value >= 0 && retirableCarrierPlace ns fuel base
        | _ => false
    | _ => false

/-- A direct mutable-reference destination can retire its old carrier after
its RHS has run. Moving the old reference out makes the runtime retirement a
no-op. Imprecise indexes and whole aggregates remain unsupported. -/
private def carrierRetirement? (ns : ValidatedNamespace) (placeId : PlaceId)
    (value : ExprId) (before : BorrowState) (carried : Array LoanFact) : Option LoanDeath := do
  if ns.profile == some .move || !retirableCarrierPlace ns (ns.places.size + 1) placeId then none else do
  let expression ← ns.expressions[value.index]?
  let reference ← referenceType? ns expression.typeId
  if reference.kind != .mutable then none else do
  let place ← loanPlace? ns placeId
  let holder ← place.root.local?
  let oldLoans := loansHeldAtPlace ns before placeId
  -- A plain local's known lexical loan already has the ordinary checked death.
  -- Incoming loans and nested carriers still need the explicit path retirement.
  if place.projections.isEmpty && !oldLoans.isEmpty &&
      oldLoans.all (fun loan => loan.source.expression?.isSome) then none else do
  -- The RHS carries the same loan onward; do not settle that identity.
  if oldLoans.any (fun old =>
      carried.any (·.source == old.source)) then none else
  some { anchor := value, holders := #[holder], holderPaths := #[(holder, place.projections)], alone := true }

private def addCarrierRetirement (log : DeathLog) (death : Option LoanDeath) : DeathLog :=
  match death with
  | none => log
  | some death => { log with carrierDeaths := log.carrierDeaths.push death }

/-- A replacement stored through caller-owned reference storage escapes even
when the function returns unit. Local lenders cannot be stored there. -/
private def replacementEscapeDiagnostics (ns : ValidatedNamespace) (state : BorrowState)
    (placeId : PlaceId) (carried : Array LoanFact) (loc : LocId) : Array Diagnostic :=
  if ns.profile == some .move then #[] else
  match loanPlace? ns placeId >>= referenceCarrier? with
  | none => #[]
  | some carrier =>
      let parents := loansCarriedAt state carrier
      if (parents.isEmpty || parents.any (·.canEscape)) && carried.any (!·.canEscape) then
        #[.at "VIR-SEMANTIC-BORROW-ESCAPE"
          "a reference to frame-owned storage cannot be written through a caller-owned reference" loc]
      else #[]

/-- The locals an expression declares, by `let` or a match arm. -/
private partial def bodyDeclaredLocals (ns : ValidatedNamespace) (root : ExprId)
    (found : Array LocalId := #[]) : Array LocalId :=
  match ns.expressions[root.index]? with
  | none => found
  | some expression =>
      let found := match expression.kind with
        | ExprKind.letDecl pattern _ _ => appendUniqueLocals found (patternVariables ns pattern)
        | ExprKind.match_ _ arms =>
            arms.foldl (fun found arm => appendUniqueLocals found (patternVariables ns arm.pattern))
              found
        | _ => found
      (expressionChildren expression.kind).foldl
        (fun found child => bodyDeclaredLocals ns child found) found

private def directLocalPlace? (ns : ValidatedNamespace) (placeId : PlaceId) : Option LocalId :=
  match ns.places[placeId.index]? with
  | some (.localVar localId) => some localId
  | _ => none

private def namespaceAt? (unit : ValidatedUnit) (namespaceId : NamespaceId) :
    Option ValidatedNamespace :=
  unit.namespaces[namespaceId.index]?

private def functionAt? (unit : ValidatedUnit) (reference : QualifiedRef) :
    Option (ValidatedNamespace × FunctionDecl FunctionBody) := do
  let ns ← namespaceAt? unit reference.namespaceId
  let declaration ← ns.functions.find? (·.name == reference.name)
  some (ns, declaration)

private def nominalAt? (unit : ValidatedUnit) (name : NameId) :
    Option (ValidatedNamespace × StructDecl) := do
  let qualified ← unit.tables.names[name.index]?
  let ns ← namespaceAt? unit qualified.namespaceId
  let declaration ← ns.structs.find? (·.name == name)
  some (ns, declaration)

private partial def typeMayContainReference (ns : ValidatedNamespace) (typeId : TypeId)
    (fuel : Nat := 0) : Bool :=
  let fuel := if fuel == 0 then ns.tables.types.size + 1 else fuel
  match fuel, ns.tables.types[typeId.index]? with
  | 0, _ | _, none => true
  | _, some (.reference _) => true
  | fuel + 1, some (.tuple elements) =>
      elements.any fun element => typeMayContainReference ns element fuel
  | fuel + 1, some (.vector element _) | fuel + 1, some (.slice element) => typeMayContainReference ns element fuel
  | fuel + 1, some (.function arguments result _) =>
      arguments.any (typeMayContainReference ns · fuel) ||
        typeMayContainReference ns result fuel
  -- Move structs and enums have no reference fields, and Move type arguments
  -- hold no reference: validation rejects a Move declaration with a reference
  -- field (`moveReferenceFieldDiagnostics` in `Validation/Check.lean`) and a
  -- Move type argument that holds one, explicit or inferred
  -- (`moveTypeArgumentDiagnostics` in `Validation/Capability.lean`). Other
  -- profiles may carry references in both.
  | _, some (.nominal ..) | _, some (.typeParameter _) => ns.profile != some .move
  | _, some (.profile _) => true
  | _, some (.typeDomain type) => typeMayContainReference ns type (fuel - 1)
  | _, some (.resourceDomain _ (some arguments)) =>
      arguments.any (typeMayContainReference ns · (fuel - 1))
  | _, some (.unit) | _, some (.never) | _, some (.bool) | _, some (.character) |
      _, some (.string) | _, some (.bytes) | _, some (.address) |
      _, some (.signer) | _, some (.integer ..) |
      _, some (.range) | _, some (.eventStore) | _, some (.resourceDomain _ none) |
      _, some (.stateDomain) => false

private def instantiatedLifetime (ns : ValidatedNamespace)
    (instantiations : Array GenericArgument) (lifetime : LifetimeId) : LifetimeId :=
  match ns.tables.lifetimes[lifetime.index]? with
  | some { kind := .parameter index, .. } => match instantiations[index]? with
      | some (.lifetime value) => value
      | _ => lifetime
  | _ => lifetime

private partial def expressionContains (ns : ValidatedNamespace) (root target : ExprId)
    (fuel : Nat) : Bool :=
  if root == target then true else match fuel, ns.expressions[root.index]? with
    | 0, _ | _, none => false
    | fuel + 1, some expression =>
        let childrenContain (children : Array ExprId) :=
          children.any (expressionContains ns · target fuel)
        match expression.kind with
        | .operation _ _ arguments _ => childrenContain arguments
        | .block statements result => childrenContain (statements ++ result.toArray)
        | .letDecl _ initializer body => childrenContain (initializer.toArray.push body)
        | .ifElse condition thenBranch elseBranch =>
            childrenContain (#[condition, thenBranch] ++ elseBranch.toArray)
        | .match_ scrutinee arms => expressionContains ns scrutinee target fuel ||
            arms.any fun arm => childrenContain (arm.guard.toArray.push arm.body)
        | .loop _ body => expressionContains ns body target fuel
        | .break_ _ value => childrenContain value.toArray
        | .return_ values | .throw_ _ values => childrenContain values
        | .assign _ value | .assignPattern _ value =>
            expressionContains ns value target fuel
        | .quantifier _ binders triggers condition body =>
            let domains := binders.map (·.domain)
            let triggers := triggers.foldl (· ++ ·) #[]
            childrenContain (domains ++ triggers ++ condition.toArray.push body)
        | .value .. | .constant _ | .localVar _ | .continue_ _ | .spec _ => false

/-- How an expression leaves its continuation on every path: it leaves the
function (a return or a throw), or it jumps (a break or a continue), which a
loop's continuation follows. A loop's body is not looked into. -/
private inductive Exit where
  | leaves
  | jumps
  deriving BEq

private def exitOf (ns : ValidatedNamespace) : Nat → ExprId → Option Exit
  | 0, _ => none
  | fuel + 1, id =>
    let both (left right : Option Exit) : Option Exit := match left, right with
      | some .leaves, some .leaves => some .leaves
      | some _, some _ => some .jumps
      | _, _ => none
    match ns.expressions[id.index]? with
    | some { kind := .return_ _, .. } | some { kind := .throw_ .., .. } => some .leaves
    | some { kind := .break_ .., .. } | some { kind := .continue_ _, .. } => some .jumps
    | some { kind := .block statements result, .. } =>
        statements.findSome? (exitOf ns fuel) <|> result.bind (exitOf ns fuel)
    | some { kind := .letDecl _ value body, .. } =>
        value.bind (exitOf ns fuel) <|> exitOf ns fuel body
    | some { kind := .ifElse condition thenBranch elseBranch, .. } =>
        exitOf ns fuel condition <|>
          elseBranch.bind fun branch => both (exitOf ns fuel thenBranch) (exitOf ns fuel branch)
    | some { kind := .match_ scrutinee arms, .. } =>
        exitOf ns fuel scrutinee <|>
          if arms.isEmpty then none
          else arms.foldl (fun exit arm => both exit (exitOf ns fuel arm.body))
            (exitOf ns fuel arms[0]!.body)
    | some { kind := .operation _ _ arguments _, .. } => arguments.findSome? (exitOf ns fuel)
    | _ => none

/-- The loans a branch preserves for what follows it: none where it leaves
the function, which no continuation follows. -/
private def branchPreserved (ns : ValidatedNamespace) (branch : ExprId)
    (preserved : Array LoanSource) : Array LoanSource :=
  if exitOf ns (ns.expressions.size + 1) branch == some .leaves then #[] else preserved

/-- What evaluation reaches after a branch: nothing where it leaves the function. -/
private def branchFuture (ns : ValidatedNamespace) (branch : ExprId) (future : List ExprId) :
    List ExprId :=
  if exitOf ns (ns.expressions.size + 1) branch == some .leaves then [] else future

private def loanFlowsFromExpression (ns : ValidatedNamespace) (loan : LoanFact)
    (expression : ExprId) : Bool :=
  loan.source.expression?.any (fun source =>
    expressionContains ns expression source (ns.expressions.size + 1)) ||
    loan.holders.any fun holder =>
      expressionUsesLocal ns expression holder (ns.expressions.size + 1)

private def appendUniqueProjectionPaths (left right : Array (Array LoanProjection)) :
    Array (Array LoanProjection) :=
  right.foldl (init := left) fun paths path =>
    if paths.contains path then paths else paths.push path

private def stripProjectionPrefix? : List LoanProjection → List LoanProjection →
    Option (List LoanProjection)
  | [], projections => some projections
  | expected :: prefixes, projection :: projections =>
      if expected == projection then stripProjectionPrefix? prefixes projections else none
  | _ :: _, [] => none

private def stripPatternProjectionPrefix? (projectionPrefix path : Array LoanProjection) :
    Option (Array LoanProjection) :=
  match stripProjectionPrefix? projectionPrefix.toList path.toList with
  | some suffix => some suffix.toArray
  | none => match stripProjectionPrefix? path.toList projectionPrefix.toList with
      | some _ => some #[]
      | none => none

private def loanPathsAtPlace (loan : LoanFact) (place : LoanPlace) :
    Array (Array LoanProjection) :=
  let carriers := loan.holderPlaces.filter (·.root == place.root)
  if carriers.isEmpty then
    if place.root.local?.any loan.holders.contains then #[#[]] else #[]
  else carriers.filterMap fun carrier => do
    match stripProjectionPrefix? place.projections.toList carrier.projections.toList with
    | some suffix => some suffix.toArray
    | none => if placesOverlap carrier place then some #[] else none

/-- A region of a signature: a lifetime binder of the declaration, `'static`, or
one nothing is declared of, an elided lifetime or one inside a type argument,
which is the caller's. -/
private inductive SignatureRegion where
  | binder (index : Nat)
  | static
  | unknown
  /-- A position at the calling function's own type parameter. -/
  | callerParameter
  deriving BEq, Inhabited

private def appendUniqueRegions (left right : Array SignatureRegion) : Array SignatureRegion :=
  right.foldl (init := left) fun regions region =>
    if regions.contains region then regions else regions.push region

/-- A lifetime of a declared type under the arguments of the nominals around it,
innermost first. `foreign` is a position inside a type argument. -/
private def regionInContexts (ns : ValidatedNamespace) (foreign : Bool) :
    List (Array GenericArgument) → LifetimeId → SignatureRegion
  | [], lifetime =>
      match ns.tables.lifetimes[lifetime.index]? with
      | some { kind := .static, .. } => if foreign then .unknown else .static
      | some { kind := .parameter index, .. } => if foreign then .unknown else .binder index
      | some { kind := .inference, .. } | some { kind := .local, .. } | none => .unknown
  | arguments :: outer, lifetime =>
      match ns.tables.lifetimes[lifetime.index]? with
      | some { kind := .static, .. } => if foreign then .unknown else .static
      | some { kind := .parameter index, .. } =>
          if foreign then .unknown else
          match arguments[index]? with
          | some (.lifetime value) => regionInContexts ns foreign outer value
          | _ => .unknown
      | some { kind := .inference, .. } | some { kind := .local, .. } | none => .unknown

/-- Whether a value of a type can hold a loan of the frame: a lifetime names one, and
a reference among a nominal's declared fields holds one, whether or not its surface
spells a lifetime. A type parameter's regions outlive the frame. -/
private partial def typeMayHoldFrameLoan (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (typeId : TypeId) (fuel : Nat := ns.tables.types.size + 1) : Bool :=
  match fuel, ns.tables.types[typeId.index]? with
  | 0, _ => typeMayContainReference ns typeId
  | _, some (.typeParameter _) => false
  | fuel + 1, some (.tuple elements) => elements.any (typeMayHoldFrameLoan unit ns · fuel)
  | fuel + 1, some (.vector element _) | fuel + 1, some (.slice element) => typeMayHoldFrameLoan unit ns element fuel
  | fuel + 1, some (.nominal name arguments) =>
      ns.profile != some .move && (arguments.any (fun
        | .lifetime _ => true
        | .typeArg use => typeMayHoldFrameLoan unit ns use.typeId fuel
        | .const _ | .evidence _ => false) ||
        match unit.tables.names[name.index]? with
        | none => true
        | some qualified => match unit.namespaces[qualified.namespaceId.index]? with
          | none => true
          | some owner => match owner.structs.find? (·.name == name) with
            | none => true
            | some declaration =>
                (declaration.variants.foldl (fun fields variant => fields ++ variant.fields)
                  declaration.fields).any (typeMayHoldFrameLoan unit ns ·.type.typeId fuel))
  | _, _ => typeMayContainReference ns typeId

/-- The regions a type mentions. A nominal's fields mention only what its
arguments do. A type binder of the signature is its instantiation's, all unknown,
and without one, in the function's own body, it holds no loan of this frame. -/
private partial def regionsBelow (ns : ValidatedNamespace)
    (instantiations : Option (Array GenericArgument)) (foreign : Bool)
    (contexts : List (Array GenericArgument)) (fuel : Nat) (typeId : TypeId) :
    Array SignatureRegion :=
  match fuel, ns.tables.types[typeId.index]? with
  | 0, _ | _, none => #[.unknown]
  | fuel + 1, some type =>
    let below := regionsBelow ns instantiations foreign contexts fuel
    match type with
    | .reference reference =>
        appendUniqueRegions #[regionInContexts ns foreign contexts reference.lifetime]
          (below reference.referent)
    | .tuple elements =>
        elements.foldl (fun regions element => appendUniqueRegions regions (below element)) #[]
    | .vector element _ | .slice element | .typeDomain element => below element
    | .nominal _ arguments => arguments.foldl (init := #[]) fun regions argument =>
        match argument with
        | .lifetime lifetime =>
            appendUniqueRegions regions #[regionInContexts ns foreign contexts lifetime]
        | .typeArg use => appendUniqueRegions regions (below use.typeId)
        | .const _ | .evidence _ => regions
    | .typeParameter index => match contexts with
        | arguments :: outer => match arguments[index]? with
            | some (.typeArg use) =>
                regionsBelow ns instantiations foreign outer fuel use.typeId
            | _ => #[.unknown]
        | [] =>
            if foreign then #[.unknown] else
            match instantiations with
            | none => #[]
            | some arguments => match arguments[index]? with
                | some (.typeArg use) => regionsBelow ns instantiations true [] fuel use.typeId
                | _ => #[.unknown]
    | .function .. | .profile _ | .resourceDomain .. => #[.unknown]
    | .unit | .never | .bool | .character | .string | .bytes | .address | .signer
    | .integer .. | .range | .eventStore | .stateDomain => #[]

/-- The regions of what rests at a path of a value of a type: those below the
place the path reaches, or below where it leaves the type's owned structure. -/
private partial def regionsAtPath (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (instantiations : Option (Array GenericArgument)) (foreign : Bool)
    (contexts : List (Array GenericArgument)) (variant : Option NameId) (fuel : Nat)
    (typeId : TypeId) : List LoanProjection → Array SignatureRegion
  | [] => regionsBelow ns instantiations foreign contexts fuel typeId
  | path@(projection :: rest) =>
    let whole := regionsBelow ns instantiations foreign contexts fuel typeId
    match fuel, ns.tables.types[typeId.index]? with
    | fuel + 1, some (.tuple elements) => match projection with
        | .index (some index) => match elements[index]? with
            | some element =>
                regionsAtPath unit ns instantiations foreign contexts none fuel element rest
            | none => whole
        | _ => whole
    | fuel + 1, some (.vector element _) | fuel + 1, some (.slice element) => match projection with
        | .index _ => regionsAtPath unit ns instantiations foreign contexts none fuel element rest
        | _ => whole
    | fuel + 1, some (.typeParameter index) => match contexts with
        | arguments :: outer => match arguments[index]? with
            | some (.typeArg use) =>
                regionsAtPath unit ns instantiations foreign outer none fuel use.typeId path
            | _ => whole
        | [] => whole
    | fuel + 1, some (.nominal name arguments) => match projection, nominalAt? unit name with
        | .downcast selected, some _ =>
            regionsAtPath unit ns instantiations foreign contexts (some selected) fuel typeId rest
        | .field field, some (_, declaration) =>
            let fields : Array FieldDecl := match variant with
              | none => declaration.fields
              | some selected => match declaration.variants.find?
                  (fun (candidate : VariantDecl) => candidate.name == selected) with
                  | some found => found.fields
                  | none => #[]
            match fields.find? (fun (candidate : FieldDecl) => candidate.name == field) with
            | some found =>
                regionsAtPath unit ns instantiations foreign (arguments :: contexts) none fuel
                  found.type.typeId rest
            | none => whole
        | _, _ => whole
    | _, _ => whole

/-- The references a value of a type holds by value, with their paths and
regions: the positions a loan can rest at. An opaque nominal's are its own
regions. Unknown where the type does not say (a function value). -/
private partial def signaturePositions (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (instantiations : Option (Array GenericArgument)) (foreign : Bool)
    (contexts : List (Array GenericArgument)) (prefix_ : Array LoanProjection)
    (visiting : Array NameId) (fuel : Nat) (typeId : TypeId) :
    Option (Array (Array LoanProjection × SignatureRegion)) :=
  match fuel, ns.tables.types[typeId.index]? with
  | 0, _ | _, none => none
  | _, some (.reference reference) =>
      some #[(prefix_, regionInContexts ns foreign contexts reference.lifetime)]
  | fuel + 1, some (.tuple elements) =>
      elements.zipIdx.foldlM (m := Option) (init := #[]) fun positions (element, index) => do
        let nested ← signaturePositions unit ns instantiations foreign contexts
          (prefix_.push (.index (some index))) visiting fuel element
        pure (positions ++ nested)
  | fuel + 1, some (.vector element _) | fuel + 1, some (.slice element) =>
      signaturePositions unit ns instantiations foreign contexts (prefix_.push (.index none))
        visiting fuel element
  | fuel + 1, some (.typeDomain element) =>
      signaturePositions unit ns instantiations foreign contexts prefix_ visiting fuel element
  | fuel + 1, some (.typeParameter index) => match contexts with
      | arguments :: outer => match arguments[index]? with
          | some (.typeArg use) =>
              signaturePositions unit ns instantiations foreign outer prefix_ visiting fuel
                use.typeId
          | _ => none
      | [] =>
          if foreign then some #[(prefix_, .callerParameter)] else
          match instantiations with
          | none => some #[(prefix_, .unknown)]
          | some arguments => match arguments[index]? with
              | some (.typeArg use) =>
                  signaturePositions unit ns instantiations true [] prefix_ visiting fuel use.typeId
              | _ => none
  | fuel + 1, some (.nominal name arguments) =>
      if visiting.contains name then some #[(prefix_, .unknown)] else do
      let (_, declaration) ← nominalAt? unit name
      if declaration.fields.isEmpty && declaration.variants.isEmpty then
        return (regionsBelow ns instantiations foreign contexts (fuel + 1) typeId).map (prefix_, ·)
      let visiting := visiting.push name
      let inFields (prefix_ : Array LoanProjection) (fields : Array FieldDecl) :=
        fields.foldlM (m := Option) (init := #[]) fun positions field => do
          let nested ← signaturePositions unit ns instantiations foreign (arguments :: contexts)
            (prefix_.push (.field field.name)) visiting fuel field.type.typeId
          pure (positions ++ nested)
      if declaration.variants.isEmpty then inFields prefix_ declaration.fields
      else declaration.variants.foldlM (m := Option) (init := #[]) fun positions variant => do
        let nested ← inFields (prefix_.push (.downcast variant.name)) variant.fields
        pure (positions ++ nested)
  | _, some (.profile _) | _, some (.function ..) |
      _, some (.resourceDomain ..) => none
  | _, some .unit | _, some .never | _, some .bool | _, some .character |
      _, some .string | _, some .bytes | _, some .address | _, some .signer |
      _, some (.integer ..) | _, some .range |
      _, some .eventStore | _, some .stateDomain => some #[]

/-- The outlives bounds a type implies, as rustc assumes them of a signature: what
a reference's referent mentions outlives the reference. -/
private partial def impliedRegionEdges (ns : ValidatedNamespace)
    (fuel : Nat) (found : Array (SignatureRegion × SignatureRegion)) (typeId : TypeId) :
    Array (SignatureRegion × SignatureRegion) :=
  match fuel, ns.tables.types[typeId.index]? with
  | 0, _ | _, none => found
  | fuel + 1, some type => match type with
    | .reference reference =>
        let shorter := regionInContexts ns false [] reference.lifetime
        let found := (regionsBelow ns none false [] fuel reference.referent).foldl
          (init := found) fun found longer => found.push (longer, shorter)
        impliedRegionEdges ns fuel found reference.referent
    | .tuple elements => elements.foldl (impliedRegionEdges ns fuel) found
    | .vector element _ | .slice element | .typeDomain element => impliedRegionEdges ns fuel found element
    | .nominal _ arguments => arguments.foldl (init := found) fun found argument =>
        match argument with
        | .typeArg use => impliedRegionEdges ns fuel found use.typeId
        | .lifetime _ | .const _ | .evidence _ => found
    | .typeParameter _ | .function .. | .profile _ | .resourceDomain ..
    | .unit | .never | .bool | .character | .string | .bytes | .address | .signer
    | .integer .. | .range | .eventStore | .stateDomain => found

/-- The outlives bounds of a signature, declared and implied. -/
private def signatureRegionEdges (ns : ValidatedNamespace)
    (signature : Signature) : Array (SignatureRegion × SignatureRegion) :=
  let predicates := signature.generics.foldl
    (fun predicates binder => predicates ++ binder.predicates) signature.predicates
  let declared := predicates.filterMap fun predicate => match predicate with
    | .lifetimeOutlives longer shorter =>
        some (regionInContexts ns false [] longer, regionInContexts ns false [] shorter)
    | _ => none
  let types := signature.parameters.map (·.typeUse.typeId) ++ signature.results.map (·.typeId)
  types.foldl (impliedRegionEdges ns (ns.tables.types.size + 1)) declared

/-- Reachability over outlives edges. The worklist carries a visited set, keeping
the search polynomial; enumerating paths instead is exponential on diamond- or
cycle-shaped predicate edges. -/
private def regionOutlivesVia (edges : Array (SignatureRegion × SignatureRegion))
    (longer shorter : SignatureRegion) : Bool :=
  go (edges.size + 1) #[longer] #[]
where
  go : Nat → Array SignatureRegion → Array SignatureRegion → Bool
    | 0, _, _ => false
    | fuel + 1, frontier, visited =>
        frontier.contains shorter ||
          (let visited := visited ++ frontier
           let next := edges.foldl (init := #[]) fun next edge =>
             if frontier.contains edge.1 && !visited.contains edge.2 &&
                 !next.contains edge.2 then next.push edge.2 else next
           !next.isEmpty && go fuel next visited)

/-- Whether a loan resting at `source` may rest at `target`. A region nothing is
declared of may be related to any by what this unit does not see. -/
private def regionsRelated (edges : Array (SignatureRegion × SignatureRegion))
    (source target : SignatureRegion) : Bool :=
  source == .unknown || target == .unknown || target == .callerParameter ||
    source == .static || regionOutlivesVia edges source target

/-- Whether a loan passed at `sources` may rest at a call result's position. A caller's
type parameter outlives the caller's body, so no loan of its frame rests there. -/
private def loanMayRestAt (edges : Array (SignatureRegion × SignatureRegion)) (loan : LoanFact)
    (sources : Array SignatureRegion) (target : SignatureRegion) : Bool :=
  (target != .callerParameter || loan.canEscape) && sources.any (regionsRelated edges · target)

/-- The positions of a call's result: each result in turn, with a path below its
index where the function has several. -/
private def callResultPositions? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (signature : Signature) (instantiations : Option (Array GenericArgument)) :
    Option (Array (Array LoanProjection × SignatureRegion)) :=
  let fuel := ns.tables.types.size + 1
  match signature.results.toList with
  | [] => some #[]
  | [result] => signaturePositions unit ns instantiations false [] #[] #[] fuel result.typeId
  | _ => signature.results.zipIdx.foldlM (m := Option) (init := #[]) fun positions (result, index) => do
      let nested ← signaturePositions unit ns instantiations false [] #[.index (some index)] #[]
        fuel result.typeId
      pure (positions ++ nested)

/-- The regions of the parameters a loan is passed at, at its paths in each
argument. -/
private def loanRegionsAtCall (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (signature : Signature) (instantiations : Array GenericArgument) (arguments : Array ExprId)
    (loan : LoanFact) (pathsIn : ExprId → Array (Array LoanProjection)) : Array SignatureRegion :=
  (arguments.zip signature.parameters).foldl (init := #[]) fun regions (argument, parameter) =>
    if !loanFlowsFromExpression ns loan argument then regions else
    (pathsIn argument).foldl (init := regions) fun regions path =>
      appendUniqueRegions regions <| regionsAtPath unit ns (some instantiations) false [] none
        (ns.tables.types.size + 1) parameter.typeUse.typeId path.toList

private partial def loanPathsInExpression (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (loan : LoanFact)
    (exprId : ExprId) (fuel : Nat) : Array (Array LoanProjection) :=
  match fuel, ns.expressions[exprId.index]? with
  | 0, _ | _, none => if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]
  | _, some { kind := .localVar holder, .. } =>
      let carriers := loan.holderPlaces.filter (·.root == .local holder)
      if carriers.isEmpty then
        if loan.holders.contains holder then #[#[]] else #[]
      else carriers.map (·.projections)
  | fuel + 1, some { kind := .operation operation instantiations arguments _, .. } =>
      match operation with
      | .move place | .copy place | .read place =>
          match loanPlace? ns place with
          | some place => loanPathsAtPlace loan place
          | none => if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]
      | .primitive .tuple | .primitive .vector =>
          (arguments.zip (Array.range arguments.size)).foldl (init := #[]) fun paths pair =>
            let nested := loanPathsInExpression unit ns loan pair.1 fuel |>.map fun path =>
              #[.index (some pair.2)] ++ path
            appendUniqueProjectionPaths paths nested
      -- A call's result carries only loans its arguments carry, at the
      -- components whose regions those at the arguments outlive.
      | .call (.function reference) =>
          if !loanFlowsFromExpression ns loan exprId then #[] else
          match functionAt? unit reference with
            | some (_, declaration) =>
                let signature := declaration.signature
                match callResultPositions? unit ns signature (some instantiations) with
                | none => #[#[]]
                | some positions =>
                    let edges := signatureRegionEdges ns signature
                    let sources := loanRegionsAtCall unit ns signature instantiations arguments loan
                      (loanPathsInExpression unit ns loan · fuel)
                    positions.foldl (init := #[]) fun paths (path, target) =>
                      if loanMayRestAt edges loan sources target then
                        appendUniqueProjectionPaths paths #[path]
                      else paths
            | none => #[#[]]
      | .call (.constructor reference variantName) =>
          match nominalAt? unit reference.name with
            | some (declarationNs, declaration) =>
                let selected : Option (Option NameId × Array FieldDecl) := match variantName with
                  | none => if declaration.variants.isEmpty then
                      some (none, declaration.fields)
                    else none
                  | some spelling => do
                      let variant ← declaration.variants.find? fun candidate =>
                        (declarationNs.tables.names[candidate.name.index]?).any (·.name == spelling)
                      some (some variant.name, variant.fields)
                match selected with
                | none => if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]
                | some (variant, fields) => if fields.size != arguments.size then
                  if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]
                else (fields.zip arguments).foldl (init := #[])
                  fun paths pair =>
                    let nested := loanPathsInExpression unit ns loan pair.2 fuel |>.map fun path =>
                      (variant.map (#[.downcast ·]) |>.getD #[]) ++
                        #[.field pair.1.name] ++ path
                    appendUniqueProjectionPaths paths nested
            | none => if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]
      | _ => if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]
  | fuel + 1, some { kind := .block _ (some result), .. } =>
      loanPathsInExpression unit ns loan result fuel
  | fuel + 1, some { kind := .letDecl _ _ body, .. } =>
      loanPathsInExpression unit ns loan body fuel
  | fuel + 1, some { kind := .ifElse _ thenBranch elseBranch, .. } =>
      let paths := loanPathsInExpression unit ns loan thenBranch fuel
      match elseBranch with
      | some branch => appendUniqueProjectionPaths paths
          (loanPathsInExpression unit ns loan branch fuel)
      | none => paths
  | fuel + 1, some { kind := .match_ _ arms, .. } =>
      arms.foldl (init := #[]) fun paths arm =>
        appendUniqueProjectionPaths paths (loanPathsInExpression unit ns loan arm.body fuel)
  | _, some _ => if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]

private def bindObservedLoansFromExpressionAtProjection (unit : ValidatedUnit)
    (ns : ValidatedNamespace)
    (state : BorrowState) (observed : Array LoanFact) (holder : LocalId)
    (expression : ExprId) (projectionPrefix : Array LoanProjection) : BorrowState :=
  { active := state.active.map fun loan =>
      match observed.find? (·.source == loan.source) with
      | none => loan
      | some carried =>
          let paths := loanPathsInExpression unit ns carried expression (ns.expressions.size + 1)
            |>.filterMap (stripPatternProjectionPrefix? projectionPrefix)
          if paths.isEmpty then loan else
            let holderPlaces := paths.map fun projections =>
              ({ root := .local holder, projections } : LoanPlace)
            { loan with
              holders := if loan.holders.contains holder then loan.holders else loan.holders.push holder
              holderPlaces := appendUniqueLoanPlaces loan.holderPlaces holderPlaces } }

private def bindObservedLoansFromExpression (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (state : BorrowState)
    (observed : Array LoanFact) (holder : LocalId) (expression : ExprId) : BorrowState :=
  bindObservedLoansFromExpressionAtProjection unit ns state observed holder expression #[]

private def bindObservedLoansAtPlaceFromExpression (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (state : BorrowState) (observed : Array LoanFact) (placeId : PlaceId)
    (expression : ExprId) : BorrowState :=
  match loanPlace? ns placeId with
  | none => state
  | some place =>
    if !isOwnedCarrierPlace place then
      let callerStorage := ns.profile != some .move &&
        (referenceCarrier? place).any fun carrier =>
          let parents := loansCarriedAt state carrier
          parents.isEmpty || parents.any (·.canEscape)
      { active := state.active.map fun loan =>
          if callerStorage && loan.canEscape && observed.any (·.source == loan.source) then
            { loan with callerHeld := true }
          else loan }
    else
    { active := state.active.map fun loan =>
        match observed.find? (·.source == loan.source) with
        | none => loan
        | some carried =>
            let paths := loanPathsInExpression unit ns carried expression (ns.expressions.size + 1)
            if paths.isEmpty then loan else
              let holderPlaces := paths.map fun projections =>
                ({ place with projections := place.projections ++ projections } : LoanPlace)
              { loan with
                holders := match place.root.local? with
                  | some rootLocal =>
                      if loan.holders.contains rootLocal then loan.holders
                      else loan.holders.push rootLocal
                  | none => loan.holders
                holderPlaces := appendUniqueLoanPlaces loan.holderPlaces holderPlaces } }

private def replaceObservedLoansAtPlaceFromExpression (unit : ValidatedUnit)
    (ns : ValidatedNamespace)
    (state : BorrowState) (observed : Array LoanFact) (placeId : PlaceId)
    (expression : ExprId) : BorrowState :=
  bindObservedLoansAtPlaceFromExpression unit ns
    (releasePlaceLoans ns state observed placeId) observed placeId expression

private def patternProjectionChildren? (unit : ValidatedUnit) :
    PatternKind → Option (Array (PatternId × Array LoanProjection))
  | .tuple patterns => some <| patterns.zipIdx.map fun pair =>
      (pair.1, #[.index (some pair.2)])
  | .constructor name _ variant patterns => do
      let (declarationNs, declaration) ← nominalAt? unit name
      let selected : Option (Option NameId × Array FieldDecl) := match variant with
        | none => if declaration.variants.isEmpty then some (none, declaration.fields) else none
        | some spelling => do
            let variant ← declaration.variants.find? fun candidate =>
              (declarationNs.tables.names[candidate.name.index]?).any (·.name == spelling)
            some (some variant.name, variant.fields)
      let (variant, fields) ← selected
      if patterns.size != fields.size then none else
      some <| (patterns.zip fields).map fun pair =>
        (pair.1, (variant.map (#[.downcast ·]) |>.getD #[]) ++ #[.field pair.2.name])
  | _ => none

/-- Bind or replace aggregate-pattern holders using the matching value operand.
Unknown producer shapes retain the prior all-to-all behavior rather than
silently dropping a possible alias. -/
private partial def updatePatternLoansFromExpression
    (replace : Bool)
    (unit : ValidatedUnit) (ns : ValidatedNamespace) (state : BorrowState)
    (observed : Array LoanFact)
    (pattern : PatternId) (expression : ExprId) (projectionPrefix : Array LoanProjection)
    (fuel : Nat) : BorrowState :=
  let updateFallback (state : BorrowState) (holder : LocalId) :=
    if replace then replaceHolderLoans state observed holder
    else bindObservedLoans state observed holder
  match fuel, ns.patterns[pattern.index]? with
  | 0, _ | _, none =>
      (patternVariables ns pattern).foldl updateFallback state
  | _, some { kind := .variable holder, typeId, .. } =>
      -- A holder whose type holds no reference holds no loan.
      if !typeMayContainReference ns typeId then state else
      let selected := observed.filter (loanFlowsFromExpression ns · expression)
      if replace then
        bindObservedLoansFromExpressionAtProjection unit ns
          (releaseHolderLoans state selected holder) selected holder expression projectionPrefix
      else bindObservedLoansFromExpressionAtProjection unit ns state selected holder expression
        projectionPrefix
  | fuel + 1, some aggregate@{ kind := .tuple _, .. } |
      fuel + 1, some aggregate@{ kind := .constructor .., .. } =>
      match patternProjectionChildren? unit aggregate.kind with
      | some children => children.foldl (init := state) fun state child =>
          updatePatternLoansFromExpression replace unit ns state observed child.1 expression
            (projectionPrefix ++ child.2) fuel
      | none => (patternVariables ns pattern).foldl updateFallback state
  | _, some { kind := .wildcard, .. } | _, some { kind := .literal _, .. } |
      _, some { kind := .range .., .. } => state

private def bindPatternLoans (unit : ValidatedUnit) (ns : ValidatedNamespace) (state : BorrowState)
    (observed : Array LoanFact) (pattern : PatternId) (expression : ExprId) : BorrowState :=
  updatePatternLoansFromExpression false unit ns state observed pattern expression
    #[] (ns.patterns.size + 1)

private def replacePatternLoans (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (state : BorrowState)
    (observed : Array LoanFact) (pattern : PatternId) (expression : ExprId) : BorrowState :=
  updatePatternLoansFromExpression true unit ns state observed pattern expression
    #[] (ns.patterns.size + 1)

private def instantiatedLifetimeInContexts (ns : ValidatedNamespace) :
    List (Array GenericArgument) → LifetimeId → LifetimeId
  | [], lifetime => lifetime
  | instantiations :: outer, lifetime =>
      match ns.tables.lifetimes[lifetime.index]? with
      | some { kind := .parameter index, .. } => match instantiations[index]? with
          | some (.lifetime value) => instantiatedLifetimeInContexts ns outer value
          | _ => lifetime
      | _ => lifetime

/-- The loans a direct call's result carries: those passed at a parameter whose
region outlives the region of a position of the result. Unknown where the callee
or its result type is. -/
private def directCallResultLoans? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (reference : QualifiedRef)
    (instantiations : Array GenericArgument) (arguments : Array ExprId)
    (observed : Array LoanFact) : Option (Array LoanFact) := do
  let (_, declaration) ← functionAt? unit reference
  let signature := declaration.signature
  let positions ← callResultPositions? unit ns signature (some instantiations)
  if positions.isEmpty then some #[] else
  let edges := signatureRegionEdges ns signature
  some <| observed.filter fun loan =>
    let sources := loanRegionsAtCall unit ns signature instantiations arguments loan
      (loanPathsInExpression unit ns loan · (ns.expressions.size + 1))
    positions.any fun (_, target) => loanMayRestAt edges loan sources target

private def discardUncarriedTemporaries (state : BorrowState)
    (observed carried : Array LoanFact) : BorrowState :=
  { active := state.active.filter fun loan =>
      loan.callerHeld || !loan.holders.isEmpty || !observed.any (·.source == loan.source) ||
        carried.any (·.source == loan.source) }

/-- The flow after a call, direct or through a function value, whose result
carries `directLoans` of the loans its arguments `observed`. A mutable
reference result that no argument lends is a loan of the call site: the
callee's return registers it, and its death ends it. -/
private def callResultFlow (ns : ValidatedNamespace) (exprId : ExprId) (expression : Expr)
    (afterArguments : BorrowState) (observed directLoans : Array LoanFact) :
    Option BorrowState × Array LoanFact × Array LoanFact × Array Diagnostic :=
  -- Incoming parameter origins carry provenance, but have no local runtime
  -- loan event. A mutable reference returned through them still needs the
  -- call-site registration and death that an otherwise untracked result uses.
  let onlyIncoming := directLoans.all (·.source.expression?.isNone)
  let produced := if onlyIncoming then
    match referenceType? ns expression.typeId with
    | some referenceType =>
        if referenceType.kind == .mutable then
          #[{ source := .expression exprId
              kind := .mutable
              place := { root := .external }
              lifetime := referenceType.lifetime
              loc := expression.loc
              canEscape := true : LoanFact }]
        else #[]
    | none => #[]
  else #[]
  let valueLoans := appendUniqueLoans directLoans produced
  let afterArguments := { afterArguments with
    active := appendUniqueLoans afterArguments.active produced }
  (some (discardUncarriedTemporaries afterArguments observed valueLoans), valueLoans, produced, #[])

/-- At a function-result boundary, only mutable loans carried by the result
remain live. Every other mutable loan has a statically known death at that
boundary, where the semantics ends it; immutable loans need no prophetic
write-back and record no death. -/
private def retainResultLoans (state : BorrowState)
    (carried : Array LoanFact) : BorrowState :=
  { active := state.active.filter fun loan =>
      loan.callerHeld || loan.kind != .mutable || carried.any (·.source == loan.source) }

private def releaseHoldersUnusedBy (ns : ValidatedNamespace) (state : BorrowState)
    (holders : Array LocalId) (expressions : List ExprId) : BorrowState :=
  holders.foldl (init := state) fun state holder =>
    if expressions.any (expressionUsesLocal ns · holder (ns.expressions.size + 1)) then state
    else releaseHolderLoans state #[] holder

mutual
  /-- Analyze one expression and record loan deaths differentially: a
  mutable loan live at entry (or first observed inside) and gone at exit
  died within this expression. The record anchors at this node unless a
  descendant already recorded a more precise point. -/
  private partial def analyzeExpr (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (exprId : ExprId)
      (state : BorrowState) (preserved : Array LoanSource := #[])
      (discarded : Bool := false) (readOnly : Bool := false) (future : List ExprId := []) :
      BorrowFlow :=
    let flow := analyzeExprCore unit ns exprId state preserved discarded readOnly future
    let flow := { flow with deaths := flow.deaths.atPoint exprId true state }
    match flow.normal with
    | none => flow
    | some exit =>
        let flow := { flow with deaths := flow.deaths.atPoint exprId false exit }
        let candidates := appendUniqueLoans state.active flow.observed
        let died := candidates.filter fun loan =>
          loan.kind == .mutable && loan.source.expression?.isSome &&
            !exit.active.any (·.source == loan.source) &&
            !flow.deaths.deaths.any (fun death => loan.source == .expression death.1)
        -- What removes a loan without recording its death removes one no place
        -- holds: a temporary a call took, or a value it moved. A merged fact
        -- would name places that hold other references by now.
        let unheld := died.map fun loan => ({ loan with holders := #[], holderPlaces := #[] } : LoanFact)
        if died.isEmpty then flow
        else { flow with deaths := pushDeaths flow.deaths unheld { anchor := exprId } }

  /-- `readOnly` marks an operand only read through: evaluating a mutable
  reference anywhere else may write its referent. `future` is what evaluation
  can reach after the node. -/
  private partial def analyzeExprCore (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (exprId : ExprId)
      (state : BorrowState) (preserved : Array LoanSource := #[])
      (discarded : Bool := false) (readOnly : Bool := false) (future : List ExprId := []) :
      BorrowFlow :=
    match ns.expressions[exprId.index]? with
    | none => { normal := some state }
    | some expression => match expression.kind with
        | .value .. | .constant _ | .quantifier .. | .spec _ => { normal := some state }
        -- A read of a local moves nothing: `inferLocalAccess` has already made
        -- the read of a value without `Copy` a `.move`, which alone arms
        -- `movedFrom`.
        | .localVar localId =>
            let place : LoanPlace := { root := .local localId }
            let loans := state.active.filter (·.holders.contains localId)
            -- A mutable reference used other than for reading may write
            -- through it, which conflicts with any loan reborrowed through it.
            let referentDiagnostics := match referenceType? ns expression.typeId with
              | some reference =>
                  if reference.kind == .mutable && !readOnly then
                    -- A reborrow the reference itself now holds is its value.
                    let others := { state with
                      active := state.active.filter (!·.holders.contains localId) }
                    accessDiagnostics others expression.loc
                      { root := .local localId, projections := #[.dereference] } true
                      "mutable reference use"
                  else #[]
              | none => #[]
            { normal := some state
              valueLoans := loans
              observed := loans
              diagnostics := accessDiagnostics state expression.loc place false "local read" ++
                referentDiagnostics }
        | .operation operation instantiations arguments _ =>
            let overwritten := match operation with
              | .write place => overwrittenLoans ns state place
              | _ => #[]
            let argumentState := match operation with
              | .write place => retireOverwritten ns
                  (releasePlaceLoansUnusedBy ns state place arguments.toList) overwritten
                  (arguments.toList ++ future)
              -- A drop ends the loans its value alone holds, while it still
              -- holds them.
              | .drop place => releasePlaceLoans ns state #[] place
              | _ => state
            let entryDeaths := match arguments[0]? with
              | some first =>
                  pushDeaths {} (diffLoans state argumentState)
                    { anchor := first, before := true }
              | none =>
                  pushDeaths {} (diffLoans state argumentState) { anchor := exprId, before := true }
            let operationPreserved := match operationPlace? operation with
              | some place => loansUsedByPlace ns place argumentState preserved
              | none => preserved
            let readOnly := match operation with
              | .reference .dereference | .reference (.freeze _) => true
              | _ => false
            let argumentsFlow := analyzeExprList unit ns arguments.toList argumentState
              operationPreserved readOnly (future := future)
            match argumentsFlow.normal with
            | none => argumentsFlow
            | some afterArguments =>
                let (normal, valueLoans, operationObserved, diagnostics) := match operation with
                  | .global (.borrow kind) =>
                      let (state, loans, diagnostics) := addGlobalBorrow ns exprId
                        expression.loc expression.typeId kind instantiations afterArguments
                      (some state, loans, loans, diagnostics)
                  | .global .take | .global .publish =>
                      /- Removing or replacing the owner conflicts with any
                      live reference into the family, just like a local
                      consume/write. Use the same family identity as global
                      borrowing; contains does not invalidate an owner. -/
                      let diagnostics := match (instantiations[0]? : Option GenericArgument) with
                        | some (.typeArg resource) =>
                            accessDiagnostics afterArguments expression.loc
                              { root := .global resource.typeId } true "global owner write"
                        | _ => #[]
                      -- A temporary borrow, of the signer `move_to` takes, ends
                      -- with the operation: its result carries no loan.
                      (some (discardUncarriedTemporaries afterArguments argumentsFlow.observed #[]),
                        #[], #[], diagnostics)
                  | .borrow kind place =>
                      let (state, loans, diagnostics) :=
                        addBorrow ns exprId expression.loc expression.typeId kind place afterArguments
                          discarded
                      let parentLoans := loansHeldAtPlace ns afterArguments place
                      let valueLoans := appendUniqueLoans parentLoans loans
                      (some state, valueLoans, valueLoans, diagnostics)
                  | .readDiscriminant place =>
                      (some afterArguments, #[], loansHeldAtPlace ns afterArguments place,
                        checkPlaceAccess ns afterArguments expression.loc place false "enum tag read")
                  | .copy place | .read place =>
                      let accessed := loansHeldAtPlace ns afterArguments place
                      let valueLoans := if typeMayContainReference ns expression.typeId then
                        accessed else #[]
                      (some afterArguments, valueLoans, accessed,
                        checkPlaceAccess ns afterArguments expression.loc place false "place read")
                  | .move place | .drop place =>
                      let accessed := if operation matches .move _ then
                        loansHeldAtPlace ns afterArguments place else #[]
                      let valueLoans := if typeMayContainReference ns expression.typeId then
                        accessed else #[]
                      let afterConsume := releasePlaceLoans ns afterArguments valueLoans place
                      -- A loan the caller lends has no death here, and a dropped
                      -- value could not export it at the exit.
                      let droppedIncoming := state.active.any fun (loan : LoanFact) =>
                        loan.kind == ReferenceKind.mutable && loan.source.expression?.isNone &&
                          !argumentState.active.any (·.source == loan.source)
                      (some afterConsume, valueLoans, accessed,
                        checkPlaceAccess ns afterArguments expression.loc place true "place consume" ++
                          dereferenceConsumptionDiagnostics ns expression.loc place ++
                          if droppedIncoming then #[Diagnostic.at "VIR-SEMANTIC-BORROW-DROP"
                            "a drop of a mutable reference the function's caller lends is not supported"
                            expression.loc] else #[])
                  | .write place =>
                      let afterWrite := match arguments[0]? with
                        | some value => replaceObservedLoansAtPlaceFromExpression unit ns afterArguments
                            argumentsFlow.valueLoans place value
                        | none => releasePlaceLoans ns afterArguments
                            argumentsFlow.valueLoans place
                      (some (forgetMovedOutOf ns afterWrite place), #[], #[],
                        checkPlaceAccess ns afterArguments expression.loc place true "place write" ++
                          (if (arguments[0]?.bind fun value =>
                              carrierRetirement? ns place value state argumentsFlow.valueLoans).isSome then #[]
                            else overwrittenLoanDiagnostics ns state argumentState expression.loc ++
                              overwrittenLoanDiagnostics ns afterArguments afterWrite expression.loc) ++
                          replacementEscapeDiagnostics ns afterArguments place argumentsFlow.valueLoans expression.loc ++
                          overwrittenLiveDiagnostics afterWrite overwritten
                            argumentsFlow.valueLoans expression.loc)
                  | .call (.function reference) =>
                      let directLoans := if typeMayContainReference ns expression.typeId then
                        (directCallResultLoans? unit ns reference instantiations arguments
                          argumentsFlow.observed).getD argumentsFlow.observed
                        else #[]
                      callResultFlow ns exprId expression afterArguments argumentsFlow.observed
                        directLoans
                  -- The callee is unknown: the result may carry any loan an
                  -- argument lends.
                  | .call .invoke =>
                      let receiverScoped := (arguments[0]?).any
                        (ReceiverNoEscape.scopedInvocationAt ns exprId)
                      let directLoans := if typeMayContainReference ns expression.typeId then
                        argumentsFlow.observed.filter fun (loan : LoanFact) =>
                          -- Only the receiver's private loan ends. A loan also
                          -- lent by another argument or the callable remains.
                          loan.canEscape || !receiverScoped ||
                            !(arguments[1]?).any (loanFlowsFromExpression ns loan) ||
                            (arguments.zipIdx).any (fun (argument, index) =>
                              index != 1 && loanFlowsFromExpression ns loan argument)
                        else #[]
                      callResultFlow ns exprId expression afterArguments argumentsFlow.observed
                        directLoans
                  | _ =>
                      let valueLoans := if typeMayContainReference ns expression.typeId then
                        argumentsFlow.observed else #[]
                      -- A temporary borrow the result cannot carry ends with the
                      -- operation, as it does at a call.
                      (some (discardUncarriedTemporaries afterArguments argumentsFlow.observed
                        valueLoans), valueLoans, #[], #[])
                -- A write ends what it destroys once its operand is evaluated.
                let writeDeaths := match operation, normal, arguments[0]? with
                  | .write place, some after, some first =>
                      addCarrierRetirement
                        (pushDeaths {} (diffLoans afterArguments after) { anchor := first })
                        (carrierRetirement? ns place first state argumentsFlow.valueLoans)
                  | _, _, _ => {}
                { argumentsFlow with
                  normal
                  valueLoans
                  observed := appendUniqueLoans argumentsFlow.observed operationObserved
                  deaths := appendUniqueDeaths (appendUniqueDeaths entryDeaths argumentsFlow.deaths)
                    writeDeaths
                  diagnostics := argumentsFlow.diagnostics ++ diagnostics }
        | .block statements result =>
            analyzeBlockExprList unit ns (statements.toList ++ result.toList) state preserved future
        | .letDecl pattern initializer body =>
            let initializerFlow : BorrowFlow := match initializer with
              | none => { normal := some state }
              | some value =>
                  -- Only a direct Move borrow can use this rule. In
                  -- particular, do not propagate non-use through an
                  -- initializer that might observe a reference internally.
                  -- Avoid the extra liveness walk for all other lets and
                  -- when there is no existing loan to conflict with.
                  let discarded := !state.active.isEmpty && ns.profile == some .move &&
                    (match ns.expressions[value.index]? with
                      | some { kind := .operation (.borrow .mutable place) _ _ _, .. } =>
                          (directLocalPlace? ns place).isSome
                      | _ => false) &&
                    !(patternVariables ns pattern (ns.patterns.size + 1)).any
                      (fun localId => expressionUsesLocal ns body localId (ns.expressions.size + 1))
                  analyzeExpr unit ns value state preserved discarded (future := body :: future)
            match initializerFlow.normal with
            | none => initializerFlow
            | some afterInitializer =>
                let afterInitializer := (patternVariables ns pattern).foldl
                  (fun state holder => forgetMovedOut state { root := .local holder })
                  afterInitializer
                let afterInitializer := match initializer with
                  | some value => bindPatternLoans unit ns afterInitializer
                      initializerFlow.valueLoans pattern value
                  | none => afterInitializer
                let bound := afterInitializer
                -- A `let` writes every local it binds. In a loop the same
                -- local is bound again on each iteration, while a loan taken
                -- in the previous one — indexed by that local — may still be
                -- live; its death would then read the new binding.
                let indexDiagnostics := (patternVariables ns pattern (ns.patterns.size + 1)).foldl
                  (init := #[]) fun diagnostics localId =>
                    diagnostics ++ indexWriteDiagnostics ns bound expression.loc localId
                let afterInitializer := discardUncarriedTemporaries afterInitializer
                  initializerFlow.valueLoans #[]
                let initializerObserved := appendUniqueLoans initializerFlow.observed <|
                  recordBoundLoans afterInitializer initializerFlow.valueLoans
                let afterInitializer := retainLoansUsedBy ns (body :: future) afterInitializer preserved
                let entryDeaths := pushDeaths {} (diffLoans bound afterInitializer)
                  { anchor := body, before := true }
                let bodyFlow := analyzeExpr unit ns body afterInitializer preserved (future := future)
                { bodyFlow with
                  breaks := initializerFlow.breaks ++ bodyFlow.breaks
                  breakValueLoans := initializerFlow.breakValueLoans ++ bodyFlow.breakValueLoans
                  continues := initializerFlow.continues ++ bodyFlow.continues
                  returns := initializerFlow.returns ++ bodyFlow.returns
                  observed := appendUniqueLoans initializerObserved bodyFlow.observed
                  deaths := appendUniqueDeaths
                    (appendUniqueDeaths initializerFlow.deaths entryDeaths) bodyFlow.deaths
                  diagnostics := initializerFlow.diagnostics ++ indexDiagnostics ++
                    bodyFlow.diagnostics }
        | .ifElse condition thenBranch elseBranch =>
            -- After the condition one branch runs: the node itself decides.
            let conditionFlow := analyzeExpr unit ns condition state preserved
              (future := exprId :: future)
            match conditionFlow.normal with
            | none => conditionFlow
            | some branchState =>
                let thenPreserved := branchPreserved ns thenBranch preserved
                let thenFuture := branchFuture ns thenBranch future
                let thenState := retainLoansUsedBy ns (thenBranch :: thenFuture) branchState
                  thenPreserved
                let thenEntryDeaths := pushDeaths {} (diffLoans branchState thenState)
                  { anchor := thenBranch, before := true }
                let thenFlow := analyzeExpr unit ns thenBranch thenState thenPreserved
                  (future := thenFuture)
                let (elseEntryDeaths, elseFlow) := match elseBranch with
                  | some branch =>
                      let elsePreserved := branchPreserved ns branch preserved
                      let elseFuture := branchFuture ns branch future
                      let elseState := retainLoansUsedBy ns (branch :: elseFuture) branchState
                        elsePreserved
                      (pushDeaths {} (diffLoans branchState elseState)
                          { anchor := branch, before := true },
                        analyzeExpr unit ns branch elseState elsePreserved (future := elseFuture))
                  | none =>
                      -- The anchor after the conditional runs on both paths: a
                      -- loan the taken branch still holds stays live through it.
                      let thenExit := (thenFlow.normal.map (·.active)).getD #[]
                      let retained := retainLoansUsedBy ns future branchState preserved
                      let kept := branchState.active.filter fun loan =>
                        thenExit.any (·.source == loan.source) ||
                          retained.active.any (·.source == loan.source)
                      let elseState : BorrowState := { active := kept }
                      (pushDeaths {} (diffLoans branchState elseState) { anchor := exprId },
                        { normal := some elseState })
                let branches := mergeFlows thenFlow elseFlow
                { branches with
                  breaks := conditionFlow.breaks ++ branches.breaks
                  breakValueLoans := conditionFlow.breakValueLoans ++ branches.breakValueLoans
                  continues := conditionFlow.continues ++ branches.continues
                  returns := conditionFlow.returns ++ branches.returns
                  observed := appendUniqueLoans conditionFlow.observed branches.observed
                  deaths := appendUniqueDeaths conditionFlow.deaths <|
                    appendUniqueDeaths thenEntryDeaths <|
                      appendUniqueDeaths elseEntryDeaths branches.deaths
                  diagnostics := conditionFlow.diagnostics ++ branches.diagnostics }
        | .match_ scrutinee arms =>
            let scrutineeFlow := analyzeExpr unit ns scrutinee state preserved
              (future := exprId :: future)
            match scrutineeFlow.normal with
            | none => scrutineeFlow
            | some armState =>
                let armsFlow := arms.foldl (fun flow arm =>
                  let armPreserved := branchPreserved ns arm.body preserved
                  let armFuture := branchFuture ns arm.body future
                  -- A failing guard reaches the arms after it.
                  let guardFuture := exprId :: future
                  let reached := match arm.guard with
                    | some guard => guard :: guardFuture
                    | none => arm.body :: armFuture
                  let state := retainLoansUsedBy ns reached armState armPreserved
                  let entryDeaths := pushDeaths {} (diffLoans armState state)
                    { anchor := arm.guard.getD arm.body, before := true }
                  -- An arm writes every local its pattern binds, as a `let`
                  -- does, so in a loop it may re-bind the index of a loan
                  -- held across the back edge.
                  let patternLoc := (ns.patterns[arm.pattern.index]?).map (·.loc)
                    |>.getD expression.loc
                  let indexDiagnostics :=
                    (patternVariables ns arm.pattern (ns.patterns.size + 1)).foldl
                      (init := #[]) fun diagnostics localId =>
                        diagnostics ++ indexWriteDiagnostics ns armState patternLoc localId
                  let armFlow := analyzeArm unit ns arm state armPreserved armFuture guardFuture
                  mergeFlows flow
                    { armFlow with
                      deaths := appendUniqueDeaths entryDeaths armFlow.deaths
                      diagnostics := indexDiagnostics ++ armFlow.diagnostics }) {}
                { armsFlow with
                  breaks := scrutineeFlow.breaks ++ armsFlow.breaks
                  breakValueLoans := scrutineeFlow.breakValueLoans ++ armsFlow.breakValueLoans
                  continues := scrutineeFlow.continues ++ armsFlow.continues
                  returns := scrutineeFlow.returns ++ armsFlow.returns
                  observed := appendUniqueLoans scrutineeFlow.observed armsFlow.observed
                  deaths := appendUniqueDeaths scrutineeFlow.deaths armsFlow.deaths
                  diagnostics := scrutineeFlow.diagnostics ++ armsFlow.diagnostics }
        | .loop _ body =>
            let loopState := retainLoansUsedBy ns (body :: future) state preserved
            let entryDeaths := pushDeaths {} (diffLoans state loopState)
              { anchor := body, before := true }
            let flow := analyzeLoop unit ns body loopState loopState
              (loopState.active.size + ns.expressions.size + 1) preserved (future := future)
            { flow with deaths := appendUniqueDeaths entryDeaths flow.deaths }
        | .break_ nest value =>
            let valueFlow := match value with
              | some value => analyzeExpr unit ns value state preserved (future := future)
              | none => { normal := some state }
            let breaks := match valueFlow.normal with
              | some afterValue => valueFlow.breaks.push (nest, afterValue)
              | none => valueFlow.breaks
            let breakValueLoans := match valueFlow.normal with
              | some _ => valueFlow.breakValueLoans.push (nest, valueFlow.valueLoans)
              | none => valueFlow.breakValueLoans
            { valueFlow with normal := none, breaks, breakValueLoans, valueLoans := #[] }
        | .continue_ nest => { continues := #[(nest, state, exprId)] }
        | .return_ values =>
            let state := retainLoansUsedBy ns values.toList state preserved
            let valuesFlow := analyzeExprList unit ns values.toList state preserved
            match valuesFlow.normal with
            | none => valuesFlow
            | some afterValues =>
                let returned := retainResultLoans afterValues valuesFlow.valueLoans
                let died := diffLoans afterValues returned
                let returned := { returned with
                  returned := values.back?.map (·, valuesFlow.valueLoans) }
                let death := match values.back? with
                  | some anchor => ({ anchor } : LoanDeath)
                  | none => { anchor := exprId, before := true }
                { valuesFlow with
                  normal := none
                  returns := valuesFlow.returns.push returned
                  deaths := pushDeaths valuesFlow.deaths died death }
        | .throw_ _ values =>
            let valuesFlow := analyzeExprList unit ns values.toList state preserved
            match valuesFlow.normal with
            | none => valuesFlow
            | some afterValues =>
                let ended := retainResultLoans afterValues #[]
                let died := diffLoans afterValues ended
                let death := match values.back? with
                  | some anchor => ({ anchor } : LoanDeath)
                  | none => { anchor := exprId, before := true }
                { valuesFlow with
                  normal := none
                  deaths := pushDeaths valuesFlow.deaths died death }
        | .assign place value =>
            let overwritten := overwrittenLoans ns state place
            let released := releasePlaceLoansUnusedBy ns state place [value]
            let valueState := retireOverwritten ns released overwritten (value :: future)
            let entryDeaths := pushDeaths {} (diffLoans state valueState)
              { anchor := value, before := true }
            let assignmentPreserved := loansUsedByPlace ns place valueState preserved
            let valueFlow := analyzeExpr unit ns value valueState assignmentPreserved
              (future := future)
            match valueFlow.normal with
            | none => valueFlow
            | some afterValue =>
                let releasedValue := afterValue
                let bound := replaceObservedLoansAtPlaceFromExpression unit ns afterValue
                  valueFlow.valueLoans place value
                -- Once the value's loans are bound to the place, a loan
                -- neither the place nor what follows uses ends, after the
                -- value and before the write.
                let afterValue := retainLoansUsedBy ns future bound
                  (loansUsedByPlace ns place bound preserved)
                let afterValue := forgetMovedOutOf ns afterValue place
                let retirement := carrierRetirement? ns place value state valueFlow.valueLoans
                let writeDeaths := addCarrierRetirement (pushDeaths {}
                  (diffLoans (if ns.profile == some .move then bound else releasedValue) afterValue)
                  { anchor := value }) retirement
                { valueFlow with
                  normal := some afterValue
                  valueLoans := #[]
                  observed := appendUniqueLoans valueFlow.observed <|
                    recordBoundLoans afterValue valueFlow.valueLoans
                  deaths := appendUniqueDeaths (appendUniqueDeaths entryDeaths valueFlow.deaths)
                    writeDeaths
                  diagnostics := valueFlow.diagnostics ++
                    checkPlaceAccess ns (shallowWriteState ns afterValue place) expression.loc place true
                      "assignment" ++
                      (if retirement.isSome then #[] else
                        overwrittenLoanDiagnostics ns state released expression.loc ++
                        overwrittenLoanDiagnostics ns releasedValue bound expression.loc) ++
                      replacementEscapeDiagnostics ns releasedValue place valueFlow.valueLoans expression.loc ++
                      overwrittenLiveDiagnostics afterValue overwritten valueFlow.valueLoans
                        expression.loc }
        | .assignPattern pattern value =>
            let valueState := releaseHoldersUnusedBy ns state
              (patternVariables ns pattern) [value]
            let entryDeaths := pushDeaths {} (diffLoans state valueState)
              { anchor := value, before := true }
            let valueFlow := analyzeExpr unit ns value valueState preserved (future := future)
            match valueFlow.normal with
            | none => valueFlow
            | some afterValue =>
                -- A destructuring assignment writes each local it binds; any of
                -- them may be the element index of a live mutable loan.
                let indexDiagnostics := (patternVariables ns pattern).foldl (init := #[])
                  fun diagnostics localId =>
                    diagnostics ++ indexWriteDiagnostics ns afterValue expression.loc localId
                let replaced := replacePatternLoans unit ns afterValue valueFlow.valueLoans pattern value
                let replaced := (patternVariables ns pattern).foldl
                  (fun state holder => forgetMovedOut state { root := .local holder }) replaced
                let rebinding :=
                  overwrittenLoanDiagnostics ns state valueState expression.loc ++
                  overwrittenLoanDiagnostics ns afterValue replaced expression.loc
                let bindDeaths := pushDeaths {} (diffLoans afterValue replaced) { anchor := value }
                let afterValue := replaced
                let afterValue := discardUncarriedTemporaries afterValue
                  valueFlow.valueLoans #[]
                { valueFlow with
                  normal := some afterValue
                  valueLoans := #[]
                  observed := appendUniqueLoans valueFlow.observed <|
                    recordBoundLoans afterValue valueFlow.valueLoans
                  deaths := appendUniqueDeaths (appendUniqueDeaths entryDeaths valueFlow.deaths)
                    bindDeaths
                  diagnostics := valueFlow.diagnostics ++ rebinding ++ indexDiagnostics }

  /-- Operands evaluated in order. Before an operand, a loan held only by
  locals that neither it, a later operand, nor the continuation uses dies,
  unless the value of an earlier operand carries it (`carried`). -/
  private partial def analyzeExprList (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (expressions : List ExprId)
      (state : BorrowState) (preserved : Array LoanSource := #[]) (readOnly : Bool := false)
      (carried : Array LoanSource := #[]) (future : List ExprId := []) : BorrowFlow :=
    match expressions with
    | [] => { normal := some state }
    | expression :: tail =>
        let entry := state
        let state := retainLoansUsedBy ns (expressions ++ future) state (preserved ++ carried)
        let entryDeaths := pushDeaths {} (diffLoans entry state)
          { anchor := expression, before := true }
        let head := analyzeExpr unit ns expression state (preserved ++ carried) (readOnly := readOnly)
          (future := tail ++ future)
        match head.normal with
        | none => { head with deaths := appendUniqueDeaths entryDeaths head.deaths }
        | some afterHead =>
            let carried := head.valueLoans.foldl (init := carried) fun carried loan =>
              if carried.contains loan.source then carried else carried.push loan.source
            let rest := analyzeExprList unit ns tail afterHead preserved readOnly carried future
            { rest with
              breaks := head.breaks ++ rest.breaks
              breakValueLoans := head.breakValueLoans ++ rest.breakValueLoans
              continues := head.continues ++ rest.continues
              returns := head.returns ++ rest.returns
              valueLoans := if tail.isEmpty then head.valueLoans else rest.valueLoans
              observed := appendUniqueLoans head.observed rest.observed
              deaths := appendUniqueDeaths entryDeaths <|
                appendUniqueDeaths head.deaths rest.deaths
              diagnostics := head.diagnostics ++ rest.diagnostics }

  private partial def analyzeBlockExprList (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (expressions : List ExprId) (state : BorrowState)
      (preserved : Array LoanSource := #[]) (future : List ExprId := []) : BorrowFlow :=
    match expressions with
    | [] => { normal := some state }
    | expression :: tail =>
        let entry := state
        let state := retainLoansUsedBy ns (expressions ++ future) state preserved
        let entryDeaths := pushDeaths {} (diffLoans entry state)
          { anchor := expression, before := true }
        let head := analyzeExpr unit ns expression state preserved
          (future := tail ++ future)
        match head.normal with
        | none => { head with deaths := appendUniqueDeaths entryDeaths head.deaths }
        | some afterHead =>
            let discarded := if tail.isEmpty then afterHead else
              discardUncarriedTemporaries afterHead head.valueLoans #[]
            let discardDeaths := pushDeaths {} (diffLoans afterHead discarded)
              { anchor := expression }
            let rest := analyzeBlockExprList unit ns tail discarded preserved future
            { rest with
              breaks := head.breaks ++ rest.breaks
              breakValueLoans := head.breakValueLoans ++ rest.breakValueLoans
              continues := head.continues ++ rest.continues
              returns := head.returns ++ rest.returns
              valueLoans := if tail.isEmpty then head.valueLoans else rest.valueLoans
              observed := appendUniqueLoans head.observed rest.observed
              deaths := appendUniqueDeaths entryDeaths <|
                appendUniqueDeaths head.deaths <|
                  appendUniqueDeaths discardDeaths rest.deaths
              diagnostics := head.diagnostics ++ rest.diagnostics }

  private partial def analyzeArm (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (arm : MatchArm)
      (state : BorrowState) (preserved : Array LoanSource) (future guardFuture : List ExprId) :
      BorrowFlow :=
    match arm.guard with
    | none => analyzeExpr unit ns arm.body state preserved (future := future)
    | some guard =>
        let guardFlow := analyzeExpr unit ns guard state preserved
          (future := guardFuture)
        match guardFlow.normal with
        | none => guardFlow
        | some afterGuard =>
            let bodyState := retainLoansUsedBy ns (arm.body :: future) afterGuard preserved
            let entryDeaths := pushDeaths {} (diffLoans afterGuard bodyState)
              { anchor := arm.body, before := true }
            let bodyFlow := analyzeExpr unit ns arm.body bodyState preserved (future := future)
            { bodyFlow with
              breaks := guardFlow.breaks ++ bodyFlow.breaks
              breakValueLoans := guardFlow.breakValueLoans ++ bodyFlow.breakValueLoans
              continues := guardFlow.continues ++ bodyFlow.continues
              returns := guardFlow.returns ++ bodyFlow.returns
              observed := appendUniqueLoans guardFlow.observed bodyFlow.observed
              deaths := appendUniqueDeaths guardFlow.deaths <|
                appendUniqueDeaths entryDeaths bodyFlow.deaths
              diagnostics := guardFlow.diagnostics ++ bodyFlow.diagnostics }

  private partial def analyzeLoop (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (body : ExprId)
      (initial entry : BorrowState) (fuel : Nat)
      (preserved : Array LoanSource := #[]) (declared? : Option (Array LocalId) := none)
      (future : List ExprId := []) : BorrowFlow :=
    let declared := match declared? with
      | some declared => declared
      | none => bodyDeclaredLocals ns body
    -- The body runs again after itself.
    let bodyFlow := analyzeExpr unit ns body entry preserved (future := body :: future)
    let valueLoans := bodyFlow.breakValueLoans.foldl (init := #[]) fun loans pair =>
      if pair.1 == 0 then appendUniqueLoans loans pair.2 else loans
    let outerBreakValueLoans := bodyFlow.breakValueLoans.filterMap fun (nest, loans) =>
      match nest with | 0 => none | nest + 1 => some (nest, loans)
    -- A `continue` of this loop ends the loans its body's locals hold: the
    -- body declares them afresh before any use on re-entry.
    let bodyScoped (loan : LoanFact) : Bool :=
      !loan.callerHeld && !loan.holders.isEmpty && loan.holders.all declared.contains
    let continueDeaths := bodyFlow.continues.foldl (init := ({} : DeathLog)) fun deaths (nest, state, anchor) =>
      if nest == 0 then
        pushDeaths deaths (state.active.filter fun loan => loan.kind == .mutable && bodyScoped loan)
          { anchor, before := true }
      else deaths
    let reentries :=
      (match bodyFlow.normal with | some state => #[state] | none => #[]) ++
        (bodyFlow.continues.filterMap fun (nest, state, _) =>
          if nest == 0 then some { active := state.active.filter (!bodyScoped ·) } else none)
    -- Widen from the current entry rather than re-seeding from `initial`:
    -- the iterate then only grows, so the equality test converges within the
    -- finite loan lattice instead of oscillating through loan kills until the
    -- fuel is exhausted on every (possibly nested) loop.
    let nextEntry := reentries.foldl mergeState entry
    if fuel > 0 && nextEntry != entry then
      let next := analyzeLoop unit ns body initial nextEntry (fuel - 1) preserved (some declared)
        future
      { next with
        valueLoans := appendUniqueLoans valueLoans next.valueLoans
        breakValueLoans := outerBreakValueLoans ++ next.breakValueLoans
        returns := bodyFlow.returns ++ next.returns
        observed := appendUniqueLoans bodyFlow.observed next.observed
        deaths := appendUniqueDeaths (appendUniqueDeaths bodyFlow.deaths continueDeaths) next.deaths
        diagnostics := bodyFlow.diagnostics ++ next.diagnostics }
    else
      let exits := bodyFlow.breaks.filterMap fun (nest, state) =>
        if nest == 0 then some state else none
      let normal := exits.foldl (fun result state => mergeNormal result (some state)) none
      let outerBreaks := bodyFlow.breaks.filterMap fun (nest, state) =>
        match nest with | 0 => none | nest + 1 => some (nest, state)
      let outerContinues := bodyFlow.continues.filterMap fun (nest, state, anchor) =>
        match nest with | 0 => none | nest + 1 => some (nest, state, anchor)
      { normal, breaks := outerBreaks, breakValueLoans := outerBreakValueLoans
        continues := outerContinues, valueLoans
        returns := bodyFlow.returns
        observed := bodyFlow.observed, deaths := appendUniqueDeaths bodyFlow.deaths continueDeaths
        diagnostics := bodyFlow.diagnostics }
end

private def referenceParameters (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Array ReferenceParameterFact :=
  function.locals.take function.signature.parameters.size |>.filterMap fun localDecl => do
    let reference ← referenceType? ns localDecl.type.typeId
    some { localId := localDecl.id, kind := reference.kind, lifetime := reference.lifetime }

/-- The references a value of a type holds by value, with their paths: not
behind a reference, and a nominal's back edge cut. -/
private partial def heldReferences (unit : ValidatedUnit) (ns : ValidatedNamespace) :
    Nat → TypeId → List (Array GenericArgument) → Array LoanProjection → Array NameId →
      Array (Array LoanProjection × ReferenceType)
  | 0, _, _, _, _ => #[]
  | fuel + 1, typeId, contexts, path, visiting =>
      match ns.tables.types[typeId.index]? with
      | some (.reference reference) =>
          #[(path, { reference with
            lifetime := instantiatedLifetimeInContexts ns contexts reference.lifetime })]
      | some (.tuple elements) =>
          elements.zipIdx.flatMap fun (element, index) =>
            heldReferences unit ns fuel element contexts (path.push (.index (some index))) visiting
      | some (.vector element _) | some (.slice element) =>
          heldReferences unit ns fuel element contexts (path.push (.index none)) visiting
      | some (.typeParameter index) => match contexts with
          | arguments :: outer => match arguments[index]? with
              | some (.typeArg use) => heldReferences unit ns fuel use.typeId outer path visiting
              | _ => #[]
          | [] => #[]
      | some (.nominal name arguments) =>
          if visiting.contains name then #[] else
          match nominalAt? unit name with
          | none => #[]
          | some (_, declaration) =>
              let inner := visiting.push name
              let fieldsAt (start : Array LoanProjection) (fields : Array FieldDecl) :=
                fields.flatMap fun field =>
                  heldReferences unit ns fuel field.type.typeId (arguments :: contexts)
                    (start.push (.field field.name)) inner
              if declaration.variants.isEmpty then fieldsAt path declaration.fields
              else declaration.variants.flatMap fun variant =>
                fieldsAt (path.push (.downcast variant.name)) variant.fields
      | some (.profile _) | some (.function ..) | some (.typeDomain _)
      | some (.resourceDomain ..) | some .unit | some .never | some .bool | some .character
      | some .string | some .bytes | some .address | some .signer | some (.integer ..)
      | some .range | some .eventStore | some .stateDomain | none => #[]

private def appendUniqueRelation (relations : Array LifetimeRelationFact)
    (relation : LifetimeRelationFact) : Array LifetimeRelationFact :=
  if relations.contains relation then relations else relations.push relation

private def declaredLifetimeRelations
    (function : FunctionDecl FunctionBody) : Array LifetimeRelationFact :=
  let predicates := function.signature.generics.foldl
    (fun predicates binder => predicates ++ binder.predicates) function.signature.predicates
  predicates.foldl (init := #[]) fun relations predicate =>
    match predicate with
    | .lifetimeOutlives longer shorter =>
        appendUniqueRelation relations { longer, shorter }
    | _ => relations

private def reborrowLifetimeRelations (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) (loans : Array LoanFact) : Array LifetimeRelationFact :=
  loans.foldl (init := #[]) fun relations loan =>
    match loan.place.projections[0]? with
    | some .dereference =>
        match loan.place.root.local?.bind (fun localId => function.locals[localId.index]?) with
        | some localDecl => match ns.tables.types[localDecl.type.typeId.index]? with
            | some (.reference parent) => appendUniqueRelation relations {
                longer := parent.lifetime, shorter := loan.lifetime }
            | _ => relations
        | none => relations
    | _ => relations

private def appendRelations (left right : Array LifetimeRelationFact) :
    Array LifetimeRelationFact :=
  right.foldl appendUniqueRelation left

private inductive LifetimeVariance where
  | covariant
  | contravariant
  | invariant

private def composeLifetimeVariance : LifetimeVariance → LifetimeVariance → LifetimeVariance
  | .invariant, _ | _, .invariant => .invariant
  | .covariant, inner => inner
  | .contravariant, .covariant => .contravariant
  | .contravariant, .contravariant => .covariant

private def lifetimeVarianceRelations (variance : LifetimeVariance)
    (source target : LifetimeId) : Array LifetimeRelationFact :=
  match variance with
  | .covariant => #[{ longer := source, shorter := target }]
  | .contravariant => #[{ longer := target, shorter := source }]
  | .invariant => #[
      { longer := source, shorter := target },
      { longer := target, shorter := source }]

/-- Generate the lifetime constraints induced by one structural type use.
Tuples and vectors are covariant, function arguments are contravariant, and a
mutable-reference referent is invariant. Nominal variance remains deferred
until declarations retain authoritative frontend variance information. -/
private partial def structuralLifetimeRelationsFuel (fuel : Nat)
    (ns : ValidatedNamespace) (source : TypeId)
    (sourceInstantiations : Array GenericArgument) (target : TypeId)
    (targetInstantiations : Array GenericArgument) (variance : LifetimeVariance) :
    Array LifetimeRelationFact :=
  match fuel, ns.tables.types[source.index]?, ns.tables.types[target.index]? with
  | 0, _, _ | _, none, _ | _, _, none => #[]
  | fuel + 1, some (.typeParameter index), _ =>
      match sourceInstantiations[index]? with
      | some (.typeArg instantiated) =>
          structuralLifetimeRelationsFuel fuel ns instantiated.typeId #[]
            target targetInstantiations variance
      | _ => #[]
  | fuel + 1, _, some (.typeParameter index) =>
      match targetInstantiations[index]? with
      | some (.typeArg instantiated) =>
          structuralLifetimeRelationsFuel fuel ns source sourceInstantiations
            instantiated.typeId #[] variance
      | _ => #[]
  | fuel + 1, some (.reference sourceReference), some (.reference targetReference) =>
      if sourceReference.profile != targetReference.profile ||
          sourceReference.kind != targetReference.kind then #[] else
        let sourceLifetime := instantiatedLifetime ns sourceInstantiations
          sourceReference.lifetime
        let targetLifetime := instantiatedLifetime ns targetInstantiations
          targetReference.lifetime
        let relations := lifetimeVarianceRelations variance sourceLifetime targetLifetime
        let referentVariance := match sourceReference.kind with
          | .shared => .covariant
          | .mutable => .invariant
        appendRelations relations <| structuralLifetimeRelationsFuel fuel ns
          sourceReference.referent sourceInstantiations targetReference.referent
          targetInstantiations (composeLifetimeVariance variance referentVariance)
  | fuel + 1, some (.tuple sourceElements), some (.tuple targetElements) =>
      if sourceElements.size != targetElements.size then #[] else
        (sourceElements.zip targetElements).foldl (init := #[]) fun relations pair =>
          appendRelations relations <| structuralLifetimeRelationsFuel fuel ns pair.1
            sourceInstantiations pair.2 targetInstantiations variance
  | fuel + 1, some (.vector sourceElement sourceLength),
      some (.vector targetElement targetLength) =>
      if sourceLength != targetLength then #[] else
        structuralLifetimeRelationsFuel fuel ns sourceElement sourceInstantiations
          targetElement targetInstantiations variance
  | fuel + 1, some (.slice sourceElement), some (.slice targetElement) =>
      structuralLifetimeRelationsFuel fuel ns sourceElement sourceInstantiations
        targetElement targetInstantiations variance
  | fuel + 1, some (.function sourceArguments sourceResult _),
      some (.function targetArguments targetResult _) =>
      if sourceArguments.size != targetArguments.size then #[] else
        let argumentVariance := composeLifetimeVariance variance .contravariant
        let argumentRelations := (sourceArguments.zip targetArguments).foldl
          (init := #[]) fun relations pair =>
            appendRelations relations <| structuralLifetimeRelationsFuel fuel ns pair.1
              sourceInstantiations pair.2 targetInstantiations argumentVariance
        appendRelations argumentRelations <| structuralLifetimeRelationsFuel fuel ns
          sourceResult sourceInstantiations targetResult targetInstantiations variance
  | _, _, _ => #[]

private def structuralLifetimeRelations (ns : ValidatedNamespace) (source : TypeId)
    (sourceInstantiations : Array GenericArgument) (target : TypeId)
    (targetInstantiations : Array GenericArgument) : Array LifetimeRelationFact :=
  structuralLifetimeRelationsFuel (ns.tables.types.size * 2 + 1) ns source
    sourceInstantiations target targetInstantiations .covariant

private def directCallLifetimeRelations (ns : ValidatedNamespace) (resultType : TypeId)
    (reference : QualifiedRef) (instantiations : Array GenericArgument)
    (arguments : Array ExprId) : Array LifetimeRelationFact :=
  if reference.namespaceId != ns.identity then #[] else
    match ns.functions.find? (·.name == reference.name) with
    | none => #[]
    | some declaration =>
        let argumentRelations := (arguments.zip declaration.signature.parameters).foldl
          (init := #[]) fun relations pair =>
            match ns.expressions[pair.1.index]? with
            | some argument => appendRelations relations <|
                structuralLifetimeRelations ns argument.typeId #[] pair.2.typeUse.typeId
                  instantiations
            | none => relations
        let resultRelations := match declaration.signature.results.toList with
          | [result] => structuralLifetimeRelations ns result.typeId instantiations
              resultType #[]
          | _ => match ns.tables.types[resultType.index]? with
              | some (.tuple elements) =>
                  (elements.zip declaration.signature.results).foldl (init := #[])
                    fun relations pair => appendRelations relations <|
                      structuralLifetimeRelations ns pair.2.typeId instantiations
                        pair.1 #[]
              | _ => #[]
        appendRelations argumentRelations resultRelations

private def freezeLifetimeRelations (ns : ValidatedNamespace) (resultType : TypeId)
    (arguments : Array ExprId) : Array LifetimeRelationFact :=
  match arguments.toList with
  | [argument] => match ns.expressions[argument.index]?, ns.tables.types[resultType.index]? with
      | some expression, some (.reference result) =>
          match ns.tables.types[expression.typeId.index]? with
          | some (.reference source) =>
              #[{ longer := source.lifetime, shorter := result.lifetime }]
          | _ => #[]
      | _, _ => #[]
  | _ => #[]

private partial def operationLifetimeRelationsAt (ns : ValidatedNamespace) (exprId : ExprId)
    (fuel : Nat) : Array LifetimeRelationFact :=
  match fuel, ns.expressions[exprId.index]? with
  | 0, _ | _, none => #[]
  | fuel + 1, some expression =>
      let children (ids : Array ExprId) := ids.foldl (fun relations child =>
        appendRelations relations (operationLifetimeRelationsAt ns child fuel)) #[]
      match expression.kind with
      | .operation operation instantiations arguments _ =>
          let nested := children arguments
          match operation with
          | .call (.function reference) => appendRelations nested <|
              directCallLifetimeRelations ns expression.typeId reference instantiations arguments
          | .reference (.freeze _) => appendRelations nested <|
              freezeLifetimeRelations ns expression.typeId arguments
          | _ => nested
      | .block statements result => children (statements ++ result.toArray)
      | .letDecl _ initializer body => children (initializer.toArray.push body)
      | .ifElse condition thenBranch elseBranch =>
          children (#[condition, thenBranch] ++ elseBranch.toArray)
      | .match_ scrutinee arms =>
          arms.foldl (fun relations arm =>
            appendRelations relations <| children (arm.guard.toArray.push arm.body))
            (operationLifetimeRelationsAt ns scrutinee fuel)
      | .loop _ body => operationLifetimeRelationsAt ns body fuel
      | .break_ _ value => children value.toArray
      | .return_ values | .throw_ _ values => children values
      | .assign _ value | .assignPattern _ value => operationLifetimeRelationsAt ns value fuel
      | .quantifier _ binders triggers condition body =>
          let domains := binders.map (·.domain)
          let triggers := triggers.foldl (· ++ ·) #[]
          children (domains ++ triggers ++ condition.toArray.push body)
      | .value .. | .constant _ | .localVar _ | .continue_ _ | .spec _ => #[]

private def operationLifetimeRelations (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Array LifetimeRelationFact :=
  match function.body with
  | .absent => #[]
  | .structured root => operationLifetimeRelationsAt ns root (ns.expressions.size + 1)

private def valueResultLifetimeRelations (ns : ValidatedNamespace)
    (values : Array ExprId) (results : Array TypeUse) : Array LifetimeRelationFact :=
  (values.zip results).foldl (init := #[]) fun relations pair =>
    match ns.expressions[pair.1.index]? with
    | some value => appendRelations relations <|
        structuralLifetimeRelations ns value.typeId #[] pair.2.typeId #[]
    | none => relations

private partial def returnLifetimeRelationsAt (ns : ValidatedNamespace)
    (results : Array TypeUse) (exprId : ExprId) (fuel : Nat) : Array LifetimeRelationFact :=
  match fuel, ns.expressions[exprId.index]? with
  | 0, _ | _, none => #[]
  | fuel + 1, some expression =>
      let children (ids : Array ExprId) := ids.foldl (fun relations child =>
        appendRelations relations (returnLifetimeRelationsAt ns results child fuel)) #[]
      match expression.kind with
      | .return_ values => appendRelations (children values) <|
          valueResultLifetimeRelations ns values results
      | .operation _ _ arguments _ => children arguments
      | .block statements result => children (statements ++ result.toArray)
      | .letDecl _ initializer body => children (initializer.toArray.push body)
      | .ifElse condition thenBranch elseBranch =>
          children (#[condition, thenBranch] ++ elseBranch.toArray)
      | .match_ scrutinee arms =>
          arms.foldl (fun relations arm =>
            appendRelations relations <| children (arm.guard.toArray.push arm.body))
            (returnLifetimeRelationsAt ns results scrutinee fuel)
      | .loop _ body => returnLifetimeRelationsAt ns results body fuel
      | .break_ _ value => children value.toArray
      | .throw_ _ values => children values
      | .assign _ value | .assignPattern _ value =>
          returnLifetimeRelationsAt ns results value fuel
      | .quantifier _ binders triggers condition body =>
          let domains := binders.map (·.domain)
          let triggers := triggers.foldl (· ++ ·) #[]
          children (domains ++ triggers ++ condition.toArray.push body)
      | .value .. | .constant _ | .localVar _ | .continue_ _ | .spec _ => #[]

private def functionBoundaryLifetimeRelations (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Array LifetimeRelationFact :=
  match function.body with
  | .absent => #[]
  | .structured root =>
      let explicit := returnLifetimeRelationsAt ns function.signature.results root
        (ns.expressions.size + 1)
      let normal := match function.signature.results.toList with
        | [result] => valueResultLifetimeRelations ns #[root] #[result]
        | _ => #[]
      appendRelations explicit normal

private partial def closeLifetimeRelations (relations : Array LifetimeRelationFact)
    (fuel : Nat) : Array LifetimeRelationFact :=
  let closed := relations.foldl (init := relations) fun closed left =>
    relations.foldl (init := closed) fun closed right =>
      if left.shorter == right.longer then
        appendUniqueRelation closed { longer := left.longer, shorter := right.shorter }
      else closed
  if fuel > 0 && closed.size != relations.size then
    closeLifetimeRelations closed (fuel - 1)
  else closed

private def pushLifetime (found : Array LifetimeId) (lifetime : LifetimeId) : Array LifetimeId :=
  if found.contains lifetime then found else found.push lifetime

/-- The lifetimes a type mentions. -/
private partial def typeLifetimes (ns : ValidatedNamespace) (found : Array LifetimeId)
    (typeId : TypeId) : Array LifetimeId :=
  match ns.tables.types[typeId.index]? with
  | some (.reference value) => typeLifetimes ns (pushLifetime found value.lifetime) value.referent
  | some (.tuple elements) => elements.foldl (typeLifetimes ns) found
  | some (.vector element _) | some (.slice element) | some (.typeDomain element) => typeLifetimes ns found element
  | some (.function arguments result _) =>
      typeLifetimes ns (arguments.foldl (typeLifetimes ns) found) result
  | some (.nominal _ arguments) => arguments.foldl (init := found) fun found => fun
      | .typeArg use => typeLifetimes ns found use.typeId
      | .lifetime lifetime => pushLifetime found lifetime
      | _ => found
  | some (.resourceDomain _ (some arguments)) => arguments.foldl (typeLifetimes ns) found
  | _ => found

/-- The lifetimes a function mentions: in its signature, its locals, and the
types of its body. The unit's table holds every function's lifetimes, so
relating only these keeps each certificate proportional to its function. -/
private def functionLifetimes (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Array LifetimeId := Id.run do
  let mut typeIds : Std.HashSet TypeId := {}
  for parameter in function.signature.parameters do typeIds := typeIds.insert parameter.typeUse.typeId
  for result in function.signature.results do typeIds := typeIds.insert result.typeId
  for localDecl in function.locals do typeIds := typeIds.insert localDecl.type.typeId
  if let .structured root := function.body then
    let mut pending := #[root]
    let mut visited : Std.HashSet ExprId := {}
    while let some id := pending.back? do
      pending := pending.pop
      if visited.contains id then continue
      visited := visited.insert id
      let some expression := ns.expressions[id.index]? | continue
      typeIds := typeIds.insert expression.typeId
      pending := pending ++ expressionChildren expression.kind
  let mut found := function.signature.predicates.foldl (init := #[]) fun found => fun
    | .lifetimeOutlives longer shorter => pushLifetime (pushLifetime found longer) shorter
    | _ => found
  for typeId in typeIds do found := typeLifetimes ns found typeId
  return found

private def solvedLifetimeRelations (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) (loans : Array LoanFact) : Array LifetimeRelationFact :=
  let derived := declaredLifetimeRelations function ++
    reborrowLifetimeRelations ns function loans ++
    operationLifetimeRelations ns function ++
    functionBoundaryLifetimeRelations ns function
  let lifetimes := derived.foldl (init := functionLifetimes ns function) fun found relation =>
    pushLifetime (pushLifetime found relation.longer) relation.shorter
  let reflexive := lifetimes.map fun lifetime => { longer := lifetime, shorter := lifetime }
  let withStatic := lifetimes.foldl (init := reflexive) fun relations longer =>
    match ns.tables.lifetimes[longer.index]? with
    | some { kind := .static, .. } => lifetimes.foldl (init := relations) fun relations shorter =>
        appendUniqueRelation relations { longer, shorter }
    | _ => relations
  closeLifetimeRelations (derived.foldl appendUniqueRelation withStatic) (lifetimes.size + 1)

/-- What a function's value lends its caller must be what its signature says: a
caller reads from the declared regions which loans a result holds, so holding one
of a parameter whose regions do not outlive the result's would free the lender
while it is borrowed. A reborrow carries the loans of the reference it goes through. -/
private def resultRegionDiagnostics (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) (value : ExprId) (carried : Array LoanFact) :
    Array Diagnostic :=
  if ns.profile == some .move then #[] else
  match function.signature.results.toList, callResultPositions? unit ns function.signature none with
  | [_], some positions =>
      let edges := signatureRegionEdges ns function.signature
      carried.foldl (init := #[]) fun diagnostics loan =>
        match loan.source with
        | .expression _ => diagnostics
        | .parameter localId path =>
            let sources := match function.locals[localId.index]? with
              | some localDecl => regionsAtPath unit ns none false [] none
                  (ns.tables.types.size + 1) localDecl.type.typeId path.toList
              | none => #[.unknown]
            let allowed := (loanPathsInExpression unit ns loan value (ns.expressions.size + 1)).all
              fun held => positions.all fun (position, target) =>
                !projectionsOverlap held.toList position.toList ||
                  sources.any (regionsRelated edges · target)
            if allowed then diagnostics else diagnostics.push (.at "VIR-SEMANTIC-BORROW-RESULT-REGION"
              "a result holds a loan of a parameter whose declared lifetime is not known to outlive the result's: Rustc Public exposes no outlives bound, and this signature implies none"
              (ns.expressions[value.index]?.elim function.loc (·.loc)))
  | _, _ => #[]

private def borrowAnalysis (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : BorrowFlow :=
  match function.body with
  | .absent => {}
  | .structured root =>
      let parameters := referenceParameters ns function
      let incoming := parameters.map fun parameter => ({
        source := .parameter parameter.localId
        kind := parameter.kind
        place := { root := .external }
        lifetime := parameter.lifetime
        loc := function.loc
        holders := #[parameter.localId]
        holderPlaces := #[{ root := .local parameter.localId }]
        canEscape := true } : LoanFact)
      -- A reference a parameter holds by value is lent to the function too.
      let held := if ns.profile == some .move then #[] else
        (function.locals.take function.signature.parameters.size).flatMap fun localDecl =>
          if (referenceType? ns localDecl.type.typeId).isSome then #[] else
          (heldReferences unit ns (ns.tables.types.size + 1) localDecl.type.typeId [] #[] #[]).map
            fun (path, reference) => ({
              source := .parameter localDecl.id path
              kind := reference.kind
              place := { root := .external }
              lifetime := reference.lifetime
              loc := function.loc
              holders := #[localDecl.id]
              holderPlaces := #[{ root := .local localDecl.id, projections := path }]
              canEscape := true } : LoanFact)
      let flow := analyzeExpr unit ns root { active := incoming ++ held }
      let regionDiagnostics := flow.returns.foldl (init :=
          if flow.normal.isSome then resultRegionDiagnostics unit ns function root flow.valueLoans
          else #[]) fun diagnostics state =>
        match state.returned with
        | some (value, carried) => diagnostics ++ resultRegionDiagnostics unit ns function value carried
        | none => diagnostics
      let flow := { flow with diagnostics := flow.diagnostics ++ regionDiagnostics }
      -- A normally returned value is the last possible carrier. Mutable
      -- loans absent from that value die after the root evaluates; recording
      -- that point makes function-exit reconciliation explicit rather than
      -- leaving finalization to rediscover a live holder in the frame.
      let flow := match flow.normal with
        | none => flow
        | some exit =>
            let returned := retainResultLoans exit flow.valueLoans
            let died := diffLoans exit returned
            { flow with
              normal := some returned
              deaths := pushDeaths flow.deaths died { anchor := root } }
      -- Provenance was established before aliases and parent holders could
      -- be released. Absence from the final holder set cannot establish it.
      let frameOwned (loan : LoanFact) : Bool := !loan.canEscape
      let normalEscapes := match flow.normal with
        | some state => state.active.any frameOwned
        | none => false
      let returnEscapes := flow.returns.any fun state =>
        state.active.any frameOwned
      if (!normalEscapes && !returnEscapes) ||
          !function.signature.results.any (fun result =>
            typeMayHoldFrameLoan unit ns result.typeId) then flow
      else
        let diagnostic := Diagnostic.at "VIR-SEMANTIC-BORROW-ESCAPE"
          "a local or Move global loan may escape through a reference-bearing function result" function.loc
        { flow with diagnostics := flow.diagnostics.push diagnostic }

/-- Whether a path from each node of a body reads a carrier first, before and after
the node. An independent computation over the paths of the body: what follows a node
is passed in, a jump reaches its target's, nothing follows a return or a throw, and
the first access on a path decides: a read reports it, an overwrite ends the path. -/
private structure FirstAccess where
  before : Array Bool
  after : Array Bool
  deriving Inhabited

private def FirstAccess.ofSize (size : Nat) : FirstAccess :=
  { before := Array.replicate size false, after := Array.replicate size false }

/-- A carrier's first access in the rest of a path from a node, `after` being that of
what follows it, with `loops` what a `continue` and a `break` of each enclosing loop
reach, innermost first. -/
private partial def readFirst (ns : ValidatedNamespace) (carrier : LoanPlace) (exprId : ExprId)
    (after : Bool) (loops : List (Bool × Bool)) : StateM FirstAccess Bool := do
  modify fun found => { found with after := found.after.setIfInBounds exprId.index after }
  let sequence (nodes : List ExprId) (after : Bool) : StateM FirstAccess Bool :=
    nodes.foldrM (fun node after => readFirst ns carrier node after loops) after
  let binds (pattern : PatternId) : Bool :=
    carrier.root.local?.any (patternVariables ns pattern).contains
  let reads := fun (place : PlaceId) (after : Bool) =>
    placeUsesCarrier ns place carrier || after
  let overwrites := fun (place : PlaceId) (after : Bool) =>
    if (loanPlace? ns place).any (placeCovers · carrier) then false else reads place after
  let result ← match ns.expressions[exprId.index]? with
    | none => pure true
    | some expression => match expression.kind with
      | .value .. | .constant _ => pure after
      | .localVar _ | .spec _ | .quantifier .. =>
          pure (expressionUsesCarrier ns exprId carrier (ns.expressions.size + 1) || after)
      | .operation operation _ arguments _ =>
          sequence arguments.toList <| match operation with
            | .write place | .drop place => overwrites place after
            | .move place | .copy place | .borrow _ place | .read place | .readDiscriminant place => reads place after
            | _ => after
      | .block statements result => sequence (statements.toList ++ result.toList) after
      | .letDecl pattern initializer body => do
          let inBody ← readFirst ns carrier body after loops
          let afterValue := !binds pattern && inBody
          initializer.elim (pure afterValue) fun value =>
            readFirst ns carrier value afterValue loops
      | .ifElse condition thenBranch elseBranch => do
          let onThen ← readFirst ns carrier thenBranch after loops
          let onElse ← elseBranch.elim (pure after) fun branch =>
            readFirst ns carrier branch after loops
          readFirst ns carrier condition (onThen || onElse) loops
      | .match_ scrutinee arms => do
          -- A failing guard reaches the arms after its own.
          let reached ← arms.toList.foldrM (init := false) fun arm later => do
            let onBody ← readFirst ns carrier arm.body after loops
            let onBody := !binds arm.pattern && onBody
            let entered ← arm.guard.elim (pure onBody) fun guard =>
              readFirst ns carrier guard (onBody || later) loops
            pure (entered || later)
          readFirst ns carrier scrutinee reached loops
      | .loop _ body =>
          let rec iterate (head : Bool) (fuel : Nat) : StateM FirstAccess Bool := do
            let next ← readFirst ns carrier body head ((head, after) :: loops)
            if next == head || fuel == 0 then pure next else iterate next (fuel - 1)
          iterate false 2
      | .continue_ nest => pure ((loops[nest]?.map (·.1)).getD true)
      | .break_ nest value =>
          let target := (loops[nest]?.map (·.2)).getD true
          value.elim (pure target) fun value => readFirst ns carrier value target loops
      | .return_ values | .throw_ _ values => sequence values.toList false
      | .assign place value => readFirst ns carrier value (overwrites place after) loops
      | .assignPattern pattern value =>
          readFirst ns carrier value (!binds pattern && after) loops
  modify fun found => { found with before := found.before.setIfInBounds exprId.index result }
  pure result

/-- The carriers of a loan's holders. -/
private def holderCarriers (holders : Array LocalId) (places : Array LoanPlace) : Array LoanPlace :=
  holders.flatMap fun holder =>
    let carriers := places.filter (·.root == .local holder)
    if carriers.isEmpty then #[{ root := .local holder }] else carriers

/-- Whether a carrier is read first on some path from a death's anchor. -/
private def carrierReadAt (ns : ValidatedNamespace) (root : ExprId) (death : LoanDeath)
    (carrier : LoanPlace) : Bool :=
  let (_, found) := (readFirst ns carrier root false []).run (.ofSize ns.expressions.size)
  (if death.before then found.before else found.after)[death.anchor.index]?.getD true

private def recordedPlaces (death : LoanDeath) : Array LoanPlace :=
  death.holderPaths.map fun (holder, projections) => { root := .local holder, projections }

/-- Whether a holder a death records is read from its anchor on. -/
def loanDeathReadsHolder (ns : ValidatedNamespace) (root : ExprId) (death : LoanDeath) : Bool :=
  (holderCarriers death.holders (recordedPlaces death)).any (carrierReadAt ns root death)

/-- An internal check that fails closed. A death assumes each holder's current
value is final, so every holder of the loan in the merged state at the anchor must
be dead from there on: an equation over a place still read is broken by the next
write through it, and the path turns vacuous. -/
private def loanDeathLivenessDiagnostics (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) (root : ExprId) (log : DeathLog) : Array Diagnostic :=
  let refused := log.deaths.find? fun (source, death) =>
    let live := fun (holders : Array LocalId) (places : Array LoanPlace) =>
      (holderCarriers holders places).any (carrierReadAt ns root death)
    live death.holders (recordedPlaces death) || !death.alone && log.points.any fun point =>
      point.anchor == death.anchor && point.before == death.before &&
        point.loan.source == .expression source &&
          live point.loan.holders point.loan.holderPlaces
  match refused with
  | some (_, death) =>
    let name := ((unit.tables.names[function.name.index]?).map (·.name)).getD "?"
    let loc := ((ns.expressions[death.anchor.index]?).map (·.loc)).getD function.loc
    #[Diagnostic.at "VIR-SEMANTIC-LOAN-DEATH"
      s!"internal: in `{name}`, a loan dies where a place holding it is still read" loc]
  | none => #[]

/-- Whether evaluating a node may leave it other than by finishing or a panic. -/
private partial def mayExit (ns : ValidatedNamespace) (exprId : ExprId) (depth : Nat := 0) : Bool :=
  match ns.expressions[exprId.index]? with
  | none => true
  | some expression => match expression.kind with
    | .return_ _ | .throw_ .. => true
    | .continue_ nest => nest >= depth
    | .break_ nest value => nest >= depth || value.any (mayExit ns · depth)
    | .loop _ body => mayExit ns body (depth + 1)
    | .spec _ | .quantifier .. => false
    | kind => (expressionChildren kind).any (mayExit ns · depth)

/-- What runs after a node within its parents. -/
private inductive FollowingStep where
  | later (nodes : List ExprId)
  | writes (place : PlaceId)
  /-- A join, a repeated body or an exit. -/
  | boundary

/-- Whether a node overwrites a carrier (`some true`), or may leave, or uses a place
overlapping it, whole holder included, first (`some false`). -/
private partial def overwriteScan (ns : ValidatedNamespace) (carrier : LoanPlace) (exprId : ExprId) :
    Option Bool :=
  let uses := fun (node : ExprId) =>
    expressionUsesCarrier ns node carrier (ns.expressions.size + 1)
  let leaves := fun (node : ExprId) => mayExit ns node
  let sequence := fun (nodes : List ExprId) =>
    nodes.foldl (init := (none : Option Bool)) fun fate node =>
      fate.orElse fun _ => overwriteScan ns carrier node
  let write := fun (place : PlaceId) (value : ExprId) =>
    if uses value || leaves value then some false
    else if (loanPlace? ns place).any (placeCovers · carrier) then some true
    else if placeUsesCarrier ns place carrier then some false
    else none
  match ns.expressions[exprId.index]? with
  | none => some false
  | some expression => match expression.kind with
    | .assign place value => write place value
    | .operation (.write place) _ #[value] _ => write place value
    | .block statements result => sequence (statements.toList ++ result.toList)
    | .letDecl _ initializer body => sequence (initializer.toList ++ [body])
    | _ => if uses exprId || leaves exprId then some false else none

/-- Whether an overwrite of a carrier follows on the path from a node, before any
join, exit or use of a place overlapping it. -/
private partial def overwriteFollows (ns : ValidatedNamespace) (carrier : LoanPlace) :
    List FollowingStep → Bool
  | [] | .boundary :: _ => false
  | .writes place :: rest =>
      if (loanPlace? ns place).any (placeCovers · carrier) then true
      else if placeUsesCarrier ns place carrier then false
      else overwriteFollows ns carrier rest
  | .later nodes :: rest =>
      match nodes.foldl (init := (none : Option Bool)) fun fate node =>
          fate.orElse fun _ => overwriteScan ns carrier node with
      | some fate => fate
      | none => overwriteFollows ns carrier rest

/-- The moves of an owned component an overwrite follows, from a node of a body. -/
private partial def collectTransfers (ns : ValidatedNamespace) (exprId : ExprId)
    (following : List FollowingStep) (found : Array ExprId) : Array ExprId :=
  let inOrder := fun (nodes : List ExprId) (found : Array ExprId) =>
    let rec go : List ExprId → Array ExprId → Array ExprId
      | [], found => found
      | head :: tail, found => go tail (collectTransfers ns head (.later tail :: following) found)
    go nodes found
  let apart := fun (nodes : List ExprId) (found : Array ExprId) =>
    nodes.foldl (fun found node => collectTransfers ns node [.boundary] found) found
  match ns.expressions[exprId.index]? with
  | none => found
  | some expression => match expression.kind with
    | .operation (.move place) _ _ _ =>
        match loanPlace? ns place with
        | some carrier =>
            if !carrier.projections.isEmpty && isOwnedCarrierPlace carrier &&
                overwriteFollows ns carrier following then found.push exprId
            else found
        | none => found
    | .operation (.write place) _ #[value] _ | .assign place value =>
        collectTransfers ns value (.writes place :: following) found
    | .operation _ _ arguments _ => inOrder arguments.toList found
    | .block statements result => inOrder (statements.toList ++ result.toList) found
    | .letDecl _ initializer body =>
        let found := initializer.elim found fun value =>
          collectTransfers ns value (.later [body] :: following) found
        collectTransfers ns body following found
    | .assignPattern _ value => apart [value] found
    | .ifElse condition thenBranch elseBranch =>
        apart (condition :: thenBranch :: elseBranch.toList) found
    | .match_ scrutinee arms =>
        apart (scrutinee :: arms.toList.flatMap fun arm => arm.guard.toList ++ [arm.body]) found
    | .loop _ body => apart [body] found
    | .break_ _ value => apart value.toList found
    | .return_ values | .throw_ _ values => apart values.toList found
    | _ => found

/-- The moves of an owned component a namespace's bodies transfer: an overwrite of
the component follows on the path, before any join, exit or use of a place
overlapping it. Everything else reads a moved reference as a reborrow. -/
def transferredMoves (ns : ValidatedNamespace) : Array ExprId :=
  ns.functions.foldl (init := #[]) fun found function =>
    match function.body with
    | .structured root => collectTransfers ns root [] found
    | .absent => found

/-- Run the borrow analysis once for one function: conservative conflicts and
unsupported local-loan coverage diagnostics, and the proof-facing local-loan
receipt exactly when the analysis accepts the function. -/
def borrowOutcome (namespaceId : NamespaceId) (functionId : FunctionId)
    (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) (receiverNoEscape : Array Nat := #[]) :
    Array Diagnostic × Option BorrowCertificate :=
  match function.body with
  | .absent => (#[], none)
  | .structured root =>
      let analysis := borrowAnalysis unit ns function
      let diagnostics := analysis.diagnostics ++
        loanDeathLivenessDiagnostics unit ns function root analysis.deaths ++
        (if analysis.deaths.carrierDeaths.any (loanDeathReadsHolder ns root) then
          #[.at "VIR-SEMANTIC-LOAN-DEATH"
            "an overwritten reference carrier is still read before replacement" function.loc]
         else #[])
      -- A write to an element index under a live loan refuses this function
      -- only: it is read out of the diagnostics into the certificate.
      let indexWrites := diagnostics.filter (·.code == indexWriteUnderLoanCode)
      let others := diagnostics.filter (·.code != indexWriteUnderLoanCode)
      if others.isEmpty then
        let relations := solvedLifetimeRelations ns function analysis.observed
        (#[], some {
          writesIndexUnderLoan := !indexWrites.isEmpty
          namespaceId
          functionId
          root
          parameters := referenceParameters ns function
          loans := analysis.observed.filterMap fun loan => do
            let expression ← loan.source.expression?
            some {
              expression, lifetime := loan.lifetime
              deaths := analysis.deaths.deaths.filterMap fun (loanExpression, death) =>
                if loanExpression == expression then
                  -- A reference holds its loan itself.
                  some { death with holderPaths := death.holderPaths.filter fun (holder, _) =>
                    (function.locals[holder.index]?).any fun declared =>
                      (referenceType? ns declared.type.typeId).isNone }
                else none }
          lifetimeRelations := relations
          carrierDeaths := analysis.deaths.carrierDeaths
          receiverNoEscape := receiverNoEscape })
      else (others, none)

end VerexIR.Validation
