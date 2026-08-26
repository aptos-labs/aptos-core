-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.State
import MoveModel.IR.ValueTyping

/-!
# The Deep Specification Language

Move specifications remain expression trees even after code becomes
three-address bytecode.  Contracts, loop invariants, and injected `Prop`
instructions carry move-model expressions (`ExpData`).  `SpecExp` is the
corresponding deep syntax used by this formalization.

Mirrored move-model concepts:

* `SpecExp.loc` — `ExpData::Temporary`: reference to a bytecode local;
* `SpecExp.bvar` — `ExpData::LocalVar`, as de Bruijn indices;
* `SpecExp.global` and `SpecExp.exists_` mirror global memory operations.
  An optional `MemLabel` selects a saved memory snapshot.  The spec translator
  represents `old(global<R>(a))` as `global<R>(a)` at the label saved on
  function entry.  There is therefore no separate `old` constructor here.
* `SpecExp.quant` mirrors `ExpData::Quant`.  Its binder records a domain type,
  and evaluation ranges over values satisfying `IsValid` for that type.  This
  corresponds to Boogie's typed quantifiers and validity guards.  In
  particular, Boogie represents `u64` as a bounded `int`, while specification
  integers remain unbounded.

## Semantics

Specification arithmetic uses unbounded integers (`num`) and produces
`SVal`s.  Quantifiers may range over infinite domains, so evaluation is a
relational big-step judgment, `EvalSpec env e sv`, rather than an executable
function.

Evaluation is partial.  Ill-typed expressions, missing global resources, and
division by zero have no result.  `&&`, `||`, and `==>` short-circuit from the
left.  Thus a guard such as `exists<R>(a) ==> P(global<R>(a))` remains defined
when the resource is absent.  `evalSpec_det` proves determinism.

The `evalSpec_*_iff` simp pack characterizes evaluation per constructor; it
reduces concrete spec reasoning (in examples) to plain logical goals.
-/

namespace MoveModel.IR

/-- Memory-snapshot label (`MemoryLabel` in move-model): identifies a state
snapshot recorded by the `SaveMem` mechanism.  By convention `preLabel = 0`
is the snapshot of the function's pre-state, bound at function entry. -/
abbrev MemLabel := Nat

/-- The pre-state snapshot label. -/
def preLabel : MemLabel := 0

/-- Quantifier kinds (move-model `QuantKind`, restricted to `Forall` (`all`)
and `Exists` (`ex`); `Choose` is out of scope). -/
inductive QuantKind where
  | all
  | ex
  deriving DecidableEq

/-- Binary spec operators.  `and`, `or`, `implies` short-circuit; the rest
are strict (see `SpecBinop.strictEval`).  `index` selects a vector element
(undefined — stuck — out of range or at a negative index). -/
inductive SpecBinop where
  | add | sub | mul | div | mod
  | lt | le | eq
  | index
  | and | or | implies | iff
  deriving DecidableEq

/-- Deep specification expressions — the pure/spec fragment of move-model
`ExpData` (see module docs for the correspondence). -/
inductive SpecExp where
  | value (v : Value)
  | loc (t : LocalIndex)
  | bvar (k : Nat)
  | result (i : Nat)
  | binop (op : SpecBinop) (e₁ e₂ : SpecExp)
  | not (e : SpecExp)
  | select (field : Nat) (e : SpecExp)
  | len (e : SpecExp)
  | mutVal (e : SpecExp)
  | global (r : ResourceId) (lbl : Option MemLabel) (addr : SpecExp)
  | exists_ (r : ResourceId) (lbl : Option MemLabel) (addr : SpecExp)
  | ite (c t e : SpecExp)
  | quant (k : QuantKind) (dom : Ty) (body : SpecExp)

/-- Substitute declaration-scoped type parameters occurring in specification
quantifier domains.  Global-resource expressions currently name nongeneric
resources; instantiated resource selectors are introduced separately from
this structural substitution. -/
def SpecExp.instantiate (args : List Ty) : SpecExp → SpecExp
  | .value v => .value v
  | .loc i => .loc i
  | .bvar i => .bvar i
  | .result i => .result i
  | .binop op lhs rhs => .binop op (lhs.instantiate args) (rhs.instantiate args)
  | .not e => .not (e.instantiate args)
  | .select field e => .select field (e.instantiate args)
  | .len e => .len (e.instantiate args)
  | .mutVal e => .mutVal (e.instantiate args)
  | .global r label address => .global r label (address.instantiate args)
  | .exists_ r label address => .exists_ r label (address.instantiate args)
  | .ite c t e => .ite (c.instantiate args) (t.instantiate args) (e.instantiate args)
  | .quant kind domain body =>
      .quant kind (domain.instantiate args) (body.instantiate args)

/-- Spec evaluation environment: the state a specification expression is
evaluated against.  Which parts are meaningful depends on the position of
the expression (contract clause, loop invariant, …); the discipline is
documented at `Contract`. -/
structure SpecEnv where
  /-- The struct declarations, bounding quantifier domains (`IsValid`). -/
  structs : StructDecls
  /-- The locals `SpecExp.loc` refers to. -/
  locals : Locals
  /-- Return values, for `SpecExp.result` in `ensures` positions. -/
  result : List Value
  /-- Current global memory (`global`/`exists_` with label `none`). -/
  mem : Memory
  /-- Memory snapshots (`global`/`exists_` with `some` label). -/
  snaps : MemLabel → Memory
  /-- de Bruijn stack of quantifier-bound values. -/
  bound : List SVal

