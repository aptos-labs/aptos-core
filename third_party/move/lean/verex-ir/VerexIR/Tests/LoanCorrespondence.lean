-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.LoanCorrespondence

set_option linter.unusedSimpArgs false

namespace VerexIR.Proofs.Denote
open VerexIR.SemanticOperations
variable [Carriers]

namespace Regression
-- The outer reference changes from loan 2 to loan 3. Both inner prophecies
-- retain their own identity, including the discarded original loan 2.
private def redirect : LoanCorrespondence :=
  [(1, .borrow 3 (.bool false)), (2, .bool false), (3, .bool true)]

example : NTy.corresponds redirect (.ref (.ref .bool))
    ((true, false), (false, true)) (.borrow 1 (.borrow 2 (.bool true))) := by
  refine ⟨1, .borrow 2 (.bool true), .borrow 3 (.bool false), rfl, rfl, ?_, ?_⟩
  · exact ⟨2, .bool true, .bool false, rfl, rfl, rfl, rfl⟩
  · exact ⟨3, .bool false, .bool true, rfl, rfl, rfl, rfl⟩

example : redirect.Valid := by
  simp [RuntimeValue.holeless, RuntimeValue.holelessList, LoanCorrespondence.Valid, LoanCorrespondence.Settles, redirect,
    boundaryRowLoanIds, boundaryLoanIds, exportedRaw?, LoanCorrespondence.returnedRow,
    resolveReturnedBorrows, outermostBorrows, collectPruned, collectPrunedList, borrowEntry?,
    fillHole?, rewriteFirst, holeFill?]
example : redirect.Settles [1, 2, 3] #[]
    [(1, .borrow 3 (.bool false)), (2, .bool false)] := by
  simp [RuntimeValue.holeless, RuntimeValue.holelessList, LoanCorrespondence.Valid, LoanCorrespondence.Settles, redirect,
    boundaryRowLoanIds, boundaryLoanIds, exportedRaw?, LoanCorrespondence.returnedRow,
    resolveReturnedBorrows, outermostBorrows, collectPruned, collectPrunedList, borrowEntry?,
    fillHole?, rewriteFirst, holeFill?]

-- Discarding the old inner loan without an export cannot erase its promise.
example : ¬redirect.Settles [1, 2, 3] #[] [(1, .borrow 3 (.bool false))] := by
  simp [RuntimeValue.holeless, RuntimeValue.holelessList, LoanCorrespondence.Valid, LoanCorrespondence.Settles, redirect,
    boundaryRowLoanIds, boundaryLoanIds, exportedRaw?, LoanCorrespondence.returnedRow,
    resolveReturnedBorrows, outermostBorrows, collectPruned, collectPrunedList, borrowEntry?,
    fillHole?, rewriteFirst, holeFill?]

-- The same mutable reference cannot escape twice.
example : ¬redirect.Settles [1, 2, 3] #[.borrow 3 (.bool false)]
    [(1, .borrow 3 (.bool false)), (2, .bool false)] := by
  simp [RuntimeValue.holeless, RuntimeValue.holelessList, LoanCorrespondence.Valid, LoanCorrespondence.Settles, redirect,
    boundaryRowLoanIds, boundaryLoanIds, exportedRaw?, LoanCorrespondence.returnedRow,
    resolveReturnedBorrows, outermostBorrows, collectPruned, collectPrunedList, borrowEntry?,
    fillHole?, rewriteFirst, holeFill?]

-- An unrelated map entry cannot manufacture a loan or fill storage holes.
example : ¬LoanCorrespondence.Settles ((99, .bool true) :: redirect) [1, 2, 3] #[]
    [(1, .borrow 3 (.bool false)), (2, .bool false)] := by
  simp [RuntimeValue.holeless, RuntimeValue.holelessList, LoanCorrespondence.Valid, LoanCorrespondence.Settles, redirect,
    boundaryRowLoanIds, boundaryLoanIds, exportedRaw?, LoanCorrespondence.returnedRow,
    resolveReturnedBorrows, outermostBorrows, collectPruned, collectPrunedList, borrowEntry?,
    fillHole?, rewriteFirst, holeFill?]

