// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Offline Tinfoil v3-envelope verification for Atlas FormatV2 collateral.
//! Current upstream FormatV3 config/runtime collateral is explicitly rejected
//! by the parser. All roots, time, and policy inputs come from Move state;
//! no network or host-clock access is permitted.

use aptos_gas_schedule::{
    gas_feature_versions::RELEASE_V1_51,
    gas_params::natives::aptos_framework::{TINFOIL_V3_VERIFY_BASE, TINFOIL_V3_VERIFY_PER_BYTE},
};
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeError, SafeNativeResult,
};
use aptos_tinfoil_sigstore::provenance::{AuthenticatedArtifact, Shape};
use aptos_tinfoil_v3::{
    envelope::{Document, COLLATERAL_SIGSTORE_CODE_V1_FORMAT},
    tdx::AuthenticatedQuote,
    CodeStage, PlatformStage, QuoteStage,
};
use move_core_types::{
    gas_algebra::NumBytes,
    language_storage::{OPTION_NONE_TAG, OPTION_SOME_TAG},
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use serde::Deserialize;
use smallvec::{smallvec, SmallVec};
use std::{borrow::Cow, collections::VecDeque};

const EINVALID_TINFOIL_PROOF: u64 = 1;
const MAX_ROOT_BYTES: usize = 60 * 1024;
const MAX_REPO_BYTES: usize = 200;
const MAX_STAGE_BYTES: usize = 1024 * 1024;
const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
const MAX_EAT_BYTES: usize = 1024 * 1024;
const MAX_ATTESTATION_BYTES: usize = MAX_DOCUMENT_BYTES + MAX_EAT_BYTES;
const MAX_STAGE_STATE_BYTES: usize = 4096;
const ATTESTATION_PACKAGE_MAGIC: &[u8] = b"APTOS-TINFOIL-V3\0";
const ATTESTATION_PACKAGE_VERSION: u8 = 1;

/// BCS fields follow the magic prefix in this exact order. The EAT is kept
/// verbatim in the Move object for future GPU verification; no GPU claim is
/// currently accepted without a quote-bound device identity correlation.
#[derive(Deserialize)]
struct AttestationPackageV1 {
    version: u8,
    tinfoil_document: Vec<u8>,
    nras_detached_eat: Vec<u8>,
}

fn invalid() -> SafeNativeError {
    SafeNativeError::abort(EINVALID_TINFOIL_PROOF)
}

fn require_gas_feature_version(context: &SafeNativeContext) -> SafeNativeResult<()> {
    if context.gas_feature_version() < RELEASE_V1_51 {
        return Err(invalid());
    }
    Ok(())
}

/// Third-party DER/JSON verifiers contain a few guarded `expect` paths. A
/// malformed fragment must deterministically abort this transaction rather
/// than unwind across the Move VM boundary.
fn verified<T>(
    f: impl FnOnce() -> Result<T, aptos_tinfoil_v3::VerifyError>,
) -> SafeNativeResult<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .map_err(|_| invalid())?
        .map_err(|_| invalid())
}

/// Evidence rejection is a value in the object workflow, so Move can store
/// FAIL and emit its outcome event. VM limits and invariant errors still abort.
fn attempted<T>(f: impl FnOnce() -> SafeNativeResult<T>) -> SafeNativeResult<Option<T>> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(value)) => Ok(Some(value)),
        Ok(Err(SafeNativeError::Abort { abort_code, .. }))
            if abort_code == EINVALID_TINFOIL_PROOF =>
        {
            Ok(None)
        },
        Ok(Err(err)) => Err(err),
        Err(_) => Ok(None),
    }
}

fn option_to_move(context: &SafeNativeContext, value: Option<Value>) -> SafeNativeResult<Value> {
    let enum_enabled = context.get_feature_flags().is_enum_option_enabled();
    match (enum_enabled, value) {
        (true, Some(value)) => Ok(Value::struct_(Struct::pack_variant(
            OPTION_SOME_TAG,
            vec![value],
        ))),
        (true, None) => Ok(Value::struct_(Struct::pack_variant(
            OPTION_NONE_TAG,
            vec![],
        ))),
        (false, Some(value)) => Ok(Value::struct_(Struct::pack(vec![Value::vector_unchecked(
            vec![value],
        )?]))),
        (false, None) => Ok(Value::struct_(Struct::pack(vec![Value::vector_unchecked(
            vec![],
        )?]))),
    }
}

