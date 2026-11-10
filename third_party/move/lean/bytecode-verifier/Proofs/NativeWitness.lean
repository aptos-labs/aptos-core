-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Certified

/-!
# Non-vacuity of the native contracts (N3)

UNTRUSTED (checked by the kernel). The native contracts are satisfiable, by
the witness the statement describes (`NativeContractsSatisfiable`), a total
function: every native fails, except one whose result types are plain data
and whose arguments, as it sees them, conform to what it sees of its
parameter types (`RuntimeType.seen`), when those are all plain data, or when
it has the read shape and its value parameter types are plain data. It
returns a new plain value of each result type (`false`, `0`, the zero
address, the empty vector) and writes and borrows nothing; its flow creates
every result node. Such a native respects ownership whatever the policy:

- what it creates is plain data, which anyone may create;
- it passes nothing on, so of what a reference argument designates, which it
  keeps (the read shape writes nothing back), it moves and copies nothing;
- it consumes every other argument, and may: such an argument is no
  reference, and its type is plain data or, if not a value type, references
  nested in vectors, so it holds no struct, enum or closure value, only
  values whose types have `drop`.
-/

namespace MoveBytecode.Proofs

variable {P : Program}

/-! ## Plain data -/

/-- No struct, enum or closure value can be at a position of a value of this
type, through vectors but not through references: its nodes are primitive
values, signers, vectors and references. -/
def unowned : RuntimeType → Bool
  | .bool => true
  | .int _ => true
  | .address => true
  | .signer => true
  | .vector elem => unowned elem
  | .struct _ => false
  | .function .. => false
  | .ref _ _ => true

theorem IsPlain_unowned : ∀ {t : RuntimeType}, t.IsPlain = true → unowned t = true
  | .bool, _ => rfl
  | .int _, _ => rfl
  | .address, _ => rfl
  | .vector elem, h => IsPlain_unowned (t := elem) h
  | .signer, h => by simp [RuntimeType.IsPlain] at h
  | .struct _, h => by simp [RuntimeType.IsPlain] at h
  | .function .., h => by simp [RuntimeType.IsPlain] at h
  | .ref _ _, h => by simp [RuntimeType.IsPlain] at h

theorem IsPlain_isValue : ∀ {t : RuntimeType}, t.IsPlain = true → t.isValue = true
  | .bool, _ => rfl
  | .int _, _ => rfl
  | .address, _ => rfl
  | .vector elem, h => IsPlain_isValue (t := elem) h
  | .signer, h => by simp [RuntimeType.IsPlain] at h
  | .struct _, h => by simp [RuntimeType.IsPlain] at h
  | .function .., h => by simp [RuntimeType.IsPlain] at h
  | .ref _ _, h => by simp [RuntimeType.IsPlain] at h

/-- A type that is not a value type is a reference, nested in vectors. -/
theorem unowned_of_not_isValue : ∀ {t : RuntimeType}, t.isValue = false → unowned t = true
  | .vector elem, h => unowned_of_not_isValue (t := elem) h
  | .ref _ _, _ => rfl
  | .bool, h => by simp [RuntimeType.isValue] at h
  | .int _, h => by simp [RuntimeType.isValue] at h
  | .address, h => by simp [RuntimeType.isValue] at h
  | .signer, h => by simp [RuntimeType.isValue] at h
  | .struct _, h => by simp [RuntimeType.isValue] at h
  | .function .., h => by simp [RuntimeType.isValue] at h

theorem IsPlain_seen {t : RuntimeType} (h : t.IsPlain = true) : t.seen = t := by
  cases t <;> simp_all [RuntimeType.IsPlain, RuntimeType.seen]

/-- An unowned type has `drop`. -/
theorem unowned_drop : ∀ {t : RuntimeType}, unowned t = true → (t.abilities P).drop = true
  | .bool, _ => rfl
  | .int _, _ => rfl
  | .address, _ => rfl
  | .signer, _ => rfl
  | .vector elem, h => by
    have := unowned_drop (t := elem) h
    simp [RuntimeType.abilities, AbilitySet.inter, AbilitySet.forContainer, AbilitySet.primitives,
      this]
  | .ref _ _, _ => rfl
  | .struct _, h => by simp [unowned] at h
  | .function .., h => by simp [unowned] at h

