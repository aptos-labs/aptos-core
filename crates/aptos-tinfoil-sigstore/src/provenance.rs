//! Code provenance: authenticating *who built what*.
//!
//! This is the Rust port of `AuthenticateCode` in `tinfoil-go/verifier/provenance`
//! (`feat/v3`). It answers one question:
//!
//! ```text
//! trusted GitHub repo
//!         v
//! trusted GitHub Actions workflow, on a GitHub-hosted runner, from a tag
//!         v
//! Sigstore-signed in-toto statement
//!         v
//! artifact digest
//!         v
//! expected CVM measurement
//! ```
//!
//! # The repository is an input, never an output
//!
//! `repo` is supplied by the caller from *its own configuration*. It is never
//! read out of the bundle. This is the property the whole design rests on: a
//! bundle states which repository signed it, and an attacker can produce a
//! perfectly valid bundle for a repository they control. Trusting the name
//! inside the payload would authenticate the attacker's build just as happily
//! as the real one.
//!
//! # What this does not do
//!
//! It stops at "expected measurement". Binding that to a *running* CVM requires
//! comparing it against a hardware quote, which is the caller's job. See
//! [`crate::types::Measurement`] for which registers this covers (`RTMR1`,
//! `RTMR2`) and which it does not (`MRTD`, `RTMR0` — those come from the
//! platform endorsement artifact).

use crate::{
    certificate::{self, CertificateInfo, ExpectedIdentity, TAG_REF_PREFIX},
    dsse,
    error::{Error, Result},
    fulcio,
    policy::{self, Artifact},
    rekor,
    types::Measurement,
};
use serde::Deserialize;

/// A bundle whose signatures and signing identity have all verified.
///
/// Holding one of these is the precondition for reading anything out of the
/// payload. Constructing it is the only way to get there, which is what keeps
/// "parsed" from being mistaken for "authenticated".
struct VerifiedBundle {
    bundle: serde_json::Value,
    info: CertificateInfo,
    /// The transparency log's `integratedTime` for this bundle's entry, in Unix
    /// seconds — a *verified* timestamp, not one the signer chose.
    logged_at: u64,
}

/// Run the full cryptographic chain over a bundle, then check its signing
/// identity against `expected`.
///
/// Shared by every entry point in this module — code provenance, platform
/// endorsements and freshness witnesses differ only in *which identity* they
/// require, never in how much verification they do. Keeping one implementation
/// means a future check added here cannot be accidentally skipped by one
/// caller.
///
/// # Order
///
/// Signatures first, identity second. Nothing reads the payload.
#[cfg(test)]
fn verify_bundle_with_identity(
    bundle_json: &[u8],
    expected: &ExpectedIdentity,
) -> Result<VerifiedBundle> {
    verify_bundle_with_identity_and_root(
        bundle_json,
        expected,
        include_str!("../assets/trusted_root.json"),
    )
}

fn verify_bundle_with_identity_and_root(
    bundle_json: &[u8],
    expected: &ExpectedIdentity,
    trusted_root_json: &str,
) -> Result<VerifiedBundle> {
    let bundle: serde_json::Value = serde_json::from_slice(bundle_json)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to parse bundle: {e}")))?;

    // 1. DSSE signature over the PAE, plus the certificate's KeyUsage /
    //    ExtendedKeyUsage, plus embedded SCTs (duplicate log IDs rejected).
    //    Also enforces exactly one DSSE signature.
    dsse::verify_dsse_signature_with_root(&bundle, trusted_root_json)?;

    // 2. The certificate's validity window pins the clock for everything below:
    //    Rekor and Fulcio are evaluated at the time the certificate was valid,
    //    not at "now". `extract_cert_with_validity` also rejects the legacy
    //    pre-v0.3 `x509CertificateChain` bundle shape.
    let (cert_der, not_before, not_after) = crate::extract_cert_with_validity(&bundle)?;

    // 3. Transparency log. Mandatory: an entry that is not publicly logged is
    //    not verifiable after the fact. Yields the log's own timestamp, which
    //    is what dates a freshness witness — and which is covered by Rekor's
    //    signed entry timestamp, so it is attested rather than merely asserted.
    let logged_at =
        rekor::verify_rekor_entry_with_root(&bundle, not_before, not_after, trusted_root_json)?;

    // 4. The signing certificate must chain to the embedded Fulcio root.
    fulcio::verify_fulcio_chain_with_root(&cert_der, not_before, trusted_root_json)?;

    // 5. Identity: issuer, trusted repository, workflow path, GitHub-hosted
    //    runner, ref constraints, commit SHA.
    let info = crate::extract_certificate_info_from_bundle(&bundle)?;
    certificate::check_identity(&info, expected)?;

    Ok(VerifiedBundle {
        bundle,
        info,
        logged_at,
    })
}

/// An artifact whose build provenance has been cryptographically authenticated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedArtifact {
    /// The repository the build came from.
    ///
    /// This is the *trusted* name the caller passed in, echoed back after the
    /// certificate was proven to match it — not a value parsed from the bundle.
    pub repo: String,
    /// The release tag, without the `refs/tags/` prefix.
    pub tag: String,
    /// The commit the build ran against.
    pub commit: String,
    /// `subject[0].name` from the in-toto statement.
    pub subject_name: String,
    /// The artifact digest (sha256, lowercase hex).
    pub digest: String,
}

