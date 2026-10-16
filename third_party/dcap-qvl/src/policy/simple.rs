use core::time::Duration;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use {
    super::{PckCertFlag, Policy, QuoteClaims},
    crate::tcb_info::TcbStatus,
    crate::utils::parse_rfc3339_unix_secs,
    alloc::string::String,
    alloc::vec::Vec,
};

/// Built-in verification policy with builder pattern.
///
/// Covers common appraisal checks without embedding a policy engine.
/// without requiring a Rego engine. Strict by default: only `UpToDate`,
/// no grace period, no advisory blacklist.
///
/// # Example
/// ```no_run
/// use dcap_qvl::QuotePolicy;
/// use dcap_qvl::TcbStatus;
///
/// let now = 1_700_000_000u64; // unix timestamp
///
/// // Strict: only UpToDate, collateral must not be expired
/// let policy = QuotePolicy::strict(now);
///
/// // Allow an out-of-date platform for 90 days after its TCB level date.
/// use core::time::Duration;
/// let policy = QuotePolicy::strict(now)
///     .allow_status(TcbStatus::OutOfDate)
///     .platform_grace_period(Duration::from_secs(90 * 24 * 3600))
///     .reject_advisory("INTEL-SA-00334");
/// ```
#[derive(Clone, Debug)]
pub struct QuotePolicy {
    claims_only: bool,
    acceptable_statuses: u8,

    // Current time + TCB grace periods (default 0 = no tolerance)
    now: u64,
    platform_grace_period: u64,
    qe_grace_period: u64,

    // TCB evaluation
    min_tcb_eval_data_number: Option<u32>,

    // Advisory blacklist (quote is rejected if any advisory is in this set)
    rejected_advisory_ids: Vec<String>,

    // Platform flags (default false = reject if True)
    allow_dynamic_platform: bool,
    allow_cached_keys: bool,
    allow_smt: bool,

    // SGX type whitelist (None = skip check)
    accepted_sgx_types: Option<Vec<u8>>,
}

impl QuotePolicy {
    const UP_TO_DATE: u8 = 1 << 0;
    const SW_HARDENING_NEEDED: u8 = 1 << 1;
    const CONFIGURATION_NEEDED: u8 = 1 << 2;
    const CONFIGURATION_AND_SW_HARDENING_NEEDED: u8 = 1 << 3;
    const OUT_OF_DATE: u8 = 1 << 4;
    const OUT_OF_DATE_CONFIGURATION_NEEDED: u8 = 1 << 5;

    fn status_to_flag(status: TcbStatus) -> u8 {
        match status {
            TcbStatus::UpToDate => Self::UP_TO_DATE,
            TcbStatus::SWHardeningNeeded => Self::SW_HARDENING_NEEDED,
            TcbStatus::ConfigurationNeeded => Self::CONFIGURATION_NEEDED,
            TcbStatus::ConfigurationAndSWHardeningNeeded => {
                Self::CONFIGURATION_AND_SW_HARDENING_NEEDED
            }
            TcbStatus::OutOfDate => Self::OUT_OF_DATE,
            TcbStatus::OutOfDateConfigurationNeeded => Self::OUT_OF_DATE_CONFIGURATION_NEEDED,
            TcbStatus::Revoked => 0,
        }
    }

    fn new_with_statuses(now: u64, acceptable_statuses: u8) -> Self {
        Self {
            claims_only: false,
            acceptable_statuses,
            now,
            platform_grace_period: 0,
            qe_grace_period: 0,
            min_tcb_eval_data_number: None,
            rejected_advisory_ids: Vec::new(),
            allow_dynamic_platform: false,
            allow_cached_keys: false,
            allow_smt: false,
            accepted_sgx_types: None,
        }
    }

    /// Create a strict policy: only `UpToDate` status is accepted,
    /// no grace period, no advisory blacklist.
    pub fn strict(now_secs: u64) -> Self {
        Self::new_with_statuses(now_secs, Self::UP_TO_DATE)
    }

