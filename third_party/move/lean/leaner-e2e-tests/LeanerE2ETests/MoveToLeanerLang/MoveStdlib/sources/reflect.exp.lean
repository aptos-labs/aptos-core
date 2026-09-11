-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Functionality for reflection in Move. -/
leaner module 0x1::reflect where
  use 0x1::std::error::invalid_state
  use 0x1::std::features::is_function_reflection_enabled
  use 0x1::std::result::Result
  use 0x1::std::«string»::String

  /--
  This error indicates that the reflection feature is not enabled.
  -/
  const E_FEATURE_NOT_ENABLED : u64 := 0

  /--
  Resolves a function specified by address and symbolic name, with expected type, into a typed function value.

  Example usage:

  ```
     let fn : |address|u64 has store = reflect::resolve(@somewhere, utf8(b"mod"), utf8(b"fn")).unwrap();
     assert!(fn(my_addr) == some_value)
  ```

  See `ReflectionError` for the possible errors which can result. On successful resolution,
  a function value is returned which can be safely used in future executions as indicated by the requested
  type.

  In order to be accessible, the resolved function must be public. This prevents reflection to
  work around the languages modular encapsulation guarantees.

  A small set of framework functions are additionally forbidden from being resolved (the call
  returns `FunctionNotAccessible`), because their rules are enforced by the bytecode verifier at
  the call site and cannot be upheld for a dynamically-resolved function value. Currently this
  is only `0x1::event::emit`.

  The resolved function can be generic, in which case the instantiation must be inferrible
  from the provided `FuncType`. For example, `public fun foo<T>(T)`, with `FunType = |u64|`,
  `T = u64` can be derived. If not all type parameters can be inferred, an error will be
  produced.
  -/
  public fun resolve {FuncType}(
    addr : Address, module_name : &String, func_name : &String
  ) -> Result<FuncType, ReflectionError> := do
    assert!(
      is_function_reflection_enabled(), invalid_state(
        E_FEATURE_NOT_ENABLED
      )
    )
    return native_resolve::<FuncType>(addr, module_name, func_name)

  spec resolve where
    pragma opaque
    pragma verify = false

  -- Make uninterpreted
  /--
  Represents errors returned by the reflection API.
  TODO: make this public once language version 2.4 is available
  -/
  enum ReflectionError has Copy, Drop, Store where
    | InvalidIdentifier
    | FunctionNotFound
    | FunctionNotAccessible
    | FunctionIncompatibleType
    | FunctionNotInstantiated

  /--
  Returns numerical code associated with error.
  -/
  public fun error_code(self : ReflectionError) -> u64 :=
    if self is InvalidIdentifier then 0
    else
      if self is FunctionNotFound then 1
      else
        if self is FunctionNotAccessible then 2
        else
          if self is FunctionIncompatibleType then 3 else 4

  native fun native_resolve {FuncType}(
    addr : Address, module_name : &String, func_name : &String
  ) -> Result<FuncType, ReflectionError>
