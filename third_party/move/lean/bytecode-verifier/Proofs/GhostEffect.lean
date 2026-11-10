-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostTrees

/-!
# The effect of a step on occurrences

UNTRUSTED (checked by the kernel). Every annotated step rearranges the
occurrences of the state (identities paired with the values at their nodes)
in the same few ways: some occurrences stay (possibly elsewhere), some are
discarded, some are consumed by an event, some are copied, and some are
created. `Effect` records such a rearrangement, and the lemmas here derive
from it, generically, the facts D2 needs of every step: uniqueness of
identities without `copy` (D2a), no implicit destruction (D2b), provenance of
every occurrence from an event that issued its identity, and conservation
(D2c).

**Keys.** A write through a reference changes a container in place: the
container keeps its identity while its value changes (a struct value with a
new field, a vector with a new element). What D2 and D3 observe of a node is
its type, and for a signer the signer itself, and a write changes neither.
So effects are stated on *keys*: a leaf node (a boolean, an integer, an
address, a signer or a reference) whole, and a container by its type
(`Value.key`). A leaf never changes under its identity: a write replaces the
leaf node, identity and all.

The facts about the state before the step that these lemmas use are the
trace invariant's (`Proofs.Accounting`): every identity was issued, created
or borrowed, by an event, with its key; identities have one key each; and
they are below the fresh counter.
-/

namespace MoveBytecode.Proofs

/-- Occurrences: identities with the values at their nodes. -/
abbrev Occ := List (Nat × Value)

/-! ## Keys -/

/-- What D2 and D3 observe of the value at an identity (module docstring,
"Keys"). -/
inductive Key where
  | leaf (v : Value)
  | node (t : RuntimeType)
  deriving DecidableEq

/-- The key of a value. -/
def _root_.MoveBytecode.Value.key : Value → Key
  | .bool b => .leaf (.bool b)
  | .int t x => .leaf (.int t x)
  | .address a => .leaf (.address a)
  | .signer a => .leaf (.signer a)
  | .ref r => .leaf (.ref r)
  | .vector e _ => .node (.vector e)
  | .struct s _ => .node (.struct s)
  | .variant s _ _ => .node (.struct s)
  | .closure _ _ _ ps rs a => .node (.function ps rs a)
  | .table s _ _ _ => .node (.struct s)

/-- The type of the values with a key. -/
def Key.type : Key → RuntimeType
  | .leaf v => v.type
  | .node t => t

@[simp] theorem key_type (v : Value) : v.key.type = v.type := by cases v <;> rfl

/-- Keyed occurrences. -/
abbrev KOcc := List (Nat × Key)

/-- The keys of occurrences. -/
def keys (occ : Occ) : KOcc := occ.map fun o => (o.1, o.2.key)

@[simp] theorem keys_nil : keys [] = [] := rfl
@[simp] theorem keys_cons (o : Nat × Value) (occ : Occ) :
    keys (o :: occ) = (o.1, o.2.key) :: keys occ := rfl
@[simp] theorem keys_append (a b : Occ) : keys (a ++ b) = keys a ++ keys b := by
  simp [keys]

theorem keys_perm {a b : Occ} (h : a.Perm b) : (keys a).Perm (keys b) := h.map _

theorem mem_keys {occ : Occ} {o : Nat × Value} (h : o ∈ occ) : (o.1, o.2.key) ∈ keys occ :=
  List.mem_map_of_mem h

theorem mem_keys_iff {occ : Occ} {x : Nat × Key} :
    x ∈ keys occ ↔ ∃ o ∈ occ, o.1 = x.1 ∧ o.2.key = x.2 := by
  simp only [keys, List.mem_map]
  constructor
  · rintro ⟨o, ho, rfl⟩; exact ⟨o, ho, rfl, rfl⟩
  · rintro ⟨o, ho, h1, h2⟩; exact ⟨o, ho, by rw [h1, h2]⟩

/-! ## Events -/

/-- The identities and values the events create. -/
def createdOf (evs : List Event) : Occ :=
  evs.filterMap fun
    | .created i w _ => some (i, w)
    | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
    | .frameEnded .. | .privileged .. => none

