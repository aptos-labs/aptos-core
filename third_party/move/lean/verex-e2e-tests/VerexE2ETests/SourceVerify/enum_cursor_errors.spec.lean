-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A cursor over `Option<&mut T>` states what the lender will hold.

spec enum_cursor where
  ensures a == 0
  loop 1 where
    invariant cur is Some ==> final(cur.0) == final(old(cur).0)
    invariant cur is None ==> final(old(cur).0) == 0

spec enum_cursor_false where
  ensures a == 1 -- error: it holds the other value
  loop 1 where
    invariant cur is Some ==> final(cur.0) == final(old(cur).0)
    invariant cur is None ==> final(old(cur).0) == 0

spec enum_cursor_vacuous where
  ensures false -- error: the paths are not vacuous
  loop 1 where
    invariant cur is Some ==> final(cur.0) == final(old(cur).0)
    invariant cur is None ==> final(old(cur).0) == 0

spec local_enum_cursor where
  ensures result == 0
  loop 1 where
    invariant cur is Some ==> final(cur.0) == final(old(cur).0)
    invariant cur is None ==> final(old(cur).0) == 0

spec local_enum_cursor_false where
  ensures result == 1 -- error: it holds the other value
  loop 1 where
    invariant cur is Some ==> final(cur.0) == final(old(cur).0)
    invariant cur is None ==> final(old(cur).0) == 0

spec local_enum_cursor_vacuous where
  ensures false -- error: the paths are not vacuous
  loop 1 where
    invariant cur is Some ==> final(cur.0) == final(old(cur).0)
    invariant cur is None ==> final(old(cur).0) == 0
