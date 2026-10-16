use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{bail, Context, Result};
use scale::{Decode, Encode, Input, Output};
use serde::{Deserialize, Serialize};

#[cfg(feature = "borsh_schema")]
use borsh::BorshSchema;
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};

use crate::constants::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct Data<T> {
    pub data: Vec<u8>,
    _marker: core::marker::PhantomData<T>,
}

impl<T> Data<T> {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<T> Serialize for Data<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_bytes::serialize(&self.data, serializer)
    }
}

impl<'de, T> Deserialize<'de> for Data<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = serde_bytes::deserialize(deserializer)?;
        Ok(Data {
            data,
            _marker: core::marker::PhantomData,
        })
    }
}

impl<T: Decode + Into<u64>> Decode for Data<T> {
    fn decode<I: Input>(input: &mut I) -> Result<Self, scale::Error> {
        const MAX_DATA_LEN: u64 = 1_048_576; // 1 MiB upper bound for variable-length fields

        let len = T::decode(input)?;
        let len_u64 = len.into();
        if len_u64 > MAX_DATA_LEN {
            return Err(scale::Error::from("Data length exceeds maximum"));
        }

        let mut data = vec![0u8; len_u64 as usize];
        input.read(&mut data)?;
        Ok(Data {
            data,
            _marker: core::marker::PhantomData,
        })
    }
}

impl Encode for Data<u16> {
    fn encode_to<O: Output + ?Sized>(&self, output: &mut O) {
        let len = self.data.len() as u16;
        len.encode_to(output);
        output.write(&self.data);
    }
}

impl Encode for Data<u32> {
    fn encode_to<O: Output + ?Sized>(&self, output: &mut O) {
        let len = self.data.len() as u32;
        len.encode_to(output);
        output.write(&self.data);
    }
}

#[derive(
    Decode, Encode, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct Header {
    pub version: u16,
    pub attestation_key_type: u16,
    pub tee_type: u32,
    pub qe_svn: u16,
    pub pce_svn: u16,
    #[serde(with = "serde_bytes")]
    pub qe_vendor_id: [u8; 16],
    #[serde(with = "serde_bytes")]
    pub user_data: [u8; 20],
}

impl Header {
    pub fn is_sgx(&self) -> bool {
        self.tee_type == TEE_TYPE_SGX
    }
}

#[derive(Decode, Encode, Debug)]
pub struct Body {
    pub body_type: u16,
    pub size: u32,
}

#[derive(
    Decode, Encode, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct EnclaveReport {
    #[serde(with = "serde_bytes")]
    pub cpu_svn: [u8; 16],
    pub misc_select: u32,
    #[serde(with = "serde_bytes")]
    pub reserved1: [u8; 28],
    #[serde(with = "serde_bytes")]
    pub attributes: [u8; 16],
    #[serde(with = "serde_bytes")]
    pub mr_enclave: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub reserved2: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub mr_signer: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub reserved3: [u8; 96],
    pub isv_prod_id: u16,
    pub isv_svn: u16,
    #[serde(with = "serde_bytes")]
    pub reserved4: [u8; 60],
    #[serde(with = "serde_bytes")]
    pub report_data: [u8; 64],
}

/// TD Attributes as defined in Intel TDX Module specification A.3.4
#[derive(Debug, Clone)]
pub struct TDAttributes {
    /// TUD (TD Under Debug) flags (bits 7:0)
    /// If any of the bits in this group are set to 1, the TD is untrusted.
    pub tud: u8,

    /// SEC attributes that may impact the security of the TD (bits 31:8)
    pub sec: SECFlags,

    /// OTHER attributes that do not impact the security of the TD (bits 63:32)
    pub other: OTHERFlags,
}

/// TUD (TD Under Debug) flags (bits 7:0)
#[derive(Debug, Clone)]
pub struct TUDFlags {
    /// DEBUG: Defines whether the TD runs in TD debug mode (set to 1) or not (set to 0).
    /// In TD debug mode, the CPU state and private memory are accessible by the host VMM.
    pub debug: bool,

    /// Reserved for future TUD flags - must be 0 (bits 7:1)
    pub reserved: u8,
}

/// SEC attributes that may impact the security of the TD (bits 31:8)
#[derive(Debug, Clone)]
pub struct SECFlags {
    /// Reserved for future SEC flags - must be 0 (bits 27:8)
    pub reserved_lower: u32,

