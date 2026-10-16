//! Platform endorsements: which *machines* are allowed to run attested code,
//! and what their firmware is expected to measure.
//!
//! Port of `tinfoil-go/verifier/policy` (`feat/v3`).
//!
//! # Why this exists separately from code provenance
//!
//! Code provenance ([`crate::provenance`]) proves who built an image, and
//! yields the registers the *code* determines — `RTMR1` (kernel) and `RTMR2`
//! (initrd + command line). It says nothing about `MRTD` (firmware) or
//! `RTMR0`, because those depend on the host: the same image measures
//! differently on a differently shaped VM or a different QEMU/OVMF stack.
//!
//! This module supplies the other half. A signed endorsements artifact names
//! every machine permitted to host workloads (keyed by hardware identity), the
//! appraisal policy each machine must satisfy, and the `MRTD`/`RTMR0` values
//! expected for each VM shape.
//!
//! ```text
//! sigstore-platform  ->  Artifact
//!                          machines[PPID]      -> policy name    (is this host endorsed?)
//!                          policies[name].tdx  -> TDXPolicy      (appraisal rules)
//!                          measurements[ref]   -> MRTD, RTMR0    (per VM shape)
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::error::{Error, Result};

/// Required `format` of a platform-endorsements artifact.
pub const ARTIFACT_FORMAT: &str = "https://tinfoil.sh/predicate/platform-endorsements/v1";

/// The repository permitted to publish platform endorsements.
pub const PLATFORM_ENDORSEMENTS_REPO: &str = "tinfoilsh/platform-endorsements";

/// Platform label for AMD SEV-SNP policies.
pub const PLATFORM_SEV_SNP: &str = "sev-snp";
/// Platform label for Intel TDX policies.
pub const PLATFORM_TDX: &str = "tdx";

/// SEV-SNP machines are keyed by the 64-byte `CHIP_ID`.
const SEV_IDENTIFIER_HEX_LEN: usize = 128;
/// TDX machines are keyed by the 16-byte `PPID`.
const TDX_IDENTIFIER_HEX_LEN: usize = 32;

/// The canonical VM shape descriptor: the launch dimensions that determine a
/// platform measurement.
///
/// `disks` counts every attached disk (root, config, external config, one per
/// model). `gpus` is `None` when the dimension is unknown for a measured slug;
/// a code artifact always declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub cpus: u32,
    pub memory_mb: u64,
    #[serde(default)]
    pub gpus: Option<u32>,
    pub disks: u32,
}

impl Shape {
    /// Whether a measured slug shape (`self`) satisfies the shape a code
    /// artifact requires.
    ///
    /// `cpus`, `memory_mb` and `disks` are compared exactly. `gpus` is compared
    /// **only when both sides declare it** — a measured slug that does not know
    /// its GPU count must not be excluded on that basis.
    pub fn satisfies(&self, required: &Shape) -> bool {
        if self.cpus != required.cpus
            || self.memory_mb != required.memory_mb
            || self.disks != required.disks
        {
            return false;
        }
        match (self.gpus, required.gpus) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        }
    }
}

/// Host software that produced a measurement. Informational; never checked.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    #[serde(default)]
    pub qemu: Option<String>,
    #[serde(default)]
    pub ovmf: Option<String>,
}

/// One TDX platform configuration's expected registers, annotated with the VM
/// shape it was measured for.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMeasurement {
    pub mrtd: String,
    pub rtmr0: String,
    pub shape: Option<Shape>,
    #[serde(default)]
    pub stack: Option<Stack>,
}

/// A named appraisal policy. Exactly one platform block is set, matching
/// `platform`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub platform: String,
    /// SEV-SNP appraisal rules — parsed for *shape*, never interpreted.
    ///
    /// This vault is TDX-only, so nothing here appraises SEV evidence, and
    /// [`Artifact::policy_for`] refuses to hand a SEV policy to a TDX
    /// evidence lookup. The block is still modelled rather than held as an
    /// opaque `Value` so that the strictness applied everywhere else in this
    /// artifact applies inside it too: an unknown member is refused instead of
    /// silently ignored.
    ///
    /// That matters because the artifact is one signed document. A member this
    /// verifier does not recognize means the signer is asserting something it
    /// cannot evaluate, and the artifact is shared with verifiers that *do*
    /// appraise SEV. Accepting it quietly here would let the same signed bytes
    /// mean two different things.
    ///
    /// Field names track `verifier/policy/sev.go` in tinfoil-go. Values are
    /// deliberately untyped beyond what shape-checking needs: recording them
    /// as `serde_json::Value` keeps this a schema check and avoids implying
    /// the semantics are enforced.
    #[serde(default, skip_serializing)]
    pub sev_snp: Option<SevSnpPolicy>,
    #[serde(default)]
    pub tdx: Option<TdxPolicy>,
}