/// Declared resource shape of the VM an image expects to run in.
///
/// Upstream uses this to pick the right platform measurement: the same code
/// produces different `MRTD`/`RTMR0` values on differently shaped VMs, so the
/// shape is part of selecting which endorsed measurement applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    pub cpus: u32,
    pub memory_mb: u64,
    pub gpus: u32,
    pub disks: u32,
}

/// The result of authenticating a code provenance bundle.
#[derive(Debug, Clone)]
pub struct Code {
    pub artifact: AuthenticatedArtifact,
    pub measurement: Measurement,
    /// The declared VM shape, when the predicate carries one.
    ///
    /// # Why this is optional here, when upstream requires it
    ///
    /// Upstream's `shapeFromPredicate` requires a structured `vm_shape` member
    /// with all four fields. Real bundles from older CVM images (observed:
    /// `cvm-version: 0.7.3`) have no `vm_shape` at all — the shape is carried
    /// inside the base64 YAML `config` member instead. Requiring it at the
    /// *code* stage would reject genuine, correctly signed provenance for those
    /// images.
    ///
    /// This is not a weakening. The shape exists only to select which endorsed
    /// platform measurement applies, so it is required exactly where it is
    /// used: platform-endorsement resolution refuses to proceed without one.
    /// Authenticating who built what does not depend on it.
    pub shape: Option<Shape>,
}

/// Authenticate a Sigstore code-provenance bundle.
///
/// Mirrors upstream's
/// `provenance.AuthenticateCode(bundle, trustedRepo, tag, digest)`.
///
/// * `bundle_json` — the raw `sigstore_bundle` from the `sigstore-code`
///   collateral entry.
/// * `repo` — the **trusted** repository, from the verifier's configuration.
/// * `tag` — the expected release tag, when the caller pins one.
/// * `digest` — the expected artifact digest (sha256, lowercase hex).
///
/// # Order of operations
///
/// Signatures first, identity second, payload last. Nothing in the statement is
/// read until the DSSE signature, the transparency-log entry and the Fulcio
/// chain have all verified — a parsed-but-unauthenticated payload is attacker
/// input, and reading it earlier would be exactly the "Sigstore bundle parsed"
/// mistake this is meant to avoid.
#[cfg(test)]
pub fn authenticate_code(
    bundle_json: &[u8],
    repo: &str,
    tag: Option<&str>,
    digest: &str,
) -> Result<Code> {
    authenticate_code_with_root(
        bundle_json,
        repo,
        tag,
        digest,
        include_str!("../assets/trusted_root.json"),
    )
}

pub fn authenticate_code_with_root(
    bundle_json: &[u8],
    repo: &str,
    tag: Option<&str>,
    digest: &str,
    trusted_root_json: &str,
) -> Result<Code> {
    if !is_valid_repo(repo) {
        return Err(Error::SigstoreVerification(format!(
            "trusted repository {repo:?} is not a valid owner/name identifier"
        )));
    }
    if !is_sha256_hex(digest) {
        return Err(Error::SigstoreVerification(format!(
            "expected digest {digest:?} is not 32 bytes of lowercase hex"
        )));
    }

    let verified = verify_bundle_with_identity_and_root(
        bundle_json,
        &ExpectedIdentity::code(repo, tag),
        trusted_root_json,
    )?;
    let (bundle, info) = (verified.bundle, verified.info);

    // ---- Everything below this line is now authenticated payload. ----

    let statement = crate::parse_statement(&bundle)?;
    crate::enforce_subject0_digest(&statement, digest)?;

    let measurement = crate::measurement_from_statement(&statement)?;
    // Absent is tolerated (see `Code::shape`); present-but-malformed is not.
    let shape = match statement.predicate.get("vm_shape") {
        Some(_) => Some(shape_from_predicate(&statement.predicate)?),
        None => None,
    };

    let artifact = AuthenticatedArtifact {
        // The caller's trusted name, not the bundle's.
        repo: repo.to_string(),
        tag: info
            .source_repository_ref
            .strip_prefix(TAG_REF_PREFIX)
            .unwrap_or_default()
            .to_string(),
        commit: info.source_repository_digest.clone(),
        subject_name: statement
            .subject
            .first()
            .map(|s| s.name.clone())
            .unwrap_or_default(),
        digest: digest.to_ascii_lowercase(),
    };

    Ok(Code {
        artifact,
        measurement,
        shape,
    })
}

/// The result of authenticating a platform-endorsements bundle.
#[derive(Debug, Clone)]
pub struct PlatformEndorsements {
    pub artifact: AuthenticatedArtifact,
    /// The parsed policy document: endorsed machines, their appraisal
    /// policies, and the expected `MRTD`/`RTMR0` per VM shape.
    pub policy: Artifact,
}

/// A signed TDX firmware measurement from the hardware-measurements release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareMeasurement {
    pub name: String,
    pub mrtd: String,
    pub rtmr0: String,
}

#[cfg(test)]
const HARDWARE_MEASUREMENTS_V1: &str = "https://tinfoil.sh/predicate/hardware-measurements/v1";