namespace SpecEnv

/-- Enter a quantifier: push a bound value. -/
def push (env : SpecEnv) (sv : SVal) : SpecEnv :=
  { env with bound := sv :: env.bound }

/-- The memory a labeled access refers to: current for `none`, the snapshot
for `some l`. -/
def memAt (env : SpecEnv) : Option MemLabel → Memory
  | none => env.mem
  | some l => env.snaps l

/-- Pushing a bound value preserves struct declarations. -/
@[simp] theorem push_structs (env : SpecEnv) (sv : SVal) :
    (env.push sv).structs = env.structs := rfl
/-- Pushing a bound value preserves local values. -/
@[simp] theorem push_locals (env : SpecEnv) (sv : SVal) :
    (env.push sv).locals = env.locals := rfl
/-- Pushing a bound value preserves result values. -/
@[simp] theorem push_result (env : SpecEnv) (sv : SVal) :
    (env.push sv).result = env.result := rfl
/-- Pushing a bound value preserves current memory. -/
@[simp] theorem push_mem (env : SpecEnv) (sv : SVal) :
    (env.push sv).mem = env.mem := rfl
/-- Pushing a bound value preserves memory snapshots. -/
@[simp] theorem push_snaps (env : SpecEnv) (sv : SVal) :
    (env.push sv).snaps = env.snaps := rfl
/-- Pushing adds a value at the head of the bound-variable environment. -/
@[simp] theorem push_bound (env : SpecEnv) (sv : SVal) :
    (env.push sv).bound = sv :: env.bound := rfl
/-- An unlabeled memory reference selects current memory. -/
@[simp] theorem memAt_none (env : SpecEnv) : env.memAt none = env.mem := rfl
/-- A labeled memory reference selects the corresponding snapshot. -/
@[simp] theorem memAt_some (env : SpecEnv) (l : MemLabel) :
    env.memAt (some l) = env.snaps l := rfl

end SpecEnv

/-- Evaluation of the strict binary operators.  Arithmetic is over unbounded
integers; `div`/`mod` are undefined for divisor `0`.  The short-circuit
connectives `and`/`or`/`implies` and the polymorphic `eq` are `none` here —
they are given dedicated rules in `EvalSpec` (`eq` is *semantic* equality on
spec values, which requires no decision procedure). -/
def SpecBinop.strictEval : SpecBinop → SVal → SVal → Option SVal
  | .add, .int i, .int j => some (.int (i + j))
  | .sub, .int i, .int j => some (.int (i - j))
  | .mul, .int i, .int j => some (.int (i * j))
  | .div, .int i, .int j => if j = 0 then none else some (.int (i / j))
  | .mod, .int i, .int j => if j = 0 then none else some (.int (i % j))
  | .lt, .int i, .int j => some (.bool (decide (i < j)))
  | .le, .int i, .int j => some (.bool (decide (i ≤ j)))
  | .index, .vector es, .int i => if 0 ≤ i then es[i.toNat]? else none
  | .iff, .bool a, .bool b => some (.bool (a == b))
  | _, _, _ => none

