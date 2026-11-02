-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Report
import Transpiler.VIR.Codec

/-!
# Validated VIR report seed

Frontend omissions transported as checked Move-profile metadata enter the
backend report here. Printer-specific observations are appended later; they do
not rediscover semantic omissions from the projected XAST package.
-/

namespace Transpiler.VIR.Report

def initial (ns : VerexIR.Validation.ValidatedNamespace) : Except String Transpiler.Report := do
  let skipped := ns.profileMetadata.toList.filter (·.tag == "metadata.skipped")
  let unsupported ← skipped.mapM fun value => do
    match ← Codec.unpack value.payload with
    | #[name, reason] => pure s!"{name}: {reason}"
    | _ => throw "invalid skipped-declaration metadata"
  pure { unsupported }

end Transpiler.VIR.Report
