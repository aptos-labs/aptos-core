-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Linking
import Proofs.BorrowGraph

/-!
# The acquires annotations of an accepted program

UNTRUSTED (checked by the kernel). What the acquires verifier and the
reference-safety analysis's call establish about global storage: a function
annotates every type it moves from or borrows globally, and every type a
function of its own module that it calls annotates; the analysis's call
checks the callee's annotations against the caller's global borrows. A call
of a function of another module enters a module with another id.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

/-! ## The acquires verifier -/

/-- **What the acquires verifier checks of each instruction** of a function
with code: a global operation's type is annotated, and so is every type that
a called function of the same module annotates. -/
theorem acquires_instr {m : CompiledModule} {fdef : FunctionDefinition} {code : CodeUnit}
    (h : verifyAcquires m fdef code = .ok ()) {i : Bytecode} (hi : i ∈ code.code.toList) :
    (∀ d, (i = .moveFrom d ∨ i = .mutBorrowGlobal d ∨ i = .immBorrowGlobal d) →
      d ∈ fdef.acquiresGlobalResources) ∧
    (∀ hh, i = .call hh → ∀ acq, Acquires.functionAcquires m hh = .ok acq →
      ∀ d ∈ acq, d ∈ fdef.acquiresGlobalResources) := by
  unfold verifyAcquires at h
  simp only [bind_ok] at h
  obtain ⟨actual, hfold, -⟩ := h
  obtain ⟨b, r', hstep⟩ := foldlM_ok_mem hfold i hi
  constructor
  · intro d hd
    have hs : Acquires.structAcquire fdef.acquiresGlobalResources b d = .ok r' := by
      rcases hd with rfl | rfl | rfl <;> exact hstep
    simp only [Acquires.structAcquire] at hs
    split at hs
    · next hc => simpa using hc
    · simp [reject] at hs
  · rintro hh rfl acq hacq d hd
    simp only [Acquires.instr, Acquires.callAcquire, bind_ok] at hstep
    obtain ⟨a, ha, hstep⟩ := hstep
    rw [hacq, Except.ok.injEq] at ha
    subst ha
    split at hstep
    · simp [reject, Functor.map, Except.map] at hstep
    · next hc =>
      simp only [List.any_eq_true, Bool.not_eq_eq_eq_not, Bool.not_true, not_exists,
        not_and] at hc
      have := hc d hd
      simpa using this

/-- The acquires a call of a same-module handle adds: the definition's
annotations. -/
theorem functionAcquires_self {m : CompiledModule} (hu : FunctionsUnique m) {h : Nat}
    {fh : FunctionHandle} (hfh : m.functionHandles[h]? = some fh)
    (hself : fh.module = m.selfModuleHandleIdx) {di : Nat} {d : FunctionDefinition}
    (hd : m.functionDefs[di]? = some d) (hdh : d.function = h) :
    Acquires.functionAcquires m h = .ok d.acquiresGlobalResources := by
  have hmem : d ∈ m.functionDefs.toList.filter (·.function == h) :=
    List.mem_filter.2 ⟨getElem?_mem_toList hd, by simp [hdh]⟩
  obtain ⟨d', hd'⟩ : ∃ d', (m.functionDefs.toList.filter (·.function == h)).getLast? = some d' := by
    cases hl : (m.functionDefs.toList.filter (·.function == h)).getLast? with
    | none => rw [List.getLast?_eq_none_iff] at hl; rw [hl] at hmem; cases hmem
    | some d' => exact ⟨d', rfl⟩
  have hd'mem := List.mem_filter.1 (List.mem_of_getLast? hd')
  obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hd'mem.1
  rw [toList_getElem?] at hj
  have hjf : d'.function = h := by simpa using hd'mem.2
  have hji : j = di := nodup_map_index hu.defs (by rw [toList_getElem?]; exact hj)
    (by rw [toList_getElem?]; exact hd) (by simp [hjf, hdh])
  subst hji
  rw [hd, Option.some.injEq] at hj
  subst hj
  simp [Acquires.functionAcquires, lookup_ok.2 hfh, hself, Acquires.handleToDef, hd', bind,
    Except.bind, pure, Except.pure]

/-! ## The analysis's call -/

