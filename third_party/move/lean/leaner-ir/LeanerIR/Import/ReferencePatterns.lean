-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Import.Raw

/-!
# Patterns through references

A constructor pattern matched against a reference binds references to the
fields it names, `let S { f } = &mut s` a `&mut` to `s.f`. LIR's patterns
match values, so a frontend's patterns through references are rewritten here,
before validation, into what Move's compiler emits for them: tests of the
variants on the way (`TestVariant`) and borrows of the fields the pattern
binds (`BorrowField`, `BorrowVariantField`).

- A `let` or an assignment tests its variants and aborts with the profile's
  mismatch throw where one fails, then borrows its binders.
- A `match` tries its arms in order: an arm is taken where its variants
  hold and, with its binders borrowed, its guard holds; its body runs with its
  binders borrowed. The last unguarded arm of a match is taken without a
  test, as the match is exhaustive.

The borrows start at the place the reference borrows, when the scrutinee
borrows a local or reads a reference local, and otherwise at a local the
scrutinee is bound to. A field several variants name at different types is
reached through a downcast to the variant.
-/

namespace LeanerIR.Import

/-- The arenas and the locals of the function a rewrite extends. -/
structure ReferencePatternState where
  types : Array Ty
  expressions : Array Expr
  patterns : Array Pattern
  places : Array Place
  locals : Array LocalDecl
  deriving Inhabited

private abbrev RefPatM := StateT ReferencePatternState (Except String)

/-- The declarations a rewrite reads. -/
private structure RefPatContext where
  unit : RawUnit
  profile : Option Profile

private def pushExpr (loc : LocId) (typeId : TypeId) (kind : ExprKind) : RefPatM ExprId :=
  modifyGet fun state =>
    (⟨state.expressions.size⟩, { state with expressions := state.expressions.push { loc, typeId, kind } })

private def pushPlace (place : Place) : RefPatM PlaceId :=
  modifyGet fun state => (⟨state.places.size⟩, { state with places := state.places.push place })

private def pushPattern (loc : LocId) (typeId : TypeId) (kind : PatternKind) : RefPatM PatternId :=
  modifyGet fun state =>
    (⟨state.patterns.size⟩, { state with patterns := state.patterns.push { loc, typeId, kind } })

private def internType (type : Ty) : RefPatM TypeId := do
  match (← get).types.findIdx? (· == type) with
  | some index => pure ⟨index⟩
  | none => modifyGet fun state => (⟨state.types.size⟩, { state with types := state.types.push type })

private def exprAt (id : ExprId) : RefPatM Expr := do
  let some expression := (← get).expressions[id.index]? | throw "an expression is out of range"
  pure expression

private def patternAt (id : PatternId) : RefPatM Pattern := do
  let some pattern := (← get).patterns[id.index]? | throw "a pattern is out of range"
  pure pattern

private def setKind (id : ExprId) (kind : ExprKind) : RefPatM Unit :=
  modify fun state =>
    { state with expressions := state.expressions.modify id.index ({ · with kind }) }

/-- The kind and referent of a reference type. -/
private def referenceOf? (id : TypeId) : RefPatM (Option ReferenceType) := do
  match (← get).types[id.index]? with
  | some (.reference reference) => pure (some reference)
  | _ => pure none

/-- The owner and declaration a constructor names. -/
private def declarationOf (context : RefPatContext) (name : NameId) :
    Except String (QualifiedRef × StructDecl) := do
  let some qualified := context.unit.tables.names[name.index]?
    | throw "a constructor names a missing declaration"
  let owner : QualifiedRef := { namespaceId := qualified.namespaceId, name }
  let local? := (context.unit.namespaces.find? (·.identity == qualified.namespaceId)).bind
    fun ns => ns.structs.find? (·.name == name)
  let dependency? := (context.unit.dependencies.find? (·.namespaceId == qualified.namespaceId)).bind
    fun ns => ns.structs.find? (·.name == name)
  match local?.orElse fun _ => dependency? with
  | some declaration => pure (owner, declaration)
  | none => throw s!"the declaration of `{qualified.name}` is not part of the unit"

