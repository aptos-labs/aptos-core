-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace unsupported where
  @[refused = "unsupported MIR terminator: InlineAsm"]
  native fun inline_assembly() -> Unit

  -- This fixture is intentionally outside the accepted subset. M0 must
  -- observe the MIR tag so the mapper can reject it without erasing it.

-- no denotation for `inline_assembly`: a function without a body is not carried by the denotation
