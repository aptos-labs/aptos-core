-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
Module which defines SHA hashes for byte vectors.

The functions in this module are natively declared both in the Move runtime
as in the Move prover's prelude.
-/
leaner module 0x1::hash where
  public native fun sha2_256(data : Vector<u8>) -> Vector<u8>

  public native fun sha3_256(data : Vector<u8>) -> Vector<u8>
