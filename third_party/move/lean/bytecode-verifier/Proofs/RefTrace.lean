-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostModeled

/-!
# References along annotated executions

UNTRUSTED (checked by the kernel). The facts about references that D4 needs
of every annotated execution, for any program and natives:

- a step uses only references its current frame holds, with their
  identities (`exec_used`, `traced_used`);
- every reference an execution holds was created by a `borrowed` event, and
  `borrowed` events have distinct identities below the fresh counter
  (`RefTraceInv`, `refTraceInv_trace`);
- a step's frames hold only references they held before or that the step
  borrows (`traced_frameRefs`).
-/

namespace MoveBytecode.Proofs

/-! ## The references a step uses -/

/-- Reference `r`, with identity `i` in shadow frame `g`, is a slot of frame
`fr`: on its operand stack or in a local. -/
def RefSlot (g : ShadowFrame) (fr : Frame) (i : Nat) (r : Reference) : Prop :=
  ∃ t : IdTree, t.id = i ∧ ((t, Value.ref r) ∈ g.stack.zip fr.stack ∨
    (some t, some (Value.ref r)) ∈ g.locals.toList.zip fr.locals.toList)

/-- Reference `r`, with identity `i`, is on the operand stack of `fr`. -/
def StackSlot (g : ShadowFrame) (fr : Frame) (i : Nat) (r : Reference) : Prop :=
  ∃ t : IdTree, t.id = i ∧ (t, Value.ref r) ∈ g.stack.zip fr.stack

/-- Reference `r`, with identity `i`, is local `j` of `fr`. -/
def LocalSlot (g : ShadowFrame) (fr : Frame) (j i : Nat) (r : Reference) : Prop :=
  ∃ t : IdTree, t.id = i ∧ g.locals[j]? = some (some t) ∧ fr.locals[j]? = some (some (Value.ref r))

theorem StackSlot.refSlot {g : ShadowFrame} {fr : Frame} {i : Nat} {r : Reference}
    (h : StackSlot g fr i r) : RefSlot g fr i r := by
  obtain ⟨t, ht, hz⟩ := h
  exact ⟨t, ht, Or.inl hz⟩


theorem zip_of_getElem? {α β : Type} {as : List α} {bs : List β} {k : Nat} {a : α} {b : β}
    (ha : as[k]? = some a) (hb : bs[k]? = some b) : (a, b) ∈ as.zip bs := by
  have : (as.zip bs)[k]? = some (a, b) := by simp [List.getElem?_zip_eq_some, ha, hb]
  exact List.mem_of_getElem? this

theorem LocalSlot.refSlot {g : ShadowFrame} {fr : Frame} {j i : Nat} {r : Reference}
    (h : LocalSlot g fr j i r) : RefSlot g fr i r := by
  obtain ⟨t, ht, hg, hf⟩ := h
  refine ⟨t, ht, Or.inr (zip_of_getElem? (k := j) ?_ ?_)⟩
  · rw [Array.getElem?_toList]; exact hg
  · rw [Array.getElem?_toList]; exact hf

/-- The reference at depth `k` of the segment, with its identity. -/
theorem refAt_slot {g : ShadowFrame} {fr : Frame} {k : Nat} {a : Nat × Reference}
    (h : (match fr.stack[k]? with
      | some (Value.ref r) => g.stack[k]?.bind fun t => pure (t.id, r)
      | _ => none) = some a) : StackSlot g fr a.1 a.2 := by
  split at h
  · next r hr =>
    simp only [Option.bind_eq_some_iff, Option.pure_def, Option.some.injEq] at h
    obtain ⟨t, ht, rfl⟩ := h
    exact ⟨t, rfl, zip_of_getElem? ht hr⟩
  · cases h

theorem fitsAll_of_zip : ∀ {ts : List IdTree} {vs : List Value}, IdTree.fitsAll ts vs = true →
    ∀ {t : IdTree} {v : Value}, (t, v) ∈ ts.zip vs → t.fits v = true
  | [], [], _, _, _, h => by simp at h
  | t' :: ts, v' :: vs, h, t, v, hm => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    rcases List.mem_cons.1 hm with he | hm
    · simp only [Prod.mk.injEq] at he
      obtain ⟨rfl, rfl⟩ := he
      exact h.1
    · exact fitsAll_of_zip h.2 hm
  | [], _ :: _, _, _, _, h => by simp at h
  | _ :: _, [], _, _, _, h => by simp at h

theorem occ_of_zip : ∀ {ts : List IdTree} {vs : List Value} {t : IdTree} {v : Value},
    (t, v) ∈ ts.zip vs → ∀ o ∈ t.occurrences v, o ∈ IdTree.occurrencesAll ts vs
  | [], _, _, _, h => by simp at h
  | _ :: _, [], _, _, h => by simp at h
  | t' :: ts, v' :: vs, t, v, hm => by
    intro o ho
    simp only [IdTree.occurrencesAll, List.mem_append]
    rcases List.mem_cons.1 hm with he | hm
    · simp only [Prod.mk.injEq] at he
      obtain ⟨rfl, rfl⟩ := he
      exact Or.inl ho
    · exact Or.inr (occ_of_zip hm o ho)

/-- A reference slot is an occurrence of its identity. -/
theorem refSlot_occ {g : ShadowFrame} {fr : Frame} (hg : g.fits fr = true) {i : Nat} {r : Reference}
    (h : RefSlot g fr i r) : (i, Value.ref r) ∈ g.occurrences fr := by
  obtain ⟨t, rfl, hs | hl⟩ := h
  · have hft := fitsAll_of_zip (frame_stack_fits hg) hs
    rw [frame_occurrences]
    refine List.mem_append_right _ (occ_of_zip hs _ ?_)
    rw [ref_occ hft]; exact List.mem_singleton_self _
  · rw [frame_fits] at hg
    simp only [Bool.and_eq_true] at hg
    have hsl : ShadowFrame.slotFits (some t) (some (Value.ref r)) = true := by
      have := hg.1.2
      simp only [slotsFit, List.all_eq_true] at this
      exact this _ hl
    rw [frame_occurrences]
    refine List.mem_append_left _ ?_
    simp only [localsOcc, List.mem_flatMap]
    refine ⟨_, hl, ?_⟩
    simp only [ShadowFrame.slotOccurrences, ShadowFrame.slotFits] at hsl ⊢
    rw [ref_occ hsl]; exact List.mem_singleton_self _


theorem fresh_not_used {c : Creator} {v : Value} {n i : Nat} {r : Reference} :
    Event.used i r ∉ (IdTree.fresh c v n).2.2 := by
  rw [(fresh_spec c v n).2.2.2]; simp

theorem mem_zip_getElem? {α β : Type} {as : List α} {bs : List β} {a : α} {b : β}
    (h : (a, b) ∈ as.zip bs) : ∃ k : Nat, as[k]? = some a ∧ bs[k]? = some b := by
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 h
  rw [List.getElem?_zip_eq_some] at hk
  exact ⟨k, hk.1, hk.2⟩

/-- A pair of the reversed prefixes of two lists of one length is a pair of the
lists. -/
theorem zip_rev_take {α β : Type} {as : List α} {bs : List β} (hlen : as.length = bs.length)
    {n : Nat} {a : α} {b : β} (h : (a, b) ∈ (as.take n).reverse.zip (bs.take n).reverse) :
    (a, b) ∈ as.zip bs := by
  obtain ⟨k, ha, hb⟩ := mem_zip_getElem? h
  have hm : (as.take n).length = (bs.take n).length := by simp [hlen]
  have hka : k < (as.take n).length := by
    have := (List.getElem?_eq_some_iff.1 ha).1; simpa using this
  rw [List.getElem?_reverse hka] at ha
  rw [List.getElem?_reverse (by rw [← hm]; exact hka), ← hm] at hb
  rw [List.getElem?_take] at ha hb
  split at ha
  · rw [if_pos (by assumption)] at hb
    exact zip_of_getElem? ha hb
  · cases ha

/-- A pair of two reversed lists of one length is a pair of the lists. -/
theorem zip_rev {α β : Type} {as : List α} {bs : List β} (hlen : as.length = bs.length)
    {a : α} {b : β} (h : (a, b) ∈ as.reverse.zip bs.reverse) : (a, b) ∈ as.zip bs := by
  refine zip_rev_take (n := as.length) hlen ?_
  rwa [List.take_length, List.take_of_length_le (Nat.le_of_eq hlen.symm)]