/// The SEV-SNP policy block, modelled for strict parsing only.
///
/// **Nothing in this crate appraises these values.** See [`Policy::sev_snp`].
/// Every member is optional because this type exists to reject what it does
/// *not* know, not to enforce what upstream requires — validating required
/// members would be claiming an appraisal this crate does not perform.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SevSnpPolicy {
    #[serde(default)]
    pub minimum_build: Option<serde_json::Value>,
    #[serde(default)]
    pub minimum_api_version: Option<serde_json::Value>,
    #[serde(default)]
    pub minimum_abi_version: Option<serde_json::Value>,
    #[serde(default)]
    pub minimum_guest_svn: Option<serde_json::Value>,
    #[serde(default)]
    pub minimum_tcb: Option<SevTcb>,
    #[serde(default)]
    pub minimum_launch_tcb: Option<SevTcb>,
    #[serde(default)]
    pub guest_policy: Option<SevGuestPolicy>,
    #[serde(default)]
    pub platform_info: Option<SevPlatformInfo>,
    #[serde(default)]
    pub permit_provisional_firmware: Option<serde_json::Value>,
    #[serde(default)]
    pub vmpl: Option<serde_json::Value>,
    #[serde(default)]
    pub host_data: Option<serde_json::Value>,
    #[serde(default)]
    pub image_id: Option<serde_json::Value>,
    #[serde(default)]
    pub family_id: Option<serde_json::Value>,
    #[serde(default)]
    pub require_author_key: Option<serde_json::Value>,
    #[serde(default)]
    pub require_id_block: Option<serde_json::Value>,
    #[serde(default)]
    pub minimum_launch_mitigation_vector: Option<serde_json::Value>,
    #[serde(default)]
    pub minimum_current_mitigation_vector: Option<serde_json::Value>,
}

/// AMD security patch levels (`minimum_tcb`, `minimum_launch_tcb`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SevTcb {
    /// Family 1Ah (Turin) parts only.
    #[serde(default)]
    pub fmc_spl: Option<serde_json::Value>,
    #[serde(default)]
    pub bl_spl: Option<serde_json::Value>,
    #[serde(default)]
    pub tee_spl: Option<serde_json::Value>,
    #[serde(default)]
    pub snp_spl: Option<serde_json::Value>,
    #[serde(default)]
    pub ucode_spl: Option<serde_json::Value>,
}

/// SNP guest policy bits.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SevGuestPolicy {
    #[serde(default)]
    pub debug: Option<serde_json::Value>,
    #[serde(default)]
    pub smt: Option<serde_json::Value>,
    #[serde(default)]
    pub migrate_ma: Option<serde_json::Value>,
    #[serde(default)]
    pub single_socket: Option<serde_json::Value>,
    #[serde(default)]
    pub cxl_allowed: Option<serde_json::Value>,
    #[serde(default)]
    pub mem_aes256_xts: Option<serde_json::Value>,
    #[serde(default)]
    pub rapl_dis: Option<serde_json::Value>,
    #[serde(default)]
    pub ciphertext_hiding_dram: Option<serde_json::Value>,
    #[serde(default)]
    pub page_swap_disable: Option<serde_json::Value>,
}

/// SNP `PLATFORM_INFO` expectations.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SevPlatformInfo {
    #[serde(default)]
    pub smt_enabled: Option<serde_json::Value>,
    #[serde(default)]
    pub tsme_enabled: Option<serde_json::Value>,
    #[serde(default)]
    pub ecc_enabled: Option<serde_json::Value>,
    #[serde(default)]
    pub rapl_disabled: Option<serde_json::Value>,
    #[serde(default)]
    pub ciphertext_hiding_dram: Option<serde_json::Value>,
    #[serde(default)]
    pub alias_check_complete: Option<serde_json::Value>,
    #[serde(default)]
    pub iommu_write_safe: Option<serde_json::Value>,
    #[serde(default)]
    pub tio_enabled: Option<serde_json::Value>,
}

/// Intel TDX appraisal rules.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TdxPolicy {
    pub qe_vendor_id: String,
    pub minimum_tee_tcb_svn: String,
    pub mr_seam: String,
    pub td_attributes: String,
    pub xfam: String,
    pub minimum_tcb_evaluation_data_number: Option<i64>,
    pub platform_measurements: Vec<String>,
}

impl TdxPolicy {
    /// Validate the policy's own fields.
    ///
    /// Each hex field has a fixed byte length fixed by the TDX quote format, so
    /// a wrong length means the policy and the evidence disagree about what is
    /// being compared.
    pub fn validate(&self) -> Result<()> {
        for (name, value, want) in [
            ("qe_vendor_id", &self.qe_vendor_id, 16usize),
            ("minimum_tee_tcb_svn", &self.minimum_tee_tcb_svn, 16),
            ("mr_seam", &self.mr_seam, 48),
            ("td_attributes", &self.td_attributes, 8),
            ("xfam", &self.xfam, 8),
        ] {
            decode_hex(name, value, want)?;
        }
        match self.minimum_tcb_evaluation_data_number {
            Some(n) if n >= 0 => {}
            Some(n) => {
                return Err(Error::SigstoreVerification(format!(
                    "minimum_tcb_evaluation_data_number must be non-negative, got {n}"
                )));
            }
            None => {
                return Err(Error::SigstoreVerification(
                    "minimum_tcb_evaluation_data_number is required".into(),
                ));
            }
        }
        if self.platform_measurements.is_empty() {
            return Err(Error::SigstoreVerification(
                "platform_measurements must not be empty".into(),
            ));
        }
        Ok(())
    }