/-- Relational big-step evaluation of spec expressions (see module docs). -/
inductive EvalSpec : SpecEnv → SpecExp → SVal → Prop where
  | value {env : SpecEnv} {v : Value} :
      EvalSpec env (.value v) v.toSVal
  | loc {env : SpecEnv} {t : LocalIndex} {v : Value} :
      env.locals t = some v → EvalSpec env (.loc t) v.toSVal
  | bvar {env : SpecEnv} {k : Nat} {sv : SVal} :
      env.bound[k]? = some sv → EvalSpec env (.bvar k) sv
  | result {env : SpecEnv} {i : Nat} {v : Value} :
      env.result[i]? = some v → EvalSpec env (.result i) v.toSVal
  | strict {env : SpecEnv} {op : SpecBinop} {e₁ e₂ : SpecExp} {v₁ v₂ v : SVal} :
      EvalSpec env e₁ v₁ → EvalSpec env e₂ v₂ →
      op.strictEval v₁ v₂ = some v → EvalSpec env (.binop op e₁ e₂) v
  | andFalse {env : SpecEnv} {e₁ e₂ : SpecExp} :
      EvalSpec env e₁ (.bool false) →
      EvalSpec env (.binop .and e₁ e₂) (.bool false)
  | andTrue {env : SpecEnv} {e₁ e₂ : SpecExp} {b : Bool} :
      EvalSpec env e₁ (.bool true) → EvalSpec env e₂ (.bool b) →
      EvalSpec env (.binop .and e₁ e₂) (.bool b)
  | orTrue {env : SpecEnv} {e₁ e₂ : SpecExp} :
      EvalSpec env e₁ (.bool true) →
      EvalSpec env (.binop .or e₁ e₂) (.bool true)
  | orFalse {env : SpecEnv} {e₁ e₂ : SpecExp} {b : Bool} :
      EvalSpec env e₁ (.bool false) → EvalSpec env e₂ (.bool b) →
      EvalSpec env (.binop .or e₁ e₂) (.bool b)
  | impliesFalse {env : SpecEnv} {e₁ e₂ : SpecExp} :
      EvalSpec env e₁ (.bool false) →
      EvalSpec env (.binop .implies e₁ e₂) (.bool true)
  | impliesTrue {env : SpecEnv} {e₁ e₂ : SpecExp} {b : Bool} :
      EvalSpec env e₁ (.bool true) → EvalSpec env e₂ (.bool b) →
      EvalSpec env (.binop .implies e₁ e₂) (.bool b)
  | eqTrue {env : SpecEnv} {e₁ e₂ : SpecExp} {v : SVal} :
      EvalSpec env e₁ v → EvalSpec env e₂ v →
      EvalSpec env (.binop .eq e₁ e₂) (.bool true)
  | eqFalse {env : SpecEnv} {e₁ e₂ : SpecExp} {v₁ v₂ : SVal} :
      EvalSpec env e₁ v₁ → EvalSpec env e₂ v₂ → v₁ ≠ v₂ →
      EvalSpec env (.binop .eq e₁ e₂) (.bool false)
  | not {env : SpecEnv} {e : SpecExp} {b : Bool} :
      EvalSpec env e (.bool b) → EvalSpec env (.not e) (.bool !b)
  | select {env : SpecEnv} {i : Nat} {e : SpecExp} {fs : List SVal} {sv : SVal} :
      EvalSpec env e (.struct fs) → fs[i]? = some sv →
      EvalSpec env (.select i e) sv
  | len {env : SpecEnv} {e : SpecExp} {es : List SVal} :
      EvalSpec env e (.vector es) →
      EvalSpec env (.len e) (.int es.length)
  | mutVal {env : SpecEnv} {e : SpecExp} {t : RefTarget} {sv : SVal} :
      EvalSpec env e (.mut t sv) →
      EvalSpec env (.mutVal e) sv
  | global {env : SpecEnv} {r : ResourceId} {lbl : Option MemLabel}
      {addr : SpecExp} {a : Address} {v : Value} :
      EvalSpec env addr (.address a) → env.memAt lbl r a = some v →
      EvalSpec env (.global r lbl addr) v.toSVal
  | exists_ {env : SpecEnv} {r : ResourceId} {lbl : Option MemLabel}
      {addr : SpecExp} {a : Address} :
      EvalSpec env addr (.address a) →
      EvalSpec env (.exists_ r lbl addr) (.bool (env.memAt lbl r a).isSome)
  | iteTrue {env : SpecEnv} {c t e : SpecExp} {sv : SVal} :
      EvalSpec env c (.bool true) → EvalSpec env t sv →
      EvalSpec env (.ite c t e) sv
  | iteFalse {env : SpecEnv} {c t e : SpecExp} {sv : SVal} :
      EvalSpec env c (.bool false) → EvalSpec env e sv →
      EvalSpec env (.ite c t e) sv
  | allTrue {env : SpecEnv} {dom : Ty} {body : SpecExp} :
      (∀ v : Value, IsValid env.structs dom v →
        EvalSpec (env.push v.toSVal) body (.bool true)) →
      EvalSpec env (.quant .all dom body) (.bool true)
  | allFalse {env : SpecEnv} {dom : Ty} {body : SpecExp} {v : Value} :
      IsValid env.structs dom v →
      EvalSpec (env.push v.toSVal) body (.bool false) →
      EvalSpec env (.quant .all dom body) (.bool false)
  | exTrue {env : SpecEnv} {dom : Ty} {body : SpecExp} {v : Value} :
      IsValid env.structs dom v →
      EvalSpec (env.push v.toSVal) body (.bool true) →
      EvalSpec env (.quant .ex dom body) (.bool true)
  | exFalse {env : SpecEnv} {dom : Ty} {body : SpecExp} :
      (∀ v : Value, IsValid env.structs dom v →
        EvalSpec (env.push v.toSVal) body (.bool false)) →
      EvalSpec env (.quant .ex dom body) (.bool false)

/-- A (boolean) spec expression *holds* in an environment. -/
def Holds (env : SpecEnv) (e : SpecExp) : Prop :=
  EvalSpec env e (.bool true)

/-! ## Determinism -/

theorem strictEval_eq_eq_none (v₁ v₂ : SVal) :
    SpecBinop.strictEval .eq v₁ v₂ = none := by
  cases v₁ <;> cases v₂ <;> rfl

/-- Strict evaluation is undefined for short-circuit conjunction. -/
theorem strictEval_and_eq_none (v₁ v₂ : SVal) :
    SpecBinop.strictEval .and v₁ v₂ = none := by
  cases v₁ <;> cases v₂ <;> rfl

/-- Strict evaluation is undefined for short-circuit disjunction. -/
theorem strictEval_or_eq_none (v₁ v₂ : SVal) :
    SpecBinop.strictEval .or v₁ v₂ = none := by
  cases v₁ <;> cases v₂ <;> rfl

/-- Strict evaluation is undefined for short-circuit implication. -/
theorem strictEval_implies_eq_none (v₁ v₂ : SVal) :
    SpecBinop.strictEval .implies v₁ v₂ = none := by
  cases v₁ <;> cases v₂ <;> rfl

