-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Tests.DenoteCache

/-! Completed verification remains reusable after serialization and import. -/

namespace LeanerLang.Tests.DenoteCache

set_option Elab.async false

verify 0x42::denote_cache::valid

end LeanerLang.Tests.DenoteCache
