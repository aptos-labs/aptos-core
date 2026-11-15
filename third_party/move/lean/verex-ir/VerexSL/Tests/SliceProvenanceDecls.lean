-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

set_option Elab.async false

open Lean Meta Elab Tactic VerexIR VerexSL

/-! ## Slice birth, lender provenance, clearing, and settlement controls -/

verex namespace slice_provenance where
  pragma verify = false
  struct Percent where
    value : u32
  spec Percent where
    invariant value <= 100

  struct Holder where
    arr : Vector<Percent, const 2>
  spec Holder where
    invariant arr[0].value <= arr[1].value

  struct Holder3 where
    arr : Vector<Percent, const 3>
  spec Holder3 where
    invariant arr[0].value <= arr[1].value

  struct SliceSlot {a : lifetime} where
    target : &[a] mut Slice<Percent>

  -- (a) Slice birth through an open `&mut` reference (`hasDeref = true`).
  fun slice_birth_array_open_ok(r : &mut Vector<Percent, const 2>, amount : u32) -> usize := do
    (*r)[0usize].value := amount
    let s := &mut (unsize *r)
    s.length
  spec slice_birth_array_open_ok where
    requires amount <= 100

  fun slice_birth_array_open_bad(r : &mut Vector<Percent, const 2>) -> usize := do
    (*r)[0usize].value := 101u32
    let s := &mut (unsize *r)
    s.length

  fun slice_birth_reborrow_open_ok {a : lifetime} (
      r : &mut &[a] mut Slice<Percent>, amount : u32) -> usize := do
    (**r)[0usize].value := amount
    let s := &mut **r
    s.length
  spec slice_birth_reborrow_open_ok where
    requires r.length > 0
    requires amount <= 100

  fun slice_birth_reborrow_open_bad {a : lifetime} (
      r : &mut &[a] mut Slice<Percent>) -> usize := do
    (**r)[0usize].value := 101u32
    let s := &mut **r
    s.length
  spec slice_birth_reborrow_open_bad where
    requires r.length > 0

  -- (b) Enclosing `Holder` cross-element invariant across slice mutation (`SliceLender`).
  fun slice_holder_order_ok() -> u32 := do
    let mut h := new Holder {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 0u32 }] }
    let s := &mut (unsize h.arr)
    s[1usize] := new Percent { value := 100u32 }
    0u32

  fun slice_holder_order_bad() -> u32 := do
    let mut h := new Holder {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 0u32 }] }
    let s := &mut (unsize h.arr)
    s[0usize] := new Percent { value := 100u32 }
    0u32

  fun slice_holder_either_ok(flag : Bool) -> u32 := do
    let mut h1 := new Holder {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 100u32 }] }
    let mut h2 := new Holder {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 100u32 }] }
    let s := if flag then &mut (unsize h1.arr) else &mut (unsize h2.arr)
    s[0usize] := new Percent { value := 50u32 }
    0u32

  fun slice_holder_either_bad(flag : Bool) -> u32 := do
    let mut h1 := new Holder {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 0u32 }] }
    let mut h2 := new Holder {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 0u32 }] }
    let s := if flag then &mut (unsize h1.arr) else &mut (unsize h2.arr)
    s[0usize] := new Percent { value := 100u32 }
    0u32

  fun slice_holder_diff_len_ok(flag : Bool) -> u32 := do
    let mut h2 := new Holder {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 100u32 }] }
    let mut h3 := new Holder3 {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 100u32 },
        new Percent { value := 100u32 }] }
    let s := if flag then &mut (unsize h2.arr) else &mut (unsize h3.arr)
    s[0usize] := new Percent { value := 50u32 }
    0u32

  fun slice_holder_diff_len_bad(flag : Bool) -> u32 := do
    let mut h2 := new Holder {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 0u32 }] }
    let mut h3 := new Holder3 {
      arr := #[new Percent { value := 0u32 }, new Percent { value := 0u32 },
        new Percent { value := 0u32 }] }
    let s := if flag then &mut (unsize h2.arr) else &mut (unsize h3.arr)
    s[0usize] := new Percent { value := 100u32 }
    0u32

  fun slice_reborrow_single_bad(p : &mut Slice<Percent>) -> Unit := do
    let s := &mut *p
    (*s)[0usize].value := 101u32
  spec slice_reborrow_single_bad where
    requires p.length > 0

  fun slice_slot_reborrow_ok(p : &mut Slice<Percent>, amount : u32) -> Unit := do
    let slot := new SliceSlot<lifetime _> { target := &mut *p }
    let s := &mut *(slot.target)
    let elem := &mut s[0usize]
    (*elem).value := amount
  spec slice_slot_reborrow_ok where
    requires p.length > 0
    requires amount <= 100

  fun slice_slot_reborrow_bad(p : &mut Slice<Percent>) -> Unit := do
    let slot := new SliceSlot<lifetime _> { target := &mut *p }
    let s := &mut *(slot.target)
    let elem := &mut s[0usize]
    (*elem).value := 101u32
  spec slice_slot_reborrow_bad where
    requires p.length > 0

  -- (d) Slice settlement current-element check through an open reference
  -- (`SliceLender.slot` before overwrite, and `Mutables.resolve_sliceAt` at exit).
  fun slice_settle_open_ok {a : lifetime} (
      r : &mut &[a] mut Slice<Percent>, other : &[a] mut Slice<Percent>,
      amount : u32) -> Unit := do
    (**r)[0usize].value := amount
    *r := other
  spec slice_settle_open_ok where
    requires r.length > 0
    requires amount <= 100

  fun slice_settle_open_bad {a : lifetime} (
      r : &mut &[a] mut Slice<Percent>, other : &[a] mut Slice<Percent>) -> Unit := do
    (**r)[0usize].value := 101u32
    *r := other
  spec slice_settle_open_bad where
    requires r.length > 0

  fun slice_exit_open_ok {a : lifetime} (
      r : &mut &[a] mut Slice<Percent>, amount : u32) -> Unit := do
    (**r)[0usize].value := amount
  spec slice_exit_open_ok where
    requires r.length > 0
    requires amount <= 100

  fun slice_exit_open_bad {a : lifetime} (
      r : &mut &[a] mut Slice<Percent>) -> Unit := do
    (**r)[0usize].value := 101u32
  spec slice_exit_open_bad where
    requires r.length > 0


