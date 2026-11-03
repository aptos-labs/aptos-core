-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Typing

/-!
# Linking: K1 and K2 from the model

UNTRUSTED (checked by the kernel). In a program the model accepts, on the
supported subset, every function handle of every module resolves to a
definition visible from that module, whose declared types are the handle's
(`handle_resolves_all`), and every struct handle resolves to a definition of
which it is a conservative view (`struct_handle_resolves`), hence
`LinkedProperties` for the model.

A handle of the module's own function or struct resolves through the
duplication checker: every such handle has a definition, and definitions,
handles and identifiers have no duplicates, so the definition's name is
unique. A handle of an imported function resolves through the dependency
check: the owner is among the other modules, the function it found is
callable, and its declared types compare equal, struct types by module id and
name, which is how the specification names them. A handle of an imported
struct resolves the same way, and the dependency check compared its
abilities and type parameters with the definition's. Module ids are distinct
(`DistinctModuleIds`), so resolution by id finds that owner.

The same facts give `StructsAgree`, which the type certificate needs
(`structsAgree`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Lists without duplicates -/

theorem hasDuplicate_go {α : Type} [BEq α] [LawfulBEq α] :
    ∀ (xs seen : List α), hasDuplicate.go xs seen = false → xs.Nodup ∧ ∀ x ∈ xs, x ∉ seen
  | [], _, _ => by simp
  | x :: rest, seen, h => by
    simp only [hasDuplicate.go] at h
    split at h
    · simp at h
    · next hx =>
      obtain ⟨hnd, hns⟩ := hasDuplicate_go rest (x :: seen) h
      simp only [List.contains_iff_mem] at hx
      refine ⟨List.nodup_cons.2 ⟨fun hm => (hns x hm) (List.mem_cons_self ..), hnd⟩, ?_⟩
      intro y hy
      rcases List.mem_cons.1 hy with rfl | hy
      · simpa using hx
      · exact fun hs => hns y hy (List.mem_cons_of_mem _ hs)

theorem noDuplicate_ok {α : Type} [BEq α] [LawfulBEq α] {xs : List α}
    (h : noDuplicate xs = .ok ()) : xs.Nodup := by
  unfold noDuplicate at h
  split at h
  · simp [reject] at h
  · next hd => exact (hasDuplicate_go xs [] (by simpa [hasDuplicate] using hd)).1

theorem nodup_index {α : Type} : ∀ {l : List α}, l.Nodup → ∀ {i j : Nat} {x : α},
    l[i]? = some x → l[j]? = some x → i = j
  | [], _, i, j, x, hi, _ => by simp at hi
  | a :: t, hl, i, j, x, hi, hj => by
    obtain ⟨hna, hnt⟩ := List.nodup_cons.1 hl
    cases i with
    | zero =>
      cases j with
      | zero => rfl
      | succ j =>
        simp at hi hj
        subst hi
        exact absurd (List.mem_of_getElem? hj) hna
    | succ i =>
      cases j with
      | zero =>
        simp at hi hj
        subst hj
        exact absurd (List.mem_of_getElem? hi) hna
      | succ j =>
        simp at hi hj
        rw [nodup_index hnt hi hj]

theorem nodup_map_index {α β : Type} {l : List α} {f : α → β} (hl : (l.map f).Nodup) {i j : Nat}
    {x y : α} (hi : l[i]? = some x) (hj : l[j]? = some y) (hf : f x = f y) : i = j :=
  nodup_index hl (x := f x) (by simp [hi]) (by simp [hj, hf])

theorem eq_singleton_of_nodup {l : List Nat} {i : Nat} (hl : l.Nodup) (hi : i ∈ l)
    (hall : ∀ k ∈ l, k = i) : l = [i] := by
  cases l with
  | nil => simp at hi
  | cons a t =>
    have ha := hall a (List.mem_cons_self ..)
    subst ha
    cases t with
    | nil => rfl
    | cons b t' =>
      have hb := hall b (by simp)
      subst hb
      exact absurd (List.mem_cons_self ..) (List.nodup_cons.1 hl).1

/-- `uniqueIdx` finds the only index whose element satisfies `p`. -/
theorem uniqueIdx_eq {α : Type} {a : Array α} {p : α → Bool} {i : Nat} {x : α}
    (hi : a[i]? = some x) (hp : p x = true)
    (huniq : ∀ j y, a[j]? = some y → p y = true → j = i) : Program.uniqueIdx a p = some i := by
  have hlt := lt_of_getElem? hi
  unfold Program.uniqueIdx
  generalize hl : List.filter _ (List.range a.size) = l
  have hmem : ∀ k, k ∈ l ↔ k < a.size ∧ ∃ y, a[k]? = some y ∧ p y = true := by
    intro k
    rw [← hl, List.mem_filter, List.mem_range]
    constructor
    · rintro ⟨hk, hpk⟩
      refine ⟨hk, ?_⟩
      split at hpk
      · next y hy => exact ⟨y, hy, hpk⟩
      · simp at hpk
    · rintro ⟨hk, y, hy, hpy⟩
      refine ⟨hk, ?_⟩
      split
      · next z hz => rw [hy, Option.some.injEq] at hz; subst hz; exact hpy
      · next hn => rw [hy] at hn; simp at hn
  have hnd : l.Nodup := by rw [← hl]; exact (List.nodup_range).filter _
  have : l = [i] := eq_singleton_of_nodup hnd ((hmem i).2 ⟨hlt, x, hi, hp⟩)
    (fun k hk => by obtain ⟨_, y, hy, hpy⟩ := (hmem k).1 hk; exact huniq k y hy hpy)
  rw [this]

/-! ## The duplication checker -/

/-- What the duplication checker establishes about a module's functions. -/
structure FunctionsUnique (m : CompiledModule) : Prop where
  identifiers : m.identifiers.toList.Nodup
  addresses : m.addressIdentifiers.toList.Nodup
  moduleHandles : m.moduleHandles.toList.Nodup
  handles : (m.functionHandles.toList.map (fun h => (h.module, h.name))).Nodup
  defs : (m.functionDefs.toList.map (·.function)).Nodup
  ownDefs : ∀ fd ∈ m.functionDefs.toList, ∃ fh, m.functionHandles[fd.function]? = some fh ∧
    fh.module = m.selfModuleHandleIdx
  implemented : ∀ i fh, m.functionHandles[i]? = some fh → fh.module = m.selfModuleHandleIdx →
    ∃ fd ∈ m.functionDefs.toList, fd.function = i

theorem verifyDuplication_ok {m : CompiledModule} (h : verifyDuplication m = .ok ()) :
    FunctionsUnique m := by
  unfold verifyDuplication at h
  obtain ⟨hid, h⟩ := seq_ok2 h
  obtain ⟨haddr, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨hmh, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨hfh, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨h, -⟩ := seq_ok2 h
  unfold checkFunctionDefinitions at h
  obtain ⟨hdefs, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨hown, h⟩ := seq_ok2 h
  refine ⟨noDuplicate_ok hid, noDuplicate_ok haddr, noDuplicate_ok hmh, noDuplicate_ok hfh,
    noDuplicate_ok hdefs, fun fd hfd => ?_, fun i fh hi hself => ?_⟩
  · have := forM_ok hown fd hfd
    simp only [bind_ok, lookup_ok] at this
    obtain ⟨fh, hfh', hm⟩ := this
    refine ⟨fh, hfh', ?_⟩
    split at hm
    · simp [reject] at hm
    · next hne => simpa using hne
  · have hmem : (fh, i) ∈ m.functionHandles.toList.zipIdx := by
      rw [List.mem_zipIdx_iff_getElem?]
      simpa using hi
    have := forM_ok h (fh, i) hmem
    simp only at this
    split at this
    · simp [reject] at this
    · next hc =>
      simp only [hself, beq_self_eq_true, Bool.true_and, Bool.not_eq_true',
        Bool.not_eq_false, List.contains_iff_mem, List.mem_map] at hc
      obtain ⟨fd, hfd, rfl⟩ := hc
      exact ⟨fd, hfd, rfl⟩

/-- What the duplication checker establishes about a module's structs. -/
structure StructsUnique (m : CompiledModule) : Prop where
  handles : (m.structHandles.toList.map (fun h => (h.module, h.name))).Nodup
  defs : (m.structDefs.toList.map (·.structHandle)).Nodup
  ownDefs : ∀ sd ∈ m.structDefs.toList, ∃ sh, m.structHandles[sd.structHandle]? = some sh ∧
    sh.module = m.selfModuleHandleIdx
  implemented : ∀ i sh, m.structHandles[i]? = some sh → sh.module = m.selfModuleHandleIdx →
    ∃ sd ∈ m.structDefs.toList, sd.structHandle = i

theorem verifyDuplication_structs {m : CompiledModule} (h : verifyDuplication m = .ok ()) :
    StructsUnique m := by
  unfold verifyDuplication at h
  iterate 6 obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨hsh, h⟩ := seq_ok2 h
  iterate 9 obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨h, -⟩ := seq_ok2 h
  unfold checkStructDefinitions at h
  obtain ⟨hdefs, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨hown, h⟩ := seq_ok2 h
  refine ⟨noDuplicate_ok hsh, noDuplicate_ok hdefs, fun sd hsd => ?_, fun i sh hi hself => ?_⟩
  · have := forM_ok hown sd hsd
    simp only [bind_ok, lookup_ok] at this
    obtain ⟨sh, hsh', hm⟩ := this
    refine ⟨sh, hsh', ?_⟩
    split at hm
    · simp [reject] at hm
    · next hne => simpa using hne
  · have hmem : (sh, i) ∈ m.structHandles.toList.zipIdx := by
      rw [List.mem_zipIdx_iff_getElem?]
      simpa using hi
    have := forM_ok h (sh, i) hmem
    simp only at this
    split at this
    · simp [reject] at this
    · next hc =>
      simp only [hself, beq_self_eq_true, Bool.true_and, Bool.not_eq_true',
        Bool.not_eq_false, List.contains_iff_mem, List.mem_map] at hc
      obtain ⟨sd, hsd, rfl⟩ := hc
      exact ⟨sd, hsd, rfl⟩

/-! ## The bounds checker on handles -/

/-- What the bounds checker establishes about a module's handles. -/
structure HandlesInBounds (m : CompiledModule) : Prop where
  moduleHandles : ∀ mh ∈ m.moduleHandles.toList,
    mh.address < m.addressIdentifiers.size ∧ mh.name < m.identifiers.size
  self : m.selfModuleHandleIdx < m.moduleHandles.size
  structHandles : ∀ sh ∈ m.structHandles.toList,
    sh.module < m.moduleHandles.size ∧ sh.name < m.identifiers.size
  functionHandles : ∀ fh ∈ m.functionHandles.toList,
    fh.module < m.moduleHandles.size ∧ fh.name < m.identifiers.size ∧
      fh.parameters < m.signatures.size ∧ fh.returns < m.signatures.size
  friends : ∀ mh ∈ m.friendDecls.toList,
    mh.address < m.addressIdentifiers.size ∧ mh.name < m.identifiers.size

theorem verifyBounds_handles {m : CompiledModule} (hb : verifyBounds m = .ok ()) :
    HandlesInBounds m := by
  unfold verifyBounds at hb
  simp only at hb
  split at hb
  · simp [reject, bind, Except.bind] at hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨hmh, hb⟩ := seq_ok2 hb
  obtain ⟨hself, hb⟩ := seq_ok2 hb
  obtain ⟨hsh, hb⟩ := seq_ok2 hb
  obtain ⟨hfh, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨hfr, -⟩ := seq_ok2 hb
  have modH : ∀ mh, Bounds.checkModuleHandle m mh = .ok () →
      mh.address < m.addressIdentifiers.size ∧ mh.name < m.identifiers.size := by
    intro mh h
    unfold Bounds.checkModuleHandle at h
    obtain ⟨h1, h2⟩ := seq_ok2 h
    exact ⟨checkIdx_ok h1, checkIdx_ok h2⟩
  refine ⟨fun mh hm => modH mh (forM_ok hmh mh hm), checkIdx_ok hself, fun sh hs => ?_,
    fun fh hf => ?_, fun mh hm => modH mh (forM_ok hfr mh hm)⟩
  · obtain ⟨h1, h2⟩ := seq_ok2 (forM_ok hsh sh hs)
    exact ⟨checkIdx_ok h1, checkIdx_ok h2⟩
  have h := forM_ok hfh fh hf
  unfold Bounds.checkFunctionHandle at h
  obtain ⟨h1, h⟩ := seq_ok2 h
  obtain ⟨h2, h⟩ := seq_ok2 h
  obtain ⟨h3, h⟩ := seq_ok2 h
  obtain ⟨h4, -⟩ := seq_ok2 h
  exact ⟨checkIdx_ok h1, checkIdx_ok h2, checkIdx_ok h3, checkIdx_ok h4⟩

theorem getElem?_of_lt {α : Type} {a : Array α} {i : Nat} (h : i < a.size) :
    ∃ x, a[i]? = some x := ⟨a[i], Array.getElem?_eq_getElem h⟩

/-! ## Resolution of a module's own functions -/

theorem toList_getElem? {α : Type} {a : Array α} {i : Nat} : a.toList[i]? = a[i]? := by simp

/-- Function definitions of a verified module have distinct names. -/
theorem defName_unique {m : CompiledModule} (hu : FunctionsUnique m) {i j : Nat}
    {d e : FunctionDefinition} {name : String} (hi : m.functionDefs[i]? = some d)
    (hj : m.functionDefs[j]? = some e) (hd : m.functionDefName d = some name)
    (he : m.functionDefName e = some name) : i = j := by
  simp only [CompiledModule.functionDefName, bind, Option.bind_eq_some_iff] at hd he
  obtain ⟨fd, hfd, hnd⟩ := hd
  obtain ⟨fe, hfe, hne⟩ := he
  obtain ⟨fd', hfd', hmd⟩ := hu.ownDefs d (getElem?_mem_toList hi)
  obtain ⟨fe', hfe', hme⟩ := hu.ownDefs e (getElem?_mem_toList hj)
  rw [hfd, Option.some.injEq] at hfd'
  rw [hfe, Option.some.injEq] at hfe'
  subst hfd' hfe'
  have hname : fd.name = fe.name :=
    nodup_index hu.identifiers (by rw [toList_getElem?]; exact hnd)
      (by rw [toList_getElem?]; exact hne)
  have hfun : d.function = e.function :=
    nodup_map_index hu.handles (by rw [toList_getElem?]; exact hfd)
      (by rw [toList_getElem?]; exact hfe) (by simp [hmd, hme, hname])
  exact nodup_map_index hu.defs (by rw [toList_getElem?]; exact hi)
    (by rw [toList_getElem?]; exact hj) hfun

theorem findModule_eq {P : Program} (hdist : P.DistinctModuleIds) {mi : Nat} {m : CompiledModule}
    {id : ModuleId} (hm : P.modules[mi]? = some m) (hid : m.selfId = some id) :
    P.findModule id = some mi := by
  unfold Program.findModule
  refine uniqueIdx_eq hm (by simp [hid]) fun j m' hj hp => ?_
  simp only [beq_iff_eq] at hp
  exact nodup_map_index hdist (by rw [toList_getElem?]; exact hj)
    (by rw [toList_getElem?]; exact hm) (by simp [hp, hid])

/-- A handle of a module's own function resolves to its definition. -/
theorem resolve_self {P : Program} (hdist : P.DistinctModuleIds) {mi : Nat} {m : CompiledModule}
    {id : ModuleId} (hm : P.modules[mi]? = some m) (hid : m.selfId = some id)
    (hu : FunctionsUnique m) (hb : HandlesInBounds m) {h : Nat} {fh : FunctionHandle}
    (hfh : m.functionHandles[h]? = some fh) (hself : fh.module = m.selfModuleHandleIdx) :
    ∃ di d, P.resolve mi h = some ⟨mi, di, []⟩ ∧ m.functionDefs[di]? = some d ∧ d.function = h ∧
      P.function? ⟨mi, di, []⟩ = some ⟨m, id, d, fh⟩ := by
  obtain ⟨d, hd, hdh⟩ := hu.implemented h fh hfh hself
  obtain ⟨di, hdi⟩ := List.mem_iff_getElem?.1 hd
  rw [toList_getElem?] at hdi
  obtain ⟨name, hname⟩ := getElem?_of_lt (hb.functionHandles fh (getElem?_mem_toList hfh)).2.1
  have hdn : m.functionDefName d = some name := by
    simp [CompiledModule.functionDefName, hdh, hfh, hname]
  have huniq : Program.uniqueIdx m.functionDefs (fun d => m.functionDefName d == some name) =
      some di :=
    uniqueIdx_eq hdi (by simp [hdn]) fun j e hj he => by
      simp only [beq_iff_eq] at he
      exact defName_unique hu hj hdi he hdn
  have htarget : m.moduleIdAt fh.module = some id := by
    rw [hself]; exact hid
  refine ⟨di, d, ?_, hdi, hdh, ?_⟩
  · simp [Program.resolve, hm, hfh, htarget, hname, findModule_eq hdist hm hid, huniq]
  · simp [Program.function?, hm, hid, hdi, CompiledModule.functionDefHandle, hdh, hfh]

/-! ## Resolution of a module's own structs -/

/-- Struct definitions of a verified module have distinct names. -/
theorem structDefName_unique {m : CompiledModule} (hu : FunctionsUnique m) (hs : StructsUnique m)
    {i j : Nat} {d e : StructDefinition} {name : String} (hi : m.structDefs[i]? = some d)
    (hj : m.structDefs[j]? = some e) (hd : m.structDefName d = some name)
    (he : m.structDefName e = some name) : i = j := by
  simp only [CompiledModule.structDefName, CompiledModule.structDefHandle, bind,
    Option.bind_eq_some_iff] at hd he
  obtain ⟨sd, hsd, hnd⟩ := hd
  obtain ⟨se, hse, hne⟩ := he
  obtain ⟨sd', hsd', hmd⟩ := hs.ownDefs d (getElem?_mem_toList hi)
  obtain ⟨se', hse', hme⟩ := hs.ownDefs e (getElem?_mem_toList hj)
  rw [hsd, Option.some.injEq] at hsd'
  rw [hse, Option.some.injEq] at hse'
  subst hsd' hse'
  have hname : sd.name = se.name :=
    nodup_index hu.identifiers (by rw [toList_getElem?]; exact hnd)
      (by rw [toList_getElem?]; exact hne)
  have hh : d.structHandle = e.structHandle :=
    nodup_map_index hs.handles (by rw [toList_getElem?]; exact hsd)
      (by rw [toList_getElem?]; exact hse) (by simp [hmd, hme, hname])
  exact nodup_map_index hs.defs (by rw [toList_getElem?]; exact hi)
    (by rw [toList_getElem?]; exact hj) hh

/-- A struct definition of a verified module of a program defines the type its
handle names, which resolves back to it. -/
theorem resolve_own_struct {P : Program} (hdist : P.DistinctModuleIds) {mi : Nat}
    {m : CompiledModule} {id : ModuleId} (hm : P.modules[mi]? = some m)
    (hid : m.selfId = some id) (hu : FunctionsUnique m) (hs : StructsUnique m)
    (hb : HandlesInBounds m) {d : Nat} {sd : StructDefinition} (hsd : m.structDefs[d]? = some sd) :
    ∃ sh name, m.structHandles[sd.structHandle]? = some sh ∧ m.identifiers[sh.name]? = some name ∧
      m.structIdAt sd.structHandle = some ⟨id, name, []⟩ ∧
      P.resolveStruct ⟨id, name, []⟩ = some ⟨mi, d⟩ ∧
      P.struct? ⟨mi, d⟩ = some ⟨m, id, sd, sh⟩ := by
  obtain ⟨sh, hsh, hself⟩ := hs.ownDefs sd (getElem?_mem_toList hsd)
  obtain ⟨name, hname⟩ := getElem?_of_lt (hb.structHandles sh (getElem?_mem_toList hsh)).2
  have hdn : m.structDefName sd = some name := by
    simp [CompiledModule.structDefName, CompiledModule.structDefHandle, hsh, hname]
  have huniq : Program.uniqueIdx m.structDefs (fun d => m.structDefName d == some name) =
      some d :=
    uniqueIdx_eq hsd (by simp [hdn]) fun j e hj he => by
      simp only [beq_iff_eq] at he
      exact structDefName_unique hu hs hj hsd he hdn
  have hmid : m.moduleIdAt sh.module = some id := by rw [hself]; exact hid
  refine ⟨sh, name, hsh, hname, ?_, ?_, ?_⟩
  · simp [CompiledModule.structIdAt, hsh, hmid, hname]
  · simp [Program.resolveStruct, findModule_eq hdist hm hid, hm, huniq]
  · simp [Program.struct?, hm, hid, hsd, CompiledModule.structDefHandle, hsh]

/-- The type a struct definition defines, as `Pack` and `Unpack` name it. -/
theorem definedStruct_own {P : Program} (hdist : P.DistinctModuleIds) {mi : Nat}
    {m : CompiledModule} {id : ModuleId} (hm : P.modules[mi]? = some m)
    (hid : m.selfId = some id) (hu : FunctionsUnique m) (hs : StructsUnique m)
    (hb : HandlesInBounds m) (hgen : ∀ sh ∈ m.structHandles.toList, sh.typeParameters = [])
    {d : Nat} {sd : StructDefinition} (hsd : m.structDefs[d]? = some sd) :
    ∃ s id sh, P.definedStruct? mi d = some s ∧ m.structIdAt sd.structHandle = some s ∧
      m.structHandles[sd.structHandle]? = some sh ∧ P.structType? s = some ⟨m, id, sd, sh⟩ := by
  obtain ⟨sh, name, hsh, hname, hsid, hres, hst⟩ := resolve_own_struct hdist hm hid hu hs hb hsd
  have hnp := hgen sh (getElem?_mem_toList hsh)
  refine ⟨⟨id, name, []⟩, id, sh, ?_, hsid, hsh, ?_⟩
  · simp [Program.definedStruct?, hst, CompiledModule.structDefName,
      CompiledModule.structDefHandle, hsh, hname, hres]
  · simp [Program.structType?, hres, hst, hnp]

/-! ## The dependency check -/

theorem foldlM_inv {α β ε : Type} {f : β → α → Except ε β} {l : List α} {init r : β}
    (Q : β → Prop) (h0 : Q init) (hstep : ∀ b a r, a ∈ l → Q b → f b a = .ok r → Q r)
    (h : l.foldlM f init = .ok r) : Q r := by
  induction l generalizing init with
  | nil => simp [pure, Except.pure] at h; subst h; exact h0
  | cons x xs ih =>
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨b, hb, h⟩ := h
    exact ih (hstep init x b (List.mem_cons_self ..) h0 hb)
      (fun b a r ha => hstep b a r (List.mem_cons_of_mem _ ha)) h

theorem moduleIdForHandle_ok {m : CompiledModule} {mh : ModuleHandle} {id : ModuleId}
    (h : moduleIdForHandle m mh = .ok id) : m.moduleIdOf mh = some id := by
  unfold moduleIdForHandle at h
  simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨a, ha, n, hn, rfl⟩ := h
  simp [CompiledModule.moduleIdOf, ha, hn]

theorem selfModuleId_ok {m : CompiledModule} {id : ModuleId} (h : selfModuleId m = .ok id) :
    m.selfId = some id := by
  unfold selfModuleId at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨mh, hmh, h⟩ := h
  simp [CompiledModule.selfId, CompiledModule.moduleIdAt, hmh, moduleIdForHandle_ok h]

theorem dependencyLookup_some {deps : List (ModuleId × CompiledModule)} {id : ModuleId}
    {owner : CompiledModule} (h : dependencyLookup deps id = some owner) : (id, owner) ∈ deps := by
  unfold dependencyLookup at h
  simp only [Option.map_eq_some_iff] at h
  obtain ⟨e, he, rfl⟩ := h
  have hmem := List.mem_of_getLast? he
  simp only [List.mem_filter, beq_iff_eq] at hmem
  obtain ⟨hmem, rfl⟩ := hmem
  exact hmem

/-- The dependencies other than the module itself, by id. -/
theorem deps_fold {self : ModuleId} {dependencies : List CompiledModule}
    {deps : List (ModuleId × CompiledModule)}
    (h : dependencies.foldlM (fun deps d => do
      let id ← selfModuleId d
      pure (if id != self then deps ++ [(id, d)] else deps)) [] = .ok deps) :
    ∀ e ∈ deps, e.2 ∈ dependencies ∧ e.2.selfId = some e.1 ∧ e.1 ≠ self := by
  refine foldlM_inv (fun deps => ∀ e ∈ deps, e.2 ∈ dependencies ∧ e.2.selfId = some e.1 ∧
    e.1 ≠ self) (by simp) ?_ h
  intro b d r hd hb hr
  simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at hr
  obtain ⟨id, hid, rfl⟩ := hr
  split
  · next hne =>
    intro e he
    rcases List.mem_append.1 he with he | he
    · exact hb e he
    · simp only [List.mem_singleton] at he
      subst he
      exact ⟨hd, selfModuleId_ok hid, by simpa using hne⟩
  · exact hb

theorem mem_mapM_ok {α β ε : Type} {f : α → Except ε β} {l : List α} {r : List β}
    (h : l.mapM f = .ok r) {y : β} (hy : y ∈ r) : ∃ x ∈ l, f x = .ok y := by
  induction l generalizing r with
  | nil => simp [pure, Except.pure] at h; subst h; simp at hy
  | cons a t ih =>
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨b, hb, bs, hbs, rfl⟩ := h
    rcases List.mem_cons.1 hy with rfl | hy
    · exact ⟨a, List.mem_cons_self .., hb⟩
    · obtain ⟨x, hx, hfx⟩ := ih hbs hy
      exact ⟨x, List.mem_cons_of_mem _ hx, hfx⟩

/-- What `callableFunction` found. -/
theorem callableFunction_some {self : ModuleId} {dep : CompiledModule} {name : String}
    {oh od : Nat} (h : callableFunction self dep name = .ok (some (oh, od))) :
    ∃ fdef fh, dep.functionDefs[od]? = some fdef ∧ fdef.function = oh ∧
      dep.functionHandles[oh]? = some fh ∧ dep.identifiers[fh.name]? = some name ∧
      (fdef.visibility = .public_ ∨ (fdef.visibility = .friend ∧
        ∃ mh ∈ dep.friendDecls.toList, dep.moduleIdOf mh = some self)) := by
  unfold callableFunction at h
  simp only [bind_ok] at h
  obtain ⟨friends, hfriends, h⟩ := h
  refine foldlM_inv (fun (found : Option (Nat × Nat)) => ∀ oh od, found = some (oh, od) →
    ∃ fdef fh, dep.functionDefs[od]? = some fdef ∧ fdef.function = oh ∧
      dep.functionHandles[oh]? = some fh ∧ dep.identifiers[fh.name]? = some name ∧
      (fdef.visibility = .public_ ∨ (fdef.visibility = .friend ∧
        ∃ mh ∈ dep.friendDecls.toList, dep.moduleIdOf mh = some self))) (by simp) ?_ h oh od rfl
  intro found ⟨fdef, idx⟩ r hmem hfound hr
  simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at hr
  obtain ⟨fh, hfh, fname, hfname, rfl⟩ := hr
  intro oh od hsome
  rw [List.mem_zipIdx_iff_getElem?] at hmem
  simp only [Array.getElem?_toList] at hmem
  cases hv : fdef.visibility <;> simp only [hv, Bool.and_false, Bool.false_eq_true,
    ↓reduceIte, Bool.and_true, Bool.and_eq_true, beq_iff_eq] at hsome
  · -- private
    exact hfound oh od hsome
  · -- public
    split at hsome
    · next hc =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hsome
      obtain ⟨rfl, rfl⟩ := hsome
      subst hc
      exact ⟨fdef, fh, by simpa using hmem, rfl, hfh, hfname, Or.inl hv⟩
    · exact hfound oh od hsome
  · -- friend
    split at hsome
    · next hc =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hsome
      obtain ⟨rfl, rfl⟩ := hsome
      obtain ⟨rfl, hc⟩ := hc
      rw [List.contains_iff_mem] at hc
      obtain ⟨mh, hmh, hid⟩ := mem_mapM_ok hfriends hc
      exact ⟨fdef, fh, by simpa using hmem, rfl, hfh, hfname, Or.inr ⟨hv, mh, hmh,
        moduleIdForHandle_ok hid⟩⟩
    · exact hfound oh od hsome

/-- `compare_structs` accepts two struct handles that name the same type. -/
theorem compareStructs_ok {m d : CompiledModule} {i1 i2 : Nat}
    (h : compareStructs m i1 d i2 = .ok ()) :
    ∃ s, m.structIdAt i1 = some s ∧ d.structIdAt i2 = some s := by
  unfold compareStructs at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sh, hsh, mh, hmh, mid, hmid, name, hname, dsh, hdsh, dmh, hdmh, dmid, hdmid, dname,
    hdname, h⟩ := h
  split at h
  · simp [reject] at h
  · next hc =>
    simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at hc
    obtain ⟨rfl, rfl⟩ := hc
    refine ⟨⟨mid, name, []⟩, ?_, ?_⟩
    · simp [CompiledModule.structIdAt, CompiledModule.moduleIdAt, hsh, hmh,
        moduleIdForHandle_ok hmid, hname]
    · simp [CompiledModule.structIdAt, CompiledModule.moduleIdAt, hdsh, hdmh,
        moduleIdForHandle_ok hdmid, hdname]

/-- What `definedStruct` found: the handle of a struct definition of the
dependency with that name. -/
theorem definedStruct_some {dep : CompiledModule} {name : String} {idx : Nat}
    (h : definedStruct dep name = .ok (some idx)) :
    ∃ sd sh, sd ∈ dep.structDefs.toList ∧ sd.structHandle = idx ∧
      dep.structHandles[idx]? = some sh ∧ dep.identifiers[sh.name]? = some name := by
  unfold definedStruct at h
  refine foldlM_inv (fun (found : Option Nat) => ∀ idx, found = some idx →
    ∃ sd sh, sd ∈ dep.structDefs.toList ∧ sd.structHandle = idx ∧
      dep.structHandles[idx]? = some sh ∧ dep.identifiers[sh.name]? = some name)
    (by simp) ?_ h idx rfl
  intro found sd r hmem hfound hr
  simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at hr
  obtain ⟨sh, hsh, sname, hsname, rfl⟩ := hr
  intro idx hsome
  split at hsome
  · next hc =>
    simp only [Option.some.injEq] at hsome
    subst hsome
    simp only [beq_iff_eq] at hc
    subst hc
    exact ⟨sd, sh, hmem, rfl, hsh, hsname⟩
  · exact hfound idx hsome

/-- What the dependency check establishes about an imported struct. -/
def ImportStructOk (m : CompiledModule) (deps : List (ModuleId × CompiledModule))
    (sh : StructHandle) : Prop :=
  ∃ ownerId owner name defIdx dsh, m.moduleIdAt sh.module = some ownerId ∧
    dependencyLookup deps ownerId = some owner ∧ m.identifiers[sh.name]? = some name ∧
    definedStruct owner name = .ok (some defIdx) ∧ owner.structHandles[defIdx]? = some dsh ∧
    sh.abilities.subset dsh.abilities = true ∧
    compatibleStructTypeParameters sh.typeParameters dsh.typeParameters = true

/-- What the dependency check establishes about an imported function. -/
def ImportOk (m : CompiledModule) (self : ModuleId) (deps : List (ModuleId × CompiledModule))
    (fh : FunctionHandle) : Prop :=
  ∃ ownerId name owner oh od dfh params dparams returns dreturns,
    m.moduleIdAt fh.module = some ownerId ∧ m.identifiers[fh.name]? = some name ∧
    dependencyLookup deps ownerId = some owner ∧
    callableFunction self owner name = .ok (some (oh, od)) ∧
    owner.functionHandles[oh]? = some dfh ∧
    m.signatures[fh.parameters]? = some params ∧ owner.signatures[dfh.parameters]? = some dparams ∧
    compareSignatures m owner params dparams = .ok () ∧
    m.signatures[fh.returns]? = some returns ∧ owner.signatures[dfh.returns]? = some dreturns ∧
    compareSignatures m owner returns dreturns = .ok () ∧
    compatibleFunTypeParameters fh.typeParameters dfh.typeParameters = true ∧
    (FunctionAttribute.persistent ∈ fh.attributes →
      FunctionAttribute.persistent ∈ dfh.attributes ∨
        ∃ fd, owner.functionDefs[od]? = some fd ∧ fd.visibility = .public_)

theorem verifyDependencies_imports {m : CompiledModule} {dependencies : List CompiledModule}
    (h : verifyDependencies m dependencies = .ok ()) :
    ∃ self deps, selfModuleId m = .ok self ∧
      (∀ e ∈ deps, e.2 ∈ dependencies ∧ e.2.selfId = some e.1 ∧ e.1 ≠ self) ∧
      ∀ fh ∈ m.functionHandles.toList, fh.module ≠ m.selfModuleHandleIdx →
        ImportOk m self deps fh := by
  unfold verifyDependencies at h
  simp only [bind_ok] at h
  obtain ⟨self, hself, deps, hdeps, h⟩ := h
  refine ⟨self, deps, hself, deps_fold hdeps, fun fh hfh hne => ?_⟩
  obtain ⟨_, -, _, -, _, -, h⟩ := h
  have hb := forM_ok h fh hfh
  simp only [bne_iff_ne, ne_eq, hne, not_false_eq_true, ↓reduceIte, bind_ok,
    lookup_ok] at hb
  obtain ⟨mh, hmh, ownerId, hid, name, hname, hb⟩ := hb
  split at hb
  · next owner howner =>
    simp only [bind_ok] at hb
    obtain ⟨r, hr, hb⟩ := hb
    split at hb
    · next oh od =>
      simp only [bind_ok, lookup_ok] at hb
      obtain ⟨dfh, hdfh, hb⟩ := hb
      split at hb
      · simp [reject, bind, Except.bind] at hb
      · next hcompat =>
        simp only [bind_ok, lookup_ok] at hb
        obtain ⟨params, hps, dparams, hdps, u1, hc1, returns, hrs, dreturns, hdrs, u2, hc2,
          hb⟩ := hb
        cases u1; cases u2
        refine ⟨ownerId, name, owner, oh, od, dfh, params, dparams, returns, dreturns,
          by simp [CompiledModule.moduleIdAt, hmh, moduleIdForHandle_ok hid], hname, howner, hr,
          hdfh, hps, hdps, hc1, hrs, hdrs, hc2, by simpa using hcompat, fun hp => ?_⟩
        -- `persistent` cannot be removed; a public definition without attributes
        -- counts as persistent.
        have hcomp : ∀ D : List FunctionAttribute,
            (if (!attributesCompatible fh.attributes D) = true then reject "LINKER_ERROR"
              else (Except.ok () : VResult Unit)) = .ok () → FunctionAttribute.persistent ∈ D := by
          intro D hD
          split at hD
          · simp [reject] at hD
          · next hcomp =>
            simp only [Bool.not_eq_true', Bool.not_eq_false] at hcomp
            unfold attributesCompatible at hcomp
            simpa [cannotBeRemoved] using List.all_eq_true.1 hcomp _ hp
        by_cases hcond : (!fh.attributes.isEmpty && dfh.attributes.isEmpty) = true
        · simp only [hcond, ↓reduceIte, bind_ok, lookup_ok] at hb
          obtain ⟨fd, hfd, hb⟩ := hb
          split at hb
          · next hpub => exact Or.inr ⟨fd, hfd, by simpa using hpub⟩
          · simp only [pure, Except.pure, bind, Except.bind] at hb
            exact Or.inl (hcomp _ hb)
        · simp only [hcond, Bool.false_eq_true, ↓reduceIte] at hb
          simp only [pure, Except.pure, bind, Except.bind] at hb
          exact Or.inl (hcomp _ hb)
    · simp [reject] at hb
  · simp [reject] at hb

theorem verifyDependencies_structs {m : CompiledModule} {dependencies : List CompiledModule}
    (h : verifyDependencies m dependencies = .ok ()) :
    ∃ self deps, selfModuleId m = .ok self ∧
      (∀ e ∈ deps, e.2 ∈ dependencies ∧ e.2.selfId = some e.1 ∧ e.1 ≠ self) ∧
      ∀ sh ∈ m.structHandles.toList, sh.module ≠ m.selfModuleHandleIdx →
        ImportStructOk m deps sh := by
  unfold verifyDependencies at h
  simp only [bind_ok] at h
  obtain ⟨self, hself, deps, hdeps, h⟩ := h
  refine ⟨self, deps, hself, deps_fold hdeps, fun sh hsh hne => ?_⟩
  obtain ⟨_, -, _, -, _, h, -⟩ := h
  have hb := forM_ok h sh hsh
  simp only [bne_iff_ne, ne_eq, hne, not_false_eq_true, ↓reduceIte, bind_ok,
    lookup_ok] at hb
  obtain ⟨mh, hmh, ownerId, hid, hb⟩ := hb
  split at hb
  · next owner howner =>
    simp only [bind_ok, lookup_ok] at hb
    obtain ⟨name, hname, r, hr, hb⟩ := hb
    split at hb
    · next defIdx =>
      simp only [bind_ok, lookup_ok] at hb
      obtain ⟨dsh, hdsh, hb⟩ := hb
      split at hb
      · simp [reject] at hb
      · next hc =>
        simp only [Bool.or_eq_true, Bool.not_eq_true', not_or, Bool.not_eq_false] at hc
        exact ⟨ownerId, owner, name, defIdx, dsh,
          by simp [CompiledModule.moduleIdAt, hmh, moduleIdForHandle_ok hid], howner, hname, hr,
          hdsh, hc.1, hc.2⟩
    · simp [reject] at hb
  · simp [reject] at hb

theorem mem_others {P : Program} {i : Nat} {x : CompiledModule} (h : x ∈ P.others i) :
    ∃ j, j ≠ i ∧ P.modules[j]? = some x := by
  simp only [Program.others, List.mem_map, List.mem_filter, bne_iff_ne, ne_eq] at h
  obtain ⟨⟨y, j⟩, ⟨hmem, hj⟩, rfl⟩ := h
  rw [List.mem_zipIdx_iff_getElem?] at hmem
  exact ⟨j, hj, by simpa using hmem⟩

/-- A called handle is in bounds. -/
theorem call_bound {m : CompiledModule} (hb : verifyBounds m = .ok ())
    {fdef : FunctionDefinition} (hd : fdef ∈ m.functionDefs.toList) {code : CodeUnit}
    (hcode : fdef.code = some code) {h : FunctionHandleIndex} (hi : Bytecode.call h ∈ code.code.toList) :
    h < m.functionHandles.size := by
  obtain ⟨n, params, hcc⟩ := checkFunctionDef_code (verifyBounds_defs hb fdef hd) hcode
  obtain ⟨lc, hins⟩ := checkCode_instrs hcc
  have hc := hins _ hi
  simp only [Bounds.checkInstr] at hc
  exact checkIdx_ok hc

/-- The bounds checker keeps a closure's target handle in range. -/
theorem packClosure_bound {m : CompiledModule} (hb : verifyBounds m = .ok ())
    {fdef : FunctionDefinition} (hd : fdef ∈ m.functionDefs.toList) {code : CodeUnit}
    (hcode : fdef.code = some code) {h : FunctionHandleIndex} {mask : BitVec 64}
    (hi : Bytecode.packClosure h mask ∈ code.code.toList) : h < m.functionHandles.size := by
  obtain ⟨n, params, hcc⟩ := checkFunctionDef_code (verifyBounds_defs hb fdef hd) hcode
  obtain ⟨lc, hins⟩ := checkCode_instrs hcc
  have hc := hins _ hi
  simp only [Bounds.checkInstr] at hc
  exact checkIdx_ok hc

theorem module_selfId {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {mi : Nat} {m : CompiledModule} (hm : P.modules[mi]? = some m) : ∃ id, m.selfId = some id := by
  have := (wellFormed_module hwf hm).2
  rw [verifyLinking, show VerifierConfig.production.scope = .everything from rfl] at this
  obtain ⟨self, -, hself, -⟩ := verifyDependencies_imports (by simpa using this)
  exact ⟨self, selfModuleId_ok hself⟩

/-! ## Struct handles -/

theorem conservativeFor_refl (sh : StructHandle) : sh.ConservativeFor sh := by
  refine ⟨AbilitySet.subset_refl _, rfl, fun i p q hp hq => ?_⟩
  rw [hp, Option.some.injEq] at hq
  subst hq
  exact ⟨id, AbilitySet.subset_refl _⟩

/-- The dependency check's comparison of an imported struct handle with the
definition's is `StructHandle.ConservativeFor`. -/
theorem conservativeFor_of_compatible {sh dsh : StructHandle}
    (hab : sh.abilities.subset dsh.abilities = true)
    (h : compatibleStructTypeParameters sh.typeParameters dsh.typeParameters = true) :
    sh.ConservativeFor dsh := by
  unfold compatibleStructTypeParameters at h
  simp only [Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at h
  obtain ⟨hlen, hall⟩ := h
  refine ⟨hab, hlen, fun i p q hp hq => ?_⟩
  have hmem : (p, q) ∈ sh.typeParameters.zip dsh.typeParameters := by
    rw [List.mem_iff_getElem?]
    exact ⟨i, by simp [List.getElem?_zip_eq_some, hp, hq]⟩
  have := hall _ hmem
  simp only [Bool.or_eq_true, Bool.not_eq_true', compatibleConstraints] at this
  obtain ⟨hph, hc⟩ := this
  refine ⟨fun hp' => ?_, hc⟩
  rcases hph with h | h
  · rw [hp'] at h; cases h
  · exact h

/-- **K1** for one struct handle. A handle of the module's own type resolves
through the duplication checker, to the definition whose handle it is. A
handle of an imported type resolves through the dependency check, which found
a definition of that name in the owner and compared the handles. -/
theorem struct_handle_resolves {P : Program}
    (hwf : WellFormedBy verifier VerifierConfig.production P) {mi : Nat} {m : CompiledModule}
    (hm : P.modules[mi]? = some m) {h : StructHandleIndex} {sh : StructHandle}
    (hsh : m.structHandles[h]? = some sh) :
    ∃ s r info, m.structIdAt h = some s ∧ P.resolveStruct s = some r ∧ P.struct? r = some info ∧
      sh.ConservativeFor info.handle := by
  have hdist := hwf.1
  have hver : ∀ j m', P.modules[j]? = some m' →
      FunctionsUnique m' ∧ StructsUnique m' ∧ HandlesInBounds m' := fun j m' hj =>
    let hv := verifyModule_ok (wellFormed_module hwf hj).1
    ⟨verifyDuplication_ok hv.2.1, verifyDuplication_structs hv.2.1, verifyBounds_handles hv.1⟩
  obtain ⟨hu, hsu, hb⟩ := hver mi m hm
  obtain ⟨id, hid⟩ := module_selfId hwf hm
  by_cases hself : sh.module = m.selfModuleHandleIdx
  · obtain ⟨sd, hsd, rfl⟩ := hsu.implemented h sh hsh hself
    obtain ⟨d, hd⟩ := List.mem_iff_getElem?.1 hsd
    rw [toList_getElem?] at hd
    obtain ⟨sh', name, hsh', -, hsid, hres, hst⟩ := resolve_own_struct hdist hm hid hu hsu hb hd
    rw [hsh, Option.some.injEq] at hsh'
    subst hsh'
    exact ⟨_, _, _, hsid, hres, hst, conservativeFor_refl sh⟩
  · obtain ⟨self, deps, -, hdeps, himp⟩ := verifyDependencies_structs (by
      have := (wellFormed_module hwf hm).2
      rw [verifyLinking, show VerifierConfig.production.scope = .everything from rfl] at this
      simpa using this)
    obtain ⟨ownerId, owner, name, defIdx, dsh, hmid, howner, hname, hdef, hdsh, hab, hcompat⟩ :=
      himp sh (getElem?_mem_toList hsh) hself
    obtain ⟨hin, hoid, -⟩ := hdeps _ (dependencyLookup_some howner)
    obtain ⟨mi', -, hmi'⟩ := mem_others hin
    obtain ⟨hu', hsu', hb'⟩ := hver mi' owner hmi'
    obtain ⟨sd, sh', hsd, rfl, hsh', hname'⟩ := definedStruct_some hdef
    rw [hdsh, Option.some.injEq] at hsh'
    subst hsh'
    obtain ⟨od, hod⟩ := List.mem_iff_getElem?.1 hsd
    rw [toList_getElem?] at hod
    obtain ⟨dsh', name', hdsh', hname'', -, hres, hst⟩ :=
      resolve_own_struct hdist hmi' hoid hu' hsu' hb' hod
    rw [hdsh, Option.some.injEq] at hdsh'
    subst hdsh'
    rw [hname', Option.some.injEq] at hname''
    subst hname''
    refine ⟨⟨ownerId, name, []⟩, _, _, ?_, hres, hst, conservativeFor_of_compatible hab hcompat⟩
    simp [CompiledModule.structIdAt, hsh, hmid, hname]

/-- The facts about a module's structs that the type certificate needs. -/
theorem structsAgree {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) {mi : Nat} {m : CompiledModule} (hm : P.modules[mi]? = some m) :
    StructsAgree P mi m := by
  have hsup := hs m (module_mem hm)
  refine ⟨fun h sh s hsh hs' => ?_, fun d sd hsd => ?_⟩
  · obtain ⟨s', r, info, hs'', hres, hst, hcons⟩ := struct_handle_resolves hwf hm hsh
    rw [hs', Option.some.injEq] at hs''
    subst hs''
    have hnp : info.handle.typeParameters = [] := by
      have := hcons.2.1
      rw [hsup.2.2.1 sh (getElem?_mem_toList hsh)] at this
      exact List.eq_nil_of_length_eq_zero this.symm
    have hta : s.typeArgs = [] := by rw [← structIdAt_typeArgs hs']
    have hty : P.structType? s = some info := by simp [Program.structType?, hres, hst, hnp, hta]
    simp only [RuntimeType.abilities, hty, hta, hnp, RuntimeType.argumentAbilities,
      AbilitySet.all_inter]
    exact hcons.1
  · have hv := verifyModule_ok (wellFormed_module hwf hm).1
    obtain ⟨id, hid⟩ := module_selfId hwf hm
    exact definedStruct_own hwf.1 hm hid (verifyDuplication_ok hv.2.1)
      (verifyDuplication_structs hv.2.1) (verifyBounds_handles hv.1) hsup.2.2.1 hsd

/-! ## Declared abilities -/

theorem verifyModule_signatures {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) :
    verifySignatures VerifierConfig.production m = .ok () := by
  unfold verifyModule at h
  simp only [show (VerifierConfig.production.scope == .nothing) = false from rfl,
    Bool.false_eq_true, ↓reduceIte, bind_ok] at h
  obtain ⟨-, -, -, -, -, -, -, -, u5, h5, -⟩ := h
  cases u5
  exact h5

theorem verifySignatures_structDefs {config : VerifierConfig} {m : CompiledModule}
    (h : verifySignatures config m = .ok ()) :
    ∀ sd ∈ m.structDefs.toList, SignatureV2.verifyStructDef config m sd = .ok () := by
  unfold verifySignatures SignatureV2.verify at h
  split at h
  · simp [reject, bind, Except.bind] at h
  · simp only [bind_ok] at h
    obtain ⟨-, -, -, -, -, -, -, -, -, -, -, -, -, -, -, -, h⟩ := h
    exact forM_ok h

theorem assertAbilities_ok {a req : AbilitySet} (h : SignatureV2.assertAbilities a req = .ok ()) :
    req.subset a = true := by
  unfold SignatureV2.assertAbilities at h
  split at h
  · assumption
  · simp [reject] at h

theorem requires_forContainer {a x : AbilitySet} (h : (requires a).subset x = true) :
    a.subset x.forContainer = true := by
  cases a; cases x
  simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.forContainer, requires]
  grind

theorem requires_vector {req x : AbilitySet} (hreq : req.subset SignatureV2.vectorSet = true)
    (h : (requires req).subset x = true) :
    req.subset (AbilitySet.primitives.inter x.forContainer) = true :=
  AbilitySet.subset_inter.2 ⟨hreq, requires_forContainer h⟩

/-- Field types come from the fields of a resolved definition. -/
theorem fieldTypes_parts {P : Program} {s : StructId} {tys : List RuntimeType}
    (h : P.fieldTypes s = some tys ∨ ∃ tag, P.variantFieldTypes s tag = some tys) :
    ∃ info, P.structType? s = some info ∧ ∃ fields : List FieldDefinition,
      (∀ f ∈ fields,
        f ∈ CompiledModule.StructFieldInformation.allFields info.definition.fieldInformation) ∧
      fields.mapM (fun f => RuntimeType.ofTokenWith info.module s.typeArgs f.signature) =
        some tys := by
  rcases h with h | ⟨tag, h⟩
  · simp only [Program.fieldTypes, bind, Option.bind_eq_some_iff] at h
    obtain ⟨info, hinfo, h⟩ := h
    refine ⟨info, hinfo, ?_⟩
    split at h
    · next fields hfi =>
      exact ⟨fields, fun f hf => by
        simp [CompiledModule.StructFieldInformation.allFields, hfi, hf], h⟩
    · simp at h
    · simp at h
  · simp only [Program.variantFieldTypes, bind, Option.bind_eq_some_iff] at h
    obtain ⟨info, hinfo, h⟩ := h
    refine ⟨info, hinfo, ?_⟩
    split at h
    · next variants hfi =>
      simp only [Option.bind_eq_some_iff] at h
      obtain ⟨vd, hvd, h⟩ := h
      refine ⟨vd.fields, fun f hf => ?_, h⟩
      simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap]
      exact ⟨vd, List.mem_of_getElem? hvd, hf⟩
    · simp at h
    · simp at h

theorem structType_parts {P : Program} {s : StructId} {info : StructInfo}
    (h : P.structType? s = some info) :
    ∃ (mi di : Nat), P.modules[mi]? = some info.module ∧
      info.module.structDefs[di]? = some info.definition ∧
      info.module.structHandles[info.definition.structHandle]? = some info.handle := by
  simp only [Program.structType?, bind, Option.bind_eq_some_iff] at h
  obtain ⟨⟨mi, di⟩, -, info', hinfo, h⟩ := h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    simp only [Program.struct?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hinfo
    obtain ⟨m, hm, id, -, sd, hsd, sh, hsh, rfl⟩ := hinfo
    exact ⟨mi, di, hm, hsd, hsh⟩
  · simp at h

/-! ## K1–K2 for generic code

The dependency check compares a handle's signature tokens with the
definition's structurally: struct handles by module id and name, type
parameters by index. Struct handles of two modules that name one type are
conservative views of one definition, so they have its arity (`sameArity`),
and compared tokens translate to the same runtime types under any type
arguments (`compareSignatures_ofSignatureWith`). -/

/-- Struct handles of modules `m` and `d` that name the same type have the
same number of type parameters. -/
def SameArity (m d : CompiledModule) : Prop :=
  ∀ i1 i2 s sh1 sh2, m.structIdAt i1 = some s → d.structIdAt i2 = some s →
    m.structHandles[i1]? = some sh1 → d.structHandles[i2]? = some sh2 →
    sh1.typeParameters.length = sh2.typeParameters.length

theorem ofTokenWith_struct_eq {m : CompiledModule} {tys : List RuntimeType}
    {h : StructHandleIndex} :
    RuntimeType.ofTokenWith m tys (.struct h) = RuntimeType.ofToken m (.struct h) := by
  simp only [RuntimeType.ofToken, RuntimeType.ofTokenWith]

theorem ofToken_struct_inv' {m : CompiledModule} {h : StructHandleIndex} {u : RuntimeType}
    (hu : RuntimeType.ofToken m (.struct h) = some u) :
    ∃ sh s, m.structHandles[h]? = some sh ∧ sh.typeParameters = [] ∧ m.structIdAt h = some s ∧
      u = .struct s := by
  simp only [RuntimeType.ofToken, RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at hu
  obtain ⟨sh, hsh, hu⟩ := hu
  split at hu
  · next hnp =>
    simp only [pure, Option.bind_eq_some_iff, Option.some.injEq] at hu
    obtain ⟨s, hs, rfl⟩ := hu
    exact ⟨sh, s, hsh, by simpa using hnp, hs, rfl⟩
  · simp at hu

theorem structIdAt_handle {m : CompiledModule} {h : StructHandleIndex} {s : StructId}
    (hs : m.structIdAt h = some s) : ∃ sh, m.structHandles[h]? = some sh := by
  simp only [CompiledModule.structIdAt, bind, Option.bind_eq_some_iff] at hs
  obtain ⟨sh, hsh, -⟩ := hs
  exact ⟨sh, hsh⟩

theorem compareTypes_isRef {m d : CompiledModule} {t1 t2 : SignatureToken}
    (h : compareTypes m d t1 t2 = .ok ()) : t2.isRefToken = t1.isRefToken := by
  cases t1 <;> cases t2 <;> first | rfl | simp [compareTypes, reject] at h

theorem ofSlotTokensWith_cons_nonRef {m : CompiledModule} {tys : List RuntimeType}
    {t : SignatureToken} {ts : List SignatureToken} (hr : t.isRefToken = false) :
    RuntimeType.ofSlotTokensWith m tys (t :: ts) =
      (do pure ((← RuntimeType.ofTokenWith m tys t) :: (← RuntimeType.ofSlotTokensWith m tys ts))) := by
  cases t <;> simp_all [RuntimeType.ofSlotTokensWith, SignatureToken.isRefToken]

mutual
/-- Types the dependency check compares equal translate to the same runtime
type under any type arguments, when struct handles naming the same type have
the same arity: struct types compare by module id and name, type parameters by
index. -/
theorem compareTypes_ofTokenWith {m d : CompiledModule} (ha : SameArity m d)
    {tys : List RuntimeType} :
    ∀ (t1 t2 : SignatureToken) {u : RuntimeType}, compareTypes m d t1 t2 = .ok () →
      RuntimeType.ofTokenWith m tys t1 = some u → RuntimeType.ofTokenWith d tys t2 = some u
  | .vector a, t2, u, h, hu => by
    cases t2 <;> simp [compareTypes, reject] at h
    rename_i b
    simp only [RuntimeType.ofTokenWith, Option.map_eq_some_iff] at hu ⊢
    obtain ⟨u', hu', rfl⟩ := hu
    exact ⟨u', compareTypes_ofTokenWith ha a b h hu', rfl⟩
  | .struct i1, t2, u, h, hu => by
    cases t2 <;> simp [compareTypes, reject] at h
    rename_i i2
    obtain ⟨s, h1, h2⟩ := compareStructs_ok h
    rw [ofTokenWith_struct_eq] at hu ⊢
    obtain ⟨sh, s', hsh, hnp, hs', rfl⟩ := ofToken_struct_inv' hu
    rw [h1, Option.some.injEq] at hs'
    subst hs'
    obtain ⟨dsh, hdsh⟩ := structIdAt_handle h2
    have hlen := ha i1 i2 s sh dsh h1 h2 hsh hdsh
    rw [hnp] at hlen
    exact ofToken_struct hdsh (List.eq_nil_of_length_eq_zero hlen.symm) h2
  | .structInstantiation i1 a1, t2, u, h, hu => by
    cases t2 with
    | structInstantiation i2 a2 =>
      simp only [compareTypes] at h
      obtain ⟨hcs, hca⟩ := seq_ok2 h
      obtain ⟨s, h1, h2⟩ := compareStructs_ok hcs
      simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at hu ⊢
      obtain ⟨sh, hsh, sid, hsid, targs, htargs, hu⟩ := hu
      rw [h1, Option.some.injEq] at hsid
      subst hsid
      obtain ⟨dsh, hdsh⟩ := structIdAt_handle h2
      have hlen := ha i1 i2 _ sh dsh h1 h2 hsh hdsh
      refine ⟨dsh, hdsh, _, h2, targs, compareSignatures_ofTokenListWith ha a1 a2 hca htargs, ?_⟩
      split at hu
      · next hl => rw [if_pos (by omega)]; exact hu
      · simp at hu
    | _ => simp [compareTypes, reject] at h
  | .typeParameter i1, t2, u, h, hu => by
    cases t2 <;> simp [compareTypes, reject] at h
    rename_i i2
    have : i1 = i2 := by
      revert h; cases hne : (i1 != i2) <;> simp_all [pure, Except.pure]
    subst this
    simpa only [RuntimeType.ofTokenWith] using hu
  | .bool, t2, _, h, hu | .u8, t2, _, h, hu | .u16, t2, _, h, hu | .u32, t2, _, h, hu
  | .u64, t2, _, h, hu | .u128, t2, _, h, hu | .u256, t2, _, h, hu | .i8, t2, _, h, hu
  | .i16, t2, _, h, hu | .i32, t2, _, h, hu | .i64, t2, _, h, hu | .i128, t2, _, h, hu
  | .i256, t2, _, h, hu | .address, t2, _, h, hu | .signer, t2, _, h, hu => by
    cases t2 <;> simp [compareTypes, reject] at h <;> simpa [RuntimeType.ofTokenWith] using hu
  | .function ps rs ab, t2, u, h, hu => by
    cases t2 with
    | function ps' rs' ab' =>
      simp only [compareTypes] at h
      obtain ⟨h1, h23⟩ := seq_ok2 h
      obtain ⟨h2, h3⟩ := seq_ok2 h23
      have hab : ab = ab' := by
        revert h3; cases hne : (ab != ab') <;> simp_all [pure, Except.pure, reject]
      subst hab
      simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff, pure,
        Option.some.injEq] at hu ⊢
      obtain ⟨us, hus, vs, hvs, rfl⟩ := hu
      exact ⟨us, compareSignatures_ofSlotTokensWith ha ps ps' h1 hus, vs,
        compareSignatures_ofSlotTokensWith ha rs rs' h2 hvs, rfl⟩
    | _ => simp [compareTypes, reject] at h
  | .reference _, _, _, _, hu | .mutableReference _, _, _, _, hu => by
    simp [RuntimeType.ofTokenWith] at hu

theorem compareSignatures_ofTokenListWith {m d : CompiledModule} (ha : SameArity m d)
    {tys : List RuntimeType} :
    ∀ (a b : List SignatureToken) {r : List RuntimeType}, compareSignatures m d a b = .ok () →
      RuntimeType.ofTokenListWith m tys a = some r → RuntimeType.ofTokenListWith d tys b = some r
  | [], [], _, _, h => h
  | [], _ :: _, _, h, _ => by simp [compareSignatures, reject] at h
  | _ :: _, [], _, h, _ => by simp [compareSignatures, reject] at h
  | x :: xs, y :: ys, r, h, hr => by
    simp only [compareSignatures] at h
    obtain ⟨h1, h2⟩ := seq_ok2 h
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hr ⊢
    obtain ⟨u, hu, us, hus, rfl⟩ := hr
    exact ⟨u, compareTypes_ofTokenWith ha x y h1 hu, us,
      compareSignatures_ofTokenListWith ha xs ys h2 hus, rfl⟩

/-- The same for the slot types of a function type: references compare their
targets. -/
theorem compareSignatures_ofSlotTokensWith {m d : CompiledModule} (ha : SameArity m d)
    {tys : List RuntimeType} :
    ∀ (a b : List SignatureToken) {r : List RuntimeType}, compareSignatures m d a b = .ok () →
      RuntimeType.ofSlotTokensWith m tys a = some r → RuntimeType.ofSlotTokensWith d tys b = some r
  | [], [], _, _, h => h
  | [], _ :: _, _, h, _ => by simp [compareSignatures, reject] at h
  | _ :: _, [], _, h, _ => by simp [compareSignatures, reject] at h
  | x :: xs, y :: ys, r, h, hr => by
    simp only [compareSignatures] at h
    obtain ⟨h1, h2⟩ := seq_ok2 h
    cases hrx : x.isRefToken
    · have hry : y.isRefToken = false := (compareTypes_isRef h1).trans hrx
      rw [ofSlotTokensWith_cons_nonRef hrx] at hr
      rw [ofSlotTokensWith_cons_nonRef hry]
      simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hr ⊢
      obtain ⟨u, hu, us, hus, rfl⟩ := hr
      exact ⟨u, compareTypes_ofTokenWith ha x y h1 hu, us,
        compareSignatures_ofSlotTokensWith ha xs ys h2 hus, rfl⟩
    · cases x with
      | reference a =>
        cases y <;> simp [compareTypes, reject] at h1
        rename_i b
        simp only [RuntimeType.ofSlotTokensWith, bind, Option.bind_eq_some_iff, pure,
          Option.some.injEq] at hr ⊢
        obtain ⟨u, hu, us, hus, rfl⟩ := hr
        exact ⟨u, compareTypes_ofTokenWith ha a b h1 hu, us,
          compareSignatures_ofSlotTokensWith ha xs ys h2 hus, rfl⟩
      | mutableReference a =>
        cases y <;> simp [compareTypes, reject] at h1
        rename_i b
        simp only [RuntimeType.ofSlotTokensWith, bind, Option.bind_eq_some_iff, pure,
          Option.some.injEq] at hr ⊢
        obtain ⟨u, hu, us, hus, rfl⟩ := hr
        exact ⟨u, compareTypes_ofTokenWith ha a b h1 hu, us,
          compareSignatures_ofSlotTokensWith ha xs ys h2 hus, rfl⟩
      | _ => simp [SignatureToken.isRefToken] at hrx
end

theorem ofSlotTokenWith_nonRef {m : CompiledModule} {tys : List RuntimeType} {t : SignatureToken}
    (hr : t.isRefToken = false) :
    RuntimeType.ofSlotTokenWith m tys t = RuntimeType.ofTokenWith m tys t := by
  cases t <;> first | rfl | simp [SignatureToken.isRefToken] at hr

/-- The same for slot types: references compare their targets. -/
theorem compareTypes_ofSlotTokenWith {m d : CompiledModule} (ha : SameArity m d)
    {tys : List RuntimeType} (t1 t2 : SignatureToken) {u : RuntimeType}
    (h : compareTypes m d t1 t2 = .ok ()) (hu : RuntimeType.ofSlotTokenWith m tys t1 = some u) :
    RuntimeType.ofSlotTokenWith d tys t2 = some u := by
  cases hr : t1.isRefToken
  · have hr2 : t2.isRefToken = false := (compareTypes_isRef h).trans hr
    rw [ofSlotTokenWith_nonRef hr] at hu
    rw [ofSlotTokenWith_nonRef hr2]
    exact compareTypes_ofTokenWith ha t1 t2 h hu
  · cases t1 with
    | reference a =>
      cases t2 <;> simp [compareTypes, reject] at h
      rename_i b
      simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at hu ⊢
      obtain ⟨u', hu', rfl⟩ := hu
      exact ⟨u', compareTypes_ofTokenWith ha a b h hu', rfl⟩
    | mutableReference a =>
      cases t2 <;> simp [compareTypes, reject] at h
      rename_i b
      simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at hu ⊢
      obtain ⟨u', hu', rfl⟩ := hu
      exact ⟨u', compareTypes_ofTokenWith ha a b h hu', rfl⟩
    | _ => simp [SignatureToken.isRefToken] at hr

theorem compareSignatures_ofSignatureWith {m d : CompiledModule} (ha : SameArity m d)
    {tys : List RuntimeType} :
    ∀ {a b : List SignatureToken} {r : List RuntimeType}, compareSignatures m d a b = .ok () →
      RuntimeType.ofSignatureWith m tys a = some r → RuntimeType.ofSignatureWith d tys b = some r
  | [], [], _, _, h => h
  | [], _ :: _, _, h, _ => by simp [compareSignatures, reject] at h
  | _ :: _, [], _, h, _ => by simp [compareSignatures, reject] at h
  | x :: xs, y :: ys, r, h, hr => by
    simp only [compareSignatures] at h
    obtain ⟨h1, h2⟩ := seq_ok2 h
    simp only [RuntimeType.ofSignatureWith, List.mapM_cons, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hr ⊢
    obtain ⟨u, hu, us, hus, rfl⟩ := hr
    exact ⟨u, compareTypes_ofSlotTokenWith ha x y h1 hu, us,
      compareSignatures_ofSignatureWith ha h2 hus, rfl⟩

theorem sameArity {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {mi di : Nat} {m d : CompiledModule} (hm : P.modules[mi]? = some m)
    (hd : P.modules[di]? = some d) : SameArity m d := by
  intro i1 i2 s sh1 sh2 h1 h2 hs1 hs2
  obtain ⟨s1, r1, info1, hs1', hr1, hst1, hc1⟩ := struct_handle_resolves hwf hm hs1
  obtain ⟨s2, r2, info2, hs2', hr2, hst2, hc2⟩ := struct_handle_resolves hwf hd hs2
  have e1 := hs1'.symm.trans h1
  have e2 := hs2'.symm.trans h2
  cases e1; cases e2
  rw [hr1] at hr2
  cases hr2
  rw [hst1] at hst2
  cases hst2
  rw [hc1.2.1, hc2.2.1]

/-- A function instantiation's handle is in bounds. -/
theorem verifyBounds_functionInstantiations {m : CompiledModule} (hb : verifyBounds m = .ok ()) :
    ∀ fi ∈ m.functionInstantiations.toList, fi.handle < m.functionHandles.size := by
  unfold verifyBounds at hb
  simp only at hb
  split at hb
  · simp [reject, bind, Except.bind] at hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨hfi, -⟩ := seq_ok2 hb
  intro fi hfi'
  exact checkIdx_ok (seq_ok2 (forM_ok hfi fi hfi')).1

/-- **K1** for one function handle, generic or not, with its visibility (K2):
it resolves to a visible definition with as many type parameters, whose
constraints the handle's include, and whose parameter and return types are
the handle's at every instance. -/
theorem handle_resolves_full {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) {mi : Nat} {m : CompiledModule} {id : ModuleId}
    (hm : P.modules[mi]? = some m) (hid : m.selfId = some id) {h : FunctionHandleIndex}
    {fh : FunctionHandle} (hfh : m.functionHandles[h]? = some fh) :
    ∃ f info, P.resolve mi h = some f ∧ P.function? f = some info ∧
      P.visibleFrom id f = true ∧
      info.handle.typeParameters.length = fh.typeParameters.length ∧
      (∀ (i : Nat) (c d : AbilitySet), fh.typeParameters[i]? = some c →
        info.handle.typeParameters[i]? = some d → d.subset c = true) ∧
      (∀ tys : List RuntimeType, tys.length = fh.typeParameters.length →
        (∃ ps, P.handleParamTypesAt mi h tys = some ps ∧
          P.paramTypes { f with typeArgs := tys } = some ps) ∧
        (∃ rs, P.handleReturnTypesAt mi h tys = some rs ∧
          P.returnTypes { f with typeArgs := tys } = some rs)) ∧
      (FunctionAttribute.persistent ∈ fh.attributes → P.storable f = true) := by
  have hdist := hwf.1
  have hver : ∀ j m', P.modules[j]? = some m' →
      FunctionsUnique m' ∧ HandlesInBounds m' ∧ m'.Supported := fun j m' hj =>
    let hv := verifyModule_ok (wellFormed_module hwf hj).1
    ⟨verifyDuplication_ok hv.2.1, verifyBounds_handles hv.1, hs.1 _ (module_mem hj)⟩
  obtain ⟨hu, hb, hsup⟩ := hver mi m hm
  obtain ⟨⟨psig, hpsig, hpall⟩, ⟨rsig, hrsig, hrall⟩⟩ := hsup.1 fh (getElem?_mem_toList hfh)
  by_cases hself : fh.module = m.selfModuleHandleIdx
  · obtain ⟨di, d, hres, hd, hdh, hf⟩ := resolve_self hdist hm hid hu hb hfh hself
    refine ⟨_, _, hres, hf, ?_, rfl, fun i c c' hc hc' => ?_, fun tys htys => ?_, fun hp => ?_⟩
    · simp only [Program.visibleFrom, hf]
      cases d.visibility <;> simp
    · simp only at hc'
      rw [hc, Option.some.injEq] at hc'
      subst hc'
      exact AbilitySet.subset_refl _
    · obtain ⟨ps, hps⟩ := Option.isSome_iff_exists.1 (hpall tys htys)
      obtain ⟨rs, hrs⟩ := Option.isSome_iff_exists.1 (hrall tys htys)
      have hf' : P.function? ⟨mi, di, tys⟩ = some ⟨m, id, d, fh⟩ := hf
      exact ⟨⟨ps, by simp [Program.handleParamTypesAt, hm, hfh, hpsig, hps],
          by simp [Program.paramTypes, hf', hpsig, hps]⟩,
        ⟨rs, by simp [Program.handleReturnTypesAt, hm, hfh, hrsig, hrs],
          by simp [Program.returnTypes, hf', hrsig, hrs]⟩⟩
    · simp [Program.storable, hf, hp]
  · obtain ⟨self, deps, hselfId, hdeps, himp⟩ :=
      verifyDependencies_imports (by
        have := (wellFormed_module hwf hm).2
        rw [verifyLinking, show VerifierConfig.production.scope = .everything from rfl] at this
        simpa using this)
    rw [selfModuleId_ok hselfId, Option.some.injEq] at hid
    subst hid
    obtain ⟨ownerId, name, owner, oh, od, dfh, params, dparams, returns, dreturns, hmid, hname,
      howner, hcall, hdfh, hps, hdps, hc1, hrs, hdrs, hc2, hcompat, hpers⟩ :=
      himp fh (getElem?_mem_toList hfh) hself
    obtain ⟨hin, hoid, -⟩ := hdeps _ (dependencyLookup_some howner)
    obtain ⟨mi', -, hmi'⟩ := mem_others hin
    obtain ⟨hu', -, -⟩ := hver mi' owner hmi'
    obtain ⟨fdef, fh', hfd, hfdf, hfh', hfname, hvis⟩ := callableFunction_some hcall
    rw [hdfh, Option.some.injEq] at hfh'
    subst hfh' hfdf
    have hdn : owner.functionDefName fdef = some name := by
      simp [CompiledModule.functionDefName, hdfh, hfname]
    have huniq : Program.uniqueIdx owner.functionDefs
        (fun d => owner.functionDefName d == some name) = some od :=
      uniqueIdx_eq hfd (by simp [hdn]) fun j e hj he => by
        simp only [beq_iff_eq] at he
        exact defName_unique hu' hj hfd he hdn
    have hres : P.resolve mi h = some ⟨mi', od, []⟩ := by
      simp only [Program.resolve, hm, hfh, hmid, hname, findModule_eq hdist hmi' hoid,
        Option.bind_eq_bind, Option.bind_some]
      rw [hmi']
      simp [huniq]
    have hf : P.function? ⟨mi', od, []⟩ = some ⟨owner, ownerId, fdef, dfh⟩ := by
      simp [Program.function?, hmi', hoid, hfd, CompiledModule.functionDefHandle, hdfh]
    rw [hpsig, Option.some.injEq] at hps
    subst hps
    rw [hrsig, Option.some.injEq] at hrs
    subst hrs
    have har := sameArity hwf hm hmi'
    unfold compatibleFunTypeParameters at hcompat
    simp only [Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at hcompat
    obtain ⟨hlen, hall⟩ := hcompat
    refine ⟨_, _, hres, hf, ?_, hlen.symm, fun i c c' hc hc' => ?_, fun tys htys => ?_,
      fun hp => ?_⟩
    · simp only [Program.visibleFrom, hf]
      rcases hvis with hv | ⟨hv, mh, hmh, hmid'⟩
      · simp [hv]
      · simp only [hv, Bool.or_eq_true, beq_iff_eq, Array.any_eq_true]
        have hidx := Array.mem_iff_getElem.1 (Array.mem_toList_iff.1 hmh)
        exact Or.inr ⟨_, hidx.choose_spec.1, by rw [hidx.choose_spec.2, hmid']⟩
    · have hmem : (c, c') ∈ fh.typeParameters.zip dfh.typeParameters := by
        rw [List.mem_iff_getElem?]
        exact ⟨i, by simp [List.getElem?_zip_eq_some, hc, hc']⟩
      simpa [compatibleConstraints] using hall _ hmem
    · obtain ⟨ps, hps⟩ := Option.isSome_iff_exists.1 (hpall tys htys)
      obtain ⟨rs, hrs⟩ := Option.isSome_iff_exists.1 (hrall tys htys)
      have hf' : P.function? ⟨mi', od, tys⟩ = some ⟨owner, ownerId, fdef, dfh⟩ := hf
      exact ⟨⟨ps, by simp [Program.handleParamTypesAt, hm, hfh, hpsig, hps],
          by simp [Program.paramTypes, hf', hdps, compareSignatures_ofSignatureWith har hc1 hps]⟩,
        ⟨rs, by simp [Program.handleReturnTypesAt, hm, hfh, hrsig, hrs],
          by simp [Program.returnTypes, hf', hdrs, compareSignatures_ofSignatureWith har hc2 hrs]⟩⟩
    · simp only [Program.storable, hf]
      rcases hpers hp with hp' | ⟨fd, hfd', hpub⟩
      · simp [hp']
      · rw [hfd, Option.some.injEq] at hfd'
        subst hfd'
        simp [hpub]

theorem handle_resolves_all {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) {mi : Nat} {m : CompiledModule} {id : ModuleId}
    (hm : P.modules[mi]? = some m) (hid : m.selfId = some id) {h : FunctionHandleIndex}
    {fh : FunctionHandle} (hfh : m.functionHandles[h]? = some fh) :
    ∃ f info, P.resolve mi h = some f ∧ P.function? f = some info ∧
      P.visibleFrom id f = true ∧
      info.handle.typeParameters.length = fh.typeParameters.length ∧
      (∀ (i : Nat) (c d : AbilitySet), fh.typeParameters[i]? = some c →
        info.handle.typeParameters[i]? = some d → d.subset c = true) ∧
      ∀ tys : List RuntimeType, tys.length = fh.typeParameters.length →
        (∃ ps, P.handleParamTypesAt mi h tys = some ps ∧
          P.paramTypes { f with typeArgs := tys } = some ps) ∧
        (∃ rs, P.handleReturnTypesAt mi h tys = some rs ∧
          P.returnTypes { f with typeArgs := tys } = some rs) := by
  obtain ⟨f, info, hres, hf, hvis, hlen, hcons, htys, -⟩ := handle_resolves_full hwf hs hm hid hfh
  exact ⟨f, info, hres, hf, hvis, hlen, hcons, htys⟩

/-- **K2**, storability: the link check keeps `persistent` from an imported
handle only when the definition carries it or, carrying no attributes, is
public; a module's own handle is its definition's. -/
theorem persistenceResolution {P : Program}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported) :
    P.PersistenceResolution := by
  intro mi m hm h fh hfh hp
  obtain ⟨id, hid⟩ := module_selfId hwf hm
  obtain ⟨f, -, hres, -, -, -, -, -, hst⟩ := handle_resolves_full hwf hs hm hid hfh
  exact ⟨f, hres, hst hp⟩

/-- **K1–K2** for the model, on the whole stage-A subset, generic code
included. -/
theorem linkedProperties : LinkedProperties verifier := by
  intro P hwf _ hs
  refine ⟨fun mi m hm h fh hfh => ?_, fun mi m hm id hid d hd code hcode => ⟨fun h hmem => ?_,
    fun i x hmem hx => ?_, fun h mask hmem => ?_, fun i mask x hmem _ => ?_⟩,
    fun mi m hm h sh hsh => struct_handle_resolves hwf hm hsh, persistenceResolution hwf hs⟩
  · obtain ⟨id, hid⟩ := module_selfId hwf hm
    obtain ⟨f, info, hres, hf, -, hlen, hcons, htys⟩ := handle_resolves_all hwf hs hm hid hfh
    exact ⟨f, info, hres, hf, hlen, hcons, htys⟩
  · have hb := (verifyModule_ok (wellFormed_module hwf hm).1).1
    obtain ⟨fh, hfh⟩ := getElem?_of_lt (call_bound hb hd hcode hmem)
    obtain ⟨f, -, hres, -, hvis, -⟩ := handle_resolves_all hwf hs hm hid hfh
    exact ⟨f, hres, hvis⟩
  · have hb := (verifyModule_ok (wellFormed_module hwf hm).1).1
    obtain ⟨fh, hfh⟩ :=
      getElem?_of_lt (verifyBounds_functionInstantiations hb x (getElem?_mem_toList hx))
    obtain ⟨f, -, hres, -, hvis, -⟩ := handle_resolves_all hwf hs hm hid hfh
    exact ⟨f, hres, hvis⟩
  · -- A closure's target resolves, visibly, as a call's does.
    have hb := (verifyModule_ok (wellFormed_module hwf hm).1).1
    obtain ⟨fh, hfh⟩ := getElem?_of_lt (packClosure_bound hb hd hcode hmem)
    obtain ⟨f, -, hres, -, hvis, -⟩ := handle_resolves_all hwf hs hm hid hfh
    exact ⟨f, hres, hvis⟩
  · have hb := (verifyModule_ok (wellFormed_module hwf hm).1).1
    obtain ⟨fh, hfh⟩ :=
      getElem?_of_lt (verifyBounds_functionInstantiations hb x (getElem?_mem_toList ‹_›))
    obtain ⟨f, -, hres, -, hvis, -⟩ := handle_resolves_all hwf hs hm hid hfh
    exact ⟨f, hres, hvis⟩

end MoveBytecode.Proofs
