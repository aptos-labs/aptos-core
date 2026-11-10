-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.DanglingCreates

/-!
# The accesses a step records

UNTRUSTED (checked by the kernel). What the ghost layer's `accessed` events
of an instruction are (`exec_accessed`): an inspection; a read or write
through a reference on the current frame's operand stack, of the reference's
location; a direct read or write of a local of the current frame; or a direct
write of a resource in global storage, by `MoveFrom` or `MoveTo`. A step that
borrows a reference only inspects (`exec_borrow_inspect`).
-/

namespace MoveBytecode.Proofs

/-- A read or write through the reference at depth `k` of the operand stack. -/
def RefOperand (instr : Bytecode) (k : Nat) (a : Access) : Prop :=
  (instr = .eq ∨ instr = .neq) ∧ k ≤ 1 ∧ a = .read ∨
    instr = .readRef ∧ k = 0 ∧ a = .read ∨
    instr = .writeRef ∧ k = 0 ∧ a = .write ∨
    (∃ sig, instr = .vecPushBack sig) ∧ k = 1 ∧ a = .write ∨
    (∃ sig, instr = .vecPopBack sig) ∧ k = 0 ∧ a = .write ∨
    (∃ sig, instr = .vecSwap sig) ∧ k = 2 ∧ a = .write ∨
    ((∃ d, instr = .moveTo d) ∨ ∃ i, instr = .moveToGeneric i) ∧ k = 1 ∧ a = .read

/-- A direct read or write of local `j`. -/
def LocalOperand (instr : Bytecode) (j : Nat) (a : Access) : Prop :=
  instr = .copyLoc j ∧ a = .read ∨ instr = .moveLoc j ∧ a = .write ∨ instr = .stLoc j ∧ a = .write

/-- A direct write of a resource. -/
def GlobalOperand (instr : Bytecode) : Prop :=
  (∃ d, instr = .moveFrom d) ∨ (∃ d, instr = .moveTo d) ∨ (∃ i, instr = .moveFromGeneric i) ∨
    (∃ i, instr = .moveToGeneric i)

/-- How instruction `instr` of frame `fr`, whose shadow is `g`, accesses a
location (`exec_accessed`). -/
inductive AccessKind (g : ShadowFrame) (fr : Frame) (instr : Bytecode) :
    Access → Location → Option Nat → Prop where
  | inspect {k t : Nat} {r : Reference} : StackSlot g fr t r →
      fr.stack[k]? = some (.ref r) → g.stack[k]?.map (·.id) = some t →
      AccessKind g fr instr .inspect r.location (some t)
  | viaRef {a : Access} {k t : Nat} {r : Reference} : StackSlot g fr t r →
      fr.stack[k]? = some (.ref r) → g.stack[k]?.map (·.id) = some t → RefOperand instr k a →
      AccessKind g fr instr a r.location (some t)
  | direct {a : Access} {j : Nat} : LocalOperand instr j a →
      AccessKind g fr instr a ⟨.local_ fr.id j, []⟩ none
  | global {addr : Address} {sid : StructId} : GlobalOperand instr →
      AccessKind g fr instr .write ⟨.global addr sid, []⟩ none

theorem fresh_not_accessed {c : Creator} {v : Value} {n : Nat} {a : Access} {l : Location}
    {via : Option Nat} : Event.accessed a l via ∉ (IdTree.fresh c v n).2.2 := by
  rw [(fresh_spec c v n).2.2.2]; simp

