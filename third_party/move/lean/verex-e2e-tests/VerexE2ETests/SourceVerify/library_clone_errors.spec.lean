-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec clone_vec where
  ensures result.length == 0 -- error: it is a copy

spec clone_string where
  ensures result.length == 0 -- error: it is a copy

spec clone_option where
  ensures result is None -- error: it is a copy

spec slice_vec where
  ensures result.length == 0 -- error: it is a copy

spec owned where
  ensures result.length == 0 -- error: it is a copy

spec displayed where
  ensures result.length == 0 -- error: it is a copy

spec view_length where
  ensures result == 0 -- error: not of a non-empty string