/// Authenticate and decode the legacy hardware-measurements artifact.
///
/// This matches the Go verifier's `FetchHardwareMeasurements`: the caller
/// supplies the trusted repository and release digest, the bundle must be
/// signed by that repository from a tagged GitHub Actions workflow, and each
/// predicate member supplies one accepted MRTD/RTMR0 pair.
#[cfg(test)]
pub fn authenticate_hardware_measurements(
    bundle_json: &[u8],
    repo: &str,
    digest: &str,
) -> Result<Vec<HardwareMeasurement>> {
    if !is_valid_repo(repo) {
        return Err(Error::SigstoreVerification(format!(
            "trusted repository {repo:?} is not a valid owner/name identifier"
        )));
    }
    if !is_sha256_hex(digest) {
        return Err(Error::SigstoreVerification(format!(
            "expected digest {digest:?} is not 32 bytes of lowercase hex"
        )));
    }

    // The reference verifier accepts any tagged workflow in the trusted
    // hardware repository; the artifact digest remains pinned by subject[0].
    let verified = verify_bundle_with_identity(bundle_json, &ExpectedIdentity::code(repo, None))?;
    let statement = crate::parse_statement(&verified.bundle)?;
    crate::enforce_subject0_digest(&statement, digest)?;
    if statement.predicate_type != HARDWARE_MEASUREMENTS_V1 {
        return Err(Error::SigstoreVerification(format!(
            "unexpected hardware predicate type: {}",
            statement.predicate_type
        )));
    }
    parse_hardware_measurements(&statement.predicate, digest)
}

#[cfg(test)]
fn parse_hardware_measurements(
    predicate: &serde_json::Value,
    digest: &str,
) -> Result<Vec<HardwareMeasurement>> {
    let entries = predicate.as_object().ok_or_else(|| {
        Error::SigstoreVerification("hardware measurements predicate is not an object".into())
    })?;
    let mut measurements = Vec::with_capacity(entries.len());
    for (name, value) in entries {
        let mrtd = hardware_register(value, name, "mrtd")?;
        let rtmr0 = hardware_register(value, name, "rtmr0")?;
        measurements.push(HardwareMeasurement {
            name: format!("{name}@{digest}"),
            mrtd,
            rtmr0,
        });
    }
    if measurements.is_empty() {
        return Err(Error::SigstoreVerification(
            "hardware measurements predicate is empty".into(),
        ));
    }
    Ok(measurements)
}

#[cfg(test)]
fn hardware_register(value: &serde_json::Value, name: &str, field: &str) -> Result<String> {
    let register = value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            Error::SigstoreVerification(format!(
                "hardware measurement {name:?} has no {field} string"
            ))
        })?;
    if register.len() != 96
        || !register
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::SigstoreVerification(format!(
            "hardware measurement {name:?} has malformed {field}"
        )));
    }
    Ok(register.to_owned())
}

/// Authenticate a Sigstore platform-endorsements bundle.
///
/// Mirrors upstream's
/// `provenance.AuthenticatePlatformEndorsements(bundle, repo, tag, digest)`.
///
/// # Why the repository is fixed rather than caller-chosen
///
/// Unlike [`authenticate_code`], where the trusted repository is deployment
/// configuration, the endorsements publisher is a constant of the Tinfoil
/// ecosystem: [`policy::PLATFORM_ENDORSEMENTS_REPO`]. This artifact decides
/// *which machines may host workloads at all*, so allowing an operator to point
/// it at an arbitrary repository would let one misconfiguration endorse an
/// attacker's fleet. The `repo` parameter exists only so a caller can be told
/// it disagrees — it is checked against the constant, never trusted.
#[cfg(test)]
pub fn authenticate_platform_endorsements(
    bundle_json: &[u8],
    repo: &str,
    tag: Option<&str>,
    digest: &str,
) -> Result<PlatformEndorsements> {
    authenticate_platform_endorsements_with_root(
        bundle_json,
        repo,
        tag,
        digest,
        include_str!("../assets/trusted_root.json"),
    )
}