/-- The reference at depth `k` of the segment, with its identity. -/
theorem refAt_spec {g : ShadowFrame} {fr : Frame} {k : Nat} {a : Nat × Reference}
    (h : (match fr.stack[k]? with
      | some (Value.ref r) => g.stack[k]?.bind fun t => pure (t.id, r)
      | _ => none) = some a) :
    fr.stack[k]? = some (.ref a.2) ∧ g.stack[k]?.map (·.id) = some a.1 ∧ StackSlot g fr a.1 a.2 := by
  refine ⟨?_, ?_, refAt_slot h⟩
  · split at h
    · next r hr =>
      simp only [Option.bind_eq_some_iff, Option.pure_def, Option.some.injEq] at h
      obtain ⟨t, -, rfl⟩ := h
      exact hr
    · cases h
  · split at h
    · simp only [Option.bind_eq_some_iff, Option.pure_def, Option.some.injEq] at h
      obtain ⟨t, ht, rfl⟩ := h
      simp [ht]
    · cases h

/-- An access through the reference at depth `k`. -/
theorem AccessKind.ofRefAt {g : ShadowFrame} {fr : Frame} {instr : Bytecode} {a : Access}
    {k : Nat} {x : Nat × Reference}
    (hx : (match fr.stack[k]? with
      | some (Value.ref r) => g.stack[k]?.bind fun t => pure (t.id, r)
      | _ => none) = some x) (hop : RefOperand instr k a) :
    AccessKind g fr instr a x.2.location (some x.1) := by
  obtain ⟨h1, h2, h3⟩ := refAt_spec hx
  exact .viaRef h3 h1 h2 hop

/-- An inspection through the reference at depth `k`. -/
theorem AccessKind.inspectAt {g : ShadowFrame} {fr : Frame} {instr : Bytecode}
    {k : Nat} {x : Nat × Reference}
    (hx : (match fr.stack[k]? with
      | some (Value.ref r) => g.stack[k]?.bind fun t => pure (t.id, r)
      | _ => none) = some x) :
    AccessKind g fr instr .inspect x.2.location (some x.1) := by
  obtain ⟨h1, h2, h3⟩ := refAt_spec hx
  exact .inspect h3 h1 h2

