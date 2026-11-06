-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StackUsage
import Proofs.Values

/-!
# Constants decode

UNTRUSTED (checked by the kernel). A constant that the model's constants pass
accepts decodes, under the specification's BCS decoding, to a value of its
type, vectors included. So `LdConst` never goes wrong with `badConstant`, and
the value it pushes conforms to the constant's type (L8).

The model skips over a value of the constant's type; the specification
decodes it. Both read a vector's length as a canonical ULEB128 integer of at
most `2^31 - 1` (`uleb128_groups`), then the elements in turn.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

theorem natOfBytesLE_lt (l : List UInt8) : natOfBytesLE l < 256 ^ l.length := by
  induction l with
  | nil => simp [natOfBytesLE]
  | cons b l ih =>
    have hb : b.toNat < 256 := by simpa using b.toNat_lt
    simp only [natOfBytesLE, List.foldr_cons, List.length_cons] at ih ⊢
    rw [Nat.pow_succ]
    have := Nat.mul_le_mul_right 256 (Nat.succ_le_of_lt ih)
    simp only [Nat.succ_mul] at this
    omega

theorem natOfBytesLE_take_lt (l : List UInt8) (n : Nat) :
    natOfBytesLE (l.take n) < 256 ^ n := by
  have h1 := natOfBytesLE_lt (l.take n)
  have h2 : 256 ^ (l.take n).length ≤ 256 ^ n :=
    Nat.pow_le_pow_right (by decide) (List.length_take_le _ _)
  omega

/-- At least `width / 8` bytes decode to an integer in range, leaving the
rest. -/
theorem bcsDecode_int {it : IntType} {data : List UInt8} (hlen : it.width / 8 ≤ data.length) :
    ∃ x, bcsDecode (.int it) data = some (.int it x, data.drop (it.width / 8)) ∧
      it.inRange x = true := by
  have hraw := natOfBytesLE_take_lt data (it.width / 8)
  refine ⟨if it.isSigned && natOfBytesLE (data.take (it.width / 8)) ≥ 2 ^ (it.width - 1)
    then (natOfBytesLE (data.take (it.width / 8)) : Int) - 2 ^ it.width
    else natOfBytesLE (data.take (it.width / 8)), ?_, ?_⟩
  · simp [bcsDecode, Nat.not_lt.2 hlen]
  · generalize natOfBytesLE (data.take (it.width / 8)) = raw at hraw ⊢
    cases it <;> simp only [IntType.width] at hraw <;>
      simp [IntType.width, IntType.isSigned, IntType.inRange, IntType.min,
        IntType.max] at hraw ⊢ <;> (try split) <;> omega

/-- The value of ULEB128 groups, least significant first. -/
def groupsValue (groups : List Nat) : Nat := groups.foldr (fun g acc => acc * 128 + g) 0

theorem uleb128Groups_ne_nil {data rest : List UInt8} {groups : List Nat}
    (h : uleb128Groups data = some (groups, rest)) : groups ≠ [] := by
  cases data with
  | nil => simp [uleb128Groups] at h
  | cons b data =>
    simp only [uleb128Groups] at h
    split at h
    · simp only [Option.some.injEq, Prod.mk.injEq] at h
      rw [← h.1]; simp
    · simp only [bind, Option.bind_eq_some_iff] at h
      obtain ⟨⟨gs, r⟩, -, h⟩ := h
      simp only [pure, Option.some.injEq, Prod.mk.injEq] at h
      rw [← h.1]; simp

/-- The model's ULEB128 reading agrees with the specification's groups: the
same remainder, the same value, and a last group that is not zero unless it
is the only one read from the start. -/
theorem uleb128_groups : ∀ (data : List UInt8) (shift acc v : Nat) (rest : List UInt8),
    uleb128 data shift acc = some (v, rest) →
    ∃ groups, uleb128Groups data = some (groups, rest) ∧
      v = acc + 2 ^ shift * groupsValue groups ∧
      (0 < shift ∨ 1 < groups.length → groups.getLast? ≠ some 0)
  | [], _, _, _, _, h => by simp [uleb128] at h
  | b :: data, shift, acc, v, rest, h => by
    have hb : b.toNat < 256 := by simpa using b.toNat_lt
    simp only [uleb128] at h
    split at h
    · simp at h
    · split at h
      · next hlt =>
        split at h
        · simp at h
        · next hz =>
          simp only [Option.some.injEq, Prod.mk.injEq] at h
          obtain ⟨rfl, rfl⟩ := h
          refine ⟨[b.toNat], by simp [uleb128Groups, hlt], ?_, ?_⟩
          · simp [groupsValue, Nat.mod_eq_of_lt hlt]; rw [Nat.mul_comm]
          · intro hs
            simp only [List.length_singleton, Nat.lt_irrefl, or_false] at hs
            simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq, not_and] at hz
            simpa using hz hs
      · next hge =>
        split at h
        · simp at h
        · obtain ⟨groups, hg, hv, hc⟩ := uleb128_groups data (shift + 7) _ v rest h
          refine ⟨(b.toNat - 128) :: groups, ?_, ?_, ?_⟩
          · simp [uleb128Groups, hge, hg]
          · rw [hv]
            have hmod : b.toNat % 128 = b.toNat - 128 := by omega
            simp only [groupsValue, List.foldr_cons, hmod, Nat.pow_add]
            generalize 2 ^ shift = P
            generalize List.foldr (fun g acc => acc * 128 + g) 0 groups = f
            simp only [Nat.mul_add, show (2:Nat) ^ 7 = 128 from rfl]
            rw [Nat.mul_comm (b.toNat - 128) P, Nat.mul_assoc, Nat.mul_comm 128 f]
            omega
          · intro _
            cases groups with
            | nil => exact absurd rfl (uleb128Groups_ne_nil hg)
            | cons g gs =>
              simp only [List.getLast?_cons_cons]
              exact hc (Or.inl (by omega))

