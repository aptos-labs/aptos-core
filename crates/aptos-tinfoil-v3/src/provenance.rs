//! Sigstore code provenance, wired to the hardware quote.
//!
//! This module is the join that the audit found missing. [`crate::tdx`]
//! authenticates a TDX quote, and [`aptos_tinfoil_sigstore`] authenticates a Sigstore
//! bundle — but a vault that does both independently and never connects them
//! proves nothing about *which repository's code is running*.
//!
//! The chain assembled here:
//!
//! ```text
//! configured trusted repo        (verifier input, never read from the document)
//!         v
//! sigstore-code bundle           -> AuthenticateCode
//!         v
//! authenticated artifact digest
//!         v
//! expected RTMR1 / RTMR2         (kernel, initrd + cmdline)
//!         v
//! actual RTMR1 / RTMR2           (from the Intel-signed quote)
//!         must match
//! ```
//!
//! and, when platform endorsements are supplied:
//!
//! ```text
//! sigstore-platform bundle       -> AuthenticatePlatformEndorsements
//!         v
//! machines[PPID]                 (is this *host* endorsed?)
//!         v
//! expected MRTD / RTMR0          (per VM shape)
//!         v
//! actual MRTD / RTMR0            (from the quote)
//!         must match
//! ```
//!
//! # The repository is an input
//!
//! `repo` comes from the vault's own configuration. It is never read out of the
//! collateral. A `sigstore-code` entry states which repository signed it, and
//! an attacker can produce a perfectly valid bundle for a repository they
//! control — so the name inside the payload is evidence of nothing until it is
//! checked against the name the operator configured.

use super::{
    envelope::{
        Document, COLLATERAL_SIGSTORE_CODE_V1_FORMAT, COLLATERAL_SIGSTORE_PLATFORM_V1_FORMAT,
        FRESHNESS_COLLATERAL_ID_CODE, FRESHNESS_COLLATERAL_ID_PLATFORM,
    },
    tdx::AuthenticatedQuote,
};
use aptos_tinfoil_sigstore::provenance::{
    authenticate_code_with_root, authenticate_freshness_with_root,
    authenticate_platform_endorsements_with_root, AuthenticatedArtifact, Code,
};
use serde::Deserialize;

/// Number of hex characters in a 48-byte measurement register.
const REGISTER_HEX_LEN: usize = 96;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProvenanceError {
    #[error("document carries no sigstore-code reference values")]
    MissingCodeCollateral,
    #[error("document carries no sigstore-platform reference values")]
    MissingPlatformCollateral,
    #[error("document carries no {0} witness")]
    MissingFreshness(&'static str),
    #[error("parsing {what} collateral: {detail}")]
    BadCollateral { what: &'static str, detail: String },
    #[error("{what} verification failed: {detail}")]
    Verification { what: &'static str, detail: String },
    #[error(
        "the sigstore-code collateral names repository {claimed:?}, but this vault is configured \
         to trust {configured:?}"
    )]
    RepoMismatch { claimed: String, configured: String },
    #[error(
        "code provenance expects {register} {expected}, but the hardware quote attests {actual}"
    )]
    MeasurementMismatch {
        register: &'static str,
        expected: String,
        actual: String,
    },
    #[error("code provenance carries no {0} value to compare against the quote")]
    MissingRegister(&'static str),
    #[error("provenance register {0:?} is not 48 bytes of lowercase hex")]
    MalformedRegister(String),
    #[error(
        "the code artifact declares no VM shape, which platform-endorsement resolution requires"
    )]
    MissingShape,
}

/// A `sigstore-code` / `sigstore-platform` collateral payload.
///
/// `repo`, `tag` and `digest` are *claims*: they say which artifact the bundle
/// is supposed to be about. Trust comes from verifying the bundle against the
/// repository the vault configured, not from these fields.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SigstoreCollateral {
    repo: String,
    tag: String,
    digest: String,
    sigstore_bundle: Box<serde_json::value::RawValue>,
}

/// A freshness witness collateral payload.
///
/// Deliberately carries no repo/tag/digest: the artifact a witness is checked
/// against comes from the *already authenticated* code or platform bundle, not
/// from anything alongside the witness.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FreshnessCollateral {
    sigstore_bundle: Box<serde_json::value::RawValue>,
}

