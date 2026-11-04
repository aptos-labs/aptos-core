-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace «abort» where
  fun abort_now() -> Never := panic()
