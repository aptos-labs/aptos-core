-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec clone_vec where
  ensures result == values

spec clone_string where
  ensures result == text

spec clone_option where
  ensures result == value

spec slice_vec where
  ensures result == values

spec owned where
  ensures result == text

spec displayed where
  ensures result == text

spec view_length where
  ensures result == text.length
