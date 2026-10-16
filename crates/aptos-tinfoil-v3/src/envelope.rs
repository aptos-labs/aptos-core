//! Strict parsing and challenge-checking of Atlas Tinfoil v3 envelopes with
//! legacy collateral FormatV2.
//!
//! This is a Rust port of `verifier/envelope` in `tinfoilsh/tinfoil-go`
//! (`feat/v3`), which is what `attestation.BuildAttestation` in `cvmimage`
//! produces. The shape is:
//!
//! ```json
//! {
//!   "format": "https://tinfoil.sh/predicate/attestation/v3",
//!   "challenge": { "nonce": "<hex32>", "report_data": "<hex64>",
//!                  "report_data_algorithm": "https://tinfoil.sh/report-data/v1" },
//!   "cpu_evidence": { "format": "...", "report_base64": "...",
//!                     "endorsed": { "crypto_material_hash": "<hex32>",
//!                                   "device_evidence_hash": "<hex32>" } },
//!   "crypto_material": "<base64 of the crypto_material section>",
//!   "device_evidence": "<base64 of the device_evidence section>",
//!   "collateral_format": "https://tinfoil.sh/predicate/attestation-collaterals/v2",
//!   "collateral": [ { "id": ..., "role": ..., "format": ..., "subjects": [...], "data": {...} } ]
//! }
//! ```
//!
//! The two endorsed sections travel base64-encoded precisely so a verifier can
//! recover the exact bytes that were hashed. Their SHA-256 digests, together
//! with the challenge nonce, derive the 64-byte `REPORT_DATA` that the hardware
//! quote signs — which is how the nonce, the TLS key fingerprint and the device
//! evidence all become part of one hardware-authenticated statement.

use std::collections::BTreeSet;

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::NONCE_SIZE;

pub const ATTESTATION_V3_FORMAT: &str = "https://tinfoil.sh/predicate/attestation/v3";
pub const REPORT_DATA_V1_ALGORITHM: &str = "https://tinfoil.sh/report-data/v1";
pub const CRYPTO_MATERIAL_V1_FORMAT: &str = "https://tinfoil.sh/crypto-material/v1";
pub const DEVICE_EVIDENCE_V1_FORMAT: &str = "https://tinfoil.sh/device-evidence/v1";

/// The Atlas v3 envelope uses the legacy workload-release collateral policy.
/// Current Tinfoil config/runtime collateral has a distinct v3 format and
/// requires additional, independently trusted endorsement signing keys.
pub const COLLATERAL_FORMAT_V2: &str = "https://tinfoil.sh/predicate/attestation-collaterals/v2";
pub const COLLATERAL_FORMAT_V3: &str = "https://tinfoil.sh/predicate/attestation-collaterals/v3";

pub const TDX_QUOTE_V1_FORMAT: &str = "https://tinfoil.sh/format/tdx-quote/v1";

pub const KEY_SPKI_FP_SHA256_V1_FORMAT: &str = "https://tinfoil.sh/key/spki-fp-sha256/v1";
pub const KEY_X25519_HPKE_V1_FORMAT: &str = "https://tinfoil.sh/key/x25519-hpke/v1";

/// Intel DCAP quote-verification collateral (TCB info, QE identity, CRLs),
/// carried as Intel PCS responses captured *verbatim*.
///
/// The TD quote embeds its own PCK certificate chain, but the rest of the
/// collateral does not travel in the quote. Carrying it here is what lets the
/// vault verify a TDX quote without contacting Intel's PCS. The `data` member
/// deserializes into [`IntelPcsCollateral`].
///
/// Responses are captured rather than pre-parsed because Intel's signatures
/// cover exact bytes, and because the issuer chains are delivered in response
/// *headers* rather than in the body. `src/attestation/tdx.rs` adapts them onto
/// the `dcap_qvl::QuoteCollateralV3` this vault's verifier consumes.
pub const COLLATERAL_INTEL_PCS_V1_FORMAT: &str = "https://tinfoil.sh/collateral/intel-pcs/v1";

/// Sigstore code-provenance reference values (`sigstore-code`).
pub const COLLATERAL_SIGSTORE_CODE_V1_FORMAT: &str =
    "https://tinfoil.sh/collateral/sigstore-code/v1";
