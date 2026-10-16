// Adapted from sigstore-rs (Apache 2.0 License)
// https://github.com/sigstore/sigstore-rs/blob/34f232af72ba6108f001f1612fdb03c87af8ca62/src/rekor/models/checkpoint.rs
//
// Copyright 2023 The Sigstore Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Modifications: replaced CosignVerificationKey with direct p256/ed25519-dalek
// verification, removed serde impls and is_valid_for_proof.

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use std::fmt::Write;
use std::fmt::{Display, Formatter};

use crate::error::{Error, Result};

/// A checkpoint (also known as a signed tree head) that is served by the log.
/// The `note` field stores the tree state, and its authenticity can be verified
/// with the data in `signatures`.
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct SignedCheckpoint {
    pub note: Checkpoint,
    pub signatures: Vec<CheckpointSignature>,
}

/// The metadata that is contained in a checkpoint.
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Checkpoint {
    /// origin is the unique identifier/version string
    pub origin: String,
    /// merkle tree size
    pub size: u64,
    /// merkle tree root hash
    pub hash: [u8; 32],
    /// catches the rest of the content
    pub other_content: Vec<OtherContent>,
}

/// The signature that is contained in a checkpoint.
/// The `key_fingerprint` are the first four bytes of the key hash of the corresponding log public key.
/// The actual signature is stored in `raw`.
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct CheckpointSignature {
    pub key_fingerprint: [u8; 4],
    pub raw: Vec<u8>,
    pub name: String,
}

/// Checkpoints can contain additional data.
/// The `KeyValue` variant is for lines that are in the format `<KEY>: <VALUE>`.
/// Everything else is stored in the `Value` variant.
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum OtherContent {
    KeyValue(String, String),
    Value(String),
}

impl Display for OtherContent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OtherContent::KeyValue(k, v) => write!(f, "{k}: {v}"),
            OtherContent::Value(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ParseCheckpointError {
    DecodeError(String),
}

impl SignedCheckpoint {
    /// Decode from the signed note format used by Rekor for envelopes.
    /// See https://github.com/transparency-dev/formats/blob/2de64aa755f08489bda36125786ced79688af872/log/README.md#signed-envelope
    pub fn decode(s: &str) -> Result<Self> {
        let checkpoint = s.trim_start_matches('"').trim_end_matches('"');

        let Some((note, sigs)) = checkpoint.split_once("\n\n") else {
            return Err(Error::SigstoreVerification(
                "Invalid checkpoint format: missing signature section".into(),
            ));
        };

        let signatures: Vec<CheckpointSignature> = sigs
            .lines()
            .filter(|s| !s.trim().is_empty())
            .map(CheckpointSignature::decode)
            .collect::<std::result::Result<_, _>>()?;

        if signatures.is_empty() {
            return Err(Error::SigstoreVerification(
                "Checkpoint has no signatures".into(),
            ));
        }

        let note = Checkpoint::unmarshal(note)?;

        Ok(SignedCheckpoint { note, signatures })
    }

    /// Encode into the signed note format used by Rekor.
    #[cfg(test)]
    fn encode(&self) -> String {
        let note = self.note.marshal() + "\n";
        let empty_line = "\n";
        let signatures = self
            .signatures
            .iter()
            .map(|s| s.encode())
            .collect::<Vec<_>>()
            .join("\n");
        format!("{note}{empty_line}{signatures}")
    }

    /// Verify that at least one of the checkpoint's signatures is valid.
    /// The signed message is the marshaled note body (which already ends with a newline).
    pub fn verify_signature(&self, key_der: &[u8], key_type: &str) -> Result<()> {
        let message = self.note.marshal();
        let message_bytes = message.as_bytes();

        for sig in &self.signatures {
            let result = match key_type {
                "PKIX_ECDSA_P256_SHA_256" => verify_ecdsa_p256(message_bytes, &sig.raw, key_der),
                "PKIX_ED25519" => verify_ed25519(message_bytes, &sig.raw, key_der),
                _ => continue,
            };
            if result.is_ok() {
                return Ok(());
            }
        }

        Err(Error::SigstoreVerification(
            "No valid checkpoint signature found".into(),
        ))
    }
}

impl Checkpoint {
    /// Marshal the note body.
    /// See https://github.com/transparency-dev/formats/blob/2de64aa755f08489bda36125786ced79688af872/log/README.md#checkpoint-body
    pub fn marshal(&self) -> String {
        let hash_b64 = BASE64_STANDARD.encode(self.hash);
        let other_content: String = self.other_content.iter().fold(String::new(), |mut acc, c| {
            writeln!(acc, "{c}").expect("failed to write to string");
            acc
        });
        format!(
            "{}\n{}\n{hash_b64}\n{other_content}",
            self.origin, self.size
        )
    }

    /// Unmarshal parses the note body.
    fn unmarshal(s: &str) -> Result<Self> {
        // The note is in the form:
        // <Origin string>
        // <Decimal log size>
        // <Base64 log root hash>
        // [other data]
        let split_note = s.split('\n').collect::<Vec<_>>();
        let [origin, size, hash_b64, other_content @ ..] = split_note.as_slice() else {
            return Err(Error::SigstoreVerification(
                "Checkpoint note not in expected format".into(),
            ));
        };

        if origin.trim().is_empty() {
            return Err(Error::SigstoreVerification(
                "Checkpoint origin string must not be empty".into(),
            ));
        }

        let size = size.parse::<u64>().map_err(|_| {
            Error::SigstoreVerification("Checkpoint size is not a valid decimal number".into())
        })?;

        let hash = BASE64_STANDARD
            .decode(hash_b64)
            .map_err(|_| {
                Error::SigstoreVerification("Failed to decode checkpoint root hash".into())
            })
            .and_then(|v| {
                <[u8; 32]>::try_from(v).map_err(|_| {
                    Error::SigstoreVerification("Checkpoint root hash is not 32 bytes".into())
                })
            })?;

        let other_content = other_content
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.split_once(": ")
                    .map(|(k, v)| OtherContent::KeyValue(k.to_string(), v.to_string()))
                    .unwrap_or(OtherContent::Value(s.to_string()))
            })
            .collect();

        Ok(Checkpoint {
            origin: origin.to_string(),
            size,
            hash,
            other_content,
        })
    }
}

