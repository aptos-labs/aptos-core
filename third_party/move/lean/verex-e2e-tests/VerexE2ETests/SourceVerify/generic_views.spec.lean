-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec viewing_reference where
  ensures result == 7

spec nested_reference where
  ensures result == 7

spec stored_reference where
  ensures result == 7

spec enum_reference where
  ensures result == 7

spec passed_reference where
  ensures result == 7

spec holder_reference where
  ensures result == 7

spec viewing_value where
  ensures result == v

spec observed_at_site where
  ensures result == v

spec observed_stored_at_site where
  ensures result == v

spec observed_enum_at_site where
  ensures result == v

spec peek_site where
  ensures result == 7

spec take_site where
  ensures result == 7

spec take_seen_site where
  ensures result == 7

spec made_site where
  ensures result == v

spec deep_site where
  ensures result == 7

spec body_local_view where
  ensures result == v

spec body_local_through where
  ensures result == v