-- Distinct maps cannot give two final referents to the same loan.
example : ¬LoanCorrespondence.Valid ((3, .bool false) :: redirect) := by
  simp [RuntimeValue.holeless, RuntimeValue.holelessList, LoanCorrespondence.Valid, LoanCorrespondence.Settles, redirect,
    boundaryRowLoanIds, boundaryLoanIds, exportedRaw?, LoanCorrespondence.returnedRow,
    resolveReturnedBorrows, outermostBorrows, collectPruned, collectPrunedList, borrowEntry?,
    fillHole?, rewriteFirst, holeFill?]
-- Swapping two nested references preserves both inner identities without
-- treating either original referent as discarded or duplicating ownership.
example : LoanCorrespondence.Settles
    [(1, .borrow 4 (.bool false)), (2, .bool true),
      (3, .borrow 2 (.bool true)), (4, .bool false)] [1, 2, 3, 4]
    #[] [(1, .borrow 4 (.bool false)), (3, .borrow 2 (.bool true))] := by
  simp [LoanCorrespondence.Settles, boundaryRowLoanIds, boundaryLoanIds,
    exportedRaw?, LoanCorrespondence.returnedRow, resolveReturnedBorrows,
    outermostBorrows, collectPruned, collectPrunedList, borrowEntry?,
    fillHole?, rewriteFirst, holeFill?]

-- A freshly minted inner loan may escape inside the returned outer reference.
example : LoanCorrespondence.Settles
    [(1, .borrow 4 (.bool true)), (2, .bool false), (4, .bool true)] [1, 2]
    #[.borrow 1 (.borrow 4 (.bool true))] [(2, .bool false)] := by
  simp [LoanCorrespondence.Settles, boundaryRowLoanIds, boundaryLoanIds,
    exportedRaw?, LoanCorrespondence.returnedRow, resolveReturnedBorrows,
    outermostBorrows, collectPruned, collectPrunedList, borrowEntry?,
    fillHole?, rewriteFirst, holeFill?]

-- Dead exports cannot justify each other's holes through a circular promise.
example : ¬LoanCorrespondence.Settles [(1, .bool true), (2, .bool true)] [1, 2]
    #[] [(1, .loanHole 2), (2, .loanHole 1)] := by
  simp [LoanCorrespondence.Settles, boundaryRowLoanIds, boundaryLoanIds,
    exportedRaw?, LoanCorrespondence.returnedRow, resolveReturnedBorrows]

-- Storage observation uses nested returned identities, but never unrelated
-- entries in the correspondence.
example : LoanCorrespondence.returnedRow [(1, .borrow 4 (.bool true)),
    (4, .bool true), (99, .bool false)] #[.borrow 1 (.borrow 4 (.bool false))] =
    #[.borrow 1 (.borrow 4 (.bool true)), .borrow 4 (.bool true)] := by
  simp [LoanCorrespondence.returnedRow, boundaryRowLoanIds, boundaryLoanIds, exportedRaw?]

private def shortSlice : SpecSliceRef Bool := SpecSliceRef.settled ⟨#[true], by decide⟩
private def longSlice : SpecSliceRef Bool := SpecSliceRef.settled ⟨#[false, true], by decide⟩
private def sliceRedirect : LoanCorrespondence :=
  [(1, .borrow 3 (.vector #[.bool false, .bool true])),
    (2, .vector #[.bool true]), (3, .vector #[.bool false, .bool true])]

-- Replacing a reference may change the extent of the slice it points to;
-- each individual slice loan still has the same current and final extent.
example : NTy.corresponds sliceRedirect (.ref (.sliceRef .bool))
    (shortSlice, longSlice) (.borrow 1 (.borrow 2 (.vector #[.bool true]))) := by
  refine ⟨1, _, _, rfl, rfl, ?_, ?_⟩
  · refine ⟨2, ?_, ?_⟩ <;>
      simp [sliceRedirect, exportedRaw?, shortSlice, SpecSliceRef.settled_currentVector,
        SpecSliceRef.settled_finalVector] <;> rfl
  · refine ⟨3, ?_, ?_⟩ <;>
      simp [sliceRedirect, exportedRaw?, longSlice, SpecSliceRef.settled_currentVector,
        SpecSliceRef.settled_finalVector] <;> rfl

end Regression

end VerexIR.Proofs.Denote
