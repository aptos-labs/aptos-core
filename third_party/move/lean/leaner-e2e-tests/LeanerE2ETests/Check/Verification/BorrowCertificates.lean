-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Source borrow certificates

Port of v0's `Verification/BorrowCertificates.lean`. The Leaner validation
certificate records the shared reference parameter directly; returning that
parameter creates no local loan and leaves no borrow diagnostic.
-/

namespace LeanerLang.Tests.VerificationBorrowCertificates

leaner module 0x42::borrow_certificates where
  public fun identity_ref(input : &u64) -> &u64 := input

  spec identity_ref where
    ensures result == input

#leaner_unit 0x42::borrow_certificates

example :
    («0x42».borrow_certificates.unit.borrowCertificates[0]!).parameters.size = 1 := by
  decide
example :
    («0x42».borrow_certificates.unit.borrowCertificates[0]!).parameters[0]!.kind =
      LeanerIR.ReferenceKind.shared := by
  rfl
example :
    («0x42».borrow_certificates.unit.borrowCertificates[0]!).loans.isEmpty = true := by
  decide
example :
    «0x42».borrow_certificates.unit.borrowDiagnostics.isEmpty = true := by decide

end LeanerLang.Tests.VerificationBorrowCertificates
