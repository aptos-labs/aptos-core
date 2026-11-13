-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR.Proofs

-- The block sees the conjunction once, before any clause becomes a leaf.
-- Its case split must preserve the source names on both resulting paths.
example (flag : Bool) (n : Int) :
    Obligation "shared" 0 1 (n < n + 1) ∧
    Obligation "shared" 2 3 (n < n + 2) ∧
    Obligation "shared" 6 7 (∀ m : Int, n < m → n < m + 1) := by
  verex_obligations [] naming
      [("first", "shared", 0, 1), ("second", "shared", 2, 3), ("third", "shared", 6, 7)] preparing
    run_tac do
      unless (← Lean.Elab.Tactic.getMainTarget).isAppOfArity ``And 2 do
        throwError "shared preparation received an already split clause"
    cases flag
    all_goals verex_prepare values
    -- Substitution/map lifting may annotate the new target.
    all_goals run_tac do
      let goal ← Lean.Elab.Tactic.getMainGoal
      Lean.Elab.Tactic.setGoals [← goal.replaceTargetDefEq (.mdata {} (← goal.getType))]
  verex_on
  | first => verex_paths; all_goals (simp only [Obligation_eq]; change n < n + 1; omega)
  | second => verex_paths; all_goals (simp only [Obligation_eq]; change n < n + 2; omega)
  | third => verex_paths; all_goals (simp only [Obligation_eq]; intro m h; omega)
  | _ => fail "shared preparation lost a clause name"

-- The broader preparation also derives map-position facts. It must retain
-- the goal markers while unwrapping the assumptions those facts read.
example (m : VerexIR.RuntimeValue) :
    Obligation "map" 0 1 (VerexIR.Maps.rank m (.integer 0) < VerexIR.Maps.rank m (.integer 0) + 1) ∧
    Obligation "map" 2 3 (VerexIR.Maps.rank m (.integer 0) < VerexIR.Maps.rank m (.integer 0) + 2) := by
  verex_obligations [] naming [("first", "map", 0, 1), ("second", "map", 2, 3)] preparing
    verex_prepare
  verex_on
  | first, second => verex_paths; all_goals (simp only [Obligation_eq]; omega)
  | _ => fail "map-position preparation lost a clause name"

-- Markers on assumptions must not hide contradictions during preparation.
example (h : Obligation "shared" 4 5 False) : False := by
  verex_obligations [] preparing
    verex_prepare values
  done

-- A preparation block can close some branches, but must retain the others.
-- An invalid second conjunct is not proved by closing the first one.
example : True := by
  fail_if_success
    have : (0 : Int) ≤ 0 ∧ (0 : Int) < 0 := by
      verex_obligations [] preparing
        constructor
        · exact Int.le_refl 0
        · skip
      all_goals omega
  trivial

-- The verifier must inject the generated call/loop metadata while retaining
-- the authored block. The modifier remains usable as an ordinary identifier.
verex module 0x42::shared_preparation where
  fun source(x : u64) -> u64 := x
  spec source where
    pragma opaque
    aborts_if false
    ensures result == x

  fun copy_twice(x : u64) -> u64 := source(source(x))
  spec copy_twice where
    aborts_if false
    ensures result == x
    ensures result <= x
    ensures result >= x

  fun count(limit : u64) -> u64 := do
    let mut i := 0
    while i < limit do
      i := source(i) + 1
    spec do
      invariant i <= limit
    source(i)
  spec count where
    aborts_if false
    ensures result == limit
    ensures result <= limit

verify 0x42::shared_preparation::source
verify 0x42::shared_preparation::copy_twice by lean =>
  verex_obligations preparing
    verex_prepare values
    all_goals have preparing : True := trivial
  all_goals verex_decide

-- Call and loop obligations share the same preparation interface.
verify 0x42::shared_preparation::count by lean =>
  verex_obligations preparing
    verex_prepare values
  all_goals verex_decide

