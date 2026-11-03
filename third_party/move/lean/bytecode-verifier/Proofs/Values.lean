-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec
import Proofs.NonGeneric

/-!
# Values, conformance and integer ranges

UNTRUSTED (checked by the kernel). Facts about the specification's values
used by the soundness proof: conformance of value lists, the abilities a
conforming value has, and that every integer operation the semantics
completes yields a value in its type's range.

A value's abilities (`Value.abilities`) are its type's, narrowed by every
value inside it. When every declared ability of a struct or enum type is
justified by its field types (`AbilitiesSound`, from L6 and K1), a
well-formed value has all the abilities of its type (`abilities_of_conforms`):
so the semantics' ability checks, made on values, follow from the static ones,
made on types.
-/

namespace MoveBytecode.Proofs

variable {P : Program}

/-! ## Assignability

A value conforms to a type its own type is assignable to
(`RuntimeType.assignable`): the type itself, or, for a function type, one with
fewer abilities, and for an immutable reference, one to a type its target is
assignable to. To a type that is neither a function type nor an immutable
reference, a *rigid* type, only the type itself is assignable. -/

/-- A type to which only itself is assignable. -/
def rigid : RuntimeType → Bool
  | .function .. => false
  | .ref false _ => false
  | _ => true

@[simp] theorem assignable_refl : ∀ t : RuntimeType, t.assignable t = true
  | .function ps rs a => by
    cases a with | mk c d s k => cases c <;> cases d <;> cases s <;> cases k <;>
      simp [RuntimeType.assignable, MoveBytecode.AbilitySet.subset]
  | .ref false t => by simp only [RuntimeType.assignable]; exact assignable_refl t
  | .ref true t => by simp [RuntimeType.assignable]
  | .bool => by simp [RuntimeType.assignable]
  | .int _ => by simp [RuntimeType.assignable]
  | .address => by simp [RuntimeType.assignable]
  | .signer => by simp [RuntimeType.assignable]
  | .vector _ => by simp [RuntimeType.assignable]
  | .struct _ => by simp [RuntimeType.assignable]

theorem assignable_rigid {t u : RuntimeType} (h : t.assignable u = true) (hu : rigid u = true) :
    t = u := by
  cases u with
  | function => simp [rigid] at hu
  | ref m w =>
    cases m with
    | false => simp [rigid] at hu
    | true => cases t <;> simp_all [RuntimeType.assignable]
  | _ => cases t <;> simp_all [RuntimeType.assignable]

