-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A cursor states through `final` what the lender will hold.

spec step where
  ensures final(result) == r
  ensures result == old(r)

spec cursor where
  ensures a == 0
  loop 1 where
    invariant final(h.r) == final(old(h).r)

spec cursor_false where
  ensures a == 1 -- error: it holds the other value
  loop 1 where
    invariant final(h.r) == final(old(h).r)

spec cursor_vacuous where
  ensures false -- error: the paths are not vacuous
  loop 1 where
    invariant final(h.r) == final(old(h).r)

spec local_cursor where
  ensures result == 0
  loop 1 where
    invariant final(h.r) == final(old(h).r)

spec local_cursor_false where
  ensures result == 1 -- error: it holds the other value
  loop 1 where
    invariant final(h.r) == final(old(h).r)

spec local_cursor_vacuous where
  ensures false -- error: the paths are not vacuous
  loop 1 where
    invariant final(h.r) == final(old(h).r)

spec final_through_view where
  loop 1 where
    invariant final(h.r) == 0 -- error: a shared view holds no prophecy
