// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(test, feature = "fuzzing"))]

/// The length in bytes of the AES-256-GCM authentication tag.
pub const AES_GCM_TAG_LEN: usize = 16;

/// The length in bytes of the AES-256-GCM nonce.
pub const AES_GCM_NONCE_LEN: usize = 12;

/// The length in bytes of the `shared_val_netaddr_key` and per-validator
/// `derived_key`.
pub const KEY_LEN: usize = 32;

/// Convenient type alias for the `shared_val_netaddr_key` as an array.
pub type Key = [u8; KEY_LEN];
pub type KeyVersion = u32;

/// Constant key + version so we can push `NetworkAddress` everywhere
/// without worrying about getting the key in the right places. these will be
/// test-only soon.
// TODO(philiphayes): feature gate for testing/fuzzing only
pub const TEST_SHARED_VAL_NETADDR_KEY: Key = [0u8; KEY_LEN];
pub const TEST_SHARED_VAL_NETADDR_KEY_VERSION: KeyVersion = 0;

/// We salt the HKDF for deriving the account keys to provide application
/// separation.
///
/// Note: modifying this salt is a backwards-incompatible protocol change.
///
/// For readers, the HKDF salt is equal to the following hex string:
/// `"7ffda2ae982a2ebfab2a4da62f76fe33592c85e02445b875f02ded51a520ba2a"` which is
/// also equal to the hash value `SHA3-256(b"DIEM_ENCRYPTED_NETWORK_ADDRESS_SALT")`.
///
/// ```
/// use diem_types::network_address::encrypted::HKDF_SALT;
/// use diem_crypto::hash::HashValue;
///
/// let derived_salt = HashValue::sha3_256_of(b"DIEM_ENCRYPTED_NETWORK_ADDRESS_SALT");
/// assert_eq!(HKDF_SALT.as_ref(), derived_salt.as_ref());
/// ```
pub const HKDF_SALT: [u8; 32] = [
    0x7f, 0xfd, 0xa2, 0xae, 0x98, 0x2a, 0x2e, 0xbf, 0xab, 0x2a, 0x4d, 0xa6, 0x2f, 0x76, 0xfe, 0x33,
    0x59, 0x2c, 0x85, 0xe0, 0x24, 0x45, 0xb8, 0x75, 0xf0, 0x2d, 0xed, 0x51, 0xa5, 0x20, 0xba, 0x2a,
];
