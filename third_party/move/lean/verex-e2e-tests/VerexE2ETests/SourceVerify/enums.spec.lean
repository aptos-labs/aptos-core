-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec make_point where
  ensures result is Point
  ensures result.0 == x

spec size where
  ensures result <= 2
  ensures shape is Empty ==> result == 0

spec first where
  ensures shape is Empty ==> result == fallback
  ensures !(shape is Empty) ==> result == shape.0

spec sum where
  ensures shape is Empty ==> result == 0

spec read where
  ensures result == borrowed.value

spec use_first where
  ensures result == value

spec use_second where
  ensures result == flag