pub fn authenticate_platform_endorsements_with_root(
    bundle_json: &[u8],
    repo: &str,
    tag: Option<&str>,
    digest: &str,
    trusted_root_json: &str,
) -> Result<PlatformEndorsements> {
    if repo != policy::PLATFORM_ENDORSEMENTS_REPO {
        return Err(Error::SigstoreVerification(format!(
            "platform endorsements repo {repo:?} does not equal {:?}",
            policy::PLATFORM_ENDORSEMENTS_REPO
        )));
    }
    if !is_sha256_hex(digest) {
        return Err(Error::SigstoreVerification(format!(
            "expected digest {digest:?} is not 32 bytes of lowercase hex"
        )));
    }

    let verified = verify_bundle_with_identity_and_root(
        bundle_json,
        &ExpectedIdentity::platform_endorsements(tag),
        trusted_root_json,
    )?;
    let (bundle, info) = (verified.bundle, verified.info);

    // ---- Everything below this line is now authenticated payload. ----

    let statement = crate::parse_statement(&bundle)?;
    crate::enforce_subject0_digest(&statement, digest)?;

    if statement.predicate_type != policy::ARTIFACT_FORMAT {
        return Err(Error::SigstoreVerification(format!(
            "unexpected predicate type: {}",
            statement.predicate_type
        )));
    }

    // Upstream re-marshals the predicate (a protobuf `Struct`) with protojson
    // before handing it to the strict parser. Here the predicate is already
    // plain JSON, so it is serialized once and parsed with the same strictness:
    // unknown members, duplicate members and dangling references all refused.
    let predicate_json = serde_json::to_vec(&statement.predicate).map_err(|e| {
        Error::SigstoreVerification(format!("encoding platform endorsements predicate: {e}"))
    })?;
    let parsed = Artifact::parse(&predicate_json)?;

    let artifact = AuthenticatedArtifact {
        repo: policy::PLATFORM_ENDORSEMENTS_REPO.to_string(),
        tag: info
            .source_repository_ref
            .strip_prefix(TAG_REF_PREFIX)
            .unwrap_or_default()
            .to_string(),
        commit: info.source_repository_digest.clone(),
        subject_name: statement
            .subject
            .first()
            .map(|s| s.name.clone())
            .unwrap_or_default(),
        digest: digest.to_ascii_lowercase(),
    };

    Ok(PlatformEndorsements {
        artifact,
        policy: parsed,
    })
}

/// Predicate type of a freshness witness.
pub const FRESHNESS_PREDICATE_FORMAT: &str = "https://tinfoil.sh/predicate/freshness-witness/v1";

/// How old a freshness witness may be before it is considered stale.
///
/// Seven days, matching upstream's `MaxFreshnessAge`.
pub const MAX_FRESHNESS_AGE_SECS: u64 = 7 * 24 * 60 * 60;

/// How far into the future a witness timestamp may sit before it is rejected.
///
/// Five minutes, matching upstream's `MaxFreshnessFutureSkew`. Transparency-log
/// clocks and the verifier's clock are independent, so a small forward skew is
/// normal; anything beyond it is not.
pub const MAX_FRESHNESS_FUTURE_SKEW_SECS: u64 = 5 * 60;

