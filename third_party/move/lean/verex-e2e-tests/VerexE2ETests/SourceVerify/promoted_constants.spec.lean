-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Promoted constants are evaluated by the exporter, so this call verifies.

spec no_contract where
  ensures result == 5