namespace SliceProvenanceInspection
open VerexIR VerexIR.Validation VerexIR.Proofs.Denote

def describeSliceLenders {Γ : NRow} {σ : NTy} : SliceLender Γ σ → List String
  | .slot => ["slot"]
  | .array .. => ["array"]
  | .vector .. => ["vector"]
  | .slice .. => ["slice"]
  | .candidateArray .. => ["candidateArray"]
  | .candidateVector .. => ["candidateVector"]
  | .candidateSlice .. => ["candidateSlice"]
  | .referent => ["referent"]
  | .equation => ["equation"]
  | .seq first second => describeSliceLenders first ++ describeSliceLenders second

mutual
partial def collectSliceLenders {unit : ValidatedUnit} {ρ : ResultShape} :
    {Γ : NRow} → {τ : NTy} → Term unit ρ Γ τ → List String
  | _, _, .resolveSliceAt _ _ _ lender => describeSliceLenders lender
  | _, _, .let_ _ value body => collectSliceLenders value ++ collectSliceLenders body
  | _, _, .letRow _ value body => collectSliceLenders value ++ collectSliceLenders body
  | _, _, .letFields _ value body => collectSliceLenders value ++ collectSliceLenders body
  | _, _, .drop first second => collectSliceLenders first ++ collectSliceLenders second
  | _, _, .seqAfter first second => collectSliceLenders first ++ collectSliceLenders second
  | _, _, .ite _ cond left right =>
      collectSliceLenders cond ++ collectSliceLenders left ++ collectSliceLenders right
  | _, _, .caseOf scrutinee arms => collectSliceLenders scrutinee ++ collectArmsSliceLenders arms
  | _, _, .loop _ body => collectSliceLenders body
  | _, _, .return_ value => collectSliceLenders value
  | _, _, .assign _ value => collectSliceLenders value
  | _, _, .mutate _ value => collectSliceLenders value
  | _, _, .writePlace _ _ _ _ value => collectSliceLenders value
  | _, _, _ => []

partial def collectArmsSliceLenders {unit : ValidatedUnit} {ρ : ResultShape} :
    {Γ : NRow} → {σ τ : NTy} → Arms unit ρ Γ σ τ → List String
  | _, _, _, .nil _ => []
  | _, _, _, .cons _ _ body rest => collectSliceLenders body ++ collectArmsSliceLenders rest
  | _, _, _, .guarded _ _ guard body rest =>
      collectSliceLenders guard ++ collectSliceLenders body ++ collectArmsSliceLenders rest
end

def functionSliceLenders (unit : ValidatedUnit) (name : String) : Except String (List String) := do
  let some ns := unit.namespaces[0]? | .error "missing namespace 0"
  let some fnIdx := ns.functions.findIdx? fun fn =>
      unit.tables.names[fn.name.index]?.any (·.name == name)
    | .error s!"missing function {name}"
  let some cert := ns.borrowCertificates.find? (·.functionId == ⟨fnIdx⟩)
    | .error s!"missing borrow certificate for {name}"
  if cert.loans.isEmpty && cert.carrierDeaths.isEmpty then
    .error s!"expected loan or carrier-death facts for {name}"
  let compiled ← compileFunction unit { namespaceId := ⟨0⟩, functionId := ⟨fnIdx⟩ }
  .ok (collectSliceLenders compiled.body)

end SliceProvenanceInspection

run_cmd do
  let env ← getEnv
  let some unit := VerexSL.registeredUnit? env `slice_provenance
    | throwError "missing registered unit `slice_provenance`"
  for (fnName, expected) in [
      ("slice_holder_order_bad", ["array"]),
      ("slice_holder_either_bad", ["referent", "candidateArray", "candidateArray", "equation"]),
      ("slice_holder_diff_len_bad", ["referent", "candidateArray", "candidateArray", "equation"]),
      ("slice_reborrow_single_bad", ["slice"]),
      ("slice_slot_reborrow_bad",
        ["referent", "candidateSlice", "candidateSlice", "equation",
         "referent", "candidateSlice", "equation"]),
      ("slice_settle_open_bad", ["slot"])] do
    match SliceProvenanceInspection.functionSliceLenders unit fnName with
    | .ok actual =>
        unless actual == expected do
          throwError "{fnName}: expected slice lenders {expected}, found {actual}"
    | .error err => throwError "{fnName}: inspection failed: {err}"
