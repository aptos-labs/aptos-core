-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace never where
  fun spin_forever() -> Never := loop continue