-- A premise introduced for one clause stays inside that clause when its
-- siblings share an authored block; the block sees the common conjunction.
open VerexIR.Proofs.Denote in
example (n : Int) :
    Obligation "shared" 0 1 (n < n + 1) ∧
    Obligation "shared" 2 3 (n = n → n < n + 2) := by
  verex_obligations [] naming
    [("ensures_1", "shared", 0, 1), ("ensures_2", "shared", 2, 3)] preparing
      verex_step_prepare
  verex_step_route
    on ensures 1, ensures 2 {
      lean =>
        run_tac do
          unless (← Lean.Elab.Tactic.getMainTarget).isAppOfArity ``And 2 do
            throwError "sibling clauses did not share their path"
    }

-- Clause-local equalities must still rewrite shared hypotheses after grouping.
open VerexIR.Proofs.Denote in
example (P : Int → Prop) (n : Int) (h : P n) :
    Obligation "shared" 0 1 (n = 0 → P 0) ∧
    Obligation "shared" 2 3 (n = 1 → P 1) := by
  verex_obligations [] naming
    [("ensures_1", "shared", 0, 1), ("ensures_2", "shared", 2, 3)] preparing
      verex_step_prepare
  verex_step_route
    on ensures 1, ensures 2 { }

-- The prefix runs on the shared conjunction, even when it splits a value.
-- Clause binders remain private until entering the corresponding child.
open VerexIR.Proofs.Denote in
example (flag : Bool) (n : Int) :
    Obligation "shared" 0 1 (∀ i : Int, n ≤ i → n < i + 1) ∧
    Obligation "shared" 2 3 (n < n + 2) := by
  verex_obligations [] naming
    [("ensures_1", "shared", 0, 1), ("ensures_2", "shared", 2, 3)] preparing
      verex_step_prepare
  verex_step_route
    on ensures 1, ensures 2 {
      lean =>
        run_tac Lean.Elab.Tactic.withMainContext do
          unless (← Lean.Elab.Tactic.getMainTarget).isAppOfArity ``And 2 do
            throwError "shared prefix was not grouped"
          if (← Lean.getLCtx).any (fun decl => decl.userName.eraseMacroScopes == `i) then
            throwError "clause binder escaped into shared prefix"
        have shared : n < n + 1 := by omega
        cases flag

      on ensures 1 {
        lean =>
          run_tac Lean.Elab.Tactic.withMainContext do
            unless (← Lean.getLCtx).any (fun decl => decl.userName.eraseMacroScopes == `i) do
              throwError "child lost its clause binder"
          omega
      }
      on ensures 2 { lean => omega }
    }

-- Exercise lowering and clause-local names through the public language.
verex module 0x42::nested_scope where
  fun advance(x : u64) -> u64 := x + 1
  spec advance where
    requires x < 100
    aborts_if false
    ensures ∀ (i : u64), result <= i ==> x < i
    ensures result > x

  verify advance by
    on ensures 1, ensures 2 {
      assert result > x by arith
      on ensures 1 { assert x < i by arith }
    }
    on _ { }

/--
error: `i` is not in scope at this obligation; in scope: x, old_x
---
error: verex verification failed: the proof of `advance` does not establish its specification
-/
#guard_msgs in
verex module 0x42::nested_bad_scope where
  fun advance(x : u64) -> u64 := x + 1
  spec advance where
    requires x < 100
    aborts_if false
    ensures ∀ (i : u64), result <= i ==> x < i
    ensures result > x

  verify advance by
    on ensures 1, ensures 2 {
      assert i == i
      on ensures 1 { assert x < i by arith }
    }
    on _ { }

/--
error: the nested `on` names a clause outside its parent block
---
error: verex verification failed: the proof of `advance` does not establish its specification
-/
#guard_msgs in
verex module 0x42::nested_bad_clause where
  fun advance(x : u64) -> u64 := x + 1
  spec advance where
    requires x < 100
    aborts_if false
    ensures ∀ (i : u64), result <= i ==> x < i
    ensures result > x

  verify advance by
    on ensures 1, ensures 2 {
      assert result > x by arith
      on ensures 3 { }
    }
    on _ { }

-- An inlined helper can reuse a reference name for a plain vector.
verex module 0x42::shadowed_scope where
  fun size(self : &mut u64, xs : Vector<u64>) -> u64 := do
    let self := xs
    self.length
  spec size where
    aborts_if false
    ensures result == xs.length
  verify size by
    on ensures 1 {
      assert result == xs.length by eval
    }
    on _ { }