/// The artifact a witness endorses.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WitnessEndorsement {
    repo: String,
    tag: String,
    commit: String,
    subject: WitnessSubject,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WitnessSubject {
    name: String,
    /// `sha256:<64 lowercase hex>` — note the prefix, unlike the bare hex in
    /// the in-toto subject.
    digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FreshnessWitness {
    format: String,
    endorses: WitnessEndorsement,
}

/// Authenticate a freshness witness for an already-authenticated artifact.
///
/// Mirrors upstream's `provenance.AuthenticateFreshness(bundle, expected, now)`.
///
/// # What a witness is for
///
/// Code provenance proves an artifact was built from a trusted repository, but
/// says nothing about *when* — a correctly signed bundle stays valid forever.
/// A witness is a separate, continuously republished attestation that a given
/// artifact was still current recently. Requiring one bounds how long a
/// withdrawn or superseded release stays acceptable.
///
/// # Where the time comes from
///
/// Not from the witness payload: a signer could put anything there. The date is
/// the **transparency log's** `integratedTime` for the witness's own Rekor
/// entry, verified as part of the bundle. Upstream takes the earliest such
/// timestamp; our Rekor verification already requires exactly one tlog entry,
/// so there is exactly one to take.
///
/// That timestamp is covered by Rekor's Signed Entry Timestamp, which is
/// verified against the log's key in the trusted root. It is therefore signed
/// by the log rather than merely constrained by the signing certificate's
/// validity window.
///
/// `now_secs` is supplied by the caller rather than read from the clock here, so
/// verification stays a pure function of its inputs.
#[cfg(test)]
pub fn authenticate_freshness(
    bundle_json: &[u8],
    expected: &AuthenticatedArtifact,
    now_secs: u64,
) -> Result<u64> {
    authenticate_freshness_with_root(
        bundle_json,
        expected,
        now_secs,
        include_str!("../assets/trusted_root.json"),
    )
}

pub fn authenticate_freshness_with_root(
    bundle_json: &[u8],
    expected: &AuthenticatedArtifact,
    now_secs: u64,
    trusted_root_json: &str,
) -> Result<u64> {
    validate_authenticated_artifact(expected)?;

    let verified = verify_bundle_with_identity_and_root(
        bundle_json,
        &ExpectedIdentity::freshness_witness(),
        trusted_root_json,
    )?;

    // ---- Everything below this line is now authenticated payload. ----

    let statement = crate::parse_statement(&verified.bundle)?;

    require_v1_statement(&statement.type_)?;

    // The witness must be *about* this artifact, bound three separate ways: the
    // Sigstore artifact digest, the in-toto subject, and the predicate's
    // `endorses` block. A witness for some other release must not keep this one
    // looking current.
    crate::enforce_subject0_digest(&statement, &expected.digest)?;

    if statement.predicate_type != FRESHNESS_PREDICATE_FORMAT {
        return Err(Error::SigstoreVerification(format!(
            "unexpected freshness predicate type: {}",
            statement.predicate_type
        )));
    }

    if statement.subject.len() != 1 {
        return Err(Error::SigstoreVerification(format!(
            "freshness statement must have exactly 1 subject, got {}",
            statement.subject.len()
        )));
    }
    let subject = &statement.subject[0];
    let subject_digest = subject.digest.get("sha256").map(String::as_str);
    if subject.name != expected.subject_name || subject_digest != Some(expected.digest.as_str()) {
        return Err(Error::SigstoreVerification(
            "freshness statement subject does not match authenticated artifact".into(),
        ));
    }

    let witness: FreshnessWitness = serde_json::from_value(statement.predicate.clone())
        .map_err(|e| Error::SigstoreVerification(format!("parsing freshness predicate: {e}")))?;

    // `predicate.format` is checked as well as `predicateType`: they are two
    // separate assertions, and a witness must not satisfy one while carrying
    // something else in the other.
    if witness.format != FRESHNESS_PREDICATE_FORMAT {
        return Err(Error::SigstoreVerification(format!(
            "unexpected freshness predicate format: {}",
            witness.format
        )));
    }

    validate_witness(&witness, expected)?;

    validate_freshness_time(verified.logged_at, now_secs)
}

/// The witness's `endorses` block must name exactly this artifact.
fn validate_witness(witness: &FreshnessWitness, expected: &AuthenticatedArtifact) -> Result<()> {
    let endorses = &witness.endorses;
    // Note the digest here is `sha256:`-prefixed, unlike the bare hex in the
    // in-toto subject.
    let want_digest = format!("sha256:{}", expected.digest);
    if endorses.repo != expected.repo
        || endorses.tag != expected.tag
        || endorses.commit != expected.commit
        || endorses.subject.name != expected.subject_name
        || endorses.subject.digest != want_digest
    {
        return Err(Error::SigstoreVerification(
            "freshness witness does not match authenticated artifact".into(),
        ));
    }
    Ok(())
}

/// The artifact a witness is being checked against must itself be well formed.
///
/// A caller that passes a half-populated artifact would otherwise get
/// vacuous equality checks — empty strings matching empty strings.
fn validate_authenticated_artifact(expected: &AuthenticatedArtifact) -> Result<()> {
    if !is_valid_repo(&expected.repo) {
        return Err(Error::SigstoreVerification(
            "authenticated artifact repository is invalid".into(),
        ));
    }
    if expected.tag.is_empty() {
        return Err(Error::SigstoreVerification(
            "authenticated artifact tag is empty".into(),
        ));
    }
    if !is_commit_sha(&expected.commit) {
        return Err(Error::SigstoreVerification(
            "authenticated artifact commit is malformed".into(),
        ));
    }
    if expected.subject_name.is_empty() {
        return Err(Error::SigstoreVerification(
            "authenticated artifact subject name is empty".into(),
        ));
    }
    if !is_sha256_hex(&expected.digest) {
        return Err(Error::SigstoreVerification(
            "authenticated artifact digest is malformed".into(),
        ));
    }
    Ok(())
}

/// Bound how old — and how far in the future — a witness timestamp may be.
fn validate_freshness_time(logged_at: u64, now_secs: u64) -> Result<u64> {
    if logged_at > now_secs.saturating_add(MAX_FRESHNESS_FUTURE_SKEW_SECS) {
        return Err(Error::SigstoreVerification(
            "freshness witness timestamp is in the future".into(),
        ));
    }
    if now_secs.saturating_sub(logged_at) > MAX_FRESHNESS_AGE_SECS {
        return Err(Error::SigstoreVerification(
            "freshness witness is stale".into(),
        ));
    }
    Ok(logged_at)
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The `vm_shape` member of a code predicate.
///
/// All four fields are required. Upstream refuses a partial shape rather than
/// defaulting the missing values, because a wrong shape selects a wrong
/// platform measurement.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawShape {
    cpus: serde_json::Number,
    memory_mb: serde_json::Number,
    gpus: serde_json::Number,
    disks: serde_json::Number,
}

/// Extract the VM shape from an authenticated predicate.
pub fn shape_from_predicate(predicate: &serde_json::Value) -> Result<Shape> {
    let raw = predicate
        .get("vm_shape")
        .ok_or_else(|| Error::SigstoreVerification("Missing vm_shape in predicate".into()))?;
    let raw: RawShape = serde_json::from_value(raw.clone())
        .map_err(|e| Error::SigstoreVerification(format!("Invalid vm_shape: {e}")))?;

    Ok(Shape {
        cpus: integral_u32(&raw.cpus, "cpus")?,
        memory_mb: integral(&raw.memory_mb, "memory_mb")?,
        gpus: integral_u32(&raw.gpus, "gpus")?,
        disks: integral_u32(&raw.disks, "disks")?,
    })
}

/// Require a JSON number to be a non-negative, finite integer in range.
///
/// A float, a negative, or an out-of-range value means the producer and the
/// verifier disagree about what the predicate says, which is not something to
/// paper over with a cast.
fn integral(value: &serde_json::Number, field: &str) -> Result<u64> {
    if let Some(n) = value.as_u64() {
        return Ok(n);
    }
    Err(Error::SigstoreVerification(format!(
        "vm_shape.{field} must be a non-negative integer, got {value}"
    )))
}

/// Require a freshness statement to be in-toto v1 specifically.
///
/// [`crate::parse_statement`] also accepts the legacy v0.1, because real older
/// code and endorsement bundles exist in that form. No freshness witness is
/// produced that way, and upstream's `freshness.go` pins v1 outright, so
/// accepting v0.1 here would only widen what this verifier reads for no
/// deployment benefit.
fn require_v1_statement(type_: &str) -> Result<()> {
    if type_ != crate::IN_TOTO_STATEMENT_V1 {
        return Err(Error::SigstoreVerification(format!(
            "freshness statement must be {}, got {:?}",
            crate::IN_TOTO_STATEMENT_V1,
            type_
        )));
    }
    Ok(())
}

/// The same, for a field [`Shape`] stores as a `u32`.
///
/// The narrowing is a checked conversion rather than an `as` cast, because a
/// truncating cast would make the verifier read a *different* shape than the
/// signer wrote: `cpus: 4294967304` would become `8` and then match an endorsed
/// 8-CPU measurement. Since the shape selects which platform measurement
/// applies, that is a wrong-measurement selection, which is exactly what
/// [`integral`]'s contract exists to prevent.
fn integral_u32(value: &serde_json::Number, field: &str) -> Result<u32> {
    let n = integral(value, field)?;
    u32::try_from(n).map_err(|_| {
        Error::SigstoreVerification(format!(
            "vm_shape.{field} must fit in a 32-bit unsigned integer, got {n}"
        ))
    })
}

/// A repository identifier is `owner/name` with conservative character rules.
///
/// Validated before it reaches the SAN regex: the repository is escaped there,
/// but rejecting a malformed name outright is cheaper than reasoning about what
/// an escaped oddity would match.
fn is_valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    [owner, name].iter().all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    })
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> String {
        "ab".repeat(32)
    }

    /// The bundle must never be consulted when the caller's own inputs are
    /// malformed: these are configuration errors, caught before any parsing.
    #[test]
    fn caller_inputs_are_validated_first() {
        for bad_repo in ["", "noslash", "a/b/c", "/b", "a/", "a b/c"] {
            let err = authenticate_code(b"not json", bad_repo, None, &digest()).unwrap_err();
            assert!(
                err.to_string().contains("not a valid owner/name"),
                "{bad_repo:?} must be refused before parsing, got {err}"
            );
        }

        for bad_digest in ["", "xyz", &"ab".repeat(31), &"AB".repeat(32)] {
            let err = authenticate_code(b"not json", "owner/repo", None, bad_digest).unwrap_err();
            assert!(
                err.to_string().contains("not 32 bytes of lowercase hex"),
                "{bad_digest:?} must be refused, got {err}"
            );
        }
    }

    #[test]
    fn a_malformed_bundle_is_refused() {
        let err = authenticate_code(b"not json", "owner/repo", None, &digest()).unwrap_err();
        assert!(err.to_string().contains("Failed to parse bundle"), "{err}");
    }

    /// An empty bundle must fail at the *signature* stage, not by silently
    /// finding nothing to check.
    #[test]
    fn an_empty_bundle_fails_signature_verification() {
        let err = authenticate_code(b"{}", "owner/repo", None, &digest()).unwrap_err();
        assert!(
            err.to_string().contains("certificate"),
            "expected a signature-stage failure, got {err}"
        );
    }

    #[test]
    fn a_complete_shape_parses() {
        let predicate = serde_json::json!({
            "vm_shape": { "cpus": 8, "memory_mb": 16384, "gpus": 1, "disks": 2 }
        });
        assert_eq!(shape_from_predicate(&predicate).unwrap(), Shape {
            cpus: 8,
            memory_mb: 16384,
            gpus: 1,
            disks: 2
        });
    }

    /// A partial shape must be refused rather than defaulted: a wrong shape
    /// selects a wrong platform measurement.
    #[test]
    fn a_partial_shape_is_refused() {
        for predicate in [
            serde_json::json!({}),
            serde_json::json!({ "vm_shape": {} }),
            serde_json::json!({ "vm_shape": { "cpus": 8 } }),
            serde_json::json!({ "vm_shape": { "cpus": 8, "memory_mb": 1, "gpus": 0 } }),
        ] {
            assert!(
                shape_from_predicate(&predicate).is_err(),
                "partial shape must be refused: {predicate}"
            );
        }
    }

    #[test]
    fn a_non_integral_or_negative_shape_is_refused() {
        for value in [
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!("8"),
        ] {
            let predicate = serde_json::json!({
                "vm_shape": { "cpus": value, "memory_mb": 1, "gpus": 0, "disks": 1 }
            });
            assert!(
                shape_from_predicate(&predicate).is_err(),
                "{value} must be refused"
            );
        }
    }

    /// A value too large for the field's type must be refused, not truncated.
    ///
    /// This is the case a plain `as u32` cast would get wrong: 2^32 + 8
    /// truncates to 8, so a predicate declaring an absurd CPU count would be
    /// read as an ordinary 8-CPU shape and match an endorsed 8-CPU
    /// measurement. Because the shape selects *which* platform measurement
    /// applies, that is a wrong-measurement selection.
    #[test]
    fn an_out_of_range_shape_value_is_refused_not_truncated() {
        for field in ["cpus", "gpus", "disks"] {
            let mut shape = serde_json::json!({
                "cpus": 8, "memory_mb": 1, "gpus": 0, "disks": 1
            });
            // 2^32 + 8: truncates to exactly the plausible value 8.
            shape[field] = serde_json::json!(4_294_967_304u64);
            let predicate = serde_json::json!({ "vm_shape": shape });

            let err = shape_from_predicate(&predicate)
                .expect_err("an out-of-range {field} must be refused");
            assert!(
                err.to_string().contains(field),
                "the error must name the offending field, got {err}"
            );
        }

        // `memory_mb` is a u64, so the same value is legitimately in range.
        let predicate = serde_json::json!({
            "vm_shape": { "cpus": 8, "memory_mb": 4_294_967_304u64, "gpus": 0, "disks": 1 }
        });
        assert_eq!(
            shape_from_predicate(&predicate)
                .expect("in range for u64")
                .memory_mb,
            4_294_967_304
        );
    }

    /// An unknown member in `vm_shape` means the producer is saying something
    /// this verifier does not understand.
    #[test]
    fn an_unknown_shape_member_is_refused() {
        let predicate = serde_json::json!({
            "vm_shape": { "cpus": 8, "memory_mb": 1, "gpus": 0, "disks": 1, "extra": 1 }
        });
        assert!(shape_from_predicate(&predicate).is_err());
    }

    #[test]
    fn hardware_measurements_are_named_by_release_digest() {
        let digest = digest();
        let measurements = parse_hardware_measurements(
            &serde_json::json!({
                "large_1d_v011": {
                    "mrtd": "11".repeat(48),
                    "rtmr0": "22".repeat(48),
                }
            }),
            &digest,
        )
        .unwrap();
        assert_eq!(measurements, vec![HardwareMeasurement {
            name: format!("large_1d_v011@{digest}"),
            mrtd: "11".repeat(48),
            rtmr0: "22".repeat(48),
        }]);
    }

    #[test]
    fn hardware_measurements_require_complete_lowercase_registers() {
        for predicate in [
            serde_json::json!({}),
            serde_json::json!({"machine": {"mrtd": "11".repeat(48)}}),
            serde_json::json!({
                "machine": {"mrtd": "11".repeat(48), "rtmr0": "AA".repeat(48)}
            }),
        ] {
            assert!(parse_hardware_measurements(&predicate, &digest()).is_err());
        }
    }
}