    /// SEPT_VE_DISABLE: Disable EPT violation conversion to #VE on TD access of PENDING pages
    pub sept_ve_disable: bool,

    /// Reserved for future SEC flags - must be 0 (bit 29)
    pub reserved_bit29: bool,

    /// PKS: TD is allowed to use Supervisor Protection Keys
    pub pks: bool,

    /// KL: TD is allowed to use Key Locker
    pub kl: bool,
}

/// OTHER attributes that do not impact the security of the TD (bits 63:32)
#[derive(Debug, Clone)]
pub struct OTHERFlags {
    /// Reserved for future OTHER flags - must be 0 (bits 62:32)
    pub reserved: u32,

    /// PERFMON: TD is allowed to use Perfmon and PERF_METRICS capabilities
    pub perfmon: bool,
}

impl TDAttributes {
    pub fn parse(input: [u8; 8]) -> Result<Self, scale::Error> {
        let tud = input[0];
        // Extract SEC flags (27:8 bits, bytes 1-3 and part of byte 4)
        let reserved_lower =
            (((input[3] & 0x0f) as u32) << 16) | ((input[2] as u32) << 8) | (input[1] as u32);
        let sept_ve_disable = (input[3] & 0x10) != 0; // Bit 28
        let reserved_bit29 = (input[3] & 0x20) != 0; // Bit 29
        let pks = (input[3] & 0x40) != 0; // Bit 30
        let kl = (input[3] & 0x80) != 0; // Bit 31

        // Extract OTHER flags (bytes 4-7)
        let reserved_other = ((input[7] as u32) << 24)
            | ((input[6] as u32) << 16)
            | ((input[5] as u32) << 8)
            | ((input[4] as u32) & 0x7F);
        let perfmon = (input[7] & 0x80) != 0; // Bit 63

        Ok(TDAttributes {
            tud,
            sec: SECFlags {
                reserved_lower,
                sept_ve_disable,
                reserved_bit29,
                pks,
                kl,
            },
            other: OTHERFlags {
                reserved: reserved_other,
                perfmon,
            },
        })
    }
}

#[derive(
    Decode, Encode, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct TDReport10 {
    #[serde(with = "serde_bytes")]
    pub tee_tcb_svn: [u8; 16],
    #[serde(with = "serde_bytes")]
    pub mr_seam: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub mr_signer_seam: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub seam_attributes: [u8; 8],
    #[serde(with = "serde_bytes")]
    pub td_attributes: [u8; 8],
    #[serde(with = "serde_bytes")]
    pub xfam: [u8; 8],
    #[serde(with = "serde_bytes")]
    pub mr_td: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub mr_config_id: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub mr_owner: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub mr_owner_config: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub rt_mr0: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub rt_mr1: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub rt_mr2: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub rt_mr3: [u8; 48],
    #[serde(with = "serde_bytes")]
    pub report_data: [u8; 64],
}

#[derive(
    Decode, Encode, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct TDReport15 {
    pub base: TDReport10,
    #[serde(with = "serde_bytes")]
    pub tee_tcb_svn2: [u8; 16],
    #[serde(with = "serde_bytes")]
    pub mr_service_td: [u8; 48],
}

#[derive(Decode, Encode, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct CertificationData {
    pub cert_type: u16,
    pub body: Data<u32>,
}

impl core::fmt::Debug for CertificationData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let body_str = String::from_utf8_lossy(&self.body.data);
        f.debug_struct("CertificationData")
            .field("cert_type", &self.cert_type)
            .field("body", &body_str)
            .finish()
    }
}

#[derive(
    Decode, Encode, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct QEReportCertificationData {
    #[serde(with = "serde_bytes")]
    pub qe_report: [u8; ENCLAVE_REPORT_BYTE_LEN],
    #[serde(with = "serde_bytes")]
    pub qe_report_signature: [u8; QE_REPORT_SIG_BYTE_LEN],
    pub qe_auth_data: Data<u16>,
    pub certification_data: CertificationData,
}

