-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod convert where
  mod num where
    trait impl «std::convert::num::<impl std::convert::TryFrom<u128> for u64>» where
      spec try_from at {num::error::TryFromIntError} where
        ensures (result is Ok) == (_1 <= MAX_U64)
        ensures result is Ok ==> core.data.selectVariants[result::Result<u64, core::num::error::TryFromIntError>, Ok.0](result) == _1

    trait impl «std::convert::num::<impl std::convert::From<u64> for u128>» where
      spec from where
        ensures result == _1