impl CheckpointSignature {
    /// Encode into the sumdb note signature format: `— <name> <base64(key_hint + signature)>`
    ///
    /// Only the round-trip tests encode; verification only ever decodes. Gated
    /// to match `Checkpoint::encode`, its sole caller.
    #[cfg(test)]
    fn encode(&self) -> String {
        let sig_b64 =
            BASE64_STANDARD.encode([self.key_fingerprint.as_slice(), self.raw.as_slice()].concat());
        format!("\u{2014} {} {sig_b64}\n", self.name)
    }

    /// Decode from the sumdb note signature format.
    fn decode(s: &str) -> Result<Self> {
        let s = s.trim_start_matches('\n').trim_end_matches('\n');
        if !s.starts_with('\u{2014}') {
            return Err(Error::SigstoreVerification(
                "Checkpoint signature line missing em dash".into(),
            ));
        }
        let parts: Vec<&str> = s.split(' ').collect();
        let [_emdash, name, sig_b64] = parts.as_slice() else {
            return Err(Error::SigstoreVerification(format!(
                "Unexpected checkpoint signature format: {s:?}"
            )));
        };
        let sig = BASE64_STANDARD.decode(sig_b64.trim_end()).map_err(|_| {
            Error::SigstoreVerification("Failed to decode checkpoint signature".into())
        })?;

        let (key_fingerprint, sig) = sig.split_at_checked(4).ok_or_else(|| {
            Error::SigstoreVerification("Checkpoint signature too short for key fingerprint".into())
        })?;
        let key_fingerprint: [u8; 4] = key_fingerprint.try_into().map_err(|_| {
            Error::SigstoreVerification("Failed to parse checkpoint key fingerprint".into())
        })?;

        Ok(CheckpointSignature {
            key_fingerprint,
            name: name.to_string(),
            raw: sig.to_vec(),
        })
    }
}

/// Verify ECDSA P-256 signature over a message.
pub(crate) fn verify_ecdsa_p256(message: &[u8], signature: &[u8], key_der: &[u8]) -> Result<()> {
    use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
    use p256::pkcs8::DecodePublicKey;

    let verifying_key = VerifyingKey::from_public_key_der(key_der).map_err(|e| {
        Error::SigstoreVerification(format!("Invalid Rekor ECDSA public key: {}", e))
    })?;

    let sig = Signature::from_der(signature).map_err(|e| {
        Error::SigstoreVerification(format!("Invalid ECDSA signature format: {}", e))
    })?;

    verifying_key.verify(message, &sig).map_err(|_| {
        Error::SigstoreVerification("Checkpoint ECDSA signature verification failed".into())
    })
}

