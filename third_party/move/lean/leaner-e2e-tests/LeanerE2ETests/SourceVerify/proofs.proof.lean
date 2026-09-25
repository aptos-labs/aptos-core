-- The obligations the closer leaves: the abort condition needs the square's
-- sign, the result its expansion.
verify square_of_sum by
  case leaf_1 =>
    have := Int.mul_nonneg ‹0 ≤ a.val + b.val› ‹0 ≤ a.val + b.val›
    omega
  case leaf_2 =>
    rw [Int.add_mul, Int.mul_add, Int.mul_add, Int.mul_comm b.val a.val, Int.mul_assoc]
    omega