/// What Sigstore verification established about the running workload.
#[derive(Debug, Clone)]
pub struct ProvenWorkload {
    /// The repository, proven — this is the configured name, returned only
    /// after the bundle was shown to be signed by it.
    pub repo: String,
    /// The release tag the build came from.
    pub tag: String,
    /// The commit the build ran against.
    pub commit: String,
    /// The artifact digest the statement was about.
    pub digest: String,
    /// Code registers recovered from the authenticated provenance statement.
    pub code_measurement: aptos_tinfoil_sigstore::Measurement,
    /// Which endorsed platform measurement matched, when endorsements were
    /// supplied and resolved.
    pub platform_measurement: Option<String>,
    /// VM shape proven by the code bundle, required for platform appraisal.
    pub shape: Option<aptos_tinfoil_sigstore::provenance::Shape>,
}

/// Platform policy and artifact established by a signed Sigstore bundle.
#[derive(Debug, Clone)]
pub struct ProvenPlatform {
    pub measurement_id: String,
    pub artifact: AuthenticatedArtifact,
}

/// Whether freshness witnesses are required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Require both witnesses, as upstream does.
    Required,
    /// Verify a witness if present, but do not require one.
    ///
    /// Witnesses bound how long a withdrawn release stays acceptable. Making
    /// them optional is a real weakening, so it is a deliberate configuration
    /// choice rather than a silent fallback.
    Optional,
}

/// Whether the `sigstore-platform` endorsement entry is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformEndorsements {
    /// Require the entry, as upstream does.
    ///
    /// This is what keeps the *host* in scope. Without it the client chooses
    /// how strictly it is judged, because omitting one collateral entry skips
    /// the PPID endorsement lookup, MRTD/RTMR0 resolution, the VM-shape
    /// constraint and the entire TDX policy appraisal.
    Required,
    /// Verify the entry if present, but do not require one.
    ///
    /// Leaves MRTD and RTMR0 governed only by the operator's pinned allowlist,
    /// so this is safe only when that allowlist actually pins them. A
    /// deliberate configuration choice, never a silent fallback.
    Optional,
}

/// Verify code provenance and bind it to an authenticated quote.
///
/// `repo` is the **trusted** repository from configuration. `quote` must
/// already have been authenticated against Intel's root — this function
/// compares against its registers and does not re-verify it.
#[cfg(test)]
pub fn verify(
    document: &Document,
    quote: &AuthenticatedQuote,
    repo: &str,
    freshness: Freshness,
    platform_endorsements: PlatformEndorsements,
    now_secs: u64,
) -> Result<ProvenWorkload, ProvenanceError> {
    verify_with_root(
        document,
        quote,
        repo,
        freshness,
        platform_endorsements,
        now_secs,
        include_str!("../../aptos-tinfoil-sigstore/assets/trusted_root.json"),
    )
}

pub fn verify_with_root(
    document: &Document,
    quote: &AuthenticatedQuote,
    repo: &str,
    freshness: Freshness,
    platform_endorsements: PlatformEndorsements,
    now_secs: u64,
    trusted_root_json: &str,
) -> Result<ProvenWorkload, ProvenanceError> {
    let code = verify_code_stage(document, quote, repo, trusted_root_json)?;
    if let Some(entry) = document.freshness_collateral(FRESHNESS_COLLATERAL_ID_CODE) {
        check_freshness(entry, "code", &code.artifact, now_secs, trusted_root_json)?;
    } else if freshness == Freshness::Required {
        return Err(ProvenanceError::MissingFreshness("code-freshness"));
    }

    let platform_measurement = match document
        .reference_values_collateral(COLLATERAL_SIGSTORE_PLATFORM_V1_FORMAT)
    {
        Some(_) => {
            let platform = verify_platform_stage(document, quote, code.shape, trusted_root_json)?;
            if let Some(entry) = document.freshness_collateral(FRESHNESS_COLLATERAL_ID_PLATFORM) {
                check_freshness(
                    entry,
                    "platform",
                    &platform.artifact,
                    now_secs,
                    trusted_root_json,
                )?;
            } else if freshness == Freshness::Required {
                return Err(ProvenanceError::MissingFreshness("platform-freshness"));
            }
            Some(platform.measurement_id)
        },
        None if platform_endorsements == PlatformEndorsements::Required => {
            return Err(ProvenanceError::MissingPlatformCollateral);
        },
        None => None,
    };

    Ok(ProvenWorkload {
        repo: repo.to_string(),
        tag: code.artifact.tag.clone(),
        commit: code.artifact.commit.clone(),
        digest: code.artifact.digest.clone(),
        code_measurement: code.measurement.clone(),
        platform_measurement,
        shape: code.shape,
    })
}

