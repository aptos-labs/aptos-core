//! Attestation document fixtures.
//!
//! These build *structurally* genuine v3 documents: every hash and the
//! `REPORT_DATA` derivation are computed exactly as `BuildAttestation` computes
//! them. What they cannot forge is Intel's signature over a TDX quote, which is
//! precisely the property the verifier tests rely on.

use super::envelope;
use base64::Engine;
use sha2::{Digest, Sha256};

/// SHA-256 SPKI fingerprint used as the endorsed TLS key in fixtures.
pub const TLS_FP: &str = "1111111111111111111111111111111111111111111111111111111111111111";
/// X25519 HPKE public key used in fixtures.
pub const HPKE_KEY: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// A genuine, Intel-signed TDX quote and its matching DCAP collateral.
///
/// Vendored from the `dcap-qvl` crate's `sample/` directory (MIT, Copyright (c)
/// Phala Network). Unlike the synthetic fixtures this module builds — which are
/// *structurally* genuine but carry no valid signature — this really was signed
/// by Intel, so it exercises the full DCAP chain up to the pinned Intel root.
///
/// Two consequences shape the tests that use it:
///
/// 1. Its `REPORTDATA` is fixed at [`TDX_QUOTE_REPORT_DATA`]. Nothing can make
///    a genuine Intel signature commit to a nonce we chose, so a positive test
///    must pass that value in as the expected binding.
/// 2. Its collateral expired on 2025-07-19, so verification must be pinned to
///    [`TDX_COLLATERAL_VALID_AT`]. Using the real clock would fail on validity,
///    not on anything the test means to assert.
pub const TDX_QUOTE: &[u8] = include_bytes!("../assets/testdata/tdx_quote.bin");
/// DCAP collateral matching [`TDX_QUOTE`], as captured Intel PCS responses.
///
/// This is the `intel-pcs/v1` wire format a real `cvmimage` document carries.
/// See `assets/testdata/README.md` for how it was produced and what is and is
/// not genuine about it.
pub const TDX_PCS_COLLATERAL_JSON: &str =
    include_str!("../assets/testdata/tdx_pcs_collateral.json");

/// The `REPORTDATA` [`TDX_QUOTE`] actually commits to.
pub const TDX_QUOTE_REPORT_DATA: &str = "9a9d48e7f6799642d3d1b34e1e5e1742d4bb02dd6ddd551862c1211d35c304f9eca3efdbb481601c163cf52493d6e44aed55d51ec39b7e518fadb92c2b523f20";
/// The authenticated `MRTD` of [`TDX_QUOTE`].
pub const TDX_QUOTE_MRTD: &str = "91eb2b44d141d4ece09f0c75c2c53d247a3c68edd7fafe8a3520c942a604a407de03ae6dc5f87f27428b2538873118b7";
/// The authenticated `RTMR0` of [`TDX_QUOTE`].
pub const TDX_QUOTE_RTMR0: &str = "44c0197b39157fdd7a4dcc44767f9d6b0bb3977c7a8e347b8492f827fe9d9e5c48aca29b220b80b6a540cf994b9bc9c0";

/// A timestamp inside the validity window of [`TDX_PCS_COLLATERAL_JSON`]
/// (2025-06-19 .. 2025-07-19); this is 2025-07-01T00:00:00Z.
pub const TDX_COLLATERAL_VALID_AT: u64 = 1_751_328_000;

/// The 64 `REPORTDATA` bytes of [`TDX_QUOTE`], as the verifier expects them.
pub fn tdx_quote_report_data() -> [u8; 64] {
    let mut out = [0u8; 64];
    hex::decode_to_slice(TDX_QUOTE_REPORT_DATA, &mut out).expect("valid fixture hex");
    out
}

