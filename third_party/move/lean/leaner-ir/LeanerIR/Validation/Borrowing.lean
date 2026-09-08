-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Diagnostic
import LeanerIR.Validation.PlaceIndex
import LeanerIR.Validation.Validated

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
Function call and result boundaries recursively
retain structural tuple, vector, reference, and function-pointer lifetime
constraints without guessing nominal variance. Explicit reference freezes
record that the mutable source lifetime outlives the shared result lifetime.
-/

namespace LeanerIR.Validation

private inductive LoanProjection where
  | dereference
  | field (name : NameId)
  | index (literal : Option Nat)
  | subslice (start stop : Nat) (fromEnd : Bool)
  | downcast (variant : NameId)
  deriving Repr, BEq, Inhabited

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

private structure LoanFact where
  expression : ExprId
  kind : ReferenceKind
  place : LoanPlace
  lifetime : LifetimeId
  loc : LocId
  holders : Array LocalId := #[]
  holderPlaces : Array LoanPlace := #[]
  deriving Repr, BEq, Inhabited

private structure BorrowState where
  active : Array LoanFact := #[]
  deriving BEq, Inhabited

private structure BorrowFlow where
  normal : Option BorrowState := none
  breaks : Array (Nat × BorrowState) := #[]
  breakValueLoans : Array (Nat × Array LoanFact) := #[]
  continues : Array (Nat × BorrowState) := #[]
  returns : Array BorrowState := #[]
  valueLoans : Array LoanFact := #[]
  observed : Array LoanFact := #[]
  /-- Death points of mutable loans, keyed by the loan's borrow expression.
  Recorded where the flow releases a loan; the elimination stage turns them
  into `endLoan` markers. -/
  deaths : Array (ExprId × LoanDeath) := #[]
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
    if loans.any (·.expression == loan.expression) then
      loans.map fun current =>
        if current.expression == loan.expression then
          { current with
            holders := appendUniqueLocals current.holders loan.holders
            holderPlaces := appendUniqueLoanPlaces current.holderPlaces loan.holderPlaces }
        else current
    else loans.push loan

/-- Mutable loans of `before` no longer active in `after`. -/
private def diffLoans (before after : BorrowState) : Array LoanFact :=
  before.active.filter fun loan =>
    loan.kind == .mutable &&
      !after.active.any (·.expression == loan.expression)

private def appendUniqueDeaths (left right : Array (ExprId × LoanDeath)) :
    Array (ExprId × LoanDeath) :=
  right.foldl (init := left) fun deaths death =>
    if deaths.contains death then deaths else deaths.push death

private def pushDeaths (deaths : Array (ExprId × LoanDeath))
    (died : Array LoanFact) (death : LoanDeath) : Array (ExprId × LoanDeath) :=
  died.foldl (init := deaths) fun deaths loan =>
    appendUniqueDeaths deaths #[(loan.expression, death)]

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
  | fuel + 1, some (.field base _ field) => do
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