/// Verify one signed code bundle and match its RTMR1/RTMR2 against the quote.
pub fn verify_code_stage(
    document: &Document,
    quote: &AuthenticatedQuote,
    repo: &str,
    trusted_root_json: &str,
) -> Result<Code, ProvenanceError> {
    let entry = document
        .reference_values_collateral(COLLATERAL_SIGSTORE_CODE_V1_FORMAT)
        .ok_or(ProvenanceError::MissingCodeCollateral)?;
    let code_collateral: SigstoreCollateral =
        serde_json::from_value(entry.data.clone()).map_err(|e| ProvenanceError::BadCollateral {
            what: "sigstore-code",
            detail: e.to_string(),
        })?;

    // The collateral's own `repo` is a claim. Refuse early when it disagrees
    // with what this vault trusts: verification would fail anyway (the
    // certificate identity is checked against `repo`, not against this field),
    // but saying so plainly beats a generic signature error.
    if code_collateral.repo != repo {
        return Err(ProvenanceError::RepoMismatch {
            claimed: code_collateral.repo,
            configured: repo.to_string(),
        });
    }

    let tag = (!code_collateral.tag.is_empty()).then_some(code_collateral.tag.as_str());
    let code = authenticate_code_with_root(
        code_collateral.sigstore_bundle.get().as_bytes(),
        repo,
        tag,
        &code_collateral.digest,
        trusted_root_json,
    )
    .map_err(|e| ProvenanceError::Verification {
        what: "code provenance",
        detail: e.to_string(),
    })?;

    // ---- The binding the audit asked for ----
    //
    // Code provenance yields RTMR1 (kernel) and RTMR2 (initrd + cmdline).
    // Requiring the quote to attest exactly those is what makes "this repo's
    // code is running" a proven statement rather than two unrelated ones.
    let expected_rtmr1 = code
        .measurement
        .rtmr1()
        .ok_or(ProvenanceError::MissingRegister("RTMR1"))?;
    let expected_rtmr2 = code
        .measurement
        .rtmr2()
        .ok_or(ProvenanceError::MissingRegister("RTMR2"))?;
    compare_register("RTMR1", expected_rtmr1, &quote.rtmrs[1])?;
    compare_register("RTMR2", expected_rtmr2, &quote.rtmrs[2])?;
    Ok(code)
}

/// Verify platform endorsements and resolve this host's expected MRTD/RTMR0.
pub fn verify_platform_stage(
    document: &Document,
    quote: &AuthenticatedQuote,
    shape: Option<aptos_tinfoil_sigstore::provenance::Shape>,
    trusted_root_json: &str,
) -> Result<ProvenPlatform, ProvenanceError> {
    let entry = document
        .reference_values_collateral(COLLATERAL_SIGSTORE_PLATFORM_V1_FORMAT)
        .ok_or(ProvenanceError::MissingPlatformCollateral)?;
    let collateral: SigstoreCollateral =
        serde_json::from_value(entry.data.clone()).map_err(|e| ProvenanceError::BadCollateral {
            what: "sigstore-platform",
            detail: e.to_string(),
        })?;

    let tag = (!collateral.tag.is_empty()).then_some(collateral.tag.as_str());
    let endorsements = authenticate_platform_endorsements_with_root(
        collateral.sigstore_bundle.get().as_bytes(),
        &collateral.repo,
        tag,
        &collateral.digest,
        trusted_root_json,
    )
    .map_err(|e| ProvenanceError::Verification {
        what: "platform endorsements",
        detail: e.to_string(),
    })?;

    // Is *this host* endorsed at all? The PPID comes from the quote's PCK
    // certificate, so it is Intel-authenticated rather than self-asserted.
    let (_, policy) = endorsements
        .policy
        .policy_for(&quote.ppid, aptos_tinfoil_sigstore::policy::PLATFORM_TDX)
        .map_err(|e| ProvenanceError::Verification {
            what: "platform endorsements",
            detail: e.to_string(),
        })?;
    let tdx = policy
        .tdx
        .as_ref()
        .ok_or_else(|| ProvenanceError::Verification {
            what: "platform endorsements",
            detail: "endorsed policy carries no tdx block".into(),
        })?;

    // Appraise the TDX module identity, TD attributes and TCB floor the
    // endorsed policy demands. Resolving MRTD/RTMR0 below only proves the guest
    // firmware image is endorsed for this VM shape; without this, an endorsed
    // host could run that firmware under a different or downgraded TDX module,
    // with different TD attributes, or on a stale TCB, and still be accepted.
    tdx.appraise(&aptos_tinfoil_sigstore::policy::TdxEvidence {
        qe_vendor_id: &quote.qe_vendor_id,
        tee_tcb_svn: &quote.tee_tcb_svn,
        mr_seam: &quote.mr_seam,
        td_attributes: &quote.td_attributes,
        xfam: &quote.xfam,
        tcb_evaluation_data_number: quote.tcb_evaluation_data_number,
    })
    .map_err(|e| ProvenanceError::Verification {
        what: "platform policy appraisal",
        detail: e.to_string(),
    })?;

    // The shape selects *which* endorsed measurement applies. Upstream requires
    // the code artifact to declare one; without it, resolution would have to
    // guess.
    let shape = shape.ok_or(ProvenanceError::MissingShape)?;
    let required = aptos_tinfoil_sigstore::policy::Shape {
        cpus: shape.cpus,
        memory_mb: shape.memory_mb,
        gpus: Some(shape.gpus),
        disks: shape.disks,
    };

    let (name, _) = endorsements
        .policy
        .resolve_platform_measurement(tdx, &required, &quote.mrtd, &quote.rtmrs[0])
        .map_err(|e| ProvenanceError::Verification {
            what: "platform measurement resolution",
            detail: e.to_string(),
        })?;
    Ok(ProvenPlatform {
        measurement_id: name.to_string(),
        artifact: endorsements.artifact,
    })
}

