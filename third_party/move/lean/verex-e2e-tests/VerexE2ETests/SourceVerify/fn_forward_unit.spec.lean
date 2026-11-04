-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec unit_mut where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), callback)

spec unit_forward where
  pragma opaque
  ensures callback == old(callback)
  ensures ensures_of<__trait_Fn_call>(old(callback))
