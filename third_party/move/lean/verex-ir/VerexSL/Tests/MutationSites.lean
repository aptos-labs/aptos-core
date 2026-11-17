-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
The sites a function owes a mutation end at, as `mutationConditions`
selects them: the generator observes one mutation both where the loan dies
and at the write into the lending local, so the same pair recurs, while the
conditions of distinct sites are distinct and two locals at one site are
two conditions where the marker's step reads one.
-/

namespace VerexSL.Tests.MutationSites
open VerexSL.Verify

-- The same (site, local) observed again is the one condition of that site.
/-- info: Except.ok #[(4, 1)] -/
#guard_msgs in
#eval mutationSitesOwed #[(4, 1), (4, 1), (4, 1)]

-- Distinct sites are both kept, in the order they were observed.
/-- info: Except.ok #[(9, 2), (4, 1)] -/
#guard_msgs in
#eval mutationSitesOwed #[(9, 2), (4, 1), (9, 2)]

-- Two locals at one site: refused, naming both, never the first alone.
/--
info: Except.error "internal: the mutation ending at the site 4 is owed of two locals, 1 and 2: its marker checks one condition"
-/
#guard_msgs in
#eval mutationSitesOwed #[(4, 1), (4, 2)]

end VerexSL.Tests.MutationSites