/// A collateral entry carrying the genuine Intel DCAP collateral, in the
/// `intel-pcs/v1` captured-response format.
pub fn tdx_collateral_entry() -> serde_json::Value {
    serde_json::json!({
        "id": "intel-pcs",
        "role": envelope::ROLE_ENDORSEMENT,
        "format": envelope::COLLATERAL_INTEL_PCS_V1_FORMAT,
        "subjects": [envelope::SUBJECT_CPU],
        "data": serde_json::from_str::<serde_json::Value>(TDX_PCS_COLLATERAL_JSON)
            .expect("valid fixture JSON"),
    })
}

/// A generated fixture document plus the values needed to assert on it.
pub struct Fixture {
    pub json: String,
    pub nonce: [u8; 32],
    pub nonce_hex: String,
    pub report_data_hex: String,
    pub crypto_material_b64: String,
}

/// Builds v3 documents, optionally with specific defects.
pub struct DocumentBuilder {
    nonce: [u8; 32],
    tls_fp: String,
    hpke_key: String,
    cpu_evidence_format: String,
    report_base64: Option<String>,
    tamper_crypto_material: bool,
    forge_report_data: bool,
    collateral: Vec<serde_json::Value>,
}

impl Default for DocumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentBuilder {
    pub fn new() -> Self {
        Self {
            nonce: [0x42; 32],
            tls_fp: TLS_FP.to_string(),
            hpke_key: HPKE_KEY.to_string(),
            cpu_evidence_format: envelope::TDX_QUOTE_V1_FORMAT.to_string(),
            report_base64: None,
            tamper_crypto_material: false,
            forge_report_data: false,
            collateral: Vec::new(),
        }
    }

    pub fn nonce(mut self, nonce: [u8; 32]) -> Self {
        self.nonce = nonce;
        self
    }

    /// Endorse a different TLS key fingerprint than the fixture default.
    pub fn tls_fingerprint(mut self, fingerprint: &str) -> Self {
        self.tls_fp = fingerprint.to_string();
        self
    }

    pub fn cpu_evidence_format(mut self, format: &str) -> Self {
        self.cpu_evidence_format = format.to_string();
        self
    }

    /// Present the genuine Intel-signed [`TDX_QUOTE`] as the CPU evidence,
    /// together with its matching DCAP collateral.
    ///
    /// The document's own `challenge.report_data` still derives from the nonce
    /// as usual, so it will *not* equal the quote's fixed `REPORTDATA`. Tests
    /// that expect verification to succeed must therefore verify against
    /// [`tdx_quote_report_data`]; the mismatch is otherwise exactly the
    /// challenge-binding failure the vault is supposed to catch.
    pub fn with_real_tdx_quote(mut self) -> Self {
        let engine = base64::engine::general_purpose::STANDARD;
        self.cpu_evidence_format = envelope::TDX_QUOTE_V1_FORMAT.to_string();
        self.report_base64 = Some(engine.encode(TDX_QUOTE));
        self.collateral.push(tdx_collateral_entry());
        self
    }

    /// Declare TDX evidence but attach no Intel collateral.
    pub fn with_tdx_quote_missing_collateral(mut self) -> Self {
        let engine = base64::engine::general_purpose::STANDARD;
        self.cpu_evidence_format = envelope::TDX_QUOTE_V1_FORMAT.to_string();
        self.report_base64 = Some(engine.encode(TDX_QUOTE));
        self
    }

    /// Present a TDX quote whose bytes have been altered, invalidating Intel's
    /// signature while keeping the collateral intact.
    pub fn with_tampered_tdx_quote(mut self) -> Self {
        let engine = base64::engine::general_purpose::STANDARD;
        let mut quote = TDX_QUOTE.to_vec();
        // Flip a bit inside the TD report body (MRTD), which the quote
        // signature covers.
        let mrtd_offset = 48 + 136;
        quote[mrtd_offset] ^= 0x01;
        self.cpu_evidence_format = envelope::TDX_QUOTE_V1_FORMAT.to_string();
        self.report_base64 = Some(engine.encode(quote));
        self.collateral.push(tdx_collateral_entry());
        self
    }

    /// Use an explicit base64 payload as the CPU evidence.
    pub fn report_base64(mut self, value: String) -> Self {
        self.report_base64 = Some(value);
        self
    }