/// Full documents are stored once in a verification object and read by each
/// advance transaction. Charge for all bytes parsed by that stage, including
/// the root and authenticated state supplied by Move.
fn charge_document(
    context: &mut SafeNativeContext,
    document: &[u8],
    root: &[u8],
    extra_len: usize,
) -> SafeNativeResult<()> {
    let len = document
        .len()
        .saturating_add(root.len())
        .saturating_add(extra_len);
    context.charge(TINFOIL_V3_VERIFY_BASE + TINFOIL_V3_VERIFY_PER_BYTE * NumBytes::new(len as u64))
}

fn check_attestation_args(
    attestation: &[u8],
    root: &[u8],
    extra_len: usize,
) -> SafeNativeResult<()> {
    if attestation.is_empty()
        || attestation.len() > MAX_ATTESTATION_BYTES
        || root.is_empty()
        || root.len() > MAX_ROOT_BYTES
        || extra_len > MAX_STAGE_STATE_BYTES
    {
        return Err(invalid());
    }
    Ok(())
}

fn tinfoil_document(attestation: &[u8]) -> SafeNativeResult<Cow<'_, [u8]>> {
    if attestation
        .iter()
        .copied()
        .find(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        == Some(b'{')
    {
        if attestation.len() > MAX_DOCUMENT_BYTES {
            return Err(invalid());
        }
        return Ok(Cow::Borrowed(attestation));
    }
    let package_bytes = attestation
        .strip_prefix(ATTESTATION_PACKAGE_MAGIC)
        .ok_or_else(invalid)?;
    let package: AttestationPackageV1 = bcs::from_bytes(package_bytes).map_err(|_| invalid())?;
    if package.version != ATTESTATION_PACKAGE_VERSION
        || package.tinfoil_document.is_empty()
        || package.tinfoil_document.len() > MAX_DOCUMENT_BYTES
        || package.nras_detached_eat.len() > MAX_EAT_BYTES
    {
        return Err(invalid());
    }
    Ok(Cow::Owned(package.tinfoil_document))
}

fn charge_stage(
    context: &mut SafeNativeContext,
    fragment: &[u8],
    nonce: &[u8],
    root: &[u8],
    extra_len: usize,
) -> SafeNativeResult<()> {
    let len = fragment
        .len()
        .saturating_add(nonce.len())
        .saturating_add(root.len())
        .saturating_add(extra_len);
    context
        .charge(TINFOIL_V3_VERIFY_BASE + TINFOIL_V3_VERIFY_PER_BYTE * NumBytes::new(len as u64))?;
    if fragment.is_empty()
        || fragment.len() > MAX_STAGE_BYTES
        || nonce.len() != 32
        || root.is_empty()
        || root.len() > MAX_ROOT_BYTES
        || extra_len > MAX_STAGE_STATE_BYTES
    {
        return Err(invalid());
    }
    Ok(())
}

fn fixed<const N: usize>(bytes: Vec<u8>) -> SafeNativeResult<[u8; N]> {
    bytes.try_into().map_err(|_| invalid())
}

fn utf8(bytes: Vec<u8>, max_len: usize) -> SafeNativeResult<String> {
    if bytes.is_empty() || bytes.len() > max_len {
        return Err(invalid());
    }
    String::from_utf8(bytes).map_err(|_| invalid())
}

fn lower_hex(s: &str, bytes: usize) -> SafeNativeResult<Vec<u8>> {
    if s.len() != bytes * 2
        || !s
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(invalid());
    }
    hex::decode(s).map_err(|_| invalid())
}

fn lower_hex_bounded(s: &str, max_bytes: usize) -> SafeNativeResult<Vec<u8>> {
    if s.is_empty()
        || !s.len().is_multiple_of(2)
        || s.len() > max_bytes * 2
        || !s
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(invalid());
    }
    hex::decode(s).map_err(|_| invalid())
}