mutual
/-- Every node of a value conforming to an unowned type has an unowned type. -/
theorem nodes_unowned : ∀ (v : Value) {t : RuntimeType}, unowned t = true →
    v.conforms P t = true → ∀ x ∈ v.nodes, unowned x.2.type = true
  | .vector e vs, t, ht, hv, x, hx => by
    rcases assignable_cases (conforms_assignable hv) with ⟨_, _, _, _, h, -⟩ | ⟨_, _, h, -⟩ | h
    · simp [Value.type] at h
    · simp [Value.type] at h
    · simp only [Value.type] at h
      subst h
      simp only [Value.nodes, List.mem_cons] at hx
      rcases hx with rfl | hx
      · exact ht
      · exact nodesFrom_unowned 0 vs (e := e) ht
          (by simpa [Value.wellFormed] using conforms_wellFormed hv) x hx
  | .struct s fs, t, ht, hv, _, _ => by
    rcases assignable_cases (conforms_assignable hv) with ⟨_, _, _, _, h, -⟩ | ⟨_, _, h, -⟩ | h
    · simp [Value.type] at h
    · simp [Value.type] at h
    · simp only [Value.type] at h
      subst h
      simp [unowned] at ht
  | .variant s _ fs, t, ht, hv, _, _ => by
    rcases assignable_cases (conforms_assignable hv) with ⟨_, _, _, _, h, -⟩ | ⟨_, _, h, -⟩ | h
    · simp [Value.type] at h
    · simp [Value.type] at h
    · simp only [Value.type] at h
      subst h
      simp [unowned] at ht
  | .table s fs ks es, t, ht, hv, _, _ => by
    rcases assignable_cases (conforms_assignable hv) with ⟨_, _, _, _, h, -⟩ | ⟨_, _, h, -⟩ | h
    · simp [Value.type] at h
    · simp [Value.type] at h
    · simp only [Value.type] at h
      subst h
      simp [unowned] at ht
  | .closure .., t, ht, hv, _, _ => by
    rcases assignable_cases (conforms_assignable hv) with ⟨_, _, _, _, -, h, -⟩ | ⟨_, _, h, -⟩ | h
    · subst h
      simp [unowned] at ht
    · simp [Value.type] at h
    · simp only [Value.type] at h
      subst h
      simp [unowned] at ht
  | .bool _, _, _, _, x, hx => by
    simp only [Value.nodes, List.mem_singleton] at hx
    subst hx; rfl
  | .int _ _, _, _, _, x, hx => by
    simp only [Value.nodes, List.mem_singleton] at hx
    subst hx; rfl
  | .address _, _, _, _, x, hx => by
    simp only [Value.nodes, List.mem_singleton] at hx
    subst hx; rfl
  | .signer _, _, _, _, x, hx => by
    simp only [Value.nodes, List.mem_cons] at hx
    rcases hx with rfl | rfl | h
    · rfl
    · rfl
    · simp at h
  | .ref _, _, _, _, x, hx => by
    simp only [Value.nodes, List.mem_singleton] at hx
    subst hx; rfl

theorem nodesFrom_unowned : ∀ (i : Nat) (vs : List Value) {e : RuntimeType}, unowned e = true →
    Value.allConform P vs e = true → ∀ x ∈ Value.nodesFrom i vs, unowned x.2.type = true
  | _, [], _, _, _, x, hx => by simp [Value.nodesFrom] at hx
  | i, v :: vs, e, he, hall, x, hx => by
    simp only [Value.allConform, Bool.and_eq_true] at hall
    simp only [Value.nodesFrom, List.mem_append, List.mem_map] at hx
    rcases hx with ⟨y, hy, rfl⟩ | hx
    · exact nodes_unowned v he (by simpa [Value.conforms] using hall.1) y hy
    · exact nodesFrom_unowned (i + 1) vs he hall.2 x hx
end

