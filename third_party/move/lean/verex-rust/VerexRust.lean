-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.Registry
import VerexRust.Source
import VerexRust.Equivalence

/-!
# Verex Rust profile

The Rust frontend package owns Rust-profile policy and integration with the
project-owned Rustc Public exporter. It depends on the shared VIR boundary but
does not create a second semantic representation or wire format.
-/
