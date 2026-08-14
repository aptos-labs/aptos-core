-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Examples.MasmSource
import Move.Examples.MoveSource
import Move.Examples.ElimSource
import Move.Examples.Adequacy

/-!
# Frontend and Proof Examples

Elaborating these modules invokes the Aptos CLI's `aptos move exchange`
command: set `APTOS_CLI` to the binary, or have `aptos` on `PATH`.

`Move.Examples.Adequacy` is self-contained and instantiates every assumption
of the end-to-end prover theorem for a minimal typed identity function.
-/
