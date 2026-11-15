-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec copied_value where
  ensures result is Some -- error: not of `None`

spec cloned_value where
  ensures value is Some ==> result.0 == value.0 + 1 -- error: it is the value