/-- What assignability to a type means, by the type. -/
theorem assignable_cases {t u : RuntimeType} (h : t.assignable u = true) :
    (∃ ps rs a a', t = .function ps rs a ∧ u = .function ps rs a' ∧ a'.subset a = true) ∨
    (∃ v w, t = .ref false v ∧ u = .ref false w ∧ v.assignable w = true) ∨ t = u := by
  cases u with
  | function ps rs a' =>
    cases t with
    | function ps0 rs0 a0 =>
      simp only [RuntimeType.assignable, Bool.and_eq_true, beq_iff_eq] at h
      obtain ⟨⟨rfl, rfl⟩, hs⟩ := h
      exact Or.inl ⟨ps0, rs0, a0, a', rfl, rfl, hs⟩
    | _ => simp_all [RuntimeType.assignable]
  | ref m w =>
    cases m with
    | false =>
      cases t with
      | ref m' v => cases m' <;> simp_all [RuntimeType.assignable]
      | _ => simp_all [RuntimeType.assignable]
    | true => right; right; cases t <;> simp_all [RuntimeType.assignable]
  | _ => right; right; cases t <;> simp_all [RuntimeType.assignable]

theorem ability_subset_trans {a b c : AbilitySet} (h1 : a.subset b = true)
    (h2 : b.subset c = true) : a.subset c = true := by
  cases a; cases b; cases c
  simp_all [MoveBytecode.AbilitySet.subset]
  grind

theorem assignable_trans : ∀ {a b c : RuntimeType}, a.assignable b = true →
    b.assignable c = true → a.assignable c = true
  | _, _, _, h1, h2 => by
    rcases assignable_cases h2 with ⟨ps, rs, ab, ac, rfl, rfl, hs⟩ | ⟨v, w, rfl, rfl, hvw⟩ | rfl
    · rcases assignable_cases h1 with ⟨ps', rs', aa, ab', rfl, he, hs'⟩ | ⟨_, _, _, he, _⟩ | rfl
      · simp only [RuntimeType.function.injEq] at he
        obtain ⟨rfl, rfl, rfl⟩ := he
        simp [RuntimeType.assignable, ability_subset_trans hs hs']
      · cases he
      · exact h2
    · rcases assignable_cases h1 with ⟨_, _, _, _, _, he, _⟩ | ⟨u, v', rfl, he, huv⟩ | rfl
      · cases he
      · simp only [RuntimeType.ref.injEq, true_and] at he
        subst he
        simp only [RuntimeType.assignable]
        exact assignable_trans huv hvw
      · exact h2
    · exact h1
termination_by _ _ c => sizeOf c

/-- A type assignable to `u` has at least `u`'s abilities. -/
theorem assignable_abilities {t u : RuntimeType} (h : t.assignable u = true) :
    (u.abilities P).subset (t.abilities P) = true := by
  rcases assignable_cases h with ⟨ps, rs, a, a', rfl, rfl, hs⟩ | ⟨v, w, rfl, rfl, _⟩ | rfl
  · simpa [RuntimeType.abilities] using hs
  · simp [RuntimeType.abilities, MoveBytecode.AbilitySet.subset]
  · generalize RuntimeType.abilities P t = x
    cases x with | mk c d s k => cases c <;> cases d <;> cases s <;> cases k <;> rfl

theorem assignable_isValue {t u : RuntimeType} (h : t.assignable u = true) :
    t.isValue = u.isValue := by
  rcases assignable_cases h with ⟨ps, rs, a, a', rfl, rfl, hs⟩ | ⟨v, w, rfl, rfl, _⟩ | rfl
  · rfl
  · rfl
  · rfl

@[simp] theorem comparable_refl : ∀ t : RuntimeType, t.comparable t = true
  | .function ps rs a => by simp [RuntimeType.comparable]
  | .ref false t => by simp only [RuntimeType.comparable]; exact comparable_refl t
  | .ref true t => by simp [RuntimeType.comparable]
  | .bool => by simp [RuntimeType.comparable]
  | .int _ => by simp [RuntimeType.comparable]
  | .address => by simp [RuntimeType.comparable]
  | .signer => by simp [RuntimeType.comparable]
  | .vector _ => by simp [RuntimeType.comparable]
  | .struct _ => by simp [RuntimeType.comparable]

@[simp] theorem assignableAll_refl : ∀ ts : List RuntimeType, RuntimeType.assignableAll ts ts = true
  | [] => rfl
  | t :: ts => by simp [RuntimeType.assignableAll, assignableAll_refl ts]

theorem assignableAll_cons {t u : RuntimeType} {ts us : List RuntimeType} :
    RuntimeType.assignableAll (t :: ts) (u :: us) = true ↔
      t.assignable u = true ∧ RuntimeType.assignableAll ts us = true := by
  simp [RuntimeType.assignableAll]

theorem assignableAll_length : ∀ {ts us : List RuntimeType},
    RuntimeType.assignableAll ts us = true → ts.length = us.length
  | [], [], _ => rfl
  | _ :: _, _ :: _, h => by
    simp only [RuntimeType.assignableAll, Bool.and_eq_true] at h
    simp [assignableAll_length h.2]
  | [], _ :: _, h | _ :: _, [], h => by simp [RuntimeType.assignableAll] at h

theorem assignableAll_getElem : ∀ {ts us : List RuntimeType},
    RuntimeType.assignableAll ts us = true → ∀ {i : Nat} {t u : RuntimeType},
      ts[i]? = some t → us[i]? = some u → t.assignable u = true
  | [], _, _, _, _, _, ht, _ => by simp at ht
  | _ :: _, [], h, _, _, _, _, _ => by simp [RuntimeType.assignableAll] at h
  | t' :: ts, u' :: us, h, i, t, u, ht, hu => by
    simp only [RuntimeType.assignableAll, Bool.and_eq_true] at h
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at ht hu
      subst ht hu
      exact h.1
    | succ i =>
      simp only [List.getElem?_cons_succ] at ht hu
      exact assignableAll_getElem h.2 ht hu

theorem assignableAll_append : ∀ {a b c d : List RuntimeType},
    RuntimeType.assignableAll a c = true → RuntimeType.assignableAll b d = true →
    RuntimeType.assignableAll (a ++ b) (c ++ d) = true
  | [], b, [], d, _, h2 => by simpa using h2
  | x :: a, b, y :: c, d, h1, h2 => by
    simp only [RuntimeType.assignableAll, Bool.and_eq_true] at h1
    simp [RuntimeType.assignableAll, h1.1, assignableAll_append h1.2 h2]
  | [], _, _ :: _, _, h, _ | _ :: _, _, [], _, h, _ => by simp [RuntimeType.assignableAll] at h

theorem assignableAll_reverse : ∀ {a b : List RuntimeType},
    RuntimeType.assignableAll a b = true → RuntimeType.assignableAll a.reverse b.reverse = true
  | [], [], _ => rfl
  | x :: a, y :: b, h => by
    simp only [RuntimeType.assignableAll, Bool.and_eq_true] at h
    simp only [List.reverse_cons]
    exact assignableAll_append (assignableAll_reverse h.2) (by simp [RuntimeType.assignableAll, h.1])
  | [], _ :: _, h | _ :: _, [], h => by simp [RuntimeType.assignableAll] at h

/-- Values conforming to types assignable, one for one, to others conform to
those. -/
theorem conformsAll_assignable : ∀ {vs : List Value} {ts us : List RuntimeType},
    conformsAll P vs ts = true → RuntimeType.assignableAll ts us = true → conformsAll P vs us = true
  | [], [], [], _, _ => rfl
  | v :: vs, t :: ts, u :: us, h1, h2 => by
    simp only [conformsAll, Bool.and_eq_true] at h1
    simp only [RuntimeType.assignableAll, Bool.and_eq_true] at h2
    simp only [conformsAll, Bool.and_eq_true]
    exact ⟨⟨assignable_trans h1.1.1 h2.1, h1.1.2⟩, conformsAll_assignable h1.2 h2.2⟩
  | [], [], _ :: _, _, h2 => by simp [RuntimeType.assignableAll] at h2
  | [], _ :: _, _, h1, _ => by simp [conformsAll] at h1
  | _ :: _, [], _, h1, _ => by simp [conformsAll] at h1
  | _ :: _, _ :: _, [], _, h2 => by simp [RuntimeType.assignableAll] at h2

/-- Values of types assignable to comparable types are comparable: through
immutable references, comparability is equality but at function types, where
only the parameters and results must agree. -/
theorem comparable_of_assignable : ∀ {a b t u : RuntimeType}, t.comparable u = true →
    a.assignable t = true → b.assignable u = true → a.comparable b = true
  | a, b, _, _, htu, ha, hb => by
    rcases assignable_cases ha with ⟨ps, rs, x, x', rfl, rfl, _⟩ | ⟨v, w, rfl, rfl, hvw⟩ | rfl
    · rcases assignable_cases hb with ⟨ps', rs', y, y', rfl, rfl, _⟩ | ⟨_, _, rfl, rfl, _⟩ | rfl
      · simpa [RuntimeType.comparable] using htu
      · simp [RuntimeType.comparable] at htu
      · cases b <;> simp_all [RuntimeType.comparable]
    · rcases assignable_cases hb with ⟨_, _, _, _, rfl, rfl, _⟩ | ⟨v', w', rfl, rfl, hvw'⟩ | rfl
      · simp [RuntimeType.comparable] at htu
      · simp only [RuntimeType.comparable] at htu ⊢
        exact comparable_of_assignable htu hvw hvw'
      · cases b with
        | ref m u0 =>
          cases m
          · simp only [RuntimeType.comparable] at htu ⊢
            exact comparable_of_assignable htu hvw (assignable_refl u0)
          · simp [RuntimeType.comparable] at htu
        | _ => simp [RuntimeType.comparable] at htu
    · rcases assignable_cases hb with ⟨_, _, _, _, rfl, rfl, _⟩ | ⟨v', w', rfl, rfl, hvw'⟩ | rfl
      · cases a <;> simp_all [RuntimeType.comparable]
      · cases a with
        | ref m t0 =>
          cases m
          · simp only [RuntimeType.comparable] at htu ⊢
            exact comparable_of_assignable htu (assignable_refl t0) hvw'
          · simp [RuntimeType.comparable] at htu
        | _ => simp [RuntimeType.comparable] at htu
      · exact htu
termination_by _ _ t => sizeOf t

/-! ## Conformance -/

theorem conforms_iff {v : Value} {t : RuntimeType} :
    v.conforms P t = true ↔ v.type.assignable t = true ∧ v.wellFormed P = true := by
  simp [Value.conforms]

/-- A conforming value's type is assignable to the declared type. -/
theorem conforms_assignable {v : Value} {t : RuntimeType} (h : v.conforms P t = true) :
    v.type.assignable t = true :=
  (conforms_iff.1 h).1

/-- A value conforming to a rigid type has that type. -/
theorem conforms_type {v : Value} {t : RuntimeType} (h : v.conforms P t = true)
    (ht : rigid t = true) : v.type = t :=
  assignable_rigid (conforms_iff.1 h).1 ht

/-- A value of a type assignable to `t` conforms to `t` when well formed. -/
theorem conforms_of {v : Value} {t : RuntimeType} (h : v.type.assignable t = true)
    (hw : v.wellFormed P = true) : v.conforms P t = true :=
  conforms_iff.2 ⟨h, hw⟩

/-- Conformance is inherited along assignability. -/
theorem conforms_trans {v : Value} {t u : RuntimeType} (h : v.conforms P t = true)
    (htu : t.assignable u = true) : v.conforms P u = true :=
  conforms_iff.2 ⟨assignable_trans (conforms_assignable h) htu, (conforms_iff.1 h).2⟩

theorem conforms_wellFormed {v : Value} {t : RuntimeType} (h : v.conforms P t = true) :
    v.wellFormed P = true :=
  (conforms_iff.1 h).2

theorem conforms_bool {v : Value} (h : v.conforms P .bool = true) : ∃ b, v = .bool b := by
  have := conforms_type h rfl
  cases v <;> simp_all [Value.type]

theorem conforms_int {v : Value} {t : IntType} (h : v.conforms P (.int t) = true) :
    ∃ x, v = .int t x ∧ t.inRange x = true := by
  have ht := conforms_type h rfl
  have hw := conforms_wellFormed h
  cases v <;> simp_all [Value.type, Value.wellFormed]

@[simp] theorem conforms_int_iff {t t' : IntType} {x : Int} :
    (Value.int t x).conforms P (.int t') = true ↔ t = t' ∧ t.inRange x = true := by
  simp [Value.conforms, Value.type, Value.wellFormed, RuntimeType.assignable]

@[simp] theorem conforms_bool_iff {b : Bool} : (Value.bool b).conforms P .bool = true := by
  simp [Value.conforms, Value.type, Value.wellFormed]

@[simp] theorem conformsAll_nil_nil : conformsAll P [] [] = true := by simp [conformsAll]

@[simp] theorem conformsAll_cons {v : Value} {vs : List Value} {t : RuntimeType}
    {ts : List RuntimeType} :
    conformsAll P (v :: vs) (t :: ts) = true ↔
      v.conforms P t = true ∧ conformsAll P vs ts = true := by
  simp [conformsAll, Value.conforms]

@[simp] theorem conformsAll_nil_cons {t : RuntimeType} {ts : List RuntimeType} :
    conformsAll P [] (t :: ts) = false := by simp [conformsAll]

@[simp] theorem conformsAll_cons_nil {v : Value} {vs : List Value} :
    conformsAll P (v :: vs) [] = false := by simp [conformsAll]

theorem conformsAll_length {vs : List Value} {ts : List RuntimeType}
    (h : conformsAll P vs ts = true) : vs.length = ts.length := by
  induction vs generalizing ts with
  | nil => cases ts <;> simp_all
  | cons v vs ih =>
    cases ts with
    | nil => simp at h
    | cons t ts => simp only [conformsAll_cons] at h; simp [ih h.2]

theorem conformsAll_append {vs ws : List Value} {ts us : List RuntimeType}
    (h1 : conformsAll P vs ts = true) (h2 : conformsAll P ws us = true) :
    conformsAll P (vs ++ ws) (ts ++ us) = true := by
  induction vs generalizing ts with
  | nil => cases ts <;> simp_all
  | cons v vs ih => cases ts <;> simp_all

theorem conformsAll_reverse {vs : List Value} {ts : List RuntimeType}
    (h : conformsAll P vs ts = true) : conformsAll P vs.reverse ts.reverse = true := by
  induction vs generalizing ts with
  | nil => cases ts <;> simp_all
  | cons v vs ih =>
    cases ts with
    | nil => simp at h
    | cons t ts =>
      simp only [conformsAll_cons] at h
      simp only [List.reverse_cons]
      exact conformsAll_append (ih h.2) (by simp [h.1])

theorem conformsAll_take {vs : List Value} {ts : List RuntimeType} (n : Nat)
    (h : conformsAll P vs ts = true) : conformsAll P (vs.take n) (ts.take n) = true := by
  induction vs generalizing ts n with
  | nil => cases ts <;> simp_all
  | cons v vs ih => cases ts <;> cases n <;> simp_all

theorem conformsAll_drop {vs : List Value} {ts : List RuntimeType} (n : Nat)
    (h : conformsAll P vs ts = true) : conformsAll P (vs.drop n) (ts.drop n) = true := by
  induction vs generalizing ts n with
  | nil => cases ts <;> simp_all
  | cons v vs ih => cases ts <;> cases n <;> simp_all

theorem conformsAll_mem {vs : List Value} {ts : List RuntimeType}
    (h : conformsAll P vs ts = true) : ∀ v ∈ vs, ∃ t ∈ ts, v.conforms P t = true := by
  induction vs generalizing ts with
  | nil => simp
  | cons v vs ih =>
    cases ts with
    | nil => simp at h
    | cons t ts =>
      simp only [conformsAll_cons] at h
      intro w hw
      rcases List.mem_cons.1 hw with rfl | hw
      · exact ⟨t, List.mem_cons_self .., h.1⟩
      · obtain ⟨u, hu, hc⟩ := ih h.2 w hw
        exact ⟨u, List.mem_cons_of_mem _ hu, hc⟩

@[simp] theorem allConform_cons {v : Value} {vs : List Value} {t : RuntimeType} :
    Value.allConform P (v :: vs) t = true ↔
      v.conforms P t = true ∧ Value.allConform P vs t = true := by
  simp [Value.allConform, Value.conforms]

theorem allConform_iff {vs : List Value} {t : RuntimeType} :
    Value.allConform P vs t = true ↔ ∀ v ∈ vs, v.conforms P t = true := by
  induction vs with
  | nil => simp [Value.allConform]
  | cons v vs ih => simp [ih]

/-- Conformance to `n` copies of one type. -/
theorem conformsAll_replicate {vs : List Value} {t : RuntimeType} :
    conformsAll P vs (List.replicate vs.length t) = true ↔ ∀ v ∈ vs, v.conforms P t = true := by
  induction vs with
  | nil => simp
  | cons v vs ih => simp [List.replicate_succ, ih]

theorem wellFormed_vector {t : RuntimeType} {vs : List Value} :
    (Value.vector t vs).wellFormed P = true ↔ ∀ v ∈ vs, v.conforms P t = true := by
  simp [Value.wellFormed, allConform_iff]

theorem wellFormed_struct {s : StructId} {fs : List Value} :
    (Value.struct s fs).wellFormed P = true ↔
      s.isTable = false ∧ ∃ tys, P.fieldTypes s = some tys ∧ conformsAll P fs tys = true := by
  simp only [Value.wellFormed]
  cases P.fieldTypes s <;> simp

/-- A table is well formed: it has the table type, its fields conform, one key
per entry, and, with entries, its program's tables may hold entries, keys and
entries conform, the keys are distinct, and the type's abilities are no more
than the entries allow (`Value.wellFormed`). -/
theorem wellFormed_table {s : StructId} {fs ks es : List Value} :
    (Value.table s fs ks es).wellFormed P = true ↔
      s.isTable = true ∧ ks.length = es.length ∧
      (∃ tys, P.fieldTypes s = some tys ∧ conformsAll P fs tys = true) ∧
      (es = [] ∨ P.tablesHoldEntries = true ∧ ∃ k v, s.typeArgs = [k, v] ∧
        Value.allConform P ks k = true ∧ Value.allConform P es (tableEntryType v) = true ∧
        ks.Nodup ∧ ((RuntimeType.struct s).abilities P).copy = false ∧
        ((RuntimeType.struct s).abilities P).subset
          ((tableEntryType v).abilities P).forContainer = true) := by
  simp only [Value.wellFormed]
  cases P.fieldTypes s <;> rcases s.typeArgs with _ | ⟨k, _ | ⟨v, _ | ⟨w, ts⟩⟩⟩ <;> simp [and_assoc]

theorem wellFormed_variant {s : StructId} {tag : VariantIndex} {fs : List Value} :
    (Value.variant s tag fs).wellFormed P = true ↔
      ∃ tys, P.variantFieldTypes s tag = some tys ∧ conformsAll P fs tys = true := by
  simp only [Value.wellFormed]
  cases P.variantFieldTypes s tag <;> simp

/-! ## Ability sets -/

namespace AbilitySet

theorem subset_refl (a : AbilitySet) : a.subset a = true := by
  cases a with | mk c d s k => cases c <;> cases d <;> cases s <;> cases k <;> rfl

theorem subset_trans {a b c : AbilitySet} (h1 : a.subset b = true) (h2 : b.subset c = true) :
    a.subset c = true := by
  cases a; cases b; cases c
  simp_all [MoveBytecode.AbilitySet.subset]
  grind

theorem subset_inter {a b c : AbilitySet} :
    a.subset (b.inter c) = true ↔ a.subset b = true ∧ a.subset c = true := by
  cases a; cases b; cases c
  simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
  grind

theorem subset_all (a : AbilitySet) : a.subset AbilitySet.all = true := by
  cases a; simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.all]

theorem forContainer_mono {a b : AbilitySet} (h : a.subset b = true) :
    a.forContainer.subset b.forContainer = true := by
  cases a; cases b
  simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.forContainer]
  all_goals grind

theorem inter_subset_right (a b : AbilitySet) : (a.inter b).subset b = true := by
  cases a; cases b; simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]; grind

theorem inter_subset_left (a b : AbilitySet) : (a.inter b).subset a = true := by
  cases a; cases b; simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]; grind

