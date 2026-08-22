-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Init.System.IO

/-- Maximum number of concurrent Lake worker threads used to compile tests. -/
def maxConcurrentTestBuilds : String := "2"

def main : IO UInt32 := do
  let concurrency := (← IO.getEnv "LEAN_NUM_THREADS").getD maxConcurrentTestBuilds
  let child ← IO.Process.spawn {
    cmd := "lake"
    args := #["build", "MoveModelTests"]
    env := #[
      ("LEAN_NUM_THREADS", some concurrency)
    ]
  }
  child.wait
