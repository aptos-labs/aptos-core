-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Import
import Tests.Import

/-!
# Configuration correspondence (A3)

`Tests/fixtures/verifier_config.json` is the output of `config-export/`, which
calls `aptos_prod_verifier_config` on the recorded inputs. The value recorded
in `Spec/Config.lean` must be exactly that.
-/

namespace MoveBytecode.Tests

open MoveBytecode.Import

#eval show IO Unit from do
  let (inputs, config) ← IO.ofExcept (recordedConfig (← readFixture "verifier_config.json"))
  check (inputs == productionInputs) "recorded inputs"
  check (config == VerifierConfig.production) "recorded production configuration"

end MoveBytecode.Tests
