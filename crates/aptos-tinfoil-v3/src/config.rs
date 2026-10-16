use serde::Deserialize;

/// Hardware-evidence policy used by the offline TDX verifier.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttestationConfig {
    /// Require an endorsed X25519 HPKE public key.
    #[serde(default = "default_true")]
    pub require_hpke_key: bool,
    /// Allowed TD launch measurements; empty requires explicit acknowledgement.
    #[serde(default)]
    pub allowed_mrtd: Vec<String>,
    /// Acknowledge accepting any hardware-authenticated launch measurement.
    #[serde(default)]
    pub allow_any_measurement: bool,
    /// Optional allowlist for RTMR0.
    #[serde(default)]
    pub allowed_rtmr0: Vec<String>,
    /// Optional allowlist for RTMR1.
    #[serde(default)]
    pub allowed_rtmr1: Vec<String>,
    /// Optional allowlist for RTMR2.
    #[serde(default)]
    pub allowed_rtmr2: Vec<String>,
    /// Optional allowlist for RTMR3.
    #[serde(default)]
    pub allowed_rtmr3: Vec<String>,
    /// TCB statuses accepted in addition to `UpToDate`.
    #[serde(default)]
    pub allowed_tcb_statuses: Vec<String>,
    /// Accept a PCK certificate marked as a dynamic platform.
    #[serde(default = "default_true")]
    pub allow_dynamic_platform: bool,
    /// Accept a PCK certificate marked as using cached provisioning keys.
    #[serde(default = "default_true")]
    pub allow_cached_keys: bool,
    /// Accept a platform with simultaneous multithreading enabled.
    #[serde(default = "default_true")]
    pub allow_smt: bool,
    /// Require freshness witnesses for Sigstore collateral.
    #[serde(default = "default_true")]
    pub require_freshness: bool,
    /// Require signed platform endorsements in addition to code provenance.
    #[serde(default = "default_true")]
    pub require_platform_endorsements: bool,
}

impl Default for AttestationConfig {
    fn default() -> Self {
        Self {
            require_hpke_key: true,
            allowed_mrtd: Vec::new(),
            allow_any_measurement: false,
            allowed_rtmr0: Vec::new(),
            allowed_rtmr1: Vec::new(),
            allowed_rtmr2: Vec::new(),
            allowed_rtmr3: Vec::new(),
            allowed_tcb_statuses: Vec::new(),
            allow_dynamic_platform: true,
            allow_cached_keys: true,
            allow_smt: true,
            require_freshness: true,
            require_platform_endorsements: true,
        }
    }
}

impl AttestationConfig {
    /// The four RTMR allowlists, indexed by register number.
    pub fn allowed_rtmrs(&self) -> [&Vec<String>; 4] {
        [
            &self.allowed_rtmr0,
            &self.allowed_rtmr1,
            &self.allowed_rtmr2,
            &self.allowed_rtmr3,
        ]
    }

    /// Convert the configured freshness requirement to verifier policy.
    pub fn freshness(&self) -> crate::provenance::Freshness {
        if self.require_freshness {
            crate::provenance::Freshness::Required
        } else {
            crate::provenance::Freshness::Optional
        }
    }

    /// Convert the platform-endorsement requirement to verifier policy.
    pub fn platform_endorsements(&self) -> crate::provenance::PlatformEndorsements {
        if self.require_platform_endorsements {
            crate::provenance::PlatformEndorsements::Required
        } else {
            crate::provenance::PlatformEndorsements::Optional
        }
    }
}

const fn default_true() -> bool {
    true
}
