-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Frontend.Cli
import VerexMove.Frontend.VIR.Backend
import VerexMove.Frontend.LoopInvariants

/-!
# The Move exchange frontend

The Move-profile frontend onto the unified VIR: `move exchange --format ast`
invocation, XAST decoding, and encoding into a validated `RawUnit`.
-/
