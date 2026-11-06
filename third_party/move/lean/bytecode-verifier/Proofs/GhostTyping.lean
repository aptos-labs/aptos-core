-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostExec
import Proofs.ByValue
import Proofs.NativeSigs
import Proofs.DisjointWrites

/-!
# References designate values of their target types

UNTRUSTED (checked by the kernel). In every state an execution reaches, a
reference whose location resolves designates a value of the reference's
target type (`RefsTyped`). The ghost layer needs this for the vector
updates through references: they keep the vector node's identity, so its
type must not change.

The semantics checks each write against the writing reference's target type,
and creates each reference with the type of what it designates. What makes
this an invariant (`StateTyped`):

- every value in the state is *shaped*: its fields, variant fields and
  elements have their declared types (`shaped`, well-formedness without
  integer ranges, which play no part here);
- a root's value has the root's type (`rootType`): a local its declared
  type, a resource its struct type, a held value its own;
- a reference's target is the type its path leads to from its root's type
  (`pathType`), which does not depend on the value there;
- a reference into a local names a frame that exists or has returned
  (`refsBelow`): frames that do not exist yet never have the identity.

None of this uses the verifier.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-! ## Shaped values -/

mutual
/-- A value whose fields, variant fields and elements have their declared
types, all the way down: `Value.wellFormed` without integer ranges. -/
def shaped (P : Program) : Value → Bool
  | .bool _ => true
  | .int _ _ => true
  | .address _ => true
  | .signer _ => true
  | .vector e elems => shapedAll P elems e
  | .struct s fields =>
    match P.fieldTypes s with
    | some tys => shapedFields P fields tys
    | none => false
  | .variant s tag fields =>
    match P.variantFieldTypes s tag with
    | some tys => shapedFields P fields tys
    | none => false
  | .ref r => r.target.isValue
  | .closure f mask captured ps rs a =>
    match P.closureType? f mask with
    | some (capturedTys, remaining, results) =>
      (ps == remaining && rs == results && a == P.closureValueAbilities f captured) &&
        shapedFields P captured capturedTys
    | none => false
  | .table s fields _ entries =>
    s.isTable &&
      (match P.fieldTypes s with
       | some tys => shapedFields P fields tys
       | none => false) &&
      (entries.isEmpty || P.tablesHoldEntries &&
        match s.typeArgs with
        | [_, v] =>
          shapedAll P entries (tableEntryType v) &&
            ((RuntimeType.struct s).abilities P).subset ((tableEntryType v).abilities P).forContainer
        | _ => false)

def shapedAll (P : Program) : List Value → RuntimeType → Bool
  | [], _ => true
  | v :: vs, t => (v.type.assignable t && shaped P v) && shapedAll P vs t

def shapedFields (P : Program) : List Value → List RuntimeType → Bool
  | [], [] => true
  | v :: vs, t :: ts => (v.type.assignable t && shaped P v) && shapedFields P vs ts
  | [], _ :: _ => false
  | _ :: _, [] => false
end

/-- The type one step inside a value of type `t`. -/
def stepType (P : Program) : RuntimeType → PathStep → Option RuntimeType
  | .struct s, .field i => do (← P.fieldTypes s)[i]?
  | .struct s, .variantField tag i => do (← P.variantFieldTypes s tag)[i]?
  | .vector t, .element _ => some t
  | .signer, .signerAddress => some .address
  | .struct s, .tableEntry _ =>
    match s.isTable, s.typeArgs with
    | true, [_, v] => some (tableEntryType v)
    | _, _ => none
  | .struct _, .element _ => none
  | .struct _, .signerAddress => none
  | .vector _, .signerAddress => none
  | .vector _, .field _ => none
  | .vector _, .variantField _ _ => none
  | .vector _, .tableEntry _ => none
  | .signer, .tableEntry _ => none
  | .bool, _ => none
  | .int _, _ => none
  | .address, _ => none
  | .signer, .field _ => none
  | .signer, .variantField _ _ => none
  | .signer, .element _ => none
  | .ref _ _, _ => none
  | .function .., _ => none

/-- The type at `path` inside a value of type `t`. -/
def pathType (P : Program) (t : RuntimeType) : List PathStep → Option RuntimeType
  | [] => some t
  | step :: path => do pathType P (← stepType P t step) path

section Shaped

variable {P : Program}

theorem shapedAll_iff {vs : List Value} {t : RuntimeType} :
    shapedAll P vs t = true ↔ ∀ v ∈ vs, v.type.assignable t = true ∧ shaped P v = true := by
  induction vs with
  | nil => simp [shapedAll]
  | cons v vs ih => simp [shapedAll, ih, and_assoc]

theorem shapedFields_get {vs : List Value} {ts : List RuntimeType}
    (h : shapedFields P vs ts = true) {i : Nat} {v : Value} (hv : vs[i]? = some v) :
    ∃ t, ts[i]? = some t ∧ v.type.assignable t = true ∧ shaped P v = true := by
  induction vs generalizing ts i with
  | nil => simp at hv
  | cons w ws ih =>
    cases ts with
    | nil => simp [shapedFields] at h
    | cons t ts =>
      simp only [shapedFields, Bool.and_eq_true] at h
      cases i with
      | zero =>
        simp only [List.getElem?_cons_zero, Option.some.injEq] at hv
        subst hv
        exact ⟨t, rfl, h.1.1, h.1.2⟩
      | succ i => simpa using ih h.2 (by simpa using hv)

theorem shapedFields_set {vs : List Value} {ts : List RuntimeType}
    (h : shapedFields P vs ts = true) {i : Nat} {v w : Value} (hv : vs[i]? = some v)
    (hw : shaped P w = true) (hty : ∀ t, ts[i]? = some t → w.type.assignable t = true) :
    shapedFields P (vs.set i w) ts = true := by
  induction vs generalizing ts i with
  | nil => simp at hv
  | cons u us ih =>
    cases ts with
    | nil => simp [shapedFields] at h
    | cons t ts =>
      simp only [shapedFields, Bool.and_eq_true] at h
      cases i with
      | zero =>
        simp only [List.getElem?_cons_zero, Option.some.injEq] at hv
        subst hv
        simp [shapedFields, hty t rfl, hw, h.2]
      | succ i =>
        simp only [List.set_cons_succ, shapedFields, Bool.and_eq_true]
        exact ⟨h.1, ih h.2 (by simpa using hv) (fun t' ht' => hty t' (by simpa using ht'))⟩

theorem shapedAll_set {vs : List Value} {t : RuntimeType} (h : shapedAll P vs t = true) {i : Nat}
    {w : Value} (hw : shaped P w = true) (hty : w.type.assignable t = true) :
    shapedAll P (vs.set i w) t = true := by
  rw [shapedAll_iff] at h ⊢
  intro x hx
  rcases List.mem_or_eq_of_mem_set hx with hx | rfl
  · exact h x hx
  · exact ⟨hty, hw⟩

/-- One step inside a shaped value: the child is shaped, of a type
assignable to the step's type. -/
theorem child_shaped {v c : Value} {step : PathStep} (hv : shaped P v = true)
    (hc : v.child? step = some c) :
    ∃ t, stepType P v.type step = some t ∧ c.type.assignable t = true ∧ shaped P c = true := by
  cases v <;> cases step <;> simp only [Value.child?] at hc <;> try (simp at hc; done)
  case vector.element e es i =>
    simp only [shaped, shapedAll_iff] at hv
    have := hv c (List.mem_of_getElem? hc)
    exact ⟨e, rfl, this.1, this.2⟩
  case signer.signerAddress a =>
    simp only [Option.some.injEq] at hc
    subst hc
    exact ⟨.address, rfl, rfl, rfl⟩
  case struct.field s fs i =>
    simp only [shaped] at hv
    split at hv
    · next tys htys =>
      obtain ⟨t, h1, h2, h3⟩ := shapedFields_get hv hc
      exact ⟨t, by simp [stepType, Value.type, htys, h1], h2, h3⟩
    · simp at hv
  case variant.variantField s tag fs t i =>
    split at hc
    · next ht =>
      subst ht
      simp only [shaped] at hv
      split at hv
      · next tys htys =>
        obtain ⟨t', h1, h2, h3⟩ := shapedFields_get hv hc
        exact ⟨t', by simp [stepType, Value.type, htys, h1], h2, h3⟩
      · simp at hv
    · simp at hc
  case table.field s fs ks es i =>
    simp only [shaped, Bool.and_eq_true] at hv
    obtain ⟨⟨-, hv⟩, -⟩ := hv
    split at hv
    · next tys htys =>
      obtain ⟨t, h1, h2, h3⟩ := shapedFields_get hv hc
      exact ⟨t, by simp [stepType, Value.type, htys, h1], h2, h3⟩
    · simp at hv
  case table.tableEntry s fs ks es i =>
    simp only [shaped, Bool.and_eq_true, Bool.or_eq_true, List.isEmpty_iff] at hv
    obtain ⟨⟨hst, -⟩, hes | ⟨-, hv⟩⟩ := hv
    · subst hes; simp at hc
    · split at hv
      · next k v hargs =>
        simp only [Bool.and_eq_true] at hv
        have := (shapedAll_iff.1 hv.1) c (List.mem_of_getElem? hc)
        exact ⟨tableEntryType v, by simp [stepType, Value.type, hst, hargs], this.1, this.2⟩
      · simp at hv

/-- A value with a child has no subtypes: a type assignable to another is
that type. -/
theorem assignable_of_child {v c : Value} {step : PathStep} (hc : v.child? step = some c)
    {t : RuntimeType} (h : v.type.assignable t = true) : v.type = t := by
  rcases assignable_cases h with ⟨ps, rs, a, a', hv, -, -⟩ | ⟨u, w, hv, -, -⟩ | heq
  · cases v <;> simp_all [Value.type, Value.child?]
  · cases v <;> simp_all [Value.type, Value.child?]
  · exact heq

/-- A path inside a shaped value leads to a shaped value of a type assignable
to the path's type. -/
theorem get_shaped : ∀ (path : List PathStep) {v w : Value}, shaped P v = true →
    v.get? path = some w →
      ∃ t, pathType P v.type path = some t ∧ w.type.assignable t = true ∧ shaped P w = true
  | [], v, w, hv, hw => by
    simp only [Value.get?, Option.some.injEq] at hw
    subst hw
    exact ⟨v.type, rfl, assignable_refl _, hv⟩
  | step :: path, v, w, hv, hw => by
    obtain ⟨c, hc, hw⟩ := get_cons hw
    obtain ⟨t, h1, h2, h3⟩ := child_shaped hv hc
    cases path with
    | nil =>
      simp only [Value.get?, Option.some.injEq] at hw
      subst hw
      exact ⟨t, by simp [pathType, h1], h2, h3⟩
    | cons step' path' =>
      obtain ⟨c', hc', -⟩ := get_cons hw
      have hct := assignable_of_child hc' h2
      obtain ⟨t', h4, h5, h6⟩ := get_shaped (step' :: path') h3 hw
      exact ⟨t', by simp only [pathType, h1, Option.bind_eq_bind, Option.bind_some]; rw [← hct]; exact h4,
        h5, h6⟩

theorem withChild_shaped {v c w v' : Value} {step : PathStep} (hv : shaped P v = true)
    (hc : v.child? step = some c) (hw : shaped P w = true)
    (hty : ∀ t, stepType P v.type step = some t → w.type.assignable t = true)
    (hv' : v.withChild? step w = some v') : shaped P v' = true ∧ v'.type = v.type := by
  cases v <;> cases step <;> simp only [Value.child?] at hc <;> try (simp at hc; done)
  case signer.signerAddress => simp [Value.withChild?] at hv'
  case vector.element e es i =>
    simp only [Value.withChild?] at hv'
    split at hv'
    · simp only [Option.some.injEq] at hv'
      subst hv'
      simp only [shaped] at hv ⊢
      exact ⟨shapedAll_set hv hw (hty e rfl), rfl⟩
    · simp at hv'
  case struct.field s fs i =>
    simp only [Value.withChild?] at hv'
    split at hv'
    · simp only [Option.some.injEq] at hv'
      subst hv'
      simp only [shaped] at hv ⊢
      split at hv
      · next tys htys =>
        simp [shapedFields_set hv hc hw (fun t ht => hty t (by simp [stepType, Value.type, htys, ht])),
          Value.type]
      · simp at hv
    · simp at hv'
  case variant.variantField s tag fs t i =>
    split at hc
    · next ht =>
      subst ht
      simp only [Value.withChild?, true_and] at hv'
      split at hv'
      · simp only [Option.some.injEq] at hv'
        subst hv'
        simp only [shaped] at hv ⊢
        split at hv
        · next tys htys =>
          simp [shapedFields_set hv hc hw (fun t' ht => hty t' (by simp [stepType, Value.type, htys, ht])),
            Value.type]
        · simp at hv
      · simp at hv'
    · simp at hc
  case table.field s fs ks es i =>
    simp only [Value.withChild?] at hv'
    split at hv'
    · simp only [Option.some.injEq] at hv'
      subst hv'
      refine ⟨?_, rfl⟩
      simp only [shaped, Bool.and_eq_true] at hv ⊢
      obtain ⟨⟨hst, hv⟩, hes⟩ := hv
      refine ⟨⟨hst, ?_⟩, hes⟩
      split at hv
      · next tys htys =>
        try simp only [htys]
        exact shapedFields_set hv hc hw (fun t ht => hty t (by simp [stepType, Value.type, htys, ht]))
      · simp at hv
    · simp at hv'
  case table.tableEntry s fs ks es i =>
    simp only [Value.withChild?] at hv'
    split at hv'
    · simp only [Option.some.injEq] at hv'
      subst hv'
      refine ⟨?_, rfl⟩
      simp only [shaped, Bool.and_eq_true, Bool.or_eq_true, List.isEmpty_iff] at hv ⊢
      obtain ⟨⟨hst, hf⟩, hes | ⟨htab, hv⟩⟩ := hv
      · subst hes; simp at hc
      · refine ⟨⟨hst, hf⟩, Or.inr ⟨htab, ?_⟩⟩
        split at hv
        · next k v hargs =>
          try simp only [hargs]
          simp only [Bool.and_eq_true] at hv ⊢
          exact ⟨shapedAll_set hv.1 hw (hty _ (by simp [stepType, Value.type, hst, hargs])), hv.2⟩
        · simp at hv
    · simp at hv'

/-- Writing a shaped value of the old value's type at a path keeps a shaped
value shaped, and of its type. -/
theorem set_shaped : ∀ (path : List PathStep) {v w w' v' : Value}, shaped P v = true →
    v.get? path = some w → shaped P w' = true →
    (∀ t, pathType P v.type path = some t → w'.type.assignable t = true) →
    v.set? path w' = some v' → shaped P v' = true ∧ (path ≠ [] → v'.type = v.type)
  | [], v, w, w', v', hv, hw, hw', hty, hv' => by
    simp only [Value.set?, Option.some.injEq] at hv'
    subst hv'
    exact ⟨hw', fun h => absurd rfl h⟩
  | step :: path, v, w, w', v', hv, hw, hw', hty, hv' => by
    obtain ⟨c, hc, hw⟩ := get_cons hw
    obtain ⟨tc, htc, hca, hcs⟩ := child_shaped hv hc
    simp only [Value.set?, bind, Option.bind_eq_some_iff, hc, Option.bind_some] at hv'
    obtain ⟨c', hc', hv'⟩ := hv'
    cases path with
    | nil =>
      simp only [Value.set?, Option.some.injEq] at hc'
      subst hc'
      refine ⟨(withChild_shaped hv hc hw' (fun t ht => ?_) hv').1, fun _ =>
        (withChild_shaped hv hc hw' (fun t ht => ?_) hv').2⟩ <;>
      · rw [htc, Option.some.injEq] at ht
        subst ht
        exact hty tc (by simp [pathType, htc])
    | cons step' path' =>
      obtain ⟨c₂, hc₂, -⟩ := get_cons hw
      have hct := assignable_of_child hc₂ hca
      obtain ⟨hcs', hcty⟩ := set_shaped (step' :: path') hcs hw hw' (fun t ht => hty t (by
        simp only [pathType, htc, Option.bind_eq_bind, Option.bind_some]; rw [← hct]; exact ht)) hc'
      have hc't : c'.type = c.type := hcty (by simp)
      refine ⟨(withChild_shaped hv hc hcs' (fun t ht => ?_) hv').1, fun _ =>
        (withChild_shaped hv hc hcs' (fun t ht => ?_) hv').2⟩ <;>
      · rw [htc, Option.some.injEq] at ht
        subst ht
        rw [hc't, hct]
        exact assignable_refl _

mutual
theorem wellFormed_shaped : ∀ (v : Value), v.wellFormed P = true → shaped P v = true
  | .bool _, _ | .int _ _, _ | .address _, _ | .signer _, _ => by simp [shaped]
  | .ref r, h => by simpa [shaped, Value.wellFormed] using h
  | .vector e elems, h => by
    simp only [Value.wellFormed] at h
    simp only [shaped]
    exact allConform_shaped elems e h
  | .struct s fields, h => by
    obtain ⟨-, tys, htys, h⟩ := wellFormed_struct.1 h
    simp only [shaped, htys]
    exact conformsAll_shaped fields tys h
  | .variant s tag fields, h => by
    simp only [Value.wellFormed] at h
    simp only [shaped]
    split at h
    · next tys htys => simp only [htys]; exact conformsAll_shaped fields tys h
    · simp at h
  | .closure f mask cs ps rs a, h => by
    simp only [Value.wellFormed] at h
    simp only [shaped]
    split at h
    · next ctys rem rts hct =>
      simp only [hct, Bool.and_eq_true] at h ⊢
      exact ⟨h.1, conformsAll_shaped cs ctys h.2⟩
    · simp at h
  | .table s fs ks es, h => by
    obtain ⟨hst, -, ⟨tys, htys, hc⟩, hes⟩ := wellFormed_table.1 h
    simp only [shaped, hst, htys, conformsAll_shaped fs tys hc, Bool.and_self, Bool.true_and,
      Bool.or_eq_true, List.isEmpty_iff]
    rcases hes with rfl | ⟨htab, k, v, hargs, -, hce, -, -, hsub⟩
    · exact Or.inl rfl
    · refine Or.inr ?_
      simp only [htab, hargs, Bool.true_and, hsub, Bool.and_true]
      exact allConform_shaped es _ hce

theorem allConform_shaped : ∀ (vs : List Value) (t : RuntimeType),
    Value.allConform P vs t = true → shapedAll P vs t = true
  | [], _, _ => by simp [shapedAll]
  | v :: vs, t, h => by
    simp only [Value.allConform, Bool.and_eq_true] at h
    simp only [shapedAll, Bool.and_eq_true]
    exact ⟨⟨h.1.1, wellFormed_shaped v h.1.2⟩, allConform_shaped vs t h.2⟩

theorem conformsAll_shaped : ∀ (vs : List Value) (ts : List RuntimeType),
    conformsAll P vs ts = true → shapedFields P vs ts = true
  | [], [], _ => by simp [shapedFields]
  | v :: vs, t :: ts, h => by
    simp only [conformsAll, Bool.and_eq_true] at h
    simp only [shapedFields, Bool.and_eq_true]
    exact ⟨⟨h.1.1, wellFormed_shaped v h.1.2⟩, conformsAll_shaped vs ts h.2⟩
  | [], _ :: _, h => by simp [conformsAll] at h
  | _ :: _, [], h => by simp [conformsAll] at h
end

theorem conforms_shaped {v : Value} {t : RuntimeType} (h : v.conforms P t = true) :
    shaped P v = true ∧ v.type.assignable t = true := by
  simp only [Value.conforms, Bool.and_eq_true] at h
  exact ⟨wellFormed_shaped v h.2, h.1⟩

end Shaped

/-! ## The typing invariant -/

/-- The type of the value at a root: a local's declared type, a resource's
struct type, a held value's own type. -/
def rootType (P : Program) (s : State) : Root → Option RuntimeType
  | .local_ f i => do (← P.localTypes (← s.frame? f).function)[i]?
  | .global _ t => some (.struct t)
  | .held k => (s.held[k]?).map (·.type)

/-- The values in a frame: its segment and its locals. -/
def frameVals (fr : Frame) : List Value := fr.stack ++ fr.locals.toList.filterMap id

/-- A reference is typed in state `s`: its target is the type its path leads
to from its root's type, a local it names belongs to a frame created before,
and a held value it designates is shaped. (An admissible initial state may
hold values no reference designates, of any shape.)

A reference to a whole held value is the exception to the first: the
transaction lends it at a type its value's type is assignable to, and a write
through it may replace the value by one of another type assignable to that
target, so for function types the two need only both have subtypes
(`rigid`). -/
def RefTyped (P : Program) (s : State) (r : Reference) : Prop :=
  (∀ T, rootType P s r.location.root = some T →
      pathType P T r.location.path = some r.target ∨
        ((∃ k, r.location.root = .held k) ∧ r.location.path = [] ∧ rigid T = false ∧
          rigid r.target = false)) ∧
    (∀ f i, r.location.root = .local_ f i → f < s.nextFrame) ∧
    ∀ k v, r.location.root = .held k → s.held[k]? = some v → shaped P v = true

/-- The typing invariant (module docstring). -/
structure StateTyped (P : Program) (s : State) : Prop where
  valsShaped : ∀ fr ∈ s.frames, ∀ v ∈ frameVals fr, shaped P v = true
  storageShaped : ∀ r ∈ s.storage, shaped P r.value = true
  locals : ∀ fr ∈ s.frames, ∃ tys, P.localTypes fr.function = some tys ∧
    ∀ (i : Nat) (v : Value), fr.locals[i]? = some (some v) →
      ∃ t, tys[i]? = some t ∧ v.type.assignable t = true
  refs : ∀ fr ∈ s.frames, ∀ r, Value.ref r ∈ frameVals fr → RefTyped P s r
  /-- The values the transaction holds are shaped values of value types. -/
  held : ∀ v ∈ s.held, shaped P v = true ∧ v.type.isValue = true
  /-- Every frame runs an instance whose type arguments are value types. -/
  args : ∀ fr ∈ s.frames, ∀ x ∈ fr.function.typeArgs, x.isValue = true

section StateTyped

variable {P : Program}

theorem mem_frameVals_locals {fr : Frame} {i : Nat} {v : Value} (h : fr.locals[i]? = some (some v)) :
    v ∈ frameVals fr := by
  simp only [frameVals, List.mem_append, List.mem_filterMap, id]
  exact Or.inr ⟨some v, by rw [← Array.getElem?_toList] at h; exact List.mem_of_getElem? h, rfl⟩

theorem mem_frameVals_stack {fr : Frame} {v : Value} (h : v ∈ fr.stack) : v ∈ frameVals fr := by
  simp [frameVals, h]

theorem frame_mem {s : State} {f : FrameId} {fr : Frame} (h : s.frame? f = some fr) :
    fr ∈ s.frames :=
  List.mem_of_find?_eq_some h

/-- A shaped reference's target is a value type, so a value of it is no
reference. -/
theorem not_ref_of_type {v : Value} {t : RuntimeType} (hv : v.type = t) (ht : t.isValue = true) :
    ∀ r, v ≠ .ref r := by
  rintro r rfl
  simp [Value.type] at hv
  subst hv
  simp [RuntimeType.isValue] at ht

/-- The value at a path below a root, whose value is of a type assignable to
the root's type, is of a type assignable to the path's type from the root's
type. -/
theorem path_assignable {rv v : Value} {T t : RuntimeType} {path : List PathStep}
    (hrv : shaped P rv = true) (hget : rv.get? path = some v) (hT : rv.type.assignable T = true)
    (hpt : pathType P T path = some t) : v.type.assignable t = true ∧ shaped P v = true := by
  cases path with
  | nil =>
    simp only [Value.get?, Option.some.injEq] at hget
    simp only [pathType, Option.some.injEq] at hpt
    subst hget hpt
    exact ⟨hT, hrv⟩
  | cons step path' =>
    obtain ⟨c, hc, -⟩ := get_cons hget
    have hrt := assignable_of_child hc hT
    obtain ⟨t', h1, h2, h3⟩ := get_shaped (step :: path') hrv hget
    rw [hrt, hpt, Option.some.injEq] at h1
    subst h1
    exact ⟨h2, h3⟩

/-- A type with subtypes has no steps inside. -/
theorem stepType_nonrigid {t : RuntimeType} (h : rigid t = false) (step : PathStep) :
    stepType P t step = none := by
  cases t with
  | ref m w => simp [stepType]
  | function => simp [stepType]
  | _ => simp [rigid] at h

theorem pathType_nonrigid {T t : RuntimeType} (h : rigid T = false) :
    ∀ {path : List PathStep}, pathType P T path = some t → path = [] ∧ t = T
  | [], hp => by simp only [pathType, Option.some.injEq] at hp; exact ⟨rfl, hp.symm⟩
  | step :: _, hp => by simp [pathType, stepType_nonrigid h step] at hp

theorem rigid_assignable {t u : RuntimeType} (h : t.assignable u = true) : rigid t = rigid u := by
  rcases assignable_cases h with ⟨ps, rs, a, a', rfl, rfl, -⟩ | ⟨v, w, rfl, rfl, -⟩ | rfl <;> rfl

/-- A type assignable to another is it, or both have subtypes. -/
theorem eq_or_nonrigid {t u : RuntimeType} (h : t.assignable u = true) :
    t = u ∨ (rigid t = false ∧ rigid u = false) := by
  rcases assignable_cases h with ⟨ps, rs, a, a', rfl, rfl, -⟩ | ⟨v, w, rfl, rfl, -⟩ | rfl
  · exact Or.inr ⟨rfl, rfl⟩
  · exact Or.inr ⟨rfl, rfl⟩
  · exact Or.inl rfl

/-- The value at a root has a type assignable to the root's type, and is
shaped. -/
theorem root_typed {s : State} (ht : StateTyped P s) (hinv : StateInv σ P s) {r : Reference}
    (hrt : RefTyped P s r) {rv : Value} (hrv : s.root? r.location.root = some rv) :
    ∃ T, rootType P s r.location.root = some T ∧ rv.type.assignable T = true ∧
      shaped P rv = true := by
  cases hroot : r.location.root with
  | local_ f i =>
    rw [hroot] at hrv
    simp only [State.root?, bind, Option.bind_eq_some_iff] at hrv
    obtain ⟨fr', hfr', slot, hslot, rfl⟩ := hrv
    obtain ⟨tys, htys, hloc⟩ := ht.locals fr' (frame_mem hfr')
    obtain ⟨t, ht', hta⟩ := hloc i rv hslot
    exact ⟨t, by simp [rootType, hfr', htys, ht'], hta,
      ht.valsShaped fr' (frame_mem hfr') rv (mem_frameVals_locals hslot)⟩
  | global a t =>
    rw [hroot] at hrv
    obtain ⟨k, res, -, hres, rfl, -, hrt⟩ := storage_get hrv
    have hmem := List.mem_of_getElem? hres
    refine ⟨.struct t, by simp [rootType], ?_, ht.storageShaped res hmem⟩
    rw [hinv.typed res hmem, hrt]
    exact assignable_refl _
  | held k =>
    rw [hroot] at hrv
    simp only [State.root?] at hrv
    exact ⟨rv.type, by simp [rootType, hrv], assignable_refl _, hrt.2.2 k rv hroot hrv⟩