private def nameText (context : RefPatContext) (name : NameId) : Except String String :=
  match context.unit.tables.names[name.index]? with
  | some qualified => pure qualified.name
  | none => throw "a name is out of range"

/-- One field step of a path: its owner and field, the variants of an enum a
selection of the field lists, the variant it is reached through when its
name is ambiguous, and the type of the field's value. -/
private structure FieldStep where
  owner : QualifiedRef
  field : NameId
  fieldText : String
  variants : Array String
  downcast : Option NameId
  valueType : TypeId

/-- Where a path starts: a local holding the value, or a local holding a
reference to it. -/
private inductive PathRoot where
  | owned (localId : LocalId) (valueType : TypeId)
  | through (localId : LocalId) (referenceType : TypeId) (valueType : TypeId)

private structure Path where
  root : PathRoot
  steps : Array FieldStep := #[]
  loc : LocId

/-- A step of matching a pattern through a reference. -/
private inductive MatchStep where
  /-- The value on the path holds the variant. -/
  | test (path : Path) (owner : QualifiedRef) (variant : String)
  /-- The value on the path equals a literal. -/
  | literal (path : Path) (value : ConstValue) (valueType : TypeId)
  /-- The local borrows the place of the path. -/
  | bind (localId : LocalId) (kind : BorrowKind) (path : Path) (referenceType : TypeId)

/-- The place a path names. -/
private def pathPlace (path : Path) : RefPatM PlaceId := do
  let mut place ← match path.root with
    | .owned localId _ => pushPlace (.localVar localId)
    | .through localId _ _ => do pushPlace (.deref (← pushPlace (.localVar localId)))
  for step in path.steps do
    let base ← match step.downcast with
      | some variant => pushPlace (.downcast place variant)
      | none => pure place
    place ← pushPlace (.field base step.owner step.field)
  pure place