/// Verify Ed25519 signature over a message.
pub(crate) fn verify_ed25519(message: &[u8], signature: &[u8], key_der: &[u8]) -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    if signature.len() < 64 {
        return Err(Error::SigstoreVerification(
            "Ed25519 checkpoint signature too short".into(),
        ));
    }
    let sig_bytes = &signature[..64];

    // Ed25519 public key in SPKI format: skip the SPKI header to get raw 32-byte key
    // SPKI for Ed25519: 30 2a 30 05 06 03 2b 65 70 03 21 00 <32 bytes>
    if key_der.len() < 44 {
        return Err(Error::SigstoreVerification(
            "Invalid Ed25519 SPKI key length".into(),
        ));
    }
    let raw_key = &key_der[key_der.len() - 32..];

    let verifying_key = VerifyingKey::try_from(raw_key).map_err(|e| {
        Error::SigstoreVerification(format!("Invalid Rekor Ed25519 public key: {}", e))
    })?;

    let sig = Signature::try_from(sig_bytes).map_err(|e| {
        Error::SigstoreVerification(format!("Invalid Ed25519 signature format: {}", e))
    })?;

    verifying_key.verify(message, &sig).map_err(|_| {
        Error::SigstoreVerification("Checkpoint Ed25519 signature verification failed".into())
    })
}

#[cfg(test)]
mod test {
    mod test_checkpoint_note {
        use crate::checkpoint::Checkpoint;
        use crate::checkpoint::OtherContent::{KeyValue, Value};

        #[test]
        fn test_marshal() {
            let test_cases = [
                (
                    "Log Checkpoint v0",
                    123,
                    [0; 32],
                    vec![],
                    "Log Checkpoint v0\n123\nAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
                ),
                (
                    "Banana Checkpoint v5",
                    9944,
                    [1; 32],
                    vec![],
                    "Banana Checkpoint v5\n9944\nAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=\n",
                ),
                (
                    "Banana Checkpoint v7",
                    9943,
                    [2; 32],
                    vec![Value("foo".to_string()), Value("bar".to_string())],
                    "Banana Checkpoint v7\n9943\nAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=\nfoo\nbar\n",
                ),
            ];

            for (origin, size, hash, other_content, expected) in test_cases {
                assert_eq!(
                    Checkpoint {
                        size,
                        origin: origin.to_string(),
                        hash,
                        other_content,
                    }
                    .marshal(),
                    expected
                );
            }
        }

        #[test]
        fn test_unmarshal_valid() {
            let test_cases = [
                (
                    "valid",
                    "Log Checkpoint v0",
                    123,
                    [0; 32],
                    vec![],
                    "Log Checkpoint v0\n123\nAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
                ),
                (
                    "valid",
                    "Banana Checkpoint v5",
                    9944,
                    [1; 32],
                    vec![],
                    "Banana Checkpoint v5\n9944\nAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=\n",
                ),
                (
                    "valid with multiple trailing data lines",
                    "Banana Checkpoint v7",
                    9943,
                    [2; 32],
                    vec![Value("foo".to_string()), Value("bar".to_string())],
                    "Banana Checkpoint v7\n9943\nAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=\nfoo\nbar\n",
                ),
                (
                    "valid with key-value data line",
                    "Banana Checkpoint v7",
                    9943,
                    [2; 32],
                    vec![KeyValue(
                        "Timestamp".to_string(),
                        "1689748607742585419".to_string(),
                    )],
                    "Banana Checkpoint v7\n9943\nAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=\nTimestamp: 1689748607742585419\n",
                ),
                (
                    "valid with trailing newlines",
                    "Banana Checkpoint v7",
                    9943,
                    [2; 32],
                    vec![],
                    "Banana Checkpoint v7\n9943\nAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=\n\n\n\n",
                ),
            ];

            for (desc, origin, size, hash, other_content, input) in test_cases {
                let got = Checkpoint::unmarshal(input);
                let expected = Checkpoint {
                    size,
                    origin: origin.to_string(),
                    hash,
                    other_content,
                };
                assert_eq!(got.unwrap(), expected, "failed test case: {desc}");
            }
        }