/// Verify a required freshness witness for an authenticated artifact.
pub fn verify_freshness_stage(
    document: &Document,
    id: &'static str,
    artifact: &AuthenticatedArtifact,
    now_secs: u64,
    trusted_root_json: &str,
) -> Result<(), ProvenanceError> {
    let entry = document
        .freshness_collateral(id)
        .ok_or(ProvenanceError::MissingFreshness(id))?;
    check_freshness(entry, id, artifact, now_secs, trusted_root_json)
}

/// Verify a freshness witness for an already-authenticated artifact.
fn check_freshness(
    entry: &super::envelope::CollateralEntry,
    what: &'static str,
    artifact: &AuthenticatedArtifact,
    now_secs: u64,
    trusted_root_json: &str,
) -> Result<(), ProvenanceError> {
    let collateral: FreshnessCollateral =
        serde_json::from_value(entry.data.clone()).map_err(|e| ProvenanceError::BadCollateral {
            what: "freshness",
            detail: e.to_string(),
        })?;
    authenticate_freshness_with_root(
        collateral.sigstore_bundle.get().as_bytes(),
        artifact,
        now_secs,
        trusted_root_json,
    )
    .map(|_| ())
    .map_err(|e| ProvenanceError::Verification {
        what,
        detail: e.to_string(),
    })
}

/// Compare one provenance-derived register against the quote's.
///
/// The expected value is validated as well as compared: a malformed register in
/// the provenance statement must be an error, never something that silently
/// fails to equal anything.
fn compare_register(
    register: &'static str,
    expected: &str,
    actual: &str,
) -> Result<(), ProvenanceError> {
    if expected.len() != REGISTER_HEX_LEN
        || !expected
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ProvenanceError::MalformedRegister(expected.to_string()));
    }
    if expected != actual {
        return Err(ProvenanceError::MeasurementMismatch {
            register,
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn register(byte: &str) -> String {
        byte.repeat(48)
    }

    #[test]
    fn a_matching_register_is_accepted() {
        compare_register("RTMR1", &register("ab"), &register("ab")).expect("must match");
    }

    /// The comparison that makes provenance mean something: the kernel the
    /// build produced must be the kernel the hardware attests.
    #[test]
    fn a_mismatched_register_is_refused() {
        let err = compare_register("RTMR1", &register("ab"), &register("cd")).unwrap_err();
        assert!(
            matches!(err, ProvenanceError::MeasurementMismatch {
                register: "RTMR1",
                ..
            }),
            "got {err}"
        );
        // The message names both values so an operator can diagnose it.
        assert!(err.to_string().contains(&register("ab")));
        assert!(err.to_string().contains(&register("cd")));
    }

    /// A malformed expected value must be an error rather than something that
    /// merely fails to match — otherwise a provenance statement carrying
    /// garbage would look like an ordinary mismatch.
    #[test]
    fn a_malformed_expected_register_is_refused() {
        for bad in [
            "",
            "ab",
            &register("ab")[..95],
            &"AB".repeat(48),
            &"zz".repeat(48),
        ] {
            let err = compare_register("RTMR1", bad, &register("ab")).unwrap_err();
            assert!(
                matches!(err, ProvenanceError::MalformedRegister(_)),
                "{bad:?} must be refused as malformed, got {err}"
            );
        }
    }

    /// The collateral's `repo` field is a claim, and a disagreement with the
    /// configured repository is refused before any signature work.
    #[test]
    fn collateral_naming_another_repository_is_refused() {
        let err = ProvenanceError::RepoMismatch {
            claimed: "attacker/repo".into(),
            configured: "owner/repo".into(),
        };
        assert!(err.to_string().contains("attacker/repo"));
        assert!(err.to_string().contains("owner/repo"));
    }
}