/// Sigstore platform-endorsement reference values (`sigstore-platform`).
pub const COLLATERAL_SIGSTORE_PLATFORM_V1_FORMAT: &str =
    "https://tinfoil.sh/collateral/sigstore-platform/v1";
/// Freshness witness bundles (`code-freshness`, `platform-freshness`).
pub const COLLATERAL_SIGSTORE_FRESHNESS_V1_FORMAT: &str =
    "https://tinfoil.sh/collateral/sigstore-freshness/v1";

/// Collateral entry id of the code freshness witness.
pub const FRESHNESS_COLLATERAL_ID_CODE: &str = "code-freshness";
/// Collateral entry id of the platform freshness witness.
pub const FRESHNESS_COLLATERAL_ID_PLATFORM: &str = "platform-freshness";

pub const CRYPTO_MATERIAL_ID_TLS: &str = "tls";
pub const CRYPTO_MATERIAL_ID_HPKE: &str = "hpke";
pub const SUBJECT_CPU: &str = "cpu";

pub const ROLE_ENDORSEMENT: &str = "endorsement";
pub const ROLE_REFERENCE_VALUES: &str = "reference-values";

/// Largest attestation document the vault will parse.
pub const MAX_DOCUMENT_BYTES: usize = 4 << 20;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EnvelopeError {
    #[error("document exceeds {MAX_DOCUMENT_BYTES} bytes")]
    TooLarge,
    #[error("parsing attestation document: {0}")]
    Parse(String),
    #[error("duplicate JSON member {0:?}")]
    DuplicateMember(String),
    #[error("unsupported document format {0:?}")]
    UnsupportedFormat(String),
    #[error("{field} is not {want} bytes of lowercase hex")]
    Hex { field: String, want: usize },
    #[error("{0} is not canonical base64")]
    Base64(String),
    #[error("{0}")]
    Malformed(String),
    #[error("challenge nonce does not match the issued nonce")]
    NonceMismatch,
    #[error("{0} does not match its endorsed hash")]
    EndorsedHashMismatch(&'static str),
    #[error("challenge report_data does not match the recomputed value")]
    ReportDataMismatch,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawDocument {
    pub format: String,
    pub challenge: Challenge,
    pub cpu_evidence: CpuEvidence,
    pub crypto_material: String,
    pub device_evidence: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub collateral_format: String,
    #[serde(default)]
    pub collateral: Vec<CollateralEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub nonce: String,
    pub report_data: String,
    pub report_data_algorithm: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CpuEvidence {
    pub format: String,
    pub report_base64: String,
    pub endorsed: EndorsedHashes,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EndorsedHashes {
    pub crypto_material_hash: String,
    pub device_evidence_hash: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollateralEntry {
    pub id: String,
    pub role: String,
    pub format: String,
    #[serde(default)]
    pub subjects: Vec<String>,
    pub data: serde_json::Value,
}

/// The `data` payload of a [`COLLATERAL_INTEL_PCS_V1_FORMAT`] entry.
///
/// Mirrors `envelope.IntelPCSCollateral` in `tinfoilsh/tinfoil-go` (`feat/v3`):
/// Intel PCS responses captured verbatim so a verifier can replay them instead
/// of fetching. This is untrusted transport — every byte is re-authenticated by
/// Intel's signature chain during quote verification, so a substituted or
/// malformed response can only cause verification to fail.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntelPcsCollateral {
    pub responses: Vec<PcsResponse>,
}

/// One captured Intel PCS response.
///
/// `headers` matters as much as `body_base64`: Intel delivers the TCB-info, QE-
/// identity and PCK-CRL issuer chains in response headers, so a capture that
/// dropped them could not be verified.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PcsResponse {
    pub url: String,
    #[serde(default)]
    pub headers: std::collections::BTreeMap<String, Vec<String>>,
    pub body_base64: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CryptoMaterialSection {
    pub format: String,
    pub items: Vec<CryptoMaterialItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CryptoMaterialItem {
    pub id: String,
    pub format: String,
    pub data: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceEvidenceSection {
    pub format: String,
    pub items: Vec<DeviceEvidenceItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceEvidenceItem {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub vendor: String,
    pub format: String,
    pub evidence: serde_json::Value,
}

/// A parsed, structurally valid v3 document.
///
/// Parsing proves nothing about authenticity: nothing here is trustworthy until
/// the hardware quote is shown to bind [`Document::report_data`].
#[derive(Debug, Clone)]
pub struct Document {
    pub raw: RawDocument,
    pub crypto_material: CryptoMaterialSection,
    pub device_evidence: DeviceEvidenceSection,
    crypto_material_bytes: Vec<u8>,
    device_evidence_bytes: Vec<u8>,
}

impl Document {
    /// Strictly parse a v3 document.
    ///
    /// Unknown members, duplicate members, non-lowercase hex, non-canonical
    /// base64 and duplicate item ids are all rejected, mirroring
    /// `envelope.Parse`.
    pub fn parse(bytes: &[u8]) -> Result<Self, EnvelopeError> {
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return Err(EnvelopeError::TooLarge);
        }
        reject_duplicate_members(bytes)?;

        let raw: RawDocument =
            serde_json::from_slice(bytes).map_err(|err| EnvelopeError::Parse(err.to_string()))?;

        if raw.format != ATTESTATION_V3_FORMAT {
            return Err(EnvelopeError::UnsupportedFormat(raw.format));
        }
        // The official parser treats an absent or empty format as legacy V2.
        // Reject current config/runtime V3 before any collateral can be used:
        // its endorsement and freshness signatures require a different trust
        // policy that this verifier does not yet implement.
        if !raw.collateral_format.is_empty() && raw.collateral_format != COLLATERAL_FORMAT_V2 {
            return Err(EnvelopeError::UnsupportedFormat(raw.collateral_format));
        }
        if raw.challenge.report_data_algorithm != REPORT_DATA_V1_ALGORITHM {
            return Err(EnvelopeError::UnsupportedFormat(
                raw.challenge.report_data_algorithm,
            ));
        }

        decode_lower_hex("challenge.nonce", &raw.challenge.nonce, NONCE_SIZE)?;
        decode_lower_hex("challenge.report_data", &raw.challenge.report_data, 64)?;
        decode_lower_hex(
            "cpu_evidence.endorsed.crypto_material_hash",
            &raw.cpu_evidence.endorsed.crypto_material_hash,
            32,
        )?;
        decode_lower_hex(
            "cpu_evidence.endorsed.device_evidence_hash",
            &raw.cpu_evidence.endorsed.device_evidence_hash,
            32,
        )?;
        if raw.cpu_evidence.format.is_empty() || raw.cpu_evidence.report_base64.is_empty() {
            return Err(EnvelopeError::Malformed(
                "cpu_evidence is incomplete".into(),
            ));
        }
        if raw.crypto_material.is_empty() {
            return Err(EnvelopeError::Malformed(
                "crypto_material section is missing".into(),
            ));
        }

        let crypto_material_bytes =
            decode_canonical_base64("crypto_material", &raw.crypto_material)?;
        let device_evidence_bytes =
            decode_canonical_base64("device_evidence", &raw.device_evidence)?;

        reject_duplicate_members(&crypto_material_bytes)?;
        let crypto_material: CryptoMaterialSection = serde_json::from_slice(&crypto_material_bytes)
            .map_err(|err| EnvelopeError::Parse(format!("crypto_material: {err}")))?;
        if crypto_material.format != CRYPTO_MATERIAL_V1_FORMAT {
            return Err(EnvelopeError::UnsupportedFormat(crypto_material.format));
        }

        let mut seen = BTreeSet::new();
        for item in &crypto_material.items {
            if item.id.is_empty() || item.format.is_empty() {
                return Err(EnvelopeError::Malformed(
                    "crypto_material item is incomplete".into(),
                ));
            }
            if !seen.insert(item.id.clone()) {
                return Err(EnvelopeError::Malformed(format!(
                    "duplicate crypto_material item id {:?}",
                    item.id
                )));
            }
            match item.format.as_str() {
                KEY_SPKI_FP_SHA256_V1_FORMAT | KEY_X25519_HPKE_V1_FORMAT => {
                    decode_lower_hex(
                        &format!("crypto_material item {:?} data", item.id),
                        &item.data,
                        32,
                    )?;
                },
                _ => {
                    if item.data.is_empty() || item.data.len() % 2 != 0 || !is_lower_hex(&item.data)
                    {
                        return Err(EnvelopeError::Malformed(format!(
                            "crypto_material item {:?} data is not lowercase hex",
                            item.id
                        )));
                    }
                },
            }
        }

        reject_duplicate_members(&device_evidence_bytes)?;
        let device_evidence: DeviceEvidenceSection = serde_json::from_slice(&device_evidence_bytes)
            .map_err(|err| EnvelopeError::Parse(format!("device_evidence: {err}")))?;
        if device_evidence.format != DEVICE_EVIDENCE_V1_FORMAT {
            return Err(EnvelopeError::UnsupportedFormat(device_evidence.format));
        }
        let mut seen = BTreeSet::new();
        for item in &device_evidence.items {
            if item.id.is_empty() {
                return Err(EnvelopeError::Malformed(
                    "device_evidence item has no id".into(),
                ));
            }
            if !seen.insert(item.id.clone()) {
                return Err(EnvelopeError::Malformed(format!(
                    "duplicate device_evidence item id {:?}",
                    item.id
                )));
            }
        }

        let mut seen = BTreeSet::new();
        for (index, entry) in raw.collateral.iter().enumerate() {
            if entry.id.is_empty() || entry.format.is_empty() {
                return Err(EnvelopeError::Malformed(format!(
                    "collateral entry {index} is incomplete"
                )));
            }
            if !seen.insert(entry.id.clone()) {
                return Err(EnvelopeError::Malformed(format!(
                    "duplicate collateral entry id {:?}",
                    entry.id
                )));
            }
            if entry.role != ROLE_ENDORSEMENT && entry.role != ROLE_REFERENCE_VALUES {
                return Err(EnvelopeError::Malformed(format!(
                    "collateral entry {:?} has unknown role {:?}",
                    entry.id, entry.role
                )));
            }
        }

        Ok(Self {
            raw,
            crypto_material,
            device_evidence,
            crypto_material_bytes,
            device_evidence_bytes,
        })
    }

    /// Parse a document and check every challenge binding it claims.
    ///
    /// Returns the `REPORT_DATA` the CPU quote is required to carry. Checking
    /// is not authentication — [`crate::tdx`] does that — but a
    /// document that fails here can be rejected before any crypto is attempted.
    pub fn check(bytes: &[u8], expected_nonce: &[u8]) -> Result<(Self, [u8; 64]), EnvelopeError> {
        let doc = Self::parse(bytes)?;

        if expected_nonce.len() != NONCE_SIZE {
            return Err(EnvelopeError::Malformed(
                "expected nonce must be 32 bytes".into(),
            ));
        }
        if doc.raw.challenge.nonce != hex::encode(expected_nonce) {
            return Err(EnvelopeError::NonceMismatch);
        }

        let crypto_hash: [u8; 32] = Sha256::digest(&doc.crypto_material_bytes).into();
        let device_hash: [u8; 32] = Sha256::digest(&doc.device_evidence_bytes).into();
        if hex::encode(crypto_hash) != doc.raw.cpu_evidence.endorsed.crypto_material_hash {
            return Err(EnvelopeError::EndorsedHashMismatch("crypto_material"));
        }
        if hex::encode(device_hash) != doc.raw.cpu_evidence.endorsed.device_evidence_hash {
            return Err(EnvelopeError::EndorsedHashMismatch("device_evidence"));
        }

        let report_data = compute_report_data(expected_nonce, &crypto_hash, &device_hash)?;
        if hex::encode(report_data) != doc.raw.challenge.report_data {
            return Err(EnvelopeError::ReportDataMismatch);
        }
        Ok((doc, report_data))
    }

    /// The endorsed crypto-material item with this id.
    pub fn crypto_material_item(&self, id: &str) -> Option<&CryptoMaterialItem> {
        self.crypto_material.items.iter().find(|item| item.id == id)
    }

    /// The endorsed TLS SPKI fingerprint (lowercase hex), if present.
    ///
    /// This is the value the vault compares against the SPKI fingerprint of the
    /// live TLS client certificate.
    pub fn tls_key_fingerprint(&self) -> Option<&str> {
        self.crypto_material_item(CRYPTO_MATERIAL_ID_TLS)
            .filter(|item| item.format == KEY_SPKI_FP_SHA256_V1_FORMAT)
            .map(|item| item.data.as_str())
    }

    /// The endorsed HPKE public key (lowercase hex), if present.
    pub fn hpke_public_key(&self) -> Option<&str> {
        self.crypto_material_item(CRYPTO_MATERIAL_ID_HPKE)
            .filter(|item| item.format == KEY_X25519_HPKE_V1_FORMAT)
            .map(|item| item.data.as_str())
    }

    /// The first reference-values collateral entry of `format`.
    ///
    /// Matches on role and format only — not on id, and not on subjects —
    /// mirroring upstream's `ReferenceValuesCollateral`. The Sigstore payload
    /// inside is *not* trusted by being found here: it names a repository, and
    /// that name is checked against the verifier's own configured repository
    /// before any of it means anything.
    pub fn reference_values_collateral(&self, format: &str) -> Option<&CollateralEntry> {
        self.raw
            .collateral
            .iter()
            .find(|entry| entry.role == ROLE_REFERENCE_VALUES && entry.format == format)
    }

    /// The freshness collateral entry with this id.
    ///
    /// Matches on id, role **and** format together, mirroring upstream's
    /// `FreshnessCollateral`. Requiring all three means a `code-freshness` id
    /// carrying some other format cannot stand in for the real witness.
    ///
    /// `Document::parse` already rejects duplicate collateral ids globally, so
    /// at most one entry can match.
    pub fn freshness_collateral(&self, id: &str) -> Option<&CollateralEntry> {
        self.raw.collateral.iter().find(|entry| {
            entry.id == id
                && entry.role == ROLE_REFERENCE_VALUES
                && entry.format == COLLATERAL_SIGSTORE_FRESHNESS_V1_FORMAT
        })
    }

    /// The first endorsement-role collateral entry of `format` covering `subject`.
    pub fn endorsement_collateral(&self, format: &str, subject: &str) -> Option<&CollateralEntry> {
        self.raw.collateral.iter().find(|entry| {
            entry.role == ROLE_ENDORSEMENT
                && entry.format == format
                && entry.subjects.iter().any(|s| s == subject)
        })
    }

    /// The exact bytes whose SHA-256 the quote endorses.
    pub fn crypto_material_bytes(&self) -> &[u8] {
        &self.crypto_material_bytes
    }

    pub fn device_evidence_bytes(&self) -> &[u8] {
        &self.device_evidence_bytes
    }
}

/// Derive the 64-byte `REPORT_DATA` per `https://tinfoil.sh/report-data/v1`:
/// SHA-256 over the algorithm URI (domain separation) followed by the three
/// fixed-length 32-byte inputs, left-aligned in a zero-padded 64-byte field.
pub fn compute_report_data(
    nonce: &[u8],
    crypto_material_hash: &[u8],
    device_evidence_hash: &[u8],
) -> Result<[u8; 64], EnvelopeError> {
    if nonce.len() != 32 || crypto_material_hash.len() != 32 || device_evidence_hash.len() != 32 {
        return Err(EnvelopeError::Malformed(
            "report data inputs must be 32 bytes each".into(),
        ));
    }
    let mut hasher = Sha256::new();
    hasher.update(REPORT_DATA_V1_ALGORITHM.as_bytes());
    hasher.update(nonce);
    hasher.update(crypto_material_hash);
    hasher.update(device_evidence_hash);

    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&hasher.finalize());
    Ok(out)
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn decode_lower_hex(field: &str, value: &str, want: usize) -> Result<Vec<u8>, EnvelopeError> {
    if value.len() != want * 2 || !is_lower_hex(value) {
        return Err(EnvelopeError::Hex {
            field: field.to_string(),
            want,
        });
    }
    hex::decode(value).map_err(|_| EnvelopeError::Hex {
        field: field.to_string(),
        want,
    })
}

/// Decode standard base64 and reject any non-canonical spelling, so exactly one
/// encoding of a given byte string is accepted (the hashes must be reproducible).
fn decode_canonical_base64(field: &str, value: &str) -> Result<Vec<u8>, EnvelopeError> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| EnvelopeError::Base64(field.to_string()))?;
    if base64::engine::general_purpose::STANDARD.encode(&decoded) != value {
        return Err(EnvelopeError::Base64(field.to_string()));
    }
    Ok(decoded)
}

/// Reject duplicate object member names anywhere in a JSON document.
///
/// `serde_json` silently keeps the last of a repeated key, which would let a
/// document present one value to the hash check and another to a later reader.
/// Go's `strictjson` rejects duplicates outright; this does the same.
fn reject_duplicate_members(bytes: &[u8]) -> Result<(), EnvelopeError> {
    struct DupCheck;

    impl<'de> serde::de::Deserialize<'de> for DupCheck {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct V;

            impl<'de> serde::de::Visitor<'de> for V {
                type Value = DupCheck;

                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("any JSON value")
                }

                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> Result<DupCheck, A::Error> {
                    let mut seen = BTreeSet::new();
                    while let Some(key) = map.next_key::<String>()? {
                        if !seen.insert(key.clone()) {
                            return Err(serde::de::Error::custom(format!(
                                "duplicate member {key:?}"
                            )));
                        }
                        map.next_value::<DupCheck>()?;
                    }
                    Ok(DupCheck)
                }

                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut seq: A,
                ) -> Result<DupCheck, A::Error> {
                    while seq.next_element::<DupCheck>()?.is_some() {}
                    Ok(DupCheck)
                }

                fn visit_bool<E>(self, _: bool) -> Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_i64<E>(self, _: i64) -> Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_u64<E>(self, _: u64) -> Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_f64<E>(self, _: f64) -> Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_str<E>(self, _: &str) -> Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_none<E>(self) -> Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_unit<E>(self) -> Result<DupCheck, E> {
                    Ok(DupCheck)
                }

                fn visit_some<D: serde::Deserializer<'de>>(
                    self,
                    d: D,
                ) -> Result<DupCheck, D::Error> {
                    d.deserialize_any(V)
                }
            }

            deserializer.deserialize_any(V)
        }
    }

    serde_json::from_slice::<DupCheck>(bytes)
        .map(|_| ())
        .map_err(|err| {
            let message = err.to_string();
            if let Some(rest) = message.strip_prefix("duplicate member ") {
                EnvelopeError::DuplicateMember(
                    rest.split(" at line")
                        .next()
                        .unwrap_or(rest)
                        .trim_matches('"')
                        .to_string(),
                )
            } else {
                EnvelopeError::Parse(message)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    #[test]
    fn parses_a_well_formed_document() {
        let doc = testdata::DocumentBuilder::new().build();
        let parsed = Document::parse(doc.json.as_bytes()).unwrap();
        assert_eq!(parsed.raw.format, ATTESTATION_V3_FORMAT);
        assert_eq!(parsed.tls_key_fingerprint(), Some(testdata::TLS_FP));
        assert_eq!(parsed.hpke_public_key(), Some(testdata::HPKE_KEY));
    }

    #[test]
    fn accepts_legacy_collateral_format_and_rejects_config_runtime_v3() {
        let doc = testdata::DocumentBuilder::new().build();
        let mut json: serde_json::Value = serde_json::from_str(&doc.json).unwrap();
        json["collateral_format"] = COLLATERAL_FORMAT_V2.into();
        let v2 = serde_json::to_vec(&json).unwrap();
        assert_eq!(
            Document::check(&v2, &doc.nonce)
                .unwrap()
                .0
                .raw
                .collateral_format,
            COLLATERAL_FORMAT_V2
        );

        json["collateral_format"] = COLLATERAL_FORMAT_V3.into();
        let v3 = serde_json::to_vec(&json).unwrap();
        assert_eq!(
            Document::check(&v3, &doc.nonce).unwrap_err(),
            EnvelopeError::UnsupportedFormat(COLLATERAL_FORMAT_V3.into())
        );
    }

    #[test]
    fn check_recomputes_every_binding() {
        let doc = testdata::DocumentBuilder::new().build();
        let (_, report_data) = Document::check(doc.json.as_bytes(), &doc.nonce).unwrap();
        assert_eq!(hex::encode(report_data), doc.report_data_hex);
    }

    #[test]
    fn check_rejects_a_different_nonce() {
        let doc = testdata::DocumentBuilder::new().build();
        let err = Document::check(doc.json.as_bytes(), &[9u8; 32]).unwrap_err();
        assert_eq!(err, EnvelopeError::NonceMismatch);
    }

    #[test]
    fn check_rejects_a_tampered_crypto_material_section() {
        // Swap in a different TLS fingerprint without updating the endorsed
        // hash: exactly the "steal another CVM's evidence" manipulation.
        let doc = testdata::DocumentBuilder::new()
            .tamper_crypto_material()
            .build();
        let err = Document::check(doc.json.as_bytes(), &doc.nonce).unwrap_err();
        assert_eq!(err, EnvelopeError::EndorsedHashMismatch("crypto_material"));
    }

    #[test]
    fn check_rejects_a_forged_report_data() {
        let doc = testdata::DocumentBuilder::new().forge_report_data().build();
        let err = Document::check(doc.json.as_bytes(), &doc.nonce).unwrap_err();
        assert_eq!(err, EnvelopeError::ReportDataMismatch);
    }

    #[test]
    fn rejects_unknown_and_duplicate_members() {
        let doc = testdata::DocumentBuilder::new().build();

        let with_extra = doc.json.replacen('{', r#"{"mystery":1,"#, 1);
        assert!(matches!(
            Document::parse(with_extra.as_bytes()),
            Err(EnvelopeError::Parse(_))
        ));

        let duplicated = doc.json.replacen(
            r#""format":"#,
            r#""format":"https://tinfoil.sh/predicate/attestation/v3","format":"#,
            1,
        );
        assert!(matches!(
            Document::parse(duplicated.as_bytes()),
            Err(EnvelopeError::DuplicateMember(_))
        ));
    }

    #[test]
    fn rejects_non_lowercase_hex_and_non_canonical_base64() {
        // A nonce whose hex spelling actually contains letters, so that
        // uppercasing it is a meaningful change.
        let doc = testdata::DocumentBuilder::new().nonce([0xab; 32]).build();
        assert_ne!(doc.nonce_hex, doc.nonce_hex.to_uppercase());

        let upper = doc
            .json
            .replace(&doc.nonce_hex, &doc.nonce_hex.to_uppercase());
        assert!(matches!(
            Document::parse(upper.as_bytes()),
            Err(EnvelopeError::Hex { .. })
        ));

        // A section that is not base64 at all is refused.
        let garbage = doc.json.replace(&doc.crypto_material_b64, "not!base64");
        assert!(matches!(
            Document::parse(garbage.as_bytes()),
            Err(EnvelopeError::Base64(_))
        ));
    }

    #[test]
    fn base64_decoding_accepts_exactly_one_spelling_per_byte_string() {
        // Canonical encodings round-trip.
        let canonical = base64::engine::general_purpose::STANDARD.encode(b"vault");
        assert_eq!(
            decode_canonical_base64("section", &canonical).unwrap(),
            b"vault"
        );

        // "ZZ==" decodes to the same single byte as the canonical "ZQ==" but
        // sets unused padding bits. Accepting both would give one byte string
        // two spellings — and the endorsed hash covers the *bytes*, so a second
        // spelling is a second document that hashes differently.
        assert_eq!(
            decode_canonical_base64("section", "ZQ==").unwrap(),
            vec![0x65]
        );
        assert!(matches!(
            decode_canonical_base64("section", "ZZ=="),
            Err(EnvelopeError::Base64(_))
        ));

        // Embedded whitespace is not a canonical spelling either.
        assert!(matches!(
            decode_canonical_base64("section", &format!("{canonical}\n")),
            Err(EnvelopeError::Base64(_))
        ));
    }

    #[test]
    fn rejects_an_oversized_document() {
        let bytes = vec![b' '; MAX_DOCUMENT_BYTES + 1];
        assert!(matches!(
            Document::parse(&bytes),
            Err(EnvelopeError::TooLarge)
        ));
    }

    #[test]
    fn report_data_matches_the_documented_algorithm() {
        let report_data = compute_report_data(&[1u8; 32], &[2u8; 32], &[3u8; 32]).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(b"https://tinfoil.sh/report-data/v1");
        hasher.update([1u8; 32]);
        hasher.update([2u8; 32]);
        hasher.update([3u8; 32]);
        let expected: [u8; 32] = hasher.finalize().into();
        assert_eq!(&report_data[..32], &expected[..]);
        assert_eq!(&report_data[32..], &[0u8; 32]);
    }
}