theorem filterAuxM_ok {α ε : Type} {f : α → Except ε Bool} :
    ∀ (l acc r : List α), List.filterAuxM f l acc = .ok r →
      ∀ x, x ∈ r ↔ x ∈ acc ∨ (x ∈ l ∧ f x = .ok true)
  | [], acc, r, h, x => by
    simp only [List.filterAuxM, pure, Except.pure, Except.ok.injEq] at h
    subst h; simp
  | y :: l, acc, r, h, x => by
    simp only [List.filterAuxM, bind_ok] at h
    obtain ⟨b, hb, h⟩ := h
    rw [filterAuxM_ok l _ r h x]
    cases b with
    | true =>
      simp only [cond_true, List.mem_cons]
      constructor
      · rintro ((rfl | h) | ⟨h1, h2⟩)
        · exact Or.inr ⟨Or.inl rfl, hb⟩
        · exact Or.inl h
        · exact Or.inr ⟨Or.inr h1, h2⟩
      · rintro (h | ⟨rfl | h1, h2⟩)
        · exact Or.inl (Or.inr h)
        · exact Or.inl (Or.inl rfl)
        · exact Or.inr ⟨h1, h2⟩
    | false =>
      simp only [cond_false, List.mem_cons]
      constructor
      · rintro (h | ⟨h1, h2⟩)
        · exact Or.inl h
        · exact Or.inr ⟨Or.inr h1, h2⟩
      · rintro (h | ⟨rfl | h1, h2⟩)
        · exact Or.inl h
        · rw [hb] at h2; cases h2
        · exact Or.inr ⟨h1, h2⟩

theorem filterM_ok {α ε : Type} {f : α → Except ε Bool} {l r : List α}
    (h : l.filterM f = .ok r) : ∀ x, x ∈ r ↔ x ∈ l ∧ f x = .ok true := by
  unfold List.filterM at h
  simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨r', h', rfl⟩ := h
  intro x
  rw [List.mem_reverse, filterAuxM_ok l [] r' h' x]
  simp

theorem mem_foldl_insertNat {x : Nat} :
    ∀ (l acc : List Nat), x ∈ l.foldl (fun s y => insertNat y s) acc ↔ x ∈ acc ∨ x ∈ l
  | [], acc => by simp
  | y :: l, acc => by
    rw [List.foldl_cons, mem_foldl_insertNat l, mem_insertNat]
    simp only [List.mem_cons]
    constructor
    · rintro ((h | h) | h)
      · exact Or.inr (Or.inl h)
      · exact Or.inl h
      · exact Or.inr (Or.inr h)
    · rintro (h | h | h)
      · exact Or.inl (Or.inr h)
      · exact Or.inl (Or.inl h)
      · exact Or.inr h

/-- **The analysis's call of a same-module handle checks the callee's
annotations.** -/
theorem acquiredBy_self {m : CompiledModule} (hu : FunctionsUnique m) {h : Nat} {fh : FunctionHandle}
    (hfh : m.functionHandles[h]? = some fh) (hself : fh.module = m.selfModuleHandleIdx)
    {di : Nat} {d : FunctionDefinition} (hd : m.functionDefs[di]? = some d) (hdh : d.function = h)
    {acq : List Nat} (hacq : ReferenceSafety.acquiredBy m fh = .ok acq) :
    ∀ x ∈ d.acquiresGlobalResources, x ∈ acq := by
  unfold ReferenceSafety.acquiredBy ReferenceSafety.nameDef at hacq
  simp only [bind_ok] at hacq
  obtain ⟨o, ho, hacq⟩ := hacq
  obtain ⟨named, hnamed, ho⟩ := ho
  simp only [pure, Except.pure, Except.ok.injEq] at ho
  subst ho
  have hmemN := filterM_ok hnamed
  -- Every definition whose handle has the callee's name is the callee's.
  have hall : ∀ p ∈ named, p = (d, di) := by
    rintro ⟨e, j⟩ hp
    obtain ⟨hmem, hok⟩ := (hmemN _).1 hp
    rw [List.mem_zipIdx_iff_getElem?] at hmem
    rw [toList_getElem?] at hmem
    simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at hok
    obtain ⟨eh, heh, hname⟩ := hok
    obtain ⟨eh', heh', hemod⟩ := hu.ownDefs e (getElem?_mem_toList hmem)
    rw [heh, Option.some.injEq] at heh'
    subst heh'
    have hfun : e.function = h := nodup_map_index hu.handles (by rw [toList_getElem?]; exact heh)
      (by rw [toList_getElem?]; exact hfh) (by simp [hemod, hself, (beq_iff_eq.1 hname)])
    have hj : j = di := nodup_map_index hu.defs (by rw [toList_getElem?]; exact hmem)
      (by rw [toList_getElem?]; exact hd) (by simp [hfun, hdh])
    subst hj
    rw [hd, Option.some.injEq] at hmem
    rw [hmem]
  have hin : (d, di) ∈ named := by
    refine (hmemN _).2 ⟨?_, ?_⟩
    · rw [List.mem_zipIdx_iff_getElem?]; simpa using hd
    · simp [bind_ok, lookup_ok, hdh, hfh, pure, Except.pure]
  obtain ⟨p, hp⟩ : ∃ p, named.getLast? = some p := by
    cases hl : named.getLast? with
    | none => rw [List.getLast?_eq_none_iff] at hl; rw [hl] at hin; cases hin
    | some p => exact ⟨p, rfl⟩
  have hpe := hall p (List.mem_of_getLast? hp)
  subst hpe
  simp only [hp, Option.map_some, bind_ok, lookup_ok] at hacq
  obtain ⟨fd, hfd, h', hh', hacq⟩ := hacq
  rw [hd, Option.some.injEq] at hfd
  subst hfd
  rw [hdh, hfh, Option.some.injEq] at hh'
  subst hh'
  simp only [beq_self_eq_true, ↓reduceIte, pure, Except.pure, Except.ok.injEq] at hacq
  subst hacq
  intro x hx
  exact (mem_foldl_insertNat _ [] ).2 (Or.inr hx)

