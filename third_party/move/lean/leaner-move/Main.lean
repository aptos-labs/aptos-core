-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Cli

def main (arguments : List String) : IO UInt32 :=
  LeanerMove.Cli.run arguments
