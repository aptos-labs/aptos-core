-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostSteps

/-!
# Reading and writing through references, and global storage

UNTRUSTED (checked by the kernel). The ghost facts of the steps that reach
beyond the current frame: reading through a reference copies the identities
of a value anywhere in the state (`readRef_ghost`); writing through one
replaces a value anywhere in the state, its containers keeping their
identities (`mutate_ghost`); `MoveFrom` and `MoveTo` move a value between
the operand stack and global storage (`moveFrom_ghost`, `moveTo_ghost`).

The ghost layer finds a location from the state before the step and the
shadow with the step's operands popped; the state finds it after popping.
Popping changes neither the frames' identities and locals nor global storage,
so both find the same (`tree_eq`, `read_eq`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-! ## Finding locations after popping -/

theorem framePos_congr {s s' : State} (h : s'.frames.map (·.id) = s.frames.map (·.id))
    (f : FrameId) : Shadow.framePos s' f = Shadow.framePos s f := by
  have h1 : Shadow.framePos s' f = (s'.frames.map (·.id)).findIdx? (· == f) := by
    simp [Shadow.framePos, Function.comp_def]
  have h2 : Shadow.framePos s f = (s.frames.map (·.id)).findIdx? (· == f) := by
    simp [Shadow.framePos, Function.comp_def]
  rw [h1, h2, h]

theorem resourcePos_congr {s s' : State} (h : s'.storage = s.storage) (a : Address) (t : StructId) :
    Shadow.resourcePos s' a t = Shadow.resourcePos s a t := by
  simp [Shadow.resourcePos, h]

