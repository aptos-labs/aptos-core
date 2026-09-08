-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Report
import Transpiler.LIR.Codec

/-!
# Validated LIR report seed

Frontend omissions transported as checked Move-profile metadata enter the
backend report here. Printer-specific observations are appended later; they do
not rediscover semantic omissions from the projected XAST package.
-/

namespace Transpiler.LIR.Report

def initial (ns : LeanerIR.Validation.ValidatedNamespace) : Except String Transpiler.Report := do
  let skipped := ns.profileMetadata.toList.filter (·.tag == "metadata.skipped")
  let unsupported ← skipped.mapM fun value => do
    match ← Codec.unpack value.payload with
    | #[name, reason] => pure s!"{name}: {reason}"
    | _ => throw "invalid skipped-declaration metadata"
  pure { unsupported }

end Transpiler.LIR.Report
