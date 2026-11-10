-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Diagnostic
import VerexIR.Validation.PlaceIndex
import VerexIR.Validation.Validated

/-!
# Definite initialization

This pass tracks whether each function local and supported static projection
is initialized on every path to an expression. It is deliberately independent
of Move/Rust ability rules: moving or dropping a supported local path consumes
it in either profile, while a later assignment or pattern binding initializes
it again.

A boolean local assigned a literal also carries what is initialized while it
holds `true`, and a branch on it assumes that. Rust's drop elaboration keeps
such a flag for each value moved on some paths only and guards the value's drop
with it. A borrowed local's later writes are not seen, so it carries nothing
from its first borrow on.
-/

namespace VerexIR.Validation

private inductive InitProjection where
  | field (name : NameId)
  | index (value : Nat)
  | downcast (variant : NameId)
  deriving BEq, Inhabited

private abbrev InitPath := Array InitProjection

private structure LocalInitialization where
  facts : Array (InitPath × Bool)
  deriving BEq, Inhabited


private def pathPrefix (candidate path : InitPath) : Bool :=
  candidate.size <= path.size && (Array.range candidate.size).all fun index =>
    candidate[index]! == path[index]!

private def pathBaseInitialized (entry : LocalInitialization) (path : InitPath) : Bool :=
  (entry.facts.foldl (init := (0, false)) fun best fact =>
    if pathPrefix fact.1 path && fact.1.size >= best.1 then (fact.1.size, fact.2) else best).2

private def pathInitialized (entry : LocalInitialization) (path : InitPath) : Bool :=
  pathBaseInitialized entry path && !entry.facts.any fun fact =>
    !fact.2 && pathPrefix path fact.1

private def setPath (entry : LocalInitialization) (path : InitPath)
    (initialized : Bool) : LocalInitialization :=
  { facts := (entry.facts.filter fun fact => !pathPrefix path fact.1).push (path, initialized) }

