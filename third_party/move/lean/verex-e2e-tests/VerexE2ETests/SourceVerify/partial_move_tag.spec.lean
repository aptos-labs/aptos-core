-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec take where
  ensures (result is Some) == (value is Ok)
  ensures result is Some ==> result.0 == value.0
