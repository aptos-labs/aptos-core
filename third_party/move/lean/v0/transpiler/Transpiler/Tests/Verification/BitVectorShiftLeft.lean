-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Tests.Programs.MoveStdlib.Std.BitVector

/-! A hand proof that the transpiled `bit_vector::shift_left_for_verification_only`
satisfies its declared contract.  Its three sequential loops (clear-all when the
shift covers the whole vector; shift-down then clear-the-tail otherwise) are each
discharged with `Move.Verify.wp_withInvariant_fix_frame` and the `uint_arith`
closer, exercising the closer on genuinely shifted indices -- the case the
automatic verifier cannot yet handle. -/

open Move
open scoped Move Move.Spec
open Std.bit_vector

set_option maxHeartbeats 1000000

private theorem bitVector_length_toNat (self : BitVector) :
    MoveInt.toNat self.length = self.bit_field.toList.length := by
  have h := self.dataInvariant
  simp only [Std.bit_vector.BitVector.Invariant, move_norm,
    Move.UInt.toInt_eq_toNat] at h
  exact_mod_cast h

private theorem bitVector_set_invariant (self : BitVector) (index : U64)
    (value : Bool) :
    BitVector.Invariant
      { length := self.length, bit_field := Move.Vector.set self.bit_field index value } := by
  have h := self.dataInvariant
  simp only [BitVector.Invariant, move_norm, Move.UInt.toInt_eq_toNat,
    Move.Vector.length_toNat, Move.Vector.toList_set, List.length_set] at h ⊢
  exact h