    /// Create a policy that only supplies the trusted verification time.
    ///
    /// Use this when policy appraisal is performed by a downstream engine.
    pub fn claims_only(now_secs: u64) -> Self {
        let mut policy = Self::new_with_statuses(now_secs, 0);
        policy.claims_only = true;
        policy
    }

    /// Allow an additional TCB status.
    pub fn allow_status(mut self, status: TcbStatus) -> Self {
        self.acceptable_statuses |= Self::status_to_flag(status);
        self
    }

    /// Set platform grace period (default: zero). When TCB status is
    /// OutOfDate or OutOfDateConfigurationNeeded, accepts quotes where
    /// `tcb_level_date_tag + grace_period >= now`. Skipped for UpToDate/ConfigNeeded/SWHardening.
    pub fn platform_grace_period(mut self, duration: Duration) -> Self {
        self.platform_grace_period = duration.as_secs();
        self
    }

    /// Set QE grace period (default: zero). When QE TCB status is `OutOfDate`,
    /// accepts quotes where `qe_tcb_level.tcb_date + grace_period >= now`.
    pub fn qe_grace_period(mut self, duration: Duration) -> Self {
        self.qe_grace_period = duration.as_secs();
        self
    }

    /// Set minimum TCB evaluation data number. Rejects quotes with
    /// `tcb_eval_data_number` below this threshold.
    pub fn min_tcb_eval_data_number(mut self, min: u32) -> Self {
        self.min_tcb_eval_data_number = Some(min);
        self
    }

    /// Reject a specific advisory ID. Quotes containing any advisory in the
    /// rejected set fail validation. By default the set is empty, allowing all
    /// advisory IDs.
    pub fn reject_advisory(mut self, id: impl Into<String>) -> Self {
        self.rejected_advisory_ids.push(id.into());
        self
    }

    /// Reject multiple advisory IDs at once.
    pub fn reject_advisories(mut self, ids: &[impl AsRef<str>]) -> Self {
        self.rejected_advisory_ids
            .extend(ids.iter().map(|id| id.as_ref().to_string()));
        self
    }

    /// Set whether dynamic platforms are allowed. If `false` (default), rejects
    /// quotes where `dynamic_platform` is `True`.
    pub fn allow_dynamic_platform(mut self, allow: bool) -> Self {
        self.allow_dynamic_platform = allow;
        self
    }

    /// Set whether cached keys are allowed. If `false` (default), rejects
    /// quotes where `cached_keys` is `True`.
    pub fn allow_cached_keys(mut self, allow: bool) -> Self {
        self.allow_cached_keys = allow;
        self
    }

    /// Set whether SMT (simultaneous multithreading / hyperthreading) is allowed.
    /// If `false` (default), rejects quotes where `smt_enabled` is `True`.
    pub fn allow_smt(mut self, allow: bool) -> Self {
        self.allow_smt = allow;
        self
    }

    /// Set accepted SGX types (0=Standard, 1=Scalable, 2=ScalableWithIntegrity).
    /// Rejects quotes with `sgx_type` not in this list. Default: skip check.
    pub fn accepted_sgx_types(mut self, types: &[u8]) -> Self {
        self.accepted_sgx_types = Some(types.to_vec());
        self
    }

    /// Check if a TCB status is acceptable according to this policy.
    pub fn is_status_acceptable(&self, status: TcbStatus) -> bool {
        let flag = Self::status_to_flag(status);
        (self.acceptable_statuses & flag) != 0
    }
}

