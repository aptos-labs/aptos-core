-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace variant_move where
  struct Leaf where
    value : u32

  enum Container where
    | Item (0 : Leaf) = 0

  fun take(container : Container) -> Leaf := move(container.0)
