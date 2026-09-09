-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeFlow

namespace LeanerIR.Proofs.NativeReturnFlow

/-- At the function boundary the target payload is a typed returned value.
Normal control alone executes the shared continuation. -/
def finish (first : Spec State Error (NativeFlow.Flow Locals Result))
    (next : Locals → Spec State Error Result) : Spec State Error Result :=
  Spec.bind first fun
    | .normal locals => next locals
    | .continue_ result | .break_ result => Spec.pure result

end LeanerIR.Proofs.NativeReturnFlow
