-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR

/-!
# Initial Rust semantic profile

M0 registers the Rust profile and deliberately admits no Rust-specific
extension tag.  Core LIR values can already be validated under this profile;
the MIR mapper will add supported Rust vocabulary only through a reviewed
shared-LIR extension, never through an unchecked string payload.
-/

namespace LeanerIR.Rust

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

def profileName : String := "rust"
def profileVersion : Nat := 2

/-- Pointer width used by synthetic in-tree Rust fixtures. Detached artifacts
record the selected rustc target's width instead. -/
def defaultTargetPointerWidth : Nat := 64

def config : ProfileConfig :=
  { profile := .rust
    name := profileName
    version := profileVersion
    options := #[
      ("panic", "abort"),
      ("unsafe", "reject"),
      ("target_pointer_width", toString defaultTargetPointerWidth)
    ] }

private def unknown (kind : String) (value : ProfileValue) : Array Diagnostic :=
  #[.error "LIR-RUST-TAG" s!"unsupported Rust {kind} tag `{value.tag}`"]

def schema : ProfileSchema where
  profile := .rust
  name := profileName
  version := profileVersion
  checkConfig := fun config =>
    let panic := config.options.find? (·.1 == "panic") |>.map (·.2)
    let unsafeMode := config.options.find? (·.1 == "unsafe") |>.map (·.2)
    let targetPointerWidth := config.options.find? (·.1 == "target_pointer_width")
      |>.bind (·.2.toNat?)
    (if panic == some "abort" then #[] else
      #[.error "LIR-RUST-PANIC" "the initial Rust profile requires `panic=abort`"]) ++
    (if unsafeMode == some "reject" then #[] else
      #[.error "LIR-RUST-UNSAFE" "the initial Rust profile accepts only safe Rust"]) ++
    (if targetPointerWidth.any supportedTargetPointerWidth then #[] else
      #[.error "LIR-RUST-TARGET-WIDTH"
        "the Rust profile requires `target_pointer_width` to be 16, 32, or 64"])
  checkReference := fun _ => #[]
  checkType := unknown "type"
  checkOperation := unknown "operation"
  checkSurface := unknown "surface"
  checkProperty := unknown "property"

/-- M0 semantic policy for core-only Rust-profile fixtures. Rust-specific
operations are not silently accepted until the profile inventory is added. -/
def semantics : SemanticProfile where
  profile := .rust
  name := profileName
  version := profileVersion
  classify := fun _ _ => none
  rollbackThrow := fun _ => false

/-- Validate an isolated Rust-profile raw unit. Multi-profile clients build a
registry explicitly. -/
def validate (unit : RawUnit) : Except (Array Diagnostic) ValidatedUnit :=
  LeanerIR.Validation.validate #[schema] unit

/-- Decode the shared RawUnit JSON exchange format and apply the Rust profile's
ordinary shared validation path. -/
def decodeAndValidate (text : String) : Except (Array Diagnostic) ValidatedUnit := do
  let unit ← match Import.decodeJson text with
    | .ok unit => .ok unit
    | .error message => .error #[.error "LIR-JSON" message]
  validate unit

end LeanerIR.Rust
