// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! FN-DSA-512 (Falcon-512) signatures. EXPERIMENT ONLY.
//!
//! This crate wraps the fixed-point C implementation by Thomas Pornin
//! (<https://github.com/pornin/c-fn-dsa-fx32.64>), which uses no floating point.
//! FIPS 206 is not published yet, so the encoding may change, and this code is not a
//! conformant FN-DSA implementation.
//!
//! Messages are signed raw (no pre-hash). Signing is deterministic for a given seed.

use std::{
    ffi::{c_char, c_int, c_uint, c_void},
    fmt,
};

/// log2 of the degree. 9 is FN-DSA-512 (NIST level 1).
const LOGN: c_uint = 9;

pub const SIGNING_KEY_LENGTH: usize = 1345;
pub const VERIFYING_KEY_LENGTH: usize = 897;
pub const SIGNATURE_LENGTH: usize = 666;
/// The FN-DSA standard uses 40-byte signing seeds.
pub const SIGN_SEED_LENGTH: usize = 40;
/// The longest domain separation context.
pub const MAX_CONTEXT_LENGTH: usize = 255;

const EXPANDED_KEY_LENGTH: usize = ((3 * 9 + 25) << 9) + 136;
const SIGN_TMP_LENGTH: usize = (34 << 9) + 31;

/// The hash identifier for a raw (not pre-hashed) message: an empty C string.
const HASH_ID_RAW: &std::ffi::CStr = c"";

unsafe extern "C" {
    fn fndsa_keygen_seeded(
        logn: c_uint,
        seed: *const c_void,
        seed_len: usize,
        sign_key: *mut c_void,
        vrfy_key: *mut c_void,
    );

    fn fndsa_expand_signkey(
        sign_key: *const c_void,
        sign_key_len: usize,
        ekey: *mut c_void,
        ekey_len: usize,
    ) -> *mut c_void;

    fn fndsa_signexp_seeded_temp(
        exp_key: *const c_void,
        ctx: *const c_void,
        ctx_len: usize,
        id: *const c_char,
        hv: *const c_void,
        hv_len: usize,
        seed: *const c_void,
        seed_len: usize,
        sig: *mut c_void,
        max_sig_len: usize,
        tmp: *mut c_void,
        tmp_len: usize,
    ) -> usize;

    fn fndsa_verify(
        sig: *const c_void,
        sig_len: usize,
        vrfy_key: *const c_void,
        vrfy_key_len: usize,
        ctx: *const c_void,
        ctx_len: usize,
        id: *const c_char,
        hv: *const c_void,
        hv_len: usize,
    ) -> c_int;
}

/// An encoded key pair.
pub struct KeyPair {
    pub signing_key: Box<[u8; SIGNING_KEY_LENGTH]>,
    pub verifying_key: [u8; VERIFYING_KEY_LENGTH],
}

/// Derives a key pair from a seed. The derivation is deterministic, but it is not part of
/// the standard and can change between versions of the C code. The seed should carry at
/// least 256 bits of entropy.
pub fn keygen_from_seed(seed: &[u8]) -> KeyPair {
    let mut signing_key = Box::new([0u8; SIGNING_KEY_LENGTH]);
    let mut verifying_key = [0u8; VERIFYING_KEY_LENGTH];
    // SAFETY: the output buffers have the sizes that the C code writes for LOGN = 9.
    unsafe {
        fndsa_keygen_seeded(
            LOGN,
            seed.as_ptr().cast(),
            seed.len(),
            signing_key.as_mut_ptr().cast(),
            verifying_key.as_mut_ptr().cast(),
        );
    }
    KeyPair {
        signing_key,
        verifying_key,
    }
}

/// A signing key in the expanded form, which signs about twice as fast. It holds about
/// 26 KB.
pub struct ExpandedSigningKey {
    /// The C code places the expanded key at an aligned offset in this buffer. The buffer
    /// must not move while `entry` is in use, so it is boxed and never resized.
    buffer: Box<[u8]>,
    entry: *const c_void,
}

// SAFETY: after construction, the C code only reads the expanded key.
unsafe impl Send for ExpandedSigningKey {}
unsafe impl Sync for ExpandedSigningKey {}

