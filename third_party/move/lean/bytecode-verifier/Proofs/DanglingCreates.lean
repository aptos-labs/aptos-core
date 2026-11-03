-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.DanglingDefs
import Proofs.RefTrace
import Proofs.RefUpdate

/-!
# A borrowed reference is valid

UNTRUSTED (checked by the kernel). Every reference a core step borrows is
valid in the state after the step (`coreStep_creates`): a borrow of a local
or a resource designates a value that is there, a field, variant field or
element borrow designates a value inside the one its parent designates, which
the step dereferenced, and `FreezeRef` designates what its parent designates,
which the step uses.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

theorem creates_fresh_not_borrowed {c : Creator} {v : Value} {n i : Nat} {r : Reference}
    {par : Option Nat} : Event.borrowed i r par ∉ (IdTree.fresh c v n).2.2 := by
  rw [(fresh_spec c v n).2.2.2]; simp

/-- The instructions whose annotation records a `borrowed` event. -/
def BorrowsNew (instr : Bytecode) (fr : Frame) (evs : List Event) : Prop :=
  (instr = .freezeRef ∧ ∃ t rp, fr.stack[0]? = some (.ref rp) ∧ Event.used t rp ∈ evs) ∨
  (∃ j, instr = .mutBorrowLoc j ∨ instr = .immBorrowLoc j) ∨
  (∃ k, instr = .mutBorrowField k ∨ instr = .immBorrowField k ∨
    instr = .mutBorrowFieldGeneric k ∨ instr = .immBorrowFieldGeneric k) ∨
  (∃ k, instr = .mutBorrowVariantField k ∨ instr = .immBorrowVariantField k ∨
    instr = .mutBorrowVariantFieldGeneric k ∨ instr = .immBorrowVariantFieldGeneric k) ∨
  (∃ k, instr = .vecImmBorrow k ∨ instr = .vecMutBorrow k) ∨
  (∃ d, instr = .mutBorrowGlobal d ∨ instr = .immBorrowGlobal d ∨
    instr = .mutBorrowGlobalGeneric d ∨ instr = .immBorrowGlobalGeneric d)

