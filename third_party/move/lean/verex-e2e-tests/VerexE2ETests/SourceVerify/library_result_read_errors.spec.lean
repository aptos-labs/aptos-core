-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec or_panic where
  panics_if false -- error: it panics at `Err`

spec success where
  ensures result is Some -- error: not of `Err`

spec failure where
  ensures result is Some -- error: not of `Ok`