/-- A length the model reads is one the specification decodes. -/
theorem decodeLength_of_uleb128 {data rest : List UInt8} {len : Nat}
    (h : uleb128 data 0 0 = some (len, rest)) (hmax : len ≤ maxSequenceLength) :
    decodeLength data = some (len, rest) := by
  obtain ⟨groups, hg, hv, hc⟩ := uleb128_groups data 0 0 len rest h
  simp only [Nat.pow_zero, Nat.one_mul, Nat.zero_add] at hv
  have hcanon : ¬ (1 < groups.length ∧ groups.getLast? = some 0) :=
    fun ⟨h1, h2⟩ => hc (Or.inr h1) h2
  have hmax' : ¬ bcsMaxSequenceLength < groupsValue groups := by
    simp only [maxSequenceLength] at hmax
    simp only [bcsMaxSequenceLength]
    omega
  simp only [decodeLength, hg, bind, Option.bind, groupsValue] at hcanon hmax' ⊢
  simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at *
  rw [if_neg hcanon, if_neg hmax']
  subst hv
  rfl

/-- What the specification decodes from a value the model skips over: the
same remainder, and a value of the type in every program. -/
def Decodes (rt : RuntimeType) (data rest : List UInt8) : Prop :=
  ∃ v, bcsDecode rt data = some (v, rest) ∧ ∀ P : Program, v.conforms P rt = true

theorem decodeSeq_of_elems {t : SignatureToken} {rt : RuntimeType}
    (ih : ∀ data rest, bcsSkip t data = some rest → Decodes rt data rest) :
    ∀ (n : Nat) (data rest : List UInt8), bcsSkip.elems t n data = some rest →
      ∃ vs, decodeSeq (bcsDecode rt) n data = some (vs, rest) ∧
        ∀ P : Program, ∀ v ∈ vs, v.conforms P rt = true
  | 0, data, rest, h => by
    rw [bcsSkip.elems] at h
    simp only [Option.some.injEq] at h
    subst h
    exact ⟨[], by simp [decodeSeq], by simp⟩
  | n + 1, data, rest, h => by
    rw [bcsSkip.elems] at h
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨mid, hmid, h⟩ := h
    obtain ⟨v, hv, hvc⟩ := ih data mid hmid
    obtain ⟨vs, hvs, hvsc⟩ := decodeSeq_of_elems ih n mid rest h
    refine ⟨v :: vs, ?_, fun P w hw => ?_⟩
    · simp [decodeSeq, hv, hvs, bind, pure]
    · rcases List.mem_cons.1 hw with rfl | hw
      · exact hvc P
      · exact hvsc P w hw

theorem intDecodes (it : IntType) (k : Nat) (hk : it.width / 8 = k) (data rest : List UInt8)
    (h : (if data.length ≥ k then some (data.drop k) else none) = some rest) :
    Decodes (.int it) data rest := by
  split at h
  · next hlen =>
    simp only [Option.some.injEq] at h
    subst h hk
    obtain ⟨x, hx, hr⟩ := bcsDecode_int hlen
    exact ⟨.int it x, hx, fun _ => by simp [hr]⟩
  · simp at h

/-- **A value the model skips over decodes**, for every constant type. -/
theorem bcsSkip_decodes (m : CompiledModule) :
    ∀ (t : SignatureToken), isValidForConstant t = true →
      ∃ rt, RuntimeType.ofToken m t = some rt ∧
        ∀ data rest, bcsSkip t data = some rest → Decodes rt data rest := by
  intro t
  induction t using SignatureToken.rec (motive_2 := fun _ => True) with
  | bool =>
    intro _
    refine ⟨.bool, rfl, fun data rest h => ?_⟩
    match data, h with
    | b :: rest', h =>
      simp only [bcsSkip] at h
      split at h
      · simp only [Option.some.injEq] at h
        subst h
        by_cases h0 : b = 0
        · exact ⟨.bool false, by simp [bcsDecode, h0], fun _ => by simp⟩
        · have h0' : b.toNat ≠ 0 := fun e => h0 (UInt8.toNat_inj.1 (by simpa using e))
          have h1 : b = 1 := UInt8.toNat_inj.1 (by simp; omega)
          exact ⟨.bool true, by simp [bcsDecode, h1], fun _ => by simp⟩
      · simp at h
    | [], h => simp [bcsSkip] at h
  | address =>
    intro _
    refine ⟨.address, rfl, fun data rest h => ?_⟩
    simp only [bcsSkip] at h
    split at h
    · next hlen =>
      simp only [Option.some.injEq] at h
      subst h
      exact ⟨.address (BitVec.ofNat 256 (natOfBytesBE (data.take 32))),
        by simp [bcsDecode, Nat.not_lt.2 hlen],
        fun _ => by simp [Value.conforms, Value.type, Value.wellFormed]⟩
    · simp at h
  | vector t ih =>
    intro hv
    simp only [isValidForConstant] at hv
    obtain ⟨rt, hrt, hdec⟩ := ih hv
    refine ⟨.vector rt, by simp only [RuntimeType.ofToken] at hrt ⊢; simp [RuntimeType.ofTokenWith, hrt],
      fun data rest h => ?_⟩
    simp only [bcsSkip, bind, Option.bind_eq_some_iff] at h
    obtain ⟨⟨len, rest'⟩, hlen, h⟩ := h
    split at h
    · simp at h
    · next hmax =>
      have hdl := decodeLength_of_uleb128 hlen (Nat.le_of_not_gt hmax)
      obtain ⟨vs, hvs, hvsc⟩ := decodeSeq_of_elems hdec len rest' rest h
      refine ⟨.vector rt vs, ?_, fun P => ?_⟩
      · simp [bcsDecode, hdl, hvs, bind, pure]
      · exact conforms_of (assignable_refl _) (wellFormed_vector.2 (hvsc P))
  | signer | function | struct | structInstantiation | reference | mutableReference
  | typeParameter =>
    intro hv; simp [isValidForConstant] at hv
  | nil | cons => trivial
  | u8 => intro _; exact ⟨.int .u8, rfl, fun d r h =>
      intDecodes .u8 1 rfl d r (by simpa only [bcsSkip] using h)⟩
  | u16 => intro _; exact ⟨.int .u16, rfl, fun d r h =>
      intDecodes .u16 2 rfl d r (by simpa only [bcsSkip] using h)⟩
  | u32 => intro _; exact ⟨.int .u32, rfl, fun d r h =>
      intDecodes .u32 4 rfl d r (by simpa only [bcsSkip] using h)⟩
  | u64 => intro _; exact ⟨.int .u64, rfl, fun d r h =>
      intDecodes .u64 8 rfl d r (by simpa only [bcsSkip] using h)⟩
  | u128 => intro _; exact ⟨.int .u128, rfl, fun d r h =>
      intDecodes .u128 16 rfl d r (by simpa only [bcsSkip] using h)⟩
  | u256 => intro _; exact ⟨.int .u256, rfl, fun d r h =>
      intDecodes .u256 32 rfl d r (by simpa only [bcsSkip] using h)⟩
  | i8 => intro _; exact ⟨.int .i8, rfl, fun d r h =>
      intDecodes .i8 1 rfl d r (by simpa only [bcsSkip] using h)⟩
  | i16 => intro _; exact ⟨.int .i16, rfl, fun d r h =>
      intDecodes .i16 2 rfl d r (by simpa only [bcsSkip] using h)⟩
  | i32 => intro _; exact ⟨.int .i32, rfl, fun d r h =>
      intDecodes .i32 4 rfl d r (by simpa only [bcsSkip] using h)⟩
  | i64 => intro _; exact ⟨.int .i64, rfl, fun d r h =>
      intDecodes .i64 8 rfl d r (by simpa only [bcsSkip] using h)⟩
  | i128 => intro _; exact ⟨.int .i128, rfl, fun d r h =>
      intDecodes .i128 16 rfl d r (by simpa only [bcsSkip] using h)⟩
  | i256 => intro _; exact ⟨.int .i256, rfl, fun d r h =>
      intDecodes .i256 32 rfl d r (by simpa only [bcsSkip] using h)⟩


/-- **Constants decode.** Every constant of a module the constants pass
accepts has a constant type and decodes to a value of it, in every program. -/
theorem constants_decode {m : CompiledModule} (h : verifyConstants m = .ok ()) :
    ∀ c ∈ m.constantPool.toList, isValidForConstant c.type = true ∧
      ∃ rt v, RuntimeType.ofToken m c.type = some rt ∧ decodeConstant rt c.data = some v ∧
        ∀ P : Program, v.conforms P rt = true := by
  intro c hc
  have hv := forM_ok h c hc
  unfold verifyConstant at hv
  split at hv
  · simp [reject] at hv
  · next hvalid =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hvalid
    split at hv
    · next hs =>
      obtain ⟨rt, hrt, hdec⟩ := bcsSkip_decodes m c.type hvalid
      obtain ⟨v, hv, hvc⟩ := hdec c.data [] hs
      exact ⟨hvalid, rt, v, hrt, by simp [decodeConstant, hv], hvc⟩
    · simp [reject] at hv
    · simp [reject] at hv
end MoveBytecode.Proofs
