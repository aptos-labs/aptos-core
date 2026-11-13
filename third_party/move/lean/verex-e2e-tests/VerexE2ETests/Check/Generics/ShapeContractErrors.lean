-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Contracts at a shape a library may not state

Two shapes one instantiation matches are an error at the later item, wherever
the instantiation may be. A shape has one type per type binder and belongs to a
function without a body in the Rust profile.
-/

namespace VerexSL.Tests.Check.Generics.ShapeContractErrors

verex namespace overlapping_shapes where
  enum Opt {T} has Copy where
    | None
    | Some (value : T)

  native fun same_variable {A} {B}(_1 : A, _2 : B) -> B
  spec same_variable at {A} {u32} where
    ensures true
  spec same_variable at {u32} {A} where
    ensures true

  native fun identical {T}(_1 : T) -> T
  spec identical at {u32} where
    ensures true
  spec identical at {u32} where
    ensures true

  native fun through_twice {A} {B}(_1 : A, _2 : B) -> B
  spec through_twice at {A} {A} where
    ensures true
  spec through_twice at {u32} {B} where
    ensures true

  -- The verifier's types do not tell these apart at `{u32} {usize}` and `{u32} {u64}`,
  -- and they differ in more than the pointer-width integer.
  native fun same_width {T} {U}(_1 : T, _2 : U) -> T
  spec same_width at {T} {usize} where
    ensures true
  spec same_width at {u32} {u64} where
    ensures true

  native fun with_plain {T}(_1 : T) -> T
  spec with_plain where
    ensures true
  spec with_plain at {u32} where
    ensures true

  native fun plain_later {T}(_1 : T) -> T
  spec plain_later at {u32} where
    ensures true
  spec plain_later where
    ensures true

verex namespace disjoint_shapes where
  enum Opt {T} has Copy where
    | None
    | Some (value : T)

  native fun apart {A} {B}(_1 : A, _2 : B) -> B
  spec apart at {A} {A} where
    ensures true
  spec apart at {u32} {Bool} where
    ensures true
  -- A variable cannot be its own option: the shapes are apart.
  spec apart at {Opt<B>} {B} where
    ensures true

  native fun other_width {T}(_1 : T) -> T
  spec other_width at {usize} where
    ensures true
  spec other_width at {u32} where
    ensures true

verex namespace misspelled_type where
  native fun misspelled {T}(_1 : T) -> T
  spec misspelled at {bool} where -- error: `bool` is neither a type nor a binder
    ensures true

verex namespace wrong_arity where
  native fun two {A} {B}(_1 : A, _2 : B) -> B
  spec two at {u32} where
    ensures true

verex namespace on_a_body where
  fun has_body {T}(value : T) -> T := value
  spec has_body at {u32} where
    ensures result == value

verex module 0x48::in_move where
  native fun generic {T}(value : T) -> T
  spec generic at {u64} where
    ensures true

verex namespace on_a_type where
  struct Plain {T} where
    value : T
  spec Plain at {u32} where
    ensures true

end VerexSL.Tests.Check.Generics.ShapeContractErrors