/-- Any consumer may consume a value of an unowned type: the type has `drop`,
and it is no struct type. -/
theorem permits_unowned (π : NativePolicy) (c : Consumer) {w : Value}
    (h : unowned w.type = true) : c.Permits P π w = true := by
  simp only [Consumer.Permits, unowned_drop h, Bool.true_or, Bool.true_and]
  generalize w.type = t at h
  cases t <;> simp_all [unowned]

/-- A new plain value of type `t`: `false`, `0`, the zero address, the empty
vector. Not used at other types. -/
def plainDefault : RuntimeType → Value
  | .bool => .bool false
  | .int t => .int t 0
  | .address => .address 0
  | .vector elem => .vector elem []
  | .signer => .bool false
  | .struct _ => .bool false
  | .function .. => .bool false
  | .ref _ _ => .bool false

theorem plainDefault_conforms {t : RuntimeType} (h : t.IsPlain = true) :
    (plainDefault t).conforms P t = true := by
  cases t with
  | bool => rfl
  | int it =>
    simp only [plainDefault, conforms_int_iff, true_and]
    cases it <;> decide
  | address => rfl
  | vector elem =>
    simp [plainDefault, Value.conforms, Value.type, Value.wellFormed, Value.allConform]
  | signer => simp [RuntimeType.IsPlain] at h
  | struct => simp [RuntimeType.IsPlain] at h
  | function => simp [RuntimeType.IsPlain] at h
  | ref => simp [RuntimeType.IsPlain] at h

theorem conformsAll_plainDefault :
    ∀ {tys : List RuntimeType}, (∀ t ∈ tys, t.IsPlain = true) →
      conformsAll P (tys.map plainDefault) tys = true
  | [], _ => rfl
  | t :: ts, h => by
    simp only [List.map_cons, conformsAll_cons]
    exact ⟨plainDefault_conforms (h t (List.mem_cons_self ..)),
      conformsAll_plainDefault fun u hu => h u (List.mem_cons_of_mem _ hu)⟩

/-- A new plain value is its only node, and anyone may create it. -/
theorem plainDefault_nodes {t : RuntimeType} {x : Path × Value} (h : x ∈ (plainDefault t).nodes) :
    x.2 = plainDefault t := by
  cases t <;> simp_all [plainDefault, Value.nodes, Value.nodesFrom]

theorem plainDefault_permits (π : NativePolicy) (A : List StructId) (auth : List Address)
    (c : Creator) (t : RuntimeType) : c.Permits π A auth (plainDefault t) = true := by
  cases t <;> rfl

theorem mem_nodesOf {vs : List Value} {k : Nat} {q : Path} {w : Value}
    (h : (k, q, w) ∈ nodesOf vs) : ∃ v, vs[k]? = some v ∧ (q, w) ∈ v.nodes := by
  simp only [nodesOf, List.mem_flatMap, List.mem_map] at h
  obtain ⟨⟨v, j⟩, hv, ⟨q', w'⟩, hq, he⟩ := h
  simp only [Prod.mk.injEq] at he
  obtain ⟨rfl, rfl, rfl⟩ := he
  exact ⟨v, List.mem_zipIdx_iff_getElem?.1 hv, hq⟩

/-! ## The witness -/

/-- When the witness returns: its results are plain data, and it sees its
arguments conforming to what it sees of its parameters, which are all plain
data, or, for a read-shape native, plain data where they are value types. -/
def returnsPlain (P : Program) (f : FunctionRef) (params returns : List RuntimeType)
    (views : List Value) : Bool :=
  returns.all (·.IsPlain) && conformsAll P views (params.map RuntimeType.seen) &&
    (params.all (·.IsPlain) ||
      (P.nativeShape? f == some .read && params.all fun t => !t.isValue || t.IsPlain))

/-- The results of the witness: a new plain value of each result type, when
it returns. -/
def plainResults (P : Program) (f : FunctionRef) (views : List Value) : Option (List Value) := do
  let params ← P.paramTypes f
  let returns ← P.returnTypes f
  if returnsPlain P f params returns views then some (returns.map plainDefault) else none

/-- The witness natives: they return new plain data where `returnsPlain`
holds, and fail otherwise. -/
def plainNatives (P : Program) : NativeImpl := fun f views =>
  match plainResults P f views with
  | some vals => .returned vals [] []
  | none => .failed (.abort 0)

