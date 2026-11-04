-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec literal where
  ensures result == 'a'

spec same where
  ensures result == (left == right)

spec classify where
  ensures result <= 1
  ensures value == 'a' ==> result == 1
