-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide
open VerexIR VerexIR.Proofs.Denote

namespace VerexIR.Tests.VectorWriteSizes

-- Certification and a preceding swap preserve the size used by the removal guard.

theorem erase_after_swap [Carriers] (v : (NTy.vector (.param 0)).plain)
    (held : NTy.deepHolds (.vector (.param 0)) v) (i j : Nat)
    (within : j < v.values.size) :
    (((NTy.ofDeepHolds (.vector (.param 0)) v held).values.swapIfInBounds i j).eraseIdxIfInBounds j).size + 1 =
      v.values.size := by
  verex_lookups_after_writes
  omega

set_option linter.unusedVariables false in
example [Carriers] (v : (NTy.vector (.param 0)).plain)
    (held : NTy.deepHolds (.vector (.param 0)) v) (i j : Nat) : True := by
  fail_if_success
    have : (((NTy.ofDeepHolds (.vector (.param 0)) v held).values.swapIfInBounds i j).eraseIdxIfInBounds j).size + 1 = v.values.size := by
      verex_lookups_after_writes
      omega
  trivial

end VerexIR.Tests.VectorWriteSizes
