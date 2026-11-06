-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.NoEscapeM3

/-!
# What a traced step does to the frames

UNTRUSTED (checked by the kernel). A traced step either runs an instruction
of the current frame, which keeps every frame's identity, function and
activation (`Frame.key`) and records no frame entry or end; or calls a
function with code, pushing a frame entered with the identities of the
reference arguments as its parameters; or returns to a caller, which gets
the returned references (`traced_shape`). A return from the transaction
ends the only frame (`traced_returned_shape`). The values passed and
returned conform to slot types, so their references are whole slot values.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-- What identifies a frame across steps: its identity, function and whether it
counts as an activation. -/
def _root_.MoveBytecode.Frame.key (fr : Frame) : FrameId × FunctionRef × Bool :=
  (fr.id, fr.function, fr.countedActive)

/-! ## The keys of the frames across a step of the current frame -/

theorem setRoot_keys (s : State) (root : Root) (v : Value) :
    (s.setRoot root v).frames.map Frame.key = s.frames.map Frame.key := by
  cases root <;> simp [State.setRoot, Function.comp_def]
  intro a _
  split <;> rfl

theorem update_keys {s s₂ : State} {l : Location} {v : Value} (h : update s l v = .ok s₂) :
    s₂.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨s₁, hw, rfl⟩ := update_ok h
  simp only [State.write?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
    Option.some.injEq] at hw
  obtain ⟨w, -, w', -, rfl⟩ := hw
  exact setRoot_keys s l.root w'

