// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Noise IK handshake and session encryption adapted for UDP datagrams.
//!
//! This wraps the `aptos_crypto::noise` module to provide:
//! - A datagram-oriented handshake (messages fit in a single UDP packet)
//! - Per-packet encryption/decryption using the post-handshake Noise session

use crate::error::{QuicLikeError, Result};
use aptos_crypto::{noise, x25519};

/// Wraps a Noise IK handshake for UDP.
///
/// The IK pattern: the initiator knows the responder's static public key.
/// Round 1: initiator -> responder (e, es, s, ss + payload)
/// Round 2: responder -> initiator (e, ee, se + payload)
pub struct NoiseHandshake {
    noise_config: noise::NoiseConfig,
}

impl NoiseHandshake {
    pub fn new(private_key: x25519::PrivateKey) -> Self {
        Self {
            noise_config: noise::NoiseConfig::new(private_key),
        }
    }

    pub fn public_key(&self) -> x25519::PublicKey {
        self.noise_config.public_key()
    }

    /// Initiator: build the first handshake message.
    /// Returns (handshake_state, message_bytes).
    pub fn build_initiator_message(
        &self,
        prologue: &[u8],
        remote_public: x25519::PublicKey,
        payload: &[u8],
    ) -> Result<(noise::InitiatorHandshakeState, Vec<u8>)> {
        let msg_len = noise::handshake_init_msg_len(payload.len());
        let mut buffer = vec![0u8; msg_len];
        let mut rng = rand::rngs::OsRng;

        let state = self
            .noise_config
            .initiate_connection(
                &mut rng,
                prologue,
                remote_public,
                Some(payload),
                &mut buffer,
            )
            .map_err(|e| QuicLikeError::NoiseHandshake(format!("initiator build failed: {}", e)))?;

        Ok((state, buffer))
    }

    /// Responder: parse the initiator message and build a response.
    /// Returns (remote_public_key, noise_session, initiator_payload, response_bytes).
    pub fn handle_initiator_message(
        &self,
        prologue: &[u8],
        message: &[u8],
        payload: Option<&[u8]>,
    ) -> Result<(x25519::PublicKey, noise::NoiseSession, Vec<u8>, Vec<u8>)> {
        let (remote_pub, handshake_state, received_payload) = self
            .noise_config
            .parse_client_init_message(prologue, message)
            .map_err(|e| QuicLikeError::NoiseHandshake(format!("parse init failed: {}", e)))?;

        let payload_len = payload.map(|p| p.len()).unwrap_or(0);
        let resp_len = noise::handshake_resp_msg_len(payload_len);
        let mut response = vec![0u8; resp_len];
        let mut rng = rand::rngs::OsRng;

        let session = self
            .noise_config
            .respond_to_client(&mut rng, handshake_state, payload, &mut response)
            .map_err(|e| {
                QuicLikeError::NoiseHandshake(format!("responder build failed: {}", e))
            })?;

        Ok((remote_pub, session, received_payload, response))
    }

    /// Initiator: finalize the handshake after receiving the responder's message.
    /// Returns (responder_payload, noise_session).
    pub fn finalize_initiator(
        &self,
        state: noise::InitiatorHandshakeState,
        response: &[u8],
    ) -> Result<(Vec<u8>, noise::NoiseSession)> {
        let (payload, session) = self
            .noise_config
            .finalize_connection(state, response)
            .map_err(|e| {
                QuicLikeError::NoiseHandshake(format!("initiator finalize failed: {}", e))
            })?;

        Ok((payload, session))
    }
}

/// Wraps a post-handshake Noise session for per-datagram encryption.
pub struct DatagramCrypto {
    session: noise::NoiseSession,
}

impl DatagramCrypto {
    pub fn new(session: noise::NoiseSession) -> Self {
        Self { session }
    }

    /// Encrypt a plaintext datagram payload.
    /// Returns ciphertext (plaintext encrypted in-place + appended auth tag).
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut buf = plaintext.to_vec();
        let tag = self
            .session
            .write_message_in_place(&mut buf)
            .map_err(|e| QuicLikeError::NoiseEncrypt(format!("{}", e)))?;
        buf.extend_from_slice(&tag);
        Ok(buf)
    }

    /// Decrypt a ciphertext datagram payload.
    /// Returns the plaintext.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let mut buf = ciphertext.to_vec();
        let plaintext = self
            .session
            .read_message_in_place(&mut buf)
            .map_err(|e| QuicLikeError::NoiseDecrypt(format!("{}", e)))?;
        Ok(plaintext.to_vec())
    }

    pub fn remote_public_key(&self) -> x25519::PublicKey {
        self.session.get_remote_static()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::Uniform;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_keypair(seed: [u8; 32]) -> (x25519::PrivateKey, x25519::PublicKey) {
        let mut rng = StdRng::from_seed(seed);
        let priv_key = x25519::PrivateKey::generate(&mut rng);
        let pub_key = priv_key.public_key();
        (priv_key, pub_key)
    }

    #[test]
    fn test_noise_handshake_and_encrypt() {
        let (init_priv, _init_pub) = make_keypair([1u8; 32]);
        let (resp_priv, resp_pub) = make_keypair([2u8; 32]);

        let initiator = NoiseHandshake::new(init_priv);
        let responder = NoiseHandshake::new(resp_priv);

        let prologue = b"aptos-quic-like-udp-v1";
        let init_payload = b"hello from initiator";

        // Step 1: initiator builds message
        let (init_state, init_msg) = initiator
            .build_initiator_message(prologue, resp_pub, init_payload)
            .unwrap();

        // Step 2: responder handles it
        let (remote_pub, resp_session, received_payload, resp_msg) = responder
            .handle_initiator_message(prologue, &init_msg, None)
            .unwrap();

        assert_eq!(remote_pub, initiator.public_key());
        assert_eq!(received_payload, init_payload);

        // Step 3: initiator finalizes
        let (_resp_payload, init_session) = initiator
            .finalize_initiator(init_state, &resp_msg)
            .unwrap();

        // Now test encryption
        let mut client_crypto = DatagramCrypto::new(init_session);
        let mut server_crypto = DatagramCrypto::new(resp_session);

        let message = b"stormlight archive";
        let encrypted = client_crypto.encrypt(message).unwrap();
        assert_ne!(encrypted.as_slice(), message.as_slice());

        let decrypted = server_crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, message);

        // Reverse direction
        let message2 = b"the way of kings";
        let encrypted2 = server_crypto.encrypt(message2).unwrap();
        let decrypted2 = client_crypto.decrypt(&encrypted2).unwrap();
        assert_eq!(decrypted2, message2);
    }
}