/-- Spec evaluation is deterministic: the short-circuit rules and the
quantifier rules do not overlap. -/
theorem evalSpec_det {env : SpecEnv} {e : SpecExp} {sv : SVal}
    (h : EvalSpec env e sv) :
    ∀ {sv' : SVal}, EvalSpec env e sv' → sv = sv' := by
  induction h with
  | value => intro _ h'; cases h'; rfl
  | loc ht =>
    intro _ h'; cases h' with
    | loc ht' => rw [ht] at ht'; injection ht' with hv; rw [hv]
  | bvar hb =>
    intro _ h'; cases h' with
    | bvar hb' => rw [hb] at hb'; injection hb'
  | result hr =>
    intro _ h'; cases h' with
    | result hr' => rw [hr] at hr'; injection hr' with hv; rw [hv]
  | strict _ _ hop ih₁ ih₂ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' =>
      cases ih₁ h₁'; cases ih₂ h₂'
      rw [hop] at hop'; exact Option.some.inj hop'
    | andFalse h₁' => rw [strictEval_and_eq_none] at hop; cases hop
    | andTrue h₁' h₂' => rw [strictEval_and_eq_none] at hop; cases hop
    | orTrue h₁' => rw [strictEval_or_eq_none] at hop; cases hop
    | orFalse h₁' h₂' => rw [strictEval_or_eq_none] at hop; cases hop
    | impliesFalse h₁' => rw [strictEval_implies_eq_none] at hop; cases hop
    | impliesTrue h₁' h₂' => rw [strictEval_implies_eq_none] at hop; cases hop
    | eqTrue h₁' h₂' => rw [strictEval_eq_eq_none] at hop; cases hop
    | eqFalse h₁' h₂' hne => rw [strictEval_eq_eq_none] at hop; cases hop
  | andFalse h₁ ih₁ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' => rw [strictEval_and_eq_none] at hop'; cases hop'
    | andFalse h₁' => rfl
    | andTrue h₁' h₂' => have := ih₁ h₁'; simp at this
  | andTrue h₁ h₂ ih₁ ih₂ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' => rw [strictEval_and_eq_none] at hop'; cases hop'
    | andFalse h₁' => have := ih₁ h₁'; simp at this
    | andTrue h₁' h₂' => exact ih₂ h₂'
  | orTrue h₁ ih₁ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' => rw [strictEval_or_eq_none] at hop'; cases hop'
    | orTrue h₁' => rfl
    | orFalse h₁' h₂' => have := ih₁ h₁'; simp at this
  | orFalse h₁ h₂ ih₁ ih₂ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' => rw [strictEval_or_eq_none] at hop'; cases hop'
    | orTrue h₁' => have := ih₁ h₁'; simp at this
    | orFalse h₁' h₂' => exact ih₂ h₂'
  | impliesFalse h₁ ih₁ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' => rw [strictEval_implies_eq_none] at hop'; cases hop'
    | impliesFalse h₁' => rfl
    | impliesTrue h₁' h₂' => have := ih₁ h₁'; simp at this
  | impliesTrue h₁ h₂ ih₁ ih₂ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' => rw [strictEval_implies_eq_none] at hop'; cases hop'
    | impliesFalse h₁' => have := ih₁ h₁'; simp at this
    | impliesTrue h₁' h₂' => exact ih₂ h₂'
  | eqTrue h₁ h₂ ih₁ ih₂ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' => rw [strictEval_eq_eq_none] at hop'; cases hop'
    | eqTrue h₁' h₂' => rfl
    | eqFalse h₁' h₂' hne => exact absurd ((ih₁ h₁').symm.trans (ih₂ h₂')) hne
  | eqFalse h₁ h₂ hne ih₁ ih₂ =>
    intro _ h'; cases h' with
    | strict h₁' h₂' hop' => rw [strictEval_eq_eq_none] at hop'; cases hop'
    | eqTrue h₁' h₂' => exact absurd ((ih₁ h₁').trans (ih₂ h₂').symm) hne
    | eqFalse h₁' h₂' hne' => rfl
  | not _ ih =>
    intro _ h'; cases h' with
    | not h₁' => injection ih h₁' with hb; rw [hb]
  | select _ hf ih =>
    intro _ h'; cases h' with
    | select h₁' hf' =>
      injection ih h₁' with hfs
      rw [hfs] at hf; rw [hf] at hf'; exact Option.some.inj hf'
  | len _ ih =>
    intro _ h'; cases h' with
    | len h₁' => injection ih h₁' with hes; rw [hes]
  | mutVal _ ih =>
    intro _ h'; cases h' with
    | mutVal h₁' => injection ih h₁'
  | global _ hm ih =>
    intro _ h'; cases h' with
    | global h₁' hm' =>
      injection ih h₁' with ha
      rw [ha] at hm; rw [hm] at hm'; injection hm' with hv; rw [hv]
  | exists_ _ ih =>
    intro _ h'; cases h' with
    | exists_ h₁' => injection ih h₁' with ha; rw [ha]
  | iteTrue _ _ ihc iht =>
    intro _ h'; cases h' with
    | iteTrue hc' ht' => exact iht ht'
    | iteFalse hc' he' => have := ihc hc'; simp at this
  | iteFalse _ _ ihc ihe =>
    intro _ h'; cases h' with
    | iteTrue hc' ht' => have := ihc hc'; simp at this
    | iteFalse hc' he' => exact ihe he'
  | allTrue _ ih =>
    intro _ h'; cases h' with
    | allTrue _ => rfl
    | allFalse hval hf => have := ih _ hval hf; simp at this
  | allFalse hval hf ih =>
    intro _ h'; cases h' with
    | allTrue ht => have := ih (ht _ hval); simp at this
    | allFalse _ _ => rfl
  | exTrue hval ht ih =>
    intro _ h'; cases h' with
    | exTrue _ _ => rfl
    | exFalse hf => have := ih (hf _ hval); simp at this
  | exFalse _ ih =>
    intro _ h'; cases h' with
    | exTrue hval ht => have := ih _ hval ht; simp at this
    | exFalse _ => rfl

/-! ## Inversion simp pack

One `iff` characterization per constructor (and per short-circuit operator),
turning `EvalSpec`/`Holds` goals on concrete spec expressions into plain
logical goals.
-/

@[simp] theorem evalSpec_value_iff {env : SpecEnv} {v : Value} {sv : SVal} :
    EvalSpec env (.value v) sv ↔ sv = v.toSVal := by
  constructor
  · intro h; cases h; rfl
  · rintro rfl; exact .value

/-- Characterize evaluation of a local specification expression. -/
@[simp] theorem evalSpec_loc_iff {env : SpecEnv} {t : LocalIndex} {sv : SVal} :
    EvalSpec env (.loc t) sv ↔ ∃ v, env.locals t = some v ∧ sv = v.toSVal := by
  constructor
  · intro h; cases h with
    | loc ht => exact ⟨_, ht, rfl⟩
  · rintro ⟨v, ht, rfl⟩; exact .loc ht

/-- Characterize evaluation of a bound-variable expression. -/
@[simp] theorem evalSpec_bvar_iff {env : SpecEnv} {k : Nat} {sv : SVal} :
    EvalSpec env (.bvar k) sv ↔ env.bound[k]? = some sv := by
  constructor
  · intro h; cases h with
    | bvar hb => exact hb
  · exact .bvar

/-- Characterize evaluation of a function-result expression. -/
@[simp] theorem evalSpec_result_iff {env : SpecEnv} {i : Nat} {sv : SVal} :
    EvalSpec env (.result i) sv ↔
      ∃ v, env.result[i]? = some v ∧ sv = v.toSVal := by
  constructor
  · intro h; cases h with
    | result hr => exact ⟨_, hr, rfl⟩
  · rintro ⟨v, hr, rfl⟩; exact .result hr

/-- Characterize evaluation of Boolean negation. -/
@[simp] theorem evalSpec_not_iff {env : SpecEnv} {e : SpecExp} {sv : SVal} :
    EvalSpec env (.not e) sv ↔
      ∃ b, EvalSpec env e (.bool b) ∧ sv = .bool !b := by
  constructor
  · intro h; cases h with
    | not he => exact ⟨_, he, rfl⟩
  · rintro ⟨b, he, rfl⟩; exact .not he

/-- Characterize evaluation of struct-field selection. -/
@[simp] theorem evalSpec_select_iff {env : SpecEnv} {i : Nat} {e : SpecExp}
    {sv : SVal} :
    EvalSpec env (.select i e) sv ↔
      ∃ fs, EvalSpec env e (.struct fs) ∧ fs[i]? = some sv := by
  constructor
  · intro h; cases h with
    | select he hf => exact ⟨_, he, hf⟩
  · rintro ⟨fs, he, hf⟩; exact .select he hf

/-- Characterize evaluation of vector length. -/
@[simp] theorem evalSpec_len_iff {env : SpecEnv} {e : SpecExp} {sv : SVal} :
    EvalSpec env (.len e) sv ↔
      ∃ es, EvalSpec env e (.vector es) ∧ sv = .int es.length := by
  constructor
  · intro h; cases h with
    | len he => exact ⟨_, he, rfl⟩
  · rintro ⟨es, he, rfl⟩; exact .len he

/-- Characterize evaluation of a mutation payload. -/
@[simp] theorem evalSpec_mutVal_iff {env : SpecEnv} {e : SpecExp}
    {sv : SVal} :
    EvalSpec env (.mutVal e) sv ↔ ∃ t, EvalSpec env e (.mut t sv) := by
  constructor
  · intro h; cases h with
    | mutVal he => exact ⟨_, he⟩
  · rintro ⟨t, he⟩; exact .mutVal he

/-- Characterize evaluation of a global resource lookup. -/
@[simp] theorem evalSpec_global_iff {env : SpecEnv} {r : ResourceId}
    {lbl : Option MemLabel} {addr : SpecExp} {sv : SVal} :
    EvalSpec env (.global r lbl addr) sv ↔
      ∃ a v, EvalSpec env addr (.address a) ∧ env.memAt lbl r a = some v ∧
        sv = v.toSVal := by
  constructor
  · intro h; cases h with
    | global ha hm => exact ⟨_, _, ha, hm, rfl⟩
  · rintro ⟨a, v, ha, hm, rfl⟩; exact .global ha hm

/-- Characterize evaluation of a global-resource existence test. -/
@[simp] theorem evalSpec_exists_iff {env : SpecEnv} {r : ResourceId}
    {lbl : Option MemLabel} {addr : SpecExp} {sv : SVal} :
    EvalSpec env (.exists_ r lbl addr) sv ↔
      ∃ a, EvalSpec env addr (.address a) ∧
        sv = .bool (env.memAt lbl r a).isSome := by
  constructor
  · intro h; cases h with
    | exists_ ha => exact ⟨_, ha, rfl⟩
  · rintro ⟨a, ha, rfl⟩; exact .exists_ ha

/-- Characterize evaluation of a conditional specification expression. -/
@[simp] theorem evalSpec_ite_iff {env : SpecEnv} {c t e : SpecExp} {sv : SVal} :
    EvalSpec env (.ite c t e) sv ↔
      (EvalSpec env c (.bool true) ∧ EvalSpec env t sv) ∨
      (EvalSpec env c (.bool false) ∧ EvalSpec env e sv) := by
  constructor
  · intro h; cases h with
    | iteTrue hc ht => exact .inl ⟨hc, ht⟩
    | iteFalse hc he => exact .inr ⟨hc, he⟩
  · rintro (⟨hc, ht⟩ | ⟨hc, he⟩)
    · exact .iteTrue hc ht
    · exact .iteFalse hc he

/-- Characterize evaluation of universal quantification. -/
@[simp] theorem evalSpec_all_iff {env : SpecEnv} {dom : Ty} {body : SpecExp}
    {sv : SVal} :
    EvalSpec env (.quant .all dom body) sv ↔
      (sv = .bool true ∧ ∀ v, IsValid env.structs dom v →
        EvalSpec (env.push v.toSVal) body (.bool true)) ∨
      (sv = .bool false ∧ ∃ v, IsValid env.structs dom v ∧
        EvalSpec (env.push v.toSVal) body (.bool false)) := by
  constructor
  · intro h; cases h with
    | allTrue ht => exact .inl ⟨rfl, ht⟩
    | allFalse hval hf => exact .inr ⟨rfl, _, hval, hf⟩
  · rintro (⟨rfl, ht⟩ | ⟨rfl, v, hval, hf⟩)
    · exact .allTrue ht
    · exact .allFalse hval hf

/-- Characterize evaluation of existential quantification. -/
@[simp] theorem evalSpec_ex_iff {env : SpecEnv} {dom : Ty} {body : SpecExp}
    {sv : SVal} :
    EvalSpec env (.quant .ex dom body) sv ↔
      (sv = .bool true ∧ ∃ v, IsValid env.structs dom v ∧
        EvalSpec (env.push v.toSVal) body (.bool true)) ∨
      (sv = .bool false ∧ ∀ v, IsValid env.structs dom v →
        EvalSpec (env.push v.toSVal) body (.bool false)) := by
  constructor
  · intro h; cases h with
    | exTrue hval ht => exact .inl ⟨rfl, _, hval, ht⟩
    | exFalse hf => exact .inr ⟨rfl, hf⟩
  · rintro (⟨rfl, v, hval, ht⟩ | ⟨rfl, hf⟩)
    · exact .exTrue hval ht
    · exact .exFalse hf

/-- Master inversion for binary operators (strict rule vs. the short-circuit
rules).  The per-operator simp lemmas below are derived from it. -/
theorem evalSpec_binop_inv {env : SpecEnv} {op : SpecBinop} {e₁ e₂ : SpecExp}
    {sv : SVal} (h : EvalSpec env (.binop op e₁ e₂) sv) :
    (∃ v₁ v₂, EvalSpec env e₁ v₁ ∧ EvalSpec env e₂ v₂ ∧
      op.strictEval v₁ v₂ = some sv) ∨
    (op = .and ∧
      ((EvalSpec env e₁ (.bool false) ∧ sv = .bool false) ∨
       (∃ b, EvalSpec env e₁ (.bool true) ∧ EvalSpec env e₂ (.bool b) ∧
         sv = .bool b))) ∨
    (op = .or ∧
      ((EvalSpec env e₁ (.bool true) ∧ sv = .bool true) ∨
       (∃ b, EvalSpec env e₁ (.bool false) ∧ EvalSpec env e₂ (.bool b) ∧
         sv = .bool b))) ∨
    (op = .implies ∧
      ((EvalSpec env e₁ (.bool false) ∧ sv = .bool true) ∨
       (∃ b, EvalSpec env e₁ (.bool true) ∧ EvalSpec env e₂ (.bool b) ∧
         sv = .bool b))) ∨
    (op = .eq ∧
      ((∃ v, EvalSpec env e₁ v ∧ EvalSpec env e₂ v ∧ sv = .bool true) ∨
       (∃ v₁ v₂, EvalSpec env e₁ v₁ ∧ EvalSpec env e₂ v₂ ∧ v₁ ≠ v₂ ∧
         sv = .bool false))) := by
  cases h with
  | strict h₁ h₂ hop => exact .inl ⟨_, _, h₁, h₂, hop⟩
  | andFalse h₁ => exact .inr (.inl ⟨rfl, .inl ⟨h₁, rfl⟩⟩)
  | andTrue h₁ h₂ => exact .inr (.inl ⟨rfl, .inr ⟨_, h₁, h₂, rfl⟩⟩)
  | orTrue h₁ => exact .inr (.inr (.inl ⟨rfl, .inl ⟨h₁, rfl⟩⟩))
  | orFalse h₁ h₂ => exact .inr (.inr (.inl ⟨rfl, .inr ⟨_, h₁, h₂, rfl⟩⟩))
  | impliesFalse h₁ => exact .inr (.inr (.inr (.inl ⟨rfl, .inl ⟨h₁, rfl⟩⟩)))
  | impliesTrue h₁ h₂ =>
    exact .inr (.inr (.inr (.inl ⟨rfl, .inr ⟨_, h₁, h₂, rfl⟩⟩)))
  | eqTrue h₁ h₂ =>
    exact .inr (.inr (.inr (.inr ⟨rfl, .inl ⟨_, h₁, h₂, rfl⟩⟩)))
  | eqFalse h₁ h₂ hne =>
    exact .inr (.inr (.inr (.inr ⟨rfl, .inr ⟨_, _, h₁, h₂, hne, rfl⟩⟩)))

/-- Characterize short-circuit conjunction evaluation. -/
@[simp] theorem evalSpec_and_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .and e₁ e₂) sv ↔
      (EvalSpec env e₁ (.bool false) ∧ sv = .bool false) ∨
      (∃ b, EvalSpec env e₁ (.bool true) ∧ EvalSpec env e₂ (.bool b) ∧
        sv = .bool b) := by
  constructor
  · intro h
    rcases evalSpec_binop_inv h with
      ⟨_, _, _, _, hop⟩ | ⟨_, hd⟩ | ⟨hop, _⟩ | ⟨hop, _⟩ | ⟨hop, _⟩
    · rw [strictEval_and_eq_none] at hop; cases hop
    · exact hd
    · cases hop
    · cases hop
    · cases hop
  · rintro (⟨h₁, rfl⟩ | ⟨b, h₁, h₂, rfl⟩)
    · exact .andFalse h₁
    · exact .andTrue h₁ h₂

