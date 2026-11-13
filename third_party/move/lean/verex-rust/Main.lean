-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.Cli

def main (arguments : List String) : IO UInt32 :=
  VerexIR.Rust.Cli.run arguments
