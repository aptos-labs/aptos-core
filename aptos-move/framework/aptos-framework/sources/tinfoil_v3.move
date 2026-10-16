/// An object-based, multi-transaction verifier for Tinfoil v3 envelopes
/// carrying legacy FormatV2 (`codeReferences`) collateral. Each object stores
/// one complete submitted report; `advance_verification` performs one bounded
/// proof stage and records CONTINUE, FAIL or PASS. The older challenge and
/// fragment entry points remain available for compatibility.
/// The newer FormatV3 (`configReferences`) protocol needs separate verifier
/// stages and is rejected by the native parser until implemented. Intel and
/// Sigstore roots come only from quorum-observed chain state, and repositories
/// are approved by Aptos framework governance.
/// A business contract must bind the authenticated TLS/HPKE key and challenge
/// to its worker identity. Any account can submit a copy of another report.
module aptos_framework::tinfoil_v3 {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::option;
    use std::option::Option;
    use aptos_std::table_with_length;
    use aptos_std::table_with_length::TableWithLength;
    use aptos_framework::chain_id;
    use aptos_framework::event::emit;
    use aptos_framework::object;
    use aptos_framework::randomness;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::trust_observation;

    // Native aborts with this code when supported FormatV2 collateral fails.
    const EINVALID_TINFOIL_PROOF: u64 = 1;
    const EINVALID_REPOSITORY: u64 = 2;
    const EUNAPPROVED_REPOSITORY: u64 = 3;
    const ETOO_MANY_REPOSITORIES: u64 = 4;
    const EREQUEST_PENDING: u64 = 5;
    const ENO_PENDING_REQUEST: u64 = 6;
    const EEXPIRED_REQUEST: u64 = 7;
    const EINVALID_REQUEST: u64 = 8;
    const ETRUST_ROOT_UNAVAILABLE: u64 = 9;
    const EINVALID_VERIFIED_CLAIMS: u64 = 10;
    const ENOT_READY_FOR_CLEANUP: u64 = 11;
    const EWRONG_STAGE: u64 = 12;
    const ETRUST_ROOT_CHANGED: u64 = 13;
    const EUNKNOWN_VERIFICATION: u64 = 14;
    const EVERIFICATION_NOT_PASSED: u64 = 15;

    const INTEL_ROOT_TARGET: vector<u8> = b"intel_sgx_root_ca";
    const SIGSTORE_ROOT_TARGET: vector<u8> = b"sigstore_trusted_root";
    const CHALLENGE_DOMAIN: vector<u8> = b"aptos.tinfoil_v3.v1";
    const MAX_REPOSITORIES: u64 = 16;
    const MAX_REPOSITORY_BYTES: u64 = 200;
    const TLS_SPKI_SHA256_BYTES: u64 = 32;
    const MAX_CHUNK_BYTES: u64 = 48 * 1024;
    const MAX_DOCUMENT_BYTES: u64 = 4 * 1024 * 1024;
    /// Native verification charges for the complete object on each advance.
    /// Larger inputs cannot fit the normal transaction execution gas budget.
    const MAX_VERIFICATION_BYTES: u64 = 256 * 1024;
    const MAX_INLINE_VERIFICATION_BYTES: u64 = 48 * 1024;
    const MAX_VERIFICATION_CHUNKS: u64 = 6;
    const MAX_STAGE_BYTES: u64 = 1024 * 1024;
    const MAX_CHUNKS: u64 = 96;
    const MAX_CLEANUP_CHUNKS_PER_CALL: u64 = 8;
    const MAX_REQUEST_LIFETIME_SECS: u64 = 3600;
    const MAX_RESULT_AGE_SECS: u64 = 3600;
    const STAGE_QUOTE: u8 = 0;
    const STAGE_CODE: u8 = 1;
    const STAGE_CODE_FRESHNESS: u8 = 2;
    const STAGE_PLATFORM: u8 = 3;
    const STAGE_PLATFORM_FRESHNESS: u8 = 4;
    const STAGE_COMPLETE: u8 = 5;
    const STAGE_UPLOAD: u8 = 6;
    const VERIFICATION_CONTINUE: u8 = 0;
    const VERIFICATION_FAIL: u8 = 1;
    const VERIFICATION_PASS: u8 = 2;

    /// Result codes carried by `VerificationAdvanced`. These are diagnostic;
    /// consumers should make authorization decisions from the status and
    /// authenticated fields, not from this code.
    const REASON_NONE: u8 = 0;
    const REASON_INVALID_PROOF: u8 = 1;
    const REASON_REPOSITORY_REVOKED: u8 = 2;
    const REASON_TRUST_ROOT_CHANGED: u8 = 3;
    const REASON_UNSUPPORTED_GPU_BINDING: u8 = 4;
    const REASON_EXPIRED: u8 = 5;

    /// The first stage returns both the authenticated CPU quote and the
    /// untrusted routing hints extracted from the same full document. A later
    /// Sigstore stage must authenticate `claimed_repo` before it is a claim.
    struct QuoteOutcome has drop {
        nonce: vector<u8>,
        claimed_repo: vector<u8>,
        quote: QuoteStage,
    }

    /// A sticky, untransferable object created by `new_verification`. The
    /// submitted bytes and creator never change. Only the verifier's own stage
    /// state advances. No object capability is retained by the creator.
    struct Verification has key {
        creator: address,
        /// A small, one-transaction report lives inline. Larger reports live
        /// in distinct table entries, each below the per-write state limit.
        attestation: vector<u8>,
        chunks: TableWithLength<u64, vector<u8>>,
        attestation_sha3_256: vector<u8>,
        expected_bytes: u64,
        uploaded_chunks: u64,
        uploaded_bytes: u64,
        status: u8,
        stage: u8,
        proof_time_secs: u64,
        nonce: vector<u8>,
        claimed_repo: vector<u8>,
        intel_root_sha3_256: vector<u8>,
        sigstore_root_sha3_256: vector<u8>,
        quote: Option<QuoteStage>,
        code: Option<CodeStage>,
        platform: Option<PlatformStage>,
        result: Option<VerifiedTinfoilV3Result>,
    }

    #[event]
    struct VerificationCreated has drop, store {
        creator: address,
        verification: address,
        attestation_sha3_256: vector<u8>,
    }

    #[event]
    struct VerificationAdvanced has drop, store {
        verification: address,
        status: u8,
        next_stage: u8,
        reason: u8,
    }

    struct ChallengeMaterial has drop {
        domain: vector<u8>,
        chain_id: u8,
        owner: address,
        request_id: u64,
        entropy: vector<u8>,
    }

    /// The framework allowlist is empty until governance explicitly adds a
    /// repository. A revocation takes effect even for an in-flight request.
    struct ApprovedRepositories has key {
        entries: vector<vector<u8>>,
    }

    struct PendingRequest has store {
        request_id: u64,
        nonce: vector<u8>,
        trusted_repo: vector<u8>,
        expected_tls_spki_sha256: vector<u8>,
        expires_at_secs: u64,
        chunks: TableWithLength<u64, vector<u8>>,
        chunk_count: u64,
        total_bytes: u64,
        stage: u8,
        stage_start_chunk: u64,
        stage_bytes: u64,
        proof_time_secs: u64,
        intel_root_sha3_256: vector<u8>,
        sigstore_root_sha3_256: vector<u8>,
        quote: Option<QuoteStage>,
        code: Option<CodeStage>,
        platform: Option<PlatformStage>,
        /// Verification or cancellation closes the request before cleanup.
        closed: bool,
        cleanup_cursor: u64,
    }