private def combineLocalInitialization (combine : Bool → Bool → Bool)
    (left right : LocalInitialization) : LocalInitialization :=
  let paths := (left.facts ++ right.facts).foldl (init := #[]) fun paths fact =>
    if paths.contains fact.1 then paths else paths.push fact.1
  { facts := paths.map fun path =>
      (path, combine (pathBaseInitialized left path) (pathBaseInitialized right path)) }

/-- What holds on either of two paths. -/
private def mergeLocals (left right : Array LocalInitialization) : Array LocalInitialization :=
  Array.range (min left.size right.size) |>.map fun index =>
    combineLocalInitialization (· && ·) left[index]! right[index]!

/-- What holds when both hold. -/
private def strengthenLocals (left right : Array LocalInitialization) :
    Array LocalInitialization :=
  Array.range (min left.size right.size) |>.map fun index =>
    combineLocalInitialization (· || ·) left[index]! right[index]!

/-- What the analysis knows of a local's boolean value. -/
private inductive FlagFact where
  | unknown
  /-- The local was borrowed, so a write through the borrow may change it. -/
  | escaped
  | holdsFalse
  /-- While the local holds `true`, these locals are initialized. -/
  | whenTrue (locals : Array LocalInitialization)
  deriving BEq, Inhabited

private def mergeFlag : FlagFact → FlagFact → FlagFact
  | .escaped, _ | _, .escaped => .escaped
  | .unknown, _ | _, .unknown => .unknown
  | .holdsFalse, fact | fact, .holdsFalse => fact
  | .whenTrue left, .whenTrue right => .whenTrue (mergeLocals left right)

private structure InitState where
  locals : Array LocalInitialization := #[]
  /-- May contain a value on any incoming path; unlike definite initialization, joins use OR. -/
  occupied : Array Bool := #[]
  /-- Locals whose unresolved by-value parameters may hide mutable loan carriers. -/
  genericCarriers : Array Bool := #[]
  flags : Array FlagFact := #[]
  /-- Static declarations for direct owned enum locals, never a borrowed projection. -/
  enumVariants : Array (Array (Int × String)) := #[]
  /-- A scalar local holding the tag of this still-unmodified owned enum local. -/
  tags : Array (Option LocalId) := #[]
  deriving BEq, Inhabited

private structure InitFlow where
  normal : Option InitState := none
  breaks : Array (Nat × InitState) := #[]
  continues : Array (Nat × InitState) := #[]
  diagnostics : Array Diagnostic := #[]
  deriving Inhabited

private def mergeState (left right : InitState) : InitState := {
  locals := mergeLocals left.locals right.locals
  occupied := Array.range (min left.occupied.size right.occupied.size) |>.map fun index =>
    left.occupied[index]! || right.occupied[index]!
  genericCarriers := left.genericCarriers
  flags := Array.range (min left.flags.size right.flags.size) |>.map fun index =>
    mergeFlag left.flags[index]! right.flags[index]!
  enumVariants := left.enumVariants
  tags := Array.range (min left.tags.size right.tags.size) |>.map fun index =>
    if left.tags[index]! == right.tags[index]! then left.tags[index]! else none }

private def mergeNormal : Option InitState → Option InitState → Option InitState
  | none, state | state, none => state
  | some left, some right => some (mergeState left right)

private def mergeFlows (left right : InitFlow) : InitFlow := {
  normal := mergeNormal left.normal right.normal
  breaks := left.breaks ++ right.breaks
  continues := left.continues ++ right.continues
  diagnostics := left.diagnostics ++ right.diagnostics }

private def uninitialized (loc : LocId) (localId : LocalId) : Diagnostic :=
  Diagnostic.at "VIR-SEMANTIC-INITIALIZATION"
    s!"local {localId.index} may be uninitialized at this use" loc

private def requireLocalPath (state : InitState) (loc : LocId) (localId : LocalId)
    (path : InitPath) :
    Array Diagnostic :=
  match state.locals[localId.index]? with
  | some entry => if pathInitialized entry path then #[] else #[uninitialized loc localId]
  | none => #[] -- Structural validation owns bounds errors.

private def requireLocal (state : InitState) (loc : LocId) (localId : LocalId) :
    Array Diagnostic :=
  requireLocalPath state loc localId #[]

private def requireLocalStoragePath (state : InitState) (loc : LocId) (localId : LocalId)
    (path : InitPath) : Array Diagnostic :=
  match state.locals[localId.index]? with
  | some entry => if pathBaseInitialized entry path then #[] else #[uninitialized loc localId]
  | none => #[]

/-- Change one local path's initialization, also in what each flag records
for while it holds `true`; a write to a flag forgets its value. -/
private def setLocalPath (state : InitState) (localId : LocalId) (path : InitPath)
    (initialized : Bool) : InitState :=
  -- A terminal downcast narrows the enum variant but names the same storage.
  -- Moving/writing that whole place consumes/replaces the enum, not a payload.
  let path := path.toList.reverse.dropWhile (fun
    | .downcast _ => true
    | _ => false) |>.reverse.toArray
  let update (locals : Array LocalInitialization) :=
    locals.modify localId.index (setPath · path initialized)
  { state with
    locals := update state.locals
    occupied := state.occupied.modify localId.index fun old =>
      if path.isEmpty then initialized else old || initialized
    tags := state.tags.mapIdx fun index source =>
      if index == localId.index || (path.isEmpty && source == some localId) then none else source
    flags := state.flags.mapIdx fun index fact =>
      if index == localId.index then
        if fact matches .escaped then .escaped else .unknown
      else match fact with
        | .whenTrue locals => .whenTrue (update locals)
        | fact => fact }

private def initializeLocal (state : InitState) (localId : LocalId) : InitState :=
  setLocalPath state localId #[] true

/-- The flag now holds `value`; nothing initialized changed. -/
private def setFlag (state : InitState) (flag : LocalId) (value : Bool) : InitState :=
  { state with flags := state.flags.modify flag.index fun
      | .escaped => .escaped
      | _ => if value then .whenTrue state.locals else .holdsFalse }

/-- The flag holds `value` here, as a branch on it shows. -/
private def assumeFlag (state : InitState) (flag : LocalId) (value : Bool) : InitState :=
  match state.flags[flag.index]?, value with
  | some (.whenTrue locals), true => { state with locals := strengthenLocals state.locals locals }
  | some .escaped, _ | none, _ => state
  | some _, true => state
  | some _, false => { state with flags := state.flags.set! flag.index .holdsFalse }

private def escapeLocal (state : InitState) (localId : LocalId) : InitState :=
  { state with
    flags := state.flags.modify localId.index fun _ => .escaped
    tags := state.tags.mapIdx fun index source =>
      if index == localId.index || source == some localId then none else source }

/-- No instantiated retirement certificate exists for an unresolved by-value carrier.
Inspect the post-RHS state: a self move has already vacated its destination. -/
private def genericOverwrite (state : InitState) (loc : LocId) (localId : LocalId) :
    Array Diagnostic :=
  if state.genericCarriers[localId.index]?.getD false && state.occupied[localId.index]?.getD false then
    #[Diagnostic.at "VIR-SEMANTIC-BORROW-OVERWRITE"
      "an initialized generic value may hold loans; instantiated carrier retirement is not yet supported" loc]
  else #[]

