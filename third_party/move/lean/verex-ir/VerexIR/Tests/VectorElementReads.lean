-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! # The element a program's read of a vector yields, and its fact

A program reads an element through `withVectorElem`, which holds the vector
itself. The walk's split of that read (`splitVectorRead?`) states the
element's fact where the context proves the vector's fact whole: the read's
equation is of the vector's array once it is normalized. Where it does not
and the vector is an operation's result, the split keeps what the element's
fact owes to the result's as hidden evidence (`ElementFactOf`), which the
openers of facts pay when the context proves the result's fact. The controls
fix who states the fact of which read: the split for a read whose vector's
fact it finds, the evidence for a read of an operation's result whose fact
arrives later, the observer of the context's reads (`observedElementFacts`)
for the others, and never two of them.
-/

namespace VerexIR.Tests.VectorElementReads

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
open Lean Meta Elab Tactic

/-- The split of the read the goal runs. -/
elab "split_vector_read" : tactic => do
  let goal ← getMainGoal
  let goal ← goal.replaceTargetDefEq (← whnfR (← instantiateMVars (← goal.getType)))
  let some cases ← splitVectorRead? goal
    | throwError "the goal runs no read of a vector's element"
  replaceMainGoal cases

/-- The facts the context states of values that are no vectors, the visible
ones and the ones an opener hid. -/
private def elementFacts : TacticM (Nat × Nat) := withMainContext do
  let mut visible := 0
  let mut hidden := 0
  for decl in ← getLCtx do
    if decl.isAuxDecl then continue
    let type := (← instantiateMVars decl.type).consumeMData
    unless type.isAppOfArity ``NTy.Holds 3 do continue
    if (type.getArg! 1).isAppOfArity ``NTy.vector 1 then continue
    if decl.isImplementationDetail then hidden := hidden + 1 else visible := visible + 1
  return (visible, hidden)

elab "assert_element_facts " visible:num hidden:num : tactic => do
  let found ← elementFacts
  unless found == (visible.getNat, hidden.getNat) do
    throwError "expected {visible.getNat} visible and {hidden.getNat} hidden element facts, found {found}"

/-- The evidence of reads that owe an element's fact to a vector's
(`ElementFactOf`), every one of them hidden. -/
elab "assert_evidence " expected:num : tactic => withMainContext do
  let mut found := 0
  for decl in ← getLCtx do
    if decl.isAuxDecl then continue
    if (← instantiateMVars decl.type).consumeMData.isAppOfArity ``ElementFactOf 4 then
      unless decl.isImplementationDetail do
        throwError "the evidence {decl.userName} is a visible hypothesis"
      found := found + 1
  unless found == expected.getNat do
    throwError "expected {expected.getNat} hypotheses of read evidence, found {found}"

/-- The hypotheses of read evidence left in the context, hidden or not. -/
elab "assert_owed " expected:num : tactic => withMainContext do
  let mut found := 0
  for decl in ← getLCtx do
    if decl.isAuxDecl then continue
    if (← instantiateMVars decl.type).consumeMData.isAppOfArity ``ElementFactOf 4 then
      found := found + 1
  unless found == expected.getNat do
    throwError "expected {expected.getNat} hypotheses of read evidence left, found {found}"

/-- The visible clauses of a declaration's own invariant. -/
elab "assert_own_clauses " expected:num : tactic => withMainContext do
  let mut found := 0
  for decl in ← getLCtx do
    if decl.isImplementationDetail || decl.isLet then continue
    if (← instantiateMVars decl.type).consumeMData.isAppOfArity ``Carriers.own 4 then
      found := found + 1
  unless found == expected.getNat do
    throwError "expected {expected.getNat} visible own clauses, found {found}"

/-- The array the context's one optional read with a value is of, which must
be an application of the given constant (a local where the name is
anonymous), and the read itself. -/
private def readEquation (head : Name) : TacticM Lean.Expr := withMainContext do
  let mut found := none
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type := (← instantiateMVars decl.type).consumeMData
    let some (_, lookup, value) := type.eq? | continue
    unless lookup.isAppOfArity ``GetElem?.getElem? 7 && value.isAppOfArity ``Option.some 2 do
      continue
    if found.isSome then throwError "more than one read with a value"
    let array := (lookup.getArg! 5).consumeMData
    unless array.getAppFn.constName?.getD .anonymous == head do
      throwError "the read is of {← ppExpr array}, not of an application of {head}"
    found := some type
  let some read := found | throwError "no read with a value in the context"
  return read

