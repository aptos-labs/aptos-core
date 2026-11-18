-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `split_reference_end` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/split_reference_end.move` that the
source-verification driver writes to `.verex/split_reference_end.move.lean`,
with the items of `VerexE2ETests/SourceVerify/split_reference_end.proof.lean`
as items of the module and the `pragma proof_file` that names that file
dropped: the driver defers this fixture's verification to the proof file,
which a lone elaboration does not read, so the snapshot carries the proofs
themselves and proves what the fixture owes. Its `verex sources at` command is
dropped and the diagnostics are those of `split_reference_end.exp`, which
holds the proof file's messages as well. Nothing here is edited by hand:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders and what the
proof file says.
-/

/--
error: `decide` does not close the obligation from the block's facts:
  case data_invariant
  Θ✝ : Proofs.Denote.Skolems unit
  typeInstantiation✝ : Array (TypeId × TypeId)
  frame✝¹ :
    Proofs.Denote.FrameOf unit { namespaceId := { index := 0 }, functionId := { index := 0 } } [BinderKind.typeArg]
      typeInstantiation✝
  old_v : (Proofs.Denote.NTy.param 0).vector.carrier
  assumed permitted : True
  v_valid : (Proofs.Denote.NTy.param 0).vector.Holds old_v
  state✝ : Proofs.Denote.Memory unit
  fst✝² : Option (Proofs.Denote.NTy.param 0).carrier
  fst✝¹ : Option (Proofs.Denote.NTy.param 0).vector.carrier
  v : (Proofs.Denote.NTy.param 0).vector.carrier
  n : (Proofs.Denote.NTy.int 64 false).carrier
  left✝⁴ : fst✝².elim True fun value => (Proofs.Denote.NTy.param 0).Holds value
  left✝³ : fst✝¹.elim True fun value => (Proofs.Denote.NTy.param 0).vector.Holds value
  frame✝ : (Proofs.Denote.NTy.param 0).vector.Holds v
  n_upper : n ≤ 0
  a✝ : v.values.size == 0
  value✝ : (Proofs.Denote.NTy.param 0).carrier
  fst✝ snd✝ : (Proofs.Denote.NTy.param 0).vector.carrier
  left✝² : (Proofs.Denote.NTy.param 0).vector.Holds fst✝
  right✝² : (Proofs.Denote.NTy.param 0).vector.Holds snd✝
  w✝ : Int
  left✝¹ : 0 ≤ w✝
  left✝ : w✝ < ↑fst✝.values.size
  right✝¹ : fst✝.values[w✝.toNat]? == some value✝
  bounds✝² : old_v.values.size < 2 ^ 64
  bounds✝¹ : fst✝.values.size < 2 ^ 64
  bounds✝ : snd✝.values.size < 2 ^ 64
  invariant_1 : n == ↑0
  n_lower : 0 ≤ n
  right✝ : n ≤ 18446744073709551615
  bounds : True
  ⊢ (Proofs.Denote.NTy.param 0).Holds value✝
---
error: verex verification failed: the proof of `drain` does not establish its specification
-/
#guard_rendering in
verex module 0x42::split_reference_end where

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- A loop whose body takes a `&mut` of the whole vector: `drain`'s authored
  -- `compute` route leaves `data_invariant` unproved while a prepared leaf's
  -- context holds that invariant fact; the first missing correspondence and
  -- cause are unlocalized. `drain_automatic` is the control.
  fun drain {T has Drop}(mut v : Vector<T>) -> Unit := do
    let n := v.length
    while n > 0 do
      let r := &mut v
      if r.length == 0 then moveVectorError(2)
      let (_t1, _t2) := core.prim.removeVector(*r, r.length - 1)
      *r := _t2
      _t1
      n := n - 1
    where
      invariant n == v.length
    core.prim.destroyEmptyVector(v)

  spec drain where
    aborts_if false

  fun drain_automatic {T has Drop}(mut v : Vector<T>) -> Unit := do
    let n := v.length
    while n > 0 do
      let r := &mut v
      if r.length == 0 then moveVectorError(2)
      let (_t1, _t2) := core.prim.removeVector(*r, r.length - 1)
      *r := _t2
      _t1
      n := n - 1
    where
      invariant n == v.length
    core.prim.destroyEmptyVector(v)

  spec drain_automatic where
    aborts_if false

  verify drain by
    on _ { compute }