theorem drop_of_subset {a b : AbilitySet} (h : a.subset b = true) (ha : a.drop = true) :
    b.drop = true := by
  cases a; cases b; simp_all [MoveBytecode.AbilitySet.subset]

theorem copy_of_subset {a b : AbilitySet} (h : a.subset b = true) (ha : a.copy = true) :
    b.copy = true := by
  cases a; cases b; simp_all [MoveBytecode.AbilitySet.subset]

theorem key_of_subset {a b : AbilitySet} (h : a.subset b = true) (ha : a.key = true) :
    b.key = true := by
  cases a; cases b; simp_all [MoveBytecode.AbilitySet.subset]

end AbilitySet

/-! ## The abilities of conforming values -/

/-- Every declared ability of a struct or enum type is justified by the
field rule on every field type of every variant. -/
def AbilitiesSound (P : Program) : Prop :=
  ∀ s tys, (P.fieldTypes s = some tys ∨ ∃ tag, P.variantFieldTypes s tag = some tys) →
    ∀ t ∈ tys, ((RuntimeType.struct s).abilities P).subset (t.abilities P).forContainer = true

theorem contentAbilities_ge (A : AbilitySet) :
    ∀ (vs : List Value) (tys : List RuntimeType), conformsAll P vs tys = true →
      (∀ v ∈ vs, v.wellFormed P = true → (v.type.abilities P).subset (v.abilities P) = true) →
      (∀ t ∈ tys, A.subset (t.abilities P).forContainer = true) →
      A.subset (Value.contentAbilities P vs) = true
  | [], _, _, _, _ => by simp [Value.contentAbilities, AbilitySet.subset_all]
  | v :: vs, t :: ts, h, hv, ht => by
    simp only [conformsAll_cons] at h
    simp only [Value.contentAbilities, AbilitySet.subset_inter]
    refine ⟨?_, contentAbilities_ge A vs ts h.2 (fun w hw => hv w (List.mem_cons_of_mem _ hw))
      (fun u hu => ht u (List.mem_cons_of_mem _ hu))⟩
    have := ability_subset_trans (assignable_abilities (conforms_assignable h.1))
      (hv v (List.mem_cons_self ..) (conforms_wellFormed h.1))
    exact AbilitySet.subset_trans (ht t (List.mem_cons_self ..)) (AbilitySet.forContainer_mono this)
  | _ :: _, [], h, _, _ => by simp at h

