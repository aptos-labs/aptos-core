-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ByValue

/-!
# Natives that take and return values

UNTRUSTED (checked by the kernel). A native that is not modeled
(`Program.nativeShape?`) and whose arguments are values behaves as before the
modeled natives: it sees its arguments as they are (`State.nativeView`), they
overlap nowhere (`exclusiveArguments`), its result fits when its values
conform to its return types and it returns nothing else
(`Program.nativeResultFits`), and resuming pushes those values
(`resumeNative_values`).

A modeled native has a reference parameter (`modeledNative?_hasRef`), so no
module whose signatures name no reference declares one.
-/

namespace MoveBytecode.Proofs

variable {P : Program}

theorem nativeShape?_typeArgs (f : FunctionRef) (tys : List RuntimeType) :
    P.nativeShape? { f with typeArgs := tys } = P.nativeShape? f := rfl

/-- Values conforming to value types are no references. -/
theorem noRef_of_conformsAll {vs : List Value} {ts : List RuntimeType}
    (h : conformsAll P vs ts = true) (hts : ∀ t ∈ ts, t.isValue = true) :
    ∀ v ∈ vs, ∀ r, v ≠ .ref r := by
  intro v hv
  obtain ⟨t, ht, hvt⟩ := conformsAll_mem h v hv
  exact not_ref_of_value hvt (hts t ht)

theorem nativeView_of_noRefs (s : State) :
    ∀ {args : List Value}, (∀ v ∈ args, ∀ r, v ≠ .ref r) → s.nativeView args = some args
  | [], _ => rfl
  | v :: vs, h => by
    have hv := h v List.mem_cons_self
    have ih := nativeView_of_noRefs s (args := vs) fun w hw => h w (List.mem_cons_of_mem _ hw)
    unfold State.nativeView at ih ⊢
    rw [List.mapM_cons, ih]
    cases v with
    | ref r => exact absurd rfl (hv r)
    | _ => rfl

theorem argRefs_nil_of_noRefs :
    ∀ {args : List Value}, (∀ v ∈ args, ∀ r, v ≠ .ref r) →
      (args.filterMap fun v => match v with
        | .ref r => some r
        | _ => none) = []
  | [], _ => rfl
  | v :: vs, h => by
    have hv := h v List.mem_cons_self
    have ih := argRefs_nil_of_noRefs (args := vs) fun w hw => h w (List.mem_cons_of_mem _ hw)
    rw [List.filterMap_cons]
    cases v with
    | ref r => exact absurd rfl (hv r)
    | _ => exact ih

theorem exclusiveArguments_of_noRefs {args : List Value} (h : ∀ v ∈ args, ∀ r, v ≠ .ref r) :
    Step.exclusiveArguments args = true := by
  unfold Step.exclusiveArguments
  rw [List.filterMap_eq_nil_iff.2]
  · rfl
  · intro v hv
    cases v with
    | ref r => exact absurd rfl (h _ hv r)
    | _ => rfl

theorem nativeResultFits_none {f : FunctionRef} (hf : P.nativeShape? f = none)
    (tys : List RuntimeType) (views vals writes : List Value) (places : List NativePlace) :
    P.nativeResultFits f tys views vals writes places =
      (conformsAll P vals tys && writes.isEmpty && places.isEmpty) := by
  simp only [Program.nativeResultFits, hf]

theorem writtenReferences_none {f : FunctionRef} (hf : P.nativeShape? f = none)
    (args : List Value) : P.writtenReferences f args = [] := by
  simp [Program.writtenReferences, hf]

theorem placeReferences_nil (s : State) (tys : List RuntimeType) (args : List Value) :
    placeReferences s tys args [] = some [] := by
  simp [placeReferences]

theorem nativeResultsIn_values {α : Type} :
    ∀ {tys : List RuntimeType} {vals : List α}, (∀ t ∈ tys, t.isValue = true) →
      vals.length = tys.length → nativeResultsIn tys vals [] = some vals
  | [], [], _, _ => by simp [nativeResultsIn]
  | t :: tys, v :: vals, h, hl => by
    have ht := h t List.mem_cons_self
    have ih := nativeResultsIn_values (tys := tys) (vals := vals)
      (fun u hu => h u (List.mem_cons_of_mem _ hu)) (by simpa using hl)
    simp [nativeResultsIn, ht, ih]
  | [], _ :: _, _, hl => by simp at hl
  | _ :: _, [], _, hl => by simp at hl

/-- **Resuming a native that is not modeled**, from arguments that are values,
with return types that are values: its result fits exactly when its values
conform and it returns nothing else, and then they are pushed. -/
theorem resumeNative_values {s : State} {fr : Frame} {rest : List Frame} {f : FunctionRef}
    {args vals writes : List Value} {places : List NativePlace} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest) (htys : P.returnTypes f = some tys)
    (hshape : P.nativeShape? f = none) (hargs : ∀ v ∈ args, ∀ r, v ≠ .ref r)
    (hval : ∀ t ∈ tys, t.isValue = true) :
    resumeNative P s f args (.returned vals writes places) =
      if conformsAll P vals tys && writes.isEmpty && places.isEmpty then
        .running { s with frames := { fr with stack := vals.reverse ++ fr.stack, pc := fr.pc + 1 } :: rest }
      else .wrong .nativeContract := by
  simp only [resumeNative, hsf, htys, nativeView_of_noRefs s hargs, nativeResultFits_none hshape,
    writtenReferences_none hshape]
  by_cases hc : (conformsAll P vals tys && writes.isEmpty && places.isEmpty) = true
  · have hc' := hc
    simp only [Bool.and_eq_true, List.isEmpty_iff] at hc'
    obtain ⟨⟨hconf, rfl⟩, rfl⟩ := hc'
    simp only [hc, Bool.not_true, Bool.false_eq_true, ↓reduceIte, writeBack, placeReferences_nil,
      Option.bind_some, nativeResultsIn_values hval (conformsAll_length hconf), hsf]
  · simp only [Bool.not_eq_true] at hc
    simp [hc]