impl Policy for QuotePolicy {
    fn validate(&self, data: &QuoteClaims) -> Result<()> {
        if self.claims_only {
            return Ok(());
        }
        fn within_grace(date_tag: u64, grace_period: u64, now: u64) -> bool {
            date_tag.saturating_add(grace_period) >= now
        }

        fn advisory_rejected(rejected_advisory_ids: &[String], id: &str) -> bool {
            rejected_advisory_ids
                .iter()
                .any(|a| a.eq_ignore_ascii_case(id))
        }

        // 1. TCB status whitelist
        if !self.is_status_acceptable(data.tcb.status) {
            bail!(
                "TCB status {:?} is not acceptable by policy",
                data.tcb.status
            );
        }

        // 3. Collateral expiration. The verification pipeline already enforces
        // this at the same trusted time; retain the check for delayed appraisal.
        if data.earliest_expiration_date < self.now {
            bail!(
                "Collateral expired: earliest_expiration {} < now {}",
                data.earliest_expiration_date,
                self.now
            );
        }

        // 4. Platform TCB freshness: platform tcb_date_tag + grace >= now.
        let platform_is_out_of_date = matches!(
            data.platform.tcb_level.tcb_status,
            TcbStatus::OutOfDate | TcbStatus::OutOfDateConfigurationNeeded
        );
        let platform_in_grace = platform_is_out_of_date
            && within_grace(
                data.platform.tcb_date_tag,
                self.platform_grace_period,
                self.now,
            );
        if platform_is_out_of_date && !platform_in_grace {
            bail!(
                "Platform TCB too old: tcb_date_tag {} + grace {} < now {}",
                data.platform.tcb_date_tag,
                self.platform_grace_period,
                self.now
            );
        }

        // 4b. QE TCB freshness: QE tcb_date + grace >= now.
        let qe_tcb_date_tag = parse_rfc3339_unix_secs(&data.qe.tcb_level.tcb_date)
            .map_err(|e| anyhow::anyhow!("Failed to parse QE TCB date: {e}"))?;
        let qe_is_out_of_date = data.qe.tcb_level.tcb_status == TcbStatus::OutOfDate;
        let qe_in_grace =
            qe_is_out_of_date && within_grace(qe_tcb_date_tag, self.qe_grace_period, self.now);
        if qe_is_out_of_date && !qe_in_grace {
            bail!(
                "QE TCB too old: tcb_date_tag {} + grace {} < now {}",
                qe_tcb_date_tag,
                self.qe_grace_period,
                self.now
            );
        }

        // 2. Advisory ID blacklist.
        for id in &data.platform.tcb_level.advisory_ids {
            if advisory_rejected(&self.rejected_advisory_ids, id) {
                bail!("Advisory ID {id} is rejected by policy");
            }
        }
        for id in &data.qe.tcb_level.advisory_ids {
            if advisory_rejected(&self.rejected_advisory_ids, id) {
                bail!("Advisory ID {id} is rejected by policy");
            }
        }

        // 5. Minimum TCB evaluation data number
        if let Some(min) = self.min_tcb_eval_data_number {
            if data.tcb.eval_data_number < min {
                bail!(
                    "TCB eval data number {} is below minimum {}",
                    data.tcb.eval_data_number,
                    min
                );
            }
        }

        // 6. Dynamic platform flag
        if !self.allow_dynamic_platform && data.platform.pck.dynamic_platform == PckCertFlag::True {
            bail!("Dynamic platform is not allowed by policy");
        }

        // 7. Cached keys flag
        if !self.allow_cached_keys && data.platform.pck.cached_keys == PckCertFlag::True {
            bail!("Cached keys are not allowed by policy");
        }

        // 8. SMT flag
        if !self.allow_smt && data.platform.pck.smt_enabled == PckCertFlag::True {
            bail!("SMT (hyperthreading) is not allowed by policy");
        }

        // 9. SGX type whitelist
        if let Some(ref types) = self.accepted_sgx_types {
            if !types.contains(&data.platform.pck.sgx_type) {
                bail!(
                    "SGX type {} is not in accepted types {:?}",
                    data.platform.pck.sgx_type,
                    types
                );
            }
        }

        Ok(())
    }
}

