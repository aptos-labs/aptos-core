-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.Cli
import LeanerMove.Frontend.LIR.Backend

/-!
# The Move exchange frontend

The Move-profile frontend onto the unified LIR: `move exchange --format ast`
invocation, XAST decoding, and encoding into a validated `RawUnit`. Ported
from the deprecated transpiler package, which retains its frozen copy for
reference.
-/
