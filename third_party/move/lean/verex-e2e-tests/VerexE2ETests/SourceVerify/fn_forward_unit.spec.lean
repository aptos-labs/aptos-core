-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Keep this helper transparent so the two unit calls verify without
-- reconstructing their result-free invocation histories.
spec unit_mut where
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), callback)

spec unit_forward where
  pragma opaque
  ensures callback == old(callback)
  ensures ensures_of<__trait_Fn_call>(old(callback))