theorem contentAbilities_ge_all (A : AbilitySet) (t : RuntimeType) :
    ∀ (vs : List Value), (∀ v ∈ vs, v.conforms P t = true) →
      (∀ v ∈ vs, v.wellFormed P = true → (v.type.abilities P).subset (v.abilities P) = true) →
      A.subset (t.abilities P).forContainer = true →
      A.subset (Value.contentAbilities P vs) = true
  | [], _, _, _ => by simp [Value.contentAbilities, AbilitySet.subset_all]
  | v :: vs, h, hv, ht => by
    simp only [Value.contentAbilities, AbilitySet.subset_inter]
    refine ⟨?_, contentAbilities_ge_all A t vs (fun w hw => h w (List.mem_cons_of_mem _ hw))
      (fun w hw => hv w (List.mem_cons_of_mem _ hw)) ht⟩
    have hc := h v (List.mem_cons_self ..)
    have := ability_subset_trans (assignable_abilities (conforms_assignable hc))
      (hv v (List.mem_cons_self ..) (conforms_wellFormed hc))
    exact AbilitySet.subset_trans ht (AbilitySet.forContainer_mono this)

theorem subset_forContainer_of {a x : AbilitySet} (h : a.subset x = true) (hk : a.key = false) :
    a.subset x.forContainer = true := by
  cases a; cases x
  simp_all [MoveBytecode.AbilitySet.subset, AbilitySet.forContainer]