/-- The value a path names, read the way a source spells it: the local, the
referent of the reference, and the selections of its fields. -/
private def pathValue (path : Path) : RefPatM ExprId := do
  let mut value ← match path.root with
    | .owned localId valueType => pushExpr path.loc valueType (.localVar localId)
    | .through localId referenceType valueType => do
        let reference ← pushExpr path.loc referenceType (.localVar localId)
        pushExpr path.loc valueType (.operation (.reference .dereference) #[] #[reference])
  for step in path.steps do
    let operation : DataOperation := if step.variants.isEmpty then
        .select step.owner step.fieldText
      else .selectVariants step.owner (step.variants.map (·, step.fieldText))
    value ← pushExpr path.loc step.valueType (.operation (.data operation) #[] #[value])
  pure value

/-- Whether every variant naming a field gives it one type. -/
private def uniformField (declaration : StructDecl) (field : FieldDecl) : Bool :=
  declaration.variants.all fun variant => variant.fields.all fun other =>
    other.name != field.name || other.type.typeId == field.type.typeId

/-- The steps of matching a pattern through a reference at a path. -/
private partial def planPattern (context : RefPatContext) (id : PatternId) (path : Path) :
    RefPatM (Array MatchStep) := do
  let pattern ← patternAt id
  match pattern.kind with
  | .wildcard => pure #[]
  | .variable localId =>
      let some reference ← referenceOf? pattern.typeId
        | throw "a binder of a pattern through a reference is not a reference"
      let kind : BorrowKind := if reference.kind == .mutable then .mutable else .immutable
      pure #[.bind localId kind path pattern.typeId]
  | .literal value =>
      let some reference ← referenceOf? pattern.typeId
        | throw "a literal of a pattern through a reference is not typed by a reference"
      pure #[.literal path value reference.referent]
  | .constructor name _ variant children => do
      let (owner, declaration) ← liftM (declarationOf context name)
      let variantDecl? ← match variant with
        | none => pure none
        | some text => do
            let some variantDecl ← declaration.variants.findM? fun candidate => do
                pure ((← liftM (nameText context candidate.name)) == text)
              | throw s!"a pattern names an unknown variant `{text}`"
            pure (some variantDecl)
      let (fields, test, enum) := match variant, variantDecl? with
        | some text, some variantDecl =>
            (variantDecl.fields, #[MatchStep.test path owner text], true)
        | _, _ => (declaration.fields, #[], false)
      unless fields.size == children.size do
        throw "a constructor pattern's arity differs from its declaration's"
      let variantName := variantDecl?.map (·.name)
      let mut steps := test
      for (child, field) in children.zip fields do
        let childPattern ← patternAt child
        let some reference ← referenceOf? childPattern.typeId
          | throw "a part of a pattern through a reference is not typed by a reference"
        let downcast := if enum && !uniformField declaration field then variantName else none
        -- A field every variant naming it gives one type is selected across
        -- them, another one of the variant matched.
        let variants ← if !enum then pure #[]
          else if downcast.isNone then
            declaration.variants.filterMapM fun candidate => do
              if candidate.fields.any (·.name == field.name) then
                pure (some (← liftM (nameText context candidate.name)))
              else pure none
          else pure (variant.toArray)
        let step : FieldStep := {
          owner, field := field.name, fieldText := ← liftM (nameText context field.name),
          variants, downcast, valueType := reference.referent }
        steps := steps ++ (← planPattern context child { path with steps := path.steps.push step })
      pure steps
  | .tuple _ | .range .. => throw "a tuple or range pattern through a reference"

/-- Whether a pattern matches through a reference. -/
private def throughReference (id : PatternId) : RefPatM Bool := do
  let pattern ← patternAt id
  match pattern.kind with
  | .constructor .. | .literal _ => return (← referenceOf? pattern.typeId).isSome
  | _ => return false

/-- A fresh local of a type, named apart from the function's locals. -/
private def freshLocal (type : TypeId) (loc : LocId) : RefPatM LocalId := do
  let locals := (← get).locals
  let mut index := 0
  while locals.any (·.name == s!"_ref{index}") do index := index + 1
  let localId : LocalId := ⟨locals.size⟩
  let declaration : LocalDecl :=
    { id := localId, name := s!"_ref{index}", type := { typeId := type, loc }, loc }
  modify fun state => { state with locals := state.locals.push declaration }
  pure localId

/-- Where the paths of a scrutinee start, and the binding of the scrutinee
to a local when there is no place to start from. -/
private def scrutineeRoot (scrutinee : ExprId) : RefPatM (PathRoot × Option (PatternId × ExprId)) := do
  let expression ← exprAt scrutinee
  let some reference ← referenceOf? expression.typeId
    | throw "a pattern through a reference matches a value that is not a reference"
  let state ← get
  let localPlace? (place : PlaceId) : Option LocalId :=
    match state.places[place.index]? with
    | some (.localVar localId) => some localId
    | _ => none
  match expression.kind with
  | .operation (.borrow _ place) _ #[] _ =>
      if let some localId := localPlace? place then
        return (.owned localId reference.referent, none)
  | .localVar localId => return (.through localId expression.typeId reference.referent, none)
  | .operation (.copy place) _ #[] _ | .operation (.move place) _ #[] _ =>
      if let some localId := localPlace? place then
        return (.through localId expression.typeId reference.referent, none)
  | _ => pure ()
  let localId ← freshLocal expression.typeId expression.loc
  let pattern ← pushPattern expression.loc expression.typeId (.variable localId)
  return (.through localId expression.typeId reference.referent, some (pattern, scrutinee))

/-- The Boolean expression of a test. -/
private def testExpr (boolType : TypeId) (step : MatchStep) : RefPatM (Option ExprId) := do
  match step with
  | .test path owner variant =>
      let value ← pathValue path
      some <$> pushExpr path.loc boolType
        (.operation (.data (.testVariants owner #[variant])) #[] #[value])
  | .literal path literal valueType =>
      let value ← pathValue path
      let expected ← pushExpr path.loc valueType (.value literal)
      some <$> pushExpr path.loc boolType (.operation (.primitive .equal) #[] #[value, expected])
  | .bind .. => pure none

/-- The conjunction of a match's tests, short-circuiting; `none` when there
are none. -/
private def conjunction (boolType : TypeId) (loc : LocId) (steps : Array MatchStep) :
    RefPatM (Option ExprId) := do
  let mut tests := #[]
  for step in steps do
    if let some test ← testExpr boolType step then tests := tests.push test
  let some last := tests.back? | return none
  let mut result := last
  for test in tests.pop.reverse do
    let false_ ← pushExpr loc boolType (.value (.bool false))
    result ← pushExpr loc boolType (.ifElse test result (some false_))
  return some result

/-- Wrap a body in the borrows binding a match's locals. -/
private def withBinds (steps : Array MatchStep) (body : ExprId) : RefPatM ExprId := do
  let bodyNode ← exprAt body
  let mut result := body
  for step in steps.reverse do
    if let .bind localId kind path referenceType := step then
      let place ← pathPlace path
      let borrow ← pushExpr path.loc referenceType (.operation (.borrow kind place) #[] #[])
      let pattern ← pushPattern path.loc referenceType (.variable localId)
      result ← pushExpr bodyNode.loc bodyNode.typeId (.letDecl pattern (some borrow) result)
  pure result

/-- The throw of a pattern mismatch: the profile's (`patternMismatch?`). -/
private def mismatchThrow (context : RefPatContext) (loc : LocId) : RefPatM ExprId := do
  let some (kind, codes) := patternMismatch? context.profile
    | throw "a pattern through a reference that may not match, in a profile without a mismatch throw"
  let codeType ← internType (.integer (.bits 64) false)
  let arguments ← codes.mapM fun code => pushExpr loc codeType (.value (.integer code))
  pushExpr loc (← internType .never) (.throw_ kind arguments)

/-- The statement making the mismatch throw unless the tests hold; `none`
without tests. -/
private def mismatchCheck (context : RefPatContext) (loc : LocId) (steps : Array MatchStep) :
    RefPatM (Option ExprId) := do
  let boolType ← internType .bool
  let some condition ← conjunction boolType loc steps | return none
  let negated ← pushExpr loc boolType (.operation (.primitive .logicalNot) #[] #[condition])
  let thrown ← mismatchThrow context loc
  some <$> pushExpr loc (← internType .unit) (.ifElse negated thrown none)

/-- Bind a scrutinee to its local around a rewritten node, when it needs one. -/
private def bindScrutinee (id : ExprId) (binding : Option (PatternId × ExprId))
    (kind : ExprKind) : RefPatM Unit := do
  match binding with
  | none => setKind id kind
  | some (pattern, scrutinee) =>
      let node ← exprAt id
      let inner ← pushExpr node.loc node.typeId kind
      setKind id (.letDecl pattern (some scrutinee) inner)

/-- The borrow statements of an assignment through a reference. -/
private def assignBinds (steps : Array MatchStep) : RefPatM (Array ExprId) := do
  let unitType ← internType .unit
  let mut statements := #[]
  for step in steps do
    if let .bind localId kind path referenceType := step then
      let place ← pathPlace path
      let borrow ← pushExpr path.loc referenceType (.operation (.borrow kind place) #[] #[])
      let target ← pushPlace (.localVar localId)
      statements := statements.push (← pushExpr path.loc unitType (.assign target borrow))
  pure statements

/-- Rewrite one node holding a pattern through a reference. -/
private def rewriteNode (context : RefPatContext) (id : ExprId) : RefPatM Unit := do
  let node ← exprAt id
  match node.kind with
  | .letDecl pattern (some value) body =>
      unless ← throughReference pattern do return
      let (root, binding) ← scrutineeRoot value
      let steps ← planPattern context pattern { root, loc := node.loc }
      let bound ← withBinds steps body
      let kind ← match ← mismatchCheck context node.loc steps with
        | some check => pure (ExprKind.block #[check] (some bound))
        | none => pure (← exprAt bound).kind
      bindScrutinee id binding kind
  | .assignPattern pattern value =>
      unless ← throughReference pattern do return
      let (root, binding) ← scrutineeRoot value
      let steps ← planPattern context pattern { root, loc := node.loc }
      let check ← mismatchCheck context node.loc steps
      bindScrutinee id binding (.block (check.toArray ++ (← assignBinds steps)) none)
  | .match_ scrutinee arms =>
      unless ← arms.anyM (throughReference ·.pattern) do return
      let (root, binding) ← scrutineeRoot scrutinee
      let boolType ← internType .bool
      -- From the last arm: an arm is tried where the later ones are its
      -- fallback; the last unguarded arm is taken untested.
      let mut next : Option ExprId := none
      for arm in arms.reverse do
        let steps ← planPattern context arm.pattern { root, loc := node.loc }
        let body ← withBinds steps arm.body
        let tests ← conjunction boolType node.loc steps
        let guard ← arm.guard.mapM (withBinds steps ·)
        let condition ← match tests, guard with
          | some tests, some guard => do
              let false_ ← pushExpr node.loc boolType (.value (.bool false))
              some <$> pushExpr node.loc boolType (.ifElse tests guard (some false_))
          | some tests, none => pure (some tests)
          | none, guard => pure guard
        next := some <| ← match next, condition, arm.guard with
          | none, _, none => pure body
          | none, some condition, some _ => do
              let thrown ← mismatchThrow context node.loc
              pushExpr node.loc node.typeId (.ifElse condition body (some thrown))
          | _, none, _ => pure body
          | some fallback, some condition, _ =>
              pushExpr node.loc node.typeId (.ifElse condition body (some fallback))
      let some result := next | throw "a match has no arms"
      bindScrutinee id binding (← exprAt result).kind
  | _ => pure ()

/-- The expressions of a body, in an order that visits every node once. -/
private partial def bodyNodes (expressions : Array Expr) (root : ExprId) (found : Array ExprId) :
    Array ExprId :=
  match expressions[root.index]? with
  | none => found
  | some expression =>
      let found := found.push root
      let children : Array ExprId := match expression.kind with
        | .operation _ _ arguments _ => arguments
        | .block statements result => statements ++ result.toArray
        | .letDecl _ value body => value.toArray.push body
        | .ifElse condition thenBranch elseBranch => #[condition, thenBranch] ++ elseBranch.toArray
        | .match_ scrutinee arms =>
            arms.foldl (init := #[scrutinee]) fun children arm =>
              children ++ arm.guard.toArray |>.push arm.body
        | .loop _ body => #[body]
        | .break_ _ value => value.toArray
        | .return_ values => values
        | .throw_ _ arguments => arguments
        | .assign _ value => #[value]
        | .assignPattern _ value => #[value]
        | _ => #[]
      children.foldl (fun found child => bodyNodes expressions child found) found

/-- Rewrite the patterns through references of a unit's function bodies. -/
def elaborateReferencePatterns (unit : RawUnit) : Except String RawUnit := do
  let mut types := unit.tables.types
  let mut namespaces := #[]
  for ns in unit.namespaces do
    let context : RefPatContext := { unit, profile := ns.profile }
    let mut state : ReferencePatternState :=
      { types, expressions := ns.expressions, patterns := ns.patterns, places := ns.places,
        locals := #[] }
    let mut functions := #[]
    for function in ns.functions do
      match function.body with
      | .structured root =>
          let nodes := bodyNodes state.expressions root #[]
          state := { state with locals := function.locals }
          for node in nodes do
            let ((), next) ← (rewriteNode context node).run state
            state := next
          functions := functions.push { function with locals := state.locals }
      | _ => functions := functions.push function
    types := state.types
    namespaces := namespaces.push { ns with
      expressions := state.expressions, patterns := state.patterns, places := state.places,
      functions }
  return { unit with tables := { unit.tables with types }, namespaces }

end LeanerIR.Import