/-- Characterize short-circuit disjunction evaluation. -/
@[simp] theorem evalSpec_or_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .or e₁ e₂) sv ↔
      (EvalSpec env e₁ (.bool true) ∧ sv = .bool true) ∨
      (∃ b, EvalSpec env e₁ (.bool false) ∧ EvalSpec env e₂ (.bool b) ∧
        sv = .bool b) := by
  constructor
  · intro h
    rcases evalSpec_binop_inv h with
      ⟨_, _, _, _, hop⟩ | ⟨hop, _⟩ | ⟨_, hd⟩ | ⟨hop, _⟩ | ⟨hop, _⟩
    · rw [strictEval_or_eq_none] at hop; cases hop
    · cases hop
    · exact hd
    · cases hop
    · cases hop
  · rintro (⟨h₁, rfl⟩ | ⟨b, h₁, h₂, rfl⟩)
    · exact .orTrue h₁
    · exact .orFalse h₁ h₂

/-- Characterize short-circuit implication evaluation. -/
@[simp] theorem evalSpec_implies_iff {env : SpecEnv} {e₁ e₂ : SpecExp}
    {sv : SVal} :
    EvalSpec env (.binop .implies e₁ e₂) sv ↔
      (EvalSpec env e₁ (.bool false) ∧ sv = .bool true) ∨
      (∃ b, EvalSpec env e₁ (.bool true) ∧ EvalSpec env e₂ (.bool b) ∧
        sv = .bool b) := by
  constructor
  · intro h
    rcases evalSpec_binop_inv h with
      ⟨_, _, _, _, hop⟩ | ⟨hop, _⟩ | ⟨hop, _⟩ | ⟨_, hd⟩ | ⟨hop, _⟩
    · rw [strictEval_implies_eq_none] at hop; cases hop
    · cases hop
    · cases hop
    · exact hd
    · cases hop
  · rintro (⟨h₁, rfl⟩ | ⟨b, h₁, h₂, rfl⟩)
    · exact .impliesFalse h₁
    · exact .impliesTrue h₁ h₂

