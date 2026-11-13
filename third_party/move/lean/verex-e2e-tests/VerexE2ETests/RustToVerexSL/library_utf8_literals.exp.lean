-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod result where
    enum Result {T} {E} has Copy where
      | Ok (0 : T) = 0
      | Err (0 : E) = 1

    impl Result where
      native fun is_err {T} {E}(_1 : &Result<T, E>) -> Bool

      native fun is_ok {T} {E}(_1 : &Result<T, E>) -> Bool

  mod str where
    mod error where
      opaque struct Utf8Error has Copy where

    mod converts where
      native fun from_utf8(
        _1 : &Slice<u8>
      ) -> result::Result<&string, error::Utf8Error>

verex namespace library_utf8_literals where
  use core::result::Result
  use core::result::Result::is_err
  use core::result::Result::is_ok
  use core::str::converts::from_utf8
  use core::str::error::Utf8Error

  fun valid_empty() -> Bool := do
    let _1 : Vector<u8, const 0> := #[]
    is_ok(&from_utf8(&(unsize _1)))

  fun valid_ascii() -> Bool := do
    let bytes : Vector<u8, const 3> := #[65u8, 0u8, 127u8]
    is_ok(&from_utf8(&(unsize bytes)))

  fun valid_two_byte() -> Bool := do
    let bytes : Vector<u8, const 2> := #[195u8, 169u8]
    is_ok(&from_utf8(&(unsize bytes)))

  fun valid_three_byte() -> Bool := do
    let bytes : Vector<u8, const 3> := #[228u8, 184u8, 173u8]
    is_ok(&from_utf8(&(unsize bytes)))

  fun valid_four_byte() -> Bool := do
    let bytes : Vector<u8, const 4> := #[240u8, 159u8, 166u8, 128u8]
    is_ok(&from_utf8(&(unsize bytes)))

  fun invalid_ff() -> Bool := do
    let bytes : Vector<u8, const 1> := #[255u8]
    is_err(&from_utf8(&(unsize bytes)))

  fun invalid_continuation() -> Bool := do
    let bytes : Vector<u8, const 1> := #[128u8]
    is_err(&from_utf8(&(unsize bytes)))

  fun invalid_overlong() -> Bool := do
    let bytes : Vector<u8, const 2> := #[192u8, 175u8]
    is_err(&from_utf8(&(unsize bytes)))

  fun invalid_truncated() -> Bool := do
    let bytes : Vector<u8, const 2> := #[226u8, 130u8]
    is_err(&from_utf8(&(unsize bytes)))

  fun invalid_surrogate() -> Bool := do
    let bytes : Vector<u8, const 3> := #[237u8, 160u8, 128u8]
    is_err(&from_utf8(&(unsize bytes)))

  fun invalid_beyond_unicode() -> Bool := do
    let bytes : Vector<u8, const 4> := #[244u8, 144u8, 128u8, 128u8]
    is_err(&from_utf8(&(unsize bytes)))
