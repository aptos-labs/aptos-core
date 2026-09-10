-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Decode
import LeanerIR.Proofs.Certify

/-!
# Storage through the representation

The normalization route reads and writes storage as runtime values; the
contract speaks of typed twins.  The two tactics here bridge them from the
context alone, with no plan of the body:

* `leaner_expose_resources` turns every `requires`-side presence fact
  (`(globals.lookup key).isSome`) into the twin the key holds, destructured
  to its scalar fields with their certificates, and the runtime spelling of
  that value at the key, which is the fact normalization reads through.
* `leaner_represent_writes` finds, in the goal, every keyed write of a
  nominal literal over a represented map and asserts the representation of
  the written map, the twin found by decoding the literal; the closer then
  reads the final storage through the contract's typed contents.
-/

namespace LeanerIR.Proofs.Denotation.RowSpec

open Lean Meta Elab Tactic

/-! ## Recognizing the facts -/

/-- A family's representation in the context: `FamilyRepresentation erase
ns ty contents globals`, or its unfolding `∀ key, globals.lookup ⟨ns, ty,
key⟩ = (contents key).map erase`. -/
structure Represented where
  twin : Lean.Expr
  erase : Lean.Expr
  namespaceId : Lean.Expr
  typeId : Lean.Expr
  contents : Lean.Expr
  globals : Lean.Expr
  proof : Lean.Expr

def represented? (proof : Lean.Expr) (type : Lean.Expr) : Option Represented := do
  if type.isAppOfArity ``LeanerIR.FamilyRepresentation 6 then
    let args := type.getAppArgs
    return ⟨args[0]!, args[1]!, args[2]!, args[3]!, args[4]!, args[5]!, proof⟩
  let .forallE _ _ body _ := type | none
  guard (body.isAppOfArity ``Eq 3)
  let lhs := body.getArg! 1
  let rhs := body.getArg! 2
  guard (lhs.isAppOfArity ``LeanerIR.GlobalMap.lookup 2)
  let key := lhs.getArg! 1
  guard (key.isAppOfArity ``LeanerIR.GlobalKey.mk 3 && key.getArg! 2 == .bvar 0)
  guard (rhs.isAppOfArity ``Option.map 4)
  let contentsApp := rhs.getArg! 3
  guard (contentsApp.isApp && contentsApp.appArg! == .bvar 0)
  let globals := lhs.getArg! 0
  let namespaceId := key.getArg! 0
  let typeId := key.getArg! 1
  let erase := rhs.getArg! 2
  let contents := contentsApp.appFn!
  guard (!globals.hasLooseBVars && !namespaceId.hasLooseBVars && !typeId.hasLooseBVars &&
    !erase.hasLooseBVars && !contents.hasLooseBVars)
  return ⟨rhs.getArg! 0, erase, namespaceId, typeId, contents, globals, proof⟩

/-- The families represented in the context. -/
def representations : MetaM (Array Represented) := do
  let mut found := #[]
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    let type ← instantiateMVars declaration.type
    if let some r := represented? declaration.toExpr type then
      found := found.push r
  return found

/-- A presence fact: `(globals.lookup ⟨ns, ty, key⟩).isSome = true`. -/
def presence? (type : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr) := do
  guard (type.isAppOfArity ``Eq 3 && (type.getArg! 2).isConstOf ``Bool.true)
  let lhs := type.getArg! 1
  guard (lhs.isAppOfArity ``Option.isSome 2)
  let lookup := lhs.getArg! 1
  guard (lookup.isAppOfArity ``LeanerIR.GlobalMap.lookup 2)
  let key := lookup.getArg! 1
  guard (key.isAppOfArity ``LeanerIR.GlobalKey.mk 3)
  return (lookup.getArg! 0, key.getArg! 0, key.getArg! 1, key.getArg! 2)

/-- The representation of the map `globals` for the family `ns`/`ty`;
the spellings of the ids are compared up to reduction. -/
private def mappedTypeIdEq (mapped symbolic : Lean.Expr) : MetaM Bool := do
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    let equality ← instantiateMVars declaration.type
    unless equality.isAppOfArity ``Eq 3 do continue
    let lhs := equality.getArg! 1
    let rhs := equality.getArg! 2
    let matchMapping (instantiated actual : Lean.Expr) : MetaM Bool := do
      unless instantiated.isAppOfArity
          ``LeanerIR.SemanticOperations.instantiatedTypeId 2 do
        return false
      return (← isDefEq (instantiated.getArg! 1) symbolic) &&
        (← isDefEq actual mapped)
    if (← matchMapping lhs rhs) || (← matchMapping rhs lhs) then return true
  return false

def representationOf (reps : Array Represented) (globals namespaceId typeId : Lean.Expr) :
    MetaM (Option Represented) := do
  for r in reps do
    if (← isDefEq r.globals globals) && (← isDefEq r.namespaceId namespaceId) &&
        ((← isDefEq r.typeId typeId) || (← mappedTypeIdEq r.typeId typeId)) then
      return some r
  return none

/-! ## Exposing the present resources -/

/-- The erasure closure of a twin: its own `erase`, its fields', and the
runtime encoders they reach. -/
def eraseClosure (erase : Lean.Name) : MetaM (Array Lean.Name) :=
  unfoldClosureWith erase fun name =>
    name.getString! == "erase" || (`LeanerIR).isPrefixOf name