private partial def genericPatternOverwrite (ns : ValidatedNamespace) (state : InitState)
    (patternId : PatternId) : Array Diagnostic :=
  match ns.patterns[patternId.index]? with
  | none => #[]
  | some pattern => match pattern.kind with
    | .variable localId => genericOverwrite state pattern.loc localId
    | .tuple elements | .constructor _ _ _ elements =>
        elements.foldl (fun found element => found ++ genericPatternOverwrite ns state element) #[]
    | _ => #[]

private partial def initializePattern (ns : ValidatedNamespace) (state : InitState)
    (patternId : PatternId) : InitState :=
  match ns.patterns[patternId.index]? with
  | none => state
  | some pattern => match pattern.kind with
      | .variable localId => initializeLocal state localId
      | .tuple elements | .constructor _ _ _ elements =>
          elements.foldl (initializePattern ns) state
      | .wildcard | .literal _ | .range .. => state

private partial def placeRoot? (ns : ValidatedNamespace) (placeId : PlaceId)
    (fuel : Nat) : Option (LocalId × Bool) :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none => none
  | _ + 1, some (.localVar localId) => some (localId, false)
  | fuel + 1, some (.deref base) => do
      let (localId, _) ← placeRoot? ns base fuel
      some (localId, true)
  | fuel + 1, some (.field base _ _) | fuel + 1, some (.ghostField base _ _) |
      fuel + 1, some (.index base _) | fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.unsize base) | fuel + 1, some (.downcast base _) => do
      let (localId, _) ← placeRoot? ns base fuel
      some (localId, true)

private partial def localStaticPath? (ns : ValidatedNamespace) (placeId : PlaceId)
    (fuel : Nat) : Option (LocalId × InitPath) :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none => none
  | _ + 1, some (.localVar localId) => some (localId, #[])
  | fuel + 1, some (.field base _ field) | fuel + 1, some (.ghostField base _ field) => do
      let (localId, path) ← localStaticPath? ns base fuel
      some (localId, path.push (.field field))
  | fuel + 1, some (.index base indexExpression) => do
      let expression ← ns.expressions[indexExpression.index]?
      let .value (.integer value) _ := expression.kind | none
      if value < 0 then none else
        let (localId, path) ← localStaticPath? ns base fuel
        some (localId, path.push (.index value.toNat))
  | fuel + 1, some (.downcast base variant) => do
      let (localId, path) ← localStaticPath? ns base fuel
      some (localId, path.push (.downcast variant))
  | _, some (.deref _) | _, some (.subslice ..) | _, some (.unsize _) => none

private partial def ownedLocalRoot? (ns : ValidatedNamespace) (placeId : PlaceId)
    (fuel : Nat) : Option LocalId :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none => none
  | _ + 1, some (.localVar localId) => some localId
  | fuel + 1, some (.field base _ _) | fuel + 1, some (.ghostField base _ _) |
      fuel + 1, some (.index base _) | fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.unsize base) | fuel + 1, some (.downcast base _) =>
      ownedLocalRoot? ns base fuel
  | _, some (.deref _) => none

private def literalBool? (ns : ValidatedNamespace) (value : ExprId) : Option Bool :=
  match ns.expressions[value.index]? with
  | some { kind := .value (.bool literal) _, .. } => some literal
  | _ => none