fn struct_fields(s: Struct, expected: usize) -> SafeNativeResult<std::vec::IntoIter<Value>> {
    let values: Vec<_> = s
        .unpack()
        .map_err(SafeNativeError::InvariantViolation)?
        .collect();
    if values.len() != expected {
        return Err(invalid());
    }
    Ok(values.into_iter())
}

fn next_bytes(fields: &mut std::vec::IntoIter<Value>) -> SafeNativeResult<Vec<u8>> {
    fields
        .next()
        .ok_or_else(invalid)?
        .value_as::<Vec<u8>>()
        .map_err(SafeNativeError::InvariantViolation)
}

fn next_u64(fields: &mut std::vec::IntoIter<Value>) -> SafeNativeResult<u64> {
    fields
        .next()
        .ok_or_else(invalid)?
        .value_as::<u64>()
        .map_err(SafeNativeError::InvariantViolation)
}

/// Field order matches private `0x1::tinfoil_v3::QuoteStage`.
fn quote_to_move(stage: QuoteStage) -> SafeNativeResult<Value> {
    let q = stage.quote;
    let eval_number = u64::try_from(q.tcb_evaluation_data_number).map_err(|_| invalid())?;
    if q.tcb_status.len() > 64 {
        return Err(invalid());
    }
    Ok(Value::struct_(Struct::pack(vec![
        Value::vector_u8(stage.report_data.to_vec()),
        Value::vector_u8(stage.tls_spki_sha256.to_vec()),
        Value::vector_u8(stage.hpke_key.to_vec()),
        Value::vector_u8(stage.quote_sha256.to_vec()),
        Value::vector_u8(lower_hex(&q.mrtd, 48)?),
        Value::vector_u8(lower_hex(&q.rtmrs[0], 48)?),
        Value::vector_u8(lower_hex(&q.rtmrs[1], 48)?),
        Value::vector_u8(lower_hex(&q.rtmrs[2], 48)?),
        Value::vector_u8(lower_hex(&q.rtmrs[3], 48)?),
        Value::vector_u8(q.tcb_status.into_bytes()),
        Value::vector_u8(lower_hex_bounded(&q.ppid, 32)?),
        Value::vector_u8(lower_hex(&q.qe_vendor_id, 16)?),
        Value::vector_u8(lower_hex(&q.tee_tcb_svn, 16)?),
        Value::vector_u8(lower_hex(&q.mr_seam, 48)?),
        Value::vector_u8(lower_hex(&q.td_attributes, 8)?),
        Value::vector_u8(lower_hex(&q.xfam, 8)?),
        Value::u64(eval_number),
    ])))
}

fn quote_from_move(s: Struct) -> SafeNativeResult<QuoteStage> {
    let mut f = struct_fields(s, 17)?;
    let report_data = fixed::<64>(next_bytes(&mut f)?)?;
    if report_data[32..].iter().any(|b| *b != 0) {
        return Err(invalid());
    }
    let tls_spki_sha256 = fixed::<32>(next_bytes(&mut f)?)?;
    let hpke_key = fixed::<32>(next_bytes(&mut f)?)?;
    let quote_sha256 = fixed::<32>(next_bytes(&mut f)?)?;
    let mrtd = hex::encode(fixed::<48>(next_bytes(&mut f)?)?);
    let rtmrs = [
        hex::encode(fixed::<48>(next_bytes(&mut f)?)?),
        hex::encode(fixed::<48>(next_bytes(&mut f)?)?),
        hex::encode(fixed::<48>(next_bytes(&mut f)?)?),
        hex::encode(fixed::<48>(next_bytes(&mut f)?)?),
    ];
    let tcb_status = utf8(next_bytes(&mut f)?, 64)?;
    let ppid = hex::encode(next_bytes(&mut f)?);
    if ppid.is_empty() || ppid.len() > 64 {
        return Err(invalid());
    }
    let qe_vendor_id = hex::encode(fixed::<16>(next_bytes(&mut f)?)?);
    let tee_tcb_svn = hex::encode(fixed::<16>(next_bytes(&mut f)?)?);
    let mr_seam = hex::encode(fixed::<48>(next_bytes(&mut f)?)?);
    let td_attributes = hex::encode(fixed::<8>(next_bytes(&mut f)?)?);
    let xfam = hex::encode(fixed::<8>(next_bytes(&mut f)?)?);
    let tcb_evaluation_data_number = i64::try_from(next_u64(&mut f)?).map_err(|_| invalid())?;
    Ok(QuoteStage {
        report_data,
        tls_spki_sha256,
        hpke_key,
        quote_sha256,
        quote: AuthenticatedQuote {
            mrtd,
            rtmrs,
            tcb_status,
            advisory_ids: vec![],
            qe_vendor_id,
            tee_tcb_svn,
            mr_seam,
            td_attributes,
            xfam,
            tcb_evaluation_data_number,
            ppid,
        },
    })
}

