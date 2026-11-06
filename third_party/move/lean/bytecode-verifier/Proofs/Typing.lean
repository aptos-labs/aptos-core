-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Acceptance
import Proofs.ByValue
import Proofs.TypeSafety
import Proofs.Values

/-!
# The type certificate C1 from the model

UNTRUSTED (checked by the kernel). The model's type-safety pass computes an
abstract stack of signature tokens before each instruction (`typesBefore`).
On the supported subset every such token translates to a runtime type, and
translating the stacks gives a certificate that is consistent in the
specification's sense (`TypeCertificate.Consistent`): the model's acceptance
of an instruction implies the specification's typing rule `typeAfter`, and
the stacks agree along every edge.

The correspondence is instruction by instruction. The model's checks are
written against signature tokens (`isAssignableFrom`, `==`, `abilities`), the
specification's against runtime types. On the subset, assignability is token
equality, and equal tokens translate to equal types. Abilities differ for
struct types: the model reads those the module's own handle declares, the
specification those of the definition. K1 makes the former at most the
latter, so every ability the model requires holds in the specification too
(`abilities_sub`). The facts about struct handles and definitions that link
checks establish are collected in `StructsAgree`, which `Proofs.Linking`
proves.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Signature tokens of the subset -/

section Tokens

variable {m : CompiledModule}

theorem ofSignature_nil : RuntimeType.ofSignature m [] = some [] := rfl

/-- A value type translates the same at the top of a slot. -/
theorem ofSlotToken_of_ofToken {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofToken m t = some u) : RuntimeType.ofSlotToken m t = some u := by
  cases t <;> first | exact h | (simp [RuntimeType.ofToken, RuntimeType.ofTokenWith] at h)

/-- A slot token that is not a reference translates as a value type. -/
theorem ofToken_of_ofSlotToken {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofSlotToken m t = some u) (hr : t.isRefToken = false) :
    RuntimeType.ofToken m t = some u := by
  cases t <;> first | exact h | (simp [SignatureToken.isRefToken] at hr)

theorem ofSignature_cons {t : SignatureToken} {ts : List SignatureToken} {r : List RuntimeType} :
    RuntimeType.ofSignature m (t :: ts) = some r ↔
      ∃ u us, RuntimeType.ofSlotToken m t = some u ∧ RuntimeType.ofSignature m ts = some us ∧
        r = u :: us := by
  simp only [RuntimeType.ofSignature, RuntimeType.ofSignatureWith, List.mapM_cons, bind, Option.bind_eq_some_iff, pure,
    Option.some.injEq]
  constructor
  · rintro ⟨u, hu, us, hus, rfl⟩; exact ⟨u, us, hu, hus, rfl⟩
  · rintro ⟨u, us, hu, hus, rfl⟩; exact ⟨u, hu, us, hus, rfl⟩

theorem ofSignature_cons_slot {t : SignatureToken} {ts : List SignatureToken} {u : RuntimeType}
    {us : List RuntimeType} (hu : RuntimeType.ofSlotToken m t = some u)
    (hus : RuntimeType.ofSignature m ts = some us) :
    RuntimeType.ofSignature m (t :: ts) = some (u :: us) :=
  ofSignature_cons.2 ⟨u, us, hu, hus, rfl⟩

theorem ofSignature_cons_eq {t : SignatureToken} {ts : List SignatureToken} {u : RuntimeType}
    {us : List RuntimeType} (hu : RuntimeType.ofToken m t = some u)
    (hus : RuntimeType.ofSignature m ts = some us) :
    RuntimeType.ofSignature m (t :: ts) = some (u :: us) :=
  ofSignature_cons_slot (ofSlotToken_of_ofToken hu) hus

theorem ofSignature_append {a b : List SignatureToken} {r : List RuntimeType} :
    RuntimeType.ofSignature m (a ++ b) = some r ↔
      ∃ ra rb, RuntimeType.ofSignature m a = some ra ∧ RuntimeType.ofSignature m b = some rb ∧
        r = ra ++ rb := by
  induction a generalizing r with
  | nil => simp [ofSignature_nil]
  | cons t ts ih =>
    rw [List.cons_append, ofSignature_cons]
    constructor
    · rintro ⟨u, us, hu, hus, rfl⟩
      obtain ⟨ra, rb, ha, hb, rfl⟩ := ih.1 hus
      exact ⟨u :: ra, rb, ofSignature_cons_slot hu ha, hb, rfl⟩
    · rintro ⟨ra, rb, ha, hb, rfl⟩
      obtain ⟨u, us, hu, hus, rfl⟩ := ofSignature_cons.1 ha
      exact ⟨u, us ++ rb, hu, ih.2 ⟨us, rb, hus, hb, rfl⟩, rfl⟩

