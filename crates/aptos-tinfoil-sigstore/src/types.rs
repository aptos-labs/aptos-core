//! Measurement types recovered from an authenticated in-toto statement.
//!
//! Adapter for the vendored tinfoil-rs modules, which were written against
//! their parent crate's `attestation::types`. The register ordering below is
//! load-bearing and matches upstream `verifier/measurement`.

/// The in-toto predicate type a measurement was recovered from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateType {
    /// `https://tinfoil.sh/predicate/snp-tdx-multiplatform/v1`
    ///
    /// The only predicate type Tinfoil's code provenance uses. Registers are
    /// `[snp_measurement, rtmr1, rtmr2]`.
    SnpTdxMultiPlatformV1,
}

impl PredicateType {
    /// The predicate type URI, as it appears in the statement.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SnpTdxMultiPlatformV1 => SNP_TDX_MULTIPLATFORM_V1,
        }
    }
}

impl std::fmt::Display for PredicateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The predicate type URI for multiplatform code measurements.
pub const SNP_TDX_MULTIPLATFORM_V1: &str = "https://tinfoil.sh/predicate/snp-tdx-multiplatform/v1";

/// A code measurement recovered from an authenticated provenance statement.
///
/// # Register semantics
///
/// For [`PredicateType::SnpTdxMultiPlatformV1`], `registers` is exactly:
///
/// ```text
/// registers[0]  snp_measurement  (SEV-SNP launch measurement)
/// registers[1]  rtmr1            (TDX: kernel)
/// registers[2]  rtmr2            (TDX: initrd + kernel command line)
/// ```
///
/// **This covers only the code half of a TDX image.** `MRTD` (firmware) and
/// `RTMR0` are *not* here: upstream resolves those from the signed platform
/// endorsement artifact, keyed by PPID and constrained by VM shape. A verifier
/// that treats this measurement as the whole expected image leaves the firmware
/// unconstrained.
///
/// `RTMR3` is never measured by the boot chain and must be 48 zero bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measurement {
    pub type_: PredicateType,
    /// Lowercase-hex registers, in the order documented above.
    pub registers: Vec<String>,
}

impl Measurement {
    /// The TDX `RTMR1` value (kernel), if this measurement carries one.
    pub fn rtmr1(&self) -> Option<&str> {
        self.registers.get(1).map(String::as_str)
    }

    /// The TDX `RTMR2` value (initrd + command line), if present.
    pub fn rtmr2(&self) -> Option<&str> {
        self.registers.get(2).map(String::as_str)
    }

    /// The SEV-SNP launch measurement, if present.
    pub fn snp_measurement(&self) -> Option<&str> {
        self.registers.first().map(String::as_str)
    }
}
