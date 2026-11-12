-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec answer where
  ensures result

spec limit where
  ensures result == 100

spec through_dependency where
  ensures result == 41 -- error: the dependency's function has no contract yet