theorem capturedAbilities_sub :
    ∀ {tys : List RuntimeType} {t : RuntimeType}, t ∈ tys →
      (RuntimeType.capturedAbilities P tys).subset (t.abilities P) = true
  | _ :: _, _, .head _ => by
    simp only [RuntimeType.capturedAbilities]
    cases (RuntimeType.abilities P _); cases (RuntimeType.capturedAbilities P _)
    simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
    grind
  | u :: us, t, .tail _ hm => by
    simp only [RuntimeType.capturedAbilities]
    exact ability_subset_trans (AbilitySet.inter_subset_right _ _) (capturedAbilities_sub hm)

theorem capturedAbilities_ge {a : AbilitySet} :
    ∀ {us : List RuntimeType}, (∀ u ∈ us, a.subset (u.abilities P) = true) →
      a.subset (RuntimeType.capturedAbilities P us) = true
  | [], _ => AbilitySet.subset_all a
  | u :: us, h => by
    simp only [RuntimeType.capturedAbilities]
    exact AbilitySet.subset_inter.2 ⟨h u (List.mem_cons_self ..),
      capturedAbilities_ge (fun w hw => h w (List.mem_cons_of_mem _ hw))⟩

/-- Values well formed conform to their own types. -/
theorem conformsAll_types :
    ∀ {vs : List Value}, (∀ v ∈ vs, v.wellFormed P = true) → conformsAll P vs (vs.map Value.type) = true
  | [], _ => conformsAll_nil_nil
  | v :: vs, h => by
    simp only [List.map_cons, conformsAll_cons]
    exact ⟨conforms_of (assignable_refl _) (h v (List.mem_cons_self ..)),
      conformsAll_types (fun w hw => h w (List.mem_cons_of_mem _ hw))⟩

