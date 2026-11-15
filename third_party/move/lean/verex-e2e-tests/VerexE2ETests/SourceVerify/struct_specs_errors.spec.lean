-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec Range where
  invariant low <= high

spec invert where
  requires low <= high