set_option pp.proofs false in
-- The definedness and loop-step obligations share one structural-normalization
-- lemma set each; individual goals exercise different subsets, so the unused-arg
-- linter (whose note documents this switch) is too fine-grained here.
set_option linter.unusedSimpArgs false in
verify Std.bit_vector.shift_left_for_verification_only by
  contract_intro
  clear permitted
  obtain ⟨self, amount⟩ := args
  simp only [is_index_set.sourceSpec, set.sourceSpec, unset.sourceSpec,
    wp_norm, move_norm, move_data, Nat.reducePow]
  intro future
  split
  · -- amount ≥ length: loop A clears every bit, ascending
    rename_i hge
    refine Move.Verify.wp_withInvariant_fix_frame ?entryA ?stepA
    case entryA =>
      dsimp only
      have hself : MoveInt.toNat self.length = self.bit_field.toList.length :=
        bitVector_length_toNat self
      refine ⟨⟨?_, ?_⟩, ?_⟩
      · uint_arith
      · intro k hk0 hk; exfalso; uint_arith
      · intro k _ _; rfl
    case stepA =>
      intro recA recVA st inv
      obtain ⟨i, cur⟩ := st
      obtain ⟨⟨hlen, hcleared⟩, hunchanged⟩ := inv
      dsimp only at hlen hcleared hunchanged ⊢
      -- The data invariants of the values in scope, as Nat length facts, so
      -- the numeric closer relates lengths across the loop.
      have hself : MoveInt.toNat self.length = self.bit_field.toList.length :=
        bitVector_length_toNat self
      have hcur : MoveInt.toNat cur.current.length = cur.current.bit_field.toList.length :=
        bitVector_length_toNat cur.current
      split
      · -- continue: clear bit i (to false), recurse
        rename_i hlt
        simp only [wp_norm, move_norm, move_data, Nat.reducePow, Nat.reduceMod]
        intro future_1
        rw [List.getElem?_eq_getElem (by uint_arith)]
        intro future_2 hfv hfut1
        subst hfv; subst hfut1
        refine ⟨?_, ?_⟩
        · -- data invariant of the rebuilt vector (set preserves length)
          exact bitVector_set_invariant cur.current i false
        · intro holds
          refine ⟨fun _ => ?_, ?_⟩
          · apply recVA
            refine ⟨⟨?_, ?_⟩, ?_⟩
            · -- length preserved
              simp only [Std.bit_vector.BitVector.Invariant, Move.Vector.toList_mk,
                Move.Vector.toList_set, Move.Vector.elems_eq_toList, List.length_set] <;> uint_arith
            · -- bits below i+1 are cleared
              intro k hk0 hk
              change ¬(cur.current.bit_field.toList.set
                (MoveInt.toNat i) false)[k.toNat]! = true
              rw [List.getElem!_set]
              split
              · simp
              · exact hcleared k hk0 (by uint_arith)
            · -- bits from i+1 up are unchanged
              intro k hk0 hk
              change (cur.current.bit_field.toList.set
                (MoveInt.toNat i) false)[k.toNat]! = self.bit_field[k]!
              rw [List.getElem!_set, if_neg (by uint_arith)]
              exact hunchanged k (by uint_arith) (by uint_arith)
          · uint_arith
      · -- exit: all bits cleared; amount-clauses vacuous
        rename_i hge2
        simp only [wp_norm, move_norm, move_data, Nat.reducePow, Nat.reduceMod]
        intro hrec
        rw [← hrec]; clear hrec
        refine ⟨⟨?_, ?_, ?_⟩, trivial⟩
        · intro _ k hk0 hk
          exact hcleared k hk0 (by uint_arith)
        · intro hlt2; exfalso; uint_arith
        · intro hlt2; exfalso; uint_arith
  · -- amount < length: loops B (shift down) and C (clear tail)
    rename_i hlt0
    refine Move.Verify.wp_withInvariant_fix_frame ?entryB ?stepB
    case entryB =>
      dsimp only
      refine ⟨⟨⟨⟨?_, ?_⟩, ?_⟩, ?_⟩, ?_⟩
      · uint_arith
      · rfl
      · intro j hj hj2; exfalso; uint_arith
      · intro j _ _; rfl
      · intro k hk0 hk; exfalso; uint_arith
    case stepB =>
      intro recB recVB st inv
      obtain ⟨i, cur⟩ := st
      have hself : MoveInt.toNat self.length = self.bit_field.toList.length :=
        bitVector_length_toNat self
      have hcur : MoveInt.toNat cur.current.length = cur.current.bit_field.toList.length :=
        bitVector_length_toNat cur.current
      obtain ⟨⟨⟨⟨hige, hlenB⟩, hshift⟩, hkept⟩, hlow⟩ := inv
      dsimp only at hige hlenB hshift hkept hlow ⊢
      simp only [Move.UInt.toInt_eq_toNat] at hshift hkept hlow
      split
      · -- exit (i ≥ len): i := len - amount, then loop C clears the tail
        rename_i hexitB
        simp only [wp_norm, move_norm, move_data, Nat.reducePow, Nat.reduceMod]
        -- the checked subtraction `len - amount` does not underflow (amount < len)
        refine ⟨fun _ => ?_, ?_⟩
        · -- loop C: clear the tail [len-amount, len)
          refine Move.Verify.wp_withInvariant_fix_frame ?entryC ?stepC
          case entryC =>
            dsimp only
            refine ⟨⟨⟨?_, ?_⟩, ?_⟩, ?_⟩
            · exact hlenB
            · intro j hj hj2; exfalso; uint_arith
            · intro k hk0 hk
              simp only [Move.UInt.toInt_eq_toNat]
              exact hlow k hk0 (by uint_arith)
            · uint_arith
          case stepC =>
            intro recC recVC st inv
            obtain ⟨ic, curc⟩ := st
            have hselfc : MoveInt.toNat self.length = self.bit_field.toList.length :=
              bitVector_length_toNat self
            have hcurc : MoveInt.toNat curc.current.length =
                curc.current.bit_field.toList.length :=
              bitVector_length_toNat curc.current
            obtain ⟨⟨⟨hlenC, hclearedC⟩, hlowC⟩, higeC⟩ := inv
            dsimp only at hlenC hclearedC hlowC higeC ⊢
            simp only [Move.UInt.toInt_eq_toNat] at hclearedC hlowC higeC
            split
            · -- exit (ic ≥ len): reconcile; the function postcondition holds
              rename_i hgeC
              simp only [wp_norm, move_norm, move_data, Nat.reducePow, Nat.reduceMod]
              intro hrec
              rw [← hrec]; clear hrec
              simp only [Move.UInt.toInt_eq_toNat]
              refine ⟨⟨?_, ?_, ?_⟩, trivial⟩
              · intro hamt; exfalso; uint_arith
              · intro _ k hk0 hk
                exact hclearedC k (by uint_arith) (by uint_arith)
              · intro _ k hk0 hk
                exact hlowC k (by uint_arith) (by uint_arith)
            · -- continue (ic < len): clear bit ic, recurse
              rename_i hltC
              simp only [wp_norm, move_norm, move_data, Nat.reducePow, Nat.reduceMod]
              intro future_1
              rw [if_pos (by uint_arith)]
              intro future_2
              rw [List.getElem?_eq_getElem (by uint_arith)]
              intro future_3 hf3 hrec2
              subst hf3; subst hrec2
              refine ⟨?_, ?_⟩
              · -- definedness of the rebuilt vector (set preserves length)
                exact bitVector_set_invariant curc.current ic false
              · intro holds hrec3
                refine ⟨fun _ => ?_, ?_⟩
                · apply recVC
                  rw [← hrec3]
                  refine ⟨⟨⟨?_, ?_⟩, ?_⟩, ?_⟩
                  · exact hlenC
                  · -- cleared tail extends to ic+1
                    intro k hk0 hk
                    change ¬(curc.current.bit_field.toList.set
                      (MoveInt.toNat ic) false)[k.toNat]! = true
                    rw [List.getElem!_set]
                    split
                    · simp
                    · exact hclearedC k (by uint_arith) (by uint_arith)
                  · -- low region unchanged (k < len-amount ≤ ic, so k is not the set index)
                    intro k hk0 hk
                    change (curc.current.bit_field.toList.set
                      (MoveInt.toNat ic) false)[k.toNat]! =
                        self.bit_field[k + MoveInt.toInt amount]!
                    rw [List.getElem!_set, if_neg (by uint_arith)]
                    simp only [Move.UInt.toInt_eq_toNat]
                    exact hlowC k hk0 (by uint_arith)
                  · uint_arith
                · uint_arith
        · uint_arith
      · -- continue (i < len): shift bit i down to i - amount, recurse
        rename_i hcontB
        simp only [is_index_set.sourceSpec, set.sourceSpec, unset.sourceSpec,
          wp_norm, move_norm, move_data, Nat.reducePow, Nat.reduceMod]
        have hbc := Move.Vector.elems_length_lt_size cur.current.bit_field
        rw [if_pos (by uint_arith)]
        -- split the read's wp: the value branch, then the vacuous `none` case
        refine ⟨?_, ?_⟩
        intro value hval
        -- the read bit `value` equals both `cur[i]` and (via the not-yet-shifted
        -- invariant clause) the source bit `self[i]`.
        have hvi : cur.current.bit_field.toList[MoveInt.toNat i]! = value := by
          rw [Move.Vector.toList, List.getElem!_eq_getElem?_getD, hval]; rfl
        have hsi : self.bit_field.toList[MoveInt.toNat i]! = value := by
          have h := hkept (MoveInt.toInt i) (by uint_arith) (by uint_arith)
          simp only [Move.UInt.toInt_eq_toNat, Int.toNat_natCast] at h
          change self.bit_field.toList[MoveInt.toNat i]! =
            cur.current.bit_field.toList[MoveInt.toNat i]! at h
          exact h.trans hvi
        split
        · -- read bit is set: write `true` to i - amount
          rename_i hvtrue
          subst hvtrue
          refine ⟨fun hle future_1 => ?_, fun _ => by uint_arith⟩
          rw [if_pos (by uint_arith)]
          intro future_2
          rw [List.getElem?_eq_getElem (by uint_arith)]
          intro future_3 hf3 hrec2
          subst hf3; subst hrec2
          refine ⟨?_, ?_⟩
          · have h := cur.current.dataInvariant
            simp only [BitVector.Invariant, move_norm, Move.UInt.toInt_eq_toNat,
              Move.Vector.length_toNat, Move.Vector.toList_mk,
              Move.Vector.elems_eq_toList, List.length_set] at h ⊢
            exact h
          · intro holds hrec3
            refine ⟨fun _ => ?_, ?_⟩
            · apply recVB
              rw [← hrec3]
              refine ⟨⟨⟨⟨?_, ?_⟩, ?_⟩, ?_⟩, ?_⟩
              · uint_arith
              · exact hlenB
              · -- shifted region [amount, i+1): new index j=i is the written bit
                intro j hj hj2
                change self.bit_field[j]! =
                  (cur.current.bit_field.toList.set
                    (MoveInt.toNat i - MoveInt.toNat amount) true)[
                      (j - MoveInt.toInt amount).toNat]!
                rw [List.getElem!_set]
                split
                · change self.bit_field.toList[j.toNat]! = true
                  rw [(by uint_arith : j.toNat = MoveInt.toNat i)]
                  exact hsi
                · simp only [Move.UInt.toInt_eq_toNat]
                  exact hshift j (by uint_arith) (by uint_arith)
              · -- not-yet-shifted [i+1-amount, len): j ≠ i-amount, unchanged
                intro j hj hj2
                change self.bit_field[j]! =
                  (cur.current.bit_field.toList.set
                    (MoveInt.toNat i - MoveInt.toNat amount) true)[j.toNat]!
                rw [List.getElem!_set]
                split
                · exfalso; uint_arith
                · exact hkept j (by uint_arith) (by uint_arith)
              · -- low region [0, i+1-amount): k=i-amount is the written bit
                intro k hk0 hk
                change (cur.current.bit_field.toList.set
                    (MoveInt.toNat i - MoveInt.toNat amount) true)[k.toNat]! =
                  self.bit_field[k + MoveInt.toInt amount]!
                rw [List.getElem!_set]
                split
                · change true =
                    self.bit_field.toList[(k + MoveInt.toInt amount).toNat]!
                  rw [(by uint_arith :
                    (k + MoveInt.toInt amount).toNat = MoveInt.toNat i)]
                  exact hsi.symm
                · simp only [Move.UInt.toInt_eq_toNat]
                  exact hlow k (by uint_arith) (by uint_arith)
            · uint_arith
        · -- read bit is clear: write `false` to i - amount
          rename_i hvfalse
          simp only [Bool.not_eq_true] at hvfalse
          subst hvfalse
          refine ⟨fun hle future_1 => ?_, fun _ => by uint_arith⟩
          rw [if_pos (by uint_arith)]
          intro future_2
          rw [List.getElem?_eq_getElem (by uint_arith)]
          intro future_3 hf3 hrec2
          subst hf3; subst hrec2
          refine ⟨?_, ?_⟩
          · have h := cur.current.dataInvariant
            simp only [BitVector.Invariant, move_norm, Move.UInt.toInt_eq_toNat,
              Move.Vector.length_toNat, Move.Vector.toList_mk,
              Move.Vector.elems_eq_toList, List.length_set] at h ⊢
            exact h
          · intro holds hrec3
            refine ⟨fun _ => ?_, ?_⟩
            · apply recVB
              rw [← hrec3]
              refine ⟨⟨⟨⟨?_, ?_⟩, ?_⟩, ?_⟩, ?_⟩
              · uint_arith
              · exact hlenB
              · -- shifted region [amount, i+1): new index j=i is the written bit
                intro j hj hj2
                change self.bit_field[j]! =
                  (cur.current.bit_field.toList.set
                    (MoveInt.toNat i - MoveInt.toNat amount) false)[
                      (j - MoveInt.toInt amount).toNat]!
                rw [List.getElem!_set]
                split
                · change self.bit_field.toList[j.toNat]! = false
                  rw [(by uint_arith : j.toNat = MoveInt.toNat i)]
                  exact hsi
                · simp only [Move.UInt.toInt_eq_toNat]
                  exact hshift j (by uint_arith) (by uint_arith)
              · -- not-yet-shifted [i+1-amount, len): j ≠ i-amount, unchanged
                intro j hj hj2
                change self.bit_field[j]! =
                  (cur.current.bit_field.toList.set
                    (MoveInt.toNat i - MoveInt.toNat amount) false)[j.toNat]!
                rw [List.getElem!_set]
                split
                · exfalso; uint_arith
                · exact hkept j (by uint_arith) (by uint_arith)
              · -- low region [0, i+1-amount): k=i-amount is the written bit
                intro k hk0 hk
                change (cur.current.bit_field.toList.set
                    (MoveInt.toNat i - MoveInt.toNat amount) false)[k.toNat]! =
                  self.bit_field[k + MoveInt.toInt amount]!
                rw [List.getElem!_set]
                split
                · change false =
                    self.bit_field.toList[(k + MoveInt.toInt amount).toNat]!
                  rw [(by uint_arith :
                    (k + MoveInt.toInt amount).toNat = MoveInt.toNat i)]
                  exact hsi.symm
                · simp only [Move.UInt.toInt_eq_toNat]
                  exact hlow k (by uint_arith) (by uint_arith)
            · uint_arith
        -- the read's `none` (out-of-bounds) case is vacuous, since i < len
        intro hnone
        rw [List.getElem?_eq_getElem (by uint_arith)] at hnone
        exact absurd hnone (Option.some_ne_none _)