/-- Inversion for a strict operator: reduces to `strictEval`.  The
per-operator simp lemmas below instantiate it. -/
theorem evalSpec_strict_iff {env : SpecEnv} {op : SpecBinop} {e₁ e₂ : SpecExp}
    {sv : SVal} (hop : op ≠ .and) (hop' : op ≠ .or) (hop'' : op ≠ .implies)
    (hop''' : op ≠ .eq) :
    EvalSpec env (.binop op e₁ e₂) sv ↔
      ∃ v₁ v₂, EvalSpec env e₁ v₁ ∧ EvalSpec env e₂ v₂ ∧
        op.strictEval v₁ v₂ = some sv := by
  constructor
  · intro h
    rcases evalSpec_binop_inv h with hd | ⟨he, _⟩ | ⟨he, _⟩ | ⟨he, _⟩ | ⟨he, _⟩
    · exact hd
    · exact absurd he hop
    · exact absurd he hop'
    · exact absurd he hop''
    · exact absurd he hop'''
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩; exact .strict h₁ h₂ hop

/-- Characterize specification addition. -/
@[simp] theorem evalSpec_add_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .add e₁ e₂) sv ↔
      ∃ i j, EvalSpec env e₁ (.int i) ∧ EvalSpec env e₂ (.int j) ∧
        sv = .int (i + j) := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    exact ⟨_, _, h₁, h₂, hop.symm⟩
  · rintro ⟨i, j, h₁, h₂, rfl⟩; exact ⟨_, _, h₁, h₂, rfl⟩

/-- Characterize specification subtraction. -/
@[simp] theorem evalSpec_sub_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .sub e₁ e₂) sv ↔
      ∃ i j, EvalSpec env e₁ (.int i) ∧ EvalSpec env e₂ (.int j) ∧
        sv = .int (i - j) := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    exact ⟨_, _, h₁, h₂, hop.symm⟩
  · rintro ⟨i, j, h₁, h₂, rfl⟩; exact ⟨_, _, h₁, h₂, rfl⟩

