-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace «abort» where
  fun abort_now() -> Never := panic()