theorem conformsAll_wellFormed :
    ∀ {vs : List Value} {ts : List RuntimeType}, conformsAll P vs ts = true →
      ∀ v ∈ vs, v.wellFormed P = true
  | [], _, _, _, hv => by simp at hv
  | _ :: _, [], h, _, _ => by simp at h
  | w :: ws, t :: ts, h, v, hv => by
    rw [conformsAll_cons] at h
    rcases List.mem_cons.1 hv with rfl | hv
    · exact conforms_wellFormed h.1
    · exact conformsAll_wellFormed h.2 v hv

/-- A closure's abilities are within the abilities of each captured value's
type, as for every container. -/
theorem closureValueAbilities_sub {f : FunctionRef} {captured : List Value} {t : RuntimeType}
    (ht : t ∈ captured.map Value.type) :
    (P.closureValueAbilities f captured).subset (t.abilities P) = true :=
  ability_subset_trans (AbilitySet.inter_subset_right _ _) (capturedAbilities_sub ht)

theorem closureAbilities_key (f : FunctionRef) {b : AbilitySet} :
    ((P.closureAbilities f).inter b).key = false := by
  simp [Program.closureAbilities, MoveBytecode.AbilitySet.inter]

/-- **A well-formed value has the abilities of its type**, in a program whose
declared abilities are justified by their fields. -/
theorem abilities_of_wellFormed (hP : AbilitiesSound P) :
    ∀ v : Value, v.wellFormed P = true → (v.type.abilities P).subset (v.abilities P) = true := by
  intro v
  induction v using Value.rec
    (motive_2 := fun vs => ∀ v ∈ vs, v.wellFormed P = true →
      (v.type.abilities P).subset (v.abilities P) = true) with
  | bool b => intro _; exact AbilitySet.subset_refl _
  | int t x => intro _; exact AbilitySet.subset_refl _
  | address a => intro _; exact AbilitySet.subset_refl _
  | signer a => intro _; exact AbilitySet.subset_refl _
  | ref r => intro _; exact AbilitySet.subset_refl _
  | closure f mask captured ps rs a ih =>
    intro hw
    simp only [Value.wellFormed] at hw
    split at hw
    · next cap rem res hct =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hw
      obtain ⟨⟨⟨rfl, rfl⟩, rfl⟩, hc⟩ := hw
      simp only [Value.type, Value.abilities, AbilitySet.subset_inter]
      refine ⟨AbilitySet.subset_refl _, contentAbilities_ge _ captured (captured.map Value.type)
        (conformsAll_types (conformsAll_wellFormed hc)) ih ?_⟩
      intro t ht
      simp only [RuntimeType.abilities]
      exact subset_forContainer_of (closureValueAbilities_sub ht) (closureAbilities_key f)
    · simp at hw
  | vector t vs ih =>
    intro hw
    rw [wellFormed_vector] at hw
    simp only [Value.type, Value.abilities, AbilitySet.subset_inter]
    refine ⟨AbilitySet.subset_refl _, contentAbilities_ge_all _ t vs hw ih ?_⟩
    exact AbilitySet.inter_subset_right _ _
  | struct s fs ih =>
    intro hw
    obtain ⟨-, tys, htys, hc⟩ := wellFormed_struct.1 hw
    simp only [Value.type, Value.abilities, AbilitySet.subset_inter]
    exact ⟨AbilitySet.subset_refl _, contentAbilities_ge _ fs tys hc ih (hP s tys (Or.inl htys))⟩
  | table s fs ks es ihf _ ihe =>
    intro hw
    obtain ⟨-, -, ⟨tys, htys, hc⟩, hes⟩ := wellFormed_table.1 hw
    simp only [Value.type, Value.abilities, AbilitySet.subset_inter]
    refine ⟨⟨AbilitySet.subset_refl _, contentAbilities_ge _ fs tys hc ihf (hP s tys (Or.inl htys))⟩,
      ?_⟩
    rcases hes with rfl | ⟨-, k, v, -, -, hce, -, -, hsub⟩
    · simp [Value.contentAbilities, AbilitySet.subset_all]
    · exact contentAbilities_ge_all _ _ es (allConform_iff.1 hce) ihe hsub
  | variant s tag fs ih =>
    intro hw
    obtain ⟨tys, htys, hc⟩ := wellFormed_variant.1 hw
    simp only [Value.type, Value.abilities, AbilitySet.subset_inter]
    exact ⟨AbilitySet.subset_refl _,
      contentAbilities_ge _ fs tys hc ih (hP s tys (Or.inr ⟨tag, htys⟩))⟩
  | nil => simp_all
  | cons v vs ihv ihvs =>
    rename_i w hw hwf
    rcases List.mem_cons.1 hw with rfl | hw
    · exact ihv hwf
    · exact ihvs w hw hwf

