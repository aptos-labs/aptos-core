-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Verification simp inventories

This intentionally has no dependency on `Move.Attributes`: importing that
module before global semantics introduces the Move `Key` ability into scope,
which changes the interpretation of existing semantic binders named `Key`.
-/

register_simp_attr move_norm
register_simp_attr wp_norm
register_simp_attr move_spec
