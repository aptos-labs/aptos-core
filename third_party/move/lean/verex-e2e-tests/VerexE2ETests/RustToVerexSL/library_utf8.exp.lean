-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

  mod result where
    enum Result {T} {E} has Copy where
      | Ok (0 : T) = 0
      | Err (0 : E) = 1

    impl Result where
      native fun is_ok {T} {E}(_1 : &Result<T, E>) -> Bool

  mod str where
    mod error where
      opaque struct Utf8Error has Copy where

    mod converts where
      native fun from_utf8(
        _1 : &Slice<u8>
      ) -> result::Result<&string, error::Utf8Error>

  mod primitive where
    impl str where
      native fun as_bytes(_1 : &string) -> &Slice<u8>

verex namespace library_utf8 where
  use core::option::Option
  use core::primitive::str::as_bytes
  use core::result::Result
  use core::result::Result::is_ok
  use core::str::converts::from_utf8
  use core::str::error::Utf8Error

  fun roundtrip(text : &string) -> Bool := is_ok(&from_utf8(as_bytes(text)))

  fun success_bytes(bytes : &Slice<u8>) -> Option<&Slice<u8> > := do
    let _2 := from_utf8(bytes)
    let mut _0 : Option<&Slice<u8> >
    match core.readDiscriminant[isize](_2) with
      | 0 => do
        let text := copy((downcast _2 as Result::Ok).0)
        _0 := new Option<&Slice<u8> >::Some { 0 := as_bytes(text) }
      | 1 => _0 := new Option<&Slice<u8> >::None {}
    _0

  fun success_length(bytes : &Slice<u8>) -> Bool := do
    let _2 := from_utf8(bytes)
    let mut _0 : Bool
    match core.readDiscriminant[isize](_2) with
      | 0 => do
        let text := copy((downcast _2 as Result::Ok).0)
        _0 := text.length == bytes.length
      | 1 => _0 := true
    _0

  fun decode(bytes : &Slice<u8>) -> Result<&string, Utf8Error> :=
    from_utf8(bytes)
