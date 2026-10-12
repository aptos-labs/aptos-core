-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The proof of `product` does not establish its clause: `omega` decides
-- linear arithmetic only. `square_of_sum` and `twice` have no proof here.
verify product by
  leaner_obligations
  all_goals (first | leaner_decide | (leaner_prepare; all_goals try leaner_decide))
  omega
