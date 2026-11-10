-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Certified

/-!
# Non-vacuity (N2)

UNTRUSTED (checked by the kernel). Every function with code whose parameter
and local types are in the subset, and whose parameter types are
constructible, can be started: every constructible type has a conforming
value, built bottom-up as `Program.Constructible` is (`false`, `0`, the zero
address or signer, the empty vector, and a struct or enum value of
conforming fields), so arguments conforming to the parameter types exist.

N2 for transactions starts the entry function the same way, over empty
global storage, after leading signers all for one authenticated address: a
`signer` parameter gets the signer, and an `&signer` parameter a reference to
the next signer the transaction holds, so the references are to distinct
whole held values.
-/

namespace MoveBytecode.Proofs

variable {P : Program}

/-- Values conforming pointwise to types that each have one. -/
theorem conformsAll_of_forall :
    ∀ {tys : List RuntimeType}, (∀ t ∈ tys, ∃ v : Value, v.conforms P t = true) →
      ∃ vs, conformsAll P vs tys = true
  | [], _ => ⟨[], by simp⟩
  | t :: ts, h => by
    obtain ⟨v, hv⟩ := h t (List.mem_cons_self ..)
    obtain ⟨vs, hvs⟩ := conformsAll_of_forall (fun u hu => h u (List.mem_cons_of_mem _ hu))
    exact ⟨v :: vs, by simp [hv, hvs]⟩

/-- **A constructible type has a conforming value.** -/
theorem constructible_conforms {t : RuntimeType} (h : P.Constructible t) :
    ∃ v : Value, v.conforms P t = true := by
  induction h with
  | bool => exact ⟨.bool false, rfl⟩
  | int it =>
    refine ⟨.int it 0, ?_⟩
    simp only [conforms_int_iff, true_and]
    cases it <;> decide
  | address => exact ⟨.address 0, rfl⟩
  | signer => exact ⟨.signer 0, rfl⟩
  | vector elem => exact ⟨.vector elem [], by simp [Value.conforms, Value.type, Value.wellFormed,
      Value.allConform]⟩
  | struct s tys hft _ ih =>
    obtain ⟨vs, hvs⟩ := conformsAll_of_forall ih
    exact ⟨packedValue s vs, packedValue_conforms hft hvs⟩
  | variant s tag tys hft _ ih =>
    obtain ⟨vs, hvs⟩ := conformsAll_of_forall ih
    exact ⟨.variant s tag vs, by simp [Value.conforms, Value.type, Value.wellFormed, hft, hvs]⟩

/-- A constructible parameter type is a value type: a slot type is a
reference or a value type, and no reference type is constructible. -/
theorem constructible_slot_value {f : FunctionRef} (hf : ∀ x ∈ f.typeArgs, x.isValue = true)
    {ps : List RuntimeType} (hps : P.paramTypes f = some ps) {t : RuntimeType} (ht : t ∈ ps)
    (hc : P.Constructible t) : t.isValue = true := by
  simp only [Program.paramTypes, bind, Option.bind_eq_some_iff] at hps
  obtain ⟨info, -, sig, -, hps⟩ := hps
  obtain ⟨tok, -, htok⟩ := mapM_some_mem hps t ht
  by_cases hr : tok.isRefToken = true
  · cases tok <;> simp [SignatureToken.isRefToken] at hr <;>
      simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at htok <;>
      obtain ⟨_, -, rfl⟩ := htok <;> cases hc
  · exact ofSlotTokenWith_value hf (by simpa using hr) htok

/-- Value arguments are admissible with nothing held: no reference among them
needs a held value to designate. -/
theorem heldArguments_values {P : Program} {args : List Value}
    (h : ∀ v ∈ args, v.type.isValue = true) : heldArguments P [] args = true := by
  have hnr : ∀ v ∈ args, ∀ r, v ≠ .ref r := by
    rintro v hv r rfl
    simpa [Value.type, RuntimeType.isValue] using h _ hv
  simp only [heldArguments, List.all_nil, Bool.true_and, Bool.and_eq_true, List.all_eq_true]
  refine ⟨⟨fun v hv => ?_, fun r hr => ?_⟩, ?_⟩
  · cases v <;> first | exact absurd rfl (hnr _ hv _) | exact h _ hv
  · obtain ⟨v, hv, hvr⟩ := List.mem_filterMap.1 hr
    cases v <;> first | exact absurd rfl (hnr _ hv _) | simp at hvr
  · rw [decide_eq_true_iff, List.Nodup, List.pairwise_map, List.pairwise_filterMap]
    refine List.pairwise_of_forall_mem_list fun a ha _ _ => ?_
    cases a <;> first | exact absurd rfl (hnr _ ha _) | simp