/-! ## Module identities -/

theorem uniqueIdx_some {α : Type} {a : Array α} {p : α → Bool} {i : Nat}
    (h : Program.uniqueIdx a p = some i) : ∃ y, a[i]? = some y ∧ p y = true := by
  unfold Program.uniqueIdx at h
  split at h
  · next j heq =>
    simp only [Option.some.injEq] at h
    subst h
    have hmem := List.mem_singleton_self j
    rw [← heq, List.mem_filter] at hmem
    obtain ⟨-, hp⟩ := hmem
    split at hp
    · next y hy => exact ⟨y, hy, hp⟩
    · simp at hp
  · simp at h

theorem moduleIdAt_inj {m : CompiledModule} (hu : FunctionsUnique m) {i j : Nat} {x : ModuleId}
    (hi : m.moduleIdAt i = some x) (hj : m.moduleIdAt j = some x) : i = j := by
  simp only [CompiledModule.moduleIdAt, CompiledModule.moduleIdOf, bind, Option.bind_eq_some_iff,
    pure, Option.some.injEq] at hi hj
  obtain ⟨mh, hmh, a, ha, n, hn, rfl⟩ := hi
  obtain ⟨mh', hmh', a', ha', n', hn', he⟩ := hj
  simp only [ModuleId.mk.injEq] at he
  obtain ⟨rfl, rfl⟩ := he
  have h1 : mh.address = mh'.address := nodup_index hu.addresses
    (by rw [toList_getElem?]; exact ha) (by rw [toList_getElem?]; exact ha')
  have h2 : mh.name = mh'.name := nodup_index hu.identifiers
    (by rw [toList_getElem?]; exact hn) (by rw [toList_getElem?]; exact hn')
  have : mh = mh' := by cases mh; cases mh'; simp_all
  subst this
  exact nodup_index hu.moduleHandles (by rw [toList_getElem?]; exact hmh)
    (by rw [toList_getElem?]; exact hmh')

/-- **A call of another module's handle enters a module with another id.** -/
theorem resolve_cross {P : Program} {mi : Nat} {m : CompiledModule}
    {id : ModuleId} (hm : P.modules[mi]? = some m) (hid : m.selfId = some id)
    (hu : FunctionsUnique m) {h : Nat} {fh : FunctionHandle}
    (hfh : m.functionHandles[h]? = some fh) (hcross : fh.module ≠ m.selfModuleHandleIdx)
    {callee : FunctionRef} (hres : P.resolve mi h = some callee) {cinfo : FunctionInfo}
    (hcf : P.function? callee = some cinfo) : cinfo.moduleId ≠ id := by
  simp only [Program.resolve, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hres
  obtain ⟨m', hm', fh', hfh', targetId, htarget, name, -, mi', hmi', target, htm, di, -, rfl⟩ :=
    hres
  rw [hm, Option.some.injEq] at hm'
  subst hm'
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  have hcf' := function?_parts hcf
  rw [htm, Option.some.injEq] at hcf'
  have htsel : target.selfId = some targetId := by
    unfold Program.findModule at hmi'
    have := uniqueIdx_some hmi'
    simp only [htm, beq_iff_eq] at this
    obtain ⟨t', ht', hsel⟩ := this
    rw [Option.some.injEq] at ht'
    subst ht'
    exact hsel
  have hcm : cinfo.moduleId = targetId := by
    rw [hcf'.1, hcf'.2.1, Option.some.injEq] at htsel
    exact htsel
  rw [hcm]
  intro e
  subst e
  exact hcross (moduleIdAt_inj hu htarget hid)

end MoveBytecode.Proofs