    /// Authenticated TDX quote output. The native checks REPORTDATA, DCAP,
    /// collateral signatures and the Intel root before constructing this.
    struct QuoteStage has copy, drop, store {
        report_data: vector<u8>,
        tls_spki_sha256: vector<u8>,
        hpke_key: vector<u8>,
        quote_sha256: vector<u8>,
        mrtd: vector<u8>,
        rtmr0: vector<u8>,
        rtmr1: vector<u8>,
        rtmr2: vector<u8>,
        rtmr3: vector<u8>,
        tcb_status: vector<u8>,
        ppid: vector<u8>,
        qe_vendor_id: vector<u8>,
        tee_tcb_svn: vector<u8>,
        mr_seam: vector<u8>,
        td_attributes: vector<u8>,
        xfam: vector<u8>,
        tcb_evaluation_data_number: u64,
    }

    /// Authenticated code provenance after Sigstore verification.
    struct CodeStage has copy, drop, store {
        repo: vector<u8>,
        tag: vector<u8>,
        commit: vector<u8>,
        subject_name: vector<u8>,
        digest: vector<u8>,
        shape_cpus: u64,
        shape_memory_mb: u64,
        shape_gpus: u64,
        shape_disks: u64,
    }

    /// Authenticated platform endorsement after Sigstore verification.
    struct PlatformStage has copy, drop, store {
        repo: vector<u8>,
        tag: vector<u8>,
        commit: vector<u8>,
        subject_name: vector<u8>,
        digest: vector<u8>,
        measurement_id: vector<u8>,
    }

    /// Fields returned only after Rust validates all supported FormatV2
    /// collateral: TDX quote, Intel evidence, Sigstore provenance, freshness,
    /// and the report-data join. `shape_gpus` is the signed VM shape matched
    /// to CPU-bound device topology; the verifier relies on the measured guest
    /// boot path for NVIDIA appraisal rather than checking NVIDIA signatures.
    struct VerifiedTinfoilV3Claims has copy, drop, store {
        tls_spki_sha256: vector<u8>,
        hpke_key: vector<u8>,
        release_digest: vector<u8>,
        trusted_repo: vector<u8>,
        nonce: vector<u8>,
        mrtd: vector<u8>,
        rtmr0: vector<u8>,
        rtmr1: vector<u8>,
        rtmr2: vector<u8>,
        rtmr3: vector<u8>,
        tcb_status: vector<u8>,
        release_tag: vector<u8>,
        release_commit: vector<u8>,
        platform_measurement_id: vector<u8>,
        shape_gpus: u64,
    }

    struct VerifiedTinfoilV3Result has copy, drop, store {
        request_id: u64,
        verified_at_secs: u64,
        intel_root_sha3_256: vector<u8>,
        sigstore_root_sha3_256: vector<u8>,
        claims: VerifiedTinfoilV3Claims,
    }

    struct State has key {
        next_request_id: u64,
        pending: Option<PendingRequest>,
        last_verified: Option<VerifiedTinfoilV3Result>,
    }

    #[event]
    struct ChallengeIssued has drop, store {
        owner: address,
        request_id: u64,
        nonce: vector<u8>,
        trusted_repo: vector<u8>,
        expires_at_secs: u64,
    }

    #[event]
    struct TinfoilV3FormatV2Verified has drop, store {
        owner: address,
        result: VerifiedTinfoilV3Result,
    }

    #[event]
    struct StageVerified has drop, store {
        owner: address,
        request_id: u64,
        completed_stage: u8,
    }

