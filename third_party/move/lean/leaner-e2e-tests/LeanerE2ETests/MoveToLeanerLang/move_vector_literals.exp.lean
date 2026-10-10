-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x47::vector_literals where
  public fun pair(a : u64, b : u64) -> Vector<u64> := vector<u64>[a, b]

  public fun nested(n : u64) -> Vector<Vector<u64> > :=
    vector<Vector<u64> >[vector<u64>[n], vector<u64>[n, n], vector<u64>[]]

  public fun deep(n : u64) -> Vector<Vector<Vector<u64> > > :=
    vector<Vector<Vector<u64> > >[vector<Vector<u64> >[vector<u64>[n]]]

  public fun empty() -> Vector<u64> := vector<u64>[]