/-- The local a branch condition reads whole. -/
private def conditionLocal? (ns : ValidatedNamespace) (condition : ExprId) : Option LocalId :=
  match ns.expressions[condition.index]? with
  | some { kind := .localVar localId, .. } => some localId
  | some { kind := .operation (.copy place) _ #[] _, .. }
  | some { kind := .operation (.read place) _ #[] _, .. } =>
      match ns.places[place.index]? with
      | some (.localVar localId) => some localId
      | _ => none
  | _ => none

/-- Tag provenance is limited to a direct owned local and scalar aliases.
A borrow permanently disables it: a later write through that reference is not
visible to this local initialization analysis. -/
private def tagSubject? (ns : ValidatedNamespace) (state : InitState) (id : ExprId) :
    Option LocalId := do
  let expression ← ns.expressions[id.index]?
  let source ← match expression.kind with
    | .operation (.readDiscriminant place) #[] #[] _ => do
        let .localVar subject ← ns.places[place.index]? | none
        if (state.enumVariants[subject.index]?.getD #[]).isEmpty then none else some subject
    | .localVar tag => state.tags[tag.index]?.join
    | .operation (.copy place) #[] #[] _ | .operation (.read place) #[] #[] _ |
        .operation (.move place) #[] #[] _ => do
        let .localVar tag ← ns.places[place.index]? | none
        state.tags[tag.index]?.join
    | _ => none
  if state.flags[source.index]? == some .escaped then none else some source

private def setTag (state : InitState) (target : LocalId) (source : Option LocalId) : InitState :=
  { state with tags := state.tags.modify target.index fun _ =>
      if state.flags[target.index]? == some .escaped then none else source }

/-- A proven tag value excludes payload paths of different variants. Never
remove the root fact, a field path without a root downcast, or a path of the
selected variant. Unknown/default patterns and duplicate discriminants do not
refine anything. -/
private def assumeTagPattern (ns : ValidatedNamespace) (state : InitState)
    (source : Option LocalId) (pattern : PatternId) : InitState := Id.run do
  let some source := source | return state
  let some { kind := .literal (.integer value), .. } := ns.patterns[pattern.index]?
    | return state
  let variants := state.enumVariants[source.index]?.getD #[]
  let matching := variants.filter (·.1 == value)
  if matching.size != 1 then return state
  let selected := matching[0]!.2
  return { state with locals := state.locals.modify source.index fun entry =>
    { facts := entry.facts.filter fun (fact : InitPath × Bool) => match (fact.1)[0]? with
      | some (InitProjection.downcast variant) => match ns.nameOf variant with
          | some name => name.name == selected || !variants.any (·.2 == name.name)
          | none => true
      | _ => true } }

private partial def placeIndexDiagnostics (ns : ValidatedNamespace) (state : InitState)
    (loc : LocId) (placeId : PlaceId) (fuel : Nat) : Array Diagnostic :=
  match fuel, ns.places[placeId.index]? with
  | 0, _ | _, none | _, some (.localVar _) => #[]
  | fuel + 1, some (.deref base) | fuel + 1, some (.field base _ _) |
      fuel + 1, some (.ghostField base _ _) | fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.unsize base) | fuel + 1, some (.downcast base _) =>
      placeIndexDiagnostics ns state loc base fuel
  | fuel + 1, some (.index base index) =>
      let baseErrors := placeIndexDiagnostics ns state loc base fuel
      match placeIndexForm? ns index with
      | some (.local localId) | some (.copyLocal localId) =>
          baseErrors ++ requireLocal state loc localId
      | some (.fromEnd sourcePlace _) =>
          match placeRoot? ns sourcePlace (ns.places.size + 1) with
              | some (localId, _) => baseErrors ++ requireLocal state loc localId
              | none => baseErrors
      | _ => baseErrors

private def readPlace (ns : ValidatedNamespace) (state : InitState) (loc : LocId)
    (placeId : PlaceId) : Array Diagnostic :=
  let indexErrors := placeIndexDiagnostics ns state loc placeId (ns.places.size + 1)
  match localStaticPath? ns placeId (ns.places.size + 1) with
  | some (localId, path) => requireLocalPath state loc localId path ++ indexErrors
  | none => match placeRoot? ns placeId (ns.places.size + 1) with
      | some (localId, _) => requireLocal state loc localId ++ indexErrors
      | none => indexErrors

/-- A tag lives in enum storage independently of its payload initialization.
Dynamic projections and dereferences still require their root value, including
any reference used to reach the storage, to be initialized. -/
private def readDiscriminantPlace (ns : ValidatedNamespace) (state : InitState) (loc : LocId)
    (placeId : PlaceId) : Array Diagnostic :=
  let indexErrors := placeIndexDiagnostics ns state loc placeId (ns.places.size + 1)
  match localStaticPath? ns placeId (ns.places.size + 1) with
  | some (localId, path) => requireLocalStoragePath state loc localId path ++ indexErrors
  | none => readPlace ns state loc placeId

private def unresolvedCarrier (ns : ValidatedNamespace) : Nat → TypeId → Bool
  | 0, _ => true
  | fuel + 1, typeId => match ns.tables.types[typeId.index]? with
    | some (.typeParameter _) => true
    | some (.tuple fields) => fields.any (unresolvedCarrier ns fuel)
    | some (.vector element _) | some (.slice element) => unresolvedCarrier ns fuel element
    | some (.nominal _ arguments) => arguments.any fun
        | .typeArg use => unresolvedCarrier ns fuel use.typeId
        | _ => false
    | _ => false

/-- Static assignment typing equates the RHS and destination types. Inspecting
that type avoids mistaking a scalar field of `Generic<T>` for the whole carrier.
Occupancy remains conservative for generic fields: a partial move does not prove
that every hidden loan in the owning aggregate is absent. -/
private def genericWriteOverwrite (ns : ValidatedNamespace) (state : InitState)
    (loc : LocId) (localId : LocalId) (value : Option ExprId) : Array Diagnostic :=
  let unknown := match value.bind (fun id => ns.expressions[id.index]?) with
    | some expression => unresolvedCarrier ns (ns.tables.types.size + 1) expression.typeId
    | none => true
  if unknown then genericOverwrite state loc localId else #[]

private def writePlace (ns : ValidatedNamespace) (state : InitState) (loc : LocId)
    (placeId : PlaceId) (value : Option ExprId) : InitState × Array Diagnostic :=
  let indexErrors := placeIndexDiagnostics ns state loc placeId (ns.places.size + 1)
  match localStaticPath? ns placeId (ns.places.size + 1) with
  | some (localId, path) =>
      let storageErrors := if path.isEmpty then #[] else
        requireLocalStoragePath state loc localId path.pop
      (setLocalPath state localId path true, storageErrors ++ indexErrors ++
        genericWriteOverwrite ns state loc localId value)
  | none => match placeRoot? ns placeId (ns.places.size + 1) with
      | some (localId, _) => (state, requireLocal state loc localId ++ indexErrors ++
          genericWriteOverwrite ns state loc localId value)
      | none => (state, indexErrors)

mutual
  private partial def analyzeExpr (ns : ValidatedNamespace) (exprId : ExprId)
      (state : InitState) : InitFlow :=
    match ns.expressions[exprId.index]? with
    | none => { normal := some state }
    | some expression => match expression.kind with
        | .value .. | .constant _ | .spec _ | .quantifier .. =>
            { normal := some state }
        | .localVar localId => {
            normal := some state
            diagnostics := requireLocal state expression.loc localId }
        | .operation operation _ arguments _ =>
            let operands := analyzeExprList ns arguments.toList state
            match operands.normal with
            | none => operands
            | some afterOperands =>
                let (normal, errors) := match operation with
                  | .move place | .drop place =>
                      match localStaticPath? ns place (ns.places.size + 1) with
                      | some (localId, path) =>
                          (some (setLocalPath afterOperands localId path false),
                            readPlace ns afterOperands expression.loc place)
                      | none => match ownedLocalRoot? ns place (ns.places.size + 1) with
                          | some localId =>
                              (some (setLocalPath afterOperands localId #[] false),
                                readPlace ns afterOperands expression.loc place)
                          | none => (some afterOperands,
                              readPlace ns afterOperands expression.loc place)
                  | .readDiscriminant place =>
                      (some afterOperands, readDiscriminantPlace ns afterOperands expression.loc place)
                  | .copy place | .read place =>
                      (some afterOperands, readPlace ns afterOperands expression.loc place)
                  | .borrow _ place =>
                      let escaped := match placeRoot? ns place (ns.places.size + 1) with
                        | some (localId, _) => escapeLocal afterOperands localId
                        | none => afterOperands
                      (some escaped, readPlace ns afterOperands expression.loc place)
                  | .write place =>
                      let (afterWrite, errors) :=
                        writePlace ns afterOperands expression.loc place arguments[0]?
                      let afterWrite := match ns.places[place.index]? with
                        | some (.localVar target) =>
                            setTag afterWrite target (arguments[0]?.bind (tagSubject? ns state))
                        | _ => afterWrite
                      (some afterWrite, errors)
                  | _ => (some afterOperands, #[])
                { operands with normal, diagnostics := operands.diagnostics ++ errors }
        | .block statements result =>
            let statementsFlow := analyzeExprList ns statements.toList state
            match statementsFlow.normal, result with
            | none, _ | some _, none => statementsFlow
            | some afterStatements, some result =>
                let resultFlow := analyzeExpr ns result afterStatements
                { resultFlow with
                  breaks := statementsFlow.breaks ++ resultFlow.breaks
                  continues := statementsFlow.continues ++ resultFlow.continues
                  diagnostics := statementsFlow.diagnostics ++ resultFlow.diagnostics }
        | .letDecl pattern initializer body =>
            let initialized : InitFlow := match initializer with
              | none => { normal := some state }
              | some value => analyzeExpr ns value state
            match initialized.normal with
            | none => initialized
            | some afterInitializer =>
                let bodyState := if initializer.isSome then
                    initializePattern ns afterInitializer pattern
                  else afterInitializer
                let bodyState :=
                  match ns.patterns[pattern.index]?, initializer.bind (literalBool? ns) with
                  | some { kind := .variable flag, .. }, some literal =>
                      setFlag bodyState flag literal
                  | _, _ => bodyState
                let bodyState := match ns.patterns[pattern.index]? with
                  | some { kind := .variable target, .. } =>
                      setTag bodyState target (initializer.bind (tagSubject? ns state))
                  | _ => bodyState
                let bodyFlow := analyzeExpr ns body bodyState
                { bodyFlow with
                  breaks := initialized.breaks ++ bodyFlow.breaks
                  continues := initialized.continues ++ bodyFlow.continues
                  diagnostics := initialized.diagnostics ++
                    (if initializer.isSome then genericPatternOverwrite ns afterInitializer pattern else #[]) ++
                    bodyFlow.diagnostics }
        | .ifElse condition thenBranch elseBranch =>
            let conditionFlow := analyzeExpr ns condition state
            match conditionFlow.normal with
            | none => conditionFlow
            | some branchState =>
                let (thenState, elseState) := match conditionLocal? ns condition with
                  | some flag => (assumeFlag branchState flag true, assumeFlag branchState flag false)
                  | none => (branchState, branchState)
                let thenFlow := analyzeExpr ns thenBranch thenState
                let elseFlow := match elseBranch with
                  | some branch => analyzeExpr ns branch elseState
                  | none => { normal := some elseState }
                let branches := mergeFlows thenFlow elseFlow
                { branches with
                  breaks := conditionFlow.breaks ++ branches.breaks
                  continues := conditionFlow.continues ++ branches.continues
                  diagnostics := conditionFlow.diagnostics ++ branches.diagnostics }
        | .match_ scrutinee arms =>
            let scrutineeFlow := analyzeExpr ns scrutinee state
            match scrutineeFlow.normal with
            | none => scrutineeFlow
            | some armState =>
                let armsFlow := arms.foldl (fun flow arm =>
                  mergeFlows flow (analyzeArm ns arm
                    (assumeTagPattern ns armState (tagSubject? ns state scrutinee) arm.pattern))) {}
                { armsFlow with
                  breaks := scrutineeFlow.breaks ++ armsFlow.breaks
                  continues := scrutineeFlow.continues ++ armsFlow.continues
                  diagnostics := scrutineeFlow.diagnostics ++ armsFlow.diagnostics }
        | .loop _ body => analyzeLoop ns body state state
            ((state.locals.size + 1) * (state.flags.size + 4))
        | .break_ nest value =>
            let valueFlow := match value with
              | some value => analyzeExpr ns value state
              | none => { normal := some state }
            let newBreaks := match valueFlow.normal with
              | some afterValue => valueFlow.breaks.push (nest, afterValue)
              | none => valueFlow.breaks
            { valueFlow with normal := none, breaks := newBreaks }
        | .continue_ nest => { continues := #[(nest, state)] }
        | .return_ values | .throw_ _ values =>
            let valuesFlow := analyzeExprList ns values.toList state
            { valuesFlow with normal := none }
        | .assign place value =>
            let valueFlow := analyzeExpr ns value state
            match valueFlow.normal with
            | none => valueFlow
            | some afterValue =>
                let (afterWrite, errors) := writePlace ns afterValue expression.loc place (some value)
                let afterWrite := match ns.places[place.index]?, literalBool? ns value with
                  | some (.localVar flag), some literal => setFlag afterWrite flag literal
                  | _, _ => afterWrite
                let afterWrite := match ns.places[place.index]? with
                  | some (.localVar target) => setTag afterWrite target (tagSubject? ns state value)
                  | _ => afterWrite
                { normal := some afterWrite
                  breaks := valueFlow.breaks
                  continues := valueFlow.continues
                  diagnostics := valueFlow.diagnostics ++ errors }
        | .assignPattern pattern value =>
            let valueFlow := analyzeExpr ns value state
            match valueFlow.normal with
            | none => valueFlow
            | some afterValue =>
                let initialized := initializePattern ns afterValue pattern
                let initialized := match ns.patterns[pattern.index]? with
                  | some { kind := .variable target, .. } =>
                      setTag initialized target (tagSubject? ns state value)
                  | _ => initialized
                { valueFlow with
                  normal := some initialized
                  diagnostics := valueFlow.diagnostics ++ genericPatternOverwrite ns afterValue pattern }

  private partial def analyzeExprList (ns : ValidatedNamespace) (expressions : List ExprId)
      (state : InitState) : InitFlow :=
    match expressions with
    | [] => { normal := some state }
    | expression :: tail =>
        let head := analyzeExpr ns expression state
        match head.normal with
        | none => head
        | some afterHead =>
            let rest := analyzeExprList ns tail afterHead
            { rest with
              breaks := head.breaks ++ rest.breaks
              continues := head.continues ++ rest.continues
              diagnostics := head.diagnostics ++ rest.diagnostics }

  private partial def analyzeArm (ns : ValidatedNamespace) (arm : MatchArm)
      (state : InitState) : InitFlow :=
    let patternState := initializePattern ns state arm.pattern
    let errors := genericPatternOverwrite ns state arm.pattern
    match arm.guard with
    | none =>
        let flow := analyzeExpr ns arm.body patternState
        { flow with diagnostics := errors ++ flow.diagnostics }
    | some guard =>
        let guardFlow := analyzeExpr ns guard patternState
        match guardFlow.normal with
        | none => { guardFlow with diagnostics := errors ++ guardFlow.diagnostics }
        | some afterGuard =>
            let bodyFlow := analyzeExpr ns arm.body afterGuard
            { bodyFlow with
              breaks := guardFlow.breaks ++ bodyFlow.breaks
              continues := guardFlow.continues ++ bodyFlow.continues
              diagnostics := errors ++ guardFlow.diagnostics ++ bodyFlow.diagnostics }

  private partial def analyzeLoop (ns : ValidatedNamespace) (body : ExprId)
      (initial entry : InitState) (fuel : Nat := 0) : InitFlow :=
    let bodyFlow := analyzeExpr ns body entry
    let reentryStates :=
      (match bodyFlow.normal with | some state => #[state] | none => #[]) ++
        (bodyFlow.continues.filterMap fun (nest, state) =>
          if nest == 0 then some state else none)
    let nextEntry := reentryStates.foldl mergeState initial
    if fuel > 0 && nextEntry != entry then
      analyzeLoop ns body initial nextEntry (fuel - 1)
    else
      let exits := bodyFlow.breaks.filterMap fun (nest, state) =>
        if nest == 0 then some state else none
      let normal := exits.foldl (fun result state => mergeNormal result (some state)) none
      let outerBreaks := bodyFlow.breaks.filterMap fun (nest, state) =>
        match nest with | 0 => none | nest + 1 => some (nest, state)
      let outerContinues := bodyFlow.continues.filterMap fun (nest, state) =>
        match nest with | 0 => none | nest + 1 => some (nest, state)
      let convergenceErrors := if nextEntry == entry then #[] else
        #[Diagnostic.at "VIR-SEMANTIC-INITIALIZATION-FIXPOINT"
          "loop initialization analysis did not reach a stable state"
          ((ns.expressions[body.index]?).map (·.loc) |>.getD ⟨0⟩)]
      { normal, breaks := outerBreaks, continues := outerContinues
        diagnostics := bodyFlow.diagnostics ++ convergenceErrors }
end

-- Force exhaustion of the private loop engine without adding a public testing
-- parameter or increasing its budget. A stable zero-budget iteration remains valid.
#guard (let ns : ValidatedNamespace := { (default : ValidatedNamespace) with
    places := #[.localVar ⟨0⟩]
    expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation (.move ⟨0⟩) #[] #[] }] }
  let state : InitState := { locals := #[{ facts := #[(#[], true)] }], occupied := #[true] }
  (analyzeLoop ns ⟨0⟩ state state 0).diagnostics.any fun diagnostic =>
    diagnostic.code == "VIR-SEMANTIC-INITIALIZATION-FIXPOINT")
#guard (let ns : ValidatedNamespace := { (default : ValidatedNamespace) with
    expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value .unit }] }
  (analyzeLoop ns ⟨0⟩ {} {} 0).diagnostics.isEmpty)

/-- Whether structured evaluation can complete an expression normally. This
reuses the control-flow component of definite-initialization analysis; local
state does not affect whether a return, throw, break, or continue is abrupt. -/
def expressionCanFallThrough (ns : ValidatedNamespace) (expression : ExprId) : Bool :=
  (analyzeExpr ns expression {}).normal.isSome

/-- Whether a loop body returns to the loop's head on no path: it neither
completes normally nor continues the loop, so the loop runs it once. -/
def loopBodyRunsOnce (ns : ValidatedNamespace) (body : ExprId) : Bool :=
  let flow := analyzeExpr ns body {}
  flow.normal.isNone && !flow.continues.any (·.1 == 0)

/-- The declared discriminants of a direct local enum. Ref/aggregate paths
are deliberately excluded from tag provenance. -/
private def localEnumVariants (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (localDecl : LocalDecl) : Array (Int × String) := (do
  let .nominal name _ ← unit.tables.types[(ns.typeId localDecl.type.typeId).index]? | none
  let declarationId ← unit.resolution.nominal? name
  let qualified ← unit.tables.names[name.index]?
  let targetNs ← unit.namespaces[qualified.namespaceId.index]?
  let declaration ← targetNs.structs[declarationId.index]?
  declaration.variants.mapM fun (variant : VariantDecl) => do
    let value ← variant.discriminant
    let name ← targetNs.nameOf variant.name
    pure (value, name.name)).getD #[]

/-- Run definite-initialization once for one function: diagnose local reads
which are not definitely initialized on every structured path reaching them,
and construct the receipt exactly when the analysis accepts the body. Function
parameters initialize the leading locals, matching `initialFrame?`; all
remaining locals start uninitialized. Absent bodies have no checked root. -/
def initializationOutcome (namespaceId : NamespaceId) (functionId : FunctionId)
    (unit : ValidatedUnit) (ns : ValidatedNamespace) (function : FunctionDecl FunctionBody) :
    Array Diagnostic × Option InitializationCertificate :=
  match function.body with
  | .absent => (#[], none)
  | .structured root =>
      let state : InitState := {
        locals := Array.range function.locals.size |>.map fun index =>
          { facts := #[(#[], decide (index < function.signature.parameters.size))] }
        occupied := Array.range function.locals.size |>.map fun index =>
          decide (index < function.signature.parameters.size)
        genericCarriers := function.locals.map fun localDecl =>
          ns.profile != some .move && unresolvedCarrier ns (ns.tables.types.size + 1) localDecl.type.typeId
        flags := Array.replicate function.locals.size .unknown
        enumVariants := function.locals.map (localEnumVariants unit ns)
        tags := Array.replicate function.locals.size none }
      let diagnostics := (analyzeExpr ns root state).diagnostics
      if diagnostics.isEmpty then
        (#[], some {
          namespaceId
          functionId
          root
          parameterLocals := function.locals.take function.signature.parameters.size |>.map (·.id)
          localCount := function.locals.size })
      else (diagnostics, none)

end VerexIR.Validation