/-- A value conforming to a type has the type's abilities. -/
theorem abilities_of_conforms (hP : AbilitiesSound P) {v : Value} {t : RuntimeType}
    (h : v.conforms P t = true) : (t.abilities P).subset (v.abilities P) = true :=
  ability_subset_trans (assignable_abilities (conforms_assignable h))
    (abilities_of_wellFormed hP v (conforms_wellFormed h))

/-! ## Ranges -/

theorem width_pos (t : IntType) : 0 < t.width := by cases t <;> decide

theorem inRange_unsigned {t : IntType} (hs : t.isSigned = false) {x : Int} :
    t.inRange x = true ↔ 0 ≤ x ∧ x < 2 ^ t.width := by
  simp [IntType.inRange, IntType.min, IntType.max, hs]
  omega

theorem inRange_checked {t : IntType} {x r : Int} (h : t.checked x = some r) :
    t.inRange r = true := by
  unfold IntType.checked at h
  split at h <;> simp_all


/-- A literal of the type's width is in range. -/
theorem inRange_ofBitVec {t : IntType} {n : Nat} {v : BitVec n} (hn : n = t.width) :
    t.inRange (t.ofBitVec v) = true := by
  subst hn
  have h1 := BitVec.toInt_lt (x := v)
  have h2 := BitVec.le_toInt v
  have h3 := BitVec.isLt v
  unfold IntType.ofBitVec IntType.inRange IntType.min IntType.max
  cases hs : t.isSigned <;> simp
  · have : ((2 ^ t.width : Nat) : Int) = (2 : Int) ^ t.width := by simp
    omega
  · omega

theorem natCast_two_pow (w : Nat) : ((2 ^ w : Nat) : Int) = (2 : Int) ^ w := by simp

theorem toNat_lt_of_inRange {t : IntType} (hs : t.isSigned = false) {x : Int}
    (h : t.inRange x = true) : x.toNat < 2 ^ t.width ∧ x = x.toNat := by
  rw [inRange_unsigned hs] at h
  have := natCast_two_pow t.width
  omega

theorem inRange_ofNat_lt {t : IntType} (hs : t.isSigned = false) {n : Nat} (h : n < 2 ^ t.width) :
    t.inRange (Int.ofNat n) = true := by
  rw [inRange_unsigned hs]
  have := natCast_two_pow t.width
  simp only [Int.ofNat_eq_natCast]
  omega

theorem inRange_bitOr {t : IntType} (hs : t.isSigned = false) {a b : Int}
    (ha : t.inRange a = true) (hb : t.inRange b = true) : t.inRange (IntType.bitOr a b) = true :=
  inRange_ofNat_lt hs (Nat.or_lt_two_pow (toNat_lt_of_inRange hs ha).1 (toNat_lt_of_inRange hs hb).1)

