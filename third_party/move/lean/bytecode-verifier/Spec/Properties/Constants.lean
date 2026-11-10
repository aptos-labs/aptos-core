-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Program

/-!
# Constants (L8)

TRUSTED. Property L8 of one module's text (plan §7, group L): every constant
has a type whose values can be written down as data, and its bytes encode one
value of that type. `LdConst` relies on it: the semantics decodes the constant
on every load and goes `wrong` (`badConstant`) when decoding fails.

Status: complete for every constant type, vectors included. One decoder,
`decodeConstant` in `Spec.Semantics.Values`, serves both the semantics and
this property.
-/

namespace MoveBytecode

/-- A type that may be the type of a constant: booleans, integers of every
width, addresses, and vectors of these (plan §7, L8: "only primitive and
vector types"). The Move book (`constants.md`, "Valid Expressions") limits
source constants to a subset of these; bytecode admits the whole family.

Excluded: `signer`, which is a primitive type but "cannot be created via
literals or instructions — only by the Move VM" (`primitive-types.md`,
"Signer"; D3b); references, which denote locations; function values; struct
and enum values, which only their module's code creates (D3a); and type
parameters, since a constant's type is closed. -/
def SignatureToken.isConstantType : SignatureToken → Bool
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => true
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => true
  | .address => true
  | .vector elem => elem.isConstantType
  | .signer => false
  | .function .. => false
  | .struct _ => false
  | .structInstantiation .. => false
  | .reference _ => false
  | .mutableReference _ => false
  | .typeParameter _ => false

/-- **L8.** Every constant of the module has a constant type
(`isConstantType`), and its bytes decode to exactly one value of that type,
with no bytes left over. The value conforms to the type in every program: a
constant type names no struct, so conformance does not depend on the program's
declarations. -/
def CompiledModule.ConstantsWellFormed (m : CompiledModule) : Prop :=
  ∀ c ∈ m.constantPool.toList,
    c.type.isConstantType ∧
    ∃ t v, RuntimeType.ofToken m c.type = some t ∧ decodeConstant t c.data = some v ∧
      ∀ P : Program, v.conforms P t = true

end MoveBytecode