/-- **The accesses of an instruction.** -/
theorem exec_accessed {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {a : Access} {l : Location}
    {via : Option Nat} (ha : Event.accessed a l via ∈ evs) : AccessKind g fr instr a l via := by
  cases instr <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Prod.mk.injEq, guard, reduceCtorEq] at hex
  all_goals (try simp_all)
  case ret =>
    split at hex <;>
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp at ha
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldConst | ldTrue | ldFalse =>
    obtain ⟨a, -, -, rfl⟩ := hex
    exact absurd ha fresh_not_accessed
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 | not | negate | add | sub | mul | div | mod | bitOr | bitAnd
    | xor | or | and | lt | gt | le | ge | shl | shr =>
    obtain ⟨-, a, -, -, rfl⟩ := hex
    exact absurd ha fresh_not_accessed
  case pack | packVariant | vecPack | packGeneric | packVariantGeneric | packClosure
    | packClosureGeneric =>
    obtain ⟨a, -, -, -, rfl⟩ := hex
    simp at ha
  case callClosure =>
    split at hex
    all_goals (try split at hex)
    all_goals (try split at hex)
    all_goals (try split at hex)
    all_goals first
      | cases hex
      | (obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
         simp only [Option.some.injEq, Prod.mk.injEq] at hex
         obtain ⟨-, rfl⟩ := hex
         simp at ha)
      | simp at hex
  case unpack | unpackVariant | vecUnpack | unpackGeneric | unpackVariantGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨v, -, -, rfl⟩ := hex
      simp at ha
    · cases hex
  case eq | neq =>
    obtain ⟨a, b, ⟨-, a₁, -, -, rfl⟩, hm⟩ := hex
    split at hm
    · next _ _ ta ra tb rb h1 h0 =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, false_or,
        ] at ha
      rcases ha with ⟨rfl, rfl, rfl⟩ | ⟨rfl, rfl, rfl⟩ | ha
      · exact AccessKind.ofRefAt (x := (ta, ra)) h1 (by simp [RefOperand])
      · exact AccessKind.ofRefAt (x := (tb, rb)) h0 (by simp [RefOperand])
      · exact absurd ha fresh_not_accessed
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact absurd ha fresh_not_accessed
  case copyLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, Event.accessed.injEq] at ha
      rcases ha with ⟨rfl, rfl, rfl⟩ | ha
      · exact .direct (by simp [LocalOperand])
      · split at ha <;> simp at ha
    · cases hex
  case moveLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, Event.accessed.injEq, List.mem_append] at ha
      rcases ha with ⟨rfl, rfl, rfl⟩ | ha | ha
      · exact .direct (by simp [LocalOperand])
      · split at ha <;> simp at ha
      · simp at ha
    · cases hex
  case stLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, Event.accessed.injEq] at ha
      rcases ha with ⟨rfl, rfl, rfl⟩ | ha
      · exact .direct (by simp [LocalOperand])
      · split at ha <;> simp at ha
    · cases hex
  case call | callGeneric =>
    split at hex
    · split at hex
      · obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
        simp only [Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, rfl⟩ := hex
        simp at ha
      · cases hex
    · cases hex
  case readRef =>
    obtain ⟨a, b, hra, a₁, -, -, rfl⟩ := hex
    simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil,
      or_false] at ha
    obtain ⟨rfl, rfl, rfl⟩ := ha
    exact AccessKind.ofRefAt (x := (a, b)) hra (by simp [RefOperand])
  case writeRef =>
    obtain ⟨a, b, hra, a₁, -, a₂, -, -, rfl⟩ := hex
    simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil,
      or_false] at ha
    obtain ⟨rfl, rfl, rfl⟩ := ha
    exact AccessKind.ofRefAt (x := (a, b)) hra (by simp [RefOperand])
  case freezeRef =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp at ha
    · cases hm
  case testVariant | testVariantGeneric =>
    obtain ⟨a, b, hra, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨x, hx, -, rfl⟩ := hm
      obtain ⟨-, -, v, -, rfl⟩ := hx
      simp only [List.mem_cons, List.mem_append, Event.accessed.injEq, reduceCtorEq, false_or,
        List.not_mem_nil, or_false] at ha
      rcases ha with ⟨rfl, rfl, rfl⟩ | ha
      · exact AccessKind.inspectAt (x := (a, b)) hra
      · exact absurd ha fresh_not_accessed
    · cases hm
  case mutBorrowField | immBorrowField | mutBorrowFieldGeneric | immBorrowFieldGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        simp at ha
      · cases hm
    · cases hm
  case mutBorrowVariantField | immBorrowVariantField | mutBorrowVariantFieldGeneric
    | immBorrowVariantFieldGeneric =>
    obtain ⟨a, b, hra, hm⟩ := hex
    split at hm
    · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq,
          List.not_mem_nil, or_false] at ha
        obtain ⟨rfl, rfl, rfl⟩ := ha
        exact AccessKind.inspectAt (x := (a, b)) hra
      · cases hm
    · cases hm
  case vecLen =>
    obtain ⟨a, b, hra, a₁, b₁, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, false_or] at ha
    rcases ha with ⟨rfl, rfl, rfl⟩ | ha
    · exact AccessKind.inspectAt (x := (a, b)) hra
    · exact absurd ha fresh_not_accessed
  case vecImmBorrow | vecMutBorrow =>
    obtain ⟨a, b, hra, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq,
        List.not_mem_nil, or_false] at ha
      obtain ⟨rfl, rfl, rfl⟩ := ha
      exact AccessKind.inspectAt (x := (a, b)) hra
    · cases hm
  case vecPushBack =>
    obtain ⟨a, -, a₁, b, hra, a₂, -, -, rfl⟩ := hex
    simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil,
      or_false] at ha
    obtain ⟨rfl, rfl, rfl⟩ := ha
    exact AccessKind.ofRefAt (x := (a₁, b)) hra (by simp [RefOperand])
  case vecPopBack =>
    obtain ⟨a, b, hra, a₁, -, a₂, -, a₃, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil,
        or_false] at ha
      obtain ⟨rfl, rfl, rfl⟩ := ha
      exact AccessKind.ofRefAt (x := (a, b)) hra (by simp [RefOperand])
    · cases hm
  case vecSwap =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨x, hra, y, -, -, rfl⟩ := hex
        simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil,
          or_false] at ha
        obtain ⟨rfl, rfl, rfl⟩ := ha
        exact AccessKind.ofRefAt hra (by simp [RefOperand])
      · cases hex
    · cases hex
  case exists_ =>
    obtain ⟨a, -, a₁, b, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_append, List.mem_singleton, reduceCtorEq, or_false] at ha
    exact absurd ha fresh_not_accessed
  case existsGeneric =>
    obtain ⟨_, _, _, _, _, _, _, _, a, b, ⟨_, a', _, _, rfl⟩, _, rfl⟩ := hex
    simp only [List.mem_append, List.mem_singleton, reduceCtorEq, or_false] at ha
    exact absurd ha fresh_not_accessed
  case moveFrom | moveFromGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff] at hex
      obtain ⟨v, -, hex⟩ := hex
      split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨k, -, t, -, -, rfl⟩ := hex
        simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil,
          or_false] at ha
        obtain ⟨rfl, rfl, rfl⟩ := ha
        exact .global (by simp [GlobalOperand])
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
        simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil,
          or_false] at ha
        rcases ha with ⟨rfl, rfl, rfl⟩ | ⟨rfl, rfl, rfl⟩
        · exact AccessKind.ofRefAt hra (by simp [RefOperand])
        · exact .global (by simp [GlobalOperand])
      · cases hm
    · cases hm
  case mutBorrowLoc | immBorrowLoc =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp at ha
    · cases hex
  case mutBorrowGlobal | immBorrowGlobal | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        simp at ha
      · cases hex
    · cases hex

