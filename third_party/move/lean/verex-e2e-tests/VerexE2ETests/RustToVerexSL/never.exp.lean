-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace never where
  fun spin_forever() -> Never := loop continue

-- no denotation for `spin_forever`: the type of the result is not carried by the denotation
