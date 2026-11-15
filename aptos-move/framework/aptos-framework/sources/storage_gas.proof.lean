-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "storage_gas.move"
verex proofs where

-- The curve's invariants are the negation of `validate_points`'s abort conditions, instantiated.
verify new_gas_curve by
  on requires of validate_points, data invariant 1 { }
  on data invariant 3 {
    if points.length > 0 {
      assert points[0].x != 0
      assert points[0].x >= 0
      assert points[points.length - 1].x != BASIS_POINT_DENOMINATION
      assert points[points.length - 1].x <= BASIS_POINT_DENOMINATION
    } else { }
  }
  on aborts_if 1 when aborts at call 1 of validate_points {
    split points.length > 0 && points[points.length - 1].x == BASIS_POINT_DENOMINATION
    split points.length > 0 && points[0].x == 0
  }
  on aborts_if 1 { }
  on _ { }

-- Each iteration checks one adjacency of the points with their sentinels: its check extends the
-- invariants, its failure is a witness of the `aborts_if`, and at the exit the invariants at
-- position `k + 1` refute each disjunct.
verify validate_points by
  on invariant 2 step {
    split j == i
  }
  on invariant 3 step {
    split i == 0
  }
  on invariant 4 step {
    split i == len
  }
  on aborts_if 1 when returns { }
  on aborts_if 1 {
    if i != 0 && i != len {
      assert ∃ (k : Int), 0 <= k && k < len - 1 && (points[k + 1].x <= points[k].x || points[k + 1].y < points[k].y) by witness i - 1
    } else { }
    if i == 0 && len > 0 {
      split i == 0
      assert points[0].x == 0
    } else { }
    if i == len && len > 0 {
      assert points[len - 1].x <= BASIS_POINT_DENOMINATION
      assert points[len - 1].y <= BASIS_POINT_DENOMINATION
      assert points[len - 1].x == BASIS_POINT_DENOMINATION
    } else { }
  }
  on _ { }