    /// Appraise an authenticated quote's TDX module identity against this
    /// policy.
    ///
    /// [`Artifact::resolve_platform_measurement`] answers a different question:
    /// it matches `MRTD`/`RTMR0`, which describe the *guest firmware image* for
    /// a VM shape. It says nothing about the TDX module underneath, the TD's
    /// attributes, or how current Intel's TCB data is. Without this appraisal
    /// an endorsed host could run the endorsed firmware under a different
    /// (older, or debug-enabling) TDX module and still resolve a measurement.
    ///
    /// Every field is compared against values taken from the *authenticated*
    /// report body, never from anything the TD asserts alongside it.
    pub fn appraise(&self, evidence: &TdxEvidence<'_>) -> Result<()> {
        // Exact-match identity fields. `td_attributes` is the load-bearing one
        // for confidentiality: its DEBUG bit decides whether the TD's memory is
        // inspectable from the host, so a policy pinning it is what rules out a
        // debuggable TD wearing endorsed firmware measurements.
        for (name, expected, actual, want) in [
            (
                "qe_vendor_id",
                &self.qe_vendor_id,
                evidence.qe_vendor_id,
                16usize,
            ),
            ("mr_seam", &self.mr_seam, evidence.mr_seam, 48),
            (
                "td_attributes",
                &self.td_attributes,
                evidence.td_attributes,
                8,
            ),
            ("xfam", &self.xfam, evidence.xfam, 8),
        ] {
            // Decode the attested value too: a malformed one must be an error,
            // never something that merely fails to equal the policy.
            decode_hex(&format!("attested {name}"), actual, want)?;
            if expected.as_str() != actual {
                return Err(Error::SigstoreVerification(format!(
                    "{name} mismatch: policy requires {expected}, quote attests {actual}"
                )));
            }
        }

        // TEE_TCB_SVN is a floor, not an equality: it is 16 independent SVN
        // components, and Intel raises them as the TDX module is patched. Every
        // component must be at least what the policy requires, so a module
        // downgraded in any single dimension is refused.
        let minimum = decode_hex("minimum_tee_tcb_svn", &self.minimum_tee_tcb_svn, 16)?;
        let actual = decode_hex("attested tee_tcb_svn", evidence.tee_tcb_svn, 16)?;
        for (index, (want, got)) in minimum.iter().zip(actual.iter()).enumerate() {
            if got < want {
                return Err(Error::SigstoreVerification(format!(
                    "tee_tcb_svn component {index} is {got}, below the required minimum {want} \
                     (policy {}, quote attests {})",
                    self.minimum_tee_tcb_svn, evidence.tee_tcb_svn
                )));
            }
        }

        // The TCB evaluation data number is likewise a floor: it increases when
        // Intel publishes new TCB recovery data, so requiring a minimum stops a
        // verdict drawn from stale collateral that predates a known advisory.
        //
        // `validate` already rejected a missing or negative value, so `None`
        // here is unreachable for a parsed artifact; treat it as unmet rather
        // than silently skipping the check.
        let required = self.minimum_tcb_evaluation_data_number.ok_or_else(|| {
            Error::SigstoreVerification("minimum_tcb_evaluation_data_number is required".into())
        })?;
        if evidence.tcb_evaluation_data_number < required {
            return Err(Error::SigstoreVerification(format!(
                "tcb_evaluation_data_number is {}, below the required minimum {required}",
                evidence.tcb_evaluation_data_number
            )));
        }
        Ok(())
    }
}

/// The authenticated TDX quote values a [`TdxPolicy`] appraises.
///
/// Borrowed lowercase-hex strings, mirroring how the fields appear in a policy.
/// Kept free of any quote-library type so this crate stays independent of the
/// verifier that produced them; the caller is responsible for populating it
/// from an *already authenticated* report body.
#[derive(Debug, Clone, Copy)]
pub struct TdxEvidence<'a> {
    /// Quoting Enclave vendor ID from the quote header (16 bytes).
    pub qe_vendor_id: &'a str,
    /// TDX module SVN components, `TEE_TCB_SVN` (16 bytes).
    pub tee_tcb_svn: &'a str,
    /// TDX module measurement, `MRSEAM` (48 bytes).
    pub mr_seam: &'a str,
    /// TD attributes (8 bytes).
    pub td_attributes: &'a str,
    /// Extended features allowed mask, `XFAM` (8 bytes).
    pub xfam: &'a str,
    /// Intel's `tcbEvaluationDataNumber` behind the TCB verdict.
    pub tcb_evaluation_data_number: i64,
}

/// The parsed policy document: named platform measurements, named policies, and
/// the machines map keying both by hardware identity.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub format: String,
    pub measurements: BTreeMap<String, PlatformMeasurement>,
    /// Maps a machine's hardware identifier to its policy name.
    ///
    /// SEV-SNP machines are keyed by the 64-byte `CHIP_ID` (128 lowercase hex
    /// chars), TDX machines by the 16-byte `PPID` (32 lowercase hex chars).
    pub machines: BTreeMap<String, String>,
    pub policies: BTreeMap<String, Policy>,
}