private def conflictDiagnostic (loc : LocId) (loan : LoanFact) (action : String) : Diagnostic := {
  code := "LIR-SEMANTIC-BORROW-CONFLICT"
  message := s!"{action} conflicts with an active {repr loan.kind} loan"
  primary := some loc
  related := #[{ loc := loan.loc, message := "active loan originates here" }] }

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

private def addBorrow (ns : ValidatedNamespace) (exprId : ExprId) (loc : LocId)
    (resultType : TypeId) (kind : BorrowKind) (placeId : PlaceId)
    (state : BorrowState) (discarded : Bool := false) :
    BorrowState × Array LoanFact × Array Diagnostic :=
  match kind, loanPlace? ns placeId, referenceType? ns resultType with
  | .immutable, some place, some reference =>
      let kind := ReferenceKind.shared
      let loan : LoanFact := { expression := exprId, kind, place, lifetime := reference.lifetime, loc }
      let prior := { state with active := state.active.filter (·.expression != exprId) }
      let diagnostics := accessDiagnostics prior loc place false "borrow"
      let active := if state.active.any (·.expression == exprId) then state.active else state.active.push loan
      ({ active }, #[loan], diagnostics)
  | .mutable, some place, some reference =>
      let kind := ReferenceKind.mutable
      let loan : LoanFact := { expression := exprId, kind, place, lifetime := reference.lifetime, loc }
      let prior := { state with active := state.active.filter (·.expression != exprId) }
      -- Move's local-handle creation does not itself read or write the
      -- referent. An unobserved direct-local result can therefore take an
      -- empty live range even when another handle exists. The let analysis
      -- below proves non-use and retires this loan before entering its body.
      -- Projected/global resolution and Rust keep their existing checks.
      let diagnostics := if discarded && ns.profile == some .move &&
          place.projections.isEmpty then #[]
        else accessDiagnostics prior loc place true "borrow"
      let active := if state.active.any (·.expression == exprId) then state.active else state.active.push loan
      ({ active }, #[loan], diagnostics)
  | .profile _, _, _ =>
      (state, #[], #[.at "LIR-SEMANTIC-BORROW-COVERAGE"
        "profile-specific borrow modes are outside the initial loan certificate" loc])
  | _, none, _ =>
      (state, #[], #[.at "LIR-SEMANTIC-BORROW-COVERAGE"
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
        expression := exprId, kind, place, lifetime := reference.lifetime, loc }
      let prior := { state with active := state.active.filter (·.expression != exprId) }
      let diagnostics := accessDiagnostics prior loc place (kind == .mutable) "global borrow"
      let active := if state.active.any (·.expression == exprId) then state.active
        else state.active.push loan
      ({ active }, #[loan], diagnostics)
  | _, _ => (state, #[], #[])

private partial def placeIndexAccessDiagnostics (ns : ValidatedNamespace)
    (state : BorrowState) (loc : LocId) (placeId : PlaceId) (fuel : Nat) : Array Diagnostic :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none | _, some (.localVar _) => #[]
  | fuel + 1, some (.deref base) | fuel + 1, some (.field base _ _) |
      fuel + 1, some (.subslice base ..) | fuel + 1, some (.downcast base _) =>
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
  placeDiagnostics ++ placeIndexAccessDiagnostics ns state loc placeId (ns.places.size + 1)

private partial def placeContainsDereference (ns : ValidatedNamespace) (placeId : PlaceId)
    (fuel : Nat) : Bool :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none | _, some (.localVar _) => false
  | _, some (.deref _) => true
  | fuel + 1, some (.field base _ _) | fuel + 1, some (.index base _) |
      fuel + 1, some (.subslice base ..) | fuel + 1, some (.downcast base _) =>
      placeContainsDereference ns base fuel

private def dereferenceConsumptionDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (placeId : PlaceId) : Array Diagnostic :=
  if placeContainsDereference ns placeId (ns.places.size + 1) then
    #[.at "LIR-SEMANTIC-BORROW-DEREF-CONSUME"
      "moving or dropping through a reference is not supported by the safe reference model" loc]
  else #[]

private partial def placeUsesLocal (ns : ValidatedNamespace) (placeId : PlaceId)
    (localId : LocalId) (fuel : Nat) : Bool :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none => true
  | _ + 1, some (.localVar candidate) => candidate == localId
  | fuel + 1, some (.deref base) | fuel + 1, some (.field base _ _) |
      fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.downcast base _) => placeUsesLocal ns base localId fuel
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
        | .move place | .copy place | .borrow _ place | .read place |
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
  | _, some { kind := .quantifier .., .. } => true
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
        | .move place | .copy place | .borrow _ place | .read place |
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
  | _, some { kind := .quantifier .., .. } => true
  | _, some { kind := .value .., .. } | _, some { kind := .constant _, .. } |
      _, some { kind := .continue_ _, .. } => false

private def loanUsedBy (ns : ValidatedNamespace) (expressions : List ExprId)
    (loan : LoanFact) : Bool :=
  loan.holders.any fun holder =>
    let carriers := loan.holderPlaces.filter (·.root == .local holder)
    if carriers.isEmpty then expressions.any fun expression =>
      expressionUsesLocal ns expression holder (ns.expressions.size + 1)
    else carriers.any fun carrier => expressions.any fun expression =>
      expressionUsesCarrier ns expression carrier (ns.expressions.size + 1)

private def loansUsedBy (ns : ValidatedNamespace) (expressions : List ExprId)
    (state : BorrowState) (initial : Array ExprId := #[]) : Array ExprId :=
  state.active.foldl (init := initial) fun preserved loan =>
    if preserved.contains loan.expression || !loanUsedBy ns expressions loan then preserved
    else preserved.push loan.expression

private def loanUsedByPlace (ns : ValidatedNamespace) (placeId : PlaceId)
    (loan : LoanFact) : Bool :=
  loan.holders.any fun holder =>
    let carriers := loan.holderPlaces.filter (·.root == .local holder)
    if carriers.isEmpty then placeUsesLocal ns placeId holder (ns.places.size + 1)
    else carriers.any (placeUsesCarrier ns placeId)

private def loansUsedByPlace (ns : ValidatedNamespace) (placeId : PlaceId)
    (state : BorrowState) (initial : Array ExprId := #[]) : Array ExprId :=
  state.active.foldl (init := initial) fun preserved loan =>
    if preserved.contains loan.expression || !loanUsedByPlace ns placeId loan then preserved
    else preserved.push loan.expression

private def operationPlace? : Operation → Option PlaceId
  | .move place | .copy place | .borrow _ place | .read place |
      .write place | .drop place => some place
  | _ => none

private def retainLoansUsedBy (ns : ValidatedNamespace) (expressions : List ExprId)
    (state : BorrowState) (preserved : Array ExprId := #[]) : BorrowState :=
  { active := state.active.filter fun loan =>
      loan.holders.isEmpty || preserved.contains loan.expression ||
        loanUsedBy ns expressions loan }

private def bindObservedLoans (state : BorrowState) (observed : Array LoanFact)
    (holder : LocalId) : BorrowState :=
  let holderPlace : LoanPlace := { root := .local holder }
  { active := state.active.map fun loan =>
      if observed.any (·.expression == loan.expression) then
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
        if holders.isEmpty && !carried.any (·.expression == loan.expression) then none
        else some { loan with holders, holderPlaces } }

private def releasePlaceLoans (ns : ValidatedNamespace) (state : BorrowState)
    (carried : Array LoanFact) (placeId : PlaceId) : BorrowState :=
  match loanPlace? ns placeId with
  | none => state
  | some place =>
    let active := state.active.filterMap fun loan =>
      let carriers := loan.holderPlaces.filter (·.root == place.root)
      if carriers.isEmpty then
        match place.root.local? with
        | some rootLocal =>
            if place.projections.isEmpty && loan.holders.contains rootLocal then
              let holders := loan.holders.filter (· != rootLocal)
              if holders.isEmpty && !carried.any (·.expression == loan.expression) then none
              else some ({ loan with holders } : LoanFact)
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
          if holders.isEmpty && !carried.any (·.expression == loan.expression) then none
          else some ({ loan with holders, holderPlaces } : LoanFact)
    { active }

private def isOwnedCarrierPlace (place : LoanPlace) : Bool :=
  !place.projections.any (· matches .dereference)

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
              if holders.isEmpty then none else some ({ loan with holders } : LoanFact)
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
          if holders.isEmpty then none
          else some ({ loan with holders, holderPlaces } : LoanFact)
    { active }

private def replaceHolderLoans (state : BorrowState) (observed : Array LoanFact)
    (holder : LocalId) : BorrowState :=
  bindObservedLoans (releaseHolderLoans state observed holder) observed holder

private def recordBoundLoans (state : BorrowState) (observed : Array LoanFact) : Array LoanFact :=
  appendUniqueLoans observed <| state.active.filter fun loan =>
    observed.any (·.expression == loan.expression)

private def loansHeldAtPlace (ns : ValidatedNamespace) (state : BorrowState)
    (placeId : PlaceId) : Array LoanFact :=
  match loanPlace? ns placeId with
  | some place => state.active.filter fun loan =>
      let carriers := loan.holderPlaces.filter (·.root == place.root)
      if carriers.isEmpty then place.root.local?.any loan.holders.contains
      else carriers.any (placesOverlap · place)
  | none => #[]

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
  | fuel + 1, some (.vector element _) => typeMayContainReference ns element fuel
  | fuel + 1, some (.function arguments result _) =>
      arguments.any (typeMayContainReference ns · fuel) ||
        typeMayContainReference ns result fuel
  | _, some (.nominal ..) | _, some (.typeParameter _) | _, some (.profile _) => true
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

private def loanFlowsFromExpression (ns : ValidatedNamespace) (loan : LoanFact)
    (expression : ExprId) : Bool :=
  expressionContains ns expression loan.expression (ns.expressions.size + 1) ||
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

/-- Reachability over instantiated outlives edges. The worklist carries a
visited set, keeping the search polynomial; enumerating paths instead is
exponential on diamond- or cycle-shaped instantiated predicate edges. -/
private def lifetimeOutlivesVia
    (edges : Array (LifetimeId × LifetimeId)) (longer shorter : LifetimeId)
    (fuel : Nat) : Bool :=
  go fuel #[longer] #[]
where
  go : Nat → Array LifetimeId → Array LifetimeId → Bool
    | 0, _, _ => false
    | fuel + 1, frontier, visited =>
        frontier.contains shorter ||
          (let visited := visited ++ frontier
           let next := edges.foldl (init := #[]) fun next edge =>
             if frontier.contains edge.1 && !visited.contains edge.2 &&
                 !next.contains edge.2 then next.push edge.2 else next
           !next.isEmpty && go fuel next visited)

private def callResultLifetimeInContexts (ns : ValidatedNamespace) :
    List (Array GenericArgument) → LifetimeId → LifetimeId
  | [], lifetime => lifetime
  | arguments :: outer, lifetime =>
      match ns.tables.lifetimes[lifetime.index]? with
      | some { kind := .parameter index, .. } => match arguments[index]? with
          | some (.lifetime value) => callResultLifetimeInContexts ns outer value
          | _ => lifetime
      | _ => lifetime

private partial def structuralCallResultLoanPathsFuel (unit : ValidatedUnit)
    (ns : ValidatedNamespace)
    (typeId : TypeId) (instantiations : Array GenericArgument)
    (edges : Array (LifetimeId × LifetimeId)) (sourceLifetime : LifetimeId)
    (contexts : List (Array GenericArgument))
    (pathPrefix : Array LoanProjection) (visiting : Array NameId)
    (fuel : Nat) :
    Option (Array (Array LoanProjection)) :=
  -- The fuel is seeded once by the wrapper below. Re-seeding on zero inside
  -- the recursion would never terminate on an identity type-parameter
  -- instantiation, such as a generic callee applied to the caller's own
  -- type parameter.
  match fuel, ns.tables.types[typeId.index]? with
  | 0, _ | _, none => none
  | _, some (.reference reference) =>
      let target := instantiatedLifetime ns instantiations <|
        callResultLifetimeInContexts ns contexts reference.lifetime
      if lifetimeOutlivesVia edges sourceLifetime target
          (ns.tables.lifetimes.size + 1) then some #[pathPrefix]
      else some #[]
  | fuel + 1, some (.tuple elements) =>
      (elements.zip (Array.range elements.size)).foldlM (m := Option)
        (init := #[]) fun paths pair => do
          let nested ← structuralCallResultLoanPathsFuel unit ns pair.1 instantiations edges
            sourceLifetime contexts (pathPrefix.push (.index (some pair.2))) visiting fuel
          pure (appendUniqueProjectionPaths paths nested)
  | fuel + 1, some (.vector element _) =>
      structuralCallResultLoanPathsFuel unit ns element instantiations edges sourceLifetime
        contexts (pathPrefix.push (.index none)) visiting fuel
  | fuel + 1, some (.typeParameter index) => match contexts with
      | arguments :: outer => match arguments[index]? with
          | some (.typeArg typeUse) => structuralCallResultLoanPathsFuel unit ns typeUse.typeId
              instantiations edges sourceLifetime outer pathPrefix visiting fuel
          | _ => none
      | [] => match instantiations[index]? with
          | some (.typeArg typeUse) => structuralCallResultLoanPathsFuel unit ns typeUse.typeId
              instantiations edges sourceLifetime [] pathPrefix visiting fuel
          | _ => none
  | fuel + 1, some (.typeDomain element) =>
      structuralCallResultLoanPathsFuel unit ns element instantiations edges sourceLifetime contexts
        pathPrefix visiting fuel
  | fuel + 1, some (.nominal name arguments) =>
      if visiting.contains name then some #[pathPrefix] else do
      let (_, declaration) ← nominalAt? unit name
      let visiting := visiting.push name
      if declaration.variants.isEmpty then
        declaration.fields.foldlM (m := Option) (init := #[]) fun paths field => do
          let nested ← structuralCallResultLoanPathsFuel unit ns field.type.typeId instantiations
            edges sourceLifetime (arguments :: contexts)
            (pathPrefix.push (.field field.name)) visiting fuel
          pure (appendUniqueProjectionPaths paths nested)
      else declaration.variants.foldlM (m := Option) (init := #[]) fun paths variant => do
        let variantPrefix := pathPrefix.push (.downcast variant.name)
        let nested ← variant.fields.foldlM (m := Option) (init := #[]) fun paths field => do
          let fieldPaths ← structuralCallResultLoanPathsFuel unit ns field.type.typeId instantiations
            edges sourceLifetime (arguments :: contexts)
            (variantPrefix.push (.field field.name)) visiting fuel
          pure (appendUniqueProjectionPaths paths fieldPaths)
        pure (appendUniqueProjectionPaths paths nested)
  | _, some (.profile _) | _, some (.function ..) |
      _, some (.resourceDomain ..) => none
  | _, some .unit | _, some .never | _, some .bool | _, some .character |
      _, some .string | _, some .bytes | _, some .address | _, some .signer |
      _, some (.integer ..) | _, some .range |
      _, some .eventStore | _, some .stateDomain => some #[]

private def structuralCallResultLoanPaths? (unit : ValidatedUnit)
    (ns : ValidatedNamespace)
    (typeId : TypeId) (instantiations : Array GenericArgument)
    (edges : Array (LifetimeId × LifetimeId)) (sourceLifetime : LifetimeId)
    (contexts : List (Array GenericArgument) := [])
    (pathPrefix : Array LoanProjection := #[]) :
    Option (Array (Array LoanProjection)) :=
  structuralCallResultLoanPathsFuel unit ns typeId instantiations edges sourceLifetime
    contexts pathPrefix #[] (ns.tables.types.size + 1)

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
      | .call (.function reference) =>
          match functionAt? unit reference with
            | some (_, declaration) =>
                  let predicates := declaration.signature.generics.foldl
                    (fun predicates binder => predicates ++ binder.predicates)
                    declaration.signature.predicates
                  let edges := predicates.filterMap fun predicate => match predicate with
                    | .lifetimeOutlives longer shorter => some (
                        instantiatedLifetime ns instantiations longer,
                        instantiatedLifetime ns instantiations shorter)
                    | _ => none
                  let paths? := match declaration.signature.results.toList with
                    | [] => some #[]
                    | [result] => structuralCallResultLoanPaths? unit ns result.typeId
                        instantiations edges loan.lifetime
                    | _ => (declaration.signature.results.zip
                        (Array.range declaration.signature.results.size)).foldlM
                        (m := Option) (init := #[]) fun paths (pair : TypeUse × Nat) => do
                          let nested ← structuralCallResultLoanPaths? unit ns pair.1.typeId
                            instantiations edges loan.lifetime []
                            #[.index (some pair.2)]
                          pure (appendUniqueProjectionPaths paths nested)
                  match paths? with
                  | some paths => if paths.isEmpty && loanFlowsFromExpression ns loan exprId then
                      #[#[]] else paths
                  | none => if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]
            | none => if loanFlowsFromExpression ns loan exprId then #[#[]] else #[]
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
      match observed.find? (·.expression == loan.expression) with
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
    if !isOwnedCarrierPlace place then state else
    { active := state.active.map fun loan =>
        match observed.find? (·.expression == loan.expression) with
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
  | _, some { kind := .variable holder, .. } =>
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

private def instantiatedOutlivesEdges (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) (instantiations : Array GenericArgument) :
    Array (LifetimeId × LifetimeId) :=
  let predicates := declaration.signature.generics.foldl
    (fun predicates binder => predicates ++ binder.predicates) declaration.signature.predicates
  predicates.filterMap fun predicate => match predicate with
    | .lifetimeOutlives longer shorter => some (
        instantiatedLifetime ns instantiations longer,
        instantiatedLifetime ns instantiations shorter)
    | _ => none

private def appendUniqueLifetime (lifetimes : Array LifetimeId) (lifetime : LifetimeId) :
    Array LifetimeId :=
  if lifetimes.contains lifetime then lifetimes else lifetimes.push lifetime

private def instantiatedLifetimeInContexts (ns : ValidatedNamespace) :
    List (Array GenericArgument) → LifetimeId → LifetimeId
  | [], lifetime => lifetime
  | instantiations :: outer, lifetime =>
      match ns.tables.lifetimes[lifetime.index]? with
      | some { kind := .parameter index, .. } => match instantiations[index]? with
          | some (.lifetime value) => instantiatedLifetimeInContexts ns outer value
          | _ => lifetime
      | _ => lifetime

private partial def instantiatedTypeUseInContexts (ns : ValidatedNamespace)
    (contexts : List (Array GenericArgument)) (typeUse : TypeUse) (fuel : Nat) : TypeUse :=
  match fuel, contexts, ns.tables.types[typeUse.typeId.index]? with
  | 0, _, _ | _, [], _ => typeUse
  | fuel + 1, instantiations :: outer, some (.typeParameter index) =>
      match instantiations[index]? with
      | some (.typeArg instantiated) =>
          instantiatedTypeUseInContexts ns outer instantiated fuel
      | _ => typeUse
  | _, _, _ => typeUse

private def instantiatedArgumentsInContexts (ns : ValidatedNamespace)
    (contexts : List (Array GenericArgument)) (arguments : Array GenericArgument) :
    Array GenericArgument :=
  arguments.map fun argument => match argument with
    | .lifetime lifetime => .lifetime (instantiatedLifetimeInContexts ns contexts lifetime)
    | .typeArg typeUse => .typeArg <|
        instantiatedTypeUseInContexts ns contexts typeUse (ns.tables.types.size + 1)
    | argument => argument

private partial def knownReferenceLifetimes? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (typeId : TypeId)
    (contexts : List (Array GenericArgument)) (fuel : Nat)
    (visiting : Array (NameId × Array GenericArgument) := #[]) : Option (Array LifetimeId) :=
  match fuel, ns.tables.types[typeId.index]? with
  | 0, _ | _, none => none
  | _, some (.reference reference) =>
      some #[instantiatedLifetimeInContexts ns contexts reference.lifetime]
  | fuel + 1, some (.tuple elements) =>
      elements.foldlM (m := Option) (init := #[]) fun lifetimes element => do
        let nested ← knownReferenceLifetimes? unit ns element contexts fuel visiting
        pure (nested.foldl appendUniqueLifetime lifetimes)
  | fuel + 1, some (.vector element _) | fuel + 1, some (.typeDomain element) =>
      knownReferenceLifetimes? unit ns element contexts fuel visiting
  | fuel + 1, some (.nominal name arguments) =>
      let frame := (name, instantiatedArgumentsInContexts ns contexts arguments)
      if visiting.contains frame then some #[]
      else if visiting.any (·.1 == name) then none
      else do
      let (_, declaration) ← nominalAt? unit name
      let fields := declaration.variants.foldl
        (fun fields variant => fields ++ variant.fields) declaration.fields
      fields.foldlM (m := Option) (init := #[]) fun lifetimes field => do
        let nested ← knownReferenceLifetimes? unit ns field.type.typeId
          (arguments :: contexts) fuel (visiting.push frame)
        pure (nested.foldl appendUniqueLifetime lifetimes)
  | fuel + 1, some (.typeParameter index) => match contexts with
      | instantiations :: outer => match instantiations[index]? with
          | some (.typeArg instantiated) =>
              knownReferenceLifetimes? unit ns instantiated.typeId outer fuel visiting
          | _ => none
      | [] => none
  | _, some (.profile _) | _, some (.function ..) |
      _, some (.resourceDomain ..) => none
  | _, some .unit | _, some .never | _, some .bool | _, some .character |
      _, some .string | _, some .bytes | _, some .address | _, some .signer |
      _, some (.integer ..) | _, some .range |
      _, some .eventStore | _, some .stateDomain => some #[]

private def directCallResultLoans? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (reference : QualifiedRef)
    (instantiations : Array GenericArgument) (arguments : Array ExprId)
    (observed : Array LoanFact) : Option (Array LoanFact) := do
  let (_, declaration) ← functionAt? unit reference
  let edges := instantiatedOutlivesEdges ns declaration instantiations
  let resultLifetimes ← declaration.signature.results.foldlM (m := Option)
    (init := #[]) fun lifetimes result => do
      let nested ← knownReferenceLifetimes? unit ns result.typeId [instantiations]
        (ns.tables.types.size + 1)
      pure <| nested.foldl appendUniqueLifetime lifetimes
  let referenceArguments ← (arguments.zip declaration.signature.parameters).foldlM
    (m := Option) (init := #[]) fun sources pair => do
      let lifetimes ← knownReferenceLifetimes? unit ns pair.2.typeUse.typeId
        [instantiations] (ns.tables.types.size + 1)
      pure <| if lifetimes.isEmpty then sources else sources.push (pair.1, lifetimes)
  if resultLifetimes.isEmpty then some #[] else
  if referenceArguments.isEmpty then some #[] else
    let sources := referenceArguments.filter fun source =>
      source.2.any fun sourceLifetime => resultLifetimes.any fun resultLifetime =>
        lifetimeOutlivesVia edges sourceLifetime resultLifetime (ns.tables.lifetimes.size + 1)
    if sources.isEmpty then none else
      some (observed.filter fun loan =>
        sources.any fun source => loanFlowsFromExpression ns loan source.1)

private def discardUncarriedTemporaries (state : BorrowState)
    (observed carried : Array LoanFact) : BorrowState :=
  { active := state.active.filter fun loan =>
      !loan.holders.isEmpty || !observed.any (·.expression == loan.expression) ||
        carried.any (·.expression == loan.expression) }

/-- At a function-result boundary, only mutable loans carried by the result
remain live. Every other mutable loan has a statically known death at that
boundary and is materialized as `endLoan`; immutable loans need no prophetic
write-back and remain irrelevant to death marking. -/
private def retainResultLoans (state : BorrowState)
    (carried : Array LoanFact) : BorrowState :=
  { active := state.active.filter fun loan =>
      loan.kind != .mutable || carried.any (·.expression == loan.expression) }

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
      (state : BorrowState) (preserved : Array ExprId := #[])
      (discarded : Bool := false) : BorrowFlow :=
    let flow := analyzeExprCore unit ns exprId state preserved discarded
    match flow.normal with
    | none => flow
    | some exit =>
        let candidates := appendUniqueLoans state.active flow.observed
        let died := candidates.filter fun loan =>
          loan.kind == .mutable &&
            !exit.active.any (·.expression == loan.expression) &&
            !flow.deaths.any (·.1 == loan.expression)
        if died.isEmpty then flow
        else { flow with deaths := pushDeaths flow.deaths died { anchor := exprId } }

  private partial def analyzeExprCore (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (exprId : ExprId)
      (state : BorrowState) (preserved : Array ExprId := #[])
      (discarded : Bool := false) : BorrowFlow :=
    match ns.expressions[exprId.index]? with
    | none => { normal := some state }
    | some expression => match expression.kind with
        | .value .. | .constant _ | .quantifier .. | .spec _ => { normal := some state }
        | .localVar localId =>
            let place : LoanPlace := { root := .local localId }
            let loans := state.active.filter (·.holders.contains localId)
            { normal := some state
              valueLoans := loans
              observed := loans
              diagnostics := accessDiagnostics state expression.loc place false "local read" }
        | .operation operation instantiations arguments _ =>
            let argumentState := match operation with
              | .write place => releasePlaceLoansUnusedBy ns state place arguments.toList
              | _ => state
            let entryDeaths := match arguments[0]? with
              | some first =>
                  pushDeaths #[] (diffLoans state argumentState)
                    { anchor := first, before := true }
              | none => pushDeaths #[] (diffLoans state argumentState) { anchor := exprId }
            let operationPreserved := match operationPlace? operation with
              | some place => loansUsedByPlace ns place argumentState preserved
              | none => preserved
            let argumentsFlow := analyzeExprList unit ns arguments.toList argumentState operationPreserved
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
                      (some afterArguments, #[], #[], diagnostics)
                  | .borrow kind place =>
                      let (state, loans, diagnostics) :=
                        addBorrow ns exprId expression.loc expression.typeId kind place afterArguments
                          discarded
                      let parentLoans := loansHeldAtPlace ns afterArguments place
                      let valueLoans := appendUniqueLoans parentLoans loans
                      (some state, valueLoans, valueLoans, diagnostics)
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
                      (some afterConsume, valueLoans, accessed,
                        checkPlaceAccess ns afterArguments expression.loc place true "place consume" ++
                          dereferenceConsumptionDiagnostics ns expression.loc place)
                  | .write place =>
                      let afterWrite := match arguments[0]? with
                        | some value => replaceObservedLoansAtPlaceFromExpression unit ns afterArguments
                            argumentsFlow.valueLoans place value
                        | none => releasePlaceLoans ns afterArguments
                            argumentsFlow.valueLoans place
                      (some afterWrite, #[], #[],
                        checkPlaceAccess ns afterArguments expression.loc place true "place write")
                  | .call (.function reference) =>
                      let directLoans := if typeMayContainReference ns expression.typeId then
                        (directCallResultLoans? unit ns reference instantiations arguments
                          argumentsFlow.observed).getD argumentsFlow.observed
                        else #[]
                      let produced := if directLoans.isEmpty &&
                          argumentsFlow.observed.isEmpty then
                        match referenceType? ns expression.typeId with
                        | some referenceType =>
                            if referenceType.kind == .mutable then
                              let loan : LoanFact := {
                                expression := exprId
                                kind := .mutable
                                place := { root := .external }
                                lifetime := referenceType.lifetime
                                loc := expression.loc }
                              #[loan]
                            else #[]
                        | none => #[]
                      else #[]
                      let valueLoans := appendUniqueLoans directLoans produced
                      let afterArguments := { afterArguments with
                        active := appendUniqueLoans afterArguments.active produced }
                      let afterCall := discardUncarriedTemporaries afterArguments
                        argumentsFlow.observed valueLoans
                      (some afterCall, valueLoans, produced, #[])
                  | _ =>
                      let valueLoans := if typeMayContainReference ns expression.typeId then
                        argumentsFlow.observed else #[]
                      (some afterArguments, valueLoans, #[], #[])
                { argumentsFlow with
                  normal
                  valueLoans
                  observed := appendUniqueLoans argumentsFlow.observed operationObserved
                  deaths := appendUniqueDeaths entryDeaths argumentsFlow.deaths
                  diagnostics := argumentsFlow.diagnostics ++ diagnostics }
        | .block statements result =>
            analyzeBlockExprList unit ns (statements.toList ++ result.toList) state preserved
        | .letDecl pattern initializer body =>
            let bodyProtected := loansUsedBy ns [body] state preserved
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
                  analyzeExpr unit ns value state bodyProtected discarded
            match initializerFlow.normal with
            | none => initializerFlow
            | some afterInitializer =>
                let afterInitializer := match initializer with
                  | some value => bindPatternLoans unit ns afterInitializer
                      initializerFlow.valueLoans pattern value
                  | none => afterInitializer
                let bound := afterInitializer
                let afterInitializer := discardUncarriedTemporaries afterInitializer
                  initializerFlow.valueLoans #[]
                let initializerObserved := appendUniqueLoans initializerFlow.observed <|
                  recordBoundLoans afterInitializer initializerFlow.valueLoans
                let afterInitializer := retainLoansUsedBy ns [body] afterInitializer preserved
                let entryDeaths := pushDeaths #[] (diffLoans bound afterInitializer)
                  { anchor := body, before := true }
                let bodyFlow := analyzeExpr unit ns body afterInitializer preserved
                { bodyFlow with
                  breaks := initializerFlow.breaks ++ bodyFlow.breaks
                  breakValueLoans := initializerFlow.breakValueLoans ++ bodyFlow.breakValueLoans
                  continues := initializerFlow.continues ++ bodyFlow.continues
                  returns := initializerFlow.returns ++ bodyFlow.returns
                  observed := appendUniqueLoans initializerObserved bodyFlow.observed
                  deaths := appendUniqueDeaths
                    (appendUniqueDeaths initializerFlow.deaths entryDeaths) bodyFlow.deaths
                  diagnostics := initializerFlow.diagnostics ++ bodyFlow.diagnostics }
        | .ifElse condition thenBranch elseBranch =>
            let branchExpressions := thenBranch :: elseBranch.toList
            let conditionProtected := loansUsedBy ns branchExpressions state preserved
            let conditionFlow := analyzeExpr unit ns condition state conditionProtected
            match conditionFlow.normal with
            | none => conditionFlow
            | some branchState =>
                let thenState := retainLoansUsedBy ns [thenBranch] branchState preserved
                let thenEntryDeaths := pushDeaths #[] (diffLoans branchState thenState)
                  { anchor := thenBranch, before := true }
                let thenFlow := analyzeExpr unit ns thenBranch thenState preserved
                let (elseEntryDeaths, elseFlow) := match elseBranch with
                  | some branch =>
                      let elseState := retainLoansUsedBy ns [branch] branchState preserved
                      (pushDeaths #[] (diffLoans branchState elseState)
                          { anchor := branch, before := true },
                        analyzeExpr unit ns branch elseState preserved)
                  | none =>
                      let elseState := retainLoansUsedBy ns [] branchState preserved
                      (pushDeaths #[] (diffLoans branchState elseState) { anchor := exprId },
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
            let armExpressions := arms.foldl (fun expressions arm =>
              expressions ++ arm.guard.toList ++ [arm.body]) []
            let scrutineeProtected := loansUsedBy ns armExpressions state preserved
            let scrutineeFlow := analyzeExpr unit ns scrutinee state scrutineeProtected
            match scrutineeFlow.normal with
            | none => scrutineeFlow
            | some armState =>
                let armsFlow := arms.foldl (fun flow arm =>
                  let expressions := arm.guard.toList ++ [arm.body]
                  let state := retainLoansUsedBy ns expressions armState preserved
                  let entryDeaths := pushDeaths #[] (diffLoans armState state)
                    { anchor := arm.guard.getD arm.body, before := true }
                  let armFlow := analyzeArm unit ns arm state preserved
                  mergeFlows flow
                    { armFlow with
                      deaths := appendUniqueDeaths entryDeaths armFlow.deaths }) {}
                { armsFlow with
                  breaks := scrutineeFlow.breaks ++ armsFlow.breaks
                  breakValueLoans := scrutineeFlow.breakValueLoans ++ armsFlow.breakValueLoans
                  continues := scrutineeFlow.continues ++ armsFlow.continues
                  returns := scrutineeFlow.returns ++ armsFlow.returns
                  observed := appendUniqueLoans scrutineeFlow.observed armsFlow.observed
                  deaths := appendUniqueDeaths scrutineeFlow.deaths armsFlow.deaths
                  diagnostics := scrutineeFlow.diagnostics ++ armsFlow.diagnostics }
        | .loop _ body =>
            let loopState := retainLoansUsedBy ns [body] state preserved
            let entryDeaths := pushDeaths #[] (diffLoans state loopState)
              { anchor := body, before := true }
            let flow := analyzeLoop unit ns body loopState loopState
              (loopState.active.size + ns.expressions.size + 1) preserved
            { flow with deaths := appendUniqueDeaths entryDeaths flow.deaths }
        | .break_ nest value =>
            let valueFlow := match value with
              | some value => analyzeExpr unit ns value state preserved
              | none => { normal := some state }
            let breaks := match valueFlow.normal with
              | some afterValue => valueFlow.breaks.push (nest, afterValue)
              | none => valueFlow.breaks
            let breakValueLoans := match valueFlow.normal with
              | some _ => valueFlow.breakValueLoans.push (nest, valueFlow.valueLoans)
              | none => valueFlow.breakValueLoans
            { valueFlow with normal := none, breaks, breakValueLoans, valueLoans := #[] }
        | .continue_ nest => { continues := #[(nest, state)] }
        | .return_ values =>
            let state := retainLoansUsedBy ns values.toList state preserved
            let valuesFlow := analyzeExprList unit ns values.toList state preserved
            match valuesFlow.normal with
            | none => valuesFlow
            | some afterValues =>
                let returned := retainResultLoans afterValues valuesFlow.valueLoans
                let died := diffLoans afterValues returned
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
            let valueState := releasePlaceLoansUnusedBy ns state place [value]
            let entryDeaths := pushDeaths #[] (diffLoans state valueState)
              { anchor := value, before := true }
            let assignmentPreserved := loansUsedByPlace ns place valueState preserved
            let valueFlow := analyzeExpr unit ns value valueState assignmentPreserved
            match valueFlow.normal with
            | none => valueFlow
            | some afterValue =>
                let afterValue := replaceObservedLoansAtPlaceFromExpression unit ns afterValue
                  valueFlow.valueLoans place value
                { valueFlow with
                  normal := some afterValue
                  valueLoans := #[]
                  observed := appendUniqueLoans valueFlow.observed <|
                    recordBoundLoans afterValue valueFlow.valueLoans
                  deaths := appendUniqueDeaths entryDeaths valueFlow.deaths
                  diagnostics := valueFlow.diagnostics ++
                    checkPlaceAccess ns afterValue expression.loc place true "assignment" }
        | .assignPattern pattern value =>
            let valueState := releaseHoldersUnusedBy ns state
              (patternVariables ns pattern) [value]
            let entryDeaths := pushDeaths #[] (diffLoans state valueState)
              { anchor := value, before := true }
            let valueFlow := analyzeExpr unit ns value valueState preserved
            match valueFlow.normal with
            | none => valueFlow
            | some afterValue =>
                let afterValue := replacePatternLoans unit ns afterValue valueFlow.valueLoans pattern value
                let afterValue := discardUncarriedTemporaries afterValue
                  valueFlow.valueLoans #[]
                { valueFlow with
                  normal := some afterValue
                  valueLoans := #[]
                  observed := appendUniqueLoans valueFlow.observed <|
                    recordBoundLoans afterValue valueFlow.valueLoans
                  deaths := appendUniqueDeaths entryDeaths valueFlow.deaths }

  private partial def analyzeExprList (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (expressions : List ExprId)
      (state : BorrowState) (preserved : Array ExprId := #[]) : BorrowFlow :=
    match expressions with
    | [] => { normal := some state }
    | expression :: tail =>
        let futureProtected := loansUsedBy ns tail state preserved
        let head := analyzeExpr unit ns expression state futureProtected
        match head.normal with
        | none => head
        | some afterHead =>
            let rest := analyzeExprList unit ns tail afterHead preserved
            { rest with
              breaks := head.breaks ++ rest.breaks
              breakValueLoans := head.breakValueLoans ++ rest.breakValueLoans
              continues := head.continues ++ rest.continues
              returns := head.returns ++ rest.returns
              valueLoans := if tail.isEmpty then head.valueLoans else rest.valueLoans
              observed := appendUniqueLoans head.observed rest.observed
              deaths := appendUniqueDeaths head.deaths rest.deaths
              diagnostics := head.diagnostics ++ rest.diagnostics }

  private partial def analyzeBlockExprList (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (expressions : List ExprId) (state : BorrowState)
      (preserved : Array ExprId := #[]) : BorrowFlow :=
    match expressions with
    | [] => { normal := some state }
    | expression :: tail =>
        let entry := state
        let state := retainLoansUsedBy ns expressions state preserved
        let entryDeaths := pushDeaths #[] (diffLoans entry state)
          { anchor := expression, before := true }
        let futureProtected := loansUsedBy ns tail state preserved
        let head := analyzeExpr unit ns expression state futureProtected
        match head.normal with
        | none => { head with deaths := appendUniqueDeaths entryDeaths head.deaths }
        | some afterHead =>
            let discarded := if tail.isEmpty then afterHead else
              discardUncarriedTemporaries afterHead head.valueLoans #[]
            let discardDeaths := pushDeaths #[] (diffLoans afterHead discarded)
              { anchor := expression }
            let rest := analyzeBlockExprList unit ns tail discarded preserved
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
      (state : BorrowState) (preserved : Array ExprId := #[]) : BorrowFlow :=
    match arm.guard with
    | none => analyzeExpr unit ns arm.body state preserved
    | some guard =>
        let guardProtected := loansUsedBy ns [arm.body] state preserved
        let guardFlow := analyzeExpr unit ns guard state guardProtected
        match guardFlow.normal with
        | none => guardFlow
        | some afterGuard =>
            let bodyFlow := analyzeExpr unit ns arm.body afterGuard preserved
            { bodyFlow with
              breaks := guardFlow.breaks ++ bodyFlow.breaks
              breakValueLoans := guardFlow.breakValueLoans ++ bodyFlow.breakValueLoans
              continues := guardFlow.continues ++ bodyFlow.continues
              returns := guardFlow.returns ++ bodyFlow.returns
              observed := appendUniqueLoans guardFlow.observed bodyFlow.observed
              deaths := appendUniqueDeaths guardFlow.deaths bodyFlow.deaths
              diagnostics := guardFlow.diagnostics ++ bodyFlow.diagnostics }

  private partial def analyzeLoop (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (body : ExprId)
      (initial entry : BorrowState) (fuel : Nat)
      (preserved : Array ExprId := #[]) : BorrowFlow :=
    let bodyFlow := analyzeExpr unit ns body entry preserved
    let valueLoans := bodyFlow.breakValueLoans.foldl (init := #[]) fun loans pair =>
      if pair.1 == 0 then appendUniqueLoans loans pair.2 else loans
    let outerBreakValueLoans := bodyFlow.breakValueLoans.filterMap fun (nest, loans) =>
      match nest with | 0 => none | nest + 1 => some (nest, loans)
    let reentries :=
      (match bodyFlow.normal with | some state => #[state] | none => #[]) ++
        (bodyFlow.continues.filterMap fun (nest, state) => if nest == 0 then some state else none)
    -- Widen from the current entry rather than re-seeding from `initial`:
    -- the iterate then only grows, so the equality test converges within the
    -- finite loan lattice instead of oscillating through loan kills until the
    -- fuel is exhausted on every (possibly nested) loop.
    let nextEntry := reentries.foldl mergeState entry
    if fuel > 0 && nextEntry != entry then
      let next := analyzeLoop unit ns body initial nextEntry (fuel - 1) preserved
      { next with
        valueLoans := appendUniqueLoans valueLoans next.valueLoans
        breakValueLoans := outerBreakValueLoans ++ next.breakValueLoans
        returns := bodyFlow.returns ++ next.returns
        observed := appendUniqueLoans bodyFlow.observed next.observed
        deaths := appendUniqueDeaths bodyFlow.deaths next.deaths
        diagnostics := bodyFlow.diagnostics ++ next.diagnostics }
    else
      let exits := bodyFlow.breaks.filterMap fun (nest, state) =>
        if nest == 0 then some state else none
      let normal := exits.foldl (fun result state => mergeNormal result (some state)) none
      let outerBreaks := bodyFlow.breaks.filterMap fun (nest, state) =>
        match nest with | 0 => none | nest + 1 => some (nest, state)
      let outerContinues := bodyFlow.continues.filterMap fun (nest, state) =>
        match nest with | 0 => none | nest + 1 => some (nest, state)
      { normal, breaks := outerBreaks, breakValueLoans := outerBreakValueLoans
        continues := outerContinues, valueLoans
        returns := bodyFlow.returns
        observed := bodyFlow.observed, deaths := bodyFlow.deaths
        diagnostics := bodyFlow.diagnostics }
end

private def referenceParameters (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Array ReferenceParameterFact :=
  function.locals.take function.signature.parameters.size |>.filterMap fun localDecl => do
    let reference ← referenceType? ns localDecl.type.typeId
    some { localId := localDecl.id, kind := reference.kind, lifetime := reference.lifetime }

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

private def solvedLifetimeRelations (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) (loans : Array LoanFact) : Array LifetimeRelationFact :=
  let reflexive := Array.range ns.tables.lifetimes.size |>.map fun index =>
    { longer := ⟨index⟩, shorter := ⟨index⟩ }
  let withStatic := ns.tables.lifetimes.zipIdx.foldl (init := reflexive)
    fun relations (lifetime, index) => match lifetime.kind with
      | .static => (Array.range ns.tables.lifetimes.size).foldl
          (fun relations shorter => appendUniqueRelation relations {
            longer := ⟨index⟩, shorter := ⟨shorter⟩ }) relations
      | _ => relations
  let seeded := (declaredLifetimeRelations function ++
    reborrowLifetimeRelations ns function loans ++
    operationLifetimeRelations ns function ++
    functionBoundaryLifetimeRelations ns function).foldl appendUniqueRelation withStatic
  closeLifetimeRelations seeded (ns.tables.lifetimes.size + 1)

private def borrowAnalysis (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : BorrowFlow :=
  match function.body with
  | .absent => {}
  | .structured root =>
      let flow := analyzeExpr unit ns root {}
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
      -- A loan taken through a dereference of a reference parameter reborrows
      -- caller-owned storage and escapes with the parameter's lifetime; only
      -- loans of frame-owned storage must not outlive the frame. Move's VM
      -- also roots global resources in the invocation frame: references to
      -- those resources may not escape, even through another local holder.
      -- Reborrows through local reference variables stay conservative.
      let parameters := referenceParameters ns function
      let escapingRoot (root : LoanRoot) : Bool :=
        match root with
        | .external => true
        | .global _ => ns.profile != some .move
        | _ => false
      let frameOwned (state : BorrowState) (loan : LoanFact) : Bool :=
        let rootedInEscapingReference :=
          loan.place.projections[0]? == some .dereference &&
            loan.place.root.local?.any fun rootLocal =>
              parameters.any (·.localId == rootLocal) ||
                state.active.any fun parent =>
                  parent.expression != loan.expression &&
                    parent.holders.contains rootLocal &&
                    escapingRoot parent.place.root
        !(rootedInEscapingReference ||
          escapingRoot loan.place.root)
      let normalEscapes := match flow.normal with
        | some state => state.active.any (frameOwned state)
        | none => false
      let returnEscapes := flow.returns.any fun state =>
        state.active.any (frameOwned state)
      if (!normalEscapes && !returnEscapes) ||
          !function.signature.results.any (fun result =>
            typeMayContainReference ns result.typeId) then flow
      else
        let diagnostic := Diagnostic.at "LIR-SEMANTIC-BORROW-ESCAPE"
          "a local or Move global loan may escape through a reference-bearing function result" function.loc
        { flow with diagnostics := flow.diagnostics.push diagnostic }

/-- Run the borrow analysis once for one function: conservative conflicts and
unsupported local-loan coverage diagnostics, and the proof-facing local-loan
receipt exactly when the analysis accepts the function. -/
def borrowOutcome (namespaceId : NamespaceId) (functionId : FunctionId)
    (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) :
    Array Diagnostic × Option BorrowCertificate :=
  match function.body with
  | .absent => (#[], none)
  | .structured root =>
      let analysis := borrowAnalysis unit ns function
      if analysis.diagnostics.isEmpty then
        let relations := solvedLifetimeRelations ns function analysis.observed
        (#[], some {
          namespaceId
          functionId
          root
          parameters := referenceParameters ns function
          loans := analysis.observed.map fun loan =>
            { expression := loan.expression, lifetime := loan.lifetime
              holders := loan.holders
              deaths := analysis.deaths.filterMap fun (loanExpression, death) =>
                if loanExpression == loan.expression then some death else none }
          lifetimeRelations := relations })
      else (analysis.diagnostics, none)

end LeanerIR.Validation
