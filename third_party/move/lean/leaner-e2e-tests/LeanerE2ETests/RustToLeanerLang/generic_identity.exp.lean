-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace generic_identity where
  fun identity {T}(value : T) -> T := value

  fun choose {T}(value : &T) -> &T := value

  fun round_trip {T}(value : T) -> T := identity(value)
