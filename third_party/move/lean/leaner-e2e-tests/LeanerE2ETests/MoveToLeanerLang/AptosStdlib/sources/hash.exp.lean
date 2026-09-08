-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
Cryptographic hashes:
- Keccak-256: see https://keccak.team/keccak.html

In addition, SHA2-256 and SHA3-256 are available in `std::hash`. Note that SHA3-256 is a variant of Keccak: it is
NOT the same as Keccak-256.

Non-cryptograhic hashes:
- SipHash: an add-rotate-xor (ARX) based family of pseudorandom functions created by Jean-Philippe Aumasson and Daniel J. Bernstein in 2012
-/
leaner module 0x1::aptos_hash where
  use 0x1::std::bcs::to_bytes
  use 0x1::std::error::invalid_state
  use 0x1::std::features::blake2b_256_enabled
  use 0x1::std::features::sha_512_and_ripemd_160_enabled

  --
  -- Constants
  --
  /--
  A newly-added native function is not yet enabled.
  -/
  const E_NATIVE_FUN_NOT_AVAILABLE : u64 := 1

  --
  -- Functions
  --
  /--
  Returns the (non-cryptographic) SipHash of `bytes`. See https://en.wikipedia.org/wiki/SipHash
  -/
  public native fun sip_hash(bytes : Vector<u8>) -> u64

  /--
  Returns the (non-cryptographic) SipHash of the BCS serialization of `v`. See https://en.wikipedia.org/wiki/SipHash
  -/
  public fun sip_hash_from_value {MoveValue}(v : &MoveValue) -> u64 := do
    let bytes := to_bytes(v)
    return sip_hash(bytes)

  /--
  Returns the Keccak-256 hash of `bytes`.
  -/
  public native fun keccak256(bytes : Vector<u8>) -> Vector<u8>

  /--
  Returns the SHA2-512 hash of `bytes`.
  -/
  public fun sha2_512(bytes : Vector<u8>) -> Vector<u8> := do
    if !sha_512_and_ripemd_160_enabled() then
      abort(invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    return sha2_512_internal(bytes)

  public fun sha2_512_value {T}(val : &T) -> Vector<u8> := do
    let bytes := to_bytes(val)
    return sha2_512(bytes)

  /--
  Returns the SHA3-512 hash of `bytes`.
  -/
  public fun sha3_512(bytes : Vector<u8>) -> Vector<u8> := do
    if !sha_512_and_ripemd_160_enabled() then
      abort(invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    return sha3_512_internal(bytes)

  /--
  Returns the RIPEMD-160 hash of `bytes`.

  WARNING: Only 80-bit security is provided by this function. This means an adversary who can compute roughly 2^80
  hashes will, with high probability, find a collision x_1 != x_2 such that RIPEMD-160(x_1) = RIPEMD-160(x_2).
  -/
  public fun ripemd160(bytes : Vector<u8>) -> Vector<u8> := do
    if !sha_512_and_ripemd_160_enabled() then
      abort(invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    return ripemd160_internal(bytes)

  /--
  Returns the BLAKE2B-256 hash of `bytes`.
  -/
  public fun blake2b_256(bytes : Vector<u8>) -> Vector<u8> := do
    if !blake2b_256_enabled() then
      abort(invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    return blake2b_256_internal(bytes)

  --
  -- Private native functions
  --
  /--
  Returns the SHA2-512 hash of `bytes`.
  -/
  native fun sha2_512_internal(bytes : Vector<u8>) -> Vector<u8>

  /--
  Returns the SHA3-512 hash of `bytes`.
  -/
  native fun sha3_512_internal(bytes : Vector<u8>) -> Vector<u8>

  /--
  Returns the RIPEMD-160 hash of `bytes`.

  WARNING: Only 80-bit security is provided by this function. This means an adversary who can compute roughly 2^80
  hashes will, with high probability, find a collision x_1 != x_2 such that RIPEMD-160(x_1) = RIPEMD-160(x_2).
  -/
  native fun ripemd160_internal(bytes : Vector<u8>) -> Vector<u8>

  /--
  Returns the BLAKE2B-256 hash of `bytes`.
  -/
  native fun blake2b_256_internal(bytes : Vector<u8>) -> Vector<u8>

  --
  -- Testing
  --
  -- We need to enable the feature in order for the native call to be allowed.
  -- From https://emn178.github.io/online-tools/sha512.html
  -- We need to enable the feature in order for the native call to be allowed.
  -- From https://emn178.github.io/online-tools/sha3_512.html
  -- We need to enable the feature in order for the native call to be allowed.
  -- From https://www.browserling.com/tools/ripemd160-hash
  -- We disable the feature to make sure the `blake2b_256` call aborts
  -- We need to enable the feature in order for the native call to be allowed.
  -- empty message doesn't yield an output on the online generator
  -- From https://www.toolkitbay.com/tkb/tool/BLAKE2b_256
  --
  -- For computing the hash of an empty string, we use the following Python3 script:
  -- ```
  --   #!/usr/bin/python3
  --
  --   import hashlib
  --
  --   print(hashlib.blake2b(b'', digest_size=32).hexdigest());
  -- ```