#[cfg(test)]
mod freshness_tests {
    use super::*;

    fn artifact() -> AuthenticatedArtifact {
        AuthenticatedArtifact {
            repo: "owner/repo".into(),
            tag: "v1.2.3".into(),
            commit: "a".repeat(40),
            subject_name: "tinfoil-deployment.json".into(),
            digest: "ab".repeat(32),
        }
    }

    fn witness_for(expected: &AuthenticatedArtifact) -> FreshnessWitness {
        FreshnessWitness {
            format: FRESHNESS_PREDICATE_FORMAT.into(),
            endorses: WitnessEndorsement {
                repo: expected.repo.clone(),
                tag: expected.tag.clone(),
                commit: expected.commit.clone(),
                subject: WitnessSubject {
                    name: expected.subject_name.clone(),
                    digest: format!("sha256:{}", expected.digest),
                },
            },
        }
    }

    // ---- Witness -> artifact binding ----

    #[test]
    fn a_matching_witness_is_accepted() {
        let expected = artifact();
        validate_witness(&witness_for(&expected), &expected).expect("baseline must pass");
    }

    /// The point of the binding: a witness for some *other* release must not
    /// keep this one looking current. Each field is load-bearing, so each is
    /// perturbed independently.
    #[test]
    fn a_witness_for_another_artifact_is_refused() {
        let expected = artifact();

        let mut wrong_repo = witness_for(&expected);
        wrong_repo.endorses.repo = "attacker/repo".into();
        assert!(validate_witness(&wrong_repo, &expected).is_err(), "repo");

        let mut wrong_tag = witness_for(&expected);
        wrong_tag.endorses.tag = "v9.9.9".into();
        assert!(validate_witness(&wrong_tag, &expected).is_err(), "tag");

        let mut wrong_commit = witness_for(&expected);
        wrong_commit.endorses.commit = "b".repeat(40);
        assert!(
            validate_witness(&wrong_commit, &expected).is_err(),
            "commit"
        );

        let mut wrong_name = witness_for(&expected);
        wrong_name.endorses.subject.name = "other.json".into();
        assert!(validate_witness(&wrong_name, &expected).is_err(), "name");

        let mut wrong_digest = witness_for(&expected);
        wrong_digest.endorses.subject.digest = format!("sha256:{}", "cd".repeat(32));
        assert!(
            validate_witness(&wrong_digest, &expected).is_err(),
            "digest"
        );
    }