/-- **N2.** -/
theorem entriesAdmissible : EntriesAdmissible := by
  intro P f localTys paramTys htargs hl hp hcons
  obtain ⟨ps, ls, hps, rfl⟩ := localTypes_split hl
  rw [hps, Option.some.injEq] at hp
  subst hp
  obtain ⟨args, hargs⟩ := conformsAll_of_forall (fun t ht => constructible_conforms (hcons t ht))
  have hl' := hl
  simp only [Program.localTypes, bind, Option.bind_eq_some_iff] at hl'
  obtain ⟨info, hf, code, hcode, -⟩ := hl'
  have hvals : ∀ v ∈ args, v.type.isValue = true := by
    intro v hv
    obtain ⟨t, ht, hvt⟩ := conformsAll_mem hargs v hv
    rw [assignable_isValue (conforms_assignable hvt)]
    exact constructible_slot_value (typeArgsOk_values htargs) hps ht (hcons t ht)
  have hinit : ∃ s₀, initialState P f args [] = some s₀ ∧
      ∃ fr, s₀.frames = [fr] ∧ fr.function = f ∧ fr.pc = 0 := by
    simp only [initialState, hf, hcode, hl, htargs, Bool.not_true, Bool.false_eq_true,
      ↓reduceIte, bind, Option.bind_some]
    rw [if_neg (by simp [heldArguments_values hvals])]
    exact ⟨_, rfl, _, rfl, rfl, rfl⟩
  obtain ⟨s₀, hs, fr, hfr, h1, h2⟩ := hinit
  exact ⟨s₀, fr, ⟨f, args, ps, [], [], hps, hargs, rfl, hs⟩, hfr, h1, h2⟩

/-! ## N2 for transactions -/