impl Artifact {
    /// Strictly decode and validate a policy artifact.
    ///
    /// Unknown members are rejected case-sensitively, as are duplicate member
    /// names — `serde_json` would otherwise silently keep the last of a
    /// repeated key, letting one document present two different values for the
    /// same field.
    pub fn parse(data: &[u8]) -> Result<Self> {
        reject_duplicate_members(data)?;
        let artifact: Self = serde_json::from_slice(data)
            .map_err(|e| Error::SigstoreVerification(format!("parsing policy artifact: {e}")))?;
        if artifact.format != ARTIFACT_FORMAT {
            return Err(Error::SigstoreVerification(format!(
                "unsupported artifact format {:?}",
                artifact.format
            )));
        }
        artifact.validate()?;
        Ok(artifact)
    }

    fn validate(&self) -> Result<()> {
        for (name, policy) in &self.policies {
            match policy.platform.as_str() {
                PLATFORM_SEV_SNP => {
                    if policy.sev_snp.is_none() || policy.tdx.is_some() {
                        return Err(Error::SigstoreVerification(format!(
                            "policy {name:?}: platform sev-snp requires exactly the sev_snp block"
                        )));
                    }
                }
                PLATFORM_TDX => {
                    let Some(tdx) = &policy.tdx else {
                        return Err(Error::SigstoreVerification(format!(
                            "policy {name:?}: platform tdx requires exactly the tdx block"
                        )));
                    };
                    if policy.sev_snp.is_some() {
                        return Err(Error::SigstoreVerification(format!(
                            "policy {name:?}: platform tdx requires exactly the tdx block"
                        )));
                    }
                    // Unwrap the inner message rather than nesting a full
                    // `Display`, which would repeat the "sigstore verification
                    // failed" prefix inside itself.
                    tdx.validate().map_err(|e| {
                        let Error::SigstoreVerification(detail) = e;
                        Error::SigstoreVerification(format!("policy {name:?}: {detail}"))
                    })?;
                    for reference in &tdx.platform_measurements {
                        if !self.measurements.contains_key(reference) {
                            return Err(Error::SigstoreVerification(format!(
                                "policy {name:?}: platform_measurements ref {reference:?} not in \
                                 measurements"
                            )));
                        }
                    }
                }
                other => {
                    return Err(Error::SigstoreVerification(format!(
                        "policy {name:?}: unsupported platform {other:?}"
                    )));
                }
            }
        }

        for (name, measurement) in &self.measurements {
            if measurement.shape.is_none() {
                return Err(Error::SigstoreVerification(format!(
                    "measurement {name:?}: shape is required"
                )));
            }
        }

        for (identifier, policy_name) in &self.machines {
            let Some(policy) = self.policies.get(policy_name) else {
                return Err(Error::SigstoreVerification(format!(
                    "machine {}...: unknown policy {policy_name:?}",
                    trunc_id(identifier)
                )));
            };
            if identifier.is_empty() || !is_lower_hex(identifier) {
                return Err(Error::SigstoreVerification(format!(
                    "machine {}...: identifier is not lowercase hex",
                    trunc_id(identifier)
                )));
            }
            let want = match policy.platform.as_str() {
                PLATFORM_SEV_SNP => Some(SEV_IDENTIFIER_HEX_LEN),
                PLATFORM_TDX => Some(TDX_IDENTIFIER_HEX_LEN),
                _ => None,
            };
            if let Some(want) = want {
                if identifier.len() != want {
                    return Err(Error::SigstoreVerification(format!(
                        "machine {}...: {} identifier must be {want} hex chars, got {}",
                        trunc_id(identifier),
                        policy.platform,
                        identifier.len()
                    )));
                }
            }
        }
        Ok(())
    }

    /// Look up the appraisal policy for an authenticated platform identifier.
    ///
    /// `identifier_hex` must come from *verified* evidence — for TDX, the PPID
    /// recovered from the quote's PCK certificate.
    ///
    /// An identifier absent from the machines map is an error, not a default:
    /// the machine is simply not endorsed. This is the check that stops a
    /// genuine TDX quote from an unendorsed host being accepted.
    pub fn policy_for(&self, identifier_hex: &str, platform: &str) -> Result<(&str, &Policy)> {
        let Some((name, policy)) = self
            .machines
            .get(identifier_hex)
            .and_then(|name| self.policies.get_key_value(name))
        else {
            return Err(Error::SigstoreVerification(format!(
                "platform identifier {}... is not endorsed",
                trunc_id(identifier_hex)
            )));
        };
        if policy.platform != platform {
            return Err(Error::SigstoreVerification(format!(
                "policy {name:?} is for platform {:?}, evidence is {platform:?}",
                policy.platform
            )));
        }
        Ok((name.as_str(), policy))
    }