/-- The keyed reads of a tree: the family read and the parameter slot the
key is taken from. -/
partial def treeReads (e : Lean.Expr) (acc : Array (Lean.Expr × Lean.Expr × Nat)) :
    Array (Lean.Expr × Lean.Expr × Nat) :=
  let acc := match read? e with
    | some read => if acc.contains read then acc else acc.push read
    | none => acc
  match e with
  | .app f a => treeReads a (treeReads f acc)
  | .lam _ t b _ | .forallE _ t b _ => treeReads b (treeReads t acc)
  | .letE _ t v b _ => treeReads b (treeReads v (treeReads t acc))
  | .mdata _ b => treeReads b acc
  | .proj _ _ b => treeReads b acc
  | _ => acc
where
  read? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Nat) := do
    guard (e.isAppOfArity ``Tree.global 2)
    let operation := (e.getArg! 0).consumeMData
    let operands := (e.getArg! 1).consumeMData
    let resource ←
      if operation.isAppOfArity ``GlobalLocationOperation.contains 1 ||
          operation.isAppOfArity ``GlobalLocationOperation.take 1 then
        some (operation.getArg! 0).consumeMData
      else if operation.isAppOfArity ``GlobalLocationOperation.borrow 1 then
        let site := (operation.getArg! 0).consumeMData
        guard (site.isAppOfArity ``BorrowLocation.mk 4)
        some (site.getArg! 0).consumeMData
      else none
    guard (resource.isAppOfArity ``ResourceLocation.mk 2)
    guard (operands.isAppOfArity ``Operands.cons 2)
    let head := (operands.getArg! 0).consumeMData
    guard (head.isAppOfArity ``Tree.localVar 1)
    let localId := (head.getArg! 0).consumeMData
    guard (localId.isAppOfArity ``LeanerIR.LocalId.mk 1)
    let index ← (localId.getArg! 0).nat?
    return (resource.getArg! 0, resource.getArg! 1, index)