/-- The witness flows: every output node is created. -/
def freshFlows : NativeFlows := fun _ _ _ _ _ => .fresh

theorem plainNatives_returned {f : FunctionRef} {views vals writes : List Value}
    {places : List NativePlace} (h : plainNatives P f views = .returned vals writes places) :
    writes = [] ∧ places = [] ∧ ∃ params returns, P.paramTypes f = some params ∧
      P.returnTypes f = some returns ∧ returnsPlain P f params returns views = true ∧
      vals = returns.map plainDefault := by
  simp only [plainNatives] at h
  split at h
  · next vals' hr =>
    simp only [NativeResult.returned.injEq] at h
    obtain ⟨rfl, rfl, rfl⟩ := h
    simp only [plainResults, bind, Option.bind_eq_some_iff] at hr
    obtain ⟨params, hp, returns, hrt, hr⟩ := hr
    split at hr
    · next hc =>
      exact ⟨rfl, rfl, params, returns, hp, hrt, hc, (Option.some.inj hr).symm⟩
    · simp at hr
  · simp at h

/-- A parameter whose argument the witness consumes is no reference: the
native sees the argument itself, and of an unowned type. -/
theorem consumed_unowned {f : FunctionRef} {params returns : List RuntimeType}
    {views : List Value} (hc : returnsPlain P f params returns views = true) {t : RuntimeType}
    (ht : t ∈ params) (hk : P.nativeKeeps f t = false) : t.seen = t ∧ unowned t = true := by
  simp only [returnsPlain, Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq, List.all_eq_true] at hc
  obtain ⟨⟨-, -⟩, hplain | ⟨hread, hvals⟩⟩ := hc
  · exact ⟨IsPlain_seen (hplain t ht), IsPlain_unowned (hplain t ht)⟩
  · have hseen : t.seen = t := by
      cases t with
      | ref m u => simp [Program.nativeKeeps, hread] at hk
      | _ => rfl
    refine ⟨hseen, ?_⟩
    cases hv : t.isValue
    · exact unowned_of_not_isValue hv
    · have := hvals t ht
      simp only [hv, Bool.not_true] at this
      exact IsPlain_unowned (this.resolve_left Bool.false_ne_true)

theorem nativesRespect_plain : NativesRespect P fun f views r => r = plainNatives P f views := by
  intro f args vals writes places h
  obtain ⟨rfl, rfl, params, returns, hp, hr, hc, rfl⟩ := plainNatives_returned h.symm
  refine ⟨returns, hr, ?_⟩
  have hc' := hc
  simp only [returnsPlain, Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq, List.all_eq_true] at hc'
  obtain ⟨⟨hrets, -⟩, hparams⟩ := hc'
  have hconf := conformsAll_plainDefault (P := P) hrets
  unfold Program.nativeResultFits
  split
  · next hshape =>
    rcases hparams with hplain | ⟨hread, -⟩
    · have hmut : RuntimeType.mutableTargets params = [] := by
        refine List.filterMap_eq_nil_iff.2 fun t ht => ?_
        have := hplain t ht
        cases t with
        | ref => simp [RuntimeType.IsPlain] at this
        | _ => rfl
      simp [hp, hconf, hmut]
    · rw [hshape] at hread; cases hread
  · have hval : returns.filter (·.isValue) = returns :=
      List.filter_eq_self.2 fun t ht => IsPlain_isValue (hrets t ht)
    have href : returns.filter (!·.isValue) = [] :=
      List.filter_eq_nil_iff.2 fun t ht => by simp [IsPlain_isValue (hrets t ht)]
    simp [hp, hval, href, hconf]
  all_goals simp [hconf]