    /// Select the endorsed measurement entry whose `MRTD`/`RTMR0` equal the
    /// quote's authenticated values.
    ///
    /// Only entries measured for the required VM shape are candidates, so a
    /// quote from a machine-endorsed but wrong-shaped VM resolves nothing.
    ///
    /// The two failure modes are reported distinctly on purpose: "no entry has
    /// your shape" is an endorsement-coverage problem, while "your shape is
    /// covered but the registers differ" means the firmware is not what the
    /// endorsement expects.
    ///
    /// Upstream takes the **first** shape-matching entry whose registers agree
    /// rather than proving uniqueness; this mirrors that. Two entries with the
    /// same shape and registers would be redundant rather than ambiguous, since
    /// resolution only reports which entry matched.
    pub fn resolve_platform_measurement(
        &self,
        policy: &TdxPolicy,
        required: &Shape,
        mrtd_hex: &str,
        rtmr0_hex: &str,
    ) -> Result<(&str, &PlatformMeasurement)> {
        let mut any_shape_match = false;
        for reference in &policy.platform_measurements {
            let Some((name, measurement)) = self.measurements.get_key_value(reference) else {
                continue;
            };
            let Some(shape) = &measurement.shape else {
                continue;
            };
            if !shape.satisfies(required) {
                continue;
            }
            any_shape_match = true;
            if measurement.mrtd == mrtd_hex && measurement.rtmr0 == rtmr0_hex {
                return Ok((name.as_str(), measurement));
            }
        }
        if !any_shape_match {
            return Err(Error::SigstoreVerification(format!(
                "no endorsed platform measurement matches the required VM shape {required:?}"
            )));
        }
        Err(Error::SigstoreVerification(format!(
            "platform measurements (mrtd {}...) do not match any allowed configuration",
            trunc_id(mrtd_hex)
        )))
    }
}