theorem ofSignature_append_eq {a b : List SignatureToken} {ra rb : List RuntimeType}
    (ha : RuntimeType.ofSignature m a = some ra) (hb : RuntimeType.ofSignature m b = some rb) :
    RuntimeType.ofSignature m (a ++ b) = some (ra ++ rb) :=
  ofSignature_append.2 ⟨ra, rb, ha, hb, rfl⟩

theorem ofSignature_reverse {a : List SignatureToken} {r : List RuntimeType}
    (h : RuntimeType.ofSignature m a = some r) :
    RuntimeType.ofSignature m a.reverse = some r.reverse := by
  induction a generalizing r with
  | nil => simp at h ⊢; simpa [ofSignature_nil] using h
  | cons t ts ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSignature_cons.1 h
    rw [List.reverse_cons, List.reverse_cons]
    exact ofSignature_append_eq (ih hus) (ofSignature_cons_slot hu ofSignature_nil)

theorem ofSignature_getElem {a : List SignatureToken} {r : List RuntimeType} {i : Nat}
    {t : SignatureToken} (h : RuntimeType.ofSignature m a = some r) (ht : a[i]? = some t) :
    ∃ u, r[i]? = some u ∧ RuntimeType.ofSlotToken m t = some u := by
  induction a generalizing r i with
  | nil => simp at ht
  | cons x xs ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSignature_cons.1 h
    cases i with
    | zero => simp at ht; subst ht; exact ⟨u, rfl, hu⟩
    | succ i => simpa using ih hus (by simpa using ht)

theorem ofSignature_drop {a : List SignatureToken} {r : List RuntimeType} (n : Nat)
    (h : RuntimeType.ofSignature m a = some r) :
    RuntimeType.ofSignature m (a.drop n) = some (r.drop n) := by
  induction a generalizing r n with
  | nil => simp [ofSignature_nil] at h; subst h; simp [ofSignature_nil]
  | cons x xs ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSignature_cons.1 h
    cases n with
    | zero => simpa using ofSignature_cons_slot hu hus
    | succ n => simpa using ih n hus

theorem ofSignature_replicate {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofToken m t = some u) :
    ∀ n, RuntimeType.ofSignature m (List.replicate n t) = some (List.replicate n u)
  | 0 => rfl
  | n + 1 => ofSignature_cons_eq h (ofSignature_replicate h n)

theorem ofSignature_of_forall {sig : List SignatureToken}
    (h : ∀ t ∈ sig, (RuntimeType.ofToken m t).isSome) :
    ∃ r, RuntimeType.ofSignature m sig = some r := by
  induction sig with
  | nil => exact ⟨[], rfl⟩
  | cons t ts ih =>
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 (h t (List.mem_cons_self ..))
    obtain ⟨us, hus⟩ := ih (fun x hx => h x (List.mem_cons_of_mem _ hx))
    exact ⟨_, ofSignature_cons_eq hu hus⟩

theorem ofSignature_tokens {st : List SignatureToken} {r : List RuntimeType}
    (h : RuntimeType.ofSignature m st = some r) :
    ∀ a ∈ st, (RuntimeType.ofSlotToken m a).isSome := by
  induction st generalizing r with
  | nil => simp
  | cons x xs ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSignature_cons.1 h
    intro a ha
    rcases List.mem_cons.1 ha with rfl | ha
    · simp [hu]
    · exact ih hus a ha