    /// The predicate digest is `sha256:`-prefixed while the in-toto subject
    /// digest is bare hex. A witness carrying the bare form must not pass:
    /// that would mean the two encodings were being compared loosely.
    #[test]
    fn the_endorsed_digest_must_carry_its_prefix() {
        let expected = artifact();
        let mut unprefixed = witness_for(&expected);
        unprefixed.endorses.subject.digest = expected.digest.clone();
        assert!(validate_witness(&unprefixed, &expected).is_err());
    }

    // ---- Caller-input validation ----

    /// A half-populated artifact would make the equality checks vacuous —
    /// empty strings matching empty strings — so it is refused up front.
    #[test]
    fn a_malformed_artifact_is_refused() {
        type Mutate = fn(&mut AuthenticatedArtifact);
        let cases: [(&str, Mutate); 5] = [
            ("repo", |a| a.repo = "noslash".into()),
            ("tag", |a| a.tag = String::new()),
            ("commit", |a| a.commit = "short".into()),
            ("subject_name", |a| a.subject_name = String::new()),
            ("digest", |a| a.digest = "xyz".into()),
        ];
        for (field, mutate) in cases {
            let mut bad = artifact();
            mutate(&mut bad);
            assert!(
                validate_authenticated_artifact(&bad).is_err(),
                "{field} must be validated"
            );
        }
        validate_authenticated_artifact(&artifact()).expect("the baseline must be valid");
    }