/-- The shadow's tree at a root depends on the frames' identities, the
shadow's locals, and global storage and held values only. -/
theorem rootTree_eq {h h' : Shadow} {s s' : State}
    (hids : s'.frames.map (·.id) = s.frames.map (·.id)) (hst : s'.storage = s.storage)
    (hloc : h'.frames.map (·.locals) = h.frames.map (·.locals)) (hhst : h'.storage = h.storage)
    (hhe : h'.held = h.held) (root : Root) : h'.rootTree? s' root = h.rootTree? s root := by
  cases root with
  | local_ f i =>
    simp only [Shadow.rootTree?, framePos_congr hids]
    cases Shadow.framePos s f with
    | none => rfl
    | some k =>
      have := congrArg (·[k]?) hloc
      simp only [List.getElem?_map] at this
      simp only [bind, Option.bind_some]
      cases h1 : h'.frames[k]? <;> cases h2 : h.frames[k]? <;> simp_all
  | global a t => simp [Shadow.rootTree?, resourcePos_congr hst, hhst]
  | held k => simp [Shadow.rootTree?, hhe]

theorem tree_eq {h h' : Shadow} {s s' : State}
    (hids : s'.frames.map (·.id) = s.frames.map (·.id)) (hst : s'.storage = s.storage)
    (hloc : h'.frames.map (·.locals) = h.frames.map (·.locals)) (hhst : h'.storage = h.storage)
    (hhe : h'.held = h.held) (l : Location) (hroot : s'.root? l.root = s.root? l.root) :
    h'.tree? s' l = h.tree? s l := by
  simp [Shadow.tree?, rootTree_eq hids hst hloc hhst hhe, hroot]

/-- A state's value at a root depends on its frames' identities and locals,
global storage and held values only. -/
theorem root_eq {s s' : State}
    (hfr : s'.frames.map (fun fr => (fr.id, fr.locals)) = s.frames.map (fun fr => (fr.id, fr.locals)))
    (hst : s'.storage = s.storage) (hhe : s'.held = s.held) (root : Root) :
    s'.root? root = s.root? root := by
  cases root with
  | local_ f i =>
    have key : ∀ s : State, (s.frame? f).map (·.locals) =
        ((s.frames.map (fun fr => (fr.id, fr.locals))).find? (·.1 == f)).map (·.2) := by
      intro s
      simp [State.frame?, List.find?_map, Function.comp_def, Option.map_map]
    have h1 := key s'
    rw [hfr, ← key s] at h1
    simp only [State.root?, bind]
    cases ha : s'.frame? f <;> cases hb : s.frame? f <;> simp_all
  | global a t => simp [State.root?, hst]
  | held k => simp [State.root?, hhe]

theorem read_eq {s s' : State}
    (hfr : s'.frames.map (fun fr => (fr.id, fr.locals)) = s.frames.map (fun fr => (fr.id, fr.locals)))
    (hst : s'.storage = s.storage) (hhe : s'.held = s.held) (l : Location) :
    s'.read? l = s.read? l := by
  simp [State.read?, root_eq hfr hst hhe]

theorem deref_eq {s s' : State}
    (hfr : s'.frames.map (fun fr => (fr.id, fr.locals)) = s.frames.map (fun fr => (fr.id, fr.locals)))
    (hst : s'.storage = s.storage) (hhe : s'.held = s.held)
    (hinv : s'.invalidations = s.invalidations) (l : Location) (n : Nat) :
    s'.deref? l n = s.deref? l n := by
  simp [State.deref?, State.invalidatedSince, hinv, read_eq hfr hst hhe]

/-! ## The current frame's segment -/

/-- The occurrences of a shadow and state whose current frames are `g'` and
`fr'`, above the frames `hs` and `rest`, with the global storage and held
values of `h` and `s`. -/
theorem occ_head {h' : Shadow} {s' : State} {g' : ShadowFrame} {fr' : Frame}
    {hs : List ShadowFrame} {rest : List Frame} (hh : h'.frames = g' :: hs)
    (hs' : s'.frames = fr' :: rest) (h : Shadow) (s : State) (hst : h'.storage = h.storage)
    (hhe : h'.held = h.held) (sst : s'.storage = s.storage) (she : s'.held = s.held) :
    h'.occurrences s' = g'.occurrences fr' ++ framesOcc hs rest ++ extOcc h s := by
  rw [shadow_occurrences, hh, hs', framesOcc_cons, extOcc_congr hst hhe sst she, List.append_assoc]

theorem fits_head {h' : Shadow} {s' : State} {g' : ShadowFrame} {fr' : Frame}
    {hs : List ShadowFrame} {rest : List Frame} (hh : h'.frames = g' :: hs)
    (hs' : s'.frames = fr' :: rest) (h : Shadow) (s : State) (hst : h'.storage = h.storage)
    (hhe : h'.held = h.held) (sst : s'.storage = s.storage) (she : s'.held = s.held) :
    h'.fits s' = (g'.fits fr' && framesFit hs rest && extFit h s) := by
  rw [shadow_fits, hh, hs', framesFit_cons, extFit_congr hst hhe sst she]

/-- Popping `n` values off the current frame's segment. -/
theorem occ_pop {h : Shadow} {s : State} {g : ShadowFrame} {fr : Frame} {hs : List ShadowFrame}
    {rest : List Frame} (hhf : h.frames = g :: hs) (hsf : s.frames = fr :: rest)
    (hfa : IdTree.fitsAll g.stack fr.stack = true) (n : Nat) :
    (h.occurrences s).Perm (IdTree.occurrencesAll (g.stack.take n) (fr.stack.take n) ++
      Shadow.occurrences { h with frames := { g with stack := g.stack.drop n } :: hs }
        { s with frames := { fr with stack := fr.stack.drop n } :: rest }) := by
  rw [occ_head hhf hsf h s rfl rfl rfl rfl,
    occ_head (h' := { h with frames := { g with stack := g.stack.drop n } :: hs })
      (s' := { s with frames := { fr with stack := fr.stack.drop n } :: rest }) rfl rfl h s rfl rfl
      rfl rfl,
    frame_occurrences, frame_occurrences, stack_split hfa n]
  simp only [localsOcc]
  perm_parts

/-- Pushing values onto the current frame's segment. -/
theorem occ_push {h : Shadow} {s : State} {g : ShadowFrame} {fr : Frame} {hs : List ShadowFrame}
    {rest : List Frame} (hhf : h.frames = g :: hs) (hsf : s.frames = fr :: rest)
    {ts : List IdTree} {vs : List Value} (hl : ts.length = vs.length) (s₃ : State)
    (hst : s₃.storage = s.storage) (hhe : s₃.held = s.held) (pc : Nat) (f' : Nat) :
    (Shadow.occurrences { h with frames := { g with stack := ts ++ g.stack } :: hs, fresh := f' }
        { s₃ with frames := { fr with stack := vs ++ fr.stack, pc := pc } :: rest }).Perm
      (IdTree.occurrencesAll ts vs ++ h.occurrences s) := by
  rw [occ_head hhf hsf h s rfl rfl rfl rfl,
    occ_head (h' := { h with frames := { g with stack := ts ++ g.stack } :: hs, fresh := f' })
      (s' := { s₃ with frames := { fr with stack := vs ++ fr.stack, pc := pc } :: rest }) rfl rfl h s
      rfl rfl hst hhe,
    frame_occurrences, frame_occurrences, occurrencesAll_append hl]
  simp only [localsOcc]
  perm_parts

/-! ## Reading through a reference -/

section Read

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}

theorem ref_drop {r : Reference} : ((Value.ref r).abilities P).drop = true := by
  simp [Value.abilities, RuntimeType.abilities]

theorem ref_occ {t : IdTree} {r : Reference} (ht : t.fits (.ref r) = true) :
    t.occurrences (.ref r) = [(t.id, .ref r)] := by
  obtain ⟨i, cs⟩ := t
  simp only [IdTree.fits, Value.children] at ht
  cases cs with
  | nil => simp [IdTree.occurrencesAll, IdTree.id]
  | cons => simp [IdTree.fitsAll] at ht

/-- `ReadRef`: the reference is discarded, and the identities of the value
at its location occur again on the segment. -/
theorem readRef_ghost {s s₂ : State} {h : Shadow} {fr : Frame} {rest : List Frame}
    {g : ShadowFrame} {hs : List ShadowFrame} (hp : Pre σ P s h) (hsf : s.frames = fr :: rest)
    (hhf : h.frames = g :: hs) {r : Reference} {st : List Value} (hst : fr.stack = .ref r :: st)
    {v : Value} (hv : s.read? r.location = some v) (hc : (v.abilities P).copy = true)
    (h2st : s₂.storage = s.storage) (h2he : s₂.held = s.held) (h2nf : s₂.nextFrame = s.nextFrame)
    {evs : List Event}
    (hevs : ∀ e ∈ evs, e.Silent = true ∧ Encapsulation.structsRespected π A e = true) :
    ∃ t ts tv, g.stack = t :: ts ∧ h.tree? s r.location = some tv ∧
      StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh
        (.running { s₂ with frames := { fr with stack := v :: st, pc := fr.pc + 1 } :: rest })
        (.running { h with frames := { g with stack := tv :: ts } :: hs, fresh := h.fresh }) evs := by
  have hfit := hp.fits
  rw [shadow_fits, hhf, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hfa := frame_stack_fits hg
  rw [hst] at hfa
  obtain ⟨t, ts, hgst, ht, hts⟩ := fitsAll_cons_right hfa
  -- The state and shadow with the reference popped.
  let h₁ : Shadow := { h with frames := { g with stack := ts } :: hs }
  let s₁ : State := { s with frames := { fr with stack := st } :: rest }
  have hfit₁ : h₁.fits s₁ = true := by
    rw [fits_head (h' := h₁) (s' := s₁) rfl rfl h s rfl rfl rfl rfl]
    simp [frame_fits_stack hg hts, hrest, hext]
  have hv₁ : s₁.read? r.location = some v := by
    have := read_eq (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl rfl r.location
    rw [this]; exact hv
  obtain ⟨tv, htv, hfv, R, hR⟩ := tree_read hfit₁ hv₁
  have htv' : h.tree? s r.location = some tv := by
    rw [← tree_eq (h := h) (h' := h₁) (s := s) (s' := s₁) (by simp [s₁, hsf]) rfl
      (by simp [h₁, hhf]) rfl rfl _ (root_eq (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl rfl _)]
    exact htv
  have hsil := silent_of fun e he => (hevs e he).1
  have hvp : ∀ u ∈ subvalues v, u.isRef = false :=
    hp.refsTop.deref fr (by rw [hsf]; exact List.mem_cons_self ..) r
      (Or.inl (by rw [hst]; exact List.mem_cons_self ..)) v hv
  have hpre : (h.occurrences s).Perm ([(t.id, .ref r)] ++ h₁.occurrences s₁) := by
    rw [occ_head hhf hsf h s rfl rfl rfl rfl, occ_head (h' := h₁) (s' := s₁) rfl rfl h s rfl rfl rfl rfl,
      frame_occurrences, frame_occurrences, hgst, hst]
    simp only [IdTree.occurrencesAll, ref_occ ht, localsOcc]
    perm_parts
  let h₃ : Shadow := { h with frames := { g with stack := tv :: ts } :: hs, fresh := h.fresh }
  let s₃ : State := { s₂ with frames := { fr with stack := v :: st, pc := fr.pc + 1 } :: rest }
  have hpost : (h₃.occurrences s₃).Perm (tv.occurrences v ++ h₁.occurrences s₁) := by
    rw [occ_head (h' := h₃) (s' := s₃) rfl rfl h s rfl rfl h2st h2he,
      occ_head (h' := h₁) (s' := s₁) rfl rfl h s rfl rfl rfl rfl, frame_occurrences,
      frame_occurrences]
    simp only [IdTree.occurrencesAll, localsOcc]
    perm_parts
  refine ⟨t, ts, tv, hgst, htv', ?_, ?_, ?_, ?_, ?_, frames_inv hp.inv hsf h2st h2he h2nf rfl,
    createdPlain_of hsil.1, ?_, ?_, closures_silent fun e he => (hevs e he).1,
    ClosuresKept.of_sub fun x hx _ => ?_⟩
  rotate_right
  · rcases List.mem_append.1 (hpost.subset hx) with h1 | h1
    · exact hpre.symm.subset (List.mem_append_right _ (hR.symm.subset (List.mem_append_left _ h1)))
    · exact hpre.symm.subset (List.mem_append_right _ h1)
  · simp only [ShadowPost.fits]
    rw [fits_head (h' := { h with frames := { g with stack := tv :: ts } :: hs, fresh := h.fresh })
      (s' := { s₂ with frames := { fr with stack := v :: st, pc := fr.pc + 1 } :: rest })
      rfl rfl h s rfl rfl h2st h2he]
    simp only [Bool.and_eq_true]
    exact ⟨⟨frame_fits_stack hg (by simp [IdTree.fitsAll, hfv, hts]) _, hrest⟩, hext⟩
  · show EffectK P _ _ _ _ h.fresh
    -- Before: the reference and everything else; after: the copy and everything else.
    refine EffectK.mk' (keys (h₁.occurrences s₁)) [(t.id, (Value.ref r).key)]
      (keys (tv.occurrences v)) [] [] hsil.2.2 (by simp [hsil.2.1])
      ((keys_perm hpre).trans (by simp only [keys_cons,
        List.singleton_append, List.append_nil]; perm_parts))
      ((keys_perm hpost).trans (by simpa using List.perm_append_comm))
      (by simp [Value.type, RuntimeType.abilities, key_type]) ?_ (by simp) (by simp) (Nat.le_refl _)
    intro x hx
    obtain ⟨o, ho, h1, h2⟩ := mem_keys_iff.1 hx
    refine ⟨?_, by rw [← h2, key_type]; exact deep_copy v hc o.2 (occurrences_mem hfv ho)⟩
    have : x = (o.1, o.2.key) := by rw [h1, h2]
    rw [this]
    exact mem_keys (hR.symm.subset (List.mem_append_left _ ho))
  · intro i w c he
    exact absurd (consumed_mem he) (by simp [hsil.2.1])
  · intro e he
    exact ⟨(hevs e he).2, signers_respected (hevs e he).1 auth⟩
  · intro x hx
    simp [hsil.2.2] at hx
  · intro d r' hm
    refine Or.inl ?_
    simp only [postFrames, framesOcc_cons, frame_occurrences, localsOcc_stack,
      IdTree.occurrencesAll, List.mem_append] at hm
    rw [hhf, hsf, framesOcc_cons, frame_occurrences, hgst, hst]
    simp only [IdTree.occurrencesAll, List.mem_append]
    rcases hm with (hm | hm | hm) | hm
    · exact Or.inl (Or.inl hm)
    · have := hvp _ (occurrences_mem hfv hm)
      simp [Value.isRef] at this
    · exact Or.inl (Or.inr (Or.inr hm))
    · exact Or.inr hm
  · rw [hhf, hsf]
    refine refsPerFrame_top rfl rfl fun d r' hm => Or.inl ?_
    simp only [frame_occurrences, localsOcc_stack, IdTree.occurrencesAll, List.mem_append] at hm
    rw [frame_occurrences, hgst, hst]
    simp only [IdTree.occurrencesAll, List.mem_append]
    rcases hm with hm | hm | hm
    · exact Or.inl hm
    · have := hvp _ (occurrences_mem hfv hm)
      simp [Value.isRef] at this
    · exact Or.inr (Or.inr hm)

end Read

/-! ## Writing through a reference -/

section Write

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}

/-- The shadow's trees at locations depend on the state through its frames'
identities, its resources and the value at the location's root only. -/
theorem tree_state {h : Shadow} {s s' : State} (hids : s'.frames.map (·.id) = s.frames.map (·.id))
    (hst : s'.storage = s.storage) (l : Location) (t : IdTree)
    (hroot : s'.root? l.root = s.root? l.root) :
    h.tree? s' l = h.tree? s l ∧ h.withTree? s' l t = h.withTree? s l t := by
  have := rootTree_congr (h := h) hids (by rw [hst])
  simp [Shadow.tree?, Shadow.withTree?, this.1, this.2, hroot]

theorem shadow_eta {h : Shadow} {g : ShadowFrame} {hs : List ShadowFrame} (hh : h.frames = g :: hs) :
    ({ h with frames := { g with stack := [] ++ g.stack } :: hs } : Shadow) = h := by
  cases h; simp_all

theorem setRoot_storage_typed {s : State} (hinv : StateInv σ P s) {root : Root} {v v'' : Value}
    (hv : s.root? root = some v) (hty : ∀ a t, root = .global a t → v''.type = v.type) :
    ∀ r ∈ (s.setRoot root v'').storage, r.value.type = .struct r.type := by
  cases root with
  | local_ f i => simpa [State.setRoot] using hinv.typed
  | held k => simpa [State.setRoot] using hinv.typed
  | global a t =>
    simp only [State.setRoot, GlobalStorage.update, List.mem_map]
    rintro r ⟨r', hr', rfl⟩
    split
    · next hk =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hk
      obtain ⟨k, res, -, hres, rfl, hra, hrt⟩ := storage_get hv
      have := hinv.typed res (List.mem_of_getElem? hres)
      simp only [hty a t rfl, this, hrt, hk.2]
    · exact hinv.typed r' hr'

/-! ### Closures across a write -/

/-- A value with a child, and every value of its key, is no closure. -/
theorem child_not_closure {v c w : Value} {step : PathStep} (h : v.child? step = some c)
    (hk : w.key = v.key) : IsClosure w = false := by
  cases v <;> cases step <;> simp [Value.child?] at h <;> cases w <;> simp_all [Value.key, IsClosure]

/-- **Writing a path keeps every closure**: a closure node of the new value
is one of the old value off the path, or of the written value. -/
theorem path_write_closures : ∀ (path : List PathStep) {t : IdTree} {v w : Value} {tw' : IdTree}
    {w' : Value}, t.fits v = true → v.get? path = some w →
      ∀ {v'' : Value} {t'' : IdTree}, v.set? path w' = some v'' →
        t.set? (v.indicesOf path) tw' = some t'' →
        ∀ x ∈ t''.occurrences v'', IsClosure x.2 = true →
          x ∈ t.occurrences v ∨ x ∈ tw'.occurrences w'
  | [], t, v, w, tw', w', _, _, v'', t'', hset, htset, x, hx, _ => by
    simp only [Value.set?, Option.some.injEq] at hset
    simp only [Value.indicesOf, IdTree.set?, Option.some.injEq] at htset
    subst hset htset
    exact Or.inr hx
  | step :: path, .node i cs, v, w, tw', w', ht, hw, v'', t'', hset, htset, x, hx, hcl => by
    obtain ⟨c, hc', hw⟩ := get_cons hw
    simp only [IdTree.fits] at ht
    have hcc := child_children hc'
    obtain ⟨tc, htc, hfc⟩ := fitsAll_get ht hcc
    simp only [Value.set?, bind, Option.bind_eq_some_iff, hc', Option.bind_some] at hset
    obtain ⟨c'', hc'', hset⟩ := hset
    simp only [indicesOf_child hc', IdTree.set?, IdTree.children, bind, Option.bind_eq_some_iff, htc,
      Option.bind_some, pure, IdTree.id] at htset
    obtain ⟨tc'', htc'', hT⟩ := htset; simp only [Option.some.injEq] at hT; subst hT
    obtain ⟨v₂, hwc, hch, hkey⟩ := withChild_spec hc' (withChild?_writable hset) c''
    rw [hwc, Option.some.injEq] at hset
    subst hset
    obtain ⟨R', hR', hR'set⟩ := occurrencesAll_set htc hcc tc'' c''
    rw [occurrences_node, hch] at hx
    rcases List.mem_cons.1 hx with rfl | hx
    · rw [child_not_closure hc' hkey] at hcl
      cases hcl
    · rcases List.mem_append.1 (hR'set.subset hx) with hx | hx
      · rcases path_write_closures path hfc hw hc'' htc'' x hx hcl with h1 | h1
        · exact Or.inl (by
            rw [occurrences_node]
            exact List.mem_cons_of_mem _ (hR'.symm.subset (List.mem_append_left _ h1)))
        · exact Or.inr h1
      · exact Or.inl (by
          rw [occurrences_node]
          exact List.mem_cons_of_mem _ (hR'.symm.subset (List.mem_append_right _ hx)))

/-- **Writing a location keeps every closure**: a closure of the state after
the write was in it before, or in the written value. -/
theorem tree_write_closures {h : Shadow} {s : State} (hfit : h.fits s = true)
    (hids : (s.frames.map (·.id)).Nodup)
    (hres : (s.storage.map fun r => (r.address, r.type)).Nodup) {l : Location} {w : Value}
    (hr : s.read? l = some w) {tw' : IdTree} {w' : Value} (hf' : tw'.fits w' = true)
    {h' : Shadow} {s' : State} (hwith : h.withTree? s l tw' = some h')
    (hwrite : s.write? l w' = some s') :
    ∀ x ∈ h'.occurrences s', IsClosure x.2 = true → x ∈ h.occurrences s ∨ x ∈ tw'.occurrences w' := by
  simp only [State.read?, bind, Option.bind_eq_some_iff] at hr
  obtain ⟨v, hv, hw⟩ := hr
  obtain ⟨t, ht, hft, -⟩ := root_read hfit hv
  obtain ⟨tw, v'', t'', -, -, hset, htset, hft'', -, -⟩ :=
    path_write l.path hft hw hf' (write?_writable hwrite)
  have hw' : h.withRootTree? s l.root t'' = some h' := by
    simpa [Shadow.withTree?, ht, Location.indicesIn, hv, htset] using hwith
  have hs' : s' = s.setRoot l.root v'' := by
    simp only [State.write?, hv, hset, bind, Option.bind_some, pure, Option.some.injEq] at hwrite
    exact hwrite.symm
  subst hs'
  obtain ⟨t₀, h₀, ht₀, -, hwr, -, -, -, R, hR, hR'⟩ := root_write hfit hids hres hv hft''
  rw [ht, Option.some.injEq] at ht₀
  subst ht₀
  rw [hw', Option.some.injEq] at hwr
  subst hwr
  intro x hx hc
  rcases List.mem_append.1 (hR'.subset hx) with hx | hx
  · rcases path_write_closures l.path hft hw hset htset x hx hc
      with h1 | h1
    · exact Or.inl (hR.symm.subset (List.mem_append_left _ h1))
    · exact Or.inr h1
  · exact Or.inl (hR.symm.subset (List.mem_append_right _ hx))

/-- Writing a value at a location through a reference the step popped,
then pushing `vals`: the old value at the location and the popped values
are kept or dropped as `hpre` and `hpost` say; the containers above the
location keep their identities and keys. -/
theorem mutate_ghost {s s₂ s₃ : State} {h h₂ : Shadow} {fr : Frame} {rest : List Frame}
    {g : ShadowFrame} {hs : List ShadowFrame} (hp : Pre σ P s h) (hsf : s.frames = fr :: rest)
    (hhf : h.frames = g :: hs) {n : Nat} (_hn : n ≤ fr.stack.length)
    {l : Location} (hheld : σ = true → l.root.isHeld = false) {w : Value}
    (hread : s.read? l = some w)
    {tw tw' : IdTree} {w' : Value}
    (htw : Shadow.tree? { h with frames := { g with stack := g.stack.drop n } :: hs } s l = some tw)
    (hfw' : tw'.fits w' = true)
    (htype : l.path = [] → ∀ a t, l.root = .global a t → w'.type = w.type)
    (hwith : Shadow.withTree? { h with frames := { g with stack := g.stack.drop n } :: hs } s l tw' =
      some h₂)
    (hwrite : State.write? { s with frames := { fr with stack := fr.stack.drop n } :: rest } l w' =
      some s₂)
    {fr₂ : Frame} {rest₂ : List Frame} (hs₂ : s₂.frames = fr₂ :: rest₂)
    {vals : List Value} {tvs : List IdTree} (htvs : IdTree.fitsAll tvs vals = true)
    (hw'p : ∀ u ∈ subvalues w', u.isRef = false)
    (hvp : ∀ v ∈ vals, ∀ u ∈ subvalues v, u.isRef = false)
    {keep dropped : KOcc}
    (hpre : (keys (IdTree.occurrencesAll (g.stack.take n) (fr.stack.take n)) ++
      keys (tw.occurrences w)).Perm (keep ++ dropped))
    (hpost : (keys (IdTree.occurrencesAll tvs vals) ++ keys (tw'.occurrences w')).Perm keep)
    (hdrop : ∀ x ∈ dropped, (x.2.type.abilities P).drop = true)
    (hclo : ∀ x ∈ IdTree.occurrencesAll tvs vals ++ tw'.occurrences w', IsClosure x.2 = true →
      x ∈ IdTree.occurrencesAll (g.stack.take n) (fr.stack.take n) ++ tw.occurrences w)
    {evs : List Event}
    (hevs : ∀ e ∈ evs, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (h3st : s₃.storage = s₂.storage) (h3he : s₃.held = s₂.held)
    (h3nf : s₃.nextFrame = s₂.nextFrame) (pc : Nat) :
    ∃ g₂ hs₂, h₂.frames = g₂ :: hs₂ ∧ fr₂.id = fr.id ∧
      StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh
        (.running { s₃ with frames := { fr₂ with stack := vals ++ fr₂.stack, pc := pc } :: rest₂ })
        (.running { h₂ with frames := { g₂ with stack := tvs ++ g₂.stack } :: hs₂ }) evs := by
  have hfit := hp.fits
  rw [shadow_fits, hhf, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hfa := frame_stack_fits hg
  obtain ⟨htake, hdropf⟩ := fitsAll_take_drop n hfa
  let h₁ : Shadow := { h with frames := { g with stack := g.stack.drop n } :: hs }
  let s₁ : State := { s with frames := { fr with stack := fr.stack.drop n } :: rest }
  have hids₁ : s₁.frames.map (·.id) = s.frames.map (·.id) := by simp [s₁, hsf]
  have hfit₁ : h₁.fits s₁ = true := by
    rw [fits_head (h' := h₁) (s' := s₁) rfl rfl h s rfl rfl rfl rfl]
    simp only [Bool.and_eq_true]
    exact ⟨⟨frame_fits_stack hg hdropf _, hrest⟩, hext⟩
  have hread₁ : s₁.read? l = some w := by
    have := read_eq (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl rfl l
    rw [this]; exact hread
  have hids : (s₁.frames.map (·.id)).Nodup := by rw [hids₁]; exact hp.inv.frameIds
  obtain ⟨tw₀, v, v'', h₂', hv, htw₀, hfw, hset, hwr, hwith', hfit₂, hfresh₂, hkey, hfrm, R, hR,
    hR'⟩ := tree_write hfit₁ hids hp.inv.resources hread₁ hfw' (write?_writable hwrite)
  have hts := tree_state (h := h₁) hids₁ rfl l tw' (root_eq (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl rfl _)
  rw [hts.1, htw, Option.some.injEq] at htw₀
  subst htw₀
  rw [hts.2, hwith, Option.some.injEq] at hwith'
  subst hwith'
  rw [hwrite, Option.some.injEq] at hwr
  subst hwr
  -- The frames after the write.
  have hids₂ := setRoot_ids s₁ l.root v''
  rw [hs₂] at hids₂
  have hfr₂ : fr₂.id = fr.id := by
    have := congrArg List.head? hids₂
    simp [s₁] at this
    exact this
  obtain ⟨g₂, hs₂', hh₂⟩ : ∃ g₂ hs₂', h₂.frames = g₂ :: hs₂' := by
    have := hfit₂
    rw [shadow_fits, hs₂, Bool.and_eq_true] at this
    cases hh : h₂.frames with
    | nil => rw [hh] at this; simp [framesFit] at this
    | cons g₂ hs₂' => exact ⟨g₂, hs₂', rfl⟩
  have hfit₂' := hfit₂
  rw [shadow_fits, hs₂, hh₂, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit₂'
  obtain ⟨⟨hg₂, hrest₂⟩, hext₂⟩ := hfit₂'
  have hsil := silent_of fun e he => (hevs e he).1
  let s₄ : State := { s₃ with frames := { fr₂ with stack := vals ++ fr₂.stack, pc := pc } :: rest₂ }
  let h₄ : Shadow := { h₂ with frames := { g₂ with stack := tvs ++ g₂.stack } :: hs₂' }
  have hck : ClosuresKept (h.occurrences s) (h₄.occurrences s₄) evs := by
    refine ClosuresKept.of_sub fun x hx hc => ?_
    have hpush := occ_push (h := h₂) (s := s₁.setRoot l.root v'') hh₂ hs₂
      (fitsAll_length htvs) s₃ h3st h3he pc h₂.fresh
    have hpop := occ_pop hhf hsf hfa n
    have htwm : ∀ y ∈ tw.occurrences w, y ∈ h₁.occurrences s₁ := by
      obtain ⟨tv, htv, -, Rr, hRr⟩ := tree_read hfit₁ hread₁
      rw [hts.1, htw, Option.some.injEq] at htv
      subst htv
      exact fun y hy => hRr.symm.subset (List.mem_append_left _ hy)
    have hold : ∀ y ∈ IdTree.occurrencesAll (g.stack.take n) (fr.stack.take n) ++
        tw.occurrences w, y ∈ h.occurrences s := by
      intro y hy
      rcases List.mem_append.1 hy with hy | hy
      · exact hpop.symm.subset (List.mem_append_left _ hy)
      · exact hpop.symm.subset (List.mem_append_right _ (htwm y hy))
    rcases List.mem_append.1 (hpush.subset hx) with hx | hx
    · exact hold x (hclo x (List.mem_append_left _ hx) hc)
    · rcases tree_write_closures hfit₁ hids hp.inv.resources hread₁ hfw'
        (by rw [hts.2]; exact hwith) hwrite x hx hc with h1 | h1
      · exact hpop.symm.subset (List.mem_append_right _ h1)
      · exact hold x (hclo x (List.mem_append_right _ h1) hc)
  refine ⟨g₂, hs₂', hh₂, hfr₂, ?_, ?_, ?_, ?_, ?_, ?_, createdPlain_of hsil.1,
    ?_, ?_, closures_silent fun e he => (hevs e he).1, hck⟩
  · show h₄.fits s₄ = true
    rw [fits_head (h' := h₄) (s' := s₄) rfl rfl h₂ (s₁.setRoot l.root v'') rfl rfl h3st h3he]
    have hfa₂ := frame_stack_fits hg₂
    simp only [Bool.and_eq_true]
    refine ⟨⟨frame_fits_stack hg₂ ?_ _, hrest₂⟩, hext₂⟩
    rw [fitsAll_append (fitsAll_length htvs)]
    simp [htvs, hfa₂]
  · show EffectK P _ (keys (h₄.occurrences s₄)) _ _ h₂.fresh
    rw [hfresh₂]
    have hpre' : (keys (h.occurrences s)).Perm (keep ++ dropped ++ R) := by
      refine (keys_perm (occ_pop hhf hsf hfa n)).trans ?_
      rw [keys_append]
      refine (List.Perm.append_left _ hR).trans ?_
      rw [← List.append_assoc]
      exact hpre.append_right R
    have hpost' : (keys (h₄.occurrences s₄)).Perm (keep ++ R) := by
      have hpush := occ_push (h := h₂) (s := s₁.setRoot l.root v'') hh₂ hs₂
        (fitsAll_length htvs) s₃ h3st h3he pc h₂.fresh
      refine (keys_perm hpush).trans ?_
      rw [keys_append]
      refine (List.Perm.append_left _ hR').trans ?_
      rw [← List.append_assoc]
      exact hpost.append_right R
    refine EffectK.mk' (keep ++ R) dropped [] [] [] hsil.2.2 (by simp [hsil.2.1])
      (hpre'.trans (by perm_parts)) (by simpa using hpost') hdrop (by simp) (by simp) (by simp)
      (Nat.le_refl _)
  · intro i w c he
    exact absurd (consumed_mem he) (by simp [hsil.2.1])
  · intro e he
    exact ⟨(hevs e he).2, signers_respected (hevs e he).1 auth⟩
  · intro x hx
    simp [hsil.2.2] at hx
  · -- The state invariant.
    have hinv := hp.inv
    have hinv₁ : StateInv σ P s₁ := frames_inv hinv hsf rfl rfl rfl rfl
    have hst₂ := setRoot_resources s₁ l.root v''
    refine ⟨?_, ?_, ?_, ?_, ?_⟩
    · show ((fr₂ :: rest₂).map (·.id)).Nodup
      simp only [List.map_cons] at hids₂ ⊢
      rw [hids₂]
      exact hinv₁.frameIds
    · intro x hx
      show x.id < s₃.nextFrame
      rw [h3nf]
      have hnf₂ : (s₁.setRoot l.root v'').nextFrame = s₁.nextFrame := by
        cases l.root <;> rfl
      rw [hnf₂]
      have hx' : x.id ∈ (fr₂ :: rest₂).map (·.id) := by
        simp only [List.map_cons, List.mem_cons, List.mem_map] at hx ⊢
        rcases hx with rfl | hx
        · exact Or.inl rfl
        · exact Or.inr ⟨x, hx, rfl⟩
      simp only [List.map_cons] at hids₂ hx'
      rw [hids₂] at hx'
      obtain ⟨y, hy, hyx⟩ := List.mem_map.1 (by simpa using hx')
      rw [← hyx]
      exact hinv₁.framesBelow y hy
    · show (s₃.storage.map _).Nodup
      rw [h3st, hst₂]; exact hinv₁.resources
    · intro r hr
      rw [h3st] at hr
      refine setRoot_storage_typed hinv₁ hv (fun a t hroot => ?_) r hr
      by_cases hp' : l.path = []
      · have hv' : v = w := by
          simp only [State.read?, hv, bind, Option.bind_some, hp', Value.get?,
            Option.some.injEq] at hread₁
          exact hread₁
        have : v'' = w' := by
          simp only [hp', Value.set?, Option.some.injEq] at hset; exact hset.symm
        rw [this, hv', htype hp' a t hroot]
      · rw [← key_type, hkey hp', key_type]
    · intro hσ v' hv'
      rw [h3he] at hv'
      have : (s₁.setRoot l.root v'').held = s₁.held := by
        have := hheld hσ
        cases hr : l.root <;> simp_all [State.setRoot, Root.isHeld]
      rw [this] at hv'
      exact hinv₁.heldDrop hσ v' hv'
  · intro d r' hm
    refine Or.inl ?_
    simp only [postFrames, framesOcc_cons, frame_occurrences, localsOcc_stack,
      occurrencesAll_append (fitsAll_length htvs), List.mem_append] at hm
    have hpre : (d, Value.ref r') ∈ framesOcc h₂.frames (s₁.setRoot l.root v'').frames := by
      rw [hh₂, hs₂, framesOcc_cons, frame_occurrences]
      simp only [List.mem_append]
      rcases hm with (hm | hm | hm) | hm
      · exact Or.inl (Or.inl hm)
      · obtain ⟨u, hu, hsub⟩ := occurrencesAll_mem htvs hm
        have := hvp u hu _ hsub
        simp [Value.isRef] at this
      · exact Or.inl (Or.inr hm)
      · exact Or.inr hm
    rcases hfrm _ hpre with hk | hk
    · obtain ⟨o, ho, h1, h2⟩ := mem_keys_iff.1 hk
      obtain ⟨o1, o2⟩ := o
      simp only at h1 h2
      subst h1
      rw [key_ref h2] at ho
      have hsub : framesOcc h₁.frames s₁.frames ⊆ framesOcc h.frames s.frames := by
        intro y hy
        simp only [h₁, s₁, framesOcc_cons, frame_occurrences, localsOcc_stack,
          List.mem_append] at hy
        rw [hhf, hsf, framesOcc_cons, frame_occurrences, stack_split hfa n]
        simp only [List.mem_append]
        rcases hy with (hy | hy) | hy
        · exact Or.inl (Or.inl hy)
        · exact Or.inl (Or.inr (Or.inr hy))
        · exact Or.inr hy
      exact hsub ho
    · obtain ⟨o, ho, h1, h2⟩ := mem_keys_iff.1 hk
      have := hw'p _ (occurrences_mem hfw' ho)
      simp only at h2
      rw [key_ref h2] at this
      simp [Value.isRef] at this
  · -- Per frame: the write changes one frame's root, to values without references.
    have hTW := tree_write_frames hfit₁ hids hread₁ hfw' (hts.2.trans hwith) hwrite
    have hkey : ∀ (occ : Occ) (d : Nat) (r' : Reference), (d, (Value.ref r').key) ∈ keys occ →
        (d, Value.ref r') ∈ occ := by
      intro occ d r' hk
      obtain ⟨o, ho, e1, e2⟩ := mem_keys_iff.1 hk
      obtain ⟨o1, o2⟩ := o
      simp only at e1 e2
      subst e1
      rw [key_ref e2] at ho
      exact ho
    have hnot : ∀ (d : Nat) (r' : Reference),
        (d, (Value.ref r').key) ∉ keys (tw'.occurrences w') := by
      intro d r' hk
      obtain ⟨o, ho, -, e2⟩ := mem_keys_iff.1 hk
      have := hw'p _ (occurrences_mem hfw' ho)
      simp only at e2
      rw [key_ref e2] at this
      simp [Value.isRef] at this
    refine refsPerFrame_of fun k g' fr' g₀ fr₀ h1 h2 h3 h4 d r' hm => Or.inl ?_
    cases k with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at h1 h2
      rw [hhf, List.getElem?_cons_zero, Option.some.injEq] at h3
      rw [hsf, List.getElem?_cons_zero, Option.some.injEq] at h4
      subst h1 h2 h3 h4
      simp only [frame_occurrences, localsOcc_stack, occurrencesAll_append (fitsAll_length htvs),
        List.mem_append] at hm
      have hm' : (d, Value.ref r') ∈ g₂.occurrences fr₂ := by
        rw [frame_occurrences, List.mem_append]
        rcases hm with hm | hm | hm
        · exact Or.inl hm
        · obtain ⟨u, hu, hsub⟩ := occurrencesAll_mem htvs hm
          have := hvp u hu _ hsub
          simp [Value.isRef] at this
        · exact Or.inr hm
      rcases hTW 0 g₂ fr₂ { g with stack := g.stack.drop n } { fr with stack := fr.stack.drop n }
          (by rw [hh₂]; rfl) (by rw [hs₂]; rfl) rfl rfl _ hm' with hk | hk
      · have := hkey _ d r' hk
        simp only [frame_occurrences, localsOcc_stack, List.mem_append] at this
        rw [frame_occurrences, stack_split hfa n, List.mem_append, List.mem_append]
        rcases this with h' | h'
        · exact Or.inl h'
        · exact Or.inr (Or.inr h')
      · exact absurd hk (hnot d r')
    | succ k =>
      simp only [List.getElem?_cons_succ] at h1 h2
      rw [hhf, List.getElem?_cons_succ] at h3
      rw [hsf, List.getElem?_cons_succ] at h4
      rcases hTW (k + 1) g' fr' g₀ fr₀ (by rw [hh₂, List.getElem?_cons_succ]; exact h1)
          (by rw [hs₂, List.getElem?_cons_succ]; exact h2)
          (by simp only [h₁, List.getElem?_cons_succ]; exact h3)
          (by simp only [s₁, List.getElem?_cons_succ]; exact h4) _ hm with hk | hk
      · exact hkey _ d r' hk
      · exact absurd hk (hnot d r')

/-- The tree at a location the state reads, in the shadow with the current
segment's top `n` slots popped. -/
theorem tree_at {s : State} {h : Shadow} {fr : Frame} {rest : List Frame} {g : ShadowFrame}
    {hs : List ShadowFrame} (hp : Pre σ P s h) (hsf : s.frames = fr :: rest)
    (hhf : h.frames = g :: hs) (n : Nat) {l : Location} {w : Value} (hread : s.read? l = some w) :
    ∃ tw, Shadow.tree? { h with frames := { g with stack := g.stack.drop n } :: hs } s l = some tw ∧
      tw.fits w = true := by
  have hfit := hp.fits
  rw [shadow_fits, hhf, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  obtain ⟨-, hdropf⟩ := fitsAll_take_drop n (frame_stack_fits hg)
  let h₁ : Shadow := { h with frames := { g with stack := g.stack.drop n } :: hs }
  let s₁ : State := { s with frames := { fr with stack := fr.stack.drop n } :: rest }
  have hfit₁ : h₁.fits s₁ = true := by
    rw [fits_head (h' := h₁) (s' := s₁) rfl rfl h s rfl rfl rfl rfl]
    simp only [Bool.and_eq_true]
    exact ⟨⟨frame_fits_stack hg hdropf _, hrest⟩, hext⟩
  have hread₁ : s₁.read? l = some w := by
    have := read_eq (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl rfl l
    rw [this]; exact hread
  obtain ⟨tw, htw, hfw, -⟩ := tree_read hfit₁ hread₁
  refine ⟨tw, ?_, hfw⟩
  rw [← (tree_state (h := h₁) (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl l tw
    (root_eq (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl rfl _)).1]
  exact htw

/-- Writing a location the state reads, after popping the current segment's
top `n` slots: the state and the shadow can be written. -/
theorem write_exists {s : State} {h : Shadow} {fr : Frame} {rest : List Frame} {g : ShadowFrame}
    {hs : List ShadowFrame} (hp : Pre σ P s h) (hsf : s.frames = fr :: rest)
    (hhf : h.frames = g :: hs) (n : Nat) {l : Location} {w : Value} (hread : s.read? l = some w)
    {tw' : IdTree} {w' : Value} (hfw' : tw'.fits w' = true)
    (hwr : l.path.all PathStep.writable = true) :
    ∃ h₂ s₂ fr₂ rest₂,
      Shadow.withTree? { h with frames := { g with stack := g.stack.drop n } :: hs } s l tw' =
        some h₂ ∧
      State.write? { s with frames := { fr with stack := fr.stack.drop n } :: rest } l w' = some s₂ ∧
      s₂.frames = fr₂ :: rest₂ := by
  have hfit := hp.fits
  rw [shadow_fits, hhf, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  obtain ⟨-, hdropf⟩ := fitsAll_take_drop n (frame_stack_fits hg)
  let h₁ : Shadow := { h with frames := { g with stack := g.stack.drop n } :: hs }
  let s₁ : State := { s with frames := { fr with stack := fr.stack.drop n } :: rest }
  have hids₁ : s₁.frames.map (·.id) = s.frames.map (·.id) := by simp [s₁, hsf]
  have hfit₁ : h₁.fits s₁ = true := by
    rw [fits_head (h' := h₁) (s' := s₁) rfl rfl h s rfl rfl rfl rfl]
    simp only [Bool.and_eq_true]
    exact ⟨⟨frame_fits_stack hg hdropf _, hrest⟩, hext⟩
  have hread₁ : s₁.read? l = some w := by
    have := read_eq (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl rfl l
    rw [this]; exact hread
  have hids : (s₁.frames.map (·.id)).Nodup := by rw [hids₁]; exact hp.inv.frameIds
  obtain ⟨-, -, v'', h₂, -, -, -, -, hwr, hwith, -⟩ :=
    tree_write hfit₁ hids hp.inv.resources hread₁ hfw' hwr
  have hfr := setRoot_ids s₁ l.root v''
  obtain ⟨fr₂, rest₂, hs₂⟩ : ∃ fr₂ rest₂, (s₁.setRoot l.root v'').frames = fr₂ :: rest₂ := by
    cases hc : (s₁.setRoot l.root v'').frames with
    | nil => rw [hc] at hfr; simp [s₁] at hfr
    | cons fr₂ rest₂ => exact ⟨fr₂, rest₂, rfl⟩
  refine ⟨h₂, _, fr₂, rest₂, ?_, hwr, hs₂⟩
  rw [← (tree_state (h := h₁) (s' := s₁) (s := s) hids₁ rfl l tw'
    (root_eq (s' := s₁) (s := s) (by simp [s₁, hsf]) rfl rfl _)).2]
  exact hwith

end Write

/-! ## Global storage -/

section Storage

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}

theorem map_eraseIdx' {α β : Type} (f : α → β) : ∀ (l : List α) (k : Nat),
    (l.eraseIdx k).map f = (l.map f).eraseIdx k
  | [], _ => rfl
  | _ :: _, 0 => rfl
  | a :: l, k + 1 => by simp [map_eraseIdx' f l k]

theorem occurrencesAll_eraseIdx : ∀ {ts : List IdTree} {vs : List Value} {k : Nat} {t : IdTree}
    {v : Value}, ts[k]? = some t → vs[k]? = some v →
      (IdTree.occurrencesAll ts vs).Perm
        (t.occurrences v ++ IdTree.occurrencesAll (ts.eraseIdx k) (vs.eraseIdx k))
  | [], _, _, _, _, ht, _ => by simp at ht
  | _ :: _, [], _, _, _, _, hv => by simp at hv
  | t' :: ts, v' :: vs, 0, t, v, ht, hv => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at ht hv
    subst ht hv
    simp [IdTree.occurrencesAll]
  | t' :: ts, v' :: vs, k + 1, t, v, ht, hv => by
    simp only [List.getElem?_cons_succ] at ht hv
    simp only [IdTree.occurrencesAll, List.eraseIdx_cons_succ]
    refine (List.Perm.append_left _ (occurrencesAll_eraseIdx ht hv)).trans ?_
    perm_parts

theorem fitsAll_eraseIdx : ∀ {ts : List IdTree} {vs : List Value}, IdTree.fitsAll ts vs = true →
    ∀ k, IdTree.fitsAll (ts.eraseIdx k) (vs.eraseIdx k) = true
  | [], [], _, _ => by simp [IdTree.fitsAll]
  | _ :: _, _ :: _, h, 0 => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simpa using h.2
  | _ :: ts, _ :: vs, h, k + 1 => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp [IdTree.fitsAll, h.1, fitsAll_eraseIdx h.2 k]
  | [], _ :: _, h, _ => by simp [IdTree.fitsAll] at h
  | _ :: _, [], h, _ => by simp [IdTree.fitsAll] at h

/-- Removing the only entry with a key removes its position. -/
theorem filter_eq_eraseIdx {α β : Type} [DecidableEq β] {key : α → β} {p : α → Bool} :
    ∀ {l : List α} {k : Nat} {x : α}, (l.map key).Nodup → l[k]? = some x →
      (∀ y ∈ l, p y = false ↔ key y = key x) → l.filter p = l.eraseIdx k
  | [], _, _, _, hx, _ => by simp at hx
  | y :: l, 0, x, hnd, hx, hp => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hx
    subst hx
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at hnd
    have hpy : p y = false := (hp y (List.mem_cons_self ..)).2 rfl
    simp only [List.filter_cons, hpy, Bool.false_eq_true, ↓reduceIte, List.eraseIdx_cons_zero]
    rw [List.filter_eq_self]
    intro z hz
    cases hpz : p z
    · exact absurd ⟨z, hz, (hp z (List.mem_cons_of_mem _ hz)).1 hpz⟩ hnd.1
    · rfl
  | y :: l, k + 1, x, hnd, hx, hp => by
    simp only [List.getElem?_cons_succ] at hx
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at hnd
    have hpy : p y = true := by
      cases hpy : p y
      · exact absurd ⟨x, List.mem_of_getElem? hx,
          ((hp y (List.mem_cons_self ..)).1 hpy).symm⟩ hnd.1
      · rfl
    simp only [List.filter_cons, hpy, ↓reduceIte, List.eraseIdx_cons_succ, List.cons.injEq,
      true_and]
    exact filter_eq_eraseIdx hnd.2 hx fun z hz => hp z (List.mem_cons_of_mem _ hz)

theorem remove_eq_eraseIdx {s : State} (hinv : StateInv σ P s) {a : Address} {sid : StructId}
    {k : Nat} {res : Resource} (hres : s.storage[k]? = some res) (ha : res.address = a)
    (ht : res.type = sid) : s.storage.remove a sid = s.storage.eraseIdx k := by
  unfold GlobalStorage.remove
  refine filter_eq_eraseIdx (key := fun r => (r.address, r.type)) hinv.resources hres ?_
  intro y _
  simp [ha, ht]

theorem address_drop {a : Address} : ((Value.address a).abilities P).drop = true := by
  simp [Value.abilities, RuntimeType.abilities, AbilitySet.primitives]

theorem address_occ {t : IdTree} {a : Address} (ht : t.fits (.address a) = true) :
    t.occurrences (.address a) = [(t.id, .address a)] := by
  obtain ⟨i, cs⟩ := t
  simp only [IdTree.fits, Value.children] at ht
  cases cs with
  | nil => simp [IdTree.occurrencesAll, IdTree.id]
  | cons => simp [IdTree.fitsAll] at ht

/-- `MoveFrom`: the address is discarded, and the resource's identities move
from global storage onto the segment. -/
theorem moveFrom_ghost {s s₂ : State} {h : Shadow} {fr : Frame} {rest : List Frame}
    {g : ShadowFrame} {hs : List ShadowFrame} (hp : Pre σ P s h) (hsf : s.frames = fr :: rest)
    (hhf : h.frames = g :: hs) {a : Address} {sid : StructId} {st : List Value}
    (hst : fr.stack = .address a :: st) {v : Value} (hget : s.storage.get? a sid = some v)
    (h2st : s₂.storage = s.storage.remove a sid) (h2he : s₂.held = s.held)
    (h2nf : s₂.nextFrame = s.nextFrame) {evs : List Event}
    (hevs : ∀ e ∈ evs, e.Silent = true ∧ Encapsulation.structsRespected π A e = true) :
    v.type = .struct sid ∧ ∃ k t, Shadow.resourcePos s a sid = some k ∧ h.storage[k]? = some t ∧
      StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh
        (.running { s₂ with frames := { fr with stack := v :: st, pc := fr.pc + 1 } :: rest })
        (.running { h with frames := { g with stack := t :: g.stack.drop 1 } :: hs,
                           storage := h.storage.eraseIdx k }) evs := by
  have hfit := hp.fits
  rw [shadow_fits, hhf, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hext' := hext
  simp only [extFit, Bool.and_eq_true] at hext'
  have hfa := frame_stack_fits hg
  rw [hst] at hfa
  obtain ⟨ta, ts, hgst, hta, hts⟩ := fitsAll_cons_right hfa
  obtain ⟨k, res, hk, hres, rfl, hra, hrt⟩ := storage_get hget
  have hvk : (s.storage.map (·.value))[k]? = some res.value := by simp [hres]
  obtain ⟨t, htk, ht⟩ := fitsAll_get hext'.1 hvk
  have htype : res.value.type = .struct sid := by
    rw [← hrt]; exact hp.inv.typed res (List.mem_of_getElem? hres)
  have hremove := remove_eq_eraseIdx hp.inv hres hra hrt
  have hsil := silent_of fun e he => (hevs e he).1
  have hvp : ∀ u ∈ subvalues res.value, u.isRef = false :=
    hp.refsTop.storage res (List.mem_of_getElem? hres)
  have hperm : (h.occurrences s).Perm ((ta.id, Value.address a) :: ShadowPost.occurrences
      (.running { h with frames := { g with stack := t :: g.stack.drop 1 } :: hs,
                         storage := h.storage.eraseIdx k })
      (.running { s₂ with frames := { fr with stack := res.value :: st, pc := fr.pc + 1 } :: rest })) := by
    have hstore := occurrencesAll_eraseIdx htk hvk
    rw [shadow_occurrences, hhf, hsf]
    simp only [ShadowPost.occurrences, shadow_occurrences, framesOcc_cons, frame_occurrences,
      extOcc, h2st, h2he, hremove, map_eraseIdx', localsOcc_stack]
    rw [hgst, hst]
    simp only [IdTree.occurrencesAll, List.drop_succ_cons, List.drop_zero, address_occ hta]
    rw [List.perm_iff_count]; intro x
    have := List.perm_iff_count.1 hstore x
    simp only [List.count_append, List.count_cons, List.cons_append,
      List.nil_append] at this ⊢
    omega
  refine ⟨htype, k, t, hk, htk, ?_, ?_, ?_, ?_, ?_, ?_, createdPlain_of hsil.1,
    ?_, ?_, closures_silent fun e he => (hevs e he).1,
    ClosuresKept.of_sub fun x hx _ => hperm.symm.subset (List.mem_cons_of_mem _ hx)⟩
  · simp only [ShadowPost.fits, shadow_fits, framesFit_cons, extFit, h2st, h2he, hremove,
      map_eraseIdx', Bool.and_eq_true]
    refine ⟨⟨frame_fits_stack hg ?_ _, hrest⟩, fitsAll_eraseIdx hext'.1 k, hext'.2⟩
    simp [IdTree.fitsAll, ht, hgst, hts]
  · show EffectK P _ _ _ _ h.fresh
    have hstore := occurrencesAll_eraseIdx htk hvk
    refine EffectK.mk' (keys (localsOcc g fr ++ IdTree.occurrencesAll ts st ++
        framesOcc hs rest ++ t.occurrences res.value ++
        IdTree.occurrencesAll (h.storage.eraseIdx k) ((s.storage.map (·.value)).eraseIdx k) ++
        IdTree.occurrencesAll h.held s.held)) [(ta.id, (Value.address a).key)] [] [] []
      hsil.2.2 (by simp [hsil.2.1]) ?_ ?_ (by simp [Value.type, RuntimeType.abilities,
        AbilitySet.primitives]) (by simp) (by simp) (by simp) (Nat.le_refl _)
    · rw [shadow_occurrences, hhf, hsf, framesOcc_cons, frame_occurrences, extOcc, hgst, hst]
      simp only [IdTree.occurrencesAll, address_occ hta]
      rw [List.perm_iff_count]; intro x
      have := List.perm_iff_count.1 (keys_perm hstore) x
      simp only [keys_append, keys_cons, List.count_append, List.count_cons,
        List.count_nil, List.cons_append, List.nil_append, List.append_nil] at this ⊢
      omega
    · simp only [ShadowPost.occurrences, shadow_occurrences, framesOcc_cons, frame_occurrences,
        extOcc, h2st, h2he, hremove, map_eraseIdx', IdTree.occurrencesAll, localsOcc]
      rw [show g.stack.drop 1 = ts by rw [hgst]; rfl]
      simp only [keys_append]
      perm_parts
  · intro i w c he
    exact absurd (consumed_mem he) (by simp [hsil.2.1])
  · intro e he
    exact ⟨(hevs e he).2, signers_respected (hevs e he).1 auth⟩
  · intro x hx
    simp [hsil.2.2] at hx
  · have hinv := hp.inv
    refine ⟨?_, ?_, ?_, ?_, ?_⟩
    · have := hinv.frameIds; rw [hsf] at this; simpa using this
    · intro x hx
      show x.id < s₂.nextFrame
      rw [h2nf]
      simp only [List.mem_cons] at hx
      rcases hx with rfl | hx
      · exact hinv.framesBelow fr (by rw [hsf]; exact List.mem_cons_self ..)
      · exact hinv.framesBelow x (by rw [hsf]; exact List.mem_cons_of_mem _ hx)
    · show (s₂.storage.map _).Nodup
      rw [h2st, hremove]
      exact hinv.resources.sublist ((List.eraseIdx_sublist _ k).map _)
    · intro r hr
      rw [show ({ s₂ with frames := { fr with stack := res.value :: st, pc := fr.pc + 1 } :: rest } :
        State).storage = s₂.storage from rfl, h2st, hremove] at hr
      exact hinv.typed r ((List.eraseIdx_sublist _ k).subset hr)
    · intro hσ v' hv'
      exact hinv.heldDrop hσ v' (by rw [← h2he]; exact hv')
  · intro d r' hm
    refine Or.inl ?_
    simp only [postFrames, framesOcc_cons, frame_occurrences, localsOcc_stack,
      IdTree.occurrencesAll, List.mem_append] at hm
    rw [hhf, hsf, framesOcc_cons, frame_occurrences, hgst, hst]
    rw [hgst] at hm
    simp only [IdTree.occurrencesAll, List.mem_append, List.drop_succ_cons, List.drop_zero] at hm ⊢
    rcases hm with (hm | hm | hm) | hm
    · exact Or.inl (Or.inl hm)
    · have := hvp _ (occurrences_mem ht hm)
      simp [Value.isRef] at this
    · exact Or.inl (Or.inr (Or.inr hm))
    · exact Or.inr hm
  · rw [hhf, hsf]
    refine refsPerFrame_top rfl rfl fun d r' hm => Or.inl ?_
    simp only [frame_occurrences, localsOcc_stack, IdTree.occurrencesAll, List.mem_append] at hm
    rw [frame_occurrences, hgst, hst]
    rw [hgst] at hm
    simp only [IdTree.occurrencesAll, List.mem_append, List.drop_succ_cons, List.drop_zero] at hm ⊢
    rcases hm with hm | hm | hm
    · exact Or.inl hm
    · have := hvp _ (occurrences_mem ht hm)
      simp [Value.isRef] at this
    · exact Or.inr (Or.inr hm)

/-- `MoveTo`: the signer reference is discarded, and the value's identities
move from the segment into global storage. -/
theorem moveTo_ghost {s s₂ : State} {h : Shadow} {fr : Frame} {rest : List Frame}
    {g : ShadowFrame} {hs : List ShadowFrame} (hp : Pre σ P s h) (hsf : s.frames = fr :: rest)
    (hhf : h.frames = g :: hs) {v : Value} {r : Reference} {st : List Value}
    (hst : fr.stack = v :: .ref r :: st) {a : Address} {sid : StructId}
    (hget : s.storage.get? a sid = none) (htype : v.type = .struct sid)
    (h2st : s₂.storage = s.storage.insert a sid v) (h2he : s₂.held = s.held)
    (h2nf : s₂.nextFrame = s.nextFrame) {evs : List Event}
    (hevs : ∀ e ∈ evs, e.Silent = true ∧ Encapsulation.structsRespected π A e = true) :
    ∃ tv tr ts, g.stack = tv :: tr :: ts ∧
      StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh
        (.running { s₂ with frames := { fr with stack := st, pc := fr.pc + 1 } :: rest })
        (.running { h with frames := { g with stack := g.stack.drop 2 } :: hs,
                           storage := h.storage ++ [tv] }) evs := by
  have hfit := hp.fits
  rw [shadow_fits, hhf, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hext' := hext
  simp only [extFit, Bool.and_eq_true] at hext'
  have hfa := frame_stack_fits hg
  rw [hst] at hfa
  obtain ⟨tv, ts₁, hgst, htv, hts₁⟩ := fitsAll_cons_right hfa
  obtain ⟨tr, ts, rfl, htr, hts⟩ := fitsAll_cons_right hts₁
  have hsil := silent_of fun e he => (hevs e he).1
  have hperm : (h.occurrences s).Perm ((tr.id, Value.ref r) :: ShadowPost.occurrences
      (.running { h with frames := { g with stack := g.stack.drop 2 } :: hs,
                         storage := h.storage ++ [tv] })
      (.running { s₂ with frames := { fr with stack := st, pc := fr.pc + 1 } :: rest })) := by
    rw [shadow_occurrences, hhf, hsf]
    simp only [ShadowPost.occurrences, shadow_occurrences, framesOcc_cons, frame_occurrences,
      extOcc, h2st, h2he, GlobalStorage.insert, List.map_append, List.map_cons, List.map_nil,
      localsOcc_stack]
    rw [occurrencesAll_append (fitsAll_length hext'.1), hgst, hst]
    simp only [IdTree.occurrencesAll, List.drop_succ_cons, List.drop_zero, List.append_nil,
      ref_occ htr]
    perm_parts
  refine ⟨tv, tr, ts, hgst, ?_, ?_, ?_, ?_, ?_, ?_, createdPlain_of hsil.1,
    ?_, ?_, closures_silent fun e he => (hevs e he).1,
    ClosuresKept.of_sub fun x hx _ => hperm.symm.subset (List.mem_cons_of_mem _ hx)⟩
  · simp only [ShadowPost.fits, shadow_fits, framesFit_cons, extFit, h2st, h2he,
      GlobalStorage.insert, Bool.and_eq_true, List.map_append, List.map_cons, List.map_nil]
    refine ⟨⟨frame_fits_stack hg ?_ _, hrest⟩, ?_, hext'.2⟩
    · rw [hgst]; exact hts
    · rw [fitsAll_append (fitsAll_length hext'.1)]
      simp [hext'.1, IdTree.fitsAll, htv]
  · show EffectK P _ _ _ _ h.fresh
    refine EffectK.mk' (keys (localsOcc g fr ++ IdTree.occurrencesAll ts st ++
        framesOcc hs rest ++ IdTree.occurrencesAll h.storage (s.storage.map (·.value)) ++
        tv.occurrences v ++ IdTree.occurrencesAll h.held s.held)) [(tr.id, (Value.ref r).key)] []
        [] [] hsil.2.2 (by simp [hsil.2.1]) ?_ ?_
      (by simp [Value.type, RuntimeType.abilities]) (by simp) (by simp) (by simp) (Nat.le_refl _)
    · rw [shadow_occurrences, hhf, hsf, framesOcc_cons, frame_occurrences, extOcc, hgst, hst]
      simp only [IdTree.occurrencesAll, ref_occ htr]
      simp only [keys_append, keys_cons, List.cons_append, List.nil_append,
        List.append_nil]
      perm_parts
    · simp only [ShadowPost.occurrences, shadow_occurrences, framesOcc_cons, frame_occurrences,
        extOcc, h2st, h2he, GlobalStorage.insert, List.map_append, List.map_cons, List.map_nil,
        localsOcc]
      rw [occurrencesAll_append (fitsAll_length hext'.1), hgst]
      simp only [IdTree.occurrencesAll, List.drop_succ_cons, List.drop_zero, List.append_nil]
      simp only [keys_append]
      perm_parts
  · intro i w c he
    exact absurd (consumed_mem he) (by simp [hsil.2.1])
  · intro e he
    exact ⟨(hevs e he).2, signers_respected (hevs e he).1 auth⟩
  · intro x hx
    simp [hsil.2.2] at hx
  · have hinv := hp.inv
    refine ⟨?_, ?_, ?_, ?_, ?_⟩
    · have := hinv.frameIds; rw [hsf] at this; simpa using this
    · intro x hx
      show x.id < s₂.nextFrame
      rw [h2nf]
      simp only [List.mem_cons] at hx
      rcases hx with rfl | hx
      · exact hinv.framesBelow fr (by rw [hsf]; exact List.mem_cons_self ..)
      · exact hinv.framesBelow x (by rw [hsf]; exact List.mem_cons_of_mem _ hx)
    · show (s₂.storage.map _).Nodup
      rw [h2st]
      simp only [GlobalStorage.insert, List.map_append, List.map_cons, List.map_nil]
      refine List.nodup_append.2 ⟨hinv.resources, by simp, ?_⟩
      intro x hx y hy hxy
      simp only [List.mem_singleton] at hy
      subst hy hxy
      obtain ⟨res, hres, hk⟩ := List.mem_map.1 hx
      simp only [GlobalStorage.get?, Option.map_eq_none_iff, List.find?_eq_none] at hget
      have := hget res hres
      simp only [Prod.mk.injEq] at hk
      simp [hk.1, hk.2] at this
    · intro r' hr
      rw [show ({ s₂ with frames := { fr with stack := st, pc := fr.pc + 1 } :: rest } :
        State).storage = s₂.storage from rfl, h2st] at hr
      simp only [GlobalStorage.insert, List.mem_append, List.mem_singleton] at hr
      rcases hr with hr | rfl
      · exact hinv.typed r' hr
      · exact htype
    · intro hσ v' hv'
      exact hinv.heldDrop hσ v' (by rw [← h2he]; exact hv')
  · intro d r' hm
    refine Or.inl ?_
    simp only [postFrames, framesOcc_cons, frame_occurrences, localsOcc_stack,
      List.mem_append] at hm
    rw [hhf, hsf, framesOcc_cons, frame_occurrences, hgst, hst]
    rw [hgst] at hm
    simp only [IdTree.occurrencesAll, List.mem_append, List.drop_succ_cons, List.drop_zero] at hm ⊢
    rcases hm with (hm | hm) | hm
    · exact Or.inl (Or.inl hm)
    · exact Or.inl (Or.inr (Or.inr (Or.inr hm)))
    · exact Or.inr hm
  · rw [hhf, hsf]
    refine refsPerFrame_top rfl rfl fun d r' hm => Or.inl ?_
    simp only [frame_occurrences, localsOcc_stack, List.mem_append] at hm
    rw [frame_occurrences, hgst, hst]
    rw [hgst] at hm
    simp only [IdTree.occurrencesAll, List.mem_append, List.drop_succ_cons, List.drop_zero] at hm ⊢
    rcases hm with hm | hm
    · exact Or.inl hm
    · exact Or.inr (Or.inr (Or.inr hm))

end Storage

end MoveBytecode.Proofs