/-- Characterize specification multiplication. -/
@[simp] theorem evalSpec_mul_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .mul e₁ e₂) sv ↔
      ∃ i j, EvalSpec env e₁ (.int i) ∧ EvalSpec env e₂ (.int j) ∧
        sv = .int (i * j) := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    exact ⟨_, _, h₁, h₂, hop.symm⟩
  · rintro ⟨i, j, h₁, h₂, rfl⟩; exact ⟨_, _, h₁, h₂, rfl⟩

/-- Characterize specification division. -/
@[simp] theorem evalSpec_div_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .div e₁ e₂) sv ↔
      ∃ i j, EvalSpec env e₁ (.int i) ∧ EvalSpec env e₂ (.int j) ∧
        j ≠ 0 ∧ sv = .int (i / j) := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    rcases hop with ⟨hj, hv⟩
    exact ⟨_, _, h₁, h₂, hj, hv.symm⟩
  · rintro ⟨i, j, h₁, h₂, hj, rfl⟩
    exact ⟨_, _, h₁, h₂, by simp [SpecBinop.strictEval, hj]⟩

/-- Characterize specification remainder. -/
@[simp] theorem evalSpec_mod_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .mod e₁ e₂) sv ↔
      ∃ i j, EvalSpec env e₁ (.int i) ∧ EvalSpec env e₂ (.int j) ∧
        j ≠ 0 ∧ sv = .int (i % j) := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    rcases hop with ⟨hj, hv⟩
    exact ⟨_, _, h₁, h₂, hj, hv.symm⟩
  · rintro ⟨i, j, h₁, h₂, hj, rfl⟩
    exact ⟨_, _, h₁, h₂, by simp [SpecBinop.strictEval, hj]⟩