    /// Submit one complete Atlas attestation up to 48 KiB. The event exposes the address
    /// of the untransferable object containing these exact bytes and its
    /// verification state. Larger reports use the chunked constructor below.
    /// Accepted package bytes are either bare Tinfoil JSON or
    /// `b"APTOS-TINFOIL-V3\0" || bcs(PackageV1 { version: 1,
    /// tinfoil_document: vector<u8>, nras_detached_eat: vector<u8> })`.
    /// Callers do not choose the object's address.
    public entry fun new_verification(creator: &signer, attestation: vector<u8>) {
        assert!(!attestation.is_empty()
            && attestation.length() <= MAX_INLINE_VERIFICATION_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        let bytes = attestation.length();
        let digest = hash::sha3_256(copy attestation);
        create_verification(creator, attestation, bytes, digest, STAGE_QUOTE);
    }

    /// Start the same object flow when a report exceeds the transaction size.
    /// The declared byte length and digest are fixed at creation. Only the
    /// creator may append chunks; no verification can start until sealing.
    public entry fun new_verification_chunked(
        creator: &signer, expected_bytes: u64, expected_sha3_256: vector<u8>,
    ) {
        assert!(expected_bytes > 0 && expected_bytes <= MAX_VERIFICATION_BYTES
            && expected_sha3_256.length() == 32,
            error::invalid_argument(EINVALID_REQUEST));
        create_verification(
            creator, vector[], expected_bytes, expected_sha3_256, STAGE_UPLOAD,
        );
    }

    fun create_verification(
        creator: &signer,
        attestation: vector<u8>,
        expected_bytes: u64,
        attestation_sha3_256: vector<u8>,
        stage: u8,
    ): address {
        let creator_address = creator.address_of();
        let constructor = object::create_sticky_object(creator_address);
        let verification = constructor.address_from_constructor_ref();
        constructor.set_untransferable();
        let object_signer = constructor.generate_signer();
        move_to(&object_signer, Verification {
            creator: creator_address,
            attestation,
            chunks: table_with_length::new(),
            attestation_sha3_256: copy attestation_sha3_256,
            expected_bytes,
            uploaded_chunks: 0,
            uploaded_bytes: if (stage == STAGE_QUOTE) { expected_bytes } else { 0 },
            status: VERIFICATION_CONTINUE,
            stage,
            proof_time_secs: 0,
            nonce: vector[],
            claimed_repo: vector[],
            intel_root_sha3_256: vector[],
            sigstore_root_sha3_256: vector[],
            quote: option::none(),
            code: option::none(),
            platform: option::none(),
            result: option::none(),
        });
        emit(VerificationCreated { creator: creator_address, verification, attestation_sha3_256 });
        verification
    }

    /// Append one bounded chunk to an unsealed report. The report becomes
    /// immutable after `seal_verification` verifies its declared digest.
    public entry fun append_verification_chunk(
        creator: &signer, verification: address, chunk: vector<u8>,
    ) {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        assert!(!chunk.is_empty() && chunk.length() <= MAX_CHUNK_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        let state = &mut Verification[verification];
        assert!(state.creator == creator.address_of() && state.stage == STAGE_UPLOAD
            && state.status == VERIFICATION_CONTINUE,
            error::permission_denied(EINVALID_REQUEST));
        assert!(state.uploaded_chunks < MAX_VERIFICATION_CHUNKS
            && state.uploaded_bytes + chunk.length() <= state.expected_bytes,
            error::resource_exhausted(EINVALID_REQUEST));
        state.uploaded_bytes += chunk.length();
        state.chunks.add(state.uploaded_chunks, chunk);
        state.uploaded_chunks += 1;
    }

    /// Seal the uploaded bytes, or durably FAIL if they do not match the
    /// declared digest. The object then uses exactly `advance_verification`.
    public entry fun seal_verification(creator: &signer, verification: address) {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        let state = &Verification[verification];
        assert!(state.creator == creator.address_of() && state.stage == STAGE_UPLOAD
            && state.status == VERIFICATION_CONTINUE,
            error::permission_denied(EINVALID_REQUEST));
        assert!(state.uploaded_bytes == state.expected_bytes,
            error::invalid_state(EINVALID_REQUEST));
        let digest = hash::sha3_256(full_attestation(state));
        if (digest != state.attestation_sha3_256) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        Verification[verification].stage = STAGE_QUOTE;
        emit_progress(verification, VERIFICATION_CONTINUE, REASON_NONE);
    }

    /// Execute exactly one bounded verification stage. Anyone may advance an
    /// object, so progress does not depend on its creator staying online.
    /// `VerificationAdvanced` tells the submitter to continue, or that the
    /// object has reached a durable FAIL/PASS state.
    public entry fun advance_verification(_caller: &signer, verification: address) {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        let status = Verification[verification].status;
        if (status != VERIFICATION_CONTINUE) {
            emit_progress(verification, status, REASON_NONE);
            return
        };
        let stage = Verification[verification].stage;
        if (stage == STAGE_UPLOAD) {
            emit_progress(verification, VERIFICATION_CONTINUE, REASON_NONE);
        } else if (stage == STAGE_QUOTE) {
            advance_quote_verification(verification);
        } else if (stage == STAGE_CODE) {
            advance_code_verification(verification);
        } else if (stage == STAGE_CODE_FRESHNESS) {
            advance_code_freshness_verification(verification);
        } else if (stage == STAGE_PLATFORM) {
            advance_platform_verification(verification);
        } else if (stage == STAGE_PLATFORM_FRESHNESS) {
            advance_platform_freshness_verification(verification);
        } else {
            fail_verification(verification, REASON_INVALID_PROOF);
        };
    }

    fun advance_quote_verification(verification: address) {
        let intel_opt = trust_observation::get_effective_value(INTEL_ROOT_TARGET);
        let sigstore_opt = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET);
        if (intel_opt.is_none() || sigstore_opt.is_none()) {
            emit_progress(verification, VERIFICATION_CONTINUE, REASON_NONE);
            return
        };
        let intel_root = intel_opt.extract();
        let sigstore_root = sigstore_opt.extract();
        if (intel_root.is_empty() || sigstore_root.is_empty()) {
            emit_progress(verification, VERIFICATION_CONTINUE, REASON_NONE);
            return
        };
        let proof_time_secs = timestamp::now_seconds();
        let attestation = full_attestation(&Verification[verification]);
        let outcome_opt = try_verify_tinfoil_v3_quote(
            attestation, proof_time_secs, copy intel_root,
        );
        if (outcome_opt.is_none()) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        let QuoteOutcome { nonce, claimed_repo, quote } = outcome_opt.extract();
        if (nonce.length() != 32 || claimed_repo.is_empty()
            || claimed_repo.length() > MAX_REPOSITORY_BYTES || !valid_quote_claims(&quote)) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        if (!is_repository_approved(copy claimed_repo)) {
            fail_verification(verification, REASON_REPOSITORY_REVOKED);
            return
        };
        let state = &mut Verification[verification];
        state.nonce = nonce;
        state.claimed_repo = claimed_repo;
        state.proof_time_secs = proof_time_secs;
        state.intel_root_sha3_256 = hash::sha3_256(intel_root);
        state.sigstore_root_sha3_256 = hash::sha3_256(sigstore_root);
        state.quote.fill(quote);
        state.stage = STAGE_CODE;
        emit_progress(verification, VERIFICATION_CONTINUE, REASON_NONE);
    }

    fun advance_code_verification(verification: address) {
        if (!object_policy_still_valid(verification)) { return };
        let state = &Verification[verification];
        let sigstore_root = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET).extract();
        let code_opt = try_verify_tinfoil_v3_code(
            full_attestation(state), state.nonce, state.claimed_repo, sigstore_root,
            *state.quote.borrow(),
        );
        if (code_opt.is_none()) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        let code = code_opt.extract();
        if (code.repo != state.claimed_repo || code.digest.length() != 32
            || code.tag.is_empty() || code.commit.length() != 40) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        let state = &mut Verification[verification];
        state.code.fill(code);
        state.stage = STAGE_CODE_FRESHNESS;
        emit_progress(verification, VERIFICATION_CONTINUE, REASON_NONE);
    }