theorem nativesRespectOwnership_plain (π : NativePolicy) :
    NativesRespectOwnership P (fun f views r => r = plainNatives P f views) freshFlows π := by
  intro f args vals writes places h
  obtain ⟨rfl, rfl, params, returns, hp, hr, hc, rfl⟩ := plainNatives_returned h.symm
  have hp' := hp
  simp only [Program.paramTypes, bind, Option.bind_eq_some_iff] at hp'
  obtain ⟨info, hf, -⟩ := hp'
  refine ⟨info, hf, ?_⟩
  have hconf : conformsAll P args (params.map RuntimeType.seen) = true := by
    simp only [returnsPlain, Bool.and_eq_true] at hc
    exact hc.1.2
  have horig : passedOn (freshFlows f args (returns.map plainDefault ++ []))
      (returns.map plainDefault ++ []) = [] := by
    simp [passedOn, freshFlows]
  unfold NativeCallRespects
  simp only [horig]
  simp only [freshFlows, hp, Option.bind_some, Bool.and_eq_true, List.all_eq_true]
  refine ⟨fun ⟨j, p, w⟩ hx => ?_, fun ⟨k, q, w⟩ hx => ?_⟩
  · obtain ⟨v, hv, hpw⟩ := mem_nodesOf hx
    obtain ⟨t, -, rfl⟩ := List.mem_map.1 (List.mem_of_getElem? (by simpa using hv))
    rw [plainDefault_nodes hpw]
    exact plainDefault_permits ..
  · obtain ⟨v, hv, hqw⟩ := mem_nodesOf hx
    split
    · simp
    · next hk =>
      simp only [List.contains_nil, Bool.or_true, List.count_nil,
        Nat.zero_le, decide_true, Bool.true_and, Bool.false_or]
      obtain ⟨t', ht', hvt⟩ := conformsAll_getElem hconf hv
      rw [List.getElem?_map, Option.map_eq_some_iff] at ht'
      obtain ⟨t, ht, rfl⟩ := ht'
      rw [ht] at hk
      have hk : P.nativeKeeps f t = false := by simpa using hk
      obtain ⟨hseen, hun⟩ := consumed_unowned hc (List.mem_of_getElem? ht) hk
      rw [hseen] at hvt
      exact permits_unowned π _ (nodes_unowned v hun hvt (q, w) hqw)

theorem plainNatives_of {f : FunctionRef} {params returns : List RuntimeType} {views : List Value}
    (hp : P.paramTypes f = some params) (hr : P.returnTypes f = some returns)
    (hc : returnsPlain P f params returns views = true) :
    plainNatives P f views = .returned (returns.map plainDefault) [] [] := by
  simp only [plainNatives, plainResults, hp, hr, hc, bind, Option.bind_some, ↓reduceIte]

theorem map_seen_of_plain : ∀ {ts : List RuntimeType}, (∀ t ∈ ts, t.IsPlain = true) →
    ts.map RuntimeType.seen = ts
  | [], _ => rfl
  | t :: ts, h => by
    rw [List.map_cons, IsPlain_seen (h t (List.mem_cons_self ..)),
      map_seen_of_plain fun u hu => h u (List.mem_cons_of_mem _ hu)]

/-- **N3.** -/
theorem nativeContractsSatisfiable : NativeContractsSatisfiable := by
  intro P π
  refine ⟨fun f views r => r = plainNatives P f views, freshFlows, nativesRespect_plain,
    nativesRespectOwnership_plain π, fun f args => ⟨_, rfl⟩, ?_, ?_⟩
  · intro f params returns args hp hr hplain hconf
    refine ⟨returns.map plainDefault, (plainNatives_of hp hr ?_).symm⟩
    have hps : ∀ t ∈ params, t.IsPlain = true := fun t ht => hplain t (List.mem_append_left _ ht)
    simp only [returnsPlain, map_seen_of_plain hps, hconf, Bool.and_true, Bool.and_eq_true,
      Bool.or_eq_true, List.all_eq_true]
    exact ⟨fun t ht => hplain t (List.mem_append_right _ ht), Or.inl hps⟩
  · intro f params returns views hshape hp hr hvals hrets hconf
    refine ⟨returns.map plainDefault, (plainNatives_of hp hr ?_).symm⟩
    simp only [returnsPlain, hshape, hconf, Bool.and_true, Bool.and_eq_true, Bool.or_eq_true,
      List.all_eq_true, beq_self_eq_true, Bool.true_and]
    refine ⟨hrets, Or.inr fun t ht => ?_⟩
    cases hv : t.isValue
    · exact Or.inl rfl
    · exact Or.inr (hvals t ht hv)

end MoveBytecode.Proofs