fn artifact_to_values(a: AuthenticatedArtifact) -> SafeNativeResult<Vec<Value>> {
    if a.repo.is_empty()
        || a.repo.len() > MAX_REPO_BYTES
        || a.tag.is_empty()
        || a.tag.len() > 256
        || a.commit.len() != 40
        || a.subject_name.is_empty()
        || a.subject_name.len() > 512
    {
        return Err(invalid());
    }
    Ok(vec![
        Value::vector_u8(a.repo.into_bytes()),
        Value::vector_u8(a.tag.into_bytes()),
        Value::vector_u8(a.commit.into_bytes()),
        Value::vector_u8(a.subject_name.into_bytes()),
        Value::vector_u8(lower_hex(&a.digest, 32)?),
    ])
}

fn artifact_from_fields(
    fields: &mut std::vec::IntoIter<Value>,
) -> SafeNativeResult<AuthenticatedArtifact> {
    let repo = utf8(next_bytes(fields)?, MAX_REPO_BYTES)?;
    let tag = utf8(next_bytes(fields)?, 256)?;
    let commit = utf8(next_bytes(fields)?, 40)?;
    if commit.len() != 40 {
        return Err(invalid());
    }
    let subject_name = utf8(next_bytes(fields)?, 512)?;
    let digest = hex::encode(fixed::<32>(next_bytes(fields)?)?);
    Ok(AuthenticatedArtifact {
        repo,
        tag,
        commit,
        subject_name,
        digest,
    })
}

/// Field order matches private `0x1::tinfoil_v3::CodeStage`.
fn code_to_move(stage: CodeStage) -> SafeNativeResult<Value> {
    let mut fields = artifact_to_values(stage.artifact)?;
    fields.extend([
        Value::u64(stage.shape.cpus.into()),
        Value::u64(stage.shape.memory_mb),
        Value::u64(stage.shape.gpus.into()),
        Value::u64(stage.shape.disks.into()),
    ]);
    Ok(Value::struct_(Struct::pack(fields)))
}

fn code_from_move(s: Struct) -> SafeNativeResult<CodeStage> {
    let mut f = struct_fields(s, 9)?;
    let artifact = artifact_from_fields(&mut f)?;
    let shape = Shape {
        cpus: u32::try_from(next_u64(&mut f)?).map_err(|_| invalid())?,
        memory_mb: next_u64(&mut f)?,
        gpus: u32::try_from(next_u64(&mut f)?).map_err(|_| invalid())?,
        disks: u32::try_from(next_u64(&mut f)?).map_err(|_| invalid())?,
    };
    Ok(CodeStage { artifact, shape })
}

/// Field order matches private `0x1::tinfoil_v3::PlatformStage`.
fn platform_to_move(stage: PlatformStage) -> SafeNativeResult<Value> {
    if stage.measurement_id.is_empty() || stage.measurement_id.len() > 256 {
        return Err(invalid());
    }
    let mut fields = artifact_to_values(stage.artifact)?;
    fields.push(Value::vector_u8(stage.measurement_id.into_bytes()));
    Ok(Value::struct_(Struct::pack(fields)))
}

