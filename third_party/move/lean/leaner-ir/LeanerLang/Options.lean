-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-- The heartbeat budget of a generated verification theorem, in the same
thousands-of-heartbeats units as `maxHeartbeats`. -/
register_option leaner.verifyHeartbeats : Nat := {
  defValue := 400000000
  descr := "heartbeat budget of a generated verification theorem (thousands)"
}
