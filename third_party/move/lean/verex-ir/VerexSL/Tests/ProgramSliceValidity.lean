-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

/-! ## The fact of a program's slice and reversed range

A program's slice of a vector denotes the vector a specification states of
the range (`SpecVector.extract`, `denote_slice_direct`), and its reversal of
a range denotes `reverseSliceBounded` (`denote_reverseSlice_direct`). Both
keep the vector they are of, so the fact of the result is that vector's by
the operation's law (`NTy.Holds.extract`, `NTy.Holds.reverseSliceBounded`)
where it is in hand; where it is not, the result is read element by element
as before. A range outside the vector aborts as before.

The controls run both operations through the walk at an opaque and at a
declared element type, as the final value of a mutable parameter and as a
returned value; with an element the type's invariant refuses left in the
vector; and with a range nothing bounds.
-/

namespace VerexSL.Tests.ProgramSliceValidity

set_option Elab.async false

open Lean Meta Elab VerexIR VerexSL

verex module 0x42::program_slice_validity where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  fun slice_mut_generic_ok {T}(v : &mut Vector<T>, a : u64, b : u64) -> Unit :=
    *v := core.prim.slice(*v, a, b)
  spec slice_mut_generic_ok where
    requires a <= b && b <= v.length
  fun slice_mut_percent_ok(v : &mut Vector<Percent>, a : u64, b : u64) -> Unit :=
    *v := core.prim.slice(*v, a, b)
  spec slice_mut_percent_ok where
    requires a <= b && b <= v.length
  fun slice_ret_generic_ok {T}(v : Vector<T>, a : u64, b : u64) -> Vector<T> :=
    core.prim.slice(v, a, b)
  spec slice_ret_generic_ok where
    requires a <= b && b <= v.length
  fun slice_ret_percent_ok(v : Vector<Percent>, a : u64, b : u64) -> Vector<Percent> :=
    core.prim.slice(v, a, b)
  spec slice_ret_percent_ok where
    requires a <= b && b <= v.length
    ensures result.length == b - a

  fun reverse_mut_generic_ok {T}(v : &mut Vector<T>, a : u64, b : u64) -> Unit :=
    *v := core.prim.reverseSliceVector(*v, a, b)
  spec reverse_mut_generic_ok where
    requires a <= b && b <= v.length
  fun reverse_mut_percent_ok(v : &mut Vector<Percent>, a : u64, b : u64) -> Unit :=
    *v := core.prim.reverseSliceVector(*v, a, b)
  spec reverse_mut_percent_ok where
    requires a <= b && b <= v.length
  fun reverse_ret_generic_ok {T}(v : Vector<T>, a : u64, b : u64) -> Vector<T> :=
    core.prim.reverseSliceVector(v, a, b)
  spec reverse_ret_generic_ok where
    requires a <= b && b <= v.length
  fun reverse_ret_percent_ok(v : Vector<Percent>, a : u64, b : u64) -> Vector<Percent> :=
    core.prim.reverseSliceVector(v, a, b)
  spec reverse_ret_percent_ok where
    requires a <= b && b <= v.length
    ensures result.length == v.length

  -- An element the invariant refuses stays in the range that is kept.
  fun slice_invalid_bad(v : &mut Vector<Percent>, a : u64, b : u64) -> Unit := do
    (*v)[a].value := 101u64
    *v := core.prim.slice(*v, a, b)
  spec slice_invalid_bad where
    requires a < b && b <= v.length
  fun reverse_invalid_bad(v : &mut Vector<Percent>, a : u64, b : u64) -> Unit := do
    (*v)[a].value := 101u64
    *v := core.prim.reverseSliceVector(*v, a, b)
  spec reverse_invalid_bad where
    requires a < b && b <= v.length

  -- A range nothing bounds: the operation aborts on it.
  fun slice_range_bad(v : Vector<Percent>, a : u64, b : u64) -> Vector<Percent> :=
    core.prim.slice(v, a, b)
  spec slice_range_bad where
    aborts_if false
  fun reverse_range_bad(v : Vector<Percent>, a : u64, b : u64) -> Vector<Percent> :=
    core.prim.reverseSliceVector(v, a, b)
  spec reverse_range_bad where
    aborts_if false

/-! ### Negative controls -/

open LoanDeathControls in
run_cmd do
  let copiedOut := "where a value is copied out of a mutable reference"
  expectCuts `«0x42».program_slice_validity.slice_invalid_bad
    [("invariant value <= 100", copiedOut)]
    (← `(command| verify 0x42::program_slice_validity::slice_invalid_bad)) (some [])
  expectCuts `«0x42».program_slice_validity.reverse_invalid_bad
    [("invariant value <= 100", copiedOut)]
    (← `(command| verify 0x42::program_slice_validity::reverse_invalid_bad)) (some [])
  expectUnestablished `«0x42».program_slice_validity.slice_range_bad
    ["aborts_if false"]
    (← `(command| verify 0x42::program_slice_validity::slice_range_bad))
  expectUnestablished `«0x42».program_slice_validity.reverse_range_bad
    ["aborts_if false"]
    (← `(command| verify 0x42::program_slice_validity::reverse_range_bad))

/-! ### Positive controls -/

verify 0x42::program_slice_validity::slice_mut_generic_ok
verify 0x42::program_slice_validity::slice_mut_percent_ok
verify 0x42::program_slice_validity::slice_ret_generic_ok
verify 0x42::program_slice_validity::slice_ret_percent_ok
verify 0x42::program_slice_validity::reverse_mut_generic_ok
verify 0x42::program_slice_validity::reverse_mut_percent_ok
verify 0x42::program_slice_validity::reverse_ret_generic_ok
verify 0x42::program_slice_validity::reverse_ret_percent_ok

end VerexSL.Tests.ProgramSliceValidity
