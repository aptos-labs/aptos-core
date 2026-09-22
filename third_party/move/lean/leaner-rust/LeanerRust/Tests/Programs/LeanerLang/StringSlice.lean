-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace string_slice where
  fun identity(value : &string) -> &string := value

  fun identity_mut(value : &mut string) -> &mut string := value
