-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod fmt where
    mod rt where
      opaque struct Argument {a : lifetime} has Copy where

      impl Argument where
        native fun new_display {l0 : lifetime} {T}(
          _1 : &T
        ) -> Argument<lifetime _>

    opaque struct Arguments {a : lifetime} has Copy where

    impl Arguments where
      native fun from_str {a : lifetime}(
        _1 : &[static] string
      ) -> Arguments<lifetime a>

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

    impl Option where
      native fun expect {T}(_1 : Option<T>, _2 : &string) -> T

      native fun unwrap {T}(_1 : Option<T>) -> T

  mod result where
    enum Result {T} {E} has Copy where
      | Ok (0 : T) = 0
      | Err (0 : E) = 1

    impl Result where
      native fun expect {T} {E}(_1 : Result<T, E>, _2 : &string) -> T

      native fun unwrap {T} {E}(_1 : Result<T, E>) -> T

  mod panicking where
    native fun «panic»(_1 : &string) -> Never

    native fun panic_fmt(_1 : fmt::Arguments<lifetime _>) -> Never

verex namespace panic_macros where
  use core::option::Option
  use core::result::Result

  fun plain_assert(c : u64) -> u64 :=
    if c != 0u64 then c else panic()

  fun message_assert(c : u64, x : u64) -> u64 :=
    if c != 0u64 then c else panic()

  fun literal_message_assert(c : u64) -> u64 :=
    if c != 0u64 then c else panic()

  fun literal_panic(c : u64) -> u64 :=
    if c == 0u64 then panic() else c

  fun format_panic(c : u64, x : u64) -> u64 :=
    if c == 0u64 then panic() else c

  fun unreachable_arm(c : u64) -> u64 :=
    if c > 0u64 then c else panic()

  fun debug_only(c : u64) -> u64 :=
    if c != 0u64 then c else panic()

  fun expect_option(value : Option<u64>) -> u64 :=
    core::option::Option::expect(value, &"a value")

  fun unwrap_option(value : Option<u64>) -> u64 :=
    core::option::Option::unwrap(value)

  fun expect_result(value : Result<u64, u64>) -> u64 :=
    core::result::Result::expect(value, &"a value")

  fun unwrap_result(value : Result<u64, u64>) -> u64 :=
    core::result::Result::unwrap(value)

  fun static_read() -> u64 := do
    let _1 := &7u64
    *_1

  fun const_limit(c : u64) -> u64 :=
    if c < 100u64 then core.prim.checkedSubtractPanic(100u64, c) else panic()

  fun maximum_guard(c : u64) -> u64 :=
    if c < 18446744073709551615u64 then core.prim.checkedAddPanic(c, 1u64)
    else panic()

  fun always() -> u64 := panic()

  fun join_assert(a : Bool, b : u64, c : u64) -> u64 := do
    let mut x := 0u64
    if a then
      if 2u64 == 0u64 then panic()
      else x := core.prim.checkedDividePanic(b, 2u64)
    else
      if 3u64 == 0u64 then panic()
      else x := core.prim.checkedDividePanic(b, 3u64)
    if c != 0u64 then x else panic()

  fun join_panic(a : Bool, b : u64) -> u64 := do
    let mut x := 0u64
    if a then
      if 2u64 == 0u64 then panic()
      else x := core.prim.checkedDividePanic(b, 2u64)
    else
      if 3u64 == 0u64 then panic()
      else x := core.prim.checkedDividePanic(b, 3u64)
    if x > 100u64 then x else panic()

  fun unguarded() -> u64 := panic()