/-- The identities and values the events consume. -/
def consumedOf (evs : List Event) : Occ :=
  evs.filterMap fun
    | .consumed i w _ => some (i, w)
    | .created .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
    | .frameEnded .. | .privileged .. => none

/-- The identities the events issue, with their keys: created values and
borrowed references. -/
def issuedOf (evs : List Event) : KOcc :=
  evs.filterMap fun
    | .created i w _ => some (i, w.key)
    | .borrowed i r _ => some (i, (Value.ref r).key)
    | .consumed .. | .used .. | .accessed .. | .vacated .. | .entered .. | .frameEnded ..
    | .privileged .. => none

/-- An event that neither issues nor consumes an identity: one of D4's
records, or D3a's record of a privileged operation. -/
def _root_.MoveBytecode.Event.Silent : Event → Bool
  | .used .. | .accessed .. | .vacated .. | .entered .. | .frameEnded .. | .privileged .. => true
  | .created .. | .consumed .. | .borrowed .. => false

@[simp] theorem createdOf_nil : createdOf [] = [] := rfl
@[simp] theorem consumedOf_nil : consumedOf [] = [] := rfl
@[simp] theorem issuedOf_nil : issuedOf [] = [] := rfl

@[simp] theorem createdOf_append (a b : List Event) :
    createdOf (a ++ b) = createdOf a ++ createdOf b := by
  simp [createdOf, List.filterMap_append]

@[simp] theorem consumedOf_append (a b : List Event) :
    consumedOf (a ++ b) = consumedOf a ++ consumedOf b := by
  simp [consumedOf, List.filterMap_append]

@[simp] theorem issuedOf_append (a b : List Event) :
    issuedOf (a ++ b) = issuedOf a ++ issuedOf b := by
  simp [issuedOf, List.filterMap_append]

@[simp] theorem createdOf_cons_created (i : Nat) (w : Value) (c : Creator) (evs : List Event) :
    createdOf (.created i w c :: evs) = (i, w) :: createdOf evs := rfl

@[simp] theorem createdOf_cons_consumed (i : Nat) (w : Value) (c : Consumer) (evs : List Event) :
    createdOf (.consumed i w c :: evs) = createdOf evs := rfl

@[simp] theorem createdOf_cons_borrowed (i : Nat) (r : Reference) (p : Option Nat)
    (evs : List Event) : createdOf (.borrowed i r p :: evs) = createdOf evs := rfl

@[simp] theorem consumedOf_cons_created (i : Nat) (w : Value) (c : Creator) (evs : List Event) :
    consumedOf (.created i w c :: evs) = consumedOf evs := rfl

@[simp] theorem consumedOf_cons_consumed (i : Nat) (w : Value) (c : Consumer) (evs : List Event) :
    consumedOf (.consumed i w c :: evs) = (i, w) :: consumedOf evs := rfl

@[simp] theorem consumedOf_cons_borrowed (i : Nat) (r : Reference) (p : Option Nat)
    (evs : List Event) : consumedOf (.borrowed i r p :: evs) = consumedOf evs := rfl

@[simp] theorem issuedOf_cons_created (i : Nat) (w : Value) (c : Creator) (evs : List Event) :
    issuedOf (.created i w c :: evs) = (i, w.key) :: issuedOf evs := rfl

@[simp] theorem issuedOf_cons_consumed (i : Nat) (w : Value) (c : Consumer) (evs : List Event) :
    issuedOf (.consumed i w c :: evs) = issuedOf evs := rfl

@[simp] theorem issuedOf_cons_borrowed (i : Nat) (r : Reference) (p : Option Nat)
    (evs : List Event) : issuedOf (.borrowed i r p :: evs) = (i, (Value.ref r).key) :: issuedOf evs :=
  rfl

theorem silent_cons {e : Event} (he : e.Silent = true) (evs : List Event) :
    createdOf (e :: evs) = createdOf evs ∧ consumedOf (e :: evs) = consumedOf evs ∧
      issuedOf (e :: evs) = issuedOf evs := by
  cases e <;> simp_all [Event.Silent, createdOf, consumedOf, issuedOf]

