-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.TableLoans

namespace LeanerIR.Tests.TableLoans
open SemanticOperations

private def slot : GlobalKey := ⟨⟨0⟩, ⟨0⟩, .address "table-a"⟩
private def contents (value : Int) : RuntimeValue :=
  .vector #[.tuple #[.integer 7, .integer value], .tuple #[.integer 8, .integer 17]]

private def initial : RuntimeState := {
  globals := ⟨#[⟨slot, .integer 44⟩]⟩
  tables := {
    contents := ⟨#[⟨slot, contents 5⟩]⟩
    allocated := #["destroyed-table", "table-a"] }
  nextLoan := 42 }

-- Actual key lookup and registration, followed by the same frame export
-- operation that reconciles ordinary mutable references.
private def writeEntry? (state : RuntimeState) (key value : RuntimeValue) : Option RuntimeState := do
  let (lent, .borrow loan _) ← borrowTableEntry? state slot key | none
  some (exportFrameLoans { locals := #[some (.borrow loan value)] } lent)

#guard readTableEntry? initial slot (.integer 7) == some (.integer 5)
#guard readTableEntry? initial slot (.integer 8) == some (.integer 17)
#guard borrowTableEntry? initial slot (.integer 99) == none
#guard borrowTableEntryAt? initial slot 2 == none
#guard borrowTableEntry? {} slot (.integer 7) == none

#guard writeEntry? initial (.integer 7) (.integer 9) == some
  { initial with tables.contents := ⟨#[⟨slot, contents 9⟩]⟩, nextLoan := 43 }

#guard (do
  let first ← writeEntry? initial (.integer 7) (.integer 9)
  writeEntry? first (.integer 7) (.integer 13)) == some
    { initial with tables.contents := ⟨#[⟨slot, contents 13⟩]⟩, nextLoan := 44 }

#guard (borrowTableEntry? initial slot (.integer 7)).map (·.1.storageLoans) ==
  some [(42, .table slot)]

-- A malformed tuple must not match a key, even when its first field matches.
#guard tableEntryIndex? (.vector #[.tuple #[.integer 7, .integer 5, .integer 6]])
  (.integer 7) == none

end LeanerIR.Tests.TableLoans
