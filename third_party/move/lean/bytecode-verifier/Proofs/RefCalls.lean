-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefGlobalFacts

/-!
# Calls and returns keep the reference invariant

UNTRUSTED (checked by the kernel). A call passes the callee references the
caller tracks, which the analysis checked are exclusive where mutable: the
callee's first frame agrees with the certificate, its reference parameters
being those references. The caller suspends with the analysis's state at the
call. A return hands back references inside the callee's parameters, which
the callee's own checks made compatible with each other; the caller resumes
with them as the analysis's new references after the call. A native takes
and returns values, and resumes the caller the same way.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference}

/-! ## References among values -/

theorem mem_argRefs {l : List Value} {r : Reference} : r ∈ argRefs l ↔ .ref r ∈ l := by
  unfold argRefs
  simp only [List.mem_filterMap]
  constructor
  · rintro ⟨v, hv, hr⟩
    cases v <;> simp at hr
    subst hr; exact hv
  · intro h; exact ⟨_, h, rfl⟩

theorem argRefs_pairwise_get {R : Reference → Reference → Prop} :
    ∀ {l : List Value}, (argRefs l).Pairwise R → ∀ {i j : Nat} {a b : Reference}, i < j →
      l[i]? = some (.ref a) → l[j]? = some (.ref b) → R a b
  | [], _, i, _, _, _, _, hi, _ => by simp at hi
  | v :: l, h, i, j, a, b, hij, hi, hj => by
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hi
      subst hi
      rw [argRefs_cons_ref, List.pairwise_cons] at h
      obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
      exact h.1 b (mem_argRefs.2 (List.mem_of_getElem? (by simpa using hj)))
    | succ i =>
      obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
      have h' : (argRefs l).Pairwise R := by
        cases v with
        | ref r => rw [argRefs_cons_ref, List.pairwise_cons] at h; exact h.2
        | _ => exact h
      exact argRefs_pairwise_get (i := i) (j := j') h' (by omega) (by simpa using hi)
        (by simpa using hj)

/-- A closure's captured arguments, values, add no references to the provided
ones: the arguments `maskCompose` builds have the provided ones' references,
in order, unless it drops some. -/
theorem ofSlotTokensWith_eq (m : CompiledModule) (τ : List RuntimeType) :
    ∀ l, RuntimeType.ofSlotTokensWith m τ l = RuntimeType.ofSignatureWith m τ l
  | [] => rfl
  | t :: ts => by
    have ih := ofSlotTokensWith_eq m τ ts
    simp only [RuntimeType.ofSignatureWith] at ih ⊢
    rw [List.mapM_cons, ← ih]
    cases t <;> simp only [RuntimeType.ofSlotTokensWith, RuntimeType.ofSlotTokenWith] <;>
      cases RuntimeType.ofSlotTokensWith m τ ts <;>
      cases RuntimeType.ofTokenWith m τ _ <;> simp [Option.map_eq_bind]

theorem argRefs_cons_nonRef {c : Value} (hc : ∀ r, c ≠ .ref r) (l : List Value) :
    argRefs (c :: l) = argRefs l := by
  cases c <;> first | rfl | exact absurd rfl (hc _)

theorem argRefs_maskCompose_go : ∀ (bits : List Bool) (cs vs : List Value),
    (∀ c ∈ cs, ∀ r, c ≠ .ref r) → (argRefs (maskCompose.go bits cs vs)).Sublist (argRefs vs)
  | [], _, _, _ => by simp [maskCompose.go, argRefs]
  | true :: bits, c :: cs, vs, hc => by
    simp only [maskCompose.go]
    rw [argRefs_cons_nonRef (hc c List.mem_cons_self)]
    exact argRefs_maskCompose_go bits cs vs fun c' h => hc c' (List.mem_cons_of_mem _ h)
  | false :: bits, cs, p :: vs, hc => by
    simp only [maskCompose.go]
    have ih := argRefs_maskCompose_go bits cs vs hc
    by_cases hp : ∃ r, p = .ref r
    · obtain ⟨r, rfl⟩ := hp
      exact ih.cons_cons r
    · have hp' : ∀ r, p ≠ .ref r := fun r e => hp ⟨r, e⟩
      rw [argRefs_cons_nonRef hp', argRefs_cons_nonRef hp']
      exact ih
  | true :: _, [], _, _ => by simp [maskCompose.go, argRefs]
  | false :: _, _, [], _ => by simp [maskCompose.go, argRefs]

/-! ### Slot types under type arguments -/

/-- A slot token that is no reference translates to a value type, under type
arguments that are value types. -/
theorem ofSlotTokenWith_isValue {m : CompiledModule} {targs : List RuntimeType}
    (htargs : ∀ x ∈ targs, x.isValue = true) {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofSlotTokenWith m targs t = some u) (hk : refKind t = none) :
    u.isValue = true := by
  cases t <;> simp [refKind] at hk <;>
    exact ofTokenWith_isValue htargs (by simpa [RuntimeType.ofSlotTokenWith] using h)

theorem ofSignatureWith_cons {m : CompiledModule} {targs : List RuntimeType} {t : SignatureToken}
    {ts : List SignatureToken} {r : List RuntimeType} :
    RuntimeType.ofSignatureWith m targs (t :: ts) = some r ↔
      ∃ u us, RuntimeType.ofSlotTokenWith m targs t = some u ∧
        RuntimeType.ofSignatureWith m targs ts = some us ∧ r = u :: us := by
  simp only [RuntimeType.ofSignatureWith, List.mapM_cons, bind, Option.bind_eq_some_iff, pure,
    Option.some.injEq]
  constructor
  · rintro ⟨u, hu, us, hus, rfl⟩; exact ⟨u, us, hu, hus, rfl⟩
  · rintro ⟨u, us, hu, hus, rfl⟩; exact ⟨u, hu, us, hus, rfl⟩

theorem ofSignatureWith_getElem {m : CompiledModule} {targs : List RuntimeType}
    {a : List SignatureToken} {r : List RuntimeType} {i : Nat} {t : SignatureToken}
    (h : RuntimeType.ofSignatureWith m targs a = some r) (ht : a[i]? = some t) :
    ∃ u, r[i]? = some u ∧ RuntimeType.ofSlotTokenWith m targs t = some u := by
  induction a generalizing r i with
  | nil => simp at ht
  | cons x xs ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSignatureWith_cons.1 h
    cases i with
    | zero => simp at ht; subst ht; exact ⟨u, rfl, hu⟩
    | succ i => simpa using ih hus (by simpa using ht)

theorem ofSignatureWith_append_eq {m : CompiledModule} {targs : List RuntimeType}
    {a b : List SignatureToken} {ra rb : List RuntimeType}
    (ha : RuntimeType.ofSignatureWith m targs a = some ra)
    (hb : RuntimeType.ofSignatureWith m targs b = some rb) :
    RuntimeType.ofSignatureWith m targs (a ++ b) = some (ra ++ rb) := by
  induction a generalizing ra with
  | nil =>
    simp only [RuntimeType.ofSignatureWith, List.mapM_nil, Option.pure_def,
      Option.some.injEq] at ha
    subst ha
    simpa using hb
  | cons t ts ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSignatureWith_cons.1 ha
    exact ofSignatureWith_cons.2 ⟨u, us ++ rb, hu, ih hus, rfl⟩

/-- A slot token of reference kind translates to a reference type. -/
theorem ofSlotToken_ref {m : CompiledModule} {targs : List RuntimeType} {t : SignatureToken}
    {u : RuntimeType} {b : Bool}
    (h : RuntimeType.ofSlotTokenWith m targs t = some u) (hk : refKind t = some b) :
    ∃ v, u = .ref b v := by
  cases t <;> simp [refKind] at hk
  · subst hk
    simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at h
    obtain ⟨v, -, rfl⟩ := h
    exact ⟨v, rfl⟩
  · subst hk
    simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at h
    obtain ⟨v, -, rfl⟩ := h
    exact ⟨v, rfl⟩

/-- The token a slot type comes from is a reference token when the type is a
reference type, under type arguments that are value types. -/
theorem refKind_of_ofSlotToken {m : CompiledModule} {targs : List RuntimeType}
    (htargs : ∀ x ∈ targs, x.isValue = true) {t : SignatureToken} {b : Bool}
    {v : RuntimeType} (h : RuntimeType.ofSlotTokenWith m targs t = some (.ref b v)) :
    refKind t = some b := by
  cases hk : refKind t with
  | none =>
    have := ofSlotTokenWith_isValue htargs h hk
    simp [RuntimeType.isValue] at this
  | some b' =>
    obtain ⟨v', hv'⟩ := ofSlotToken_ref h hk
    simp only [RuntimeType.ref.injEq] at hv'
    rw [hv'.1]

/-- A slot type is a value type or a reference type, under type arguments
that are value types. -/
theorem slotType_of_ofSignature {m : CompiledModule} {targs : List RuntimeType}
    (htargs : ∀ x ∈ targs, x.isValue = true) :
    ∀ {stys : List SignatureToken} {tys : List RuntimeType},
    RuntimeType.ofSignatureWith m targs stys = some tys → ∀ t ∈ tys,
      t.isValue = true ∨ ∃ b u, t = .ref b u
  | [], tys, h, t, ht => by
    simp only [RuntimeType.ofSignatureWith, List.mapM_nil, Option.pure_def,
      Option.some.injEq] at h
    subst h
    cases ht
  | tok :: stys, tys, h, t, ht => by
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSignatureWith_cons.1 h
    rcases List.mem_cons.1 ht with rfl | ht
    · cases hk : refKind tok with
      | none => exact Or.inl (ofSlotTokenWith_isValue htargs hu hk)
      | some b =>
        obtain ⟨v, rfl⟩ := ofSlotToken_ref hu hk
        exact Or.inr ⟨b, v, rfl⟩
    · exact slotType_of_ofSignature htargs hus t ht

/-- A local holding a reference has a reference token in the function's view. -/
theorem localToken_ref {fr : Frame} {info : FunctionInfo} {code : CodeUnit} {view : FunctionView}
    (hfn : P.function? fr.function = some info) (hbody : info.definition.code = some code)
    (hview : ∃ cfg, FunctionView.function info.module fr.function.index code info.handle cfg =
      .ok view)
    (hloc : LocalsOk P fr) (htargs : ∀ x ∈ fr.function.typeArgs, x.isValue = true) {j : Nat}
    {r : Reference} (hj : fr.locals[j]? = some (some (.ref r))) :
    ∃ t, (view.parameters ++ view.locals)[j]? = some t ∧ refKind t = some r.mutable := by
  obtain ⟨cfg, hv⟩ := hview
  obtain ⟨tys, htys, -, hconf⟩ := hloc
  obtain ⟨T, hT, hrT⟩ := hconf j _ hj
  obtain ⟨u, rfl⟩ := assignable_from_ref (m := r.mutable) (t := r.target) (conforms_assignable hrT)
  simp only [FunctionView.function, bind, Except.bind, pure, Except.pure] at hv
  split at hv
  · cases hv
  next psig hps =>
  split at hv
  · cases hv
  next rsig hrs =>
  split at hv
  · cases hv
  next lsig hls =>
  simp only [Except.ok.injEq] at hv
  subst hv
  rw [lookup_ok] at hps hls
  simp only [Program.localTypes, hfn, hbody, hps, hls, bind, Option.bind_some,
    Option.bind_eq_some_iff, pure, Option.some.injEq] at htys
  obtain ⟨ps, hps', ls, hls', rfl⟩ := htys
  have hsig := ofSignatureWith_append_eq hps' hls'
  have hlen := ofSignature_length hsig
  have hjl : j < (psig ++ lsig).length := by
    rw [← hlen]; exact (List.getElem?_eq_some_iff.1 hT).1
  obtain ⟨t, ht⟩ : ∃ t, (psig ++ lsig)[j]? = some t := ⟨_, List.getElem?_eq_getElem hjl⟩
  obtain ⟨u, hu, hut⟩ := ofSignatureWith_getElem hsig ht
  rw [hT, Option.some.injEq] at hu
  subst hu
  exact ⟨t, ht, refKind_of_ofSlotToken htargs hut⟩

/-- **Abstract args are values where the concrete ones are**: the top `n`
operands, none of them a reference. -/
theorem Desc.take_nonRef {gsid : Nat → Option StructId} {L : Nat → Option Reference}
    (hd : Desc A fr gsid ps L) (hlen : A.stack.length = fr.stack.length)
    {n : Nat} (hpop : ∀ v ∈ fr.stack.take n, ∀ r, v ≠ .ref r) :
    ∀ v ∈ A.stack.take n, v = .nonReference := by
  intro v hv
  cases v with
  | nonReference => rfl
  | reference x =>
    exfalso
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hv
    have hkn : k < n := by
      have := (List.getElem?_eq_some_iff.1 hk).1; simp at this; omega
    have hkl : k < A.stack.length := by
      have := (List.getElem?_eq_some_iff.1 hk).1; simp at this; omega
    rw [List.getElem?_take_of_lt hkn] at hk
    have hrev : A.stack.reverse[A.stack.length - 1 - k]? = some (.reference x) := by
      rw [List.getElem?_reverse (by omega), show A.stack.length - 1 - (A.stack.length - 1 - k) = k
        by omega]
      exact hk
    obtain ⟨r, hr, -⟩ := hd.stack _ x hrev
    rw [List.getElem?_reverse (by omega), hlen,
      show fr.stack.length - 1 - (fr.stack.length - 1 - k) = k by omega] at hr
    exact hpop _ (List.mem_of_getElem? (by rw [List.getElem?_take_of_lt hkn]; exact hr)) r rfl

/-- **A concrete reference operand has an abstract one**, which describes it. -/
theorem abs_of_ref {m : CompiledModule} {gsid : Nat → Option StructId}
    {L : Nat → Option Reference} {ltys : List SignatureToken} {avail : LocalIndex → Availability}
    {τ : List RuntimeType} (hd : Desc A fr gsid ps L) (ht : Typed ltys stys avail A)
    (hτ : ∀ x ∈ τ, x.isValue = true)
    (hsig : RuntimeType.ofSignatureWith m τ stys = some tys) (hconf : conformsAll P fr.stack tys = true)
    (hlen : A.stack.length = fr.stack.length) {k : Nat} {r : Reference}
    (hk : fr.stack[k]? = some (.ref r)) : ∃ x, A.stack[k]? = some (.reference x) ∧ L x = some r := by
  have hkl : k < fr.stack.length := (List.getElem?_eq_some_iff.1 hk).1
  obtain ⟨va, hva⟩ : ∃ va, A.stack[k]? = some va := ⟨_, List.getElem?_eq_getElem (by omega)⟩
  cases va with
  | nonReference => exact absurd rfl (notRef_of_abs ht hτ hsig hconf hva hk r)
  | reference x =>
    refine ⟨x, hva, ?_⟩
    have hrev : A.stack.reverse[A.stack.length - 1 - k]? = some (.reference x) := by
      rw [List.getElem?_reverse (by omega), show A.stack.length - 1 - (A.stack.length - 1 - k) = k
        by omega]
      exact hva
    obtain ⟨r', hr', hL⟩ := hd.stack _ x hrev
    rw [List.getElem?_reverse (by omega), hlen,
      show fr.stack.length - 1 - (fr.stack.length - 1 - k) = k by omega, hk,
      Option.some.injEq, Value.ref.injEq] at hr'
    rw [hr']; exact hL

/-! ## The callee's first frame -/

/-- The callee's references: reference parameter `i` holds argument `i`. -/
def argMap (args : List Value) : Nat → Option Reference := fun i =>
  match args[i]? with
  | some (.ref r) => some r
  | _ => none

theorem argMap_some {args : List Value} {i : Nat} {r : Reference} :
    argMap args i = some r ↔ args[i]? = some (.ref r) := by
  unfold argMap
  constructor
  · intro h
    split at h
    · next r' hr => simp only [Option.some.injEq] at h; subst h; exact hr
    · cases h
  · intro h; simp [h]

section Entry

variable (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : P.ApiSigs)

include hwf hs hapi in
/-- **A function's first frame agrees with the reference certificate**: its
reference parameters are its reference arguments, which are exclusive where
mutable. -/
theorem frameOk_entry' {f : FunctionRef} {finfo : FunctionInfo} {fcode : CodeUnit}
    (hf : P.function? f = some finfo) (hcode : finfo.definition.code = some fcode)
    (htok : P.typeArgsOk f = true) {frame : Frame}
    {args : List Value} {params : List RuntimeType} (hpar : P.paramTypes f = some params) {n : Nat}
    (hloc : frame.locals = (args.map some ++ List.replicate (n - args.length) none).toArray)
    (hargs : conformsAll P args params = true) (hexcl : (argRefs args).Pairwise Compat)
    (hok : ∀ r ∈ argRefs args, RefOk P s frame r)
    (hbelow : ∀ r ∈ argRefs args, ∀ f' i, r.location.root = .local_ f' i → f' < frame.id) :
    ∃ A0, modelRef P f 0 = some A0 ∧ FrameOk P s frame (argRefs args) A0 := by
  obtain ⟨cfg, view, hcf, hview, hcert⟩ := refCert_gen hwf hs hapi hf hcode htok
  obtain ⟨initial, A0, hinit, hA0, hle⟩ := hcert.entry
  obtain ⟨-, -, -, hpsv, -, -, -⟩ := functionView_fields hview
  obtain ⟨sig, hsig, hps⟩ := paramTypes_sig hf hpar
  rw [hpsv, Option.some.injEq] at hsig
  subst hsig
  have ht := typed_new_general (avail := fun _ => .unavailable) hinit (fun i h => by cases h)
  obtain ⟨st₁, hI, g, hg, rfl⟩ := new_general hinit
  have hlen := conformsAll_length hargs
  have hplen := ofSignature_length hps
  -- A reference parameter holds a reference argument of its mutability.
  have hparam : ∀ (i : Nat) (m : Bool), RefParam view.parameters i m →
      ∃ r : Reference, args[i]? = some (Value.ref r) ∧ r.mutable = m := by
    intro i m hrp
    obtain ⟨t, ht', htm⟩ := hrp
    obtain ⟨u, hu, hut⟩ := ofSignatureWith_getElem hps ht'
    obtain ⟨v, rfl⟩ := ofSlotToken_ref hut htm
    have hil : i < args.length := by
      rw [hlen]; exact (List.getElem?_eq_some_iff.1 hu).1
    obtain ⟨u', hu', hvu⟩ := conformsAll_getElem hargs (List.getElem?_eq_getElem hil)
    rw [hu, Option.some.injEq] at hu'
    subst hu'
    obtain ⟨r, hr, hrm, -⟩ := conforms_ref_assignable hvu
    exact ⟨r, by rw [List.getElem?_eq_getElem hil, hr], hrm⟩
  -- The references the initial state holds.
  have hheld : ∀ x, x ∈ heldIds { st₁ with graph := g } →
      ∃ r, args[x]? = some (.ref r) ∧ argMap args x = some r ∧
        g.mutability x = some r.mutable := by
    intro x hx
    simp only [heldIds, hI.stack, List.filterMap_nil, List.append_nil] at hx
    obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 (Typed.mem_filterMap_refId.1 hx)
    obtain ⟨rfl, hil, m, hrp⟩ := hI.locals i x hi
    obtain ⟨r, hr, hrm⟩ := hparam x m hrp
    refine ⟨r, hr, argMap_some.2 hr, ?_⟩
    rw [Graph.newRef_mutability hg x (by omega), hI.muts x m hil hrp, hrm]
  have hgL : ∀ x r, argMap args x = some r → r ∈ argRefs args := fun x r h =>
    mem_argRefs.2 (List.mem_of_getElem? (argMap_some.1 h))
  -- Two held references at different parameters.
  have hcompat : ∀ (p x : Nat) (rp rx : Reference), p ≠ x → args[p]? = some (Value.ref rp) →
      args[x]? = some (Value.ref rx) → Compat rp rx := by
    intro p x rp rx hpx hp hx
    rcases Nat.lt_or_gt_of_ne hpx with h | h
    · exact argRefs_pairwise_get hexcl h hp hx
    · have h' := argRefs_pairwise_get hexcl h hx hp
      exact ⟨h'.2.1, h'.1, h'.2.2.2, h'.2.2.1⟩
  have hroot : argMap args (st₁.locals.length) = none := by
    unfold argMap
    rw [hI.len, List.getElem?_eq_none (by omega)]
  have hc : Complete g (argMap args) (· ∈ heldIds { st₁ with graph := g })
      st₁.locals.length frame.id (gsidOf P (s.frames.map Frame.skel)) (argRefs args) := {
    wf := ht.wf
    back := ht.back
    rootNone := hroot
    rootMut := ht.rootMut
    rootNotHeld := ht.rootFree
    keys := ht.keys
    held := fun x hx => by
      obtain ⟨r, -, hr, hm⟩ := hheld x hx
      exact ⟨r, hr, hm⟩
    strict := fun p rp x rx hp hx hpL hxL hm hin => by
      exfalso
      obtain ⟨rp', hrp', hpL', -⟩ := hheld p hp
      obtain ⟨rx', hrx', hxL', -⟩ := hheld x hx
      rw [hpL, Option.some.injEq] at hpL'; subst hpL'
      rw [hxL, Option.some.injEq] at hxL'; subst hxL'
      by_cases hpx : p = x
      · subst hpx; rw [hrp', Option.some.injEq, Value.ref.injEq] at hrx'; subst hrx'
        exact StrictlyInside.ne hin
      · exact ((hcompat p x _ _ hpx hrp' hrx').1 hm).1 hin
    same := fun p rp x rx hp hx hne hpL hxL hm heq => by
      exfalso
      obtain ⟨rp', hrp', hpL', -⟩ := hheld p hp
      obtain ⟨rx', hrx', hxL', -⟩ := hheld x hx
      rw [hpL, Option.some.injEq] at hpL'; subst hpL'
      rw [hxL, Option.some.injEq] at hxL'; subst hxL'
      exact ((hcompat p x _ _ hne hrp' hrx').1 hm).2 heq
    immInside := fun p rp x rx hp hx hpL hxL hm hin => by
      obtain ⟨rp', hrp', hpL', -⟩ := hheld p hp
      obtain ⟨rx', hrx', hxL', -⟩ := hheld x hx
      rw [hpL, Option.some.injEq] at hpL'; subst hpL'
      rw [hxL, Option.some.injEq] at hxL'; subst hxL'
      by_cases hpx : p = x
      · subst hpx; rw [hrp', Option.some.injEq, Value.ref.injEq] at hrx'; subst hrx'
        exact absurd hin StrictlyInside.ne
      · exact (hcompat p x _ _ hpx hrp' hrx').2.2.2 hm hin
    rootLocal := fun x rx i hx hxL hr => by
      exfalso
      have := hbelow rx (hgL x rx hxL) frame.id i hr
      exact Nat.lt_irrefl _ this
    prov := fun x rx hx hxL =>
      Or.inr (Or.inr ⟨rx, hgL x rx hxL, Location.Inside.refl _, id⟩) }
  refine ⟨A0, hA0, FrameOk.le ⟨argMap args, ⟨fun i x hi => ?_, fun k x hk => ?_, hc⟩,
    fun x r hx hxL => hok r (hgL x r hxL)⟩ hle⟩
  · simp only at hi
    obtain ⟨rfl, hil, m, hrp⟩ := hI.locals i x hi
    obtain ⟨r, hr, -⟩ := hparam x m hrp
    have hia : x < args.length := by rw [hlen, hplen]; exact hil
    refine ⟨r, ?_, argMap_some.2 hr⟩
    rw [hloc, entry_locals_of hia, hr]
  · simp [hI.stack] at hk

end Entry

/-- **A function's first frame agrees with the reference certificate**, in
an accepted M3 program. -/
theorem frameOk_entry (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hm3 : P.M3) {f : FunctionRef} {finfo : FunctionInfo} {fcode : CodeUnit}
    (hf : P.function? f = some finfo) (hcode : finfo.definition.code = some fcode)
    (htok : P.typeArgsOk f = true) {frame : Frame}
    {args : List Value} {params : List RuntimeType} (hpar : P.paramTypes f = some params) {n : Nat}
    (hloc : frame.locals = (args.map some ++ List.replicate (n - args.length) none).toArray)
    (hargs : conformsAll P args params = true) (hexcl : (argRefs args).Pairwise Compat)
    (hok : ∀ r ∈ argRefs args, RefOk P s frame r)
    (hbelow : ∀ r ∈ argRefs args, ∀ f' i, r.location.root = .local_ f' i → f' < frame.id) :
    ∃ A0, modelRef P f 0 = some A0 ∧ FrameOk P s frame (argRefs args) A0 :=
  frameOk_entry' hwf (supported_of_nonGeneric hwf hs) hm3.apiSigs hf hcode htok hpar hloc hargs hexcl
    hok hbelow

/-! ## What a return hands back -/

theorem isMutable_of {g : Graph} {id : Nat} {b : Bool} (h : g.mutability id = some b) :
    g.isMutable id = .ok b := by
  simp only [Graph.mutability, Option.map_eq_some_iff] at h
  obtain ⟨r, hr, rfl⟩ := h
  simp [Graph.isMutable, Graph.get, hr, bind, Except.bind, pure, Except.pure]

theorem argRefs_pairwise_of {R : Reference → Reference → Prop} :
    ∀ {l : List Value}, (∀ (i j : Nat) a b, i < j → l[i]? = some (.ref a) →
      l[j]? = some (.ref b) → R a b) → (argRefs l).Pairwise R
  | [], _ => List.Pairwise.nil
  | v :: l, h => by
    have ih := argRefs_pairwise_of (l := l) (fun i j a b hij hi hj =>
      h (i + 1) (j + 1) a b (by omega) (by simpa using hi) (by simpa using hj))
    cases v with
    | ref a =>
      rw [argRefs_cons_ref, List.pairwise_cons]
      refine ⟨fun b hb => ?_, ih⟩
      obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 (mem_argRefs.1 hb)
      exact h 0 (j + 1) a b (by omega) rfl (by simpa using hj)
    | _ => exact ih

/-- Distinct reference slots of a list without repeated references hold
distinct references. -/
theorem refId_inj : ∀ {l : List AbsValue}, (l.filterMap AbsValue.refId?).Nodup →
    ∀ {i j x : Nat}, l[i]? = some (.reference x) → l[j]? = some (.reference x) → i = j
  | [], _, i, j, x, hi, _ => by simp at hi
  | v :: l, h, i, j, x, hi, hj => by
    cases i with
    | zero =>
      cases j with
      | zero => rfl
      | succ j =>
        simp only [List.getElem?_cons_zero, Option.some.injEq] at hi
        subst hi
        simp only [List.filterMap_cons, AbsValue.refId?, List.nodup_cons] at h
        exact absurd (Typed.mem_filterMap_refId.2 (List.mem_of_getElem? (by simpa using hj))) h.1
    | succ i =>
      cases j with
      | zero =>
        simp only [List.getElem?_cons_zero, Option.some.injEq] at hj
        subst hj
        simp only [List.filterMap_cons, AbsValue.refId?, List.nodup_cons] at h
        exact absurd (Typed.mem_filterMap_refId.2 (List.mem_of_getElem? (by simpa using hi))) h.1
      | succ j =>
        have h' : (l.filterMap AbsValue.refId?).Nodup := by
          cases v with
          | reference y =>
            simp only [List.filterMap_cons, AbsValue.refId?, List.nodup_cons] at h; exact h.2
          | nonReference => simpa [AbsValue.refId?] using h
        have := refId_inj (i := i) (j := j) (x := x) h' (by simpa using hi) (by simpa using hj)
        rw [this]

/-- **What a return hands back**: references inside the frame's reference
parameters, compatible with each other, tracked, and into no local of the
frame. -/
theorem return_contract (hctx : TopCtx P s fr info code .ret view A stys tys ps)
    (hlen : A.stack.length ≤ view.returns.length) :
    (∀ r ∈ argRefs fr.stack, ∃ π ∈ ps, r.location.Inside π.location ∧
      (r.mutable = true → π.mutable = true)) ∧
    (argRefs fr.stack.reverse).Pairwise Compat ∧
    (∀ r ∈ argRefs fr.stack, RefOk P s fr r) ∧
    (∀ r ∈ argRefs fr.stack, ∀ i, r.location.root ≠ .local_ fr.id i) := by
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨A', hx⟩ := hctx.exec
  obtain ⟨-, g, hrel, hcb, hwr⟩ := exec_ret hx
  rw [List.take_of_length_le hlen] at hwr
  have hc := releaseAll_complete (localRefs A.locals) (localRefs_nodup _)
    (fun a ha => mem_heldIds.2 (Or.inl (mem_localRefs.1 ha))) hd.complete hrel
  have hnd := hctx.typed.distinct
  have hstkNd : (A.stack.filterMap AbsValue.refId?).Nodup := by
    simp only [heldIds, List.nodup_append] at hnd; exact hnd.2.1
  -- A returned reference has an abstract one, held after the release.
  have habs : ∀ (k : Nat) (r : Reference), fr.stack[k]? = some (Value.ref r) →
      ∃ x, A.stack[k]? = some (AbsValue.reference x) ∧
        L x = some r ∧ (x ∈ heldIds A ∧ x ∉ localRefs A.locals) := by
    intro k r hk
    obtain ⟨x, hx, hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hk
    refine ⟨x, hx, hxL, mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx)), fun hloc => ?_⟩
    simp only [heldIds, List.nodup_append] at hnd
    exact hnd.2.2 x (Typed.mem_filterMap_refId.2 (mem_localRefs.1 hloc)) x
      (Typed.mem_filterMap_refId.2 (List.mem_of_getElem? hx)) rfl
  have hwr' : ∀ x rx, (x ∈ heldIds A ∧ x ∉ localRefs A.locals) → L x = some rx →
      rx.mutable = true → g.isWritable x = .ok true := by
    intro x rx hx hxL hm
    have hst : AbsValue.reference x ∈ A.stack := by
      rcases mem_heldIds.1 hx.1 with h | h
      · exact absurd (mem_localRefs.2 h) hx.2
      · exact h
    exact hwr x hst (isMutable_of (by rw [hc.held_mut hx hxL, hm]))
  have hstk : ∀ r ∈ argRefs fr.stack, ∃ k : Nat, fr.stack[k]? = some (Value.ref r) := fun r hr =>
    List.mem_iff_getElem?.1 (mem_argRefs.1 hr)
  have hnone : ∀ (e : BorrowGraph.Edge) n, g.Edge A.locals.length e n → False := by
    intro e n he
    have := Graph.hasConsistentBorrows_false hcb he
    have h2 : Graph.atLabel none e = true := by
      unfold Graph.atLabel; cases e.path <;> rfl
    rw [h2] at this; cases this
  refine ⟨fun r hr => ?_, ?_, fun r hr => ?_, fun r hr i hi => ?_⟩
  · obtain ⟨k, hk⟩ := hstk r hr
    obtain ⟨x, -, hxL, hxH⟩ := habs k r hk
    rcases hc.prov x r hxH hxL with ⟨i, hi⟩ | ⟨d, e, n, rn, he, -⟩ | h
    · obtain ⟨e, n, rn, he, -, -, -, -⟩ := hc.rootLocal x r i hxH hxL hi
      exact (hnone e n he).elim
    · exact (hnone e n he).elim
    · exact h
  · refine argRefs_pairwise_of fun i j a b hij hi hj => ?_
    have hil : i < fr.stack.length := by
      have := (List.getElem?_eq_some_iff.1 hi).1; simpa using this
    have hjl : j < fr.stack.length := by
      have := (List.getElem?_eq_some_iff.1 hj).1; simpa using this
    rw [List.getElem?_reverse hil] at hi
    rw [List.getElem?_reverse hjl] at hj
    obtain ⟨x, hx, hxL, hxH⟩ := habs _ a hi
    obtain ⟨y, hy, hyL, hyH⟩ := habs _ b hj
    have hxy : x ≠ y := by
      rintro rfl
      have := refId_inj hstkNd hx hy
      omega
    exact compat_of_complete hc hwr' hxH hyH hxy hxL hyL
  · obtain ⟨k, hk⟩ := hstk r hr
    obtain ⟨x, -, hxL, hxH⟩ := habs k r hk
    exact hrefs x r hxH.1 hxL
  · obtain ⟨k, hk⟩ := hstk r hr
    obtain ⟨x, -, hxL, hxH⟩ := habs k r hk
    obtain ⟨e, n, rn, he, -, -, -, -⟩ := hc.rootLocal x r i hxH hxL hi
    exact hnone e n he

/-! ## The caller's resumption -/

/-- A result of reference type is a reference of its mutability. -/
theorem result_ref {m : CompiledModule} {τ : List RuntimeType} {rs : List SignatureToken}
    {rets : List RuntimeType} {rv : List Value}
    (hrets : RuntimeType.ofSignatureWith m τ rs = some rets)
    (hconf : conformsAll P rv rets = true) {j : Nat} {t : SignatureToken} {b : Bool}
    (hj : rs[j]? = some t) (hk : refKind t = some b) :
    ∃ r, rv[j]? = some (.ref r) ∧ r.mutable = b := by
  obtain ⟨u, hu, hut⟩ := ofSigW_getElem hrets hj
  obtain ⟨w, rfl⟩ := ofSlotToken_ref hut hk
  have hjl : j < rv.length := by
    rw [conformsAll_length hconf]; exact (List.getElem?_eq_some_iff.1 hu).1
  obtain ⟨v, hv⟩ : ∃ v, rv[j]? = some v := ⟨_, List.getElem?_eq_getElem hjl⟩
  obtain ⟨u', hu', hvu⟩ := conformsAll_getElem hconf hv
  rw [hu, Option.some.injEq] at hu'
  subst hu'
  obtain ⟨r, rfl, hm, -⟩ := conforms_ref_assignable hvu
  exact ⟨r, hv, hm⟩

/-- A result of value type is no reference. -/
theorem result_val {m : CompiledModule} {τ : List RuntimeType} (hτ : ∀ x ∈ τ, x.isValue = true)
    {rs : List SignatureToken} {rets : List RuntimeType} {rv : List Value}
    (hrets : RuntimeType.ofSignatureWith m τ rs = some rets)
    (hconf : conformsAll P rv rets = true) {j : Nat} {t : SignatureToken}
    (hj : rs[j]? = some t) (hk : refKind t = none) {v : Value} (hv : rv[j]? = some v) :
    ∀ r, v ≠ .ref r := by
  obtain ⟨u, hu, hut⟩ := ofSigW_getElem hrets hj
  obtain ⟨u', hu', hvu⟩ := conformsAll_getElem hconf hv
  rw [hu, Option.some.injEq] at hu'
  subst hu'
  exact not_ref_of_conforms hvu (ofSlotToken_isValue hτ hut hk)

theorem take_drop_disjoint {l : List AbsValue} (h : (l.filterMap AbsValue.refId?).Nodup)
    {n x : Nat} (ht : AbsValue.reference x ∈ l.take n) (hd : AbsValue.reference x ∈ l.drop n) :
    False := by
  rw [← List.take_append_drop n l, List.filterMap_append, List.nodup_append] at h
  exact h.2.2 x (Typed.mem_filterMap_refId.2 ht) x (Typed.mem_filterMap_refId.2 hd) rfl

theorem locals_stack_disjoint {A : RefState} (h : (heldIds A).Nodup) {x : Nat}
    (hl : AbsValue.reference x ∈ A.locals) (hs : AbsValue.reference x ∈ A.stack) : False := by
  simp only [heldIds, List.nodup_append] at h
  exact h.2.2 x (Typed.mem_filterMap_refId.2 hl) x (Typed.mem_filterMap_refId.2 hs) rfl

/-- **The caller resumes** in a state the analysis's call describes: its
references stay tracked, and each reference result is a new reference
borrowed from the arguments it may be inside of. -/
theorem resume_frameOk {m : CompiledModule} {rs : List SignatureToken} {rets : List RuntimeType}
    {args rv : List Value} {caller caller' : Frame} {ps : List Reference} {A A' : RefState}
    {L : Nat → Option Reference} {gsid : Nat → Option StructId}
    (hgs : gsidOf P (s'.frames.map Frame.skel) = gsid) {τ : List RuntimeType}
    (hτ : ∀ x ∈ τ, x.isValue = true) (hrets : RuntimeType.ofSignatureWith m τ rs = some rets)
    (hcore : CoreRun A A' args.length rs)
    (hdist : (heldIds A).Nodup)
    (hd : Desc A { caller with stack := args.reverse ++ caller.stack } gsid ps L)
    (hheight : A.stack.length = args.length + caller.stack.length)
    (hconf : conformsAll P rv rets = true)
    (hrc1 : ∀ r ∈ argRefs rv, ∃ π ∈ argRefs args, r.location.Inside π.location ∧
      (r.mutable = true → π.mutable = true))
    (hcps : ∀ π ∈ argRefs args, ∃ x, AbsValue.reference x ∈ A.stack.take args.length ∧
      L x = some π)
    (hrc2 : (argRefs rv).Pairwise Compat)
    (hid : caller'.id = caller.id) (hloc : caller'.locals = caller.locals)
    (hstk : caller'.stack = rv.reverse ++ caller.stack)
    (hold : ∀ x r, x ∈ heldIds { A with stack := A.stack.drop args.length } → L x = some r →
      RefOk P s' caller' r)
    (hnew : ∀ r ∈ argRefs rv, RefOk P s' caller' r) :
    FrameOk P s' caller' ps A' := by
  obtain ⟨all, muts, vals, st₂, g, -, hall, hmuts, -, hwr, hsorted, hmap, hg, rfl⟩ := hcore
  have hc0 := hd.complete
  obtain ⟨-, -, -, hvref, -, hloc2, hstk2, -, -, -, -⟩ :=
    callResults_run rs _ st₂ vals hmap hc0.wf
  have hloc2' : st₂.locals = A.locals := hloc2
  have hstk2' : st₂.stack = A.stack.drop args.length := hstk2
  have hstkNd : (A.stack.filterMap AbsValue.refId?).Nodup := by
    simp only [heldIds, List.nodup_append] at hdist; exact hdist.2.1
  have hallH : ∀ a ∈ all, a ∈ heldIds A := fun a ha =>
    mem_heldIds.2 (Or.inr (List.mem_of_mem_take ((hall a).1 ha)))
  have hma : ∀ a ∈ muts, a ∈ all := fun a ha => (hall a).2 ((hmuts a).1 ha).1
  have hmuts' : ∀ a ∈ all, ∀ ra, L a = some ra → (a ∈ muts ↔ ra.mutable = true) := by
    intro a ha ra haL
    have hm := isMutable_of (hc0.held_mut (hallH a ha) haL)
    rw [hmuts a, hm]
    constructor
    · rintro ⟨-, h⟩; exact Except.ok.inj h
    · intro h; exact ⟨(hall a).1 ha, by rw [h]⟩
  have hvH : ∀ x, AbsValue.reference x ∈ vals → x ∉ heldIds A := by
    intro x hx hxH
    obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hx
    obtain ⟨-, -, -, -, -, hfresh⟩ := hvref j x hj
    exact hc0.not_held_of_fresh hfresh hxH
  have hrvlen : rv.length = rs.length := by
    rw [conformsAll_length hconf, ofSignature_length hrets]
  have hrc1' : ∀ r ∈ argRefs rv, ∃ a ∈ all, ∃ ra, L a = some ra ∧
      r.location.Inside ra.location ∧ (r.mutable = true → ra.mutable = true) := by
    intro r hr
    obtain ⟨π, hπ, hin, hm⟩ := hrc1 r hr
    obtain ⟨x, hx, hxL⟩ := hcps π hπ
    exact ⟨x, (hall x).2 hx, π, hxL, hin, hm⟩
  obtain ⟨L', hc', hext, hmap'⟩ := results_complete hc0 hallH hma hmuts'
    (fun a ha => hwr a ((hmuts a).1 ha).1 ((hmuts a).1 ha).2) rs rv _ st₂ vals L
    (· ∈ heldIds A) [] hmap hrvlen (fun j t b hj hk => result_ref hrets hconf hj hk)
    (fun j t hj hk v hv => result_val hτ hrets hconf hj hk hv) hrc1'
    (by rw [List.nil_append]; exact hrc2) hc0 (Graph.Covers.refl _) (fun x hx => hx)
    (fun x r _ h => h) (fun x hx => hx) (fun x rx hx hxL => Or.inl ⟨hx, hxL⟩)
  have hrel := releaseAll_complete all (hsorted.imp Nat.ne_of_lt)
    (fun a ha => Or.inl (hallH a ha)) hc' hg
  have hmem : ∀ x, ((x ∈ heldIds A ∨ AbsValue.reference x ∈ vals) ∧ x ∉ all) ↔
      x ∈ heldIds { st₂ with graph := g, stack := vals.reverse ++ st₂.stack } := by
    intro x
    simp only [mem_heldIds, hloc2', hstk2', List.mem_append, List.mem_reverse]
    rw [hall x]
    constructor
    · rintro ⟨(h | h) | h, hn⟩
      · exact Or.inl h
      · rw [← List.take_append_drop args.length A.stack, List.mem_append] at h
        rcases h with h | h
        · exact absurd h hn
        · exact Or.inr (Or.inr h)
      · exact Or.inr (Or.inl h)
    · rintro (h | h | h)
      · exact ⟨Or.inl (Or.inl h), fun ht => locals_stack_disjoint hdist h (List.mem_of_mem_take ht)⟩
      · exact ⟨Or.inr h, fun ht => hvH x h (mem_heldIds.2 (Or.inr (List.mem_of_mem_take ht)))⟩
      · exact ⟨Or.inl (Or.inr (List.mem_of_mem_drop h)), fun ht => take_drop_disjoint hstkNd ht h⟩
  have hlenD : (A.stack.drop args.length).length = caller.stack.length := by
    rw [List.length_drop]; omega
  refine ⟨L', ⟨fun i x hi => ?_, fun k x hk => ?_, ?_⟩, fun x r hx hxr => ?_⟩
  · -- The locals are the caller's.
    have hi' : A.locals[i]? = some (.reference x) := by rw [← hloc2']; exact hi
    obtain ⟨r, hr, hxL⟩ := hd.locals i x hi'
    exact ⟨r, by rw [hloc]; exact hr,
      hext x r (mem_heldIds.2 (Or.inl (List.mem_of_getElem? hi'))) hxL⟩
  · -- The operands below the arguments are the caller's; above them, the results.
    have hA's : (vals.reverse ++ st₂.stack).reverse = (A.stack.drop args.length).reverse ++ vals := by
      rw [hstk2', List.reverse_append, List.reverse_reverse]
    change (vals.reverse ++ st₂.stack).reverse[k]? = some (AbsValue.reference x) at hk
    rw [hA's] at hk
    rcases Nat.lt_or_ge k (A.stack.drop args.length).reverse.length with hkc | hkc
    · rw [List.getElem?_append_left hkc,
        reverse_drop_getElem? _ _ _ (by rw [List.length_reverse] at hkc; exact hkc)] at hk
      obtain ⟨r, hr, hxL⟩ := hd.stack k x hk
      have hkc' : k < caller.stack.reverse.length := by
        rw [List.length_reverse, ← hlenD, ← List.length_reverse]; exact hkc
      change (args.reverse ++ caller.stack).reverse[k]? = _ at hr
      rw [List.reverse_append, List.reverse_reverse, List.getElem?_append_left hkc'] at hr
      refine ⟨r, ?_, hext x r ?_ hxL⟩
      · rw [hstk, List.reverse_append, List.reverse_reverse, List.getElem?_append_left hkc']
        exact hr
      · exact mem_heldIds.2 (Or.inr (List.mem_reverse.1 (List.mem_of_getElem? hk)))
    · rw [List.getElem?_append_right hkc] at hk
      obtain ⟨t, mm, hjt, hkt, -, -⟩ := hvref _ x hk
      obtain ⟨r, hr, -⟩ := result_ref hrets hconf hjt hkt
      refine ⟨r, ?_, hmap' _ x r hk hr⟩
      have hkc' : caller.stack.reverse.length ≤ k := by
        rw [List.length_reverse, ← hlenD, ← List.length_reverse]; exact hkc
      have hjk : k - caller.stack.reverse.length =
          k - (A.stack.drop args.length).reverse.length := by
        rw [List.length_reverse, List.length_reverse, hlenD]
      rw [hstk, List.reverse_append, List.reverse_reverse, List.getElem?_append_right hkc', hjk]
      exact hr
  · -- The graph, after the results and the release of the arguments.
    have e1 : ({ st₂ with graph := g, stack := vals.reverse ++ st₂.stack } : RefState).locals.length =
        A.locals.length := by
      show st₂.locals.length = _; rw [hloc2']
    rw [e1, hid, hgs]
    exact hrel.congrH hmem
  · rcases (hmem x).2 hx with ⟨hx0 | hxv, hxa⟩
    · obtain ⟨r0, hr0, -⟩ := hc0.held x hx0
      have e := hext x r0 hx0 hr0
      rw [hxr, Option.some.injEq] at e
      subst e
      refine hold x _ ?_ hr0
      rw [mem_heldIds] at hx0 ⊢
      rcases hx0 with h | h
      · exact Or.inl h
      · right
        rw [← List.take_append_drop args.length A.stack, List.mem_append] at h
        rcases h with h | h
        · exact absurd ((hall x).2 h) hxa
        · exact h
    · obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hxv
      obtain ⟨t, mm, hjt, hkt, -, -⟩ := hvref j x hj
      obtain ⟨r', hr', -⟩ := result_ref hrets hconf hjt hkt
      have e := hmap' j x r' hj hr'
      rw [hxr, Option.some.injEq] at e
      subst e
      exact hnew _ (mem_argRefs.2 (List.mem_of_getElem? hr'))

/-! ## Returning -/

/-- **Returning values** to a suspended caller keeps the reference invariant:
the caller resumes in a state its certificate covers, the references returned
being the analysis's results of the call. -/
theorem refInv_return (hr : RefInvariant P s) (hctx : TopCtx P s fr info code .ret view A stys tys ps)
    {caller : Frame} {rest' : List Frame} (hsf : s.frames = fr :: caller :: rest')
    (hs'f : s'.frames =
      { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 } :: rest')
    (hnext : s'.nextFrame = s.nextFrame) (hstor : s'.storage = s.storage)
    (hinv : s'.invalidations = s.invalidations) (hheld : s'.held = s.held)
    {rets : List RuntimeType} (hrets : P.returnTypes fr.function = some rets)
    (hconf : conformsAll P fr.stack.reverse rets = true) {id : ModuleId}
    (hfm : funModule P fr.function = some id)
    (hact : s'.active = (if fr.countedActive then s.decActive id else s).active) :
    RefInvariant P s' := by
  obtain ⟨hids, hbelow, hstor0, -, hactv, hheldT⟩ := hr
  obtain ⟨S, -, psC, SC, hsusp, hchain⟩ := hctx.chain _ hsf
  obtain ⟨m, rs, rets', args, A0, A0', A0'', L, hf⟩ := hsusp.call
  have e := hf.calleeRets
  rw [hrets, Option.some.injEq] at e
  subst e
  obtain ⟨rv0, hrv0, hsig0⟩ := hctx.cert.setting.agrees.returns
  rw [hrets, Option.some.injEq] at hrv0
  subst hrv0
  have hlenA : A.stack.length ≤ view.returns.length := by
    rw [hctx.height, ← List.length_reverse, conformsAll_length hconf, ofSignature_length hsig0]
    exact Nat.le_refl _
  obtain ⟨hrc1, hrc2, hrc0, -⟩ := return_contract hctx hlenA
  have horder : caller.id < fr.id := hsusp.order
  have hids' : (s'.frames.map (·.id)).Nodup := by
    rw [hs'f, List.map_cons]
    rw [hsf, List.map_cons, List.map_cons, List.nodup_cons] at hids
    exact hids.2
  -- The frames no newer than the caller keep their functions and locals.
  have hfr : ∀ f, f ≤ caller.id → ∀ g, s.frame? f = some g →
      ∃ g', s'.frame? f = some g' ∧ g'.function = g.function ∧ g'.locals = g.locals := by
    intro f hf g hg
    have hne : fr.id ≠ f := by
      intro e; rw [← e] at hf; exact Nat.lt_irrefl _ (Nat.lt_of_le_of_lt hf horder)
    have h1 : (fr.id == f) = false := by simpa using hne
    simp only [State.frame?, hsf, hs'f, List.find?_cons, h1] at hg ⊢
    by_cases h2 : caller.id = f
    · have h2' : (caller.id == f) = true := by simpa using h2
      simp only [h2', Option.some.injEq] at hg ⊢
      subst hg
      exact ⟨_, rfl, rfl, rfl⟩
    · have h2' : (caller.id == f) = false := by simpa using h2
      simp only [h2'] at hg ⊢
      exact ⟨g, hg, rfl, rfl⟩
  have hrv : ∀ r, r ∈ argRefs fr.stack.reverse → r ∈ argRefs fr.stack := fun r hr =>
    mem_argRefs.2 (List.mem_reverse.1 (mem_argRefs.1 hr))
  have hok : FrameOk P s' { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 }
      psC A0' := by
    obtain ⟨τ, hτ, hrets⟩ := hf.retTypes
    refine resume_frameOk (rv := fr.stack.reverse) (by rw [hs'f]; rfl) hτ hrets hf.core
      hf.distinct hf.desc hf.height hconf (fun r hr => ?_) hf.cpsArgs hrc2 rfl rfl
      (by rw [List.reverse_reverse]) (fun x r hx hxr => ?_) (fun r hr => ?_)
    · obtain ⟨π, hπ, hin, hm⟩ := hrc1 r (hrv r hr)
      exact ⟨π, hf.cpsSub.subset hπ, hin, hm⟩
    · obtain ⟨hok, -⟩ := hf.refs x r hx hxr
      exact hok.agree hok.below hfr hheld hstor hinv (Nat.le_refl _)
    · obtain ⟨π, hπ, hin, -⟩ := hrc1 r (hrv r hr)
      have hb : RootBelow caller.id r.location.root := by
        rw [hin.1]; exact hf.cpsBelow π hπ
      exact (hrc0 r (hrv r hr)).agree hb hfr hheld hstor hinv (Nat.le_refl _)
  refine ⟨hids', fun g hg => ?_, fun r hr' => hstor0 r (by rw [← hstor]; exact hr'), ⟨_, rest',
    psC, hs'f, ⟨A0'', hf.after, hok.le hf.le⟩, SC, hsusp.glob,
    ChainRef.transferBelow (fun f hf => hfr f (Nat.le_of_lt hf)) hheld hstor hinv rest' caller.id
      caller.function psC SC (Nat.le_refl _) hchain⟩, hactv.pop hsf hs'f rfl hfm hact,
    hheldT.of fun g' hg' rx hm k hk => ?_⟩
  · rw [hnext]
    rw [hs'f] at hg
    rcases List.mem_cons.1 hg with rfl | hg
    · exact hbelow caller (by rw [hsf]; exact List.mem_cons_of_mem _ List.mem_cons_self)
    · exact hbelow g (by rw [hsf]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hg))
  · rw [hs'f] at hg'
    rcases List.mem_cons.1 hg' with rfl | hg'
    · simp only [frameVals, List.mem_append] at hm
      rcases hm with (hm | hm) | hm
      · exact ⟨fr, by rw [hsf]; exact List.mem_cons_self, rx, mem_frameVals_stack hm, rfl, rfl⟩
      · exact ⟨caller, by rw [hsf]; exact List.mem_cons_of_mem _ List.mem_cons_self, rx,
          mem_frameVals_stack hm, rfl, rfl⟩
      · exact ⟨caller, by rw [hsf]; exact List.mem_cons_of_mem _ List.mem_cons_self, rx,
          by simp only [frameVals, List.mem_append]; exact Or.inr hm, rfl, rfl⟩
    · exact ⟨g', by rw [hsf]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hg'), rx, hm,
        rfl, rfl⟩

/-! ## Entering a callee -/

theorem Compat.symm {a b : Reference} (h : Compat a b) : Compat b a :=
  ⟨h.2.1, h.1, h.2.2.2, h.2.2.1⟩

/-- **Two held references are compatible** when each is writable if mutable. -/
theorem compat_of_writable {g : Graph} {L : Nat → Option Reference} {H : Nat → Prop}
    {root : Nat} {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}
    (hc : Complete g L H root fid gsid ps) {x y : Nat} {rx ry : Reference} (hx : H x) (hy : H y)
    (hxy : x ≠ y) (hxL : L x = some rx) (hyL : L y = some ry)
    (hwx : rx.mutable = true → g.isWritable x = .ok true)
    (hwy : ry.mutable = true → g.isWritable y = .ok true) : Compat rx ry := by
  have hnx : rx.mutable = true → ∀ w, ¬ Chain g L x w := fun hm w h => by
    obtain ⟨e, n, -, -, -, -, he, -⟩ := h.first
    exact (Graph.isWritable_true (hwx hm)).2 e n he
  have hny : ry.mutable = true → ∀ w, ¬ Chain g L y w := fun hm w h => by
    obtain ⟨e, n, -, -, -, -, he, -⟩ := h.first
    exact (Graph.isWritable_true (hwy hm)).2 e n he
  refine ⟨fun hm => ⟨fun hin => hnx hm y (hc.strict x rx y ry hx hy hxL hyL hm hin), fun heq => ?_⟩,
    fun hm => ⟨fun hin => hny hm x (hc.strict y ry x rx hy hx hyL hxL hm hin), fun heq => ?_⟩,
    fun hm hin => hc.immInside y ry x rx hy hx hyL hxL hm hin,
    fun hm hin => hc.immInside x rx y ry hx hy hxL hyL hm hin⟩
  · rcases hc.same x rx y ry hx hy hxy hxL hyL hm heq with h | h
    · exact hnx hm y h
    · cases hym : ry.mutable
      · have := (h.facts hyL hxL).2.2 hym; rw [hm] at this; cases this
      · exact hny hym x h
  · rcases hc.same y ry x rx hy hx (Ne.symm hxy) hyL hxL hm heq with h | h
    · exact hny hm x h
    · cases hxm : rx.mutable
      · have := (h.facts hxL hyL).2.2 hxm; rw [hm] at this; cases this
      · exact hnx hxm y h

/-- The global types the references an abstract state holds are rooted in. -/
def HeldGlob (A : RefState) (L : Nat → Option Reference) (t : StructId) : Prop :=
  ∃ x ∈ heldIds A, ∃ r a, L x = some r ∧ r.location.root = .global a t

/-- What the caller's references in global storage are: global borrows of
types the caller acquires, while its module is active once, or in global
storage of types the frames below protect. -/
theorem globRef_of {A : RefState} {stk : List Value} {L : Nat → Option Reference}
    {S : StructId → Prop}
    (hd : Desc A { fr with stack := stk }
      (gsidOf P ((fr.function, fr.countedActive) :: rest.map Frame.skel)) ps L)
    (hglob : GlobOk P fr.function ((fr :: rest).map Frame.skel) ps S) :
    ∀ x r a t, x ∈ heldIds A → L x = some r → r.location.root = .global a t →
      (acqTypes P fr.function t ∧
        countActive P ((fr.function, fr.countedActive) :: rest.map Frame.skel) t.module ≤ 1 ∧
        ∃ d, RootChain A.graph L A.locals.length fr.id
          (gsidOf P ((fr.function, fr.countedActive) :: rest.map Frame.skel)) (some (.global d)) x) ∨
      S t := by
  intro x r a t hx hxr hr
  rcases hd.complete.prov x r hx hxr with ⟨i, hi⟩ | ⟨d, hrc⟩ | ⟨π, hπ, hin, -⟩
  · rw [hr] at hi; cases hi
  · rcases hrc.root_of hxr with ⟨i, hi⟩ | ⟨a', d', t', hd', hr'⟩
    · rw [hr] at hi; cases hi
    · rw [hr] at hr'
      simp only [Root.global.injEq] at hr'
      obtain ⟨-, rfl⟩ := hr'
      obtain ⟨hacqT, hcnt⟩ := gsidOf_cons hd'
      exact Or.inl ⟨hacqT, hcnt, d, hrc⟩
  · exact Or.inr (hglob.params π hπ a t (by rw [← hin.1, hr]))

/-- **A caller suspends at the core of a call**: from the analysis's state
`A₀` before the core of the call (`CoreRun`) and the caller's frame with
operand stack `stk`, the `n` arguments' references are tracked and exclusive
where mutable, the references the caller keeps are shielded from the mutable
ones, the callee's reference parameters `cps` being among the arguments', and
the frames from the callee up protect the global types the frames below and
the caller's references protect or are in. `Call` runs it on the state at the
call (`call_suspendAt`), `CallClosure` once the closure is popped
(`closure_suspendAt`). -/
theorem suspend_core {stk : List Value} {A₀ A' A'' : RefState} {L : Nat → Option Reference}
    {m : CompiledModule} {rs : List SignatureToken} {rets : List RuntimeType} {n : Nat}
    {τ : List RuntimeType} {callee : FunctionRef} {cid : FrameId} {cps : List Reference}
    (hcounted : CountedOk P ((fr :: rest).map Frame.skel))
    {S : StructId → Prop} (hglob : GlobOk P fr.function ((fr :: rest).map Frame.skel) ps S)
    (hlocOk : LocalsOk P fr) (hcid : fr.id < cid)
    (hd : Desc A₀ { fr with stack := stk } (gsidOf P ((fr :: rest).map Frame.skel)) ps L)
    (hrefs : ∀ x r, x ∈ heldIds A₀ → L x = some r → RefOk P s fr r)
    (hdist : (heldIds A₀).Nodup)
    (habs : ∀ (k : Nat) (π : Reference), stk[k]? = some (Value.ref π) →
      ∃ x, A₀.stack[k]? = some (AbsValue.reference x) ∧ L x = some π)
    (hconf : ∀ v ∈ stk, ∃ t, v.conforms P t = true ∧ (t.isValue = true ∨ ∃ b u, t = .ref b u))
    (hheight : A₀.stack.length = stk.length)
    (hlocT : ∀ j r, fr.locals[j]? = some (some (.ref r)) →
      modelAvail P fr.function fr.pc j = .available →
      ∃ x, A₀.locals[j]? = some (.reference x) ∧ L x = some r)
    (hcore : CoreRun A₀ A' n rs) (hA'' : modelRef P fr.function (fr.pc + 1) = some A'')
    (hle : RefLe A' A'') (hτ : ∀ x ∈ τ, x.isValue = true)
    (hrets : RuntimeType.ofSignatureWith m τ rs = some rets)
    (hcr : P.returnTypes callee = some rets) (hn : n ≤ stk.length)
    (hcps : cps.Sublist (argRefs (stk.take n).reverse)) :
    (argRefs (stk.take n).reverse).Pairwise Compat ∧
    (∀ r ∈ argRefs (stk.take n).reverse, RefOk P s fr r) ∧
    SuspOk P s { fr with stack := stk.drop n } rest ps S cid callee cps
      (fun t => S t ∨ HeldGlob A₀ L t) ∧
    (∀ t, (S t ∨ HeldGlob A₀ L t) →
      1 ≤ countActive P ((fr.function, fr.countedActive) :: rest.map Frame.skel) t.module) ∧
    (∀ π ∈ argRefs (stk.take n).reverse, ∀ a t, π.location.root = .global a t →
      HeldGlob A₀ L t) := by
  have hsk : (fr :: rest).map Frame.skel =
      (fr.function, fr.countedActive) :: rest.map Frame.skel := rfl
  rw [hsk] at hd hcounted
  have hargsLen : (stk.take n).reverse.length = n := by
    rw [List.length_reverse, List.length_take, Nat.min_eq_left hn]
  have hcore' := hcore
  obtain ⟨-, -, -, -, -, -, -, -, -, hwr, -, -, -, -⟩ := hcore'
  have hc0 := hd.complete
  have hstkNd : (A₀.stack.filterMap AbsValue.refId?).Nodup := by
    simp only [heldIds, List.nodup_append] at hdist; exact hdist.2.1
  -- A reference argument's abstract slot.
  have hslot : ∀ k π, k < n → stk[k]? = some (.ref π) →
      ∃ x, A₀.stack[k]? = some (.reference x) ∧ L x = some π ∧ x ∈ heldIds A₀ ∧
        AbsValue.reference x ∈ A₀.stack.take n := by
    intro k π hk hπ
    obtain ⟨x, hx, hxL⟩ := habs k π hπ
    exact ⟨x, hx, hxL, mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx)),
      List.mem_iff_getElem?.2 ⟨k, by rw [List.getElem?_take_of_lt hk]; exact hx⟩⟩
  have hmemArg : ∀ π ∈ argRefs (stk.take n).reverse, ∃ k, k < n ∧ stk[k]? = some (.ref π) := by
    intro π hπ
    have h1 : Value.ref π ∈ stk.take n := List.mem_reverse.1 (mem_argRefs.1 hπ)
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 h1
    have hkn : k < n := by
      have := (List.getElem?_eq_some_iff.1 hk).1; simp at this; omega
    rw [List.getElem?_take_of_lt hkn] at hk
    exact ⟨k, hkn, hk⟩
  -- The analysis checked that a mutable argument is writable.
  have hwrA : ∀ x rx, AbsValue.reference x ∈ A₀.stack.take n → L x = some rx →
      rx.mutable = true → A₀.graph.isWritable x = .ok true := by
    intro x rx hx hxL hm
    have hH : x ∈ heldIds A₀ := mem_heldIds.2 (Or.inr (List.mem_of_mem_take hx))
    exact hwr x hx (by rw [isMutable_of (hc0.held_mut hH hxL), hm])
  have hargOk : ∀ π ∈ argRefs (stk.take n).reverse, RefOk P s fr π := by
    intro π hπ
    obtain ⟨k, hk, hkπ⟩ := hmemArg π hπ
    obtain ⟨x, -, hxL, hxH, -⟩ := hslot k π hk hkπ
    exact hrefs x π hxH hxL
  -- The arguments are compatible.
  have hpw : (argRefs (stk.take n)).Pairwise Compat := by
    refine argRefs_pairwise_of fun i j a b hij hia hjb => ?_
    have hil : i < n := by
      have := (List.getElem?_eq_some_iff.1 hia).1; simp at this; omega
    have hjl : j < n := by
      have := (List.getElem?_eq_some_iff.1 hjb).1; simp at this; omega
    rw [List.getElem?_take_of_lt hil] at hia
    rw [List.getElem?_take_of_lt hjl] at hjb
    obtain ⟨x, hx, hxL, hxH, hxt⟩ := hslot i a hil hia
    obtain ⟨y, hy, hyL, hyH, hyt⟩ := hslot j b hjl hjb
    have hxy : x ≠ y := by
      rintro rfl
      have := refId_inj hstkNd hx hy
      omega
    exact compat_of_writable hc0 hxH hyH hxy hxL hyL (hwrA x a hxt hxL) (hwrA y b hyt hyL)
  have hexcl : (argRefs (stk.take n).reverse).Pairwise Compat := by
    unfold argRefs
    rw [List.filterMap_reverse, List.pairwise_reverse]
    exact hpw.imp Compat.symm
  have hglobRef := globRef_of hd hglob
  have hactive : ∀ t, (S t ∨ HeldGlob A₀ L t) →
      1 ≤ countActive P ((fr.function, fr.countedActive) :: rest.map Frame.skel) t.module := by
    rintro t (hS | ⟨x, hx, r, a, hxr, hr⟩)
    · exact hglob.active t hS
    · rcases hglobRef x r a t hx hxr hr with ⟨hacqT, -, -⟩ | hS
      · exact countActive_self hcounted (acqTypes_funModule hacqT)
      · exact hglob.active t hS
  refine ⟨hexcl, hargOk,
    ⟨hcid, hlocOk, hglob, fun t hS => Or.inl hS, m, rs, rets, (stk.take n).reverse, A₀, A', A'',
    L,
    { retTypes := ⟨τ, hτ, hrets⟩
      calleeRets := hcr
      core := by rw [hargsLen]; exact hcore
      after := hA''
      le := hle
      cpsSub := hcps
      cpsBelow := fun π hπ => (hargOk π (hcps.subset hπ)).below
      cpsArgs := fun π hπ => by
        obtain ⟨k, hk, hkπ⟩ := hmemArg π hπ
        obtain ⟨x, -, hxL, -, hxt⟩ := hslot k π hk hkπ
        exact ⟨x, by rw [hargsLen]; exact hxt, hxL⟩
      distinct := hdist
      desc := ?_
      height := ?_
      refs := ?_
      exposure := ?_
      globals := fun x r hx hxr a t hr => Or.inr ⟨x, hx, r, a, hxr, hr⟩
      stackConf := fun v hv => hconf v (List.mem_of_mem_drop hv)
      immSame := ?_
      tracked := ?_ }⟩, hactive, fun π hπ a t hr => ?_⟩
  · have e : ({ ({ fr with stack := stk.drop n } : Frame) with
        stack := (stk.take n).reverse.reverse ++
          ({ fr with stack := stk.drop n } : Frame).stack } : Frame) = { fr with stack := stk } := by
      simp only [List.reverse_reverse, List.take_append_drop]
    rw [e]; exact hd
  · show A₀.stack.length = (stk.take n).reverse.length + (stk.drop n).length
    rw [hargsLen, List.length_drop, hheight]
    omega
  · intro x r hx hxr
    have hx' : AbsValue.reference x ∈ A₀.locals ∨
        AbsValue.reference x ∈ A₀.stack.drop n := by
      have := mem_heldIds.1 hx
      rw [hargsLen] at this
      exact this
    have hxH : x ∈ heldIds A₀ := by
      rcases hx' with h | h
      · exact mem_heldIds.2 (Or.inl h)
      · exact mem_heldIds.2 (Or.inr (List.mem_of_mem_drop h))
    have hok := hrefs x r hxH hxr
    refine ⟨⟨hok.root, hok.below, hok.value, hok.since, hok.valid, hok.writable⟩,
      fun π hπ hπm hin => ?_⟩
    obtain ⟨k, hk, hkπ⟩ := hmemArg π (hcps.subset hπ)
    obtain ⟨y, -, hyL, hyH, hyt⟩ := hslot k π hk hkπ
    obtain ⟨e, n, -, -, -, -, he, -⟩ := (hc0.strict y π x r hyH hxH hyL hxr hπm hin).first
    exact (Graph.isWritable_true (hwrA y π hyt hyL hπm)).2 e n he
  · intro π hπ hπm
    obtain ⟨k, hk, hkπ⟩ := hmemArg π (hcps.subset hπ)
    obtain ⟨y, -, hyL, hyH, -⟩ := hslot k π hk hkπ
    rcases hc0.prov3 hyH hyL with ⟨i, hi⟩ | h | ⟨π', hπ', hin, hm⟩
    · exact Or.inl ⟨i, hi⟩
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr ⟨π', hπ', hm hπm, hin⟩)
  · -- An immutable reference the caller keeps is not at a mutable argument's location.
    intro x r hx hxr hrm π hπ hπm heq
    have hx' : AbsValue.reference x ∈ A₀.locals ∨
        AbsValue.reference x ∈ A₀.stack.drop n := by
      have := mem_heldIds.1 hx
      rw [hargsLen] at this
      exact this
    have hxH : x ∈ heldIds A₀ := by
      rcases hx' with h | h
      · exact mem_heldIds.2 (Or.inl h)
      · exact mem_heldIds.2 (Or.inr (List.mem_of_mem_drop h))
    obtain ⟨k, hk, hkπ⟩ := hmemArg π (hcps.subset hπ)
    obtain ⟨y, -, hyL, hyH, hyt⟩ := hslot k π hk hkπ
    have hyx : y ≠ x := by
      rintro rfl
      rw [hyL, Option.some.injEq] at hxr
      subst hxr
      rw [hπm] at hrm
      cases hrm
    rcases hc0.same y π x r hyH hxH hyx hyL hxr hπm heq with hc | hc
    · obtain ⟨e, n, -, -, -, -, he, -⟩ := hc.first
      exact (Graph.isWritable_true (hwrA y π hyt hyL hπm)).2 e n he
    · have := hc.mutable_src hxr hyL hπm
      rw [hrm] at this
      cases this
  · -- The references the caller keeps are tracked.
    intro r hr
    rcases hr with hr | ⟨j, hj, hav⟩
    · obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hr
      simp only [List.getElem?_drop] at hk
      obtain ⟨x, hx, hxL⟩ := habs _ r hk
      refine ⟨x, mem_heldIds.2 (Or.inr ?_), hxL⟩
      simp only [hargsLen]
      exact List.mem_of_getElem? (by rw [List.getElem?_drop]; exact hx)
    · obtain ⟨x, hx, hxL⟩ := hlocT j r hj hav
      exact ⟨x, mem_heldIds.2 (Or.inl (List.mem_of_getElem? hx)), hxL⟩
  · obtain ⟨k, hk, hkπ⟩ := hmemArg π hπ
    obtain ⟨y, -, hyL, hyH, -⟩ := hslot k π hk hkπ
    exact ⟨y, hyH, π, a, hyL, hr⟩

/-- **The caller suspends at a call**: the analysis's state at the call
describes it, its reference arguments are tracked and exclusive where
mutable, the references it keeps are shielded from the mutable ones, and the
callee protects the global types the caller's references are in. -/
theorem call_suspendAt (hwf : WellFormedBy verifier VerifierConfig.production P)
    {h : FunctionHandleIndex} {instr : Bytecode}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hlocOk : LocalsOk P fr) (htargs : ∀ x ∈ fr.function.typeArgs, x.isValue = true)
    (hi : code.code[fr.pc]? = some instr) (hcallI : CallsHandle info.module instr h)
    (hret : instr ≠ .ret) (hctl : instr.control = .fallThrough)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view (.call h)).run A = .ok ((), A'))
    (hfn : P.function? fr.function = some info) (hbody : info.definition.code = some code)
    {fh : FunctionHandle} (hfh : info.module.functionHandles[h]? = some fh)
    (hattr : ReferenceSafety.borrowFieldMutable fh.attributes = none)
    {params rets : List RuntimeType} {c0 : FunctionRef} {cinfo : FunctionInfo}
    (hres : P.resolve fr.function.module h = some c0) (hcf : P.function? c0 = some cinfo)
    {τ : List RuntimeType} (hτ : ∀ x ∈ τ, x.isValue = true)
    (hparams : P.handleParamTypesAt fr.function.module h τ = some params)
    (hreturns : P.handleReturnTypesAt fr.function.module h τ = some rets)
    (hcr : P.returnTypes { c0 with typeArgs := τ } = some rets)
    (hn : params.length ≤ fr.stack.length) {cid : FrameId} (hcid : fr.id < cid)
    (hsf : s.frames = fr :: rest) (hcounted : CountedOk P (s.frames.map Frame.skel))
    {S : StructId → Prop} (hglob : GlobOk P fr.function ((fr :: rest).map Frame.skel) ps S) :
    ∃ Sc : StructId → Prop,
    (argRefs (fr.stack.take params.length).reverse).Pairwise Compat ∧
    (∀ r ∈ argRefs (fr.stack.take params.length).reverse, RefOk P s fr r) ∧
    SuspOk P s { fr with stack := fr.stack.drop params.length } rest ps S cid
      { c0 with typeArgs := τ } (argRefs (fr.stack.take params.length).reverse) Sc ∧
    GlobOk P { c0 with typeArgs := τ }
      (({ c0 with typeArgs := τ }, cinfo.moduleId != info.moduleId) :: (fr :: rest).map Frame.skel)
      (argRefs (fr.stack.take params.length).reverse) Sc := by
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hsk : s.frames.map Frame.skel = (fr :: rest).map Frame.skel := by rw [hsf]
  rw [hsk] at hd hcounted
  obtain ⟨A', hx, hsucc⟩ := hctx.next hret
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, hctl])
  have hx := hrun _ hx
  obtain ⟨psig, rs, hps, hrs, hcore⟩ := exec_call_core hx hfh hattr
  obtain ⟨fh'', hfh'', hacqChk⟩ := exec_call_acquires hx
  rw [hfh, Option.some.injEq] at hfh''
  subst hfh''
  obtain ⟨acq, hacq, hchk⟩ := hacqChk hattr
  have hmod := (function?_parts hfn).1
  have hpsig : RuntimeType.ofSignatureWith info.module τ psig = some params := by
    have h1 : P.handleParamTypesAt fr.function.module h τ = some params := hparams
    simp only [Program.handleParamTypesAt, hmod, hfh, hps, bind,
      Option.bind_some] at h1
    exact h1
  have hrets : RuntimeType.ofSignatureWith info.module τ rs = some rets := by
    have h1 : P.handleReturnTypesAt fr.function.module h τ = some rets := hreturns
    simp only [Program.handleReturnTypesAt, hmod, hfh, hrs, bind,
      Option.bind_some] at h1
    exact h1
  have hnp : params.length = psig.length := ofSignature_length hpsig
  rw [← hnp] at hcore
  obtain ⟨hexcl, hargOk, hsusp, hactive, hparG⟩ := suspend_core (stk := fr.stack) hcounted hglob
    hlocOk hcid hd hrefs hctx.typed.distinct
    (fun k π hπ => abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hπ)
    (fun v hv => by
      obtain ⟨t, ht, hc⟩ := conformsAll_mem hctx.conf v hv
      exact ⟨t, hc, slotType_of_ofSignature hctx.targs hctx.sig t ht⟩)
    hctx.height
    (fun j r hj hav => by
      obtain ⟨t, ht, hkind⟩ := localToken_ref hfn hbody hctx.view hlocOk htargs hj
      obtain ⟨x, hx⟩ := hctx.typed.locKind j t r.mutable ht hkind hav
      obtain ⟨r', hr', hxL⟩ := hd.locals j x hx
      rw [hj] at hr'
      simp only [Option.some.injEq, Value.ref.injEq] at hr'
      subst hr'
      exact ⟨x, hx, hxL⟩)
    hcore hA'' hle hτ hrets hcr hn (List.Sublist.refl _)
  refine ⟨_, hexcl, hargOk, hsusp, ?_⟩
  have hglobRef := globRef_of (stk := fr.stack) hd hglob
  -- What the callee protects.
  have hcnt : ∀ M, countActive P (({ c0 with typeArgs := τ }, cinfo.moduleId != info.moduleId) ::
      (fr :: rest).map Frame.skel) M =
      (if (cinfo.moduleId != info.moduleId) = true ∧
        funModule P { c0 with typeArgs := τ } = some M then 1 else 0) +
        countActive P ((fr.function, fr.countedActive) :: rest.map Frame.skel) M := by
    intro M; rw [countActive_cons]; rfl
  refine ⟨fun t hSc => ?_, fun t hSc hacqT => ?_, fun π hπ a t hr => Or.inr (hparG π hπ a t hr)⟩
  · rw [hcnt]; exact Nat.le_trans (hactive t hSc) (Nat.le_add_left _ _)
  · rw [hcnt]
    have hcm : funModule P { c0 with typeArgs := τ } = some cinfo.moduleId := by
      simp [funModule, function?_typeArgs, hcf]
    have htm := acqTypes_funModule hacqT
    rw [hcm, Option.some.injEq] at htm
    by_cases hself : fh.module = info.module.selfModuleHandleIdx
    · -- A callee of the caller's module: it acquires only what the caller does, and
      -- nothing the caller borrowed.
      obtain ⟨hcmod, hsubF, hsubA⟩ := acqOf_call hwf hfn hbody hi hcallI hfh hself hres
      have hsame : cinfo.moduleId = info.moduleId := by
        obtain ⟨hm, hid, -, -⟩ := function?_parts hfn
        obtain ⟨hm', hid', -, -⟩ := function?_parts hcf
        rw [hcmod, hm, Option.some.injEq] at hm'
        rw [← hm', hid, Option.some.injEq] at hid'
        exact hid'.symm
      have hacqF : acqTypes P fr.function t := by
        obtain ⟨d, hd', hdef⟩ := hacqT
        exact ⟨d, hsubF d hd', by rw [← hcmod]; exact hdef⟩
      simp only [hsame, bne_self_eq_false, Bool.false_eq_true, false_and, if_false, Nat.zero_add]
      rcases hSc with hS | ⟨x, hx, r, a, hxr, hr⟩
      · exact hglob.acq t hS hacqF
      · rcases hglobRef x r a t hx hxr hr with ⟨-, -, d, hrc⟩ | hS
        · exact absurd hacqT (rootChain_not_acq hrc hxr hr hchk (hsubA acq hacq) hcmod)
        · exact hglob.acq t hS hacqF
    · -- A callee of another module: it counts as another activation.
      have hne := resolve_cross (function?_parts hfn).1 (function?_parts hfn).2.1
        (verifyDuplication_ok (verifyModule_ok (wellFormed_module hwf
          (function?_parts hfn).1).1).2.1) hfh hself hres hcf
      have hb : (cinfo.moduleId != info.moduleId) = true := by simpa using hne
      rw [if_pos ⟨hb, by rw [hcm, htm]⟩]
      have := hactive t hSc
      omega

/-- **The caller suspends at a call** of a non-generic function. -/
theorem call_suspend (hwf : WellFormedBy verifier VerifierConfig.production P)
    {h : FunctionHandleIndex}
    (hctx : TopCtx P s fr info code (.call h) view A stys tys ps)
    (hlocOk : LocalsOk P fr) (htargs : ∀ x ∈ fr.function.typeArgs, x.isValue = true)
    (hi : code.code[fr.pc]? = some (.call h))
    (hfn : P.function? fr.function = some info) (hbody : info.definition.code = some code)
    {fh : FunctionHandle} (hfh : info.module.functionHandles[h]? = some fh)
    (hattr : ReferenceSafety.borrowFieldMutable fh.attributes = none)
    {params rets : List RuntimeType} {callee : FunctionRef} {cinfo : FunctionInfo}
    (hres : P.resolve fr.function.module h = some callee) (hcf : P.function? callee = some cinfo)
    (hparams : P.handleParamTypes fr.function.module h = some params)
    (hreturns : P.handleReturnTypes fr.function.module h = some rets)
    (hcr : P.returnTypes callee = some rets)
    (hn : params.length ≤ fr.stack.length) {cid : FrameId} (hcid : fr.id < cid)
    (hsf : s.frames = fr :: rest) (hcounted : CountedOk P (s.frames.map Frame.skel))
    {S : StructId → Prop} (hglob : GlobOk P fr.function ((fr :: rest).map Frame.skel) ps S) :
    ∃ Sc : StructId → Prop,
    (argRefs (fr.stack.take params.length).reverse).Pairwise Compat ∧
    (∀ r ∈ argRefs (fr.stack.take params.length).reverse, RefOk P s fr r) ∧
    SuspOk P s { fr with stack := fr.stack.drop params.length } rest ps S cid callee
      (argRefs (fr.stack.take params.length).reverse) Sc ∧
    GlobOk P callee ((callee, cinfo.moduleId != info.moduleId) :: (fr :: rest).map Frame.skel)
      (argRefs (fr.stack.take params.length).reverse) Sc := by
  have hceq := withTypeArgs_self (resolve_typeArgs hres)
  have := call_suspendAt (τ := []) hwf hctx hlocOk htargs hi (Or.inl rfl) (by intro h; cases h) rfl
    (fun _ hx => hx) hfn hbody hfh hattr hres hcf (by simp) hparams hreturns
    (by rw [hceq]; exact hcr) hn hcid hsf hcounted hglob
  rwa [hceq] at this

/-- **The caller suspends at a closure call**: the analysis's state at the
call, the closure popped, describes the caller's frame with the closure
popped, and the caller suspends at the core of the call (`suspend_core`).
The callee's frame counts as an activation: the frames from it up protect
the global types the callee acquires, of the callee's module, which is
active below. -/
theorem closure_suspendAt {sig : SignatureIndex}
    (hctx : TopCtx P s fr info code (.callClosure sig) view A stys tys ps)
    (hlocOk : LocalsOk P fr) (htargs : ∀ x ∈ fr.function.typeArgs, x.isValue = true)
    (hi : code.code[fr.pc]? = some (.callClosure sig))
    (hfn : P.function? fr.function = some info) (hbody : info.definition.code = some code)
    {c : Value} {vs : List Value} (hstk : fr.stack = c :: vs) (hc : ∀ r, c ≠ .ref r)
    {ps0 rs0 : List SignatureToken} {a : AbilitySet} {pts rts : List RuntimeType}
    (htoks : info.module.signatures[sig]? = some [.function ps0 rs0 a])
    (hpts : RuntimeType.ofSlotTokensWith info.module fr.function.typeArgs ps0 = some pts)
    (hrts : RuntimeType.ofSlotTokensWith info.module fr.function.typeArgs rs0 = some rts)
    {callee : FunctionRef} (hcr : P.returnTypes callee = some rts)
    (hn : pts.length ≤ vs.length) {cid : FrameId} (hcid : fr.id < cid)
    (hsf : s.frames = fr :: rest) (hcounted : CountedOk P (s.frames.map Frame.skel))
    {S : StructId → Prop} (hglob : GlobOk P fr.function ((fr :: rest).map Frame.skel) ps S)
    {cps : List Reference} (hcps : cps.Sublist (argRefs (vs.take pts.length).reverse)) :
    ∃ Sc : StructId → Prop,
    (argRefs (vs.take pts.length).reverse).Pairwise Compat ∧
    (∀ r ∈ argRefs (vs.take pts.length).reverse, RefOk P s fr r) ∧
    SuspOk P s { fr with stack := vs.drop pts.length } rest ps S cid callee cps Sc ∧
    GlobOk P callee ((callee, true) :: (fr :: rest).map Frame.skel) cps Sc := by
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hsk : s.frames.map Frame.skel = (fr :: rest).map Frame.skel := by rw [hsf]
  rw [hsk] at hd hcounted
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
  obtain ⟨ps', rs', ab, v, rest', hhead, hs, hcore⟩ := exec_callClosure hx
  simp only [htoks, Option.bind_some, List.head?_cons, Option.some.injEq,
    SignatureToken.function.injEq] at hhead
  obtain ⟨rfl, rfl, -⟩ := hhead
  have hpts' : RuntimeType.ofSignatureWith info.module fr.function.typeArgs ps0 = some pts := by
    rw [← ofSlotTokensWith_eq]; exact hpts
  have hrts' : RuntimeType.ofSignatureWith info.module fr.function.typeArgs rs0 = some rts := by
    rw [← ofSlotTokensWith_eq]; exact hrts
  have hnp : pts.length = ps0.length := ofSignature_length hpts'
  rw [← hnp] at hcore
  -- The closure is a value.
  have hpop1 : ∀ w ∈ fr.stack.take 1, ∀ r, w ≠ .ref r := by
    rw [hstk]; simpa using hc
  have hv : v = .nonReference :=
    hd.take_nonRef hctx.height hpop1 v (by rw [hs]; exact List.mem_cons_self)
  subst hv
  have hvs := ValueStep.popOne hs
  have hd₀ : Desc { A with stack := rest' } { fr with stack := vs }
      (gsidOf P ((fr :: rest).map Frame.skel)) ps L :=
    hd.valueStep hvs rfl rfl (new := []) (j' := 1) (by simp [hstk]) hpop1
  have hheld₀ : heldIds { A with stack := rest' } = heldIds A := hvs.heldIds
  obtain ⟨hexcl, hargOk, hsusp, hactive, hparG⟩ := suspend_core (stk := vs)
    (τ := fr.function.typeArgs) (callee := callee) hcounted hglob hlocOk hcid hd₀
    (fun x r hx hxr => hrefs x r (by rw [← hheld₀]; exact hx) hxr)
    (by rw [hheld₀]; exact hctx.typed.distinct)
    (fun k π hπ => by
      have hk : fr.stack[k + 1]? = some (.ref π) := by rw [hstk]; exact hπ
      obtain ⟨x, hx, hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hk
      rw [hs] at hx
      exact ⟨x, hx, hxL⟩)
    (fun w hw => by
      obtain ⟨t, ht, hct⟩ :=
        conformsAll_mem hctx.conf w (by rw [hstk]; exact List.mem_cons_of_mem _ hw)
      exact ⟨t, hct, slotType_of_ofSignature hctx.targs hctx.sig t ht⟩)
    (by have := hctx.height; rw [hs, hstk] at this; simpa using this)
    (fun j r hj hav => by
      obtain ⟨t, ht, hkind⟩ := localToken_ref hfn hbody hctx.view hlocOk htargs hj
      obtain ⟨x, hx⟩ := hctx.typed.locKind j t r.mutable ht hkind hav
      obtain ⟨r', hr', hxL⟩ := hd.locals j x hx
      rw [hj] at hr'
      simp only [Option.some.injEq, Value.ref.injEq] at hr'
      subst hr'
      exact ⟨x, hx, hxL⟩)
    hcore hA'' hle htargs hrts' hcr hn hcps
  refine ⟨_, hexcl, hargOk, hsusp, fun t hSc => ?_, fun t hSc hacqT => ?_,
    fun π hπ a t hr => Or.inr (hparG π (hcps.subset hπ) a t hr)⟩
  · rw [countActive_cons]
    exact Nat.le_trans (hactive t hSc) (Nat.le_add_left _ _)
  · -- The callee acquires only types of its own module, active below it.
    rw [countActive_cons]
    have htm := acqTypes_funModule hacqT
    have h1 : 1 ≤ countActive P ((fr :: rest).map Frame.skel) t.module := hactive t hSc
    simp only [htm, and_self, if_true]
    omega

/-- The frames older than a new callee keep their functions and locals. -/
theorem enter_frames (hsf : s.frames = fr :: rest) {fr1 frame : Frame}
    (hs'f : s'.frames = frame :: fr1 :: rest) (hid1 : fr1.id = fr.id)
    (hloc1 : fr1.locals = fr.locals) (hfun1 : fr1.function = fr.function)
    (hfid : frame.id = s.nextFrame) :
    ∀ f, f < s.nextFrame → ∀ g, s.frame? f = some g →
      ∃ g', s'.frame? f = some g' ∧ g'.function = g.function ∧ g'.locals = g.locals := by
  intro f hf g hg
  have h0 : (frame.id == f) = false := by
    have : frame.id ≠ f := by
      rw [hfid]; intro e; rw [e] at hf; exact Nat.lt_irrefl _ hf
    simpa using this
  simp only [State.frame?, hsf, hs'f, List.find?_cons, h0, hid1] at hg ⊢
  by_cases h1 : fr.id = f
  · have h1' : (fr.id == f) = true := by simpa using h1
    simp only [h1', Option.some.injEq] at hg ⊢
    subst hg
    exact ⟨fr1, rfl, hfun1, hloc1⟩
  · have h1' : (fr.id == f) = false := by simpa using h1
    simp only [h1'] at hg ⊢
    exact ⟨g, hg, rfl, rfl⟩

/-- **Entering a callee** keeps the reference invariant, given the callee's
first frame and the suspended chain from the caller down. -/
theorem refInv_enter (hr : RefInvariant P s) (hsf : s.frames = fr :: rest) {fr1 frame : Frame}
    (hs'f : s'.frames = frame :: fr1 :: rest)
    (hid1 : fr1.id = fr.id) (hloc1 : fr1.locals = fr.locals) (hfun1 : fr1.function = fr.function)
    (hfid : frame.id = s.nextFrame)
    (hnext : s'.nextFrame = s.nextFrame + 1) (hstor : s'.storage = s.storage)
    (hinv : s'.invalidations = s.invalidations) (hheld : s'.held = s.held) {cps : List Reference}
    (hca1 : fr1.countedActive = fr.countedActive) {id cid : ModuleId}
    (hfm : funModule P fr.function = some id) (hcm : funModule P frame.function = some cid)
    (hcross : cid ≠ id → frame.countedActive = true)
    (hact : s'.active = (if frame.countedActive then s.incActive cid else s).active)
    (htop : ∃ A0, modelRef P frame.function frame.pc = some A0 ∧ FrameOk P s' frame cps A0)
    {Sc : StructId → Prop}
    (hglobC : GlobOk P frame.function ((frame :: fr1 :: rest).map Frame.skel) cps Sc)
    (hchain : ChainRef P s s.nextFrame frame.function cps Sc (fr1 :: rest))
    (hfromC : RefsFrom fr frame) (hfrom1 : RefsFrom fr fr1) :
    RefInvariant P s' := by
  obtain ⟨hids, hbelow, hstor0, -, hactv, hheldT⟩ := hr
  have hfresh : s.nextFrame ∉ s.frames.map (·.id) := fun h => by
    obtain ⟨g, hg, hge⟩ := List.mem_map.1 h
    have := hbelow g hg
    have hge' : g.id = s.nextFrame := hge
    rw [hge'] at this
    exact Nat.lt_irrefl _ this
  have hids' : (s'.frames.map (·.id)).Nodup := by
    rw [hs'f, List.map_cons, List.map_cons, hfid, hid1]
    rw [hsf, List.map_cons] at hids hfresh
    exact List.nodup_cons.2 ⟨hfresh, hids⟩
  refine ⟨hids', fun g hg => ?_, fun r hr' => hstor0 r (by rw [← hstor]; exact hr'), ⟨frame,
    fr1 :: rest, cps, hs'f, htop, Sc, by rw [← hs'f] at hglobC ⊢; exact hglobC, ?_⟩,
    hactv.push hsf hs'f (by simp only [Frame.skel, hfun1, hca1]) hfm hcm hcross hact,
    hheldT.of fun g' hg' rx hm k hk => ?_⟩
  · rw [hnext]
    rw [hs'f] at hg
    rcases List.mem_cons.1 hg with rfl | hg
    · rw [hfid]; exact Nat.lt_succ_self _
    rcases List.mem_cons.1 hg with rfl | hg
    · rw [hid1]; exact Nat.lt_succ_of_lt (hbelow fr (by rw [hsf]; exact List.mem_cons_self))
    · exact Nat.lt_succ_of_lt (hbelow g (by rw [hsf]; exact List.mem_cons_of_mem _ hg))
  · rw [hfid]
    exact ChainRef.transferBelow (enter_frames hsf hs'f hid1 hloc1 hfun1 hfid) hheld hstor hinv _ _ _
      _ _ (Nat.le_refl _) hchain
  · have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self
    rw [hs'f] at hg'
    rcases List.mem_cons.1 hg' with rfl | hg'
    · obtain ⟨r₀, h₀, h₁, h₂⟩ := hfromC rx hm k hk
      exact ⟨fr, hfr, r₀, h₀, h₁, h₂⟩
    rcases List.mem_cons.1 hg' with rfl | hg'
    · obtain ⟨r₀, h₀, h₁, h₂⟩ := hfrom1 rx hm k hk
      exact ⟨fr, hfr, r₀, h₀, h₁, h₂⟩
    · exact ⟨g', by rw [hsf]; exact List.mem_cons_of_mem _ hg', rx, hm, rfl, rfl⟩

section Steps

variable (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hm3 : P.M3) (hapi : P.ApiSigs)

include hwf hs hapi in
/-- **Calling code** keeps the reference invariant: the callee's first frame
agrees with its certificate, and the caller suspends at its call. The
callee's frame counts as an activation (`b`) at least when its module is
another one. -/
theorem refInv_call' (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    {fr1 frame : Frame} (hs'f : s'.frames = frame :: fr1 :: rest)
    (hid1 : fr1.id = fr.id) (hloc1 : fr1.locals = fr.locals) (hfun1 : fr1.function = fr.function)
    (hfid : frame.id = s.nextFrame) (hpc0 : frame.pc = 0)
    (hnext : s'.nextFrame = s.nextFrame + 1) (hstor : s'.storage = s.storage)
    (hinv : s'.invalidations = s.invalidations) (hheld : s'.held = s.held)
    (hca1 : fr1.countedActive = fr.countedActive)
    {callee : FunctionRef} {cinfo : FunctionInfo} {ccode : CodeUnit}
    (hcf : P.function? callee = some cinfo) (hcode : cinfo.definition.code = some ccode)
    (htok : P.typeArgsOk callee = true)
    {id : ModuleId} (hfm : funModule P fr.function = some id) {b : Bool}
    (hcross : frame.countedActive = b) (hcb : cinfo.moduleId ≠ id → b = true)
    (hact : s'.active = (if frame.countedActive then s.incActive cinfo.moduleId else s).active)
    (hfun : frame.function = callee)
    {args : List Value} {params : List RuntimeType} (hpar : P.paramTypes callee = some params)
    {k : Nat} (hloc : frame.locals = (args.map some ++ List.replicate (k - args.length) none).toArray)
    (hargs : conformsAll P args params = true) (hexcl : (argRefs args).Pairwise Compat)
    (hargOk : ∀ r ∈ argRefs args, RefOk P s fr r) {S Sc : StructId → Prop}
    (hsusp : SuspOk P s fr1 rest ps S s.nextFrame callee (argRefs args) Sc)
    (hglobC : GlobOk P callee ((callee, b) :: (fr :: rest).map Frame.skel) (argRefs args) Sc)
    (hchain : ChainRef P s fr.id fr.function ps S rest)
    (hfromC : RefsFrom fr frame) (hfrom1 : RefsFrom fr fr1) : RefInvariant P s' := by
  have hfr := enter_frames hsf hs'f hid1 hloc1 hfun1 hfid
  have hlt : fr.id < s.nextFrame := hr.below fr (by rw [hsf]; exact List.mem_cons_self)
  have hok : ∀ r ∈ argRefs args, RefOk P s' frame r := fun r hr' =>
    (hargOk r hr').agree (hargOk r hr').below (fun f hf g hg => hfr f (Nat.lt_of_le_of_lt hf hlt) g hg)
      hheld hstor hinv (by rw [hfid]; exact Nat.le_of_lt hlt)
  have hbelow : ∀ r ∈ argRefs args, ∀ f' i, r.location.root = .local_ f' i → f' < frame.id := by
    intro r hr' f' i hρ
    have hb := (hargOk r hr').below
    rw [hρ] at hb
    rw [hfid]
    exact Nat.lt_of_le_of_lt hb hlt
  obtain ⟨A0, hA0, hok0⟩ := frameOk_entry' hwf hs hapi hcf hcode htok hpar hloc hargs hexcl hok
    hbelow
  have hcm : funModule P frame.function = some cinfo.moduleId := by
    simp [funModule, hfun, hcf]
  have hskC : (frame :: fr1 :: rest).map Frame.skel =
      (callee, b) :: (fr :: rest).map Frame.skel := by
    simp only [List.map_cons, Frame.skel, hfun, hcross, hfun1, hca1]
  exact refInv_enter hr hsf hs'f hid1 hloc1 hfun1 hfid hnext hstor hinv hheld hca1 hfm hcm
    (fun hne => hcross.trans (hcb hne)) hact ⟨A0, by rw [hfun, hpc0]; exact hA0, hok0⟩ (hglobC.skel hfun hskC)
    ⟨ps, S, by rw [hfun]; exact hsusp, by rw [hid1, hfun1]; exact hchain⟩ hfromC hfrom1

include hwf hs in
/-- **`Call`** keeps the reference invariant, into the callee or, for a
native, back in the caller. -/
theorem refRuns_call' (hapi : P.ApiSigs) (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {h : FunctionHandleIndex}
    (hctx : TopCtx P s fr info code (.call h) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.call h))
    (hattrH : ∀ fh, info.module.functionHandles[h]? = some fh →
      ReferenceSafety.borrowFieldMutable fh.attributes = none)
    (hnm : NoModeledCall P fr.function.module info.module (.call h)) :
    ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code (.call h)) := by
  obtain ⟨callee, cinfo, hres, hvis, hcf, htp, hpar, hret⟩ :=
    hc.calls fr.function info code hact.fn hact.body fr.pc h hact.reachable hi
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨cfh, hcfh, hta⟩ := hta
  split at hta
  · simp at hta
  rename_i hcne
  have hcfh0 : cfh.typeParameters = [] := by simpa using hcne
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨params, hparams, returns, hreturns, hta⟩ := hta
  split at hta
  case isFalse => simp at hta
  rename_i hpre
  have hstack := hctx.conf
  have hlen := conformsAll_length hstack
  have htl := assignableAll_length hpre
  have hge : params.length ≤ fr.stack.length := by
    rw [hlen]; simp only [List.length_take, List.length_reverse] at htl; omega
  have hargs : conformsAll P (fr.stack.take params.length).reverse params = true := by
    have := conformsAll_reverse
      (conformsAll_assignable (conformsAll_take params.length hstack) hpre)
    rwa [List.reverse_reverse] at this
  have hpar' : P.paramTypes callee = some params := hpar.trans hparams
  have hret' : P.returnTypes callee = some returns := hret.trans hreturns
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
  obtain ⟨fh, hfh, -⟩ := exec_call hx
  have hattr := hattrH fh hfh
  have hmod := (function?_parts hact.fn).1
  have hceq := withTypeArgs_self (resolve_typeArgs hres)
  rw [hfh, Option.some.injEq] at hcfh
  subst hcfh
  simp only [Step.exec, Step.call, Step.need, hres, hvis, hcf, hfh, hcfh0, htp, hceq, hpar', hret',
    hparams, hreturns, RuntimeType.argumentsWellFormed, List.length_nil,
    Step.popN, Nat.not_lt.2 hge, hargs, bne_self_eq_false, Bool.or_false, Bool.not_true,
    Bool.false_eq_true, ↓reduceIte, bind, Except.bind, pure, Except.pure]
  cases hcode : cinfo.definition.code with
  | none =>
    replace hnm := hnm _ hres
    have hnv := hc.nativeValues _ _ hcf hcode (typeArgsOk_nil hcf (resolve_typeArgs hres) htp) hnm
    have hval : ((params ++ returns).all RuntimeType.isValue) = true := by
      rw [List.all_eq_true]
      intro t ht
      rcases List.mem_append.1 ht with ht | ht
      · exact hnv.1 _ hpar' t ht
      · exact hnv.2 _ hret' t ht
    simp only [Option.isNone_none, hval, Bool.not_true, Bool.and_false, Bool.false_eq_true,
      ↓reduceIte]
    -- A native takes and returns values: the analysis's call is a step on values.
    obtain ⟨fh', hfh', hvs⟩ := exec_call_values hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    obtain ⟨ps', rs, hps, hrs, hvs⟩ := hvs hattr
    have hpl : params.length = ps'.length := by
      have h1 : P.handleParamTypes fr.function.module h = some params := hparams
      simp only [Program.handleParamTypes, Program.handleParamTypesAt, hmod, hfh, hps, bind,
      Option.bind_some] at h1
      exact ofSignature_length h1
    have hrsig : RuntimeType.ofSignature info.module rs = some returns := by
      have h1 : P.handleReturnTypes fr.function.module h = some returns := hreturns
      simp only [Program.handleReturnTypes, Program.handleReturnTypesAt, hmod, hfh, hrs, bind,
      Option.bind_some] at h1
      exact h1
    have hnoref : ∀ v ∈ fr.stack.take params.length, ∀ r, v ≠ .ref r := fun v hv =>
      noRef_of_conformsAll hargs (hnv.1 _ hpar') v (List.mem_reverse.2 hv)
    have hrsv : ∀ t ∈ rs, refKind t = none := by
      intro t ht
      obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 ht
      obtain ⟨u, hu, hut⟩ := ofSignature_getElem hrsig hj
      cases hk : refKind t with
      | none => rfl
      | some b =>
        obtain ⟨v, rfl⟩ := ofSlotToken_ref hut hk
        have := hnv.2 _ hret' _ (List.mem_of_getElem? hu)
        simp [RuntimeType.isValue] at this
    obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
    have hvs' : ValueStep ps'.length rs.length A A' :=
      hvs (hpl ▸ hd.take_nonRef hctx.height hnoref) hrsv
    have hnoref' : ∀ v ∈ (fr.stack.take params.length).reverse, ∀ r, v ≠ .ref r :=
      fun v hv => hnoref v (List.mem_reverse.1 hv)
    simp only [nativeView_of_noRefs _ hnoref', exclusiveArguments_of_noRefs hnoref',
      Bool.not_true, Bool.false_eq_true, ↓reduceIte]
    intro r _
    cases r with
    | failed e => trivial
    | returned vals writes places =>
      rw [resumeNative_values rfl hret' hnm hnoref' (hnv.2 _ hret')]
      split
      · next hcv =>
        replace hcv : conformsAll P vals returns = true := by
          simp only [Bool.and_eq_true] at hcv; exact hcv.1.1
        exact refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage
          (es := []) rfl (storageKept_of_eq rfl) allowedBy_nil
          (hd.valueStep hvs' rfl rfl (new := vals.reverse) (j' := params.length) rfl hnoref)
          (fun x r hx hxr => (hrefs x r (by rw [hvs'.heldIds] at hx; exact hx) hxr).same hsf rfl
            rfl rfl rfl rfl rfl rfl) hA'' hle
          (RefsFrom.next (new := vals.reverse) (n := params.length) rfl rfl fun v hv =>
            .of_not_ref (noRef_of_conformsAll hcv (hnv.2 _ hret') v (List.mem_reverse.1 hv)))
      · trivial
  | some ccode =>
    simp only [Option.isNone_some, Bool.false_and, Bool.false_eq_true, ↓reduceIte]
    obtain ⟨ltys, hltys⟩ := hc.locals callee cinfo ccode hcf hcode
      (typeArgsOk_nil hcf (resolve_typeArgs hres) htp)
    have hcid : fr.id < s.nextFrame := hr.below fr (by rw [hsf]; exact List.mem_cons_self)
    obtain ⟨S, hglob, hchainS⟩ := hctx.chain _ hsf
    obtain ⟨Sc, hexcl, hargOk, hsusp, hglobC⟩ := call_suspend hwf hctx hact.locals
      (typeArgsOk_values hact.typeArgs) hi hact.fn
      hact.body hfh hattr hres hcf hparams hreturns hret' hge hcid hsf hr.active.counted hglob
    have hfm0 : funModule P fr.function = some info.moduleId := by simp [funModule, hact.fn]
    simp only [hltys]
    repeat' split
    all_goals first
      | trivial
      | (show RefInvariant P _
         exact refInv_call' hwf hs hapi hr hsf rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl hcf hcode
          (typeArgsOk_nil hcf (resolve_typeArgs hres) htp)
          hfm0 rfl (fun hne => by simpa using hne)
          (by split <;> first | rfl | (exfalso; simp_all)) rfl hpar' rfl hargs hexcl
          hargOk hsusp hglobC hchainS
          (RefsFrom.enter rfl rfl fun v hv => List.mem_of_mem_take (List.mem_reverse.1 hv))
          (RefsFrom.next (new := []) (n := params.length) (by simp) rfl (by simp)))


include hwf hs in
/-- **A call** of handle `h` at type arguments `τ`, `Call` or `CallGeneric`,
keeps the reference invariant, into the callee or, for a native, back in the
caller, given the analysis runs it as `Call(h)` and the facts of the call the
certificates establish. -/
theorem refRuns_callWith (hapi : P.ApiSigs) (hc : Certified P (modelTypes P) (modelAvail P))
    {N : NativeRel} (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {instr : Bytecode}
    {h : FunctionHandleIndex} {τ : List RuntimeType} {c0 : FunctionRef} {cinfo : FunctionInfo}
    {fh : FunctionHandle} {params returns : List RuntimeType}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hi : code.code[fr.pc]? = some instr) (hcallI : CallsHandle info.module instr h)
    (hret : instr ≠ .ret) (hctl : instr.control = .fallThrough)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view (.call h)).run A = .ok ((), A'))
    (hattr : ReferenceSafety.borrowFieldMutable fh.attributes = none)
    (hres : P.resolve fr.function.module h = some c0)
    (hvis : P.visibleFrom info.moduleId c0 = true) (hcf : P.function? c0 = some cinfo)
    (hfh : info.module.functionHandles[h]? = some fh)
    (hlen : fh.typeParameters.length = τ.length)
    (hclen : cinfo.handle.typeParameters.length = τ.length)
    (hwfa : RuntimeType.argumentsWellFormed P τ cinfo.handle.typeParameters = true)
    (hpar' : P.paramTypes { c0 with typeArgs := τ } = some params)
    (hret' : P.returnTypes { c0 with typeArgs := τ } = some returns)
    (hparams : P.handleParamTypesAt fr.function.module h τ = some params)
    (hreturns : P.handleReturnTypesAt fr.function.module h τ = some returns)
    (hpre : RuntimeType.assignableAll (tys.take params.length) params.reverse = true)
    (hnm : P.nativeShape? c0 = none) :
    ExecRuns P N (RefInvariant P) (Step.call P s fr rest info h τ) := by
  have hcf' : P.function? { c0 with typeArgs := τ } = some cinfo := by
    rw [function?_typeArgs]; exact hcf
  have htok : P.typeArgsOk { c0 with typeArgs := τ } = true := by
    simp only [Program.typeArgsOk, hcf']; exact hwfa
  have hτ : ∀ x ∈ τ, x.isValue = true := typeArgsOk_values htok
  have hstack := hctx.conf
  have hlen' := conformsAll_length hstack
  have htl := assignableAll_length hpre
  have hge : params.length ≤ fr.stack.length := by
    rw [hlen']; simp only [List.length_take, List.length_reverse] at htl; omega
  have hargs : conformsAll P (fr.stack.take params.length).reverse params = true := by
    have := conformsAll_reverse
      (conformsAll_assignable (conformsAll_take params.length hstack) hpre)
    rwa [List.reverse_reverse] at this
  obtain ⟨A', hx, hsucc⟩ := hctx.next hret
  have hx := hrun _ hx
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, hctl])
  have hmod := (function?_parts hact.fn).1
  simp only [Step.call, Step.need, hres, hvis, hcf, hfh, hlen, hclen, hwfa, hpar', hret',
    hparams, hreturns, Step.popN, Nat.not_lt.2 hge, hargs, bne_self_eq_false, Bool.or_false,
    Bool.not_true, Bool.false_eq_true, ↓reduceIte, bind, Except.bind, pure, Except.pure]
  cases hcode : cinfo.definition.code with
  | none =>
    have hnv := hc.nativeValues _ _ hcf' hcode htok hnm
    have hval : ((params ++ returns).all RuntimeType.isValue) = true := by
      rw [List.all_eq_true]
      intro t ht
      rcases List.mem_append.1 ht with ht | ht
      · exact hnv.1 _ hpar' t ht
      · exact hnv.2 _ hret' t ht
    simp only [Option.isNone_none, hval, Bool.not_true, Bool.and_false, Bool.false_eq_true,
      ↓reduceIte]
    -- A native takes and returns values: the analysis's call is a step on values.
    obtain ⟨fh', hfh', hvs⟩ := exec_call_values hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    obtain ⟨ps', rs, hps, hrs, hvs⟩ := hvs hattr
    have hpl : params.length = ps'.length := by
      have h1 : P.handleParamTypesAt fr.function.module h τ = some params := hparams
      simp only [Program.handleParamTypesAt, hmod, hfh, hps, bind,
      Option.bind_some] at h1
      exact ofSignature_length h1
    have hrsig : RuntimeType.ofSignatureWith info.module τ rs = some returns := by
      have h1 : P.handleReturnTypesAt fr.function.module h τ = some returns := hreturns
      simp only [Program.handleReturnTypesAt, hmod, hfh, hrs, bind,
      Option.bind_some] at h1
      exact h1
    have hnoref : ∀ v ∈ fr.stack.take params.length, ∀ r, v ≠ .ref r := fun v hv =>
      noRef_of_conformsAll hargs (hnv.1 _ hpar') v (List.mem_reverse.2 hv)
    have hrsv : ∀ t ∈ rs, refKind t = none := by
      intro t ht
      obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 ht
      obtain ⟨u, hu, hut⟩ := ofSigW_getElem hrsig hj
      cases hk : refKind t with
      | none => rfl
      | some b =>
        obtain ⟨v, rfl⟩ := ofSlotToken_ref hut hk
        have := hnv.2 _ hret' _ (List.mem_of_getElem? hu)
        simp [RuntimeType.isValue] at this
    obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
    have hvs' : ValueStep ps'.length rs.length A A' :=
      hvs (hpl ▸ hd.take_nonRef hctx.height hnoref) hrsv
    have hnoref' : ∀ v ∈ (fr.stack.take params.length).reverse, ∀ r, v ≠ .ref r :=
      fun v hv => hnoref v (List.mem_reverse.1 hv)
    simp only [nativeView_of_noRefs _ hnoref', exclusiveArguments_of_noRefs hnoref',
      Bool.not_true, Bool.false_eq_true, ↓reduceIte]
    intro r _
    cases r with
    | failed e => trivial
    | returned vals writes places =>
      rw [resumeNative_values rfl hret' hnm hnoref' (hnv.2 _ hret')]
      split
      · next hcv =>
        replace hcv : conformsAll P vals returns = true := by
          simp only [Bool.and_eq_true] at hcv; exact hcv.1.1
        exact refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage
          (es := []) rfl (storageKept_of_eq rfl) allowedBy_nil
          (hd.valueStep hvs' rfl rfl (new := vals.reverse) (j' := params.length) rfl hnoref)
          (fun x r hx hxr => (hrefs x r (by rw [hvs'.heldIds] at hx; exact hx) hxr).same hsf rfl
            rfl rfl rfl rfl rfl rfl) hA'' hle
          (RefsFrom.next (new := vals.reverse) (n := params.length) rfl rfl fun v hv =>
            .of_not_ref (noRef_of_conformsAll hcv (hnv.2 _ hret') v (List.mem_reverse.1 hv)))
      · trivial
  | some ccode =>
    simp only [Option.isNone_some, Bool.false_and, Bool.false_eq_true, ↓reduceIte]
    obtain ⟨ltys, hltys⟩ := hc.locals _ cinfo ccode hcf' hcode htok
    have hcid : fr.id < s.nextFrame := hr.below fr (by rw [hsf]; exact List.mem_cons_self)
    obtain ⟨S, hglob, hchainS⟩ := hctx.chain _ hsf
    obtain ⟨Sc, hexcl, hargOk, hsusp, hglobC⟩ := call_suspendAt hwf hctx hact.locals
      (typeArgsOk_values hact.typeArgs) hi hcallI hret hctl hrun hact.fn
      hact.body hfh hattr hres hcf hτ hparams hreturns hret' hge hcid hsf hr.active.counted hglob
    have hfm0 : funModule P fr.function = some info.moduleId := by simp [funModule, hact.fn]
    simp only [hltys]
    repeat' split
    all_goals first
      | trivial
      | (show RefInvariant P _
         exact refInv_call' hwf hs hapi hr hsf rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl hcf'
          hcode htok hfm0 rfl (fun hne => by simpa using hne)
          (by split <;> first | rfl | (exfalso; simp_all)) rfl hpar' rfl hargs
          hexcl hargOk hsusp hglobC hchainS
          (RefsFrom.enter rfl rfl fun v hv => List.mem_of_mem_take (List.mem_reverse.1 hv))
          (RefsFrom.next (new := []) (n := params.length) (by simp) rfl (by simp)))

/-- The captured types of a closure are values. -/
theorem closureType?_values {f : FunctionRef} {mask : BitVec 64}
    {ctys rem rts : List RuntimeType} (h : P.closureType? f mask = some (ctys, rem, rts)) :
    ∀ t ∈ ctys, t.isValue = true := by
  unfold Program.closureType? at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, code, hcode, h⟩ := h
  split at h
  · simp at h
  · obtain ⟨params, hps, h⟩ := Option.bind_eq_some_iff.1 h
    obtain ⟨results, hrs, h⟩ := Option.bind_eq_some_iff.1 h
    split at h
    · simp at h
    · split at h
      · simp at h
      · next hv =>
        simp only [pure, Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl, rfl⟩ := h
        simpa using hv

/-- **A frame entered** with arguments each from the operand stack of `g` or
no reference. -/
theorem RefsFrom.enter' {g g' : Frame} {args : List Value} {k : Nat}
    (hstk : g'.stack = []) (hloc : g'.locals = (args.map some ++ List.replicate k none).toArray)
    (hargs : ∀ v ∈ args, ValFrom g v) : RefsFrom g g' := by
  refine RefsFrom.of_vals fun v hv => ?_
  simp only [frameVals, hstk, hloc, List.nil_append, List.mem_filterMap, id,
    List.toList_toArray, List.mem_append, List.mem_map, List.mem_replicate] at hv
  obtain ⟨o, ho, ho'⟩ := hv
  rcases ho with ⟨w, hw, rfl⟩ | ⟨-, ho⟩
  · simp only [Option.some.injEq] at ho'
    rw [← ho']
    exact hargs w hw
  · subst ho; cases ho'

include hwf hs in
/-- **`CallClosure`** keeps the reference invariant into the closure's
target: the caller suspends at the call's core with the closure popped
(`closure_suspendAt`), the captured arguments, values, joining the provided
ones in the target's first frame, and the target's frame counts as an
activation. -/
theorem refRuns_callClosure (hapi : P.ApiSigs) (hc : Certified P (modelTypes P) (modelAvail P))
    {N : NativeRel} (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {sig : SignatureIndex}
    (hctx : TopCtx P s fr info code (.callClosure sig) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.callClosure sig)) :
    ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code (.callClosure sig)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hstack := hctx.conf
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨toks, htoks, hta⟩ := hta
  split at hta
  rotate_left
  · simp at hta
  next ps0 rs0 a =>
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨pts, hps, rts, hrs, hta⟩ := hta
  split at hta
  rotate_left
  · simp at hta
  next ps' rs' a' brest =>
  split at hta
  rotate_left
  · simp at hta
  next hcond =>
  simp only [Option.some.injEq] at hta
  subst hta
  simp only [Bool.and_eq_true, decide_eq_true_eq] at hcond
  obtain ⟨⟨⟨h1, h2⟩, hsub⟩, hpre⟩ := hcond
  subst ps' rs'
  obtain ⟨c, vs, hcs, hc1, hvs⟩ := conformsAll_cons_right hstack
  obtain ⟨target, mask, captured, ca, rfl, hsub', hw⟩ := conforms_function hc1
  have hw' := hw
  simp only [Value.wellFormed] at hw'
  split at hw'
  rotate_left
  · simp at hw'
  next ctys rem rts' hct =>
  simp only [Bool.and_eq_true, beq_iff_eq] at hw'
  obtain ⟨⟨⟨rfl, rfl⟩, -⟩, hccap⟩ := hw'
  obtain ⟨tinfo, tcode, params, htf, htcode, htok, hpar, hret, hsplit⟩ := closureType?_parts hct
  have hprov : conformsAll P vs (pts.reverse ++ brest.drop pts.length) = true := by
    have h1 := conformsAll_assignable (conformsAll_take pts.length hvs) hpre
    have h2 := conformsAll_drop pts.length hvs
    have := conformsAll_append h1 h2
    rwa [List.take_append_drop] at this
  obtain ⟨hpop, hargs, hrest⟩ := popN_conforms (fr := { fr with stack := vs }) hprov
  have hcomp := conformsAll_maskCompose (cs := captured) hsplit hccap hargs
  obtain ⟨ltys, hltys⟩ := hc.locals target tinfo tcode htf htcode htok
  have hn : pts.length ≤ vs.length := by
    have := conformsAll_length hargs
    simp only [List.length_reverse, List.length_take] at this
    omega
  have hcapNR : ∀ w ∈ captured, ∀ r, w ≠ .ref r :=
    noRef_of_conformsAll hccap (closureType?_values hct)
  have hsubl : (argRefs (maskCompose mask captured (vs.take pts.length).reverse)).Sublist
      (argRefs (vs.take pts.length).reverse) := by
    unfold maskCompose
    exact argRefs_maskCompose_go _ _ _ hcapNR
  have hcid : fr.id < s.nextFrame := hr.below fr (by rw [hsf]; exact List.mem_cons_self)
  obtain ⟨S, hglob, hchainS⟩ := hctx.chain _ hsf
  obtain ⟨Sc, hexcl, hargOk, hsusp, hglobC⟩ := closure_suspendAt hctx hact.locals
    (typeArgsOk_values hact.typeArgs) hi hact.fn hact.body hcs (fun r h => by cases h) htoks hps
    hrs hret hn hcid hsf hr.active.counted hglob hsubl
  have hfm0 : funModule P fr.function = some info.moduleId := by simp [funModule, hact.fn]
  dsimp only [Step.exec]
  simp only [Step.callClosure, Step.need, htoks, hps, hrs, Step.pop, hcs, bind, Except.bind, pure,
    Except.pure]
  simp only [bne_self_eq_false, Bool.or_false, ability_subset_trans hsub hsub', Bool.not_true,
    Bool.false_eq_true, ↓reduceIte, hpop, hargs, htf, htcode, hpar, hret, hcomp, hltys]
  repeat' split
  all_goals first
    | trivial
    | (show RefInvariant P _
       exact refInv_call' hwf hs hapi hr hsf rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl htf
        htcode htok hfm0 rfl (fun _ => rfl) rfl rfl hpar rfl hcomp (hexcl.sublist hsubl)
        (fun r hr' => hargOk r (hsubl.subset hr')) hsusp hglobC hchainS
        (RefsFrom.enter' rfl rfl fun v hv => by
          rcases maskCompose_mem hv with hv | hv
          · exact .of_not_ref (hcapNR v hv)
          · refine .of_mem (mem_frameVals_stack ?_)
            rw [hcs]
            exact List.mem_cons_of_mem _ (List.mem_of_mem_take (List.mem_reverse.1 hv)))
        (RefsFrom.next (new := []) (n := pts.length + 1) (by simp [hcs]) rfl (by simp)))

include hwf hs hm3 in
/-- **`Call`** of an M3 program keeps the reference invariant. -/
theorem refRuns_call (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {h : FunctionHandleIndex}
    (hctx : TopCtx P s fr info code (.call h) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.call h))
    (hnm : NoModeledCall P fr.function.module info.module (.call h)) :
    ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code (.call h)) :=
  refRuns_call' hwf hs hm3.apiSigs hc hr hsf hact hctx hi
    (fun fh hfh => (function_m3 hm3 hact.fn) fh (getElem?_mem_toList hfh)) hnm
end Steps

/-- **`Ret`** keeps the reference invariant: the caller resumes. -/
theorem refRuns_ret {N : NativeRel} (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hctx : TopCtx P s fr info code .ret view A stys tys ps)
    (hfn : P.function? fr.function = some info) :
    ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code .ret) := by
  cases hrt : P.returnTypes fr.function with
  | none =>
    simp only [Step.exec, Step.ret, hrt, Step.need, bind, Except.bind]
    trivial
  | some rets =>
    simp only [Step.exec, Step.ret, hrt, Step.need, bind, Except.bind, pure, Except.pure]
    by_cases h1 : fr.stack.length < rets.length
    · simp only [h1, ↓reduceIte]; trivial
    by_cases h2 : fr.stack.length > rets.length
    · simp only [h1, h2, ↓reduceIte]; trivial
    by_cases h3 : conformsAll P fr.stack.reverse rets = true
    case neg =>
      simp only [h1, h2, ↓reduceIte, Bool.eq_false_iff.2 h3, Bool.not_false]; trivial
    simp only [h1, h2, h3, ↓reduceIte, Bool.not_true, Bool.false_eq_true]
    split
    · trivial
    cases rest with
    | nil => trivial
    | cons caller rest' =>
      cases hca : fr.countedActive <;> cases hle : fr.lockEntered <;>
        simp only [Bool.false_eq_true, if_false, if_true] <;>
        exact refInv_return hr hctx hsf rfl rfl rfl rfl rfl hrt h3 (id := info.moduleId)
          (by simp [funModule, hfn]) (by rw [hca]; rfl)

end MoveBytecode.Proofs