/-- The translation of a struct's field signatures, which are value types, is
the specification's translation of its fields (`Program.fieldTypes`). -/
theorem ofSignature_fields {fields : List FieldDefinition}
    (h : ∀ f ∈ fields, (RuntimeType.ofToken m f.signature).isSome) :
    RuntimeType.ofSignature m (fields.map (·.signature)) =
      fields.mapM (fun f => RuntimeType.ofToken m f.signature) := by
  induction fields with
  | nil => rfl
  | cons f fs ih =>
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 (h f (List.mem_cons_self ..))
    have ih := ih (fun g hg => h g (List.mem_cons_of_mem _ hg))
    simp only [List.map_cons, RuntimeType.ofSignature, RuntimeType.ofSignatureWith, List.mapM_cons] at ih ⊢
    have hu' : RuntimeType.ofSlotTokenWith m [] f.signature = some u := ofSlotToken_of_ofToken hu
    rw [ih, hu', hu]

theorem ofToken_struct {h : StructHandleIndex} {sh : StructHandle} {s : StructId}
    (hsh : m.structHandles[h]? = some sh) (hnp : sh.typeParameters = [])
    (hs : m.structIdAt h = some s) : RuntimeType.ofToken m (.struct h) = some (.struct s) := by
  simp [RuntimeType.ofToken, RuntimeType.ofTokenWith, hsh, hnp, hs]

theorem ofToken_struct_inv {h : StructHandleIndex} {u : RuntimeType}
    (hu : RuntimeType.ofToken m (.struct h) = some u) :
    ∃ s, m.structIdAt h = some s ∧ u = .struct s := by
  simp only [RuntimeType.ofToken, RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at hu
  obtain ⟨sh, -, hu⟩ := hu
  split at hu
  · simp only [pure, Option.bind_eq_some_iff, Option.some.injEq] at hu
    obtain ⟨s, hs, rfl⟩ := hu
    exact ⟨s, hs, rfl⟩
  · simp at hu

/-- A closed instantiation of a struct handle without type parameters names
the struct type itself. -/
theorem ofToken_structInst_inv (hng : ∀ sh ∈ m.structHandles.toList, sh.typeParameters = [])
    {h : StructHandleIndex} {args : List SignatureToken} {u : RuntimeType}
    (hu : RuntimeType.ofToken m (.structInstantiation h args) = some u) :
    ∃ sh s, m.structHandles[h]? = some sh ∧ m.structIdAt h = some s ∧ args = [] ∧
      u = .struct s := by
  simp only [RuntimeType.ofToken, RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at hu
  obtain ⟨sh, hsh, sid, hsid, targs, htargs, hu⟩ := hu
  have hnp := hng sh (getElem?_mem_toList hsh)
  split at hu
  · next hlen =>
    rw [hnp] at hlen
    simp only [pure, Option.some.injEq] at hu
    subst hu
    have ht : targs = [] := List.eq_nil_of_length_eq_zero hlen
    subst ht
    have ha : args = [] := by
      cases args with
      | nil => rfl
      | cons x xs =>
        simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
          Option.some.injEq] at htargs
        obtain ⟨_, -, _, -, h⟩ := htargs
        cases h
    exact ⟨sh, sid, hsh, hsid, ha, by rw [structIdAt_typeArgs hsid]⟩
  · simp at hu

theorem isInteger_ofToken {t : SignatureToken} (h : isInteger t = true) :
    ∃ it, RuntimeType.ofToken m t = some (.int it) := by
  cases t <;> simp_all [isInteger, RuntimeType.ofToken, RuntimeType.ofTokenWith]

theorem isUnsigned_ofToken {t : SignatureToken} (h : isUnsignedInteger t = true) :
    ∃ it, RuntimeType.ofToken m t = some (.int it) ∧ it.isSigned = false := by
  cases t <;> simp_all [isUnsignedInteger, RuntimeType.ofToken, RuntimeType.ofTokenWith, IntType.isSigned]

theorem isSigned_ofToken {t : SignatureToken} (h : isSignedInteger t = true) :
    ∃ it, RuntimeType.ofToken m t = some (.int it) ∧ it.isSigned = true := by
  cases t <;> simp_all [isSignedInteger, RuntimeType.ofToken, RuntimeType.ofTokenWith, IntType.isSigned]

/-- The element type of a vector instruction whose type operand is in the
subset. -/
theorem elementType_supported {s : SignatureIndex} {elem : SignatureToken}
    (hops : (m.vectorElemType s).isSome) (h : elementType m s = .ok elem) :
    ∃ t, m.vectorElemType s = some t ∧ RuntimeType.ofToken m elem = some t := by
  unfold elementType at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sig, hsig, h⟩ := h
  unfold CompiledModule.vectorElemType CompiledModule.vectorElemTypeWith at hops ⊢
  rw [hsig] at hops ⊢
  rcases sig with _ | ⟨t, _ | ⟨t2, rest⟩⟩
  · simp at hops
  · simp only [List.head?_cons, pure, Except.pure, Except.ok.injEq] at h
    subst h
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 hops
    exact ⟨u, hu, hu⟩
  · simp at hops

end Tokens

/-! ## Abilities -/

theorem inter_subset_inter_right {a b c : AbilitySet} (h : b.subset c = true) :
    (a.inter b).subset (a.inter c) = true := by
  cases a; cases b; cases c
  simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
  grind

/-! ## Agreement of the model's view with the program -/

/-- How the struct handles and definitions of module `m`, at index `mi`,
relate to the program: what link checks establish. Every struct handle
declares at most the abilities of the type it names (K1), and every struct
definition defines the type its own handle names, which resolves back to it. -/
structure StructsAgree (P : Program) (mi : Nat) (m : CompiledModule) : Prop where
  abilities : ∀ h sh s, m.structHandles[h]? = some sh → m.structIdAt h = some s →
    sh.abilities.subset ((RuntimeType.struct s).abilities P) = true
  defs : ∀ d sd, m.structDefs[d]? = some sd → ∃ s id sh, P.definedStruct? mi d = some s ∧
    m.structIdAt sd.structHandle = some s ∧ m.structHandles[sd.structHandle]? = some sh ∧
    P.structType? s = some ⟨m, id, sd, sh⟩

theorem localAt_ok {view : FunctionView} {i : Nat} {t : SignatureToken}
    (h : localAt view i = .ok t) : (view.parameters ++ view.locals)[i]? = some t := by
  unfold localAt at h
  split at h
  · next hi =>
    rw [lookup_ok] at h
    rw [List.getElem?_append_left hi]
    simpa using h
  · next hi =>
    rw [lookup_ok] at h
    rw [List.getElem?_append_right (by omega)]
    simpa using h

theorem popPush_append {ra rb : List RuntimeType} {r : RuntimeType} :
    popPush ra.reverse r (ra ++ rb) = some (r :: rb) := by
  simp [popPush, assignableAll_refl]

/-- Operands assignable to the popped types split off the stack. -/
theorem assignable_take {ra rb tys : List RuntimeType}
    (h : RuntimeType.assignableAll ra tys.reverse = true) :
    (ra ++ rb).take tys.length = ra ∧ (ra ++ rb).drop tys.length = rb := by
  have hl : ra.length = tys.length := by simpa using assignableAll_length h
  rw [← hl]
  simp

theorem popPush_assignable {ra rb tys : List RuntimeType} {r : RuntimeType}
    (h : RuntimeType.assignableAll ra tys.reverse = true) :
    popPush tys r (ra ++ rb) = some (r :: rb) := by
  obtain ⟨h1, h2⟩ := assignable_take (rb := rb) h
  simp [popPush, h1, h2, h]

/-! ## References and global storage -/

theorem rejectIf_ok {c : Bool} {code : String} {u : Unit} :
    rejectIf c code = .ok u ↔ c = false := by
  cases c <;> simp [rejectIf, reject, pure, Except.pure]

theorem ofSlotToken_refTo {m : CompiledModule} {mut_ : Bool} {t : SignatureToken}
    {u : RuntimeType} (h : RuntimeType.ofToken m t = some u) :
    RuntimeType.ofSlotToken m (refTo mut_ t) = some (.ref mut_ u) := by
  cases mut_ <;> simp [refTo, RuntimeType.ofSlotToken, RuntimeType.ofSlotTokenWith, h]

/-- The slot type of a reference token. -/
theorem ofSlotToken_refTarget {m : CompiledModule} {op inner : SignatureToken}
    (h : refTarget op = some inner) {u : RuntimeType} (hu : RuntimeType.ofSlotToken m op = some u) :
    ∃ mut_ v, op = refTo mut_ inner ∧ RuntimeType.ofToken m inner = some v ∧ u = .ref mut_ v := by
  cases op <;> simp [refTarget] at h <;> subst h <;>
    simp only [RuntimeType.ofSlotToken, RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at hu <;>
    obtain ⟨v, hv, rfl⟩ := hu
  · exact ⟨false, v, rfl, hv, rfl⟩
  · exact ⟨true, v, rfl, hv, rfl⟩

theorem isMutableReference_refTo {mut_ : Bool} {t : SignatureToken} :
    isMutableReference (refTo mut_ t) = mut_ := by
  cases mut_ <;> rfl

theorem mapM_getElem? {α β : Type} {f : α → Option β} :
    ∀ {l : List α} {l' : List β}, l.mapM f = some l' → ∀ {i : Nat} {a : α}, l[i]? = some a →
      ∃ b, l'[i]? = some b ∧ f a = some b
  | [], _, _, _, _, ha => by simp at ha
  | x :: l, l', h, i, a, ha => by
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨b, hb, l'', hl'', rfl⟩ := h
    cases i with
    | zero => simp at ha; subst ha; exact ⟨b, rfl, hb⟩
    | succ i =>
      simp only [List.getElem?_cons_succ] at ha ⊢
      exact mapM_getElem? hl'' ha

/-- A field borrow the model accepts, with its operand and its result. -/
theorem fieldBorrow_ok {m : CompiledModule} {st st' : List SignatureToken} {mut_ : Bool} {h : Nat}
    (hok : borrowField m st mut_ false h = .ok st') :
    ∃ op rest fh sd, st = op :: rest ∧ (mut_ && !isMutableReference op) = false ∧
      m.fieldHandles[h]? = some fh ∧ m.structDefs[fh.owner]? = some sd ∧
      refTarget op = some (.struct sd.structHandle) ∧
      st' = (match (structFields sd.fieldInformation none)[fh.field]? with
        | some fd => refTo mut_ fd.signature
        | none => .bool) :: rest := by
  simp only [borrowField, bind_ok, fieldHandleParts, Bool.false_eq_true, ↓reduceIte, lookup_ok,
    pure, Except.pure] at hok
  obtain ⟨⟨op, rest⟩, hpop, u, hmut, p, ⟨fh, hfh, hp⟩, sd, hsd, u', htar, h⟩ := hok
  rw [popType_ok] at hpop
  subst hpop
  simp only [Except.ok.injEq] at hp
  subst hp
  rw [rejectIf_ok] at hmut htar
  refine ⟨op, rest, fh, sd, rfl, hmut, hfh, hsd, by simpa using htar, ?_⟩
  revert h
  dsimp only
  cases (structFields sd.fieldInformation none)[fh.field]? <;> simp [bind, Except.bind, eq_comm]

/-- One step of the model's variant field type. -/
def vftStep (sd : StructDefinition) (field : Nat) (acc : Option SignatureToken) (v : Nat) :
    VResult (Option SignatureToken) :=
  match (structFields sd.fieldInformation (some v))[field]? with
  | some fd =>
    match acc with
    | some t => if fd.signature != t then reject "BORROWFIELD_TYPE_MISMATCH_ERROR" else pure acc
    | none => pure (some fd.signature)
  | none => reject "BORROWFIELD_BAD_FIELD_ERROR"

theorem variantFieldType_eq (sd : StructDefinition) (field : Nat) (vs : List Nat) :
    variantFieldType sd field vs = vs.foldlM (vftStep sd field) none := rfl

theorem vft_some {sd : StructDefinition} {field : Nat} :
    ∀ {vs : List Nat} {t : SignatureToken} {r : Option SignatureToken},
      vs.foldlM (vftStep sd field) (some t) = .ok r → r = some t ∧
        ∀ v ∈ vs, ∃ fd, (structFields sd.fieldInformation (some v))[field]? = some fd ∧
          fd.signature = t
  | [], t, r, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    exact ⟨h.symm, by simp⟩
  | v :: vs, t, r, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨acc', hstep, h⟩ := h
    unfold vftStep at hstep
    cases hfd : (structFields sd.fieldInformation (some v))[field]? with
    | none => simp [hfd, reject] at hstep
    | some fd =>
      simp only [hfd] at hstep
      by_cases heq : fd.signature = t
      · simp only [heq, bne_self_eq_false, Bool.false_eq_true, ↓reduceIte, pure, Except.pure,
          Except.ok.injEq] at hstep
        subst hstep
        obtain ⟨hr, hall⟩ := vft_some h
        refine ⟨hr, fun w hw => ?_⟩
        rcases List.mem_cons.1 hw with rfl | hw
        · exact ⟨fd, hfd, heq⟩
        · exact hall w hw
      · simp [heq, reject] at hstep

theorem vft_none {sd : StructDefinition} {field : Nat} {vs : List Nat} {r : Option SignatureToken}
    (h : vs.foldlM (vftStep sd field) none = .ok r) (hne : vs ≠ []) :
    ∃ t, r = some t ∧ ∀ v ∈ vs, ∃ fd, (structFields sd.fieldInformation (some v))[field]? = some fd ∧
      fd.signature = t := by
  cases vs with
  | nil => exact absurd rfl hne
  | cons v vs =>
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨acc', hstep, h⟩ := h
    unfold vftStep at hstep
    cases hfd : (structFields sd.fieldInformation (some v))[field]? with
    | none => simp [hfd, reject] at hstep
    | some fd =>
      simp only [hfd, pure, Except.pure, Except.ok.injEq] at hstep
      subst hstep
      obtain ⟨hr, hall⟩ := vft_some h
      refine ⟨fd.signature, hr, fun w hw => ?_⟩
      rcases List.mem_cons.1 hw with rfl | hw
      · exact ⟨fd, hfd, rfl⟩
      · exact hall w hw

/-- A variant field borrow the model accepts, with its operand and its
result. -/
theorem variantBorrow_ok {m : CompiledModule} {st st' : List SignatureToken} {mut_ : Bool}
    {h : Nat} (hok : borrowField m st mut_ true h = .ok st') :
    ∃ op rest fh sd, st = op :: rest ∧ (mut_ && !isMutableReference op) = false ∧
      m.variantFieldHandles[h]? = some fh ∧ m.structDefs[fh.structIndex]? = some sd ∧
      refTarget op = some (.struct sd.structHandle) ∧ fh.variants ≠ [] ∧
      ∃ ft, st' = refTo mut_ ft :: rest ∧ ∀ v ∈ fh.variants, ∃ fd,
        (structFields sd.fieldInformation (some v))[fh.field]? = some fd ∧ fd.signature = ft := by
  simp only [borrowField, bind_ok, fieldHandleParts, ↓reduceIte, lookup_ok, pure,
    Except.pure] at hok
  obtain ⟨⟨op, rest⟩, hpop, u, hmut, p, ⟨fh, hfh, hp⟩, sd, hsd, u', htar, h⟩ := hok
  rw [popType_ok] at hpop
  subst hpop
  simp only [Except.ok.injEq] at hp
  subst hp
  rw [rejectIf_ok] at hmut htar
  dsimp only at h
  split at h
  · simp [reject, bind, Except.bind] at h
  · next hne =>
    have hne' : fh.variants ≠ [] := by simpa using hne
    simp only [bind_ok] at h
    obtain ⟨r, hr, h⟩ := h
    rw [variantFieldType_eq] at hr
    obtain ⟨ft, rfl, hall⟩ := vft_none hr hne'
    simp only [Except.ok.injEq] at h
    exact ⟨op, rest, fh, sd, rfl, hmut, hfh, hsd, by simpa using htar, hne', ft, h.symm, hall⟩

/-- A global borrow the model accepts. -/
theorem borrowGlobal_ok {m : CompiledModule} {cs : List AbilitySet} {st st' : List SignatureToken}
    {mut_ : Bool} {d : Nat} (h : borrowGlobal m cs st mut_ d = .ok st') :
    ∃ rest sd a, st = .address :: rest ∧ m.structDefs[d]? = some sd ∧
      abilities m cs (.struct sd.structHandle) = .ok a ∧ a.key = true ∧
      st' = refTo mut_ (.struct sd.structHandle) :: rest := by
  simp only [borrowGlobal, bind_ok, lookup_ok, pure, Except.pure] at h
  obtain ⟨⟨op, rest⟩, hpop, u, hop, sd, hsd, a, ha, u', hk, h⟩ := h
  rw [popType_ok] at hpop
  subst hpop
  rw [rejectIf_ok] at hop hk
  simp only [bne_eq_false_iff_eq, Bool.not_eq_false'] at hop hk
  simp only [Except.ok.injEq] at h
  exact ⟨rest, sd, a, by rw [hop], hsd, ha, hk, h.symm⟩

theorem refTarget_refTo {mut_ : Bool} {t : SignatureToken} : refTarget (refTo mut_ t) = some t := by
  cases mut_ <;> rfl

theorem vectorRefElement_some {op e : SignatureToken} {mutOnly : Bool}
    (h : vectorRefElement op mutOnly = some e) :
    ∃ mut', op = refTo mut' (.vector e) ∧ (mutOnly = true → mut' = true) := by
  unfold vectorRefElement at h
  split at h
  · split at h
    · simp at h
    · next hm =>
      simp only [Option.some.injEq] at h
      subst h
      exact ⟨false, rfl, fun h' => absurd h' hm⟩
  · simp only [Option.some.injEq] at h
    subst h
    exact ⟨true, rfl, fun _ => rfl⟩
  · simp at h

/-- The slot type of a reference to a vector. -/
theorem ofSlotToken_vectorRef {m : CompiledModule} {mut' : Bool} {e : SignatureToken}
    {t : RuntimeType} (he : RuntimeType.ofToken m e = some t) {u : RuntimeType}
    (hu : RuntimeType.ofSlotToken m (refTo mut' (.vector e)) = some u) :
    u = .ref mut' (.vector t) := by
  obtain ⟨mm, v, hmm, hv, rfl⟩ := ofSlotToken_refTarget refTarget_refTo hu
  simp only [RuntimeType.ofToken, RuntimeType.ofTokenWith, he, Option.map_some, Option.some.injEq] at hv
  subst hv
  cases mut' <;> cases mm <;> simp_all [refTo]

theorem mapM_const {α β : Type} {g : α → Option β} {c : β} :
    ∀ {l : List α}, (∀ x ∈ l, g x = some c) → l.mapM g = some (List.replicate l.length c)
  | [], _ => rfl
  | x :: l, h => by
    rw [List.mapM_cons, h x (List.mem_cons_self ..), mapM_const fun y hy => h y
      (List.mem_cons_of_mem _ hy)]
    rfl

/-! ## The certificate -/

theorem functionView_fields {m : CompiledModule} {index : Nat} {c : CodeUnit}
    {handle : FunctionHandle} {cfg : Cfg} {view : FunctionView}
    (h : FunctionView.function m index c handle cfg = .ok view) :
    view.code = c ∧ view.handle = handle ∧ view.cfg = cfg ∧
      m.signatures[handle.parameters]? = some view.parameters ∧
      m.signatures[handle.returns]? = some view.returns ∧
      m.signatures[c.locals]? = some view.locals ∧
      ∀ cfg', FunctionView.function m index c handle cfg' = .ok { view with cfg := cfg' } := by
  unfold FunctionView.function at h ⊢
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨ps, hps, rs, hrs, ls, hls, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  subst h
  refine ⟨rfl, rfl, rfl, hps, hrs, hls, fun cfg' => ?_⟩
  simp [bind_ok, lookup_ok, hps, hrs, hls, pure, Except.pure]

theorem typeInstr_cfg (m : CompiledModule) (view : FunctionView) (c : Cfg) :
    typeInstr m { view with cfg := c } = typeInstr m view := rfl

theorem typeStep_cfg (m : CompiledModule) (view : FunctionView) (c : Cfg) :
    typeStep m { view with cfg := c } = typeStep m view := rfl

/-- The type certificate does not depend on the control-flow graph. -/
theorem typesBefore_cfg (m : CompiledModule) (view : FunctionView) (c : Cfg) :
    ∀ pc, typesBefore m { view with cfg := c } pc = typesBefore m view pc
  | 0 => rfl
  | pc + 1 => by
    simp only [typesBefore, typeStep_cfg, typesBefore_cfg m view c pc]

/-- The model's operand-stack types, as runtime types at the instance: what
the type-safety pass computes for each function with code, over its type
parameters, translated under the instance's type arguments. -/
def modelTypes (P : Program) : TypeCertificate := fun f pc =>
  match P.function? f with
  | none => none
  | some info =>
    match info.definition.code with
    | none => none
    | some code =>
      match FunctionView.function info.module f.index code info.handle default with
      | .error _ => none
      | .ok view =>
        (typesBefore info.module view pc).bind (RuntimeType.ofSignatureWith info.module f.typeArgs)

theorem modelTypes_eq {P : Program} {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    {cfg : Cfg} {view : FunctionView} (hf : P.function? f = some info)
    (hcode : info.definition.code = some code)
    (hview : FunctionView.function info.module f.index code info.handle cfg = .ok view) (pc : Nat) :
    modelTypes P f pc =
      (typesBefore info.module view pc).bind (RuntimeType.ofSignatureWith info.module f.typeArgs) := by
  obtain ⟨-, -, -, -, -, -, hany⟩ := functionView_fields hview
  simp only [modelTypes, hf, hcode, hany default, typesBefore_cfg]

theorem typeStep_eq {m : CompiledModule} {view : FunctionView} {st : List SignatureToken}
    {pc : Nat} {instr : Bytecode} (hi : view.code.code[pc]? = some instr) :
    typeStep m view st pc = typeInstr m view st instr := by
  simp [typeStep, bind, Except.bind, lookup, hi, pure, Except.pure]

/-! ## Accepted code type-checks -/

/-- The generic instruction forms. -/
def genericForm : Bytecode → Bool
  | .callGeneric _ | .packGeneric _ | .packVariantGeneric _ | .unpackGeneric _
  | .unpackVariantGeneric _ | .testVariantGeneric _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ | .immBorrowFieldGeneric _ | .immBorrowVariantFieldGeneric _
  | .mutBorrowGlobalGeneric _ | .immBorrowGlobalGeneric _ | .existsGeneric _ | .moveFromGeneric _
  | .moveToGeneric _ => true
  | _ => false

/-- The closure instructions. -/
def closureForm : Bytecode → Bool
  | .packClosure _ _ | .packClosureGeneric _ _ | .callClosure _ => true
  | _ => false

/-- Every instruction is of the non-generic subset, a generic form, or a
closure instruction. -/
theorem typeInstr_subset {m : CompiledModule} {view : FunctionView} {st st' : List SignatureToken}
    {i : Bytecode} (_h : typeInstr m view st i = .ok st') :
    i.inSubsetNG = true ∨ genericForm i = true ∨ closureForm i = true := by
  cases i <;> first | exact Or.inl rfl | exact Or.inr (Or.inl rfl) | exact Or.inr (Or.inr rfl)

/-- Every instruction of a function the model accepts type-checks. -/
theorem typed_of_verified {config : VerifierConfig} {m : CompiledModule} {index : Nat}
    {fdef : FunctionDefinition} {code : CodeUnit} {handle : FunctionHandle}
    (hv : FunctionVerified config m index fdef code handle) :
    ∀ i ∈ code.code.toList, ∃ view st st', typeInstr m view st i = .ok st' := by
  obtain ⟨cfg, view, hcf, hview, -, hts, -⟩ := hv.views
  obtain ⟨hvc, -, hvcfg, -, -, -, -⟩ := functionView_fields hview
  have hblocks : view.cfg.blocks = buildBlocks view.code.code := by
    rw [hvcfg, verifyControlFlow_blocks hcf, hvc]
  intro i hi
  rw [← hvc] at hi
  obtain ⟨st, st', h⟩ := typed_of_typeSafety hts hblocks i hi
  exact ⟨view, st, st', h⟩

/-- Every instruction of a function of a program the model accepts
type-checks. -/
theorem typed_of_accepted {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {mi : Nat} {m : CompiledModule} (hm : P.modules[mi]? = some m) {d : FunctionDefinition}
    (hd : d ∈ m.functionDefs.toList) {code : CodeUnit} (hcode : d.code = some code) :
    ∀ i ∈ code.code.toList, ∃ view st st', typeInstr m view st i = .ok st' := by
  obtain ⟨index, hidx⟩ := List.mem_iff_getElem?.1 hd
  rw [Array.getElem?_toList] at hidx
  obtain ⟨-, -, -, hcu⟩ := verifyModule_ok (wellFormed_module hwf hm).1
  obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hidx
  obtain ⟨handle, hv⟩ := verifyFunction_parts hcode hn
  exact typed_of_verified hv

/-- **No code of the program packs or calls a closure.** -/
def NoClosureCode (P : Program) : Prop :=
  ∀ f info code, P.function? f = some info → info.definition.code = some code →
    ∀ instr ∈ code.code.toList, (∀ h mask, instr ≠ .packClosure h mask) ∧
      (∀ i mask, instr ≠ .packClosureGeneric i mask) ∧ ∀ s, instr ≠ .callClosure s

end MoveBytecode.Proofs
