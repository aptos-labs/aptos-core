-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod str where
  trait impl «std::str::<impl std::borrow::ToOwned for str>» where
    spec to_owned where
      ensures result == _1