fn platform_from_move(s: Struct) -> SafeNativeResult<PlatformStage> {
    let mut f = struct_fields(s, 6)?;
    let artifact = artifact_from_fields(&mut f)?;
    let measurement_id = utf8(next_bytes(&mut f)?, 256)?;
    Ok(PlatformStage {
        artifact,
        measurement_id,
    })
}

fn stage_repo(bytes: Vec<u8>) -> SafeNativeResult<String> {
    utf8(bytes, MAX_REPO_BYTES)
}

fn native_verify_tinfoil_v3_quote(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 4);
    require_gas_feature_version(context)?;
    let intel_root = safely_pop_arg!(args, Vec<u8>);
    let proof_time = safely_pop_arg!(args, u64);
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let fragment = safely_pop_arg!(args, Vec<u8>);
    charge_stage(context, &fragment, &nonce, &intel_root, 0)?;
    let stage = verified(|| {
        aptos_tinfoil_v3::verify_quote_stage(&fragment, &nonce, proof_time, &intel_root)
    })?;
    Ok(smallvec![quote_to_move(stage)?])
}

fn native_verify_tinfoil_v3_code(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 5);
    require_gas_feature_version(context)?;
    let quote = quote_from_move(safely_pop_arg!(args, Struct))?;
    let sigstore_root = safely_pop_arg!(args, Vec<u8>);
    let repo = stage_repo(safely_pop_arg!(args, Vec<u8>))?;
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let fragment = safely_pop_arg!(args, Vec<u8>);
    charge_stage(
        context,
        &fragment,
        &nonce,
        &sigstore_root,
        repo.len() + 1024,
    )?;
    let stage = verified(|| {
        aptos_tinfoil_v3::verify_code_stage(&fragment, &nonce, &repo, &sigstore_root, &quote)
    })?;
    Ok(smallvec![code_to_move(stage)?])
}

fn native_verify_tinfoil_v3_code_freshness(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 6);
    require_gas_feature_version(context)?;
    let code = code_from_move(safely_pop_arg!(args, Struct))?;
    let quote = quote_from_move(safely_pop_arg!(args, Struct))?;
    let sigstore_root = safely_pop_arg!(args, Vec<u8>);
    let proof_time = safely_pop_arg!(args, u64);
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let fragment = safely_pop_arg!(args, Vec<u8>);
    charge_stage(context, &fragment, &nonce, &sigstore_root, 2048)?;
    verified(|| {
        aptos_tinfoil_v3::verify_code_freshness_stage(
            &fragment,
            &nonce,
            proof_time,
            &sigstore_root,
            &quote,
            &code,
        )
    })?;
    Ok(smallvec![])
}

fn native_verify_tinfoil_v3_platform(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 6);
    require_gas_feature_version(context)?;
    let code = code_from_move(safely_pop_arg!(args, Struct))?;
    let quote = quote_from_move(safely_pop_arg!(args, Struct))?;
    let sigstore_root = safely_pop_arg!(args, Vec<u8>);
    let proof_time = safely_pop_arg!(args, u64);
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let fragment = safely_pop_arg!(args, Vec<u8>);
    charge_stage(context, &fragment, &nonce, &sigstore_root, 2048)?;
    let stage = verified(|| {
        aptos_tinfoil_v3::verify_platform_stage(
            &fragment,
            &nonce,
            proof_time,
            &sigstore_root,
            &quote,
            &code,
        )
    })?;
    Ok(smallvec![platform_to_move(stage)?])
}

fn native_verify_tinfoil_v3_platform_freshness(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 6);
    require_gas_feature_version(context)?;
    let platform = platform_from_move(safely_pop_arg!(args, Struct))?;
    let quote = quote_from_move(safely_pop_arg!(args, Struct))?;
    let sigstore_root = safely_pop_arg!(args, Vec<u8>);
    let proof_time = safely_pop_arg!(args, u64);
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let fragment = safely_pop_arg!(args, Vec<u8>);
    charge_stage(context, &fragment, &nonce, &sigstore_root, 2048)?;
    verified(|| {
        aptos_tinfoil_v3::verify_platform_freshness_stage(
            &fragment,
            &nonce,
            proof_time,
            &sigstore_root,
            &quote,
            &platform,
        )
    })?;
    Ok(smallvec![])
}

