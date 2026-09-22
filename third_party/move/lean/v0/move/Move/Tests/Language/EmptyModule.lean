-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move

/-! An empty module is valid after compiler-expanded inline declarations have
been removed from a backend projection. -/

open Move
open scoped Move

module EmptyModule at 0x42 where