/-- The context's read is of an application of the given constant and is
unchanged by normalizing the context once more. -/
elab "assert_normal_read " head:ident : tactic => do
  let head ← realizeGlobalConstNoOverloadWithInfo head
  let before ← readEquation head
  evalTactic (← `(tactic| try verex_normalize at *))
  let after ← readEquation head
  unless before == after do
    throwError "the read changes under a second normalization:\n{← ppExpr before}\n{← ppExpr after}"

/-- The element facts the observer of the context's reads builds from the
container fact a hypothesis states. -/
elab "assert_observed " container:ident expected:num : tactic => withMainContext do
  let proof ← getFVarId container
  let type := (← instantiateMVars (← proof.getType)).consumeMData
  unless type.isAppOfArity ``NTy.Holds 3 do throwError "no fact of a value"
  let built ← observedElementFacts (type.getArg! 0) (type.getArg! 1) (type.getArg! 2) (mkFVar proof)
  unless built.size == expected.getNat do
    throwError "expected the observer to build {expected.getNat} element facts, it built {built.size}"

/-- The consumer of read evidence, asked for all the evidence of the context
with nothing paid and nothing known beside the context: how many facts it
builds, and how many evidence hypotheses it retires without building one. -/
elab "assert_paid " built:num retired:num : tactic => withMainContext do
  let mut evidence := #[]
  for decl in ← getLCtx do
    if decl.isAuxDecl then continue
    if (← instantiateMVars decl.type).consumeMData.isAppOfArity ``ElementFactOf 4 then
      evidence := evidence.push decl
  let paid ← paidElementFacts evidence #[] #[]
  let facts := (paid.filter (·.1.isSome)).size
  unless facts == built.getNat && paid.size - facts == retired.getNat do
    throwError "expected {built.getNat} facts built and {retired.getNat} evidence retired without one, \
      found {facts} and {paid.size - facts}"

/-- No read of a vector's element remains in a goal or its context. -/
elab "assert_no_read_residue" : tactic => do
  for goal in ← getGoals do
    goal.withContext do
      let residue (e : Lean.Expr) := (e.find? (·.isConstOf ``withVectorElem)).isSome
      if residue (← instantiateMVars (← goal.getType)) then
        throwError "a goal still runs a read of a vector's element"
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        if residue (← instantiateMVars decl.type) then
          throwError "{decl.userName} still states a read of a vector's element"

local notation "Element" t => NTy.struct t NRow.nil NRow.nil

/-- The read whose value must satisfy its declaration's own clauses. -/
private abbrev readsOwn [Carriers] (t : StructHandle) (values : (NTy.vector (Element t)).carrier)
    (index : Nat) : Prop :=
  wp (σ := Unit) (ε := Unit)
    (withVectorElem (Element t) values index (fun element => Spec.pure element) (Spec.abort ()))
    (fun result _ => Carriers.own t .nil (NTy.encode (Element t) result)) (fun _ => True) ()

/-- The read with nothing asked of its value. -/
private abbrev readsAny [Carriers] (t : StructHandle) (values : (NTy.vector (Element t)).carrier)
    (index : Nat) : Prop :=
  wp (σ := Unit) (ε := Unit)
    (withVectorElem (Element t) values index (fun element => Spec.pure element) (Spec.abort ()))
    (fun _ _ => True) (fun _ => True) ()

/-- The read whose value must have its fact. -/
private abbrev readsHolds [Carriers] (t : StructHandle) (values : (NTy.vector (Element t)).carrier)
    (index : Nat) : Prop :=
  wp (σ := Unit) (ε := Unit)
    (withVectorElem (Element t) values index (fun element => Spec.pure element) (Spec.abort ()))
    (fun result _ => NTy.Holds (Element t) result) (fun _ => True) ()

macro "finish_read" : tactic =>
  `(tactic| (simp only [VerexIR.Proofs.wp_pure, VerexIR.Proofs.wp_abort]))

/-! ## Negative controls -/

-- No fact of the vector: the split states no fact of the element, the
-- observer has no container to read, and the clause is not established.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (k : Nat) : True := by
  have : readsAny t v k := by
    split_vector_read
    case none => finish_read
    case some =>
      assert_element_facts 0 0
      assert_evidence 0
      finish_read
  fail_if_success
    have : readsOwn t v k := by
      split_vector_read
      case none => finish_read
      case some =>
        finish_read
        verex_certified_clauses
        verex_observed_elements
        assumption
  trivial

-- The fact of another vector is no fact of this one's elements.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v w : (NTy.vector (Element t)).carrier) (k : Nat)
    (hw : NTy.Holds (.vector (Element t)) w) : True := by
  have : readsAny t v k := by
    split_vector_read
    case none => finish_read
    case some =>
      assert_element_facts 0 0
      assert_evidence 0
      assert_observed hw 0
      finish_read
  fail_if_success
    have : readsOwn t v k := by
      split_vector_read
      case none => finish_read
      case some =>
        finish_read
        verex_certified_clauses
        verex_observed_elements
        assumption
  trivial

-- The fact of the vector an operation is of is no fact of the operation's
-- result where the operation puts an element of its own into it whose fact
-- the context does not state: the closed reading selects nothing and leaves
-- no goal behind, the split's two cases are all there is. The case keeps the
-- read's evidence, which no opener can pay, and which alone proves nothing.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier)
    (x : (Element t).carrier) (k : Nat) (within : 0 < v.values.size)
    (hv : NTy.Holds (.vector (Element t)) v) : True := by
  have : readsAny t (v.set 0 x) k := by
    split_vector_read
    case none => finish_read
    case some =>
      assert_element_facts 0 0
      assert_evidence 1
      verex_certified_clauses
      verex_observed_elements
      assert_evidence 1
      assert_element_facts 0 0
      assert_own_clauses 0
      finish_read
  fail_if_success
    have : readsOwn t (v.set 0 x) k := by
      split_vector_read
      case none => finish_read
      case some =>
        finish_read
        verex_certified_clauses
        verex_observed_elements
        assumption
  fail_if_success
    have : readsHolds t (v.set 0 x) k := by
      split_vector_read
      case none => finish_read
      case some =>
        finish_read
        first | assumption | (verex_holds; done)
  trivial

/-! ## The split states the fact of the element it reads -/

-- A local vector whose fact the context states: the split states the
-- element's, the opener opens it once and hides it, and the observer builds
-- no second copy from the vector's fact, before the opening or after it,
-- across a substitution, and in each case of a later split.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (k j : Nat)
    (same : k = j) (hv : NTy.Holds (.vector (Element t)) v) : readsOwn t v k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_element_facts 1 0
    assert_evidence 0
    assert_observed hv 0
    verex_certified_clauses
    assert_element_facts 0 1
    assert_own_clauses 1
    assert_observed hv 0
    verex_observed_elements
    assert_own_clauses 1
    subst same
    assert_element_facts 0 1
    assert_observed hv 0
    verex_observed_elements
    assert_own_clauses 1
    by_cases first : k = 0
    all_goals
      assert_observed hv 0
      verex_observed_elements
      assert_own_clauses 1
      assumption

-- A value has the facts of several types where two declarations share its
-- plain type: the split states the element's fact at the read's own type,
-- and the observer still instantiates the fact of the other type from the
-- vector's fact at that type, and only that one.
example [Carriers] (s t : StructHandle) (v : (NTy.vector (Element s)).carrier) (k : Nat)
    (hs : NTy.Holds (.vector (Element s)) v) (ht : NTy.Holds (.vector (Element t)) v) :
    wp (σ := Unit) (ε := Unit)
      (withVectorElem (Element s) v k (fun element => Spec.pure element) (Spec.abort ()))
      (fun result _ => Carriers.own s .nil (NTy.encode (Element s) result) ∧
        Carriers.own t .nil (NTy.encode (Element t) result)) (fun _ => True) () := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_element_facts 1 0
    assert_observed hs 0
    assert_observed ht 1
    verex_certified_clauses
    exact ⟨by assumption, by assumption⟩

-- A read the context states and no program made is the observer's.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (k : Nat)
    (x : (Element t).carrier) (found : v.values[k]? = some x)
    (hv : NTy.Holds (.vector (Element t)) v) :
    Carriers.own t .nil (NTy.encode (Element t) x) := by
  assert_observed hv 1
  verex_observed_elements
  assert_own_clauses 1
  assumption

-- The result of an operation, read at a position the context does not place:
-- the element's fact follows from the fact of the vector the operation is of,
-- and the read's equation is the read of the array's operation, unchanged by
-- a second normalization.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (i j k : Nat)
    (hv : NTy.Holds (.vector (Element t)) v) : readsOwn t (swapBounded v i j) k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_element_facts 1 0
    verex_normalize at heq
    assert_normal_read Array.swapIfInBounds
    verex_certified_clauses
    assert_own_clauses 1
    assumption

-- The same where the context states the fact of the result and none of the
-- vector the operation is of.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (i j k : Nat)
    (held : NTy.Holds (.vector (Element t)) (swapBounded v i j)) :
    readsOwn t (swapBounded v i j) k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_element_facts 1 0
    verex_normalize at heq
    assert_normal_read Array.swapIfInBounds
    verex_certified_clauses
    assumption

-- A removal's result whose fact the context states, of a vector whose fact
-- it does not: the element's fact is stated before the lookup laws read the
-- equation as a read of that vector, and it is the only fact the leaf has.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (i : Int)
    (nonempty : 0 < v.values.size) (nonnegative : 0 ≤ i)
    (held : NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    readsOwn t (eraseBounded v 0) i.toNat := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_element_facts 1 0
    verex_normalize at heq
    assert_normal_read Array.eraseIdxIfInBounds
    verex_lookups_after_writes
    assert_normal_read SpecVector.values
    assert_element_facts 1 0
    clear held
    verex_certified_clauses
    assumption

-- An operation that puts an element of its own into the vector: the fact of
-- the result is the context's where it states that element's fact too.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier)
    (x : (Element t).carrier) (k : Nat) (within : 0 < v.values.size)
    (hv : NTy.Holds (.vector (Element t)) v) (hx : NTy.Holds (Element t) x) :
    readsOwn t (v.set 0 x) k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_element_facts 2 0
    verex_certified_clauses
    assumption

/-! ## A fact that arrives after the read

The late fact is behind a premise the normal form does not decide, so that
normalizing the context does not state it early. -/

-- Of a local vector: the read's equation still names the vector, and the
-- observer states the element's fact once the vector's is there.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (k : Nat)
    (gate : Prop) (passed : gate) (late : gate → NTy.Holds (.vector (Element t)) v) : readsOwn t v k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_element_facts 0 0
    have hv := late passed
    assert_observed hv 1
    verex_observed_elements
    assumption

-- Of an operation's result: the normalized equation is a read of the array's
-- operation, which names no vector, so the observer builds nothing from the
-- result's fact; the evidence the split kept gives the element's fact once,
-- and is gone: no second copy after a further opening, a substitution or a
-- later split. The substitution and the split come after the evidence was
-- paid: they show that the fact is not produced again, and nothing about
-- evidence still unpaid when the vector it names is rewritten by an
-- equation, which no control here claims.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (k j : Nat)
    (same : k = j) (gate : Prop) (passed : gate) (late : gate → NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    readsOwn t (eraseBounded v 0) k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_element_facts 0 0
    assert_evidence 1
    verex_normalize at heq
    assert_normal_read Array.eraseIdxIfInBounds
    verex_certified_clauses
    verex_observed_elements
    assert_evidence 1
    assert_own_clauses 0
    have held := late passed
    assert_observed held 0
    verex_certified_clauses
    assert_evidence 0
    assert_own_clauses 1
    verex_certified_clauses
    verex_observed_elements
    assert_own_clauses 1
    subst same
    verex_certified_clauses
    verex_observed_elements
    assert_own_clauses 1
    by_cases first : k = 0
    all_goals
      verex_certified_clauses
      verex_observed_elements
      assert_own_clauses 1
      assumption

-- The same through the opener of observed reads, with the read's equation
-- gone from the context: the evidence is the read it sees.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (k : Nat)
    (gate : Prop) (passed : gate) (late : gate → NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    readsOwn t (eraseBounded v 0) k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    clear heq
    have held := late passed
    verex_observed_elements
    assert_evidence 0
    assert_own_clauses 1
    assumption

-- The fact that arrives is the one of the vector a preserving operation is
-- of: the result's fact follows from it, and pays the evidence.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (i j k : Nat)
    (gate : Prop) (passed : gate) (late : gate → NTy.Holds (.vector (Element t)) v) : readsOwn t (swapBounded v i j) k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_evidence 1
    verex_normalize at heq
    assert_normal_read Array.swapIfInBounds
    have hv := late passed
    assert_observed hv 0
    verex_certified_clauses
    assert_evidence 0
    assert_own_clauses 1
    assumption

-- The fact of a written element arrives inside the fact of a value that
-- holds it, which the same opening reads: the evidence is paid by what that
-- worklist has read, with the vector's fact the context states.
set_option linter.unusedVariables false in
example [Carriers] (s t : StructHandle) (v : (NTy.vector (Element t)).carrier)
    (x : (Element t).carrier) (k : Nat) (within : 0 < v.values.size)
    (hv : NTy.Holds (.vector (Element t)) v)
    (gate : Prop) (passed : gate) (late : gate → NTy.Holds (.struct s .nil (.cons (Element t) .nil)) (x, ())) :
    readsOwn t (v.set 0 x) k := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    assert_evidence 1
    have holder := late passed
    verex_certified_clauses
    assert_evidence 0
    first
      | assumption
      | fail "the element's own clause is not among the opened ones"

-- The fact arrives in one case of a later split only: the element's fact is
-- there, and in the other case the evidence is as it was.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier) (k : Nat)
    (arrives : Prop) (late : arrives → NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    wp (σ := Unit) (ε := Unit)
      (withVectorElem (Element t) (eraseBounded v 0) k (fun element => Spec.pure element)
        (Spec.abort ()))
      (fun result _ => arrives → Carriers.own t .nil (NTy.encode (Element t) result))
      (fun _ => True) () := by
  split_vector_read
  case none => finish_read
  case some =>
    finish_read
    by_cases here : arrives
    · intro _
      have held := late here
      verex_certified_clauses
      assert_evidence 0
      assumption
    · verex_certified_clauses
      verex_observed_elements
      assert_evidence 1
      assert_element_facts 0 0
      assert_own_clauses 0
      intro there
      exact absurd there here

-- Two hypotheses of evidence that owe one fact, as two reads of one position
-- leave them or two element names made equal: the consumer builds the fact
-- once and retires the other without a proof, and the opener clears both.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier)
    (x : (Element t).carrier)
    (first second : ElementFactOf (Element t) (eraseBounded v 0) x)
    (held : NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    Carriers.own t .nil (NTy.encode (Element t) x) := by
  assert_paid 1 1
  verex_certified_clauses
  assert_paid 0 0
  assert_own_clauses 1
  assumption

set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier)
    (x y : (Element t).carrier) (same : x = y)
    (first : ElementFactOf (Element t) (eraseBounded v 0) x)
    (second : ElementFactOf (Element t) (eraseBounded v 0) y)
    (held : NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    Carriers.own t .nil (NTy.encode (Element t) y) := by
  assert_paid 2 0
  subst same
  assert_paid 1 1
  verex_observed_elements
  assert_paid 0 0
  assert_own_clauses 1
  assumption

-- Evidence at two types of one value owes two facts: both are built.
example [Carriers] (s t : StructHandle) (v : (NTy.vector (Element s)).carrier)
    (x : (Element s).carrier)
    (first : ElementFactOf (Element s) (eraseBounded v 0) x)
    (second : ElementFactOf (Element t) (eraseBounded v 0) x)
    (hs : NTy.Holds (.vector (Element s)) (eraseBounded v 0))
    (ht : NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    Carriers.own s .nil (NTy.encode (Element s) x) ∧
      Carriers.own t .nil (NTy.encode (Element t) x) := by
  assert_paid 2 0
  verex_certified_clauses
  assert_paid 0 0
  exact ⟨by assumption, by assumption⟩

-- A write over a write at one position, where the fact of the first write's
-- result is one the same opening reads out of a value that holds it, and the
-- context states no fact of the vector under both writes: the first write is
-- not dead to the proof, its result's fact is the base the second write is
-- proved over.
set_option linter.unusedVariables false in
theorem written_over_known_write [Carriers] (s t : StructHandle) (v : (NTy.vector (Element t)).carrier)
    (x y e : (Element t).carrier) (within : 0 < v.values.size)
    (owed : ElementFactOf (Element t) ((v.set 0 x).set 0 y) e)
    (hy : NTy.Holds (Element t) y)
    (holder : NTy.Holds (.struct s .nil (.cons (.vector (Element t)) .nil)) (v.set 0 x, ())) :
    Carriers.own t .nil (NTy.encode (Element t) e) := by
  assert_owed 1
  verex_certified_clauses
  assert_owed 0
  first
    | assumption
    | fail "the element's own clause is not among the opened ones"

-- The evidence is paid only where the worklist has a round left to read the
-- fact it gives. With one round, which the fact of the vector itself takes,
-- the evidence is still in the context after the opening and nothing was
-- stated for it; with two it is paid, read and retired once. The limit is
-- the opener's own parameter, lowered here: the comparison that guards the
-- payment is the one that runs at the default.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier)
    (x : (Element t).carrier) (owed : ElementFactOf (Element t) (eraseBounded v 0) x)
    (held : NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    Carriers.own t .nil (NTy.encode (Element t) x) := by
  run_tac assertCertifiedClauses pure none (rounds := 1)
  assert_paid 1 0
  assert_own_clauses 0
  run_tac assertCertifiedClauses pure none (rounds := 2)
  assert_paid 0 0
  assert_own_clauses 1
  assumption

-- The opener of observed reads at its limit: with no round the evidence
-- stays, with its rounds it is paid and retired once.
example [Carriers] (t : StructHandle) (v : (NTy.vector (Element t)).carrier)
    (x : (Element t).carrier) (owed : ElementFactOf (Element t) (eraseBounded v 0) x)
    (held : NTy.Holds (.vector (Element t)) (eraseBounded v 0)) :
    Carriers.own t .nil (NTy.encode (Element t) x) := by
  run_tac assertObservedElementClauses pure (rounds := 0)
  assert_paid 1 0
  assert_own_clauses 0
  run_tac assertObservedElementClauses pure
  assert_paid 0 0
  assert_own_clauses 1
  assumption

-- The fact of another vector pays nothing, and the evidence alone proves
-- neither the element's fact nor its clause.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle) (v w : (NTy.vector (Element t)).carrier) (k : Nat)
    (gate : Prop) (passed : gate) (late : gate → NTy.Holds (.vector (Element t)) (eraseBounded w 0)) : True := by
  have : readsAny t (eraseBounded v 0) k := by
    split_vector_read
    case none => finish_read
    case some =>
      assert_evidence 1
      have other := late passed
      verex_certified_clauses
      verex_observed_elements
      assert_evidence 1
      assert_element_facts 0 0
      assert_own_clauses 0
      finish_read
  fail_if_success
    have : readsOwn t (eraseBounded v 0) k := by
      split_vector_read
      case none => finish_read
      case some =>
        finish_read
        have other := late passed
        verex_certified_clauses
        verex_observed_elements
        assumption
  fail_if_success
    have : readsHolds t (eraseBounded v 0) k := by
      split_vector_read
      case none => finish_read
      case some =>
        finish_read
        first | assumption | (verex_holds; done)
  trivial

/-! ## Reads the array decides -/

private def three : SpecVector (SpecInt (.bits 64) false) :=
  ⟨#[⟨10, by decide⟩, ⟨20, by decide⟩, ⟨30, by decide⟩], by decide⟩

-- A literal vector: the read takes its branch in the normal form, with no
-- split and no equation, at a position it holds and at one it does not.
example [Carriers] :
    wp (σ := Unit) (ε := Unit)
      (withVectorElem (.int 64 false) three 1 (fun element => Spec.pure element.val) (Spec.abort ()))
      (fun result _ => result = 20) (fun _ => False) () := by
  simp only [three, vir_denote_norm]

example [Carriers] :
    wp (σ := Unit) (ε := Unit)
      (withVectorElem (.int 64 false) three 3 (fun element => Spec.pure element.val) (Spec.abort ()))
      (fun _ _ => False) (fun _ => True) () := by
  simp only [three, vir_denote_norm]

-- A read no array decides stays a read under normalization, and its split
-- leaves none.
example [Carriers] (v : (NTy.vector (.int 64 false)).carrier) (k : Nat) :
    wp (σ := Unit) (ε := Unit)
      (withVectorElem (.int 64 false) v k (fun element => Spec.pure element.val) (Spec.abort ()))
      (fun _ _ => True) (fun _ => True) () := by
  fail_if_success (simp only [vir_denote_norm])
  split_vector_read
  all_goals assert_no_read_residue
  all_goals finish_read

end VerexIR.Tests.VectorElementReads