    // ---- Time window ----

    const NOW: u64 = 1_800_000_000;

    #[test]
    fn a_recent_witness_is_fresh() {
        let logged_at = NOW - 3600;
        assert_eq!(validate_freshness_time(logged_at, NOW).unwrap(), logged_at);
        // Exactly now is fine.
        assert!(validate_freshness_time(NOW, NOW).is_ok());
    }

    /// Seven days is the boundary, and it is inclusive: upstream compares with
    /// strictly-greater, so a witness exactly at the limit is still fresh.
    #[test]
    fn the_staleness_boundary_is_inclusive() {
        assert!(
            validate_freshness_time(NOW - MAX_FRESHNESS_AGE_SECS, NOW).is_ok(),
            "exactly the max age must still be fresh"
        );
        let err = validate_freshness_time(NOW - MAX_FRESHNESS_AGE_SECS - 1, NOW).unwrap_err();
        assert!(err.to_string().contains("stale"), "{err}");
    }

    #[test]
    fn a_stale_witness_is_refused() {
        let err = validate_freshness_time(NOW - 30 * 24 * 3600, NOW).unwrap_err();
        assert!(err.to_string().contains("stale"), "{err}");
    }

    /// A small forward skew is normal — the log's clock and ours are
    /// independent — but a witness dated well into the future is not.
    #[test]
    fn future_skew_is_tolerated_only_within_the_window() {
        assert!(
            validate_freshness_time(NOW + MAX_FRESHNESS_FUTURE_SKEW_SECS, NOW).is_ok(),
            "skew at exactly the limit must be tolerated"
        );
        let err =
            validate_freshness_time(NOW + MAX_FRESHNESS_FUTURE_SKEW_SECS + 1, NOW).unwrap_err();
        assert!(err.to_string().contains("in the future"), "{err}");
    }

    /// A witness far in the future must be reported as such rather than
    /// wrapping around into "fresh" through unsigned subtraction.
    #[test]
    fn a_far_future_timestamp_does_not_wrap() {
        let err = validate_freshness_time(u64::MAX, NOW).unwrap_err();
        assert!(err.to_string().contains("in the future"), "{err}");
    }

    /// The constants must match upstream exactly, since they define how long a
    /// withdrawn release stays acceptable.
    #[test]
    fn the_freshness_constants_match_upstream() {
        assert_eq!(MAX_FRESHNESS_AGE_SECS, 7 * 24 * 60 * 60);
        assert_eq!(MAX_FRESHNESS_FUTURE_SKEW_SECS, 5 * 60);
    }

    // ---- Entry point ----

    #[test]
    fn a_malformed_artifact_is_refused_before_the_bundle_is_parsed() {
        let mut bad = artifact();
        bad.repo = "noslash".into();
        let err = authenticate_freshness(b"not json", &bad, NOW).unwrap_err();
        assert!(
            err.to_string().contains("repository is invalid"),
            "caller input must be checked before parsing, got {err}"
        );
    }

    #[test]
    fn a_malformed_bundle_is_refused() {
        let err = authenticate_freshness(b"not json", &artifact(), NOW).unwrap_err();
        assert!(err.to_string().contains("Failed to parse bundle"), "{err}");
    }

    /// Freshness witnesses are pinned to in-toto v1, as upstream pins them.
    ///
    /// The legacy v0.1 is accepted by `parse_statement` for the code and
    /// endorsement paths, where genuine older bundles exist. It must not be
    /// accepted for a witness, which no producer emits that way.
    #[test]
    fn a_freshness_statement_must_be_in_toto_v1() {
        require_v1_statement(crate::IN_TOTO_STATEMENT_V1).expect("v1 is the pinned type");

        for rejected in [
            crate::IN_TOTO_STATEMENT_V0_1,
            "https://in-toto.io/Statement/v2",
            "",
        ] {
            let err = require_v1_statement(rejected)
                .expect_err("only in-toto v1 may date a freshness witness");
            assert!(
                err.to_string().contains(crate::IN_TOTO_STATEMENT_V1),
                "the error must name the required type, got {err}"
            );
        }
    }
}
