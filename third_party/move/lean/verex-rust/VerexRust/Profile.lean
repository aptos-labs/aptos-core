-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR

/-!
# Initial Rust semantic profile

M0 registers the Rust profile and admits four Rust-specific tags: the property
`struct.opaque` of a dependency's type whose shape the crate does not see, the
property `struct.destructor` of a type whose drop runs a destructor, the
property `function.dependency` of a dependency's function, declared without its
body, and the property `function.opaque` of one the crate cannot state.
Core VIR values can already be validated under this profile; the MIR mapper
adds supported Rust vocabulary only through a reviewed shared-VIR extension,
never through an unchecked string payload.
-/

namespace VerexIR.Rust

open VerexIR
open VerexIR.Import
open VerexIR.Validation

def profileName : String := "rust"
def profileVersion : Nat := 5

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
  #[.error "VIR-RUST-TAG" s!"unsupported Rust {kind} tag `{value.tag}`"]

/-- See `VerexIR.rustOpaqueTag`. -/
def opaqueTag : String := VerexIR.rustOpaqueTag

/-- See `VerexIR.rustDependencyTag`. -/
def dependencyTag : String := VerexIR.rustDependencyTag

/-- See `VerexIR.rustOpaqueFunctionTag`. -/
def opaqueFunctionTag : String := VerexIR.rustOpaqueFunctionTag

/-- See `VerexIR.rustClosureTag`. -/
def closureTag : String := VerexIR.rustClosureTag

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
      #[.error "VIR-RUST-PANIC" "the initial Rust profile requires `panic=abort`"]) ++
    (if unsafeMode == some "reject" then #[] else
      #[.error "VIR-RUST-UNSAFE" "the initial Rust profile accepts only safe Rust"]) ++
    (if targetPointerWidth.any supportedTargetPointerWidth then #[] else
      #[.error "VIR-RUST-TARGET-WIDTH"
        "the Rust profile requires `target_pointer_width` to be 16, 32, or 64"])
  checkReference := fun _ => #[]
  checkType := unknown "type"
  checkOperation := fun value =>
    if value.tag == "borrow.two_phase" || value.tag == "borrow.reference_copy" then #[]
    else unknown "operation" value
  checkSurface := unknown "surface"
  checkProperty := fun value =>
    if value.tag == closureTag || value.tag == VerexIR.rustRefusedTag ||
        (value.tag == opaqueTag || value.tag == destructorTag || value.tag == dependencyTag ||
          value.tag == VerexIR.rustShapeOnlyTag ||
          value.tag == opaqueFunctionTag) && value.payload.isEmpty then #[]
    else unknown "property" value
  opaqueNominal := fun properties => properties.any (·.tag == opaqueTag)
  bodiless := fun properties => some (properties.any (·.tag == dependencyTag))

/-- M0 semantic policy for core-only Rust-profile fixtures. Rust-specific
operations are not silently accepted until the profile inventory is added. -/
def semantics : SemanticProfile where
  profile := .rust
  name := profileName
  version := profileVersion
  classify := fun site value => match site with
    | .property =>
        if value.tag == opaqueTag || value.tag == destructorTag || value.tag == dependencyTag ||
            value.tag == opaqueFunctionTag || value.tag == closureTag ||
            value.tag == VerexIR.rustRefusedTag || value.tag == VerexIR.rustShapeOnlyTag then
          some .frontendOnly
        else none
    | _ => none
  rollbackThrow := fun _ => false

/-- Validate an isolated Rust-profile raw unit. Multi-profile clients build a
registry explicitly. -/
def validate (unit : RawUnit) : Except (Array Diagnostic) ValidatedUnit :=
  VerexIR.Validation.validate #[schema] unit

/-- Decode the shared RawUnit JSON exchange format and apply the Rust profile's
ordinary shared validation path. -/
def decodeAndValidate (text : String) : Except (Array Diagnostic) ValidatedUnit := do
  let unit ← match Import.decodeJson text with
    | .ok unit => .ok unit
    | .error message => .error #[.error "VIR-JSON" message]
  validate unit

end VerexIR.Rust
