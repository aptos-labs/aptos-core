use anyhow::Result;
use serde::{Deserialize, Serialize};

use {
    crate::constants::*,
    crate::qe_identity::QeTcbLevel,
    crate::quote::{EnclaveReport, Header, Report},
    crate::tcb_info::{TcbLevel, TcbStatus},
    alloc::string::String,
    alloc::vec::Vec,
};

#[cfg(feature = "borsh_schema")]
use borsh::BorshSchema;
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};

mod simple;
pub use simple::{QuotePolicy, QuotePolicyConfig};

/// Policy trait for customizing quote verification behavior.
///
/// Implement this trait to define custom validation logic for [`QuoteClaims`].
/// The library provides [`QuotePolicy`] as a comprehensive built-in implementation
/// that covers all common checks from Intel's Appraisal framework.
///
/// For most use cases, [`QuotePolicy`] with its builder methods is sufficient:
/// ```no_run
/// use dcap_qvl::QuotePolicy;
/// use dcap_qvl::TcbStatus;
/// use core::time::Duration;
///
/// let now_unix_secs = 1_700_000_000u64;
///
/// let policy = QuotePolicy::strict(now_unix_secs)
///     .allow_status(TcbStatus::OutOfDate)
///     .platform_grace_period(Duration::from_secs(90 * 24 * 3600))
///     .reject_advisory("INTEL-SA-00334");
/// ```
///
/// Implement this trait directly only for logic that [`QuotePolicy`] cannot express.
pub trait Policy {
    /// Validate verified quote claims against this policy.
    ///
    /// Return `Ok(())` to accept, or `Err(...)` to reject.
    fn validate(&self, data: &QuoteClaims) -> Result<()>;
}

/// PCK certificate flag, matching Intel's `pck_cert_flag_enum_t`.
///
/// These flags are only present in PCK certificates issued by the **Platform CA**.
/// For Processor CA certificates, the value is [`Undefined`](PckCertFlag::Undefined).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
#[cfg_attr(feature = "borsh", borsh(use_discriminant = true))]
pub enum PckCertFlag {
    /// The flag is explicitly false (ASN.1 BOOLEAN FALSE).
    False = 0,
    /// The flag is explicitly true (ASN.1 BOOLEAN TRUE).
    True = 1,
    /// The flag is not present in the certificate (Processor CA certs).
    Undefined = 2,
}

impl From<Option<bool>> for PckCertFlag {
    fn from(v: Option<bool>) -> Self {
        match v {
            Some(true) => PckCertFlag::True,
            Some(false) => PckCertFlag::False,
            None => PckCertFlag::Undefined,
        }
    }
}

/// Detailed, serializable claims produced by cryptographic quote verification.
///
/// Organized into structured sub-groups:
/// - [`tcb`](Self::tcb): Merged TCB verdict
/// - [`platform`](Self::platform): Platform-level details from PCK certificate and TCB matching
/// - [`qe`](Self::qe): QE (Quoting Enclave) verification results
///
/// Also includes the collateral time window (8 sources: TCBInfo, QEIdentity, 2 CRLs,
/// 4 certificate chains) and the quote report body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteClaims {
    /// Version of this claims schema.
    pub claims_version: u16,
    /// Authenticated quote header, including quote version, key type, TEE type,
    /// QE/PCE SVN, QE vendor ID, and user data.
    pub header: Header,
    /// TEE type: `0x00000000` for SGX, `0x00000081` for TDX.
    pub tee_type: u32,
    /// Merged TCB verdict (worst of platform + QE).
    pub tcb: TcbVerdict,
    /// Platform verification details.
    pub platform: PlatformInfo,
    /// QE verification details.
    pub qe: QeInfo,
    /// `min(issueDate / thisUpdate / notBefore)` across all 8 collateral sources.
    pub earliest_issue_date: u64,
    /// `max(issueDate / thisUpdate / notBefore)` across all 8 collateral sources.
    pub latest_issue_date: u64,
    /// `min(nextUpdate / notAfter)` across all 8 collateral sources (the "weakest link").
    pub earliest_expiration_date: u64,
    /// `min(issueDate / notBefore)` across QE Identity sources (issuer chain + JSON).
    pub qe_iden_earliest_issue_date: u64,
    /// `max(issueDate / notBefore)` across QE Identity sources (issuer chain + JSON).
    pub qe_iden_latest_issue_date: u64,
    /// `min(nextUpdate / notAfter)` across QE Identity sources (issuer chain + JSON).
    pub qe_iden_earliest_expiration_date: u64,
    /// Quote report body (SGX enclave report, TDX TD10/TD15).
    pub report: Report,
}

