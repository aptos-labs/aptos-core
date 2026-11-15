-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types
import VerexIR.Validation.Check

namespace VerexIR.Tests.ClosureSized
open VerexIR VerexIR.Import VerexIR.Validation VerexIR.Proofs.Denote

private def schema : ProfileSchema := { profile := .rust, name := "closure-sized" }

private def fixture : RawUnit where
  tables := {
    locations := #[{}]
    origins := #[{kind := .rustMir, location := ⟨0⟩}]
    alignments := #[{source := ⟨0⟩, trust := .checked, description := "Sized closure"}]
    namespaces := #[{segments := #["ClosureSized"]}]
    names := #[{namespaceId := ⟨0⟩, name := "constant"}]
    types := #[.bool, .typeParameter 0, .slice ⟨0⟩, .vector ⟨0⟩ none]
  }
  profiles := #[{profile := .rust, name := "closure-sized"}]
  namespaces := #[{
    loc := ⟨0⟩, identity := ⟨0⟩, profile := some .rust
    expressions := #[{loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.bool true)}]
    functions := #[{
      loc := ⟨0⟩, name := ⟨0⟩, profile := .rust
      signature := {
        generics := #[{loc := ⟨0⟩, name := "T", kind := .typeArg}]
        results := #[{loc := ⟨0⟩, typeId := ⟨0⟩}]
      }
      body := .structured ⟨0⟩
      origin := ⟨0⟩, alignment := ⟨0⟩
    }]
  }]

private def accepted (actual : Nat) : Option Bool := do
  let unit ← (validate #[schema] fixture).toOption
  return closureFaithful unit ⟨⟨0⟩, ⟨0⟩⟩ #[(⟨1⟩, ⟨actual⟩)]

-- Both actuals are reference-free and have native sequence carriers. The
-- closure's generic binder still requires Sized even when its body ignores T.
#eval show IO Unit from do
  unless accepted 2 == some false do throw (IO.userError "unsized Slice closure actual accepted")
  unless accepted 3 == some true do throw (IO.userError "sized Vector closure actual rejected")

end VerexIR.Tests.ClosureSized
