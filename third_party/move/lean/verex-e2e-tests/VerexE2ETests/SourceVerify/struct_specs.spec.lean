-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec Range where
  invariant low <= high

spec widen where
  ensures range.low == old(range).low
  ensures range.high >= old(range).high

spec make where
  ensures result.low == low