/// Merged TCB verdict from platform and QE status convergence.
///
/// Uses Intel's `convergeTcbStatusWithQeTcbStatus` logic to produce the
/// worst-case status and union of advisory IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TcbVerdict {
    /// Merged TCB status (worst of platform TCB + QE TCB).
    pub status: TcbStatus,
    /// Merged advisory IDs (union of platform + QE advisories).
    pub advisory_ids: Vec<String>,
    /// Lower of TCBInfo and QEIdentity `tcbEvaluationDataNumber` values.
    pub eval_data_number: u32,
}

/// Platform-level verification results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// The matched platform TCB level (unmerged).
    pub tcb_level: TcbLevel,
    /// Platform TCB level date as unix timestamp (precomputed from `tcb_level.tcb_date`).
    pub tcb_date_tag: u64,
    /// PCK certificate identity fields.
    pub pck: PckIdentity,
    /// SHA-384 of root CA's raw public key bytes, matching Intel's `root_key_id`.
    pub root_key_id: Vec<u8>,
    /// CRL number from PCK Certificate Revocation List.
    pub pck_crl_num: u32,
    /// CRL number from Root CA Certificate Revocation List.
    pub root_ca_crl_num: u32,
}

/// QE (Quoting Enclave) verification results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QeInfo {
    /// The matched QE TCB level (unmerged).
    pub tcb_level: QeTcbLevel,
    /// The QE's enclave report.
    pub report: EnclaveReport,
    /// TCB evaluation data number from QE Identity (unmerged).
    pub tcb_eval_data_number: u32,
}

/// PCK certificate identity fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PckIdentity {
    /// Platform Provisioning ID (PPID).
    pub ppid: Vec<u8>,
    /// CPU Security Version Number (16 bytes).
    pub cpu_svn: CpuSvn,
    /// PCE ISV Security Version Number.
    pub pce_svn: Svn,
    /// PCE ID (raw value from the PCK certificate SGX extension).
    pub pce_id: Vec<u8>,
    /// FMSPC (6 bytes).
    pub fmspc: Fmspc,
    /// SGX type: 0=Standard, 1=Scalable, 2=ScalableWithIntegrity.
    pub sgx_type: u8,
    /// Platform Instance ID (16 bytes, Platform CA only).
    pub platform_instance_id: Option<[u8; 16]>,
    /// Dynamic platform flag.
    pub dynamic_platform: PckCertFlag,
    /// Cached keys flag.
    pub cached_keys: PckCertFlag,
    /// SMT (hyperthreading) flag.
    pub smt_enabled: PckCertFlag,
    /// Platform Provider ID (Platform CA only).
    ///
    /// Note: Intel's upstream appraisal schema includes this field, but the
    /// upstream QvE measurement producer currently leaves it unpopulated
    /// (`//obj_plat_tcb.AddMember("platform_provider_id", , allocator);`).
    ///
    /// Upstream references:
    /// - QvE TODO:
    ///   <https://github.com/intel/confidential-computing.tee.dcap/blob/main/ae/QvE/qve/qve.cpp>
    pub platform_provider_id: Option<String>,
}