/// The repository in the envelope is a *claim*, not a trust root. Move checks
/// its governance allowlist before passing it to the code verifier, which then
/// authenticates the Sigstore certificate identity against that exact name.
fn claimed_repo_and_nonce(document: &[u8]) -> SafeNativeResult<(String, [u8; 32])> {
    let parsed = Document::parse(document).map_err(|_| invalid())?;
    let nonce = fixed::<32>(lower_hex(&parsed.raw.challenge.nonce, 32)?)?;
    let code = parsed
        .reference_values_collateral(COLLATERAL_SIGSTORE_CODE_V1_FORMAT)
        .ok_or_else(invalid)?;
    let repo = code
        .data
        .get("repo")
        .and_then(|value| value.as_str())
        .ok_or_else(invalid)?;
    Ok((stage_repo(repo.as_bytes().to_vec())?, nonce))
}

/// First advancement of an object-style verification. The full envelope is
/// kept in the object; no caller-selected fragment is allowed to substitute
/// a different collateral entry later in the workflow.
fn native_try_verify_tinfoil_v3_quote(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 3);
    require_gas_feature_version(context)?;
    let intel_root = safely_pop_arg!(args, Vec<u8>);
    let proof_time = safely_pop_arg!(args, u64);
    let attestation = safely_pop_arg!(args, Vec<u8>);
    charge_document(context, &attestation, &intel_root, 0)?;
    let outcome = attempted(|| {
        check_attestation_args(&attestation, &intel_root, 0)?;
        let document = tinfoil_document(&attestation)?;
        let (repo, nonce) = claimed_repo_and_nonce(&document)?;
        let quote = verified(|| {
            aptos_tinfoil_v3::verify_quote_stage(&document, &nonce, proof_time, &intel_root)
        })?;
        Ok(Value::struct_(Struct::pack(vec![
            Value::vector_u8(nonce.to_vec()),
            Value::vector_u8(repo.into_bytes()),
            quote_to_move(quote)?,
        ])))
    })?;
    Ok(smallvec![option_to_move(context, outcome)?])
}

fn native_try_verify_tinfoil_v3_code(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 5);
    require_gas_feature_version(context)?;
    let quote = quote_from_move(safely_pop_arg!(args, Struct))?;
    let sigstore_root = safely_pop_arg!(args, Vec<u8>);
    let repo = stage_repo(safely_pop_arg!(args, Vec<u8>))?;
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let attestation = safely_pop_arg!(args, Vec<u8>);
    charge_document(context, &attestation, &sigstore_root, repo.len() + 1024)?;
    let stage = attempted(|| {
        check_attestation_args(&attestation, &sigstore_root, repo.len() + 1024)?;
        let document = tinfoil_document(&attestation)?;
        let code = verified(|| {
            aptos_tinfoil_v3::verify_code_stage(&document, &nonce, &repo, &sigstore_root, &quote)
        })?;
        code_to_move(code)
    })?;
    Ok(smallvec![option_to_move(context, stage)?])
}

fn native_try_verify_tinfoil_v3_code_freshness(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 6);
    require_gas_feature_version(context)?;
    let code = code_from_move(safely_pop_arg!(args, Struct))?;
    let quote = quote_from_move(safely_pop_arg!(args, Struct))?;
    let sigstore_root = safely_pop_arg!(args, Vec<u8>);
    let proof_time = safely_pop_arg!(args, u64);
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let attestation = safely_pop_arg!(args, Vec<u8>);
    charge_document(context, &attestation, &sigstore_root, 2048)?;
    let passed = attempted(|| {
        check_attestation_args(&attestation, &sigstore_root, 2048)?;
        let document = tinfoil_document(&attestation)?;
        verified(|| {
            aptos_tinfoil_v3::verify_code_freshness_stage(
                &document,
                &nonce,
                proof_time,
                &sigstore_root,
                &quote,
                &code,
            )
        })
    })?
    .is_some();
    Ok(smallvec![Value::bool(passed)])
}

