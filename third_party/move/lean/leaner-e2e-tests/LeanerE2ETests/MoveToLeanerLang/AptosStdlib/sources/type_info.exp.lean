-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x1::type_info where
  use 0x1::std::bcs::serialize
  use 0x1::std::bcs::serialized_size
  use 0x1::std::error::invalid_state
  use 0x1::std::features::aptos_stdlib_chain_id_enabled
  use 0x1::std::features::spec_is_enabled
  use 0x1::std::«string»::String

  --
  -- Error codes
  --
  const E_NATIVE_FUN_NOT_AVAILABLE : u64 := 1

  --
  -- Structs
  --
  struct TypeInfo has Copy, Drop, Store where
    account_address : Address
    module_name : Vector<u8>
    struct_name : Vector<u8>

  --
  -- Public functions
  --
  public fun account_address(self : &TypeInfo) -> Address :=
    self.account_address

  public fun module_name(self : &TypeInfo) -> Vector<u8> := self.module_name

  public fun struct_name(self : &TypeInfo) -> Vector<u8> := self.struct_name

  /--
  Returns the current chain ID, mirroring what `aptos_framework::chain_id::get()` would return, except in `#[test]`
  functions, where this will always return `4u8` as the chain ID, whereas `aptos_framework::chain_id::get()` will
  return whichever ID was passed to `aptos_framework::chain_id::initialize_for_test()`.
  -/
  public fun chain_id() -> u8 := do
    if !aptos_stdlib_chain_id_enabled() then
      abort(invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    return chain_id_internal()

  spec chain_id where
    aborts_if !spec_is_enabled(4)
    ensures result == spec_chain_id_internal()

  /--
  Return the `TypeInfo` struct containing  for the type `T`.
  -/
  public native fun type_of {T}() -> TypeInfo

  /--
  Return the human readable string for the type, including the address, module name, and any type arguments.
  Example: 0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>
  Or: 0x1::table::Table<0x1::string::String, 0x1::string::String>
  -/
  public native fun type_name {T}() -> String

  native fun chain_id_internal() -> u8

  spec chain_id_internal where
    pragma opaque
    aborts_if false
    ensures result == spec_chain_id_internal()

  /--
  Return the BCS size, in bytes, of value at `val_ref`.

  See the [BCS spec](https://github.com/diem/bcs)

  See `test_size_of_val()` for an analysis of common types and
  nesting patterns, as well as `test_size_of_val_vectors()` for an
  analysis of vector size dynamism.
  -/
  public fun size_of_val {T}(val_ref : &T) -> u64 := serialized_size(val_ref)

  spec size_of_val where
    aborts_if false
    ensures result == spec_size_of_val(val_ref)

  -- We need to enable the feature in order for the native call to be allowed.
  -- The testing environment chain ID is 4u8.
  -- vector
  -- struct
  fun verify_type_of() -> Unit := do
    let type_info := type_of::<TypeInfo>()
    let account_address := type_info.account_address()
    let module_name := type_info.module_name()
    let struct_name := type_info.struct_name()
    spec do
      assert account_address == @0x1
      assert module_name == b"type_info"
      assert struct_name == b"TypeInfo"

  fun verify_type_of_generic {T}() -> Unit := do
    let type_info := type_of::<T>()
    let account_address := type_info.account_address()
    let module_name := type_info.module_name()
    let struct_name := type_info.struct_name()
    spec do
      assert account_address == type_of::<T>().account_address
      assert module_name == type_of::<T>().module_name
      assert struct_name == type_of::<T>().struct_name

  spec verify_type_of_generic where
    aborts_if !spec_is_struct::<T>()

  -- Bool takes 1 byte.
  -- u8 takes 1 byte.
  -- u64 takes 8 bytes.
  -- u128 takes 16 bytes.
  -- Address is a u256.
  -- Signer is an address.
  -- Assert custom type without fields has size 1.
  -- Declare a simple struct with a 1-byte field.
  -- Assert size is indicated as 1 byte.
  -- Declare a complex struct with another nested inside.
  -- Assert size is bytewise sum of components.
  -- Declare a struct with two boolean values.
  -- Assert size is two bytes.
  -- Declare an empty vector of element type u64.
  -- Declare an empty vector of element type u128.
  -- Assert size is 1 byte regardless of underlying element type.
  -- Assert size is 1 byte regardless of underlying element type.
  -- Declare a bool in a vector.
  -- Push back another bool.
  -- Assert size is 3 bytes (1 per element, 1 for base vector).
  -- Get a some option, which is implemented as a vector.
  -- Assert size is 9 bytes (8 per element, 1 for base vector).
  -- Remove the value inside.
  -- Assert size reduces to 1 byte.
  -- Declare vector base sizes.
  -- A base size of 1 applies for 127 or less elements.
  -- (2 ^ 7) ^ 1 - 1.
  -- A base size of 2 applies for 128 < n <= 16384 elements.
  -- (2 ^ 7) ^ 2 - 1.
  -- Declare empty vector.
  -- Declare a null element.
  -- Get element size.
  -- Vector size is 1 byte when length is 0.
  -- Declare loop counter.
  -- Iterate until first cutoff:
  -- Add an element.
  -- Increment counter.
  -- Vector base size is still 1 byte.
  -- Add another element, exceeding the cutoff.
  -- Increment counter.
  -- Vector base size is now 2 bytes.
  -- Iterate until second cutoff:
  -- Add an element.
  -- Increment counter.
  -- Vector base size is still 2 bytes.
  -- Add another element, exceeding the cutoff.
  -- Increment counter.
  -- Vector base size is now 3 bytes.
  -- Repeat for custom struct.
  -- Declare a null element.
  -- Get element size.
  -- Vector size is 1 byte when length is 0.
  -- Re-initialize loop counter.
  -- Iterate until first cutoff:
  -- Add an element.
  -- Increment counter.
  -- Vector base size is still 1 byte.
  -- Add another element, exceeding the cutoff.
  -- Increment counter.
  -- Vector base size is now 2 bytes.
  -- Iterate until second cutoff:
  -- Add an element.
  -- Increment counter.
  -- Vector base size is still 2 bytes.
  -- Add another element, exceeding the cutoff.
  -- Increment counter.
  -- Vector base size is now 3 bytes.
  opaque spec fun spec_is_struct {T}() : Bool

  -- Move Prover natively supports this function.
  -- This function will abort if `T` is not a struct type.
  -- Move Prover natively supports this function.
  -- The chain ID is modeled as an uninterpreted function.
  opaque spec fun spec_chain_id_internal() : Int

  spec fun spec_size_of_val {T}(val_ref : T) : Int := serialize(val_ref).length
