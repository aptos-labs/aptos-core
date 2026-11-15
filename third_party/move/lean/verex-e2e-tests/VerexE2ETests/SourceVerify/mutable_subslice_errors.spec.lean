-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A mutable subslice is rejected: the false contract must not verify.

spec through_subslice where
  ensures values == old(values)