        #[test]
        fn test_unmarshal_invalid() {
            let test_cases = [
                ("invalid - insufficient lines", "Head\n9944\n"),
                (
                    "invalid - empty header",
                    "\n9944\ndGhlIHZpZXcgZnJvbSB0aGUgdHJlZSB0b3BzIGlzIGdyZWF0IQ==\n",
                ),
                (
                    "invalid - empty origin",
                    "123\ndGhlIHZpZXcgZnJvbSB0aGUgdHJlZSB0b3BzIGlzIGdyZWF0IQ==\nother data\n",
                ),
                (
                    "invalid - missing newline on roothash",
                    "Log Checkpoint v0\n123\nYmFuYW5hcw==",
                ),
                (
                    "invalid size - not a number",
                    "Log Checkpoint v0\nbananas\ndGhlIHZpZXcgZnJvbSB0aGUgdHJlZSB0b3BzIGlzIGdyZWF0IQ==\n",
                ),
                (
                    "invalid size - negative",
                    "Log Checkpoint v0\n-34\ndGhlIHZpZXcgZnJvbSB0aGUgdHJlZSB0b3BzIGlzIGdyZWF0IQ==\n",
                ),
                (
                    "invalid size - too large",
                    "Log Checkpoint v0\n3438945738945739845734895735\ndGhlIHZpZXcgZnJvbSB0aGUgdHJlZSB0b3BzIGlzIGdyZWF0IQ==\n",
                ),
                (
                    "invalid roothash - not base64",
                    "Log Checkpoint v0\n123\nThisIsn'tBase64\n",
                ),
            ];
            for (desc, data) in test_cases {
                assert!(
                    Checkpoint::unmarshal(data).is_err(),
                    "accepted invalid note: {desc}"
                );
            }
        }
    }

    mod test_checkpoint_signature {
        use crate::checkpoint::{Checkpoint, CheckpointSignature, SignedCheckpoint};

        #[test]
        fn test_to_string_valid_with_url_name() {
            let got = CheckpointSignature {
                name: "log.example.org".to_string(),
                key_fingerprint: [0; 4],
                raw: vec![1; 32],
            }
            .encode();
            let expected =
                "\u{2014} log.example.org AAAAAAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB\n";
            assert_eq!(got, expected)
        }

        #[test]
        fn test_to_string_valid_with_id_name() {
            let got = CheckpointSignature {
                name: "815f6c60aab9".to_string(),
                key_fingerprint: [0; 4],
                raw: vec![1; 32],
            }
            .encode();
            let expected =
                "\u{2014} 815f6c60aab9 AAAAAAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB\n";
            assert_eq!(got, expected)
        }

        #[test]
        fn test_from_str_valid_with_url_name() {
            let input =
                "\u{2014} log.example.org AAAAAAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB\n";
            let expected = CheckpointSignature {
                name: "log.example.org".to_string(),
                key_fingerprint: [0; 4],
                raw: vec![1; 32],
            };
            let got = CheckpointSignature::decode(input);
            assert_eq!(got.unwrap(), expected)
        }

        #[test]
        fn test_from_str_valid_with_id_name() {
            let input = "\u{2014} 815f6c60aab9 AAAAAAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB\n";
            let expected = CheckpointSignature {
                name: "815f6c60aab9".to_string(),
                key_fingerprint: [0; 4],
                raw: vec![1; 32],
            };
            let got = CheckpointSignature::decode(input);
            assert_eq!(got.unwrap(), expected)
        }

        #[test]
        fn test_from_str_valid_with_whitespace() {
            let input =
                "\n\u{2014} log.example.org AAAAAAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB\n\n";
            let expected = CheckpointSignature {
                name: "log.example.org".to_string(),
                key_fingerprint: [0; 4],
                raw: vec![1; 32],
            };
            let got = CheckpointSignature::decode(input);
            assert_eq!(got.unwrap(), expected)
        }

        #[test]
        fn test_from_str_invalid_with_spaces_in_name() {
            let input = "\u{2014} Foo Bar AAAAAAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB\n";
            let got = CheckpointSignature::decode(input);
            assert!(got.is_err())
        }

        #[test]
        fn test_checkpoint_encode_decode_multiple_signatures() {
            let note = Checkpoint {
                origin: "Test Log".to_string(),
                size: 42,
                hash: [7; 32],
                other_content: vec![],
            };
            let sig1 = CheckpointSignature {
                name: "log1.example.org".to_string(),
                key_fingerprint: [1, 2, 3, 4],
                raw: vec![5; 32],
            };
            let sig2 = CheckpointSignature {
                name: "log2.example.org".to_string(),
                key_fingerprint: [9, 8, 7, 6],
                raw: vec![6; 32],
            };
            let checkpoint = SignedCheckpoint {
                note: note.clone(),
                signatures: vec![sig1.clone(), sig2.clone()],
            };
            let encoded = checkpoint.encode();
            let decoded = SignedCheckpoint::decode(&encoded).expect("decode should succeed");
            assert_eq!(decoded.note, note);
            assert_eq!(decoded.signatures.len(), 2);
            assert_eq!(decoded.signatures[0], sig1);
            assert_eq!(decoded.signatures[1], sig2);
        }
    }
}
