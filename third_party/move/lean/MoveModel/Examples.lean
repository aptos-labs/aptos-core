-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Examples.MasmSource
import MoveModel.Examples.MoveSource
import MoveModel.Examples.ElimSource
import MoveModel.Examples.Adequacy

/-!
# Frontend and Proof Examples

Elaborating these modules invokes the Aptos CLI's `aptos move exchange`
command: set `APTOS_CLI` to the binary, or have `aptos` on `PATH`.

`MoveModel.Examples.Adequacy` is self-contained and instantiates every assumption
of the end-to-end prover theorem for a minimal typed identity function.
-/
