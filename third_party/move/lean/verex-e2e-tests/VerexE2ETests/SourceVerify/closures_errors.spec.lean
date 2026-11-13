-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec direct where
  requires c <= 1000
  ensures result == 2 * c + 3

spec direct_false where
  requires c <= 1000
  ensures result == 2 * c + 4 -- error: 1 + c + 2 + c

spec no_upvars where
  requires x <= 1000
  ensures result == x + 1

spec no_upvars_false where
  requires x <= 1000
  ensures result == x -- error: the closure adds 1

spec once where
  ensures result == v.length

spec once_false where
  ensures result == 0 -- error: not for a non-empty vector

spec consume where
  ensures result == v

spec consume_false where
  ensures result.length == 0 -- error: the vector comes back whole

-- The panic of the closure's body surfaces at the caller's call.
spec panics where
  ensures result == c + 1

spec panics_guarded where
  requires c <= 1000
  ensures result == c + 1

spec nested where
  requires x <= 1000
  ensures result == x + 3

spec nested_false where
  requires x <= 1000
  ensures result == x -- error: both closures add

spec copied where
  requires c <= 1000
  ensures result == 2 * c + 3

spec zero_sized_copy where
  requires x <= 1000
  ensures result == 2 * x + 2

spec zero_sized_capture where
  requires x <= 1000
  ensures result == x + 1

spec zero_sized_capture_false where
  requires x <= 1000
  ensures result == x -- error: the captured closure adds 1

spec counter where
  requires n0 <= 1000
  ensures result == n0 + 3

spec counter_false where
  requires n0 <= 1000
  ensures result == n0 -- error: the lender sees the closure's writes

spec write_through where
  ensures a == b

spec write_through_false where
  ensures a == old(a) -- error: written through the upvar

spec moved_counter where
  requires start <= 1000
  ensures result == start + 2

spec moved_counter_false where
  requires start <= 1000
  ensures result == start + 1 -- error: the second call sees the first's write

spec moved_copy_keeps where
  requires start <= 1000
  ensures result == start

spec moved_copy_keeps_false where
  requires start <= 1000
  ensures result == start + 1 -- error: the closure writes its copy

spec moved_mut where
  ensures x == 4

spec moved_mut_false where
  ensures x == old(x) -- error: the closure wrote it

spec generic where
  ensures result == x

spec holds_guard where
  ensures true

spec shared_view where
  ensures result == x

spec shared_view_false where
  ensures result == x + 1 -- error: it reads the referent

spec holds_closure where
  requires c <= 1000
  ensures result == c + 1

spec holds_closure_false where
  requires c <= 1000
  ensures result == c -- error: the inner closure adds 1

spec holds_mut_closure where
  requires n0 <= 1000
  ensures result == n0 + 2

spec holds_mut_closure_false where
  requires n0 <= 1000
  ensures result == n0 -- error: the lender sees both writes

spec borrows_mut_closure where
  requires n0 <= 1000
  ensures result == n0 + 2

spec borrows_mut_closure_false where
  requires n0 <= 1000
  ensures result == n0 -- error: the lender sees both writes

spec use_make where
  requires x <= 1000
  ensures result == x + 1

spec use_make_false where
  requires x <= 1000
  ensures result == x -- error: the closure adds 1

spec unused where
  ensures result == x

spec holds_guards where
  ensures true