/-- What the typing invariant says of the value at a frame reference's
location, valid or not: it is shaped, of a value type, and of the
reference's target type if that has no subtypes. -/
theorem read_typed {s : State} (ht : StateTyped P s) (hinv : StateInv σ P s) {fr : Frame}
    (hfr : fr ∈ s.frames) {r : Reference} (hr : Value.ref r ∈ frameVals fr) {v : Value}
    (hv : s.read? r.location = some v) :
    shaped P v = true ∧ v.type.isValue = true ∧ (rigid r.target = true → v.type = r.target) := by
  simp only [State.read?, bind, Option.bind_eq_some_iff] at hv
  obtain ⟨rv, hrv, hget⟩ := hv
  have hrt := ht.refs fr hfr r hr
  obtain ⟨T, hT, hrvT, hrs⟩ := root_typed ht hinv hrt hrv
  have htv : r.target.isValue = true := by simpa [shaped] using ht.valsShaped fr hfr _ hr
  rcases hrt.1 T hT with hpt | ⟨⟨k, hk⟩, hp, -, hrig⟩
  · obtain ⟨h1, h2⟩ := path_assignable hrs hget hrvT hpt
    exact ⟨h2, by rw [assignable_isValue h1]; exact htv, fun hr => assignable_rigid h1 hr⟩
  · rw [hp] at hget
    simp only [Value.get?, Option.some.injEq] at hget
    subst hget
    rw [hk] at hrv
    simp only [State.root?] at hrv
    exact ⟨hrs, (ht.held rv (List.mem_of_getElem? hrv)).2, fun h => by rw [hrig] at h; cases h⟩