    fun advance_code_freshness_verification(verification: address) {
        if (!object_policy_still_valid(verification)) { return };
        let state = &Verification[verification];
        let sigstore_root = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET).extract();
        let valid = try_verify_tinfoil_v3_code_freshness(
            full_attestation(state), state.nonce, state.proof_time_secs, sigstore_root,
            *state.quote.borrow(), *state.code.borrow(),
        );
        if (!valid) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        Verification[verification].stage = STAGE_PLATFORM;
        emit_progress(verification, VERIFICATION_CONTINUE, REASON_NONE);
    }

    fun advance_platform_verification(verification: address) {
        if (!object_policy_still_valid(verification)) { return };
        let state = &Verification[verification];
        let sigstore_root = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET).extract();
        let platform_opt = try_verify_tinfoil_v3_platform(
            full_attestation(state), state.nonce, state.proof_time_secs, sigstore_root,
            *state.quote.borrow(), *state.code.borrow(),
        );
        if (platform_opt.is_none()) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        let platform = platform_opt.extract();
        if (platform.digest.length() != 32 || platform.measurement_id.is_empty()) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        let state = &mut Verification[verification];
        state.platform.fill(platform);
        state.stage = STAGE_PLATFORM_FRESHNESS;
        emit_progress(verification, VERIFICATION_CONTINUE, REASON_NONE);
    }

    fun advance_platform_freshness_verification(verification: address) {
        if (!object_policy_still_valid(verification)) { return };
        let state = &Verification[verification];
        let sigstore_root = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET).extract();
        let valid = try_verify_tinfoil_v3_platform_freshness(
            full_attestation(state), state.nonce, state.proof_time_secs, sigstore_root,
            *state.quote.borrow(), *state.platform.borrow(),
        );
        if (!valid) {
            fail_verification(verification, REASON_INVALID_PROOF);
            return
        };
        let quote = *state.quote.borrow();
        let code = *state.code.borrow();
        let platform = *state.platform.borrow();
        // An NRAS EAT has no signed digest of the CPU-bound raw GPU evidence.
        // Until a sound binding is specified, GPU-bearing reports fail closed.
        if (code.shape_gpus > 0) {
            fail_verification(verification, REASON_UNSUPPORTED_GPU_BINDING);
            return
        };
        let claims = VerifiedTinfoilV3Claims {
            tls_spki_sha256: quote.tls_spki_sha256,
            hpke_key: quote.hpke_key,
            release_digest: code.digest,
            trusted_repo: code.repo,
            nonce: state.nonce,
            mrtd: quote.mrtd,
            rtmr0: quote.rtmr0,
            rtmr1: quote.rtmr1,
            rtmr2: quote.rtmr2,
            rtmr3: quote.rtmr3,
            tcb_status: quote.tcb_status,
            release_tag: code.tag,
            release_commit: code.commit,
            platform_measurement_id: platform.measurement_id,
            shape_gpus: code.shape_gpus,
        };
        let result = VerifiedTinfoilV3Result {
            request_id: 0,
            verified_at_secs: state.proof_time_secs,
            intel_root_sha3_256: state.intel_root_sha3_256,
            sigstore_root_sha3_256: state.sigstore_root_sha3_256,
            claims,
        };
        let state = &mut Verification[verification];
        state.result.fill(result);
        state.stage = STAGE_COMPLETE;
        state.status = VERIFICATION_PASS;
        emit_progress(verification, VERIFICATION_PASS, REASON_NONE);
    }

    fun object_policy_still_valid(verification: address): bool {
        let proof_time_secs = Verification[verification].proof_time_secs;
        let now = timestamp::now_seconds();
        if (now < proof_time_secs || now - proof_time_secs > MAX_RESULT_AGE_SECS) {
            fail_verification(verification, REASON_EXPIRED);
            return false
        };
        if (!object_roots_match(verification)) {
            fail_verification(verification, REASON_TRUST_ROOT_CHANGED);
            return false
        };
        if (!is_repository_approved(Verification[verification].claimed_repo)) {
            fail_verification(verification, REASON_REPOSITORY_REVOKED);
            return false
        };
        true
    }

    fun object_roots_match(verification: address): bool {
        let state = &Verification[verification];
        let intel_opt = trust_observation::get_effective_value(INTEL_ROOT_TARGET);
        let sigstore_opt = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET);
        if (intel_opt.is_none() || sigstore_opt.is_none()) { return false };
        hash::sha3_256(intel_opt.extract()) == state.intel_root_sha3_256
            && hash::sha3_256(sigstore_opt.extract()) == state.sigstore_root_sha3_256
    }

    fun full_attestation(state: &Verification): vector<u8> {
        if (state.uploaded_chunks == 0) { return *(&state.attestation) };
        let bytes = vector[];
        let index = 0;
        while (index < state.uploaded_chunks) {
            bytes.append(*state.chunks.borrow(index));
            index += 1;
        };
        bytes
    }

    fun valid_quote_claims(quote: &QuoteStage): bool {
        quote.report_data.length() == 64
            && quote.tls_spki_sha256.length() == 32
            && quote.hpke_key.length() == 32
            && quote.quote_sha256.length() == 32
            && quote.mrtd.length() == 48
            && quote.rtmr0.length() == 48
            && quote.rtmr1.length() == 48
            && quote.rtmr2.length() == 48
            && quote.rtmr3.length() == 48
    }

    fun fail_verification(verification: address, reason: u8) {
        Verification[verification].status = VERIFICATION_FAIL;
        emit_progress(verification, VERIFICATION_FAIL, reason);
    }

    fun emit_progress(verification: address, status: u8, reason: u8) {
        emit(VerificationAdvanced {
            verification,
            status,
            next_stage: Verification[verification].stage,
            reason,
        });
    }

    #[view]
    /// 0 = CONTINUE, 1 = FAIL, 2 = PASS. Unknown addresses abort.
    public fun verification_status(verification: address): u8 {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        Verification[verification].status
    }

    #[view]
    /// Next internal stage. Stage 6 means a chunked report is still uploading.
    public fun verification_stage(verification: address): u8 {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        Verification[verification].stage
    }

    #[view]
    public fun verification_creator(verification: address): address {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        Verification[verification].creator
    }

    #[view]
    /// This commits to the submitted bytes. It does not authenticate them.
    public fun submitted_attestation_sha3_256(verification: address): vector<u8> {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        Verification[verification].attestation_sha3_256
    }

    #[view]
    /// The bytes are available through the object and this bounded chunk view.
    /// Index zero returns the inline document for a one-transaction object.
    public fun submitted_attestation_chunk(verification: address, index: u64): vector<u8> {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        let state = &Verification[verification];
        if (state.uploaded_chunks == 0) {
            assert!(index == 0 && !state.attestation.is_empty(),
                error::invalid_argument(EINVALID_REQUEST));
            *(&state.attestation)
        } else {
            assert!(index < state.uploaded_chunks,
                error::invalid_argument(EINVALID_REQUEST));
            *state.chunks.borrow(index)
        }
    }

    #[view]
    public fun submitted_attestation_chunk_count(verification: address): u64 {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        let state = &Verification[verification];
        if (state.uploaded_chunks == 0 && !state.attestation.is_empty()) {
            1
        } else {
            state.uploaded_chunks
        }
    }

    #[view]
    /// True only while the PASS result is recent and its observed roots and
    /// repository approval still match the values used by verification.
    public fun is_current_verification(verification: address, max_age_secs: u64): bool {
        if (max_age_secs > MAX_RESULT_AGE_SECS || !exists<Verification>(verification)
            || Verification[verification].status != VERIFICATION_PASS) { return false };
        let state = &Verification[verification];
        let result = state.result.borrow();
        let now = timestamp::now_seconds();
        now >= result.verified_at_secs
            && now - result.verified_at_secs <= max_age_secs
            && is_repository_approved(result.claims.trusted_repo)
            && object_roots_match(verification)
    }

    fun passed_result(verification: address): VerifiedTinfoilV3Result {
        assert!(exists<Verification>(verification),
            error::not_found(EUNKNOWN_VERIFICATION));
        assert!(Verification[verification].status == VERIFICATION_PASS
            && Verification[verification].result.is_some(),
            error::invalid_state(EVERIFICATION_NOT_PASSED));
        assert!(is_current_verification(verification, MAX_RESULT_AGE_SECS),
            error::invalid_state(EVERIFICATION_NOT_PASSED));
        *Verification[verification].result.borrow()
    }

    #[view]
    public fun verified_report(verification: address): VerifiedTinfoilV3Result {
        passed_result(verification)
    }

    #[view]
    public fun verified_tls_spki_sha256(verification: address): vector<u8> {
        passed_result(verification).claims.tls_spki_sha256
    }

    #[view]
    public fun verified_hpke_key(verification: address): vector<u8> {
        passed_result(verification).claims.hpke_key
    }

    #[view]
    public fun verified_release_digest(verification: address): vector<u8> {
        passed_result(verification).claims.release_digest
    }

    #[view]
    public fun verified_trusted_repo(verification: address): vector<u8> {
        passed_result(verification).claims.trusted_repo
    }

    #[view]
    public fun verified_nonce(verification: address): vector<u8> {
        passed_result(verification).claims.nonce
    }

    #[view]
    public fun verified_mrtd(verification: address): vector<u8> {
        passed_result(verification).claims.mrtd
    }

    #[view]
    public fun verified_rtmr0(verification: address): vector<u8> {
        passed_result(verification).claims.rtmr0
    }

    #[view]
    public fun verified_rtmr1(verification: address): vector<u8> {
        passed_result(verification).claims.rtmr1
    }

    #[view]
    public fun verified_rtmr2(verification: address): vector<u8> {
        passed_result(verification).claims.rtmr2
    }

    #[view]
    public fun verified_rtmr3(verification: address): vector<u8> {
        passed_result(verification).claims.rtmr3
    }

    #[view]
    public fun verified_tcb_status(verification: address): vector<u8> {
        passed_result(verification).claims.tcb_status
    }

    #[view]
    public fun verified_release_tag(verification: address): vector<u8> {
        passed_result(verification).claims.release_tag
    }

    #[view]
    public fun verified_release_commit(verification: address): vector<u8> {
        passed_result(verification).claims.release_commit
    }

    #[view]
    public fun verified_platform_measurement_id(verification: address): vector<u8> {
        passed_result(verification).claims.platform_measurement_id
    }

    #[view]
    public fun verified_shape_gpus(verification: address): u64 {
        passed_result(verification).claims.shape_gpus
    }

    /// Governance explicitly enables or revokes a GitHub repository. This
    /// string must match the repository identity in verified Sigstore bundles.
    public fun set_repository_approved(framework: &signer, repo: vector<u8>, approved: bool) {
        system_addresses::assert_aptos_framework(framework);
        assert!(repo.length() > 0 && repo.length() <= MAX_REPOSITORY_BYTES,
            error::invalid_argument(EINVALID_REPOSITORY));
        if (!exists<ApprovedRepositories>(@aptos_framework)) {
            move_to(framework, ApprovedRepositories { entries: vector[] });
        };
        let entries = &mut ApprovedRepositories[@aptos_framework].entries;
        let (found, index) = entries.index_of(&repo);
        if (approved && !found) {
            assert!(entries.length() < MAX_REPOSITORIES,
                error::resource_exhausted(ETOO_MANY_REPOSITORIES));
            entries.push_back(repo);
        } else if (!approved && found) {
            entries.remove(index);
        };
    }

    #[view]
    public fun is_repository_approved(repo: vector<u8>): bool {
        exists<ApprovedRepositories>(@aptos_framework)
            && ApprovedRepositories[@aptos_framework].entries.contains(&repo)
    }

    /// Creates a fresh nonce from validator randomness, scoped to this chain,
    /// account and request ID. The v3 document must quote exactly these 32 bytes.
    /// This must only be called by the private `#[randomness]` entry point.
    fun begin(
        owner: &signer,
        trusted_repo: vector<u8>,
        expected_tls_spki_sha256: vector<u8>,
        lifetime_secs: u64,
    ): vector<u8> {
        assert!(is_repository_approved(copy trusted_repo),
            error::permission_denied(EUNAPPROVED_REPOSITORY));
        assert!(expected_tls_spki_sha256.length() == TLS_SPKI_SHA256_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        assert!(lifetime_secs > 0 && lifetime_secs <= MAX_REQUEST_LIFETIME_SECS,
            error::invalid_argument(EINVALID_REQUEST));
        let addr = owner.address_of();
        if (!exists<State>(addr)) {
            move_to(owner, State {
                next_request_id: 0,
                pending: option::none(),
                last_verified: option::none(),
            });
        };
        let state = &mut State[addr];
        assert!(state.pending.is_none(), error::invalid_state(EREQUEST_PENDING));
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        let nonce = hash::sha3_256(bcs::to_bytes(&ChallengeMaterial {
            domain: CHALLENGE_DOMAIN,
            chain_id: chain_id::get(),
            owner: addr,
            request_id,
            entropy: randomness::bytes(32),
        }));
        let expires_at_secs = timestamp::now_seconds() + lifetime_secs;
        state.pending.fill(PendingRequest {
            request_id,
            nonce: copy nonce,
            trusted_repo: copy trusted_repo,
            expected_tls_spki_sha256,
            expires_at_secs,
            chunks: table_with_length::new(),
            chunk_count: 0,
            total_bytes: 0,
            stage: STAGE_QUOTE,
            stage_start_chunk: 0,
            stage_bytes: 0,
            proof_time_secs: 0,
            intel_root_sha3_256: vector[],
            sigstore_root_sha3_256: vector[],
            quote: option::none(),
            code: option::none(),
            platform: option::none(),
            closed: false,
            cleanup_cursor: 0,
        });
        emit(ChallengeIssued { owner: addr, request_id, nonce: copy nonce, trusted_repo, expires_at_secs });
        nonce
    }

    #[randomness]
    /// Transaction entry point; reads the nonce from `pending_nonce` or the event.
    /// It is private so a caller cannot inspect and abort on the random result.
    entry fun begin_entry(
        owner: &signer,
        trusted_repo: vector<u8>,
        expected_tls_spki_sha256: vector<u8>,
        lifetime_secs: u64,
    ) {
        begin(owner, trusted_repo, expected_tls_spki_sha256, lifetime_secs);
    }

    /// Uploads one stage's fragment in transaction-sized chunks. A stage
    /// transition seals its current fragment; subsequent chunks belong only
    /// to the next stage.
    public entry fun append_chunk(owner: &signer, chunk: vector<u8>) {
        let addr = owner.address_of();
        assert!(exists<State>(addr) && State[addr].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        assert!(chunk.length() > 0 && chunk.length() <= MAX_CHUNK_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        let pending = State[addr].pending.borrow_mut();
        assert!(!pending.closed, error::invalid_state(ENO_PENDING_REQUEST));
        assert!(pending.stage < STAGE_COMPLETE, error::invalid_state(EWRONG_STAGE));
        assert!(timestamp::now_seconds() <= pending.expires_at_secs,
            error::invalid_state(EEXPIRED_REQUEST));
        assert!(pending.chunk_count < MAX_CHUNKS
            && pending.total_bytes + chunk.length() <= MAX_DOCUMENT_BYTES
            && pending.stage_bytes + chunk.length() <= MAX_STAGE_BYTES,
            error::resource_exhausted(EINVALID_REQUEST));
        pending.total_bytes += chunk.length();
        pending.stage_bytes += chunk.length();
        pending.chunks.add(pending.chunk_count, chunk);
        pending.chunk_count += 1;
    }

    /// Closes a request. Its chunks can be removed in bounded transactions with
    /// `clean_up`, including by a third party after the request expires.
    public entry fun cancel(owner: &signer) {
        let addr = owner.address_of();
        assert!(exists<State>(addr) && State[addr].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        State[addr].pending.borrow_mut().closed = true;
        clean_up(addr, MAX_CLEANUP_CHUNKS_PER_CALL);
    }

    /// Frees at most eight staged chunks. Anyone can clean up an expired or
    /// completed request; no untrusted caller can erase an active one.
    public entry fun clean_up(owner: address, max_chunks: u64) {
        assert!(exists<State>(owner) && State[owner].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        assert!(max_chunks > 0 && max_chunks <= MAX_CLEANUP_CHUNKS_PER_CALL,
            error::invalid_argument(EINVALID_REQUEST));
        let pending = State[owner].pending.borrow_mut();
        assert!(pending.closed || timestamp::now_seconds() > pending.expires_at_secs,
            error::invalid_state(ENOT_READY_FOR_CLEANUP));
        let removed = 0;
        while (removed < max_chunks && pending.cleanup_cursor < pending.chunk_count) {
            pending.chunks.remove(pending.cleanup_cursor);
            pending.cleanup_cursor += 1;
            removed += 1;
        };
        if (pending.cleanup_cursor == pending.chunk_count) {
            let consumed = State[owner].pending.extract();
            destroy_empty_pending(consumed);
        };
    }

    /// Stage 0: verify the TDX quote and Intel collateral, then pin both
    /// effective roots and the chain time used for every later stage.
    public entry fun verify_quote(owner: &signer) {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_QUOTE);
        let pending = State[addr].pending.borrow();
        let (intel_root, sigstore_root) = effective_roots();
        let intel_root_hash = hash::sha3_256(copy intel_root);
        let sigstore_root_hash = hash::sha3_256(sigstore_root);
        let proof_time_secs = timestamp::now_seconds();
        let quote = verify_tinfoil_v3_quote(
            current_fragment(pending),
            pending.nonce,
            proof_time_secs,
            intel_root,
        );
        assert_quote_claims(pending, &quote);
        let request_id = pending.request_id;
        let pending = State[addr].pending.borrow_mut();
        pending.quote.fill(quote);
        pending.proof_time_secs = proof_time_secs;
        pending.intel_root_sha3_256 = intel_root_hash;
        pending.sigstore_root_sha3_256 = sigstore_root_hash;
        advance_stage(pending, STAGE_CODE);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_QUOTE });
    }

    /// Stage 1: authenticate the code-release Sigstore bundle and compare its
    /// reference values with the already authenticated TDX measurements.
    public entry fun verify_code(owner: &signer) {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_CODE);
        let pending = State[addr].pending.borrow();
        let (_, sigstore_root) = checked_roots(pending);
        let code = verify_tinfoil_v3_code(
            current_fragment(pending),
            pending.nonce,
            pending.trusted_repo,
            sigstore_root,
            *pending.quote.borrow(),
        );
        assert_code_claims(pending, &code);
        let request_id = pending.request_id;
        let pending = State[addr].pending.borrow_mut();
        pending.code.fill(code);
        advance_stage(pending, STAGE_CODE_FRESHNESS);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_CODE });
    }

    /// Stage 2: authenticate the freshness witness for the code release.
    public entry fun verify_code_freshness(owner: &signer) {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_CODE_FRESHNESS);
        let pending = State[addr].pending.borrow();
        let (_, sigstore_root) = checked_roots(pending);
        verify_tinfoil_v3_code_freshness(
            current_fragment(pending), pending.nonce, pending.proof_time_secs,
            sigstore_root, *pending.quote.borrow(), *pending.code.borrow(),
        );
        let request_id = pending.request_id;
        advance_stage(State[addr].pending.borrow_mut(), STAGE_PLATFORM);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_CODE_FRESHNESS });
    }

    /// Stage 3: authenticate the platform endorsement and match its reference
    /// measurements and platform identity to the TDX quote.
    public entry fun verify_platform(owner: &signer) {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_PLATFORM);
        let pending = State[addr].pending.borrow();
        let (_, sigstore_root) = checked_roots(pending);
        let platform = verify_tinfoil_v3_platform(
            current_fragment(pending), pending.nonce, pending.proof_time_secs,
            sigstore_root, *pending.quote.borrow(), *pending.code.borrow(),
        );
        assert!(platform.digest.length() == 32 && platform.measurement_id.length() > 0,
            error::invalid_state(EINVALID_VERIFIED_CLAIMS));
        let request_id = pending.request_id;
        let pending = State[addr].pending.borrow_mut();
        pending.platform.fill(platform);
        advance_stage(pending, STAGE_PLATFORM_FRESHNESS);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_PLATFORM });
    }

    /// Stage 4: authenticate the platform freshness witness. Only this stage
    /// can publish a final result, joining native-verified private outputs.
    public fun verify_platform_freshness(owner: &signer): VerifiedTinfoilV3Result {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_PLATFORM_FRESHNESS);
        let pending = State[addr].pending.borrow();
        let (_, sigstore_root) = checked_roots(pending);
        verify_tinfoil_v3_platform_freshness(
            current_fragment(pending), pending.nonce, pending.proof_time_secs,
            sigstore_root, *pending.quote.borrow(), *pending.platform.borrow(),
        );
        let quote = *pending.quote.borrow();
        let code = *pending.code.borrow();
        let platform = *pending.platform.borrow();
        let claims = VerifiedTinfoilV3Claims {
            tls_spki_sha256: quote.tls_spki_sha256,
            hpke_key: quote.hpke_key,
            release_digest: code.digest,
            trusted_repo: pending.trusted_repo,
            nonce: pending.nonce,
            mrtd: quote.mrtd,
            rtmr0: quote.rtmr0,
            rtmr1: quote.rtmr1,
            rtmr2: quote.rtmr2,
            rtmr3: quote.rtmr3,
            tcb_status: quote.tcb_status,
            release_tag: code.tag,
            release_commit: code.commit,
            platform_measurement_id: platform.measurement_id,
            shape_gpus: code.shape_gpus,
        };
        let result = VerifiedTinfoilV3Result {
            request_id: pending.request_id,
            verified_at_secs: pending.proof_time_secs,
            intel_root_sha3_256: pending.intel_root_sha3_256,
            sigstore_root_sha3_256: pending.sigstore_root_sha3_256,
            claims,
        };
        let request_id = pending.request_id;
        let pending = State[addr].pending.borrow_mut();
        pending.closed = true;
        advance_stage(pending, STAGE_COMPLETE);
        State[addr].last_verified = option::some(copy result);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_PLATFORM_FRESHNESS });
        emit(TinfoilV3FormatV2Verified { owner: addr, result: copy result });
        result
    }

    /// Transaction entry point; a successful result is stored and emitted.
    public entry fun verify_entry(owner: &signer) {
        verify_platform_freshness(owner);
    }

    #[view]
    public fun pending_nonce(owner: address): Option<vector<u8>> {
        if (!exists<State>(owner) || State[owner].pending.is_none()
            || State[owner].pending.borrow().closed) {
            option::none()
        } else {
            option::some(State[owner].pending.borrow().nonce)
        }
    }

    // The next proof stage, or none when no active request exists.
    #[view]
    public fun pending_stage(owner: address): Option<u8> {
        if (!exists<State>(owner) || State[owner].pending.is_none()
            || State[owner].pending.borrow().closed) {
            option::none()
        } else {
            option::some(State[owner].pending.borrow().stage)
        }
    }

    // The result proves only v3-envelope / legacy FormatV2 collateral, and is
    // current only while both trust roots and the repository approval remain.
    #[view]
    public fun last_verified(owner: address): Option<VerifiedTinfoilV3Result> {
        if (!exists<State>(owner)) { option::none() } else { State[owner].last_verified }
    }

    #[view]
    /// Returns false after root rotation, repository revocation or age limit.
    /// Applications may impose a shorter `max_age_secs` than one hour.
    public fun is_current(owner: address, max_age_secs: u64): bool {
        if (max_age_secs > MAX_RESULT_AGE_SECS || !exists<State>(owner)
            || State[owner].last_verified.is_none()) { return false };
        let result = State[owner].last_verified.borrow();
        let now = timestamp::now_seconds();
        if (now < result.verified_at_secs || now - result.verified_at_secs > max_age_secs
            || !is_repository_approved(result.claims.trusted_repo)) { return false };
        let intel_root = trust_observation::get_effective_value(INTEL_ROOT_TARGET);
        let sigstore_root = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET);
        intel_root.is_some() && sigstore_root.is_some()
            && hash::sha3_256(intel_root.extract()) == result.intel_root_sha3_256
            && hash::sha3_256(sigstore_root.extract()) == result.sigstore_root_sha3_256
    }

    public fun request_id(self: &VerifiedTinfoilV3Result): u64 { self.request_id }

    public fun verified_at_secs(self: &VerifiedTinfoilV3Result): u64 { self.verified_at_secs }

    public fun intel_root_sha3_256(self: &VerifiedTinfoilV3Result): &vector<u8> {
        &self.intel_root_sha3_256
    }

    public fun sigstore_root_sha3_256(self: &VerifiedTinfoilV3Result): &vector<u8> {
        &self.sigstore_root_sha3_256
    }

    public fun claims(self: &VerifiedTinfoilV3Result): &VerifiedTinfoilV3Claims { &self.claims }

    public fun tls_spki_sha256(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.tls_spki_sha256 }

    public fun hpke_key(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.hpke_key }

    public fun release_digest(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.release_digest }

    public fun trusted_repo(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.trusted_repo }

    public fun nonce(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.nonce }

    public fun mrtd(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.mrtd }

    public fun rtmr0(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.rtmr0 }

    public fun rtmr1(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.rtmr1 }

    public fun rtmr2(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.rtmr2 }

    public fun rtmr3(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.rtmr3 }

    public fun tcb_status(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.tcb_status }

    public fun release_tag(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.release_tag }

    public fun release_commit(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.release_commit }

    public fun platform_measurement_id(self: &VerifiedTinfoilV3Claims): &vector<u8> {
        &self.platform_measurement_id
    }

    /// GPU count from the signed VM shape whose TDX-bound device topology
    /// was checked. Device signatures are not independently verified on chain.
    public fun shape_gpus(self: &VerifiedTinfoilV3Claims): u64 { self.shape_gpus }

    fun assert_open_stage(owner: address, stage: u8) {
        assert!(exists<State>(owner) && State[owner].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        let pending = State[owner].pending.borrow();
        assert!(!pending.closed && pending.stage == stage,
            error::invalid_state(EWRONG_STAGE));
        assert!(timestamp::now_seconds() <= pending.expires_at_secs,
            error::invalid_state(EEXPIRED_REQUEST));
        assert!(is_repository_approved(pending.trusted_repo),
            error::permission_denied(EUNAPPROVED_REPOSITORY));
        assert!(pending.stage_bytes > 0 && pending.stage_bytes <= MAX_STAGE_BYTES,
            error::invalid_state(EINVALID_REQUEST));
    }

    fun assert_quote_claims(pending: &PendingRequest, quote: &QuoteStage) {
        assert!(quote.report_data.length() == 64
            && quote.tls_spki_sha256.length() == 32
            && quote.hpke_key.length() == 32
            && quote.quote_sha256.length() == 32
            && quote.mrtd.length() == 48
            && quote.rtmr0.length() == 48
            && quote.rtmr1.length() == 48
            && quote.rtmr2.length() == 48
            && quote.rtmr3.length() == 48
            && quote.tls_spki_sha256 == pending.expected_tls_spki_sha256,
            error::invalid_state(EINVALID_VERIFIED_CLAIMS));
    }

    fun assert_code_claims(pending: &PendingRequest, code: &CodeStage) {
        assert!(code.repo == pending.trusted_repo && code.digest.length() == 32
            && code.tag.length() > 0 && code.commit.length() == 40,
            error::invalid_state(EINVALID_VERIFIED_CLAIMS));
    }

    fun effective_roots(): (vector<u8>, vector<u8>) {
        let intel = trust_observation::get_effective_value(INTEL_ROOT_TARGET);
        let sigstore = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET);
        assert!(intel.is_some() && sigstore.is_some(),
            error::invalid_state(ETRUST_ROOT_UNAVAILABLE));
        let intel = intel.extract();
        let sigstore = sigstore.extract();
        assert!(!intel.is_empty() && !sigstore.is_empty(),
            error::invalid_state(ETRUST_ROOT_UNAVAILABLE));
        (intel, sigstore)
    }

    fun checked_roots(pending: &PendingRequest): (vector<u8>, vector<u8>) {
        let (intel, sigstore) = effective_roots();
        assert!(hash::sha3_256(copy intel) == pending.intel_root_sha3_256
            && hash::sha3_256(copy sigstore) == pending.sigstore_root_sha3_256,
            error::invalid_state(ETRUST_ROOT_CHANGED));
        (intel, sigstore)
    }

    fun current_fragment(pending: &PendingRequest): vector<u8> {
        let fragment = vector[];
        let index = pending.stage_start_chunk;
        while (index < pending.chunk_count) {
            fragment.append(*pending.chunks.borrow(index));
            index += 1;
        };
        assert!(fragment.length() == pending.stage_bytes,
            error::invalid_state(EINVALID_REQUEST));
        fragment
    }

    fun advance_stage(pending: &mut PendingRequest, next: u8) {
        pending.stage = next;
        pending.stage_start_chunk = pending.chunk_count;
        pending.stage_bytes = 0;
    }

    fun destroy_empty_pending(pending: PendingRequest) {
        let PendingRequest {
            request_id: _,
            nonce: _,
            trusted_repo: _,
            expected_tls_spki_sha256: _,
            expires_at_secs: _,
            chunks,
            chunk_count,
            total_bytes: _,
            stage: _,
            stage_start_chunk: _,
            stage_bytes: _,
            proof_time_secs: _,
            intel_root_sha3_256: _,
            sigstore_root_sha3_256: _,
            quote: _,
            code: _,
            platform: _,
            closed: _,
            cleanup_cursor,
        } = pending;
        assert!(cleanup_cursor == chunk_count, error::invalid_state(EINVALID_REQUEST));
        chunks.destroy_empty();
    }

    native fun verify_tinfoil_v3_quote(
        fragment: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        intel_root_der: vector<u8>,
    ): QuoteStage;

    /// These object-flow natives return ordinary invalid evidence as none or
    /// false, allowing `advance_verification` to persist a FAIL state. Runtime
    /// failures such as exhausted gas still abort the transaction.
    native fun try_verify_tinfoil_v3_quote(
        attestation: vector<u8>,
        proof_time_secs: u64,
        intel_root_der: vector<u8>,
    ): Option<QuoteOutcome>;

    native fun try_verify_tinfoil_v3_code(
        attestation: vector<u8>,
        nonce: vector<u8>,
        trusted_repo: vector<u8>,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
    ): Option<CodeStage>;

    native fun try_verify_tinfoil_v3_code_freshness(
        attestation: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        code: CodeStage,
    ): bool;

    native fun try_verify_tinfoil_v3_platform(
        attestation: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        code: CodeStage,
    ): Option<PlatformStage>;

    native fun try_verify_tinfoil_v3_platform_freshness(
        attestation: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        platform: PlatformStage,
    ): bool;

    native fun verify_tinfoil_v3_code(
        fragment: vector<u8>,
        nonce: vector<u8>,
        trusted_repo: vector<u8>,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
    ): CodeStage;

    native fun verify_tinfoil_v3_code_freshness(
        fragment: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        code: CodeStage,
    );

    native fun verify_tinfoil_v3_platform(
        fragment: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        code: CodeStage,
    ): PlatformStage;

    native fun verify_tinfoil_v3_platform_freshness(
        fragment: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        platform: PlatformStage,
    );

    #[test_only]
    fun filled_byte(value: u8, count: u64): vector<u8> {
        let bytes = vector[];
        for (_i in 0..count) { bytes.push_back(value); };
        bytes
    }

    #[test_only]
    /// The former challenge preimage, retained to catch a return to a
    /// predictable nonce derived solely from public request fields.
    struct PredictableChallengeMaterial has drop {
        domain: vector<u8>,
        chain_id: u8,
        owner: address,
        request_id: u64,
    }

    #[test_only]
    fun setup_roots_for_test(framework: &signer) {
        aptos_framework::account::create_account_for_test(@aptos_framework);
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        aptos_framework::config_buffer::initialize(framework);
        aptos_framework::reconfiguration::initialize_for_test(framework);
        trust_observation::initialize(framework);
        trust_observation::upsert_target_for_next_epoch(
            framework, INTEL_ROOT_TARGET, std::string::utf8(b"https://intel.example/root.cer"), 1, 4096,
        );
        trust_observation::upsert_target_for_next_epoch(
            framework, SIGSTORE_ROOT_TARGET, std::string::utf8(b"https://sigstore.example/trusted_root.json"), 2, 4096,
        );
        trust_observation::activate_for_test(framework);
        trust_observation::set_replacement_patch(framework, INTEL_ROOT_TARGET, b"intel root v1");
        trust_observation::set_replacement_patch(framework, SIGSTORE_ROOT_TARGET, b"sigstore root v1");
        set_repository_approved(framework, b"tinfoilsh/example", true);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    fun test_nonce_requires_unpredictable_entropy(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        let predictable = hash::sha3_256(bcs::to_bytes(&PredictableChallengeMaterial {
            domain: CHALLENGE_DOMAIN,
            chain_id: chain_id::get(),
            owner: owner.address_of(),
            request_id: 0,
        }));
        let issued = begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        assert!(issued != predictable, 1);
        assert!(pending_nonce(owner.address_of()).extract() == issued, 2);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x3000C, location = Self)]
    fun test_reject_stage_out_of_order(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        append_chunk(owner, b"untrusted code fragment");
        verify_code(owner);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_revocation_blocks_in_flight_request(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        append_chunk(owner, b"untrusted quote fragment");
        set_repository_approved(framework, b"tinfoilsh/example", false);
        verify_quote(owner);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x3000D, location = Self)]
    fun test_root_rotation_blocks_later_stage(framework: &signer, owner: &signer) {
        setup_roots_for_test(framework);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        append_chunk(owner, b"untrusted code fragment");
        // Simulate the quote stage having pinned the old roots. No proof output
        // is fabricated: verify_code aborts at the root check before native use.
        let pending = State[owner.address_of()].pending.borrow_mut();
        pending.stage = STAGE_CODE;
        pending.intel_root_sha3_256 = hash::sha3_256(b"intel root v1");
        pending.sigstore_root_sha3_256 = hash::sha3_256(b"sigstore root v1");
        trust_observation::set_replacement_patch(framework, INTEL_ROOT_TARGET, b"intel root v2");
        verify_code(owner);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x3000A, location = Self)]
    fun test_quote_key_must_match_request(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        let quote = QuoteStage {
            report_data: filled_byte(0, 64),
            tls_spki_sha256: filled_byte(2, 32),
            hpke_key: filled_byte(3, 32),
            quote_sha256: filled_byte(4, 32),
            mrtd: filled_byte(0, 48),
            rtmr0: filled_byte(0, 48),
            rtmr1: filled_byte(0, 48),
            rtmr2: filled_byte(0, 48),
            rtmr3: filled_byte(0, 48),
            tcb_status: b"UpToDate",
            ppid: filled_byte(0, 16),
            qe_vendor_id: filled_byte(0, 16),
            tee_tcb_svn: filled_byte(0, 16),
            mr_seam: filled_byte(0, 48),
            td_attributes: filled_byte(0, 8),
            xfam: filled_byte(0, 8),
            tcb_evaluation_data_number: 1,
        };
        assert_quote_claims(State[owner.address_of()].pending.borrow(), &quote);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x3000A, location = Self)]
    fun test_code_repo_must_match_request(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        let code = CodeStage {
            repo: b"attacker/other", tag: b"v1", commit: filled_byte(97, 40),
            subject_name: b"image", digest: filled_byte(2, 32),
            shape_cpus: 1, shape_memory_mb: 1024, shape_gpus: 0, shape_disks: 1,
        };
        assert_code_claims(State[owner.address_of()].pending.borrow(), &code);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = EINVALID_TINFOIL_PROOF, location = Self)]
    fun test_reject_malformed_quote_fragment(framework: &signer, owner: &signer) {
        setup_roots_for_test(framework);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        append_chunk(owner, b"malformed v3 fragment");
        verify_quote(owner);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    fun test_chunked_lifecycle(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        let tls_spki = vector[];
        for (i in 0..32) { tls_spki.push_back(1); };
        let first = begin(owner, b"tinfoilsh/example", copy tls_spki, 60);
        assert!(first.length() == 32, 1);
        append_chunk(owner, b"first");
        append_chunk(owner, b"second");
        for (_i in 0..8) { append_chunk(owner, b"x"); };
        assert!(State[owner.address_of()].pending.borrow().total_bytes == 19, 2);
        cancel(owner);
        assert!(pending_nonce(owner.address_of()).is_none(), 3);
        assert!(State[owner.address_of()].pending.is_some(), 4);
        clean_up(owner.address_of(), 8);
        let second = begin(owner, b"tinfoilsh/example", tls_spki, 60);
        assert!(first != second, 5);
        set_repository_approved(framework, b"tinfoilsh/example", false);
        assert!(!is_repository_approved(b"tinfoilsh/example"), 6);
    }

    #[test(creator = @0x42)]
    fun test_verification_object_contains_immutable_inline_report(creator: &signer) {
        let document = b"a small submitted report";
        let digest = hash::sha3_256(copy document);
        let address = create_verification(
            creator, copy document, document.length(), copy digest, STAGE_QUOTE,
        );
        assert!(verification_creator(address) == creator.address_of(), 1);
        assert!(verification_status(address) == VERIFICATION_CONTINUE, 2);
        assert!(submitted_attestation_sha3_256(address) == digest, 3);
        assert!(submitted_attestation_chunk(address, 0) == document, 4);
        assert!(submitted_attestation_chunk_count(address) == 1, 5);
        assert!(object::address_to_object<Verification>(address).is_untransferable(), 6);
    }

    #[test(creator = @0x42)]
    fun test_chunked_object_stores_separate_entries(creator: &signer) {
        let chunk = filled_byte(7, MAX_CHUNK_BYTES);
        let document = vector[];
        for (_i in 0..5) { document.append(copy chunk); };
        let digest = hash::sha3_256(copy document);
        let address = create_verification(
            creator, vector[], document.length(), digest, STAGE_UPLOAD,
        );
        for (_i in 0..5) {
            append_verification_chunk(creator, address, copy chunk);
        };
        assert!(Verification[address].attestation.is_empty(), 1);
        assert!(submitted_attestation_chunk_count(address) == 5, 2);
        assert!(submitted_attestation_chunk(address, 4) == chunk, 3);
        seal_verification(creator, address);
        assert!(verification_stage(address) == STAGE_QUOTE, 4);
        assert!(verification_status(address) == VERIFICATION_CONTINUE, 5);
    }

    #[test(creator = @0x42)]
    #[expected_failure(abort_code = 0x10008, location = Self)]
    fun test_chunked_object_rejects_unadvanceable_size(creator: &signer) {
        new_verification_chunked(
            creator, MAX_VERIFICATION_BYTES + 1, filled_byte(0, 32),
        );
    }

    #[test(framework = @aptos_framework, creator = @0x42)]
    fun test_invalid_object_proof_is_durable_fail(framework: &signer, creator: &signer) {
        setup_roots_for_test(framework);
        let document = b"malformed attestation";
        let address = create_verification(
            creator, copy document, document.length(), hash::sha3_256(document), STAGE_QUOTE,
        );
        advance_verification(creator, address);
        assert!(verification_status(address) == VERIFICATION_FAIL, 1);
        advance_verification(creator, address);
        assert!(verification_status(address) == VERIFICATION_FAIL, 2);
    }

    #[test(creator = @0x42)]
    #[expected_failure(abort_code = 0x3000F, location = Self)]
    fun test_verified_getter_rejects_unfinished_object(creator: &signer) {
        let document = b"not verified";
        let address = create_verification(
            creator, copy document, document.length(), hash::sha3_256(document), STAGE_QUOTE,
        );
        verified_tls_spki_sha256(address);
    }
}
