-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Cli

def main (arguments : List String) : IO UInt32 :=
  VerexMove.Cli.run arguments
