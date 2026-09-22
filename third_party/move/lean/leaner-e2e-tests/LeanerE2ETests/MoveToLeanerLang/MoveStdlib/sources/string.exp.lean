-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! The `string` module defines the `String` type which represents UTF8 encoded strings. -/
leaner module 0x1::«string» where
  use 0x1::std::option::Option
  use 0x1::std::option::none
  use 0x1::std::option::some
  use 0x1::std::vector

  /--
  An invalid UTF8 encoding.
  -/
  const EINVALID_UTF8 : u64 := 1

  /--
  Index out of range.
  -/
  const EINVALID_INDEX : u64 := 2

  /--
  A `String` holds a sequence of bytes which is guaranteed to be in utf8 format.
  -/
  struct String has Copy, Drop, Store where
    bytes : Vector<u8>

  /--
  Creates a new string from a sequence of bytes. Aborts if the bytes do not represent valid utf8.
  -/
  public fun utf8(bytes : Vector<u8>) -> String := do
    assert!(internal_check_utf8(&bytes), EINVALID_UTF8)
    return new String { bytes }

  spec utf8 where
    pragma opaque
    aborts_if !spec_internal_check_utf8(bytes)
    ensures result == spec_utf8(bytes)

  /--
  Tries to create a new string from a sequence of bytes.
  -/
  public fun try_utf8(bytes : Vector<u8>) -> Option<String> :=
    if internal_check_utf8(&bytes) then some(new String { bytes })
    else none::<String>()

  /--
  Returns a reference to the underlying byte vector.
  -/
  public fun bytes(self : &String) -> &Vector<u8> := &self.bytes

  /--
  Returns the underlying byte vector.
  -/
  public fun into_bytes(self : String) -> Vector<u8> := do
    let String { bytes := bytes } := self
    return bytes

  /--
  Checks whether this string is empty.
  -/
  public fun is_empty(self : &String) -> Bool := self.bytes.is_empty()

  /--
  Returns the length of this string, in bytes.
  -/
  public fun length(self : &String) -> u64 := self.bytes.length

  /--
  Appends a string.
  -/
  public fun append(self : &mut String, r : String) -> Unit := do
    self.bytes.append(r.bytes)

  /--
  Appends bytes which must be in valid utf8 format.
  -/
  public fun append_utf8(self : &mut String, bytes : Vector<u8>) -> Unit := do
    self.append(utf8(bytes))

  /--
  Insert the other string at the byte index in given string. The index must be at a valid utf8 char
  boundary.
  -/
  public fun insert(self : &mut String, at : u64, o : String) -> Unit := do
    let bytes := &self.bytes
    assert!(
      bytes.length >= at && internal_is_char_boundary(bytes, at),
      EINVALID_INDEX
    )
    let l := self.length()
    let mut front := self.sub_string(0, at)
    let end := self.sub_string(at, l)
    front.append(o)
    front.append(end)
    *self := front

  /--
  Returns a sub-string using the given byte indices, where `i` is the first byte position and `j` is the start
  of the first byte not included (or the length of the string). The indices must be at valid utf8 char boundaries,
  guaranteeing that the result is valid utf8.
  -/
  public fun sub_string(self : &String, i : u64, j : u64) -> String := do
    let bytes := &self.bytes
    let l := bytes.length
    assert!(
      j <= l && i <= j && internal_is_char_boundary(bytes, i)
        && internal_is_char_boundary(bytes, j),
      EINVALID_INDEX
    )
    return new String { bytes := internal_sub_string(bytes, i, j) }

  /--
  Computes the index of the first occurrence of a string. Returns `length(s)` if no occurrence found.
  -/
  public fun index_of(self : &String, r : &String) -> u64 :=
    internal_index_of(&self.bytes, &r.bytes)

  -- Native API
  public native fun internal_check_utf8(v : &Vector<u8>) -> Bool

  spec internal_check_utf8 where
    pragma opaque
    aborts_if [abstract] false
    ensures [abstract] result == spec_internal_check_utf8(v)

  native fun internal_is_char_boundary(v : &Vector<u8>, i : u64) -> Bool

  spec internal_is_char_boundary where
    pragma opaque
    aborts_if [abstract] false
    ensures [abstract] result == spec_internal_is_char_boundary(v, i)

  native fun internal_sub_string(
    v : &Vector<u8>, i : u64, j : u64
  ) -> Vector<u8>

  spec internal_sub_string where
    pragma opaque
    aborts_if [abstract] false
    ensures [abstract] result == spec_internal_sub_string(v, i, j)

  native fun internal_index_of(v : &Vector<u8>, r : &Vector<u8>) -> u64

  spec internal_index_of where
    pragma opaque
    aborts_if [abstract] false
    ensures [abstract] result == spec_internal_index_of(v, r)

  spec fun spec_utf8(bytes : Vector<u8>) : String := new String { bytes }

  opaque spec fun spec_internal_check_utf8(v : Vector<u8>) : Bool

  opaque spec fun spec_internal_is_char_boundary(v : Vector<u8>, i : Int) : Bool

  opaque spec fun spec_internal_sub_string(
    v : Vector<u8>, i : Int, j : Int
  ) : Vector<u8>

  opaque spec fun spec_internal_index_of(v : Vector<u8>, r : Vector<u8>) : Int
