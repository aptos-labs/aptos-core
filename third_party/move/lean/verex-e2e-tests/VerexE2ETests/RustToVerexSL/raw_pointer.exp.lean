-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace raw_pointer where
  @[refused = "its signature states `*const u32`, which is not modelled, so a call of it refuses its caller"]
  native fun raw_pointer() -> Never

-- no denotation for `raw_pointer`: a function without a body is not carried by the denotation
