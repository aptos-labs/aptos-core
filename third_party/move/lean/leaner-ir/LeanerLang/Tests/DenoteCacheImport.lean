-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Tests.DenoteCache

/-! Completed verification remains reusable after serialization and import. -/

namespace LeanerLang.Tests.DenoteCache

set_option Elab.async false

#leaner_verify 0x42::denote_cache::valid
#leaner_require_native 0x42::denote_cache::valid

end LeanerLang.Tests.DenoteCache
