-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefUpdate

/-!
# The writes through references, safe

UNTRUSTED (checked by the kernel). `WriteRef`, `VecPushBack`, `VecPopBack` and
`VecSwap` change in place the value a tracked mutable reference designates, a
value inside a local of the current frame. The analysis allows it only through
a writable reference, so nothing tracked is strictly inside the location
written, the only one the write invalidates; every tracked reference stays
valid, and the local still conforms to its declared type.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference}

/-! ## Writing at a path -/

theorem withChild_some {v c c' : Value} {st : PathStep} (h : v.child? st = some c)
    (hst : st.writable = true) : ∃ v', v.withChild? st c' = some v' := by
  cases v <;> cases st <;> simp only [Value.child?] at h <;> try (simp at h; done)
  all_goals try (simp [PathStep.writable] at hst; done)
  case struct.field sid fs i =>
    exact ⟨.struct sid (fs.set i c'), by simp [Value.withChild?, (List.getElem?_eq_some_iff.1 h).1]⟩
  case variant.variantField sid tag fs t i =>
    split at h
    · next ht =>
      exact ⟨.variant sid tag (fs.set i c'),
        by simp [Value.withChild?, ht, (List.getElem?_eq_some_iff.1 h).1]⟩
    · simp at h
  case vector.element e es i =>
    exact ⟨.vector e (es.set i c'), by simp [Value.withChild?, (List.getElem?_eq_some_iff.1 h).1]⟩
  case table.field sid fs ks es i =>
    exact ⟨.table sid (fs.set i c') ks es,
      by simp [Value.withChild?, (List.getElem?_eq_some_iff.1 h).1]⟩
  case table.tableEntry sid fs ks es i =>
    exact ⟨.table sid fs ks (es.set i c'),
      by simp [Value.withChild?, (List.getElem?_eq_some_iff.1 h).1]⟩

/-- A writable path that resolves can be written. -/
theorem set_some : ∀ (path : List PathStep) {v u : Value} (w : Value), v.get? path = some u →
    path.all PathStep.writable = true → ∃ v', v.set? path w = some v'
  | [], v, u, w, _, _ => ⟨w, rfl⟩
  | st :: path, v, u, w, h, hwr => by
    simp only [List.all_cons, Bool.and_eq_true] at hwr
    obtain ⟨c, hc, hcu⟩ := get_cons h
    obtain ⟨c', hc'⟩ := set_some path w hcu hwr.2
    obtain ⟨v', hv'⟩ := withChild_some (c' := c') hc hwr.1
    exact ⟨v', by simp [Value.set?, hc, hc', hv']⟩

/-! ## Writing in place -/

/-- The value at a tracked reference's root, and at its location. -/
theorem RefOk.root_value {r : Reference} {frx : Frame} (h : RefOk P s frx r) :
    ∃ w u, s.root? r.location.root = some w ∧ w.get? r.location.path = some u ∧
      s.deref? r.location r.since = some u := by
  obtain ⟨u, hu⟩ := h.deref_some
  have hread := deref?_read hu
  simp only [State.read?, bind, Option.bind_eq_some_iff] at hread
  obtain ⟨w, hw, hwu⟩ := hread
  exact ⟨w, u, hw, hwu, hu⟩

theorem root?_withStack (hsf : s.frames = fr :: rest) (st : List Value) (ρ : Root) :
    ({ s with frames := { fr with stack := st } :: rest } : State).root? ρ = s.root? ρ :=
  root?_replaceTop (s₁ := s) (s₂ := { s with frames := { fr with stack := st } :: rest }) hsf rfl
    rfl rfl rfl rfl ρ

theorem frameVals_updFrame {l : Location} {w' : Value} {g : Frame}
    (hw' : ∀ i, l.root = .local_ g.id i → ∀ r, w' ≠ .ref r) {r : Reference}
    (h : Value.ref r ∈ frameVals (updFrame l w' g)) : Value.ref r ∈ frameVals g := by
  rw [mem_frameVals] at h ⊢
  rcases h with h | ⟨i, h⟩
  · left; rwa [(updFrame_eq l w' g).2.2.2] at h
  · right; exact ⟨i, updFrame_ref hw' h⟩

/-- **Closing a write in place** through tracked writable reference `r` of the
current frame: the root of `r` now holds `w'`, the frame pushes `vals`; the
invariant holds after it. -/
theorem m2_writeClose (hc : Certified P (modelTypes P) (modelAvail P))
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest)
    {instr : Bytecode} (hi : code.code[fr.pc]? = some instr) (hctrl : instr.control = .fallThrough)
    (hav : ∀ a, availabilityAfter instr a = a)
    {before after : List RuntimeType} (hbefore : modelTypes P fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module instr before = some after)
    {ps : List Reference} {L : Nat → Option Reference}
    (hd : Desc A fr (gsidOf P (s.frames.map Frame.skel)) ps L)
    (hrefs : ∀ x r, x ∈ heldIds A → L x = some r → RefOk P s fr r)
    (hchainR : Below P s fr ps rest) (_hroots : RootsOk P s)
    {p : Nat} {r : Reference} (hp : p ∈ heldIds A) (hpL : L p = some r)
    (hpw : A.graph.isWritable p = .ok true) (hpm : r.mutable = true)
    {A' : RefState} {fr0 : Frame} (hd0 : Desc A' fr0 (gsidOf P (s.frames.map Frame.skel)) ps L)
    (hheld : ∀ x ∈ heldIds A', x ∈ heldIds A)
    {A'' : RefState} (hA'' : modelRef P fr.function (fr.pc + 1) = some A'') (hle : RefLe A' A'')
    {w w' : Value} {st vals : List Value} (hid0 : fr0.id = fr.id) (hloc0 : fr0.locals = fr.locals)
    (hstk0 : fr0.stack = vals.reverse ++ st)
    (hstk : conformsAll P (vals.reverse ++ st) after = true)
    (hwf : WriteFacts P s (writeState s { fr with stack := st } rest r.location w' vals) r.location
      w w')
    (hsuf : ∃ pre, fr.stack = pre ++ st) (hvals : ∀ x ∈ vals, ∀ q, x ≠ .ref q) :
    InvRefs P (writeState s { fr with stack := st } rest r.location w' vals) := by
  have hnd := hr.ids
  have hfrm : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self
  have hval : ∀ g ∈ s.frames, ∀ i, r.location.root = .local_ g.id i →
      g.locals[i]? = some (some w) ∧ ∀ r', w ≠ .ref r' := fun g hg => hwf.slot g hg
  have hslot : ∀ g ∈ s.frames, ∀ i, r.location.root = .local_ g.id i →
      ∃ w₀, g.locals[i]? = some (some w₀) := fun g hg i hi => ⟨w, (hval g hg i hi).1⟩
  obtain ⟨hid1, hfun1, hpc1, hstk1⟩ := updFrame_eq r.location w' { fr with stack := st }
  have hloc1 : (updFrame r.location w' { fr with stack := st }).locals =
      (updFrame r.location w' fr).locals := updFrame_locals_eq rfl rfl
  have hlocalsTop := hwf.locals fr hfrm
  have hupd := hact.upd (l := r.location) (w' := w') (hslot fr hfrm) hlocalsTop
  obtain ⟨-, hfunU, hpcU, -⟩ := updFrame_eq r.location w' fr
  have hsk : (writeState s { fr with stack := st } rest r.location w' vals).frames.map Frame.skel =
      s.frames.map Frame.skel := by
    rw [hsf]
    show (Step.advance (Step.pushAll (updFrame r.location w' { fr with stack := st }) vals)).skel ::
        (rest.map (updFrame r.location w')).map Frame.skel = fr.skel :: rest.map Frame.skel
    congr 1
    · exact updFrame_skel r.location w' { fr with stack := st }
    · exact map_updFrame_skel r.location w' rest
  have hgs : s.frames.map Frame.skel = (fr.function, fr.countedActive) :: rest.map Frame.skel := by
    rw [hsf]; rfl
  refine ⟨⟨_, rest.map (updFrame r.location w'), rfl, ⟨info, code, ?_⟩, ?_⟩, ?_⟩
  · refine activeAt_succ hc hact hi hbefore hafter
      (by simp only [Step.advance, Step.pushAll]; rw [hfun1])
      (by simp only [Step.advance, Step.pushAll]; rw [hpc1]; simp [successors, hi, hctrl])
      (by simp only [Step.advance, Step.pushAll]; rw [hstk1]; exact hstk) ?_ (fun i hi' => ?_)
      (fun i hi' => ?_)
    · obtain ⟨tys, h1, h2, h3⟩ := hlocalsTop
      refine ⟨tys, ?_, ?_, ?_⟩
      · simp only [Step.advance, Step.pushAll]; rw [hfun1]; rw [hfunU] at h1; exact h1
      · simp only [Step.advance, Step.pushAll]; rw [hloc1]; exact h2
      · simp only [Step.advance, Step.pushAll]; rw [hloc1]; exact h3
    · rw [hav] at hi'
      have := hupd.available i (by rw [hfunU, hpcU]; exact hi')
      simp only [Step.advance, Step.pushAll]; rw [hloc1]; exact this
    · rw [hav] at hi'
      have := hupd.unavailable i (by rw [hfunU, hpcU]; exact hi')
      simp only [Step.advance, Step.pushAll]; rw [hloc1]; exact this
  · have := ChainOk.upd (l := r.location) (w' := w') hchain (fun g hg => ⟨hslot g (by
      rw [hsf]; exact List.mem_cons_of_mem _ hg), hwf.locals g
        (by rw [hsf]; exact List.mem_cons_of_mem _ hg)⟩)
    simp only [Step.advance, Step.pushAll]
    rw [hfun1]
    exact this
  · have hids : ((writeState s { fr with stack := st } rest r.location w' vals).frames.map
        (·.id)) = s.frames.map (·.id) := by
      simp only [writeState, hsf, List.map_cons, List.map_map, Step.advance, Step.pushAll, hid1]
      congr 1
      exact List.map_congr_left fun g _ => (updFrame_eq r.location w' g).1
    obtain ⟨S, hglob, hch⟩ := hchainR
    refine ⟨by rw [hids]; exact hnd, fun g hg => ?_, hwf.storage, ⟨_,
      rest.map (updFrame r.location w'), ps, rfl, ⟨A'', ?_, ?_⟩, S, ?_, ?_⟩, ?_, ?_⟩
    · have hgid : g.id ∈ s.frames.map (·.id) := by
        rw [← hids]; exact List.mem_map_of_mem hg
      obtain ⟨g0, hg0, he⟩ := List.mem_map.1 hgid
      rw [← he]
      exact hr.below g0 hg0
    · simp only [Step.advance, Step.pushAll]
      rw [hfun1, hpc1]
      exact hA''
    · refine FrameOk.le ⟨L, ?_, fun x rx hx hxL => ?_⟩ hle
      · rw [hsk]
        refine hd0.upd (l := r.location) (w' := w') (fun i hi' => ?_)
          (by simp only [Step.advance, Step.pushAll]; rw [hid1, hid0])
          (by simp only [Step.advance, Step.pushAll]; rw [hstk1, hstk0])
          (by simp only [Step.advance, Step.pushAll]; exact updFrame_locals_eq (by rw [hid0]) (by
            rw [hloc0]))
        rw [hid0] at hi'
        obtain ⟨hs, hwr⟩ := hval fr hfrm i hi'
        exact ⟨w, by rw [hloc0]; exact hs, hwr⟩
      · have hxA := hheld x hx
        exact hwf.refOk fr hfrm rx (hd.mem_vals hxA hxL) _ _ (hrefs x rx hxA hxL)
          (by simp only [Step.advance, Step.pushAll]; rw [hid1])
          (hd.complete.writable_inside hp hpL hpw hxA hxL)
    · refine hglob.skel (by simp only [Step.advance, Step.pushAll]; rw [hfun1]) ?_
      have := hsk
      rw [hsf] at this
      exact this
    · have hdc := hd.complete
      rw [hgs] at hdc
      have := ChainRef.afterWrite hwf rest fr.id
        fr.function ps S (fun g hg => by rw [hsf]; exact List.mem_cons_of_mem _ hg)
        (allowed_write hdc hglob hp hpL hpm) hch
      simp only [Step.advance, Step.pushAll]
      rw [hid1, hfun1]
      exact this
    · exact hr.active.skel hsk rfl
    · refine hr.heldTargets.of fun g' hg' rx hm k hk => ?_
      simp only [writeState, List.mem_cons, List.mem_map] at hg'
      rcases hg' with rfl | ⟨g, hg, rfl⟩
      · rw [mem_frameVals] at hm
        simp only [Step.advance, Step.pushAll, List.mem_append, List.mem_reverse] at hm
        rcases hm with (hm | hm) | ⟨i, hi⟩
        · exact absurd rfl (hvals _ hm rx)
        · rw [(updFrame_eq _ _ _).2.2.2] at hm
          obtain ⟨pre, hpre⟩ := hsuf
          exact ⟨fr, hfrm, rx, mem_frameVals_stack (by rw [hpre]; exact List.mem_append_right _ hm),
            rfl, rfl⟩
        · have := updFrame_ref (g := { fr with stack := st }) (fun i hi => hwf.value fr hfrm i hi) hi
          exact ⟨fr, hfrm, rx, mem_frameVals_locals this, rfl, rfl⟩
      · have hgm : g ∈ s.frames := by rw [hsf]; exact List.mem_cons_of_mem _ hg
        exact ⟨g, hgm, rx, frameVals_updFrame (hwf.value g hgm) hm, rfl, rfl⟩

/-- **A write through a tracked reference to a value of a rigid type**: the
type along its location is its target. -/
theorem WriteFacts.ofRigid {s₂ : State} {r : Reference} {frx : Frame} {v w w' : Value}
    (hw : AfterWrite P s s₂ r.location w') (hok : RefOk P s frx r) (hrig : rigid r.target = true)
    (hnd : (s.frames.map (·.id)).Nodup) (hroots : RootsOk P s)
    (hstor : ∀ r ∈ s.storage, r.value.conforms P (.struct r.type) = true)
    (hroot : s.root? r.location.root = some w) (hset : w.set? r.location.path v = some w')
    (hv : v.conforms P r.target = true) : WriteFacts P s s₂ r.location w w' := by
  obtain ⟨⟨T, T', hT, hpath, hfit⟩, -, htv, -, -⟩ := hok
  have hT' : T' = r.target := by
    rcases hfit with h | ⟨-, h⟩
    · exact h
    · exact assignable_rigid h hrig
  subst hT'
  exact WriteFacts.exact hw hnd hroots hstor hroot hset hT hpath hv htv

theorem heldIds_sub {A' : RefState} (hloc : A'.locals = A.locals)
    (hstk : ∀ x, AbsValue.reference x ∈ A'.stack → AbsValue.reference x ∈ A.stack) :
    ∀ x ∈ heldIds A', x ∈ heldIds A := by
  intro x hx
  rcases mem_heldIds.1 hx with hx | hx
  · exact mem_heldIds.2 (Or.inl (hloc ▸ hx))
  · exact mem_heldIds.2 (Or.inr (hstk x hx))

/-- The current frame's other frames: distinct identities. -/
theorem ids_withStack (hr : RefInvariant P s) (hsf : s.frames = fr :: rest) (st : List Value) :
    ((({ s with frames := { fr with stack := st } :: rest } : State)).frames.map (·.id)).Nodup := by
  have := hr.ids; rw [hsf] at this; simpa using this

/-- **`WriteRef`.** -/
theorem refSafe_writeRef (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest)
    (hctx : TopCtx P s fr info code .writeRef view A stys tys ps)
    (hi : code.code[fr.pc]? = some .writeRef) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code .writeRef) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc .writeRef hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hafter' := hafter
  dsimp only [typeAfter] at hafter'
  split at hafter'
  · next t₀ t brest =>
    split at hafter'
    · next hcond =>
      simp only [Bool.and_eq_true] at hcond
      obtain ⟨hasg, hdrop⟩ := hcond
      simp only [Option.some.injEq] at hafter'
      subst hafter'
      obtain ⟨rv, st1, hst1, hrv, h1⟩ := conformsAll_cons_right hctx.conf
      obtain ⟨r, rfl, hrm, hrt⟩ := conforms_ref hrv (by simp [rigid])
      obtain ⟨v, st, rfl, hv₀, hstc⟩ := conformsAll_cons_right h1
      have hv : v.conforms P r.target = true := by rw [hrt]; exact conforms_trans hv₀ hasg
      obtain ⟨A', hx, hsucc⟩ := hctx.next (by decide)
      obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
      obtain ⟨id, ra, g, hsa, hwr, hrel, rfl⟩ := exec_writeRef hx
      have hlen : ra.length = st.length := by
        have := hctx.height; rw [hsa, hst1] at this; simpa using this
      obtain ⟨r', hr', hL⟩ := hd.top hsa hst1 (by simp [hlen])
      simp only [Value.ref.injEq] at hr'
      subst hr'
      have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
      have hok₁ : RefOk P { s with frames := { fr with stack := st } :: rest }
          { fr with stack := st } r := (hrefs id r hp hL).same hsf rfl rfl rfl rfl rfl rfl rfl
      obtain ⟨w, old, hroot, hwo, hold⟩ := hok₁.root_value
      obtain ⟨w', hset⟩ := set_some _ v hwo (hok₁.writable hrm)
      have holdc : old.conforms P t₀ = true := by
        rw [← hrt]
        exact hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hold
      have holdd : (old.abilities P).drop = true :=
        AbilitySet.drop_of_subset (abilities_of_conforms hc.abilities holdc) hdrop
      have hupd := update_eq hroot hset
      have hvt₀ : v.conforms P t₀ = true := conforms_trans hv₀ hasg
      have hstep : Step.exec P s fr rest info code .writeRef = .ok (.done (.running
          (writeState s { fr with stack := st } rest r.location w' []))) := by
        simp only at hold hupd
        simp only [Step.exec, Step.popRef, Step.pop, hst1, bind, Except.bind, pure, Except.pure,
          hrm, hrt, hvt₀, Bool.not_true, Bool.false_eq_true, if_false, Step.deref, Step.need, hold,
          holdd, hupd, Step.finish, List.map_cons, writeState]
      rw [hstep]
      obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
      have hd₁ := hd.popRelease hctx.typed hsa hst1 (by simp [hlen]) hrel rfl
        (fr' := { fr with stack := v :: st }) rfl rfl rfl
      have hd₀ := hd₁.pop (A' := { A with stack := ra, graph := g })
        (fr' := { fr with stack := st }) rfl rfl hlen rfl rfl rfl rfl rfl
      have hroot₀ : s.root? r.location.root = some w := (root?_withStack hsf st _).symm.trans hroot
      have hfrm : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self
      have hw := afterWrite_writeState (P := P) hsf st r.location w' []
      have hwf : WriteFacts P s (writeState s { fr with stack := st } rest r.location w' [])
          r.location w w' := by
        obtain ⟨⟨T, T', hT, hpath, hfit⟩, -, htv, -, -⟩ := hrefs id r hp hL
        by_cases hex : T' = r.target
        · subst hex
          exact WriteFacts.exact hw hr.ids hctx.roots hr.storage hroot₀ hset hT hpath hv htv
        rcases hfit with hfit | ⟨⟨hpe, hnl⟩, hasg'⟩
        · exact absurd hfit hex
        have hTT : T' = T := by rw [hpe] at hpath; simp only [pathType] at hpath; exact
          (Option.some.inj hpath).symm
        subst hTT
        have hrig : rigid T' = false := by
          cases h : rigid T' with
          | false => rfl
          | true => exact absurd (assignable_rigid_left hasg' h) hex
        cases hρ : r.location.root with
        | local_ f i => exact absurd hρ (hnl f i)
        | global a t' =>
          exfalso
          rw [hρ] at hT hroot₀
          simp only [State.root?] at hroot₀
          simp only [RootType, hroot₀, Option.bind_some] at hT
          split at hT
          · simp only [Option.some.injEq] at hT
            unfold GlobalStorage.get? at hroot₀
            rw [Option.map_eq_some_iff] at hroot₀
            obtain ⟨r1, hr1, hw1⟩ := hroot₀
            have hr1m := List.find?_some hr1
            simp only [Bool.and_eq_true, beq_iff_eq] at hr1m
            have hc1 := hr.storage r1 (List.mem_of_find?_eq_some hr1)
            rw [hw1, hr1m.2] at hc1
            rw [← hT, conforms_type hc1 rfl] at hrig
            simp [rigid] at hrig
          · cases hT
        | held k =>
          have hl : r.location = ⟨.held k, []⟩ := by
            rcases hl : r.location with ⟨ρ, q⟩
            rw [hl] at hρ hpe
            simp only at hρ hpe
            rw [hρ, hpe]
          rw [hl] at hset hT hroot₀ ⊢
          have hw'v : w' = v := (Option.some.inj hset).symm
          subst hw'v
          obtain ⟨τ, hτ⟩ := hr.heldTargets
          have hrmem : Value.ref r ∈ frameVals fr :=
            mem_frameVals_stack (by rw [hst1]; exact List.mem_cons_self)
          have hrτ := hτ fr hfrm r hrmem k hl
          rw [hl] at hw
          exact WriteFacts.held hw hctx.roots hr.storage hroot₀ hT hrig hv
            (fun g hg rx hm hx => (hτ g hg rx hm k hx).trans hrτ.symm)
      exact m2_writeClose hc hr hsf hact hchain hi rfl (fun _ => rfl) hbefore hafter hd hrefs
        (hctx.chain _ hsf) hctx.roots hp hL hwr hrm hd₀ (heldIds_sub rfl (fun x hx => by
          rw [hsa]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hx))) hA'' hle rfl rfl
        (by simp)
        (by simpa [Step.advance, Step.pushAll] using hstc) hwf ⟨[.ref r, v], by simp [hst1]⟩
        (by simp)
    · simp at hafter'
  · simp at hafter'

theorem typeAfter_vecPushBack {f : FunctionRef} {m : CompiledModule} {sig : Nat}
    {before after : List RuntimeType} (h : typeAfter P f m (.vecPushBack sig) before = some after) :
    ∃ t t' brest, m.vectorElemTypeWith f.typeArgs sig = some t ∧
      before = t' :: .ref true (.vector t) :: brest ∧ t'.assignable t = true ∧ after = brest := by
  have h' : (do
      let t ← m.vectorElemTypeWith f.typeArgs sig
      match before with
      | t' :: .ref true (.vector t'') :: rest => if t'.assignable t && t'' = t then some rest else none
      | _ => none) = some after := h
  clear h
  cases het : m.vectorElemTypeWith f.typeArgs sig with
  | none => simp [het] at h'
  | some t =>
    refine ⟨t, ?_⟩
    simp only [het, bind, Option.bind_some] at h'
    split at h'
    · next t' t'' brest =>
      split at h'
      · next hc' =>
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hc'
        obtain ⟨hasg, rfl⟩ := hc'
        exact ⟨t', brest, rfl, rfl, hasg, (Option.some.inj h').symm⟩
      · simp at h'
    · simp at h'

theorem typeAfter_vecPopBack {f : FunctionRef} {m : CompiledModule} {sig : Nat}
    {before after : List RuntimeType} (h : typeAfter P f m (.vecPopBack sig) before = some after) :
    ∃ t brest, m.vectorElemTypeWith f.typeArgs sig = some t ∧
      before = .ref true (.vector t) :: brest ∧ after = t :: brest := by
  have h' : (do
      let t ← m.vectorElemTypeWith f.typeArgs sig
      match before with
      | .ref true (.vector t') :: rest => if t' = t then some (t :: rest) else none
      | _ => none) = some after := h
  clear h
  cases het : m.vectorElemTypeWith f.typeArgs sig with
  | none => simp [het] at h'
  | some t =>
    refine ⟨t, ?_⟩
    simp only [het, bind, Option.bind_some] at h'
    split at h'
    · next t' brest =>
      split at h'
      · next hc' =>
        subst hc'
        exact ⟨brest, rfl, rfl, (Option.some.inj h').symm⟩
      · simp at h'
    · simp at h'

theorem typeAfter_vecSwap {f : FunctionRef} {m : CompiledModule} {sig : Nat}
    {before after : List RuntimeType} (h : typeAfter P f m (.vecSwap sig) before = some after) :
    ∃ t brest, m.vectorElemTypeWith f.typeArgs sig = some t ∧
      before = .int .u64 :: .int .u64 :: .ref true (.vector t) :: brest ∧ after = brest := by
  have h' : (do
      let t ← m.vectorElemTypeWith f.typeArgs sig
      match before with
      | .int .u64 :: .int .u64 :: .ref true (.vector t') :: rest =>
        if t' = t then some rest else none
      | _ => none) = some after := h
  clear h
  cases het : m.vectorElemTypeWith f.typeArgs sig with
  | none => simp [het] at h'
  | some t =>
    refine ⟨t, ?_⟩
    simp only [het, bind, Option.bind_some] at h'
    split at h'
    · next t' brest =>
      split at h'
      · next hc' =>
        subst hc'
        exact ⟨brest, rfl, rfl, (Option.some.inj h').symm⟩
      · simp at h'
    · simp at h'

theorem conforms_vector_mk {t : RuntimeType} {elems : List Value}
    (h : ∀ e ∈ elems, e.conforms P t = true) : (Value.vector t elems).conforms P (.vector t) = true :=
  conforms_iff.2 ⟨assignable_refl _, wellFormed_vector.2 h⟩

/-- Popping a mutable reference to a vector of `t`. -/
theorem popRefTo_vec {r : Reference} {st : List Value} {t : RuntimeType}
    (hst : fr.stack = .ref r :: st) (hrm : r.mutable = true) (hrt : r.target = .vector t) :
    Step.popRefTo fr (.vector t) true = .ok (r, { fr with stack := st }) := by
  simp [Step.popRefTo, Step.popRef, Step.pop, hst, bind, Except.bind, pure, Except.pure, hrt, hrm]

/-- **`VecPushBack`.** -/
theorem refSafe_vecPushBack (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {sig : Nat}
    (hctx : TopCtx P s fr info code (.vecPushBack sig) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.vecPushBack sig)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.vecPushBack sig)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨t, t', brest, het, rfl, hasg, rfl⟩ := typeAfter_vecPushBack hafter
  obtain ⟨v, st1, hst1, hv₀, h1⟩ := conformsAll_cons_right hctx.conf
  have hv : v.conforms P t = true := conforms_trans hv₀ hasg
  obtain ⟨rv, st, hst2, hrv, hstc⟩ := conformsAll_cons_right h1
  subst hst2
  obtain ⟨r, rfl, hrm, hrt⟩ := conforms_ref hrv (by simp [rigid])
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨id, ra, g, hsa, hwr, hrel, rfl⟩ := exec_vecPushBack hx
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst1] at this; simpa using this
  obtain ⟨t0, stys0, hstys⟩ : ∃ t0 stys0, stys = t0 :: stys0 := by
    have := hctx.typed.nstack
    rw [hsa] at this
    cases stys with
    | nil => simp at this
    | cons t0 stys0 => exact ⟨t0, stys0, rfl⟩
  have ht1 := hctx.typed.popValue hsa hstys
  have hd1 := hd.pop (A' := { A with stack := .reference id :: ra })
    (fr' := { fr with stack := .ref r :: st }) hsa hst1 (by simp [hlen]) rfl rfl rfl rfl rfl
  obtain ⟨r', hr', hL⟩ := hd1.top rfl rfl hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by
    rw [hsa]; exact List.mem_cons_of_mem _ List.mem_cons_self))
  have hok₁ : RefOk P { s with frames := { fr with stack := st } :: rest }
      { fr with stack := st } r := (hrefs id r hp hL).same hsf rfl rfl rfl rfl rfl rfl rfl
  obtain ⟨w, old, hroot, hwo, hold⟩ := hok₁.root_value
  have holdc : old.conforms P (.vector t) = true := by
    rw [← hrt]
    exact hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hold
  obtain ⟨elems, rfl, helems⟩ := conforms_vector holdc
  have hnv : (Value.vector t (elems ++ [v])).conforms P (.vector t) = true := by
    refine conforms_vector_mk fun e he => ?_
    rcases List.mem_append.1 he with he | he
    · exact helems e he
    · rw [List.mem_singleton.1 he]; exact hv
  obtain ⟨w', hset⟩ := set_some _ (.vector t (elems ++ [v])) hwo (hok₁.writable hrm)
  have hupd := update_eq hroot hset
  have hstep : Step.exec P s fr rest info code (.vecPushBack sig) = .ok (.done (.running
          (writeState s { fr with stack := st } rest r.location w' []))) := by
    simp only at hold hupd
    simp only [Step.exec, elemType_ok het, Step.pop, hst1, bind, Except.bind, pure,
      Except.pure, popRefTo_vec (fr := { fr with stack := .ref r :: st }) rfl hrm hrt, hv, Bool.not_true, Bool.false_eq_true, if_false,
      Step.derefVector, Step.deref, Step.need, hold, hupd, Step.finish, List.map_cons, writeState]
  rw [hstep]
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
  have hd₀ := hd1.popRelease ht1 rfl rfl hlen hrel (A' := { A with stack := ra, graph := g })
    (fr' := { fr with stack := st }) rfl rfl rfl rfl
  exact m2_writeClose hc hr hsf hact hchain hi rfl (fun _ => rfl) hbefore hafter hd hrefs
    (hctx.chain _ hsf) hctx.roots hp hL hwr hrm hd₀ (heldIds_sub rfl (fun x hx => by
      rw [hsa]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hx))) hA'' hle rfl rfl (by simp)
    (by simpa [Step.advance, Step.pushAll] using hstc)
    (WriteFacts.ofRigid (afterWrite_writeState hsf st r.location w' []) (hrefs id r hp hL)
      (by rw [hrt]; rfl) hr.ids hctx.roots hr.storage ((root?_withStack hsf st _).symm.trans hroot)
      hset (by rw [hrt]; exact hnv)) ⟨[v, .ref r], by simp [hst1]⟩ (by simp)

/-- **`VecPopBack`.** -/
theorem refSafe_vecPopBack (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {sig : Nat}
    (hctx : TopCtx P s fr info code (.vecPopBack sig) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.vecPopBack sig)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.vecPopBack sig)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨t, brest, het, rfl, rfl⟩ := typeAfter_vecPopBack hafter
  obtain ⟨rv, st, hst1, hrv, hstc⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨r, rfl, hrm, hrt⟩ := conforms_ref hrv (by simp [rigid])
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨id, ra, g, hsa, hwr, hrel, rfl⟩ :=
    exec_vecPopBack (vectorElemTypeWith_head (by rw [het]; rfl)) hx
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst1] at this; simpa using this
  obtain ⟨r', hr', hL⟩ := hd.top hsa hst1 hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
  have hok₁ : RefOk P { s with frames := { fr with stack := st } :: rest }
      { fr with stack := st } r := (hrefs id r hp hL).same hsf rfl rfl rfl rfl rfl rfl rfl
  obtain ⟨w, old, hroot, hwo, hold⟩ := hok₁.root_value
  have holdc : old.conforms P (.vector t) = true := by
    rw [← hrt]
    exact hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hold
  obtain ⟨elems, rfl, helems⟩ := conforms_vector holdc
  cases hrev : elems.reverse with
  | nil =>
    have hstep : Step.exec P s fr rest info code (.vecPopBack sig) = .error (.failed .emptyVector) := by
      simp only [Step.exec, elemType_ok het, popRefTo_vec hst1 hrm hrt, Step.derefVector,
        Step.deref, Step.need, hold, hrev, bind, Except.bind, pure, Except.pure, Step.fail]
    rw [hstep]
    trivial
  | cons last init =>
    have hmem : ∀ e, e ∈ last :: init → e ∈ elems := fun e he => by
      rw [← hrev] at he; exact List.mem_reverse.1 he
    have hlast : last.conforms P t = true := helems last (hmem last List.mem_cons_self)
    have hnv : (Value.vector t init.reverse).conforms P (.vector t) = true :=
      conforms_vector_mk fun e he =>
        helems e (hmem e (List.mem_cons_of_mem _ (List.mem_reverse.1 he)))
    obtain ⟨w', hset⟩ := set_some _ (.vector t init.reverse) hwo (hok₁.writable hrm)
    have hupd := update_eq hroot hset
    have hstep : Step.exec P s fr rest info code (.vecPopBack sig) = .ok (.done (.running
          (writeState s { fr with stack := st } rest r.location w' [last]))) := by
      simp only at hold hupd
      simp only [Step.exec, elemType_ok het, popRefTo_vec hst1 hrm hrt,
        Step.derefVector, Step.deref, Step.need, hold, hrev, bind, Except.bind, pure, Except.pure,
        hupd, Step.finish, List.map_cons, writeState]
    rw [hstep]
    obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
    have hd₁ := hd.popRelease hctx.typed hsa hst1 hlen hrel rfl (fr' := { fr with stack := st })
      rfl rfl rfl
    have hd₀ := hd₁.pushVal (v := last) (A' := { A with stack := .nonReference :: ra, graph := g })
      (fr' := { fr with stack := last :: st }) rfl rfl rfl rfl
    exact m2_writeClose hc hr hsf hact hchain hi rfl (fun _ => rfl) hbefore hafter hd hrefs
      (hctx.chain _ hsf) hctx.roots hp hL hwr hrm hd₀ (heldIds_sub rfl (fun x hx => by
        rw [hsa]
        rcases List.mem_cons.1 hx with hx | hx
        · cases hx
        · exact List.mem_cons_of_mem _ hx)) hA'' hle rfl rfl (by simp)
      (by simp [conformsAll_cons, hstc, hlast])
      (WriteFacts.ofRigid (afterWrite_writeState hsf st r.location w' [last]) (hrefs id r hp hL)
        (by rw [hrt]; rfl) hr.ids hctx.roots hr.storage ((root?_withStack hsf st _).symm.trans hroot)
        hset (by rw [hrt]; exact hnv)) ⟨[.ref r], by simp [hst1]⟩
      (fun x hx => by
        simp only [List.mem_singleton] at hx; subst hx
        exact not_ref_of_conforms hlast (vectorElemTypeWith_value (typeArgsOk_values hact.typeArgs) het))

/-- **`VecSwap`.** -/
theorem refSafe_vecSwap (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {sig : Nat}
    (hctx : TopCtx P s fr info code (.vecSwap sig) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.vecSwap sig)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.vecSwap sig)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨t, brest, het, rfl, rfl⟩ := typeAfter_vecSwap hafter
  obtain ⟨vj, st1, hst1, hvj, h1⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨jv, rfl, -⟩ := conforms_int hvj
  obtain ⟨vi, st2, hst2, hvi, h2⟩ := conformsAll_cons_right h1
  subst hst2
  obtain ⟨iv, rfl, -⟩ := conforms_int hvi
  obtain ⟨rv, st, hst3, hrv, hstc⟩ := conformsAll_cons_right h2
  subst hst3
  obtain ⟨r, rfl, hrm, hrt⟩ := conforms_ref hrv (by simp [rigid])
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨id, ra, g, hsa, hwr, hrel, rfl⟩ := exec_vecSwap hx
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst1] at this; simpa using this
  obtain ⟨t0, t1, stys0, hstys⟩ : ∃ t0 t1 stys0, stys = t0 :: t1 :: stys0 := by
    have := hctx.typed.nstack
    rw [hsa] at this
    match stys, this with
    | t0 :: t1 :: stys0, _ => exact ⟨t0, t1, stys0, rfl⟩
  have ht1 := hctx.typed.popValue hsa hstys
  have ht2 := ht1.popValue (st := { A with stack := .nonReference :: .reference id :: ra })
    (rest := .reference id :: ra) rfl rfl
  have hd1 := hd.pop (A' := { A with stack := .nonReference :: .reference id :: ra })
    (fr' := { fr with stack := .int .u64 iv :: .ref r :: st }) hsa hst1 (by simp [hlen]) rfl rfl
    rfl rfl rfl
  have hd2 := hd1.pop (A' := { A with stack := .reference id :: ra })
    (fr' := { fr with stack := .ref r :: st }) rfl rfl (by simp [hlen]) rfl rfl rfl rfl rfl
  obtain ⟨r', hr', hL⟩ := hd2.top rfl rfl hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by
    rw [hsa]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ List.mem_cons_self)))
  have hok₁ : RefOk P { s with frames := { fr with stack := st } :: rest }
      { fr with stack := st } r := (hrefs id r hp hL).same hsf rfl rfl rfl rfl rfl rfl rfl
  obtain ⟨w, old, hroot, hwo, hold⟩ := hok₁.root_value
  have holdc : old.conforms P (.vector t) = true := by
    rw [← hrt]
    exact hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hold
  obtain ⟨elems, rfl, helems⟩ := conforms_vector holdc
  have hj : Step.popIndex fr = .ok (jv.toNat, { fr with stack := .int .u64 iv :: .ref r :: st }) := by
    simp [Step.popIndex, Step.popInt, Step.pop, hst1, bind, Except.bind, pure, Except.pure]
  have hi' : Step.popIndex { fr with stack := .int .u64 iv :: .ref r :: st } =
      .ok (iv.toNat, { fr with stack := .ref r :: st }) := by
    simp [Step.popIndex, Step.popInt, Step.pop, bind, Except.bind, pure, Except.pure]
  cases hx' : elems[iv.toNat]? with
  | none =>
    have hstep : Step.exec P s fr rest info code (.vecSwap sig) = .error (.failed .vectorIndex) := by
      simp only [Step.exec, elemType_ok het, hj, hi', popRefTo_vec (fr := { fr with stack := .ref r :: st }) rfl hrm hrt,
        Step.derefVector, Step.deref, Step.need, hold, hx', bind, Except.bind, pure, Except.pure,
        Step.fail]
    rw [hstep]
    trivial
  | some xe =>
    cases hy' : elems[jv.toNat]? with
    | none =>
      have hstep : Step.exec P s fr rest info code (.vecSwap sig) =
          .error (.failed .vectorIndex) := by
        simp only [Step.exec, elemType_ok het, hj, hi', popRefTo_vec (fr := { fr with stack := .ref r :: st }) rfl hrm hrt,
          Step.derefVector, Step.deref, Step.need, hold, hx', hy', bind, Except.bind, pure,
          Except.pure, Step.fail]
      rw [hstep]
      trivial
    | some ye =>
      have hnv : (Value.vector t ((elems.set iv.toNat ye).set jv.toNat xe)).conforms P
          (.vector t) = true := by
        refine conforms_vector_mk fun e he => ?_
        rcases List.mem_or_eq_of_mem_set he with he | rfl
        · rcases List.mem_or_eq_of_mem_set he with he | rfl
          · exact helems e he
          · exact helems _ (List.mem_of_getElem? hy')
        · exact helems _ (List.mem_of_getElem? hx')
      obtain ⟨w', hset⟩ := set_some _ (.vector t ((elems.set iv.toNat ye).set jv.toNat xe)) hwo
        (hok₁.writable hrm)
      have hupd := update_eq hroot hset
      have hstep : Step.exec P s fr rest info code (.vecSwap sig) = .ok (.done (.running
          (writeState s { fr with stack := st } rest r.location w' []))) := by
        simp only at hold hupd
        simp only [Step.exec, elemType_ok het, hj, hi', popRefTo_vec (fr := { fr with stack := .ref r :: st }) rfl hrm hrt,
          Step.derefVector, Step.deref, Step.need, hold, hx', hy', bind, Except.bind, pure,
          Except.pure, hupd, Step.finish, List.map_cons, writeState]
      rw [hstep]
      obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
      have hd₀ := hd2.popRelease ht2 rfl rfl hlen hrel (A' := { A with stack := ra, graph := g })
        (fr' := { fr with stack := st }) rfl rfl rfl rfl
      exact m2_writeClose hc hr hsf hact hchain hi rfl (fun _ => rfl) hbefore hafter hd hrefs
        (hctx.chain _ hsf) hctx.roots hp hL hwr hrm hd₀ (heldIds_sub rfl (fun x hx => by
          rw [hsa]
          exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hx))))
        hA'' hle rfl rfl (by simp)
        (by simpa [Step.advance, Step.pushAll] using hstc)
        (WriteFacts.ofRigid (afterWrite_writeState hsf st r.location w' []) (hrefs id r hp hL)
          (by rw [hrt]; rfl) hr.ids hctx.roots hr.storage ((root?_withStack hsf st _).symm.trans hroot)
          hset (by rw [hrt]; exact hnv)) ⟨[.int .u64 jv, .int .u64 iv, .ref r], by simp [hst1]⟩
        (by simp)

/-! ## Equality -/

theorem typeAfter_eq {f : FunctionRef} {m : CompiledModule} {positive : Bool}
    {before after : List RuntimeType}
    (h : typeAfter P f m (if positive then .eq else .neq) before = some after) :
    ∃ t t' brest, before = t' :: t :: brest ∧ t.comparable t' = true ∧
      (t.abilities P).drop = true ∧ (t'.abilities P).drop = true ∧ after = .bool :: brest := by
  have h' : typeAfter P f m .eq before = some after := by cases positive <;> exact h
  clear h
  dsimp only [typeAfter] at h'
  split at h'
  · next t' t brest =>
    split at h'
    · next hc' =>
      simp only [Bool.and_eq_true] at hc'
      obtain ⟨⟨hcmp, hdrop⟩, hdrop'⟩ := hc'
      exact ⟨t, t', brest, rfl, hcmp, hdrop, hdrop', (Option.some.inj h').symm⟩
    · simp at h'
  · simp at h'

theorem equality_ref {positive : Bool} {ra rb : Reference} {vs : List Value} {x y : Value}
    (hst : fr.stack = .ref rb :: .ref ra :: vs)
    (hty : RuntimeType.comparable (Value.ref ra).type (Value.ref rb).type = true)
    (ha : ((Value.ref ra).abilities P).drop = true) (hb : ((Value.ref rb).abilities P).drop = true)
    (hx : s.deref? ra.location ra.since = some x) (hy : s.deref? rb.location rb.since = some y) :
    Step.equality P s fr positive = .ok (Step.advance (Step.push { fr with stack := vs }
      (.bool (if positive then x == y else x != y)))) := by
  simp only [Step.equality, Step.pop, hst, bind, Except.bind, pure, Except.pure, hty,
    Bool.false_eq_true, ↓reduceIte, ha, hb, Bool.not_true, Bool.or_self,
    Step.deref, Step.need, hx, hy]

theorem exec_equality {positive : Bool} :
    Step.exec P s fr rest info code (if positive then .eq else .neq) =
      (Step.equality P s fr positive >>= fun fr => Step.next s rest fr) := by
  cases positive <;> rfl

/-- **Closing a comparison**: the frame has the boolean on top. -/
theorem refSafe_eqClose (hc : Certified P (modelTypes P) (modelAvail P))
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest)
    (hchainR : Below P s fr ps rest) {positive : Bool}
    (hi : code.code[fr.pc]? = some (if positive then .eq else .neq))
    {before brest : List RuntimeType} (hbefore : modelTypes P fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module (if positive then .eq else .neq) before =
      some (.bool :: brest))
    {vs : List Value} (hvs : conformsAll P vs brest = true) (hvsuf : fr.stack.drop 2 = vs)
    {L : Nat → Option Reference}
    (hrefs : ∀ x r, x ∈ heldIds A → L x = some r → RefOk P s fr r) (c : Bool) {A' : RefState}
    (hd' : Desc A' (Step.advance (Step.push { fr with stack := vs } (.bool c))) (gsidOf P (s.frames.map Frame.skel)) ps L)
    (hheld : ∀ x ∈ heldIds A', x ∈ heldIds A)
    {A'' : RefState} (hA'' : modelRef P fr.function (fr.pc + 1) = some A'') (hle : RefLe A' A'') :
    InvRefs P { s with frames := Step.advance (Step.push { fr with stack := vs } (.bool c)) :: rest } := by
  refine ⟨?_, ?_⟩
  · exact fallThrough_safe (N := fun _ _ _ => False) (after := .bool :: brest) hc hact hchain hi
      (by cases positive <;> rfl) (fun _ => by cases positive <;> rfl) hbefore hafter rfl rfl rfl
      (by simp [Step.advance, Step.push, hvs, Value.conforms, Value.type, Value.wellFormed])
  · exact refInv_finish hr hsf hchainR rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage (es := []) rfl
      (storageKept_of_eq rfl) allowedBy_nil hd'
      (fun x r hx hxr => (hrefs x r (hheld x hx) hxr).same hsf rfl rfl rfl rfl rfl rfl rfl) hA'' hle
      (RefsFrom.next (new := [.bool c]) (n := 2) (by simp [Step.advance, Step.push, hvsuf]) rfl
        (fun v hv => by
          simp only [List.mem_singleton] at hv; subst hv; exact .of_not_ref (fun _ h => by cases h)))

/-- **`Eq`, `Neq`.** -/
theorem refSafe_eq (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {positive : Bool}
    (hctx : TopCtx P s fr info code (if positive then .eq else .neq) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (if positive then .eq else .neq)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (if positive then .eq else .neq)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨t, t', brest, rfl, hcmp, hdrop, hdrop', rfl⟩ := typeAfter_eq hafter
  obtain ⟨b, st1, hst1, hb, h1⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨a, vs, hst2, ha, hvs⟩ := conformsAll_cons_right h1
  subst hst2
  have hda := AbilitySet.drop_of_subset (abilities_of_conforms hc.abilities ha) hdrop
  have hdb := AbilitySet.drop_of_subset (abilities_of_conforms hc.abilities hb) hdrop'
  have hty : a.type.comparable b.type = true :=
    comparable_of_assignable hcmp (conforms_assignable ha) (conforms_assignable hb)
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by cases positive <;> intro h <;> cases h)
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1)
    (by cases positive <;> simp [successors, hi, Bytecode.control])
  obtain ⟨v1, v2, ra, hsa, hcase⟩ := exec_eq (by cases positive <;> simp) hx
  have hlen : ra.length = vs.length := by
    have := hctx.height; rw [hsa, hst1] at this; simpa using this
  rw [exec_equality]
  rcases hcase with ⟨rfl, rfl, rfl⟩ | ⟨id1, id2, g1, g2, rfl, rfl, hrel1, hrel2, rfl⟩
  · -- Values.
    have hra : ∀ r, a ≠ .ref r := notRef_of_abs hctx.typed hctx.targs hctx.sig hctx.conf (k := 1)
      (by rw [hsa]; rfl) (by rw [hst1]; rfl)
    have hd1 := hd.pop (A' := { A with stack := .nonReference :: ra })
      (fr' := { fr with stack := a :: vs }) hsa hst1 (by simp [hlen]) rfl rfl rfl rfl rfl
    have hd2 := hd1.pop (A' := { A with stack := ra }) (fr' := { fr with stack := vs }) rfl rfl
      hlen rfl rfl rfl rfl rfl
    refine equality_safe hst1 hty hda hdb hra ?_
    exact refSafe_eqClose hc hr hsf hact hchain (hctx.chain _ hsf) hi hbefore hafter hvs
      (by simp [hst1]) hrefs
      (if positive then a == b else a != b)
      (hd2.pushVal (A' := { A with stack := .nonReference :: ra })
        (v := .bool (if positive then a == b else a != b)) rfl rfl rfl
        (by simp [Step.advance, Step.push]))
      (heldIds_sub rfl (fun x hx => by
        rw [hsa]
        rcases List.mem_cons.1 hx with hx | hx
        · cases hx
        · exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hx))) hA'' hle
  · -- References.
    obtain ⟨rb, rfl, hL1⟩ := hd.top hsa hst1 (by simp [hlen])
    obtain ⟨ra', hra', hL2⟩ := hd.stack ra.length id2 (by
      rw [hsa, reverse_cons_getElem?_lt (by simp), reverse_cons_getElem?_top])
    rw [hst1, reverse_cons_getElem?_lt (by simp [hlen]), hlen, reverse_cons_getElem?_top,
      Option.some.injEq] at hra'
    subst hra'
    have hp1 : id1 ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
    have hp2 : id2 ∈ heldIds A := mem_heldIds.2 (Or.inr (by
      rw [hsa]; exact List.mem_cons_of_mem _ List.mem_cons_self))
    obtain ⟨x, hxv⟩ := (hrefs id2 ra' hp2 hL2).deref_some
    obtain ⟨y, hyv⟩ := (hrefs id1 rb hp1 hL1).deref_some
    obtain ⟨t1, t2, stys0, hstys⟩ : ∃ t1 t2 stys0, stys = t1 :: t2 :: stys0 := by
      have := hctx.typed.nstack
      rw [hsa] at this
      match stys, this with
      | t1 :: t2 :: stys0, _ => exact ⟨t1, t2, stys0, rfl⟩
    have ht1 := hctx.typed.popRelease hsa hrel1 hstys
    have hd1 := hd.popRelease hctx.typed hsa hst1 (by simp [hlen]) hrel1 rfl
      (fr' := { fr with stack := .ref ra' :: vs }) rfl rfl rfl
    have hd2 := hd1.popRelease ht1 rfl rfl hlen hrel2 rfl (fr' := { fr with stack := vs }) rfl rfl rfl
    rw [equality_ref hst1 hty hda hdb hxv hyv]
    exact refSafe_eqClose hc hr hsf hact hchain (hctx.chain _ hsf) hi hbefore hafter hvs
      (by simp [hst1]) hrefs
      (if positive then x == y else x != y)
      (hd2.pushVal (A' := { A with stack := .nonReference :: ra, graph := g2 })
        (v := .bool (if positive then x == y else x != y)) rfl rfl rfl
        (by simp [Step.advance, Step.push]))
      (heldIds_sub rfl (fun x hx => by
        rw [hsa]
        rcases List.mem_cons.1 hx with hx | hx
        · cases hx
        · exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hx))) hA'' hle

end MoveBytecode.Proofs