/-- Characterize specification less-than comparison. -/
@[simp] theorem evalSpec_lt_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .lt e₁ e₂) sv ↔
      ∃ i j, EvalSpec env e₁ (.int i) ∧ EvalSpec env e₂ (.int j) ∧
        sv = .bool (decide (i < j)) := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    exact ⟨_, _, h₁, h₂, hop.symm⟩
  · rintro ⟨i, j, h₁, h₂, rfl⟩; exact ⟨_, _, h₁, h₂, rfl⟩

/-- Characterize specification less-than-or-equal comparison. -/
@[simp] theorem evalSpec_le_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .le e₁ e₂) sv ↔
      ∃ i j, EvalSpec env e₁ (.int i) ∧ EvalSpec env e₂ (.int j) ∧
        sv = .bool (decide (i ≤ j)) := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    exact ⟨_, _, h₁, h₂, hop.symm⟩
  · rintro ⟨i, j, h₁, h₂, rfl⟩; exact ⟨_, _, h₁, h₂, rfl⟩

/-- Characterize specification vector indexing. -/
@[simp] theorem evalSpec_index_iff {env : SpecEnv} {e₁ e₂ : SpecExp}
    {sv : SVal} :
    EvalSpec env (.binop .index e₁ e₂) sv ↔
      ∃ es i, EvalSpec env e₁ (.vector es) ∧ EvalSpec env e₂ (.int i) ∧
        0 ≤ i ∧ es[i.toNat]? = some sv := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    rcases hop with ⟨hi, hv⟩
    exact ⟨_, _, h₁, h₂, hi, hv⟩
  · rintro ⟨es, i, h₁, h₂, hi, hv⟩
    exact ⟨_, _, h₁, h₂, by simp [SpecBinop.strictEval, hi, hv]⟩

/-- Characterize specification equality. -/
@[simp] theorem evalSpec_eq_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .eq e₁ e₂) sv ↔
      (∃ v, EvalSpec env e₁ v ∧ EvalSpec env e₂ v ∧ sv = .bool true) ∨
      (∃ v₁ v₂, EvalSpec env e₁ v₁ ∧ EvalSpec env e₂ v₂ ∧ v₁ ≠ v₂ ∧
        sv = .bool false) := by
  constructor
  · intro h
    rcases evalSpec_binop_inv h with
      ⟨_, _, _, _, hop⟩ | ⟨hop, _⟩ | ⟨hop, _⟩ | ⟨hop, _⟩ | ⟨_, hd⟩
    · rw [strictEval_eq_eq_none] at hop; cases hop
    · cases hop
    · cases hop
    · cases hop
    · exact hd
  · rintro (⟨v, h₁, h₂, rfl⟩ | ⟨v₁, v₂, h₁, h₂, hne, rfl⟩)
    · exact .eqTrue h₁ h₂
    · exact .eqFalse h₁ h₂ hne

/-- Characterize specification Boolean equivalence. -/
@[simp] theorem evalSpec_iff_iff {env : SpecEnv} {e₁ e₂ : SpecExp} {sv : SVal} :
    EvalSpec env (.binop .iff e₁ e₂) sv ↔
      ∃ a b, EvalSpec env e₁ (.bool a) ∧ EvalSpec env e₂ (.bool b) ∧
        sv = .bool (a == b) := by
  rw [evalSpec_strict_iff (by intro h; cases h) (by intro h; cases h)
    (by intro h; cases h) (by intro h; cases h)]
  constructor
  · rintro ⟨v₁, v₂, h₁, h₂, hop⟩
    cases v₁ <;> cases v₂ <;> simp [SpecBinop.strictEval] at hop
    exact ⟨_, _, h₁, h₂, hop.symm⟩
  · rintro ⟨a, b, h₁, h₂, rfl⟩; exact ⟨_, _, h₁, h₂, rfl⟩

end MoveModel.IR