/-- **References designate values of their target types**, for targets
without subtypes. -/
theorem typed_refs {s : State} (ht : StateTyped P s) (hinv : StateInv σ P s) : RefsTyped s := by
  intro fr hfr r hr v hv
  have hr' : Value.ref r ∈ frameVals fr := by
    rcases hr with hr | hr
    · exact mem_frameVals_stack hr
    · simp only [frameVals, List.mem_append, List.mem_filterMap, id]; exact Or.inr ⟨_, hr, rfl⟩
  exact (read_typed ht hinv hfr hr' (deref?_read hv)).2.2

/-- The root types depend on the frames' identities and functions, and on
the held values, only. -/
theorem rootType_congr {s s' : State}
    (hfr : s'.frames.map (fun fr => (fr.id, fr.function)) = s.frames.map (fun fr => (fr.id, fr.function)))
    (hhe : s'.held.map (·.type) = s.held.map (·.type)) : rootType P s' = rootType P s := by
  funext root
  cases root with
  | local_ f i =>
    have key : ∀ s : State, (s.frame? f).map (·.function) =
        ((s.frames.map (fun fr => (fr.id, fr.function))).find? (·.1 == f)).map (·.2) := by
      intro s
      simp [State.frame?, List.find?_map, Function.comp_def, Option.map_map]
    have h1 := key s'
    rw [hfr, ← key s] at h1
    simp only [rootType, bind]
    cases ha : s'.frame? f <;> cases hb : s.frame? f <;> simp_all
  | global a t => rfl
  | held k =>
    simp only [rootType]
    rw [← List.getElem?_map, ← List.getElem?_map, hhe]

theorem refTyped_congr {s s' : State} {r : Reference} (h : RefTyped P s r)
    (hroot : rootType P s' = rootType P s) (hnf : s.nextFrame ≤ s'.nextFrame)
    (hhe : ∀ (k : Nat) (v : Value), s'.held[k]? = some v → s.held[k]? = some v ∨
      shaped P v = true) :
    RefTyped P s' r := by
  refine ⟨fun T hT => h.1 T (by rw [← hroot]; exact hT),
    fun f i hf => Nat.lt_of_lt_of_le (h.2.1 f i hf) hnf, fun k v hk hv => ?_⟩
  rcases hhe k v hv with hv | hv
  · exact h.2.2 k v hk hv
  · exact hv

/-- The held values are the same. -/
theorem held_same {s s' : State} (hhe : s'.held = s.held) :
    ∀ (k : Nat) (v : Value), s'.held[k]? = some v → s.held[k]? = some v ∨ shaped P v = true :=
  fun _ _ hv => Or.inl (hhe ▸ hv)

/-- A step that changes only the current frame keeps the invariant when
every new value in it is shaped and every new reference typed, and every new
local has its declared type. -/
theorem typed_head {s s₂ : State} {fr fr' : Frame} {rest : List Frame} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hs₂ : s₂.frames = fr' :: rest)
    (hst : ∀ r ∈ s₂.storage, shaped P r.value = true)
    (hhe : s₂.held = s.held) (hnf : s₂.nextFrame = s.nextFrame) (hid : fr'.id = fr.id)
    (hfn : fr'.function = fr.function)
    (hvals : ∀ v ∈ frameVals fr', v ∈ frameVals fr ∨
      (shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r))
    (hloc : ∀ (i : Nat) (v : Value), fr'.locals[i]? = some (some v) → fr.locals[i]? = some (some v) ∨
      ∃ tys t, P.localTypes fr.function = some tys ∧ tys[i]? = some t ∧ v.type.assignable t = true) :
    StateTyped P s₂ := by
  have hroot : rootType P s₂ = rootType P s :=
    rootType_congr (by rw [hs₂, hsf]; simp [hid, hfn]) (by rw [hhe])
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
  have hrest : ∀ x ∈ rest, x ∈ s.frames := fun x hx => by rw [hsf]; exact List.mem_cons_of_mem _ hx
  refine ⟨?_, ?_, ?_, ?_, fun v hv => ht.held v (hhe ▸ hv), ?_⟩
  · intro x hx v hv
    rw [hs₂] at hx
    rcases List.mem_cons.1 hx with rfl | hx
    · rcases hvals v hv with hv | hv
      · exact ht.valsShaped fr hfr v hv
      · exact hv.1
    · exact ht.valsShaped x (hrest x hx) v hv
  · exact hst
  · intro x hx
    rw [hs₂] at hx
    rcases List.mem_cons.1 hx with rfl | hx
    · obtain ⟨tys, htys, hl⟩ := ht.locals fr hfr
      refine ⟨tys, by rw [hfn]; exact htys, fun i v hv => ?_⟩
      rcases hloc i v hv with hv | ⟨tys', t, htys', hi, hta⟩
      · exact hl i v hv
      · rw [htys] at htys'; simp only [Option.some.injEq] at htys'; subst htys'; exact ⟨t, hi, hta⟩
    · exact ht.locals x (hrest x hx)
  · intro x hx r hr
    refine refTyped_congr ?_ hroot (by rw [hnf]; exact Nat.le_refl _) (held_same hhe)
    rw [hs₂] at hx
    rcases List.mem_cons.1 hx with rfl | hx
    · rcases hvals _ hr with hr | hr
      · exact ht.refs fr hfr r hr
      · exact hr.2 r rfl
    · exact ht.refs x (hrest x hx) r hr
  · intro x hx
    rw [hs₂] at hx
    rcases List.mem_cons.1 hx with rfl | hx
    · rw [hfn]; exact ht.args fr hfr
    · exact ht.args x (hrest x hx)

end StateTyped

/-! ## Preservation helpers -/

section Helpers

variable {P : Program}

theorem pathType_append (T : RuntimeType) : ∀ (path : List PathStep) (step : PathStep),
    pathType P T (path ++ [step]) = (pathType P T path).bind (stepType P · step)
  | [], step => by simp [pathType]
  | st :: path, step => by
    simp only [List.cons_append, pathType, bind]
    cases stepType P T st with
    | none => rfl
    | some T' => exact pathType_append T' path step

/-- A reference borrowed one step inside a typed reference, to the step's
type, is typed. -/
theorem child_typed {s : State} {r : Reference} (hr : RefTyped P s r) {step : PathStep}
    {t : RuntimeType} (hst : stepType P r.target step = some t) (mutable : Bool) (n : Nat) :
    RefTyped P s ⟨mutable, t, r.location.child step, n⟩ := by
  refine ⟨fun T hT => Or.inl ?_, fun f i hf => hr.2.1 f i hf, fun k v hk hv => hr.2.2 k v hk hv⟩
  simp only [Location.child] at hT ⊢
  rcases hr.1 T hT with h | ⟨-, -, -, hrig⟩
  · rw [pathType_append, h]
    exact hst
  · rw [stepType_nonrigid hrig] at hst
    cases hst

theorem bcsDecode_shaped : ∀ (t : RuntimeType) {d r : List UInt8} {v : Value},
    bcsDecode t d = some (v, r) → shaped P v = true ∧ v.type = t
  | .vector elem, d, r, v, h => by
    simp only [bcsDecode, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨⟨n, d₁⟩, -, ⟨elems, d₂⟩, h2, h⟩ := h
    simp only [Prod.mk.injEq] at h
    obtain ⟨rfl, -⟩ := h
    refine ⟨?_, rfl⟩
    simp only [shaped, shapedAll_iff]
    intro e he
    obtain ⟨d', r', hd'⟩ := decodeSeq_mem h2 e he
    obtain ⟨h1, h2⟩ := bcsDecode_shaped elem hd'
    exact ⟨by rw [h2]; exact assignable_refl _, h1⟩
  | .bool, d, r, v, h => by
    unfold bcsDecode at h
    split at h
    · split at h
      · simp only [Option.some.injEq, Prod.mk.injEq] at h; obtain ⟨rfl, -⟩ := h; simp [shaped, Value.type]
      · split at h
        · simp only [Option.some.injEq, Prod.mk.injEq] at h; obtain ⟨rfl, -⟩ := h
          simp [shaped, Value.type]
        · simp at h
    · simp at h
  | .int it, d, r, v, h => by
    unfold bcsDecode at h
    simp only [] at h
    split at h
    · simp at h
    · simp only [Option.some.injEq, Prod.mk.injEq] at h; obtain ⟨rfl, -⟩ := h; simp [shaped, Value.type]
  | .address, d, r, v, h => by
    unfold bcsDecode at h
    split at h
    · simp at h
    · simp only [Option.some.injEq, Prod.mk.injEq] at h; obtain ⟨rfl, -⟩ := h; simp [shaped, Value.type]
  | .signer, _, _, _, h => by simp [bcsDecode] at h
  | .struct _, _, _, _, h => by simp [bcsDecode] at h
  | .ref _ _, _, _, _, h => by simp [bcsDecode] at h

theorem decodeConstant_shaped {t : RuntimeType} {d : List UInt8} {v : Value}
    (h : decodeConstant t d = some v) : shaped P v = true := by
  unfold decodeConstant at h
  split at h
  · next v' heq => simp only [Option.some.injEq] at h; subst h; exact (bcsDecode_shaped t heq).1
  · simp at h
  · simp at h

/-- A shaped new value that is not a reference. -/
theorem plain_new {s : State} {v : Value} (hv : shaped P v = true) (hnr : ∀ r, v ≠ .ref r) :
    shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r :=
  ⟨hv, fun r hr => absurd hr (hnr r)⟩

theorem prim_new {s : State} {v : Value} (hv : Primitive v = true) :
    shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r := by
  refine ⟨by cases v <;> simp_all [Primitive, shaped], fun r hr => ?_⟩
  subst hr; simp [Primitive] at hv

/-- The values of a state whose current frame is `fr`. -/
theorem typed_frame {s : State} {fr : Frame} {rest : List Frame} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) {v : Value} (hv : v ∈ frameVals fr) :
    shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r := by
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
  exact ⟨ht.valsShaped fr hfr v hv, fun r hr => ht.refs fr hfr r (hr ▸ hv)⟩

theorem frameVals_with {fr : Frame} {st : List Value} {pc : Nat} :
    frameVals { fr with stack := st, pc := pc } = st ++ fr.locals.toList.filterMap id := rfl

/-- Pushing new values after popping `n`: the invariant holds when the new
values are shaped and typed. -/
theorem typed_push {s s₂ : State} {fr : Frame} {rest : List Frame} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) {n : Nat} {new : List Value}
    (hnew : ∀ v ∈ new, shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r) {pc : Nat}
    (hs₂ : s₂.frames = { fr with stack := new ++ fr.stack.drop n, pc := pc } :: rest)
    (hst : s₂.storage = s.storage) (hhe : s₂.held = s.held) (hnf : s₂.nextFrame = s.nextFrame) :
    StateTyped P s₂ := by
  refine typed_head ht hsf hs₂ (by rw [hst]; exact ht.storageShaped) hhe hnf rfl rfl (fun v hv => ?_)
    (fun i v hv => Or.inl hv)
  rw [frameVals_with] at hv
  simp only [List.mem_append] at hv
  rcases hv with (hv | hv) | hv
  · exact Or.inr (hnew v hv)
  · exact Or.inl (mem_frameVals_stack (List.mem_of_mem_drop hv))
  · exact Or.inl (by simp only [frameVals, List.mem_append]; exact Or.inr hv)

/-- Recording an invalidation keeps the invariant. -/
theorem typed_invalidate {s : State} (ht : StateTyped P s) (e : Invalidation) : StateTyped P (s.invalidate e) :=
  ⟨ht.valsShaped, ht.storageShaped, ht.locals, fun fr hfr r hr => ht.refs fr hfr r hr, ht.held,
    ht.args⟩

/-- The value at a typed reference's valid location: shaped, of the target
type. -/
theorem deref_shaped {s : State} (ht : StateTyped P s) (hinv : StateInv σ P s) {fr : Frame}
    (hfr : fr ∈ s.frames) {r : Reference} (hr : Value.ref r ∈ fr.stack) {v : Value}
    (hv : s.deref? r.location r.since = some v) :
    shaped P v = true ∧ v.type.isValue = true ∧ (rigid r.target = true → v.type = r.target) :=
  read_typed ht hinv hfr (mem_frameVals_stack hr) (deref?_read hv)

/-- The same, for a target without subtypes. -/
theorem deref_shaped_rigid {s : State} (ht : StateTyped P s) (hinv : StateInv σ P s) {fr : Frame}
    (hfr : fr ∈ s.frames) {r : Reference} (hr : Value.ref r ∈ fr.stack) {v : Value}
    (hv : s.deref? r.location r.since = some v) (hrig : rigid r.target = true) :
    shaped P v = true ∧ v.type = r.target :=
  ⟨(deref_shaped ht hinv hfr hr hv).1, (deref_shaped ht hinv hfr hr hv).2.2 hrig⟩

/-- Writing a shaped value, not a reference, of a type assignable to the
root's type at a root keeps the invariant, if a held value keeps its type. -/
theorem typed_setRoot {s : State} (ht : StateTyped P s) (hids : (s.frames.map (·.id)).Nodup)
    (hres : (s.storage.map fun r => (r.address, r.type)).Nodup) {root : Root} {v v'' : Value}
    (hv : s.root? root = some v) (hsh : shaped P v'' = true)
    (hty : ∀ T, rootType P s root = some T → v''.type.assignable T = true)
    (hheld : ∀ k, root = .held k → v''.type = v.type)
    (hnr : ∀ r, v'' ≠ .ref r) : StateTyped P (s.setRoot root v'') := by
  have hroot : rootType P (s.setRoot root v'') = rootType P s := by
    apply rootType_congr
    · cases root <;> simp [State.setRoot, Function.comp_def]
      intro a _; split <;> exact ⟨rfl, rfl⟩
    · cases root with
      | local_ f i => rfl
      | global a t => rfl
      | held k =>
        simp only [State.root?] at hv
        simp only [State.setRoot, List.map_set]
        obtain ⟨hk, -⟩ := List.getElem?_eq_some_iff.1 hv
        apply List.ext_getElem?
        intro j
        rw [List.getElem?_set]
        split
        · next hkj => subst hkj; simp [hheld _ rfl, hk, (List.getElem?_eq_some_iff.1 hv).2]
        · rfl
  cases root with
  | local_ f i =>
    simp only [State.root?, bind, Option.bind_eq_some_iff] at hv
    obtain ⟨fr0, hfr0, slot, hslot, rfl⟩ := hv
    have hfr0' : s.frames.find? (·.id == f) = some fr0 := hfr0
    have hpf : (fr0.id == f) = true := List.find?_some (p := fun (x : Frame) => x.id == f) hfr0'
    obtain ⟨k, -, hk⟩ := find_findIdx hfr0'
    have hmap : s.frames.map (fun fr' => if (fr'.id == f) = true then
          { fr' with locals := fr'.locals.setIfInBounds i (some v'') } else fr') =
        s.frames.set k { fr0 with locals := fr0.locals.setIfInBounds i (some v'') } := by
      refine map_ite_set (p := fun fr' => fr'.id == f) hk hpf fun j y hy hpy => ?_
      exact key_unique hids hy hk (by simp at hpf hpy; rw [hpy, hpf])
    have hfr0m := frame_mem hfr0
    have hmem : ∀ x ∈ (s.setRoot (.local_ f i) v'').frames,
        x ∈ s.frames ∨ x = { fr0 with locals := fr0.locals.setIfInBounds i (some v'') } := by
      intro x hx
      simp only [State.setRoot, hmap] at hx
      rcases List.mem_or_eq_of_mem_set hx with hx | rfl
      · exact Or.inl hx
      · exact Or.inr rfl
    have hvals : ∀ w ∈ frameVals { fr0 with locals := fr0.locals.setIfInBounds i (some v'') },
        w ∈ frameVals fr0 ∨ w = v'' := by
      intro w hw
      simp only [frameVals, List.mem_append, List.mem_filterMap, id, Array.toList_setIfInBounds] at hw
      rcases hw with hw | ⟨o, ho, rfl⟩
      · exact Or.inl (mem_frameVals_stack hw)
      · rcases List.mem_or_eq_of_mem_set ho with ho | ho
        · exact Or.inl (by simp only [frameVals, List.mem_append, List.mem_filterMap, id]; exact Or.inr ⟨_, ho, rfl⟩)
        · simp only [Option.some.injEq] at ho; exact Or.inr ho
    refine ⟨?_, ht.storageShaped, ?_, ?_, ht.held, ?_⟩
    rotate_right
    · intro x hx
      rcases hmem x hx with hx | rfl
      · exact ht.args x hx
      · exact ht.args fr0 hfr0m
    · intro x hx w hw
      rcases hmem x hx with hx | rfl
      · exact ht.valsShaped x hx w hw
      · rcases hvals w hw with hw | rfl
        · exact ht.valsShaped fr0 hfr0m w hw
        · exact hsh
    · intro x hx
      rcases hmem x hx with hx | rfl
      · exact ht.locals x hx
      · obtain ⟨tys, htys, hl⟩ := ht.locals fr0 hfr0m
        refine ⟨tys, htys, fun j w hw => ?_⟩
        have hw' : (fr0.locals.setIfInBounds i (some v''))[j]? = some (some w) := hw
        rw [Array.getElem?_setIfInBounds] at hw'
        split at hw'
        · next hij =>
          subst hij
          split at hw'
          · simp only [Option.some.injEq] at hw'
            obtain ⟨t, hti, -⟩ := hl i v hslot
            rw [← hw']
            exact ⟨t, hti, hty t (by simp [rootType, hfr0, htys, hti])⟩
          · simp at hw'
        · exact hl j w hw'
    · intro x hx r hr
      refine refTyped_congr ?_ hroot (Nat.le_refl _) (held_same rfl)
      rcases hmem x hx with hx | rfl
      · exact ht.refs x hx r hr
      · rcases hvals _ hr with hr | hr
        · exact ht.refs fr0 hfr0m r hr
        · exact absurd hr.symm (hnr r)
  | global a t =>
    obtain ⟨k, res, -, hresk, rfl, hra, hrt⟩ := storage_get hv
    have hpf : (res.address == a && res.type == t) = true := by simp [hra, hrt]
    have hmap : s.storage.update a t v'' = s.storage.set k { res with value := v'' } := by
      unfold GlobalStorage.update
      refine map_ite_set (p := fun r => r.address == a && r.type == t) hresk hpf
        fun j y hy hpy => ?_
      refine key_unique hres hy hresk ?_
      simp only [Bool.and_eq_true, beq_iff_eq] at hpf hpy
      rw [hpy.1, hpy.2, hpf.1, hpf.2]
    refine ⟨ht.valsShaped, ?_, ht.locals, fun fr hfr r hr =>
      refTyped_congr (ht.refs fr hfr r hr) hroot (Nat.le_refl _) (held_same rfl), ht.held, ht.args⟩
    intro r hr
    simp only [State.setRoot, hmap] at hr
    rcases List.mem_or_eq_of_mem_set hr with hr | rfl
    · exact ht.storageShaped r hr
    · exact hsh
  | held k =>
    refine ⟨ht.valsShaped, ht.storageShaped, ht.locals, fun fr hfr r hr =>
      refTyped_congr (ht.refs fr hfr r hr) hroot (Nat.le_refl _) fun j w hw => ?_, ?_, ht.args⟩
    rotate_left
    · intro w hw
      simp only [State.setRoot] at hw
      rcases List.mem_or_eq_of_mem_set hw with hw | rfl
      · exact ht.held w hw
      · have hold := ht.held v (List.mem_of_getElem? hv)
        exact ⟨hsh, by rw [hheld k rfl]; exact hold.2⟩
    simp only [State.setRoot, List.getElem?_set] at hw
    split at hw
    · next hkj =>
      split at hw
      · simp only [Option.some.injEq] at hw; subst hw; exact Or.inr hsh
      · simp at hw
    · exact Or.inl hw

/-- Replacing a held value of a type with subtypes by a shaped value of a
value type with subtypes keeps the invariant: the references to it are to the
whole value, at types with subtypes. -/
theorem typed_setHeld {s : State} (ht : StateTyped P s) {k : Nat} {v v'' : Value}
    (hv : s.held[k]? = some v) (hsh : shaped P v'' = true) (hval : v''.type.isValue = true)
    (hrig : rigid v.type = false) (hrig'' : rigid v''.type = false) :
    StateTyped P (s.setRoot (.held k) v'') := by
  have hk : k < s.held.length := (List.getElem?_eq_some_iff.1 hv).1
  have hthis : rootType P (s.setRoot (.held k) v'') (.held k) = some v''.type := by
    simp [rootType, State.setRoot, hk]
  have hother : ∀ root, (∀ j, root = .held j → j ≠ k) →
      rootType P (s.setRoot (.held k) v'') root = rootType P s root := by
    intro root hr
    cases root with
    | local_ f i => rfl
    | global a t => rfl
    | held j =>
      have hjk := hr j rfl
      simp only [rootType, State.setRoot]
      rw [List.getElem?_set_ne (Ne.symm hjk)]
  refine ⟨ht.valsShaped, ht.storageShaped, ht.locals, fun fr hfr r hr => ?_, ?_, ht.args⟩
  · obtain ⟨h1, h2, h3⟩ := ht.refs fr hfr r hr
    refine ⟨fun T hT => ?_, h2, fun j w hj hw => ?_⟩
    · by_cases hk : r.location.root = .held k
      · rw [hk, hthis, Option.some.injEq] at hT
        subst hT
        have hold : rootType P s r.location.root = some v.type := by rw [hk]; simp [rootType, hv]
        right
        rcases h1 _ hold with hpt | ⟨-, hp, -, hr'⟩
        · obtain ⟨hp, ht'⟩ := pathType_nonrigid hrig hpt
          exact ⟨⟨k, hk⟩, hp, hrig'', by rw [ht']; exact hrig⟩
        · exact ⟨⟨k, hk⟩, hp, hrig'', hr'⟩
      · rw [hother _ (fun j hj => by rintro rfl; exact hk hj)] at hT
        exact h1 T hT
    · simp only [State.setRoot] at hw
      by_cases hjk : k = j
      · subst hjk
        rw [List.getElem?_set_self hk, Option.some.injEq] at hw
        subst hw; exact hsh
      · rw [List.getElem?_set_ne hjk] at hw
        exact h3 j w hj hw
  · intro w hw
    simp only [State.setRoot] at hw
    rcases List.mem_or_eq_of_mem_set hw with hw | rfl
    · exact ht.held w hw
    · exact ⟨hsh, hval⟩

/-- Pushing new values onto the current frame of `s₂` and advancing. -/
theorem typed_finish {s₂ s₃ : State} {fr₂ : Frame} {rest₂ : List Frame} (ht : StateTyped P s₂)
    (hs₂ : s₂.frames = fr₂ :: rest₂) {vals : List Value}
    (hvals : ∀ v ∈ vals, shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s₂ r)
    (hs₃ : s₃.frames = advance (pushAll fr₂ vals) :: rest₂) (hst : s₃.storage = s₂.storage)
    (hhe : s₃.held = s₂.held) (hnf : s₃.nextFrame = s₂.nextFrame) : StateTyped P s₃ :=
  typed_push (n := 0) (new := vals.reverse) (pc := fr₂.pc + 1) ht hs₂
    (fun v hv => hvals v (List.mem_reverse.1 hv)) (by rw [hs₃]; simp [advance, pushAll]) hst hhe hnf

end Helpers

/-! ## Every step keeps the invariant -/

/-- The invariant after a step. -/
def PostTyped (P : Program) : Outcome → Prop
  | .running s => StateTyped P s
  | .returned _ _ => True
  | .failed _ => True
  | .wrong _ => True

section Steps

variable {P : Program} {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {o : Outcome}

/-- A step that replaces the current frame by `fr'`, whose segment is new
values over the old segment without its top `n`. -/
theorem typed_next {fr' : Frame} {n : Nat} {new : List Value} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest)
    (hnew : ∀ v ∈ new, shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r)
    (hstk : fr'.stack = new ++ fr.stack.drop n) (hid : fr'.id = fr.id)
    (hfn : fr'.function = fr.function) (hloc : fr'.locals = fr.locals)
    {s₂ : State} (hst : ∀ r ∈ s₂.storage, shaped P r.value = true) (hhe : s₂.held = s.held)
    (hnf : s₂.nextFrame = s.nextFrame) :
    StateTyped P { s₂ with frames := fr' :: rest } := by
  refine typed_head ht hsf rfl hst hhe hnf hid hfn (fun v hv => ?_) (fun i v hv => by
    rw [hloc] at hv; exact Or.inl hv)
  simp only [frameVals, hstk, hloc, List.mem_append] at hv
  rcases hv with (hv | hv) | hv
  · exact Or.inr (hnew v hv)
  · exact Or.inl (mem_frameVals_stack (List.mem_of_mem_drop hv))
  · exact Or.inl (by simp only [frameVals, List.mem_append]; exact Or.inr hv)

theorem typed_replaces {fr' : Frame} {n : Nat} {v : Value} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hR : Replaces fr fr' n v)
    (hv : shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r) :
    PostTyped P (.running { s with frames := fr' :: rest }) := by
  obtain ⟨-, rfl⟩ := hR
  exact typed_next (fr' := advance (push { fr with stack := fr.stack.drop n } v)) (new := [v]) (n := n)
    ht hsf (by simpa using hv) (by simp [advance, push]) rfl rfl rfl ht.storageShaped rfl rfl


/-- A step that pushes `v`, after popping `n`, with the state `s₂` beside the
frames. -/
theorem typed_push1 {fr' : Frame} {n : Nat} {v : Value} {s₂ : State} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest)
    (hv : shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r)
    (hfr' : fr' = { fr with stack := v :: fr.stack.drop n, pc := fr.pc + 1 })
    (hst : ∀ r ∈ s₂.storage, shaped P r.value = true) (hhe : s₂.held = s.held)
    (hnf : s₂.nextFrame = s.nextFrame) :
    PostTyped P (.running { s₂ with frames := fr' :: rest }) := by
  subst hfr'
  exact typed_next (fr' := { fr with stack := v :: fr.stack.drop n, pc := fr.pc + 1 }) (new := [v])
    (n := n) ht hsf (by simpa using hv) rfl rfl rfl rfl hst hhe hnf

/-- A step that pops `n` and moves the program counter. -/
theorem typed_pop1 {fr' : Frame} {n : Nat} {s₂ : State} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) {pc : Nat}
    (hfr' : fr' = { fr with stack := fr.stack.drop n, pc := pc })
    (hst : ∀ r ∈ s₂.storage, shaped P r.value = true) (hhe : s₂.held = s.held)
    (hnf : s₂.nextFrame = s.nextFrame) :
    PostTyped P (.running { s₂ with frames := fr' :: rest }) := by
  subst hfr'
  exact typed_next (fr' := { fr with stack := fr.stack.drop n, pc := pc }) (new := []) (n := n) ht hsf
    (by simp) rfl rfl rfl rfl hst hhe hnf

theorem typed_ref_frame {r : Reference} (ht : StateTyped P s) (hsf : s.frames = fr :: rest)
    (hr : Value.ref r ∈ fr.stack) : shaped P (.ref r) = true ∧ RefTyped P s r :=
  ⟨(typed_frame ht hsf (mem_frameVals_stack hr)).1,
    (typed_frame ht hsf (mem_frameVals_stack hr)).2 r rfl⟩

theorem fields_new {fields : List Value} {tys : List RuntimeType}
    (hsh : shapedFields P fields tys = true) (hval : ∀ t ∈ tys, t.isValue = true) :
    ∀ v ∈ fields.reverse, shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P s r := by
  intro v hv
  rw [List.mem_reverse, List.mem_iff_getElem?] at hv
  obtain ⟨i, hi⟩ := hv
  obtain ⟨t, h1, h2, h3⟩ := shapedFields_get hsh hi
  exact plain_new h3 (not_ref_of_type rfl (by
    rw [assignable_isValue h2]; exact hval _ (List.mem_of_getElem? h1)))

theorem typed_unpackFields {c : Value} {st fs : List Value} {tys : List RuntimeType}
    (ht : StateTyped P s) (hsf : s.frames = fr :: rest) (hst : fr.stack = c :: st)
    (hch : shaped P c = true → shapedFields P fs tys = true)
    (hval : ∀ t ∈ tys, t.isValue = true) :
    PostTyped P (.running { s with frames := advance (pushAll { fr with stack := st } fs) :: rest }) := by
  have hc := (typed_frame ht hsf (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self ..))).1
  exact typed_next (fr' := advance (pushAll { fr with stack := st } fs))
    (new := fs.reverse) (n := 1) ht hsf (fields_new (hch hc) hval)
    (by simp [advance, pushAll, hst]) rfl rfl rfl ht.storageShaped rfl rfl

theorem typed_unpack {c : Value} {st : List Value} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hst : fr.stack = c :: st)
    (hch : shaped P c = true → shapedFields P c.children tys = true)
    (hval : ∀ t ∈ tys, t.isValue = true) :
    PostTyped P (.running { s with frames := advance (pushAll { fr with stack := st } c.children) :: rest }) :=
  typed_unpackFields ht hsf hst hch hval

/-! ### Helpers for references, writes, calls and returns -/

theorem root_shaped {s : State} (ht : StateTyped P s) {root : Root} {rv : Value}
    (h : s.root? root = some rv) (hheld : ∀ k, root = .held k → shaped P rv = true) :
    shaped P rv = true := by
  cases root with
  | local_ f i =>
    simp only [State.root?, bind, Option.bind_eq_some_iff] at h
    obtain ⟨fr', hfr', slot, hslot, rfl⟩ := h
    exact ht.valsShaped fr' (frame_mem hfr') rv (mem_frameVals_locals hslot)
  | global a t =>
    obtain ⟨k, res, -, hres, rfl, -, -⟩ := storage_get h
    exact ht.storageShaped res (List.mem_of_getElem? hres)
  | held k => exact hheld k rfl

theorem get_not_ref {rv w : Value} : ∀ {path : List PathStep}, rv.get? path = some w →
    w.type.isValue = true → ∀ r, rv ≠ .ref r
  | [], h, hw, r, hr => by
    simp only [Value.get?, Option.some.injEq] at h
    subst h hr; simp [Value.type, RuntimeType.isValue] at hw
  | step :: path, h, hw, r, hr => by
    obtain ⟨c, hc, -⟩ := get_cons h
    subst hr; cases step <;> simp [Value.child?] at hc

theorem not_ref_of_type_eq {v w : Value} (h : v.type = w.type) (hw : ∀ r, w ≠ .ref r) :
    ∀ r, v ≠ .ref r := by
  rintro r rfl
  cases w <;> simp_all [Value.type]

theorem child_not_ref {v c : Value} {step : PathStep} (hc : v.child? step = some c) :
    ∀ r, v ≠ .ref r := by
  rintro r rfl
  cases step <;> simp [Value.child?] at hc

/-- Writing through a typed reference, to a value type, a shaped value of a
type assignable to its target keeps the invariant. -/
theorem typed_write {s₁ s₂ : State} (ht : StateTyped P s₁) (hinv : StateInv σ P s₁)
    {r : Reference} (hrt : RefTyped P s₁ r) (htv : r.target.isValue = true) {w w' : Value}
    (hread : s₁.read? r.location = some w) (hw' : shaped P w' = true)
    (hty : w'.type.assignable r.target = true) (hwr : s₁.write? r.location w' = some s₂) :
    StateTyped P s₂ := by
  simp only [State.read?, bind, Option.bind_eq_some_iff] at hread
  obtain ⟨rv, hrv, hget⟩ := hread
  simp only [State.write?, bind, Option.bind_eq_some_iff, hrv, Option.bind_some, pure,
    Option.some.injEq] at hwr
  obtain ⟨v'', hset, rfl⟩ := hwr
  obtain ⟨T, hT, hrvT, hrs⟩ := root_typed ht hinv hrt hrv
  have hids := hinv.frameIds
  have hres := hinv.resources
  have hw'v : w'.type.isValue = true := by rw [assignable_isValue hty]; exact htv
  cases hpath : r.location.path with
  | cons step path' =>
    rw [hpath] at hget hset
    obtain ⟨c, hc, -⟩ := get_cons hget
    have hrvT' := assignable_of_child hc hrvT
    have hpt : pathType P T (step :: path') = some r.target := by
      rcases hrt.1 T hT with h | ⟨-, hp, -⟩
      · rw [← hpath]; exact h
      · rw [hpath] at hp; cases hp
    obtain ⟨hs'', hty''⟩ := set_shaped (step :: path') hrs hget hw' (fun t ht' => by
      rw [hrvT', hpt, Option.some.injEq] at ht'
      subst ht'
      exact hty) hset
    have heq := hty'' (by simp)
    exact typed_setRoot ht hids hres hrv hs'' (fun T' hT' => by
        rw [hT, Option.some.injEq] at hT'
        subst hT'
        rw [heq]; exact hrvT) (fun _ _ => heq)
      (not_ref_of_type_eq heq (child_not_ref hc))
  | nil =>
    rw [hpath] at hget hset
    simp only [Value.get?, Option.some.injEq] at hget
    simp only [Value.set?, Option.some.injEq] at hset
    subst w v''
    have hnr : ∀ r', w' ≠ .ref r' := not_ref_of_type rfl hw'v
    cases hroot : r.location.root with
    | held k =>
      rw [hroot] at hrv hT
      simp only [State.root?] at hrv
      simp only [rootType, hrv, Option.map_some, Option.some.injEq] at hT
      subst hT
      cases hrig : rigid r.target with
      | true =>
        have hTt : rv.type = r.target := by
          rcases hrt.1 rv.type (by rw [hroot]; simp [rootType, hrv]) with h | ⟨-, -, -, h⟩
          · rw [hpath] at h; simp only [pathType, Option.some.injEq] at h; exact h
          · rw [hrig] at h; cases h
        have heq : w'.type = rv.type := by rw [hTt]; exact assignable_rigid hty hrig
        exact typed_setRoot ht hids hres hrv hw'
          (fun T' hT' => by
            simp only [rootType, hrv, Option.map_some, Option.some.injEq] at hT'
            subst hT'; rw [heq]; exact assignable_refl _)
          (fun _ _ => heq) hnr
      | false =>
        have hrigT : rigid rv.type = false := by
          rcases hrt.1 rv.type (by rw [hroot]; simp [rootType, hrv]) with h | ⟨-, -, h, -⟩
          · rw [hpath] at h; simp only [pathType, Option.some.injEq] at h; rw [h]; exact hrig
          · exact h
        exact typed_setHeld ht hrv hw' hw'v hrigT (by rw [rigid_assignable hty]; exact hrig)
    | local_ f i =>
      have hTt : T = r.target := by
        rcases hrt.1 T hT with h | ⟨⟨k, hk⟩, -⟩
        · rw [hpath] at h; simp only [pathType, Option.some.injEq] at h; exact h
        · rw [hroot] at hk; cases hk
      exact typed_setRoot ht hids hres (by rw [← hroot]; exact hrv) hw' (fun T' hT' => by
          rw [← hroot, hT, Option.some.injEq] at hT'
          subst hT'; rw [hTt]; exact hty) (fun k hk => by cases hk) hnr
    | global a t =>
      have hTt : T = r.target := by
        rcases hrt.1 T hT with h | ⟨⟨k, hk⟩, -⟩
        · rw [hpath] at h; simp only [pathType, Option.some.injEq] at h; exact h
        · rw [hroot] at hk; cases hk
      exact typed_setRoot ht hids hres (by rw [← hroot]; exact hrv) hw' (fun T' hT' => by
          rw [← hroot, hT, Option.some.injEq] at hT'
          subst hT'; rw [hTt]; exact hty) (fun k hk => by cases hk) hnr

theorem frame_head {s : State} {fr : Frame} {rest : List Frame} (hsf : s.frames = fr :: rest) :
    s.frame? fr.id = some fr := by
  simp [State.frame?, hsf]

/-- A reference to a local of the current frame, borrowed whole. -/
theorem local_typed {s : State} {fr : Frame} {rest : List Frame}
    (hinv : StateInv σ P s) (hsf : s.frames = fr :: rest) {i : Nat} {tys : List RuntimeType}
    {t : RuntimeType} (htys : P.localTypes fr.function = some tys) (hi : tys[i]? = some t)
    (mutable : Bool) (n : Nat) :
    RefTyped P s ⟨mutable, t, ⟨.local_ fr.id i, []⟩, n⟩ := by
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
  refine ⟨fun T hT => Or.inl ?_, fun f j hf => ?_, fun k w hk => by simp at hk⟩
  · simp only [rootType, frame_head hsf, htys, hi, bind, Option.bind_some,
      Option.some.injEq] at hT
    subst hT; rfl
  · simp only [Root.local_.injEq] at hf
    obtain ⟨rfl, -⟩ := hf
    exact hinv.framesBelow fr hfr


/-! ### Steps through references and global storage -/

theorem stack_top_ref {r : Reference} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) {k : Nat} (hk : fr.stack[k]? = some (.ref r)) :
    shaped P (.ref r) = true ∧ RefTyped P s r :=
  typed_ref_frame ht hsf (List.mem_of_getElem? hk)

theorem typed_borrowLoc {mutable : Bool} {i : LocalIndex} (ht : StateTyped P s) (hinv : StateInv σ P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (borrowLoc P s fr rest mutable i) = .done o) : PostTyped P o := by
  unfold borrowLoc at h
  obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h hcont).elim
  | some v =>
    dsimp only at h
    obtain ⟨tys, htys, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, hti, h⟩ := done_bind h hcont (ends_need _ _)
    split at h
    · exact (done_wrong h hcont).elim
    · next hval =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hval
      rw [done_next h]
      exact typed_push1 (n := 0) (v := .ref ⟨mutable, t, ⟨.local_ fr.id i, []⟩, s.invalidations.length⟩)
        ht hsf ⟨by simp [shaped, hval], fun r hr => by
          cases hr; exact local_typed hinv hsf (need_ok' htys) (need_ok' hti) _ _⟩
        (by simp [advance, push, borrow]) ht.storageShaped rfl rfl

theorem typed_borrowGlobal {mutable : Bool} {d : StructDefinitionIndex} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    {tys : List RuntimeType}
    (h : stepOf (borrowGlobal P s fr rest mutable d tys) = .done o) : PostTyped P o := by
  unfold borrowGlobal at h
  obtain ⟨sid, -, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hst, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
  split at h
  · rw [done_next h]
    refine typed_push1 (n := 1) (v := .ref ⟨mutable, .struct sid, ⟨.global a sid, []⟩, s.invalidations.length⟩)
      ht hsf ⟨by simp [shaped, RuntimeType.isValue], fun r hr => ?_⟩
      (by simp [advance, push, borrow, hst]) ht.storageShaped rfl rfl
    cases hr
    refine ⟨fun T hT => Or.inl ?_, fun f i hf => by simp at hf, fun k w hk => by simp at hk⟩
    simp only [rootType, Option.some.injEq] at hT
    subst hT; rfl
  · exact (done_fail h hcont).elim

theorem typed_borrowField {mutable : Bool} {hh : FieldHandleIndex} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    {tys0 : List RuntimeType}
    (h : stepOf (borrowField P s fr rest info mutable hh tys0) = .done o) : PostTyped P o := by
  unfold borrowField at h
  obtain ⟨fh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, htys, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, htt, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst, rfl, htarget, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h hcont deref_ends
    rw [done_finish rfl h]
    obtain ⟨-, hrt⟩ := stack_top_ref ht hsf (k := 0) (r := r) (by simp [hst])
    have htys' := need_ok' htys
    have htt' := need_ok' htt
    refine typed_push1 (n := 1) (v := .ref ⟨mutable, t, r.location.child (.field fh.field),
        ({ s with frames := { fr with stack := st } :: rest } : State).invalidations.length⟩)
      (s₂ := { s with frames := { fr with stack := st } :: rest }) ht hsf
      ⟨by simp [shaped, fieldTypes_values (fieldTypes_args htys') htys' t (List.mem_of_getElem? htt')],
        fun r' hr' => ?_⟩
      (by simp [advance, pushAll, borrow, hst]) ht.storageShaped rfl rfl
    cases hr'
    exact child_typed hrt (by simp [stepType, htarget, htys', htt']) _ _
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem mapM_exec_getElem? {α β : Type} {g : α → Exec β} :
    ∀ {l : List α} {ys : List β}, l.mapM g = .ok ys → ∀ {j : Nat} {x : α}, l[j]? = some x →
      ∃ y, ys[j]? = some y ∧ g x = .ok y
  | [], ys, _, j, x, hx => by simp at hx
  | a :: l, ys, h, j, x, hx => by
    rw [List.mapM_cons, bind_ok'] at h
    obtain ⟨b, hb, h⟩ := h
    rw [bind_ok'] at h
    obtain ⟨bs, hbs, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    cases j with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hx
      subst hx
      exact ⟨b, rfl, hb⟩
    | succ j =>
      simp only [List.getElem?_cons_succ] at hx ⊢
      exact mapM_exec_getElem? hbs hx

theorem typed_borrowVariantField {mutable : Bool} {hh : VariantFieldHandleIndex} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    {tys0 : List RuntimeType}
    (h : stepOf (borrowVariantField P s fr rest info mutable hh tys0) = .done o) :
    PostTyped P o := by
  unfold borrowVariantField at h
  obtain ⟨vfh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, htys, h⟩ := done_bind h hcont (mapM_ends (fun _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst, rfl, htarget, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h hcont deref_ends
    split at h
    · next tag _ =>
      split at h
      · next t hfind =>
        rw [done_finish rfl h]
        obtain ⟨-, hrt⟩ := stack_top_ref ht hsf (k := 0) (r := r) (by simp [hst])
        -- The field's type in the value's variant.
        obtain ⟨hmem, hftag⟩ : (_, t) ∈ vfh.variants.zip tys ∧ _ := ⟨List.mem_of_find?_eq_some hfind,
          List.find?_some hfind⟩
        simp only [beq_iff_eq] at hftag
        obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hmem
        rw [List.getElem?_zip_eq_some] at hj
        obtain ⟨hj1, hj2⟩ := hj
        obtain ⟨y, hy, hg⟩ := mapM_exec_getElem? htys hj1
        rw [hj2, Option.some.injEq] at hy
        subst hy
        rw [bind_ok'] at hg
        obtain ⟨_, -, hg⟩ := hg
        rw [bind_ok'] at hg
        obtain ⟨fields, hfields, hg⟩ := hg
        have hfields' := need_ok' hfields
        have hg' := need_ok' hg
        rw [hftag] at hfields'
        refine typed_push1 (n := 1) (v := .ref ⟨mutable, t, r.location.child (.variantField tag vfh.field),
            ({ s with frames := { fr with stack := st } :: rest } : State).invalidations.length⟩)
          (s₂ := { s with frames := { fr with stack := st } :: rest }) ht hsf
          ⟨by simp [shaped, variantFieldTypes_values (variantFieldTypes_args hfields') hfields' t
              (List.mem_of_getElem? hg')],
            fun r' hr' => ?_⟩
          (by simp [advance, pushAll, borrow, hst]) ht.storageShaped rfl rfl
        cases hr'
        exact child_typed hrt (by simp [stepType, htarget, hfields', hg']) _ _
      · exact (done_fail h hcont).elim
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem typed_borrowElement {mutable : Bool} {sig : SignatureIndex} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (borrowElement s fr rest info mutable sig) = .done o) : PostTyped P o := by
  unfold borrowElement at h
  obtain ⟨t, het, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨i, fr₁⟩, hpi, h⟩ := done_bind h hcont popIndex_ends
  obtain ⟨x, st₁, hst₁, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst', rfl, htarget, -⟩ := popRefTo_ok hpr
  simp only at hst'
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h hcont derefVector_ends
  split at h
  · have hstk : fr.stack = .int .u64 x :: .ref r :: st := by rw [hst₁, hst']
    rw [done_finish rfl h]
    obtain ⟨-, hrt⟩ := stack_top_ref ht hsf (k := 1) (r := r) (by simp [hstk])
    refine typed_push1 (n := 2) (v := .ref ⟨mutable, t, r.location.child (.element x.toNat),
        ({ s with frames := { fr with stack := st } :: rest } : State).invalidations.length⟩)
      (s₂ := { s with frames := { fr with stack := st } :: rest }) ht hsf
      ⟨by simp [shaped, elemTypeWith_value (ht.args fr (by rw [hsf]; exact List.mem_cons_self ..)) het], fun r' hr' => ?_⟩
      (by simp [advance, pushAll, borrow, hstk]) ht.storageShaped rfl rfl
    cases hr'
    exact child_typed hrt (by simp [stepType, htarget]) _ _
  · exact (done_fail h hcont).elim

/-- The current frame with the top `n` popped: typed, with the state
invariant. -/
theorem typed_popped (ht : StateTyped P s) (hinv : StateInv σ P s) (hsf : s.frames = fr :: rest) (n : Nat) :
    StateTyped P { s with frames := { fr with stack := fr.stack.drop n } :: rest } ∧
      StateInv σ P { s with frames := { fr with stack := fr.stack.drop n } :: rest } :=
  ⟨typed_next (fr' := { fr with stack := fr.stack.drop n }) (new := []) (n := n) ht hsf (by simp)
    (by simp) rfl rfl rfl ht.storageShaped rfl rfl, frames_inv hinv hsf rfl rfl rfl rfl⟩

/-- A write through a typed reference popped from the current frame,
followed by pushing `vals`. -/
theorem typed_through {n : Nat} {r : Reference} (ht : StateTyped P s) (hinv : StateInv σ P s)
    (hsf : s.frames = fr :: rest) (hr : Value.ref r ∈ fr.stack) {w w' : Value}
    (hd : s.deref? r.location r.since = some w) (hw' : shaped P w' = true)
    (hty : w'.type.assignable r.target = true)
    {s₂ : State} (hwr : State.write? { s with frames := { fr with stack := fr.stack.drop n } :: rest }
      r.location w' = some s₂)
    {vals : List Value} (hvals : ∀ v ∈ vals, shaped P v = true ∧ ∀ r, v ≠ .ref r)
    {fr₂ : Frame} {rest₂ : List Frame} (hs₂ : s₂.frames = fr₂ :: rest₂) (e : Invalidation) :
    PostTyped P (.running { s₂.invalidate e with frames := advance (pushAll fr₂ vals) :: rest₂ }) := by
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
  obtain ⟨hrs, hrt⟩ := typed_ref_frame ht hsf hr
  obtain ⟨ht₁, hinv₁⟩ := typed_popped ht hinv hsf n
  have hread : State.read? { s with frames := { fr with stack := fr.stack.drop n } :: rest } r.location =
      some w := by
    rw [read_eq (s' := { s with frames := { fr with stack := fr.stack.drop n } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl]
    exact deref?_read hd
  have hrt₁ : RefTyped P { s with frames := { fr with stack := fr.stack.drop n } :: rest } r :=
    refTyped_congr hrt (rootType_congr (by simp [hsf]) rfl) (Nat.le_refl _) (held_same rfl)
  have ht₂ := typed_write ht₁ hinv₁ hrt₁ (by simpa [shaped] using hrs) hread hw' hty hwr
  have ht₃ := typed_invalidate ht₂ e
  exact typed_finish ht₃ (s₂ := s₂.invalidate e) hs₂
    (fun v hv => plain_new (hvals v hv).1 (hvals v hv).2) rfl rfl rfl rfl

theorem typed_moveFrom {d : StructDefinitionIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hinv : StateInv σ P s) (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (moveResourceFrom P s fr rest d tys) = .done o) : PostTyped P o := by
  obtain ⟨sid, -, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hst, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
  split at h
  · next v hget =>
    rw [done_finish rfl h]
    obtain ⟨k, res, -, hres, rfl, -, hrt⟩ := storage_get hget
    have hmem := List.mem_of_getElem? hres
    refine typed_push1 (n := 1) (v := res.value) ht hsf
      (plain_new (ht.storageShaped res hmem) (not_ref_of_type (hinv.typed res hmem) (by simp [RuntimeType.isValue])))
      (by simp [advance, pushAll, hst]) ?_ rfl rfl
    intro r hr
    simp only [State.invalidate, GlobalStorage.remove, List.mem_filter] at hr
    exact ht.storageShaped r hr.1
  · exact (done_fail h hcont).elim

theorem typed_moveTo {d : StructDefinitionIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (moveResourceTo P s fr rest d tys) = .done o) : PostTyped P o := by
  obtain ⟨sid, -, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st₁, hst₁, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst₂, rfl, -, -⟩ := popRefTo_ok hpr
  simp only at hst₂
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hconf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨sv, -, h⟩ := done_bind h hcont deref_ends
  split at h
  · next a =>
    obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
    split at h
    · exact (done_fail h hcont).elim
    · have hstk : fr.stack = v :: .ref r :: st := by rw [hst₁, hst₂]
      rw [done_finish rfl h]
      refine typed_pop1 (n := 2) (pc := fr.pc + 1) ht hsf (by simp [advance, pushAll, hstk]) ?_ rfl rfl
      intro r hr
      simp only [GlobalStorage.insert, List.mem_append, List.mem_singleton] at hr
      rcases hr with hr | rfl
      · exact ht.storageShaped r hr
      · exact (conforms_shaped hconf).1
  · exact (done_wrong h hcont).elim


/-! ### Calls and returns -/

theorem localTypes_params {f : FunctionRef} {tys ps : List RuntimeType}
    (h : P.localTypes f = some tys) (hp : P.paramTypes f = some ps) : ∃ ls, tys = ps ++ ls := by
  simp only [Program.localTypes, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨info, hinfo, code, -, sig, hsig, ps', hps', lsig, -, ls, -, rfl⟩ := h
  have : ps' = ps := by
    simp only [Program.paramTypes, hinfo, hsig, bind, Option.bind_some] at hp
    rw [hps'] at hp; simpa using hp
  exact ⟨ls, by rw [this]⟩

theorem conformsAll_get {vs : List Value} {ts : List RuntimeType} {j : Nat} {v : Value}
    (h : conformsAll P vs ts = true) (hv : vs[j]? = some v) :
    ∃ t, ts[j]? = some t ∧ v.conforms P t = true := by
  induction vs generalizing ts j with
  | nil => simp at hv
  | cons w ws ih =>
    cases ts with
    | nil => simp [conformsAll] at h
    | cons t ts =>
      rw [conformsAll_cons] at h
      cases j with
      | zero => simp at hv; subst hv; exact ⟨t, rfl, h.1⟩
      | succ j => simpa using ih (ts := ts) (j := j) h.2 (by simpa using hv)

/-- The locals of a new frame: the arguments, then empty locals. -/
theorem args_locals_typed {args : List Value} {ps ls : List RuntimeType} {m : Nat}
    (hconf : conformsAll P args ps = true) :
    ∀ (i : Nat) (v : Value), ((args.map some ++ List.replicate m none).toArray)[i]? = some (some v) →
      ∃ t, (ps ++ ls)[i]? = some t ∧ v.type.assignable t = true := by
  intro i v hv
  rw [← Array.getElem?_toList, List.toList_toArray, List.getElem?_append] at hv
  have hlen := conformsAll_length hconf
  split at hv
  · next hi =>
    simp only [List.getElem?_map, Option.map_eq_some_iff, Option.some.injEq] at hv
    obtain ⟨v', hv', rfl⟩ := hv
    have hi' : i < ps.length := by simp at hi; omega
    rw [List.getElem?_append_left hi']
    obtain ⟨t, ht', hct⟩ := conformsAll_get hconf hv'
    exact ⟨t, ht', (conforms_shaped hct).2⟩
  · simp only [List.getElem?_replicate] at hv
    split at hv <;> simp at hv

/-- The new state after a call of a function with code: a new frame, with a
new identity, holding the arguments. -/
theorem typed_call_state {n : Nat} {args : List Value} {ps : List RuntimeType}
    {localTys : List RuntimeType}
    {callee : Frame} {s₂ : State} (ht : StateTyped P s) (hsf : s.frames = fr :: rest)
    (hargs' : ∀ v ∈ args, v ∈ frameVals fr ∨ (shaped P v = true ∧ ∀ r, v ≠ .ref r))
    (hconf : conformsAll P args ps = true)
    (hps : P.paramTypes callee.function = some ps) (hlt : P.localTypes callee.function = some localTys)
    (hloc : callee.locals = (args.map some ++
      List.replicate (localTys.length - args.length) none).toArray)
    (hstk : callee.stack = []) (hid : callee.id = s.nextFrame) {caller' : Frame}
    (h2f : s₂.frames = callee :: caller' :: rest) (hcl : caller' = { fr with stack := fr.stack.drop n })
    (h2st : s₂.storage = s.storage) (h2he : s₂.held = s.held)
    (h2nf : s₂.nextFrame = s.nextFrame + 1)
    (hta : ∀ x ∈ callee.function.typeArgs, x.isValue = true) : PostTyped P (.running s₂) := by
  subst hcl
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
  have hrest : ∀ x ∈ rest, x ∈ s.frames := fun x hx => by rw [hsf]; exact List.mem_cons_of_mem _ hx
  have hroot : ∀ f i T, f < s.nextFrame → rootType P s₂ (.local_ f i) = some T →
      rootType P s (.local_ f i) = some T := by
    intro f i T hf hT
    simp only [rootType, State.frame?, h2f, hsf, List.find?_cons] at hT ⊢
    have hne : (callee.id == f) = false := by
      rw [hid, beq_eq_false_iff_ne]; intro h; rw [h] at hf; exact Nat.lt_irrefl _ hf
    simp only [hne] at hT
    cases hff : fr.id == f <;> simp only [hff] at hT ⊢ <;> simpa using hT
  have hcall : ∀ r, RefTyped P s r → RefTyped P s₂ r := by
    intro r hr
    refine ⟨fun T hT => ?_, fun f i hf => ?_, fun k w hk hw => hr.2.2 k w hk (by rw [← h2he]; exact hw)⟩
    · refine hr.1 T ?_
      cases hrt : r.location.root with
      | local_ f i =>
        rw [hrt] at hT
        exact hroot f i T (hr.2.1 f i hrt) hT
      | global a t => rw [hrt] at hT; exact hT
      | held k =>
        rw [hrt] at hT
        simpa [rootType, h2he] using hT
    · rw [h2nf]; exact Nat.lt_succ_of_lt (hr.2.1 f i hf)
  have hargs : ∀ v ∈ frameVals callee,
      v ∈ frameVals fr ∨ (shaped P v = true ∧ ∀ r, v ≠ .ref r) := by
    intro v hv
    simp only [frameVals, hstk, hloc, List.nil_append, List.toList_toArray, List.mem_filterMap, id,
      List.mem_append, List.mem_map, List.mem_replicate] at hv
    obtain ⟨o, ho, hov⟩ := hv
    rcases ho with ⟨w, hw, rfl⟩ | ⟨-, rfl⟩
    · cases hov
      exact hargs' _ hw
    · cases hov
  have hcaller : ∀ v ∈ frameVals { fr with stack := fr.stack.drop n }, v ∈ frameVals fr := by
    intro v hv
    simp only [frameVals, List.mem_append] at hv ⊢
    rcases hv with hv | hv
    · exact Or.inl (List.mem_of_mem_drop hv)
    · exact Or.inr hv
  show StateTyped P s₂
  refine ⟨?_, by rw [h2st]; exact ht.storageShaped, ?_, ?_, fun v hv => ht.held v (h2he ▸ hv), ?_⟩
  rotate_right
  · intro x hx
    rw [h2f] at hx
    simp only [List.mem_cons] at hx
    rcases hx with rfl | rfl | hx
    · exact hta
    · exact ht.args fr hfr
    · exact ht.args x (hrest x hx)
  · intro x hx v hv
    rw [h2f] at hx
    simp only [List.mem_cons] at hx
    rcases hx with rfl | rfl | hx
    · rcases hargs v hv with hv | hv
      · exact ht.valsShaped fr hfr v hv
      · exact hv.1
    · exact ht.valsShaped fr hfr v (hcaller v hv)
    · exact ht.valsShaped x (hrest x hx) v hv
  · intro x hx
    rw [h2f] at hx
    simp only [List.mem_cons] at hx
    rcases hx with rfl | rfl | hx
    · obtain ⟨ls, rfl⟩ := localTypes_params hlt hps
      exact ⟨_, hlt, by rw [hloc]; exact args_locals_typed hconf⟩
    · exact ht.locals fr hfr
    · exact ht.locals x (hrest x hx)
  · intro x hx r hr
    rw [h2f] at hx
    simp only [List.mem_cons] at hx
    rcases hx with rfl | rfl | hx
    · rcases hargs _ hr with hr | hr
      · exact hcall r (ht.refs fr hfr r hr)
      · exact absurd rfl (hr.2 r)
    · exact hcall r (ht.refs fr hfr r (hcaller _ hr))
    · exact hcall r (ht.refs x (hrest x hx) r hr)

theorem typed_call {hd : FunctionHandleIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (Step.call P s fr rest info hd tys) = .done o) : PostTyped P o := by
  unfold Step.call at h
  obtain ⟨callee, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨calleeInfo, hci, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hwf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hwf
  obtain ⟨paramTys, hps, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hps', -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hrs, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨⟨args, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hconf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
  split at h
  · exact (done_native h hcont).elim
  obtain ⟨localTys, hlt, h⟩ := done_bind h hcont (ends_need _ _)
  split at h <;> split at h
  all_goals first
    | exact (done_fail h hcont).elim
    | skip
  all_goals
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    subst h
    exact typed_call_state ht hsf
      (fun v hv => Or.inl (mem_frameVals_stack (List.mem_of_mem_take (List.mem_reverse.1 hv))))
      hconf (need_ok' hps) (need_ok' hlt) rfl rfl
      (by split <;> rfl) rfl rfl (by split <;> rfl) (by split <;> rfl) (by split <;> rfl)
      (argumentsWellFormed_values hwf)

/-- The state after a return to a caller: the caller's frame takes the
results; the returning frame's identity is no other frame's. -/
theorem typed_ret_state {caller : Frame} {rest' : List Frame} {s₂ : State} (ht : StateTyped P s)
    (hinv : StateInv σ P s) (hsf : s.frames = fr :: caller :: rest')
    (h2f : s₂.frames = { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 } :: rest')
    (h2st : s₂.storage = s.storage) (h2he : s₂.held = s.held) (h2nf : s₂.nextFrame = s.nextFrame) :
    PostTyped P (.running s₂) := by
  have hnd := hinv.frameIds
  rw [hsf, List.map_cons, List.map_cons, List.nodup_cons, List.nodup_cons] at hnd
  obtain ⟨hfn, -, -⟩ := hnd
  have hfc : fr.id ≠ caller.id := fun h => hfn (by rw [h]; exact List.mem_cons_self ..)
  have hfr' : ∀ x ∈ rest', x.id ≠ fr.id := fun x hx h =>
    hfn (List.mem_cons_of_mem _ (List.mem_map.2 ⟨x, hx, h⟩))
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
  have hca : caller ∈ s.frames := by rw [hsf]; simp
  have hrest : ∀ x ∈ rest', x ∈ s.frames := fun x hx => by rw [hsf]; simp [hx]
  have hroot : ∀ f i T, rootType P s₂ (.local_ f i) = some T → rootType P s (.local_ f i) = some T := by
    intro f i T hT
    simp only [rootType, State.frame?, h2f, hsf, List.find?_cons] at hT ⊢
    by_cases hff : fr.id = f
    · subst hff
      have hc : (caller.id == fr.id) = false := by
        rw [beq_eq_false_iff_ne]; exact fun h => hfc h.symm
      have hn : List.find? (fun x => x.id == fr.id) rest' = none := by
        rw [List.find?_eq_none]
        intro x hx hxe
        exact hfr' x hx (by simpa using hxe)
      simp [hc, hn] at hT
    · have hf' : (fr.id == f) = false := by rw [beq_eq_false_iff_ne]; exact hff
      simp only [hf']
      cases hcf : caller.id == f <;> simp only [hcf] at hT ⊢ <;> simpa using hT
  have hret : ∀ r, RefTyped P s r → RefTyped P s₂ r := by
    intro r hr
    refine ⟨fun T hT => ?_, fun f i hf => by rw [h2nf]; exact hr.2.1 f i hf,
      fun k w hk hw => hr.2.2 k w hk (by rw [← h2he]; exact hw)⟩
    refine hr.1 T ?_
    cases hrt : r.location.root with
    | local_ f i => rw [hrt] at hT; exact hroot f i T hT
    | global a t => rw [hrt] at hT; exact hT
    | held k =>
      rw [hrt] at hT
      simpa [rootType, h2he] using hT
  have hvals : ∀ v ∈ frameVals { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 },
      v ∈ frameVals fr ∨ v ∈ frameVals caller := by
    intro v hv
    simp only [frameVals, List.mem_append] at hv
    rcases hv with (hv | hv) | hv
    · exact Or.inl (mem_frameVals_stack hv)
    · exact Or.inr (mem_frameVals_stack hv)
    · exact Or.inr (by simp only [frameVals, List.mem_append]; exact Or.inr hv)
  show StateTyped P s₂
  refine ⟨?_, by rw [h2st]; exact ht.storageShaped, ?_, ?_, fun v hv => ht.held v (h2he ▸ hv), ?_⟩
  rotate_right
  · intro x hx
    rw [h2f] at hx
    rcases List.mem_cons.1 hx with rfl | hx
    · exact ht.args caller (by rw [hsf]; simp)
    · exact ht.args x (by rw [hsf]; simp [hx])
  · intro x hx v hv
    rw [h2f] at hx
    simp only [List.mem_cons] at hx
    rcases hx with rfl | hx
    · rcases hvals v hv with hv | hv
      · exact ht.valsShaped fr hfr v hv
      · exact ht.valsShaped caller hca v hv
    · exact ht.valsShaped x (hrest x hx) v hv
  · intro x hx
    rw [h2f] at hx
    simp only [List.mem_cons] at hx
    rcases hx with rfl | hx
    · exact ht.locals caller hca
    · exact ht.locals x (hrest x hx)
  · intro x hx r hr
    rw [h2f] at hx
    simp only [List.mem_cons] at hx
    rcases hx with rfl | hx
    · rcases hvals _ hr with hr | hr
      · exact hret r (ht.refs fr hfr r hr)
      · exact hret r (ht.refs caller hca r hr)
    · exact hret r (ht.refs x (hrest x hx) r hr)

theorem typed_ret (ht : StateTyped P s) (hinv : StateInv σ P s) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done o) : PostTyped P o := by
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  cases rest with
  | nil =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    subst h; trivial
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    subst h
    exact typed_ret_state ht hinv hsf rfl (by split <;> split <;> rfl) (by split <;> split <;> rfl)
      (by split <;> split <;> rfl)

theorem done_finish_ex {s₁ : State} {vals : List Value} (hcont : o.continues = true)
    (h : stepOf (finish s₁ vals) = .done o) :
    ∃ fr₁ rest₁, s₁.frames = fr₁ :: rest₁ ∧
      o = .running { s₁ with frames := advance (pushAll fr₁ vals) :: rest₁ } := by
  cases hs : s₁.frames with
  | nil =>
    unfold finish at h
    rw [hs] at h
    exact (done_wrong h hcont).elim
  | cons fr₁ rest₁ => exact ⟨fr₁, rest₁, rfl, done_finish hs h⟩

theorem typed_packStruct {d : StructDefinitionIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (packStruct P s fr rest d tys) = .done o) : PostTyped P o := by
  have hstg := ht.storageShaped
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, htys, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
    obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · next hc =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hc
      rw [done_next h]
      exact typed_push1 (n := tys.length) (v := packedValue sid (fr.stack.take tys.length).reverse)
        ht hsf (plain_new (by
          unfold packedValue
          split <;> simp_all [shaped, need_ok' htys, conformsAll_shaped _ _ hc])
          (packedValue_ne_ref _ _)) rfl hstg rfl rfl
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem typed_packEnum {hv : StructVariantHandleIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (packEnum P s fr rest info hv tys) = .done o) : PostTyped P o := by
  have hstg := ht.storageShaped
  obtain ⟨vh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, htys, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
    obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · next hc =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hc
      rw [done_next h]
      exact typed_push1 (n := tys.length)
        (v := .variant sid vh.variant (fr.stack.take tys.length).reverse) ht hsf
        (plain_new (by simp [shaped, need_ok' htys, conformsAll_shaped _ _ hc]) (by simp)) rfl hstg
        rfl rfl
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem typed_unpackStruct {d : StructDefinitionIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (unpackStruct P s fr rest d tys) = .done o) : PostTyped P o := by
  have hstg := ht.storageShaped
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpp
    dsimp only at h
    split at h
    · next sid' fields =>
      split at h
      · exact (done_wrong h hcont).elim
      · rw [done_next h]
        cases hft : P.fieldTypes sid' with
        | none =>
          have hc := (typed_frame ht hsf (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self ..))).1
          simp [shaped, hft] at hc
        | some tys =>
          exact typed_unpack (tys := tys) ht hsf hst (fun hc => by simpa [shaped, hft, Value.children] using hc)
            (fieldTypes_values (fieldTypes_args hft) hft)
    · next sid' fields ks es =>
      split at h
      · exact (done_wrong h hcont).elim
      · rw [done_next h]
        cases hft : P.fieldTypes sid' with
        | none =>
          have hc := (typed_frame ht hsf (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self ..))).1
          simp [shaped, hft] at hc
        | some tys =>
          exact typed_unpackFields (tys := tys) ht hsf hst
            (fun hc => by simp only [shaped, hft, Bool.and_eq_true] at hc; exact hc.1.2)
            (fieldTypes_values (fieldTypes_args hft) hft)
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem typed_unpackEnum {hv : StructVariantHandleIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (unpackEnum P s fr rest info hv tys) = .done o) : PostTyped P o := by
  have hstg := ht.storageShaped
  obtain ⟨vh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpp
    dsimp only at h
    split at h
    · next sid' tag fields =>
      split at h
      · exact (done_wrong h hcont).elim
      · split at h
        · exact (done_fail h hcont).elim
        · rw [done_next h]
          cases hft : P.variantFieldTypes sid' tag with
          | none =>
            have hc := (typed_frame ht hsf (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self ..))).1
            simp [shaped, hft] at hc
          | some tys =>
            exact typed_unpack (tys := tys) ht hsf hst
              (fun hc => by simpa [shaped, hft, Value.children] using hc) (variantFieldTypes_values (variantFieldTypes_args hft) hft)
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem typed_testEnum {hh : StructVariantHandleIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (testEnum P s fr rest info hh tys) = .done o) : PostTyped P o := by
  have hstg := ht.storageShaped
  obtain ⟨vh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst', rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h hcont deref_ends
    split at h
    · next tag _ =>
      rw [done_finish rfl h]
      exact typed_push1 (n := 1) (v := .bool (tag == vh.variant))
        (s₂ := { s with frames := { fr with stack := st } :: rest }) ht hsf (prim_new rfl)
        (by simp [advance, pushAll, hst']) hstg rfl rfl
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem typed_exists {d : StructDefinitionIndex} {tys : List RuntimeType} (ht : StateTyped P s)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (existsResource P s fr rest d tys) = .done o) : PostTyped P o := by
  have hstg := ht.storageShaped
  obtain ⟨sid, -, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hstk, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
  rw [done_next h]
  exact typed_push1 (n := 1) (v := .bool (s.storage.get? a sid).isSome) ht hsf (prim_new rfl)
    (by simp [advance, push, hstk]) hstg rfl rfl

/-! ### Closures -/

/-- What a closure type of a function instance and mask says: the instance
satisfies its constraints, and the mask splits its parameters. -/
theorem closureType?_some {f : FunctionRef} {mask : BitVec 64} {ctys rem rts : List RuntimeType}
    (h : P.closureType? f mask = some (ctys, rem, rts)) :
    P.typeArgsOk f = true ∧ ∃ params, P.paramTypes f = some params ∧
      maskWithin mask params.length = true ∧ maskSplit mask params = (ctys, rem) := by
  unfold Program.closureType? at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, -, _, -, h⟩ := h
  split at h
  · simp at h
  · next hok =>
    obtain ⟨params, hps, h⟩ := Option.bind_eq_some_iff.1 h
    obtain ⟨results, -, h⟩ := Option.bind_eq_some_iff.1 h
    split at h
    · simp at h
    · next hw =>
      split at h
      · simp at h
      · simp only [pure, Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl, -⟩ := h
        exact ⟨by simpa using hok, params, hps, by simpa using hw, rfl⟩

theorem maskSplit_go {α : Type} (mask : BitVec 64) : ∀ (xs : List α) (k : Nat),
    ((List.range' k xs.length).map mask.getLsbD).count true =
        ((xs.zipIdx k).filter fun (_, i) => mask.getLsbD i).length ∧
      ((xs.zipIdx k).filter fun (_, i) => mask.getLsbD i).length +
        ((xs.zipIdx k).filter fun (_, i) => !mask.getLsbD i).length = xs.length
  | [], k => by simp
  | x :: xs, k => by
    obtain ⟨h1, h2⟩ := maskSplit_go mask xs (k + 1)
    cases hb : mask.getLsbD k <;>
      simp [List.range'_succ, hb] at h1 h2 ⊢ <;> omega

theorem shapedFields_length : ∀ {vs : List Value} {ts : List RuntimeType},
    shapedFields P vs ts = true → vs.length = ts.length
  | [], [], _ => rfl
  | _ :: _, _ :: _, h => by
    simp only [shapedFields, Bool.and_eq_true] at h
    simp [shapedFields_length h.2]
  | [], _ :: _, h => by simp [shapedFields] at h
  | _ :: _, [], h => by simp [shapedFields] at h

/-- A shaped closure composes its captured and provided values completely. -/
theorem composes_of_shaped {v : Value} (hs : shaped P v = true) : Composes P v := by
  cases v with
  | closure f mask cs ps rs a =>
    simp only [shaped] at hs
    split at hs
    · next ctys rem rts hct =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hs
      obtain ⟨⟨⟨rfl, -⟩, -⟩, hcs⟩ := hs
      obtain ⟨-, params, -, -, hsplit⟩ := closureType?_some hct
      have hlen := shapedFields_length hcs
      obtain ⟨h1, h2⟩ := maskSplit_go mask params 0
      simp only [maskSplit, Prod.mk.injEq] at hsplit
      obtain ⟨rfl, rfl⟩ := hsplit
      simp only [List.length_map] at hlen ⊢
      simp only [Composes, hlen, List.length_map, h2, List.range_eq_range', h1]
    · simp at hs
  | table s fs ks es =>
    intro hne
    simp only [shaped, Bool.and_eq_true, Bool.or_eq_true, List.isEmpty_iff] at hs
    obtain ⟨⟨hst, -⟩, hes | ⟨htab, -⟩⟩ := hs
    · exact absurd hes hne
    · exact ⟨htab, hst⟩
  | _ => trivial

/-- The frames' values compose completely. -/
theorem composes_of_typed {s : State} (ht : StateTyped P s) :
    ∀ fr ∈ s.frames, ∀ v ∈ fr.stack, Composes P v :=
  fun fr hfr v hv => composes_of_shaped (ht.valsShaped fr hfr v (mem_frameVals_stack hv))

theorem maskCompose_go_mem {α : Type} : ∀ (bits : List Bool) (cs ps : List α) {v : α},
    v ∈ maskCompose.go bits cs ps → v ∈ cs ∨ v ∈ ps
  | [], _, _, _, h => by simp [maskCompose.go] at h
  | true :: bits, c :: cs, ps, v, h => by
    simp only [maskCompose.go, List.mem_cons] at h
    rcases h with rfl | h
    · exact Or.inl (List.mem_cons_self ..)
    · rcases maskCompose_go_mem bits cs ps h with h | h
      · exact Or.inl (List.mem_cons_of_mem _ h)
      · exact Or.inr h
  | false :: bits, cs, p :: ps, v, h => by
    simp only [maskCompose.go, List.mem_cons] at h
    rcases h with rfl | h
    · exact Or.inr (List.mem_cons_self ..)
    · rcases maskCompose_go_mem bits cs ps h with h | h
      · exact Or.inl h
      · exact Or.inr (List.mem_cons_of_mem _ h)
  | true :: _, [], _, _, h => by simp [maskCompose.go] at h
  | false :: _, _, [], _, h => by simp [maskCompose.go] at h

theorem maskCompose_mem {α : Type} {mask : BitVec 64} {cs ps : List α} {v : α}
    (h : v ∈ maskCompose mask cs ps) : v ∈ cs ∨ v ∈ ps :=
  maskCompose_go_mem _ cs ps h

/-- `PackClosure` pushes a shaped closure: its function's instance satisfies
its constraints, and the captured values conform. -/
theorem typed_packClosure {hd : FunctionHandleIndex} {mask : BitVec 64} {tys : List RuntimeType}
    (ht : StateTyped P s) (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (h : stepOf (Step.packClosure P s fr rest info hd mask tys) = .done o) : PostTyped P o := by
  have hstg := ht.storageShaped
  unfold Step.packClosure at h
  obtain ⟨target, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨targetInfo, hti, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hwf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hwf
  obtain ⟨paramTys, hps, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨retTys, hrs, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hps', -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hrs', -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hcode =>
  split at h
  · exact (done_wrong h hcont).elim
  next hmask =>
  rcases hsplit : maskSplit mask paramTys with ⟨capturedTys, remaining⟩
  rw [hsplit] at h
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hcap =>
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hconf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf hmask hcap
  rw [done_next h]
  have hti' := need_ok' hti
  obtain ⟨c, hc⟩ : ∃ c, targetInfo.definition.code = some c := by
    cases hcd : targetInfo.definition.code with
    | none => simp [hcd] at hcode
    | some c => exact ⟨c, rfl⟩
  have hct : P.closureType? { target with typeArgs := tys } mask =
      some (capturedTys, remaining, retTys) := by
    have hok : P.typeArgsOk { target with typeArgs := tys } = true := by
      simp only [Program.typeArgsOk, show P.function? { target with typeArgs := tys } =
        some targetInfo from hti']
      exact hwf
    simp only [Program.closureType?, show P.function? { target with typeArgs := tys } =
      some targetInfo from hti', hc, hok, need_ok' hps, need_ok' hrs, hmask, hsplit, hcap, bind,
      Option.bind_some, Bool.not_true, pure]
    rfl
  exact typed_push1 (n := capturedTys.length)
    (v := .closure { target with typeArgs := tys } mask (fr.stack.take capturedTys.length).reverse
      remaining retTys (P.closureValueAbilities { target with typeArgs := tys }
        (fr.stack.take capturedTys.length).reverse)) ht hsf
    (plain_new (by simp only [shaped, hct]; simp [conformsAll_shaped _ _ hconf]) (by simp))
    (by simp [advance, push]) hstg rfl rfl

/-- `CallClosure` runs the closure's function with its captured and the
provided values as arguments, which conform to its parameter types. -/
theorem typed_callClosure {sig : SignatureIndex} (ht : StateTyped P s) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.callClosure P s fr rest info sig) = .done o) : PostTyped P o := by
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
  unfold Step.callClosure at h
  obtain ⟨sigTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  dsimp only at h
  split at h
  · obtain ⟨ps, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨rs, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨_, hx, h⟩ := done_bind h hcont (ends_pure _)
    simp only [pure, Except.pure, Except.ok.injEq] at hx
    subst hx
    dsimp only at h
    obtain ⟨⟨c, fr₁⟩, hpop, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpop
    dsimp only at h
    split at h
    · next target mask captured cps crs ca =>
      split at h
      · exact (done_wrong h hcont).elim
      obtain ⟨⟨provided, fr₂⟩, hpop2, h⟩ := done_bind h hcont popN_ends
      obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop2
      dsimp only at h
      split at h
      · exact (done_wrong h hcont).elim
      obtain ⟨targetInfo, -, h⟩ := done_bind h hcont (ends_need _ _)
      split at h
      · exact (done_wrong h hcont).elim
      obtain ⟨paramTys, hps, h⟩ := done_bind h hcont (ends_need _ _)
      obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
      split at h
      · exact (done_wrong h hcont).elim
      next hconf =>
      simp only [Bool.or_eq_true, Bool.not_eq_true', not_or, Bool.not_eq_false] at hconf
      obtain ⟨localTys, hlt, h⟩ := done_bind h hcont (ends_need _ _)
      have hcs : shaped P (.closure target mask captured cps crs ca) = true :=
        ht.valsShaped fr hfr _ (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self ..))
      simp only [shaped] at hcs
      split at hcs
      · next ctys rem rts hct =>
        simp only [Bool.and_eq_true] at hcs
        have hargs : ∀ v ∈ maskCompose mask captured ((st.take ps.length).reverse),
            v ∈ frameVals fr ∨ (shaped P v = true ∧ ∀ r, v ≠ .ref r) := by
          intro v hv
          rcases maskCompose_mem hv with hv | hv
          · right
            obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hv
            obtain ⟨t, h1, h2, h3⟩ := shapedFields_get hcs.2 hj
            exact ⟨h3, not_ref_of_type rfl (by
              rw [assignable_isValue h2]
              exact closureType?_captured_value hct t (List.mem_of_getElem? h1))⟩
          · left
            exact mem_frameVals_stack (by
              rw [hst]; exact List.mem_cons_of_mem _ (List.mem_of_mem_take (List.mem_reverse.1 hv)))
        have hta := typeArgsOk_values (closureType?_some hct).1
        split at h <;> split at h
        all_goals first
          | exact (done_fail h hcont).elim
          | skip
        all_goals
          simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
          subst h
          exact typed_call_state (n := ps.length + 1) ht hsf hargs hconf.1 (need_ok' hps)
            (need_ok' hlt) rfl rfl (by first | rfl | (split <;> rfl)) rfl (by simp [hst])
            (by first | rfl | (split <;> rfl)) (by first | rfl | (split <;> rfl))
            (by first | rfl | (split <;> rfl)) hta
      · simp at hcs
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

/-- **Every core step keeps the typing invariant.** -/
theorem stateTyped_exec (ht : StateTyped P s) (hinv : StateInv σ P s) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {instr : Bytecode}
    (h : stepOf (Step.exec P s fr rest info code instr) = .done o) : PostTyped P o := by
  have hstg := ht.storageShaped
  cases instr
  case pop =>
    obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · rw [done_next h]; exact typed_pop1 (n := 1) (pc := fr.pc + 1) ht hsf (by simp [advance, hst]) hstg rfl rfl
  case brTrue off =>
    obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
    obtain ⟨st, hst, rfl⟩ := popBool_ok hpb
    cases b
    · rw [done_next (s := s) (rest := rest) (fr := advance { fr with stack := st }) h]
      exact typed_pop1 (n := 1) (pc := fr.pc + 1) ht hsf (by simp [advance, hst]) hstg rfl rfl
    · obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
      rw [jump_ok hj] at h
      rw [done_next h]; exact typed_pop1 (n := 1) (pc := off) ht hsf (by simp [hst]) hstg rfl rfl
  case brFalse off =>
    obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
    obtain ⟨st, hst, rfl⟩ := popBool_ok hpb
    cases b
    · obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
      rw [jump_ok hj] at h
      rw [done_next h]; exact typed_pop1 (n := 1) (pc := off) ht hsf (by simp [hst]) hstg rfl rfl
    · rw [done_next (s := s) (rest := rest) (fr := advance { fr with stack := st }) h]
      exact typed_pop1 (n := 1) (pc := fr.pc + 1) ht hsf (by simp [advance, hst]) hstg rfl rfl
  case branch off =>
    obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
    rw [jump_ok hj] at h
    rw [done_next h]; exact typed_pop1 (n := 0) (pc := off) ht hsf (by simp) hstg rfl rfl
  case nop =>
    rw [done_next h]; exact typed_pop1 (n := 0) (pc := fr.pc + 1) ht hsf (by simp [advance]) hstg rfl rfl
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 =>
    rw [done_next h]
    refine typed_push1 (n := 0) ht hsf ?_ rfl hstg rfl rfl
    exact prim_new rfl
  case ldTrue | ldFalse =>
    rw [done_next h]
    refine typed_push1 (n := 0) ht hsf ?_ rfl hstg rfl rfl
    exact prim_new rfl
  case ldConst i =>
    obtain ⟨c, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨v, hv, h⟩ := done_bind h hcont (ends_need _ _)
    have hdec := need_ok' hv
    have hplain := decodeConstant_plain hdec v (self_mem_subvalues v)
    rw [done_next h]
    exact typed_push1 (n := 0) ht hsf (plain_new (decodeConstant_shaped hdec) (fun r hr => by
      subst hr; simp [IsPlain] at hplain)) (by simp [advance, push]) hstg rfl rfl
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont cast_ends
    obtain ⟨r, hR, -⟩ := cast_ok hx
    rw [done_next h]; exact typed_replaces ht hsf hR (prim_new rfl)
  case add | sub | mul | div | mod | bitOr | bitAnd | xor =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont intBinop_ends
    obtain ⟨t, r, hR, -⟩ := intBinop_ok hx
    rw [done_next h]; exact typed_replaces ht hsf hR (prim_new rfl)
  case shl | shr =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont shift_ends
    obtain ⟨t, r, hR, -⟩ := shift_ok hx
    rw [done_next h]; exact typed_replaces ht hsf hR (prim_new rfl)
  case or | and =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont boolBinop_ends
    obtain ⟨b, hR, -⟩ := boolBinop_ok hx
    rw [done_next h]; exact typed_replaces ht hsf hR (prim_new rfl)
  case lt | gt | le | ge =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont intCompare_ends
    obtain ⟨b, hR, -⟩ := intCompare_ok hx
    rw [done_next h]; exact typed_replaces ht hsf hR (prim_new rfl)
  case eq | neq =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont equality_ends
    obtain ⟨-, -, -, -, -, -, c, hR⟩ := equality_ok hx
    rw [done_next h]; exact typed_replaces ht hsf hR (prim_new rfl)
  case not =>
    obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
    obtain ⟨st, hst, rfl⟩ := popBool_ok hpb
    rw [done_next h]
    exact typed_push1 (n := 1) ht hsf (prim_new (v := .bool !b) rfl) (by simp [advance, push, hst]) hstg rfl rfl
  case negate =>
    obtain ⟨⟨t, x, fr₁⟩, hpi, h⟩ := done_bind h hcont popInt_ends
    obtain ⟨st, hst, rfl⟩ := popInt_ok hpi
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · split at h
      · next r _ =>
        rw [done_next h]
        exact typed_push1 (n := 1) ht hsf (prim_new (v := .int t r) rfl) (by simp [advance, push, hst])
          hstg rfl rfl
      · exact (done_fail h hcont).elim
  case copyLoc i =>
    obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
    cases slot with
    | none => exact (done_wrong h hcont).elim
    | some v =>
      dsimp only at h
      split at h
      · rw [done_next h]
        exact typed_push1 (n := 0) ht hsf (typed_frame ht hsf (mem_frameVals_locals (need_ok' hslot)))
          (by simp [advance, push]) hstg rfl rfl
      · exact (done_wrong h hcont).elim
  case moveLoc i =>
    obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
    have hslot' := need_ok' hslot
    cases slot with
    | none => exact (done_wrong h hcont).elim
    | some v =>
      rw [done_next h]
      refine typed_head ht hsf rfl hstg rfl rfl rfl rfl (fun w hw => Or.inl ?_) (fun j w hw => Or.inl ?_)
      · simp only [frameVals, advance, push, List.mem_append, List.mem_cons, List.mem_filterMap, id,
          Array.toList_setIfInBounds] at hw
        rcases hw with (rfl | hw) | ⟨o, ho, rfl⟩
        · exact mem_frameVals_locals hslot'
        · exact mem_frameVals_stack hw
        · rcases List.mem_or_eq_of_mem_set ho with ho | ho
          · simp only [frameVals, List.mem_append, List.mem_filterMap, id]; exact Or.inr ⟨_, ho, rfl⟩
          · simp at ho
      · have hw' : (fr.locals.setIfInBounds i none)[j]? = some (some w) := hw
        rw [Array.getElem?_setIfInBounds] at hw'
        split at hw'
        · split at hw' <;> simp at hw'
        · exact hw'
  case stLoc i =>
    obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpop
    dsimp only at h
    obtain ⟨tys, htys, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, ht', h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨slot, -, h⟩ := done_bind h hcont (ends_need _ _)
    split at h
    · exact (done_wrong h hcont).elim
    next hconf =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
    have hvt := (conforms_shaped hconf).2
    have hgoal : ∀ s₂ : State, (∀ r ∈ s₂.storage, shaped P r.value = true) → s₂.held = s.held →
        s₂.nextFrame = s.nextFrame →
        PostTyped P (.running { s₂ with frames := advance { { fr with stack := st } with
          locals := fr.locals.setIfInBounds i (some v) } :: rest }) := by
      intro s₂ h2st h2he h2nf
      refine typed_head ht hsf rfl h2st h2he h2nf rfl rfl (fun w hw => Or.inl ?_) (fun j w hw => ?_)
      · simp only [frameVals, advance, List.mem_append, List.mem_filterMap, id,
          Array.toList_setIfInBounds] at hw
        rcases hw with hw | ⟨o, ho, rfl⟩
        · exact mem_frameVals_stack (by rw [hst]; exact List.mem_cons_of_mem _ hw)
        · rcases List.mem_or_eq_of_mem_set ho with ho | ho
          · simp only [frameVals, List.mem_append, List.mem_filterMap, id]; exact Or.inr ⟨_, ho, rfl⟩
          · simp only [Option.some.injEq] at ho; subst ho
            exact mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self ..)
      · have hw' : (fr.locals.setIfInBounds i (some v))[j]? = some (some w) := hw
        rw [Array.getElem?_setIfInBounds] at hw'
        split at hw'
        · next hij =>
          subst hij
          split at hw'
          · simp only [Option.some.injEq] at hw'
            subst hw'
            exact Or.inr ⟨tys, t, need_ok' htys, need_ok' ht', hvt⟩
          · simp at hw'
        · exact Or.inl hw'
    cases slot with
    | none => rw [done_next h]; exact hgoal s hstg rfl rfl
    | some old =>
      dsimp only at h
      split at h
      · exact (done_wrong h hcont).elim
      · rw [done_next h]; exact hgoal _ hstg rfl rfl
  case pack => exact typed_packStruct ht hsf hcont h
  case packGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_packStruct ht hsf hcont h
  case packVariant => exact typed_packEnum ht hsf hcont h
  case packVariantGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_packEnum ht hsf hcont h
  case unpack => exact typed_unpackStruct ht hsf hcont h
  case unpackGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_unpackStruct ht hsf hcont h
  case unpackVariant => exact typed_unpackEnum ht hsf hcont h
  case unpackVariantGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_unpackEnum ht hsf hcont h
  case testVariant => exact typed_testEnum ht hsf hcont h
  case testVariantGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_testEnum ht hsf hcont h
  case exists_ => exact typed_exists ht hsf hcont h
  case existsGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_exists ht hsf hcont h
  case vecPack sig n =>
    obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
    obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
    obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · next hc =>
      simp only [Bool.not_eq_true', Bool.not_eq_false, List.all_eq_true] at hc
      rw [done_next h]
      refine typed_push1 (n := n) (v := .vector t (fr.stack.take n).reverse) ht hsf
        (plain_new ?_ (by simp)) rfl hstg rfl rfl
      simp only [shaped, shapedAll_iff]
      intro x hx
      exact ⟨(conforms_shaped (hc x hx)).2, (conforms_shaped (hc x hx)).1⟩
  case vecUnpack sig n =>
    obtain ⟨t, het, h⟩ := done_bind h hcont elemType_ends
    obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpp
    dsimp only at h
    split at h
    · next t' elems =>
      split at h
      · exact (done_wrong h hcont).elim
      · next htt =>
        simp only [bne_iff_ne, ne_eq, Decidable.not_not] at htt
        split at h
        · exact (done_fail h hcont).elim
        · rw [done_next h]
          have hc := (typed_frame ht hsf (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self ..))).1
          simp only [shaped, shapedAll_iff] at hc
          refine typed_next (fr' := advance (pushAll { fr with stack := st } elems))
            (new := elems.reverse) (n := 1) ht hsf (fun x hx => ?_)
            (by simp [advance, pushAll, hst]) rfl rfl rfl hstg rfl rfl
          rw [List.mem_reverse] at hx
          exact plain_new (hc x hx).2 (not_ref_of_type rfl (by
            rw [assignable_isValue (hc x hx).1, htt]
            exact elemTypeWith_value (ht.args fr (by rw [hsf]; exact List.mem_cons_self ..)) het))
    · exact (done_wrong h hcont).elim
  case abort => exact (abort_absurd hcont h).elim
  case abortMsg => exact (abortMsg_absurd hcont h).elim
  case readRef =>
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRef_ends
    obtain ⟨st, hst', rfl⟩ := popRef_ok hpr
    dsimp only at h
    obtain ⟨v, hv, h⟩ := done_bind h hcont deref_ends
    split at h
    · exact (done_wrong h hcont).elim
    · rw [done_finish rfl h]
      have hd := deref_ok hv
      rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
        (by simp [hsf]) rfl rfl rfl] at hd
      have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
      have hr : Value.ref r ∈ fr.stack := by rw [hst']; exact List.mem_cons_self ..
      obtain ⟨hws, hwv, -⟩ := deref_shaped ht hinv hfr hr hd
      exact typed_push1 (n := 1) (v := v) (s₂ := { s with frames := { fr with stack := st } :: rest })
        ht hsf (plain_new hws (not_ref_of_type rfl hwv))
        (by simp [advance, pushAll, hst']) hstg rfl rfl
  case writeRef =>
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRef_ends
    obtain ⟨st₁, hst₁, rfl⟩ := popRef_ok hpr
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    obtain ⟨⟨v, fr₂⟩, hpv, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst₂, rfl⟩ := pop_ok hpv
    simp only at hst₂
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    next hconf =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
    obtain ⟨old, hold, h⟩ := done_bind h hcont deref_ends
    split at h
    · exact (done_wrong h hcont).elim
    obtain ⟨s₂', hu, hfin⟩ := done_bind h hcont update_ends
    obtain ⟨s₂, hwr, rfl⟩ := update_ok hu
    have hstk : fr.stack = .ref r :: v :: st := by rw [hst₁, hst₂]
    have hfr₁ : ({ s with frames := { { fr with stack := st₁ } with stack := st } :: rest } : State) =
        { s with frames := { fr with stack := fr.stack.drop 2 } :: rest } := by simp [hstk]
    rw [hfr₁] at hold hwr
    have hd := deref_ok hold
    rw [deref_eq (s' := { s with frames := { fr with stack := fr.stack.drop 2 } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl rfl] at hd
    obtain ⟨fr₃, rest₃, hs₃, rfl⟩ := done_finish_ex hcont hfin
    have hr : Value.ref r ∈ fr.stack := by rw [hstk]; exact List.mem_cons_self ..
    have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
    exact typed_through (n := 2) ht hinv hsf hr hd (conforms_shaped hconf).1
      (conforms_shaped hconf).2 hwr (vals := []) (by simp) hs₃ _
  case freezeRef =>
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRef_ends
    obtain ⟨st, hst', rfl⟩ := popRef_ok hpr
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · rw [done_next h]
      obtain ⟨hrs, hrt⟩ := stack_top_ref ht hsf (k := 0) (r := r) (by simp [hst'])
      exact typed_push1 (n := 1) (v := .ref { r with mutable := false }) ht hsf
        ⟨by simpa [shaped] using hrs, fun r' hr' => by cases hr'; exact hrt⟩
        (by simp [advance, push, hst']) hstg rfl rfl
  case mutBorrowLoc => exact typed_borrowLoc ht hinv hsf hcont h
  case immBorrowLoc => exact typed_borrowLoc ht hinv hsf hcont h
  case mutBorrowField => exact typed_borrowField ht hsf hcont h
  case immBorrowField => exact typed_borrowField ht hsf hcont h
  case mutBorrowVariantField => exact typed_borrowVariantField ht hsf hcont h
  case immBorrowVariantField => exact typed_borrowVariantField ht hsf hcont h
  case mutBorrowFieldGeneric | immBorrowFieldGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_borrowField ht hsf hcont h
  case mutBorrowVariantFieldGeneric | immBorrowVariantFieldGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_borrowVariantField ht hsf hcont h
  case vecImmBorrow => exact typed_borrowElement ht hsf hcont h
  case vecMutBorrow => exact typed_borrowElement ht hsf hcont h
  case mutBorrowGlobal => exact typed_borrowGlobal ht hsf hcont h
  case immBorrowGlobal => exact typed_borrowGlobal ht hsf hcont h
  case mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_borrowGlobal ht hsf hcont h
  case moveFrom => exact typed_moveFrom ht hinv hsf hcont h
  case moveFromGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_moveFrom ht hinv hsf hcont h
  case moveTo => exact typed_moveTo ht hsf hcont h
  case moveToGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_moveTo ht hsf hcont h
  case call => exact typed_call ht hsf hcont h
  case packClosure => exact typed_packClosure ht hsf hcont h
  case packClosureGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_packClosure ht hsf hcont h
  case callClosure => exact typed_callClosure ht hsf hcont h
  case callGeneric =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact typed_call ht hsf hcont h
  case ret => exact typed_ret ht hinv hsf hcont h
  case vecLen sig =>
    obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst', rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨elems, -, h⟩ := done_bind h hcont derefVector_ends
    split at h
    · rw [done_finish rfl h]
      exact typed_push1 (n := 1) (v := .int .u64 (elems.length : Int))
        (s₂ := { s with frames := { fr with stack := st } :: rest }) ht hsf (prim_new rfl)
        (by simp [advance, pushAll, hst']) hstg rfl rfl
    · exact (done_fail h hcont).elim
  case vecPushBack sig =>
    obtain ⟨t, het, h⟩ := done_bind h hcont elemType_ends
    obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st₁, hst₁, rfl⟩ := pop_ok hpv
    obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst₂, rfl, htarget, -⟩ := popRefTo_ok hpr
    simp only at hst₂
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    next hconf =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
    obtain ⟨elems, helems, h⟩ := done_bind h hcont derefVector_ends
    obtain ⟨s₂', hu, hfin⟩ := done_bind h hcont update_ends
    obtain ⟨s₂, hwr, rfl⟩ := update_ok hu
    have hstk : fr.stack = v :: .ref r :: st := by rw [hst₁, hst₂]
    have hfr₁ : ({ s with frames := { { fr with stack := st₁ } with stack := st } :: rest } : State) =
        { s with frames := { fr with stack := fr.stack.drop 2 } :: rest } := by simp [hstk]
    rw [hfr₁] at helems hwr
    obtain ⟨t', hd⟩ := derefVector_ok helems
    rw [deref_eq (s' := { s with frames := { fr with stack := fr.stack.drop 2 } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl rfl] at hd
    obtain ⟨fr₃, rest₃, hs₃, rfl⟩ := done_finish_ex hcont hfin
    have hr : Value.ref r ∈ fr.stack := by rw [hstk]; simp
    have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
    obtain ⟨hws, hwt⟩ := deref_shaped_rigid ht hinv hfr hr hd (by rw [htarget]; rfl)
    rw [htarget] at hwt
    simp only [Value.type, RuntimeType.vector.injEq] at hwt
    subst hwt
    simp only [shaped, shapedAll_iff] at hws
    refine typed_through (n := 2) ht hinv hsf hr hd ?_ (by rw [htarget]; exact assignable_refl _) hwr (vals := []) (by simp) hs₃ _
    simp only [shaped, shapedAll_iff]
    intro x hx
    rcases List.mem_append.1 hx with hx | hx
    · exact hws x hx
    · simp only [List.mem_singleton] at hx
      subst hx
      exact ⟨(conforms_shaped hconf).2, (conforms_shaped hconf).1⟩
  case vecPopBack sig =>
    obtain ⟨t, het, h⟩ := done_bind h hcont elemType_ends
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hstk, rfl, htarget, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨elems, helems, h⟩ := done_bind h hcont derefVector_ends
    split at h
    · next last init hrev =>
      obtain ⟨s₂', hu, hfin⟩ := done_bind h hcont update_ends
      obtain ⟨s₂, hwr, rfl⟩ := update_ok hu
      have hfr₁ : ({ s with frames := { fr with stack := st } :: rest } : State) =
          { s with frames := { fr with stack := fr.stack.drop 1 } :: rest } := by simp [hstk]
      rw [hfr₁] at helems hwr
      obtain ⟨t', hd⟩ := derefVector_ok helems
      rw [deref_eq (s' := { s with frames := { fr with stack := fr.stack.drop 1 } :: rest }) (s := s)
        (by simp [hsf]) rfl rfl rfl] at hd
      obtain ⟨fr₃, rest₃, hs₃, rfl⟩ := done_finish_ex hcont hfin
      have hr : Value.ref r ∈ fr.stack := by rw [hstk]; simp
      have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
      obtain ⟨hws, hwt⟩ := deref_shaped_rigid ht hinv hfr hr hd (by rw [htarget]; rfl)
      rw [htarget] at hwt
      simp only [Value.type, RuntimeType.vector.injEq] at hwt
      subst hwt
      have helems' : elems = init.reverse ++ [last] := by
        have := congrArg List.reverse hrev
        simpa using this
      simp only [shaped, shapedAll_iff] at hws
      rw [helems'] at hws
      refine typed_through (n := 1) ht hinv hsf hr hd ?_ (by rw [htarget]; exact assignable_refl _) hwr (vals := [last]) ?_ hs₃ _
      · simp only [shaped, shapedAll_iff]
        intro x hx
        exact hws x (List.mem_append_left _ hx)
      · intro x hx
        simp only [List.mem_singleton] at hx
        subst hx
        have hl := hws x (by simp)
        exact ⟨hl.2, not_ref_of_type rfl (by
          rw [assignable_isValue hl.1]
          exact elemTypeWith_value (ht.args fr (by rw [hsf]; exact List.mem_cons_self ..)) het)⟩
    · exact (done_fail h hcont).elim
  case vecSwap sig =>
    obtain ⟨t, het, h⟩ := done_bind h hcont elemType_ends
    obtain ⟨⟨j, fr₁⟩, hpj, h⟩ := done_bind h hcont popIndex_ends
    obtain ⟨jx, st₁, hst₁, rfl, rfl⟩ := popIndex_ok hpj
    obtain ⟨⟨i, fr₂⟩, hpi, h⟩ := done_bind h hcont popIndex_ends
    obtain ⟨ix, st₂, hst₂, rfl, rfl⟩ := popIndex_ok hpi
    obtain ⟨⟨r, fr₃⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst₃, rfl, htarget, -⟩ := popRefTo_ok hpr
    simp only at hst₂ hst₃
    dsimp only at h
    obtain ⟨elems, helems, h⟩ := done_bind h hcont derefVector_ends
    split at h
    · next x y hx hy =>
      obtain ⟨s₂', hu, hfin⟩ := done_bind h hcont update_ends
      obtain ⟨s₂, hwr, rfl⟩ := update_ok hu
      have hstk : fr.stack = .int .u64 jx :: .int .u64 ix :: .ref r :: st := by rw [hst₁, hst₂, hst₃]
      have hfr₁ : ({ s with frames := { { { fr with stack := st₁ } with stack := st₂ } with stack := st } :: rest } : State) =
          { s with frames := { fr with stack := fr.stack.drop 3 } :: rest } := by simp [hstk]
      rw [hfr₁] at helems hwr
      obtain ⟨t', hd⟩ := derefVector_ok helems
      rw [deref_eq (s' := { s with frames := { fr with stack := fr.stack.drop 3 } :: rest }) (s := s)
        (by simp [hsf]) rfl rfl rfl] at hd
      obtain ⟨fr₄, rest₄, hs₄, rfl⟩ := done_finish_ex hcont hfin
      have hr : Value.ref r ∈ fr.stack := by rw [hstk]; simp
      have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
      obtain ⟨hws, hwt⟩ := deref_shaped_rigid ht hinv hfr hr hd (by rw [htarget]; rfl)
      rw [htarget] at hwt
      simp only [Value.type, RuntimeType.vector.injEq] at hwt
      subst hwt
      simp only [shaped, shapedAll_iff] at hws
      refine typed_through (n := 3) ht hinv hsf hr hd ?_ (by rw [htarget]; exact assignable_refl _) hwr (vals := []) (by simp) hs₄ _
      simp only [shaped, shapedAll_iff]
      intro z hz
      rcases List.mem_or_eq_of_mem_set hz with hz | rfl
      · rcases List.mem_or_eq_of_mem_set hz with hz | rfl
        · exact hws z hz
        · exact hws _ (List.mem_of_getElem? hy)
      · exact hws _ (List.mem_of_getElem? hx)
    · exact (done_fail h hcont).elim
    · exact (done_fail h hcont).elim
  all_goals exact (done_wrong h hcont).elim



end Steps


/-! ## No references inside values -/

section NoRef

variable {P : Program}

/-- A shaped value of a value type holds no reference, all the way down. -/
theorem shaped_noRef (v : Value) (hs : shaped P v = true) (hv : v.type.isValue = true) :
    ∀ w ∈ subvalues v, w.isRef = false := by
  induction v using Value.rec
    (motive_2 := fun vs => ∀ c ∈ vs, shaped P c = true → c.type.isValue = true →
      ∀ w ∈ subvalues c, w.isRef = false) with
  | vector e vs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · rfl
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped, shapedAll_iff] at hs
      obtain ⟨hct, hcs⟩ := hs c hc
      exact ih c hc hcs (by
        rw [assignable_isValue hct]; simpa [Value.type, RuntimeType.isValue] using hv) w hwc
  | struct st fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · rfl
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped] at hs
      split at hs
      · next tys htys =>
        obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
        obtain ⟨t, h1, h2, h3⟩ := shapedFields_get hs hj
        exact ih c hc h3 (by
          rw [assignable_isValue h2]
          exact fieldTypes_values (fieldTypes_args htys) htys _ (List.mem_of_getElem? h1)) w hwc
      · simp at hs
  | variant st tag fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · rfl
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped] at hs
      split at hs
      · next tys htys =>
        obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
        obtain ⟨t, h1, h2, h3⟩ := shapedFields_get hs hj
        exact ih c hc h3 (by
          rw [assignable_isValue h2]
          exact variantFieldTypes_values (variantFieldTypes_args htys) htys _ (List.mem_of_getElem? h1))
          w hwc
      · simp at hs
  | closure f mask cs ps rs a ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · rfl
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped] at hs
      split at hs
      · next ctys rem rts hct =>
        simp only [Bool.and_eq_true] at hs
        obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
        obtain ⟨t, h1, h2, h3⟩ := shapedFields_get hs.2 hj
        exact ih c hc h3 (by
          rw [assignable_isValue h2]
          exact closureType?_captured_value hct t (List.mem_of_getElem? h1)) w hwc
      · simp at hs
  | table st fs ks es ihf _ ihe =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · rfl
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped, Bool.and_eq_true, Bool.or_eq_true, List.isEmpty_iff] at hs
      obtain ⟨⟨-, hf⟩, hes⟩ := hs
      simp only [Value.children, List.mem_append] at hc
      rcases hc with hc | hc
      · split at hf
        · next tys htys =>
          obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
          obtain ⟨t, h1, h2, h3⟩ := shapedFields_get hf hj
          exact ihf c hc h3 (by
            rw [assignable_isValue h2]
            exact fieldTypes_values (fieldTypes_args htys) htys _ (List.mem_of_getElem? h1)) w hwc
        · simp at hf
      · rcases hes with rfl | ⟨-, he⟩
        · simp at hc
        · split at he
          · next k v hargs =>
            simp only [Bool.and_eq_true] at he
            have := (shapedAll_iff.1 he.1) c hc
            exact ihe c hc this.2 (by rw [assignable_isValue this.1]; rfl) w hwc
          · simp at he
  | nil => rename_i hc _ _ _ _; simp at hc
  | cons x xs ihx ihxs =>
    rename_i c hc hcs hct w hw
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ihx hcs hct w hw
    · exact ihxs c hc hcs hct w hw
  | ref r => simp [Value.type, RuntimeType.isValue] at hv
  | signer a =>
    intro w hw
    simp [subvalues, Value.nodes] at hw
    rcases hw with rfl | rfl <;> rfl
  | _ =>
    intro w hw
    rw [subvalues_eq] at hw
    simp only [Value.children, List.flatMap_nil, List.mem_singleton] at hw
    subst hw
    rfl

theorem not_ref_of_isRef {w : Value} (h : w.isRef = false) : ∀ r, w ≠ .ref r := by
  rintro r rfl; simp [Value.isRef] at h

end NoRef

/-! ## Native calls in the semantics -/

/-- A partial step that does not call a native. -/
def NotNative : Step.Exec StepResult → Prop
  | .ok (.callNative ..) => False
  | .ok (.done _) => True
  | .error _ => True

theorem notNative_bind {α : Type} {x : Step.Exec α} {g : α → Step.Exec StepResult}
    (h : ∀ a, x = .ok a → NotNative (g a)) : NotNative (x >>= g) := by
  cases hx : x with
  | error e => exact trivial
  | ok a => exact h a hx

theorem notNative_absurd {x : Step.Exec StepResult} {s' : State} {f : FunctionRef}
    {args views : List Value} {C : Prop} (hn : NotNative x)
    (h : x = .ok (.callNative s' f args views)) : C := by
  subst h; exact hn.elim

/-- Discharges `NotNative` for a partial step built from binds, matches,
conditionals, `next`, `wrong` and `fail`. -/
macro "not_native" : tactic =>
  `(tactic| repeat' (first
    | exact trivial
    | (refine notNative_bind fun _ _ => ?_)
    | (unfold finish)
    | (unfold packStruct)
    | (unfold unpackStruct)
    | (unfold packEnum)
    | (unfold unpackEnum)
    | (unfold testEnum)
    | (unfold existsResource)
    | (unfold moveResourceFrom)
    | (unfold moveResourceTo)
    | (unfold borrowGlobal)
    | (unfold borrowField)
    | (unfold borrowVariantField)
    | split
    | dsimp only))

/-- **What the semantics checked of a native call** it suspends with state
`s'`, native `f`, arguments `args` and their views `views`: `f` is a native
instance whose type arguments satisfy its constraints, the arguments conform
to its parameter types, every reference argument is valid, no mutable one
overlaps another, and a native that is not modeled takes and returns values
only. -/
structure NativeCallFacts (P : Program) (s' : State) (f : FunctionRef) (args views : List Value) :
    Prop where
  view : s'.nativeView args = some views
  exclusive : Step.exclusiveArguments args = true
  info : ∃ info, P.function? f = some info ∧ info.definition.code = none
  typeArgs : P.typeArgsOk f = true
  params : ∃ ptys, P.paramTypes f = some ptys ∧ conformsAll P args ptys = true
  returns : ∃ tys, P.returnTypes f = some tys
  values : P.nativeShape? f = none →
    (∀ ptys, P.paramTypes f = some ptys → ∀ t ∈ ptys, t.isValue = true) ∧
    (∀ tys, P.returnTypes f = some tys → ∀ t ∈ tys, t.isValue = true)

/-- A call of a native pops its arguments. -/
theorem call_callNative {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {hd : FunctionHandleIndex} {tys : List RuntimeType} {f : FunctionRef}
    {args views : List Value}
    (h : Step.call P s fr rest info hd tys = .ok (.callNative s' f args views)) :
    ∃ n, n ≤ fr.stack.length ∧ args = (fr.stack.take n).reverse ∧
      s' = { s with frames := { fr with stack := fr.stack.drop n } :: rest } ∧
      NativeCallFacts P s' f args views := by
  unfold Step.call at h
  rw [bind_ok'] at h
  obtain ⟨callee, -, h⟩ := h
  split at h
  · simp [Step.wrong] at h
  rw [bind_ok'] at h
  obtain ⟨calleeInfo, hci, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨fh, -, h⟩ := h
  split at h
  · simp [Step.wrong] at h
  split at h
  · simp [Step.wrong] at h
  split at h
  · simp [Step.wrong] at h
  next hwfa =>
  rw [bind_ok'] at h
  obtain ⟨paramTys, hpt, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨retTys, hrt, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨_, -, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨_, -, h⟩ := h
  split at h
  · simp [Step.wrong] at h
  split at h
  · simp [Step.wrong] at h
  next hval =>
  rw [bind_ok'] at h
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := h
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · simp [Step.wrong] at h
  next hconf =>
  split at h
  · next hnone =>
    rw [bind_ok'] at h
    obtain ⟨views', hviews, h⟩ := h
    split at h
    · simp [Step.wrong] at h
    next hexcl =>
    simp only [pure, Except.pure, Except.ok.injEq, StepResult.callNative.injEq] at h
    obtain ⟨rfl, rfl, rfl, rfl⟩ := h
    have hci' := need_ok' hci
    refine ⟨_, hn, rfl, rfl, need_ok' hviews, by simpa using hexcl,
      ⟨calleeInfo, by rw [function?_typeArgs]; exact hci', hnone⟩,
      by simp only [Program.typeArgsOk, function?_typeArgs, hci']; simpa using hwfa,
      ⟨paramTys, need_ok' hpt, by simpa using hconf⟩, ⟨retTys, need_ok' hrt⟩, fun hsh => ?_⟩
    have hall : (paramTys ++ retTys).all RuntimeType.isValue = true := by
      simpa [hnone, hsh] using hval
    refine ⟨fun ptys hp t ht => ?_, fun tys hr t ht => ?_⟩
    · rw [need_ok' hpt, Option.some.injEq] at hp
      subst hp
      exact List.all_eq_true.1 hall t (List.mem_append_left _ ht)
    · rw [need_ok' hrt, Option.some.injEq] at hr
      subst hr
      exact List.all_eq_true.1 hall t (List.mem_append_right _ ht)
  · rw [bind_ok'] at h
    obtain ⟨_, -, h⟩ := h
    split at h <;> split at h <;> simp [Step.fail, pure, Except.pure] at h

set_option maxHeartbeats 1000000 in
theorem exec_callNative {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode} {f : FunctionRef}
    {args views : List Value}
    (h : Step.exec P s fr rest info code instr = .ok (.callNative s' f args views)) :
    ∃ n, n ≤ fr.stack.length ∧ args = (fr.stack.take n).reverse ∧
      s' = { s with frames := { fr with stack := fr.stack.drop n } :: rest } ∧
      NativeCallFacts P s' f args views := by
  cases instr
  case call hd => exact call_callNative h
  case callGeneric i =>
    unfold Step.exec at h
    rw [bind_ok'] at h
    obtain ⟨x, -, h⟩ := h
    rw [bind_ok'] at h
    obtain ⟨tys, -, h⟩ := h
    exact call_callNative h
  case ret =>
    refine notNative_absurd ?_ h
    unfold Step.exec Step.ret
    not_native
  case packClosure hd mask =>
    refine notNative_absurd ?_ h
    show NotNative (Step.packClosure P s fr rest info hd mask)
    unfold Step.packClosure
    not_native
  case packClosureGeneric i mask =>
    refine notNative_absurd ?_ h
    show NotNative (do
      let x ← need info.module.functionInstantiations[i]? .badIndex
      Step.packClosure P s fr rest info x.handle mask
        (← genericArgs info.module fr.function.typeArgs x.typeParameters))
    refine notNative_bind fun _ _ => ?_
    refine notNative_bind fun _ _ => ?_
    unfold Step.packClosure
    not_native
  case callClosure sig =>
    refine notNative_absurd ?_ h
    show NotNative (Step.callClosure P s fr rest info sig)
    unfold Step.callClosure
    not_native
  all_goals
    refine notNative_absurd ?_ h
    unfold Step.exec
    not_native

/-- A step that calls a native pops its arguments from the current frame. -/
theorem step_callNative {P : Program} {s s' : State} {f : FunctionRef} {args views : List Value}
    (h : step P s = .callNative s' f args views) :
    ∃ fr rest n, s.frames = fr :: rest ∧ n ≤ fr.stack.length ∧
      args = (fr.stack.take n).reverse ∧
      s' = { s with frames := { fr with stack := fr.stack.drop n } :: rest } ∧
      NativeCallFacts P s' f args views := by
  unfold step at h
  split at h
  · simp at h
  · next fr rest hsf =>
    split at h
    · simp at h
    · split at h
      · simp at h
      · split at h
        · simp at h
        · split at h
          · next r hr =>
            subst h
            obtain ⟨n, hn, hargs, hs', hfacts⟩ := exec_callNative hr
            exact ⟨fr, rest, n, hsf, hn, hargs, hs', hfacts⟩
          · simp at h

/-- A native a step calls is modeled or not; the native names it. -/
theorem callNative_shape {P : Program} {s s' : State} {f : FunctionRef} {args views : List Value}
    (_ : step P s = .callNative s' f args views) :
    P.nativeShape? f = none ∨ ∃ k, P.nativeShape? f = some k := by
  cases P.nativeShape? f with
  | none => exact Or.inl rfl
  | some k => exact Or.inr ⟨k, rfl⟩

/-- **What a resumed native call that continues did**: its result fit its
shape, its writes went back through its mutable reference arguments, and
the references for its places, interleaved with its values, were pushed. -/
theorem resume_spec {P : Program} {s' : State} {f : FunctionRef}
    {args vals writes : List Value} {places : List NativePlace}
    (hc : (resumeNative P s' f args (.returned vals writes places)).continues = true) :
    ∃ tys views s₁ results fr rest, P.returnTypes f = some tys ∧
      s'.nativeView args = some views ∧
      P.nativeResultFits f tys views vals writes places = true ∧
      writeBack s' (P.writtenReferences f args) writes = some s₁ ∧
      (placeReferences s₁ tys args places).bind (nativeResultsIn tys vals) = some results ∧
      s₁.frames = fr :: rest ∧
      resumeNative P s' f args (.returned vals writes places) =
        .running { s₁ with frames := { fr with stack := results.reverse ++ fr.stack, pc := fr.pc + 1 } :: rest } := by
  simp only [resumeNative] at hc ⊢
  cases hsf : s'.frames with
  | nil => simp [hsf, Outcome.continues] at hc
  | cons fr₀ rest₀ =>
  simp only [hsf] at hc ⊢
  cases htys : P.returnTypes f with
  | none => simp [htys, Outcome.continues] at hc
  | some tys =>
  simp only [htys] at hc ⊢
  cases hviews : s'.nativeView args with
  | none => simp [hviews, Outcome.continues] at hc
  | some views =>
  simp only [hviews] at hc ⊢
  by_cases hfits : P.nativeResultFits f tys views vals writes places = true
  case neg => simp [hfits, Outcome.continues] at hc
  simp only [hfits, Bool.not_true, Bool.false_eq_true, ↓reduceIte] at hc ⊢
  cases hwb : writeBack s' (P.writtenReferences f args) writes with
  | none => simp [hwb, Outcome.continues] at hc
  | some s₁ =>
  simp only [hwb] at hc ⊢
  cases hres : (placeReferences s₁ tys args places).bind (nativeResultsIn tys vals) with
  | none => simp [hres, Outcome.continues] at hc
  | some results =>
  simp only [hres] at hc ⊢
  cases hs₁ : s₁.frames with
  | nil => simp [hs₁, Outcome.continues] at hc
  | cons fr rest =>
  refine ⟨tys, views, s₁, results, fr, rest, by first | rfl | assumption,
    by first | rfl | assumption, hfits, by first | rfl | assumption,
    by first | rfl | assumption, hs₁, ?_⟩
  simp only

theorem setRoot_nextFrame (s : State) (root : Root) (v : Value) :
    (s.setRoot root v).nextFrame = s.nextFrame := by
  cases root <;> rfl

/-- Writing back keeps the next frame identity. -/
theorem writeBack_nextFrame : ∀ {s s' : State} {refs : List Reference} {writes : List Value},
    writeBack s refs writes = some s' → s'.nextFrame = s.nextFrame
  | s, s', [], [], h => by simp only [writeBack, Option.some.injEq] at h; rw [h]
  | s, s', r :: refs, w :: writes, h => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₁, hs₁, h⟩ := h
    rw [writeBack_nextFrame h]
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hs₁
    obtain ⟨_, -, _, -, rfl⟩ := hs₁
    exact setRoot_nextFrame _ _ _
  | _, _, [], _ :: _, h => by simp [writeBack] at h
  | _, _, _ :: _, [], h => by simp [writeBack] at h

/-- A resumed native call keeps the next frame identity. -/
theorem resume_nextFrame {P : Program} {s₁ s' : State} {f : FunctionRef} {args : List Value}
    {r : NativeResult} (h : resumeNative P s₁ f args r = .running s') :
    s'.nextFrame = s₁.nextFrame := by
  cases r with
  | failed e => simp [resumeNative] at h
  | returned vals writes places =>
    have hc : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
      rw [h]; rfl
    obtain ⟨_, _, sW, _, _, _, -, -, -, hwb, -, -, ho⟩ := resume_spec hc
    rw [ho, Outcome.running.injEq] at h
    subst h
    have := writeBack_nextFrame hwb
    exact this

/-- The outcome of a call of a native that is not modeled, on arguments that
are values, that continues. -/
theorem resume_running {P : Program} {s' : State} {f : FunctionRef}
    {args vals writes : List Value} {places : List NativePlace}
    {fr : Frame} {rest : List Frame} (hs : s'.frames = fr :: rest)
    (hshape : P.nativeShape? f = none) (hargs : ∀ v ∈ args, ∀ r, v ≠ .ref r)
    (hvals : ∀ tys, P.returnTypes f = some tys → ∀ t ∈ tys, t.isValue = true)
    (hc : (resumeNative P s' f args (.returned vals writes places)).continues = true) :
    resumeNative P s' f args (.returned vals writes places) =
      .running { s' with frames := { fr with
        stack := vals.reverse ++ fr.stack
        pc := fr.pc + 1 } :: rest } ∧
      (∃ tys, P.returnTypes f = some tys ∧ conformsAll P vals tys = true) ∧
      writes = [] ∧ places = [] := by
  obtain ⟨tys, -, -, -, -, -, htys, -⟩ := resume_spec hc
  rw [resumeNative_values hs htys hshape hargs (hvals tys htys)] at hc ⊢
  split at hc
  · next hfit =>
    simp only [Bool.and_eq_true, List.isEmpty_iff] at hfit
    obtain ⟨⟨hconf, rfl⟩, rfl⟩ := hfit
    rw [if_pos (by simp [hconf])]
    exact ⟨rfl, ⟨tys, htys, hconf⟩, rfl, rfl⟩
  · simp [Outcome.continues] at hc

/-- The values a native that is not modeled returns, when the execution
continues: shaped values of value types, without references inside. -/
theorem native_values {P : Program} {s s' : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace}
    (hstep : step P s = .callNative s' f args views) (hshape : P.nativeShape? f = none)
    (hcont : (resumeNative P s' f args (.returned vals writes places)).continues = true) :
    ∀ v ∈ vals, shaped P v = true ∧ v.type.isValue = true := by
  obtain ⟨fr, rest, n, -, -, -, rfl, hfacts⟩ := step_callNative hstep
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  obtain ⟨-, ⟨tys, htys, hconf⟩, -⟩ := resume_running (P := P) (f := f) (vals := vals)
    (s' := { s with frames := { fr with stack := fr.stack.drop n } :: rest }) rfl hshape
    (noRef_of_conformsAll hconfa (hv.1 _ hpt)) hv.2 hcont
  intro v hv'
  obtain ⟨t, ht, hc⟩ := conformsAll_mem hconf v hv'
  obtain ⟨hs, hty⟩ := conforms_shaped hc
  exact ⟨hs, by rw [assignable_isValue hty]; exact hv.2 tys htys t ht⟩

/-! ## Natives that write back and borrow -/

section NativeTyping

variable {P : Program}

/-- A reference argument a native sees is valid. -/
theorem nativeView_deref {s : State} :
    ∀ {args views : List Value}, s.nativeView args = some views → ∀ r, Value.ref r ∈ args →
      ∃ v, s.deref? r.location r.since = some v
  | [], _, _, _, hr => by simp at hr
  | a :: args, views, h, r, hr => by
    unfold State.nativeView at h
    rw [List.mapM_cons] at h
    simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨b, hb, bs, hbs, -⟩ := h
    rcases List.mem_cons.1 hr with rfl | hr
    · exact ⟨b, hb⟩
    · exact nativeView_deref hbs r hr

theorem setRoot_frames_fn (s : State) (ρ : Root) (v : Value) :
    (s.setRoot ρ v).frames.map (fun fr => (fr.id, fr.function)) =
      s.frames.map (fun fr => (fr.id, fr.function)) := by
  cases ρ <;> simp [State.setRoot, Function.comp_def]
  intro a _
  split <;> simp

/-- A write keeps the type of every root but a held value written whole. -/
theorem rootType_write {s s₁ : State} {l : Location} {w : Value} (hwr : s.write? l w = some s₁)
    {ρ : Root} (hρ : ρ ≠ l.root ∨ l.path ≠ []) : rootType P s₁ ρ = rootType P s ρ := by
  simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hwr
  obtain ⟨rv, hrv, v'', hset, rfl⟩ := hwr
  cases ρ with
  | local_ f i =>
    have h := congrFun (rootType_congr (P := P) (s := s) (s' := { s with frames :=
      (s.setRoot l.root v'').frames }) (setRoot_frames_fn s l.root v'') rfl) (.local_ f i)
    simp only [rootType] at h ⊢
    exact h
  | global a t => rfl
  | held k =>
    simp only [rootType]
    cases hl : l.root with
    | held k' =>
      simp only [State.setRoot]
      by_cases hk : k = k'
      · subst hk
        have hp : l.path ≠ [] := by
          rcases hρ with h | h
          · exact absurd hl.symm h
          · exact h
        rw [hl] at hrv
        simp only [State.root?] at hrv
        obtain ⟨st, q, hq⟩ : ∃ st q, l.path = st :: q := by
          cases hpp : l.path with
          | nil => exact absurd hpp hp
          | cons st q => exact ⟨st, q, rfl⟩
        rw [hq] at hset
        have hty := set?_type_cons hset
        have hlt : k < s.held.length := (List.getElem?_eq_some_iff.1 hrv).1
        have hrv' : s.held[k] = rv := by
          rw [List.getElem?_eq_getElem hlt, Option.some.injEq] at hrv; exact hrv
        simp [hrv', hlt, hty]
      · rw [List.getElem?_set_ne (Ne.symm hk)]
    | local_ _ _ => simp [State.setRoot]
    | global _ _ => simp [State.setRoot]

/-- A typed reference stays typed across a write it does not overlap, when the
state after it is typed. -/
theorem refTyped_write {s s₁ : State} {l : Location} {w : Value} (hwr : s.write? l w = some s₁)
    (ht₁ : StateTyped P s₁) {r : Reference} (hr : RefTyped P s r)
    (hno : l.overlaps r.location = false) : RefTyped P s₁ r := by
  have hρ : r.location.root ≠ l.root ∨ l.path ≠ [] := by
    by_cases he : r.location.root = l.root
    · right
      intro hp
      simp [Location.overlaps, he, hp] at hno
    · exact Or.inl he
  have hnf : s₁.nextFrame = s.nextFrame := by
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hwr
    obtain ⟨_, -, _, -, rfl⟩ := hwr
    exact setRoot_nextFrame _ _ _
  refine ⟨fun T hT => hr.1 T (by rw [← rootType_write hwr hρ]; exact hT), fun f i hf => by
    rw [hnf]; exact hr.2.1 f i hf, fun k v hk hv => (ht₁.held v (List.mem_of_getElem? hv)).1⟩

/-- A write through a typed mutable reference, at a writable path, keeps the
state invariant without held values' abilities. -/
theorem inv_write {s s₁ : State} (hinv : StateInv false P s) (ht : StateTyped P s)
    {r : Reference} (hrt : RefTyped P s r) {w : Value} (hty : w.type.assignable r.target = true)
    (hwr : s.write? r.location w = some s₁) : StateInv false P s₁ := by
  simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hwr
  obtain ⟨rv, hrv, v'', hset, rfl⟩ := hwr
  refine ⟨?_, ?_, ?_, ?_, fun h => by cases h⟩
  · rw [setRoot_ids]; exact hinv.frameIds
  · intro fr hfr
    rw [setRoot_nextFrame]
    have hm : fr.id ∈ (s.setRoot r.location.root v'').frames.map (·.id) := List.mem_map_of_mem hfr
    rw [setRoot_ids] at hm
    obtain ⟨g, hg, he⟩ := List.mem_map.1 hm
    rw [← he]; exact hinv.framesBelow g hg
  · rw [setRoot_resources]; exact hinv.resources
  · refine setRoot_storage_typed hinv hrv (fun a t hroot => ?_)
    cases hp : r.location.path with
    | cons st q => rw [hp] at hset; exact set?_type_cons hset
    | nil =>
      rw [hp] at hset
      simp only [Value.set?, Option.some.injEq] at hset
      subst hset
      have hrv' : (s.storage.get? a t) = some rv := by rw [hroot] at hrv; exact hrv
      obtain ⟨T, hT, hrvT, -⟩ := root_typed ht hinv hrt hrv
      rw [hroot] at hT
      simp only [rootType, Option.some.injEq] at hT
      subst hT
      have hpt : r.target = .struct t := by
        rcases hrt.1 (.struct t) (by rw [hroot]; rfl) with h | ⟨⟨k, hk⟩, -, -, -⟩
        · rw [hp] at h; simp only [pathType, Option.some.injEq] at h; exact h.symm
        · rw [hroot] at hk; cases hk
      rw [hpt] at hty
      rw [assignable_rigid hty rfl, assignable_rigid hrvT rfl]

theorem StateInv.weaken {σ : Bool} {s : State} (h : StateInv σ P s) : StateInv false P s :=
  ⟨h.frameIds, h.framesBelow, h.resources, h.typed, fun e => by cases e⟩

theorem inv_invalidate {σ : Bool} {s : State} (h : StateInv σ P s) (e : Invalidation) :
    StateInv σ P (s.invalidate e) :=
  ⟨h.frameIds, h.framesBelow, h.resources, h.typed, h.heldDrop⟩

/-- **Writing back keeps the typing invariant**: each write through a typed,
valid mutable reference, of a value conforming to its target, at a location no
other written reference overlaps. -/
theorem typed_writeBack : ∀ {s s' : State} {refs : List Reference} {writes : List Value},
    StateTyped P s → StateInv false P s →
    (∀ r ∈ refs, RefTyped P s r ∧ r.target.isValue = true ∧ (s.read? r.location).isSome = true) →
    refs.Pairwise (fun a b => a.location.overlaps b.location = false) →
    conformsAll P writes (refs.map (·.target)) = true →
    writeBack s refs writes = some s' → StateTyped P s' ∧ StateInv false P s'
  | s, s', [], [], ht, hinv, _, _, _, h => by
    simp only [writeBack, Option.some.injEq] at h
    subst h
    exact ⟨ht, hinv⟩
  | s, s', r :: refs, w :: writes, ht, hinv, hrefs, hdis, hconf, h => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₁, hs₁, h⟩ := h
    simp only [List.map_cons, conformsAll_cons] at hconf
    obtain ⟨hw, hconf⟩ := hconf
    obtain ⟨hrt, htv, hread⟩ := hrefs r List.mem_cons_self
    obtain ⟨v, hv⟩ := Option.isSome_iff_exists.1 hread
    obtain ⟨hws, hwt⟩ := conforms_shaped hw
    have ht₁ := typed_write ht hinv hrt htv hv hws hwt hs₁
    have hinv₁ := inv_write hinv ht hrt hwt hs₁
    rw [List.pairwise_cons] at hdis
    refine typed_writeBack (typed_invalidate ht₁ _) (inv_invalidate hinv₁ _) (fun r' hr' => ?_)
      hdis.2 hconf h
    obtain ⟨hrt', htv', hread'⟩ := hrefs r' (List.mem_cons_of_mem _ hr')
    refine ⟨refTyped_write hs₁ ht₁ hrt' (hdis.1 r' hr'), htv', ?_⟩
    show (s₁.read? r'.location).isSome = true
    rw [read_write_disjoint hs₁ (hdis.1 r' hr')]
    exact hread'
  | _, _, [], _ :: _, _, _, _, _, _, h => by simp [writeBack] at h
  | _, _, _ :: _, [], _, _, _, _, _, h => by simp [writeBack] at h

theorem pathType_eq_placeType : ∀ (t : RuntimeType) (q : List PathStep),
    pathType P t q = RuntimeType.placeType? P t q
  | _, [] => rfl
  | t, st :: q => by
    simp only [pathType, RuntimeType.placeType?, bind]
    have : stepType P t st = (match t, st with
      | .struct s, .field i => (P.fieldTypes s).bind (·[i]?)
      | .struct s, .variantField tag i => (P.variantFieldTypes s tag).bind (·[i]?)
      | .vector elem, .element _ => some elem
      | .signer, .signerAddress => some .address
      | .struct s, .tableEntry _ =>
        match s.isTable, s.typeArgs with
        | true, [_, v] => some (tableEntryType v)
        | _, _ => none
      | _, _ => none) := by
      cases t <;> cases st <;> rfl
    rw [this]
    congr 1
    funext t'
    exact pathType_eq_placeType t' q

theorem pathType_append' (T : RuntimeType) : ∀ (p q : List PathStep),
    pathType P T (p ++ q) = (pathType P T p).bind (pathType P · q)
  | [], q => by simp [pathType]
  | st :: p, q => by
    simp only [List.cons_append, pathType, bind]
    cases stepType P T st with
    | none => rfl
    | some t => exact pathType_append' t p q

/-- **The reference a borrow-shape native returns is typed**: its place's
declared type inside the argument's target is the result's target. -/
theorem typed_place {s : State} {ra : Reference} (hrt : RefTyped P s ra) {X tgt : RuntimeType}
    (hX : ra.target = X) (hrig : rigid X = true) {path : List PathStep}
    (hp : RuntimeType.placeType? P X path = some tgt) (mutable : Bool) (since : Nat) :
    RefTyped P s ⟨mutable, tgt, ra.location.descend path, since⟩ := by
  refine ⟨fun T hT => Or.inl ?_, fun f i hf => hrt.2.1 f i hf, fun k v hk hv => hrt.2.2 k v hk hv⟩
  have hT' := hrt.1 T hT
  rcases hT' with h | ⟨-, -, -, h⟩
  · simp only [Location.descend]
    rw [pathType_append', h, Option.bind_some, hX, pathType_eq_placeType]
    exact hp
  · rw [hX, hrig] at h; cases h

end NativeTyping

/-! ## Steps keep the typing invariant -/

section Step

variable {P : Program} {s : State} {o : Outcome}

theorem typed_core (ht : StateTyped P s) (hinv : StateInv σ P s) (hstep : step P s = .done o)
    (hcont : o.continues = true) : PostTyped P o := by
  unfold step at hstep
  split at hstep
  · simp only [StepResult.done.injEq] at hstep; subst hstep; simp [Outcome.continues] at hcont
  · next fr rest hsf =>
    split at hstep
    · simp only [StepResult.done.injEq] at hstep; subst hstep; simp [Outcome.continues] at hcont
    · next info _ =>
      split at hstep
      · simp only [StepResult.done.injEq] at hstep; subst hstep; simp [Outcome.continues] at hcont
      · next code hcode =>
        split at hstep
        · simp only [StepResult.done.injEq] at hstep; subst hstep
          simp [Outcome.continues] at hcont
        · next instr hinstr =>
          have hstep' : stepOf (Step.exec P s fr rest info code instr) = .done o := hstep
          exact stateTyped_exec ht hinv hsf hcont hstep'

theorem typed_native {σ : Bool} {s' : State} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace} (ht : StateTyped P s) (hinv : StateInv σ P s)
    (hstep : step P s = .callNative s' f args views)
    (hcont : (resumeNative P s' f args (.returned vals writes places)).continues = true) :
    PostTyped P (resumeNative P s' f args (.returned vals writes places)) := by
  obtain ⟨fr, rest, n, hsf, hnl, hargs, rfl, hfacts⟩ := step_callNative hstep
  cases hshape : P.nativeShape? f with
  | some k =>
    obtain ⟨tys, views', sW, results, fr', rest', htys, -, hfits, hwb, hres, hsW, ho⟩ :=
      resume_spec hcont
    rw [ho]
    show StateTyped P _
    obtain ⟨ht₁, hinv₁⟩ := typed_popped ht hinv hsf n
    obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
    have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self
    have hargRef : ∀ r, Value.ref r ∈ args →
        RefTyped P { s with frames := { fr with stack := fr.stack.drop n } :: rest } r ∧
        r.target.isValue = true ∧
        (({ s with frames := { fr with stack := fr.stack.drop n } :: rest } : State).read?
          r.location).isSome = true := by
      intro r hr
      have hrs : Value.ref r ∈ fr.stack := by
        rw [hargs] at hr; exact List.mem_of_mem_take (List.mem_reverse.1 hr)
      have hrt := ht.refs fr hfr r (mem_frameVals_stack hrs)
      refine ⟨refTyped_congr hrt (rootType_congr (by simp [hsf]) rfl) (Nat.le_refl _)
        (held_same rfl), ?_, ?_⟩
      · simpa [shaped] using ht.valsShaped fr hfr _ (mem_frameVals_stack hrs)
      · obtain ⟨v, hv⟩ := nativeView_deref hfacts.view r hr
        rw [deref?_read hv]; rfl
    -- The values written back.
    have hwbT : StateTyped P sW := by
      by_cases hku : k = .update
      · subst hku
        rw [writtenReferences_update hshape] at hwb
        have hfit' := hfits
        simp only [Program.nativeResultFits, hshape, hpt, Bool.and_eq_true] at hfit'
        exact (typed_writeBack ht₁ hinv₁.weaken
          (fun r hr => hargRef r (mem_mutRefsOf hr).1) (mutRefs_disjoint hfacts.exclusive)
          (by rw [mutRefs_targets hconfa]; exact hfit'.1.2) hwb).1
      · rw [writtenReferences_other (by rw [hshape]; simpa using hku)] at hwb
        cases writes with
        | nil =>
          simp only [writeBack, Option.some.injEq] at hwb
          subst hwb
          exact ht₁
        | cons => simp [writeBack] at hwb
    -- The results.
    have hresT : ∀ v ∈ results, shaped P v = true ∧ ∀ r, v = .ref r → RefTyped P sW r := by
      by_cases hkb : k = .borrow
      · subst hkb
        obtain ⟨rfl, rfl, m, am, X, tgt, p, rfl, rfl, harg, hpX, -, hrig, hXt, hplace, -, -⟩ :=
          borrow_result hshape hpt htys (typeArgsOk_values hfacts.typeArgs) hfits
        rw [writtenReferences_other (by rw [hshape]; simp)] at hwb
        simp only [writeBack, Option.some.injEq] at hwb
        subst hwb
        obtain ⟨ra, hra, -, hrX⟩ := conformsAll_ref_at hconfa hpX
        have hrX' : ra.target = X := assignable_rigid hrX hrig
        obtain ⟨hrta, htva, -⟩ := hargRef ra (List.mem_of_getElem? hra)
        have hpr : placeReferences { s with frames := { fr with stack := fr.stack.drop n } :: rest }
            [.ref m tgt] args [p] = some [Step.borrow
              { s with frames := { fr with stack := fr.stack.drop n } :: rest } m tgt
              (ra.location.descend p.path)] := by
          simp [placeReferences, harg, hra, RuntimeType.isValue]
        rw [hpr, Option.bind_some] at hres
        simp [nativeResultsIn, RuntimeType.isValue] at hres
        subst hres
        intro v hv
        simp only [List.mem_singleton] at hv
        subst hv
        have htgt : tgt.isValue = true := by
          rcases hXt with rfl | ⟨rfl, rfl⟩ | ⟨κ, ν, rfl, rfl⟩
          · rw [hrX'] at htva; simpa [RuntimeType.isValue] using htva
          · rfl
          · rfl
        refine ⟨by simpa [Step.borrow, shaped] using htgt, fun r hr => ?_⟩
        simp only [Step.borrow, Value.ref.injEq] at hr
        subst hr
        exact typed_place hrta hrX' hrig hplace _ _
      · have hfit' := hfits
        have hpl : places = [] := by
          cases k <;> simp_all [Program.nativeResultFits]
        have hcv : conformsAll P vals tys = true := by
          cases k <;> simp_all [Program.nativeResultFits]
        subst hpl
        have hvt := returns_values hshape hkb htys (typeArgsOk_values hfacts.typeArgs)
        simp only [placeReferences_nil, Option.bind_some,
          nativeResultsIn_values hvt (conformsAll_length hcv), Option.some.injEq] at hres
        subst hres
        intro v hv
        obtain ⟨t, ht', hc⟩ := conformsAll_mem hcv v hv
        obtain ⟨hs, hty⟩ := conforms_shaped hc
        exact ⟨hs, fun r hr => by
          subst hr
          have := hvt t ht'
          rw [← assignable_isValue hty] at this
          simp [Value.type, RuntimeType.isValue] at this⟩
    exact typed_next (s := sW) (fr := fr') (rest := rest')
      (fr' := { fr' with stack := results.reverse ++ fr'.stack, pc := fr'.pc + 1 })
      (new := results.reverse) (n := 0) hwbT hsW
      (fun v hv => hresT v (List.mem_reverse.1 hv)) (by simp) rfl rfl rfl hwbT.storageShaped
      rfl rfl
  | none =>
  have hvals := native_values hstep hshape hcont
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  rw [(resume_running (fr := { fr with stack := fr.stack.drop n }) (rest := rest) rfl hshape
    (noRef_of_conformsAll hconfa (hv.1 _ hpt)) hv.2 hcont).1]
  show StateTyped P _
  exact typed_next (fr' := { fr with stack := vals.reverse ++ fr.stack.drop n, pc := fr.pc + 1 })
    (new := vals.reverse) (n := n) ht hsf
    (fun v hv => plain_new (hvals v (List.mem_reverse.1 hv)).1
      (not_ref_of_type rfl (hvals v (List.mem_reverse.1 hv)).2))
    rfl rfl rfl rfl ht.storageShaped rfl rfl

end Step


/-- **References occur only at the top of the frames' slots** in a typed
state. -/
theorem refsTop_of_typed {s : State} (ht : StateTyped P s) (hinv : StateInv σ P s) : RefsTop s where
  targets := by
    intro fr hfr r hr
    have hm : Value.ref r ∈ frameVals fr := by
      rcases hr with hr | hr
      · exact mem_frameVals_stack hr
      · simp only [frameVals, List.mem_append, List.mem_filterMap, id]; exact Or.inr ⟨_, hr, rfl⟩
    simpa [shaped] using ht.valsShaped fr hfr _ hm
  storage := by
    intro r hr
    exact shaped_noRef r.value (ht.storageShaped r hr)
      (by rw [hinv.typed r hr]; rfl)
  deref := by
    intro fr hfr r hr v hv
    have hm : Value.ref r ∈ frameVals fr := by
      rcases hr with hr | hr
      · exact mem_frameVals_stack hr
      · simp only [frameVals, List.mem_append, List.mem_filterMap, id]; exact Or.inr ⟨_, hr, rfl⟩
    obtain ⟨hsh, hval, -⟩ := read_typed ht hinv hfr hm hv
    exact shaped_noRef v hsh hval

end MoveBytecode.Proofs