/-- **A `borrowed` event** is recorded only by a borrow or `FreezeRef`, of the
reference on top of the segment after the step. -/
theorem exec_borrowed {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {i : Nat}
    {r : Reference} {par : Option Nat} {g : ShadowFrame} {hs : List ShadowFrame}
    (hh : h.frames = g :: hs) (hex : Shadow.exec P m instr s fr o h = some (post, evs))
    (hb : Event.borrowed i r par ∈ evs) :
    Shadow.topAfter o = some (.ref r) ∧ BorrowsNew instr fr evs := by
  cases instr <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Prod.mk.injEq, guard, reduceCtorEq] at hex
  all_goals (try simp_all)
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldConst | ldTrue | ldFalse =>
    obtain ⟨a, -, -, rfl⟩ := hex
    exact absurd hb creates_fresh_not_borrowed
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 | not | negate | add | sub | mul | div | mod | bitOr | bitAnd
    | xor | or | and | lt | gt | le | ge | shl | shr =>
    obtain ⟨-, a, -, -, rfl⟩ := hex
    exact absurd hb creates_fresh_not_borrowed
  case pack | packVariant | vecPack | packGeneric | packVariantGeneric | packClosure
    | packClosureGeneric =>
    obtain ⟨a, -, -, -, rfl⟩ := hex
    simp at hb
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
         simp at hb)
      | simp at hex
  case unpack | unpackVariant | vecUnpack | unpackGeneric | unpackVariantGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨v, -, -, rfl⟩ := hex
      simp at hb
    · cases hex
  case eq | neq =>
    obtain ⟨a, b, ⟨-, a₁, -, -, rfl⟩, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, reduceCtorEq, false_or] at hb
      exact absurd hb creates_fresh_not_borrowed
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact absurd hb creates_fresh_not_borrowed
  case readRef =>
    obtain ⟨a, b, -, a₁, -, -, rfl⟩ := hex
    simp at hb
  case writeRef =>
    obtain ⟨a, b, -, a₁, -, a₂, -, -, rfl⟩ := hex
    simp at hb
  case freezeRef =>
    obtain ⟨a, b, hra, -, hm⟩ := hex
    split at hm
    · next r' htop =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.borrowed.injEq, reduceCtorEq, List.not_mem_nil, or_false,
        false_or] at hb
      obtain ⟨-, rfl, -⟩ := hb
      refine ⟨htop, Or.inl ⟨rfl, a, b, ?_, by simp⟩⟩
      split at hra
      · next r'' hr'' =>
        simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hra
        obtain ⟨-, -, -, rfl⟩ := hra
        exact hr''
      · cases hra
    · cases hm
  case testVariant | testVariantGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨x, hx, -, rfl⟩ := hm
      obtain ⟨-, -, v, -, rfl⟩ := hx
      simp only [List.mem_cons, reduceCtorEq, false_or, List.mem_append] at hb
      rcases hb with hb | hb
      · exact absurd hb creates_fresh_not_borrowed
      · cases hb
    · cases hm
  case mutBorrowField | immBorrowField | mutBorrowVariantField | immBorrowVariantField
    | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
    | immBorrowVariantFieldGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
      split at hm
      · next r' htop =>
        simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        simp only [List.mem_cons, Event.borrowed.injEq, List.not_mem_nil, reduceCtorEq, or_false,
          false_or] at hb
        obtain ⟨-, rfl, -⟩ := hb
        exact ⟨htop, by simp [BorrowsNew]⟩
      · cases hm
    · cases hm
  case vecLen =>
    obtain ⟨a, b, -, a₁, b₁, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_cons, reduceCtorEq, false_or] at hb
    exact absurd hb creates_fresh_not_borrowed
  case vecImmBorrow | vecMutBorrow =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · next r' htop =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons, Event.borrowed.injEq, List.not_mem_nil, reduceCtorEq, or_false,
        false_or] at hb
      obtain ⟨-, rfl, -⟩ := hb
      exact ⟨htop, by simp [BorrowsNew]⟩
    · cases hm
  case vecPushBack =>
    obtain ⟨a, -, a₁, b, -, a₂, -, -, rfl⟩ := hex
    simp at hb
  case vecPopBack =>
    obtain ⟨a, b, -, a₁, -, a₂, -, a₃, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp at hb
    · cases hm
  case vecSwap =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨x, -, y, -, -, rfl⟩ := hex
        simp at hb
      · cases hex
    · cases hex
  case moveTo | moveToGeneric =>
    obtain ⟨a, -, a₁, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff] at hm
      obtain ⟨x, -, hm⟩ := hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        simp at hb
      · cases hm
    · cases hm
  case mutBorrowLoc | immBorrowLoc =>
    split at hex
    · next r' htop =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, Event.borrowed.injEq, List.not_mem_nil, or_false] at hb
      obtain ⟨-, rfl, -⟩ := hb
      exact ⟨htop, by simp [BorrowsNew]⟩
    · cases hex
  case mutBorrowGlobal | immBorrowGlobal | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    split at hex
    · next r' htop =>
      split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        simp only [List.mem_cons, Event.borrowed.injEq, List.not_mem_nil, reduceCtorEq,
          or_false] at hb
        obtain ⟨-, rfl, -⟩ := hb
        exact ⟨htop, by simp [BorrowsNew]⟩
      · cases hex
    · cases hex
  case exists_ =>
    obtain ⟨a, -, a₁, b, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_append, List.mem_singleton, reduceCtorEq, or_false] at hb
    exact absurd hb creates_fresh_not_borrowed
  case existsGeneric =>
    obtain ⟨_, _, _, _, _, _, _, _, a, b, ⟨_, a', _, _, rfl⟩, _, rfl⟩ := hex
    simp only [List.mem_append, List.mem_singleton, reduceCtorEq, or_false] at hb
    exact absurd hb creates_fresh_not_borrowed
  case moveFrom | moveFromGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff] at hex
      obtain ⟨v, -, hex⟩ := hex
      split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨k, -, t, -, -, rfl⟩ := hex
        simp at hb
      · cases hex
    · cases hex
  case copyLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, reduceCtorEq, false_or] at hb
      split at hb <;> simp at hb
    · cases hex
  case moveLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, reduceCtorEq, false_or, List.mem_append] at hb
      split at hb <;> simp at hb
    · cases hex
  case stLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, reduceCtorEq, false_or] at hb
      split at hb <;> simp at hb
    · cases hex
  case call | callGeneric =>
    split at hex
    · split at hex
      · obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
        simp only [Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, rfl⟩ := hex
        simp at hb
      · cases hex
    · cases hex
  case ret =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp at hb
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp at hb


/-! ## The borrows -/

/-- Validity is kept when only the current frame's operand stack and position
change. -/
private theorem creates_valid_top {s s' : State} {fr fr' : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) (hs' : s'.frames = fr' :: rest) (hid : fr'.id = fr.id)
    (hloc : fr'.locals = fr.locals) (hst : s'.storage = s.storage) (hhe : s'.held = s.held)
    (hinv : s'.invalidations = s.invalidations) {l : Location} {n : Nat} (hv : Valid s l n) :
    Valid s' l n := by
  have hroot := root?_replaceTop hsf hs' hid hloc hst hhe l.root
  have heq := deref?_congr (l := l) hroot hinv n
  exact ⟨by rw [heq]; exact hv.1, by rw [hinv]; exact hv.2⟩

/-- A location that resolves is valid for a reference created now. -/
private theorem creates_valid_new {s : State} {l : Location} (h : (s.read? l).isSome = true) :
    Valid s l s.invalidations.length :=
  ⟨deref?_isSome.2 ⟨fun e he => by simp [invalidatedSince_self] at he, h⟩, Nat.le_refl _⟩

theorem creates_shapedFields_length {P : Program} :
    ∀ {vs : List Value} {ts : List RuntimeType}, shapedFields P vs ts = true → vs.length = ts.length
  | [], [], _ => rfl
  | _ :: _, _ :: _, h => by
    simp only [shapedFields, Bool.and_eq_true] at h
    simp [creates_shapedFields_length h.2]
  | [], _ :: _, h => by simp [shapedFields] at h
  | _ :: _, [], h => by simp [shapedFields] at h

/-- A struct with declared fields is not an enum. -/
theorem creates_not_variant {P : Program} {sid : StructId} {tys : List RuntimeType}
    {tag : VariantIndex} (h : P.fieldTypes sid = some tys) : P.variantFieldTypes sid tag = none := by
  unfold Program.fieldTypes at h
  unfold Program.variantFieldTypes
  cases hs : P.structType? sid with
  | none => rfl
  | some info =>
    rw [hs] at h
    simp only [bind, Option.bind_some] at h ⊢
    cases hf : info.definition.fieldInformation <;> simp_all

/-- The value the parent of a borrow designates, as the borrow dereferenced
it: it is the value at the parent's location in `s`, shaped, of the parent's
target type. -/
private theorem creates_parent {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    (hinv : StateInv false P s) (htyped : StateTyped P s) (hsf : s.frames = fr :: rest)
    {rp : Reference} {st : List Value} (hst : fr.stack = .ref rp :: st) {s₁ : State}
    (hs₁ : s₁ = { s with frames := { fr with stack := st } :: rest }) {w : Value}
    (hw : s₁.deref? rp.location rp.since = some w) (hrig : rigid rp.target = true) :
    s.read? rp.location = some w ∧ shaped P w = true ∧ w.type = rp.target := by
  subst hs₁
  rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
    (by simp [hsf]) rfl rfl rfl] at hw
  exact ⟨deref?_read hw,
    deref_shaped_rigid (fr := fr) htyped hinv (by simp [hsf]) (by simp [hst]) hw hrig⟩

theorem creates_mapM_zip {α β : Type} {g : α → Exec β} :
    ∀ {l : List α} {l' : List β}, l.mapM g = .ok l' → ∀ p ∈ l.zip l', g p.1 = .ok p.2
  | [], _, _, p, hp => by simp at hp
  | a :: l, l', h, p, hp => by
    rw [List.mapM_cons, bind_ok'] at h
    obtain ⟨b, hb, h⟩ := h
    rw [bind_ok'] at h
    obtain ⟨bs, hbs, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    rcases List.mem_cons.1 hp with rfl | hp
    · exact hb
    · exact creates_mapM_zip hbs p hp

private theorem creates_borrowLoc {s s' : State} {fr : Frame} {rest : List Frame}
    {mutable : Bool} {j : LocalIndex} {r : Reference} (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowLoc P s fr rest mutable j) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref r)) : Valid s' r.location r.since := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold borrowLoc at h
  obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h hcont).elim
  | some v =>
    dsimp only at h
    obtain ⟨tys, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
    split at h
    · exact (done_wrong h hcont).elim
    · have ho := Outcome.running.inj (done_next h)
      subst ho
      simp only [Shadow.topAfter, List.head?_cons, advance, push, borrow, Option.bind_eq_bind,
        Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
      subst htop
      refine creates_valid_top hsf (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (creates_valid_new ?_)
      rw [read?_local (frame?_head hsf), need_ok' hslot]
      rfl

private theorem creates_borrowGlobal {P : Program} {s s' : State} {fr : Frame}
    {rest : List Frame} {mutable : Bool} {d : StructDefinitionIndex} {tys : List RuntimeType}
    {r : Reference} (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowGlobal P s fr rest mutable d tys) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref r)) : Valid s' r.location r.since := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold borrowGlobal at h
  obtain ⟨sid, -, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, -, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
  split at h
  · next hget =>
    have ho := Outcome.running.inj (done_next h)
    subst ho
    simp only [Shadow.topAfter, List.head?_cons, advance, push, borrow, Option.bind_eq_bind,
      Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
    subst htop
    refine creates_valid_top hsf (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (creates_valid_new ?_)
    simpa [State.read?, State.root?, Value.get?] using hget
  · exact (done_fail h hcont).elim

private theorem creates_freezeRef {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {r : Reference} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .freezeRef) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref r))
    (hparent : ∀ rp, fr.stack[0]? = some (.ref rp) → Valid s rp.location rp.since) :
    Valid s' r.location r.since := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨⟨rp, fr₁⟩, hpr, h⟩ := done_bind h hcont popRef_ends
  obtain ⟨st, hst, rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · have ho := Outcome.running.inj (done_next h)
    subst ho
    simp only [Shadow.topAfter, List.head?_cons, advance, push, Option.bind_eq_bind,
      Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
    subst htop
    exact creates_valid_top hsf (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (hparent rp (by simp [hst]))

private theorem creates_borrowField {P : Program} {s s' : State} {fr : Frame}
    {rest : List Frame} {info : FunctionInfo} {mutable : Bool} {hh : FieldHandleIndex}
    {tys : List RuntimeType}
    {r : Reference} (hinv : StateInv false P s) (htyped : StateTyped P s)
    (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowField P s fr rest info mutable hh tys) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref r)) : Valid s' r.location r.since := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold borrowField at h
  obtain ⟨fh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, htys, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, ht, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨rp, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst, rfl, htarget, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨w, hw, h⟩ := done_bind h hcont deref_ends
    have ho := Outcome.running.inj (done_finish rfl h)
    subst ho
    simp only [Shadow.topAfter, List.head?_cons, advance, pushAll, borrow, List.reverse_cons,
      List.reverse_nil, List.nil_append, List.cons_append, Option.bind_eq_bind,
      Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
    subst htop
    refine creates_valid_top hsf (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (creates_valid_new ?_)
    obtain ⟨hread, hshape, htype⟩ :=
      creates_parent hinv htyped hsf hst rfl (deref_ok hw) (by rw [htarget]; rfl)
    rw [read?_child, hread]
    rw [htarget] at htype
    have htys' := need_ok' htys
    have ht' := need_ok' ht
    cases w with
    | struct sid' fields =>
      simp only [Value.type, RuntimeType.struct.injEq] at htype
      subst htype
      simp only [shaped, htys'] at hshape
      have hlen := creates_shapedFields_length hshape
      have hlt : fh.field < fields.length := by
        rw [hlen]; exact (List.getElem?_eq_some_iff.1 ht').1
      simp [Value.child?, List.getElem?_eq_getElem hlt]
    | table sid' fields ks es =>
      simp only [Value.type, RuntimeType.struct.injEq] at htype
      subst htype
      simp only [shaped, htys', Bool.and_eq_true] at hshape
      have hlen := creates_shapedFields_length hshape.1.2
      have hlt : fh.field < fields.length := by
        rw [hlen]; exact (List.getElem?_eq_some_iff.1 ht').1
      simp [Value.child?, List.getElem?_eq_getElem hlt]
    | variant sid' tag fields =>
      simp only [Value.type, RuntimeType.struct.injEq] at htype
      subst htype
      simp [shaped, creates_not_variant (tag := tag) htys'] at hshape
    | bool _ | int _ _ | address _ | signer _ | vector _ _ | ref _ | closure _ _ _ _ _ _ =>
      simp [Value.type] at htype
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

private theorem creates_borrowVariantField {P : Program} {s s' : State} {fr : Frame}
    {rest : List Frame} {info : FunctionInfo} {mutable : Bool} {hh : VariantFieldHandleIndex}
    {tys : List RuntimeType}
    {r : Reference} (hinv : StateInv false P s) (htyped : StateTyped P s)
    (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowVariantField P s fr rest info mutable hh tys) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref r)) : Valid s' r.location r.since := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold borrowVariantField at h
  obtain ⟨vfh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, htys, h⟩ := done_bind h hcont (mapM_ends (fun _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)
    obtain ⟨⟨rp, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst, rfl, htarget, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨w, hw, h⟩ := done_bind h hcont deref_ends
    split at h
    · next sid' tag fields =>
      split at h
      · next tag' t hfind =>
        have ho := Outcome.running.inj (done_finish rfl h)
        subst ho
        simp only [Shadow.topAfter, List.head?_cons, advance, pushAll, borrow, List.reverse_cons,
          List.reverse_nil, List.nil_append, List.cons_append, Option.bind_eq_bind,
          Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
        subst htop
        refine creates_valid_top hsf (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (creates_valid_new ?_)
        obtain ⟨hread, hshape, htype⟩ :=
          creates_parent hinv htyped hsf hst rfl (deref_ok hw) (by rw [htarget]; rfl)
        rw [htarget] at htype
        simp only [Value.type, RuntimeType.struct.injEq] at htype
        subst htype
        have hmem := List.mem_of_find?_eq_some hfind
        have htag : tag' = tag := by simpa using List.find?_some hfind
        subst htag
        have hg := creates_mapM_zip htys _ hmem
        simp only at hg
        rw [bind_ok'] at hg
        obtain ⟨_, -, hg⟩ := hg
        rw [bind_ok'] at hg
        obtain ⟨fts, hfts, hg⟩ := hg
        have hfts' := need_ok' hfts
        have ht' := need_ok' hg
        simp only [shaped, hfts'] at hshape
        have hlen := creates_shapedFields_length hshape
        have hlt : vfh.field < fields.length := by
          rw [hlen]; exact (List.getElem?_eq_some_iff.1 ht').1
        rw [read?_child, hread]
        simp [Value.child?, List.getElem?_eq_getElem hlt]
      · exact (done_fail h hcont).elim
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

private theorem creates_borrowElement {s s' : State} {fr : Frame}
    {rest : List Frame} {info : FunctionInfo} {mutable : Bool} {sig : SignatureIndex}
    {r : Reference} (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowElement s fr rest info mutable sig) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref r)) : Valid s' r.location r.since := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold borrowElement at h
  obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨i, fr₁⟩, hpi, h⟩ := done_bind h hcont popIndex_ends
  obtain ⟨x, st₁, hst₁, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨rp, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, helems, h⟩ := done_bind h hcont derefVector_ends
  split at h
  · next hlt =>
    have ho := Outcome.running.inj (done_finish rfl h)
    subst ho
    simp only [Shadow.topAfter, List.head?_cons, advance, pushAll, borrow, List.reverse_cons,
      List.reverse_nil, List.nil_append, List.cons_append, Option.bind_eq_bind,
      Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
    subst htop
    refine creates_valid_top hsf (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (by rfl) (creates_valid_new ?_)
    obtain ⟨u, hu⟩ := derefVector_ok helems
    rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl rfl] at hu
    rw [read?_child, deref?_read hu]
    simp [Value.child?, List.getElem?_eq_getElem hlt]
  · exact (done_fail h hcont).elim


/-! ## Core steps -/

/-- A core step that continues, unfolded: its instruction, and its
annotation. -/
private theorem creates_parts {P : Program} {s s' : State} {h : Shadow} {post : ShadowPost}
    {evs : List Event} (hstep : step P s = .done (.running s'))
    (hc : Shadow.coreStep P s (.running s') h = some (post, evs)) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧
      stepOf (Step.exec P s fr rest info code instr) = .done (.running s') ∧
      ∃ post', Shadow.exec P info.moduleId instr s fr (.running s') h = some (post', evs) := by
  unfold step at hstep
  split at hstep
  · simp at hstep
  · next fr rest hsf =>
    split at hstep
    · simp at hstep
    · next info hinfo =>
      split at hstep
      · simp at hstep
      · next code hcode =>
        split at hstep
        · simp at hstep
        · next instr hinstr =>
          refine ⟨fr, rest, info, code, instr, hsf, hstep, ?_⟩
          unfold Shadow.coreStep at hc
          simp only [hsf, hinfo, hcode, hinstr, Option.bind_eq_bind, Option.bind_some,
            Option.bind_eq_some_iff] at hc
          obtain ⟨⟨post', evs'⟩, hex, hc⟩ := hc
          split at hc
          · simp only [Option.some.injEq, Prod.mk.injEq] at hc
            obtain ⟨-, rfl⟩ := hc
            exact ⟨post', hex⟩
          · cases hc

/-- An annotated step has a current shadow frame. -/
private theorem creates_frames {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event}
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) : ∃ g hs, h.frames = g :: hs := by
  cases hfr : h.frames with
  | nil => simp [Shadow.exec, hfr] at hex
  | cons g hs => exact ⟨g, hs, rfl⟩

/-- **Every reference a core step borrows is valid** after the step: it
designates a value that is there, and nothing was invalidated since its
creation. -/
theorem coreStep_creates {P : Program} {s s' : State} {h : Shadow} {post : ShadowPost}
    {evs : List Event} (hpre : Pre false P s h) (htyped : StateTyped P s)
    (hstep : step P s = .done (.running s'))
    (hc : Shadow.coreStep P s (.running s') h = some (post, evs))
    (huses : ∀ t r, Event.used t r ∈ evs → Valid s r.location r.since)
    {i : Nat} {r : Reference} {par : Option Nat} (hb : Event.borrowed i r par ∈ evs) :
    Valid s' r.location r.since := by
  obtain ⟨fr, rest, info, code, instr, hsf, hstep', post', hex⟩ := creates_parts hstep hc
  obtain ⟨g, hs, hh⟩ := creates_frames hex
  obtain ⟨htop, hk⟩ := exec_borrowed hh hex hb
  rcases hk with ⟨rfl, t, rp, hrp, hu⟩ | ⟨j, rfl | rfl⟩ | ⟨k, rfl | rfl | rfl | rfl⟩ |
    ⟨k, rfl | rfl | rfl | rfl⟩ | ⟨k, rfl | rfl⟩ | ⟨d, rfl | rfl | rfl | rfl⟩
  · refine creates_freezeRef hsf hstep' htop fun rp' hrp' => ?_
    rw [hrp, Option.some.injEq, Value.ref.injEq] at hrp'
    subst hrp'
    exact huses t rp hu
  · exact creates_borrowLoc hsf hstep' htop
  · exact creates_borrowLoc hsf hstep' htop
  · exact creates_borrowField hpre.inv htyped hsf hstep' htop
  · exact creates_borrowField hpre.inv htyped hsf hstep' htop
  · obtain ⟨x, -, hstep'⟩ := done_bind hstep' rfl (ends_need _ _)
    obtain ⟨tys, -, hstep'⟩ := done_bind hstep' rfl genericArgs_ends
    exact creates_borrowField hpre.inv htyped hsf hstep' htop
  · obtain ⟨x, -, hstep'⟩ := done_bind hstep' rfl (ends_need _ _)
    obtain ⟨tys, -, hstep'⟩ := done_bind hstep' rfl genericArgs_ends
    exact creates_borrowField hpre.inv htyped hsf hstep' htop
  · exact creates_borrowVariantField hpre.inv htyped hsf hstep' htop
  · exact creates_borrowVariantField hpre.inv htyped hsf hstep' htop
  · obtain ⟨x, -, hstep'⟩ := done_bind hstep' rfl (ends_need _ _)
    obtain ⟨tys, -, hstep'⟩ := done_bind hstep' rfl genericArgs_ends
    exact creates_borrowVariantField hpre.inv htyped hsf hstep' htop
  · obtain ⟨x, -, hstep'⟩ := done_bind hstep' rfl (ends_need _ _)
    obtain ⟨tys, -, hstep'⟩ := done_bind hstep' rfl genericArgs_ends
    exact creates_borrowVariantField hpre.inv htyped hsf hstep' htop
  · exact creates_borrowElement hsf hstep' htop
  · exact creates_borrowElement hsf hstep' htop
  · exact creates_borrowGlobal hsf hstep' htop
  · exact creates_borrowGlobal hsf hstep' htop
  · obtain ⟨x, -, hstep'⟩ := done_bind hstep' rfl (ends_need _ _)
    obtain ⟨tys, -, hstep'⟩ := done_bind hstep' rfl genericArgs_ends
    exact creates_borrowGlobal hsf hstep' htop
  · obtain ⟨x, -, hstep'⟩ := done_bind hstep' rfl (ends_need _ _)
    obtain ⟨tys, -, hstep'⟩ := done_bind hstep' rfl genericArgs_ends
    exact creates_borrowGlobal hsf hstep' htop

end MoveBytecode.Proofs
