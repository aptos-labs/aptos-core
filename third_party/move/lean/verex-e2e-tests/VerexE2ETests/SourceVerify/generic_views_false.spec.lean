-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec viewing_reference where
  ensures result == v -- error: the write through the result reaches the lender

spec nested_reference where
  ensures result == v -- error: the write through the result reaches the lender

spec stored_reference where
  ensures result == v -- error: the write through the result reaches the lender

spec enum_reference where
  ensures result == v -- error: the write through the result reaches the lender

spec passed_reference where
  ensures result == v -- error: the write through the result reaches the lender

spec holder_reference where
  ensures result == v -- error: the write through the holder reaches the lender

spec viewing_value where
  ensures result == v + 1 -- error: the callee returns its argument

spec observed_at_site where
  ensures result == v + 1 -- error: the write stores what the view observed

spec observed_stored_at_site where
  ensures result == v + 1 -- error: the write stores what the view observed

spec observed_enum_at_site where
  ensures result == v + 1 -- error: the write stores what the view observed

spec peek_site where
  ensures result == v -- error: the write through the reference reaches the lender

spec take_site where
  ensures result == v -- error: the write through the reference reaches the lender

spec take_seen_site where
  ensures result == v -- error: the write through the reference reaches the lender

spec made_site where
  ensures result == v + 1 -- error: the write stores what the view observed

spec deep_site where
  ensures result == v -- error: the write through the result reaches the lender

spec body_local_view where
  ensures result == v + 1 -- error: the write stores what the view observed

spec body_local_through where
  ensures result == v + 1 -- error: the write stores what the view observed