-- Primitive read computation respects both sides of a removed position.
open VerexIR.Proofs VerexIR.Proofs.Denote
example (xs : Array Int) (pos : Nat) (valid : pos < xs.size)
    (i : Int) (nonnegative : 0 ≤ i) (before : i < pos) :
    Obligation "compute" 0 1 ((xs.eraseIdx pos valid)[i.toNat]? = xs[i.toNat]?) := by
  verex_obligations [] naming [("ensures_1", "compute", 0, 1)] preparing
    verex_step_prepare
  verex_step_route
    on ensures 1 { compute }

example (xs : Array Int) (pos : Nat) (valid : pos < xs.size)
    (i : Int) (after : pos ≤ i) :
    Obligation "compute" 0 1 ((xs.eraseIdx pos valid)[i.toNat]? = xs[(i + 1).toNat]?) := by
  verex_obligations [] naming [("ensures_1", "compute", 0, 1)] preparing
    verex_step_prepare
  verex_step_route
    on ensures 1 { compute }

-- Applying a lemma must retain its negative conclusion as a fact.
verex module 0x42::negative_lemma where
  fun remove_first(xs : Vector<u64>) -> Vector<u64> := do
    let (_, remaining) := core.prim.removeVector(xs, 0)
    remaining
  spec remove_first where
    requires xs.length > 0
    requires ∀ (i in 0 .. xs.length), ∀ (j in 0 .. xs.length), xs[i] == xs[j] ==> i == j
    aborts_if false
    ensures !(∃ (i in 0 .. result.length), result[i] == xs[0])
  verify remove_first by
    on ensures 1 {
      apply lemma_vector_contains_remove(xs, 0, xs[0])
    }
    on _ { }

-- A negative membership fact rules out every optional read, even outside bounds.
example {α : Type} (xs : Array α) (x : α) (i : Nat) (missing : ¬x ∈ xs) :
    xs[i]? ≠ some x := by
  verex_block_decide

-- The fast default still exposes a clause's final tuple equality.
verex module 0x42::scoped_scalar where
  struct Pair has Copy, Drop where
    a : u64
    b : u64
  fun preserve(s : &mut Pair) -> Unit := ()
  spec preserve where
    pragma opaque
    aborts_if false
    ensures s.a == old(s).a
    ensures s.b == old(s).b
  fun add(s : &mut Pair, amount : u64) -> Unit := do
    s.a := s.a + amount
    preserve(s)
  spec add where
    requires s.a + amount <= 18446744073709551615
    aborts_if false
    ensures s.a == old(s).a + amount
    ensures s.b == old(s).b
  verify add by
    on _ { }

-- A clause with multiple paths must expose its saved preparation marker.
verex module 0x42::scoped_scalar_calls where
  struct Pair has Copy, Drop where
    a : u64
    b : u64
  fun preserve(s : &mut Pair) -> Unit := ()
  spec preserve where
    pragma opaque
    aborts_if false
    ensures s.a == old(s).a
    ensures s.b == old(s).b
  fun amount_of(s : &Pair, amount : u64) -> u64 := amount
  spec amount_of where
    pragma opaque
    aborts_if false
    ensures result == amount
  fun add(s : &mut Pair, amount : u64) -> Unit := do
    if amount == 0 then return ()
    let new_shares := amount_of(s, amount)
    let _t1 := &mut s.a
    *_t1 := *_t1 + amount
    let _t1 := &mut s.b
    *_t1 := *_t1 + new_shares
    preserve(s)
  spec add where
    requires s.a + amount <= 18446744073709551615
    requires s.b + amount <= 18446744073709551615
    aborts_if false
    ensures s.a == old(s).a + amount
    ensures s.b == old(s).b + amount
  verify add by
    on _ { }