/// JSON-serializable configuration for [`QuotePolicy`].
///
/// All fields default to the strict values (zero / empty / false).
/// Pass as JSON from FFI (Go, Python) to configure verification policy.
///
/// Unknown fields are rejected so that a stale or misspelled option in a
/// security policy fails loudly instead of being silently ignored.
///
/// ```json
/// {
///   "allowed_statuses": ["UpToDate", "SWHardeningNeeded"],
///   "rejected_advisory_ids": ["INTEL-SA-00334"],
///   "platform_grace_period_secs": 2592000,
///   "allow_smt": true
/// }
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuotePolicyConfig {
    #[serde(default)]
    pub allowed_statuses: Vec<TcbStatus>,
    #[serde(default)]
    pub rejected_advisory_ids: Vec<String>,
    #[serde(default)]
    pub platform_grace_period_secs: u64,
    #[serde(default)]
    pub qe_grace_period_secs: u64,
    #[serde(default)]
    pub min_tcb_eval_data_number: u32,
    #[serde(default)]
    pub allow_dynamic_platform: bool,
    #[serde(default)]
    pub allow_cached_keys: bool,
    #[serde(default)]
    pub allow_smt: bool,
    #[serde(default)]
    pub accepted_sgx_types: Option<Vec<u8>>,
}