#[derive(
    Decode, Encode, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct AuthDataV3 {
    #[serde(with = "serde_bytes")]
    pub ecdsa_signature: [u8; ECDSA_SIGNATURE_BYTE_LEN],
    #[serde(with = "serde_bytes")]
    pub ecdsa_attestation_key: [u8; ECDSA_PUBKEY_BYTE_LEN],
    #[serde(with = "serde_bytes")]
    pub qe_report: [u8; ENCLAVE_REPORT_BYTE_LEN],
    #[serde(with = "serde_bytes")]
    pub qe_report_signature: [u8; QE_REPORT_SIG_BYTE_LEN],
    pub qe_auth_data: Data<u16>,
    pub certification_data: CertificationData,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct AuthDataV4 {
    #[serde(with = "serde_bytes")]
    pub ecdsa_signature: [u8; ECDSA_SIGNATURE_BYTE_LEN],
    #[serde(with = "serde_bytes")]
    pub ecdsa_attestation_key: [u8; ECDSA_PUBKEY_BYTE_LEN],
    pub certification_data: CertificationData,
    pub qe_report_data: QEReportCertificationData,
}

impl AuthDataV4 {
    pub fn into_v3(self) -> AuthDataV3 {
        AuthDataV3 {
            ecdsa_signature: self.ecdsa_signature,
            ecdsa_attestation_key: self.ecdsa_attestation_key,
            qe_report: self.qe_report_data.qe_report,
            qe_report_signature: self.qe_report_data.qe_report_signature,
            qe_auth_data: self.qe_report_data.qe_auth_data,
            certification_data: self.qe_report_data.certification_data,
        }
    }
}

impl Decode for AuthDataV4 {
    fn decode<I: Input>(input: &mut I) -> Result<Self, scale::Error> {
        let ecdsa_signature = Decode::decode(input)?;
        let ecdsa_attestation_key = Decode::decode(input)?;
        let certification_data: CertificationData = Decode::decode(input)?;
        let qe_report_data =
            QEReportCertificationData::decode(&mut &certification_data.body.data[..])?;
        Ok(AuthDataV4 {
            ecdsa_signature,
            ecdsa_attestation_key,
            certification_data,
            qe_report_data,
        })
    }
}

impl Encode for AuthDataV4 {
    fn encode_to<O: Output + ?Sized>(&self, output: &mut O) {
        self.ecdsa_signature.encode_to(output);
        self.ecdsa_attestation_key.encode_to(output);

        // Encode qe_report_data into certification_data body
        let mut qe_data_bytes = Vec::new();
        self.qe_report_data.encode_to(&mut qe_data_bytes);

        let cert_data = CertificationData {
            cert_type: self.certification_data.cert_type,
            body: Data {
                data: qe_data_bytes,
                _marker: core::marker::PhantomData,
            },
        };
        cert_data.encode_to(output);
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub enum AuthData {
    V3(AuthDataV3),
    V4(AuthDataV4),
}

// Manual implementation of BorshSchema for AuthData to work around
// the derive bug described in https://github.com/near/borsh-rs/issues/355
#[cfg(feature = "borsh_schema")]
impl borsh::BorshSchema for AuthData {
    fn declaration() -> borsh::schema::Declaration {
        "AuthData".to_string()
    }

    fn add_definitions_recursively(
        definitions: &mut borsh::__private::maybestd::collections::BTreeMap<
            borsh::schema::Declaration,
            borsh::schema::Definition,
        >,
    ) {
        let definition = borsh::schema::Definition::Enum {
            tag_width: 1,
            variants: vec![
                (0, "V3".to_string(), AuthDataV3::declaration()),
                (1, "V4".to_string(), AuthDataV4::declaration()),
            ],
        };

        borsh::schema::add_definition(Self::declaration(), definition, definitions);

        AuthDataV3::add_definitions_recursively(definitions);
        AuthDataV4::add_definitions_recursively(definitions);
    }
}

impl AuthData {
    pub fn into_v3(self) -> AuthDataV3 {
        match self {
            AuthData::V3(data) => data,
            AuthData::V4(data) => data.into_v3(),
        }
    }
}

impl Encode for AuthData {
    fn encode_to<O: Output + ?Sized>(&self, output: &mut O) {
        match self {
            AuthData::V3(data) => data.encode_to(output),
            AuthData::V4(data) => data.encode_to(output),
        }
    }
}

fn decode_auth_data(ver: u16, input: &mut &[u8]) -> Result<AuthData, scale::Error> {
    match ver {
        3 => {
            let auth_data = AuthDataV3::decode(input)?;
            Ok(AuthData::V3(auth_data))
        }
        4 => {
            let auth_data = AuthDataV4::decode(input)?;
            Ok(AuthData::V4(auth_data))
        }
        _ => Err(scale::Error::from("Unsupported auth data version")),
    }
}

#[derive(Decode, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub enum Report {
    SgxEnclave(EnclaveReport),
    TD10(TDReport10),
    TD15(TDReport15),
}

impl Report {
    pub fn is_sgx(&self) -> bool {
        matches!(self, Report::SgxEnclave(_))
    }

    pub fn as_td10(&self) -> Option<&TDReport10> {
        match self {
            Report::TD10(report) => Some(report),
            Report::TD15(report) => Some(&report.base),
            _ => None,
        }
    }

    pub fn as_td15(&self) -> Option<&TDReport15> {
        match self {
            Report::TD15(report) => Some(report),
            _ => None,
        }
    }

    pub fn as_sgx(&self) -> Option<&EnclaveReport> {
        match self {
            Report::SgxEnclave(report) => Some(report),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct Quote {
    pub header: Header,
    pub report: Report,
    pub auth_data: AuthData,
}

impl Decode for Quote {
    fn decode<I: Input>(input: &mut I) -> Result<Self, scale::Error> {
        let header = Header::decode(input)?;
        let report;
        match header.version {
            3 => {
                if header.tee_type != TEE_TYPE_SGX {
                    return Err(scale::Error::from("Invalid TEE type"));
                }
                report = Report::SgxEnclave(EnclaveReport::decode(input)?);
            }
            4 => match header.tee_type {
                TEE_TYPE_SGX => {
                    report = Report::SgxEnclave(EnclaveReport::decode(input)?);
                }
                TEE_TYPE_TDX => {
                    report = Report::TD10(TDReport10::decode(input)?);
                }
                _ => return Err(scale::Error::from("Invalid TEE type")),
            },
            5 => {
                let body = Body::decode(input)?;
                match body.body_type {
                    BODY_SGX_ENCLAVE_REPORT_TYPE => {
                        report = Report::SgxEnclave(EnclaveReport::decode(input)?);
                    }
                    BODY_TD_REPORT10_TYPE => {
                        report = Report::TD10(TDReport10::decode(input)?);
                    }
                    BODY_TD_REPORT15_TYPE => {
                        report = Report::TD15(TDReport15::decode(input)?);
                    }
                    _ => return Err(scale::Error::from("Unsupported body type")),
                }
            }
            _ => return Err(scale::Error::from("Unsupported quote version")),
        }
        let data = Data::<u32>::decode(input)?;
        // Quote v5 uses v4 auth data format
        let auth_version = if header.version == 5 {
            4
        } else {
            header.version
        };
        let auth_data = decode_auth_data(auth_version, &mut &data.data[..])?;
        Ok(Quote {
            header,
            report,
            auth_data,
        })
    }
}

impl Encode for Quote {
    fn encode_to<O: Output + ?Sized>(&self, output: &mut O) {
        // Encode header
        self.header.encode_to(output);

        // Encode body for version 5
        if self.header.version == 5 {
            let body = match &self.report {
                Report::SgxEnclave(_) => Body {
                    body_type: BODY_SGX_ENCLAVE_REPORT_TYPE,
                    size: ENCLAVE_REPORT_BYTE_LEN as u32,
                },
                Report::TD10(_) => Body {
                    body_type: BODY_TD_REPORT10_TYPE,
                    size: TD_REPORT10_BYTE_LEN as u32,
                },
                Report::TD15(_) => Body {
                    body_type: BODY_TD_REPORT15_TYPE,
                    size: TD_REPORT15_BYTE_LEN as u32,
                },
            };
            body.encode_to(output);
        }

        // Encode report
        match &self.report {
            Report::SgxEnclave(report) => report.encode_to(output),
            Report::TD10(report) => report.encode_to(output),
            Report::TD15(report) => report.encode_to(output),
        }

        // Encode auth data with length prefix
        let mut auth_data_bytes = Vec::new();
        self.auth_data.encode_to(&mut auth_data_bytes);
        let auth_data_len = auth_data_bytes.len() as u32;
        auth_data_len.encode_to(output);
        output.write(&auth_data_bytes);
    }
}

impl Quote {
    /// Parse a TEE quote from a byte slice.
    pub fn parse(quote: &[u8]) -> Result<Self> {
        let mut input = quote;
        let quote = Quote::decode(&mut input)?;
        Ok(quote)
    }

    /// Get the raw certificate chain from the quote.
    pub fn raw_cert_chain(&self) -> Result<&[u8]> {
        let cert_data = match &self.auth_data {
            AuthData::V3(data) => &data.certification_data,
            AuthData::V4(data) => &data.qe_report_data.certification_data,
        };
        if cert_data.cert_type != 5 {
            bail!("Unsupported cert type: {}", cert_data.cert_type);
        }
        Ok(&cert_data.body.data)
    }

    /// Get the length of signed data in the quote.
    pub fn signed_length(&self) -> usize {
        let mut len = match self.report {
            Report::SgxEnclave(_) => HEADER_BYTE_LEN + ENCLAVE_REPORT_BYTE_LEN,
            Report::TD10(_) => HEADER_BYTE_LEN + TD_REPORT10_BYTE_LEN,
            Report::TD15(_) => HEADER_BYTE_LEN + TD_REPORT15_BYTE_LEN,
        };
        #[allow(clippy::arithmetic_side_effects)]
        if self.header.version == 5 {
            len += BODY_BYTE_SIZE;
        }
        len
    }

    /// Get the inner certification data type.
    /// For V3 quotes: returns the cert_type directly.
    /// For V4 quotes with cert_type 6: returns the inner cert_type from qe_report_data.
    pub fn inner_cert_type(&self) -> u16 {
        match &self.auth_data {
            AuthData::V3(data) => data.certification_data.cert_type,
            AuthData::V4(data) => data.qe_report_data.certification_data.cert_type,
        }
    }

    /// Get the inner certification data body.
    pub fn inner_cert_data(&self) -> &[u8] {
        match &self.auth_data {
            AuthData::V3(data) => &data.certification_data.body.data,
            AuthData::V4(data) => &data.qe_report_data.certification_data.body.data,
        }
    }

    /// Get the QE report bytes.
    pub fn qe_report(&self) -> &[u8; ENCLAVE_REPORT_BYTE_LEN] {
        match &self.auth_data {
            AuthData::V3(data) => &data.qe_report,
            AuthData::V4(data) => &data.qe_report_data.qe_report,
        }
    }

    /// Get the QE ID from the quote header.
    pub fn qeid(&self) -> &[u8] {
        &self.header.user_data[..16]
    }

    /// For cert_type 3 (encrypted PPID), extract the parameters needed to fetch PCK certificate.
    /// Returns (encrypted_ppid, cpusvn, pcesvn, pceid).
    pub fn encrypted_ppid_params(&self) -> Result<EncryptedPpidParams> {
        // The cert body for encrypted PPID contains:
        // - encrypted_ppid (variable length: 256 bytes for RSA-2048, 384 bytes for RSA-3072)
        // - cpusvn (16 bytes)
        // - pcesvn (2 bytes, little endian)
        // - pceid (2 bytes, little endian)
        // Total trailer: 20 bytes
        #[derive(Decode)]
        struct EncPpidDecoder<const N: usize> {
            encrypted_ppid: [u8; N],
            cpusvn: CpuSvn,
            pcesvn: Svn,
            pceid: [u8; 2],
        }
        impl<const N: usize> EncPpidDecoder<N> {
            fn into_params(self) -> EncryptedPpidParams {
                EncryptedPpidParams {
                    encrypted_ppid: self.encrypted_ppid.to_vec(),
                    cpusvn: self.cpusvn,
                    pcesvn: self.pcesvn,
                    pceid: self.pceid,
                }
            }
        }

        let mut cert_body = self.inner_cert_data();
        let params = match self.inner_cert_type() {
            PCK_ID_ENCRYPTED_PPID_2048 => EncPpidDecoder::<256>::decode(&mut cert_body)
                .context("Failed to decode ENCRYPTED_PPID_2048")?
                .into_params(),
            PCK_ID_ENCRYPTED_PPID_3072 => EncPpidDecoder::<384>::decode(&mut cert_body)
                .context("Failed to decode ENCRYPTED_PPID_3072")?
                .into_params(),
            other => bail!("encrypted_ppid_params() requires cert_type 2 or 3, got {other}"),
        };
        Ok(params)
    }
}

/// Parameters extracted from a quote with cert_type 2/3 (encrypted PPID).
/// Used to fetch PCK certificate from PCCS.
#[derive(Debug, Clone)]
pub struct EncryptedPpidParams {
    /// The encrypted PPID (256 bytes for RSA-2048, 384 bytes for RSA-3072).
    pub encrypted_ppid: Vec<u8>,
    /// CPU SVN from certification data trailer (16 bytes).
    pub cpusvn: [u8; 16],
    /// PCE SVN from certification data trailer.
    pub pcesvn: u16,
    /// PCE ID from certification data trailer (2 bytes).
    pub pceid: [u8; 2],
}