/-- The literal arguments of the goal: the first array literal of runtime
values it mentions, which is the function's argument row. -/
partial def goalArguments? (e : Lean.Expr) : Option (Array Lean.Expr) :=
  let here : Option (Array Lean.Expr) := do
    guard (e.isAppOfArity ``List.toArray 2)
    let elements ← literalList? (e.getArg! 1)
    guard (elements.size > 0 && elements.all fun element =>
      match element.consumeMData.getAppFn.constName? with
      | some name => (`LeanerIR.RuntimeValue).isPrefixOf name
      | none => false)
    return elements
  match here with
  | some elements => some elements
  | none =>
    match e with
    | .app f a => goalArguments? f <|> goalArguments? a
    | .lam _ t b _ | .forallE _ t b _ => goalArguments? t <|> goalArguments? b
    | .letE _ t v b _ => goalArguments? t <|> goalArguments? v <|> goalArguments? b
    | .mdata _ b => goalArguments? b
    | .proj _ _ b => goalArguments? b
    | _ => none

/-- The initial state of the goal: the state the representations are over. -/
def initialState : MetaM Lean.Expr := do
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    if declaration.userName == `initial then return declaration.toExpr
  throwError "leaner_expose_resources: no `initial` state in the context"

/-- Enumerate every Boolean of the context: a twin's Boolean field is a
value the clauses compare, and the closing decides each alternative. -/
elab "leaner_split_bools" : tactic => do
  let goal ← getMainGoal
  let bools ← goal.withContext do
    let mut found := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      if (← instantiateMVars declaration.type).isConstOf ``Bool then
        found := found.push declaration.fvarId
    pure found
  let mut goals := [goal]
  for fvarId in bools do
    let mut next := []
    for g in goals do
      let subgoals ← g.cases fvarId
      next := next ++ subgoals.toList.map (·.mvarId)
    goals := next
  replaceMainGoal goals

/-- Expose the resources a body reads: for every keyed read of the tree,
the typed contents at the key are split into absent and present, the
present twin destructured to its certified fields, and the runtime value
the key holds stated as the fact normalization reads through.  A
`requires`-side presence fact closes the absent case. -/
elab "leaner_expose_resources" "[" tree:ident "]" : tactic => do
  let goal ← getMainGoal
  let reads ← goal.withContext do
    let treeName ← realizeGlobalConstNoOverload tree
    let some treeValue := (← getEnv).find? treeName |>.bind (·.value?)
      | throwError "leaner_expose_resources: {treeName} has no value"
    let reads ← lambdaTelescope treeValue fun _ body => pure (treeReads body #[])
    if reads.isEmpty then return #[]
    let target ← instantiateMVars (← goal.getType)
    let some arguments := goalArguments? target
      | throwError "leaner_expose_resources: the goal names no literal argument row:\n{target}"
    let reps ← representations
    let mut exposed : Array (Represented × Lean.Expr) := #[]
    for (namespaceId, typeId, index) in reads do
      let some argument := arguments[index]? | continue
      let key ← whnfD (mkApp (mkConst ``LeanerIR.RuntimeValue.storageKey) argument)
      let some r ← representationOf reps
          (← mkAppM ``LeanerIR.RuntimeState.globals #[← initialState]) namespaceId typeId
        | continue
      if ← exposed.anyM fun (r', key') => pure (r'.erase == r.erase) <&&> isDefEq key key' then
        continue
      exposed := exposed.push (r, key)
    pure exposed
  for (r, key) in reads do
    -- Each represented family needs a distinct lookup equation. Reusing the
    -- user name `present` made context-driven normalization see only the last
    -- family in functions that touch two resources.
    let presentName ← Lean.mkFreshUserName `resource_present
    let presentIdent := mkIdent presentName
    let contentsEqName ← Lean.mkFreshUserName `contents_eq
    let contentsEqIdent := mkIdent contentsEqName
    let keySyntax ← goal.withContext (Term.exprToSyntax key)
    let contentsSyntax ← goal.withContext (Term.exprToSyntax r.contents)
    let representedSyntax ← goal.withContext (Term.exprToSyntax r.proof)
    let eraseNames ← goal.withContext do
      let some eraseName := r.erase.getAppFn.constName? | throwError
        "leaner_expose_resources: the erasure is not a named twin erasure"
      eraseClosure eraseName
    let eraseLemmas ← eraseNames.mapM fun name =>
      `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
    let decoderLemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
      goal.withContext do
        let some eraseName := r.erase.getAppFn.constName? | return #[]
        let literalRoundtrip := eraseName.getPrefix ++ `decode?_literal
        unless (← getEnv).contains literalRoundtrip do return #[]
        return #[← `(Lean.Parser.Tactic.simpLemma|
          $(mkIdent literalRoundtrip):ident)]
    let storagePremises ← goal.withContext do
      let mut facts := #[]
      for declaration in ← getLCtx do
        if declaration.isImplementationDetail then continue
        let type ← instantiateMVars declaration.type
        if type.isArrow &&
            (type.find? (·.isConstOf ``LeanerIR.GlobalMap.lookup)).isSome then
          facts := facts.push (mkIdent declaration.userName)
      pure facts
    let presenceFacts ← goal.withContext do
      let mut facts := #[]
      for declaration in ← getLCtx do
        if declaration.isImplementationDetail then continue
        let type ← instantiateMVars declaration.type
        let some (_, namespaceId, typeId, key') := presence? type | continue
        if (← isDefEq namespaceId r.namespaceId) && (← isDefEq typeId r.typeId) &&
            (← isDefEq key' key) then
          facts := facts.push (mkIdent declaration.userName)
      pure facts
    let presenceLemmas ← presenceFacts.mapM fun fact =>
      `(Lean.Parser.Tactic.simpLemma| $fact:ident)
    evalTactic (← `(tactic|
      all_goals
        (rcases $contentsEqIdent:ident : $contentsSyntax $keySyntax with _ | twin
         all_goals
           (have $presentIdent:ident := $representedSyntax $keySyntax
            rw [$contentsEqIdent:ident] at $presentIdent:ident
            simp only [$eraseLemmas,*, Option.map_some, Option.map_none] at $presentIdent:ident)
         all_goals
           try simp only [$presentIdent:ident, Option.isSome_none, Option.isSome_some,
             Bool.false_eq_true, $presenceLemmas,*] at $presenceFacts:ident*
         all_goals
           try simp (disch := leaner_denotation_discharge) only
             [$presentIdent:ident, $decoderLemmas,*, Option.isSome_some,
              Option.bind_some, Option.getD_some, true_implies] at $storagePremises:ident*
         all_goals leaner_certify!
         all_goals leaner_split_bools)))

/-! ## Representing the written resources -/

/-- A keyed write in the goal over a represented map: the map written, its
key, the value, and the hole the write fills, if any. -/
structure Write where
  globals : Lean.Expr
  key : Lean.Expr
  namespaceId : Lean.Expr
  typeId : Lean.Expr
  storageKey : Lean.Expr
  hole : Option Lean.Expr
  /-- The nominal literal written, or `none` for a removal. -/
  value : Option Lean.Expr

/-- The writes of nominal literals in a term, over whichever map. -/
partial def collectWrites (e : Lean.Expr) (acc : Array Write) : Array Write :=
  let acc := match write? e with
    | some write => if acc.any (fun w => w.value == write.value && w.key == write.key) then acc
        else acc.push write
    | none => acc
  match e with
  | .app f a => collectWrites a (collectWrites f acc)
  | .lam _ t b _ | .forallE _ t b _ => collectWrites b (collectWrites t acc)
  | .letE _ t v b _ => collectWrites b (collectWrites v (collectWrites t acc))
  | .mdata _ b => collectWrites b acc
  | .proj _ _ b => collectWrites b acc
  | _ => acc
where
  write? (e : Lean.Expr) : Option Write := do
    guard (!e.hasLooseBVars)
    if e.isAppOfArity ``LeanerIR.GlobalMap.erase 2 then
      let inner := (e.getArg! 0).consumeMData
      let key := (e.getArg! 1).consumeMData
      guard (key.isAppOfArity ``LeanerIR.GlobalKey.mk 3)
      return ⟨inner, key, key.getArg! 0, key.getArg! 1, key.getArg! 2, none, none⟩
    guard (e.isAppOfArity ``LeanerIR.GlobalMap.insert 3)
    let inner := (e.getArg! 0).consumeMData
    let key := (e.getArg! 1).consumeMData
    let value := (e.getArg! 2).consumeMData
    guard (value.isAppOfArity ``LeanerIR.RuntimeValue.nominal 3)
    guard (key.isAppOfArity ``LeanerIR.GlobalKey.mk 3)
    let namespaceId := key.getArg! 0
    let typeId := key.getArg! 1
    let storageKey := key.getArg! 2
    if inner.isAppOfArity ``LeanerIR.GlobalMap.insert 3 &&
        (inner.getArg! 1).consumeMData == key then
      return ⟨(inner.getArg! 0).consumeMData, key, namespaceId, typeId, storageKey,
        some ((inner.getArg! 2).consumeMData), some value⟩
    return ⟨inner, key, namespaceId, typeId, storageKey, none, some value⟩

/-- The representation of one write: its statement in the goal's spelling
and its proof through the twin the literal decodes to. -/
def representWrite (r : Represented) (write : Write) :
    MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let some eraseName := r.erase.constName? | return none
  let some value := write.value
    | -- A removal: the key's contents are gone.
      let proof ← mkAppOptM ``LeanerIR.FamilyRepresentation.erase_self
        #[none, r.erase, r.namespaceId, r.typeId, r.contents, r.globals, r.proof,
          write.storageKey]
      return some (← inferType proof, proof)
  let decodeName := eraseName.getPrefix ++ `decode?
  let some twin ← decodeLiteral decodeName value | return none
  let contents ← mkAppOptM ``LeanerIR.updateContents
    #[none, r.contents, write.storageKey, ← mkAppOptM ``Option.some #[none, twin]]
  let proof ← match write.hole with
    | some hole =>
        mkAppOptM ``LeanerIR.FamilyRepresentation.insert_over_hole
          #[none, r.erase, r.namespaceId, r.typeId, r.contents, r.globals, r.proof,
            write.storageKey, hole, twin]
    | none =>
        mkAppOptM ``LeanerIR.FamilyRepresentation.insert_self
          #[none, r.erase, r.namespaceId, r.typeId, r.contents, r.globals, r.proof,
            write.storageKey, twin]
  let written := match write.hole with
    | some hole =>
        mkApp3 (mkConst ``LeanerIR.GlobalMap.insert)
          (mkApp3 (mkConst ``LeanerIR.GlobalMap.insert) r.globals write.key hole)
          write.key value
    | none => mkApp3 (mkConst ``LeanerIR.GlobalMap.insert) r.globals write.key value
  let type ← mkAppOptM ``LeanerIR.FamilyRepresentation
    #[none, r.erase, r.namespaceId, r.typeId, contents, written]
  unless ← isDefEq (← inferType proof) type do
    throwError "leaner_represent_writes: the written value {value} is not the \
      erasure of its decoding {twin}"
  return some (type, proof)

/-- Transport another family's representation across a write. Distinct
namespace/type IDs make the write disjoint from every key in that family. -/
def representOtherWrite (r : Represented) (write : Write) :
    MetaM (Option (Lean.Expr × Lean.Expr)) := do
  if (← isDefEq r.namespaceId write.namespaceId) &&
      (← isDefEq r.typeId write.typeId) then
    return none
  let distinctType ← mkAppM ``Or
    #[← mkAppM ``Ne #[write.namespaceId, r.namespaceId],
      ← mkAppM ``Ne #[write.typeId, r.typeId]]
  let distinct ← mkDecideProof distinctType
  let proof ← match write.value, write.hole with
    | none, _ =>
        mkAppOptM ``LeanerIR.FamilyRepresentation.erase_other
          #[none, none, none, none, none, none, some r.proof,
            some write.key, some distinct]
    | some value, none =>
        mkAppOptM ``LeanerIR.FamilyRepresentation.insert_other
          #[none, none, none, none, none, none, some r.proof,
            some write.key, some value, some distinct]
    | some value, some hole =>
        let overHole ← mkAppOptM ``LeanerIR.FamilyRepresentation.insert_other
          #[none, none, none, none, none, none, some r.proof,
            some write.key, some hole, some distinct]
        mkAppOptM ``LeanerIR.FamilyRepresentation.insert_other
          #[none, none, none, none, none, none, some overHole,
            some write.key, some value, some distinct]
  pure (some (← inferType proof, proof))

/-- For every write of a nominal literal over a represented map in the
goal, the representation of the written map. -/
elab "leaner_represent_writes" : tactic => do
  let goal ← getMainGoal
  let (writes, initialReps) ← goal.withContext do
    let reps ← representations
    let target ← instantiateMVars (← goal.getType)
    pure (collectWrites target #[], reps)
  let mut goal := goal
  let mut reps := initialReps
  -- The target contains a write chain outside-in. Transport every family
  -- inside-out so the next write sees representations of its actual base map.
  for write in writes.reverse do
    let current := reps
    for r in current do
      unless ← goal.withContext (isDefEq r.globals write.globals) do continue
      let represented ← goal.withContext do
        if (← isDefEq r.namespaceId write.namespaceId) &&
            (← isDefEq r.typeId write.typeId) then
          representWrite r write
        else
          representOtherWrite r write
      let some (type, proof) := represented | continue
      -- Every transported family needs an addressable hypothesis. Reusing one
      -- user name makes the syntax-based normalization inventory resolve all
      -- entries to the last family only.
      let representedName ← Lean.mkFreshUserName `represented
      let asserted ← goal.assert representedName type proof
      let (fvarId, next) ← asserted.intro1P
      goal := next
      if let some nextRep := represented? (.fvar fvarId) type then
        reps := reps.push nextRep
  replaceMainGoal [goal]

/-! ## Decoding the returned twins -/

/-- A returned value to decode: the goal `∃ result, decode literal = some
result ∧ …` with the decoder a twin's `decode?` or a codec's, at a literal
runtime value.  Returns the predicate, the whole decoding equation's
left-hand side, and the definitions its evaluation unfolds. -/
def returnedDecoding? (target : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Array Lean.Name) := do
  guard (target.isAppOfArity ``Exists 2)
  let predicate := (target.getArg! 1).consumeMData
  let .lam _ _ body _ := predicate | none
  guard (body.isAppOfArity ``And 2)
  let equation := (body.getArg! 0).consumeMData
  guard (equation.isAppOfArity ``Eq 3)
  let lhs := (equation.getArg! 1).consumeMData
  let rhs := (equation.getArg! 2).consumeMData
  guard (rhs.isAppOfArity ``Option.some 2 && (rhs.getArg! 1) == .bvar 0)
  guard (!lhs.hasLooseBVars)
  /- A tuple of results decodes through a bind that repacks the one decoded
  value; the decoding is the bind's action, and the equation stays whole. -/
  let core := if lhs.isAppOfArity ``Option.bind 4 then
      let continuation := (lhs.getArg! 3).consumeMData
      match continuation with
      | .lam _ _ body _ =>
          if body.isAppOfArity ``Option.some 2 && (body.getArg! 1) == .bvar 0 then
            (lhs.getArg! 2).consumeMData
          else lhs
      | _ => lhs
    else lhs
  guard core.isApp
  let literal := core.appArg!.consumeMData
  let some literalHead := literal.getAppFn.constName? | none
  guard ((`LeanerIR.RuntimeValue).isPrefixOf literalHead)
  let head := core.appFn!.consumeMData
  /- The decoder's definitions: a twin's `decode?`, or a codec built from
  the runtime codecs and the twins' codecs. -/
  let mut roots : Array Lean.Name := #[]
  if head.isAppOfArity ``LeanerIR.Proofs.Codec.decode? 3 then
    let codec := (head.getArg! 2).consumeMData
    for name in codec.getUsedConstants do
      if (`LeanerIR.Proofs.Codec).isPrefixOf name then roots := roots.push name
      else if name.getString! == "codec" then
        roots := roots.push name
        roots := roots.push (name.getPrefix ++ `decode?)
  else
    let some name := head.getAppFn.constName? | none
    guard (name.getString! == "decode?" ||
      [``LeanerIR.decodeInt?, ``LeanerIR.decodeBool?, ``LeanerIR.decodeString?,
        ``LeanerIR.decodeAddress?, ``LeanerIR.decodeSigner?, ``LeanerIR.decodeBytes?,
        ``LeanerIR.decodeUnit?].contains name)
    roots := roots.push name
  guard (!roots.isEmpty)
  return (predicate, lhs, roots)

/-- Instantiate the returned twin of the goal by decoding the literal the
body returns, and discharge its decoding equation. -/
elab "leaner_decode_results" : tactic => do
  let goal ← getMainGoal
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let some (predicate, application, roots) := returnedDecoding? target | return
  let some (twin, proof) ← goal.withContext (evaluateDecoding roots application) | return
  let rest ← goal.withContext do
    let body := predicate.beta #[twin]
    let rest ← mkFreshExprMVar ((body.consumeMData.getArg! 1)) (kind := .syntheticOpaque)
    let conjunction ← mkAppM ``And.intro #[proof, rest]
    let witness ← mkAppOptM ``Exists.intro #[none, some predicate, some twin, some conjunction]
    goal.assign witness
    pure rest.mvarId!
  replaceMainGoal [rest]

/-- Substitute scalar variables fixed by explicit literal equalities. Do not
unfold arbitrary hypotheses or solve arithmetic here: this bounded preparation
lets the normalizer evaluate branches and computed indices already decided by
the precondition. -/
elab "leaner_subst_decided" : tactic => do
  let goal ← getMainGoal
  let decided ← goal.withContext do
    let mut found := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let type ← instantiateMVars declaration.type
      unless type.isAppOfArity ``Eq 3 do continue
      let carrier := type.getArg! 0
      unless carrier.isConstOf ``Bool || carrier.isConstOf ``Int ||
          carrier.isConstOf ``Nat do continue
      let lhs := (type.getArg! 1).consumeMData
      let rhs := (type.getArg! 2).consumeMData
      let literal (e : Lean.Expr) : MetaM Bool := do
        if carrier.isConstOf ``Bool then
          return e.isConstOf ``Bool.true || e.isConstOf ``Bool.false
        else if carrier.isConstOf ``Int then return (← getIntValue? e).isSome
        else return (← getNatValue? e).isSome
      if (lhs.isFVar && (← literal rhs)) || (rhs.isFVar && (← literal lhs)) then
        found := found.push declaration.fvarId
    pure found
  let mut goal := goal
  for fvarId in decided do
    try
      goal ← substCore goal fvarId (symm := false) <&> (·.2)
    catch _ =>
      try goal ← substCore goal fvarId (symm := true) <&> (·.2)
      catch _ => pure ()
  replaceMainGoal [goal]

/-- Show the goal when the closer's debugging is on. -/
elab "leaner_debug_goal" : tactic => do
  if LeanerIR.Proofs.Certify.leaner.certifyDebug.get (← getOptions) then
    let goal ← getMainGoal
    logInfo m!"leaner_close_normalized goal:\n{← goal.withContext (Meta.ppGoal goal)}"

/-! ## Closing the normal form -/

/-- Close the verification condition a normalized body leaves: its
implications introduced and its conjunctions split, every keyed write
represented, and each obligation closed by the arithmetic that refutes an
unreachable branch or by the certified closing. -/
syntax "leaner_close_normalized"
  (" [" Lean.Parser.Tactic.simpLemma,* "]")? : tactic

/-- Primitive guards can leave a continuation folded until the surrounding
implication is introduced. Resume only when a computation remains; ordinary
data/arithmetic obligations do not need another normalization pass. -/
elab "leaner_resume_normalized" : tactic => do
  let target ← instantiateMVars (← getMainTarget)
  unless (target.find? (·.isConstOf ``wp)).isSome do
    throwError "no computation remains to normalize"
  evalTactic (← `(tactic| (leaner_subst_decided; leaner_normalize)))
  unless (← getGoals).isEmpty do
    let result ← instantiateMVars (← getMainTarget)
    if result == target then throwError "the remaining computation is unchanged"

macro_rules
  | `(tactic| leaner_close_normalized) =>
      `(tactic| leaner_close_normalized [])
  | `(tactic| leaner_close_normalized [$facts,*]) =>
      `(tactic|
        (repeat' first | intro _ | refine And.intro ?_ ?_
         all_goals leaner_subst_decided
         repeat' first | intro _ | refine And.intro ?_ ?_ | leaner_resume_normalized
         /- The returned value first: with it known, the clauses' own data
         terms — a returned reference resolved into the exported twin —
         evaluate as the body's did. -/
         all_goals leaner_decode_results
         /- A storage-key hypothesis can retain the syntactic `getD` used by
         the lowering even after the written key in the target has reduced.
         Normalize that tiny wrapper on both sides before applying the keyed
         map laws. -/
         all_goals try simp only [Option.getD_some, Option.getD_none] at *
         all_goals try simp (disch := assumption) only [resolveReturnedBorrows_singleBorrow,
           resolveReturnedBorrows_integer, resolveReturnedBorrows_empty, fillHole?, holeFill?,
           rewriteFirst_loanHole, rewriteFirst_integer, rewriteFirst_nominal, rewriteFirst_borrow,
           rewriteFirst_bool, rewriteFirst_address, rewriteFirst_unit, rewriteFirstList_nil,
           rewriteFirstList_cons, beq_self_eq_true, Option.getD_some, Option.getD_none,
           List.toList_toArray, ite_true, ite_false, Option.some.injEq, Option.map_some,
           Option.map_none, GlobalMap.lookup_insert_other, GlobalMap.lookup_erase_other]
         all_goals leaner_debug_goal
         all_goals
           (leaner_represent_writes
            try simp (disch := assumption) only [$facts,*]
            first
            | (apply GlobalMap.lookup_insert_other <;> assumption)
            | (apply GlobalMap.lookup_erase_other <;> assumption)
            | leaner_fresh_loan
            | (exfalso; omega)
            | leaner_certified_close!)))

end LeanerIR.Proofs.Denotation.RowSpec