/-! ## Modeled natives have a reference parameter -/

theorem matches_ref_of_ref {m : CompiledModule} {q : TokenPattern} {t : SignatureToken}
    (h : TokenPattern.matches m (.reference q) t = true ∨
      TokenPattern.matches m (.mutableReference q) t = true) : t.isRefToken = true := by
  cases t <;> simp [TokenPattern.matches] at h <;> rfl

theorem matchAll_ref {m : CompiledModule} :
    ∀ {ps : List TokenPattern} {ts : List SignatureToken}, TokenPattern.matchAll m ps ts = true →
      (∃ p ∈ ps, ∃ q, p = .reference q ∨ p = .mutableReference q) → ∃ t ∈ ts, t.isRefToken = true
  | [], [], _, ⟨_, hp, _⟩ => by simp at hp
  | p :: ps, t :: ts, h, ⟨p', hp', hq⟩ => by
    simp only [TokenPattern.matchAll, Bool.and_eq_true] at h
    rcases List.mem_cons.1 hp' with rfl | hp'
    · obtain ⟨q, rfl | rfl⟩ := hq
      · exact ⟨t, List.mem_cons_self, matches_ref_of_ref (Or.inl h.1)⟩
      · exact ⟨t, List.mem_cons_self, matches_ref_of_ref (Or.inr h.1)⟩
    · obtain ⟨u, hu, hur⟩ := matchAll_ref h.2 ⟨p', hp', hq⟩
      exact ⟨u, List.mem_cons_of_mem _ hu, hur⟩
  | [], _ :: _, h, _ => by simp [TokenPattern.matchAll] at h
  | _ :: _, [], h, _ => by simp [TokenPattern.matchAll] at h

/-- Every modeled native declares a reference parameter. -/
theorem modeledNatives_hasRef :
    ∀ e ∈ modeledNatives, ∃ p ∈ e.params, ∃ q, p = .reference q ∨ p = .mutableReference q := by
  intro e he
  simp only [modeledNatives, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl |
    rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp

/-- **A modeled native has a reference parameter** in its declared signature. -/
theorem modeledNative?_hasRef {m : CompiledModule} {d : FunctionDefinition} {k : NativeShape}
    (h : m.modeledNative? d = some k) :
    ∃ fh params, m.functionDefHandle d = some fh ∧ m.signatures[fh.parameters]? = some params ∧
      ∃ t ∈ params, t.isRefToken = true := by
  unfold CompiledModule.modeledNative? at h
  split at h
  · simp at h
  simp only [bind, Option.bind_eq_some_iff, pure] at h
  obtain ⟨_, -, _, -, fh, hfh, params, hparams, _, -, e, he, -⟩ := h
  have hmem := List.mem_of_find?_eq_some he
  have hp := List.find?_some he
  simp only [Bool.and_eq_true] at hp
  exact ⟨fh, params, hfh, hparams, matchAll_ref hp.1.2 (modeledNatives_hasRef e hmem)⟩

/-- A value signature names no reference. -/
theorem mapM_ofToken_noRef {m : CompiledModule} {τs : List RuntimeType} :
    ∀ {sig : List SignatureToken} {tys : List RuntimeType},
      sig.mapM (RuntimeType.ofTokenWith m τs) = some tys → ∀ t ∈ sig, t.isRefToken = false
  | [], _, _, _, ht => by simp at ht
  | t :: sig, tys, h, u, hu => by
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨a, ha, as, has, -⟩ := h
    rcases List.mem_cons.1 hu with rfl | hu
    · cases u <;> simp_all [RuntimeType.ofTokenWith, SignatureToken.isRefToken]
    · exact mapM_ofToken_noRef has u hu

/-- A program of the non-generic subset has no modeled natives: its natives
take and return values only. -/
theorem nativeShape?_nonGeneric (hs : P.SupportedNonGeneric) (f : FunctionRef) :
    P.nativeShape? f = none := by
  cases hf : P.function? f with
  | none => simp [Program.nativeShape?, hf]
  | some info =>
    cases hk : info.module.modeledNative? info.definition with
    | none => simp [Program.nativeShape?, hf, hk]
    | some k =>
      exfalso
      obtain ⟨fh, params, hfh, hparams, t, ht, htr⟩ := modeledNative?_hasRef hk
      have hcode : info.definition.code = none := by
        unfold CompiledModule.modeledNative? at hk
        split at hk
        · simp at hk
        · next h => simpa using h
      obtain ⟨hm, hd, -⟩ := function?_lookup hf
      have hmod := hs _ (Array.mem_toList_iff.2 (Array.mem_of_getElem? hm))
      obtain ⟨fh', hfh', ⟨sig, hsig, hsome⟩, -⟩ :=
        hmod.2.1 _ (Array.mem_toList_iff.2 (Array.mem_of_getElem? hd)) hcode
      simp only [CompiledModule.functionDefHandle] at hfh
      rw [hfh, Option.some.injEq] at hfh'
      subst hfh'
      rw [hparams, Option.some.injEq] at hsig
      subst hsig
      obtain ⟨tys, htys⟩ := Option.isSome_iff_exists.1 hsome
      rw [mapM_ofToken_noRef htys t ht] at htr
      cases htr

/-- A function with code is no modeled native. -/
theorem nativeShape?_of_code {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code) :
    P.nativeShape? f = none := by
  simp [Program.nativeShape?, hf, CompiledModule.modeledNative?, hcode]

end MoveBytecode.Proofs