impl QuotePolicyConfig {
    /// Build a [`QuotePolicy`] from this config + current timestamp.
    ///
    /// Default config (all fields zero/empty) produces `QuotePolicy::strict(now)`.
    pub fn into_policy(self, now_secs: u64) -> QuotePolicy {
        let mut policy = if self.allowed_statuses.is_empty() {
            QuotePolicy::strict(now_secs)
        } else {
            let mut p = QuotePolicy::new_with_statuses(now_secs, 0);
            for status in self.allowed_statuses {
                p = p.allow_status(status);
            }
            p
        };
        for id in self.rejected_advisory_ids {
            policy = policy.reject_advisory(id);
        }
        if self.platform_grace_period_secs > 0 {
            policy =
                policy.platform_grace_period(Duration::from_secs(self.platform_grace_period_secs));
        }
        if self.qe_grace_period_secs > 0 {
            policy = policy.qe_grace_period(Duration::from_secs(self.qe_grace_period_secs));
        }
        if self.min_tcb_eval_data_number > 0 {
            policy = policy.min_tcb_eval_data_number(self.min_tcb_eval_data_number);
        }
        policy = policy.allow_dynamic_platform(self.allow_dynamic_platform);
        policy = policy.allow_cached_keys(self.allow_cached_keys);
        policy = policy.allow_smt(self.allow_smt);
        if let Some(types) = self.accepted_sgx_types {
            policy = policy.accepted_sgx_types(&types);
        }
        policy
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::policy::{PckIdentity, PlatformInfo, QeInfo, TcbVerdict};
    use crate::tcb_info::TcbStatus::*;

    fn make_test_claims(tcb_status: TcbStatus) -> QuoteClaims {
        use crate::qe_identity::{QeTcb, QeTcbLevel};
        use crate::tcb_info::{Tcb, TcbComponents, TcbLevel};

        QuoteClaims {
            claims_version: 1,
            header: crate::quote::Header {
                version: 3,
                attestation_key_type: 2,
                tee_type: 0,
                qe_svn: 0,
                pce_svn: 0,
                qe_vendor_id: [0; 16],
                user_data: [0; 20],
            },
            tee_type: 0,
            tcb: TcbVerdict {
                status: tcb_status,
                advisory_ids: vec![],
                eval_data_number: 17,
            },
            platform: PlatformInfo {
                tcb_level: TcbLevel {
                    tcb: Tcb {
                        sgx_components: vec![TcbComponents { svn: 0 }; 16],
                        tdx_components: vec![],
                        pce_svn: 13,
                    },
                    tcb_date: "2023-07-22T00:00:00Z".to_string(),
                    tcb_status,
                    advisory_ids: vec![],
                },
                tcb_date_tag: 1_690_000_000, // ~2023-07-22
                pck: PckIdentity {
                    ppid: vec![0u8; 16],
                    cpu_svn: [0u8; 16],
                    pce_svn: 13,
                    pce_id: vec![0u8; 2],
                    fmspc: [0u8; 6],
                    sgx_type: 0,
                    platform_instance_id: None,
                    dynamic_platform: PckCertFlag::Undefined,
                    cached_keys: PckCertFlag::Undefined,
                    smt_enabled: PckCertFlag::Undefined,
                    platform_provider_id: None,
                },
                root_key_id: vec![0u8; 48],
                pck_crl_num: 1,
                root_ca_crl_num: 1,
            },
            qe: QeInfo {
                tcb_level: QeTcbLevel {
                    tcb: QeTcb { isvsvn: 8 },
                    tcb_date: "2024-03-13T00:00:00Z".to_string(),
                    tcb_status: UpToDate,
                    advisory_ids: vec![],
                },
                report: crate::quote::EnclaveReport {
                    cpu_svn: [0u8; 16],
                    misc_select: 0,
                    reserved1: [0u8; 28],
                    attributes: [0u8; 16],
                    mr_enclave: [0u8; 32],
                    reserved2: [0u8; 32],
                    mr_signer: [0u8; 32],
                    reserved3: [0u8; 96],
                    isv_prod_id: 1,
                    isv_svn: 8,
                    reserved4: [0u8; 60],
                    report_data: [0u8; 64],
                },
                tcb_eval_data_number: 17,
            },
            report: crate::quote::Report::SgxEnclave(crate::quote::EnclaveReport {
                cpu_svn: [0u8; 16],
                misc_select: 0,
                reserved1: [0u8; 28],
                attributes: [0u8; 16],
                mr_enclave: [0u8; 32],
                reserved2: [0u8; 32],
                mr_signer: [0u8; 32],
                reserved3: [0u8; 96],
                isv_prod_id: 0,
                isv_svn: 0,
                reserved4: [0u8; 60],
                report_data: [0u8; 64],
            }),
            earliest_issue_date: 1_690_000_000,
            latest_issue_date: 1_690_100_000,
            earliest_expiration_date: 1_703_000_000, // ~2023-12-19
            qe_iden_earliest_issue_date: 1_690_000_000,
            qe_iden_latest_issue_date: 1_690_100_000,
            qe_iden_earliest_expiration_date: 1_703_000_000,
        }
    }

    // -- TCB status checks --

    #[test]
    fn policy_strict_accepts_up_to_date() {
        let data = make_test_claims(UpToDate);
        let policy = QuotePolicy::strict(1_702_000_000); // within collateral window
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_strict_rejects_sw_hardening() {
        let data = make_test_claims(SWHardeningNeeded);
        let policy = QuotePolicy::strict(1_702_000_000);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("TCB status"), "{err}");
    }

    #[test]
    fn policy_out_of_date_with_fresh_tcb_date_accepts() {
        let mut data = make_test_claims(OutOfDate);
        data.platform.tcb_date_tag = 1_702_000_000;
        let policy = QuotePolicy::strict(1_702_000_000).allow_status(OutOfDate);
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_allow_status_builder() {
        let data = make_test_claims(SWHardeningNeeded);
        let policy = QuotePolicy::strict(1_702_000_000).allow_status(SWHardeningNeeded);
        assert!(policy.validate(&data).is_ok());
    }

    // -- Advisory ID blacklist --

    #[test]
    fn policy_allows_advisory_when_not_blacklisted() {
        let mut data = make_test_claims(UpToDate);
        data.tcb.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        data.platform.tcb_level.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        let policy = QuotePolicy::strict(1_702_000_000);
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_rejects_blacklisted_advisory() {
        let mut data = make_test_claims(UpToDate);
        data.tcb.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        data.platform.tcb_level.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        let policy = QuotePolicy::strict(1_702_000_000).reject_advisory("INTEL-SA-00615");
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("INTEL-SA-00615"), "{err}");
    }

    #[test]
    fn policy_advisory_blacklist_case_insensitive() {
        let mut data = make_test_claims(UpToDate);
        data.tcb.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        data.platform.tcb_level.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        let policy = QuotePolicy::strict(1_702_000_000).reject_advisory("intel-sa-00615");
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("INTEL-SA-00615"), "{err}");
    }

    #[test]
    fn policy_reject_advisories_batch() {
        let mut data = make_test_claims(UpToDate);
        data.tcb.advisory_ids = vec!["INTEL-SA-00820".to_string()];
        data.platform.tcb_level.advisory_ids = vec!["INTEL-SA-00820".to_string()];
        let policy = QuotePolicy::strict(1_702_000_000)
            .reject_advisories(&["INTEL-SA-00615", "INTEL-SA-00820"]);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("INTEL-SA-00820"), "{err}");
    }

