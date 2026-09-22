-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Syntax

namespace LeanerLang

/-- A mixed wildcard closes listed resource families except at their listed
keys, while leaving unlisted families open. This frontend-owned metadata uses
existing contract attributes without changing the interchange schema. -/
def hasLooseFrame (contract : LeanerIR.FunctionContract) : Bool :=
  contract.pragmas.any fun
    | .assign "leaner_loose_frame" (.constant (.bool true)) _ => true
    | _ => false

end LeanerLang