    /// Attach an arbitrary collateral entry.
    ///
    /// Used by the provenance tests to carry real Sigstore reference values
    /// alongside (or instead of) the Intel endorsement entry.
    pub fn collateral_entry(mut self, entry: serde_json::Value) -> Self {
        self.collateral.push(entry);
        self
    }

    /// Rewrite the crypto_material section after hashing it, so the endorsed
    /// hash no longer matches the transmitted bytes.
    pub fn tamper_crypto_material(mut self) -> Self {
        self.tamper_crypto_material = true;
        self
    }

    /// Emit a `report_data` that is not the correct derivation.
    pub fn forge_report_data(mut self) -> Self {
        self.forge_report_data = true;
        self
    }

    pub fn build(self) -> Fixture {
        let engine = base64::engine::general_purpose::STANDARD;

        let crypto_section = serde_json::json!({
            "format": envelope::CRYPTO_MATERIAL_V1_FORMAT,
            "items": [
                {
                    "id": envelope::CRYPTO_MATERIAL_ID_TLS,
                    "format": envelope::KEY_SPKI_FP_SHA256_V1_FORMAT,
                    "data": self.tls_fp,
                },
                {
                    "id": envelope::CRYPTO_MATERIAL_ID_HPKE,
                    "format": envelope::KEY_X25519_HPKE_V1_FORMAT,
                    "data": self.hpke_key,
                },
            ],
        });
        let device_section = serde_json::json!({
            "format": envelope::DEVICE_EVIDENCE_V1_FORMAT,
            "items": [],
        });

        let crypto_bytes = serde_json::to_vec(&crypto_section).expect("serializable");
        let device_bytes = serde_json::to_vec(&device_section).expect("serializable");

        // Hashes are computed over the *original* bytes; tampering afterwards
        // is what makes the endorsed hash stale.
        let crypto_hash: [u8; 32] = Sha256::digest(&crypto_bytes).into();
        let device_hash: [u8; 32] = Sha256::digest(&device_bytes).into();

        let report_data =
            envelope::compute_report_data(&self.nonce, &crypto_hash, &device_hash).expect("valid");

        let transmitted_crypto = if self.tamper_crypto_material {
            let swapped = serde_json::json!({
                "format": envelope::CRYPTO_MATERIAL_V1_FORMAT,
                "items": [
                    {
                        "id": envelope::CRYPTO_MATERIAL_ID_TLS,
                        "format": envelope::KEY_SPKI_FP_SHA256_V1_FORMAT,
                        "data": "9".repeat(64),
                    },
                ],
            });
            serde_json::to_vec(&swapped).expect("serializable")
        } else {
            crypto_bytes.clone()
        };

        let crypto_b64 = engine.encode(&transmitted_crypto);
        let device_b64 = engine.encode(&device_bytes);

        let report_data_hex = if self.forge_report_data {
            "0".repeat(128)
        } else {
            hex::encode(report_data)
        };

        let report_base64 = self
            .report_base64
            .clone()
            .unwrap_or_else(|| engine.encode([0u8; 16]));

        let collateral = self.collateral.clone();

        let document = serde_json::json!({
            "format": envelope::ATTESTATION_V3_FORMAT,
            "challenge": {
                "nonce": hex::encode(self.nonce),
                "report_data": report_data_hex,
                "report_data_algorithm": envelope::REPORT_DATA_V1_ALGORITHM,
            },
            "cpu_evidence": {
                "format": self.cpu_evidence_format,
                "report_base64": report_base64,
                "endorsed": {
                    "crypto_material_hash": hex::encode(crypto_hash),
                    "device_evidence_hash": hex::encode(device_hash),
                },
            },
            "crypto_material": crypto_b64,
            "device_evidence": device_b64,
            "collateral": collateral,
        });

        Fixture {
            json: serde_json::to_string(&document).expect("serializable"),
            nonce: self.nonce,
            nonce_hex: hex::encode(self.nonce),
            report_data_hex: hex::encode(report_data),
            crypto_material_b64: crypto_b64,
        }
    }
}
