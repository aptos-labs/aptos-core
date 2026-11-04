-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec abort_now where
  aborts_if true

spec spin_unless where
  ensures result == value
