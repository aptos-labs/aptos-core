-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Reducible
import Proofs.WithinLimits
import Proofs.InstantiationsFinite
import Proofs.LoopDepth
import Proofs.LoopSummary

/-!
# The local properties (L1–L11)

UNTRUSTED (checked by the kernel). Every module the model accepts has the
local properties of `LocalProperties`: L1–L6 and L8–L10 as before
(`Proofs/Reducible.lean`), L7 from the instantiation-loop check
(`instantiationsFinite_of`, `Proofs/InstantiationsFinite.lean`) and L11 from
the limits pass, the basic-block count of the code-unit verifier and the loop
depth of the control-flow pass (`withinLimits_of`,
`Proofs/WithinLimits.lean`; `loopDepthAtMost_of`, `Proofs/LoopDepth.lean`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- The limits pass and the instantiation-loop check of a module the model
accepts. -/
theorem verifyModule_limits {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) :
    verifyLimits VerifierConfig.production m = .ok () ∧ verifyInstantiationLoops m = .ok () := by
  unfold verifyModule at h
  simp only [show (VerifierConfig.production.scope == .nothing) = false from rfl,
    Bool.false_eq_true, ↓reduceIte, bind_ok] at h
  obtain ⟨-, -, -, -, u3, h3, -, -, -, -, -, -, -, -, -, -, -, -, u10, h10, -⟩ := h
  cases u3; cases u10
  exact ⟨h3, h10⟩

/-- No two function definitions share a handle (L2). -/
theorem functionDefs_injective {m : CompiledModule} (hu : m.Uniqueness) :
    ∀ (d1 d2 : Nat) (fd1 fd2 : FunctionDefinition), m.functionDefs[d1]? = some fd1 →
      m.functionDefs[d2]? = some fd2 → fd1.function = fd2.function → d1 = d2 := by
  intro d1 d2 fd1 fd2 h1 h2 he
  have hnd := hu.2.2.2.2.2.2.2.1
  have hl1 : d1 < (m.functionDefs.toList.map (·.function)).length := by
    simpa using (Array.getElem?_eq_some_iff.1 h1).1
  have hl2 : d2 < (m.functionDefs.toList.map (·.function)).length := by
    simpa using (Array.getElem?_eq_some_iff.1 h2).1
  refine (List.getElem?_inj hl1 hnd).1 ?_
  rw [List.getElem?_map, List.getElem?_map, Array.getElem?_toList, Array.getElem?_toList, h1, h2]
  simp [he]

/-- The code-unit verifier's block count of a function with code. -/
theorem verifyFunction_blockCount {config : VerifierConfig} {m : CompiledModule} {index n : Nat}
    {fdef : FunctionDefinition} {code : CodeUnit} (hcode : fdef.code = some code)
    (h : verifyFunction config m index fdef = .ok n) :
    ∃ cfg, verifyControlFlow config m.version code = .ok cfg ∧
      ∀ k, config.maxBasicBlocks = some k → cfg.blocks.length ≤ k := by
  unfold verifyFunction at h
  rw [hcode] at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨handle, -, cfg, hcf, view, -, h⟩ := h
  refine ⟨cfg, hcf, fun k hk => ?_⟩
  rw [hk] at h
  simp only at h
  split at h
  · simp [reject, bind, Except.bind] at h
  · omega

/-- **L1–L11** for every module the model accepts, given the phantom-position
rule for field types that mention a reference. -/
theorem localProperties_of_phantom (hph : PhantomUnderReferences) :
    LocalProperties verifier := by
  intro m hacc hv1 hv2
  obtain ⟨h1, h2, h3, h4, h5, h6, h7, h8⟩ :=
    localPropertiesNonGeneric_of_phantom hph m hacc hv1 hv2
  have h : verifyModule VerifierConfig.production m = .ok () := by
    simp only [verifier] at hacc
    cases hm : verifyModule VerifierConfig.production m with
    | ok u => rfl
    | error e => rw [hm] at hacc; simp [accepts] at hacc
  obtain ⟨hlim, hinst⟩ := verifyModule_limits h
  obtain ⟨hb, -, -, hcu⟩ := verifyModule_ok h
  refine ⟨h1, h2, h3, h4, h5, h6, h7, h8, instantiationsFinite_of hinst (functionDefs_injective h2),
    withinLimits_of (Or.inl rfl) hlim (fun d hd => ?_) (fun d hd code hcode => ?_)⟩
  · -- The handle and signatures of every function definition (L1).
    obtain ⟨fh, hfh, hrange, -⟩ := h1.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2 d hd
    obtain ⟨-, -, ⟨params, hp, -⟩, ⟨returns, hr, -⟩⟩ :=
      h1.2.2.2.1 fh (Array.mem_toList_iff.2 (Array.mem_of_getElem? hfh))
    exact ⟨fh, params, returns, hfh, hp, hr⟩
  · obtain ⟨index, hidx⟩ := List.mem_iff_getElem?.1 hd
    rw [Array.getElem?_toList] at hidx
    obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hidx
    obtain ⟨cfg, hcf, hk⟩ := verifyFunction_blockCount hcode hn
    have hbr := fun pc o i hi ho => branch_targets hb hd hcode (pc := pc) (o := o) (i := i) hi ho
    exact ⟨fun k hk' => basicBlocksAtMost_of (verifyControlFlow_blocks hcf) (hk k hk'),
      fun k hk' => loopDepthAtMost_of hcf hk' hbr
        (fun hbuild _ hSlen h0 hS => summary_facts hbuild hSlen h0 hS)⟩

/-- **L1–L11**, unconditionally, once the model's phantom check looks through
references in the production configuration. -/
theorem localProperties_of_switch
    (hsw : SignatureV2.phantomThroughReferences VerifierConfig.production = true) :
    LocalProperties verifier :=
  localProperties_of_phantom (phantomUnderReferences_of_switch hsw)

end MoveBytecode.Proofs