fn native_try_verify_tinfoil_v3_platform(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 6);
    require_gas_feature_version(context)?;
    let code = code_from_move(safely_pop_arg!(args, Struct))?;
    let quote = quote_from_move(safely_pop_arg!(args, Struct))?;
    let sigstore_root = safely_pop_arg!(args, Vec<u8>);
    let proof_time = safely_pop_arg!(args, u64);
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let attestation = safely_pop_arg!(args, Vec<u8>);
    charge_document(context, &attestation, &sigstore_root, 2048)?;
    let stage = attempted(|| {
        check_attestation_args(&attestation, &sigstore_root, 2048)?;
        let document = tinfoil_document(&attestation)?;
        let platform = verified(|| {
            aptos_tinfoil_v3::verify_platform_stage(
                &document,
                &nonce,
                proof_time,
                &sigstore_root,
                &quote,
                &code,
            )
        })?;
        platform_to_move(platform)
    })?;
    Ok(smallvec![option_to_move(context, stage)?])
}

fn native_try_verify_tinfoil_v3_platform_freshness(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 6);
    require_gas_feature_version(context)?;
    let platform = platform_from_move(safely_pop_arg!(args, Struct))?;
    let quote = quote_from_move(safely_pop_arg!(args, Struct))?;
    let sigstore_root = safely_pop_arg!(args, Vec<u8>);
    let proof_time = safely_pop_arg!(args, u64);
    let nonce = safely_pop_arg!(args, Vec<u8>);
    let attestation = safely_pop_arg!(args, Vec<u8>);
    charge_document(context, &attestation, &sigstore_root, 2048)?;
    let passed = attempted(|| {
        check_attestation_args(&attestation, &sigstore_root, 2048)?;
        let document = tinfoil_document(&attestation)?;
        verified(|| {
            aptos_tinfoil_v3::verify_platform_freshness_stage(
                &document,
                &nonce,
                proof_time,
                &sigstore_root,
                &quote,
                &platform,
            )
        })
    })?
    .is_some();
    Ok(smallvec![Value::bool(passed)])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    builder.make_named_natives([
        (
            "verify_tinfoil_v3_quote",
            native_verify_tinfoil_v3_quote as RawSafeNative,
        ),
        (
            "verify_tinfoil_v3_code",
            native_verify_tinfoil_v3_code as RawSafeNative,
        ),
        (
            "verify_tinfoil_v3_code_freshness",
            native_verify_tinfoil_v3_code_freshness as RawSafeNative,
        ),
        (
            "verify_tinfoil_v3_platform",
            native_verify_tinfoil_v3_platform as RawSafeNative,
        ),
        (
            "verify_tinfoil_v3_platform_freshness",
            native_verify_tinfoil_v3_platform_freshness as RawSafeNative,
        ),
        (
            "try_verify_tinfoil_v3_quote",
            native_try_verify_tinfoil_v3_quote as RawSafeNative,
        ),
        (
            "try_verify_tinfoil_v3_code",
            native_try_verify_tinfoil_v3_code as RawSafeNative,
        ),
        (
            "try_verify_tinfoil_v3_code_freshness",
            native_try_verify_tinfoil_v3_code_freshness as RawSafeNative,
        ),
        (
            "try_verify_tinfoil_v3_platform",
            native_try_verify_tinfoil_v3_platform as RawSafeNative,
        ),
        (
            "try_verify_tinfoil_v3_platform_freshness",
            native_try_verify_tinfoil_v3_platform_freshness as RawSafeNative,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_ok<T>(result: SafeNativeResult<T>) -> T {
        match result {
            Ok(value) => value,
            Err(_) => panic!("native stage conversion failed"),
        }
    }

    #[test]
    fn evidence_rejection_is_a_result_for_the_object_workflow() {
        assert!(native_ok(attempted::<()>(|| Err(invalid()))).is_none());
        assert_eq!(native_ok(attempted(|| Ok(7u64))), Some(7));
    }

    #[test]
    fn package_parser_requires_magic_version_and_exact_bcs() {
        assert_eq!(
            native_ok(tinfoil_document(b"{}")),
            Cow::Borrowed(&b"{}"[..])
        );
        assert_eq!(native_ok(tinfoil_document(b" \n{}")).as_ref(), b" \n{}");

        let package = bcs::to_bytes(&(1u8, b"{}".to_vec(), b"eat".to_vec())).unwrap();
        let mut wrapped = ATTESTATION_PACKAGE_MAGIC.to_vec();
        wrapped.extend_from_slice(&package);
        assert_eq!(native_ok(tinfoil_document(&wrapped)).as_ref(), b"{}");

        let mut bad_version = ATTESTATION_PACKAGE_MAGIC.to_vec();
        bad_version
            .extend_from_slice(&bcs::to_bytes(&(2u8, b"{}".to_vec(), Vec::<u8>::new())).unwrap());
        assert!(tinfoil_document(&bad_version).is_err());

        wrapped.push(0);
        assert!(tinfoil_document(&wrapped).is_err());
        assert!(tinfoil_document(b"bad input").is_err());
    }

    #[test]
    fn staged_claims_round_trip_through_move_values() {
        let quote = QuoteStage {
            report_data: [0; 64],
            quote_sha256: [9; 32],
            tls_spki_sha256: [1; 32],
            hpke_key: [2; 32],
            quote: AuthenticatedQuote {
                mrtd: "03".repeat(48),
                rtmrs: [
                    "04".repeat(48),
                    "05".repeat(48),
                    "06".repeat(48),
                    "00".repeat(48),
                ],
                tcb_status: "UpToDate".into(),
                advisory_ids: vec![],
                qe_vendor_id: "07".repeat(16),
                tee_tcb_svn: "08".repeat(16),
                mr_seam: "09".repeat(48),
                td_attributes: "0a".repeat(8),
                xfam: "0b".repeat(8),
                tcb_evaluation_data_number: 42,
                ppid: "0c".repeat(16),
            },
        };
        let round_trip = native_ok(quote_from_move(
            native_ok(quote_to_move(quote.clone())).value_as().unwrap(),
        ));
        assert_eq!(round_trip.report_data, quote.report_data);
        assert_eq!(round_trip.quote_sha256, quote.quote_sha256);
        assert_eq!(round_trip.tls_spki_sha256, quote.tls_spki_sha256);
        assert_eq!(round_trip.hpke_key, quote.hpke_key);
        assert_eq!(round_trip.quote.mrtd, quote.quote.mrtd);
        assert_eq!(round_trip.quote.rtmrs, quote.quote.rtmrs);
        assert_eq!(round_trip.quote.ppid, quote.quote.ppid);
        assert_eq!(round_trip.quote.tcb_evaluation_data_number, 42);

        let artifact = AuthenticatedArtifact {
            repo: "tinfoilsh/example".into(),
            tag: "v1".into(),
            commit: "a".repeat(40),
            subject_name: "release.bin".into(),
            digest: "b".repeat(64),
        };
        let code = CodeStage {
            artifact: artifact.clone(),
            shape: Shape {
                cpus: 4,
                memory_mb: 8192,
                gpus: 0,
                disks: 1,
            },
        };
        let code = native_ok(code_from_move(
            native_ok(code_to_move(code)).value_as().unwrap(),
        ));
        assert_eq!(code.artifact, artifact);
        assert_eq!(code.shape.cpus, 4);
        assert_eq!(code.shape.memory_mb, 8192);

        let gpu_code = CodeStage {
            artifact: artifact.clone(),
            shape: Shape {
                cpus: 32,
                memory_mb: 512 * 1024,
                gpus: 8,
                disks: 4,
            },
        };
        let gpu_code = native_ok(code_from_move(
            native_ok(code_to_move(gpu_code)).value_as().unwrap(),
        ));
        assert_eq!(gpu_code.shape.gpus, 8);

        let platform = PlatformStage {
            artifact: artifact.clone(),
            measurement_id: "tdx-platform@sha256".into(),
        };
        let platform = native_ok(platform_from_move(
            native_ok(platform_to_move(platform)).value_as().unwrap(),
        ));
        assert_eq!(platform.artifact, artifact);
        assert_eq!(platform.measurement_id, "tdx-platform@sha256");
    }
}