theorem inRange_bitXor {t : IntType} (hs : t.isSigned = false) {a b : Int}
    (ha : t.inRange a = true) (hb : t.inRange b = true) : t.inRange (IntType.bitXor a b) = true :=
  inRange_ofNat_lt hs (Nat.xor_lt_two_pow (toNat_lt_of_inRange hs ha).1 (toNat_lt_of_inRange hs hb).1)

theorem inRange_bitAnd {t : IntType} (hs : t.isSigned = false) {a b : Int}
    (ha : t.inRange a = true) : t.inRange (IntType.bitAnd a b) = true :=
  inRange_ofNat_lt hs (Nat.lt_of_le_of_lt Nat.and_le_left (toNat_lt_of_inRange hs ha).1)

theorem inRange_shl {t : IntType} (hs : t.isSigned = false) {a r : Int} {n : Nat}
    (h : IntType.shl t a n = some r) : t.inRange r = true := by
  unfold IntType.shl at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    exact inRange_ofNat_lt hs (Nat.mod_lt _ (Nat.two_pow_pos _))
  · simp at h

theorem inRange_shr {t : IntType} (hs : t.isSigned = false) {a r : Int} {n : Nat}
    (ha : t.inRange a = true) (h : IntType.shr t a n = some r) : t.inRange r = true := by
  unfold IntType.shr at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    exact inRange_ofNat_lt hs
      (Nat.lt_of_le_of_lt (Nat.shiftRight_le _ _) (toNat_lt_of_inRange hs ha).1)
  · simp at h

theorem inRange_rem {t : IntType} {a b r : Int} (h : IntType.rem t a b = some r) :
    t.inRange r = true := by
  unfold IntType.rem at h
  split at h
  · simp at h
  · split at h
    · simp at h
    · exact inRange_checked h

/-! ## What `Pack` makes -/

/-- A struct value of type `s` with fields `fs`, or a table of type `s` with
fields `fs` (`Value.table`): what `Pack` makes, and `Unpack` and field borrows
take apart. -/
def StructLike (v : Value) (s : StructId) (fs : List Value) : Prop :=
  v = .struct s fs ∨ ∃ ks es, v = .table s fs ks es

/-- The value `Pack` makes of type `s` from fields `vals`
(`Step.packStruct`). -/
def packedValue (s : StructId) (vals : List Value) : Value :=
  if s.isTable then .table s vals [] [] else .struct s vals

theorem packedValue_structLike (s : StructId) (vals : List Value) :
    StructLike (packedValue s vals) s vals := by
  unfold packedValue StructLike
  split
  · exact Or.inr ⟨[], [], rfl⟩
  · exact Or.inl rfl

@[simp] theorem packedValue_type (s : StructId) (vals : List Value) :
    (packedValue s vals).type = .struct s := by
  unfold packedValue; split <;> rfl

@[simp] theorem packedValue_children (s : StructId) (vals : List Value) :
    (packedValue s vals).children = vals := by
  unfold packedValue; split <;> simp [Value.children]

theorem packedValue_ne_ref (s : StructId) (vals : List Value) (r : Reference) :
    packedValue s vals ≠ .ref r := by
  unfold packedValue; split <;> simp

/-- What `Pack` makes conforms to its type. -/
theorem packedValue_conforms {s : StructId} {vals : List Value} {tys : List RuntimeType}
    (hf : P.fieldTypes s = some tys) (hc : conformsAll P vals tys = true) :
    (packedValue s vals).conforms P (.struct s) = true := by
  unfold packedValue
  split
  · next ht => simp [Value.conforms, Value.type, wellFormed_table, ht, hf, hc]
  · next ht => simp [Value.conforms, Value.type, wellFormed_struct, ht, hf, hc]

/-! ## Writable paths

A write goes through every step of its path but a signer's address, which
`Value.withChild?` never replaces (`PathStep.signerAddress`). -/

/-- A step a write may go through. -/
def _root_.MoveBytecode.PathStep.writable : PathStep → Bool
  | .signerAddress => false
  | .field _ | .variantField _ _ | .element _ | .tableEntry _ => true

theorem withChild?_writable {v w v' : Value} {st : PathStep} (h : v.withChild? st w = some v') :
    st.writable = true := by
  cases st
  · rfl
  · rfl
  · rfl
  · cases v <;> simp [Value.withChild?] at h
  · rfl

/-- A path that can be written has writable steps only. -/
theorem set?_writable : ∀ {path : List PathStep} {v w v' : Value}, v.set? path w = some v' →
    path.all PathStep.writable = true
  | [], _, _, _, _ => rfl
  | st :: path, v, w, v', h => by
    simp only [Value.set?, bind, Option.bind_eq_some_iff] at h
    obtain ⟨c, -, c', hc', hv'⟩ := h
    simp only [List.all_cons, withChild?_writable hv', set?_writable hc', Bool.and_self]

theorem write?_writable {s s' : State} {l : Location} {w : Value} (h : s.write? l w = some s') :
    l.path.all PathStep.writable = true := by
  simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨_, -, _, hset, -⟩ := h
  exact set?_writable hset

end MoveBytecode.Proofs