/-- Silent events issue, create and consume nothing. -/
theorem silent_of {es : List Event} (h : ∀ e ∈ es, e.Silent = true) :
    createdOf es = [] ∧ consumedOf es = [] ∧ issuedOf es = [] := by
  induction es with
  | nil => simp
  | cons e es ih =>
    obtain ⟨h1, h2, h3⟩ := silent_cons (h e (List.mem_cons_self ..)) es
    obtain ⟨i1, i2, i3⟩ := ih fun e he => h e (List.mem_cons_of_mem _ he)
    exact ⟨h1.trans i1, h2.trans i2, h3.trans i3⟩

/-- The created identities are issued with their keys. -/
theorem keys_created_sub {evs : List Event} : ∀ x ∈ keys (createdOf evs), x ∈ issuedOf evs := by
  induction evs with
  | nil => simp
  | cons e evs ih =>
    intro x hx
    cases e with
    | created i w c =>
      simp only [createdOf_cons_created, keys_cons, List.mem_cons] at hx
      simp only [issuedOf_cons_created, List.mem_cons]
      rcases hx with rfl | hx
      · exact Or.inl rfl
      · exact Or.inr (ih x hx)
    | borrowed i r p =>
      simp only [createdOf_cons_borrowed] at hx
      simp only [issuedOf_cons_borrowed, List.mem_cons]
      exact Or.inr (ih x hx)
    | consumed | used | accessed | vacated | entered | frameEnded | privileged =>
      simp only [createdOf, issuedOf, List.filterMap_cons] at hx ⊢
      exact ih x hx

