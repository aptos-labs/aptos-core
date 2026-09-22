-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Cli

def main (arguments : List String) : IO UInt32 :=
  LeanerIR.Rust.Cli.run arguments