/-- **A step that borrows only inspects.** -/
theorem exec_borrow_inspect {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event}
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {i : Nat} {r : Reference}
    {par : Option Nat} (hb : Event.borrowed i r par ∈ evs) {a : Access} {l : Location}
    {via : Option Nat} (ha : Event.accessed a l via ∈ evs) : a = .inspect := by
  obtain ⟨hframes, hst, hhe, fresh⟩ := h
  cases hframes with
  | nil => simp [Shadow.exec] at hex
  | cons g hs =>
    obtain ⟨-, hnew⟩ := exec_borrowed rfl hex hb
    cases exec_accessed rfl hex ha with
    | inspect => rfl
    | viaRef _ _ _ hop =>
      exfalso
      rcases hnew with ⟨rfl, -⟩ | ⟨_, rfl | rfl⟩ | ⟨_, rfl | rfl | rfl | rfl⟩ |
        ⟨_, rfl | rfl | rfl | rfl⟩ | ⟨_, rfl | rfl⟩ | ⟨_, rfl | rfl | rfl | rfl⟩ <;> simp [RefOperand] at hop
    | direct hop =>
      exfalso
      rcases hnew with ⟨rfl, -⟩ | ⟨_, rfl | rfl⟩ | ⟨_, rfl | rfl | rfl | rfl⟩ |
        ⟨_, rfl | rfl | rfl | rfl⟩ | ⟨_, rfl | rfl⟩ | ⟨_, rfl | rfl | rfl | rfl⟩ <;> simp [LocalOperand] at hop
    | global hop =>
      exfalso
      rcases hnew with ⟨rfl, -⟩ | ⟨_, rfl | rfl⟩ | ⟨_, rfl | rfl | rfl | rfl⟩ |
        ⟨_, rfl | rfl | rfl | rfl⟩ | ⟨_, rfl | rfl⟩ | ⟨_, rfl | rfl | rfl | rfl⟩ <;> simp [GlobalOperand] at hop

end MoveBytecode.Proofs