-- The authored normal form must also preserve scoped structure fields.
verex module 0x42::scoped_scalar_fields where
  struct Pair has Copy, Drop where
    a : u64
    b : u64
  fun preserve(s : &mut Pair) -> Unit := ()
  spec preserve where
    pragma opaque
    aborts_if false
    ensures s.a == old(s).a
    ensures s.b == old(s).b
  fun amount_of(s : &Pair, amount : u64) -> u64 := amount
  spec amount_of where
    pragma opaque
    aborts_if false
    ensures result == amount
  fun add(s : &mut Pair, amount : u64) -> Unit := do
    if amount == 0 then return ()
    let new_shares := amount_of(s, amount)
    let _t1 := &mut s.a
    *_t1 := *_t1 + amount
    let _t1 := &mut s.b
    *_t1 := *_t1 + new_shares
    preserve(s)
  spec add where
    requires s.a + amount <= 18446744073709551615
    requires s.b + amount <= 18446744073709551615
    aborts_if false
    ensures s.a == old(s).a + amount
    ensures s.b == old(s).b + amount
  verify add by
    on ensures 1, ensures 2 { }
    on _ { }

-- A quantified-instance fallback must assign the proof, not merely compare
-- two proof terms using proof irrelevance. Before the fix the kernel rejected
-- this theorem because the assertion retained an opaque metavariable.
verex module 0x42::instance_assignment where
  fun first(xs : Vector<u64>) -> u64 := xs[0]
  spec first where
    requires xs.length > 0
    requires ∀ (i : Int), 0 <= i && i < xs.length ==> xs[i] == 7
    aborts_if false
    ensures result == 7
  verify first by
    on ensures 1 {
      assert result == 7 by arith
    }
    on _ { }