theorem writeBack_keys : ∀ {s s' : State} {refs : List Reference} {ws : List Value},
    writeBack s refs ws = some s' → s'.frames.map Frame.key = s.frames.map Frame.key
  | s, s', [], [], h => by simp only [writeBack, Option.some.injEq] at h; rw [h]
  | s, s', r :: refs, w :: ws, h => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₁, hs₁, h⟩ := h
    rw [writeBack_keys h]
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hs₁
    obtain ⟨_, -, _, -, rfl⟩ := hs₁
    exact setRoot_keys _ _ _
  | _, _, [], _ :: _, h => by simp [writeBack] at h
  | _, _, _ :: _, [], h => by simp [writeBack] at h

section Keys

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

theorem fk_next {s₁ : State} {fr' : Frame} (hsf : s.frames = fr :: rest)
    (h : stepOf (next s₁ rest fr') = .done (.running s')) (hk : fr'.key = fr.key) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain rfl := Outcome.running.inj (done_next h)
  simp [hsf, hk]

theorem fk_finish {s₁ : State} {vals : List Value} (hsf : s.frames = fr :: rest)
    (h : stepOf (finish s₁ vals) = .done (.running s'))
    (hk : s₁.frames.map Frame.key = (fr :: rest).map Frame.key) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨fr₁, rest₁, hs₁, ho⟩ := done_finish_ex rfl h
  obtain rfl := Outcome.running.inj ho
  rw [hsf, ← hk, hs₁]
  rfl

theorem fk_finish_top {st : List Value} {vals : List Value} (hsf : s.frames = fr :: rest)
    (h : stepOf (finish { s with frames := { fr with stack := st } :: rest } vals) =
      .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key :=
  fk_finish hsf h rfl

theorem fk_pop (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .pop) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · exact fk_next hsf h rfl

theorem fk_copyLoc {i : LocalIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.copyLoc i)) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    dsimp only at h
    split at h
    · exact fk_next hsf h rfl
    · exact (done_wrong h rfl).elim

theorem fk_moveLoc {i : LocalIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.moveLoc i)) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v => exact fk_next hsf h rfl

theorem fk_stLoc {i : LocalIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.stLoc i)) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpop
  dsimp only at h
  obtain ⟨tys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨t, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  · cases slot with
    | none => exact fk_next hsf h rfl
    | some old =>
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      · exact fk_next hsf h rfl

theorem fk_equality {positive : Bool} (hsf : s.frames = fr :: rest)
    (h : stepOf (equality P s fr positive >>= fun fr' => next s rest fr') = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨fr', hx, h'⟩ := done_bind h rfl equality_ends
  obtain ⟨-, -, -, -, -, -, c, hR⟩ := equality_ok hx
  rw [hR.2] at h'
  exact fk_next hsf h' rfl

theorem fk_testVariant {hh : StructVariantHandleIndex} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.testEnum P s fr rest info hh tys) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨vh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
    split at h
    · exact fk_finish_top hsf h
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

theorem fk_readRef (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .readRef) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st, -, rfl⟩ := popRef_ok hpr
  dsimp only at h
  obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
  split at h
  · exact (done_wrong h rfl).elim
  · exact fk_finish_top hsf h

theorem fk_freezeRef (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .freezeRef) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st, -, rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · exact fk_next hsf h rfl

theorem fk_borrowLoc {mutable : Bool} {i : LocalIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowLoc P s fr rest mutable i) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  unfold borrowLoc at h
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    split at h
    · exact (done_wrong h rfl).elim
    · exact fk_next hsf h rfl

theorem fk_packClosure {hd : FunctionHandleIndex} {mask : BitVec 64} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.packClosure P s fr rest info hd mask tys) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  unfold Step.packClosure at h
  obtain ⟨target, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨targetInfo, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨paramTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  rcases hsplit : maskSplit mask paramTys with ⟨capturedTys, remaining⟩
  rw [hsplit] at h
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h rfl popN_ends
  obtain ⟨-, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · exact fk_next hsf h rfl

theorem fk_borrowField {mutable : Bool} {hh : FieldHandleIndex} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowField P s fr rest info mutable hh tys) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  unfold borrowField at h
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h rfl deref_ends
    exact fk_finish_top hsf h
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

theorem fk_borrowVariantField {mutable : Bool} {hh : VariantFieldHandleIndex}
    {tys : List RuntimeType} (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowVariantField P s fr rest info mutable hh tys) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  unfold borrowVariantField at h
  obtain ⟨vfh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h rfl (mapM_ends (fun _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
    split at h
    · split at h
      · exact fk_finish_top hsf h
      · exact (done_fail h rfl).elim
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

theorem fk_borrowGlobal {mutable : Bool} {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowGlobal P s fr rest mutable d tys) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  unfold borrowGlobal at h
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h rfl popAddress_ends
  obtain ⟨st, -, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
  split at h
  · exact fk_next hsf h rfl
  · exact (done_fail h rfl).elim

theorem fk_vecLen {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecLen sig)) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · exact fk_finish_top hsf h
  · exact (done_fail h rfl).elim

theorem fk_borrowElement {mutable : Bool} {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (borrowElement s fr rest info mutable sig) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  unfold borrowElement at h
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨i, fr₁⟩, hpi, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨x, st₁, -, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · exact fk_finish_top hsf h
  · exact (done_fail h rfl).elim

/-- A write in place keeps the keys. -/
theorem fk_write {st : List Value} {m : Location} {v : Value} {vals : List Value} {s₃ : State}
    (hsf : s.frames = fr :: rest)
    (hu : update { s with frames := { fr with stack := st } :: rest } m v = .ok s₃)
    (h : stepOf (finish s₃ vals) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key :=
  fk_finish hsf h (update_keys hu)

theorem fk_writeRef (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .writeRef) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st₁, -, rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨v, fr₂⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpv
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨old, -, h⟩ := done_bind h rfl deref_ends
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
  exact fk_write hsf hu h

theorem fk_vecPushBack {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecPushBack sig)) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st₁, -, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
  exact fk_write hsf hu h

theorem fk_vecPopBack {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecPopBack sig)) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
    exact fk_write hsf hu h
  · exact (done_fail h rfl).elim

theorem fk_vecSwap {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecSwap sig)) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨j, fr₁⟩, hpj, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨y, st₁, -, rfl, rfl⟩ := popIndex_ok hpj
  obtain ⟨⟨i, fr₂⟩, hpi, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨x, st₂, -, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₃⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
    exact fk_write hsf hu h
  · exact (done_fail h rfl).elim
  · exact (done_fail h rfl).elim

theorem fk_moveFrom {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.moveResourceFrom P s fr rest d tys) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h rfl popAddress_ends
  obtain ⟨st, -, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
  split at h
  · exact fk_finish hsf h rfl
  · exact (done_fail h rfl).elim

theorem fk_moveTo {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.moveResourceTo P s fr rest d tys) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st₁, -, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨sv, -, h⟩ := done_bind h rfl deref_ends
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
    split at h
    · exact (done_fail h rfl).elim
    · exact fk_finish hsf h rfl
  · exact (done_wrong h rfl).elim

/-- **A step of the current frame keeps every frame's key**: any
instruction but `Call` and `Ret`. -/
theorem exec_keys {instr : Bytecode} (hsf : s.frames = fr :: rest)
    (hi : code.code[fr.pc]? = some instr) (hcall : ∀ hd, instr ≠ .call hd)
    (hcallg : ∀ i, instr ≠ .callGeneric i) (hret : instr ≠ .ret)
    (hcallc : ∀ sig, instr ≠ .callClosure sig)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done (.running s')) :
    s'.frames.map Frame.key = s.frames.map Frame.key := by
  cases hval : instr.ValueOnly
  · cases instr
    all_goals first
      | (simp [Bytecode.ValueOnly] at hval; done)
      | skip
    case pop => exact fk_pop hsf h
    case ret => exact absurd rfl hret
    case copyLoc => exact fk_copyLoc hsf h
    case moveLoc => exact fk_moveLoc hsf h
    case stLoc => exact fk_stLoc hsf h
    case call hd => exact absurd rfl (hcall hd)
    case eq => exact fk_equality (positive := true) hsf h
    case neq => exact fk_equality (positive := false) hsf h
    case testVariant => exact fk_testVariant hsf h
    case readRef => exact fk_readRef hsf h
    case writeRef => exact fk_writeRef hsf h
    case freezeRef => exact fk_freezeRef hsf h
    case mutBorrowLoc => exact fk_borrowLoc hsf h
    case immBorrowLoc => exact fk_borrowLoc hsf h
    case mutBorrowField => exact fk_borrowField hsf h
    case immBorrowField => exact fk_borrowField hsf h
    case mutBorrowVariantField => exact fk_borrowVariantField hsf h
    case immBorrowVariantField => exact fk_borrowVariantField hsf h
    case vecLen => exact fk_vecLen hsf h
    case vecImmBorrow => exact fk_borrowElement hsf h
    case vecMutBorrow => exact fk_borrowElement hsf h
    case vecPushBack => exact fk_vecPushBack hsf h
    case vecPopBack => exact fk_vecPopBack hsf h
    case vecSwap => exact fk_vecSwap hsf h
    case moveFrom => exact fk_moveFrom hsf h
    case moveTo => exact fk_moveTo hsf h
    case mutBorrowGlobal => exact fk_borrowGlobal hsf h
    case immBorrowGlobal => exact fk_borrowGlobal hsf h
    case callGeneric i => exact absurd rfl (hcallg i)
    case callClosure sig => exact absurd rfl (hcallc sig)
    case packClosure => exact fk_packClosure hsf h
    case packClosureGeneric =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      exact fk_packClosure hsf h
    case testVariantGeneric | moveFromGeneric | moveToGeneric
      | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
      | immBorrowVariantFieldGeneric | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      first
        | exact fk_testVariant hsf h
        | exact fk_moveFrom hsf h
        | exact fk_moveTo hsf h
        | exact fk_borrowField hsf h
        | exact fk_borrowVariantField hsf h
        | exact fk_borrowGlobal hsf h
    case packGeneric | unpackGeneric | packVariantGeneric | unpackVariantGeneric
      | existsGeneric =>
      obtain ⟨⟨fr', new, j, hfr, hid, hfun, -, -, -, -, hca⟩, -⟩ := exec_valueGeneric hi rfl h
      rw [hfr, hsf]
      simp [Frame.key, hid, hfun, hca]
    all_goals exact (done_wrong h rfl).elim
  · obtain ⟨⟨fr', new, j, hfr, hid, hfun, -, -, -, -, hca⟩, -⟩ := exec_value hi hval h
    rw [hfr, hsf]
    simp [Frame.key, hid, hfun, hca]

end Keys

/-! ## The events of a step of the current frame -/

@[simp] theorem fresh_all_entered (c : Creator) (v : Value) (n : Nat) :
    (IdTree.fresh c v n).2.2.all (fun e => e.entered?.isNone) = true := by
  rw [(fresh_spec c v n).2.2.2]; simp [Event.entered?]

theorem no_entered {evs : List Event} (h : evs.all (fun e => e.entered?.isNone) = true)
    {e : Event} (he : e ∈ evs) {x : FrameId × List Nat} (hen : e.entered? = some x) : False := by
  have := List.all_eq_true.1 h e he
  rw [hen] at this
  cases this

/-- Proves that no event of an explicit list of a step's events enters a frame. -/
macro "entered_none" : tactic =>
  `(tactic| (simp only [List.all_append, List.all_cons, List.all_nil, fresh_all_entered,
      List.all_map, Function.comp_def, Bool.and_true, Bool.true_and]
             first | done | simp [Event.entered?]))

/-- **Only `Call` enters a frame.** -/
theorem exec_entered {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) (hcall : ∀ hd, instr ≠ .call hd)
    (hcallg : ∀ i, instr ≠ .callGeneric i) (hcallc : ∀ sig, instr ≠ .callClosure sig)
    {e : Event} (he : e ∈ evs) {x : FrameId × List Nat} (hen : e.entered? = some x) : False := by
  cases instr
  case call hd => exact hcall hd rfl
  case callGeneric i => exact hcallg i rfl
  case callClosure sig => exact hcallc sig rfl
  all_goals
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Prod.mk.injEq, guard, reduceCtorEq] at hex
  all_goals (try simp_all [Event.entered?])
  case ret =>
    split at hex <;>
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact no_entered (by entered_none) he hen
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldConst | ldTrue | ldFalse =>
    obtain ⟨a, -, -, rfl⟩ := hex
    exact no_entered (by simp) he hen
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 | not | negate | add | sub | mul | div | mod | bitOr | bitAnd
    | xor | or | and | lt | gt | le | ge | shl | shr =>
    obtain ⟨-, a, -, -, rfl⟩ := hex
    exact no_entered (by simp) he hen
  case pack | packVariant | vecPack | packGeneric | packVariantGeneric | packClosure
    | packClosureGeneric =>
    obtain ⟨a, -, -, -, rfl⟩ := hex
    exact no_entered (by entered_none) he hen
  case unpack | unpackVariant | vecUnpack | unpackGeneric | unpackVariantGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨v, -, -, rfl⟩ := hex
      exact no_entered (by entered_none) he hen
    · cases hex
  case eq | neq =>
    obtain ⟨a, b, ⟨-, a₁, -, -, rfl⟩, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact no_entered (by entered_none) he hen
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact no_entered (by simp) he hen
  case copyLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact no_entered (by simp only [List.all_cons]; split <;> entered_none) he hen
    · cases hex
  case moveLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact no_entered (by simp only [List.all_cons, List.all_append]; split <;> entered_none)
        he hen
    · cases hex
  case stLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact no_entered (by simp only [List.all_cons]; split <;> entered_none) he hen
    · cases hex
  case readRef =>
    obtain ⟨a, b, -, a₁, -, -, rfl⟩ := hex
    exact no_entered (by entered_none) he hen
  case writeRef =>
    obtain ⟨a, b, -, a₁, -, a₂, -, -, rfl⟩ := hex
    exact no_entered (by entered_none) he hen
  case freezeRef =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact no_entered (by entered_none) he hen
    · cases hm
  case testVariant | testVariantGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨x, hx, -, rfl⟩ := hm
      obtain ⟨-, -, v, -, rfl⟩ := hx
      exact no_entered (by entered_none) he hen
    · cases hm
  case mutBorrowField | immBorrowField | mutBorrowVariantField | immBorrowVariantField
    | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
    | immBorrowVariantFieldGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        exact no_entered (by entered_none) he hen
      · cases hm
    · cases hm
  case vecLen =>
    obtain ⟨a, b, -, a₁, b₁, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    exact no_entered (by entered_none) he hen
  case vecImmBorrow | vecMutBorrow =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact no_entered (by entered_none) he hen
    · cases hm
  case vecPushBack =>
    obtain ⟨a, -, a₁, b, -, a₂, -, -, rfl⟩ := hex
    exact no_entered (by entered_none) he hen
  case vecPopBack =>
    obtain ⟨a, b, -, a₁, -, a₂, -, a₃, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact no_entered (by entered_none) he hen
    · cases hm
  case vecSwap =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨x, -, y, -, -, rfl⟩ := hex
        exact no_entered (by entered_none) he hen
      · cases hex
    · cases hex
  case exists_ =>
    obtain ⟨a, -, a₁, b, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    exact no_entered (by entered_none) he hen
  case existsGeneric =>
    obtain ⟨_, _, _, _, _, _, _, _, a, b, ⟨_, a', _, _, rfl⟩, _, rfl⟩ := hex
    exact no_entered (by entered_none) he hen
  case moveFrom | moveFromGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff] at hex
      obtain ⟨v, -, hex⟩ := hex
      split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨k, -, t, -, -, rfl⟩ := hex
        exact no_entered (by entered_none) he hen
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
        exact no_entered (by entered_none) he hen
      · cases hm
    · cases hm
  case mutBorrowLoc | immBorrowLoc =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact no_entered (by entered_none) he hen
    · cases hex
  case mutBorrowGlobal | immBorrowGlobal | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        exact no_entered (by entered_none) he hen
      · cases hex
    · cases hex

/-! ## Values of slot types in the shadow -/

theorem paramTypes_slot {P : Program} {f : FunctionRef} {tys : List RuntimeType}
    (hf : ∀ x ∈ f.typeArgs, x.isValue = true)
    (h : P.paramTypes f = some tys) : ∀ t ∈ tys, SlotType t := by
  unfold Program.paramTypes at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, -, sig, -, hsig⟩ := h
  exact slotType_of_ofSignature hf hsig

theorem returnTypes_slot {P : Program} {f : FunctionRef} {tys : List RuntimeType}
    (hf : ∀ x ∈ f.typeArgs, x.isValue = true)
    (h : P.returnTypes f = some tys) : ∀ t ∈ tys, SlotType t := by
  unfold Program.returnTypes at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, -, sig, -, hsig⟩ := h
  exact slotType_of_ofSignature hf hsig

/-- A reference among the occurrences of a value of a slot type is the value
itself. -/
theorem occ_ref_slot {P : Program} {t : IdTree} {v : Value} {ty : RuntimeType}
    (hc : v.conforms P ty = true) (hty : SlotType ty) (hf : t.fits v = true) {d : Nat}
    {r : Reference} (hm : (d, Value.ref r) ∈ t.occurrences v) : v = .ref r ∧ d = t.id := by
  rcases hty with hty | ⟨b, u, rfl⟩
  · have := conforms_plain hc hty _ (occurrences_mem hf hm)
    simp [Value.isRef] at this
  · obtain ⟨r', rfl, -, -⟩ := conforms_ref_assignable hc
    rw [ref_occ hf] at hm
    simp only [List.mem_singleton, Prod.mk.injEq, Value.ref.injEq] at hm
    exact ⟨by rw [hm.2], hm.1⟩

/-- A reference among the occurrences of values of slot types is one of the
values, with the identity of its tree. -/
theorem occAll_ref_slot {P : Program} {ts : List IdTree} {vs : List Value}
    (hfa : IdTree.fitsAll ts vs = true)
    (hslot : ∀ v ∈ vs, ∃ ty, v.conforms P ty = true ∧ SlotType ty) {d : Nat} {r : Reference}
    (hm : (d, Value.ref r) ∈ IdTree.occurrencesAll ts vs) :
    ∃ t, (t, Value.ref r) ∈ ts.zip vs ∧ d = t.id := by
  rw [occurrencesAll_eq_flatMap, List.mem_flatMap] at hm
  obtain ⟨⟨t, v⟩, hz, ho⟩ := hm
  obtain ⟨ty, hc, hty⟩ := hslot v (List.of_mem_zip hz).2
  obtain ⟨rfl, rfl⟩ := occ_ref_slot hc hty (fitsAll_of_zip hfa hz) ho
  exact ⟨t, hz, rfl⟩

theorem conformsAll_slot {P : Program} {vs : List Value} {tys : List RuntimeType}
    (h : conformsAll P vs tys = true) (hs : ∀ t ∈ tys, SlotType t) :
    ∀ v ∈ vs, ∃ ty, v.conforms P ty = true ∧ SlotType ty := by
  intro v hv
  obtain ⟨t, ht, hc⟩ := conformsAll_mem h v hv
  exact ⟨t, hc, hs t ht⟩

/-! ## The references a call passes and a return hands back -/

theorem mem_zip_reverse {α β : Type} {as : List α} {bs : List β} (hlen : as.length = bs.length)
    {a : α} {b : β} (h : (a, b) ∈ as.zip bs) : (a, b) ∈ as.reverse.zip bs.reverse := by
  refine zip_rev (as := as.reverse) (bs := bs.reverse) (by simp [hlen]) ?_
  simpa using h

/-! ## Core steps -/

/-- An annotated core step runs an instruction of the current frame, and the
ghost layer annotates that instruction. -/
theorem core_unfold {P : Program} {s : State} {o : Outcome} {h : Shadow} {post : ShadowPost}
    {evs : List Event} (hstep : step P s = .done o)
    (hc : Shadow.coreStep P s o h = some (post, evs)) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ P.function? fr.function = some info ∧
      info.definition.code = some code ∧ code.code[fr.pc]? = some instr ∧
      stepOf (Step.exec P s fr rest info code instr) = .done o ∧
      Shadow.exec P info.moduleId instr s fr o h = some (post, evs) := by
  obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
  refine ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, ?_, hex⟩
  unfold step at hstep
  rw [hsf] at hstep
  simp only [hinfo, hcode, hinstr] at hstep
  exact hstep

section Concrete

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

/-- **What `Call` of a function with code does** to the frames. -/
theorem call_concrete {hd : FunctionHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.call P s fr rest info hd tys) = .done (.running s')) :
    ∃ callee n m paramTys, callee.function.typeArgs = tys ∧ n ≤ fr.stack.length ∧
      P.paramTypes callee.function = some paramTys ∧
      conformsAll P (fr.stack.take n).reverse paramTys = true ∧
      callee.locals = ((fr.stack.take n).reverse.map some ++ List.replicate m none).toArray ∧
      callee.stack = [] ∧ callee.id = s.nextFrame ∧
      s'.frames = callee :: { fr with stack := fr.stack.drop n } :: rest := by
  unfold Step.call at h
  obtain ⟨callee, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨calleeInfo, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  obtain ⟨paramTys, hpt, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hps, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hrs, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨args, fr₁⟩, hpop, h⟩ := done_bind h rfl popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  next hconf =>
  split at h
  · exact (done_native h rfl).elim
  obtain ⟨localTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  have hpt' := need_ok' hpt
  have hconf' : conformsAll P (fr.stack.take paramTys.length).reverse paramTys = true := by
    simpa using hconf
  split at h <;> split at h
  all_goals first
    | exact (done_fail h rfl).elim
    | skip
  all_goals
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    subst h
    refine ⟨_, paramTys.length, _, paramTys, rfl, hn, hpt', hconf', rfl, rfl, ?_, rfl⟩
    (try split) <;> rfl

/-- **What `Ret` to a caller does** to the frames. -/
theorem ret_concrete
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done (.running s')) :
    ∃ caller rest' retTys, rest = caller :: rest' ∧ P.returnTypes fr.function = some retTys ∧
      conformsAll P fr.stack.reverse retTys = true ∧
      s'.frames = { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 } :: rest' := by
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, hrt, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  next hconf =>
  split at h
  · exact (done_wrong h rfl).elim
  cases rest with
  | nil => simp [stepOf, pure, Except.pure] at h
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    refine ⟨caller, rest', retTys, rfl, need_ok' hrt, by simpa using hconf, ?_⟩
    rw [← h]

/-- **What a `Ret` that ends the execution does.** -/
theorem ret_concrete_returned {vs : List Value} {st : GlobalStorage}
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done (.returned vs st)) :
    rest = [] ∧ ∃ retTys, P.returnTypes fr.function = some retTys ∧
      conformsAll P fr.stack.reverse retTys = true := by
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, hrt, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  next hconf =>
  split at h
  · exact (done_wrong h rfl).elim
  cases rest with
  | nil => exact ⟨rfl, retTys, need_ok' hrt, by simpa using hconf⟩
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    cases h

end Concrete

/-! ## The shadow of a call and of a return -/

section Shadow

variable {P : Program} {m : ModuleId} {s s' : State} {fr : Frame} {rest : List Frame}
  {g : ShadowFrame} {hs : List ShadowFrame} {hst hhe : List IdTree} {f : Nat}
  {post : ShadowPost} {evs : List Event}

/-- **The annotation of `Call`**: the callee's locals take the arguments'
identities, and the step enters the callee with the identities of the
reference arguments. -/
theorem call_shadow {hd : FunctionHandleIndex} {callee : Frame} {n : Nat}
    (hn : n ≤ fr.stack.length) (hlen : g.stack.length = fr.stack.length)
    (h2f : s'.frames = callee :: { fr with stack := fr.stack.drop n } :: rest)
    (hex : Shadow.exec P m (.call hd) s fr (.running s') ⟨g :: hs, hst, hhe, f⟩ =
      some (post, evs)) :
    post = .running ⟨{ locals := ((g.stack.take n).reverse.map some ++
          List.replicate (callee.locals.size - n) none).toArray, stack := [] } ::
        { g with stack := g.stack.drop n } :: hs, hst, hhe, f⟩ ∧
    evs = (slotRefs (g.stack.take n).reverse (fr.stack.take n).reverse).map
        (fun (i, r) => Event.used i r) ++
      [.entered callee.id
        ((slotRefs (g.stack.take n).reverse (fr.stack.take n).reverse).map (·.1))] := by
  have hk : fr.stack.length - (fr.stack.drop n).length = n := by simp; omega
  have e : Shadow.exec P m (.call hd) s fr (.running s') ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨{ locals := ((g.stack.take n).reverse.map some ++
          List.replicate (callee.locals.size - n) none).toArray, stack := [] } ::
        { g with stack := g.stack.drop n } :: hs, hst, hhe, f⟩,
      (slotRefs (g.stack.take n).reverse (fr.stack.take n).reverse).map
          (fun (i, r) => Event.used i r) ++
        [.entered callee.id
          ((slotRefs (g.stack.take n).reverse (fr.stack.take n).reverse).map (·.1))]) := by
    simp only [Shadow.exec, h2f, hk, guard, hlen, if_pos hn]; rfl
  rw [e, Option.some.injEq, Prod.mk.injEq] at hex
  exact ⟨hex.1.symm, hex.2.symm⟩

/-- The events of `Ret`: the returned references are used, and the frame ends
returning them. -/
theorem ret_events {o : Outcome}
    (hex : Shadow.exec P m .ret s fr o ⟨g :: hs, hst, hhe, f⟩ = some (post, evs)) :
    evs = (slotRefs g.stack.reverse fr.stack.reverse).map (fun (i, r) => Event.used i r) ++
      [.frameEnded fr.id ((slotRefs g.stack.reverse fr.stack.reverse).map (·.1))] := by
  cases hs with
  | nil =>
    have e : Shadow.exec P m .ret s fr o ⟨[g], hst, hhe, f⟩ = some (.returned g.stack.reverse hst,
        (slotRefs g.stack.reverse fr.stack.reverse).map (fun (i, r) => Event.used i r) ++
          [.frameEnded fr.id ((slotRefs g.stack.reverse fr.stack.reverse).map (·.1))]) := rfl
    rw [e, Option.some.injEq, Prod.mk.injEq] at hex
    exact hex.2.symm
  | cons cg hs' =>
    have e : Shadow.exec P m .ret s fr o ⟨g :: cg :: hs', hst, hhe, f⟩ =
        some (.running ⟨{ cg with stack := g.stack ++ cg.stack } :: hs', hst, hhe, f⟩,
        (slotRefs g.stack.reverse fr.stack.reverse).map (fun (i, r) => Event.used i r) ++
          [.frameEnded fr.id ((slotRefs g.stack.reverse fr.stack.reverse).map (·.1))]) := rfl
    rw [e, Option.some.injEq, Prod.mk.injEq] at hex
    exact hex.2.symm

/-- The annotation of `Ret` to a caller: the caller's segment gets the
returned identities. -/
theorem ret_shadow {o : Outcome} {cg : ShadowFrame}
    (hex : Shadow.exec P m .ret s fr o ⟨g :: cg :: hs, hst, hhe, f⟩ = some (post, evs)) :
    post = .running ⟨{ cg with stack := g.stack ++ cg.stack } :: hs, hst, hhe, f⟩ := by
  have e : Shadow.exec P m .ret s fr o ⟨g :: cg :: hs, hst, hhe, f⟩ =
      some (.running ⟨{ cg with stack := g.stack ++ cg.stack } :: hs, hst, hhe, f⟩,
      (slotRefs g.stack.reverse fr.stack.reverse).map (fun (i, r) => Event.used i r) ++
        [.frameEnded fr.id ((slotRefs g.stack.reverse fr.stack.reverse).map (·.1))]) := rfl
  rw [e, Option.some.injEq, Prod.mk.injEq] at hex
  exact hex.1.symm

end Shadow

/-! ## What a step records of frames -/

/-- What a `Ret` of the current frame `fr` (shadow `g`) records. -/
structure RetFacts (P : Program) (s : State) (fr : Frame) (g : ShadowFrame) (evs : List Event)
    (rets : List Nat) : Prop where
  instr : ∃ info code, P.function? fr.function = some info ∧ info.definition.code = some code ∧
    code.code[fr.pc]? = some .ret
  head : s.frames.head? = some fr
  ended : Event.frameEnded fr.id rets ∈ evs
  only : ∀ e ∈ evs, e.entered? = none ∧ ∀ f rs, e = .frameEnded f rs → f = fr.id ∧ rs = rets
  rets : ∀ d ∈ rets, ∃ r, Value.ref r ∈ fr.stack ∧ (d, Value.ref r) ∈ g.occurrences fr

theorem retFacts_of {P : Program} {s : State} {fr : Frame} {rest : List Frame} {g : ShadowFrame}
    {info : FunctionInfo} {code : CodeUnit} {evs : List Event} (hsf : s.frames = fr :: rest)
    (hg : g.fits fr = true) (hinfo : P.function? fr.function = some info)
    (hcode : info.definition.code = some code) (hinstr : code.code[fr.pc]? = some .ret)
    (hevs : evs = (slotRefs g.stack.reverse fr.stack.reverse).map (fun (i, r) => Event.used i r) ++
      [.frameEnded fr.id ((slotRefs g.stack.reverse fr.stack.reverse).map (·.1))]) :
    RetFacts P s fr g evs ((slotRefs g.stack.reverse fr.stack.reverse).map (·.1)) := by
  have hlen := fitsAll_length (frame_stack_fits hg)
  subst hevs
  refine ⟨⟨info, code, hinfo, hcode, hinstr⟩, by simp [hsf], by simp, fun e he => ?_,
    fun d hd => ?_⟩
  · simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
    rcases he with ⟨⟨i, r⟩, -, rfl⟩ | rfl
    · exact ⟨rfl, fun _ _ h => by cases h⟩
    · refine ⟨rfl, fun f rs h => ?_⟩
      simp only [Event.frameEnded.injEq] at h
      exact ⟨h.1.symm, h.2.symm⟩
  · obtain ⟨⟨d', r⟩, hm, rfl⟩ := List.mem_map.1 hd
    obtain ⟨t, hz, rfl⟩ := mem_slotRefs.1 hm
    have hz' := zip_rev hlen hz
    exact ⟨r, (List.of_mem_zip hz').2, refSlot_occ hg ⟨t, rfl, Or.inl hz'⟩⟩

theorem fits_frames_length {h : Shadow} {s : State} (hf : h.fits s = true) :
    h.frames.length = s.frames.length := by
  rw [shadow_fits] at hf
  simp only [framesFit, Bool.and_eq_true, beq_iff_eq] at hf
  exact hf.1.1

/-- **Only `Ret` ends a frame.** -/
theorem exec_returned {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) (hret : instr ≠ .ret)
    {e : Event} (he : e ∈ evs) {x : FrameId} (hr : e.returned? = some x) : False := by
  cases e <;> simp [Event.returned?] at hr
  rename_i f rs
  have hk := exec_ends hh hex he (ended := fun
    | .local_ g' _ => g' == f
    | .global _ _ => false
    | .held _ => false) rfl
  cases hk with
  | frameEnded hi _ => exact hret hi

/-- A native call records no frame entry or end. -/
theorem nativeEvents_quiet {ν : NativeFlows} {f : FunctionRef} {args vals : List Value}
    {g : ShadowFrame} {m : ModuleId} {fresh : Nat} {e : Event}
    (he : e ∈ nativeEvents ν f args vals g m fresh) : e.entered? = none ∧ e.returned? = none := by
  simp only [nativeEvents, List.mem_append] at he
  rcases he with he | he
  · obtain ⟨_, _, _, _, rfl, -⟩ := consumedFrom_mem _ he
    exact ⟨rfl, rfl⟩
  · obtain ⟨_, _, _, rfl⟩ := (nativeResults_spec ν f args vals _ m fresh).2.2 _ he
    exact ⟨rfl, rfl⟩

/-- The occurrences of a callee's first frame are those of its arguments. -/
theorem callee_localsOcc {argsT : List IdTree} {argsV : List Value} {callee : Frame} {m : Nat}
    (hrl : argsT.length = argsV.length)
    (hloc : callee.locals = (argsV.map some ++ List.replicate m none).toArray) :
    localsOcc { locals := (argsT.map some ++ List.replicate (callee.locals.size - argsT.length)
      none).toArray, stack := [] } callee = IdTree.occurrencesAll argsT argsV := by
  have hsize : callee.locals.size - argsT.length = m := by rw [hloc, hrl]; simp
  rw [hsize]
  simp only [localsOcc, hloc, List.toList_toArray]
  exact args_occurrences hrl m

/-- **What a traced step that keeps running does to the frames**: a step of
the current frame keeps every frame's key and records no frame entry or end;
`Call` and `CallClosure` enter a frame whose references are its parameters,
passed by the caller (a closure's captured values hold none); `Ret` hands the
caller the returned references. -/
theorem traced_shape {P : Program} {N : NativeRel} {ν : NativeFlows} {s s' : State} {h h' : Shadow}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hpre : Pre false P s h)
    (hargs : ∀ fr ∈ s.frames, ∀ x ∈ fr.function.typeArgs, x.isValue = true)
    (hvals : ∀ fr ∈ s.frames, ∀ v ∈ frameVals fr, shaped P v = true)
    (hT : TracedStep P N ν s h o post evs) (ho : o = .running s') (hp : post = .running h') :
    (s'.frames.map Frame.key = s.frames.map Frame.key ∧ h'.frames.length = h.frames.length ∧
      ∀ e ∈ evs, e.entered? = none ∧ e.returned? = none) ∨
    (∃ fr rest g hs callee gc k pids, s.frames = fr :: rest ∧ h.frames = g :: hs ∧
      s'.frames = callee :: { fr with stack := fr.stack.drop k } :: rest ∧
      h'.frames = gc :: { g with stack := g.stack.drop k } :: hs ∧ callee.id = s.nextFrame ∧
      Event.entered callee.id pids ∈ evs ∧
      (∀ e ∈ evs, e.returned? = none ∧ ∀ f ps, e.entered? = some (f, ps) → f = callee.id ∧ ps = pids) ∧
      (∀ d r, (d, Value.ref r) ∈ gc.occurrences callee → d ∈ pids) ∧
      (∀ p ∈ pids, ∃ r, (p, Value.ref r) ∈ g.occurrences fr) ∧
      (∀ d r, (d, Value.ref r) ∈ ({ g with stack := g.stack.drop k } : ShadowFrame).occurrences
        { fr with stack := fr.stack.drop k } → (d, Value.ref r) ∈ g.occurrences fr)) ∨
    (∃ fr caller rest g gcaller hs caller' gc' rets, s.frames = fr :: caller :: rest ∧
      h.frames = g :: gcaller :: hs ∧ s'.frames = caller' :: rest ∧ h'.frames = gc' :: hs ∧
      caller'.key = caller.key ∧ RetFacts P s fr g evs rets ∧
      (∀ d r, (d, Value.ref r) ∈ gc'.occurrences caller' →
        (d, Value.ref r) ∈ gcaller.occurrences caller ∨ d ∈ rets)) := by
  have hfit := traced_fits hT
  have hlen := fits_frames_length hpre.fits
  cases hT with
  | native hstep hN hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      subst hp
      rw [ho] at hfit hn
      have hlen' := fits_frames_length (show h'.fits s' = true from hfit)
      left
      obtain ⟨fr, rest, n, g, hs, F, results, resultTrees, ts, hsf, -, -, -, -, hwb, -⟩ :=
        nativeStep_decomp hshape hstep ho hn
      have hkeys : s'.frames.map Frame.key = s.frames.map Frame.key := by
        rw [writeBack_keys hwb, hsf]; rfl
      refine ⟨hkeys, ?_, fun e he => ⟨(nativeStep_quiet hn he).1, (nativeStep_quiet hn he).2.1⟩⟩
      rw [hlen', hlen, ← List.length_map (f := Frame.key), hkeys, List.length_map]
    subst hp
    rw [ho] at hfit
    have hlen' := fits_frames_length (show h'.fits s' = true from hfit)
    left
    have hcont := (congrArg Outcome.continues ho).trans (rfl : (Outcome.running s').continues = true)
    obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    obtain ⟨fr, rest, n, hsf, -, -, rfl, hfacts⟩ := step_callNative hstep
    obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
    have hv := hfacts.values hshape
    have hs' := (resume_running (fr := { fr with stack := fr.stack.drop n }) (rest := rest) rfl
      hshape (noRef_of_conformsAll hconfa (hv.1 _ hpt)) hv.2 hcont).1
    rw [ho, Outcome.running.injEq] at hs'
    have hkeys : s'.frames.map Frame.key = s.frames.map Frame.key := by
      rw [hs', hsf]; rfl
    refine ⟨hkeys, ?_, fun e he => ?_⟩
    · rw [hlen', hlen, ← List.length_map (f := Frame.key), hkeys, List.length_map]
    · rw [hevs] at he
      exact nativeEvents_quiet he
  | core hstep hc =>
    subst ho hp
    have hlen' := fits_frames_length (show h'.fits s' = true from hfit)
    obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hstep', hex⟩ :=
      core_unfold hstep hc
    obtain ⟨hframes, hst, hhe, f⟩ := h
    obtain ⟨g, hs, rfl⟩ : ∃ g hs, hframes = g :: hs := by
      cases hframes with
      | nil => simp [hsf] at hlen
      | cons g hs => exact ⟨g, hs, rfl⟩
    have hg := (pre_head hpre hsf).1
    have hgl := fitsAll_length (frame_stack_fits hg)
    have hfa := hargs fr (by rw [hsf]; exact List.mem_cons_self ..)
    by_cases hcall : (∃ hd, instr = .call hd) ∨ ∃ i, instr = .callGeneric i
    · -- Either call: the callee's type arguments are value types.
      obtain ⟨tys, hd, htys, hstep'', hex'⟩ : ∃ tys hd, (∀ x ∈ tys, x.isValue = true) ∧
          stepOf (Step.call P s fr rest info hd tys) = .done (.running s') ∧
          Shadow.exec P info.moduleId (.call hd) s fr (.running s') ⟨g :: hs, hst, hhe, f⟩ =
            some (.running h', evs) := by
        rcases hcall with ⟨hd, rfl⟩ | ⟨i, rfl⟩
        · exact ⟨[], hd, by simp, hstep', hex⟩
        · obtain ⟨x, -, h1⟩ := done_bind hstep' rfl (ends_need _ _)
          obtain ⟨tys, htys, h1⟩ := done_bind h1 rfl genericArgs_ends
          exact ⟨tys, x.handle, typeArgsWith_values hfa (genericArgs_ok htys), h1, hex⟩
      right; left
      obtain ⟨callee, n, m, paramTys, hcta, hn, hpt, hconf, hloc, hstk, hid, h2f⟩ :=
        call_concrete hstep''
      obtain ⟨hpost, hevs⟩ := call_shadow hn hgl h2f hex'
      rw [ShadowPost.running.injEq] at hpost
      subst hpost hevs
      have hsize : callee.locals.size - n = m := by rw [hloc]; simp; omega
      have hrl : ((g.stack.take n).reverse).length = ((fr.stack.take n).reverse).length := by
        simp [hgl]
      obtain ⟨htake, hdropf⟩ := fitsAll_take_drop n (frame_stack_fits hg)
      have hslot := conformsAll_slot hconf (paramTypes_slot (by rw [hcta]; exact htys) hpt)
      refine ⟨fr, rest, g, hs, callee, _, n,
        (slotRefs (g.stack.take n).reverse (fr.stack.take n).reverse).map (·.1), hsf, rfl, h2f, rfl,
        hid, by simp, ?_, ?_, ?_, ?_⟩
      · intro e he
        simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
        rcases he with ⟨⟨i, r⟩, -, rfl⟩ | rfl
        · exact ⟨rfl, fun _ _ h => by cases h⟩
        · refine ⟨rfl, fun f ps h => ?_⟩
          simp only [Event.entered?, Option.some.injEq, Prod.mk.injEq] at h
          exact ⟨h.1.symm, h.2.symm⟩
      · intro d r hm
        rw [frame_occurrences, hstk] at hm
        simp only [IdTree.occurrencesAll, List.append_nil] at hm
        have hlo : localsOcc { locals := ((g.stack.take n).reverse.map some ++
            List.replicate (callee.locals.size - n) none).toArray, stack := [] } callee =
            IdTree.occurrencesAll (g.stack.take n).reverse (fr.stack.take n).reverse := by
          rw [hsize]
          simp only [localsOcc, hloc, List.toList_toArray]
          exact args_occurrences hrl m
        rw [hlo] at hm
        obtain ⟨t, hz, rfl⟩ := occAll_ref_slot (fitsAll_reverse htake) hslot hm
        exact List.mem_map.2 ⟨(t.id, r), mem_slotRefs.2 ⟨t, hz, rfl⟩, rfl⟩
      · intro p hp
        obtain ⟨⟨p', r⟩, hm, rfl⟩ := List.mem_map.1 hp
        obtain ⟨t, hz, rfl⟩ := mem_slotRefs.1 hm
        exact ⟨r, refSlot_occ hg ⟨t, rfl, Or.inl (zip_rev_take hgl hz)⟩⟩
      · intro d r hm
        rw [frame_occurrences] at hm ⊢
        rw [stack_split (frame_stack_fits hg) n]
        simp only [List.mem_append] at hm ⊢
        rcases hm with hm | hm
        · exact Or.inl hm
        · exact Or.inr (Or.inr hm)
    have hcall' : ∀ hd, instr ≠ .call hd := fun hd e => hcall (Or.inl ⟨hd, e⟩)
    have hcallg : ∀ i, instr ≠ .callGeneric i := fun i e => hcall (Or.inr ⟨i, e⟩)
    by_cases hclo : ∃ sig, instr = .callClosure sig
    · -- A closure call: the closure and the provided arguments are popped.
      obtain ⟨sig, rfl⟩ := hclo
      right; left
      obtain ⟨tf, mask, cs, cps, crs, ca, vst, callee, m, paramTys, ps0, rs0, a, -, hstk, hn, hps0,
        -, hprov, -, -, -, -, -, hloc, hcst, hid, h2f⟩ := callClosure_concrete (P := P) hstep'
      have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
      have hcmem : Value.closure tf mask cs cps crs ca ∈ fr.stack := by
        rw [hstk]; exact List.mem_cons_self ..
      have hcsh := hvals fr hfr _ (mem_frameVals_stack hcmem)
      have hcomp : Composes P (.closure tf mask cs cps crs ca) := hpre.composes fr hfr _ hcmem
      simp only [Composes] at hcomp
      simp only [shaped] at hcsh
      split at hcsh
      rotate_left
      · simp at hcsh
      next ctys rem rts hct =>
      simp only [Bool.and_eq_true] at hcsh
      have hcsf := hcsh.2
      have hcval := closureType?_captured_value hct
      -- The closure's tree on top of the shadow stack.
      have hfs := frame_stack_fits hg
      obtain ⟨ct, gst, hgst⟩ : ∃ ct gst, g.stack = ct :: gst := by
        cases hgs : g.stack with
        | nil => rw [hgs, hstk] at hfs; simp [IdTree.fitsAll] at hfs
        | cons ct gst => exact ⟨ct, gst, rfl⟩
      rw [hstk, hgst] at hfs
      simp only [IdTree.fitsAll, Bool.and_eq_true] at hfs
      obtain ⟨hctf, hgsf⟩ := hfs
      obtain ⟨i, cts⟩ := ct
      have hcts : IdTree.fitsAll cts cs = true := by
        simpa [IdTree.fits, Value.children] using hctf
      have hgl' := fitsAll_length hgsf
      obtain ⟨hpost, hevs⟩ := callClosure_shadow (sig := sig) (callee := callee) hstk hgst
        (by rw [hgl']; exact hn) hn h2f hex
      rw [ShadowPost.running.injEq] at hpost
      subst hpost hevs
      obtain ⟨htake, hdropf⟩ := fitsAll_take_drop cps.length hgsf
      have hpl : ((vst.take cps.length).reverse).length = cps.length := by
        rw [List.length_reverse, List.length_take, Nat.min_eq_left hn]
      have hfargs := maskCompose_fits (mask := mask) hcts (fitsAll_reverse htake)
      have hrl := fitsAll_length hfargs
      have hocc := maskCompose_occ hcts (fitsAll_reverse htake) (by rw [hpl]; exact hcomp)
      -- The provided arguments' references: of slot types.
      have hslot : ∀ v ∈ (vst.take cps.length).reverse, ∃ ty, v.conforms P ty = true ∧ SlotType ty :=
        conformsAll_slot hprov (slotType_of_ofSignature hfa (by
          rw [← ofSlotTokensWith_eq]; exact hps0))
      -- The captured values hold no reference.
      have hcapNR : ∀ v ∈ cs, ∀ w ∈ subvalues v, w.isRef = false := by
        intro v hv
        obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hv
        obtain ⟨t, ht, hta, hsv⟩ := shapedFields_get hcsf hj
        exact shaped_noRef v hsv (by
          rw [assignable_isValue hta]; exact hcval t (List.mem_of_getElem? ht))
      refine ⟨fr, rest, g, hs, callee, _, cps.length + 1,
        (slotRefs (gst.take cps.length).reverse (vst.take cps.length).reverse).map (·.1), hsf, rfl,
        by rw [h2f, hstk]; rfl, by rw [hgst]; rfl, hid, by simp, ?_, ?_, ?_, ?_⟩
      · intro e he
        simp only [List.mem_append, List.mem_map, List.mem_cons, List.not_mem_nil, or_false] at he
        rcases he with ⟨⟨i', r⟩, -, rfl⟩ | rfl | rfl
        · exact ⟨rfl, fun _ _ h => by cases h⟩
        · refine ⟨rfl, fun f ps h => ?_⟩
          simp only [Event.entered?, Option.some.injEq, Prod.mk.injEq] at h
          exact ⟨h.1.symm, h.2.symm⟩
        · exact ⟨rfl, fun _ _ h => by cases h⟩
      · intro d r hm
        rw [frame_occurrences, hcst] at hm
        simp only [IdTree.occurrencesAll, List.append_nil] at hm
        rw [callee_localsOcc hrl hloc] at hm
        rcases List.mem_append.1 (hocc.mem_iff.1 hm) with hm | hm
        · exfalso
          rw [occurrencesAll_eq_flatMap, List.mem_flatMap] at hm
          obtain ⟨⟨t, v⟩, hz, ho⟩ := hm
          have := hcapNR v (List.of_mem_zip hz).2 _
            (occurrences_mem (fitsAll_of_zip hcts hz) ho)
          simp [Value.isRef] at this
        · obtain ⟨t, hz, rfl⟩ := occAll_ref_slot (fitsAll_reverse htake) hslot hm
          exact List.mem_map.2 ⟨(t.id, r), mem_slotRefs.2 ⟨t, hz, rfl⟩, rfl⟩
      · intro p hp
        obtain ⟨⟨p', r⟩, hm, rfl⟩ := List.mem_map.1 hp
        obtain ⟨t, hz, rfl⟩ := mem_slotRefs.1 hm
        have hz' : (t, Value.ref r) ∈ g.stack.zip fr.stack := by
          rw [hgst, hstk, List.zip_cons_cons]
          exact List.mem_cons_of_mem _ (zip_rev_take hgl' hz)
        exact ⟨r, refSlot_occ hg ⟨t, rfl, Or.inl hz'⟩⟩
      · intro d r hm
        rw [frame_occurrences] at hm ⊢
        rw [stack_split (frame_stack_fits hg) (cps.length + 1)]
        simp only [List.mem_append] at hm ⊢
        rcases hm with hm | hm
        · exact Or.inl hm
        · exact Or.inr (Or.inr hm)
    have hcallc : ∀ sig, instr ≠ .callClosure sig := fun sig e => hclo ⟨sig, e⟩
    by_cases hret : instr = .ret
    · subst hret
      right; right
      obtain ⟨caller, rest', retTys, rfl, hrt, hconf, h2f⟩ := ret_concrete hstep'
      obtain ⟨cg, hs', rfl⟩ : ∃ cg hs', hs = cg :: hs' := by
        cases hs with
        | nil => simp [hsf] at hlen
        | cons cg hs' => exact ⟨cg, hs', rfl⟩
      have hpost := ret_shadow hex
      rw [ShadowPost.running.injEq] at hpost
      subst hpost
      have hcg := (pre_head hpre hsf).2
      rw [framesFit_cons, Bool.and_eq_true] at hcg
      have hslot := conformsAll_slot hconf (returnTypes_slot hfa hrt)
      refine ⟨fr, caller, rest', g, cg, hs', _, _, _, hsf, rfl, h2f, rfl, rfl,
        retFacts_of hsf hg hinfo hcode hinstr (ret_events hex), fun d r hm => ?_⟩
      rw [frame_occurrences, localsOcc_stack,
        occurrencesAll_append hgl] at hm
      rw [frame_occurrences]
      simp only [List.mem_append] at hm ⊢
      rcases hm with hm | hm | hm
      · exact Or.inl (Or.inl hm)
      · right
        have hfr := fitsAll_reverse (frame_stack_fits hg)
        have hslot' : ∀ v ∈ fr.stack, ∃ ty, v.conforms P ty = true ∧ SlotType ty :=
          fun v hv => hslot v (List.mem_reverse.2 hv)
        obtain ⟨t, hz, rfl⟩ := occAll_ref_slot (frame_stack_fits hg) hslot' hm
        exact List.mem_map.2 ⟨(t.id, r), mem_slotRefs.2 ⟨t, mem_zip_reverse hgl hz, rfl⟩, rfl⟩
      · exact Or.inl (Or.inr hm)
    · left
      refine ⟨exec_keys hsf hinstr hcall' hcallg hret hcallc hstep', ?_, fun e he => ⟨?_, ?_⟩⟩
      · rw [hlen', ← List.length_map (f := Frame.key), exec_keys hsf hinstr hcall' hcallg hret hcallc hstep',
          List.length_map, ← hlen]
      · cases hx : e.entered? with
        | none => rfl
        | some x => exact (exec_entered rfl hex hcall' hcallg hcallc he hx).elim
      · cases hx : e.returned? with
        | none => rfl
        | some x => exact (exec_returned rfl hex hret he hx).elim

/-- **A `Ret` that ends the execution** ends the only frame. -/
theorem traced_returned_shape {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State}
    {h : Shadow} {o : Outcome} {post : ShadowPost} {evs : List Event} (hpre : Pre false P s h)
    (hT : TracedStep P N ν s h o post evs) {vs : List Value} {st : GlobalStorage}
    (ho : o = .returned vs st) :
    ∃ fr g hs rets, s.frames = [fr] ∧ h.frames = g :: hs ∧ RetFacts P s fr g evs rets := by
  have hfit := traced_fits hT
  obtain ⟨hstep, hc⟩ := traced_returned hT ho
  subst ho
  have hlen := fits_frames_length hpre.fits
  obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hstep', hex⟩ :=
    core_unfold hstep hc
  obtain ⟨hframes, hst, hhe, f⟩ := h
  obtain ⟨g, hs, rfl⟩ : ∃ g hs, hframes = g :: hs := by
    cases hframes with
    | nil => simp [hsf] at hlen
    | cons g hs => exact ⟨g, hs, rfl⟩
  have hg := (pre_head hpre hsf).1
  obtain ⟨ts, sts, rfl⟩ : ∃ ts sts, post = .returned ts sts := by
    cases post with
    | running _ => simp [ShadowPost.fits] at hfit
    | returned ts sts => exact ⟨ts, sts, rfl⟩
  obtain rfl := exec_post_returned rfl hex
  obtain ⟨rfl, -⟩ := ret_concrete_returned hstep'
  exact ⟨fr, g, hs, _, hsf, rfl, retFacts_of hsf hg hinfo hcode hinstr (ret_events hex)⟩

end MoveBytecode.Proofs
