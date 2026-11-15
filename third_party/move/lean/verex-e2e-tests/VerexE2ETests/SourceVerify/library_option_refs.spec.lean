-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec copied_value where
  ensures (result is Some) == (value is Some)
  ensures value is Some ==> result.0 == value.0

spec cloned_value where
  ensures (result is Some) == (value is Some)
  ensures value is Some ==> result.0 == value.0