impl ExpandedSigningKey {
    /// Returns `None` if the signing key does not decode.
    pub fn new(signing_key: &[u8]) -> Option<Self> {
        let mut buffer = vec![0u8; EXPANDED_KEY_LENGTH].into_boxed_slice();
        // SAFETY: the buffer has the size the C code requires for LOGN = 9.
        let entry = unsafe {
            fndsa_expand_signkey(
                signing_key.as_ptr().cast(),
                signing_key.len(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
            )
        };
        if entry.is_null() {
            return None;
        }
        Some(Self {
            buffer,
            entry: entry.cast_const(),
        })
    }

    /// Signs `message` with the domain separation `context`. The same key, context,
    /// message, and seed always give the same signature.
    pub fn sign(
        &self,
        context: &[u8],
        message: &[u8],
        seed: &[u8; SIGN_SEED_LENGTH],
    ) -> [u8; SIGNATURE_LENGTH] {
        assert!(
            context.len() <= MAX_CONTEXT_LENGTH,
            "FN-DSA context too long"
        );
        // Keep the buffer alive while `entry` points into it.
        let _buffer = &self.buffer;
        let mut signature = [0u8; SIGNATURE_LENGTH];
        let mut tmp = vec![0u8; SIGN_TMP_LENGTH];
        // SAFETY: `entry` points into `self.buffer`, and the other buffers have the sizes
        // the C code requires for LOGN = 9.
        let length = unsafe {
            fndsa_signexp_seeded_temp(
                self.entry,
                context.as_ptr().cast(),
                context.len(),
                HASH_ID_RAW.as_ptr(),
                message.as_ptr().cast(),
                message.len(),
                seed.as_ptr().cast(),
                seed.len(),
                signature.as_mut_ptr().cast(),
                signature.len(),
                tmp.as_mut_ptr().cast(),
                tmp.len(),
            )
        };
        assert_eq!(length, SIGNATURE_LENGTH, "FN-DSA signing failed");
        signature
    }
}

impl fmt::Debug for ExpandedSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExpandedSigningKey(<elided>)")
    }
}

/// Verifies `signature` on `message` with the domain separation `context`.
pub fn verify(signature: &[u8], verifying_key: &[u8], context: &[u8], message: &[u8]) -> bool {
    if context.len() > MAX_CONTEXT_LENGTH {
        return false;
    }
    // SAFETY: all pointers come with their lengths, and the C code only reads them.
    unsafe {
        fndsa_verify(
            signature.as_ptr().cast(),
            signature.len(),
            verifying_key.as_ptr().cast(),
            verifying_key.len(),
            context.as_ptr().cast(),
            context.len(),
            HASH_ID_RAW.as_ptr(),
            message.as_ptr().cast(),
            message.len(),
        ) == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    const CONTEXT: &[u8] = b"APTOS_TEST";

    #[test]
    fn sign_and_verify() {
        let key_pair = keygen_from_seed(&[7u8; 32]);
        let key = ExpandedSigningKey::new(key_pair.signing_key.as_slice()).unwrap();
        let signature = key.sign(CONTEXT, b"hello", &[1u8; SIGN_SEED_LENGTH]);
        assert!(verify(
            &signature,
            &key_pair.verifying_key,
            CONTEXT,
            b"hello"
        ));
        assert!(!verify(
            &signature,
            &key_pair.verifying_key,
            CONTEXT,
            b"goodbye"
        ));
        assert!(!verify(
            &signature,
            &key_pair.verifying_key,
            b"OTHER",
            b"hello"
        ));

        let other = keygen_from_seed(&[8u8; 32]);
        assert!(!verify(&signature, &other.verifying_key, CONTEXT, b"hello"));

        let mut tampered = signature;
        tampered[100] ^= 1;
        assert!(!verify(
            &tampered,
            &key_pair.verifying_key,
            CONTEXT,
            b"hello"
        ));
        assert!(!verify(
            &signature[..10],
            &key_pair.verifying_key,
            CONTEXT,
            b"hello"
        ));
    }

    #[test]
    fn deterministic() {
        let a = keygen_from_seed(&[3u8; 32]);
        let b = keygen_from_seed(&[3u8; 32]);
        assert_eq!(a.verifying_key, b.verifying_key);
        assert_eq!(a.signing_key, b.signing_key);

        let key = ExpandedSigningKey::new(a.signing_key.as_slice()).unwrap();
        let seed = [9u8; SIGN_SEED_LENGTH];
        assert_eq!(
            key.sign(CONTEXT, b"msg", &seed),
            key.sign(CONTEXT, b"msg", &seed)
        );
        assert_ne!(
            key.sign(CONTEXT, b"msg", &seed),
            key.sign(CONTEXT, b"msg", &[10u8; SIGN_SEED_LENGTH])
        );
    }

    #[test]
    fn rejects_bad_signing_key() {
        assert!(ExpandedSigningKey::new(&[0u8; SIGNING_KEY_LENGTH]).is_none());
        assert!(ExpandedSigningKey::new(&[0u8; 10]).is_none());
    }

    /// Run with `cargo test -p aptos-fn-dsa --release -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn bench() {
        const N: u32 = 1000;
        let start = Instant::now();
        let key_pair = keygen_from_seed(&[1u8; 32]);
        println!("keygen: {:?}", start.elapsed());

        let start = Instant::now();
        let key = ExpandedSigningKey::new(key_pair.signing_key.as_slice()).unwrap();
        println!("expand: {:?}", start.elapsed());

        let message = [5u8; 200];
        let start = Instant::now();
        let mut signature = [0u8; SIGNATURE_LENGTH];
        for i in 0..N {
            let mut seed = [0u8; SIGN_SEED_LENGTH];
            seed[..4].copy_from_slice(&i.to_le_bytes());
            signature = key.sign(CONTEXT, &message, &seed);
        }
        println!("sign: {:?} each", start.elapsed() / N);

        let start = Instant::now();
        for _ in 0..N {
            assert!(verify(
                &signature,
                &key_pair.verifying_key,
                CONTEXT,
                &message
            ));
        }
        println!("verify: {:?} each", start.elapsed() / N);
    }
}