-- Matching siblings accumulate facts in source order. The default runs only
-- for the other clause, whose context must not acquire those sibling facts.
open VerexIR.Proofs.Denote in
example (P : Prop) (h : P) (n : Int) :
    Obligation "overlap" 0 1 (n < n + 1) ∧
    Obligation "overlap" 2 3 (n < n + 2) := by
  verex_obligations [] naming
    [("ensures_1", "overlap", 0, 1), ("ensures_2", "overlap", 2, 3)] preparing
      verex_step_prepare
  verex_step_route
    on ensures 1, ensures 2 {
      on ensures 1 {
        lean =>
          have accumulated : P := h
          clear h
      }
      on ensures 1 {
        lean =>
          have retained : P := accumulated
      }
      on _ {
        lean =>
          run_tac Lean.Elab.Tactic.withMainContext do
            if (← Lean.getLCtx).any (fun decl =>
                decl.userName.eraseMacroScopes == `accumulated) then
              throwError "nested default ran for a matched clause or inherited its facts"
          omega
      }
    }

-- Each nesting level rewraps the clause boundary. After a shared case split,
-- grandchildren must still select the right original clause on both paths.
open VerexIR.Proofs.Denote in
example (flag : Bool) (P : Int → Prop) (h : ∀ i, P i) :
    Obligation "deep" 0 1 (P 0) ∧
    Obligation "deep" 2 3 (P 1) := by
  verex_obligations [] naming
    [("ensures_1", "deep", 0, 1), ("ensures_2", "deep", 2, 3)] preparing
      verex_step_prepare
  verex_step_route
    on ensures 1, ensures 2 {
      lean => cases flag
      on ensures 1, ensures 2 {
        on ensures 1 { lean => exact h 0 }
        on ensures 2 { lean => exact h 1 }
      }
    }

-- An explicit closer acts on the remaining obligation, and must close it.
example (n : Int) : n < n + 1 := by
  verex_step_obligation on _ { finish by arith }

example (_P : Prop) : True := by
  fail_if_success
    have : _P := by
      verex_step_obligation on _ { finish by decide }
  trivial

-- Ground map evaluation follows a read's defining equation in the context.
example (m : VerexIR.SpecMap Int Int) (key value : Int)
    (entries : m = [(1, 4), (2, 5)]) (read : value = m.valueAt key) :
    value = (if 1 = key then 4 else if 2 = key then 5 else 0) := by
  verex_step_obligation on _ { finish by eval }

-- A sum is transported through an offset equality, beneath the tuple used
-- by specification functions. The closer needs both transitivity and
-- congruence, without evaluating the function at a symbolic argument.
private def equalityChainSum (values : Nat → Int) : Nat → Int
  | 0 => 0
  | n + 1 => equalityChainSum values n + values n

example (values : Nat → Int) (base next count : Nat) (sum : Int)
    (offset : next = base + count)
    (total : sum = equalityChainSum values (base + count)) :
    sum = equalityChainSum values next := by
  verex_step_obligation on _ { finish by congr }

example (f : Int × Int → Int) (a b c d sum : Int)
    (ab : a = b) (bc : b = c) (total : sum = f (c, d)) :
    sum = f (a, d) := by
  verex_step_obligation on _ { finish by congr }

-- Unrelated equations must not make an arbitrary function value provable;
-- the failed equality attempt also leaves the original goal usable.
example (_f : Int → Int) (a b : Int) (_ab : a = b) : True := by
  fail_if_success
    have : _f a = _f (b + 1) := by
      verex_step_obligation on _ { finish by congr }
  trivial


-- The old map-reading branch cannot instantiate a quantified fact whose
-- conclusion requires cancelling an offset. The equality premise pins the
-- binder before blockInstance normalizes that conclusion.
example (m : VerexIR.SpecMap Int Int) (key : Int) (f : Int → Int)
    (pin : f key = key)
    (fact : ∀ k : Int, f k = key → (m.valueAt k + 1) - 1 = 7) :
    m.valueAt key = 7 := by
  verex_block_decide

-- Independently, the same fallback supplies symmetric quantified instances.
example (m : VerexIR.SpecMap Int Int) (key : Int)
    (fact : ∀ k : Int, m.valueAt k = 7) : 7 = m.valueAt key := by
  verex_block_decide

-- Quantified matching reduces representation wrappers without unfolding
-- the observed map operation. Reducible-only matching misses this instance.
private def representationIndex (i : Int) : Int := i
example (m : VerexIR.SpecMap Int Int) (key : Int)
    (fact : ∀ k : Int, m.valueAt (representationIndex k) = 7) :
    m.valueAt key = 7 := by
  verex_block_decide

-- One equality operand pins the index; the whole instantiated equality
-- still has to agree after applying the frame equation.
example (m : VerexIR.SpecMap Int Int) (key result : Int) (f : Int → Int)
    (frame : result = f key) (fact : ∀ k : Int, f k = m.valueAt k) :
    result = m.valueAt key := by
  verex_block_decide

example (m : VerexIR.SpecMap Int Int) (key result : Int) (f : Int → Int)
    (_frame : result = f key) (_fact : ∀ k : Int, f k = m.valueAt k + 1) : True := by
  fail_if_success
    have : result = m.valueAt key := by
      verex_block_decide
  trivial

-- Equalities with a shared left side must also simplify one another.
example (values : Nat → Int) (index other : Nat) (left right : Int)
    (sameIndex : index = other)
    (first : equalityChainSum values index = left)
    (second : equalityChainSum values other = right) : left = right := by
  verex_step_obligation on _ { finish by congr }


-- The helper theorem returns Ne, whereas normalized authored obligations
-- can spell the same proposition as Not (Eq ...).
example (xs : Array Int) (i : Nat) (x : Int) (missing : ¬x ∈ xs) :
    xs[i]? ≠ some x := by
  verex_block_decide

example (xs : Array Int) (i : Nat) (x : Int) (missing : ¬x ∈ xs) :
    ¬(xs[i]? = some x) := by
  verex_block_decide

-- Merely matching Array/Nat must not authorize the standard array theorem
-- for a different GetElem? instance. This instance always returns 7.
section
local instance : GetElem? (Array Int) Nat Int (fun _ _ => True) where
  getElem _ _ _ := 7
  getElem? _ _ := some 7

example (xs : Array Int) (_i : Nat) (_missing : ¬(7 : Int) ∈ xs) : True := by
  fail_if_success
    have : ¬(xs[_i]? = some (7 : Int)) := by
      verex_block_decide
  trivial
end

-- Both directions can be inherited from contracts and authored facts. Orient
-- them consistently instead of cycling between a name and its definition.
example (f : Int → Int) (i sum value : Int)
    (_named : sum = f i) (_observed : f i = sum)
    (next : f (i + 1) = f i + value) : sum + value = f (i + 1) := by
  verex_block_congr

-- Opening an own predicate must terminate even when another hypothesis
-- depends on its proof, so Lean retains the original declaration.
section
open VerexIR.Proofs.Denote
set_option maxHeartbeats 20000
example {unit : VerexIR.Validation.ValidatedUnit}
    [FunctionCarriers unit] [Skolems unit]
    (source : VerexIR.StructHandle) (raw : VerexIR.RuntimeValue)
    (held : Carriers.own source NRow.nil raw) (_dependent : held = held) : True := by
  verex_split_hypotheses
  trivial
end

-- Normalization keeps a forgotten vector whole for certification and equality;
-- elementwise forgetting is exposed only when an operation reads its entries.
section
open VerexIR.Proofs VerexIR.Proofs.Denote
variable {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
example (site : Nat) (τ : NTy) (value : VerexIR.SpecVector τ.carrier) :
    certifyingAt (State := Memory unit) site .copyOut (.vector τ) (NTy.forget (.vector τ) value) =
      Spec.pure value := by
  simp only [vir_denote, vir_denote_norm]

example (τ : NTy) (left right : VerexIR.SpecVector τ.carrier)
    (equal : NTy.forget (.vector τ) left = NTy.forget (.vector τ) right) : left = right := by
  simpa only [vir_denote, vir_denote_norm] using equal
end

-- A pass that deliberately skips a premise must not cache it as normalized.
-- The next pass still needs to expose its conjunction before assumption.
example (p : Prop) (_held : True ∧ p) : p := by
  run_tac VerexIR.Proofs.Denote.normalizeContext true (fun _ => false)
  verex_normalize_context
  verex_assumption

-- Zero arithmetic propagates through named values without expanding their
-- definitions into the goal. The quotient can be declared before its numerator.
example (output numerator denominator reserve : Int)
    (quotient : output = numerator / denominator)
    (product : numerator = reserve * 0) : output = 0 := by
  verex_block_decide

example (output numerator denominator reserve : Int)
    (product : numerator = 0 * reserve)
    (quotient : output = numerator / denominator)
    (nonnegative : 0 ≤ reserve) : output ≤ reserve := by
  verex_block_decide

-- A zero value unrelated to the numerator cannot prove that quotient zero.
example (output numerator denominator reserve other : Int)
    (_product : other = reserve * 0)
    (_quotient : output = numerator / denominator) : True := by
  fail_if_success
    have : output = 0 := by verex_block_decide
  trivial

-- The zero laws belong to the standard integer operations, not to an
-- arbitrary overloaded operation with the same operand and result types.
section
local instance : HDiv Int Int Int := ⟨fun _ _ => 7⟩
example (output numerator denominator reserve : Int)
    (_product : numerator = reserve * 0)
    (_quotient : output = numerator / denominator) : True := by
  fail_if_success
    have : output = 0 := by verex_block_decide
  trivial
end
section
local instance : HMul Int Int Int := ⟨fun _ _ => 7⟩
example (output numerator denominator reserve : Int)
    (_product : numerator = reserve * 0)
    (_quotient : output = numerator / denominator) : True := by
  fail_if_success
    have : output = 0 := by verex_block_decide
  trivial
end

-- A sign fact available at the arithmetic decision can canonicalize a
-- quotient or remainder stated before that fact was introduced.
example (x y : Int) (nonnegative : 0 ≤ x)
    (monotone : x.tdiv 10 ≤ y.tdiv 10) (positive : 0 < x / 10) : 0 < y.tdiv 10 := by
  verex_block_decide
example (x remainder : Int) (nonnegative : 0 ≤ x)
    (named : remainder = x.tmod 10) (bounded : x % 10 = 3) : remainder = 3 := by
  verex_block_decide
example (x quotient : Int) (_negative : x < 0)
    (_named : quotient = x.tdiv 10) : True := by
  fail_if_success
    have : quotient = x / 10 := by verex_block_decide
  trivial

-- Structural quotient signs also need the ground denominator's sign.
example (x s o q w : Int) (x0 : 0 ≤ x) (s0 : 0 ≤ s) (o0 : 0 ≤ o)
    (monotone : (o * (x * s).tdiv 10).tdiv (x + (x * s).tdiv 10) ≤ q)
    (named : w = o * (x * s / 10) / (x + x * s / 10)) : w ≤ q := by
  verex_block_decide