/// Decode a required hex policy value of an exact byte length.
pub fn decode_hex(name: &str, value: &str, want_len: usize) -> Result<Vec<u8>> {
    if !is_lower_hex(value) {
        return Err(Error::SigstoreVerification(format!(
            "{name} is not lowercase hex"
        )));
    }
    let bytes = hex::decode(value)
        .map_err(|e| Error::SigstoreVerification(format!("{name} is not hex: {e}")))?;
    if bytes.len() != want_len {
        return Err(Error::SigstoreVerification(format!(
            "{name} must be {want_len} bytes, got {}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn is_lower_hex(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn trunc_id(id: &str) -> &str {
    if id.len() > 16 { &id[..16] } else { id }
}

/// Reject duplicate object member names anywhere in a JSON document.
fn reject_duplicate_members(bytes: &[u8]) -> Result<()> {
    struct DupCheck;

    impl<'de> serde::de::Deserialize<'de> for DupCheck {
        fn deserialize<D: serde::Deserializer<'de>>(
            deserializer: D,
        ) -> std::result::Result<Self, D::Error> {
            struct V;

            impl<'de> serde::de::Visitor<'de> for V {
                type Value = DupCheck;

                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("any JSON value")
                }

                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> std::result::Result<DupCheck, A::Error> {
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
                ) -> std::result::Result<DupCheck, A::Error> {
                    while seq.next_element::<DupCheck>()?.is_some() {}
                    Ok(DupCheck)
                }

                fn visit_unit<E>(self) -> std::result::Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_none<E>(self) -> std::result::Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_bool<E>(self, _: bool) -> std::result::Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_i64<E>(self, _: i64) -> std::result::Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_u64<E>(self, _: u64) -> std::result::Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_f64<E>(self, _: f64) -> std::result::Result<DupCheck, E> {
                    Ok(DupCheck)
                }
                fn visit_str<E>(self, _: &str) -> std::result::Result<DupCheck, E> {
                    Ok(DupCheck)
                }
            }

            deserializer.deserialize_any(V)
        }
    }

    serde_json::from_slice::<DupCheck>(bytes)
        .map(|_| ())
        .map_err(|e| Error::SigstoreVerification(format!("parsing policy artifact: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mrtd(byte: &str) -> String {
        byte.repeat(48)
    }
    fn ppid() -> String {
        "ab".repeat(16)
    }

    /// A minimal but complete TDX artifact.
    fn artifact_json() -> serde_json::Value {
        serde_json::json!({
            "format": ARTIFACT_FORMAT,
            "measurements": {
                "slug-a": {
                    "mrtd": mrtd("aa"),
                    "rtmr0": mrtd("bb"),
                    "shape": { "cpus": 8, "memory_mb": 16384, "gpus": 0, "disks": 2 }
                }
            },
            "machines": { ppid(): "fleet" },
            "policies": {
                "fleet": {
                    "platform": "tdx",
                    "tdx": {
                        "qe_vendor_id": "cd".repeat(16),
                        "minimum_tee_tcb_svn": "00".repeat(16),
                        "mr_seam": mrtd("cc"),
                        "td_attributes": "00".repeat(8),
                        "xfam": "e7".repeat(8),
                        "minimum_tcb_evaluation_data_number": 17,
                        "platform_measurements": ["slug-a"]
                    }
                }
            }
        })
    }

    fn parse(value: &serde_json::Value) -> Result<Artifact> {
        Artifact::parse(&serde_json::to_vec(value).unwrap())
    }

    #[test]
    fn a_well_formed_artifact_parses() {
        let artifact = parse(&artifact_json()).expect("must parse");
        assert_eq!(artifact.machines.len(), 1);
        let (name, policy) = artifact
            .policy_for(&ppid(), PLATFORM_TDX)
            .expect("endorsed");
        assert_eq!(name, "fleet");
        assert!(policy.tdx.is_some());
    }

    /// The check that stops a genuine quote from an unendorsed host.
    #[test]
    fn an_unendorsed_machine_is_refused() {
        let artifact = parse(&artifact_json()).unwrap();
        let err = artifact
            .policy_for(&"cd".repeat(16), PLATFORM_TDX)
            .unwrap_err();
        assert!(err.to_string().contains("not endorsed"), "{err}");
    }

    /// A SEV policy must never appraise TDX evidence, and vice versa.
    #[test]
    fn a_policy_for_another_platform_is_refused() {
        let artifact = parse(&artifact_json()).unwrap();
        let err = artifact.policy_for(&ppid(), PLATFORM_SEV_SNP).unwrap_err();
        assert!(err.to_string().contains("is for platform"), "{err}");
    }

    #[test]
    fn an_unsupported_format_is_refused() {
        let mut value = artifact_json();
        value["format"] = "https://example.invalid/v1".into();
        assert!(parse(&value).is_err());
    }

    #[test]
    fn unknown_and_duplicate_members_are_refused() {
        let mut value = artifact_json();
        value["surprise"] = 1.into();
        assert!(parse(&value).is_err(), "unknown member must be refused");

        let raw = br#"{"format":"x","format":"y"}"#;
        assert!(
            Artifact::parse(raw).is_err(),
            "duplicate member must be refused"
        );
    }

    /// A SEV-SNP machine identifier: the 64-byte chip ID, not the 16-byte PPID
    /// a TDX machine is keyed by.
    fn chip_id() -> String {
        "cd".repeat(64)
    }

    /// A complete SEV-SNP block, as `verifier/policy/sev.go` defines it.
    fn sev_artifact_json() -> serde_json::Value {
        serde_json::json!({
            "format": ARTIFACT_FORMAT,
            "measurements": {},
            "machines": { chip_id(): "fleet" },
            "policies": {
                "fleet": {
                    "platform": "sev-snp",
                    "sev_snp": {
                        "minimum_build": 21,
                        "minimum_api_version": "1.55",
                        "minimum_abi_version": "1.51",
                        "minimum_guest_svn": 0,
                        "minimum_tcb": {
                            "fmc_spl": 0, "bl_spl": 4, "tee_spl": 0,
                            "snp_spl": 22, "ucode_spl": 213
                        },
                        "minimum_launch_tcb": {
                            "bl_spl": 4, "tee_spl": 0, "snp_spl": 22, "ucode_spl": 213
                        },
                        "guest_policy": {
                            "debug": false, "smt": true, "migrate_ma": false,
                            "single_socket": false
                        },
                        "platform_info": {
                            "smt_enabled": true, "tsme_enabled": false,
                            "ecc_enabled": true, "rapl_disabled": false,
                            "ciphertext_hiding_dram": false
                        },
                        "permit_provisional_firmware": false,
                        "vmpl": 0,
                        "host_data": "00".repeat(32),
                        "image_id": "00".repeat(16),
                        "family_id": "00".repeat(16),
                        "minimum_launch_mitigation_vector": 0,
                        "minimum_current_mitigation_vector": 0
                    }
                }
            }
        })
    }

    /// A genuine endorsements artifact endorses SEV *and* TDX fleets, so
    /// modelling the SEV block must not make real documents unparseable.
    #[test]
    fn a_complete_sev_policy_block_parses() {
        let artifact = parse(&sev_artifact_json()).expect("a real SEV block must parse");
        let (_, policy) = artifact
            .policy_for(&chip_id(), PLATFORM_SEV_SNP)
            .expect("endorsed for sev-snp");
        assert!(policy.sev_snp.is_some());
        // Optional members may be absent without the block being rejected.
        assert!(
            policy
                .sev_snp
                .as_ref()
                .and_then(|s| s.minimum_launch_tcb.as_ref())
                .is_some_and(|tcb| tcb.fmc_spl.is_none()),
            "an omitted optional member must parse as absent"
        );
    }

    /// The point of modelling the block: an unrecognized member inside it is
    /// refused, exactly as it would be anywhere else in the artifact.
    ///
    /// Before this, the block was an opaque `Value` and anything at all was
    /// accepted inside it.
    #[test]
    fn an_unknown_member_inside_the_sev_block_is_refused() {
        for pointer in [
            "/policies/fleet/sev_snp",
            "/policies/fleet/sev_snp/minimum_tcb",
            "/policies/fleet/sev_snp/guest_policy",
            "/policies/fleet/sev_snp/platform_info",
        ] {
            let mut value = sev_artifact_json();
            value
                .pointer_mut(pointer)
                .expect("pointer resolves")
                .as_object_mut()
                .expect("object")
                .insert("totally_made_up".into(), 1.into());

            let err = parse(&value).expect_err("an unknown member must be refused");
            assert!(
                err.to_string().contains("totally_made_up"),
                "the error must name the unknown member, got {err}"
            );
        }
    }

    /// Duplicate members inside the SEV block were already refused by the
    /// recursive duplicate check, which runs over the raw bytes before serde
    /// sees them. Pinned so that check is not lost.
    #[test]
    fn a_duplicate_member_inside_the_sev_block_is_refused() {
        let raw = br#"{
            "format":"https://tinfoil.sh/predicate/platform-endorsements/v1",
            "measurements":{},
            "machines":{},
            "policies":{"f":{"platform":"sev-snp","sev_snp":{"vmpl":0,"vmpl":1}}}
        }"#;
        let err = Artifact::parse(raw).expect_err("a duplicate member must be refused");
        assert!(err.to_string().contains("duplicate member"), "{err}");
    }

    #[test]
    fn a_dangling_measurement_reference_is_refused() {
        let mut value = artifact_json();
        value["policies"]["fleet"]["tdx"]["platform_measurements"] = serde_json::json!(["missing"]);
        let err = parse(&value).unwrap_err();
        assert!(err.to_string().contains("not in measurements"), "{err}");
    }

    #[test]
    fn a_machine_with_the_wrong_identifier_length_is_refused() {
        let mut value = artifact_json();
        value["machines"] = serde_json::json!({ "abcd": "fleet" });
        let err = parse(&value).unwrap_err();
        assert!(err.to_string().contains("must be 32 hex chars"), "{err}");
    }

    #[test]
    fn a_machine_naming_an_unknown_policy_is_refused() {
        let mut value = artifact_json();
        value["machines"] = serde_json::json!({ ppid(): "nope" });
        let err = parse(&value).unwrap_err();
        assert!(err.to_string().contains("unknown policy"), "{err}");
    }

    #[test]
    fn a_malformed_tdx_policy_is_refused() {
        for (field, bad) in [
            ("mr_seam", "aa"),
            ("qe_vendor_id", "zz".repeat(16).as_str()),
            ("xfam", ""),
        ] {
            let mut value = artifact_json();
            value["policies"]["fleet"]["tdx"][field] = bad.into();
            assert!(parse(&value).is_err(), "{field}={bad:?} must be refused");
        }

        // The TCB evaluation floor is required, and must be non-negative.
        let mut value = artifact_json();
        value["policies"]["fleet"]["tdx"]["minimum_tcb_evaluation_data_number"] =
            serde_json::Value::Null;
        assert!(parse(&value).is_err());

        let mut value = artifact_json();
        value["policies"]["fleet"]["tdx"]["minimum_tcb_evaluation_data_number"] = (-1).into();
        assert!(parse(&value).is_err());
    }

    #[test]
    fn a_measurement_without_a_shape_is_refused() {
        let mut value = artifact_json();
        value["measurements"]["slug-a"]["shape"] = serde_json::Value::Null;
        let err = parse(&value).unwrap_err();
        assert!(err.to_string().contains("shape is required"), "{err}");
    }

    // ---- Shape matching ----

    fn shape(cpus: u32, memory_mb: u64, gpus: Option<u32>, disks: u32) -> Shape {
        Shape {
            cpus,
            memory_mb,
            gpus,
            disks,
        }
    }

    #[test]
    fn shape_compares_cpus_memory_and_disks_exactly() {
        let measured = shape(8, 16384, Some(0), 2);
        assert!(measured.satisfies(&shape(8, 16384, Some(0), 2)));
        assert!(!measured.satisfies(&shape(4, 16384, Some(0), 2)));
        assert!(!measured.satisfies(&shape(8, 8192, Some(0), 2)));
        assert!(!measured.satisfies(&shape(8, 16384, Some(0), 3)));
    }

    /// `gpus` is compared only when both sides declare it — a slug that does
    /// not know its GPU count must not be excluded on that basis.
    #[test]
    fn shape_compares_gpus_only_when_both_declare_it() {
        assert!(shape(8, 16384, Some(1), 2).satisfies(&shape(8, 16384, Some(1), 2)));
        assert!(!shape(8, 16384, Some(1), 2).satisfies(&shape(8, 16384, Some(0), 2)));
        // Either side unknown: not a reason to reject.
        assert!(shape(8, 16384, None, 2).satisfies(&shape(8, 16384, Some(1), 2)));
        assert!(shape(8, 16384, Some(1), 2).satisfies(&shape(8, 16384, None, 2)));
    }

    // ---- Measurement resolution ----

    #[test]
    fn resolution_returns_the_matching_entry() {
        let artifact = parse(&artifact_json()).unwrap();
        let policy = artifact.policies["fleet"].tdx.clone().unwrap();
        let required = shape(8, 16384, Some(0), 2);

        let (name, measurement) = artifact
            .resolve_platform_measurement(&policy, &required, &mrtd("aa"), &mrtd("bb"))
            .expect("must resolve");
        assert_eq!(name, "slug-a");
        assert_eq!(measurement.mrtd, mrtd("aa"));
    }

    /// A machine-endorsed but wrong-shaped VM resolves nothing, and says so
    /// distinctly from a register mismatch.
    #[test]
    fn resolution_distinguishes_shape_and_register_failures() {
        let artifact = parse(&artifact_json()).unwrap();
        let policy = artifact.policies["fleet"].tdx.clone().unwrap();

        let wrong_shape = shape(16, 16384, Some(0), 2);
        let err = artifact
            .resolve_platform_measurement(&policy, &wrong_shape, &mrtd("aa"), &mrtd("bb"))
            .unwrap_err();
        assert!(err.to_string().contains("required VM shape"), "{err}");

        let required = shape(8, 16384, Some(0), 2);
        let err = artifact
            .resolve_platform_measurement(&policy, &required, &mrtd("ff"), &mrtd("bb"))
            .unwrap_err();
        assert!(
            err.to_string().contains("do not match any allowed"),
            "{err}"
        );

        // RTMR0 must match too, not just MRTD.
        let err = artifact
            .resolve_platform_measurement(&policy, &required, &mrtd("aa"), &mrtd("ff"))
            .unwrap_err();
        assert!(
            err.to_string().contains("do not match any allowed"),
            "{err}"
        );
    }

    // ---- TDX policy appraisal ----

    /// Evidence that satisfies the fixture policy exactly.
    fn conforming_evidence() -> TdxEvidence<'static> {
        TdxEvidence {
            qe_vendor_id: "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
            tee_tcb_svn: "00000000000000000000000000000000",
            mr_seam: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\
                      cccccccccccccccccccccccccccccccc",
            td_attributes: "0000000000000000",
            xfam: "e7e7e7e7e7e7e7e7",
            tcb_evaluation_data_number: 17,
        }
    }

    fn fleet_policy() -> TdxPolicy {
        parse(&artifact_json()).unwrap().policies["fleet"]
            .tdx
            .clone()
            .unwrap()
    }

    #[test]
    fn conforming_evidence_is_appraised_successfully() {
        fleet_policy()
            .appraise(&conforming_evidence())
            .expect("evidence matching the policy must be accepted");
    }

    /// The regression this appraisal exists for: each identity field must be
    /// compared, so a host running a *different* TDX module, presenting a
    /// different QE, or launching the TD with different attributes is refused
    /// even though its MRTD/RTMR0 would still resolve.
    #[test]
    fn a_mismatch_in_any_identity_field_is_refused() {
        let policy = fleet_policy();
        for (field, mutate) in [
            (
                "qe_vendor_id",
                (|e: &mut TdxEvidence<'static>| e.qe_vendor_id = "abababababababababababababababab")
                    as fn(&mut TdxEvidence<'static>),
            ),
            ("mr_seam", |e| {
                e.mr_seam = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd\
                             dddddddddddddddddddddddddddddddd"
            }),
            // The DEBUG bit lives here: a debuggable TD must not pass.
            ("td_attributes", |e| e.td_attributes = "0000000000000001"),
            ("xfam", |e| e.xfam = "0000000000000003"),
        ] {
            let mut evidence = conforming_evidence();
            mutate(&mut evidence);
            let err = policy.appraise(&evidence).unwrap_err();
            assert!(
                err.to_string().contains(field) && err.to_string().contains("mismatch"),
                "{field} must be appraised, got {err}"
            );
        }
    }

    /// `TEE_TCB_SVN` is a floor across 16 independent components, so a module
    /// downgraded in any single dimension is refused while an upgrade passes.
    #[test]
    fn tee_tcb_svn_is_enforced_componentwise_as_a_floor() {
        let mut value = artifact_json();
        value["policies"]["fleet"]["tdx"]["minimum_tee_tcb_svn"] = "05".repeat(16).into();
        let policy = parse(&value).unwrap().policies["fleet"]
            .tdx
            .clone()
            .unwrap();

        let mut evidence = conforming_evidence();
        evidence.tee_tcb_svn = "05050505050505050505050505050505";
        policy.appraise(&evidence).expect("equal SVNs must pass");

        evidence.tee_tcb_svn = "06060606060606060606060606060606";
        policy.appraise(&evidence).expect("higher SVNs must pass");

        // A single component below the floor is enough to refuse, even when
        // every other component is above it.
        evidence.tee_tcb_svn = "05050505050505050505050505050504";
        let err = policy.appraise(&evidence).unwrap_err();
        assert!(
            err.to_string().contains("tee_tcb_svn component 15"),
            "{err}"
        );

        evidence.tee_tcb_svn = "04060606060606060606060606060606";
        let err = policy.appraise(&evidence).unwrap_err();
        assert!(err.to_string().contains("tee_tcb_svn component 0"), "{err}");
    }

    /// The TCB floor stops a verdict drawn from collateral that predates a
    /// known advisory.
    #[test]
    fn a_stale_tcb_evaluation_data_number_is_refused() {
        let policy = fleet_policy();
        let mut evidence = conforming_evidence();

        evidence.tcb_evaluation_data_number = 18;
        policy.appraise(&evidence).expect("a newer TCB must pass");

        evidence.tcb_evaluation_data_number = 16;
        let err = policy.appraise(&evidence).unwrap_err();
        assert!(
            err.to_string().contains("tcb_evaluation_data_number is 16"),
            "{err}"
        );
    }

    /// A malformed attested value must be a distinct error rather than
    /// something that merely fails to equal the policy.
    #[test]
    fn malformed_attested_values_are_refused_as_malformed() {
        let policy = fleet_policy();
        for mutate in [
            (|e: &mut TdxEvidence<'static>| e.qe_vendor_id = "zz") as fn(&mut TdxEvidence<'static>),
            |e: &mut TdxEvidence<'static>| e.td_attributes = "",
            |e: &mut TdxEvidence<'static>| e.xfam = "E7E7E7E7E7E7E7E7",
            |e: &mut TdxEvidence<'static>| e.tee_tcb_svn = "00",
        ] {
            let mut evidence = conforming_evidence();
            mutate(&mut evidence);
            let err = policy.appraise(&evidence).unwrap_err();
            assert!(
                err.to_string().contains("attested"),
                "malformed evidence must be refused as malformed, got {err}"
            );
        }
    }
}
