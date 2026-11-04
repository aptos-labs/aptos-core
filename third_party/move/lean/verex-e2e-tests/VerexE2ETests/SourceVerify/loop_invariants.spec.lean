-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec count where
  ensures result == n
  loop 1 where
    invariant i <= n

spec sum_to where
  requires n <= 1000
  ensures result == n * 1000
  loop 1 where
    invariant i <= n
    invariant s == i * 1000

spec search where
  ensures target < limit ==> result == target
  ensures target >= limit ==> result == limit
  loop 1 where
    invariant i <= limit
    invariant i <= target

spec nested where
  requires rows <= 1000
  requires columns <= 1000
  ensures result <= rows * 1000
  loop 1 where
    invariant row <= rows
    invariant total <= row * 1000
  loop 2 where
    invariant column <= columns
    invariant total <= row * 1000 + column

spec break_outer where
  ensures result <= rows
  loop 1 where
    invariant row <= rows
  loop 2 where
    invariant column <= rows
    invariant column <= row + 1

spec skip_rows where
  requires rows <= 1000
  requires columns <= 1000
  loop 1 where
    invariant row <= rows
    invariant count <= row * 1002
  loop 2 where
    invariant column <= columns
    invariant count <= (row - 1) * 1002 + column

spec classify where
  requires limit <= 1000
  ensures result >= limit
  loop 1 where
    invariant index <= n
    invariant index <= limit + 1
  loop 2 where
    invariant total <= limit || total == kind

spec branches where
  requires n <= 1000
  ensures flag ==> result == n
  ensures !flag ==> result <= n + 1
  loop 1 where
    invariant i <= n
  loop 2 where
    invariant i <= n + 1

spec sum where
  requires values.length <= 1000
  ensures result <= values.length * 4294967295
  loop 1 where
    invariant index <= values.length
    invariant total <= index * 4294967295

spec position where
  requires values.length <= 1000
  ensures result <= values.length
  loop 1 where
    invariant index <= values.length
