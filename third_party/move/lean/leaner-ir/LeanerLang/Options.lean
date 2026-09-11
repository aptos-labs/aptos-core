-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-- Source verification is native-only. Retired route spellings remain
diagnosable, but cannot enable a legacy proof generator or fallback. -/
register_option leaner.route : String := {
  defValue := "native"
  descr := "verification route: native only; legacy script/normalize/compose routes are disabled"
}

/-- The heartbeat budget of a generated verification theorem, in the same
thousands-of-heartbeats units as `maxHeartbeats`. -/
register_option leaner.verifyHeartbeats : Nat := {
  defValue := 400000000
  descr := "heartbeat budget of a generated verification theorem (thousands)"
}