    #[test]
    fn policy_empty_advisories_passes() {
        let data = make_test_claims(UpToDate);
        assert!(data.tcb.advisory_ids.is_empty());
        let policy = QuotePolicy::strict(1_702_000_000);
        assert!(policy.validate(&data).is_ok());
    }

    // -- Collateral expiration --

    #[test]
    fn policy_collateral_expired_rejects() {
        let data = make_test_claims(UpToDate);
        let policy = QuotePolicy::strict(1_704_000_000);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("Collateral expired"), "{err}");
    }

    #[test]
    fn policy_collateral_not_expired_passes() {
        let data = make_test_claims(UpToDate);
        let policy = QuotePolicy::strict(1_702_000_000);
        assert!(policy.validate(&data).is_ok());
    }

    // -- Platform grace period --

    #[test]
    fn policy_platform_grace_skipped_for_up_to_date() {
        let data = make_test_claims(UpToDate);
        let policy = QuotePolicy::strict(1_702_000_000);
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_platform_grace_skipped_for_sw_hardening() {
        let data = make_test_claims(SWHardeningNeeded);
        let policy = QuotePolicy::strict(1_702_000_000).allow_status(SWHardeningNeeded);
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_platform_grace_skipped_for_config_needed() {
        let data = make_test_claims(ConfigurationNeeded);
        let policy = QuotePolicy::strict(1_702_000_000).allow_status(ConfigurationNeeded);
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_platform_grace_checked_for_out_of_date_rejects() {
        let data = make_test_claims(OutOfDate);
        let policy = QuotePolicy::strict(1_702_000_000).allow_status(OutOfDate);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("Platform TCB too old"), "{err}");
    }

    #[test]
    fn policy_platform_grace_checked_for_out_of_date_accepts_with_grace() {
        let data = make_test_claims(OutOfDate);
        let policy = QuotePolicy::strict(1_702_000_000)
            .allow_status(OutOfDate)
            .platform_grace_period(Duration::from_secs(13_000_000));
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_platform_grace_too_short_rejects() {
        let data = make_test_claims(OutOfDate);
        let policy = QuotePolicy::strict(1_702_000_000)
            .allow_status(OutOfDate)
            .platform_grace_period(Duration::from_secs(11_000_000));
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("Platform TCB too old"), "{err}");
    }

    #[test]
    fn policy_platform_grace_checked_for_out_of_date_config_needed() {
        let data = make_test_claims(OutOfDateConfigurationNeeded);
        let policy = QuotePolicy::strict(1_702_000_000).allow_status(OutOfDateConfigurationNeeded);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("Platform TCB too old"), "{err}");
    }

    // -- min_tcb_eval_data_number --

    #[test]
    fn policy_min_eval_num_rejects_below() {
        let data = make_test_claims(UpToDate);
        assert_eq!(data.tcb.eval_data_number, 17);
        let policy = QuotePolicy::strict(1_702_000_000).min_tcb_eval_data_number(20);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("below minimum"), "{err}");
    }

    #[test]
    fn policy_min_eval_num_accepts_equal() {
        let data = make_test_claims(UpToDate);
        let policy = QuotePolicy::strict(1_702_000_000).min_tcb_eval_data_number(17);
        assert!(policy.validate(&data).is_ok());
    }

    // -- Platform flags --

    #[test]
    fn policy_rejects_dynamic_platform_true() {
        let mut data = make_test_claims(UpToDate);
        data.platform.pck.dynamic_platform = PckCertFlag::True;
        let policy = QuotePolicy::strict(1_702_000_000);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("Dynamic platform"), "{err}");
    }

    #[test]
    fn policy_allows_dynamic_platform_when_configured() {
        let mut data = make_test_claims(UpToDate);
        data.platform.pck.dynamic_platform = PckCertFlag::True;
        let policy = QuotePolicy::strict(1_702_000_000).allow_dynamic_platform(true);
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_undefined_platform_flags_pass() {
        let data = make_test_claims(UpToDate);
        assert_eq!(data.platform.pck.dynamic_platform, PckCertFlag::Undefined);
        assert_eq!(data.platform.pck.cached_keys, PckCertFlag::Undefined);
        assert_eq!(data.platform.pck.smt_enabled, PckCertFlag::Undefined);
        let policy = QuotePolicy::strict(1_702_000_000);
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_rejects_smt_true() {
        let mut data = make_test_claims(UpToDate);
        data.platform.pck.smt_enabled = PckCertFlag::True;
        let policy = QuotePolicy::strict(1_702_000_000);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("SMT"), "{err}");
    }

    #[test]
    fn policy_rejects_cached_keys_true() {
        let mut data = make_test_claims(UpToDate);
        data.platform.pck.cached_keys = PckCertFlag::True;
        let policy = QuotePolicy::strict(1_702_000_000);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("Cached keys"), "{err}");
    }

    // -- SGX type whitelist --

    #[test]
    fn policy_sgx_type_not_configured_passes() {
        let data = make_test_claims(UpToDate);
        let policy = QuotePolicy::strict(1_702_000_000);
        assert!(policy.validate(&data).is_ok());
    }

    #[test]
    fn policy_sgx_type_whitelist_rejects() {
        let mut data = make_test_claims(UpToDate);
        data.platform.pck.sgx_type = 1;
        let policy = QuotePolicy::strict(1_702_000_000).accepted_sgx_types(&[0]);
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("SGX type"), "{err}");
    }

    #[test]
    fn policy_sgx_type_whitelist_accepts() {
        let mut data = make_test_claims(UpToDate);
        data.platform.pck.sgx_type = 1;
        let policy = QuotePolicy::strict(1_702_000_000).accepted_sgx_types(&[0, 1, 2]);
        assert!(policy.validate(&data).is_ok());
    }

    // -- Advisory blacklist during grace --

    #[test]
    fn policy_blacklist_checked_during_platform_grace() {
        let mut data = make_test_claims(OutOfDate);
        data.tcb.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        data.platform.tcb_level.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        let policy = QuotePolicy::strict(1_702_000_000)
            .allow_status(OutOfDate)
            .platform_grace_period(Duration::from_secs(13_000_000))
            .reject_advisory("INTEL-SA-00615");
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("INTEL-SA-00615"), "{err}");
    }

    #[test]
    fn policy_blacklist_checked_for_out_of_date_config_needed() {
        let mut data = make_test_claims(OutOfDateConfigurationNeeded);
        data.tcb.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        data.platform.tcb_level.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        let policy = QuotePolicy::strict(1_702_000_000)
            .allow_status(OutOfDateConfigurationNeeded)
            .platform_grace_period(Duration::from_secs(13_000_000))
            .reject_advisory("INTEL-SA-00615");
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("INTEL-SA-00615"), "{err}");
    }

    #[test]
    fn policy_blacklist_checked_without_grace() {
        let mut data = make_test_claims(UpToDate);
        data.tcb.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        data.platform.tcb_level.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        let policy = QuotePolicy::strict(1_702_000_000).reject_advisory("INTEL-SA-00615");
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("INTEL-SA-00615"), "{err}");
    }

    #[test]
    fn policy_platform_grace_does_not_cover_qe_out_of_date() {
        let mut data = make_test_claims(OutOfDate);
        data.platform.tcb_level.tcb_status = UpToDate;
        data.platform.tcb_level.advisory_ids = vec![];
        data.platform.tcb_date_tag = 1_702_000_000;
        data.qe.tcb_level.tcb_status = OutOfDate;
        data.qe.tcb_level.tcb_date = "2023-07-22T00:00:00Z".to_string();
        data.qe.tcb_level.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        data.tcb.advisory_ids = vec!["INTEL-SA-00615".to_string()];

        let policy = QuotePolicy::strict(1_702_000_000)
            .allow_status(OutOfDate)
            .platform_grace_period(Duration::from_secs(13_000_000));
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("QE TCB too old"), "{err}");
    }

    #[test]
    fn policy_qe_grace_accepts_qe_out_of_date() {
        let mut data = make_test_claims(OutOfDate);
        data.platform.tcb_level.tcb_status = UpToDate;
        data.platform.tcb_level.advisory_ids = vec![];
        data.qe.tcb_level.tcb_status = OutOfDate;
        data.qe.tcb_level.tcb_date = "2023-07-22T00:00:00Z".to_string();
        data.qe.tcb_level.advisory_ids = vec!["INTEL-SA-00615".to_string()];
        data.tcb.advisory_ids = vec!["INTEL-SA-00615".to_string()];

        let policy = QuotePolicy::strict(1_702_000_000)
            .allow_status(OutOfDate)
            .qe_grace_period(Duration::from_secs(13_000_000));
        assert!(policy.validate(&data).is_ok());
    }

    // -- QuotePolicyConfig --

    #[test]
    fn policy_config_default_is_strict() {
        let config: QuotePolicyConfig = serde_json::from_str("{}").unwrap();
        let policy = config.into_policy(1_702_000_000);
        assert!(policy.validate(&make_test_claims(UpToDate)).is_ok());
        let err = policy
            .validate(&make_test_claims(SWHardeningNeeded))
            .unwrap_err()
            .to_string();
        assert!(err.contains("not acceptable"), "{err}");
    }

    #[test]
    fn policy_config_into_policy_applies_fields() {
        let config: QuotePolicyConfig = serde_json::from_str(
            r#"{
                "allowed_statuses": ["UpToDate", "SWHardeningNeeded"],
                "rejected_advisory_ids": ["INTEL-SA-00334"],
                "allow_smt": true
            }"#,
        )
        .unwrap();
        let policy = config.into_policy(1_702_000_000);
        assert!(policy
            .validate(&make_test_claims(SWHardeningNeeded))
            .is_ok());

        let mut data = make_test_claims(UpToDate);
        data.platform.tcb_level.advisory_ids = vec!["intel-sa-00334".to_string()];
        let err = policy.validate(&data).unwrap_err().to_string();
        assert!(err.contains("rejected by policy"), "{err}");
    }

    #[test]
    fn policy_config_rejects_unknown_fields() {
        // A stale or misspelled option in a security policy must fail loudly.
        let result: Result<QuotePolicyConfig, _> =
            serde_json::from_str(r#"{"collateral_grace_period_secs": 2592000}"#);
        assert!(result.is_err());
    }
}
