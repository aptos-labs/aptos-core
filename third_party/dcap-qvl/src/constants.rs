#![allow(dead_code)]

/// MR_SIGNER measurement (32 bytes)
pub type MrSigner = [u8; 32];
/// MR_ENCLAVE measurement (32 bytes)
pub type MrEnclave = [u8; 32];
/// FMSPC - Firmware Security Version & Package Configuration (6 bytes)
pub type Fmspc = [u8; 6];
/// CPU SVN - Security Version Number for CPU microcode (16 bytes)
pub type CpuSvn = [u8; 16];
/// SVN - Security Version Number (16-bit)
pub type Svn = u16;

pub const ATTESTATION_KEY_TYPE_ECDSA256_WITH_P256_CURVE: u16 = 2;
pub const ATTESTATION_KEY_TYPE_ECDSA484_WITH_P384_CURVE: u16 = 3;

pub const TEE_TYPE_SGX: u32 = 0x00000000;
pub const TEE_TYPE_TDX: u32 = 0x00000081;

pub const ECDSA_256_WITH_P256_CURVE: u16 = 2;
pub const ECDSA_384_WITH_P384_CURVE: u16 = 3;
pub const ECDSA_P256_SIGNATURE_BYTE_LEN: usize = 64;
pub const BODY_BYTE_SIZE: usize = 6;
pub const BODY_SGX_ENCLAVE_REPORT_TYPE: u16 = 1;
pub const BODY_TD_REPORT10_TYPE: u16 = 2;
pub const BODY_TD_REPORT15_TYPE: u16 = 3;
pub const ENCLAVE_REPORT_BYTE_LEN: usize = 384;
pub const TD_REPORT10_BYTE_LEN: usize = 584;
pub const TD_REPORT15_BYTE_LEN: usize = 648;

pub const PCK_ID_PLAIN_PPID: u16 = 1;
pub const PCK_ID_ENCRYPTED_PPID_2048: u16 = 2;
pub const PCK_ID_ENCRYPTED_PPID_3072: u16 = 3;
pub const PCK_ID_PCK_CERTIFICATE: u16 = 4;
pub const PCK_ID_PCK_CERT_CHAIN: u16 = 5;
pub const PCK_ID_QE_REPORT_CERTIFICATION_DATA: u16 = 6;

pub const ALLOWED_QUOTE_VERSIONS: [u16; 3] = [3, 4, 5];
pub const ALLOWED_BODY_TYPES: [u16; 3] = [
    BODY_SGX_ENCLAVE_REPORT_TYPE,
    BODY_TD_REPORT10_TYPE,
    BODY_TD_REPORT15_TYPE,
];
pub const ALLOWED_TEE_TYPES: [u32; 2] = [TEE_TYPE_SGX, TEE_TYPE_TDX];
pub const ALLOWED_ATTESTATION_KEY_TYPES: [u16; 1] = [ECDSA_256_WITH_P256_CURVE];
pub const INTEL_QE_VENDOR_ID: [u8; 16] = [
    0x93, 0x9A, 0x72, 0x33, 0xF7, 0x9C, 0x4C, 0xA9, 0x94, 0x0A, 0x0D, 0xB3, 0x95, 0x7F, 0x06, 0x07,
];
pub const HEADER_BYTE_LEN: usize = 48;
pub const AUTH_DATA_SIZE_BYTE_LEN: usize = 4;

pub const ECDSA_SIGNATURE_BYTE_LEN: usize = 64;
pub const ECDSA_PUBKEY_BYTE_LEN: usize = 64;
pub const QE_REPORT_BYTE_LEN: usize = ENCLAVE_REPORT_BYTE_LEN;
pub const QE_REPORT_SIG_BYTE_LEN: usize = ECDSA_SIGNATURE_BYTE_LEN;
pub const CERTIFICATION_DATA_TYPE_BYTE_LEN: usize = 2;
pub const CERTIFICATION_DATA_SIZE_BYTE_LEN: usize = 4;
pub const QE_AUTH_DATA_SIZE_BYTE_LEN: usize = 2;
pub const QE_CERT_DATA_TYPE_BYTE_LEN: usize = 2;
pub const QE_CERT_DATA_SIZE_BYTE_LEN: usize = 4;

pub const AUTH_DATA_MIN_BYTE_LEN: usize = ECDSA_SIGNATURE_BYTE_LEN
    + ECDSA_PUBKEY_BYTE_LEN
    + QE_REPORT_BYTE_LEN
    + QE_REPORT_SIG_BYTE_LEN
    + QE_AUTH_DATA_SIZE_BYTE_LEN
    + QE_CERT_DATA_TYPE_BYTE_LEN
    + QE_CERT_DATA_SIZE_BYTE_LEN;

pub const QUOTE_MIN_BYTE_LEN: usize =
    // Actual minimal size is a Quote V3 with Enclave report
    HEADER_BYTE_LEN
        + ENCLAVE_REPORT_BYTE_LEN
        + AUTH_DATA_SIZE_BYTE_LEN
        + AUTH_DATA_MIN_BYTE_LEN;

pub const ATTESTATION_KEY_LEN: usize = 64;
pub const AUTHENTICATION_DATA_LEN: usize = 32;
pub const QE_HASH_DATA_BYTE_LEN: usize = ATTESTATION_KEY_LEN + AUTHENTICATION_DATA_LEN;

pub const PCK_ID_PLAIN: u16 = 1;
pub const PCK_ID_RSA_2048_OAEP: u16 = 2;
pub const PCK_ID_RSA_3072_OAEP: u16 = 3;
pub const PCK_LEAF_CERT_PLAIN: u16 = 4;
pub const PCK_CERT_CHAIN: u16 = 5;
pub const QE_REPORT_CERT: u16 = 6;
pub const PLATFORM_MANIFEST: u16 = 7;

// PCK certificate issuer identifiers (lowercase strings used in PCS URLs
// and FFI bindings; rendered from `crate::config::PckCa::as_id_str`).
pub const PROCESSOR_ISSUER_ID: &str = "processor";
pub const PLATFORM_ISSUER_ID: &str = "platform";

// Substrings matched against the issuer DN to classify a PCK leaf as
// Processor- or Platform-issued. Crate-private; see
// `X509CertParsed::pck_ca`.
pub(crate) const PROCESSOR_ISSUER: &str = "Processor";
pub(crate) const PLATFORM_ISSUER: &str = "Platform";

// The root cert is downloaded from `https://certificates.trustedservices.intel.com/Intel_SGX_Provisioning_Certification_RootCA.cer`.
// See https://api.portal.trustedservices.intel.com/content/documentation.html#pcs for more details
pub static TRUSTED_ROOT_CA_DER: &[u8] = include_bytes!("TrustedRootCA.der");

#[cfg(test)]
#[tokio::test]
async fn dcap_roots_should_be_fresh() {
    #[allow(clippy::unwrap_used)]
    let response = reqwest::get(
        "https://certificates.trustedservices.intel.com/Intel_SGX_Provisioning_Certification_RootCA.cer",
    )
    .await
    .unwrap();

    #[allow(clippy::unwrap_used)]
    let ca_der = response.bytes().await.unwrap();

    assert_eq!(TRUSTED_ROOT_CA_DER, ca_der.as_ref());
}