/-- Without borrows, the issued identities are the created ones. -/
theorem issuedOf_created {evs : List Event} (h : ∀ i r p, Event.borrowed i r p ∉ evs) :
    issuedOf evs = keys (createdOf evs) := by
  induction evs with
  | nil => rfl
  | cons e evs ih =>
    have ih' := ih fun i r p he => h i r p (List.mem_cons_of_mem _ he)
    cases e with
    | created i w c => simp [ih']
    | borrowed i r p => exact absurd (List.mem_cons_self ..) (h i r p)
    | consumed | used | accessed | vacated | entered | frameEnded | privileged =>
      simp only [createdOf, issuedOf, List.filterMap_cons] at ih' ⊢
      exact ih'

theorem mem_createdOf {evs : List Event} {o : Nat × Value} :
    o ∈ createdOf evs ↔ ∃ c, Event.created o.1 o.2 c ∈ evs := by
  induction evs with
  | nil => simp
  | cons e evs ih =>
    cases e with
    | created i w c => simp [ih, exists_or, Prod.ext_iff, eq_comm]
    | consumed i w c => simp [ih]
    | borrowed | used | accessed | vacated | entered | frameEnded | privileged =>
      simp [createdOf] at ih ⊢; exact ih

theorem mem_consumedOf {evs : List Event} {o : Nat × Value} :
    o ∈ consumedOf evs ↔ ∃ c, Event.consumed o.1 o.2 c ∈ evs := by
  induction evs with
  | nil => simp
  | cons e evs ih =>
    cases e with
    | created i w c => simp [ih]
    | consumed i w c => simp [ih, exists_or, Prod.ext_iff, eq_comm]
    | borrowed | used | accessed | vacated | entered | frameEnded | privileged =>
      simp [consumedOf] at ih ⊢; exact ih

/-- An issued identity: created with a value of its key, or a borrowed
reference. -/
theorem mem_issuedOf {evs : List Event} {x : Nat × Key} (h : x ∈ issuedOf evs) :
    (∃ w c, Event.created x.1 w c ∈ evs ∧ w.key = x.2) ∨
      ∃ r p, Event.borrowed x.1 r p ∈ evs ∧ (Value.ref r).key = x.2 := by
  induction evs with
  | nil => simp at h
  | cons e evs ih =>
    cases e with
    | created i w c =>
      simp only [issuedOf_cons_created, List.mem_cons] at h
      rcases h with rfl | h
      · exact Or.inl ⟨w, c, List.mem_cons_self .., rfl⟩
      · rcases ih h with ⟨w', c', hm, hk⟩ | ⟨r, p, hm, hk⟩
        · exact Or.inl ⟨w', c', List.mem_cons_of_mem _ hm, hk⟩
        · exact Or.inr ⟨r, p, List.mem_cons_of_mem _ hm, hk⟩
    | borrowed i r p =>
      simp only [issuedOf_cons_borrowed, List.mem_cons] at h
      rcases h with rfl | h
      · exact Or.inr ⟨r, p, List.mem_cons_self .., rfl⟩
      · rcases ih h with ⟨w', c', hm, hk⟩ | ⟨r', p', hm, hk⟩
        · exact Or.inl ⟨w', c', List.mem_cons_of_mem _ hm, hk⟩
        · exact Or.inr ⟨r', p', List.mem_cons_of_mem _ hm, hk⟩
    | consumed | used | accessed | vacated | entered | frameEnded | privileged =>
      simp only [issuedOf, List.filterMap_cons] at h
      rcases ih h with ⟨w', c', hm, hk⟩ | ⟨r, p, hm, hk⟩
      · exact Or.inl ⟨w', c', List.mem_cons_of_mem _ hm, hk⟩
      · exact Or.inr ⟨r, p, List.mem_cons_of_mem _ hm, hk⟩

/-! ## Counting -/

/-- How many keyed occurrences have identity `i`. -/
def countK (a : KOcc) (i : Nat) : Nat := a.countP (·.1 == i)

theorem count_keys (occ : Occ) (i : Nat) : Ownership.count occ i = countK (keys occ) i := by
  simp [Ownership.count, countK, keys, List.countP_map, Function.comp_def]

theorem countK_append (a b : KOcc) (i : Nat) :
    countK (a ++ b) i = countK a i + countK b i := by
  simp [countK, List.countP_append]

theorem countK_perm {a b : KOcc} (h : a.Perm b) (i : Nat) : countK a i = countK b i :=
  List.Perm.countP_eq _ h

theorem countK_pos {a : KOcc} {x : Nat × Key} (h : x ∈ a) : 0 < countK a x.1 :=
  List.countP_pos_iff.2 ⟨x, h, by simp⟩

theorem countK_eq_zero {a : KOcc} {i : Nat} (h : ∀ x ∈ a, x.1 ≠ i) : countK a i = 0 :=
  List.countP_eq_zero.2 fun x hx => by simpa using h x hx

theorem consumptions_eq (evs : List Event) (i : Nat) :
    Ownership.consumptions evs i = countK (keys (consumedOf evs)) i := by
  rw [← count_keys]
  induction evs with
  | nil => rfl
  | cons e evs ih =>
    cases e <;> simp_all [Ownership.consumptions, Ownership.count, List.countP_cons, consumedOf]

theorem countK_le_one_of_nodup {a : KOcc} (h : (a.map (·.1)).Nodup) (i : Nat) :
    countK a i ≤ 1 := by
  induction a with
  | nil => simp [countK]
  | cons o a ih =>
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at h
    simp only [countK, List.countP_cons] at ih ⊢
    have := ih h.2
    by_cases ho : o.1 = i
    · subst ho
      have : List.countP (fun x => x.1 == o.1) a = 0 :=
        List.countP_eq_zero.2 fun x hx hxe => h.1 ⟨x, hx, by simpa using hxe⟩
      simp [this]
    · simp [ho, this]

theorem count_le_one_of_nodup {a : Occ} (h : (a.map (·.1)).Nodup) (i : Nat) :
    Ownership.count a i ≤ 1 := by
  rw [count_keys]
  exact countK_le_one_of_nodup (by simpa [keys, Function.comp_def] using h) i

/-! ## Facts about the state before a step -/

/-- The facts about a state before a step that the generic lemmas use: every
occurrence's identity was issued with the occurrence's key, by an event of
`evs`; an identity is issued with one key only; issued identities are below
the fresh counter `f`. -/
structure PreFacts (pre : Occ) (evs : List Event) (f : Nat) : Prop where
  provenance : ∀ x ∈ keys pre, x ∈ issuedOf evs
  functional : ∀ x ∈ issuedOf evs, ∀ y ∈ issuedOf evs, x.1 = y.1 → x = y
  below : ∀ x ∈ issuedOf evs, x.1 < f

/-- Two occurrences of one identity have the same key. -/
theorem PreFacts.key {pre : Occ} {evs : List Event} {f : Nat} (hp : PreFacts pre evs f)
    {x y : Nat × Key} (hx : x ∈ keys pre) (hy : y ∈ keys pre) (h : x.1 = y.1) : x = y :=
  hp.functional x (hp.provenance x hx) y (hp.provenance y hy) h

/-! ## Effects -/

/-- The effect of a step on keyed occurrences: those before it (`pre`) are
some kept, some discarded (`dropped`, of types with `drop`), and those the
step's events consume; those after it (`post`) are the kept ones, copies of
kept ones (of types with `copy`), and those the events issue, with fresh
identities from `f` below `f'`. -/
def EffectK (P : Program) (pre post : KOcc) (evs : List Event) (f f' : Nat) : Prop :=
  ∃ keep dropped copies : KOcc,
    pre.Perm (keep ++ dropped ++ keys (consumedOf evs)) ∧
    post.Perm (keep ++ copies ++ issuedOf evs) ∧
    (∀ x ∈ dropped, (x.2.type.abilities P).drop = true) ∧
    (∀ x ∈ copies, x ∈ keep ∧ (x.2.type.abilities P).copy = true) ∧
    (∀ x ∈ issuedOf evs, f ≤ x.1 ∧ x.1 < f') ∧
    ((issuedOf evs).map (·.1)).Nodup ∧ f ≤ f'

/-- The effect of a step on occurrences: its effect on their keys. -/
def Effect (P : Program) (pre post : Occ) (evs : List Event) (f f' : Nat) : Prop :=
  EffectK P (keys pre) (keys post) evs f f'

section Effect

variable {P : Program} {pre post : Occ} {evs evs' : List Event} {f f' : Nat}

theorem Effect.mem_pre (he : Effect P pre post evs' f f') {x : Nat × Key}
    (hx : x ∈ keys (consumedOf evs')) : x ∈ keys pre := by
  obtain ⟨keep, dropped, copies, hpre, -⟩ := he
  exact hpre.symm.subset (List.mem_append_right _ hx)

/-- Every occurrence after the step was there before, or was issued by it. -/
theorem Effect.sub (he : Effect P pre post evs' f f') {x : Nat × Key} (hx : x ∈ keys post) :
    x ∈ keys pre ∨ x ∈ issuedOf evs' := by
  obtain ⟨keep, dropped, copies, hpre, hpost, -, hcopy, -⟩ := he
  have hkeep : ∀ y ∈ keep, y ∈ keys pre := fun y hy =>
    hpre.symm.subset (List.mem_append_left _ (List.mem_append_left _ hy))
  rcases List.mem_append.1 (hpost.subset hx) with hkc | hn
  · rcases List.mem_append.1 hkc with hk | hc
    · exact Or.inl (hkeep x hk)
    · exact Or.inl (hkeep x (hcopy x hc).1)
  · exact Or.inr hn

/-- **D2a after a step.** -/
theorem Effect.unique (he : Effect P pre post evs' f f') (hp : PreFacts pre evs f)
    (hu : Ownership.unique P pre = true) : Ownership.unique P post = true := by
  obtain ⟨keep, dropped, copies, hpre, hpost, hdrop, hcopy, hcr, hnd, hff⟩ := he
  have hkeep : ∀ x ∈ keep, x ∈ keys pre := fun x hx =>
    hpre.symm.subset (List.mem_append_left _ (List.mem_append_left _ hx))
  simp only [Ownership.unique, List.all_eq_true, Bool.or_eq_true, decide_eq_true_eq] at hu ⊢
  intro o ho
  by_cases hc : (o.2.type.abilities P).copy = true
  · exact Or.inl hc
  · right
    have hok : (o.1, o.2.key) ∈ keys post := mem_keys ho
    rw [count_keys, countK_perm hpost, countK_append, countK_append]
    rcases List.mem_append.1 (hpost.subset hok) with hkc | hnew
    · -- An old identity: it occurs once before, no copy has it, nothing new.
      have hopre : (o.1, o.2.key) ∈ keys pre := by
        rcases List.mem_append.1 hkc with hk | hk
        · exact hkeep _ hk
        · exact hkeep _ (hcopy _ hk).1
      have hc0 : countK copies o.1 = 0 := countK_eq_zero fun x hx hxi => by
        have hxo := hp.key (hkeep x (hcopy x hx).1) hopre hxi
        have hxc := (hcopy x hx).2
        rw [hxo] at hxc
        simp only [key_type] at hxc
        exact hc hxc
      have hn0 : countK (issuedOf evs') o.1 = 0 := countK_eq_zero fun x hx hxi => by
        have := (hcr x hx).1
        have := hp.below _ (hp.provenance _ hopre)
        simp only at this
        omega
      obtain ⟨o', ho', h1, h2⟩ := mem_keys_iff.1 hopre
      have h1' : o'.1 = o.1 := h1
      have hc' : (o'.2.type.abilities P).copy = false := by
        rw [← key_type, h2, key_type]; simpa using hc
      have hcnt := (hu o' ho').resolve_left (by simp [hc'])
      rw [count_keys, h1', countK_perm hpre, countK_append, countK_append] at hcnt
      omega
    · -- A new identity: issued once, by this step only.
      have hk0 : countK keep o.1 = 0 := countK_eq_zero fun x hx hxi => by
        have := hp.below x (hp.provenance x (hkeep x hx))
        have := (hcr _ hnew).1
        simp only at this
        omega
      have hc0 : countK copies o.1 = 0 := countK_eq_zero fun x hx hxi => by
        have := hp.below x (hp.provenance x (hkeep x (hcopy x hx).1))
        have := (hcr _ hnew).1
        simp only at this
        omega
      have := countK_le_one_of_nodup hnd o.1
      omega

/-- **D2b's count after a step.** -/
theorem Effect.count (he : Effect P pre post evs' f f') (hp : PreFacts pre evs f) {o : Nat × Value}
    (ho : o ∈ pre) (hd : (o.2.type.abilities P).drop = false) :
    Ownership.count pre o.1 ≤ Ownership.count post o.1 + Ownership.consumptions evs' o.1 := by
  obtain ⟨keep, dropped, copies, hpre, hpost, hdrop, hcopy, -⟩ := he
  have hdropped : ∀ x ∈ dropped, x ∈ keys pre := fun x hx =>
    hpre.symm.subset (List.mem_append_left _ (List.mem_append_right _ hx))
  have hd0 : countK dropped o.1 = 0 := countK_eq_zero fun x hx hxi => by
    have hxo := hp.key (hdropped x hx) (mem_keys ho) hxi
    have hxd := hdrop x hx
    rw [hxo, key_type, hd] at hxd
    exact Bool.false_ne_true hxd
  rw [count_keys, count_keys, countK_perm hpre, countK_perm hpost, consumptions_eq]
  simp only [countK_append]
  omega

theorem Effect.provenance (he : Effect P pre post evs' f f') (hp : PreFacts pre evs f) :
    ∀ x ∈ keys post, x ∈ issuedOf (evs ++ evs') := by
  intro x hx
  rw [issuedOf_append]
  rcases he.sub hx with h | h
  · exact List.mem_append_left _ (hp.provenance x h)
  · exact List.mem_append_right _ h

omit P pre post evs evs' f f' in
/-- In a list whose identities are distinct, an identity determines its
entry. -/
theorem eq_of_fst_nodup {α : Type} : ∀ {l : List (Nat × α)}, (l.map (·.1)).Nodup →
    ∀ {x y : Nat × α}, x ∈ l → y ∈ l → x.1 = y.1 → x = y
  | [], _, _, _, hx, _, _ => by simp at hx
  | a :: l, h, x, y, hx, hy, hxy => by
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at h
    rcases List.mem_cons.1 hx with rfl | hx' <;> rcases List.mem_cons.1 hy with rfl | hy'
    · rfl
    · exact absurd ⟨y, hy', hxy.symm⟩ h.1
    · exact absurd ⟨x, hx', hxy⟩ h.1
    · exact eq_of_fst_nodup h.2 hx' hy' hxy

theorem Effect.functional (he : Effect P pre post evs' f f') (hp : PreFacts pre evs f) :
    ∀ x ∈ issuedOf (evs ++ evs'), ∀ y ∈ issuedOf (evs ++ evs'), x.1 = y.1 → x = y := by
  obtain ⟨-, -, -, -, -, -, -, hcr, hnd, -⟩ := he
  have hnew : ∀ x ∈ issuedOf evs', ∀ y ∈ issuedOf evs', x.1 = y.1 → x = y :=
    fun x hx y hy hxy => eq_of_fst_nodup hnd hx hy hxy
  intro x hx y hy hxy
  rw [issuedOf_append, List.mem_append] at hx hy
  rcases hx with hx | hx <;> rcases hy with hy | hy
  · exact hp.functional x hx y hy hxy
  · have := hp.below x hx; have := (hcr y hy).1; omega
  · have := hp.below y hy; have := (hcr x hx).1; omega
  · exact hnew x hx y hy hxy

theorem Effect.below (he : Effect P pre post evs' f f') (hp : PreFacts pre evs f) :
    ∀ x ∈ issuedOf (evs ++ evs'), x.1 < f' := by
  obtain ⟨-, -, -, -, -, -, -, hcr, -, hff⟩ := he
  intro x hx
  rw [issuedOf_append] at hx
  rcases List.mem_append.1 hx with hx | hx
  · have := hp.below x hx; omega
  · exact (hcr x hx).2

/-- The facts carry over to the state after the step. -/
theorem Effect.preFacts (he : Effect P pre post evs' f f') (hp : PreFacts pre evs f) :
    PreFacts post (evs ++ evs') f' :=
  ⟨he.provenance hp, he.functional hp, he.below hp⟩

/-! ## Building effects -/

theorem EffectK.perm {pre₁ pre₂ post₁ post₂ : KOcc} (he : EffectK P pre₁ post₁ evs' f f')
    (h₁ : pre₂.Perm pre₁) (h₂ : post₂.Perm post₁) : EffectK P pre₂ post₂ evs' f f' := by
  obtain ⟨keep, dropped, copies, hpre, hpost, rest⟩ := he
  exact ⟨keep, dropped, copies, h₁.trans hpre, h₂.trans hpost, rest⟩

theorem Effect.perm {pre₁ pre₂ post₁ post₂ : Occ} (he : Effect P pre₁ post₁ evs' f f')
    (h₁ : pre₂.Perm pre₁) (h₂ : post₂.Perm post₁) :
    Effect P pre₂ post₂ evs' f f' :=
  EffectK.perm he (keys_perm h₁) (keys_perm h₂)

/-- An unchanged part of the state is kept. -/
theorem EffectK.append {pre post : KOcc} (he : EffectK P pre post evs' f f') (r : KOcc) :
    EffectK P (pre ++ r) (post ++ r) evs' f f' := by
  obtain ⟨keep, dropped, copies, hpre, hpost, hdrop, hcopy, rest⟩ := he
  refine ⟨keep ++ r, dropped, copies, ?_, ?_, hdrop,
    fun o ho => ⟨List.mem_append_left _ (hcopy o ho).1, (hcopy o ho).2⟩, rest⟩
  · refine (hpre.append_right r).trans ?_
    simp only [List.append_assoc]
    apply List.Perm.append_left keep
    rw [← List.append_assoc]
    exact List.perm_append_comm
  · refine (hpost.append_right r).trans ?_
    simp only [List.append_assoc]
    apply List.Perm.append_left keep
    rw [← List.append_assoc]
    exact List.perm_append_comm

theorem Effect.append (he : Effect P pre post evs' f f') (r : Occ) :
    Effect P (pre ++ r) (post ++ r) evs' f f' := by
  unfold Effect at he ⊢
  simpa using he.append (keys r)

theorem EffectK.prepend {pre post : KOcc} (he : EffectK P pre post evs' f f') (L : KOcc) :
    EffectK P (L ++ pre) (L ++ post) evs' f f' :=
  (he.append L).perm List.perm_append_comm List.perm_append_comm

theorem Effect.prepend (he : Effect P pre post evs' f f') (L : Occ) :
    Effect P (L ++ pre) (L ++ post) evs' f f' :=
  (he.append L).perm List.perm_append_comm List.perm_append_comm

/-- Nothing happens: every occurrence is kept, in some order. -/
theorem EffectK.keep {pre post : KOcc} (h : post.Perm pre) (hf : f ≤ f') :
    EffectK P pre post [] f f' :=
  ⟨pre, [], [], by simp, by simpa using h, by simp, by simp, by simp, by simp, hf⟩

theorem Effect.keep {pre post : Occ} (h : post.Perm pre) (hf : f ≤ f') :
    Effect P pre post [] f f' :=
  EffectK.keep (keys_perm h) hf

/-- Discarding keyed occurrences of types with `drop`. -/
theorem EffectK.drop {d : KOcc} (hd : ∀ x ∈ d, (x.2.type.abilities P).drop = true)
    (r : KOcc) (hf : f ≤ f') : EffectK P (d ++ r) r [] f f' :=
  ⟨r, d, [], by simpa using List.perm_append_comm, by simp, hd, by simp, by simp, by simp, hf⟩

/-- Discarding occurrences of types with `drop`. -/
theorem Effect.drop {d : Occ} (hd : ∀ o ∈ d, (o.2.type.abilities P).drop = true)
    (r : Occ) (hf : f ≤ f') : Effect P (d ++ r) r [] f f' := by
  unfold Effect
  rw [keys_append]
  refine EffectK.drop (fun x hx => ?_) _ hf
  obtain ⟨o, ho, -, h2⟩ := mem_keys_iff.1 hx
  rw [← h2, key_type]; exact hd o ho

/-- Copying occurrences of types with `copy`. -/
theorem Effect.copy {c : Occ} (hc : ∀ o ∈ c, (o.2.type.abilities P).copy = true)
    (r : Occ) (hcr : ∀ o ∈ c, o ∈ r) (hf : f ≤ f') : Effect P r (c ++ r) [] f f' := by
  refine ⟨keys r, [], keys c, by simp, by simpa using List.perm_append_comm,
    by simp, fun x hx => ?_, by simp, by simp, hf⟩
  obtain ⟨o, ho, h1, h2⟩ := mem_keys_iff.1 hx
  refine ⟨?_, by rw [← h2, key_type]; exact hc o ho⟩
  have : x = (o.1, o.2.key) := by rw [h1, h2]
  rw [this]; exact mem_keys (hcr o ho)

/-- Adding silent events changes no effect. -/
theorem EffectK.silent {pre post : KOcc} {evs es₁ es₂ : List Event}
    (he : EffectK P pre post evs f f') (h₁ : ∀ e ∈ es₁, e.Silent = true)
    (h₂ : ∀ e ∈ es₂, e.Silent = true) : EffectK P pre post (es₁ ++ evs ++ es₂) f f' := by
  obtain ⟨-, c₁, i₁⟩ := silent_of h₁
  obtain ⟨-, c₂, i₂⟩ := silent_of h₂
  obtain ⟨keep, dropped, copies, hpre, hpost, rest⟩ := he
  refine ⟨keep, dropped, copies, ?_, ?_, ?_⟩
  · simpa [c₁, c₂] using hpre
  · simpa [i₁, i₂] using hpost
  · simpa [i₁, i₂] using rest

end Effect

end MoveBytecode.Proofs
