-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace scalar where
  fun scalar(left : u32, right : u32) -> u32 := left ^ right
