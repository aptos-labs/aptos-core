-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

namespace VerexSL.Tests.Reserved

-- A local spelled as the contextual keyword `new` ends a module's last
-- clause: the next command is not read as the type of a construction.
verex module 0x42::reserved_first where
  fun keep(new : u64) -> u64 := new
  spec keep where
    ensures result == new

verex module 0x42::reserved_second where
  fun other() -> u64 := 1

-- Printed, it re-imports.
run_cmd do
  let env ← Lean.getEnv
  let some registered := VerexSL.registeredUnit? env `«0x42».reserved_first
    | throwError "the module was not registered"
  let .ok printed := VerexSL.Print.render env registered
    | throwError "the module did not render"
  let .ok formatted := VerexSL.Print.formatSource env printed
    | throwError "the rendering did not re-import"
  unless formatted == printed do
    throwError "the rendering is not a fixed point: {formatted}"

end VerexSL.Tests.Reserved
