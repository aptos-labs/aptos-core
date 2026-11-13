-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec or_panic where
  ensures result == value.0 -- error: it panics at `None`

spec view where
  ensures result is Some -- error: not of `None`

spec or_error where
  ensures result is Ok -- error: not of `None`

spec taken where
  ensures value == old(value) -- error: it is left `None`

spec replaced where
  ensures result is None -- error: not if it held one

spec defaulted where
  ensures value is None ==> result == 1 -- error: the default is zero