/-- The references among the leading signer arguments, counting held signers
from `k`: one to each held signer in turn, from the whole of it. Stated for any
`g` that picks out references, as `heldArguments` does. -/
theorem signerArgs_refs {g : Value → Option Reference} (hr : ∀ r, g (.ref r) = some r)
    (hs : ∀ a, g (.signer a) = none) :
    ∀ (signers : List (Address × Bool)) (k : Nat),
      (signerArgs signers k).filterMap g =
        (List.range' k (signers.filter (·.2)).length).map
          fun j => (⟨false, .signer, ⟨.held j, []⟩, 0⟩ : Reference)
  | [], _ => rfl
  | (a, false) :: rest, k => by
    simp [signerArgs, hs, signerArgs_refs hr hs rest k]
  | (a, true) :: rest, k => by
    simp [signerArgs, hr, signerArgs_refs hr hs rest (k + 1), List.range'_succ]

/-- Every leading signer argument is a signer or a reference. -/
theorem mem_signerArgs : ∀ {signers : List (Address × Bool)} {k : Nat} {v : Value},
    v ∈ signerArgs signers k → (∃ a, v = .signer a) ∨ ∃ r, v = .ref r
  | [], _, _, h => by simp [signerArgs] at h
  | (a, false) :: rest, k, v, h => by
    simp only [signerArgs, List.mem_cons] at h
    rcases h with rfl | h
    · exact Or.inl ⟨a, rfl⟩
    · exact mem_signerArgs h
  | (_, true) :: rest, k, v, h => by
    simp only [signerArgs, List.mem_cons] at h
    rcases h with rfl | h
    · exact Or.inr ⟨_, rfl⟩
    · exact mem_signerArgs h

/-- Every held signer is a signer. -/
theorem mem_heldSigners {signers : List (Address × Bool)} {v : Value}
    (h : v ∈ heldSigners signers) : ∃ a, v = .signer a := by
  simp only [heldSigners, List.mem_map] at h
  obtain ⟨⟨a, _⟩, -, rfl⟩ := h
  exact ⟨a, rfl⟩

/-- The leading signer arguments are admissible with the held signers, after
value arguments: every reference among them is to a whole held signer, a
distinct one each. -/
theorem heldArguments_signers {P : Program} (signers : List (Address × Bool)) {args : List Value}
    (h : ∀ v ∈ args, v.type.isValue = true) :
    heldArguments P (heldSigners signers) (signerArgs signers 0 ++ args) = true := by
  have hnr : ∀ v ∈ args, ∀ r, v ≠ .ref r := by
    rintro v hv r rfl
    simpa [Value.type, RuntimeType.isValue] using h _ hv
  unfold heldArguments
  rw [List.filterMap_append, signerArgs_refs, List.filterMap_eq_nil_iff.2, List.append_nil]
  rotate_left
  · intro v hv
    cases v <;> first | exact absurd rfl (hnr _ hv _) | rfl
  · intro r; rfl
  · intro a; rfl
  simp only [Bool.and_eq_true, List.all_eq_true, List.all_append, decide_eq_true_eq]
  refine ⟨⟨⟨fun v hv => ?_, fun v hv => ?_, fun v hv => ?_⟩, fun r hr => ?_⟩, ?_⟩
  · obtain ⟨a, rfl⟩ := mem_heldSigners hv
    exact ⟨rfl, rfl⟩
  · rcases mem_signerArgs hv with ⟨a, rfl⟩ | ⟨r, rfl⟩ <;> rfl
  · cases v <;> first | exact absurd rfl (hnr _ hv _) | exact h _ hv
  · obtain ⟨j, hj, rfl⟩ := List.mem_map.1 hr
    have hlen : j < (heldSigners signers).length := by
      simp only [List.mem_range'_1] at hj
      simpa [heldSigners] using hj.2
    obtain ⟨a, ha⟩ := mem_heldSigners (List.getElem_mem hlen)
    simp only [List.getElem?_eq_getElem hlen, ha]
    exact ⟨rfl, rfl⟩
  · rw [List.map_map]
    exact (List.nodup_range' ..).map _ fun i j hij heq => hij (by simpa using heq)

/-- **N2 for transactions.** -/
theorem transactionEntriesAdmissible : TransactionEntriesAdmissible := by
  intro P auth A f localTys byRef tys hA hentry hret htargs hl hp htys hauth
  obtain ⟨ps, ls, hps, rfl⟩ := localTypes_split hl
  rw [hps, Option.some.injEq] at hp
  subst hp
  obtain ⟨args, hargs⟩ :=
    conformsAll_of_forall (fun t ht => constructible_conforms (htys t ht).2)
  have hl' := hl
  simp only [Program.localTypes, bind, Option.bind_eq_some_iff] at hl'
  obtain ⟨info, hf, code, hcode, -⟩ := hl'
  have hvals : ∀ v ∈ args, v.type.isValue = true := by
    intro v hv
    obtain ⟨t, ht, hvt⟩ := conformsAll_mem hargs v hv
    rw [assignable_isValue (conforms_assignable hvt)]
    exact constructible_slot_value (typeArgsOk_values htargs) hps (List.mem_append_right _ ht)
      (htys t ht).2
  let signers := byRef.map fun b => (auth.headD 0, b)
  have hmap : signers.map (fun x => signerParam x.2) = byRef.map signerParam := by
    simp [signers, List.map_map, Function.comp_def]
  have hauth' : ∀ x ∈ signers, x.1 ∈ auth := by
    intro x hx
    simp only [signers, List.mem_map] at hx
    obtain ⟨b, hb, rfl⟩ := hx
    rcases hauth with rfl | hne
    · simp at hb
    · cases auth with
      | nil => exact absurd rfl hne
      | cons a _ => exact List.mem_cons_self ..
  have hinit : ∃ s₀, initialState P f (signerArgs signers 0 ++ args) []
      (heldSigners signers) = some s₀ ∧
      ∃ fr, s₀.frames = [fr] ∧ fr.function = f ∧ fr.pc = 0 := by
    simp only [initialState, hf, hcode, hl, htargs, Bool.not_true, Bool.false_eq_true,
      ↓reduceIte, bind, Option.bind_some]
    rw [if_neg (by simp [heldArguments_signers signers hvals])]
    exact ⟨_, rfl, _, rfl, rfl, rfl⟩
  obtain ⟨s₀, hs, fr, hfr, h1, h2⟩ := hinit
  exact ⟨s₀, fr, ⟨hA, f, signers, tys, args, [], hentry, hret, by rw [hmap]; exact hps, hauth',
    fun t ht => (htys t ht).1, hargs, by simp [GlobalStorage.wellTyped], hs⟩, hfr, h1, h2⟩

end MoveBytecode.Proofs
