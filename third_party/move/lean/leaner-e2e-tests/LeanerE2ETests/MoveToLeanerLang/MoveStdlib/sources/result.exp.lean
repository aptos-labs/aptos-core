-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Provides the `Result<T, E>` type, which allows to represent a success value `T` or an error value `E`. -/
leaner module 0x1::result where
  use 0x1::std::error::invalid_argument

  /--
  Attempt to unwrap value but found error
  -/
  const E_UNWRAP_OK : u64 := 0

  /--
  Attempt to unwrap error but found value
  -/
  const E_UNWRAP_ERR : u64 := 1

  /--
  Represents the result of some computation, either a value `T` or an error `E`.
  -/
  enum Result {T} {E} has Copy, Store where
    | Ok (0 : T)
    | Err (0 : E)

  /--
  Checks whether the result is Ok.
  -/
  public fun is_ok {T} {E}(self : &Result<T, E>) -> Bool := self is Ok

  /--
  Checks whether the result is Err.
  -/
  public fun is_err {T} {E}(self : &Result<T, E>) -> Bool := self is Err

  /--
  Unpacks the `T` of Ok or aborts.
  -/
  public fun unwrap {T} {E}(self : Result<T, E>) -> T :=
    match self with
      | Result<T, E>::Ok { 0 := x } => x
      | _ => abort(invalid_argument(E_UNWRAP_OK))

  /--
  Unpacks the `E` of Err or aborts.
  -/
  public fun unwrap_err {T} {E}(self : Result<T, E>) -> E :=
    match self with
      | Result<T, E>::Err { 0 := x } => x
      | _ => abort(invalid_argument(E_UNWRAP_ERR))