/-- **The references a step uses** are slots of its current frame, with their
identities. -/
theorem exec_used {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} (hh : h.frames = g :: hs) (hg : g.fits fr = true)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) :
    ∀ i r, Event.used i r ∈ evs → StackSlot g fr i r ∨
      ∃ j, (instr = .copyLoc j ∨ instr = .moveLoc j) ∧ LocalSlot g fr j i r := by
  intro i r hu
  have hlen : g.stack.length = fr.stack.length := fitsAll_length (frame_stack_fits hg)
  cases instr <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Prod.mk.injEq, guard, reduceCtorEq] at hex
  all_goals (try simp_all)
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldConst | ldTrue | ldFalse =>
    obtain ⟨a, -, -, rfl⟩ := hex
    exact absurd hu fresh_not_used
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 | not | negate | add | sub | mul | div | mod | bitOr | bitAnd
    | xor | or | and | lt | gt | le | ge | shl | shr =>
    obtain ⟨-, a, -, -, rfl⟩ := hex
    exact absurd hu fresh_not_used
  case pack | packVariant | vecPack | packGeneric | packVariantGeneric | packClosure
    | packClosureGeneric =>
    obtain ⟨a, -, -, -, rfl⟩ := hex
    simp at hu
  case unpack | unpackVariant | vecUnpack | unpackGeneric | unpackVariantGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨v, -, -, rfl⟩ := hex
      simp at hu
    · cases hex
  case eq | neq =>
    obtain ⟨a, b, ⟨-, a₁, -, -, rfl⟩, hm⟩ := hex
    split at hm
    · next _ _ ta ra tb rb h1 h0 =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or] at hu
      rcases hu with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩ | hu
      · exact refAt_slot (a := (i, r)) h1
      · exact refAt_slot (a := (i, r)) h0
      · exact absurd hu fresh_not_used
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact absurd hu fresh_not_used
  case readRef =>
    obtain ⟨a, b, hra, a₁, -, -, rfl⟩ := hex
    simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.not_mem_nil,
      or_false] at hu
    obtain ⟨rfl, rfl⟩ := hu
    exact refAt_slot (a := (i, r)) hra
  case writeRef =>
    obtain ⟨a, b, hra, a₁, -, a₂, -, -, rfl⟩ := hex
    simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.not_mem_nil,
      or_false] at hu
    obtain ⟨rfl, rfl⟩ := hu
    exact refAt_slot (a := (i, r)) hra
  case freezeRef =>
    obtain ⟨a, b, hra, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, List.not_mem_nil,
        or_false] at hu
      obtain ⟨rfl, rfl⟩ := hu
      exact refAt_slot (a := (i, r)) hra
    · cases hm
  case testVariant | testVariantGeneric =>
    obtain ⟨a, b, hra, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨x, hx, -, rfl⟩ := hm
      obtain ⟨-, -, v, -, rfl⟩ := hx
      simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.mem_append,
        ] at hu
      rcases hu with ⟨rfl, rfl⟩ | hu | hu
      · exact refAt_slot (a := (i, r)) hra
      · exact absurd hu fresh_not_used
      · cases hu
    · cases hm
  case mutBorrowField | immBorrowField | mutBorrowVariantField | immBorrowVariantField
    | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
    | immBorrowVariantFieldGeneric =>
    obtain ⟨a, b, hra, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff] at hm
      obtain ⟨-, -, hm⟩ := hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.not_mem_nil,
          or_false] at hu
        obtain ⟨rfl, rfl⟩ := hu
        exact refAt_slot (a := (i, r)) hra
      · cases hm
    · cases hm
  case vecLen =>
    obtain ⟨a, b, hra, a₁, b₁, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or] at hu
    rcases hu with ⟨rfl, rfl⟩ | hu
    · exact refAt_slot (a := (i, r)) hra
    · exact absurd hu fresh_not_used
  case vecImmBorrow | vecMutBorrow =>
    obtain ⟨a, b, hra, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.not_mem_nil,
        or_false] at hu
      obtain ⟨rfl, rfl⟩ := hu
      exact refAt_slot (a := (i, r)) hra
    · cases hm
  case vecPushBack =>
    obtain ⟨a, -, a₁, b, hra, a₂, -, -, rfl⟩ := hex
    simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.not_mem_nil,
      or_false] at hu
    obtain ⟨rfl, rfl⟩ := hu
    exact refAt_slot (a := (i, r)) hra
  case vecPopBack =>
    obtain ⟨a, b, hra, a₁, -, a₂, -, a₃, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.not_mem_nil,
        or_false] at hu
      obtain ⟨rfl, rfl⟩ := hu
      exact refAt_slot (a := (i, r)) hra
    · cases hm
  case vecSwap =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨x, hra, y, -, -, rfl⟩ := hex
        simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.not_mem_nil,
          or_false] at hu
        obtain ⟨rfl, rfl⟩ := hu
        exact refAt_slot hra
      · cases hex
    · cases hex
  case moveTo | moveToGeneric =>
    obtain ⟨a, -, a₁, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff] at hm
      obtain ⟨x, hra, hm⟩ := hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        simp only [List.mem_cons, Event.used.injEq, reduceCtorEq, false_or, List.not_mem_nil,
          or_false] at hu
        obtain ⟨rfl, rfl⟩ := hu
        exact refAt_slot hra
      · cases hm
    · cases hm
  case mutBorrowLoc | immBorrowLoc =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp at hu
    · cases hex
  case mutBorrowGlobal | immBorrowGlobal | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        simp at hu
      · cases hex
    · cases hex
  case exists_ =>
    obtain ⟨a, -, a₁, b, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_append, List.mem_singleton, reduceCtorEq, or_false] at hu
    exact absurd hu fresh_not_used
  case existsGeneric =>
    obtain ⟨cm, -, x, -, sid, -, targs, -, a, b, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_append, List.mem_singleton, reduceCtorEq, or_false] at hu
    exact absurd hu fresh_not_used
  case moveFrom | moveFromGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff] at hex
      obtain ⟨v, -, hex⟩ := hex
      split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨k, -, t, -, -, rfl⟩ := hex
        simp at hu
      · cases hex
    · cases hex
  case copyLoc j =>
    split at hex
    · next t ht =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, reduceCtorEq, false_or] at hu
      split at hu
      · next r' hr' =>
        simp only [List.mem_singleton, Event.used.injEq] at hu
        obtain ⟨rfl, rfl⟩ := hu
        exact Or.inr ⟨t, rfl, ht, hr'⟩
      · simp at hu
    · cases hex
  case moveLoc j =>
    split at hex
    · next t ht =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, reduceCtorEq, false_or, List.mem_append, 
        ] at hu
      split at hu
      · next r' hr' =>
        simp only [List.mem_singleton, Event.used.injEq, List.not_mem_nil, or_false] at hu
        obtain ⟨rfl, rfl⟩ := hu
        exact Or.inr ⟨t, rfl, ht, hr'⟩
      · simp at hu
    · cases hex
  case stLoc j =>
    split at hex
    · next t st hgs =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, reduceCtorEq, false_or] at hu
      split at hu
      · next r' hr' =>
        simp only [List.mem_singleton, Event.used.injEq] at hu
        obtain ⟨rfl, rfl⟩ := hu
        refine ⟨t, rfl, zip_of_getElem? (k := 0) (by simp [hgs]) ?_⟩
        rw [List.head?_eq_getElem?] at hr'; exact hr'
      · simp at hu
    · cases hex
  case callClosure =>
    split at hex
    · split at hex
      · split at hex
        · next c fn mask captured params results abilities hc =>
          split at hex
          · next ct st hgs =>
            simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
            obtain ⟨-, -, -, rfl⟩ := hex
            simp only [List.mem_append, List.mem_map, List.mem_filterMap, List.mem_cons,
              List.not_mem_nil, reduceCtorEq, or_false] at hu
            obtain ⟨⟨i', r'⟩, ⟨⟨t, v⟩, hz, hv⟩, he⟩ := hu
            simp only [Event.used.injEq] at he
            obtain ⟨rfl, rfl⟩ := he
            split at hv
            · next r'' hv' =>
              simp only [Option.some.injEq, Prod.mk.injEq] at hv
              obtain ⟨rfl, rfl⟩ := hv
              simp only at hv'
              rw [hv'] at hz
              cases hfs : fr.stack with
              | nil => simp [hfs] at hc
              | cons c' rest =>
                rw [hfs] at hz hlen
                rw [hgs] at hlen
                simp only [List.length_cons, Nat.add_right_cancel_iff] at hlen
                try simp only [List.drop_succ_cons, List.drop_zero] at hz
                have hz' := zip_rev_take hlen hz
                exact ⟨t, rfl, by rw [hgs, hfs]; exact List.mem_cons_of_mem _ hz'⟩
            · cases hv
          · cases hex
        · cases hex
      · cases hex
    · cases hex
  case call | callGeneric =>
    split at hex
    · split at hex
      · simp only [Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        simp only [List.mem_append, List.mem_map, List.mem_filterMap, List.mem_singleton,
          reduceCtorEq, or_false] at hu
        obtain ⟨⟨i', r'⟩, ⟨⟨t, v⟩, hz, hv⟩, he⟩ := hu
        simp only [Event.used.injEq] at he
        obtain ⟨rfl, rfl⟩ := he
        split at hv
        · next r'' hv' =>
          simp only [Option.some.injEq, Prod.mk.injEq] at hv
          obtain ⟨rfl, rfl⟩ := hv
          simp only at hv'
          rw [hv'] at hz
          exact ⟨t, rfl, zip_rev_take (fitsAll_length (frame_stack_fits hg)) hz⟩
        · cases hv
      · cases hex
    · cases hex
  case ret =>
    have hret : ∀ evs', evs' = List.map (fun x => Event.used x.fst x.snd)
        (List.filterMap (fun x => match x.snd with
          | Value.ref r => some (x.fst.id, r)
          | _ => none) (g.stack.reverse.zip fr.stack.reverse)) ++
        [Event.frameEnded fr.id (List.map (fun x => x.fst) (List.filterMap (fun x => match x.snd with
          | Value.ref r => some (x.fst.id, r)
          | _ => none) (g.stack.reverse.zip fr.stack.reverse)))] →
        Event.used i r ∈ evs' → StackSlot g fr i r := by
      rintro evs' rfl hu
      simp only [List.mem_append, List.mem_map, List.mem_filterMap, List.mem_singleton,
        reduceCtorEq, or_false] at hu
      obtain ⟨⟨i', r'⟩, ⟨⟨t, v⟩, hz, hv⟩, he⟩ := hu
      simp only [Event.used.injEq] at he
      obtain ⟨rfl, rfl⟩ := he
      split at hv
      · next r'' hv' =>
        simp only [Option.some.injEq, Prod.mk.injEq] at hv
        obtain ⟨rfl, rfl⟩ := hv
        simp only at hv'
        rw [hv'] at hz
        exact ⟨t, rfl, zip_rev (fitsAll_length (frame_stack_fits hg)) hz⟩
      · cases hv
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact hret _ rfl hu
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact hret _ rfl hu


/-! ## The references of an annotated execution -/

/-- The identities of the `borrowed` events. -/
def borrowedIds (evs : List Event) : List Nat :=
  evs.filterMap fun e => match e with
    | .borrowed d _ _ => some d
    | .created .. | .consumed .. | .used .. | .accessed .. | .vacated .. | .entered ..
    | .frameEnded .. | .privileged .. => none

@[simp] theorem borrowedIds_append (a b : List Event) :
    borrowedIds (a ++ b) = borrowedIds a ++ borrowedIds b := by
  simp [borrowedIds, List.filterMap_append]

theorem mem_borrowedIds {evs : List Event} {d : Nat} :
    d ∈ borrowedIds evs ↔ ∃ r par, Event.borrowed d r par ∈ evs := by
  simp only [borrowedIds, List.mem_filterMap]
  constructor
  · rintro ⟨e, he, hd⟩
    cases e
    case borrowed d' r p =>
      simp only [Option.some.injEq] at hd
      subst hd
      exact ⟨r, p, he⟩
    all_goals simp at hd
  · rintro ⟨r, par, he⟩
    exact ⟨_, he, rfl⟩

/-- **The references a native call borrows** have distinct identities from
the fresh counter on. -/
theorem native_borrowedIds {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs)) :
    (borrowedIds evs).Nodup ∧ ∀ d ∈ borrowedIds evs, h.fresh ≤ d := by
  refine ⟨?_, fun d hd => ?_⟩
  · obtain ⟨g, hs, info, tys, ptys, seen, s', fr', tl, newRefs, resultTrees, h₂, -, -, -, -, -,
      -, -, -, -, -, -, -, hnew, -, -, -, -, hevs⟩ := nativeStep_spec hn
    have hb : borrowedIds evs = newRefs.map (·.1) := by
      rw [hevs]
      have hnil : ∀ l : List Event, (∀ e ∈ l, ∀ d r par, e ≠ .borrowed d r par) →
          borrowedIds l = [] := fun l hl => List.eq_nil_iff_forall_not_mem.2 fun d hd => by
        obtain ⟨r, par, hm⟩ := mem_borrowedIds.1 hd
        exact hl _ hm d r par rfl
      have h1 := hnil (consumedOfNative P f info.moduleId ptys seen views
          (passedOn (ν f views (vals ++ writes)) (vals ++ writes))) fun e he d r par hed => by
        obtain ⟨_, _, _, he'⟩ := consumedOfNative_mem he
        rw [he'] at hed; cases hed
      have h2 := hnil (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).2.2 fun e he d r par hed => by
        obtain ⟨_, _, he'⟩ := (outTreesOf_spec _ _ _ _ _).2.2.1 e he
        rw [he'] at hed; cases hed
      have hfn : ∀ {α : Type} (l : List α), l.filterMap (fun _ => (none : Option Nat)) = [] :=
        fun l => by induction l <;> simp_all
      have hfs : ∀ l : List (Nat × Reference × Nat),
          l.filterMap (fun x => some x.1) = l.map (·.1) :=
        fun l => by induction l <;> simp_all
      simp only [nativeEventsOf, borrowedIds_append, h1, h2]
      simp [borrowedIds, List.filterMap_map, Function.comp_def, hfn, hfs]
    rw [hb]
    refine nodup_of_offset (c := (outTreesOf (ν f views (vals ++ writes)) seen
      (.native f info.moduleId views) (vals ++ writes) h.fresh).2.1) fun i x hx => ?_
    rw [List.getElem?_map] at hx
    obtain ⟨y, hy, rfl⟩ := Option.map_eq_some_iff.1 hx
    unfold newRefsOf at hnew
    obtain ⟨z, hz, hzy⟩ := mapM_getElem?_rev hnew hy
    rw [List.getElem?_zipIdx] at hz
    obtain ⟨w, -, rfl⟩ := Option.map_eq_some_iff.1 hz
    simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hzy
    obtain ⟨t, -, rfl⟩ := hzy
    simp
  · obtain ⟨r, par, hm⟩ := mem_borrowedIds.1 hd
    obtain ⟨-, -, -, hle, -⟩ := nativeStep_borrowed hn hm
    exact hle

/-- The `borrowed` identities are among the issued ones. -/
theorem borrowedIds_sublist (evs : List Event) :
    (borrowedIds evs).Sublist ((issuedOf evs).map (·.1)) := by
  induction evs with
  | nil => simp [borrowedIds]
  | cons e evs ih =>
    cases e <;> simp_all [borrowedIds, issuedOf, List.Sublist.cons]

/-- With distinct `borrowed` identities, an identity is borrowed once. -/
theorem borrowed_unique {evs : List Event} (hnd : (borrowedIds evs).Nodup) {d : Nat}
    {r r' : Reference} {par par' : Option Nat} (h : Event.borrowed d r par ∈ evs)
    (h' : Event.borrowed d r' par' ∈ evs) : r = r' ∧ par = par' := by
  induction evs with
  | nil => simp at h
  | cons e evs ih =>
    cases e
    case borrowed d₀ r₀ p₀ =>
      simp only [borrowedIds, List.filterMap_cons, List.nodup_cons] at hnd
      obtain ⟨hnot, hnd⟩ := hnd
      rcases List.mem_cons.1 h with he | he <;> rcases List.mem_cons.1 h' with he' | he'
      · simp only [Event.borrowed.injEq] at he he'
        obtain ⟨rfl, rfl, rfl⟩ := he
        obtain ⟨-, rfl, rfl⟩ := he'
        exact ⟨rfl, rfl⟩
      · simp only [Event.borrowed.injEq] at he
        obtain ⟨rfl, -, -⟩ := he
        exact absurd (mem_borrowedIds.2 ⟨_, _, he'⟩) hnot
      · simp only [Event.borrowed.injEq] at he'
        obtain ⟨rfl, -, -⟩ := he'
        exact absurd (mem_borrowedIds.2 ⟨_, _, he⟩) hnot
      · exact ih hnd he he'
    all_goals
      simp only [borrowedIds, List.filterMap_cons] at hnd
      exact ih hnd (by simpa using h) (by simpa using h')

/-- **The reference facts of annotated executions** from an admissible initial
state (`σ = false`): the annotation's assumptions and the typing invariant
hold; every reference in the frames was created by a `borrowed` event; and
the `borrowed` events have distinct identities below the fresh counter. -/
structure RefTraceInv (P : Program) (s : State) (h : Shadow) (evs : List Event) : Prop where
  pre : Pre false P s h
  typed : StateTyped P s
  refs : ∀ d r, (d, Value.ref r) ∈ framesOcc h.frames s.frames →
    ∃ par, Event.borrowed d r par ∈ evs
  below : ∀ d ∈ borrowedIds evs, d < h.fresh
  unique : (borrowedIds evs).Nodup

/-- A value that is no reference has a key every invariant allows. -/
theorem key_ok_of_not_ref {σ : Bool} {w : Value} (hw : w.isRef = false) : w.key.Ok σ :=
  container_ok fun r hr => by subst hr; simp [Value.isRef] at hw

/-- **A core step keeps the reference facts.** -/
theorem refTraceInv_core {P : Program} {s s' : State} {h h' : Shadow} {evs evs' : List Event}
    (hI : RefTraceInv P s h evs) (hstep : step P s = .done (.running s'))
    (hc : Shadow.coreStep P s (.running s') h = some (.running h', evs')) :
    RefTraceInv P s' h' (evs ++ evs') := by
  obtain ⟨post, evs'', hc', hG⟩ := coreStep_annot (σ := false) (π := {}) (A := [])
    (auth := []) hI.pre hstep (by simp [Outcome.continues])
    (fun vs st ho => by cases ho)
  rw [hc, Option.some.injEq, Prod.mk.injEq] at hc'
  obtain ⟨rfl, rfl⟩ := hc'
  have hT := typed_core hI.typed hI.pre.inv hstep (by simp [Outcome.continues])
  have hinv : StateInv false P s' := hG.inv
  have heff := hG.effect
  obtain ⟨keep, dropped, copies, -, -, -, -, hiss, hnd, hff⟩ := heff
  refine ⟨⟨hG.fits, hinv, ?_, typed_refs hT hinv, refsTop_of_typed hT hinv, composes_of_typed hT⟩,
    hT, ?_, ?_, ?_⟩
  · intro x hx
    obtain ⟨o, ho, h1, h2⟩ := mem_keys_iff.1 hx
    rw [← h2]
    cases hw : o.2.isRef with
    | false => exact key_ok_of_not_ref hw
    | true =>
      obtain ⟨d, w⟩ := o
      cases w with
      | ref r =>
        rcases hG.frameRefs d r (by simpa [postFrames] using ho) with hm | ⟨par, hb⟩
        · exact hI.pre.keysOk _ (mem_keys hm)
        · exact hG.issuedOk _ (List.mem_filterMap.2 ⟨_, hb, rfl⟩)
      | _ => simp [Value.isRef] at hw
  · intro d r hm
    rcases hG.frameRefs d r (by simpa [postFrames] using hm) with hm | ⟨par, hb⟩
    · obtain ⟨par, hb⟩ := hI.refs d r hm
      exact ⟨par, List.mem_append_left _ hb⟩
    · exact ⟨par, List.mem_append_right _ hb⟩
  · intro d hd
    rw [borrowedIds_append, List.mem_append] at hd
    rcases hd with hd | hd
    · exact Nat.lt_of_lt_of_le (hI.below d hd) (by simpa [postFrames, postFresh] using hff)
    · obtain ⟨r, par, hb⟩ := mem_borrowedIds.1 hd
      have := (hiss (d, (Value.ref r).key) (List.mem_filterMap.2 ⟨_, hb, rfl⟩)).2
      simpa [postFresh] using this
  · rw [borrowedIds_append, List.nodup_append]
    refine ⟨hI.unique, (borrowedIds_sublist evs').nodup hnd, fun a ha b hb hab => ?_⟩
    subst hab
    have h1 := hI.below a ha
    obtain ⟨r, par, hb'⟩ := mem_borrowedIds.1 hb
    have h2 := (hiss (a, (Value.ref r).key) (List.mem_filterMap.2 ⟨_, hb', rfl⟩)).1
    omega



/-! ## Natives -/

/-- **What the annotation of a call of a native that is not modeled is**,
when it exists: it sees its arguments as they are, writes nothing back,
returns no places, and its annotation is that of a call on values. -/
theorem nativeStep_some {P : Program} {ν : NativeFlows} {s s₁ : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hstep : step P s = .callNative s₁ f args views) (hshape : P.nativeShape? f = none)
    (hn : Shadow.nativeStep P ν s f args views vals writes places
      (resumeNative P s₁ f args (.returned vals writes places)) h = some (post, evs)) :
    args = views ∧ writes = [] ∧ places = [] ∧
    ∃ g hs info, h.frames = g :: hs ∧ P.function? f = some info ∧ args.length ≤ g.stack.length ∧
      post = nativePost ν f args vals h g hs info.moduleId ∧
      evs = nativeEvents ν f args vals g info.moduleId h.fresh ∧
      post.fits (resumeNative P s₁ f args (.returned vals writes places)) = true := by
  have hcont := nativeStep_continues hn
  obtain ⟨fr, rest, n, hsf, hnl, hargs, rfl, hfacts⟩ := step_callNative hstep
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  have hnoref := noRef_of_conformsAll hconfa (hv.1 _ hpt)
  have hviews : views = args := by
    have := hfacts.view
    rw [nativeView_of_noRefs _ hnoref, Option.some.injEq] at this
    exact this.symm
  subst hviews
  obtain ⟨-, ⟨tys, htys, hconf⟩, rfl, rfl⟩ := resume_running (fr := { fr with stack := fr.stack.drop n })
    (rest := rest) rfl hshape hnoref hv.2 hcont
  obtain ⟨info, hinfo, -⟩ := hfacts.info
  cases hh : h.frames with
  | nil =>
    exfalso
    unfold Shadow.nativeStep at hn
    simp [hh] at hn
  | cons g hs =>
  rw [nativeStep_values hh hinfo hpt htys (hv.1 _ hpt) (hv.2 _ htys) (conformsAll_length hconf)
    hnoref] at hn
  split at hn
  · next hc =>
    simp only [Option.some.injEq, Prod.mk.injEq] at hn
    obtain ⟨rfl, rfl⟩ := hn
    exact ⟨rfl, rfl, rfl, g, hs, info, rfl, hinfo, hc.1, rfl, rfl, hc.2.2⟩
  · cases hn

/-- The results of a native call, as a fold. -/
theorem nativeResults_spec (ν : NativeFlows) (f : FunctionRef) (args vals : List Value)
    (argIds : List IdTree) (m : ModuleId) (fresh : Nat) :
    IdTree.fitsAll (nativeResults ν f args vals argIds m fresh).1 vals = true ∧
      fresh ≤ (nativeResults ν f args vals argIds m fresh).2.1 ∧
      ∀ e ∈ (nativeResults ν f args vals argIds m fresh).2.2, ∃ i w c, e = .created i w c := by
  obtain ⟨ts, n, evs, hfold, hfit, -, -, -, -, hle, hev⟩ := results_fold
    (resultIdOf (ν f args vals) argIds) (.native f m args) vals.zipIdx [] fresh []
  have heq : nativeResults ν f args vals argIds m fresh = (ts, n, evs) := by
    simp [nativeResults, hfold]
  rw [heq]
  refine ⟨by simpa [List.zipIdx_map_fst] using hfit, hle, fun e he => ?_⟩
  obtain ⟨_, _, _, _, _, i, rfl⟩ := hev e he
  exact ⟨_, _, _, rfl⟩

/-- A native call borrows nothing. -/
theorem nativeEvents_borrowedIds {ν : NativeFlows} {f : FunctionRef} {args vals : List Value}
    {g : ShadowFrame} {m : ModuleId} {fresh : Nat} :
    borrowedIds (nativeEvents ν f args vals g m fresh) = [] := by
  rw [List.eq_nil_iff_forall_not_mem]
  intro d hd
  obtain ⟨r, par, hb⟩ := mem_borrowedIds.1 hd
  simp only [nativeEvents, List.mem_append] at hb
  rcases hb with hb | hb
  · obtain ⟨_, _, _, _, he, -⟩ := consumedFrom_mem _ hb
    cases he
  · obtain ⟨_, _, _, he⟩ := (nativeResults_spec ν f args vals _ m fresh).2.2 _ hb
    cases he

/-- The frames after a native call hold the references they held before: the
call takes and returns values only. -/
theorem native_frameRefs {P : Program} {ν : NativeFlows} {s s₁ s' : State} {h h' : Shadow}
    {evs evs' : List Event} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace}
    (hI : RefTraceInv P s h evs) (hstep : step P s = .callNative s₁ f args views)
    (hshape : P.nativeShape? f = none)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs'))
    {d : Nat} {r : Reference} (hm : (d, Value.ref r) ∈ framesOcc h'.frames s'.frames) :
    (d, Value.ref r) ∈ framesOcc h.frames s.frames := by
  have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
    rw [ho]; rfl
  have hvals := native_values hstep hshape hcont
  rw [← ho] at hn
  obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, hpost, -, -⟩ := nativeStep_some hstep hshape hn
  obtain ⟨fr, rest, n, hsf, hnl, hargs, rfl, hfacts⟩ := step_callNative hstep
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  have hs' := (resume_running (fr := { fr with stack := fr.stack.drop n }) (rest := rest) rfl hshape
    (noRef_of_conformsAll hconfa (hv.1 _ hpt)) hv.2 hcont).1
  rw [ho, Outcome.running.injEq] at hs'
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  have hg : g.fits fr = true := by
    have := hI.pre.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  have hfa := frame_stack_fits hg
  obtain ⟨hfR, -, -⟩ := nativeResults_spec ν f args vals (g.stack.take args.length).reverse
    info.moduleId h.fresh
  simp only [nativePost, ShadowPost.running.injEq] at hpost
  generalize nativeResults ν f args vals (g.stack.take args.length).reverse info.moduleId h.fresh = R
    at hfR hpost
  have hh' : h' = { h with
      frames := ({ g with stack := R.1.reverse ++ g.stack.drop args.length } :: hs)
      fresh := R.2.1 } := hpost
  rw [hh', hs'] at hm
  simp only at hm
  have hlenR : R.1.reverse.length = vals.reverse.length := by
    simp [fitsAll_length hfR]
  rw [hargl, framesOcc_cons, frame_occurrences, localsOcc_stack,
    occurrencesAll_append hlenR] at hm
  rw [hh, hsf, framesOcc_cons, frame_occurrences, stack_split hfa n]
  simp only [List.mem_append] at hm ⊢
  rcases hm with (hm | hm | hm) | hm
  · exact Or.inl (Or.inl hm)
  · obtain ⟨v, hv, hsub⟩ := occurrencesAll_mem (fitsAll_reverse hfR) hm
    have := shaped_noRef v (hvals v (List.mem_reverse.1 hv)).1 (hvals v (List.mem_reverse.1 hv)).2
      _ hsub
    simp [Value.isRef] at this
  · exact Or.inl (Or.inr (Or.inr hm))
  · exact Or.inr hm

theorem mem_framesOcc_iff {gs : List ShadowFrame} {frs : List Frame} {x : Nat × Value} :
    x ∈ framesOcc gs frs ↔
      ∃ (j : Nat) (g : ShadowFrame) (fr : Frame), gs[j]? = some g ∧ frs[j]? = some fr ∧
        x ∈ g.occurrences fr := by
  simp only [framesOcc, List.mem_flatMap]
  constructor
  · rintro ⟨⟨g, fr⟩, hp, hx⟩
    obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hp
    rw [List.getElem?_zip_eq_some] at hj
    exact ⟨j, g, fr, hj.1, hj.2, hx⟩
  · rintro ⟨j, g, fr, hg, hfr, hx⟩
    exact ⟨(g, fr), List.mem_iff_getElem?.2 ⟨j, List.getElem?_zip_eq_some.2 ⟨hg, hfr⟩⟩, hx⟩

/-- **The references in each frame after a call of a modeled native** were in
it before, or, in the caller's, the call borrowed them: the values it writes
back hold none, and its results none but the new one. -/
theorem nativeStep_frameRefsAt {σ : Bool} {P : Program} {ν : NativeFlows} {s s₁ s' : State}
    {h h' : Shadow} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace} {evs : List Event} {k : NativeShape} (hpre : Pre σ P s h)
    (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) :
    s'.frames.length = s.frames.length ∧ h'.frames.length = h.frames.length ∧
    ∀ (j : Nat) (g' : ShadowFrame) (fr' : Frame) (g : ShadowFrame) (fr : Frame),
      h'.frames[j]? = some g' → s'.frames[j]? = some fr' → h.frames[j]? = some g →
      s.frames[j]? = some fr → ∀ d r, (d, Value.ref r) ∈ g'.occurrences fr' →
        (d, Value.ref r) ∈ g.occurrences fr ∨ (j = 0 ∧ ∃ par, Event.borrowed d r par ∈ evs) := by
  obtain ⟨fr₀, rest, n, g₀, hs, F, results, resultTrees, ts, hsf, hh, hnl, hng, hargs, hwb, hfold,
    hfitR, hfitT, hwr, hres, -, -⟩ := nativeStep_decomp hshape hstep ho hn
  have hfit := hpre.fits
  rw [shadow_fits, hh, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hfa := frame_stack_fits hg
  obtain ⟨-, hdropf⟩ := fitsAll_take_drop n hfa
  have hlR := fitsAll_length hfitR
  have hfit₁ : Shadow.fits
      { h with
        frames := { g₀ with stack := resultTrees.reverse ++ g₀.stack.drop n } :: hs
        fresh := F }
      { s with
        frames := { fr₀ with
          stack := results.reverse ++ fr₀.stack.drop n
          pc := fr₀.pc + 1 } :: rest } = true := by
    rw [fits_head (h' := { h with
        frames := { g₀ with stack := resultTrees.reverse ++ g₀.stack.drop n } :: hs
        fresh := F })
      (s' := { s with
        frames := { fr₀ with
          stack := results.reverse ++ fr₀.stack.drop n
          pc := fr₀.pc + 1 } :: rest }) rfl rfl h s rfl rfl rfl rfl]
    simp only [Bool.and_eq_true]
    refine ⟨⟨frame_fits_stack hg ?_ _, hrest⟩, hext⟩
    rw [fitsAll_append (by simp [hlR]), fitsAll_reverse hfitR, hdropf]
    rfl
  have hids : ((fr₀ :: rest).map (·.id)).Nodup := by rw [← hsf]; exact hpre.inv.frameIds
  obtain ⟨hfit', -, hids', -, hpf, -⟩ := writes_ghost (s₀ := s) _ ts writes hfit₁ (by simpa using hids)
    hpre.inv.resources (by simp [hsf]) rfl
    (writesAgree_native hstep (by rw [hargs]; simp; omega)
      (root_eq (by simp [hsf]) rfl rfl)) hwb hfold hfitT
  have hl1 : s'.frames.length = s.frames.length := by
    rw [← List.length_map (f := (·.id)), hids', hsf]; simp
  refine ⟨hl1, by rw [shadow_frames_length hfit', hl1, ← shadow_frames_length hpre.fits], ?_⟩
  intro j g' fr' g fr h1 h2 h3 h4 d r hm
  have hjl : j < s.frames.length := (List.getElem?_eq_some_iff.1 h4).1
  rw [hsf] at hjl h4
  rw [hh] at h3
  obtain ⟨g₁, hg₁⟩ : ∃ g₁, ({ g₀ with stack := resultTrees.reverse ++ g₀.stack.drop n } ::
      hs)[j]? = some g₁ := by
    cases j with
    | zero => exact ⟨_, rfl⟩
    | succ j => simp only [List.getElem?_cons_succ] at h3 ⊢; exact ⟨g, h3⟩
  obtain ⟨fr₁, hfr₁⟩ : ∃ fr₁, ({ fr₀ with
      stack := results.reverse ++ fr₀.stack.drop n
      pc := fr₀.pc + 1 } :: rest)[j]? = some fr₁ :=
    ⟨_, List.getElem?_eq_getElem (by simpa using hjl)⟩
  rcases hpf j g' fr' g₁ fr₁ h1 h2 hg₁ hfr₁ (d, .ref r) hm with hx | ⟨y, hy, hx⟩
  · obtain ⟨o, ho', ho1, ho2⟩ := mem_keys_iff.1 hx
    have hor : o = (d, .ref r) := by
      obtain ⟨o1, o2⟩ := o
      simp only at ho1 ho2
      rw [key_ref ho2, ho1]
    subst hor
    cases j with
    | succ j =>
      simp only [List.getElem?_cons_succ] at hg₁ hfr₁ h3 h4
      rw [h3, Option.some.injEq] at hg₁
      rw [h4, Option.some.injEq] at hfr₁
      subst hg₁ hfr₁
      exact Or.inl ho'
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hg₁ hfr₁ h3 h4
      subst hg₁ hfr₁ h3 h4
      rw [frame_occurrences, localsOcc_stack, occurrencesAll_append (by simp [hlR]),
        List.mem_append, List.mem_append] at ho'
      rcases ho' with hl | hR | hD
      · exact Or.inl (by rw [frame_occurrences]; exact List.mem_append_left _ hl)
      · have hR' := (occurrencesAll_reverse hfitR).subset hR
        rcases hres with hnr | ⟨d', b, t, rfl, rfl, hb⟩
        · obtain ⟨v, hv, hsub⟩ := occurrencesAll_mem hfitR hR'
          have := hnr v hv _ hsub
          simp [Value.isRef] at this
        · simp [IdTree.occurrencesAll, IdTree.occurrences] at hR'
          obtain ⟨rfl, rfl⟩ := hR'
          exact Or.inr ⟨rfl, _, hb⟩
      · refine Or.inl ?_
        rw [frame_occurrences, stack_split hfa n]
        exact List.mem_append_right _ (List.mem_append_right _ hD)
  · obtain ⟨o, ho', ho1, ho2⟩ := mem_keys_iff.1 hx
    have hfy := fitsAll_of_zip hfitT hy
    have hsub := occurrences_mem hfy ho'
    rw [show o.2 = .ref r from key_ref ho2] at hsub
    have := hwr y.2 (List.of_mem_zip hy).2 _ hsub
    simp [Value.isRef] at this

/-- **The references in the frames after a call of a modeled native** were in
them before, or the call borrowed them. -/
theorem nativeStep_frameRefs {σ : Bool} {P : Program} {ν : NativeFlows} {s s₁ s' : State}
    {h h' : Shadow} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace} {evs : List Event} {k : NativeShape} (hpre : Pre σ P s h)
    (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) {d : Nat} {r : Reference}
    (hm : (d, Value.ref r) ∈ framesOcc h'.frames s'.frames) :
    (d, Value.ref r) ∈ framesOcc h.frames s.frames ∨ ∃ par, Event.borrowed d r par ∈ evs := by
  obtain ⟨hl1, -, hat⟩ := nativeStep_frameRefsAt hpre hshape hstep ho hn
  obtain ⟨j, g', fr', hg', hfr', hm'⟩ := mem_framesOcc_iff.1 hm
  have hjs : j < s.frames.length := by
    rw [← hl1]; exact (List.getElem?_eq_some_iff.1 hfr').1
  have hjh : j < h.frames.length := by
    rw [shadow_frames_length hpre.fits]; exact hjs
  rcases hat j g' fr' _ _ hg' hfr' (List.getElem?_eq_getElem hjh)
    (List.getElem?_eq_getElem hjs) d r hm' with hx | ⟨-, hb⟩
  · exact Or.inl (mem_framesOcc_iff.2 ⟨j, _, _, List.getElem?_eq_getElem hjh,
      List.getElem?_eq_getElem hjs, hx⟩)
  · exact Or.inr hb

/-- **A native call keeps the reference facts**: it takes and returns values
only. -/
theorem refTraceInv_native {P : Program} {ν : NativeFlows} {s s₁ s' : State} {h h' : Shadow}
    {evs evs' : List Event} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace}
    (hI : RefTraceInv P s h evs) (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs')) :
    RefTraceInv P s' h' (evs ++ evs') := by
  cases hshape : P.nativeShape? f with
  | some k =>
    have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
      rw [ho]; rfl
    have hT := typed_native hI.typed hI.pre.inv hstep hcont
    rw [ho] at hT
    have hinv : StateInv false P s' := inv_native hI.typed hI.pre.inv hshape hstep ho
    obtain ⟨g₁, hs₁, info₁, tys₁, ptys₁, seen₁, s₂, fr₂, tl₂, newRefs₁, resultTrees₁, h₂, -, -, -, -,
      -, -, -, -, -, -, -, -, -, -, -, hpost, hfit, -⟩ := nativeStep_spec hn
    simp only [ShadowPost.running.injEq] at hpost
    subst hpost
    obtain ⟨-, -, -, -, -, F, -, -, -, -, -, -, -, -, -, -, -, -, -, -, hF, hlt, hF', -⟩ :=
      nativeStep_decomp hshape hstep ho hn
    rw [← ho] at hn
    have hfresh' : h.fresh ≤ h'.fresh := by rw [hF']; exact hF
    refine ⟨⟨hfit, hinv, ?_, typed_refs hT hinv, refsTop_of_typed hT hinv, composes_of_typed hT⟩,
      hT, ?_, ?_, ?_⟩
    · intro x hx
      obtain ⟨o, ho', h1, h2⟩ := mem_keys_iff.1 hx
      rw [← h2]
      cases hw : o.2.isRef with
      | false => exact key_ok_of_not_ref hw
      | true =>
        obtain ⟨d, w⟩ := o
        cases w with
        | ref r =>
          rw [ho] at hn
          rcases nativeStep_frameRefs hI.pre hshape hstep ho hn ho' with hm | ⟨par, hb⟩
          · exact hI.pre.keysOk _ (mem_keys hm)
          · rw [← ho] at hn
            have hp := nativeStep_borrowed_path hstep hn hb
            show Key.Ok false (.leaf (.ref r))
            refine ⟨fun h => (by cases h), fun a t hl => ?_⟩
            rw [hl] at hp
            exact absurd rfl hp
        | _ => simp [Value.isRef] at hw
    · intro d r hm
      rw [ho] at hn
      rcases nativeStep_frameRefs hI.pre hshape hstep ho hn hm with hm | ⟨par, hb⟩
      · obtain ⟨par, hb⟩ := hI.refs d r hm
        exact ⟨par, List.mem_append_left _ hb⟩
      · exact ⟨par, List.mem_append_right _ hb⟩
    · intro d hd
      rw [borrowedIds_append, List.mem_append] at hd
      rcases hd with hd | hd
      · exact Nat.lt_of_lt_of_le (hI.below d hd) hfresh'
      · obtain ⟨r, par, hb⟩ := mem_borrowedIds.1 hd
        rw [hF']
        exact hlt d r par hb
    · rw [ho] at hn
      obtain ⟨hnd, hge⟩ := native_borrowedIds hn
      rw [borrowedIds_append]
      refine List.nodup_append.2 ⟨hI.unique, hnd, fun a ha b hb hab => ?_⟩
      have h1 := hI.below a ha
      have h2 := hge b hb
      omega
  | none =>
  have hrefs : ∀ d r, (d, Value.ref r) ∈ framesOcc h'.frames s'.frames →
      (d, Value.ref r) ∈ framesOcc h.frames s.frames :=
    fun _ _ hm => native_frameRefs hI hstep hshape ho hn hm
  have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
    rw [ho]; rfl
  have hT := typed_native hI.typed hI.pre.inv hstep hcont
  rw [ho] at hT
  rw [← ho] at hn
  obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, hpost, hevs, hfit⟩ := nativeStep_some hstep hshape hn
  rw [ho] at hfit
  obtain ⟨fr, rest, n, hsf, -, -, rfl, hfacts⟩ := step_callNative hstep
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  have hs' := (resume_running (fr := { fr with stack := fr.stack.drop n }) (rest := rest) rfl hshape
    (noRef_of_conformsAll hconfa (hv.1 _ hpt)) hv.2 hcont).1
  rw [ho, Outcome.running.injEq] at hs'
  obtain ⟨-, hfresh, -⟩ := nativeResults_spec ν f args vals (g.stack.take args.length).reverse
    info.moduleId h.fresh
  simp only [nativePost, ShadowPost.running.injEq] at hpost
  generalize nativeResults ν f args vals (g.stack.take args.length).reverse info.moduleId h.fresh = R
    at hfresh hpost
  have hh' : h' = { h with
      frames := ({ g with stack := R.1.reverse ++ g.stack.drop args.length } :: hs)
      fresh := R.2.1 } := hpost
  have hinv : StateInv false P s' := by
    rw [hs']
    exact frames_inv hI.pre.inv hsf rfl rfl rfl rfl
  have hfresh' : h.fresh ≤ h'.fresh := by rw [hh']; exact hfresh
  refine ⟨⟨hfit, hinv, ?_, typed_refs hT hinv, refsTop_of_typed hT hinv, composes_of_typed hT⟩,
    hT, ?_, ?_, ?_⟩
  · intro x hx
    obtain ⟨o, ho', h1, h2⟩ := mem_keys_iff.1 hx
    rw [← h2]
    cases hw : o.2.isRef with
    | false => exact key_ok_of_not_ref hw
    | true =>
      obtain ⟨d, w⟩ := o
      cases w with
      | ref r => exact hI.pre.keysOk _ (mem_keys (hrefs d r ho'))
      | _ => simp [Value.isRef] at hw
  · intro d r hm
    obtain ⟨par, hb⟩ := hI.refs d r (hrefs d r hm)
    exact ⟨par, List.mem_append_left _ hb⟩
  · intro d hd
    rw [borrowedIds_append, hevs, nativeEvents_borrowedIds, List.append_nil] at hd
    exact Nat.lt_of_lt_of_le (hI.below d hd) hfresh'
  · rw [borrowedIds_append, hevs, nativeEvents_borrowedIds, List.append_nil]
    exact hI.unique

/-! ## The initial state -/

/-- The identities of an initial frame's reference parameters are among the
identities of its locals' occurrences. -/
theorem initParams_sublist : ∀ (L : List (Option IdTree × Option Value)),
    L.all (fun p => ShadowFrame.slotFits p.1 p.2) = true →
    ((L.filterMap fun (p : Option IdTree × Option Value) => match p.1, p.2 with
        | some t, some (.ref r) => some (t.id, r)
        | _, _ => none).map (·.1)).Sublist
      ((L.flatMap fun (p : Option IdTree × Option Value) =>
        ShadowFrame.slotOccurrences p.1 p.2).map (·.1))
  | [], _ => by simp
  | (ot, ov) :: L, h => by
    simp only [List.all_cons, Bool.and_eq_true] at h
    have ih := initParams_sublist L h.2
    rw [List.flatMap_cons, List.map_append, List.filterMap_cons]
    cases ot with
    | none =>
      simp only [ShadowFrame.slotOccurrences, List.map_nil, List.nil_append]
      exact ih
    | some t =>
      cases ov with
      | none =>
        simp only [ShadowFrame.slotOccurrences, List.map_nil, List.nil_append]
        exact ih
      | some v =>
        cases v with
        | ref r =>
          have hf : t.fits (.ref r) = true := by simpa [ShadowFrame.slotFits] using h.1
          simp only [ShadowFrame.slotOccurrences, ref_occ hf, List.map_cons, List.map_nil,
            List.singleton_append]
          exact ih.cons_cons _
        | _ => exact ih.trans (List.sublist_append_right _ _)

/-- Creations borrow nothing. -/
theorem borrowedIds_of_created {evs : List Event}
    (h : ∀ e ∈ evs, ∃ i w c, e = Event.created i w c) : borrowedIds evs = [] := by
  rw [List.eq_nil_iff_forall_not_mem]
  intro d hd
  obtain ⟨r, par, hb⟩ := mem_borrowedIds.1 hd
  obtain ⟨_, _, _, he⟩ := h _ hb
  cases he

/-- An initial frame borrows its reference parameters. -/
theorem borrowedIds_initRefs (g : ShadowFrame) (fr : Frame) :
    borrowedIds (initRefs g fr) = (initParams g fr).map (·.1) := by
  simp only [initRefs, borrowedIds_append]
  simp [borrowedIds, List.filterMap_map, Function.comp_def]

/-- **The reference facts at an admissible initial state.** -/
theorem refTraceInv_initial {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    RefTraceInv P s₀ (Shadow.initial s₀).1 (Shadow.initial s₀).2 := by
  obtain ⟨f, args, tys, storage, held, hparams, hconf, hwt, hinit⟩ := h0
  obtain ⟨fr, localTys, hfr, hst, -, -, -, hloc, hsto, hhe, -, hha⟩ := initialState_shape hinit
  -- What the entry frame's locals hold: references to held values, and values of
  -- value types, which hold no reference.
  have hargs : ∀ v, some v ∈ fr.locals.toList → ∀ w ∈ subvalues v, w.isRef = true →
      w = v ∧ ∃ r k, v = .ref r ∧ r.location = ⟨.held k, []⟩ := by
    intro v hv w hw hwr
    rw [hloc, List.toList_toArray, List.mem_append, List.mem_map] at hv
    rcases hv with ⟨v', hv', hvv⟩ | hv
    · simp only [Option.some.injEq] at hvv
      subst hvv
      simp only [heldArguments, Bool.and_eq_true, List.all_eq_true] at hha
      cases v' with
      | ref r =>
        have hw' : w = .ref r := by
          simpa [subvalues, Value.nodes] using hw
        subst hw'
        refine ⟨rfl, r, ?_⟩
        have h3 := hha.1.2 r (List.mem_filterMap.2 ⟨_, hv', rfl⟩)
        obtain ⟨-, h3⟩ := h3
        split at h3
        · next k hroot hpath => exact ⟨k, rfl, by cases hl : r.location; simp_all⟩
        · simp at h3
      | _ =>
        have hval := hha.1.1.2 _ hv'
        obtain ⟨t, -, hc⟩ := conformsAll_mem hconf _ hv'
        have := conforms_plain hc (by
          rw [← assignable_isValue (conforms_assignable hc)]; simpa using hval) w hw
        rw [hwr] at this; cases this
    · simp [List.mem_replicate] at hv
  obtain ⟨ls, evs₁, n₁, hf, hlen, hfit, hocc, hb, hnd, -, hev⟩ :=
    locals_fold fr.locals.toList [] 0 []
  obtain ⟨sts, e₂, n₂, hfS, hfitS, -, hbS, -, hleS, hevS⟩ :=
    fresh_fold .storage (s₀.storage.map (·.value)) [] n₁ []
  obtain ⟨hts, e₃, n₃, hfH, hfitH, -, -, -, hleH, hevH⟩ :=
    fresh_fold .transaction s₀.held [] n₂ []
  rw [initial_eq hfr]
  simp only [initL, initK, initS, initH, hst, freshAll_nil, hf, hfS, hfH, List.nil_append,
    List.append_nil]
  have hfit' : slotsFit ⟨ls.toArray, []⟩ fr = true := by simpa [slotsFit] using hfit
  have hlo : localsOcc ⟨ls.toArray, []⟩ fr = createdOf evs₁ := by simpa [localsOcc] using hocc
  have hg : (⟨ls.toArray, []⟩ : ShadowFrame).fits fr = true := by
    rw [frame_fits]
    simp only [Bool.and_eq_true, beq_iff_eq]
    refine ⟨⟨by simp [hlen], hfit'⟩, by rw [hst]; rfl⟩
  have hframes : framesOcc [⟨ls.toArray, []⟩] [fr] = localsOcc ⟨ls.toArray, []⟩ fr := by
    simp [framesOcc, frame_occurrences, IdTree.occurrencesAll]
  -- The references in the frame are the parameters.
  have hrefs : ∀ d r, (d, Value.ref r) ∈ localsOcc ⟨ls.toArray, []⟩ fr →
      (d, r) ∈ initParams ⟨ls.toArray, []⟩ fr := by
    intro d r hm
    simp only [localsOcc, List.mem_flatMap] at hm
    obtain ⟨⟨ot, ov⟩, hz, hm⟩ := hm
    have hsl : ShadowFrame.slotFits ot ov = true := by
      simp only [slotsFit, List.all_eq_true] at hfit'
      exact hfit' _ hz
    cases ot with
    | none => simp [ShadowFrame.slotOccurrences] at hm
    | some t =>
      cases ov with
      | none => simp [ShadowFrame.slotOccurrences] at hm
      | some v =>
        simp only [ShadowFrame.slotOccurrences] at hm
        simp only [ShadowFrame.slotFits] at hsl
        have hvm : some v ∈ fr.locals.toList := (List.of_mem_zip hz).2
        obtain ⟨hrv, r', k, rfl, -⟩ := hargs v hvm _ (occurrences_mem hsl hm) rfl
        rw [ref_occ hsl, List.mem_singleton, Prod.mk.injEq] at hm
        obtain ⟨rfl, hrr⟩ := hm
        simp only [Value.ref.injEq] at hrr
        subst hrr
        simp only [initParams, List.mem_filterMap]
        exact ⟨_, hz, rfl⟩
  have hids : borrowedIds (evs₁ ++ e₂ ++ e₃ ++ initRefs ⟨ls.toArray, []⟩ fr) =
      (initParams ⟨ls.toArray, []⟩ fr).map (·.1) := by
    have h1 := borrowedIds_of_created (evs := evs₁) fun e he => by
      obtain ⟨i, w, h, -⟩ := hev e he; exact ⟨i, w, _, h⟩
    have h2 := borrowedIds_of_created (evs := e₂) fun e he => by
      obtain ⟨i, w, h, -⟩ := hevS e he; exact ⟨i, w, _, h⟩
    have h3 := borrowedIds_of_created (evs := e₃) fun e he => by
      obtain ⟨i, w, h, -⟩ := hevH e he; exact ⟨i, w, _, h⟩
    simp only [borrowedIds_append, h1, h2, h3, borrowedIds_initRefs, List.nil_append]
  have hsub : ((initParams ⟨ls.toArray, []⟩ fr).map (·.1)).Sublist ((createdOf evs₁).map (·.1)) := by
    rw [← hocc]
    exact initParams_sublist _ hfit
  refine ⟨⟨?_, inv_initial (σ := false) hinit hwt (fun h => by cases h), ?_, typed_refs (typed_initial hinit hparams hconf hwt)
    (inv_initial (σ := false) hinit hwt (fun h => by cases h)), refsTop_of_typed (typed_initial hinit hparams hconf hwt)
    (inv_initial (σ := false) hinit hwt (fun h => by cases h)),
    composes_of_typed (typed_initial hinit hparams hconf hwt)⟩, typed_initial hinit hparams hconf hwt, ?_, ?_, ?_⟩
  · rw [shadow_fits, hfr, framesFit_cons, hg]
    simp [framesFit, extFit, hfitS, hfitH]
  · intro x hx
    simp only at hx
    rw [hfr, hframes, hlo] at hx
    obtain ⟨o, ho, -, h2⟩ := mem_keys_iff.1 hx
    rw [← h2]
    cases hw : o.2.isRef with
    | false => exact key_ok_of_not_ref hw
    | true =>
      obtain ⟨c, hc⟩ := mem_createdOf.1 ho
      obtain ⟨i, w, he, v, hv, hwv⟩ := hev _ hc
      simp only [Event.created.injEq] at he
      obtain ⟨-, hw', -⟩ := he
      rw [hw'] at hw ⊢
      obtain ⟨-, r, k, rfl, hloc'⟩ := hargs v hv w hwv hw
      have : w = .ref r := by simpa [subvalues, Value.nodes] using hwv
      rw [this]
      simp only [Value.key, Key.Ok, hloc']
      exact ⟨fun h => by simp at h, fun a t h => by simp at h⟩
  · intro d r hm
    simp only at hm
    rw [hfr, hframes] at hm
    refine ⟨none, ?_⟩
    simp only [List.mem_append, initRefs, List.mem_map]
    exact Or.inr (Or.inl ⟨(d, r), hrefs d r hm, rfl⟩)
  · intro d hd
    simp only [hids, List.mem_map] at hd
    obtain ⟨o, ho, rfl⟩ := List.mem_map.1 (hsub.subset (List.mem_map.2 hd))
    exact Nat.lt_of_lt_of_le (hb o ho).2 (Nat.le_trans hleS hleH)
  · rw [hids]
    exact hsub.nodup hnd


/-! ## Along a trace -/

/-- **One traced step keeps the reference facts.** -/
theorem refTraceInv_step {P : Program} {N : NativeRel} {ν : NativeFlows} {s s' : State}
    {h h' : Shadow} {o : Outcome} {post : ShadowPost} {evs evs' : List Event}
    (hI : RefTraceInv P s h evs) (hT : TracedStep P N ν s h o post evs')
    (ho : o = .running s') (hp : post = .running h') : RefTraceInv P s' h' (evs ++ evs') := by
  subst hp
  cases hT with
  | core hstep hc =>
    subst ho
    exact refTraceInv_core hI hstep hc
  | native hstep _ hn =>
    rw [ho] at hn
    exact refTraceInv_native hI hstep ho hn

/-- **The reference facts hold along every trace** from an admissible initial
state, whatever the natives do. -/
theorem refTraceInv_trace {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {evs : List Event} (h0 : AdmissibleInitial P s₀)
    (hT : Trace P N ν s₀ s h evs) : RefTraceInv P s h evs := by
  induction hT with
  | start => exact refTraceInv_initial h0
  | step _ hS ih => exact refTraceInv_step ih hS rfl rfl

/-! ## Traced steps -/

/-- What an annotated core step is: an instruction of the current frame. -/
theorem coreStep_exec {P : Program} {s : State} {o : Outcome} {h : Shadow} {post : ShadowPost}
    {evs : List Event} (hc : Shadow.coreStep P s o h = some (post, evs)) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ P.function? fr.function = some info ∧
      info.definition.code = some code ∧ code.code[fr.pc]? = some instr ∧
      Shadow.exec P info.moduleId instr s fr o h = some (post, evs) := by
  unfold Shadow.coreStep at hc
  split at hc
  · next fr rest hsf =>
    simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at hc
    obtain ⟨info, hinfo, code, hcode, instr, hinstr, ⟨post', evs'⟩, hex, hc⟩ := hc
    split at hc
    · simp only [Option.some.injEq, Prod.mk.injEq] at hc
      obtain ⟨rfl, rfl⟩ := hc
      exact ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩
    · cases hc
  · cases hc

/-- A native call uses no reference. -/
theorem nativeEvents_used {ν : NativeFlows} {f : FunctionRef} {args vals : List Value}
    {g : ShadowFrame} {m : ModuleId} {fresh i : Nat} {r : Reference} :
    Event.used i r ∉ nativeEvents ν f args vals g m fresh := by
  intro hb
  simp only [nativeEvents, List.mem_append] at hb
  rcases hb with hb | hb
  · obtain ⟨_, _, _, _, he, -⟩ := consumedFrom_mem _ hb
    cases he
  · obtain ⟨_, _, _, he⟩ := (nativeResults_spec ν f args vals _ m fresh).2.2 _ hb
    cases he

/-- **The references a native call uses are on the caller's segment**, with
their identities. -/
theorem native_used_top {P : Program} {ν : NativeFlows} {s s₁ : State} {h : Shadow}
    {f : FunctionRef} {args views vals writes : List Value} {places : List NativePlace}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hpre : Pre false P s h)
    (hstep : step P s = .callNative s₁ f args views)
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {i : Nat} {r : Reference} (hu : Event.used i r ∈ evs) :
    ∃ g hs fr rest, h.frames = g :: hs ∧ s.frames = fr :: rest ∧
      (i, Value.ref r) ∈ g.occurrences fr := by
  obtain ⟨g, hs, hh, -, hx⟩ := nativeStep_used hn hu
  obtain ⟨t, rfl, hz⟩ := mem_refArgsOf hx
  obtain ⟨fr, rest, n, hsf, hnl, hargs, -, -⟩ := step_callNative hstep
  have hg : g.fits fr = true := by
    have := hpre.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  subst hargs
  have hargl : (fr.stack.take n).reverse.length = n := by simp [Nat.min_eq_left hnl]
  rw [hargl] at hz
  exact ⟨g, hs, fr, rest, hh, hsf, refSlot_occ hg
    ⟨t, rfl, Or.inl (zip_rev_take (fitsAll_length (frame_stack_fits hg)) hz)⟩⟩

/-- **An access of a native call** is through one of its reference
arguments, on the caller's segment with its identity: a read of what it
designates, or, for an update-shape native, a write through a mutable one. -/
theorem native_access {σ : Bool} {P : Program} {ν : NativeFlows} {s s₁ : State} {h : Shadow}
    {f : FunctionRef} {args views vals writes : List Value} {places : List NativePlace}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hpre : Pre σ P s h)
    (hstep : step P s = .callNative s₁ f args views)
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {a : Access} {l : Location} {via : Option Nat} (ha : Event.accessed a l via ∈ evs) :
    ∃ (g : ShadowFrame) (hs : List ShadowFrame) (fr : Frame) (rest : List Frame) (n : Nat)
      (t : Nat) (r : Reference), h.frames = g :: hs ∧ s.frames = fr :: rest ∧
      n ≤ fr.stack.length ∧ args = (fr.stack.take n).reverse ∧ StackSlot g fr t r ∧
      Value.ref r ∈ fr.stack.take n ∧ via = some t ∧ l = r.location ∧
      (a = .read ∨ (a = .write ∧ r.mutable = true ∧ P.nativeShape? f = some .update)) := by
  obtain ⟨g, hs, hh, -, h1⟩ := nativeStep_events hn ha
  obtain ⟨fr, rest, n, hsf, hnl, hargs, -, -⟩ := step_callNative hstep
  have hg : g.fits fr = true := by
    have := hpre.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  have hslot : ∀ x ∈ refArgsOf (g.stack.take args.length).reverse args,
      StackSlot g fr x.1 x.2 ∧ Value.ref x.2 ∈ fr.stack.take n := by
    intro x hx
    obtain ⟨t, ht, hz⟩ := mem_refArgsOf hx
    rw [hargl, hargs] at hz
    refine ⟨⟨t, ht, zip_rev_take (fitsAll_length (frame_stack_fits hg)) hz⟩, ?_⟩
    exact List.mem_reverse.1 (List.of_mem_zip hz).2
  rcases h1 with ⟨x, hx, he⟩ | ⟨x, hx, hxm, hku, he⟩ | ⟨_, _, he⟩ | ⟨_, _, _, he, -⟩ |
    ⟨_, _, _, he⟩ | ⟨_, _, _, he⟩
  · simp only [Event.accessed.injEq] at he
    obtain ⟨rfl, rfl, rfl⟩ := he
    obtain ⟨hs', hm⟩ := hslot x hx
    exact ⟨g, hs, fr, rest, n, x.1, x.2, hh, hsf, hnl, hargs, hs', hm, rfl, rfl, Or.inl rfl⟩
  · simp only [Event.accessed.injEq] at he
    obtain ⟨rfl, rfl, rfl⟩ := he
    obtain ⟨hs', hm⟩ := hslot x hx
    exact ⟨g, hs, fr, rest, n, x.1, x.2, hh, hsf, hnl, hargs, hs', hm, rfl, rfl,
      Or.inr ⟨rfl, hxm, hku⟩⟩
  all_goals cases he

/-- A modeled native's parameters are references or value types. -/
theorem param_kinds {P : Program} {f : FunctionRef} {k : NativeShape} {ptys : List RuntimeType}
    (hshape : P.nativeShape? f = some k) (hpt : P.paramTypes f = some ptys)
    (hτ : ∀ x ∈ f.typeArgs, x.isValue = true) :
    ∀ t ∈ ptys, t.isValue = true ∨ ∃ m u, t = .ref m u := by
  obtain ⟨info, e, -, -, -, -, -, hpe, -⟩ := nativeShape_entry hshape
  rw [hpt] at hpe
  intro t ht
  obtain ⟨q, -, hq⟩ := mapM_some_mem hpe.symm t ht
  cases q with
  | reference q' =>
    simp only [TokenPattern.slotType, Option.map_eq_some_iff] at hq
    obtain ⟨u, -, rfl⟩ := hq
    exact Or.inr ⟨_, _, rfl⟩
  | mutableReference q' =>
    simp only [TokenPattern.slotType, Option.map_eq_some_iff] at hq
    obtain ⟨u, -, rfl⟩ := hq
    exact Or.inr ⟨_, _, rfl⟩
  | _ => exact Or.inl (slotType_values hτ hq)

theorem idTree_root_occ (t : IdTree) (v : Value) : (t.id, v) ∈ t.occurrences v := by
  obtain ⟨i, cs⟩ := t
  simp [IdTree.occurrences, IdTree.id]

/-- A node of a tree that fits a value is an occurrence, of a subvalue. -/
theorem node_occ : ∀ (q : Path) {t u : IdTree} {v : Value}, t.fits v = true →
    t.at? q = some u → ∃ w, (u.id, w) ∈ t.occurrences v ∧ w ∈ subvalues v
  | [], t, u, v, hf, h => by
    simp only [IdTree.at?, Option.some.injEq] at h
    subst h
    exact ⟨v, idTree_root_occ _ v, occurrences_mem hf (idTree_root_occ _ v)⟩
  | i :: p, t, u, v, hf, h => by
    obtain ⟨id, cs⟩ := t
    simp only [IdTree.at?, IdTree.children] at h
    split at h
    · next c hc =>
      have hf' := hf
      simp only [IdTree.fits] at hf'
      have hlen := fitsAll_length hf'
      have hi : i < v.children.length := by
        have := (List.getElem?_eq_some_iff.1 hc).1
        omega
      obtain ⟨cv, hcv⟩ : ∃ cv, v.children[i]? = some cv := ⟨_, List.getElem?_eq_getElem hi⟩
      obtain ⟨c', hc', hfc⟩ := fitsAll_get hf' hcv
      rw [hc, Option.some.injEq] at hc'
      subst hc'
      obtain ⟨w, hw, -⟩ := node_occ p hfc h
      have ho : (u.id, w) ∈ (IdTree.node id cs).occurrences v :=
        List.mem_cons_of_mem _ (occ_of_zip (zip_of_getElem? hc hcv) _ hw)
      exact ⟨w, ho, occurrences_mem hf ho⟩
    · cases h

/-- **The nodes a native sees** are occurrences of the state before the call,
of values that are no references: what its reference arguments designate,
and its value arguments. -/
theorem seen_nodes {σ : Bool} {P : Program} {s s₁ : State} {h : Shadow} {f : FunctionRef}
    {args views : List Value} {k : NativeShape} (hpre : Pre σ P s h) (ht : StateTyped P s)
    (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views) {g : ShadowFrame} {hs : List ShadowFrame}
    (hh : h.frames = g :: hs) {seen : List IdTree}
    (hseen : seenTrees h s args (g.stack.take args.length).reverse = some seen) :
    ∀ kk q i, seenIdAt seen kk q = some i → ∃ w, (i, w) ∈ h.occurrences s ∧ w.isRef = false := by
  obtain ⟨fr, rest, n, hsf, hnl, hargs, hs₁, hfacts⟩ := step_callNative hstep
  have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
  have hg : g.fits fr = true := by
    have := hpre.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  intro kk q i hi
  simp only [seenIdAt, Option.map_eq_some_iff, Option.bind_eq_some_iff] at hi
  obtain ⟨u, ⟨tk, htk, hu⟩, rfl⟩ := hi
  unfold seenTrees at hseen
  obtain ⟨⟨a, t⟩, hz, hF⟩ := mapM_getElem?_rev hseen htk
  rw [List.getElem?_zip_eq_some] at hz
  obtain ⟨ha, hta⟩ := hz
  by_cases hra : ∃ r, a = .ref r
  · obtain ⟨r, rfl⟩ := hra
    simp only at hF
    -- What a reference argument designates.
    have hrargs : Value.ref r ∈ args := List.mem_of_getElem? ha
    obtain ⟨v, hv⟩ := nativeView_deref hfacts.view r hrargs
    rw [hs₁] at hv
    have hv' : s.deref? r.location r.since = some v := by
      rw [← hv]
      exact (deref_eq (s := s) (s' := { s with frames := { fr with stack := fr.stack.drop n } :: rest })
        (by simp [hsf]) rfl rfl rfl _ _).symm
    obtain ⟨tw, htw, hfw, R, hR⟩ := tree_read hpre.fits (deref?_read hv')
    rw [htw, Option.some.injEq] at hF
    subst hF
    obtain ⟨w, hw, hwv⟩ := node_occ q hfw hu
    have hrs : Value.ref r ∈ fr.stack := by
      rw [hargs] at hrargs; exact List.mem_of_mem_take (List.mem_reverse.1 hrargs)
    obtain ⟨hsh, hval, -⟩ := deref_shaped ht hpre.inv hfr hrs hv'
    exact ⟨w, hR.symm.subset (List.mem_append_left _ hw), shaped_noRef v hsh hval w hwv⟩
  · -- A value argument.
    have hF' : tk = t := by
      cases a <;> simp_all
    subst hF'
    have hz' : (tk, a) ∈ (g.stack.take args.length).reverse.zip args := zip_of_getElem? hta ha
    rw [hargl] at hz'
    rw [hargs] at hz'
    have hz := zip_rev_take (fitsAll_length (frame_stack_fits hg)) hz'
    have hft := fitsAll_of_zip (frame_stack_fits hg) hz
    obtain ⟨w, hw, hwv⟩ := node_occ q hft hu
    refine ⟨w, ?_, ?_⟩
    · rw [shadow_occurrences, hh, hsf, framesOcc_cons, frame_occurrences]
      exact List.mem_append_left _ (List.mem_append_left _ (List.mem_append_right _
        (occ_of_zip hz _ hw)))
    · obtain ⟨pt, hpt', hc⟩ := conformsAll_get hconfa ha
      rcases param_kinds hshape hpt (typeArgsOk_values hfacts.typeArgs) pt
          (List.mem_of_getElem? hpt') with hpv | ⟨m, u', rfl⟩
      · obtain ⟨hsh, hty⟩ := conforms_shaped hc
        exact shaped_noRef a hsh (by rw [assignable_isValue hty]; exact hpv) w hwv
      · exfalso
        have hasg := (conforms_shaped hc).2
        cases a with
        | ref r => exact hra ⟨r, rfl⟩
        | _ => simp [Value.type, RuntimeType.assignable] at hasg

/-- **The references a traced step uses are in the frames** before it, with
their identities. -/
theorem traced_used {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State} {h : Shadow}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hpre : Pre false P s h)
    (hT : TracedStep P N ν s h o post evs) {i : Nat} {r : Reference}
    (hu : Event.used i r ∈ evs) : (i, Value.ref r) ∈ framesOcc h.frames s.frames := by
  cases hT with
  | core hstep hc =>
    obtain ⟨fr, rest, info, code, instr, hsf, -, -, -, hex⟩ := coreStep_exec hc
    obtain ⟨hframes, hst, hhe, fresh⟩ := h
    cases hframes with
    | nil =>
      have hfit := hpre.fits
      rw [shadow_fits, hsf] at hfit
      simp [framesFit] at hfit
    | cons g hs =>
      have hg : g.fits fr = true := (pre_head hpre hsf).1
      rw [hsf, framesOcc_cons]
      refine List.mem_append_left _ (refSlot_occ hg ?_)
      rcases exec_used rfl hg hex i r hu with hsl | ⟨j, -, hl⟩
      · exact hsl.refSlot
      · exact hl.refSlot
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨g, hs, fr, rest, hh, hsf, hm⟩ := native_used_top hpre hstep hn hu
      rw [hsf, hh, framesOcc_cons]
      exact List.mem_append_left _ hm
    obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at hu
    exact absurd hu nativeEvents_used


/-- **The references in the frames after a traced step** were in them before,
or the step borrowed them. -/
theorem traced_frameRefs {P : Program} {N : NativeRel} {ν : NativeFlows} {s s' : State}
    {h h' : Shadow} {o : Outcome} {post : ShadowPost} {evs evs' : List Event}
    (hI : RefTraceInv P s h evs) (hT : TracedStep P N ν s h o post evs') (ho : o = .running s')
    (hp : post = .running h') {d : Nat} {r : Reference}
    (hm : (d, Value.ref r) ∈ framesOcc h'.frames s'.frames) :
    (d, Value.ref r) ∈ framesOcc h.frames s.frames ∨ ∃ par, Event.borrowed d r par ∈ evs' := by
  subst hp
  cases hT with
  | core hstep hc =>
    subst ho
    obtain ⟨post, evs'', hc', hG⟩ := coreStep_annot (σ := false) (π := {}) (A := [])
      (auth := []) hI.pre hstep (by simp [Outcome.continues]) (fun vs st ho => by cases ho)
    rw [hc, Option.some.injEq, Prod.mk.injEq] at hc'
    obtain ⟨rfl, rfl⟩ := hc'
    exact hG.frameRefs d r (by simpa [postFrames] using hm)
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rw [ho] at hn
      exact nativeStep_frameRefs hI.pre hshape hstep ho hn hm
    rw [ho] at hn
    exact Or.inl (native_frameRefs hI hstep hshape ho hn hm)

/-! ## Accounting -/

end MoveBytecode.Proofs
